mod auth;
mod comments;
mod tasks;
mod work_lists;

use crate::models::ReadError;
use crate::password::{
    auto_unlock_ttl_seconds, missing_unlock_error, persisted_unlock_error, read_required_password,
};
use crate::projections::read_error_to_public_error;
use crate::unlock_daemon::{SessionKey, fetch_data_key, session_key, unlock};
use sealtask_client_api::PublicApiClient;
use sealtask_client_auth::{
    Credentials, load_credentials_for_url, load_persisted_data_key, normalize_api_url,
    opaque_login_finish_with_export_key, opaque_login_start, with_current_credentials,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    DataKeyCiphertextVersion, SymmetricKey, data_key_ciphertext_version, decrypt_user_data_key,
    decrypt_user_data_key_with_opaque_export_key,
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct RuntimeClient {
    pub(crate) api_url: String,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkListContext {
    pub(crate) work_list_title: Option<String>,
    pub(crate) list_key: Option<SymmetricKey>,
    pub(crate) read_error: Option<ReadError>,
}

impl RuntimeClient {
    #[must_use]
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: normalize_api_url(&api_url.into()),
            http_client: reqwest::Client::new(),
        }
    }

    #[must_use]
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    pub(crate) fn http_client(&self) -> &reqwest::Client {
        &self.http_client
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
        Ok(PublicApiClient::with_credentials(
            &self.api_url,
            credentials,
        ))
    }

    pub(crate) async fn load_work_list_context(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
        prompt_message: &str,
    ) -> PublicResult<(PublicApiClient, WorkListContext)> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(&mut credentials, password_stdin, prompt_message)
            .await?;
        let mut client = PublicApiClient::with_credentials(&self.api_url, credentials);
        let work_list = client.get_work_list(work_list_id).await?;
        let context = self.context_from_work_list_detail(&work_list, Some(&data_key));
        Ok((client, context))
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
        if password_stdin {
            let password = Zeroizing::new(read_required_password(
                password_stdin,
                Some(prompt_message),
            )?);
            return self
                .decrypt_data_key_with_password(credentials, &password)
                .await;
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
                decrypt_user_data_key(password, &credentials.data_key_ciphertext)
            }
            DataKeyCiphertextVersion::OpaqueExportKeyV2 => {
                let (opaque_state, client_login_state) = opaque_login_start(password)?;
                let mut client =
                    PublicApiClient::with_credentials(&self.api_url, credentials.clone());
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
}
