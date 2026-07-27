use super::revoke_session_with_timeout;
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
    credentials_path, load_credentials, logout as revoke_session, normalize_api_url,
    replace_credentials_atomically,
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
        let result = build_login_result(normalized_api_url, credentials.email.clone(), true)?;
        print_login_result(format, &result, api_url)?;
        return Ok(());
    }

    if non_interactive && email_flag.is_none() {
        return Err(
            PublicError::validation("--non-interactive auth login requires --email").into(),
        );
    }

    let email = match email_flag {
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

    let mut login_stdin = password_stdin.then(read_login_stdin_lines).transpose()?;
    let password = Zeroizing::new(if let Some(lines) = &mut login_stdin {
        std::mem::take(&mut lines.password)
    } else {
        read_required_password(false, None)?
    });

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
            let mut supplied_code = login_stdin.as_mut().and_then(|lines| lines.mfa_code.take());
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
                        if password_stdin {
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
    let (credentials, previous_credentials) =
        persist_final_auth_response(api_url, auth_response, |credentials| {
            replace_credentials_atomically(credentials, |previous_credentials| {
                if let Some(previous_credentials) = previous_credentials {
                    clear_previous_local_auth_state_if_changed(
                        runtime,
                        previous_credentials,
                        credentials,
                    )?;
                } else {
                    runtime.clear_read_cache()?;
                }
                Ok(())
            })
        })?;

    let mut warnings = Vec::new();
    if let Some(previous_credentials) = previous_credentials.as_ref().filter(|previous| {
        !previous.is_refresh_expired() && previous.refresh_token != credentials.refresh_token
    }) && let Some(warning) = previous_session_revoke_warning(
        with_progress(
            "Revoking the previous session…",
            revoke_session_with_timeout(
                runtime.api_transport_options().request_timeout(),
                revoke_session(
                    &client,
                    &previous_credentials.api_url,
                    &previous_credentials.refresh_token,
                ),
            ),
        )
        .await,
    ) {
        warnings.push(warning);
    }

    let print_result = build_login_result(
        credentials.api_url.clone(),
        credentials.email.clone(),
        false,
    )
    .and_then(|result| print_login_result(format, &result, api_url));
    finish_with_warnings(format, &warnings, print_result)
}

fn clear_previous_local_auth_state_if_changed(
    runtime: &RuntimeClient,
    previous: &Credentials,
    current: &Credentials,
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
    clear_persisted_data_key(previous)?;
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
) -> CliResult<(Credentials, T)> {
    let credentials = auth_response_to_credentials(api_url, auth_response);
    let persisted = persist(&credentials)?;
    Ok((credentials, persisted))
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct LoginStdinLines {
    password: String,
    mfa_code: Option<String>,
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
    })
}

fn print_login_result(format: OutputFormat, result: &LoginResult, api_url: &str) -> CliResult<()> {
    let table_message = if result.already_logged_in {
        format!(
            "Already logged in as {} ({})",
            terminal_line(&result.email),
            terminal_line(api_url)
        )
    } else {
        format!(
            "Logged in as {}\nCredentials saved to {}\nNext: sealtask auth unlock",
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

        let (credentials, ()) =
            persist_final_auth_response("https://api.example.test", response, |credentials| {
                persisted_json.replace(Some(
                    serde_json::to_string(credentials)
                        .expect("credentials should serialize for the test store"),
                ));
                Ok(())
            })
            .expect("persist final credentials");

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
