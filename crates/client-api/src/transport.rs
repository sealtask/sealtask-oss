use std::time::Duration;

use sealtask_client_auth::{Credentials, refresh_credentials_if_needed_with_timeout};
use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind, TransportFailureKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const ACCESS_TOKEN_REFRESH_WINDOW_SECONDS: i64 = 60;
const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_API_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
pub const DEFAULT_API_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_API_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_API_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const CONTROL_PLANE_USER_AGENT: &str =
    concat!("sealtask-client-api/", env!("CARGO_PKG_VERSION"));
pub(crate) const MAX_RETRY_AFTER_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ApiTransportOptions {
    connect_timeout: Duration,
    read_timeout: Duration,
    request_timeout: Duration,
    request_id: Option<Uuid>,
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
}

impl Default for ApiTransportOptions {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_API_CONNECT_TIMEOUT,
            read_timeout: DEFAULT_API_READ_TIMEOUT,
            request_timeout: DEFAULT_API_REQUEST_TIMEOUT,
            request_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RequestCorrelation {
    client_request_id: Uuid,
    response_request_id: Option<Uuid>,
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
}

#[derive(Debug, Clone)]
pub struct PublicApiClient {
    client: reqwest::Client,
    base_url: String,
    credentials: Option<Credentials>,
    transport_options: ApiTransportOptions,
    last_request_correlation: Option<RequestCorrelation>,
}

pub(crate) struct BoundedHttpResponse {
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    received_len: usize,
    body: PublicResult<Vec<u8>>,
}

impl BoundedHttpResponse {
    fn complete(status: reqwest::StatusCode, retry_after: Option<Duration>, body: Vec<u8>) -> Self {
        Self {
            status,
            retry_after,
            received_len: body.len(),
            body: Ok(body),
        }
    }

    fn failed(
        status: reqwest::StatusCode,
        retry_after: Option<Duration>,
        received_len: usize,
        error: PublicError,
    ) -> Self {
        Self {
            status,
            retry_after,
            received_len,
            body: Err(error),
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

    pub(crate) fn into_body(self) -> PublicResult<Vec<u8>> {
        self.body
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
            last_request_correlation: None,
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
            last_request_correlation: None,
        })
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
            });
            let refreshed = refresh_credentials_if_needed_with_timeout(
                &refresh_client,
                &self.base_url,
                &expected,
                ACCESS_TOKEN_REFRESH_WINDOW_SECONDS,
                self.transport_options.request_timeout(),
            )
            .await?;
            self.credentials = Some(refreshed);
        }

        self.credentials
            .as_ref()
            .map(|credentials| credentials.access_token.clone())
            .ok_or_else(|| PublicError::validation("not logged in"))
    }

    pub(crate) async fn get<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.client.get(url), path).await
    }

    pub(crate) async fn get_bounded<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send_bounded(self.client.get(url), path, max_decompressed_bytes)
            .await
    }

    pub(crate) async fn get_bounded_body(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let url = format!("{}{}", self.base_url, path);
        self.send_bounded_body(self.client.get(url), path, max_decompressed_bytes)
            .await
    }

    pub(crate) async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.client.post(url).json(body), path).await
    }

    pub(crate) async fn patch<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.client.patch(url).json(body), path).await
    }

    pub(crate) async fn post_bounded<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send_bounded(
            self.client.post(url).json(body),
            path,
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn post_no_content_bounded<B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content_bounded(
            self.client.post(url).json(body),
            path,
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
        self.send_no_content_bounded(self.client.delete(url), path, max_decompressed_bytes)
            .await
    }

    pub(crate) async fn delete_no_content_with_body<B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content(self.client.delete(url).json(body), path)
            .await
    }

    async fn send<T: for<'de> Deserialize<'de>>(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> PublicResult<T> {
        let token = self.get_access_token().await?;
        let response = self
            .send_authorized_request(request, &token, |error| map_reqwest_error(error, path))
            .await?;

        handle_response(response, path).await
    }

    async fn send_bounded<T: for<'de> Deserialize<'de>>(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let token = self.get_access_token().await?;
        let response = self
            .send_authorized_request(request, &token, map_bounded_transport_error)
            .await?;

        handle_bounded_response(response, path, max_decompressed_bytes).await
    }

    pub(crate) async fn send_json_bytes_bounded_body(
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
        self.send_bounded_body_preserving_status(request, path, max_decompressed_bytes)
            .await
    }

    async fn send_bounded_body(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        self.send_bounded_body_preserving_status(request, path, max_decompressed_bytes)
            .await
    }

    async fn send_bounded_body_preserving_status(
        &mut self,
        request: reqwest::RequestBuilder,
        _path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let token = self.get_access_token().await?;
        let response = self
            .send_authorized_request(request, &token, map_bounded_transport_error)
            .await?;

        Ok(read_bounded_response_body_preserving_status(response, max_decompressed_bytes).await)
    }

    async fn send_no_content(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> PublicResult<()> {
        let token = self.get_access_token().await?;
        let response = self
            .send_authorized_request(request, &token, |error| map_reqwest_error(error, path))
            .await?;

        handle_empty_response(response, path).await
    }

    async fn send_no_content_bounded(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let token = self.get_access_token().await?;
        let response = self
            .send_authorized_request(request, &token, map_bounded_transport_error)
            .await?;

        handle_bounded_empty_response(response, path, max_decompressed_bytes).await
    }

    async fn send_authorized_request(
        &mut self,
        request: reqwest::RequestBuilder,
        access_token: &str,
        map_error: impl FnOnce(reqwest::Error) -> PublicError,
    ) -> PublicResult<reqwest::Response> {
        let client_request_id = self
            .transport_options
            .request_id()
            .unwrap_or_else(Uuid::now_v7);
        let request_id_header = request_id_header_value(client_request_id)?;
        self.last_request_correlation = Some(RequestCorrelation {
            client_request_id,
            response_request_id: None,
        });

        let response = request
            .bearer_auth(access_token)
            .header(REQUEST_ID_HEADER, request_id_header)
            .send()
            .await
            .map_err(map_error)?;
        let response_request_id = parse_response_request_id(response.headers());
        self.last_request_correlation = Some(RequestCorrelation {
            client_request_id,
            response_request_id,
        });
        Ok(response)
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    pub error: String,
}

async fn handle_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    path: &str,
) -> PublicResult<T> {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    if status.is_success() {
        response.json().await.map_err(|error| {
            if error.is_timeout() {
                PublicError::transport(TransportFailureKind::Timeout)
            } else if error.is_connect() {
                PublicError::transport(TransportFailureKind::Connect)
            } else if error.is_body() {
                PublicError::transport(TransportFailureKind::Body)
            } else {
                PublicError::response(
                    ResponseFailureKind::JsonSchema,
                    "API response JSON does not match the expected schema",
                )
            }
        })
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        Err(map_api_error_with_retry_after(
            status.as_u16(),
            &error_text,
            path,
            retry_after,
        ))
    }
}

async fn handle_bounded_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    path: &str,
    max_decompressed_bytes: usize,
) -> PublicResult<T> {
    let response =
        read_bounded_response_body_preserving_status(response, max_decompressed_bytes).await;
    if response.status().is_success() {
        decode_bounded_json(&response.into_body()?)
    } else {
        Err(map_bounded_error_response(response, path))
    }
}

async fn handle_bounded_empty_response(
    response: reqwest::Response,
    path: &str,
    max_decompressed_bytes: usize,
) -> PublicResult<()> {
    let response =
        read_bounded_response_body_preserving_status(response, max_decompressed_bytes).await;
    if response.status().is_success() {
        response.into_body().map(|_| ())
    } else {
        Err(map_bounded_error_response(response, path))
    }
}

fn map_bounded_error_response(response: BoundedHttpResponse, path: &str) -> PublicError {
    let status = response.status().as_u16();
    let retry_after = response.retry_after();
    let body = response.into_body().unwrap_or_default();
    let error_text = String::from_utf8_lossy(&body);
    map_api_error_with_retry_after(status, &error_text, path, retry_after)
}

async fn read_bounded_response_body_preserving_status(
    mut response: reqwest::Response,
    max_decompressed_bytes: usize,
) -> BoundedHttpResponse {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    let expected_len = response.content_length();
    let mut body = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => return BoundedHttpResponse::complete(status, retry_after, body),
            Err(error) => {
                let received_len = body.len();
                return BoundedHttpResponse::failed(
                    status,
                    retry_after,
                    received_len,
                    map_bounded_body_error(&error, expected_len, received_len),
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
            );
        }
        body.extend_from_slice(&chunk);
    }
}

async fn handle_empty_response(response: reqwest::Response, path: &str) -> PublicResult<()> {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    if status.is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        Err(map_api_error_with_retry_after(
            status.as_u16(),
            &error_text,
            path,
            retry_after,
        ))
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

/// Builds a raw HTTP client with the same policy used for control-plane API requests.
///
/// The returned client sends the UUID from [`ApiTransportOptions::with_request_id`] as its default
/// `x-request-id` header, or generates one when none is configured. This is intended for
/// control-plane flows implemented outside [`PublicApiClient`], such as login and logout; it must
/// not be used for presigned storage URLs.
pub fn build_control_plane_http_client(
    transport_options: ApiTransportOptions,
) -> PublicResult<reqwest::Client> {
    let mut default_headers = reqwest::header::HeaderMap::new();
    let request_id = transport_options.request_id().unwrap_or_else(Uuid::now_v7);
    default_headers.insert(REQUEST_ID_HEADER, request_id_header_value(request_id)?);

    reqwest::Client::builder()
        .no_hickory_dns()
        .user_agent(CONTROL_PLANE_USER_AGENT)
        .default_headers(default_headers)
        .connect_timeout(transport_options.connect_timeout())
        .read_timeout(transport_options.read_timeout())
        .timeout(transport_options.request_timeout())
        .build()
        .map_err(|err| PublicError::unexpected(format!("failed to configure API client: {err}")))
}

fn request_id_header_value(request_id: Uuid) -> PublicResult<reqwest::header::HeaderValue> {
    reqwest::header::HeaderValue::from_str(&request_id.to_string())
        .map_err(|_| PublicError::unexpected("failed to encode the client request ID header"))
}

fn parse_response_request_id(headers: &reqwest::header::HeaderMap) -> Option<Uuid> {
    headers.get(REQUEST_ID_HEADER)?.to_str().ok()?.parse().ok()
}

fn map_reqwest_error(err: reqwest::Error, _path: &str) -> PublicError {
    if err.is_timeout() {
        PublicError::transport(TransportFailureKind::Timeout)
    } else if err.is_connect() {
        PublicError::transport(TransportFailureKind::Connect)
    } else {
        PublicError::transport(TransportFailureKind::Other)
    }
}

fn map_bounded_transport_error(error: reqwest::Error) -> PublicError {
    let kind = if error.is_timeout() {
        TransportFailureKind::Timeout
    } else if error.is_connect() {
        TransportFailureKind::Connect
    } else {
        TransportFailureKind::Other
    };
    PublicError::transport(kind)
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
    let value = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let seconds = value.parse::<u64>().unwrap_or(MAX_RETRY_AFTER_SECONDS);
    Some(Duration::from_secs(seconds.min(MAX_RETRY_AFTER_SECONDS)))
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Debug, Deserialize)]
    struct TestResponse {
        ok: bool,
    }

    #[test]
    fn transport_options_preserve_defaults_and_reject_invalid_timeouts() {
        let defaults = ApiTransportOptions::default();
        assert_eq!(defaults.connect_timeout(), Duration::from_secs(10));
        assert_eq!(defaults.read_timeout(), Duration::from_secs(30));
        assert_eq!(defaults.request_timeout(), Duration::from_secs(60));
        assert_eq!(defaults.request_id(), None);

        let request_id = Uuid::now_v7();
        assert_eq!(
            defaults.with_request_id(request_id).request_id(),
            Some(request_id)
        );

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

        let authenticated = PublicApiClient::with_credentials_and_options(
            "https://api.example",
            test_credentials("https://api.example"),
            options,
        )
        .expect("authenticated client");
        assert_eq!(authenticated.transport_options(), options);
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
        let _: TestResponse = client.get("/second").await.expect("second response");
        assert_eq!(
            client
                .last_request_correlation()
                .expect("second correlation")
                .effective_request_id(),
            invocation_request_id
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

    async fn read_request_headers(stream: &mut tokio::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1_024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).await.expect("request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8(request).expect("UTF-8 request headers")
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
