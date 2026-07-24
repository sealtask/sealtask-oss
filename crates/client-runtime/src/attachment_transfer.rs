use crate::attachment_reconciliation::{FailedUploadContext, compensate_failed_upload_admitted};
use crate::blocking_crypto::{BlockingCryptoAdmission, LargePayloadPermit};
use crate::models::AgentAttachment;
use crate::operation_cancellation::OperationCancellation;
use crate::reconciliation::{mutation_outcome_is_ambiguous, sanitized_error_cause};
use crate::storage::StorageTransferPolicy;
use sealtask_client_api::{
    CompleteAttachmentUploadRequest, InitiateAttachmentUploadResponse, PublicApiClient,
    UpdateTaskRequest,
};
use sealtask_client_core::{PublicError, PublicResult};
use uuid::Uuid;

pub(crate) struct PostInitiationRequest<'a> {
    pub(crate) work_list_id: Uuid,
    pub(crate) task_id: Uuid,
    pub(crate) initiated: &'a InitiateAttachmentUploadResponse,
    pub(crate) ciphertext_bytes: u64,
    pub(crate) update: &'a UpdateTaskRequest,
}

#[cfg(test)]
pub(crate) async fn finish_upload_after_initiation(
    storage_policy: &StorageTransferPolicy,
    client: &mut PublicApiClient,
    ciphertext: Vec<u8>,
    request: PostInitiationRequest<'_>,
    failed_context: FailedUploadContext<'_>,
    projected_attachment: AgentAttachment,
    cancellation: &OperationCancellation,
) -> PublicResult<AgentAttachment> {
    let blocking_crypto = BlockingCryptoAdmission::default();
    let payload_permit = blocking_crypto.admit_large_payload().await?;
    finish_upload_after_initiation_admitted(
        storage_policy,
        client,
        ciphertext,
        request,
        failed_context,
        projected_attachment,
        cancellation,
        &blocking_crypto,
        payload_permit,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn finish_upload_after_initiation_admitted(
    storage_policy: &StorageTransferPolicy,
    client: &mut PublicApiClient,
    ciphertext: Vec<u8>,
    request: PostInitiationRequest<'_>,
    failed_context: FailedUploadContext<'_>,
    projected_attachment: AgentAttachment,
    cancellation: &OperationCancellation,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
) -> PublicResult<AgentAttachment> {
    let primary =
        perform_upload_after_initiation(storage_policy, client, ciphertext, request, cancellation)
            .await;

    match primary {
        Ok(()) => Ok(projected_attachment),
        Err(primary @ PublicError::OutcomeAmbiguous { .. }) => Err(primary),
        Err(primary) => {
            compensate_failed_upload_admitted(
                client,
                failed_context,
                primary,
                blocking_crypto,
                payload_permit,
            )
            .await
        }
    }
}

pub(crate) async fn perform_upload_after_initiation(
    storage_policy: &StorageTransferPolicy,
    client: &mut PublicApiClient,
    ciphertext: Vec<u8>,
    request: PostInitiationRequest<'_>,
    cancellation: &OperationCancellation,
) -> PublicResult<()> {
    ensure_not_cancelled(cancellation)?;
    // Each side-effecting stage is awaited to its bounded transport result once
    // started. Cancellation is observed only at the checkpoints between stages,
    // so a late request cannot race request-scoped compensation unseen.
    send_presigned_attachment_upload(storage_policy, request.initiated, ciphertext).await?;
    ensure_not_cancelled(cancellation)?;
    let completion = CompleteAttachmentUploadRequest {
        ciphertext_bytes: request.ciphertext_bytes,
    };
    if let Err(primary) = client
        .complete_attachment_upload(
            request.work_list_id,
            request.initiated.attachment_id,
            &completion,
        )
        .await
    {
        if !mutation_outcome_is_ambiguous(&primary) {
            return Err(primary);
        }
        // The completion endpoint is idempotent. Retry the same operation
        // before any compensation so a lost 204 cannot turn a successfully
        // completed upload into a deletion request.
        if let Err(retry) = client
            .complete_attachment_upload(
                request.work_list_id,
                request.initiated.attachment_id,
                &completion,
            )
            .await
        {
            return Err(PublicError::outcome_ambiguous(
                "attachment upload completion",
                format!(
                    "primary={}; retry={}; the completion commit could not be confirmed after an idempotent retry",
                    sanitized_error_cause(&primary),
                    sanitized_error_cause(&retry),
                ),
            ));
        }
    }
    ensure_not_cancelled(cancellation)?;
    client
        .update_task(request.work_list_id, request.task_id, request.update)
        .await?;
    Ok(())
}

async fn send_presigned_attachment_upload(
    storage_policy: &StorageTransferPolicy,
    upload: &InitiateAttachmentUploadResponse,
    ciphertext: Vec<u8>,
) -> PublicResult<()> {
    let prepared = storage_policy
        .prepare(
            &upload.upload_url,
            &upload.upload_headers,
            upload.expires_at,
        )
        .await?;
    let response = prepared
        .client
        .put(prepared.url)
        .headers(prepared.headers)
        .body(ciphertext)
        .send()
        .await
        .map_err(|err| {
            PublicError::unexpected(format!(
                "failed to upload attachment ciphertext: {}",
                err.without_url()
            ))
        })?;
    if !response.status().is_success() {
        return Err(PublicError::unexpected(format!(
            "attachment upload failed with status {}",
            response.status()
        )));
    }
    Ok(())
}

pub(crate) fn ensure_not_cancelled(cancellation: &OperationCancellation) -> PublicResult<()> {
    if cancellation.is_cancelled() {
        Err(PublicError::cancelled("attachment upload cancelled"))
    } else {
        Ok(())
    }
}
