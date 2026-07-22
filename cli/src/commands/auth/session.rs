use super::revoke_session_with_timeout;
use crate::args::KeychainCommand;
use crate::output::{
    CliResult, OutputFormat, WarningResult, finish_with_warnings, print_simple_result,
    public_result_with_warnings, require_password_stdin_for_json_command, warning_result,
};
use sealtask_client_auth::{
    clear_credentials_if_current, clear_persisted_data_key, load_credentials_for_url,
    logout as revoke_session,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{RuntimeClient, clear_session, lock as daemon_lock, session_key};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnlockResult {
    unlocked: bool,
    mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogoutResult {
    logged_out: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedBootstrapEnvelope {
    persisted_bootstrap: PersistedBootstrapStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedBootstrapStatus {
    status: &'static str,
}

pub(super) async fn unlock(
    format: OutputFormat,
    runtime: &RuntimeClient,
    ttl_seconds: u64,
    password_stdin: bool,
) -> CliResult<()> {
    require_password_stdin_for_json_command(format, password_stdin, "auth unlock")?;
    runtime.unlock_daemon(ttl_seconds, password_stdin).await?;
    print_unlock_result(
        format,
        true,
        Some(ttl_seconds),
        &format!("Unlocked local daemon for {} seconds.", ttl_seconds),
    )
}

pub(super) fn lock(format: OutputFormat) -> CliResult<()> {
    daemon_lock()?;
    print_unlock_result(format, false, None, "Locked local daemon.")
}

pub(super) async fn keychain(
    format: OutputFormat,
    runtime: &RuntimeClient,
    command: KeychainCommand,
) -> CliResult<()> {
    let (status, table_message) = match command {
        KeychainCommand::Store { password_stdin } => {
            require_password_stdin_for_json_command(format, password_stdin, "auth keychain store")?;
            runtime.store_persisted_data_key(password_stdin).await?;
            (
                "available",
                "Stored a local bootstrap secret in the platform keychain.",
            )
        }
        KeychainCommand::Clear => {
            runtime.clear_persisted_data_key()?;
            (
                "cleared",
                "Cleared the local bootstrap secret from the platform keychain.",
            )
        }
    };

    print_simple_result(
        format,
        &PersistedBootstrapEnvelope {
            persisted_bootstrap: PersistedBootstrapStatus { status },
        },
        "serializing keychain result should succeed",
        table_message,
    )
}

pub(super) async fn logout(format: OutputFormat, runtime: &RuntimeClient) -> CliResult<()> {
    let Some(credentials) = load_credentials_for_url(runtime.api_url())? else {
        return print_simple_result(
            format,
            &LogoutResult {
                logged_out: false,
                api_url: None,
                reason: Some("not_logged_in"),
            },
            "serializing logout result should succeed",
            "Not logged in.",
        );
    };

    let client = reqwest::Client::new();
    let mut warnings = Vec::new();
    let mut local_warnings = Vec::new();
    let mut local_cleanup_error: Option<PublicError> = None;
    let clear_result = clear_credentials_if_current(&credentials, |current| {
        if let Err(err) = clear_persisted_data_key(current) {
            local_warnings.push(warning_result(
                "logout_persisted_bootstrap_clear_failed",
                format!("failed to clear platform keychain entry: {err}"),
            ));
        }
        let daemon_cleanup_result = session_key(
            &current.api_url,
            current.user_id,
            &current.data_key_ciphertext,
        )
        .and_then(|daemon_session_key| clear_session(&daemon_session_key));
        if let Err(err) = daemon_cleanup_result {
            local_cleanup_error = Some(err);
        }
        Ok(())
    });
    public_result_with_warnings(clear_result, &local_warnings)?;

    if let Some(warning) = logout_revoke_warning(
        revoke_session_with_timeout(revoke_session(
            &client,
            &credentials.api_url,
            &credentials.refresh_token,
        ))
        .await,
    ) {
        warnings.push(warning);
    }
    warnings.extend(local_warnings);
    if let Some(err) = local_cleanup_error {
        public_result_with_warnings(Err(err), &warnings)?;
    }

    let print_result = print_simple_result(
        format,
        &LogoutResult {
            logged_out: true,
            api_url: Some(runtime.api_url().to_string()),
            reason: None,
        },
        "serializing logout result should succeed",
        "Logged out successfully.",
    );
    finish_with_warnings(format, &warnings, print_result)
}

fn print_unlock_result(
    format: OutputFormat,
    unlocked: bool,
    ttl_seconds: Option<u64>,
    table_message: &str,
) -> CliResult<()> {
    print_simple_result(
        format,
        &UnlockResult {
            unlocked,
            mode: "daemon",
            ttl_seconds,
        },
        "serializing unlock result should succeed",
        table_message,
    )
}

fn logout_revoke_warning(result: PublicResult<Option<String>>) -> Option<WarningResult> {
    match result {
        Ok(Some(message)) => Some(warning_result(
            "logout_revoke_failed",
            format!("failed to revoke token on server: {message}"),
        )),
        Ok(None) => None,
        Err(err) => Some(warning_result(
            "logout_revoke_failed",
            format!("failed to revoke token on server: {err}"),
        )),
    }
}
