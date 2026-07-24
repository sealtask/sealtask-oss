#[cfg(test)]
use crate::attachment_files::{await_started_file_io, read_bounded, read_upload_file_in};
use crate::attachment_files::{
    normalize_upload_content_type, normalize_upload_file_name, read_upload_file_cancellable,
};
#[cfg(test)]
use crate::attachment_reconciliation::compensate_failed_upload_with_timeout;
use crate::attachment_reconciliation::{
    FailedUploadContext, compensate_failed_upload_admitted, reconcile_deleted_task_attachment,
    validate_task_envelope,
};
#[cfg(test)]
use crate::attachment_transfer::finish_upload_after_initiation;
#[cfg(test)]
use crate::attachment_transfer::perform_upload_after_initiation;
use crate::attachment_transfer::{
    PostInitiationRequest, ensure_not_cancelled, finish_upload_after_initiation_admitted,
};
use crate::blocking_crypto::{BlockingCryptoAdmission, LargePayloadPermit};
use crate::client::RuntimeClient;
use crate::inputs::{DeleteTaskAttachmentArgs, UploadTaskAttachmentArgs};
use crate::models::AgentAttachment;
use crate::operation_cancellation::OperationCancellation;
use crate::reconciliation::{mutation_outcome_is_ambiguous, sanitized_error_cause};
use sealtask_client_api::{
    InitiateAttachmentUploadRequest, InitiateAttachmentUploadResponse, PublicApiClient,
    UpdateTaskRequest,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    ATTACHMENT_BLOB_CONTEXT_LABEL, AttachmentBlobRef, MAX_ATTACHMENT_CIPHERTEXT_BYTES,
    SymmetricKey, TaskPayloadEnvelope, build_task_attachment_ref, build_task_payload_envelope,
    compute_payload_proof, decode_sealed_blob, decrypt_task_payload, derive_payload_binding_key,
    encode_attachment_blob_key, encrypt_attachment_bytes, encrypt_task_payload,
};
use std::time::Duration;
use std::{fmt, future::Future};
use uuid::Uuid;

const MUTATION_RECONCILIATION_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TASK_ATTACHMENTS: usize = 50;

struct UploadAttachmentMetadata {
    file_name: String,
    content_type: String,
    plaintext_bytes: u64,
}

struct PreparedPostInitiationUpload {
    projected_attachment: AgentAttachment,
    update: UpdateTaskRequest,
}

impl fmt::Debug for PreparedPostInitiationUpload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedPostInitiationUpload")
            .field("projected_attachment", &self.projected_attachment)
            .field("update", &"<redacted>")
            .finish()
    }
}

struct PreparedAttachmentUploadTask {
    envelope: TaskPayloadEnvelope,
    attachments: Vec<sealtask_client_crypto::FlexibleValue>,
    projected: Vec<AgentAttachment>,
    binding_key: SymmetricKey,
}

struct PreparedAttachmentDelete {
    previous_updated_at: chrono::DateTime<chrono::Utc>,
    update: UpdateTaskRequest,
}

enum PostInitiationPreparation {
    Prepared {
        payload_permit: LargePayloadPermit,
        prepared: PreparedPostInitiationUpload,
    },
    Reconciled(AgentAttachment),
}

impl fmt::Debug for PostInitiationPreparation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prepared { .. } => formatter.write_str("Prepared { <redacted> }"),
            Self::Reconciled(_) => formatter.write_str("Reconciled(<redacted>)"),
        }
    }
}

struct CancelUploadOnDrop {
    cancellation: OperationCancellation,
    armed: bool,
}

impl CancelUploadOnDrop {
    fn new(cancellation: OperationCancellation) -> Self {
        Self {
            cancellation,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CancelUploadOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.cancellation.cancel();
        }
    }
}

impl RuntimeClient {
    pub async fn upload_task_attachment(
        &self,
        args: UploadTaskAttachmentArgs,
    ) -> PublicResult<AgentAttachment> {
        self.upload_task_attachment_with_cancellation(args, OperationCancellation::new())
            .await
    }

    pub async fn upload_task_attachment_with_cancellation(
        &self,
        args: UploadTaskAttachmentArgs,
        cancellation: OperationCancellation,
    ) -> PublicResult<AgentAttachment> {
        #[cfg(test)]
        if let Some(workflow) = &self.upload_test_workflow {
            return await_lifecycle_owned_upload(
                &self.upload_lifecycle,
                workflow.start(cancellation.clone()),
                cancellation,
            )
            .await;
        }
        let worker_runtime = self.clone();
        let worker_cancellation = cancellation.clone();
        await_lifecycle_owned_upload(
            &self.upload_lifecycle,
            async move {
                worker_runtime
                    .upload_task_attachment_owned(args, worker_cancellation)
                    .await
            },
            cancellation,
        )
        .await
    }

    async fn upload_task_attachment_owned(
        &self,
        args: UploadTaskAttachmentArgs,
        cancellation: OperationCancellation,
    ) -> PublicResult<AgentAttachment> {
        let (mut client, context) = self
            .load_unlocked_work_list_context_with_password(
                args.work_list_id,
                args.password
                    .as_ref()
                    .map(crate::inputs::AttachmentUploadPassword::expose_secret),
                "Password required to encrypt attachment data.",
                &cancellation,
            )
            .await?;
        ensure_not_cancelled(&cancellation)?;

        let payload_permit = self
            .blocking_crypto
            .admit_large_payload_cancellable(&cancellation)
            .await?;
        let list_key = self.require_work_list_key(&context.work_list)?.clone();
        let task = cancel_safe_await(
            &cancellation,
            client.get_task(args.work_list_id, args.task_id),
        )
        .await?;
        ensure_not_cancelled(&cancellation)?;
        let expected_updated_at = task.task.updated_at;
        let task_payload_ciphertext = task.task.payload_ciphertext;
        let task_list_key = list_key.clone();
        let (payload_permit, prepared_task) = self
            .blocking_crypto
            .run_with_large_payload_cancellable(
                payload_permit,
                &cancellation,
                move || prepare_attachment_upload_task(task_payload_ciphertext, task_list_key),
                "attachment task-payload projection failed",
            )
            .await?;
        let PreparedAttachmentUploadTask {
            mut envelope,
            mut attachments,
            projected,
            binding_key,
        } = prepared_task;

        // Large-payload admission precedes the file read and remains owned
        // through encryption, upload, task update, and any compensation.
        let (plaintext, plaintext_bytes) =
            read_upload_file_cancellable(&args.path, &cancellation).await?;
        ensure_not_cancelled(&cancellation)?;
        let file_name = normalize_upload_file_name(&args.path, args.file_name.as_deref())?;
        let content_type = normalize_upload_content_type(args.content_type.as_deref(), &args.path)?;
        let (payload_permit, encrypted) = self
            .blocking_crypto
            .run_with_large_payload_cancellable(
                payload_permit,
                &cancellation,
                move || encrypt_attachment_bytes(&plaintext),
                "attachment encryption task failed",
            )
            .await?;
        ensure_not_cancelled(&cancellation)?;
        let ciphertext_bytes = u64::try_from(encrypted.ciphertext.len()).map_err(|_| {
            PublicError::validation("encrypted attachment is too large for this platform")
        })?;
        if ciphertext_bytes > MAX_ATTACHMENT_CIPHERTEXT_BYTES {
            return Err(PublicError::validation(format!(
                "encrypted attachment cannot exceed {MAX_ATTACHMENT_CIPHERTEXT_BYTES} bytes"
            )));
        }

        // Initiation is allowed to finish under the bounded control-plane timeout even if
        // cancellation arrives. Once an ID exists, this function always compensates it.
        let initiation_request = InitiateAttachmentUploadRequest {
            operation_id: Uuid::now_v7(),
            ciphertext_bytes,
        };
        let initiated = initiate_attachment_upload_with_retry(
            &mut client,
            args.work_list_id,
            &initiation_request,
        )
        .await?;
        let attachment_id = initiated.attachment_id;
        if let Err(primary) = ensure_not_cancelled(&cancellation) {
            return compensate_failed_upload_admitted(
                &mut client,
                FailedUploadContext {
                    work_list_id: args.work_list_id,
                    task_id: args.task_id,
                    attachment_id,
                    list_key: &list_key,
                },
                primary,
                &self.blocking_crypto,
                payload_permit,
            )
            .await;
        }
        let ciphertext = encrypted.ciphertext;
        let file_key = encrypted.file_key;
        let preparation_list_key = list_key.clone();
        let preparation_initiated = initiated.clone();
        let membership_id = context.membership_id;
        let prepared = prepare_post_initiation_with_compensation(
            &self.blocking_crypto,
            payload_permit,
            &mut client,
            &cancellation,
            FailedUploadContext {
                work_list_id: args.work_list_id,
                task_id: args.task_id,
                attachment_id,
                list_key: &list_key,
            },
            move || {
                let projected_attachment = build_projected_attachment(
                    &preparation_initiated,
                    &file_key,
                    &preparation_list_key,
                    membership_id,
                    ciphertext_bytes,
                    UploadAttachmentMetadata {
                        file_name,
                        content_type,
                        plaintext_bytes,
                    },
                )?;
                let new_attachment = build_task_attachment_ref(
                    projected_attachment.id,
                    projected_attachment.file_name.clone(),
                    projected_attachment.content_type.clone(),
                    projected_attachment.size_bytes,
                    projected_attachment.blob_key.clone(),
                    membership_id,
                );
                attachments.push(new_attachment);
                envelope.body.attachments = Some(attachments);
                let attachment_ids = projected
                    .iter()
                    .map(|attachment| attachment.id)
                    .chain(std::iter::once(attachment_id))
                    .collect();
                let encrypted_payload = encrypt_task_payload(
                    &build_task_payload_envelope(envelope.body, envelope.version),
                    &preparation_list_key,
                )?;
                let update = UpdateTaskRequest {
                    expected_updated_at: Some(expected_updated_at),
                    payload_ciphertext_proof: Some(compute_payload_proof(
                        &encrypted_payload.bytes,
                        &binding_key,
                    )?),
                    payload_ciphertext: Some(encrypted_payload.base64),
                    attachment_ids: Some(attachment_ids),
                    ..UpdateTaskRequest::default()
                };
                Ok(PreparedPostInitiationUpload {
                    projected_attachment,
                    update,
                })
            },
        )
        .await?;
        let (payload_permit, prepared) = match prepared {
            PostInitiationPreparation::Prepared {
                payload_permit,
                prepared,
            } => (payload_permit, prepared),
            PostInitiationPreparation::Reconciled(attachment) => return Ok(attachment),
        };
        let PreparedPostInitiationUpload {
            projected_attachment,
            update,
        } = prepared;

        finish_upload_after_initiation_admitted(
            &self.storage_policy,
            &mut client,
            ciphertext,
            PostInitiationRequest {
                work_list_id: args.work_list_id,
                task_id: args.task_id,
                initiated: &initiated,
                ciphertext_bytes,
                update: &update,
            },
            FailedUploadContext {
                work_list_id: args.work_list_id,
                task_id: args.task_id,
                attachment_id,
                list_key: &list_key,
            },
            projected_attachment,
            &cancellation,
            &self.blocking_crypto,
            payload_permit,
        )
        .await
    }

    pub async fn delete_task_attachment(&self, args: DeleteTaskAttachmentArgs) -> PublicResult<()> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to update encrypted attachment data.",
            )
            .await?;
        let payload_permit = self.blocking_crypto.admit_large_payload().await?;
        let list_key = self.require_work_list_key(&context)?.clone();
        let task = client.get_task(args.work_list_id, args.task_id).await?;
        let attachment_id = args.attachment_id;
        let task_id = args.task_id;
        let transform_list_key = list_key.clone();
        let (payload_permit, prepared) = self
            .blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    prepare_attachment_delete_task(
                        task.task.payload_ciphertext,
                        task.task.updated_at,
                        attachment_id,
                        task_id,
                        transform_list_key,
                    )
                },
                "attachment deletion task-payload transformation failed",
            )
            .await?;
        let update_result = client
            .update_task(args.work_list_id, args.task_id, &prepared.update)
            .await;
        match update_result {
            Ok(_) => Ok(()),
            Err(primary) if mutation_outcome_is_ambiguous(&primary) => {
                reconcile_deleted_task_attachment(
                    &mut client,
                    args.work_list_id,
                    args.task_id,
                    args.attachment_id,
                    prepared.previous_updated_at,
                    &list_key,
                    primary,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    &self.blocking_crypto,
                    payload_permit,
                )
                .await
            }
            Err(primary) => Err(primary),
        }
    }
}

fn prepare_attachment_upload_task(
    payload_ciphertext: String,
    list_key: SymmetricKey,
) -> PublicResult<PreparedAttachmentUploadTask> {
    let binding_key = derive_payload_binding_key(&list_key)?;
    let payload_bytes = decode_sealed_blob(&payload_ciphertext)?;
    let mut envelope = decrypt_task_payload(&list_key, &payload_bytes)?;
    validate_task_envelope(&envelope.kind, envelope.version)?;
    let attachments = envelope.body.attachments.take().unwrap_or_default();
    let projected =
        crate::projections::project_attachments(Some(attachments.clone()))?.unwrap_or_default();
    if projected.len() >= MAX_TASK_ATTACHMENTS {
        return Err(PublicError::validation(format!(
            "tasks cannot include more than {MAX_TASK_ATTACHMENTS} attachments"
        )));
    }
    Ok(PreparedAttachmentUploadTask {
        envelope,
        attachments,
        projected,
        binding_key,
    })
}

fn prepare_attachment_delete_task(
    payload_ciphertext: String,
    updated_at: chrono::DateTime<chrono::Utc>,
    attachment_id: Uuid,
    task_id: Uuid,
    list_key: SymmetricKey,
) -> PublicResult<PreparedAttachmentDelete> {
    let binding_key = derive_payload_binding_key(&list_key)?;
    let payload_bytes = decode_sealed_blob(&payload_ciphertext)?;
    let mut envelope = decrypt_task_payload(&list_key, &payload_bytes)?;
    validate_task_envelope(&envelope.kind, envelope.version)?;
    let mut attachments = envelope.body.attachments.take().unwrap_or_default();
    let projected =
        crate::projections::project_attachments(Some(attachments.clone()))?.unwrap_or_default();
    let index = projected
        .iter()
        .position(|attachment| attachment.id == attachment_id)
        .ok_or_else(|| {
            PublicError::validation(format!(
                "attachment {attachment_id} not found on task {task_id}"
            ))
        })?;
    attachments.remove(index);
    let remaining_ids = projected
        .into_iter()
        .filter(|attachment| attachment.id != attachment_id)
        .map(|attachment| attachment.id)
        .collect();
    envelope.body.attachments = Some(attachments);
    let encrypted_payload = encrypt_task_payload(
        &build_task_payload_envelope(envelope.body, envelope.version),
        &list_key,
    )?;
    let update = UpdateTaskRequest {
        expected_updated_at: Some(updated_at),
        payload_ciphertext_proof: Some(compute_payload_proof(
            &encrypted_payload.bytes,
            &binding_key,
        )?),
        payload_ciphertext: Some(encrypted_payload.base64),
        attachment_ids: Some(remaining_ids),
        ..UpdateTaskRequest::default()
    };
    Ok(PreparedAttachmentDelete {
        previous_updated_at: updated_at,
        update,
    })
}

async fn prepare_post_initiation_with_compensation<F>(
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
    client: &mut PublicApiClient,
    cancellation: &OperationCancellation,
    failed_context: FailedUploadContext<'_>,
    prepare: F,
) -> PublicResult<PostInitiationPreparation>
where
    F: FnOnce() -> PublicResult<PreparedPostInitiationUpload> + Send + 'static,
{
    let (payload_permit, prepared) = blocking_crypto
        .run_with_large_payload_preserving(
            payload_permit,
            prepare,
            "attachment task-payload preparation failed",
        )
        .await;
    let prepared = if cancellation.is_cancelled() {
        Err(PublicError::cancelled("attachment upload cancelled"))
    } else {
        prepared
    };
    match prepared {
        Ok(prepared) => Ok(PostInitiationPreparation::Prepared {
            payload_permit,
            prepared,
        }),
        Err(primary) => compensate_failed_upload_admitted(
            client,
            failed_context,
            primary,
            blocking_crypto,
            payload_permit,
        )
        .await
        .map(PostInitiationPreparation::Reconciled),
    }
}

async fn await_lifecycle_owned_upload(
    manager: &crate::upload_lifecycle::UploadLifecycleManager,
    workflow: impl Future<Output = PublicResult<AgentAttachment>> + Send + 'static,
    cancellation: OperationCancellation,
) -> PublicResult<AgentAttachment> {
    let mut drop_guard = CancelUploadOnDrop::new(cancellation.clone());
    let result_rx = manager.supervise(&cancellation, workflow).await?;
    let result = result_rx.await.map_err(|_| {
        PublicError::unexpected("attachment upload supervisor stopped before reporting its result")
    })?;
    drop_guard.disarm();
    result
}

async fn cancel_safe_await<T>(
    cancellation: &OperationCancellation,
    future: impl Future<Output = PublicResult<T>>,
) -> PublicResult<T> {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(PublicError::cancelled("attachment upload cancelled")),
        result = future => result,
    }
}

async fn initiate_attachment_upload_with_retry(
    client: &mut PublicApiClient,
    work_list_id: Uuid,
    request: &InitiateAttachmentUploadRequest,
) -> PublicResult<InitiateAttachmentUploadResponse> {
    match client
        .initiate_attachment_upload(work_list_id, request)
        .await
    {
        Err(primary) if mutation_outcome_is_ambiguous(&primary) => {
            match client
                .initiate_attachment_upload(work_list_id, request)
                .await
            {
                Ok(initiated) => Ok(initiated),
                Err(retry) => Err(PublicError::outcome_ambiguous(
                    "attachment upload initiation",
                    format!(
                        "operation_id={}; primary={}; retry={}; the original initiation may have committed",
                        request.operation_id,
                        sanitized_error_cause(&primary),
                        sanitized_error_cause(&retry),
                    ),
                )),
            }
        }
        result => result,
    }
}

fn build_projected_attachment(
    initiated: &InitiateAttachmentUploadResponse,
    file_key: &SymmetricKey,
    list_key: &SymmetricKey,
    membership_id: Uuid,
    ciphertext_bytes: u64,
    metadata: UploadAttachmentMetadata,
) -> PublicResult<AgentAttachment> {
    let blob_key = encode_attachment_blob_key(
        list_key,
        &AttachmentBlobRef {
            version: 1,
            ciphertext_bytes,
            file_key: file_key.as_bytes().to_vec(),
            enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL.to_string(),
        },
    )?;
    let value = build_task_attachment_ref(
        initiated.attachment_id,
        metadata.file_name,
        metadata.content_type,
        metadata.plaintext_bytes,
        blob_key,
        membership_id,
    );
    crate::projections::project_attachments(Some(vec![value]))?
        .and_then(|mut values| values.pop())
        .ok_or_else(|| PublicError::unexpected("failed to project uploaded attachment"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_std::ambient_authority;
    use cap_std::fs::Dir;
    use chrono::Utc;
    use sealtask_client_api::{TaskDetailResponse, TaskResponse};
    use sealtask_client_auth::Credentials;
    use sealtask_client_crypto::{KEY_SIZE, TaskPayloadBody};
    use std::io;
    use std::path::Path;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Condvar, Mutex};
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _, ReadBuf};
    use tokio::sync::Notify;

    struct CountingReader {
        consumed: usize,
    }

    #[test]
    fn prepared_post_initiation_debug_redacts_attachment_and_task_payload_secrets() {
        const FILE_NAME_CANARY: &str = "prepared-upload-file-name-canary";
        const BLOB_KEY_CANARY: &[u8] = b"prepared-upload-blob-key-canary";
        const PAYLOAD_CANARY: &str = "prepared-upload-task-payload-canary";
        let prepared = PreparedPostInitiationUpload {
            projected_attachment: AgentAttachment {
                id: Uuid::now_v7(),
                file_name: FILE_NAME_CANARY.to_string(),
                content_type: "application/octet-stream".to_string(),
                size_bytes: 7,
                blob_key: BLOB_KEY_CANARY.to_vec(),
            },
            update: UpdateTaskRequest {
                payload_ciphertext: Some(PAYLOAD_CANARY.to_string()),
                payload_ciphertext_proof: Some("prepared-upload-proof-canary".to_string()),
                ..UpdateTaskRequest::default()
            },
        };

        let debug = format!("{prepared:?}");
        assert!(!debug.contains(FILE_NAME_CANARY));
        assert!(!debug.contains(PAYLOAD_CANARY));
        assert!(!debug.contains("prepared-upload-proof-canary"));
        assert!(!debug.contains(&format!("{BLOB_KEY_CANARY:?}")));
        assert!(debug.contains("<redacted>"));
    }

    struct PostInitiationWorkflowFixture {
        api_url: String,
        storage_origin: String,
        storage_policy: crate::storage::StorageTransferPolicy,
        list_key: SymmetricKey,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        membership_id: Uuid,
    }

    fn post_initiation_workflow(
        fixture: PostInitiationWorkflowFixture,
    ) -> crate::client::TestUploadWorkflow {
        crate::client::TestUploadWorkflow::new(move |worker_cancellation| {
            let api_url = fixture.api_url.clone();
            let storage_origin = fixture.storage_origin.clone();
            let storage_policy = fixture.storage_policy.clone();
            let list_key = fixture.list_key.clone();
            let work_list_id = fixture.work_list_id;
            let task_id = fixture.task_id;
            let attachment_id = fixture.attachment_id;
            let membership_id = fixture.membership_id;
            Box::pin(async move {
                let mut client =
                    PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                        .expect("API client");
                let initiated = InitiateAttachmentUploadResponse {
                    attachment_id,
                    upload_url: format!("{storage_origin}/blocked"),
                    upload_headers: std::collections::HashMap::new(),
                    expires_at: Utc::now() + chrono::Duration::minutes(1),
                };
                let projected_attachment = build_projected_attachment(
                    &initiated,
                    &SymmetricKey::new([0x39; KEY_SIZE]),
                    &list_key,
                    membership_id,
                    4,
                    UploadAttachmentMetadata {
                        file_name: "attachment.txt".to_string(),
                        content_type: "text/plain".to_string(),
                        plaintext_bytes: 4,
                    },
                )
                .expect("projected attachment");
                let update = UpdateTaskRequest::default();
                finish_upload_after_initiation(
                    &storage_policy,
                    &mut client,
                    vec![1_u8; 4],
                    PostInitiationRequest {
                        work_list_id,
                        task_id,
                        initiated: &initiated,
                        ciphertext_bytes: 4,
                        update: &update,
                    },
                    FailedUploadContext {
                        work_list_id,
                        task_id,
                        attachment_id,
                        list_key: &list_key,
                    },
                    projected_attachment,
                    &worker_cancellation,
                )
                .await
            })
        })
    }

    impl AsyncRead for CountingReader {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let count = {
                let destination = buffer.initialize_unfilled();
                destination.fill(0xa5);
                destination.len()
            };
            buffer.advance(count);
            self.consumed += count;
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn bounded_reader_consumes_at_most_maximum_plus_one() {
        let mut reader = CountingReader { consumed: 0 };
        let bytes = read_bounded(&mut reader, 100).await.expect("bounded read");
        assert_eq!(bytes.len(), 101);
        assert_eq!(reader.consumed, 101);
    }

    #[tokio::test]
    async fn ambiguous_upload_initiation_replays_once_with_the_same_operation_id() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let work_list_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let server = tokio::spawn(async move {
            let mut operation_ids = Vec::new();
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_http_request_bytes(&mut stream).await;
                let header_end = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .expect("header terminator")
                    + 4;
                let body: serde_json::Value =
                    serde_json::from_slice(&request[header_end..]).expect("request JSON");
                operation_ids.push(
                    body["operationId"]
                        .as_str()
                        .expect("operation ID")
                        .to_string(),
                );
                if attempt == 0 {
                    // The backend may have committed, but the response was
                    // lost before the client could observe it.
                    drop(stream);
                } else {
                    let response = serde_json::to_vec(&serde_json::json!({
                        "attachmentId": attachment_id,
                        "uploadUrl": "https://storage.example/upload",
                        "uploadHeaders": {},
                        "expiresAt": Utc::now() + chrono::Duration::minutes(1),
                    }))
                    .expect("response JSON");
                    write_http_response(&mut stream, "201 Created", "application/json", &response)
                        .await;
                }
            }
            operation_ids
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let operation_id = Uuid::now_v7();
        let response = initiate_attachment_upload_with_retry(
            &mut client,
            work_list_id,
            &InitiateAttachmentUploadRequest {
                operation_id,
                ciphertext_bytes: 42,
            },
        )
        .await
        .expect("replayed initiation");

        assert_eq!(response.attachment_id, attachment_id);
        assert_eq!(
            server.await.expect("server"),
            vec![operation_id.to_string(), operation_id.to_string()]
        );
    }

    #[tokio::test]
    async fn lost_upload_initiation_then_any_retry_rejection_remains_ambiguous() {
        let cases: [(&str, &[u8], &str); 3] = [
            (
                "408 Request Timeout",
                br#"{"error":"request_timeout","message":"secret timeout detail"}"#,
                "request_timeout",
            ),
            (
                "429 Too Many Requests",
                br#"{"error":"rate_limited","message":"secret rate-limit detail"}"#,
                "rate_limited",
            ),
            (
                "402 Payment Required",
                br#"{"error":"payment_required","message":"secret entitlement detail"}"#,
                "entitlement",
            ),
        ];

        for (status, retry_body, retry_cause) in cases {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let work_list_id = Uuid::now_v7();
            let operation_id = Uuid::now_v7();
            let server = tokio::spawn(async move {
                let mut requests = Vec::new();
                for attempt in 0..2 {
                    let (mut stream, _) = listener.accept().await.expect("connection");
                    let request = read_http_request_bytes(&mut stream).await;
                    let header_end = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .expect("header terminator")
                        + 4;
                    let request_line = String::from_utf8_lossy(&request)
                        .lines()
                        .next()
                        .expect("request line")
                        .to_string();
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[header_end..]).expect("request JSON");
                    requests.push((
                        request_line,
                        body["operationId"]
                            .as_str()
                            .expect("operation ID")
                            .to_string(),
                    ));

                    if attempt == 0 {
                        // The first request may have committed. Simulate losing
                        // its response before any status can be observed.
                        drop(stream);
                    } else {
                        write_http_response(&mut stream, status, "application/json", retry_body)
                            .await;
                    }
                }
                requests
            });
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let error = initiate_attachment_upload_with_retry(
                &mut client,
                work_list_id,
                &InitiateAttachmentUploadRequest {
                    operation_id,
                    ciphertext_bytes: 42,
                },
            )
            .await
            .expect_err("a failed replay cannot disprove the first commit");

            assert!(matches!(
                &error,
                PublicError::OutcomeAmbiguous { operation, details }
                    if operation == "attachment upload initiation"
                        && details.contains(&format!("operation_id={operation_id}"))
                        && details.contains("primary=transport_other")
                        && details.contains(&format!("retry={retry_cause}"))
                        && details.contains("original initiation may have committed")
                        && !details.contains("secret")
                        && !details.contains(&api_url)
            ));
            let requests = server.await.expect("server");
            assert_eq!(requests.len(), 2);
            assert!(
                requests
                    .iter()
                    .all(|(line, seen_operation_id)| line.starts_with("POST ")
                        && !line.starts_with("DELETE ")
                        && seen_operation_id == &operation_id.to_string()),
                "the retry must reuse the operation ID and must not compensate without an attachment ID"
            );
        }
    }

    #[tokio::test]
    async fn lost_completion_response_retries_idempotently_without_compensation_delete() {
        let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("API listener");
        let api_url = format!("http://{}", api_listener.local_addr().expect("API address"));
        let storage_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("storage listener");
        let storage_origin = format!(
            "http://{}",
            storage_listener.local_addr().expect("storage address")
        );
        let storage_server = tokio::spawn(async move {
            let (mut upload, _) = storage_listener.accept().await.expect("upload connection");
            read_http_request(&mut upload).await;
            write_http_response(&mut upload, "200 OK", "text/plain", &[]).await;
        });

        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x36; KEY_SIZE]);
        let updated = encrypted_task_detail(
            &list_key,
            work_list_id,
            task_id,
            membership_id,
            Vec::new(),
            Utc::now(),
        );
        let updated_body = serde_json::to_vec(&updated).expect("task response JSON");
        let api_server = tokio::spawn(async move {
            let mut request_lines = Vec::new();

            let (mut first_completion, _) = api_listener.accept().await.expect("first completion");
            let request = read_http_request_bytes(&mut first_completion).await;
            request_lines.push(
                String::from_utf8_lossy(&request)
                    .lines()
                    .next()
                    .expect("first completion request line")
                    .to_string(),
            );
            // The backend committed, but its 204 was lost.
            drop(first_completion);

            let (mut completion_replay, _) =
                api_listener.accept().await.expect("completion replay");
            let request = read_http_request_bytes(&mut completion_replay).await;
            request_lines.push(
                String::from_utf8_lossy(&request)
                    .lines()
                    .next()
                    .expect("completion replay request line")
                    .to_string(),
            );
            write_http_response(&mut completion_replay, "204 No Content", "text/plain", &[]).await;

            let (mut task_update, _) = api_listener.accept().await.expect("task update");
            let request = read_http_request_bytes(&mut task_update).await;
            request_lines.push(
                String::from_utf8_lossy(&request)
                    .lines()
                    .next()
                    .expect("task update request line")
                    .to_string(),
            );
            write_http_response(
                &mut task_update,
                "200 OK",
                "application/json",
                &updated_body,
            )
            .await;

            request_lines
        });

        let storage_policy =
            crate::storage::StorageTransferPolicy::new(&api_url, [&storage_origin])
                .expect("storage policy");
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let initiated = InitiateAttachmentUploadResponse {
            attachment_id,
            upload_url: format!("{storage_origin}/ciphertext"),
            upload_headers: std::collections::HashMap::new(),
            expires_at: Utc::now() + chrono::Duration::minutes(1),
        };
        let projected_attachment = build_projected_attachment(
            &initiated,
            &SymmetricKey::new([0x37; KEY_SIZE]),
            &list_key,
            membership_id,
            4,
            UploadAttachmentMetadata {
                file_name: "attachment.txt".to_string(),
                content_type: "text/plain".to_string(),
                plaintext_bytes: 4,
            },
        )
        .expect("projected attachment");
        let update = UpdateTaskRequest::default();
        let cancellation = OperationCancellation::new();
        let result = finish_upload_after_initiation(
            &storage_policy,
            &mut client,
            vec![0x42; 4],
            PostInitiationRequest {
                work_list_id,
                task_id,
                initiated: &initiated,
                ciphertext_bytes: 4,
                update: &update,
            },
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            projected_attachment,
            &cancellation,
        )
        .await
        .expect("lost completion response is recovered by idempotent retry");
        assert_eq!(result.id, attachment_id);

        let request_lines = api_server.await.expect("API server");
        assert_eq!(request_lines.len(), 3);
        assert!(request_lines[0].contains("/complete"));
        assert!(request_lines[1].contains("/complete"));
        assert!(request_lines[2].starts_with("PATCH "));
        assert!(
            request_lines
                .iter()
                .all(|line| !line.starts_with("DELETE ")),
            "a successful completion must never be compensated after its first response is lost"
        );
        storage_server.await.expect("storage server");
    }

    #[tokio::test]
    async fn two_lost_completion_responses_remain_ambiguous_without_compensation_delete() {
        let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("API listener");
        let api_url = format!("http://{}", api_listener.local_addr().expect("API address"));
        let storage_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("storage listener");
        let storage_origin = format!(
            "http://{}",
            storage_listener.local_addr().expect("storage address")
        );
        let storage_server = tokio::spawn(async move {
            let (mut upload, _) = storage_listener.accept().await.expect("upload connection");
            read_http_request(&mut upload).await;
            write_http_response(&mut upload, "200 OK", "text/plain", &[]).await;
        });

        let server_done = Arc::new(Notify::new());
        let api_server_done = server_done.clone();
        let api_server = tokio::spawn(async move {
            let mut request_lines = Vec::new();
            for _ in 0..2 {
                let (mut completion, _) =
                    api_listener.accept().await.expect("completion connection");
                let request = read_http_request_bytes(&mut completion).await;
                request_lines.push(
                    String::from_utf8_lossy(&request)
                        .lines()
                        .next()
                        .expect("completion request line")
                        .to_string(),
                );
                // Both completion transactions committed, but both 204
                // responses were lost before reaching the client.
                drop(completion);
            }

            loop {
                tokio::select! {
                    biased;
                    () = api_server_done.notified() => break,
                    accepted = api_listener.accept() => {
                        let (mut request, _) = accepted.expect("unexpected follow-up connection");
                        let bytes = read_http_request_bytes(&mut request).await;
                        request_lines.push(
                            String::from_utf8_lossy(&bytes)
                                .lines()
                                .next()
                                .expect("follow-up request line")
                                .to_string(),
                        );
                        write_http_response(&mut request, "204 No Content", "text/plain", &[]).await;
                    }
                }
            }
            request_lines
        });

        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x6b; KEY_SIZE]);
        let storage_policy =
            crate::storage::StorageTransferPolicy::new(&api_url, [&storage_origin])
                .expect("storage policy");
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let initiated = InitiateAttachmentUploadResponse {
            attachment_id,
            upload_url: format!("{storage_origin}/ciphertext"),
            upload_headers: std::collections::HashMap::new(),
            expires_at: Utc::now() + chrono::Duration::minutes(1),
        };
        let projected_attachment = build_projected_attachment(
            &initiated,
            &SymmetricKey::new([0x6c; KEY_SIZE]),
            &list_key,
            membership_id,
            4,
            UploadAttachmentMetadata {
                file_name: "attachment.txt".to_string(),
                content_type: "text/plain".to_string(),
                plaintext_bytes: 4,
            },
        )
        .expect("projected attachment");
        let update = UpdateTaskRequest::default();
        let cancellation = OperationCancellation::new();
        let error = finish_upload_after_initiation(
            &storage_policy,
            &mut client,
            vec![0x42; 4],
            PostInitiationRequest {
                work_list_id,
                task_id,
                initiated: &initiated,
                ciphertext_bytes: 4,
                update: &update,
            },
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            projected_attachment,
            &cancellation,
        )
        .await
        .expect_err("two lost completion responses must remain ambiguous");
        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { operation, details }
                if operation == "attachment upload completion"
                    && details
                        == "primary=transport_other; retry=transport_other; the completion commit could not be confirmed after an idempotent retry"
        ));

        server_done.notify_one();
        let request_lines = api_server.await.expect("API server");
        assert_eq!(request_lines.len(), 2);
        assert!(request_lines.iter().all(|line| line.contains("/complete")));
        assert!(
            request_lines
                .iter()
                .all(|line| !line.starts_with("DELETE ")),
            "an unresolved completion must never trigger destructive compensation"
        );
        storage_server.await.expect("storage server");
    }

    #[tokio::test]
    async fn rejects_empty_and_non_regular_upload_paths() {
        let directory = tempfile::tempdir().expect("temp dir");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        assert!(read_upload_file_in(root, Path::new(".")).await.is_err());
        tokio::fs::write(directory.path().join("empty.txt"), [])
            .await
            .expect("empty file");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        assert!(
            read_upload_file_in(root, Path::new("empty.txt"))
                .await
                .is_err()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symbolic_link_upload_paths() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temp dir");
        let target = directory.path().join("target.txt");
        tokio::fs::write(&target, b"body").await.expect("target");
        let link = directory.path().join("link.txt");
        symlink(&target, &link).expect("symlink");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        let error = read_upload_file_in(root, Path::new("link.txt"))
            .await
            .expect_err("reject symlink");
        assert!(error.to_string().contains("symbolic link"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_upload_through_intermediate_symlink_escape() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temp dir");
        let outside = tempfile::tempdir().expect("outside temp dir");
        let target = outside.path().join("secret.txt");
        tokio::fs::write(&target, b"outside remains")
            .await
            .expect("outside target");
        symlink(outside.path(), directory.path().join("escape")).expect("directory symlink");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        let error = read_upload_file_in(root, Path::new("escape/secret.txt"))
            .await
            .expect_err("intermediate escape must be rejected");

        assert!(matches!(error, PublicError::Validation(_)));
        assert_eq!(
            tokio::fs::read(&target).await.expect("outside target"),
            b"outside remains"
        );
    }

    #[tokio::test]
    async fn rejects_absolute_and_parent_relative_upload_paths() {
        let directory = tempfile::tempdir().expect("temp dir");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        assert!(read_upload_file_in(root, directory.path()).await.is_err());

        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        assert!(
            read_upload_file_in(root, Path::new("../outside.txt"))
                .await
                .is_err()
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn rejects_upload_through_intermediate_directory_reparse_point() {
        use std::os::windows::fs::symlink_dir;

        let directory = tempfile::tempdir().expect("temp dir");
        let outside = tempfile::tempdir().expect("outside temp dir");
        let target = outside.path().join("secret.txt");
        tokio::fs::write(&target, b"outside remains")
            .await
            .expect("outside target");
        if let Err(error) = symlink_dir(outside.path(), directory.path().join("escape")) {
            if error.kind() == io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("directory reparse point: {error}");
        }
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        let error = read_upload_file_in(root, Path::new("escape/secret.txt"))
            .await
            .expect_err("intermediate escape must be rejected");

        assert!(matches!(error, PublicError::Validation(_)));
        assert_eq!(
            tokio::fs::read(&target).await.expect("outside target"),
            b"outside remains"
        );
    }

    #[tokio::test]
    async fn cancellation_token_is_race_safe() {
        let cancellation = OperationCancellation::new();
        cancellation.cancel();
        tokio::time::timeout(Duration::from_millis(50), cancellation.cancelled())
            .await
            .expect("cancel notification");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn lifecycle_tracks_started_blocking_crypto_until_join_after_cancellation() {
        let started = Arc::new(Notify::new());
        let worker_started = started.clone();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_gate = gate.clone();
        let mut runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let blocking_crypto = runtime.blocking_crypto.clone();
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(
            move |worker_cancellation| {
                let started = worker_started.clone();
                let gate = worker_gate.clone();
                let blocking_crypto = blocking_crypto.clone();
                Box::pin(async move {
                    blocking_crypto
                        .run_cancellable(
                            &worker_cancellation,
                            move || {
                                started.notify_one();
                                let (lock, condition) = &*gate;
                                let mut released = lock
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                while !*released {
                                    released = condition
                                        .wait(released)
                                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                                }
                                Ok(())
                            },
                            "injected attachment crypto failed",
                        )
                        .await?;
                    Err(PublicError::unexpected(
                        "cancelled blocking crypto unexpectedly succeeded",
                    ))
                })
            },
        ));
        let lifecycle_owner = runtime.clone();
        let cancellation = OperationCancellation::new();
        let caller_cancellation = cancellation.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment_with_cancellation(
                    UploadTaskAttachmentArgs {
                        work_list_id: Uuid::now_v7(),
                        task_id: Uuid::now_v7(),
                        path: std::path::PathBuf::from("unused-by-injected-workflow"),
                        file_name: None,
                        content_type: None,
                        password: None,
                    },
                    caller_cancellation,
                )
                .await
        });
        started.notified().await;

        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("single-worker heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "blocking encryption/KDF must run outside the Tokio worker"
        );

        caller.abort();
        tokio::time::timeout(Duration::from_secs(1), async {
            while !cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dropped caller cancels blocking crypto");
        let drain_error = lifecycle_owner
            .drain_attachment_uploads(Duration::from_millis(20))
            .await
            .expect_err("started blocking crypto keeps lifecycle active");
        assert!(matches!(
            drain_error,
            PublicError::OutcomeAmbiguous { operation, .. }
                if operation == "attachment upload drain"
        ));

        let (lock, condition) = &*gate;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        condition.notify_one();
        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("blocking crypto joins before lifecycle becomes idle");
        assert!(
            lifecycle_owner
                .take_attachment_upload_failure_reports()
                .is_empty()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn blocked_file_open_keeps_lifecycle_and_admission_owned_after_cancellation() {
        let started = Arc::new(Notify::new());
        let worker_started = started.clone();
        let release = Arc::new(Notify::new());
        let worker_release = release.clone();
        let mut runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let blocking_crypto = runtime.blocking_crypto.clone();
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(
            move |worker_cancellation| {
                let started = worker_started.clone();
                let release = worker_release.clone();
                let blocking_crypto = blocking_crypto.clone();
                Box::pin(async move {
                    let _admission = blocking_crypto
                        .admit_cancellable(&worker_cancellation)
                        .await?;
                    let file_io = tokio::spawn(async move {
                        started.notify_one();
                        release.notified().await;
                        Ok(())
                    });
                    await_started_file_io(file_io, &worker_cancellation).await?;
                    Err(PublicError::unexpected(
                        "cancelled file open unexpectedly succeeded",
                    ))
                })
            },
        ));
        let lifecycle_owner = runtime.clone();
        let cancellation = OperationCancellation::new();
        let caller_cancellation = cancellation.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment_with_cancellation(
                    UploadTaskAttachmentArgs {
                        work_list_id: Uuid::now_v7(),
                        task_id: Uuid::now_v7(),
                        path: std::path::PathBuf::from("blocked-file-open"),
                        file_name: None,
                        content_type: None,
                        password: None,
                    },
                    caller_cancellation,
                )
                .await
        });
        started.notified().await;

        caller.abort();
        tokio::time::timeout(Duration::from_secs(1), async {
            while !cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dropped caller cancels file open");
        assert_eq!(
            lifecycle_owner.blocking_crypto.available_permits(),
            1,
            "file I/O must retain its pre-encryption admission permit"
        );
        lifecycle_owner
            .drain_attachment_uploads(Duration::from_millis(20))
            .await
            .expect_err("started file I/O keeps lifecycle active");

        release.notify_one();
        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("file I/O joins before lifecycle becomes idle");
        assert_eq!(lifecycle_owner.blocking_crypto.available_permits(), 2);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn real_fifo_cancellation_drains_and_restores_all_admission() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let fifo = temporary.path().join("upload.fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo must succeed");

        let started = Arc::new(Notify::new());
        let worker_started = started.clone();
        let directory_path = temporary.path().to_path_buf();
        let mut runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let blocking_crypto = runtime.blocking_crypto.clone();
        let workflow_crypto = blocking_crypto.clone();
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(
            move |worker_cancellation| {
                let started = worker_started.clone();
                let directory_path = directory_path.clone();
                let blocking_crypto = workflow_crypto.clone();
                Box::pin(async move {
                    let _crypto_admission = blocking_crypto
                        .admit_cancellable(&worker_cancellation)
                        .await?;
                    let directory = cap_std::fs::Dir::open_ambient_dir(
                        directory_path,
                        cap_std::ambient_authority(),
                    )
                    .map_err(|err| {
                        PublicError::unexpected(format!("failed to open FIFO test dir: {err}"))
                    })?;
                    let file_io = tokio::spawn(async move {
                        read_upload_file_in(directory, std::path::Path::new("upload.fifo"))
                            .await
                            .map(|_| ())
                    });
                    started.notify_one();
                    await_started_file_io(file_io, &worker_cancellation).await?;
                    Err(PublicError::unexpected(
                        "FIFO unexpectedly passed attachment validation",
                    ))
                })
            },
        ));
        let lifecycle_owner = runtime.clone();
        let cancellation = OperationCancellation::new();
        let caller_cancellation = cancellation.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment_with_cancellation(
                    UploadTaskAttachmentArgs {
                        work_list_id: Uuid::now_v7(),
                        task_id: Uuid::now_v7(),
                        path: std::path::PathBuf::from("upload.fifo"),
                        file_name: None,
                        content_type: None,
                        password: None,
                    },
                    caller_cancellation,
                )
                .await
        });
        started.notified().await;
        cancellation.cancel();

        let error = tokio::time::timeout(Duration::from_secs(1), caller)
            .await
            .expect("FIFO upload cancellation must be bounded")
            .expect("caller task")
            .expect_err("FIFO upload must not succeed");
        assert!(matches!(
            error,
            PublicError::Cancelled(_) | PublicError::Validation(_)
        ));
        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("FIFO lifecycle drains");
        assert_eq!(blocking_crypto.available_permits(), 2);
        assert_eq!(
            lifecycle_owner
                .upload_lifecycle
                .available_admission_permits(),
            4
        );
    }

    #[tokio::test]
    async fn started_blocking_crypto_panic_is_observed_and_sanitized() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let error = runtime
            .blocking_crypto
            .run(
                || -> PublicResult<()> {
                    panic!("blocking crypto panic canary");
                },
                "attachment encryption task failed",
            )
            .await
            .expect_err("panic must be reported");

        assert!(matches!(
            error,
            PublicError::Unexpected(message)
                if message == "attachment encryption task failed: worker panicked"
                    && !message.contains("panic canary")
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn maximum_task_payload_attachment_transforms_keep_the_worker_responsive() {
        const MAX_TASK_PAYLOAD_TEST_BYTES: usize = 8 * 1024 * 1024;

        let admission = BlockingCryptoAdmission::default();
        let list_key = SymmetricKey::new([0x6c; KEY_SIZE]);
        let attachment_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment = build_task_attachment_ref(
            attachment_id,
            "large.txt".to_string(),
            "text/plain".to_string(),
            42,
            vec![0x31; 96],
            Uuid::now_v7(),
        );
        let payload = encrypt_task_payload(
            &build_task_payload_envelope(
                TaskPayloadBody {
                    title: "x".repeat(MAX_TASK_PAYLOAD_TEST_BYTES),
                    rich_text: None,
                    checklist: None,
                    attachments: Some(vec![attachment]),
                    references: None,
                    mentions: None,
                    client_meta: None,
                    recurrence_state: None,
                },
                1,
            ),
            &list_key,
        )
        .expect("maximum task payload");
        let upload_ciphertext = payload.base64.clone();
        let delete_ciphertext = payload.base64;
        let upload_key = list_key.clone();
        let delete_key = list_key;
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let worker_admission = admission.clone();
        let transform = tokio::spawn(async move {
            worker_admission
                .run_with_large_payload(
                    payload_permit,
                    move || {
                        let upload = prepare_attachment_upload_task(upload_ciphertext, upload_key)?;
                        if upload.projected.len() != 1 {
                            return Err(PublicError::unexpected(
                                "maximum upload task projection was incomplete",
                            ));
                        }
                        let delete = prepare_attachment_delete_task(
                            delete_ciphertext,
                            Utc::now(),
                            attachment_id,
                            task_id,
                            delete_key,
                        )?;
                        if delete
                            .update
                            .attachment_ids
                            .as_ref()
                            .is_none_or(|ids| !ids.is_empty())
                        {
                            return Err(PublicError::unexpected(
                                "maximum delete task projection was incomplete",
                            ));
                        }
                        Ok(())
                    },
                    "maximum attachment task-payload transform failed",
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("maximum task transform reaches the blocking pool");

        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("single-worker heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "maximum task payload transforms must not run on the current-thread Tokio worker"
        );
        let (_, ()) = transform
            .await
            .expect("transform task joins")
            .expect("maximum task payload transforms");
        assert_eq!(admission.available_large_payload_permits(), 2);
    }

    #[tokio::test]
    async fn post_initiation_crypto_panic_compensates_with_original_saturated_payload_lease() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("DELETE connection");
            let request = read_http_request_bytes(&mut stream).await;
            write_http_response(&mut stream, "204 No Content", "text/plain", &[]).await;
            String::from_utf8_lossy(&request)
                .lines()
                .next()
                .expect("DELETE request line")
                .to_string()
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let cancellation = OperationCancellation::new();
        let list_key = SymmetricKey::new([0x6a; KEY_SIZE]);
        let preparation_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("preparation payload admission");
        let held_payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("hold second payload admission");
        let mut waiting_payloads = Vec::new();
        for _ in 0..4 {
            let admission = runtime.blocking_crypto.clone();
            waiting_payloads.push(tokio::spawn(async move {
                admission.admit_large_payload().await
            }));
        }
        while runtime.blocking_crypto.large_payload_waiting_count() < 4 {
            tokio::task::yield_now().await;
        }

        let error = prepare_post_initiation_with_compensation(
            &runtime.blocking_crypto,
            preparation_permit,
            &mut client,
            &cancellation,
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            || -> PublicResult<PreparedPostInitiationUpload> {
                panic!("post-initiation secret panic canary");
            },
        )
        .await
        .expect_err("post-initiation panic must fail after compensation");

        assert!(matches!(
            error,
            PublicError::Unexpected(message)
                if message == "attachment task-payload preparation failed: worker panicked"
                    && !message.contains("panic canary")
        ));
        let request_line = server.await.expect("server");
        assert!(
            request_line.starts_with(&format!(
                "DELETE /work-lists/{work_list_id}/attachments/{attachment_id} "
            )),
            "known post-initiation attachment must be cleaned immediately: {request_line}"
        );
        assert_eq!(
            runtime.blocking_crypto.available_large_payload_permits(),
            0,
            "the held lease and saturated queue must remain admitted through compensation"
        );

        for waiting in waiting_payloads {
            waiting.abort();
            if let Ok(Ok(permit)) = waiting.await {
                drop(permit);
            }
        }
        drop(held_payload_permit);
        assert_eq!(
            runtime.blocking_crypto.available_large_payload_permits(),
            2,
            "compensation and cancelled waiters must release every payload lease"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn post_initiation_blocking_admission_rejection_still_compensates() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("DELETE connection");
            let request = read_http_request_bytes(&mut stream).await;
            write_http_response(&mut stream, "204 No Content", "text/plain", &[]).await;
            String::from_utf8_lossy(&request)
                .lines()
                .next()
                .expect("DELETE request line")
                .to_string()
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("payload admission");
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let mut blocking_tasks = Vec::new();
        for _ in 0..10 {
            let admission = runtime.blocking_crypto.clone();
            let worker_gate = gate.clone();
            blocking_tasks.push(tokio::spawn(async move {
                admission
                    .run(
                        move || {
                            let (lock, condition) = &*worker_gate;
                            let mut released = lock
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            while !*released {
                                released = condition
                                    .wait(released)
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                            }
                            Ok(())
                        },
                        "saturation task failed",
                    )
                    .await
            }));
        }
        while runtime.blocking_crypto.waiting_count() < 8 {
            tokio::task::yield_now().await;
        }
        let prepare_ran = Arc::new(AtomicBool::new(false));
        let prepare_ran_in_worker = prepare_ran.clone();
        let list_key = SymmetricKey::new([0x6b; KEY_SIZE]);

        let error = prepare_post_initiation_with_compensation(
            &runtime.blocking_crypto,
            payload_permit,
            &mut client,
            &OperationCancellation::new(),
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            move || {
                prepare_ran_in_worker.store(true, Ordering::Release);
                Err(PublicError::unexpected(
                    "preparation must not run after admission rejection",
                ))
            },
        )
        .await
        .expect_err("admission rejection must fail after compensation");

        assert!(matches!(error, PublicError::RateLimited(_)));
        assert!(!prepare_ran.load(Ordering::Acquire));
        let request_line = server.await.expect("server");
        assert!(request_line.starts_with(&format!(
            "DELETE /work-lists/{work_list_id}/attachments/{attachment_id} "
        )));

        let (lock, condition) = &*gate;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        condition.notify_all();
        for task in blocking_tasks {
            task.await
                .expect("saturation task joins")
                .expect("saturation task completes");
        }
        assert_eq!(
            runtime.blocking_crypto.available_large_payload_permits(),
            2,
            "admission rejection compensation must release the original payload lease"
        );
    }

    #[tokio::test]
    async fn post_initiation_work_error_compensates_with_original_permit() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("DELETE connection");
            let request = read_http_request_bytes(&mut stream).await;
            write_http_response(&mut stream, "204 No Content", "text/plain", &[]).await;
            String::from_utf8_lossy(&request)
                .lines()
                .next()
                .expect("DELETE request line")
                .to_string()
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("payload admission");
        let list_key = SymmetricKey::new([0x6c; KEY_SIZE]);

        let error = prepare_post_initiation_with_compensation(
            &runtime.blocking_crypto,
            payload_permit,
            &mut client,
            &OperationCancellation::new(),
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            || -> PublicResult<PreparedPostInitiationUpload> {
                Err(PublicError::crypto("injected preparation failure"))
            },
        )
        .await
        .expect_err("work failure must fail after compensation");

        assert!(
            matches!(error, PublicError::Crypto(message) if message == "injected preparation failure")
        );
        let request_line = server.await.expect("server");
        assert!(request_line.starts_with(&format!(
            "DELETE /work-lists/{work_list_id}/attachments/{attachment_id} "
        )));
        assert_eq!(
            runtime.blocking_crypto.available_large_payload_permits(),
            2,
            "work failure compensation must release the original payload lease"
        );
    }

    #[tokio::test]
    async fn dropping_public_upload_awaiter_keeps_owned_cleanup_alive() {
        let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("API listener");
        let api_url = format!("http://{}", api_listener.local_addr().expect("API address"));
        let storage_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("storage listener");
        let storage_origin = format!(
            "http://{}",
            storage_listener.local_addr().expect("storage address")
        );
        let put_blocked = Arc::new(Notify::new());
        let server_put_blocked = put_blocked.clone();
        let storage_server = tokio::spawn(async move {
            let (mut stream, _) = storage_listener.accept().await.expect("PUT connection");
            read_http_request(&mut stream).await;
            server_put_blocked.notify_one();
            tokio::time::sleep(Duration::from_millis(50)).await;
            write_http_response(&mut stream, "200 OK", "text/plain", &[]).await;
        });
        let delete_seen = Arc::new(AtomicBool::new(false));
        let server_delete_seen = delete_seen.clone();
        let api_server = tokio::spawn(async move {
            let (mut stream, _) = api_listener.accept().await.expect("DELETE connection");
            read_http_request(&mut stream).await;
            server_delete_seen.store(true, Ordering::Release);
            write_http_response(&mut stream, "204 No Content", "text/plain", &[]).await;
        });

        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let attachment_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x19; KEY_SIZE]);
        let workflow_api_url = api_url.clone();
        let workflow_storage_origin = storage_origin.clone();
        let workflow_storage_policy =
            crate::storage::StorageTransferPolicy::new(&api_url, [&storage_origin])
                .expect("storage policy");
        let workflow_list_key = list_key.clone();
        let membership_id = Uuid::now_v7();
        let mut runtime = RuntimeClient::new(&api_url).expect("runtime");
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(
            move |worker_cancellation| {
                let api_url = workflow_api_url.clone();
                let storage_origin = workflow_storage_origin.clone();
                let storage_policy = workflow_storage_policy.clone();
                let list_key = workflow_list_key.clone();
                Box::pin(async move {
                    let mut client =
                        PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                            .expect("API client");
                    let initiated = InitiateAttachmentUploadResponse {
                        attachment_id,
                        upload_url: format!("{storage_origin}/blocked"),
                        upload_headers: std::collections::HashMap::new(),
                        expires_at: Utc::now() + chrono::Duration::minutes(1),
                    };
                    let projected_attachment = build_projected_attachment(
                        &initiated,
                        &SymmetricKey::new([0x29; KEY_SIZE]),
                        &list_key,
                        membership_id,
                        4,
                        UploadAttachmentMetadata {
                            file_name: "attachment.txt".to_string(),
                            content_type: "text/plain".to_string(),
                            plaintext_bytes: 4,
                        },
                    )
                    .expect("projected attachment");
                    let update = UpdateTaskRequest::default();
                    finish_upload_after_initiation(
                        &storage_policy,
                        &mut client,
                        vec![1_u8; 4],
                        PostInitiationRequest {
                            work_list_id,
                            task_id,
                            initiated: &initiated,
                            ciphertext_bytes: 4,
                            update: &update,
                        },
                        FailedUploadContext {
                            work_list_id,
                            task_id,
                            attachment_id,
                            list_key: &list_key,
                        },
                        projected_attachment,
                        &worker_cancellation,
                    )
                    .await
                })
            },
        ));
        let lifecycle_owner = runtime.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment(UploadTaskAttachmentArgs {
                    work_list_id,
                    task_id,
                    path: std::path::PathBuf::from("unused-by-injected-workflow"),
                    file_name: None,
                    content_type: None,
                    password: None,
                })
                .await
        });
        put_blocked.notified().await;
        caller.abort();

        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("bounded lifecycle drain");
        assert!(delete_seen.load(Ordering::Acquire));
        api_server.await.expect("API server");
        assert!(
            lifecycle_owner
                .take_attachment_upload_failure_reports()
                .is_empty()
        );
        storage_server.abort();
    }

    #[tokio::test]
    async fn lifecycle_owner_reports_ambiguous_cleanup_after_upload_awaiter_is_dropped() {
        let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("API listener");
        let api_url = format!("http://{}", api_listener.local_addr().expect("API address"));
        let storage_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("storage listener");
        let storage_origin = format!(
            "http://{}",
            storage_listener.local_addr().expect("storage address")
        );
        let put_blocked = Arc::new(Notify::new());
        let server_put_blocked = put_blocked.clone();
        let storage_server = tokio::spawn(async move {
            let (_stream, _) = storage_listener.accept().await.expect("PUT connection");
            server_put_blocked.notify_one();
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        let api_server = tokio::spawn(async move {
            let (mut stream, _) = api_listener.accept().await.expect("DELETE connection");
            read_http_request(&mut stream).await;
            write_http_response(
                &mut stream,
                "500 Internal Server Error",
                "application/json",
                br#"{"error":"cleanup_failed","message":"server-secret-detail"}"#,
            )
            .await;
        });

        let storage_policy =
            crate::storage::StorageTransferPolicy::new(&api_url, [&storage_origin])
                .expect("storage policy");
        let mut runtime = RuntimeClient::new(&api_url).expect("runtime");
        runtime.upload_test_workflow =
            Some(post_initiation_workflow(PostInitiationWorkflowFixture {
                api_url: api_url.clone(),
                storage_origin,
                storage_policy,
                list_key: SymmetricKey::new([0x38; KEY_SIZE]),
                work_list_id: Uuid::now_v7(),
                task_id: Uuid::now_v7(),
                attachment_id: Uuid::now_v7(),
                membership_id: Uuid::now_v7(),
            }));
        let lifecycle_owner = runtime.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment(UploadTaskAttachmentArgs {
                    work_list_id: Uuid::now_v7(),
                    task_id: Uuid::now_v7(),
                    path: std::path::PathBuf::from("unused-by-injected-workflow"),
                    file_name: None,
                    content_type: None,
                    password: None,
                })
                .await
        });
        put_blocked.notified().await;
        caller.abort();

        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("owner drains failed cleanup worker");
        api_server.await.expect("API server");
        let reports = lifecycle_owner.take_attachment_upload_failure_reports();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].code, "outcome_ambiguous");
        assert_eq!(
            reports[0].message,
            "attachment upload outcome could not be established"
        );
        assert!(!reports[0].message.contains("server-secret-detail"));
        storage_server.abort();
    }

    #[tokio::test]
    async fn dropped_awaiter_cancels_stalled_preinit_work_without_initiation() {
        let preinit_started = Arc::new(Notify::new());
        let worker_preinit_started = preinit_started.clone();
        let initiation_seen = Arc::new(AtomicBool::new(false));
        let worker_initiation_seen = initiation_seen.clone();
        let worker_finished = Arc::new(AtomicBool::new(false));
        let finished = worker_finished.clone();
        let mut runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(
            move |worker_cancellation| {
                let preinit_started = worker_preinit_started.clone();
                let initiation_seen = worker_initiation_seen.clone();
                let finished = finished.clone();
                Box::pin(async move {
                    preinit_started.notify_one();
                    let result: PublicResult<()> = cancel_safe_await(&worker_cancellation, async {
                        std::future::pending::<PublicResult<()>>().await
                    })
                    .await;
                    if result.is_ok() {
                        initiation_seen.store(true, Ordering::Release);
                    }
                    finished.store(true, Ordering::Release);
                    result?;
                    Err(PublicError::unexpected("unreachable"))
                })
            },
        ));
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment(UploadTaskAttachmentArgs {
                    work_list_id: Uuid::now_v7(),
                    task_id: Uuid::now_v7(),
                    path: std::path::PathBuf::from("unused-by-injected-workflow"),
                    file_name: None,
                    content_type: None,
                    password: None,
                })
                .await
        });
        preinit_started.notified().await;
        caller.abort();

        tokio::time::timeout(Duration::from_secs(1), async {
            while !worker_finished.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("pre-init cancellation");
        assert!(!initiation_seen.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn lifecycle_owner_sanitizes_and_records_upload_worker_panics() {
        let initiated = Arc::new(Notify::new());
        let worker_initiated = initiated.clone();
        let mut runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(move |_| {
            let initiated = worker_initiated.clone();
            Box::pin(async move {
                initiated.notify_one();
                panic!("panic payload must never enter lifecycle reports");
            })
        }));
        let lifecycle_owner = runtime.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment(UploadTaskAttachmentArgs {
                    work_list_id: Uuid::now_v7(),
                    task_id: Uuid::now_v7(),
                    path: std::path::PathBuf::from("unused-by-injected-workflow"),
                    file_name: None,
                    content_type: None,
                    password: None,
                })
                .await
        });
        initiated.notified().await;
        let error = caller
            .await
            .expect("caller task")
            .expect_err("worker panic must be reported");

        assert!(matches!(
            error,
            PublicError::Unexpected(message)
                if message == "attachment upload worker panicked"
                    && !message.contains("panic payload")
        ));
        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("panic supervisor drained");
        let reports = lifecycle_owner.take_attachment_upload_failure_reports();
        assert_eq!(
            reports,
            vec![crate::AttachmentUploadFailureReport {
                code: "worker_panicked",
                message: "attachment upload worker panicked",
            }]
        );
    }

    #[tokio::test]
    async fn bounded_lifecycle_drain_reports_timeout_then_allows_later_completion() {
        let started = Arc::new(Notify::new());
        let worker_started = started.clone();
        let mut runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        runtime.upload_test_workflow = Some(crate::client::TestUploadWorkflow::new(move |_| {
            let started = worker_started.clone();
            Box::pin(async move {
                started.notify_one();
                tokio::time::sleep(Duration::from_millis(150)).await;
                Err(PublicError::cancelled("attachment upload cancelled"))
            })
        }));
        let lifecycle_owner = runtime.clone();
        let caller = tokio::spawn(async move {
            runtime
                .upload_task_attachment(UploadTaskAttachmentArgs {
                    work_list_id: Uuid::now_v7(),
                    task_id: Uuid::now_v7(),
                    path: std::path::PathBuf::from("unused-by-injected-workflow"),
                    file_name: None,
                    content_type: None,
                    password: Some(
                        crate::AttachmentUploadPassword::new("injected password")
                            .expect("password"),
                    ),
                })
                .await
        });
        started.notified().await;
        caller.abort();

        let error = lifecycle_owner
            .drain_attachment_uploads(Duration::from_millis(20))
            .await
            .expect_err("drain must report its bound");
        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { operation, details }
                if operation == "attachment upload drain"
                    && details.contains("backend orphan cleanup")
        ));
        lifecycle_owner
            .drain_attachment_uploads(Duration::from_secs(1))
            .await
            .expect("worker eventually drains");
        assert!(
            lifecycle_owner
                .take_attachment_upload_failure_reports()
                .is_empty()
        );
    }

    fn test_credentials(api_url: &str) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + chrono::Duration::hours(1),
            refresh_expires_at: Utc::now() + chrono::Duration::hours(2),
            user_id: Uuid::now_v7(),
            email: "agent@example.com".to_string(),
            data_key_ciphertext: "unused".to_string(),
        }
    }

    async fn read_http_request_bytes(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).await.expect("request bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let Some(header_end) = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4)
        else {
            return request;
        };
        let content_length = std::str::from_utf8(&request[..header_end])
            .ok()
            .and_then(|headers| {
                headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = stream.read(&mut buffer).await.expect("request body bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        request
    }

    async fn read_http_request(stream: &mut tokio::net::TcpStream) {
        let _ = read_http_request_bytes(stream).await;
    }

    async fn write_http_response(
        stream: &mut tokio::net::TcpStream,
        status: &str,
        content_type: &str,
        body: &[u8],
    ) {
        let headers = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(headers.as_bytes())
            .await
            .expect("response headers");
        stream.write_all(body).await.expect("response body");
    }

    fn encrypted_task_detail(
        list_key: &SymmetricKey,
        work_list_id: Uuid,
        task_id: Uuid,
        membership_id: Uuid,
        attachments: Vec<sealtask_client_crypto::FlexibleValue>,
        updated_at: chrono::DateTime<Utc>,
    ) -> TaskDetailResponse {
        let payload = encrypt_task_payload(
            &build_task_payload_envelope(
                TaskPayloadBody {
                    title: "Task".to_string(),
                    rich_text: None,
                    checklist: None,
                    attachments: Some(attachments),
                    references: None,
                    mentions: None,
                    client_meta: None,
                    recurrence_state: None,
                },
                1,
            ),
            list_key,
        )
        .expect("task payload");
        TaskDetailResponse {
            task: TaskResponse {
                id: task_id,
                work_list_id,
                created_by_membership_id: membership_id,
                title_ciphertext: payload.base64.clone(),
                payload_ciphertext: payload.base64,
                section_id: None,
                priority: None,
                position: "1".to_string(),
                due_at: None,
                start_at: None,
                completed_at: None,
                archived_at: None,
                is_completed: false,
                recurrence_id: None,
                recurrence_schedule: None,
                recurrence_iteration: None,
                materialized_at: None,
                created_at: updated_at,
                updated_at,
                comment_count: 0,
                delegations: Vec::new(),
            },
            comments: Vec::new(),
        }
    }

    #[tokio::test]
    async fn attachment_delete_reconciles_lost_and_malformed_update_responses() {
        for malformed in [false, true] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let work_list_id = Uuid::now_v7();
            let task_id = Uuid::now_v7();
            let attachment_id = Uuid::now_v7();
            let list_key = SymmetricKey::new([0x21; KEY_SIZE]);
            let updated_at = Utc::now();
            let reconciled = encrypted_task_detail(
                &list_key,
                work_list_id,
                task_id,
                Uuid::now_v7(),
                Vec::new(),
                updated_at + chrono::TimeDelta::seconds(1),
            );
            let response = serde_json::to_vec(&reconciled).expect("task JSON");
            let server = tokio::spawn(async move {
                let (mut mutation, _) = listener.accept().await.expect("mutation connection");
                read_http_request(&mut mutation).await;
                if malformed {
                    write_http_response(&mut mutation, "200 OK", "application/json", b"{").await;
                }
                drop(mutation);

                let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
                read_http_request(&mut reconcile).await;
                write_http_response(&mut reconcile, "200 OK", "application/json", &response).await;
            });
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let primary = client
                .update_task(work_list_id, task_id, &UpdateTaskRequest::default())
                .await
                .expect_err("ambiguous update response");
            let admission = BlockingCryptoAdmission::default();
            let permit = admission
                .admit_large_payload()
                .await
                .expect("payload admission");
            reconcile_deleted_task_attachment(
                &mut client,
                work_list_id,
                task_id,
                attachment_id,
                updated_at,
                &list_key,
                primary,
                Duration::from_secs(1),
                &admission,
                permit,
            )
            .await
            .expect("reconciled attachment delete");
            server.await.expect("server");
        }
    }

    async fn attachment_delete_reconciliation_error(
        status: &'static str,
        response: Vec<u8>,
        list_key: &SymmetricKey,
        primary: PublicError,
    ) -> PublicError {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            read_http_request(&mut stream).await;
            write_http_response(&mut stream, status, "application/json", &response).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let admission = BlockingCryptoAdmission::default();
        let permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let error = reconcile_deleted_task_attachment(
            &mut client,
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Utc::now(),
            list_key,
            primary,
            Duration::from_secs(1),
            &admission,
            permit,
        )
        .await
        .expect_err("reconciliation must remain ambiguous");
        server.await.expect("server");
        error
    }

    #[tokio::test]
    async fn attachment_delete_reconciliation_retains_api_decrypt_and_projection_causes() {
        let list_key = SymmetricKey::new([0x23; KEY_SIZE]);
        let api_read = attachment_delete_reconciliation_error(
            "500 Internal Server Error",
            br#"{"error":"read_failed","message":"sensitive server body"}"#.to_vec(),
            &list_key,
            PublicError::unexpected("task update response lost"),
        )
        .await;
        assert!(matches!(
            api_read,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("primary=api_mutation")
                    && details.contains("reconciliation=api_read")
                    && !details.contains("sensitive server body")
        ));

        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let encrypted_with_wrong_key = encrypted_task_detail(
            &SymmetricKey::new([0x24; KEY_SIZE]),
            work_list_id,
            task_id,
            Uuid::now_v7(),
            Vec::new(),
            Utc::now(),
        );
        let decrypt = attachment_delete_reconciliation_error(
            "200 OK",
            serde_json::to_vec(&encrypted_with_wrong_key).expect("task JSON"),
            &list_key,
            PublicError::transport(sealtask_client_core::TransportFailureKind::Timeout),
        )
        .await;
        assert!(matches!(
            decrypt,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("primary=transport_timeout")
                    && details.contains("reconciliation=decrypt")
                    && !details.contains("task mutation")
        ));

        let malformed_attachment = encrypted_task_detail(
            &list_key,
            work_list_id,
            task_id,
            Uuid::now_v7(),
            vec![sealtask_client_crypto::FlexibleValue::Map(Vec::new())],
            Utc::now(),
        );
        let projection = attachment_delete_reconciliation_error(
            "200 OK",
            serde_json::to_vec(&malformed_attachment).expect("task JSON"),
            &list_key,
            PublicError::unexpected("task update response lost"),
        )
        .await;
        assert!(matches!(
            projection,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("primary=api_mutation")
                    && details.contains("reconciliation=projection")
        ));
    }

    #[tokio::test]
    async fn stalled_attachment_delete_reconciliation_is_bounded() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("connection");
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let admission = BlockingCryptoAdmission::default();
        let permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let error = reconcile_deleted_task_attachment(
            &mut client,
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Utc::now(),
            &SymmetricKey::new([0x22; KEY_SIZE]),
            PublicError::unexpected("task update response lost"),
            Duration::from_millis(30),
            &admission,
            permit,
        )
        .await
        .expect_err("ambiguous stalled reconciliation");

        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("primary=api_mutation")
                    && details.contains("reconciliation=timeout")
        ));
        server.abort();
    }

    #[tokio::test]
    async fn ambiguous_cleanup_failure_reconciles_and_preserves_sanitized_categories() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            read_http_request(&mut stream).await;
            write_http_response(
                &mut stream,
                "500 Internal Server Error",
                "application/json",
                br#"{"error":"cleanup_failed","message":"storage unavailable"}"#,
            )
            .await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let attachment_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x11; KEY_SIZE]);
        let error = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id: Uuid::now_v7(),
                task_id: Uuid::now_v7(),
                attachment_id,
                list_key: &list_key,
            },
            PublicError::unexpected("task update response lost"),
            Duration::from_secs(1),
        )
        .await
        .expect_err("cleanup failure");
        assert_eq!(error.code(), "outcome_ambiguous");
        assert!(matches!(
            &error,
            PublicError::OutcomeAmbiguous { operation, details }
                if operation == "attachment upload cleanup"
                    && details.contains("primary=api_mutation")
                    && details.contains("cleanup=http_server_error")
                    && details.contains("reconciliation=api_read")
        ));
        assert_eq!(
            crate::upload_lifecycle::failure_report(&error)
                .expect("operator failure report")
                .message,
            "attachment upload outcome could not be established"
        );
        server.await.expect("server");
    }

    #[tokio::test]
    async fn stalled_cleanup_returns_within_independent_bound() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("connection");
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let attachment_id = Uuid::now_v7();
        let started = tokio::time::Instant::now();
        let list_key = SymmetricKey::new([0x12; KEY_SIZE]);
        let error = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id: Uuid::now_v7(),
                task_id: Uuid::now_v7(),
                attachment_id,
                list_key: &list_key,
            },
            PublicError::unexpected("upload failed"),
            Duration::from_millis(50),
        )
        .await
        .expect_err("cleanup timeout");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(error.code(), "outcome_ambiguous");
        assert!(matches!(
            &error,
            PublicError::OutcomeAmbiguous { operation, details }
                if operation == "attachment upload cleanup"
                    && details.contains("primary=api_mutation")
                    && details.contains("cleanup=timeout")
                    && details.contains("reconciliation=timeout")
        ));
        server.abort();
    }

    #[tokio::test]
    async fn definitive_cleanup_rejection_remains_a_compensation_failure() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            read_http_request(&mut stream).await;
            write_http_response(
                &mut stream,
                "422 Unprocessable Entity",
                "application/json",
                br#"{"error":"invalid_cleanup","message":"cleanup was rejected"}"#,
            )
            .await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let list_key = SymmetricKey::new([0x6d; KEY_SIZE]);
        let error = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id: Uuid::now_v7(),
                task_id: Uuid::now_v7(),
                attachment_id: Uuid::now_v7(),
                list_key: &list_key,
            },
            PublicError::unexpected("upload failed"),
            Duration::from_secs(1),
        )
        .await
        .expect_err("definitive cleanup rejection must preserve compensation failure");

        assert_eq!(error.code(), "compensation_failed");
        assert!(matches!(
            &error,
            PublicError::CompensationFailed { cleanup, .. }
                if cleanup == "cleanup=validation"
        ));
        assert_eq!(
            crate::upload_lifecycle::failure_report(&error)
                .expect("operator failure report")
                .message,
            "attachment upload failed and cleanup did not complete"
        );
        server.await.expect("server");
    }

    #[tokio::test]
    async fn cleanup_not_found_with_absent_task_reference_returns_primary_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let attachment_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x6e; KEY_SIZE]);
        let task = encrypted_task_detail(
            &list_key,
            work_list_id,
            task_id,
            membership_id,
            Vec::new(),
            Utc::now(),
        );
        let task_body = serde_json::to_vec(&task).expect("task JSON");
        let server = tokio::spawn(async move {
            let (mut cleanup, _) = listener.accept().await.expect("cleanup connection");
            read_http_request(&mut cleanup).await;
            write_http_response(
                &mut cleanup,
                "404 Not Found",
                "application/json",
                br#"{"error":"not_found","message":"attachment not found"}"#,
            )
            .await;

            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            read_http_request(&mut reconcile).await;
            write_http_response(&mut reconcile, "200 OK", "application/json", &task_body).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            PublicError::validation("task update was rejected"),
            Duration::from_secs(1),
        )
        .await
        .expect_err("the original workflow error must be restored");

        assert!(matches!(
            error,
            PublicError::Validation(message) if message == "task update was rejected"
        ));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn cleanup_not_found_with_lost_task_access_is_outcome_ambiguous() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut cleanup, _) = listener.accept().await.expect("cleanup connection");
            read_http_request(&mut cleanup).await;
            write_http_response(
                &mut cleanup,
                "404 Not Found",
                "application/json",
                br#"{"error":"not_found","message":"sensitive attachment detail"}"#,
            )
            .await;

            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            read_http_request(&mut reconcile).await;
            write_http_response(
                &mut reconcile,
                "404 Not Found",
                "application/json",
                br#"{"error":"not_found","message":"sensitive project detail"}"#,
            )
            .await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let list_key = SymmetricKey::new([0x6f; KEY_SIZE]);
        let error = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id: Uuid::now_v7(),
                task_id: Uuid::now_v7(),
                attachment_id: Uuid::now_v7(),
                list_key: &list_key,
            },
            PublicError::unexpected("task update response lost"),
            Duration::from_secs(1),
        )
        .await
        .expect_err("lost access cannot prove whether cleanup completed");

        assert!(matches!(
            &error,
            PublicError::OutcomeAmbiguous { operation, details }
                if operation == "attachment upload cleanup"
                    && details.contains("primary=api_mutation")
                    && details.contains("cleanup=not_found")
                    && details.contains("reconciliation=api_read")
                    && details.contains("inspect the task after access is restored")
                    && !details.contains("sensitive")
        ));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn cleanup_conflict_reconciles_committed_task_update_as_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let attachment_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x13; KEY_SIZE]);
        let attachment = build_task_attachment_ref(
            attachment_id,
            "attachment.txt".to_string(),
            "text/plain".to_string(),
            4,
            vec![1],
            membership_id,
        );
        let payload = encrypt_task_payload(
            &build_task_payload_envelope(
                TaskPayloadBody {
                    title: "Task".to_string(),
                    rich_text: None,
                    checklist: None,
                    attachments: Some(vec![attachment]),
                    references: None,
                    mentions: None,
                    client_meta: None,
                    recurrence_state: None,
                },
                1,
            ),
            &list_key,
        )
        .expect("task payload");
        let now = Utc::now();
        let task = TaskDetailResponse {
            task: TaskResponse {
                id: task_id,
                work_list_id,
                created_by_membership_id: membership_id,
                title_ciphertext: payload.base64.clone(),
                payload_ciphertext: payload.base64,
                section_id: None,
                priority: None,
                position: "1".to_string(),
                due_at: None,
                start_at: None,
                completed_at: None,
                archived_at: None,
                is_completed: false,
                recurrence_id: None,
                recurrence_schedule: None,
                recurrence_iteration: None,
                materialized_at: None,
                created_at: now,
                updated_at: now,
                comment_count: 0,
                delegations: Vec::new(),
            },
            comments: Vec::new(),
        };
        let task_body = serde_json::to_vec(&task).expect("task JSON");
        let server = tokio::spawn(async move {
            let (mut cleanup, _) = listener.accept().await.expect("cleanup connection");
            read_http_request(&mut cleanup).await;
            write_http_response(
                &mut cleanup,
                "409 Conflict",
                "application/json",
                br#"{"error":"conflict","message":"attachment is linked"}"#,
            )
            .await;

            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            read_http_request(&mut reconcile).await;
            write_http_response(&mut reconcile, "200 OK", "application/json", &task_body).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let reconciled = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            PublicError::unexpected("task update response lost"),
            Duration::from_secs(1),
        )
        .await
        .expect("committed upload");
        assert_eq!(reconciled.id, attachment_id);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn ambiguous_cleanup_error_reconciles_committed_task_update_as_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let attachment_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x14; KEY_SIZE]);
        let attachment = build_task_attachment_ref(
            attachment_id,
            "attachment.txt".to_string(),
            "text/plain".to_string(),
            4,
            vec![1],
            membership_id,
        );
        let task = encrypted_task_detail(
            &list_key,
            work_list_id,
            task_id,
            membership_id,
            vec![attachment],
            Utc::now(),
        );
        let task_body = serde_json::to_vec(&task).expect("task JSON");
        let server = tokio::spawn(async move {
            let (mut cleanup, _) = listener.accept().await.expect("cleanup connection");
            read_http_request(&mut cleanup).await;
            write_http_response(
                &mut cleanup,
                "500 Internal Server Error",
                "application/json",
                br#"{"error":"cleanup_failed","message":"provider detail"}"#,
            )
            .await;

            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            read_http_request(&mut reconcile).await;
            write_http_response(&mut reconcile, "200 OK", "application/json", &task_body).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let reconciled = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            PublicError::unexpected("task update response lost"),
            Duration::from_secs(1),
        )
        .await
        .expect("committed upload");

        assert_eq!(reconciled.id, attachment_id);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn cleanup_timeout_reconciles_committed_task_update_with_a_fresh_bound() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let attachment_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let list_key = SymmetricKey::new([0x15; KEY_SIZE]);
        let attachment = build_task_attachment_ref(
            attachment_id,
            "attachment.txt".to_string(),
            "text/plain".to_string(),
            4,
            vec![1],
            membership_id,
        );
        let task = encrypted_task_detail(
            &list_key,
            work_list_id,
            task_id,
            membership_id,
            vec![attachment],
            Utc::now(),
        );
        let task_body = serde_json::to_vec(&task).expect("task JSON");
        let server = tokio::spawn(async move {
            let (mut cleanup, _) = listener.accept().await.expect("cleanup connection");
            read_http_request(&mut cleanup).await;
            let stalled_cleanup = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                drop(cleanup);
            });

            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            read_http_request(&mut reconcile).await;
            write_http_response(&mut reconcile, "200 OK", "application/json", &task_body).await;
            stalled_cleanup.abort();
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let reconciled = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            PublicError::unexpected("task update response lost"),
            Duration::from_millis(50),
        )
        .await
        .expect("committed upload");

        assert_eq!(reconciled.id, attachment_id);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn cleanup_conflict_and_stalled_reconciliation_use_independent_bounds() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut cleanup, _) = listener.accept().await.expect("cleanup connection");
            read_http_request(&mut cleanup).await;
            write_http_response(
                &mut cleanup,
                "409 Conflict",
                "application/json",
                br#"{"error":"conflict","message":"attachment is linked"}"#,
            )
            .await;

            let (_reconcile, _) = listener.accept().await.expect("reconcile connection");
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let list_key = SymmetricKey::new([0x15; KEY_SIZE]);
        let started = tokio::time::Instant::now();
        let error = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id: Uuid::now_v7(),
                task_id: Uuid::now_v7(),
                attachment_id: Uuid::now_v7(),
                list_key: &list_key,
            },
            PublicError::unexpected("task update response lost"),
            Duration::from_millis(50),
        )
        .await
        .expect_err("ambiguous reconciliation timeout");

        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("primary=api_mutation")
                    && details.contains("reconciliation=timeout")
        ));
        server.abort();
    }

    #[tokio::test]
    async fn cancellation_during_put_waits_for_terminal_put_before_delete() {
        let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("API listener");
        let api_url = format!("http://{}", api_listener.local_addr().expect("API address"));
        let storage_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("storage listener");
        let storage_origin = format!(
            "http://{}",
            storage_listener.local_addr().expect("storage address")
        );
        let put_started = Arc::new(Notify::new());
        let server_put_started = put_started.clone();
        let release_put = Arc::new(Notify::new());
        let server_release_put = release_put.clone();
        let storage_server = tokio::spawn(async move {
            let (mut stream, _) = storage_listener.accept().await.expect("PUT connection");
            read_http_request(&mut stream).await;
            server_put_started.notify_one();
            server_release_put.notified().await;
            write_http_response(&mut stream, "200 OK", "text/plain", &[]).await;
        });
        let delete_seen = Arc::new(AtomicBool::new(false));
        let server_delete_seen = delete_seen.clone();
        let api_server = tokio::spawn(async move {
            let (mut stream, _) = api_listener.accept().await.expect("DELETE connection");
            read_http_request(&mut stream).await;
            server_delete_seen.store(true, Ordering::Release);
            write_http_response(&mut stream, "204 No Content", "text/plain", &[]).await;
        });

        let storage_policy =
            crate::storage::StorageTransferPolicy::new(&api_url, [&storage_origin])
                .expect("storage policy");
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let attachment_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let initiated = InitiateAttachmentUploadResponse {
            attachment_id,
            upload_url: format!("{storage_origin}/blocked"),
            upload_headers: std::collections::HashMap::new(),
            expires_at: Utc::now() + chrono::Duration::minutes(1),
        };
        let cancellation = OperationCancellation::new();
        let cancel_from_signal = cancellation.clone();
        let canceller_delete_seen = delete_seen.clone();
        let canceller = tokio::spawn(async move {
            put_started.notified().await;
            cancel_from_signal.cancel();
            tokio::task::yield_now().await;
            assert!(
                !canceller_delete_seen.load(Ordering::Acquire),
                "compensation must not race a still-running PUT"
            );
            release_put.notify_one();
        });
        let update = UpdateTaskRequest::default();
        let primary = perform_upload_after_initiation(
            &storage_policy,
            &mut client,
            vec![1_u8; 4],
            PostInitiationRequest {
                work_list_id,
                task_id,
                initiated: &initiated,
                ciphertext_bytes: 4,
                update: &update,
            },
            &cancellation,
        )
        .await
        .expect_err("cancellation checkpoint after PUT");
        let list_key = SymmetricKey::new([0x14; KEY_SIZE]);
        let result = compensate_failed_upload_with_timeout(
            &mut client,
            FailedUploadContext {
                work_list_id,
                task_id,
                attachment_id,
                list_key: &list_key,
            },
            primary,
            Duration::from_secs(1),
        )
        .await
        .expect_err("cancelled upload");

        assert!(matches!(result, PublicError::Cancelled(_)));
        assert!(delete_seen.load(Ordering::Acquire));
        canceller.await.expect("canceller");
        api_server.await.expect("API server");
        storage_server.await.expect("storage server");
    }
}
