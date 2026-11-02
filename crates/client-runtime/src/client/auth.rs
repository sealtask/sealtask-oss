use super::RuntimeClient;
use crate::password::read_required_password;
use crate::unlock_daemon;
use sealtask_client_api::{CurrentUserResponse, DashboardStatsResponse};
use sealtask_client_auth::{
    Credentials, PersistedDataKeyStatus,
    clear_persisted_data_key as clear_persisted_data_key_secret, load_credentials,
    load_credentials_for_url, persisted_data_key_status, save_persisted_data_key,
    with_current_credentials,
};
use sealtask_client_core::{PublicError, PublicResult};

impl RuntimeClient {
    pub async fn get_me(&self) -> PublicResult<CurrentUserResponse> {
        let mut client = self.authenticated_api_client()?;
        client.get_me().await
    }

    pub async fn get_stats(&self) -> PublicResult<DashboardStatsResponse> {
        let mut client = self.authenticated_api_client()?;
        client.get_dashboard_stats().await
    }

    pub async fn unlock_daemon(&self, ttl_seconds: u64, password_stdin: bool) -> PublicResult<()> {
        unlock_daemon::validate_ttl(ttl_seconds)?;
        let mut credentials = self.require_logged_in_credentials()?;
        let password = read_required_password(
            password_stdin,
            Some("Password required to unlock workspace data."),
        )
        .await?;
        self.unlock_daemon_with_credentials(&mut credentials, &password, ttl_seconds)
            .await
    }

    /// Unlock using the credentials and password from a completed login. The
    /// credential guard prevents a concurrent logout/account switch from
    /// publishing a key for a stale session.
    pub async fn unlock_daemon_with_credentials(
        &self,
        credentials: &mut Credentials,
        password: &str,
        ttl_seconds: u64,
    ) -> PublicResult<()> {
        unlock_daemon::validate_ttl(ttl_seconds)?;
        if credentials.api_url != self.api_url() {
            return Err(PublicError::validation(
                "unlock credentials belong to a different API URL",
            ));
        }
        let data_key = self
            .decrypt_data_key_with_password(credentials, password)
            .await?;
        let session_key = self.current_session_key(credentials)?;
        with_current_credentials(credentials, |_| {
            unlock_daemon::unlock(&session_key, &data_key, ttl_seconds)
        })
    }

    pub async fn store_persisted_data_key(&self, password_stdin: bool) -> PublicResult<()> {
        let mut credentials = self.require_logged_in_credentials()?;
        let password = read_required_password(
            password_stdin,
            Some("Password required to save an unlock key."),
        )
        .await?;
        let data_key = self
            .decrypt_data_key_with_password(&mut credentials, &password)
            .await?;
        with_current_credentials(&credentials, |_| {
            save_persisted_data_key(&credentials, data_key.as_bytes())
        })
    }

    pub fn clear_persisted_data_key(&self) -> PublicResult<()> {
        let credentials = match load_credentials_for_url(&self.api_url)? {
            Some(credentials) => credentials,
            None => return Ok(()),
        };
        with_current_credentials(&credentials, |current| {
            clear_persisted_data_key_secret(current)
        })
    }

    pub fn clear_unlock_daemon_session(&self) -> PublicResult<()> {
        let credentials = match load_credentials_for_url(&self.api_url)? {
            Some(credentials) => credentials,
            None => return Ok(()),
        };
        let session_key = self.current_session_key(&credentials)?;
        with_current_credentials(&credentials, |_| unlock_daemon::clear_session(&session_key))
    }

    pub fn unlock_status(&self) -> PublicResult<unlock_daemon::UnlockStatus> {
        match load_credentials()? {
            Some(credentials) => {
                let session_key = unlock_daemon::session_key(
                    &credentials.api_url,
                    credentials.user_id,
                    &credentials.data_key_ciphertext,
                )?;
                unlock_daemon::unlock_status(Some(&session_key))
            }
            None => unlock_daemon::unlock_status(None),
        }
    }

    pub fn persisted_data_key_status(&self) -> PublicResult<Option<PersistedDataKeyStatus>> {
        Ok(load_credentials_for_url(&self.api_url)?
            .map(|credentials| persisted_data_key_status(&credentials)))
    }
}
