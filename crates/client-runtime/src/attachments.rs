#[cfg(test)]
use crate::attachment_rendering::DOCX_CONTENT_TYPE;
use crate::attachment_rendering::{
    AttachmentReadStrategy, build_readable_attachment, unsupported_attachment_read_error,
};
use crate::blocking_crypto::{BlockingCryptoAdmission, LargePayloadPermit};
use crate::client::RuntimeClient;
use crate::models::{AgentAttachment, AgentTaskSummary, DownloadedAttachment, ReadableAttachment};
use crate::projections::read_error_to_public_error;
use crate::storage::StorageTransferPolicy;
use sealtask_client_api::DownloadAttachmentResponse;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    AttachmentBlobRef, decode_attachment_blob_key, decrypt_attachment_bytes,
};
use uuid::Uuid;

#[derive(Debug)]
struct ResolvedTaskAttachmentDownload {
    attachment: AgentAttachment,
    blob_ref: AttachmentBlobRef,
    download: DownloadAttachmentResponse,
}

impl AgentAttachment {
    fn blob_key(&self) -> &[u8] {
        &self.blob_key
    }
}

impl RuntimeClient {
    pub async fn read_task_attachment(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<ReadableAttachment> {
        let (payload_permit, resolved) = self
            .resolve_task_attachment_download(work_list_id, task_id, attachment_id, password_stdin)
            .await?;
        let read_strategy = resolved.attachment.read_strategy();
        if let AttachmentReadStrategy::Unsupported = read_strategy {
            return Err(unsupported_attachment_read_error(
                &resolved.attachment.file_name,
            ));
        }
        download_decrypt_and_render_attachment(
            &self.storage_policy,
            &self.blocking_crypto,
            payload_permit,
            resolved,
            read_strategy,
        )
        .await
    }

    pub async fn download_task_attachment(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<DownloadedAttachment> {
        let (payload_permit, resolved) = self
            .resolve_task_attachment_download(work_list_id, task_id, attachment_id, password_stdin)
            .await?;
        download_and_decrypt_attachment(
            &self.storage_policy,
            &self.blocking_crypto,
            payload_permit,
            resolved,
        )
        .await
    }

    async fn resolve_task_attachment_download(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<(LargePayloadPermit, ResolvedTaskAttachmentDownload)> {
        let (mut client, context) = self
            .load_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt attachment data.",
            )
            .await?;
        let payload_permit = self.blocking_crypto.admit_large_payload().await?;
        let task_detail = client.get_task(work_list_id, task_id).await?;
        let runtime = self.clone();
        let (payload_permit, (attachment, blob_ref)) = self
            .blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let list_key = runtime.require_work_list_key(&context)?;
                    let task = runtime.project_task_summary(task_detail.task, Some(&context));
                    let attachment = find_task_attachment(&task, attachment_id)?;
                    let blob_ref = decode_attachment_blob_key(list_key, attachment.blob_key())
                        .map_err(|err| {
                            PublicError::validation(format!(
                                "failed to decode attachment blob key: {err}"
                            ))
                        })?;
                    Ok((attachment, blob_ref))
                },
                "attachment task projection task failed",
            )
            .await?;
        let download = client
            .get_attachment_download(work_list_id, attachment_id)
            .await?;
        Ok((
            payload_permit,
            ResolvedTaskAttachmentDownload {
                attachment,
                blob_ref,
                download,
            },
        ))
    }
}

fn find_task_attachment(
    task: &AgentTaskSummary,
    attachment_id: Uuid,
) -> PublicResult<AgentAttachment> {
    let attachments = match task.attachments.as_ref() {
        Some(attachments) => attachments,
        None if task.read_error.is_some() => {
            return Err(read_error_to_public_error(
                task.read_error.as_ref(),
                "failed to read task attachments",
            ));
        }
        None => {
            return Err(PublicError::validation("task does not include attachments"));
        }
    };

    attachments
        .iter()
        .find(|attachment| attachment.id == attachment_id)
        .cloned()
        .ok_or_else(|| PublicError::validation(format!("attachment {attachment_id} not found")))
}

async fn download_and_decrypt_attachment(
    storage_policy: &StorageTransferPolicy,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
    resolved: ResolvedTaskAttachmentDownload,
) -> PublicResult<DownloadedAttachment> {
    let ResolvedTaskAttachmentDownload {
        attachment,
        blob_ref,
        download,
    } = resolved;
    let response = send_presigned_attachment_download(storage_policy, &download).await?;
    let ciphertext =
        read_attachment_ciphertext(response, &attachment.file_name, blob_ref.ciphertext_bytes)
            .await?;
    let (_, bytes) = blocking_crypto
        .run_with_large_payload(
            payload_permit,
            move || {
                decrypt_attachment_bytes(
                    &ciphertext,
                    &blob_ref.file_key,
                    Some(&blob_ref.enc_context),
                )
            },
            "attachment decryption task failed",
        )
        .await?;
    Ok(DownloadedAttachment { attachment, bytes })
}

async fn download_decrypt_and_render_attachment(
    storage_policy: &StorageTransferPolicy,
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
    resolved: ResolvedTaskAttachmentDownload,
    read_strategy: AttachmentReadStrategy,
) -> PublicResult<ReadableAttachment> {
    let ResolvedTaskAttachmentDownload {
        attachment,
        blob_ref,
        download,
    } = resolved;
    let response = send_presigned_attachment_download(storage_policy, &download).await?;
    let ciphertext =
        read_attachment_ciphertext(response, &attachment.file_name, blob_ref.ciphertext_bytes)
            .await?;
    let (_, readable) = blocking_crypto
        .run_with_large_payload(
            payload_permit,
            move || {
                let bytes = decrypt_attachment_bytes(
                    &ciphertext,
                    &blob_ref.file_key,
                    Some(&blob_ref.enc_context),
                )?;
                build_readable_attachment(attachment, bytes, read_strategy)
            },
            "attachment decryption and rendering task failed",
        )
        .await?;
    Ok(readable)
}

#[cfg(test)]
async fn decrypt_downloaded_attachment(
    blocking_crypto: &BlockingCryptoAdmission,
    ciphertext: Vec<u8>,
    blob_ref: AttachmentBlobRef,
) -> PublicResult<Vec<u8>> {
    let payload_permit = blocking_crypto.admit_large_payload().await?;
    blocking_crypto
        .run_with_large_payload(
            payload_permit,
            move || {
                decrypt_attachment_bytes(
                    &ciphertext,
                    &blob_ref.file_key,
                    Some(&blob_ref.enc_context),
                )
            },
            "attachment decryption task failed",
        )
        .await
        .map(|(_, bytes)| bytes)
}

async fn read_attachment_ciphertext(
    mut response: reqwest::Response,
    file_name: &str,
    expected_bytes: u64,
) -> PublicResult<Vec<u8>> {
    if let Some(content_length) = response.content_length()
        && content_length != expected_bytes
    {
        return Err(attachment_size_mismatch_error(
            file_name,
            expected_bytes,
            content_length,
        ));
    }

    let expected_len = usize::try_from(expected_bytes).map_err(|_| {
        PublicError::validation(format!(
            "attachment '{file_name}' is too large for this platform"
        ))
    })?;
    let mut ciphertext = Vec::with_capacity(expected_len.min(64 * 1024));
    while let Some(chunk) = response.chunk().await.map_err(|err| {
        PublicError::unexpected(format!(
            "failed to read attachment ciphertext: {}",
            err.without_url()
        ))
    })? {
        let received_len = ciphertext
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| PublicError::validation("attachment download size overflow"))?;
        if received_len > expected_len {
            return Err(attachment_size_mismatch_error(
                file_name,
                expected_bytes,
                received_len as u64,
            ));
        }
        ciphertext.extend_from_slice(&chunk);
    }

    if ciphertext.len() != expected_len {
        return Err(attachment_size_mismatch_error(
            file_name,
            expected_bytes,
            ciphertext.len() as u64,
        ));
    }
    Ok(ciphertext)
}

fn attachment_size_mismatch_error(
    file_name: &str,
    expected_bytes: u64,
    received_bytes: u64,
) -> PublicError {
    PublicError::validation(format!(
        "attachment '{file_name}' download size mismatch: expected {expected_bytes} bytes, got {received_bytes}"
    ))
}

async fn send_presigned_attachment_download(
    storage_policy: &StorageTransferPolicy,
    download: &DownloadAttachmentResponse,
) -> PublicResult<reqwest::Response> {
    let prepared = storage_policy
        .prepare(
            &download.download_url,
            &download.download_headers,
            download.expires_at,
        )
        .await?;
    let response = prepared
        .client
        .get(prepared.url)
        .headers(prepared.headers)
        .send()
        .await
        .map_err(|err| {
            PublicError::unexpected(format!(
                "failed to download attachment ciphertext: {}",
                err.without_url()
            ))
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(PublicError::unexpected(format!(
            "attachment download failed with status {}",
            status
        )));
    }

    Ok(response)
}

#[cfg(test)]
async fn build_readable_attachment_async(
    blocking_crypto: &BlockingCryptoAdmission,
    attachment: AgentAttachment,
    bytes: Vec<u8>,
    read_strategy: AttachmentReadStrategy,
) -> PublicResult<ReadableAttachment> {
    let payload_permit = blocking_crypto.admit_large_payload().await?;
    blocking_crypto
        .run_with_large_payload(
            payload_permit,
            move || build_readable_attachment(attachment, bytes, read_strategy),
            "attachment rendering task failed",
        )
        .await
        .map(|(_, readable)| readable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use sealtask_client_crypto::{
        ATTACHMENT_BLOB_CONTEXT_LABEL, MAX_ATTACHMENT_CIPHERTEXT_BYTES,
        MAX_ATTACHMENT_PLAINTEXT_BYTES, encrypt_attachment_bytes,
    };
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    const TEST_DOCX_BASE64: &str = "UEsDBBQAAAAIAOp8kVzXeYTq8QAAALgBAAATAAAAW0NvbnRlbnRfVHlwZXNdLnhtbH2QzU7DMBCE730Ky9cqccoBIZSkB36OwKE8wMreJFb9J69b2rdn00KREOVozXwz62nXB+/EHjPZGDq5qhspMOhobBg7+b55ru6koALBgIsBO3lEkut+0W6OCUkwHKiTUynpXinSE3qgOiYMrAwxeyj8zKNKoLcworppmlulYygYSlXmDNkvhGgfcYCdK+LpwMr5loyOpHg4e+e6TkJKzmoorKt9ML+Kqq+SmsmThyabaMkGqa6VzOL1jh/0lSfK1qB4g1xewLNRfcRslIl65xmu/0/649o4DFbjhZ/TUo4aiXh77+qL4sGG71+06jR8/wlQSwMEFAAAAAgA6nyRXCAbhuqyAAAALgEAAAsAAABfcmVscy8ucmVsc43Puw6CMBQG4J2naM4uBQdjDIXFmLAafICmPZRGeklbL7y9HRzEODie23fyN93TzOSOIWpnGdRlBQStcFJbxeAynDZ7IDFxK/nsLDJYMELXFs0ZZ57yTZy0jyQjNjKYUvIHSqOY0PBYOo82T0YXDE+5DIp6Lq5cId1W1Y6GTwPagpAVS3rJIPSyBjIsHv/h3ThqgUcnbgZt+vHlayPLPChMDB4uSCrf7TKzQHNKuorZvgBQSwMEFAAAAAgA6nyRXDbicKixAAAADAEAABEAAAB3b3JkL2RvY3VtZW50LnhtbG2PMQ+CMBCFd35F012KDsYQKIPGuLlo4lrpKST0rmmryL+3xbixfHkv9/Lurmo+ZmBvcL4nrPk6LzgDbEn3+Kz59XJc7TjzQaFWAyHUfALPG5lVY6mpfRnAwGID+nKseReCLYXwbQdG+ZwsYJw9yBkVonVPMZLT1lEL3scFZhCbotgKo3rkMmMstt5JT0nOxsoIlxDkCVQ6qhLJJLqZdjF8OO9vLFUtxpP47Unq/4f8AlBLAQIUAxQAAAAIAOp8kVzXeYTq8QAAALgBAAATAAAAAAAAAAAAAACAAQAAAABbQ29udGVudF9UeXBlc10ueG1sUEsBAhQDFAAAAAgA6nyRXCAbhuqyAAAALgEAAAsAAAAAAAAAAAAAAIABIgEAAF9yZWxzLy5yZWxzUEsBAhQDFAAAAAgA6nyRXDbicKixAAAADAEAABEAAAAAAAAAAAAAAIAB/QEAAHdvcmQvZG9jdW1lbnQueG1sUEsFBgAAAAADAAMAuQAAAN0CAAAAAA==";

    #[tokio::test(flavor = "current_thread")]
    async fn max_sized_attachment_decryption_keeps_single_worker_runtime_responsive() {
        let plaintext = vec![0x5a; MAX_ATTACHMENT_PLAINTEXT_BYTES as usize];
        let encrypted = encrypt_attachment_bytes(&plaintext).expect("encrypt maximum attachment");
        assert_eq!(
            encrypted.ciphertext.len() as u64,
            MAX_ATTACHMENT_CIPHERTEXT_BYTES
        );
        let expected = plaintext;
        let blob_ref = AttachmentBlobRef {
            version: 1,
            ciphertext_bytes: MAX_ATTACHMENT_CIPHERTEXT_BYTES,
            file_key: encrypted.file_key.as_bytes().to_vec(),
            enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL.to_string(),
        };
        let admission = BlockingCryptoAdmission::new(1);
        let decrypt_admission = admission.clone();
        let decrypt = tokio::spawn(async move {
            decrypt_downloaded_attachment(&decrypt_admission, encrypted.ciphertext, blob_ref).await
        });

        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("maximum-size decryption should start off-runtime");
        tokio::time::timeout(Duration::from_secs(1), async {
            tokio::task::yield_now().await;
        })
        .await
        .expect("runtime heartbeat must run while decryption is active");

        let decrypted = decrypt
            .await
            .expect("decryption task joins")
            .expect("decrypt maximum attachment");
        assert_eq!(decrypted, expected);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn docx_rendering_waits_for_shared_blocking_admission_capacity() {
        let admission = BlockingCryptoAdmission::new(1);
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let holder_gate = gate.clone();
        let holder_admission = admission.clone();
        let holder = tokio::spawn(async move {
            holder_admission
                .run(
                    move || {
                        let (lock, condition) = &*holder_gate;
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
                    "test admission holder failed",
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("admission holder starts");

        let attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "spec.docx".to_string(),
            content_type: DOCX_CONTENT_TYPE.to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(TEST_DOCX_BASE64)
            .expect("decode docx fixture");
        let render_admission = admission.clone();
        let render = tokio::spawn(async move {
            build_readable_attachment_async(
                &render_admission,
                attachment,
                bytes,
                AttachmentReadStrategy::DocxMarkdown,
            )
            .await
        });

        tokio::time::timeout(Duration::from_secs(1), async {
            while admission.waiting_count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("DOCX rendering should queue behind the shared admission");
        assert_eq!(admission.available_permits(), 0);

        let (lock, condition) = &*gate;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        condition.notify_all();
        holder
            .await
            .expect("admission holder joins")
            .expect("admission holder succeeds");
        let readable = render
            .await
            .expect("DOCX render task joins")
            .expect("DOCX render succeeds");
        assert_eq!(readable.text, "Heading\n\nDOCX body\n\n");
        assert_eq!(admission.available_permits(), 1);
    }
}
