use sealtask_client_core::{
    HttpFailureKind, PublicError, ResponseFailureKind, TransportFailureKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReconciliationCause {
    ApiRead,
    Decode,
    Decrypt,
    Divergent,
    Envelope,
    NotLinked,
    Projection,
    Timeout,
}

impl ReconciliationCause {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ApiRead => "api_read",
            Self::Decode => "decode",
            Self::Decrypt => "decrypt",
            Self::Divergent => "divergent",
            Self::Envelope => "envelope",
            Self::NotLinked => "not_linked",
            Self::Projection => "projection",
            Self::Timeout => "timeout",
        }
    }
}

pub(crate) fn mutation_outcome_is_ambiguous(error: &PublicError) -> bool {
    match error {
        PublicError::Unexpected(_)
        | PublicError::Response { .. }
        | PublicError::Transport(_)
        | PublicError::OutcomeAmbiguous { .. } => true,
        PublicError::Http(failure) => failure.kind() == HttpFailureKind::Server,
        _ => false,
    }
}

pub(crate) fn classify_reconciliation_error(
    error: &PublicError,
    fallback: ReconciliationCause,
) -> ReconciliationCause {
    match error {
        PublicError::Crypto(_) => ReconciliationCause::Decrypt,
        PublicError::PayloadTooLarge(_) => ReconciliationCause::Decode,
        PublicError::RequestTimeout(_) => ReconciliationCause::Timeout,
        PublicError::Validation(_) => ReconciliationCause::Envelope,
        PublicError::Unexpected(_) | PublicError::RateLimited(_) => ReconciliationCause::Projection,
        PublicError::Transport(failure) if failure.kind() == TransportFailureKind::Timeout => {
            ReconciliationCause::Timeout
        }
        PublicError::Http(failure) if failure.kind() == HttpFailureKind::RequestTimeout => {
            ReconciliationCause::Timeout
        }
        PublicError::Transport(_) | PublicError::Http(_) => ReconciliationCause::ApiRead,
        PublicError::Response {
            kind:
                ResponseFailureKind::JsonMalformed
                | ResponseFailureKind::JsonSchema
                | ResponseFailureKind::BodyTooLarge,
            ..
        } => ReconciliationCause::Decode,
        PublicError::Response { .. } => ReconciliationCause::ApiRead,
        _ => fallback,
    }
}

pub(crate) fn outcome_ambiguous(
    operation: &str,
    primary: &PublicError,
    reconciliation: ReconciliationCause,
    description: &str,
) -> PublicError {
    PublicError::outcome_ambiguous(
        operation,
        format!(
            "primary={}; reconciliation={}; {description}",
            sanitized_error_cause(primary),
            reconciliation.label()
        ),
    )
}

pub(crate) fn sanitized_error_cause(error: &PublicError) -> &'static str {
    match error {
        PublicError::Unexpected(_) => "api_mutation",
        PublicError::Response { kind, .. } => kind.code(),
        PublicError::Transport(failure) => failure.kind().code(),
        PublicError::Http(failure) => failure.kind().code(),
        PublicError::Validation(_) => "validation",
        PublicError::NotFound(_) => "not_found",
        PublicError::Conflict(_) => "conflict",
        PublicError::Entitlement(_) => "entitlement",
        PublicError::PayloadTooLarge(_) => "payload_too_large",
        PublicError::RateLimited(_) => "rate_limited",
        PublicError::RequestTimeout(_) => "request_timeout",
        PublicError::Crypto(_) => "crypto",
        PublicError::Cancelled(_) => "cancelled",
        PublicError::CompensationFailed { .. } => "compensation_failed",
        PublicError::OutcomeAmbiguous { .. } => "outcome_ambiguous",
        PublicError::CommittedButLocalProcessingFailed { .. } => {
            "committed_but_local_processing_failed"
        }
        PublicError::MfaRequiredUseBeginLogin | PublicError::MfaInputRequired => "authentication",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_400_and_422_validation_failures_never_enter_mutation_reconciliation() {
        for status in [400, 422] {
            let error = PublicError::validation(format!("plain HTTP {status} rejection"));
            assert!(
                !mutation_outcome_is_ambiguous(&error),
                "HTTP {status} validation must be definitive"
            );
        }
    }

    #[test]
    fn request_timeout_is_definitive_for_every_note_mutation_branch() {
        for operation in ["create note", "update note", "delete note"] {
            let error = PublicError::request_timeout(format!(
                "{operation} body was not received before server admission; retry the request"
            ));
            assert_eq!(error.code(), "request_timeout");
            assert!(
                !mutation_outcome_is_ambiguous(&error),
                "{operation} must return the typed 408 directly instead of reconciling an outcome that the server says never executed"
            );
            assert!(
                error.to_string().contains("retry the request"),
                "{operation} guidance must describe a safe request retry"
            );
        }
    }

    #[test]
    fn explicit_transport_ambiguity_enters_higher_level_reconciliation() {
        let error = PublicError::outcome_ambiguous(
            "attachment deletion",
            "the server may have applied the request",
        );

        assert!(mutation_outcome_is_ambiguous(&error));
    }

    #[test]
    fn outcome_context_retains_separate_sanitized_categories() {
        let error = outcome_ambiguous(
            "note update",
            &PublicError::transport(TransportFailureKind::Timeout),
            ReconciliationCause::Decrypt,
            "stored revision could not be verified",
        );

        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { details, .. }
                if details == "primary=transport_timeout; reconciliation=decrypt; stored revision could not be verified"
                    && !details.contains("/secret/path")
        ));
    }

    #[test]
    fn reconciliation_errors_keep_their_actual_sanitized_stage() {
        let cases = [
            (
                PublicError::response(ResponseFailureKind::JsonMalformed, "response copy one"),
                ReconciliationCause::Decode,
            ),
            (
                PublicError::crypto("secret key material"),
                ReconciliationCause::Decrypt,
            ),
            (
                PublicError::validation("unsupported encrypted note payload envelope"),
                ReconciliationCause::Envelope,
            ),
            (
                PublicError::unexpected("local task copy one"),
                ReconciliationCause::Projection,
            ),
            (
                PublicError::transport(TransportFailureKind::Timeout),
                ReconciliationCause::Timeout,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(
                classify_reconciliation_error(&error, ReconciliationCause::ApiRead),
                expected
            );
        }
    }
}
