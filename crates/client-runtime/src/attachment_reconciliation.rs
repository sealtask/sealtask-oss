use crate::blocking_crypto::{BlockingCryptoAdmission, LargePayloadPermit};
use crate::models::AgentAttachment;
use crate::reconciliation::{
    ReconciliationCause, classify_reconciliation_error, mutation_outcome_is_ambiguous,
    outcome_ambiguous, sanitized_error_cause,
};
use sealtask_client_api::PublicApiClient;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{SymmetricKey, decode_sealed_blob, decrypt_task_payload};
use std::time::Duration;
use uuid::Uuid;

const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct FailedUploadContext<'a> {
    pub(crate) work_list_id: Uuid,
    pub(crate) task_id: Uuid,
    pub(crate) attachment_id: Uuid,
    pub(crate) list_key: &'a SymmetricKey,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn reconcile_deleted_task_attachment(
    client: &mut PublicApiClient,
    work_list_id: Uuid,
    task_id: Uuid,
    attachment_id: Uuid,
    previous_updated_at: chrono::DateTime<chrono::Utc>,
    list_key: &SymmetricKey,
    primary: PublicError,
    timeout: Duration,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
) -> PublicResult<()> {
    let transform_key = list_key.clone();
    let response = tokio::time::timeout(timeout, async {
        let task = client.get_task(work_list_id, task_id).await?;
        blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    Ok(project_reconciled_attachments(
                        task.task.payload_ciphertext,
                        task.task.updated_at,
                        transform_key,
                    ))
                },
                "attachment deletion reconciliation task failed",
            )
            .await
    })
    .await;
    let projection = match response {
        Ok(Ok((_, projection))) => projection,
        Ok(Err(error)) => {
            let cause = classify_reconciliation_error(&error, ReconciliationCause::ApiRead);
            let description = format!(
                "the follow-up read could not establish whether the attachment was removed; follow_up={}",
                sanitized_error_cause(&error)
            );
            return Err(outcome_ambiguous(
                "attachment delete",
                &primary,
                cause,
                &description,
            ));
        }
        Err(_) => {
            return Err(outcome_ambiguous(
                "attachment delete",
                &primary,
                ReconciliationCause::Timeout,
                "the follow-up read timed out before it could establish whether the attachment was removed",
            ));
        }
    };
    let (attachments, updated_at) = match projection {
        Ok(projected) => projected,
        Err(ReconciliationCause::Decode) => {
            return Err(outcome_ambiguous(
                "attachment delete",
                &primary,
                ReconciliationCause::Decode,
                "the stored task revision could not be decoded",
            ));
        }
        Err(ReconciliationCause::Decrypt) => {
            return Err(outcome_ambiguous(
                "attachment delete",
                &primary,
                ReconciliationCause::Decrypt,
                "the stored task revision could not be decrypted",
            ));
        }
        Err(ReconciliationCause::Envelope) => {
            return Err(outcome_ambiguous(
                "attachment delete",
                &primary,
                ReconciliationCause::Envelope,
                "the stored task revision used an unsupported envelope",
            ));
        }
        Err(_) => {
            return Err(outcome_ambiguous(
                "attachment delete",
                &primary,
                ReconciliationCause::Projection,
                "the stored attachment revision could not be projected",
            ));
        }
    };
    if !attachments
        .iter()
        .any(|attachment| attachment.id == attachment_id)
    {
        return Ok(());
    }
    if updated_at == previous_updated_at {
        return Err(outcome_ambiguous(
            "attachment delete",
            &primary,
            ReconciliationCause::Divergent,
            "the follow-up read still shows the prior task revision, but the request may execute later",
        ));
    }
    Err(outcome_ambiguous(
        "attachment delete",
        &primary,
        ReconciliationCause::Divergent,
        "the task now has a divergent revision",
    ))
}

fn project_reconciled_attachments(
    payload_ciphertext: String,
    updated_at: chrono::DateTime<chrono::Utc>,
    list_key: SymmetricKey,
) -> Result<(Vec<AgentAttachment>, chrono::DateTime<chrono::Utc>), ReconciliationCause> {
    let payload =
        decode_sealed_blob(&payload_ciphertext).map_err(|_| ReconciliationCause::Decode)?;
    let envelope =
        decrypt_task_payload(&list_key, &payload).map_err(|_| ReconciliationCause::Decrypt)?;
    validate_task_envelope(&envelope.kind, envelope.version)
        .map_err(|_| ReconciliationCause::Envelope)?;
    let attachments = crate::projections::project_attachments(envelope.body.attachments)
        .map_err(|_| ReconciliationCause::Projection)?
        .unwrap_or_default();
    Ok((attachments, updated_at))
}

#[cfg(test)]
pub(crate) async fn compensate_failed_upload_with_timeout(
    client: &mut PublicApiClient,
    context: FailedUploadContext<'_>,
    primary: PublicError,
    cleanup_timeout: Duration,
) -> PublicResult<AgentAttachment> {
    let blocking_crypto = BlockingCryptoAdmission::default();
    let payload_permit = blocking_crypto.admit_large_payload().await?;
    compensate_failed_upload_with_timeout_admitted(
        client,
        context,
        primary,
        cleanup_timeout,
        &blocking_crypto,
        payload_permit,
    )
    .await
}

pub(crate) async fn compensate_failed_upload_admitted(
    client: &mut PublicApiClient,
    context: FailedUploadContext<'_>,
    primary: PublicError,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
) -> PublicResult<AgentAttachment> {
    compensate_failed_upload_with_timeout_admitted(
        client,
        context,
        primary,
        CLEANUP_TIMEOUT,
        blocking_crypto,
        payload_permit,
    )
    .await
}

async fn compensate_failed_upload_with_timeout_admitted(
    client: &mut PublicApiClient,
    context: FailedUploadContext<'_>,
    primary: PublicError,
    cleanup_timeout: Duration,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
) -> PublicResult<AgentAttachment> {
    let primary_cause = sanitized_error_cause(&primary);
    match tokio::time::timeout(
        cleanup_timeout,
        client.delete_attachment(context.work_list_id, context.attachment_id),
    )
    .await
    {
        Ok(Ok(())) => Err(primary),
        Ok(Err(cleanup))
            if matches!(&cleanup, PublicError::NotFound(_))
                || cleanup.http_status() == Some(404) =>
        {
            reconcile_after_failed_cleanup(
                client,
                context,
                primary,
                cleanup_timeout,
                FailedCleanup::NotFound,
                "not_found",
                blocking_crypto,
                payload_permit,
            )
            .await
        }
        Ok(Err(cleanup))
            if matches!(&cleanup, PublicError::Conflict(_))
                || cleanup.http_status() == Some(409) =>
        {
            reconcile_after_failed_cleanup(
                client,
                context,
                primary,
                cleanup_timeout,
                FailedCleanup::Conflict,
                sanitized_error_cause(&cleanup),
                blocking_crypto,
                payload_permit,
            )
            .await
        }
        Ok(Err(cleanup)) if mutation_outcome_is_ambiguous(&cleanup) => {
            let cleanup_cause = sanitized_error_cause(&cleanup);
            reconcile_after_failed_cleanup(
                client,
                context,
                primary,
                cleanup_timeout,
                FailedCleanup::Ambiguous,
                cleanup_cause,
                blocking_crypto,
                payload_permit,
            )
            .await
        }
        Ok(Err(cleanup)) => Err(PublicError::compensation_failed(
            "attachment upload",
            format!("primary={primary_cause}"),
            format!("cleanup={}", sanitized_error_cause(&cleanup)),
        )),
        Err(_) => {
            reconcile_after_failed_cleanup(
                client,
                context,
                primary,
                cleanup_timeout,
                FailedCleanup::Ambiguous,
                "timeout",
                blocking_crypto,
                payload_permit,
            )
            .await
        }
    }
}

#[derive(Clone, Copy)]
enum FailedCleanup {
    NotFound,
    Conflict,
    Ambiguous,
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_after_failed_cleanup(
    client: &mut PublicApiClient,
    context: FailedUploadContext<'_>,
    primary: PublicError,
    reconciliation_timeout: Duration,
    failed_cleanup: FailedCleanup,
    cleanup_cause: &'static str,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
) -> PublicResult<AgentAttachment> {
    let reconciliation = tokio::time::timeout(
        reconciliation_timeout,
        reconcile_linked_attachment(
            client,
            context.work_list_id,
            context.task_id,
            context.attachment_id,
            context.list_key,
            blocking_crypto,
            payload_permit,
        ),
    )
    .await;

    match reconciliation {
        Ok(Ok(Some(attachment))) => Ok(attachment),
        Ok(Ok(None)) if matches!(failed_cleanup, FailedCleanup::NotFound) => Err(primary),
        Ok(Ok(None)) => failed_cleanup_error(
            failed_cleanup,
            &primary,
            cleanup_cause,
            ReconciliationCause::NotLinked,
        ),
        Ok(Err(cause)) => failed_cleanup_error(failed_cleanup, &primary, cleanup_cause, cause),
        Err(_) => failed_cleanup_error(
            failed_cleanup,
            &primary,
            cleanup_cause,
            ReconciliationCause::Timeout,
        ),
    }
}

fn failed_cleanup_error(
    failed_cleanup: FailedCleanup,
    primary: &PublicError,
    cleanup_cause: &'static str,
    reconciliation: ReconciliationCause,
) -> PublicResult<AgentAttachment> {
    match failed_cleanup {
        FailedCleanup::NotFound => Err(PublicError::outcome_ambiguous(
            "attachment upload cleanup",
            format!(
                "primary={}; cleanup={cleanup_cause}; reconciliation={}; cleanup may already have completed or project access may have changed; inspect the task after access is restored before retrying cleanup",
                sanitized_error_cause(primary),
                reconciliation.label()
            ),
        )),
        FailedCleanup::Conflict => Err(outcome_ambiguous(
            "attachment upload",
            primary,
            reconciliation,
            "cleanup reported a conflict and the committed task could not be confirmed",
        )),
        FailedCleanup::Ambiguous => Err(PublicError::outcome_ambiguous(
            "attachment upload cleanup",
            format!(
                "primary={}; cleanup={cleanup_cause}; reconciliation={}; the cleanup commit could not be confirmed",
                sanitized_error_cause(primary),
                reconciliation.label()
            ),
        )),
    }
}

async fn reconcile_linked_attachment(
    client: &mut PublicApiClient,
    work_list_id: Uuid,
    task_id: Uuid,
    attachment_id: Uuid,
    list_key: &SymmetricKey,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
) -> Result<Option<AgentAttachment>, ReconciliationCause> {
    let task = client
        .get_task(work_list_id, task_id)
        .await
        .map_err(|error| classify_reconciliation_error(&error, ReconciliationCause::ApiRead))?;
    let list_key = list_key.clone();
    let (_, projection) = blocking_crypto
        .run_with_large_payload(
            payload_permit,
            move || {
                Ok(project_reconciled_attachments(
                    task.task.payload_ciphertext,
                    task.task.updated_at,
                    list_key,
                ))
            },
            "attachment upload reconciliation task failed",
        )
        .await
        .map_err(|error| classify_reconciliation_error(&error, ReconciliationCause::Projection))?;
    let (attachments, _) = projection?;
    Ok(attachments
        .into_iter()
        .find(|attachment| attachment.id == attachment_id))
}

pub(crate) fn validate_task_envelope(kind: &str, version: u8) -> PublicResult<()> {
    if kind != "task" || version != 1 {
        return Err(PublicError::validation(
            "unsupported encrypted task payload envelope",
        ));
    }
    Ok(())
}
