use super::RuntimeClient;
use crate::inputs::{CreateCommentArgs, DeleteCommentArgs, UpdateCommentArgs};
use crate::models::AgentComment;
use sealtask_client_api::{CreateCommentRequest, UpdateCommentRequest};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    CommentPayloadBody, build_comment_payload_envelope, compute_payload_proof, decode_sealed_blob,
    decrypt_comment_payload, derive_payload_binding_key, encrypt_comment_payload,
    plaintext_rich_text,
};
use uuid::Uuid;

impl RuntimeClient {
    pub async fn list_comments(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<Vec<AgentComment>> {
        let (mut client, context) = self
            .load_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt task comments.",
            )
            .await?;
        let comments = client.list_comments(work_list_id, task_id).await?;

        Ok(comments
            .into_iter()
            .map(|comment| self.project_comment(comment, context.list_key.as_ref()))
            .collect())
    }

    pub async fn create_comment(&self, args: CreateCommentArgs) -> PublicResult<AgentComment> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to create encrypted comments.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context)?;
        let binding_key = derive_payload_binding_key(list_key)?;

        let normalized_body = args.input.body.trim();
        if normalized_body.is_empty() {
            return Err(PublicError::validation("comment body is required"));
        }

        let rich_text = plaintext_rich_text(normalized_body)
            .ok_or_else(|| PublicError::validation("comment body is required"))?;
        let envelope = build_comment_payload_envelope(
            CommentPayloadBody {
                content: rich_text,
                mentions: None,
                attachments: None,
                client_meta: None,
            },
            1,
        );
        let body_ciphertext = encrypt_comment_payload(&envelope, list_key)?;
        let body_proof = compute_payload_proof(&body_ciphertext.bytes, &binding_key)?;

        let created = client
            .create_comment(
                args.work_list_id,
                args.task_id,
                &CreateCommentRequest {
                    body_ciphertext: body_ciphertext.base64,
                    body_ciphertext_proof: body_proof,
                },
            )
            .await?;
        Ok(self.project_comment(created, Some(list_key)))
    }

    pub async fn update_comment(&self, args: UpdateCommentArgs) -> PublicResult<AgentComment> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to update encrypted comments.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context)?;
        let binding_key = derive_payload_binding_key(list_key)?;

        let normalized_body = args.input.body.trim();
        if normalized_body.is_empty() {
            return Err(PublicError::validation("comment body is required"));
        }
        let rich_text = plaintext_rich_text(normalized_body)
            .ok_or_else(|| PublicError::validation("comment body is required"))?;

        let comments = client
            .list_comments(args.work_list_id, args.task_id)
            .await?;
        let existing_comment = comments
            .iter()
            .find(|comment| comment.id == args.comment_id)
            .ok_or_else(|| PublicError::validation("comment not found"))?;
        let existing_body_ciphertext = decode_sealed_blob(&existing_comment.body_ciphertext)?;
        let existing_payload = decrypt_comment_payload(list_key, &existing_body_ciphertext)?;

        let envelope = build_comment_payload_envelope(
            CommentPayloadBody {
                content: rich_text,
                mentions: existing_payload.body.mentions,
                attachments: existing_payload.body.attachments,
                client_meta: existing_payload.body.client_meta,
            },
            1,
        );
        let body_ciphertext = encrypt_comment_payload(&envelope, list_key)?;
        let body_proof = compute_payload_proof(&body_ciphertext.bytes, &binding_key)?;

        let updated = client
            .update_comment(
                args.work_list_id,
                args.task_id,
                args.comment_id,
                &UpdateCommentRequest {
                    body_ciphertext: Some(body_ciphertext.base64),
                    body_ciphertext_proof: Some(body_proof),
                },
            )
            .await?;
        Ok(self.project_comment(updated, Some(list_key)))
    }

    pub async fn delete_comment(&self, args: DeleteCommentArgs) -> PublicResult<()> {
        let mut client = self.authenticated_api_client()?;
        client
            .delete_comment(
                args.work_list_id,
                args.task_id,
                args.comment_id,
                &args.input,
            )
            .await
    }
}
