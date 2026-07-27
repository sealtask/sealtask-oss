mod auth;
mod comments;
mod notes;
mod task_references;
mod tasks;
mod work_lists;

pub use tasks::{PreparedTaskCreate, PreparedTaskUpdate, ProjectTaskSession, TaskMutationPlan};

use crate::blocking_crypto::BlockingCryptoAdmission;
use crate::models::ReadError;
use crate::password::{
    auto_unlock_ttl_seconds, missing_unlock_error, persisted_unlock_error, read_required_password,
};
use crate::projections::read_error_to_public_error;
use crate::storage::StorageTransferPolicy;
use crate::unlock_daemon::{SessionKey, fetch_data_key, session_key, unlock};
use crate::{
    operation_cancellation::OperationCancellation,
    upload_lifecycle::{AttachmentUploadFailureReport, UploadLifecycleManager},
};
use sealtask_client_api::{
    ApiCancellationToken, ApiTransportOptions, PublicApiClient, TaskReferenceSchemeResponse,
    WorkListResponse, build_control_plane_http_client,
};
use sealtask_client_auth::{
    Credentials, load_credentials_for_url, load_persisted_data_key, normalize_api_url,
    opaque_login_finish_with_export_key, opaque_login_start, with_current_credentials,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    DataKeyCiphertextVersion, SymmetricKey, TaskReferenceSchemeV1, data_key_ciphertext_version,
    decrypt_user_data_key, decrypt_user_data_key_with_opaque_export_key,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;
use zeroize::Zeroizing;

#[cfg(test)]
use crate::models::AgentAttachment;
#[cfg(test)]
use std::future::Future;
#[cfg(test)]
use std::pin::Pin;

#[cfg(test)]
type TestUploadFuture =
    Pin<Box<dyn Future<Output = PublicResult<AgentAttachment>> + Send + 'static>>;

#[cfg(test)]
#[derive(Clone)]
pub(crate) struct TestUploadWorkflow(
    Arc<dyn Fn(OperationCancellation) -> TestUploadFuture + Send + Sync>,
);

#[cfg(test)]
impl TestUploadWorkflow {
    pub(crate) fn new(
        workflow: impl Fn(OperationCancellation) -> TestUploadFuture + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(workflow))
    }

    pub(crate) fn start(&self, cancellation: OperationCancellation) -> TestUploadFuture {
        (self.0)(cancellation)
    }
}

#[cfg(test)]
impl std::fmt::Debug for TestUploadWorkflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TestUploadWorkflow(<injected>)")
    }
}

#[derive(Debug, Clone)]
struct CachedDataKey {
    session_key: SessionKey,
    data_key: SymmetricKey,
}

#[derive(Debug, Clone)]
pub struct RuntimeClient {
    pub(crate) api_url: String,
    pub(crate) api_transport_options: ApiTransportOptions,
    pub(crate) api_cancellation_token: Option<ApiCancellationToken>,
    pub(crate) storage_policy: StorageTransferPolicy,
    pub(crate) blocking_crypto: BlockingCryptoAdmission,
    pub(crate) upload_lifecycle: UploadLifecycleManager,
    command_data_key: Arc<Mutex<Option<CachedDataKey>>>,
    #[cfg(test)]
    pub(crate) upload_test_workflow: Option<TestUploadWorkflow>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkListContext {
    pub(crate) work_list_title: Option<String>,
    pub(crate) work_list_timezone: String,
    pub(crate) list_key: Option<SymmetricKey>,
    pub(crate) task_reference_schemes: Vec<TaskReferenceSchemeV1>,
    pub(crate) current_task_reference_scheme_revision: Option<i64>,
    pub(crate) current_task_reference_scheme_revision_id: Option<Uuid>,
    pub(crate) read_error: Option<ReadError>,
}

impl WorkListContext {
    pub(crate) fn current_task_reference_scheme(&self) -> Option<&TaskReferenceSchemeV1> {
        let revision = self.current_task_reference_scheme_revision?;
        let revision_id = self.current_task_reference_scheme_revision_id?;
        self.task_reference_schemes
            .iter()
            .find(|scheme| scheme.revision == revision && scheme.scheme_revision_id == revision_id)
    }
}

pub(crate) async fn load_task_reference_scheme_history(
    client: &mut PublicApiClient,
    work_list: &WorkListResponse,
) -> Vec<TaskReferenceSchemeResponse> {
    if !matches!(
        (
            work_list.task_references_enabled_at,
            work_list.current_task_reference_scheme_revision,
            work_list.current_task_reference_scheme_revision_id,
        ),
        (Some(_), Some(_), Some(_))
    ) {
        return Vec::new();
    }

    client
        .get_task_reference_schemes(work_list.id)
        .await
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub(crate) struct UnlockedWorkListContext {
    pub(crate) work_list: WorkListContext,
    pub(crate) data_key: SymmetricKey,
    pub(crate) membership_id: Uuid,
}

impl RuntimeClient {
    pub fn new(api_url: impl Into<String>) -> PublicResult<Self> {
        Self::with_storage_origins(api_url, std::iter::empty::<String>())
    }

    pub fn with_storage_origins<I, S>(
        api_url: impl Into<String>,
        storage_origins: I,
    ) -> PublicResult<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::with_storage_origins_and_transport(
            api_url,
            storage_origins,
            ApiTransportOptions::default(),
        )
    }

    pub fn with_storage_origins_and_transport<I, S>(
        api_url: impl Into<String>,
        storage_origins: I,
        api_transport_options: ApiTransportOptions,
    ) -> PublicResult<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let api_url = normalize_api_url(&api_url.into());
        let storage_policy = StorageTransferPolicy::new(&api_url, storage_origins)?;
        Ok(Self {
            api_url,
            api_transport_options,
            api_cancellation_token: None,
            storage_policy,
            blocking_crypto: BlockingCryptoAdmission::default(),
            upload_lifecycle: UploadLifecycleManager::default(),
            command_data_key: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            upload_test_workflow: None,
        })
    }

    #[must_use]
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    #[must_use]
    pub const fn api_transport_options(&self) -> ApiTransportOptions {
        self.api_transport_options
    }

    /// Installs cooperative API cancellation for this runtime clone's work.
    #[must_use]
    pub fn with_api_cancellation_token(
        mut self,
        api_cancellation_token: ApiCancellationToken,
    ) -> Self {
        self.api_cancellation_token = Some(api_cancellation_token);
        self
    }

    #[must_use]
    pub fn api_cancellation_token(&self) -> Option<ApiCancellationToken> {
        self.api_cancellation_token.clone()
    }

    pub fn control_plane_http_client(&self) -> PublicResult<reqwest::Client> {
        build_control_plane_http_client(self.api_transport_options)
    }

    pub fn current_session_key(&self, credentials: &Credentials) -> PublicResult<SessionKey> {
        session_key(
            &self.api_url,
            credentials.user_id,
            &credentials.data_key_ciphertext,
        )
    }

    pub fn require_logged_in_credentials(&self) -> PublicResult<Credentials> {
        load_credentials_for_url(&self.api_url)?.ok_or_else(|| {
            PublicError::validation("not logged in - run 'sealtask auth login' first")
        })
    }

    pub fn authenticated_api_client(&self) -> PublicResult<PublicApiClient> {
        let credentials = self.require_logged_in_credentials()?;
        if credentials.is_refresh_expired() {
            return Err(PublicError::validation(
                "session expired - run 'sealtask auth login' to authenticate",
            ));
        }
        self.api_client_with_credentials(credentials)
    }

    pub(crate) fn api_client_with_credentials(
        &self,
        credentials: Credentials,
    ) -> PublicResult<PublicApiClient> {
        let client = PublicApiClient::with_credentials_and_options(
            &self.api_url,
            credentials,
            self.api_transport_options,
        )?;
        Ok(match self.api_cancellation_token.clone() {
            Some(token) => client.with_cancellation_token(token),
            None => client,
        })
    }

    /// Waits up to `timeout` for active attachment uploads and their compensation to finish.
    ///
    /// A timeout does not abort the owned lifecycle worker. If the process is forcibly stopped
    /// (including with SIGKILL) after the timeout, durable backend orphan cleanup is the final
    /// recovery mechanism for an initiated but unlinked attachment.
    pub async fn drain_attachment_uploads(&self, timeout: Duration) -> PublicResult<()> {
        self.upload_lifecycle.drain(timeout).await
    }

    /// Removes and returns the bounded, secret-safe attachment upload failure reports.
    #[must_use]
    pub fn take_attachment_upload_failure_reports(&self) -> Vec<AttachmentUploadFailureReport> {
        self.upload_lifecycle.take_failure_reports()
    }

    pub(crate) async fn load_work_list_context(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
        prompt_message: &str,
    ) -> PublicResult<(PublicApiClient, WorkListContext)> {
        let (client, context) = self
            .load_unlocked_work_list_context(work_list_id, password_stdin, prompt_message)
            .await?;
        Ok((client, context.work_list))
    }

    pub(crate) async fn load_unlocked_work_list_context(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
        prompt_message: &str,
    ) -> PublicResult<(PublicApiClient, UnlockedWorkListContext)> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(&mut credentials, password_stdin, prompt_message)
            .await?;
        let mut client = self.api_client_with_credentials(credentials)?;
        let work_list = client.get_work_list(work_list_id).await?;
        let scheme_history =
            load_task_reference_scheme_history(&mut client, &work_list.work_list).await;
        let context = UnlockedWorkListContext {
            membership_id: work_list.work_list.membership.id,
            work_list: self.context_from_work_list_detail(
                &work_list,
                &scheme_history,
                Some(&data_key),
            ),
            data_key,
        };
        Ok((client, context))
    }

    pub(crate) async fn load_unlocked_work_list_context_with_password(
        &self,
        work_list_id: Uuid,
        password: Option<&str>,
        prompt_message: &str,
        cancellation: &OperationCancellation,
    ) -> PublicResult<(PublicApiClient, UnlockedWorkListContext)> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = match password {
            Some(password) => {
                let data_key = self
                    .decrypt_data_key_for_attachment_upload(
                        &mut credentials,
                        password,
                        cancellation,
                    )
                    .await?;
                let session_key = self.current_session_key(&credentials)?;
                self.cache_data_key(session_key, data_key.clone())?;
                data_key
            }
            None => {
                self.load_data_key(&mut credentials, false, prompt_message)
                    .await?
            }
        };
        if cancellation.is_cancelled() {
            return Err(PublicError::cancelled("attachment upload cancelled"));
        }
        let mut client = self.api_client_with_credentials(credentials)?;
        let work_list = tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                return Err(PublicError::cancelled("attachment upload cancelled"));
            }
            result = client.get_work_list(work_list_id) => result?,
        };
        let scheme_history =
            load_task_reference_scheme_history(&mut client, &work_list.work_list).await;
        let context = UnlockedWorkListContext {
            membership_id: work_list.work_list.membership.id,
            work_list: self.context_from_work_list_detail(
                &work_list,
                &scheme_history,
                Some(&data_key),
            ),
            data_key,
        };
        Ok((client, context))
    }

    async fn decrypt_data_key_for_attachment_upload(
        &self,
        credentials: &mut Credentials,
        password: &str,
        cancellation: &OperationCancellation,
    ) -> PublicResult<SymmetricKey> {
        match data_key_ciphertext_version(&credentials.data_key_ciphertext)? {
            DataKeyCiphertextVersion::LegacyPasswordV1 => {
                let password = Zeroizing::new(password.to_string());
                let ciphertext = credentials.data_key_ciphertext.clone();
                self.blocking_crypto
                    .run_cancellable(
                        cancellation,
                        move || decrypt_user_data_key(&password, &ciphertext),
                        "attachment data-key decryption task failed",
                    )
                    .await
            }
            DataKeyCiphertextVersion::OpaqueExportKeyV2 => {
                self.decrypt_data_key_with_password(credentials, password)
                    .await
            }
        }
    }

    pub(crate) fn require_work_list_key<'a>(
        &self,
        context: &'a WorkListContext,
    ) -> PublicResult<&'a SymmetricKey> {
        context.list_key.as_ref().ok_or_else(|| {
            read_error_to_public_error(
                context.read_error.as_ref(),
                "failed to resolve work list key",
            )
        })
    }

    pub(crate) async fn load_data_key(
        &self,
        credentials: &mut Credentials,
        password_stdin: bool,
        prompt_message: &str,
    ) -> PublicResult<SymmetricKey> {
        let session_key = self.current_session_key(credentials)?;
        if let Some(data_key) = self.cached_data_key(&session_key)? {
            return Ok(data_key);
        }

        if password_stdin {
            let password = Zeroizing::new(read_required_password(true, Some(prompt_message))?);
            let data_key = self
                .decrypt_data_key_with_password(credentials, &password)
                .await?;
            let current_session_key = self.current_session_key(credentials)?;
            self.cache_data_key(current_session_key, data_key.clone())?;
            return Ok(data_key);
        }

        if let Some(data_key) =
            with_current_credentials(credentials, |_| fetch_data_key(&session_key))?
        {
            return Ok(data_key);
        }

        match self.load_data_key_from_persisted_secret(credentials, &session_key) {
            Ok(Some(data_key)) => Ok(data_key),
            Ok(None) => Err(missing_unlock_error(prompt_message)),
            Err(err) => Err(persisted_unlock_error(prompt_message, err)),
        }
    }

    pub(crate) async fn decrypt_data_key_with_password(
        &self,
        credentials: &mut Credentials,
        password: &str,
    ) -> PublicResult<SymmetricKey> {
        match data_key_ciphertext_version(&credentials.data_key_ciphertext)? {
            DataKeyCiphertextVersion::LegacyPasswordV1 => {
                let password = Zeroizing::new(password.to_string());
                let ciphertext = credentials.data_key_ciphertext.clone();
                self.blocking_crypto
                    .run(
                        move || decrypt_user_data_key(&password, &ciphertext),
                        "data-key decryption task failed",
                    )
                    .await
            }
            DataKeyCiphertextVersion::OpaqueExportKeyV2 => {
                let (opaque_state, client_login_state) = opaque_login_start(password)?;
                let mut client = self.api_client_with_credentials(credentials.clone())?;
                let challenge = client.start_opaque_export_key(&client_login_state).await?;
                *credentials = client.into_credentials().ok_or_else(|| {
                    PublicError::unexpected(
                        "authenticated OPAQUE export-key client lost its credentials",
                    )
                })?;
                let finish = opaque_login_finish_with_export_key(
                    opaque_state,
                    &credentials.email,
                    password,
                    &challenge.server_login_state,
                )?;
                decrypt_user_data_key_with_opaque_export_key(
                    finish.export_key.as_bytes(),
                    &credentials.data_key_ciphertext,
                )
            }
        }
    }

    fn load_data_key_from_persisted_secret(
        &self,
        credentials: &Credentials,
        session_key: &SessionKey,
    ) -> PublicResult<Option<SymmetricKey>> {
        with_current_credentials(credentials, |current| {
            let Some(data_key_bytes) = load_persisted_data_key(current)? else {
                return Ok(None);
            };
            let data_key = SymmetricKey::from_slice(&data_key_bytes)?;
            unlock(session_key, &data_key, auto_unlock_ttl_seconds()?)?;
            Ok(Some(data_key))
        })
    }

    fn cached_data_key(&self, session_key: &SessionKey) -> PublicResult<Option<SymmetricKey>> {
        let cached = self
            .command_data_key
            .lock()
            .map_err(|_| PublicError::unexpected("command data-key cache lock is unavailable"))?;
        Ok(cached
            .as_ref()
            .filter(|cached| cached.session_key == *session_key)
            .map(|cached| cached.data_key.clone()))
    }

    fn cache_data_key(&self, session_key: SessionKey, data_key: SymmetricKey) -> PublicResult<()> {
        let mut cached = self
            .command_data_key
            .lock()
            .map_err(|_| PublicError::unexpected("command data-key cache lock is unavailable"))?;
        *cached = Some(CachedDataKey {
            session_key,
            data_key,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use sealtask_client_crypto::{DATA_KEY_SALT_BYTES, SealedPayload};
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn api_client_factory_propagates_the_invocation_cancellation_token() {
        let cancellation = ApiCancellationToken::new();
        let runtime = RuntimeClient::new("http://127.0.0.1:9")
            .expect("runtime")
            .with_api_cancellation_token(cancellation.clone());

        let client = runtime
            .api_client_with_credentials(test_legacy_credentials())
            .expect("API client");

        assert_eq!(runtime.api_cancellation_token(), Some(cancellation.clone()));
        assert_eq!(client.cancellation_token(), Some(cancellation));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn non_upload_legacy_password_kdf_keeps_the_tokio_worker_responsive() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let blocking_crypto = runtime.blocking_crypto.clone();
        let mut credentials = test_legacy_credentials();
        let decrypt = tokio::spawn(async move {
            runtime
                .decrypt_data_key_with_password(&mut credentials, "test password")
                .await
        });

        tokio::time::timeout(Duration::from_secs(1), blocking_crypto.wait_for_start())
            .await
            .expect("legacy KDF starts on the blocking pool");
        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("single-worker heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "generic legacy password KDF must not run on the Tokio worker"
        );

        decrypt
            .await
            .expect("legacy KDF caller joins")
            .expect_err("intentionally malformed sealed key fails after KDF");
    }

    #[tokio::test]
    async fn command_data_key_cache_is_shared_and_precedes_password_stdin() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let runtime_clone = runtime.clone();
        let mut credentials = test_legacy_credentials();
        let session_key = runtime
            .current_session_key(&credentials)
            .expect("session key");
        let data_key = SymmetricKey::new([0x2a; 32]);
        runtime
            .cache_data_key(session_key, data_key.clone())
            .expect("cache data key");

        let loaded = runtime_clone
            .load_data_key(
                &mut credentials,
                true,
                "The cache should avoid reading stdin.",
            )
            .await
            .expect("load cached data key");

        assert_eq!(loaded, data_key);
    }

    #[test]
    fn command_data_key_cache_is_scoped_to_the_complete_session_key() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let credentials = test_legacy_credentials();
        let session_key = runtime
            .current_session_key(&credentials)
            .expect("session key");
        let data_key = SymmetricKey::new([0x2a; 32]);
        runtime
            .cache_data_key(session_key.clone(), data_key.clone())
            .expect("cache data key");

        assert_eq!(
            runtime
                .cached_data_key(&session_key)
                .expect("read matching cache"),
            Some(data_key)
        );

        let mut mismatched_session_keys = Vec::new();
        let mut other_api = session_key.clone();
        other_api.api_url = "https://other.example".to_string();
        mismatched_session_keys.push(other_api);
        let mut other_user = session_key.clone();
        other_user.user_id = Uuid::now_v7();
        mismatched_session_keys.push(other_user);
        let mut other_fingerprint = session_key.clone();
        other_fingerprint.data_key_fingerprint = "other-fingerprint".to_string();
        mismatched_session_keys.push(other_fingerprint);
        for mismatched_session_key in mismatched_session_keys {
            assert!(
                runtime
                    .cached_data_key(&mismatched_session_key)
                    .expect("read mismatched cache")
                    .is_none()
            );
        }

        let fresh_runtime = RuntimeClient::new("http://127.0.0.1:9").expect("fresh runtime");
        assert!(
            fresh_runtime
                .cached_data_key(&session_key)
                .expect("read fresh cache")
                .is_none()
        );
    }

    fn test_legacy_credentials() -> Credentials {
        let mut ciphertext = vec![0x5a; DATA_KEY_SALT_BYTES];
        ciphertext.push(0xa5);
        let data_key_ciphertext = STANDARD_NO_PAD.encode(
            SealedPayload::new(ciphertext)
                .to_bytes()
                .expect("encode legacy data-key payload"),
        );
        Credentials {
            api_url: "http://127.0.0.1:9".to_string(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            access_expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            refresh_expires_at: chrono::Utc::now() + chrono::Duration::hours(2),
            user_id: Uuid::now_v7(),
            email: "legacy@example.com".to_string(),
            data_key_ciphertext,
        }
    }
}
