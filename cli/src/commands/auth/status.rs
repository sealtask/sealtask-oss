use crate::output::{CliResult, OutputFormat, print_pretty_json, terminal_line};
use sealtask_client_auth::{
    Credentials, PersistedDataKeyStatus, credentials_path, load_credentials, normalize_api_url,
    persisted_data_key_status,
};
use sealtask_client_runtime::{RuntimeClient, UnlockStatus};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum SessionState {
    Active,
    AccessExpired,
    RefreshExpired,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedBootstrapStatus {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnlockDaemonStatusResult {
    active: bool,
    expires_at_unix: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiUrlMismatch {
    stored_api_url: String,
    current_api_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoggedInStatusResult {
    logged_in: bool,
    email: String,
    api_url: String,
    user_id: Uuid,
    access_token_expires_at: String,
    refresh_token_expires_at: String,
    #[serde(skip_serializing)]
    access_token_expires_display: String,
    #[serde(skip_serializing)]
    refresh_token_expires_display: String,
    session_state: SessionState,
    api_url_mismatch: Option<ApiUrlMismatch>,
    unlock_daemon: UnlockDaemonStatusResult,
    persisted_bootstrap: PersistedBootstrapStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoggedOutStatusResult {
    logged_in: bool,
    credentials_path: String,
    unlock_daemon: UnlockDaemonStatusResult,
    persisted_bootstrap: PersistedBootstrapStatus,
}

enum AuthStatusResult {
    LoggedIn(LoggedInStatusResult),
    LoggedOut(LoggedOutStatusResult),
}

pub(super) fn run(format: OutputFormat, runtime: &RuntimeClient) -> CliResult<()> {
    match load_auth_status(runtime)? {
        AuthStatusResult::LoggedIn(status) => match format {
            OutputFormat::Json => {
                print_pretty_json(&status, "serializing auth status should succeed")
            }
            OutputFormat::Table => print_logged_in_auth_status(&status),
        },
        AuthStatusResult::LoggedOut(status) => match format {
            OutputFormat::Json => print_pretty_json(
                &status,
                "serializing unauthenticated auth status should succeed",
            ),
            OutputFormat::Table => print_logged_out_auth_status(&status),
        },
    }
}

fn load_auth_status(runtime: &RuntimeClient) -> CliResult<AuthStatusResult> {
    let current_api_url = normalize_api_url(runtime.api_url());
    let Some(credentials) = load_credentials()? else {
        return logged_out_auth_status(runtime);
    };

    logged_in_auth_status(runtime, credentials, &current_api_url)
}

fn logged_in_auth_status(
    runtime: &RuntimeClient,
    credentials: Credentials,
    current_api_url: &str,
) -> CliResult<AuthStatusResult> {
    let unlock_status = runtime.unlock_status()?;
    let persisted_status = runtime
        .persisted_data_key_status()?
        .unwrap_or_else(|| persisted_data_key_status(&credentials));

    Ok(AuthStatusResult::LoggedIn(LoggedInStatusResult {
        logged_in: true,
        email: credentials.email.clone(),
        api_url: credentials.api_url.clone(),
        user_id: credentials.user_id,
        access_token_expires_at: credentials.access_expires_at.to_rfc3339(),
        refresh_token_expires_at: credentials.refresh_expires_at.to_rfc3339(),
        access_token_expires_display: credentials
            .access_expires_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        refresh_token_expires_display: credentials
            .refresh_expires_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        session_state: session_state(&credentials),
        api_url_mismatch: api_url_mismatch(&credentials.api_url, current_api_url),
        unlock_daemon: unlock_daemon_status(unlock_status),
        persisted_bootstrap: persisted_bootstrap_status(persisted_status),
    }))
}

fn logged_out_auth_status(runtime: &RuntimeClient) -> CliResult<AuthStatusResult> {
    Ok(AuthStatusResult::LoggedOut(LoggedOutStatusResult {
        logged_in: false,
        credentials_path: credentials_path()?.display().to_string(),
        unlock_daemon: unlock_daemon_status(runtime.unlock_status()?),
        persisted_bootstrap: runtime
            .persisted_data_key_status()?
            .map(persisted_bootstrap_status)
            .unwrap_or_else(unavailable_persisted_bootstrap_status),
    }))
}

fn api_url_mismatch(stored_api_url: &str, current_api_url: &str) -> Option<ApiUrlMismatch> {
    (stored_api_url != current_api_url).then(|| ApiUrlMismatch {
        stored_api_url: stored_api_url.to_string(),
        current_api_url: current_api_url.to_string(),
    })
}

fn unlock_daemon_status(status: UnlockStatus) -> UnlockDaemonStatusResult {
    UnlockDaemonStatusResult {
        active: status.unlocked,
        expires_at_unix: status.expires_at_unix,
    }
}

fn persisted_bootstrap_status(status: PersistedDataKeyStatus) -> PersistedBootstrapStatus {
    match status {
        PersistedDataKeyStatus::Available => simple_persisted_bootstrap_status("available"),
        PersistedDataKeyStatus::Missing => simple_persisted_bootstrap_status("missing"),
        PersistedDataKeyStatus::Unavailable(message) => PersistedBootstrapStatus {
            status: "unavailable",
            message: Some(message),
        },
    }
}

fn simple_persisted_bootstrap_status(status: &'static str) -> PersistedBootstrapStatus {
    PersistedBootstrapStatus {
        status,
        message: None,
    }
}

fn unavailable_persisted_bootstrap_status() -> PersistedBootstrapStatus {
    PersistedBootstrapStatus {
        status: "unavailable",
        message: Some("no credentials are stored for the current target".to_string()),
    }
}

fn print_logged_in_auth_status(status: &LoggedInStatusResult) -> CliResult<()> {
    println!("Logged in as: {}", terminal_line(&status.email));
    println!("API URL: {}", terminal_line(&status.api_url));
    println!("User ID: {}", status.user_id);
    println!(
        "Access token expires: {}",
        status.access_token_expires_display
    );
    println!(
        "Refresh token expires: {}",
        status.refresh_token_expires_display
    );

    if let Some(mismatch) = status.api_url_mismatch.as_ref() {
        println!("\nNote: Stored credentials are for a different API URL.");
        println!("Stored: {}", terminal_line(&mismatch.stored_api_url));
        println!("Current: {}", terminal_line(&mismatch.current_api_url));
    }

    if let Some(notice) = session_state_notice(status.session_state) {
        println!("\n{notice}");
    }

    print_unlock_daemon_status(&status.unlock_daemon, "\n")?;
    print_persisted_bootstrap_status(&status.persisted_bootstrap)
}

fn print_logged_out_auth_status(status: &LoggedOutStatusResult) -> CliResult<()> {
    println!("Not logged in.");
    println!(
        "Credentials would be stored at: {}",
        terminal_line(&status.credentials_path)
    );
    print_unlock_daemon_status(&status.unlock_daemon, "")?;
    print_persisted_bootstrap_status(&status.persisted_bootstrap)
}

fn print_unlock_daemon_status(
    status: &UnlockDaemonStatusResult,
    line_prefix: &str,
) -> CliResult<()> {
    match (status.active, status.expires_at_unix) {
        (true, Some(expires_at_unix)) => {
            println!(
                "{line_prefix}Unlock daemon: active until unix {}",
                expires_at_unix
            )
        }
        (true, None) => println!("{line_prefix}Unlock daemon: active"),
        (false, _) => println!("{line_prefix}Unlock daemon: inactive"),
    }
    Ok(())
}

fn print_persisted_bootstrap_status(status: &PersistedBootstrapStatus) -> CliResult<()> {
    match status.message.as_deref() {
        Some(message) => println!(
            "Persisted bootstrap: {} ({})",
            status.status,
            terminal_line(message)
        ),
        None => println!("Persisted bootstrap: {}", status.status),
    }
    Ok(())
}

fn session_state(credentials: &Credentials) -> SessionState {
    if credentials.is_refresh_expired() {
        SessionState::RefreshExpired
    } else if credentials.is_access_expired() {
        SessionState::AccessExpired
    } else {
        SessionState::Active
    }
}

fn session_state_notice(session_state: SessionState) -> Option<&'static str> {
    match session_state {
        SessionState::RefreshExpired => Some("Warning: Session has expired. Please login again."),
        SessionState::AccessExpired => {
            Some("Note: Access token has expired but will be refreshed automatically.")
        }
        SessionState::Active => None,
    }
}
