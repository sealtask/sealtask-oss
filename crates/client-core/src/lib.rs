#![cfg_attr(test, allow(clippy::unwrap_used))]

use thiserror::Error;

pub type PublicResult<T> = Result<T, PublicError>;

#[derive(Debug, Error)]
pub enum PublicError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Crypto(String),
    #[error("{0}")]
    Unexpected(String),
    #[error(
        "MFA is required for this account; call begin_login and complete_mfa_login instead of login"
    )]
    MfaRequiredUseBeginLogin,
    #[error("MFA code required on the second stdin line for enrolled accounts")]
    MfaInputRequired,
}

impl PublicError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn crypto(message: impl Into<String>) -> Self {
        Self::Crypto(message.into())
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected(message.into())
    }

    pub fn mfa_required_use_begin_login() -> Self {
        Self::MfaRequiredUseBeginLogin
    }

    pub fn mfa_input_required() -> Self {
        Self::MfaInputRequired
    }
}
