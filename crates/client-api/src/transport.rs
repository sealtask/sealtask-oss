use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime};

use sealtask_client_auth::{Credentials, refresh_credentials_if_needed_with_timeout};
use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind, TransportFailureKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const ACCESS_TOKEN_REFRESH_WINDOW_SECONDS: i64 = 60;
const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_API_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_BASE_DELAY_MILLIS: u64 = 200;
const RETRY_BACKOFF_CAP_MILLIS: u64 = 2_000;
pub const DEFAULT_API_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_API_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_API_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_API_MAX_RETRIES: u8 = 0;
pub const MAX_API_RETRIES: u8 = 10;
pub const MAX_API_RETRY_DELAY: Duration = Duration::from_secs(30);
pub const CONTROL_PLANE_USER_AGENT: &str =
    concat!("sealtask-client-api/", env!("CARGO_PKG_VERSION"));
pub(crate) const MAX_RETRY_AFTER_SECONDS: u64 = 24 * 60 * 60;

#[derive(Clone, Copy)]
pub(crate) enum RequestSemantics {
    Read,
    ReplaySafeMutation,
    StateTransition { operation: &'static str },
    UnsafeMutation { operation: &'static str },
    ControlPlaneNoReplay,
    DurableMutationNoReplay { operation: &'static str },
}

impl RequestSemantics {
    const fn permits_replay(self) -> bool {
        matches!(
            self,
            Self::Read | Self::ReplaySafeMutation | Self::StateTransition { .. }
        )
    }

    const fn operation(self) -> Option<&'static str> {
        match self {
            Self::StateTransition { operation } | Self::UnsafeMutation { operation } => {
                Some(operation)
            }
            Self::DurableMutationNoReplay { operation } => Some(operation),
            Self::Read | Self::ReplaySafeMutation | Self::ControlPlaneNoReplay => None,
        }
    }

    const fn success_confirms_durable_mutation(self) -> bool {
        self.is_durable_mutation()
    }

    const fn requires_ambiguity(self, delivery_may_have_occurred: bool) -> bool {
        delivery_may_have_occurred
            && matches!(
                self,
                Self::StateTransition { .. } | Self::UnsafeMutation { .. }
            )
    }

    const fn is_durable_mutation(self) -> bool {
        !matches!(self, Self::Read | Self::ControlPlaneNoReplay)
    }
}

/// Scope-local cooperative cancellation for API work.
///
/// SDK callers do not need to configure a token. The CLI gives independently
/// supervised work its own token so a signal handler can distinguish a
/// rotating credential refresh, which must finish durable persistence, from a
/// resource mutation that may already have committed.
#[derive(Clone, Debug)]
pub struct ApiCancellationToken {
    inner: Arc<ApiCancellationState>,
}

#[derive(Debug, Default)]
struct ApiCancellationState {
    cancellation_requested: AtomicBool,
    credential_refreshes: AtomicUsize,
    credential_refresh_boundary: AtomicU8,
    mutation_requests: AtomicUsize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum CredentialRefreshBoundary {
    NotStarted = 0,
    InFlight = 1,
    DurablyPersisted = 2,
    DefinitivelyFailed = 3,
    MayHaveRotated = 4,
}

impl CredentialRefreshBoundary {
    fn load(state: &AtomicU8) -> Self {
        match state.load(Ordering::Acquire) {
            0 => Self::NotStarted,
            1 => Self::InFlight,
            2 => Self::DurablyPersisted,
            3 => Self::DefinitivelyFailed,
            4 => Self::MayHaveRotated,
            _ => unreachable!("credential refresh boundary state is internal"),
        }
    }
}

impl ApiCancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation at the next safe pre-mutation boundary.
    pub fn cancel(&self) {
        self.inner
            .cancellation_requested
            .store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancellation_requested.load(Ordering::Acquire)
    }

    /// Reports whether rotating credentials are being refreshed and durably persisted.
    #[must_use]
    pub fn credential_refresh_in_flight(&self) -> bool {
        self.inner.credential_refreshes.load(Ordering::Acquire) != 0
            || CredentialRefreshBoundary::load(&self.inner.credential_refresh_boundary)
                == CredentialRefreshBoundary::InFlight
    }

    /// Reports that a refresh response was lost after the server may have
    /// rotated the session and before durable local persistence was confirmed.
    #[must_use]
    pub fn credential_refresh_may_have_rotated(&self) -> bool {
        CredentialRefreshBoundary::load(&self.inner.credential_refresh_boundary)
            == CredentialRefreshBoundary::MayHaveRotated
    }

    /// Reports whether a requested resource mutation is currently in flight.
    #[must_use]
    pub fn mutation_request_in_flight(&self) -> bool {
        self.inner.mutation_requests.load(Ordering::Acquire) != 0
    }

    fn enter_credential_refresh(&self) -> ActiveRequestBoundaryGuard {
        // Publish the logical boundary before exposing the active guard. A
        // signal observer can therefore never see an active refresh paired
        // with stale state from an earlier refresh.
        self.inner
            .credential_refresh_boundary
            .store(CredentialRefreshBoundary::InFlight as u8, Ordering::Release);
        ActiveRequestBoundaryGuard::new(self, RequestBoundary::CredentialRefresh)
    }

    fn finish_credential_refresh(&self, result: &PublicResult<Credentials>) {
        let boundary = match result {
            Ok(_) => CredentialRefreshBoundary::DurablyPersisted,
            Err(error) if credential_refresh_failure_may_have_rotated(error) => {
                CredentialRefreshBoundary::MayHaveRotated
            }
            Err(_) => CredentialRefreshBoundary::DefinitivelyFailed,
        };
        // This is stored before the active guard drops, so a signal racing the
        // completed future sees either InFlight or the exact terminal state.
        self.inner
            .credential_refresh_boundary
            .store(boundary as u8, Ordering::Release);
    }

    fn enter_mutation_request(
        &self,
        semantics: RequestSemantics,
    ) -> Option<ActiveRequestBoundaryGuard> {
        semantics
            .is_durable_mutation()
            .then(|| ActiveRequestBoundaryGuard::new(self, RequestBoundary::Mutation))
    }
}

impl Default for ApiCancellationToken {
    fn default() -> Self {
        Self {
            inner: Arc::new(ApiCancellationState::default()),
        }
    }
}

impl PartialEq for ApiCancellationToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ApiCancellationToken {}

#[derive(Clone, Copy)]
enum RequestBoundary {
    CredentialRefresh,
    Mutation,
}

pub(crate) struct ActiveRequestBoundaryGuard {
    inner: Arc<ApiCancellationState>,
    boundary: RequestBoundary,
}

impl ActiveRequestBoundaryGuard {
    fn new(token: &ApiCancellationToken, boundary: RequestBoundary) -> Self {
        let counter = match boundary {
            RequestBoundary::CredentialRefresh => &token.inner.credential_refreshes,
            RequestBoundary::Mutation => &token.inner.mutation_requests,
        };
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                active.checked_add(1)
            })
            .expect("active API request boundary count overflowed");
        Self {
            inner: Arc::clone(&token.inner),
            boundary,
        }
    }
}

impl Drop for ActiveRequestBoundaryGuard {
    fn drop(&mut self) {
        let counter = match self.boundary {
            RequestBoundary::CredentialRefresh => &self.inner.credential_refreshes,
            RequestBoundary::Mutation => &self.inner.mutation_requests,
        };
        let decremented = counter.fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            active.checked_sub(1)
        });
        debug_assert!(
            decremented.is_ok(),
            "active API request boundary count underflowed"
        );
    }
}

/// Retry budget for one logical API request.
///
/// `max_retries` counts replay attempts after the initial request. Higher-level
/// reconciliation loops must account for their own requests separately.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ApiRetryPolicy {
    max_retries: u8,
}

impl ApiRetryPolicy {
    pub fn new(max_retries: u8) -> PublicResult<Self> {
        if max_retries > MAX_API_RETRIES {
            return Err(PublicError::validation(format!(
                "API retry count cannot exceed {MAX_API_RETRIES}"
            )));
        }
        Ok(Self { max_retries })
    }

    /// Number of replay attempts allowed after the initial request.
    #[must_use]
    pub const fn max_retries(self) -> u8 {
        self.max_retries
    }

    /// Maximum number of wire attempts for one logical request.
    #[must_use]
    pub const fn max_attempts(self) -> u8 {
        self.max_retries + 1
    }
}

impl Default for ApiRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_API_MAX_RETRIES,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ApiTransportOptions {
    connect_timeout: Duration,
    read_timeout: Duration,
    request_timeout: Duration,
    request_id: Option<Uuid>,
    retry_policy: ApiRetryPolicy,
}

impl ApiTransportOptions {
    pub fn new(
        connect_timeout: Duration,
        read_timeout: Duration,
        request_timeout: Duration,
    ) -> PublicResult<Self> {
        validate_timeout("connect", connect_timeout)?;
        validate_timeout("read", read_timeout)?;
        validate_timeout("request", request_timeout)?;
        if connect_timeout > request_timeout {
            return Err(PublicError::validation(
                "API connect timeout cannot exceed the overall request timeout",
            ));
        }
        if read_timeout > request_timeout {
            return Err(PublicError::validation(
                "API read timeout cannot exceed the overall request timeout",
            ));
        }
        Ok(Self {
            connect_timeout,
            read_timeout,
            request_timeout,
            request_id: None,
            retry_policy: ApiRetryPolicy::default(),
        })
    }

    /// Reuses `request_id` for every control-plane request made with these options.
    ///
    /// This is useful for correlating all requests made by one CLI invocation. Without this
    /// setting, [`PublicApiClient`] generates a fresh request ID for every request.
    pub const fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Applies a retry budget to each replay-safe logical request.
    #[must_use]
    pub const fn with_retry_policy(mut self, retry_policy: ApiRetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    pub const fn connect_timeout(self) -> Duration {
        self.connect_timeout
    }

    pub const fn read_timeout(self) -> Duration {
        self.read_timeout
    }

    pub const fn request_timeout(self) -> Duration {
        self.request_timeout
    }

    pub const fn request_id(self) -> Option<Uuid> {
        self.request_id
    }

    pub const fn retry_policy(self) -> ApiRetryPolicy {
        self.retry_policy
    }
}

impl Default for ApiTransportOptions {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_API_CONNECT_TIMEOUT,
            read_timeout: DEFAULT_API_READ_TIMEOUT,
            request_timeout: DEFAULT_API_REQUEST_TIMEOUT,
            request_id: None,
            retry_policy: ApiRetryPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RequestCorrelation {
    client_request_id: Uuid,
    response_request_id: Option<Uuid>,
    attempt_count: u8,
}

impl RequestCorrelation {
    pub const fn client_request_id(self) -> Uuid {
        self.client_request_id
    }

    pub const fn response_request_id(self) -> Option<Uuid> {
        self.response_request_id
    }

    pub const fn effective_request_id(self) -> Uuid {
        match self.response_request_id {
            Some(request_id) => request_id,
            None => self.client_request_id,
        }
    }

    /// Number of wire attempts made for this logical request.
    #[must_use]
    pub const fn attempt_count(self) -> u8 {
        self.attempt_count
    }
}

#[derive(Debug, Clone)]
pub struct PublicApiClient {
    client: reqwest::Client,
    base_url: String,
    credentials: Option<Credentials>,
    transport_options: ApiTransportOptions,
    cancellation_token: Option<ApiCancellationToken>,
    last_request_correlation: Option<RequestCorrelation>,
    #[cfg(test)]
    last_retry_seed: Option<Uuid>,
}

pub(crate) struct BoundedHttpResponse {
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    received_len: usize,
    body: PublicResult<Vec<u8>>,
    semantics: RequestSemantics,
    request_guard: Option<ActiveRequestBoundaryGuard>,
}

impl BoundedHttpResponse {
    fn complete(
        status: reqwest::StatusCode,
        retry_after: Option<Duration>,
        body: Vec<u8>,
        semantics: RequestSemantics,
    ) -> Self {
        Self {
            status,
            retry_after,
            received_len: body.len(),
            body: Ok(body),
            semantics,
            request_guard: None,
        }
    }

    fn failed(
        status: reqwest::StatusCode,
        retry_after: Option<Duration>,
        received_len: usize,
        error: PublicError,
        semantics: RequestSemantics,
    ) -> Self {
        Self {
            status,
            retry_after,
            received_len,
            body: Err(error),
            semantics,
            request_guard: None,
        }
    }

    pub(crate) fn status(&self) -> reqwest::StatusCode {
        self.status
    }

    pub(crate) fn received_len(&self) -> usize {
        self.received_len
    }

    pub(crate) const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    pub(crate) const fn semantics(&self) -> RequestSemantics {
        self.semantics
    }

    pub(crate) fn into_body_with_guard(
        self,
    ) -> (PublicResult<Vec<u8>>, Option<ActiveRequestBoundaryGuard>) {
        (self.body, self.request_guard)
    }

    fn with_request_guard(mut self, guard: Option<ActiveRequestBoundaryGuard>) -> Self {
        self.request_guard = guard;
        self
    }

    fn body_error(&self) -> Option<&PublicError> {
        self.body.as_ref().err()
    }
}

impl PublicApiClient {
    pub fn new(base_url: impl Into<String>) -> PublicResult<Self> {
        Self::new_with_options(base_url, ApiTransportOptions::default())
    }

    pub fn new_with_options(
        base_url: impl Into<String>,
        transport_options: ApiTransportOptions,
    ) -> PublicResult<Self> {
        Ok(Self {
            client: build_control_plane_http_client(transport_options)?,
            base_url: normalize_base_url(base_url.into()),
            credentials: None,
            transport_options,
            cancellation_token: None,
            last_request_correlation: None,
            #[cfg(test)]
            last_retry_seed: None,
        })
    }

    pub fn with_credentials(
        base_url: impl Into<String>,
        credentials: Credentials,
    ) -> PublicResult<Self> {
        Self::with_credentials_and_options(base_url, credentials, ApiTransportOptions::default())
    }

    pub fn with_credentials_and_options(
        base_url: impl Into<String>,
        credentials: Credentials,
        transport_options: ApiTransportOptions,
    ) -> PublicResult<Self> {
        Ok(Self {
            client: build_control_plane_http_client(transport_options)?,
            base_url: normalize_base_url(base_url.into()),
            credentials: Some(credentials),
            transport_options,
            cancellation_token: None,
            last_request_correlation: None,
            #[cfg(test)]
            last_retry_seed: None,
        })
    }

    /// Installs cooperative cancellation for the calling invocation.
    #[must_use]
    pub fn with_cancellation_token(mut self, cancellation_token: ApiCancellationToken) -> Self {
        self.cancellation_token = Some(cancellation_token);
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    pub const fn transport_options(&self) -> ApiTransportOptions {
        self.transport_options
    }

    #[must_use]
    pub fn cancellation_token(&self) -> Option<ApiCancellationToken> {
        self.cancellation_token.clone()
    }

    pub const fn last_request_correlation(&self) -> Option<RequestCorrelation> {
        self.last_request_correlation
    }

    pub fn into_credentials(self) -> Option<Credentials> {
        self.credentials
    }

    async fn get_access_token(&mut self) -> PublicResult<String> {
        let credentials = self
            .credentials
            .as_ref()
            .ok_or_else(|| PublicError::validation("not logged in"))?;

        if credentials.access_expires_within(ACCESS_TOKEN_REFRESH_WINDOW_SECONDS) {
            let expected = credentials.clone();
            let refresh_request_id = self
                .transport_options
                .request_id()
                .unwrap_or_else(Uuid::now_v7);
            let refresh_client = build_control_plane_http_client(
                self.transport_options.with_request_id(refresh_request_id),
            )?;
            self.last_request_correlation = Some(RequestCorrelation {
                client_request_id: refresh_request_id,
                response_request_id: None,
                attempt_count: 1,
            });
            let refresh_guard = self
                .cancellation_token
                .as_ref()
                .map(ApiCancellationToken::enter_credential_refresh);
            let refresh_result = refresh_credentials_if_needed_with_timeout(
                &refresh_client,
                &self.base_url,
                &expected,
                ACCESS_TOKEN_REFRESH_WINDOW_SECONDS,
                self.transport_options.request_timeout(),
            )
            .await;
            if let Some(cancellation) = self.cancellation_token.as_ref() {
                cancellation.finish_credential_refresh(&refresh_result);
            }
            drop(refresh_guard);
            let refreshed = refresh_result?;
            self.credentials = Some(refreshed);
        }

        self.ensure_not_cancelled()?;
        self.credentials
            .as_ref()
            .map(|credentials| credentials.access_token.clone())
            .ok_or_else(|| PublicError::validation("not logged in"))
    }

    fn ensure_not_cancelled(&self) -> PublicResult<()> {
        if self
            .cancellation_token
            .as_ref()
            .is_some_and(ApiCancellationToken::is_cancelled)
        {
            return Err(PublicError::cancelled(
                "API request cancelled before the requested resource mutation was sent",
            ));
        }
        Ok(())
    }

    pub(crate) async fn get<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.get(url),
            path,
            RequestSemantics::Read,
            usize::MAX,
        )
        .await
    }

    pub(crate) async fn get_bounded<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.get(url),
            path,
            RequestSemantics::Read,
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn get_bounded_body(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let url = format!("{}{}", self.base_url, path);
        self.execute_bounded_request(
            self.client.get(url),
            RequestSemantics::Read,
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.post(url).json(body),
            path,
            RequestSemantics::UnsafeMutation {
                operation: "API mutation",
            },
            usize::MAX,
        )
        .await
    }

    pub(crate) async fn post_replay_safe<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.post(url).json(body),
            path,
            RequestSemantics::ReplaySafeMutation,
            usize::MAX,
        )
        .await
    }

    pub(crate) async fn post_no_replay<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.post(url).json(body),
            path,
            RequestSemantics::ControlPlaneNoReplay,
            usize::MAX,
        )
        .await
    }

    pub(crate) async fn post_state_transition<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        operation: &'static str,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.post(url).json(body),
            path,
            RequestSemantics::StateTransition { operation },
            usize::MAX,
        )
        .await
    }

    pub(crate) async fn patch<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.patch(url).json(body),
            path,
            RequestSemantics::UnsafeMutation {
                operation: "API update",
            },
            usize::MAX,
        )
        .await
    }

    pub(crate) async fn post_bounded<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.post(url).json(body),
            path,
            RequestSemantics::UnsafeMutation {
                operation: "API mutation",
            },
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn post_bounded_no_replay<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(
            self.client.post(url).json(body),
            path,
            RequestSemantics::ControlPlaneNoReplay,
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn post_no_content_bounded_no_replay<B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content(
            self.client.post(url).json(body),
            path,
            RequestSemantics::DurableMutationNoReplay {
                operation: "API mutation",
            },
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn delete_no_content_bounded(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content(
            self.client.delete(url),
            path,
            RequestSemantics::UnsafeMutation {
                operation: "API deletion",
            },
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn delete_no_content_with_body<B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content(
            self.client.delete(url).json(body),
            path,
            RequestSemantics::UnsafeMutation {
                operation: "API deletion",
            },
            usize::MAX,
        )
        .await
    }

    async fn send<T: for<'de> Deserialize<'de>>(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        semantics: RequestSemantics,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let response = self
            .execute_bounded_request(request, semantics, max_decompressed_bytes)
            .await?;
        decode_response(response, path, semantics)
    }

    pub(crate) async fn send_json_bytes_bounded_body(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Vec<u8>,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let url = format!("{}{}", self.base_url, path);
        let operation = match method {
            reqwest::Method::PATCH => "note update",
            reqwest::Method::DELETE => "note deletion",
            _ => "note mutation",
        };
        let request = self
            .client
            .request(method, url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        self.execute_bounded_request(
            request,
            RequestSemantics::UnsafeMutation { operation },
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn send_no_replay_json_bytes_bounded_body(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Vec<u8>,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let url = format!("{}{}", self.base_url, path);
        let request = self
            .client
            .request(method, url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        self.execute_bounded_request(
            request,
            RequestSemantics::DurableMutationNoReplay {
                operation: "note creation",
            },
            max_decompressed_bytes,
        )
        .await
    }

    async fn send_no_content(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        semantics: RequestSemantics,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let response = self
            .execute_bounded_request(request, semantics, max_decompressed_bytes)
            .await?;
        decode_empty_response(response, path, semantics)
    }

    async fn execute_bounded_request(
        &mut self,
        request: reqwest::RequestBuilder,
        semantics: RequestSemantics,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let token = self.get_access_token().await?;
        let client_request_id = self
            .transport_options
            .request_id()
            .unwrap_or_else(Uuid::now_v7);
        // CLI invocations intentionally reuse one correlation ID. Retry jitter
        // still needs a fresh seed per logical request so concurrent jobs do
        // not enter lockstep while every wire attempt retains that header.
        let retry_seed = Uuid::now_v7();
        #[cfg(test)]
        {
            self.last_retry_seed = Some(retry_seed);
        }
        let request_id_header = request_id_header_value(client_request_id)?;
        let request = request
            .bearer_auth(token)
            .header(REQUEST_ID_HEADER, request_id_header)
            .build()
            .map_err(|_| PublicError::unexpected("failed to build API request"))?;
        let _mutation_request_guard = self
            .cancellation_token
            .as_ref()
            .and_then(|token| token.enter_mutation_request(semantics));
        self.ensure_not_cancelled()?;
        self.last_request_correlation = Some(RequestCorrelation {
            client_request_id,
            response_request_id: None,
            attempt_count: 0,
        });

        let max_retries = if semantics.permits_replay() {
            self.transport_options.retry_policy().max_retries()
        } else {
            0
        };
        let mut retry_index = 0_u8;
        let mut prior_delivery_uncertain = false;

        loop {
            let attempt_count = retry_index + 1;
            self.last_request_correlation = Some(RequestCorrelation {
                client_request_id,
                response_request_id: None,
                attempt_count,
            });
            let attempt_request = request
                .try_clone()
                .ok_or_else(|| PublicError::unexpected("API request body cannot be replayed"))?;
            let response = match self.client.execute(attempt_request).await {
                Ok(response) => response,
                Err(error) => {
                    let transient = is_transient_request_error(&error);
                    let delivery_uncertain = !error.is_connect();
                    if transient
                        && retry_index < max_retries
                        && let Some(delay) = retry_delay(retry_seed, retry_index, None)
                    {
                        prior_delivery_uncertain |= delivery_uncertain;
                        tokio::time::sleep(delay).await;
                        retry_index += 1;
                        continue;
                    }

                    let error = map_bounded_transport_error(&error);
                    if semantics.requires_ambiguity(prior_delivery_uncertain || delivery_uncertain)
                    {
                        return Err(ambiguous_outcome(semantics));
                    }
                    return Err(error);
                }
            };

            let response_request_id = parse_response_request_id(response.headers());
            self.last_request_correlation = Some(RequestCorrelation {
                client_request_id,
                response_request_id,
                attempt_count,
            });
            let status = response.status();
            let retry_after = parse_retry_after(response.headers());
            let status_delivery_uncertain = status_may_hide_committed_mutation(status);
            if is_retryable_status(status)
                && retry_index < max_retries
                && let Some(delay) = retry_delay(retry_seed, retry_index, retry_after)
            {
                prior_delivery_uncertain |= status_delivery_uncertain;
                drop(response);
                tokio::time::sleep(delay).await;
                retry_index += 1;
                continue;
            }
            if !status.is_success()
                && semantics
                    .requires_ambiguity(prior_delivery_uncertain || status_delivery_uncertain)
            {
                return Err(ambiguous_outcome(semantics));
            }

            let response = read_bounded_response_body_preserving_status(
                response,
                max_decompressed_bytes,
                semantics,
            )
            .await;
            let body_is_transient = response
                .body_error()
                .is_some_and(is_transient_response_error);
            let body_delivery_uncertain =
                body_is_transient && (status.is_success() || status_delivery_uncertain);
            if status.is_success()
                && response.body_error().is_some()
                && semantics.success_confirms_durable_mutation()
            {
                return Ok(response.with_request_guard(_mutation_request_guard));
            }
            if status.is_success()
                && body_is_transient
                && retry_index < max_retries
                && let Some(delay) = retry_delay(retry_seed, retry_index, response.retry_after())
            {
                prior_delivery_uncertain |= body_delivery_uncertain;
                tokio::time::sleep(delay).await;
                retry_index += 1;
                continue;
            }
            if body_is_transient
                && semantics.requires_ambiguity(prior_delivery_uncertain || body_delivery_uncertain)
            {
                return Err(ambiguous_outcome(semantics));
            }
            return Ok(response.with_request_guard(_mutation_request_guard));
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    pub error: String,
}

fn decode_response<T: for<'de> Deserialize<'de>>(
    response: BoundedHttpResponse,
    path: &str,
    semantics: RequestSemantics,
) -> PublicResult<T> {
    if response.status().is_success() {
        let (body, _request_guard) = response.into_body_with_guard();
        body.and_then(|body| decode_bounded_json(&body))
            .map_err(|error| successful_mutation_processing_error(semantics, error))
    } else {
        Err(map_bounded_error_response(response, path))
    }
}

fn decode_empty_response(
    response: BoundedHttpResponse,
    path: &str,
    semantics: RequestSemantics,
) -> PublicResult<()> {
    if response.status().is_success() {
        let (body, _request_guard) = response.into_body_with_guard();
        body.map(|_| ())
            .map_err(|error| successful_mutation_processing_error(semantics, error))
    } else {
        Err(map_bounded_error_response(response, path))
    }
}

fn map_bounded_error_response(response: BoundedHttpResponse, path: &str) -> PublicError {
    let status = response.status().as_u16();
    let retry_after = response.retry_after();
    let (body, _request_guard) = response.into_body_with_guard();
    let body = body.unwrap_or_default();
    let error_text = String::from_utf8_lossy(&body);
    map_api_error_with_retry_after(status, &error_text, path, retry_after)
}

async fn read_bounded_response_body_preserving_status(
    mut response: reqwest::Response,
    max_decompressed_bytes: usize,
    semantics: RequestSemantics,
) -> BoundedHttpResponse {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    let expected_len = response.content_length();
    let mut body = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => {
                return BoundedHttpResponse::complete(status, retry_after, body, semantics);
            }
            Err(error) => {
                let received_len = body.len();
                return BoundedHttpResponse::failed(
                    status,
                    retry_after,
                    received_len,
                    map_bounded_body_error(&error, expected_len, received_len),
                    semantics,
                );
            }
        };
        let Some(next_len) = body.len().checked_add(chunk.len()) else {
            return BoundedHttpResponse::failed(
                status,
                retry_after,
                usize::MAX,
                PublicError::response(
                    ResponseFailureKind::BodyTooLarge,
                    "API response body size overflowed the client safety bound",
                ),
                semantics,
            );
        };
        if next_len > max_decompressed_bytes {
            return BoundedHttpResponse::failed(
                status,
                retry_after,
                next_len,
                PublicError::response(
                    ResponseFailureKind::BodyTooLarge,
                    format!("API response body exceeds the {max_decompressed_bytes}-byte limit"),
                ),
                semantics,
            );
        }
        body.extend_from_slice(&chunk);
    }
}

fn normalize_base_url(value: String) -> String {
    value.trim_end_matches('/').to_string()
}

fn validate_timeout(name: &str, timeout: Duration) -> PublicResult<()> {
    if timeout.is_zero() {
        return Err(PublicError::validation(format!(
            "API {name} timeout must be greater than zero"
        )));
    }
    if timeout > MAX_API_TIMEOUT {
        return Err(PublicError::validation(format!(
            "API {name} timeout cannot exceed 24 hours"
        )));
    }
    Ok(())
}

/// Builds a raw HTTP client with the same headers and timeouts used for control-plane requests.
///
/// The returned client sends the UUID from [`ApiTransportOptions::with_request_id`] as its default
/// `x-request-id` header, or generates one when none is configured. This is intended for
/// control-plane flows implemented outside [`PublicApiClient`], such as login and logout; it must
/// not be used for presigned storage URLs. [`ApiRetryPolicy`] is enforced by [`PublicApiClient`]
/// after classifying replay safety, so this raw client never retries automatically.
pub fn build_control_plane_http_client(
    transport_options: ApiTransportOptions,
) -> PublicResult<reqwest::Client> {
    reqwest::Client::builder()
        .no_hickory_dns()
        .user_agent(CONTROL_PLANE_USER_AGENT)
        .default_headers(control_plane_default_headers(&transport_options)?)
        .connect_timeout(transport_options.connect_timeout())
        .read_timeout(transport_options.read_timeout())
        .timeout(transport_options.request_timeout())
        .build()
        .map_err(|err| PublicError::unexpected(format!("failed to configure API client: {err}")))
}

/// Builds the dedicated client used by a long-lived control-plane event stream.
///
/// Unlike ordinary API requests, this client deliberately has no overall request timeout. The
/// read timeout still bounds how long a silent connection can remain stuck, and redirects are
/// disabled so the query-string stream credential cannot be forwarded to another origin.
pub(crate) fn build_control_plane_stream_http_client(
    transport_options: ApiTransportOptions,
    idle_read_timeout: Duration,
) -> PublicResult<reqwest::Client> {
    validate_timeout("event stream idle read", idle_read_timeout)?;

    reqwest::Client::builder()
        .no_hickory_dns()
        .user_agent(CONTROL_PLANE_USER_AGENT)
        .default_headers(control_plane_default_headers(&transport_options)?)
        .connect_timeout(transport_options.connect_timeout())
        .read_timeout(idle_read_timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| PublicError::unexpected("failed to configure API event stream client"))
}

fn control_plane_default_headers(
    transport_options: &ApiTransportOptions,
) -> PublicResult<reqwest::header::HeaderMap> {
    let mut default_headers = reqwest::header::HeaderMap::new();
    let request_id = transport_options.request_id().unwrap_or_else(Uuid::now_v7);
    default_headers.insert(REQUEST_ID_HEADER, request_id_header_value(request_id)?);
    Ok(default_headers)
}

fn request_id_header_value(request_id: Uuid) -> PublicResult<reqwest::header::HeaderValue> {
    reqwest::header::HeaderValue::from_str(&request_id.to_string())
        .map_err(|_| PublicError::unexpected("failed to encode the client request ID header"))
}

fn parse_response_request_id(headers: &reqwest::header::HeaderMap) -> Option<Uuid> {
    headers.get(REQUEST_ID_HEADER)?.to_str().ok()?.parse().ok()
}

fn is_transient_request_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout() || error.is_body()
}

fn map_bounded_transport_error(error: &reqwest::Error) -> PublicError {
    let kind = if error.is_timeout() {
        TransportFailureKind::Timeout
    } else if error.is_connect() {
        TransportFailureKind::Connect
    } else if error.is_body() {
        TransportFailureKind::Body
    } else {
        TransportFailureKind::Other
    };
    PublicError::transport(kind)
}

fn credential_refresh_failure_may_have_rotated(error: &PublicError) -> bool {
    match error {
        PublicError::Transport(failure) => failure.kind() != TransportFailureKind::Connect,
        PublicError::Response { .. }
        | PublicError::Unexpected(_)
        | PublicError::RequestTimeout(_)
        | PublicError::CompensationFailed { .. }
        | PublicError::OutcomeAmbiguous { .. }
        | PublicError::CommittedButLocalProcessingFailed { .. } => true,
        PublicError::Validation(_)
        | PublicError::NotFound(_)
        | PublicError::Conflict(_)
        | PublicError::Entitlement(_)
        | PublicError::PayloadTooLarge(_)
        | PublicError::RateLimited(_)
        | PublicError::Crypto(_)
        | PublicError::Cancelled(_)
        | PublicError::Http(_)
        | PublicError::MfaRequiredUseBeginLogin
        | PublicError::MfaInputRequired => false,
        _ => true,
    }
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500..=599)
}

fn status_may_hide_committed_mutation(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 500..=599)
}

fn is_transient_response_error(error: &PublicError) -> bool {
    matches!(
        error.transport_failure_kind(),
        Some(
            TransportFailureKind::Timeout
                | TransportFailureKind::Connect
                | TransportFailureKind::Body
        )
    ) || matches!(
        error.response_failure_kind(),
        Some(
            ResponseFailureKind::BodyRead
                | ResponseFailureKind::BodyTruncated
                | ResponseFailureKind::Transport
        )
    )
}

fn retry_delay(
    request_id: Uuid,
    retry_index: u8,
    retry_after: Option<Duration>,
) -> Option<Duration> {
    let exponent = u32::from(retry_index.min(4));
    let ceiling_millis = RETRY_BASE_DELAY_MILLIS
        .saturating_mul(1_u64 << exponent)
        .min(RETRY_BACKOFF_CAP_MILLIS);
    let jitter_millis = retry_jitter(request_id, retry_index) % (ceiling_millis + 1);
    let delay = Duration::from_millis(jitter_millis).max(retry_after.unwrap_or_default());
    (delay <= MAX_API_RETRY_DELAY).then_some(delay)
}

fn retry_jitter(request_id: Uuid, retry_index: u8) -> u64 {
    let request_bits = request_id.as_u128();
    let mut value = (request_bits as u64)
        ^ ((request_bits >> 64) as u64)
        ^ u64::from(retry_index).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn ambiguous_outcome(semantics: RequestSemantics) -> PublicError {
    PublicError::outcome_ambiguous(
        semantics.operation().unwrap_or("API mutation"),
        "the server may have applied the request; inspect authoritative state before retrying",
    )
}

pub(crate) fn successful_mutation_processing_error(
    semantics: RequestSemantics,
    error: PublicError,
) -> PublicError {
    if !semantics.success_confirms_durable_mutation() {
        return error;
    }

    PublicError::committed_but_local_processing_failed(
        semantics.operation().unwrap_or("API mutation"),
        "the requested resource",
        format!(
            "response_processing={}; fetch the resource to inspect authoritative state and do not repeat the mutation",
            error.code()
        ),
    )
}

fn map_bounded_body_error(
    error: &reqwest::Error,
    expected_len: Option<u64>,
    received_len: usize,
) -> PublicError {
    if error.is_timeout() {
        return PublicError::transport(TransportFailureKind::Timeout);
    }
    if error.is_connect() {
        return PublicError::transport(TransportFailureKind::Connect);
    }
    if expected_len.is_some_and(|expected| {
        u64::try_from(received_len).is_ok_and(|received| received < expected)
    }) {
        return PublicError::response(
            ResponseFailureKind::BodyTruncated,
            "API response body ended before its declared length",
        );
    }
    PublicError::transport(TransportFailureKind::Body)
}

pub(crate) fn decode_bounded_json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> PublicResult<T> {
    serde_json::from_slice(body).map_err(map_bounded_json_error)
}

fn map_bounded_json_error(error: serde_json::Error) -> PublicError {
    let (kind, message) = match error.classify() {
        serde_json::error::Category::Data => (
            ResponseFailureKind::JsonSchema,
            "API response JSON does not match the expected schema",
        ),
        serde_json::error::Category::Eof | serde_json::error::Category::Syntax => (
            ResponseFailureKind::JsonMalformed,
            "API response contains malformed JSON",
        ),
        serde_json::error::Category::Io => (
            ResponseFailureKind::BodyRead,
            "API response body could not be read",
        ),
    };
    PublicError::response(kind, message)
}

pub(crate) fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    parse_retry_after_at(headers, SystemTime::now())
}

fn parse_retry_after_at(headers: &reqwest::header::HeaderMap, now: SystemTime) -> Option<Duration> {
    let value = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if value.is_empty() {
        return None;
    }
    if value.bytes().all(|byte| byte.is_ascii_digit()) {
        let seconds = value.parse::<u64>().unwrap_or(MAX_RETRY_AFTER_SECONDS);
        return Some(Duration::from_secs(seconds.min(MAX_RETRY_AFTER_SECONDS)));
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    Some(
        retry_at
            .duration_since(now)
            .unwrap_or_default()
            .min(Duration::from_secs(MAX_RETRY_AFTER_SECONDS)),
    )
}

#[cfg(test)]
pub(crate) fn map_api_error(status: u16, body: &str, path: &str) -> PublicError {
    map_api_error_with_retry_after(status, body, path, None)
}

pub(crate) fn map_api_error_with_retry_after(
    status: u16,
    body: &str,
    _path: &str,
    retry_after: Option<Duration>,
) -> PublicError {
    if let Ok(api_error) = serde_json::from_str::<ApiErrorResponse>(body) {
        return PublicError::http(status, Some(api_error.error), retry_after);
    }

    PublicError::http(status, None, retry_after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde::ser::SerializeStruct;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::UNIX_EPOCH;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Debug, Deserialize)]
    struct TestResponse {
        ok: bool,
    }

    struct CountingPayload<'a> {
        serializations: &'a AtomicUsize,
        secret: &'a str,
    }

    impl Serialize for CountingPayload<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.serializations.fetch_add(1, Ordering::SeqCst);
            let mut state = serializer.serialize_struct("CountingPayload", 1)?;
            state.serialize_field("secret", self.secret)?;
            state.end()
        }
    }

    #[derive(Debug)]
    struct ObservedRequest {
        headers: String,
        body: Vec<u8>,
    }

    #[test]
    fn retry_after_accepts_delta_seconds_and_http_dates_safely() {
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let mut headers = reqwest::header::HeaderMap::new();

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("17"),
        );
        assert_eq!(
            parse_retry_after_at(&headers, now),
            Some(Duration::from_secs(17))
        );

        let future = httpdate::fmt_http_date(now + Duration::from_secs(29));
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_str(&future).expect("future HTTP date"),
        );
        assert_eq!(
            parse_retry_after_at(&headers, now),
            Some(Duration::from_secs(29))
        );

        let past = httpdate::fmt_http_date(now - Duration::from_secs(5));
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_str(&past).expect("past HTTP date"),
        );
        assert_eq!(parse_retry_after_at(&headers, now), Some(Duration::ZERO));

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("not-an-http-date"),
        );
        assert_eq!(parse_retry_after_at(&headers, now), None);
    }

    #[test]
    fn retry_after_http_dates_honor_parse_cap_and_retry_wait_refusal() {
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let future =
            httpdate::fmt_http_date(now + Duration::from_secs(MAX_RETRY_AFTER_SECONDS + 1));
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_str(&future).expect("future HTTP date"),
        );

        let delay = parse_retry_after_at(&headers, now).expect("HTTP-date delay");
        assert_eq!(delay, Duration::from_secs(MAX_RETRY_AFTER_SECONDS));
        assert_eq!(retry_delay(Uuid::nil(), 0, Some(delay)), None);
    }

    #[test]
    fn transport_options_preserve_defaults_and_reject_invalid_timeouts() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<ApiTransportOptions>();

        let defaults = ApiTransportOptions::default();
        assert_eq!(defaults.connect_timeout(), Duration::from_secs(10));
        assert_eq!(defaults.read_timeout(), Duration::from_secs(30));
        assert_eq!(defaults.request_timeout(), Duration::from_secs(60));
        assert_eq!(defaults.request_id(), None);
        assert_eq!(defaults.retry_policy(), ApiRetryPolicy::default());
        assert_eq!(defaults.retry_policy().max_retries(), 0);
        assert_eq!(defaults.retry_policy().max_attempts(), 1);

        let request_id = Uuid::now_v7();
        assert_eq!(
            defaults.with_request_id(request_id).request_id(),
            Some(request_id)
        );
        let retry_policy = ApiRetryPolicy::new(2).expect("valid retry policy");
        assert_eq!(
            defaults.with_retry_policy(retry_policy).retry_policy(),
            retry_policy
        );
        assert_eq!(retry_policy.max_attempts(), 3);
        let error = ApiRetryPolicy::new(MAX_API_RETRIES + 1).expect_err("oversized retry budget");
        assert_eq!(error.code(), "validation");
        assert!(error.to_string().contains("cannot exceed 10"));

        for (connect, read, request, expected) in [
            (
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_secs(1),
                "connect timeout must be greater than zero",
            ),
            (
                Duration::from_secs(1),
                Duration::ZERO,
                Duration::from_secs(1),
                "read timeout must be greater than zero",
            ),
            (
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::ZERO,
                "request timeout must be greater than zero",
            ),
            (
                Duration::from_secs(2),
                Duration::from_secs(1),
                Duration::from_secs(1),
                "connect timeout cannot exceed",
            ),
            (
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(1),
                "read timeout cannot exceed",
            ),
        ] {
            let error =
                ApiTransportOptions::new(connect, read, request).expect_err("invalid timeout");
            assert!(error.to_string().contains(expected));
        }
        assert!(
            ApiTransportOptions::new(
                Duration::from_secs(1),
                Duration::from_secs(1),
                MAX_API_TIMEOUT + Duration::from_secs(1),
            )
            .expect_err("oversized timeout")
            .to_string()
            .contains("cannot exceed 24 hours")
        );
    }

    #[test]
    fn retry_semantics_and_delay_bounds_are_explicit_and_deterministic() {
        assert!(RequestSemantics::Read.permits_replay());
        assert!(RequestSemantics::ReplaySafeMutation.permits_replay());
        assert!(RequestSemantics::StateTransition { operation: "test" }.permits_replay());
        assert!(!RequestSemantics::UnsafeMutation { operation: "test" }.permits_replay());
        assert!(!RequestSemantics::ControlPlaneNoReplay.permits_replay());
        assert!(!RequestSemantics::DurableMutationNoReplay { operation: "test" }.permits_replay());
        assert!(!RequestSemantics::Read.is_durable_mutation());
        assert!(!RequestSemantics::ControlPlaneNoReplay.is_durable_mutation());
        assert!(RequestSemantics::ReplaySafeMutation.is_durable_mutation());
        assert!(RequestSemantics::StateTransition { operation: "test" }.is_durable_mutation());
        assert!(RequestSemantics::UnsafeMutation { operation: "test" }.is_durable_mutation());
        assert!(
            RequestSemantics::DurableMutationNoReplay { operation: "test" }.is_durable_mutation()
        );
        for status in [408, 429, 500, 503, 599] {
            assert!(is_retryable_status(
                reqwest::StatusCode::from_u16(status).expect("valid status")
            ));
        }
        for status in [400, 409, 499, 600] {
            assert!(!is_retryable_status(
                reqwest::StatusCode::from_u16(status).expect("valid status")
            ));
        }
        for status in [408, 500, 599] {
            assert!(status_may_hide_committed_mutation(
                reqwest::StatusCode::from_u16(status).expect("valid status")
            ));
        }
        for status in [400, 409, 429] {
            assert!(!status_may_hide_committed_mutation(
                reqwest::StatusCode::from_u16(status).expect("valid status")
            ));
        }

        let request_id =
            Uuid::parse_str("019b2ef8-49d2-7000-8000-000000000001").expect("request ID");
        for retry_index in 0..=MAX_API_RETRIES {
            let first = retry_delay(request_id, retry_index, None).expect("bounded delay");
            let second = retry_delay(request_id, retry_index, None).expect("deterministic delay");
            let ceiling = Duration::from_millis(
                RETRY_BASE_DELAY_MILLIS
                    .saturating_mul(1_u64 << u32::from(retry_index.min(4)))
                    .min(RETRY_BACKOFF_CAP_MILLIS),
            );
            assert_eq!(first, second);
            assert!(first <= ceiling);
        }

        let server_minimum = Duration::from_secs(5);
        assert!(
            retry_delay(request_id, 0, Some(server_minimum)).expect("accepted Retry-After")
                >= server_minimum
        );
        assert_eq!(
            retry_delay(request_id, 0, Some(MAX_API_RETRY_DELAY)),
            Some(MAX_API_RETRY_DELAY)
        );
        assert_eq!(
            retry_delay(
                request_id,
                0,
                Some(MAX_API_RETRY_DELAY + Duration::from_secs(1))
            ),
            None
        );
    }

    #[test]
    fn invocation_cancellation_tracks_refresh_and_mutation_boundaries_independently() {
        let cancellation = ApiCancellationToken::new();
        assert!(!cancellation.credential_refresh_in_flight());
        assert!(!cancellation.credential_refresh_may_have_rotated());
        assert!(!cancellation.mutation_request_in_flight());

        let refresh = cancellation.enter_credential_refresh();
        assert!(cancellation.credential_refresh_in_flight());
        assert!(!cancellation.mutation_request_in_flight());
        {
            let _mutation = cancellation
                .enter_mutation_request(RequestSemantics::UnsafeMutation { operation: "test" })
                .expect("durable mutation guard");
            assert!(cancellation.credential_refresh_in_flight());
            assert!(cancellation.mutation_request_in_flight());
        }
        assert!(!cancellation.mutation_request_in_flight());
        cancellation
            .finish_credential_refresh(&Err(PublicError::transport(TransportFailureKind::Body)));
        assert!(cancellation.credential_refresh_may_have_rotated());
        drop(refresh);
        assert!(!cancellation.credential_refresh_in_flight());
        assert!(cancellation.credential_refresh_may_have_rotated());

        let definitive_refresh = cancellation.enter_credential_refresh();
        assert!(!cancellation.credential_refresh_may_have_rotated());
        cancellation
            .finish_credential_refresh(&Err(PublicError::transport(TransportFailureKind::Connect)));
        drop(definitive_refresh);
        assert!(!cancellation.credential_refresh_may_have_rotated());

        let successful_refresh = cancellation.enter_credential_refresh();
        cancellation.finish_credential_refresh(&Ok(test_credentials("https://api.example")));
        drop(successful_refresh);
        assert!(!cancellation.credential_refresh_may_have_rotated());
    }

    #[test]
    fn explicit_transport_options_are_retained_by_both_client_constructors() {
        let options = ApiTransportOptions::new(
            Duration::from_secs(2),
            Duration::from_secs(3),
            Duration::from_secs(4),
        )
        .expect("transport options");
        let anonymous =
            PublicApiClient::new_with_options("https://api.example", options).expect("client");
        assert_eq!(anonymous.transport_options(), options);
        assert!(anonymous.cancellation_token().is_none());

        let authenticated = PublicApiClient::with_credentials_and_options(
            "https://api.example",
            test_credentials("https://api.example"),
            options,
        )
        .expect("authenticated client");
        assert_eq!(authenticated.transport_options(), options);
        assert!(authenticated.cancellation_token().is_none());
    }

    #[tokio::test]
    async fn replay_safe_requests_serialize_once_and_replay_exact_bytes_and_request_id() {
        const SECRET: &str = "retry-payload-secret-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let mut observed = Vec::new();
            for (status, body) in [
                (
                    "503 Service Unavailable",
                    br#"{"error":"temporary"}"#.as_slice(),
                ),
                ("200 OK", br#"{"ok":true}"#.as_slice()),
            ] {
                let (mut stream, _) = listener.accept().await.expect("connection");
                observed.push(read_http_request(&mut stream).await);
                write_test_response(
                    &mut stream,
                    status,
                    "X-Request-ID: 00000000-0000-0000-0000-000000000000\r\n",
                    body,
                )
                .await;
            }
            observed
        });

        let serializations = AtomicUsize::new(0);
        let payload = CountingPayload {
            serializations: &serializations,
            secret: SECRET,
        };
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(1),
        )
        .expect("client");

        let response: TestResponse = client
            .post_replay_safe("/replay", &payload)
            .await
            .expect("retry succeeds");
        assert!(response.ok);
        assert_eq!(serializations.load(Ordering::SeqCst), 1);

        let observed = server.await.expect("server");
        assert_eq!(observed.len(), 2);
        assert_eq!(observed[0].body, observed[1].body);
        assert_eq!(
            observed[0].body,
            format!(r#"{{"secret":"{SECRET}"}}"#).as_bytes()
        );
        for request in &observed {
            assert_eq!(
                request_header(&request.headers, REQUEST_ID_HEADER).as_deref(),
                Some("00000000-0000-0000-0000-000000000000")
            );
        }
        let correlation = client.last_request_correlation().expect("correlation");
        assert_eq!(correlation.attempt_count(), 2);
        assert_eq!(correlation.client_request_id(), Uuid::nil());
        assert_eq!(correlation.response_request_id(), Some(Uuid::nil()));
    }

    #[tokio::test]
    async fn successful_state_transition_replay_resolves_prior_uncertainty() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            for (status, body) in [
                (
                    "503 Service Unavailable",
                    br#"{"error":"temporary"}"#.as_slice(),
                ),
                ("200 OK", br#"{"ok":true}"#.as_slice()),
            ] {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let _ = read_http_request(&mut stream).await;
                write_test_response(&mut stream, status, "", body).await;
            }
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(1),
        )
        .expect("client");

        let response: TestResponse = client
            .post_state_transition(
                "/transition",
                &serde_json::json!({"expectedUpdatedAt":"fixed"}),
                "test transition",
            )
            .await
            .expect("definitive successful replay");
        assert!(response.ok);
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            2
        );
        server.await.expect("server");
    }

    #[tokio::test]
    async fn uncertain_state_transition_followed_by_conflict_remains_ambiguous() {
        const SECRET: &str = "state-transition-secret-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            for (status, body) in [
                (
                    "503 Service Unavailable",
                    br#"{"error":"temporary"}"#.as_slice(),
                ),
                ("409 Conflict", br#"{"error":"conflict"}"#.as_slice()),
            ] {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let _ = read_http_request(&mut stream).await;
                write_test_response(&mut stream, status, "", body).await;
            }
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(1),
        )
        .expect("client");

        let error = client
            .post_state_transition::<TestResponse, _>(
                "/transition",
                &serde_json::json!({"ciphertext":SECRET}),
                "test transition",
            )
            .await
            .expect_err("prior uncertain delivery makes the conflict inconclusive");
        assert_eq!(error.code(), "outcome_ambiguous");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            2
        );
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(SECRET));
            assert!(!rendered.contains(&api_url));
        }
        server.await.expect("server");
    }

    #[tokio::test]
    async fn unsafe_mutation_is_not_replayed_and_reports_ambiguous_server_failure() {
        const SECRET: &str = "unsafe-mutation-secret-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_http_request(&mut stream).await;
            write_test_response(
                &mut stream,
                "503 Service Unavailable",
                "",
                br#"{"error":"temporary"}"#,
            )
            .await;
            request
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");

        let error = client
            .post::<TestResponse, _>("/unsafe", &serde_json::json!({"ciphertext":SECRET}))
            .await
            .expect_err("unsafe 5xx must be ambiguous");
        assert_eq!(error.code(), "outcome_ambiguous");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        let observed = server.await.expect("server");
        assert!(String::from_utf8_lossy(&observed.body).contains(SECRET));
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(SECRET));
            assert!(!rendered.contains(&api_url));
        }
    }

    #[tokio::test]
    async fn replay_safe_task_create_with_malformed_success_json_is_known_committed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_http_request(&mut stream).await;
            write_test_response(&mut stream, "200 OK", "", br#"{"id":"#).await;
            request
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");
        let request = crate::CreateTaskRequest {
            title_ciphertext: "title".to_string(),
            title_ciphertext_proof: "title-proof".to_string(),
            payload_ciphertext: "payload".to_string(),
            payload_ciphertext_proof: "payload-proof".to_string(),
            attachment_ids: Vec::new(),
            priority: None,
            due_at: None,
            start_at: None,
            section_id: None,
            idempotency_key: Some("stable-task-key".to_string()),
            idempotency_commitment: Some("stable-task-commitment".to_string()),
        };

        let error = client
            .create_task(Uuid::now_v7(), &request)
            .await
            .expect_err("2xx confirms the idempotent task mutation committed");
        assert_committed_processing_failure(&error);
        assert!(
            error
                .to_string()
                .contains("response_processing=response_json_malformed")
        );
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        let observed = server.await.expect("server");
        let observed_body = String::from_utf8(observed.body).expect("JSON request");
        assert!(observed_body.contains("stable-task-key"));
        assert!(observed_body.contains("stable-task-commitment"));
    }

    #[tokio::test]
    async fn unsafe_success_with_schema_or_oversized_body_is_known_committed() {
        for (body, max_bytes, expected_code) in [
            (
                br#"{"unexpected":true}"#.as_slice(),
                usize::MAX,
                "response_json_schema",
            ),
            (br#"{"ok":true}"#.as_slice(), 1, "response_body_too_large"),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_http_request(&mut stream).await;
                write_test_response(&mut stream, "200 OK", "", body).await;
                request
            });
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("client");

            let error = client
                .post_bounded::<TestResponse, _>(
                    "/unsafe-success",
                    &serde_json::json!({"ciphertext":"request-secret-canary"}),
                    max_bytes,
                )
                .await
                .expect_err("successful mutation response processing must be classified");
            assert_committed_processing_failure(&error);
            assert!(
                error
                    .to_string()
                    .contains(&format!("response_processing={expected_code}"))
            );
            for rendered in [error.to_string(), format!("{error:?}")] {
                assert!(!rendered.contains("request-secret-canary"));
                assert!(!rendered.contains(&api_url));
            }
            let _ = server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn unsafe_success_with_truncated_body_is_known_committed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_http_request(&mut stream).await;
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 64\r\nConnection: close\r\n\r\n{",
                )
                .await
                .expect("truncated success response");
            request
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("client");

        let error = client
            .post::<TestResponse, _>(
                "/unsafe-success",
                &serde_json::json!({"ciphertext":"request-secret-canary"}),
            )
            .await
            .expect_err("truncated 2xx response follows a committed mutation");
        assert_committed_processing_failure(&error);
        assert!(
            error
                .to_string()
                .contains("response_processing=response_body_truncated")
        );
        let _ = server.await.expect("server");
    }

    #[tokio::test]
    async fn unsafe_empty_response_decode_failure_is_known_committed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_http_request(&mut stream).await;
            write_test_response(&mut stream, "200 OK", "", b"{}").await;
            request
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("client");

        let error = client
            .delete_no_content_bounded("/unsafe-empty", 1)
            .await
            .expect_err("oversized 2xx empty response follows a committed deletion");
        assert_committed_processing_failure(&error);
        assert!(
            error
                .to_string()
                .contains("response_processing=response_body_too_large")
        );
        let _ = server.await.expect("server");
    }

    #[tokio::test]
    async fn unsafe_connect_failure_before_send_is_not_reported_as_committed() {
        const SECRET: &str = "connect-failure-secret-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        drop(listener);
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");

        let error = client
            .post::<TestResponse, _>(
                "/never-connected",
                &serde_json::json!({"ciphertext":SECRET}),
            )
            .await
            .expect_err("closed listener must fail before send");
        assert_eq!(
            error.transport_failure_kind(),
            Some(TransportFailureKind::Connect)
        );
        assert_ne!(error.code(), "outcome_ambiguous");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(SECRET));
            assert!(!rendered.contains(&api_url));
        }
    }

    #[tokio::test]
    async fn definitive_client_error_is_not_replayed_to_recover_its_body() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let _ = read_http_request(&mut stream).await;
            stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 64\r\nConnection: close\r\n\r\n{",
                )
                .await
                .expect("truncated response");
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");

        let error = client
            .post_state_transition::<TestResponse, _>(
                "/definitive-rejection",
                &serde_json::json!({"expectedUpdatedAt":"fixed"}),
                "test transition",
            )
            .await
            .expect_err("known client rejection");
        assert_eq!(error.http_status(), Some(400));
        assert_eq!(error.code(), "validation");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        server.await.expect("server");
    }

    #[tokio::test]
    async fn replay_safe_success_with_a_response_read_failure_is_committed_not_replayed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let observed = read_http_request(&mut stream).await;
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 64\r\nConnection: close\r\n\r\n{",
                )
                .await
                .expect("truncated response");
            drop(stream);
            observed
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(1),
        )
        .expect("client");

        let error = client
            .post_replay_safe::<TestResponse, _>("/body-read", &serde_json::json!({"stable":true}))
            .await
            .expect_err("successful mutation must not be repeated to recover its response");
        assert_committed_processing_failure(&error);
        let observed = server.await.expect("server");
        assert_eq!(observed.body, br#"{"stable":true}"#);
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
    }

    #[tokio::test]
    async fn retry_budget_is_bounded_and_excessive_retry_after_is_refused() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let _ = read_http_request(&mut stream).await;
                write_test_response(
                    &mut stream,
                    "503 Service Unavailable",
                    "",
                    br#"{"error":"temporary"}"#,
                )
                .await;
            }
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(2),
        )
        .expect("client");
        let error = client
            .post_replay_safe::<TestResponse, _>("/bounded", &serde_json::json!({"stable":true}))
            .await
            .expect_err("retry budget exhausted");
        assert_eq!(error.http_status(), Some(503));
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            3
        );
        server.await.expect("server");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let _ = read_http_request(&mut stream).await;
            write_test_response(
                &mut stream,
                "503 Service Unavailable",
                "Retry-After: 31\r\n",
                br#"{"error":"temporary"}"#,
            )
            .await;
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");
        let error = client
            .post_replay_safe::<TestResponse, _>(
                "/excessive-retry-after",
                &serde_json::json!({"stable":true}),
            )
            .await
            .expect_err("retry wait above the safety ceiling is refused");
        assert_eq!(error.http_status(), Some(503));
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        server.await.expect("server");
    }

    #[tokio::test]
    async fn higher_level_reconciliation_requests_remain_single_wire_attempts() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_http_request(&mut stream).await;
            write_test_response(
                &mut stream,
                "503 Service Unavailable",
                "",
                br#"{"error":"temporary"}"#,
            )
            .await;
            request
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");

        let response = client
            .send_no_replay_json_bytes_bounded_body(
                reqwest::Method::POST,
                "/note-create-owned-retry",
                br#"{"idempotencyKey":"fixed"}"#.to_vec(),
                1_024,
            )
            .await
            .expect("raw bounded status");
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        let _ = server.await.expect("server");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_http_request(&mut stream).await;
            write_test_response(
                &mut stream,
                "503 Service Unavailable",
                "",
                br#"{"error":"temporary"}"#,
            )
            .await;
            request
        });
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            retry_options(MAX_API_RETRIES),
        )
        .expect("client");
        let error = client
            .post_no_content_bounded_no_replay(
                "/attachment-complete-owned-retry",
                &serde_json::json!({"ciphertextBytes":42}),
                1_024,
            )
            .await
            .expect_err("server status remains visible to runtime reconciliation");
        assert_eq!(error.http_status(), Some(503));
        assert_eq!(
            client
                .last_request_correlation()
                .expect("correlation")
                .attempt_count(),
            1
        );
        let _ = server.await.expect("server");
    }

    #[tokio::test]
    async fn control_plane_requests_have_stable_agent_fresh_ids_and_safe_correlation() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let mut observed = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_request_headers(&mut stream).await;
                let user_agent = request_header(&request, "user-agent").expect("user agent");
                let request_id = request_header(&request, REQUEST_ID_HEADER).expect("request ID");
                let request_id = Uuid::parse_str(&request_id).expect("UUID request ID");
                let body = br#"{"ok":true}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Request-ID: {request_id}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("response headers");
                stream.write_all(body).await.expect("response body");
                observed.push((user_agent, request_id));
            }
            observed
        });

        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("client");
        let first: TestResponse = client.get("/first").await.expect("first response");
        assert!(first.ok);
        let first_correlation = client
            .last_request_correlation()
            .expect("first request correlation");
        assert_eq!(
            first_correlation.response_request_id(),
            Some(first_correlation.client_request_id())
        );
        assert_eq!(
            first_correlation.effective_request_id(),
            first_correlation.client_request_id()
        );

        let second: TestResponse = client.get("/second").await.expect("second response");
        assert!(second.ok);
        let second_correlation = client
            .last_request_correlation()
            .expect("second request correlation");
        assert_ne!(
            first_correlation.client_request_id(),
            second_correlation.client_request_id()
        );

        let observed = server.await.expect("server");
        assert_eq!(observed[0].0, CONTROL_PLANE_USER_AGENT);
        assert_eq!(observed[1].0, CONTROL_PLANE_USER_AGENT);
        assert_eq!(observed[0].1, first_correlation.client_request_id());
        assert_eq!(observed[1].1, second_correlation.client_request_id());
    }

    #[tokio::test]
    async fn configured_request_id_is_reused_across_control_plane_requests() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let mut request_ids = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_request_headers(&mut stream).await;
                let request_id = request_header(&request, REQUEST_ID_HEADER).expect("request ID");
                let request_id = Uuid::parse_str(&request_id).expect("UUID request ID");
                let body = br#"{"ok":true}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Request-ID: {request_id}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("response headers");
                stream.write_all(body).await.expect("response body");
                request_ids.push(request_id);
            }
            request_ids
        });

        let invocation_request_id = Uuid::now_v7();
        let options = ApiTransportOptions::default().with_request_id(invocation_request_id);
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            options,
        )
        .expect("client");

        let _: TestResponse = client.get("/first").await.expect("first response");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("first correlation")
                .effective_request_id(),
            invocation_request_id
        );
        let first_retry_seed = client.last_retry_seed.expect("first retry seed");
        let _: TestResponse = client.get("/second").await.expect("second response");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("second correlation")
                .effective_request_id(),
            invocation_request_id
        );
        let second_retry_seed = client.last_retry_seed.expect("second retry seed");
        assert_ne!(
            first_retry_seed, second_retry_seed,
            "logical requests sharing an invocation correlation ID need distinct jitter seeds"
        );

        assert_eq!(
            server.await.expect("server"),
            vec![invocation_request_id; 2]
        );
    }

    #[tokio::test]
    async fn raw_control_plane_client_applies_shared_user_agent_and_request_id() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let mut observed = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_request_headers(&mut stream).await;
                let user_agent = request_header(&request, "user-agent").expect("user agent");
                let request_id = request_header(&request, REQUEST_ID_HEADER).expect("request ID");
                stream
                    .write_all(
                        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .expect("response");
                observed.push((user_agent, request_id));
            }
            observed
        });

        let default_client = build_control_plane_http_client(ApiTransportOptions::default())
            .expect("raw control-plane client");
        let default_response = default_client
            .get(format!("{api_url}/login"))
            .send()
            .await
            .expect("default request");
        assert_eq!(default_response.status(), reqwest::StatusCode::NO_CONTENT);

        let request_id = Uuid::now_v7();
        let client = build_control_plane_http_client(
            ApiTransportOptions::default().with_request_id(request_id),
        )
        .expect("raw control-plane client");
        let response = client
            .get(format!("{api_url}/auth"))
            .send()
            .await
            .expect("request");
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);

        let observed = server.await.expect("server");
        assert_eq!(observed[0].0, CONTROL_PLANE_USER_AGENT);
        assert_eq!(observed[1].0, CONTROL_PLANE_USER_AGENT);
        assert!(Uuid::parse_str(&observed[0].1).is_ok());
        assert_eq!(observed[1].1, request_id.to_string());
    }

    fn test_credentials(api_url: &str) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + chrono::Duration::hours(1),
            refresh_expires_at: Utc::now() + chrono::Duration::hours(2),
            user_id: Uuid::now_v7(),
            email: "agent@example.com".to_string(),
            data_key_ciphertext: "unused".to_string(),
        }
    }

    fn assert_committed_processing_failure(error: &PublicError) {
        assert_eq!(error.code(), "committed_but_local_processing_failed");
        let rendered = error.to_string();
        assert!(rendered.contains("fetch the resource"));
        assert!(rendered.contains("do not repeat the mutation"));
    }

    fn retry_options(max_retries: u8) -> ApiTransportOptions {
        ApiTransportOptions::default()
            .with_request_id(Uuid::nil())
            .with_retry_policy(ApiRetryPolicy::new(max_retries).expect("valid retry policy"))
    }

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> ObservedRequest {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1_024];
        let header_end = loop {
            if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
            let read = stream.read(&mut buffer).await.expect("request");
            assert_ne!(read, 0, "request ended before its headers");
            request.extend_from_slice(&buffer[..read]);
        };
        let headers =
            String::from_utf8(request[..header_end].to_vec()).expect("UTF-8 request headers");
        let content_length = request_header(&headers, "content-length")
            .map(|value| value.parse::<usize>().expect("numeric content length"))
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = stream.read(&mut buffer).await.expect("request body");
            assert_ne!(read, 0, "request body ended before its declared length");
            request.extend_from_slice(&buffer[..read]);
        }
        let body = request[header_end..header_end + content_length].to_vec();
        ObservedRequest { headers, body }
    }

    async fn read_request_headers(stream: &mut tokio::net::TcpStream) -> String {
        read_http_request(stream).await.headers
    }

    async fn write_test_response(
        stream: &mut tokio::net::TcpStream,
        status: &str,
        extra_headers: &str,
        body: &[u8],
    ) {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("response headers");
        stream.write_all(body).await.expect("response body");
    }

    fn request_header(request: &str, name: &str) -> Option<String> {
        request.lines().find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            header_name
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    }
}
