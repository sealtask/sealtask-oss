mod login;
mod session;
mod status;

use std::future::Future;
use std::time::Duration;

use crate::args::AuthCommand;
use crate::human_input::resolve_unlock_ttl;
use crate::output::{CliResult, OutputFormat};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::RuntimeClient;

const SESSION_REVOCATION_TIMEOUT: Duration = Duration::from_secs(5);

async fn bounded_session_revocation<F>(
    timeout: Duration,
    revocation: F,
) -> PublicResult<Option<String>>
where
    F: Future<Output = PublicResult<Option<String>>>,
{
    tokio::time::timeout(timeout, revocation)
        .await
        .map_err(|_| {
            PublicError::unexpected(format!(
                "server logout request timed out after {} seconds",
                timeout.as_secs()
            ))
        })?
}

pub(super) async fn revoke_session_with_timeout<F>(
    request_timeout: Duration,
    revocation: F,
) -> PublicResult<Option<String>>
where
    F: Future<Output = PublicResult<Option<String>>>,
{
    bounded_session_revocation(session_revocation_timeout(request_timeout), revocation).await
}

fn session_revocation_timeout(request_timeout: Duration) -> Duration {
    request_timeout.min(SESSION_REVOCATION_TIMEOUT)
}

pub(crate) async fn run_auth(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    command: AuthCommand,
) -> CliResult<()> {
    match command {
        AuthCommand::Login {
            email,
            password_stdin,
        } => login::run(format, runtime, email, password_stdin, non_interactive).await,
        AuthCommand::Unlock {
            ttl,
            ttl_seconds,
            password_stdin,
        } => {
            let ttl_seconds = resolve_unlock_ttl(ttl.as_deref(), ttl_seconds)?;
            session::unlock(
                format,
                runtime,
                ttl_seconds,
                password_stdin,
                non_interactive,
            )
            .await
        }
        AuthCommand::Lock => session::lock(format),
        AuthCommand::Keychain { command } => {
            session::keychain(format, runtime, command, non_interactive).await
        }
        AuthCommand::Logout => session::logout(format, runtime).await,
        AuthCommand::Status => status::run(format, runtime),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_should_bound_session_revocation_waits() {
        let error = bounded_session_revocation(
            Duration::from_millis(10),
            std::future::pending::<PublicResult<Option<String>>>(),
        )
        .await
        .expect_err("a hanging revocation must time out");

        assert!(error.to_string().contains("timed out"));
    }

    #[test]
    fn test_should_cap_revocation_and_honor_shorter_request_timeouts() {
        assert_eq!(
            session_revocation_timeout(Duration::from_secs(60)),
            SESSION_REVOCATION_TIMEOUT
        );
        assert_eq!(
            session_revocation_timeout(Duration::from_millis(250)),
            Duration::from_millis(250)
        );
    }
}
