#![cfg_attr(test, allow(clippy::unwrap_used))]

use std::time::Duration;

use thiserror::Error;

pub type PublicResult<T> = Result<T, PublicError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimit {
    message: String,
    retry_after: Option<Duration>,
}

impl RateLimit {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

impl std::fmt::Display for RateLimit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ResponseFailureKind {
    BodyRead,
    BodyTruncated,
    BodyTooLarge,
    Transport,
    JsonMalformed,
    JsonSchema,
}

impl ResponseFailureKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BodyRead => "response_body_read",
            Self::BodyTruncated => "response_body_truncated",
            Self::BodyTooLarge => "response_body_too_large",
            Self::Transport => "response_transport",
            Self::JsonMalformed => "response_json_malformed",
            Self::JsonSchema => "response_json_schema",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TransportFailureKind {
    Timeout,
    Connect,
    Body,
    Other,
}

impl TransportFailureKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Timeout => "transport_timeout",
            Self::Connect => "transport_connect",
            Self::Body => "transport_body",
            Self::Other => "transport_other",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportFailure {
    kind: TransportFailureKind,
    message: &'static str,
}

impl TransportFailure {
    #[must_use]
    pub const fn kind(&self) -> TransportFailureKind {
        self.kind
    }
}

impl std::fmt::Display for TransportFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HttpFailureKind {
    Authentication,
    Entitlement,
    Forbidden,
    NotFound,
    Conflict,
    PayloadTooLarge,
    RequestTimeout,
    RateLimited,
    Validation,
    Server,
    Other,
}

impl HttpFailureKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Validation => "validation",
            Self::Entitlement => "entitlement",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::PayloadTooLarge => "payload_too_large",
            Self::RequestTimeout => "request_timeout",
            Self::RateLimited => "rate_limited",
            Self::Server => "http_server_error",
            Self::Other => "http_error",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpFailure {
    status: u16,
    backend_error_code: Option<String>,
    kind: HttpFailureKind,
    message: &'static str,
    retry_after: Option<Duration>,
}

impl HttpFailure {
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn backend_error_code(&self) -> Option<&str> {
        self.backend_error_code.as_deref()
    }

    #[must_use]
    pub const fn kind(&self) -> HttpFailureKind {
        self.kind
    }

    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

impl std::fmt::Display for HttpFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PublicError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Entitlement(String),
    #[error("{0}")]
    PayloadTooLarge(String),
    #[error("{0}")]
    RateLimited(RateLimit),
    #[error("{0}")]
    RequestTimeout(String),
    #[error("{0}")]
    Crypto(String),
    #[error("{0}")]
    Unexpected(String),
    #[error("{0}")]
    Cancelled(String),
    #[error("{message}")]
    Response {
        kind: ResponseFailureKind,
        message: String,
    },
    #[error("{0}")]
    Transport(TransportFailure),
    #[error("{0}")]
    Http(HttpFailure),
    #[error("{operation} failed and cleanup also failed (primary: {primary}; cleanup: {cleanup})")]
    CompensationFailed {
        operation: String,
        primary: String,
        cleanup: String,
    },
    #[error("{operation} has an ambiguous outcome: {details}")]
    OutcomeAmbiguous { operation: String, details: String },
    #[error(
        "{operation} committed, but local response processing failed for {committed_resource}: {details}"
    )]
    CommittedButLocalProcessingFailed {
        operation: String,
        committed_resource: String,
        details: String,
    },
    #[error(
        "MFA is required for this account; call begin_login and complete_mfa_login instead of login"
    )]
    MfaRequiredUseBeginLogin,
    #[error("MFA code required on the second stdin line for enrolled accounts")]
    MfaInputRequired,
}

impl PublicError {
    /// Returns the stable machine-readable classification for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Entitlement(_) => "entitlement",
            Self::PayloadTooLarge(_) => "payload_too_large",
            Self::RateLimited(_) => "rate_limited",
            Self::RequestTimeout(_) => "request_timeout",
            Self::Crypto(_) => "crypto",
            Self::Unexpected(_) => "unexpected",
            Self::Cancelled(_) => "cancelled",
            Self::Response { kind, .. } => kind.code(),
            Self::Transport(failure) => failure.kind.code(),
            Self::Http(failure) => failure.kind.code(),
            Self::CompensationFailed { .. } => "compensation_failed",
            Self::OutcomeAmbiguous { .. } => "outcome_ambiguous",
            Self::CommittedButLocalProcessingFailed { .. } => {
                "committed_but_local_processing_failed"
            }
            Self::MfaRequiredUseBeginLogin => "mfa_required_use_begin_login",
            Self::MfaInputRequired => "mfa_input_required",
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn entitlement(message: impl Into<String>) -> Self {
        Self::Entitlement(message.into())
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::PayloadTooLarge(message.into())
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::RateLimited(RateLimit {
            message: message.into(),
            retry_after: None,
        })
    }

    pub fn rate_limited_with_retry_after(
        message: impl Into<String>,
        retry_after: Duration,
    ) -> Self {
        Self::RateLimited(RateLimit {
            message: message.into(),
            retry_after: Some(retry_after),
        })
    }

    pub fn request_timeout(message: impl Into<String>) -> Self {
        Self::RequestTimeout(message.into())
    }

    pub fn crypto(message: impl Into<String>) -> Self {
        Self::Crypto(message.into())
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected(message.into())
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::Cancelled(message.into())
    }

    pub fn response(kind: ResponseFailureKind, message: impl Into<String>) -> Self {
        Self::Response {
            kind,
            message: message.into(),
        }
    }

    pub const fn transport(kind: TransportFailureKind) -> Self {
        let message = match kind {
            TransportFailureKind::Timeout => "API transport timed out",
            TransportFailureKind::Connect => "could not connect to the API",
            TransportFailureKind::Body => "API response body transport failed",
            TransportFailureKind::Other => "API transport failed",
        };
        Self::Transport(TransportFailure { kind, message })
    }

    pub fn http(
        status: u16,
        backend_error_code: Option<String>,
        retry_after: Option<Duration>,
    ) -> Self {
        let (kind, message) = match status {
            400 | 422 => (
                HttpFailureKind::Validation,
                "API request was rejected by the server",
            ),
            401 => (HttpFailureKind::Authentication, "authentication failed"),
            402 => (HttpFailureKind::Entitlement, "payment required"),
            403 => (HttpFailureKind::Forbidden, "access denied"),
            404 => (HttpFailureKind::NotFound, "resource not found"),
            408 => (
                HttpFailureKind::RequestTimeout,
                "request timed out before completion",
            ),
            409 => (
                HttpFailureKind::Conflict,
                "request conflicted with current server state",
            ),
            413 => (
                HttpFailureKind::PayloadTooLarge,
                "request payload is too large",
            ),
            429 => (HttpFailureKind::RateLimited, "API rate limit exceeded"),
            500..=599 => (
                HttpFailureKind::Server,
                "API server could not complete the request",
            ),
            _ => (HttpFailureKind::Other, "API request failed"),
        };
        Self::Http(HttpFailure {
            status,
            backend_error_code: backend_error_code
                .filter(|code| is_stable_backend_error_code(code)),
            kind,
            message,
            retry_after: (kind == HttpFailureKind::RateLimited)
                .then_some(retry_after)
                .flatten(),
        })
    }

    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited(rate_limit) => rate_limit.retry_after(),
            Self::Http(failure) => failure.retry_after(),
            _ => None,
        }
    }

    #[must_use]
    pub const fn response_failure_kind(&self) -> Option<ResponseFailureKind> {
        match self {
            Self::Response { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    #[must_use]
    pub const fn transport_failure_kind(&self) -> Option<TransportFailureKind> {
        match self {
            Self::Transport(failure) => Some(failure.kind()),
            _ => None,
        }
    }

    #[must_use]
    pub const fn http_status(&self) -> Option<u16> {
        match self {
            Self::Http(failure) => Some(failure.status()),
            _ => None,
        }
    }

    #[must_use]
    pub fn backend_error_code(&self) -> Option<&str> {
        match self {
            Self::Http(failure) => failure.backend_error_code(),
            _ => None,
        }
    }

    #[must_use]
    pub const fn http_failure_kind(&self) -> Option<HttpFailureKind> {
        match self {
            Self::Http(failure) => Some(failure.kind()),
            _ => None,
        }
    }

    pub fn compensation_failed(
        operation: impl Into<String>,
        primary: impl Into<String>,
        cleanup: impl Into<String>,
    ) -> Self {
        Self::CompensationFailed {
            operation: operation.into(),
            primary: primary.into(),
            cleanup: cleanup.into(),
        }
    }

    pub fn outcome_ambiguous(operation: impl Into<String>, details: impl Into<String>) -> Self {
        Self::OutcomeAmbiguous {
            operation: operation.into(),
            details: details.into(),
        }
    }

    pub fn committed_but_local_processing_failed(
        operation: impl Into<String>,
        committed_resource: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::CommittedButLocalProcessingFailed {
            operation: operation.into(),
            committed_resource: committed_resource.into(),
            details: details.into(),
        }
    }

    pub fn mfa_required_use_begin_login() -> Self {
        Self::MfaRequiredUseBeginLogin
    }

    pub fn mfa_input_required() -> Self {
        Self::MfaInputRequired
    }
}

fn is_stable_backend_error_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::{HttpFailureKind, PublicError, ResponseFailureKind, TransportFailureKind};
    use std::time::Duration;

    #[test]
    fn test_should_keep_public_error_codes_stable() {
        let cases = [
            (PublicError::validation("message"), "validation"),
            (PublicError::not_found("message"), "not_found"),
            (PublicError::conflict("message"), "conflict"),
            (PublicError::entitlement("message"), "entitlement"),
            (
                PublicError::payload_too_large("message"),
                "payload_too_large",
            ),
            (PublicError::rate_limited("message"), "rate_limited"),
            (PublicError::request_timeout("message"), "request_timeout"),
            (PublicError::crypto("message"), "crypto"),
            (PublicError::unexpected("message"), "unexpected"),
            (PublicError::cancelled("message"), "cancelled"),
            (
                PublicError::response(ResponseFailureKind::BodyTruncated, "message"),
                "response_body_truncated",
            ),
            (
                PublicError::transport(TransportFailureKind::Timeout),
                "transport_timeout",
            ),
            (
                PublicError::http(503, Some("unexpected_error".to_string()), None),
                "http_server_error",
            ),
            (
                PublicError::compensation_failed("operation", "primary", "cleanup"),
                "compensation_failed",
            ),
            (
                PublicError::outcome_ambiguous("operation", "details"),
                "outcome_ambiguous",
            ),
            (
                PublicError::committed_but_local_processing_failed(
                    "operation",
                    "resource-id",
                    "details",
                ),
                "committed_but_local_processing_failed",
            ),
            (
                PublicError::mfa_required_use_begin_login(),
                "mfa_required_use_begin_login",
            ),
            (PublicError::mfa_input_required(), "mfa_input_required"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.code(), expected);
        }
    }

    #[test]
    fn test_should_keep_http_and_transport_metadata_without_sensitive_copy() {
        let secret = "ciphertext-token-private-path";
        let http = PublicError::http(409, Some("note_create_in_progress".to_string()), None);
        assert_eq!(http.http_status(), Some(409));
        assert_eq!(http.backend_error_code(), Some("note_create_in_progress"));
        assert_eq!(http.http_failure_kind(), Some(HttpFailureKind::Conflict));
        assert!(!http.to_string().contains(secret));
        assert!(!format!("{http:?}").contains(secret));

        let transport = PublicError::transport(TransportFailureKind::Connect);
        assert_eq!(
            transport.transport_failure_kind(),
            Some(TransportFailureKind::Connect)
        );
        assert!(!transport.to_string().contains(secret));
        assert!(!format!("{transport:?}").contains(secret));
    }

    #[test]
    fn test_should_discard_untrusted_backend_error_codes() {
        let secret = "ciphertext-token-private-path";
        let error = PublicError::http(500, Some(secret.to_string()), None);

        assert_eq!(error.backend_error_code(), None);
        assert!(!error.to_string().contains(secret));
        assert!(!format!("{error:?}").contains(secret));
    }

    #[test]
    fn test_should_distinguish_http_authentication_from_validation() {
        let authentication = PublicError::http(401, Some("invalid_credentials".to_string()), None);
        let validation = PublicError::http(400, Some("validation_error".to_string()), None);

        assert_eq!(authentication.code(), "authentication");
        assert_eq!(validation.code(), "validation");
    }

    #[test]
    fn test_should_preserve_typed_retry_delay_without_changing_display_message() {
        let error =
            PublicError::rate_limited_with_retry_after("retry later", Duration::from_secs(60));

        assert_eq!(error.code(), "rate_limited");
        assert_eq!(error.to_string(), "retry later");
        assert_eq!(error.retry_after(), Some(Duration::from_secs(60)));
    }
}
