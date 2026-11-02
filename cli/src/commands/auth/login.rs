use super::revoke_session_with_timeout;
use crate::human_input::resolve_unlock_ttl;
use crate::input::{prompt, read_required_password};
use crate::interaction::write_interaction_line;
use crate::output::{
    CliResult, OutputFormat, WarningResult, finish_with_warnings, print_simple_result,
    terminal_line, warning_result,
};
use crate::terminal::with_progress;
use sealtask_client_auth::{
    AuthResponse, CompleteMfaLoginError, Credentials, LoginOutcome, SecretMfaCode,
    auth_response_to_credentials, begin_login, clear_persisted_data_key, complete_mfa_login,
    credentials_path, credentials_share_session, load_credentials, logout as revoke_session,
    normalize_api_url, refresh_credentials_if_needed_with_timeout, replace_credentials_atomically,
    with_current_credential_identity,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{RuntimeClient, clear_session, session_key};
use serde::Serialize;
use std::io::{self, Read};
use std::time::Duration;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const MFA_CHALLENGE_EXPIRED_MESSAGE: &str = "MFA challenge expired; restart sign-in";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResult {
    logged_in: bool,
    already_logged_in: bool,
    email: String,
    api_url: String,
    credentials_path: String,
    unlocked: bool,
}

pub(super) async fn run(
    format: OutputFormat,
    runtime: &RuntimeClient,
    email_flag: Option<String>,
    password_stdin: bool,
    non_interactive: bool,
) -> CliResult<()> {
    let api_url = runtime.api_url();
    let normalized_api_url = normalize_api_url(api_url);
    let stored_credentials = load_credentials()?;
    if let Some(credentials) = stored_credentials.as_ref()
        && credentials.api_url == normalized_api_url
        && !credentials.is_refresh_expired()
        && email_flag
            .as_deref()
            .is_none_or(|email| credentials.email.eq_ignore_ascii_case(email.trim()))
    {
        let client = runtime.control_plane_http_client()?;
        let mut refreshed = refresh_credentials_if_needed_with_timeout(
            &client,
            api_url,
            credentials,
            30,
            runtime.api_transport_options().request_timeout(),
        )
        .await?;
        if !reused_session_is_unlocked(runtime, &refreshed)? {
            let input = read_login_input(password_stdin, non_interactive)?;
            runtime
                .unlock_daemon_with_credentials(
                    &mut refreshed,
                    &input.password,
                    resolve_unlock_ttl(None, None)?,
                )
                .await?;
        }
        let result = build_login_result(normalized_api_url, refreshed.email, true)?;
        print_login_result(format, &result, api_url)?;
        return Ok(());
    }

    let email = email_flag.or_else(|| {
        stored_credentials
            .as_ref()
            .filter(|credentials| credentials.api_url == normalized_api_url)
            .map(|credentials| credentials.email.clone())
    });
    let (credentials, warnings) = authenticate_and_unlock(
        format,
        runtime,
        email,
        password_stdin,
        non_interactive,
        resolve_unlock_ttl(None, None)?,
        stored_credentials.as_ref(),
    )
    .await?;
    let result = build_login_result(credentials.api_url, credentials.email, false)?;
    finish_with_warnings(
        format,
        &warnings,
        print_login_result(format, &result, api_url),
    )
}

fn reused_session_is_unlocked(
    runtime: &RuntimeClient,
    credentials: &Credentials,
) -> PublicResult<bool> {
    with_current_credential_identity(credentials, |_| {
        runtime.unlock_status().map(|status| status.unlocked)
    })
}

/// Shared by explicit login and expired-session unlock. Emits no success
/// output so the calling command produces exactly one result.
pub(super) async fn authenticate_and_unlock(
    format: OutputFormat,
    runtime: &RuntimeClient,
    email: Option<String>,
    password_stdin: bool,
    non_interactive: bool,
    ttl_seconds: u64,
    expected_credentials: Option<&Credentials>,
) -> CliResult<(Credentials, Vec<WarningResult>)> {
    sealtask_client_runtime::validate_unlock_ttl(ttl_seconds)?;
    let api_url = runtime.api_url();
    if non_interactive && email.is_none() {
        return Err(
            PublicError::validation("--non-interactive auth login requires --email").into(),
        );
    }
    let email = match email {
        Some(email) => email.trim().to_string(),
        None => prompt(format, "Email: ")?,
    };
    if email.is_empty() {
        return Err(PublicError::validation("email is required").into());
    }
    if non_interactive && !password_stdin {
        return Err(PublicError::validation(
            "--non-interactive auth login requires --password-stdin",
        )
        .into());
    }

    let mut input = read_login_input(password_stdin, non_interactive)?;
    let password = Zeroizing::new(std::mem::take(&mut input.password));

    let client = runtime.control_plane_http_client()?;
    let auth_response = match with_progress(
        "Authenticating…",
        begin_login(&client, api_url, &email, &password),
    )
    .await?
    {
        LoginOutcome::Authenticated(response) => response,
        LoginOutcome::MfaRequired {
            mut pending,
            mut challenge,
        } => {
            let mut supplied_code = input.mfa_code.take();
            loop {
                challenge.expires_in = pending.remaining_seconds();
                if mfa_challenge_is_terminal(challenge.expires_in, challenge.attempts_remaining) {
                    return Err(PublicError::validation(MFA_CHALLENGE_EXPIRED_MESSAGE).into());
                }
                let code = match supplied_code.take() {
                    Some(code) => code,
                    None if password_stdin || non_interactive => {
                        drop(pending);
                        return Err(PublicError::mfa_input_required().into());
                    }
                    None => prompt_mfa_code(format, &challenge)?,
                };
                match with_progress(
                    "Verifying sign-in…",
                    complete_mfa_login(&client, pending, SecretMfaCode::new(code)),
                )
                .await
                {
                    Ok(response) => break response,
                    Err(CompleteMfaLoginError::Retryable {
                        message,
                        pending: retry_pending,
                        attempts_remaining,
                        expires_in,
                        retry_after_seconds,
                    }) => {
                        if password_stdin || non_interactive {
                            drop(retry_pending);
                            return Err(PublicError::validation(message).into());
                        }
                        write_interaction_line(
                            format,
                            format_args!("{}", terminal_line(&message)),
                        )?;
                        challenge = retry_pending.challenge().clone();
                        if let Some(attempts_remaining) = attempts_remaining {
                            challenge.attempts_remaining = attempts_remaining;
                        }
                        if let Some(expires_in) = expires_in {
                            challenge.expires_in = expires_in;
                        }
                        if let Some(wait_seconds) =
                            retry_after_seconds.filter(|seconds| *seconds > 0)
                            && !wait_for_mfa_retry(format, &retry_pending, wait_seconds).await?
                        {
                            let expired = retry_pending.remaining_seconds() == 0;
                            drop(retry_pending);
                            let message = if expired {
                                MFA_CHALLENGE_EXPIRED_MESSAGE.to_string()
                            } else {
                                format!("{message} Retry the login after {wait_seconds} seconds.")
                            };
                            return Err(PublicError::validation(message).into());
                        }
                        pending = retry_pending;
                    }
                    Err(CompleteMfaLoginError::TotpLocked {
                        message,
                        pending: retry_pending,
                    }) => {
                        if password_stdin
                            || non_interactive
                            || !retry_pending
                                .challenge()
                                .methods
                                .contains(&sealtask_client_auth::MfaMethod::BackupCode)
                        {
                            drop(retry_pending);
                            return Err(PublicError::validation(message).into());
                        }
                        write_interaction_line(
                            format,
                            format_args!("{}", terminal_line(&message)),
                        )?;
                        challenge = retry_pending.challenge().clone();
                        challenge.methods = vec![sealtask_client_auth::MfaMethod::BackupCode];
                        pending = retry_pending;
                    }
                    Err(CompleteMfaLoginError::Terminal(message)) => {
                        return Err(PublicError::validation(message).into());
                    }
                }
            }
        }
    };
    let mut warnings = Vec::new();
    let mut replaced_credentials = None;
    let (mut credentials, persistence_result) =
        persist_final_auth_response(api_url, auth_response, |credentials| {
            replace_credentials_atomically(credentials, |previous_credentials| {
                let same_session = match (previous_credentials, expected_credentials) {
                    (Some(current), Some(expected)) => credentials_share_session(current, expected),
                    (None, None) => true,
                    _ => false,
                };
                if !same_session {
                    return Err(PublicError::conflict(
                        "credentials changed during sign-in; retry with the current profile",
                    ));
                }
                // Keep the actual rotated token even if save or lock release
                // reports an error after installing the new credentials.
                replaced_credentials = previous_credentials.cloned();
                if let Some(previous_credentials) = previous_credentials {
                    clear_previous_local_auth_state_if_changed(
                        runtime,
                        previous_credentials,
                        credentials,
                        &mut warnings,
                    )?;
                } else {
                    runtime.clear_read_cache()?;
                }
                Ok(())
            })
        });

    finish_credential_persistence(
        &client,
        runtime.api_transport_options().request_timeout(),
        &credentials,
        persistence_result,
        replaced_credentials.as_ref(),
        &mut warnings,
        load_credentials,
    )
    .await?;

    let unlock_result = runtime
        .unlock_daemon_with_credentials(&mut credentials, &password, ttl_seconds)
        .await;
    // Login is already persisted. Report a distinct partial result if the
    // subsequent local unlock fails so callers do not repeat MFA blindly.
    if let Err(error) = unlock_result {
        return crate::output::public_result_with_warnings(
            Err(PublicError::committed_but_local_processing_failed(
                "login",
                "session",
                format!("{error}; run 'sealtask auth unlock'"),
            )),
            &warnings,
        );
    }
    Ok((credentials, warnings))
}

async fn finish_credential_persistence(
    client: &reqwest::Client,
    timeout: Duration,
    credentials: &Credentials,
    result: PublicResult<Option<Credentials>>,
    replaced_credentials: Option<&Credentials>,
    warnings: &mut Vec<WarningResult>,
    load: impl FnOnce() -> PublicResult<Option<Credentials>>,
) -> CliResult<()> {
    match result {
        Ok(previous) => {
            revoke_previous_session(client, timeout, credentials, previous.as_ref(), warnings)
                .await;
            Ok(())
        }
        Err(error) => {
            // Save can succeed before directory sync or lock release fails.
            // Keep the installed session and retire its actual predecessor.
            if load()
                .ok()
                .flatten()
                .as_ref()
                .is_some_and(|current| credentials_share_session(current, credentials))
            {
                revoke_previous_session(
                    client,
                    timeout,
                    credentials,
                    replaced_credentials,
                    warnings,
                )
                .await;
                return crate::output::public_result_with_warnings(
                    Err(PublicError::committed_but_local_processing_failed(
                        "login",
                        "session",
                        format!("{error}; run 'sealtask auth unlock'"),
                    )),
                    warnings,
                );
            }
            // Rejected installation must not leak its newly issued session.
            let cleanup = revoke_session_with_timeout(
                timeout,
                revoke_session(client, &credentials.api_url, &credentials.refresh_token),
            )
            .await;
            let warning = match cleanup {
                Ok(None) => None,
                Ok(Some(message)) => Some(message),
                Err(error) => Some(error.to_string()),
            }
            .map(|message| {
                warning_result(
                    "login_unused_session_revoke_failed",
                    format!("failed to revoke the unused sign-in session: {message}"),
                )
            });
            warnings.extend(warning);
            crate::output::public_result_with_warnings(Err(error), warnings)
        }
    }
}

async fn revoke_previous_session(
    client: &reqwest::Client,
    timeout: Duration,
    credentials: &Credentials,
    previous: Option<&Credentials>,
    warnings: &mut Vec<WarningResult>,
) {
    if let Some(previous) = previous.filter(|previous| {
        !previous.is_refresh_expired() && previous.refresh_token != credentials.refresh_token
    }) && let Some(warning) = previous_session_revoke_warning(
        with_progress(
            "Revoking the previous session…",
            revoke_session_with_timeout(
                timeout,
                revoke_session(client, &previous.api_url, &previous.refresh_token),
            ),
        )
        .await,
    ) {
        warnings.push(warning);
    }
}

fn clear_previous_local_auth_state_if_changed(
    runtime: &RuntimeClient,
    previous: &Credentials,
    current: &Credentials,
    warnings: &mut Vec<WarningResult>,
) -> PublicResult<()> {
    let local_state_is_unchanged = normalize_api_url(&previous.api_url) == current.api_url
        && previous.user_id == current.user_id
        && previous.data_key_ciphertext.trim() == current.data_key_ciphertext.trim();
    if local_state_is_unchanged {
        return Ok(());
    }

    let previous_session_key = session_key(
        &previous.api_url,
        previous.user_id,
        &previous.data_key_ciphertext,
    )?;
    clear_session(&previous_session_key)?;
    // Keychain bootstrap is optional, including on headless machines where
    // the OS service is unavailable. Match logout: report cleanup failure
    // without making password-based login depend on a desktop keychain.
    if let Err(error) = clear_persisted_data_key(previous) {
        warnings.push(warning_result(
            "login_persisted_bootstrap_clear_failed",
            format!("failed to clear the previous platform keychain entry: {error}"),
        ));
    }
    runtime.clear_read_cache()?;
    Ok(())
}

fn previous_session_revoke_warning(result: PublicResult<Option<String>>) -> Option<WarningResult> {
    match result {
        Ok(Some(message)) => Some(warning_result(
            "login_previous_session_revoke_failed",
            format!("failed to revoke the previous account token on server: {message}"),
        )),
        Ok(None) => None,
        Err(err) => Some(warning_result(
            "login_previous_session_revoke_failed",
            format!("failed to revoke the previous account token on server: {err}"),
        )),
    }
}

fn mfa_challenge_is_terminal(expires_in: u64, attempts_remaining: u8) -> bool {
    expires_in == 0 || attempts_remaining == 0
}

async fn wait_for_mfa_retry(
    format: OutputFormat,
    pending: &sealtask_client_auth::PendingMfaLogin,
    wait_seconds: u64,
) -> CliResult<bool> {
    let deadline = pending.deadline();
    let Some(retry_at) = mfa_retry_at(std::time::Instant::now(), wait_seconds, deadline) else {
        return Ok(false);
    };

    write_interaction_line(
        format,
        format_args!("Retrying is available in {wait_seconds} seconds."),
    )?;
    Ok(tokio::select! {
        biased;
        () = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => false,
        () = tokio::time::sleep_until(tokio::time::Instant::from_std(retry_at)) => true,
    })
}

fn mfa_retry_at(
    now: std::time::Instant,
    wait_seconds: u64,
    deadline: std::time::Instant,
) -> Option<std::time::Instant> {
    if wait_seconds > 60 {
        return None;
    }
    now.checked_add(Duration::from_secs(wait_seconds))
        .filter(|retry_at| *retry_at < deadline)
}

fn persist_final_auth_response<T>(
    api_url: &str,
    auth_response: AuthResponse,
    persist: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> (Credentials, PublicResult<T>) {
    let credentials = auth_response_to_credentials(api_url, auth_response);
    let persisted = persist(&credentials);
    (credentials, persisted)
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct LoginStdinLines {
    password: String,
    mfa_code: Option<String>,
}

fn read_login_input(password_stdin: bool, non_interactive: bool) -> CliResult<LoginStdinLines> {
    if password_stdin {
        read_login_stdin_lines()
    } else if non_interactive {
        Err(
            PublicError::validation("--non-interactive auth login requires --password-stdin")
                .into(),
        )
    } else {
        Ok(LoginStdinLines {
            password: read_required_password(false, None)?,
            mfa_code: None,
        })
    }
}

fn read_login_stdin_lines() -> CliResult<LoginStdinLines> {
    read_login_stdin_lines_from(io::stdin().lock())
}

fn read_login_stdin_lines_from(mut reader: impl Read) -> CliResult<LoginStdinLines> {
    let mut input = Zeroizing::new(String::new());
    reader
        .read_to_string(&mut input)
        .map_err(|err| PublicError::unexpected(format!("failed to read login stdin: {err}")))?;

    let lines: Vec<&str> = input.lines().collect();
    let password = lines
        .first()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .ok_or_else(|| PublicError::validation("password is required on the first stdin line"))?;

    if lines.iter().skip(2).any(|line| !line.trim().is_empty()) {
        return Err(PublicError::validation(
            "login stdin accepts at most two lines: password and optional MFA code",
        )
        .into());
    }

    let mfa_code = lines
        .get(1)
        .filter(|line| !line.is_empty())
        .map(|line| (*line).to_string());

    Ok(LoginStdinLines {
        password: (*password).to_string(),
        mfa_code,
    })
}

fn prompt_mfa_code(
    format: OutputFormat,
    challenge: &sealtask_client_auth::MfaChallenge,
) -> CliResult<String> {
    if challenge
        .methods
        .contains(&sealtask_client_auth::MfaMethod::Totp)
    {
        write_interaction_line(
            format,
            format_args!(
                "Enter the 6-digit code from your authenticator app ({} attempts remaining).",
                challenge.attempts_remaining
            ),
        )?;
    } else {
        write_interaction_line(format, format_args!("Enter one of your MFA backup codes."))?;
    }
    let code = rpassword::prompt_password("Authenticator or backup code: ")
        .map_err(|err| PublicError::unexpected(format!("failed to read MFA code: {err}")))?;
    if code.is_empty() {
        return Err(PublicError::validation("MFA code is required").into());
    }
    Ok(code)
}

fn build_login_result(
    api_url: String,
    email: String,
    already_logged_in: bool,
) -> CliResult<LoginResult> {
    Ok(LoginResult {
        logged_in: true,
        already_logged_in,
        email,
        api_url,
        credentials_path: credentials_path()?.display().to_string(),
        unlocked: true,
    })
}

fn print_login_result(format: OutputFormat, result: &LoginResult, api_url: &str) -> CliResult<()> {
    let table_message = if result.already_logged_in {
        format!(
            "Already logged in as {} ({})\nWorkspace data unlocked.\nNext: sealtask lists",
            terminal_line(&result.email),
            terminal_line(api_url)
        )
    } else {
        format!(
            "Logged in as {}\nCredentials saved to {}\nWorkspace data unlocked.\nNext: sealtask lists",
            terminal_line(&result.email),
            terminal_line(&result.credentials_path)
        )
    };
    print_simple_result(
        format,
        result,
        "serializing login result should succeed",
        &table_message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_should_revoke_previous_session_after_a_committed_persistence_error() {
        use axum::{Json, Router, http::StatusCode, routing::post};
        use std::sync::{Arc, Mutex};

        for status in [StatusCode::NO_CONTENT, StatusCode::BAD_GATEWAY] {
            let observed = Arc::new(Mutex::new(Vec::<String>::new()));
            let recorded = observed.clone();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let api_url = format!("http://{}", listener.local_addr().unwrap());
            let router = Router::new().route(
                "/auth/logout",
                post(move |Json(body): Json<serde_json::Value>| {
                    let recorded = recorded.clone();
                    async move {
                        recorded
                            .lock()
                            .unwrap()
                            .push(body["refreshToken"].as_str().unwrap().to_string());
                        status
                    }
                }),
            );
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let previous = Credentials {
                local_session_id: Some("previous-login".to_string()),
                api_url: api_url.clone(),
                access_token: "previous-access-canary".to_string(),
                refresh_token: "actual-rotated-previous-refresh-canary".to_string(),
                access_expires_at: chrono::Utc::now() + chrono::Duration::minutes(15),
                refresh_expires_at: chrono::Utc::now() + chrono::Duration::days(30),
                user_id: Uuid::now_v7(),
                email: "old@example.test".to_string(),
                data_key_ciphertext: "old-key".to_string(),
            };
            let current = Credentials {
                local_session_id: Some("new-login".to_string()),
                access_token: "new-access-canary".to_string(),
                refresh_token: "new-refresh-canary".to_string(),
                user_id: Uuid::now_v7(),
                email: "new@example.test".to_string(),
                ..previous.clone()
            };
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("credentials.json");
            // Inject an error after publishing the file, matching rename followed
            // by failed directory sync. The normal completion path reads it back.
            let persist = || -> PublicResult<Option<Credentials>> {
                std::fs::write(&path, serde_json::to_vec(&current).unwrap()).unwrap();
                Err(PublicError::unexpected("injected post-save sync failure"))
            };
            let persistence_result = persist();
            let mut warnings = Vec::new();
            let error = finish_credential_persistence(
                &reqwest::Client::new(),
                Duration::from_secs(2),
                &current,
                persistence_result,
                Some(&previous),
                &mut warnings,
                || {
                    Ok(Some(
                        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap(),
                    ))
                },
            )
            .await
            .expect_err("preserve committed persistence failure");

            assert_eq!(error.code(), "committed_but_local_processing_failed");
            assert!(
                error
                    .to_string()
                    .contains("injected post-save sync failure")
            );
            assert_eq!(
                *observed.lock().unwrap(),
                vec![previous.refresh_token.clone()]
            );
            let installed: Credentials =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert_eq!(
                installed, current,
                "the newly installed session remains usable"
            );
            assert!(!error.to_string().contains("canary"));
            if status.is_success() {
                assert!(warnings.is_empty());
            } else {
                assert_eq!(warnings.len(), 1);
                assert_eq!(warnings[0].code(), "login_previous_session_revoke_failed");
                assert!(matches!(
                    error,
                    crate::output::CliError::PublicWithWarnings { .. }
                ));
            }
            server.abort();
            let _ = server.await;
        }
    }

    #[test]
    fn test_should_accept_password_only_from_login_stdin() {
        let parsed = read_login_stdin_lines_from("password\n".as_bytes()).expect("parse");
        assert_eq!(parsed.password, "password");
        assert_eq!(parsed.mfa_code, None);
    }

    #[test]
    fn test_should_preserve_optional_login_factor_line_exactly() {
        for (input, expected_password, expected_code) in [
            ("  password  \n", "password", None),
            ("  password  \r\n", "password", None),
            ("password\n\n", "password", None),
            ("password\r\n\r\n", "password", None),
            ("password\n \n", "password", Some(" ")),
            ("password\n\t\n", "password", Some("\t")),
            ("password\n 012345\n", "password", Some(" 012345")),
            ("password\n012345 \n", "password", Some("012345 ")),
            ("password\n０１２３４５\n", "password", Some("０１２３４５")),
            ("password\n012345\n", "password", Some("012345")),
            (
                "password\nST2-00112233-44556677-8899AABB-CCDDEEFF\n",
                "password",
                Some("ST2-00112233-44556677-8899AABB-CCDDEEFF"),
            ),
            (
                "password\nST2-not-a-canonical-backup-code\n",
                "password",
                Some("ST2-not-a-canonical-backup-code"),
            ),
            ("password\r\n 012345\r\n", "password", Some(" 012345")),
            ("password\n\r", "password", Some("\r")),
        ] {
            let parsed = read_login_stdin_lines_from(input.as_bytes()).expect("parse");
            assert_eq!(parsed.password, expected_password);
            assert_eq!(parsed.mfa_code.as_deref(), expected_code);
        }
    }

    #[test]
    fn test_should_zeroize_login_stdin_secret_container_on_drop() {
        fn assert_zeroize_on_drop<T: Zeroize + ZeroizeOnDrop>() {}

        assert_zeroize_on_drop::<LoginStdinLines>();
    }

    #[test]
    fn test_should_persist_credentials_only_from_final_auth_response() {
        let challenge_token = "must-not-be-persisted-challenge";
        let factor_code = "012345";
        let persisted_json = RefCell::new(None);
        let response = AuthResponse {
            access_token: "final-access-token".to_string(),
            refresh_token: "final-refresh-token".to_string(),
            expires_in: 900,
            refresh_expires_in: 2_592_000,
            token_type: "Bearer".to_string(),
            user: sealtask_client_auth::UserResponse {
                id: Uuid::now_v7(),
                email: "mfa@example.test".to_string(),
                name: "MFA Test".to_string(),
                timezone: "UTC".to_string(),
                avatar_color: "blue".to_string(),
                theme_preference: "system".to_string(),
                email_verified: true,
            },
            data_key_ciphertext: "encrypted-data-key".to_string(),
        };

        let (credentials, persisted) =
            persist_final_auth_response("https://api.example.test", response, |credentials| {
                persisted_json.replace(Some(
                    serde_json::to_string(credentials)
                        .expect("credentials should serialize for the test store"),
                ));
                Ok(())
            });
        persisted.expect("persist final credentials");

        assert_eq!(credentials.access_token, "final-access-token");
        let stored = persisted_json
            .borrow()
            .clone()
            .expect("persistence callback should run once");
        assert!(!stored.contains(challenge_token));
        assert!(!stored.contains(factor_code));
        assert!(!stored.contains("challenge"));
    }

    #[test]
    fn test_should_not_wait_to_or_beyond_mfa_deadline() {
        let now = std::time::Instant::now();
        assert_eq!(
            mfa_retry_at(now, 4, now + Duration::from_millis(4_100)),
            Some(now + Duration::from_secs(4))
        );
        assert_eq!(mfa_retry_at(now, 5, now + Duration::from_secs(5)), None);
        assert_eq!(
            mfa_retry_at(now, 3, now + Duration::from_secs(5)),
            Some(now + Duration::from_secs(3))
        );
        assert_eq!(
            mfa_retry_at(
                now + Duration::from_secs(2),
                3,
                now + Duration::from_secs(5)
            ),
            None
        );
        assert_eq!(mfa_retry_at(now, 61, now + Duration::from_secs(300)), None);
    }

    #[test]
    fn test_should_not_prompt_after_mfa_attempt_budget_is_exhausted() {
        assert!(mfa_challenge_is_terminal(240, 0));
        assert!(mfa_challenge_is_terminal(0, 7));
        assert!(!mfa_challenge_is_terminal(240, 7));
    }

    #[test]
    fn test_should_reject_additional_nonempty_login_stdin_lines() {
        let error = match read_login_stdin_lines_from("password\n012345\nunexpected\n".as_bytes()) {
            Ok(_) => panic!("extra input must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("at most two lines"));

        let parsed = read_login_stdin_lines_from("password\n012345\n \n\t\n".as_bytes())
            .expect("extra whitespace-only lines retain the existing accepted contract");
        assert_eq!(parsed.mfa_code.as_deref(), Some("012345"));
    }
}

#[cfg(test)]
mod reuse_tests;
