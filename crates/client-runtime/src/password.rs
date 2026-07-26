use rpassword::prompt_password;
use sealtask_client_core::{PublicError, PublicResult};
use std::io::{self, IsTerminal, Read};
use zeroize::Zeroizing;

const DEFAULT_AUTO_UNLOCK_TTL_SECONDS: u64 = 8 * 60 * 60;

pub(crate) fn auto_unlock_ttl_seconds() -> PublicResult<u64> {
    match std::env::var("SEALTASK_UNLOCK_TTL_SECONDS") {
        Ok(value) => {
            let trimmed = value.trim();
            let ttl_seconds = trimmed.parse::<u64>().map_err(|err| {
                PublicError::validation(format!(
                    "invalid SEALTASK_UNLOCK_TTL_SECONDS value '{trimmed}': {err}"
                ))
            })?;
            if ttl_seconds == 0 {
                return Err(PublicError::validation(
                    "SEALTASK_UNLOCK_TTL_SECONDS must be greater than zero",
                ));
            }
            Ok(ttl_seconds)
        }
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_AUTO_UNLOCK_TTL_SECONDS),
        Err(std::env::VarError::NotUnicode(_)) => Err(PublicError::validation(
            "SEALTASK_UNLOCK_TTL_SECONDS must be valid UTF-8",
        )),
    }
}

pub(crate) fn missing_unlock_error(prompt_message: &str) -> PublicError {
    PublicError::validation(format!(
        "{prompt_message} No unlocked workspace-data session or saved unlock key is available. Run 'sealtask auth unlock' for an interactive temporary session. For automation, run 'sealtask auth unlock --password-stdin' or store a key with 'sealtask auth keychain store --password-stdin'."
    ))
}

pub(crate) fn persisted_unlock_error(prompt_message: &str, err: PublicError) -> PublicError {
    PublicError::validation(format!(
        "{prompt_message} Failed to load the saved unlock key: {err}. Run 'sealtask auth unlock' for an interactive temporary session. For automation, run 'sealtask auth unlock --password-stdin' or refresh the saved key with 'sealtask auth keychain store --password-stdin'."
    ))
}

fn read_password(label: &str) -> PublicResult<String> {
    prompt_password(label)
        .map_err(|err| PublicError::unexpected(format!("failed to read password: {err}")))
}

fn read_password_from_stdin() -> PublicResult<String> {
    let mut input = Zeroizing::new(String::new());
    io::stdin().read_to_string(&mut input).map_err(|err| {
        PublicError::unexpected(format!("failed to read password from stdin: {err}"))
    })?;
    Ok(input.trim().to_string())
}

pub(crate) fn read_required_password(
    password_stdin: bool,
    prompt_message: Option<&str>,
) -> PublicResult<String> {
    let password = if password_stdin {
        read_password_from_stdin()?
    } else {
        if !io::stdin().is_terminal() {
            return Err(PublicError::validation(
                "cannot prompt for a password because stdin is not a terminal; use --password-stdin",
            ));
        }
        let prompt = prompt_message
            .map(|message| format!("{message}\nPassword: "))
            .unwrap_or_else(|| "Password: ".to_string());
        read_password(&prompt)?
    };

    if password.is_empty() {
        return Err(PublicError::validation("password is required"));
    }

    Ok(password)
}
