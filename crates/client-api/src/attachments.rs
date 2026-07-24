use super::{MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES, PublicApiClient};
use chrono::{DateTime, Utc};
use sealtask_client_core::PublicResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

impl PublicApiClient {
    pub async fn get_attachment_download(
        &mut self,
        work_list_id: Uuid,
        attachment_id: Uuid,
    ) -> PublicResult<DownloadAttachmentResponse> {
        self.get_bounded(
            &format!("/work-lists/{work_list_id}/attachments/{attachment_id}/download"),
            MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn initiate_attachment_upload(
        &mut self,
        work_list_id: Uuid,
        payload: &InitiateAttachmentUploadRequest,
    ) -> PublicResult<InitiateAttachmentUploadResponse> {
        self.post_bounded(
            &format!("/work-lists/{work_list_id}/attachments"),
            payload,
            MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn complete_attachment_upload(
        &mut self,
        work_list_id: Uuid,
        attachment_id: Uuid,
        payload: &CompleteAttachmentUploadRequest,
    ) -> PublicResult<()> {
        self.post_no_content_bounded(
            &format!("/work-lists/{work_list_id}/attachments/{attachment_id}/complete"),
            payload,
            MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn delete_attachment(
        &mut self,
        work_list_id: Uuid,
        attachment_id: Uuid,
    ) -> PublicResult<()> {
        self.delete_no_content_bounded(
            &format!("/work-lists/{work_list_id}/attachments/{attachment_id}"),
            MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES,
        )
        .await
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadAttachmentResponse {
    pub download_url: String,
    pub download_headers: HashMap<String, String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitiateAttachmentUploadRequest {
    pub operation_id: Uuid,
    pub ciphertext_bytes: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitiateAttachmentUploadResponse {
    pub attachment_id: Uuid,
    pub upload_url: String,
    pub upload_headers: HashMap<String, String>,
    pub expires_at: DateTime<Utc>,
}

impl std::fmt::Debug for InitiateAttachmentUploadResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitiateAttachmentUploadResponse")
            .field("attachment_id", &self.attachment_id)
            .field("upload_url", &"<redacted>")
            .field("upload_headers", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteAttachmentUploadRequest {
    pub ciphertext_bytes: u64,
}

impl std::fmt::Debug for DownloadAttachmentResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadAttachmentResponse")
            .field("download_url", &"<redacted>")
            .field("download_headers", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_redact_attachment_download_credentials_from_debug_output() {
        let response = DownloadAttachmentResponse {
            download_url: "https://storage.example/object?signature=secret".to_string(),
            download_headers: HashMap::from([(
                "x-amz-security-token".to_string(),
                "secret-header".to_string(),
            )]),
            expires_at: Utc::now(),
        };

        let debug = format!("{response:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("signature=secret"));
        assert!(!debug.contains("secret-header"));
    }

    #[test]
    fn test_should_redact_attachment_upload_credentials_from_debug_output() {
        let response = InitiateAttachmentUploadResponse {
            attachment_id: Uuid::now_v7(),
            upload_url: "https://storage.example/object?signature=upload-secret".to_string(),
            upload_headers: HashMap::from([(
                "x-amz-security-token".to_string(),
                "upload-secret-header".to_string(),
            )]),
            expires_at: Utc::now(),
        };

        let debug = format!("{response:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("upload-secret"));
        assert!(!debug.contains("upload-secret-header"));
    }
}
