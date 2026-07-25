use super::{AuditPatchRequest, PublicApiClient, SealedBlob};
use chrono::{DateTime, Utc};
use sealtask_client_core::PublicResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::note_transport::{
    DeleteNoteResponse, EncodedNoteRequest, EncodedNoteResponse, NoteRequestPayload,
    NoteResponsePayload, decode_json_response, sealed,
};
use crate::note_transport_limits::{
    MAX_NOTE_CURSOR_BYTES, MAX_NOTE_DECOMPRESSED_PAGE_BYTES, MAX_NOTE_PAGE_ITEMS,
    MIN_NOTE_PAGE_ITEMS,
};

impl PublicApiClient {
    pub async fn list_notes_page_encoded(
        &mut self,
        work_list_id: Uuid,
        cursor: Option<&str>,
        limit: u32,
    ) -> PublicResult<EncodedNoteResponse<NotePage>> {
        if !(MIN_NOTE_PAGE_ITEMS..=MAX_NOTE_PAGE_ITEMS).contains(&limit) {
            return Err(sealtask_client_core::PublicError::validation(format!(
                "note page limit must be between {MIN_NOTE_PAGE_ITEMS} and {MAX_NOTE_PAGE_ITEMS}"
            )));
        }
        if let Some(cursor) = cursor {
            validate_notes_cursor(cursor)?;
        }
        let cursor = cursor.map_or_else(String::new, |cursor| format!("&cursor={cursor}"));
        let path = format!("/work-lists/{work_list_id}/notes?limit={limit}{cursor}");
        let response = self
            .get_bounded_body(&path, MAX_NOTE_DECOMPRESSED_PAGE_BYTES)
            .await?;
        EncodedNoteResponse::from_complete_bounded_http(path, response)
    }

    pub async fn get_note_encoded(
        &mut self,
        work_list_id: Uuid,
        note_id: Uuid,
    ) -> PublicResult<EncodedNoteResponse<NoteResponse>> {
        let path = format!("/work-lists/{work_list_id}/notes/{note_id}");
        let response = self
            .get_bounded_body(&path, MAX_NOTE_DECOMPRESSED_PAGE_BYTES)
            .await?;
        EncodedNoteResponse::from_complete_bounded_http(path, response)
    }

    pub async fn create_note_encoded(
        &mut self,
        work_list_id: Uuid,
        encoded_payload: EncodedNoteRequest<CreateNoteRequest>,
    ) -> PublicResult<EncodedNoteResponse<NoteResponse>> {
        let path = format!("/work-lists/{work_list_id}/notes");
        let response = self
            .send_json_bytes_bounded_body(
                reqwest::Method::POST,
                &path,
                encoded_payload.into_body(),
                MAX_NOTE_DECOMPRESSED_PAGE_BYTES,
            )
            .await?;
        Ok(EncodedNoteResponse::from_bounded_http(path, response))
    }

    pub async fn update_note_encoded(
        &mut self,
        work_list_id: Uuid,
        note_id: Uuid,
        encoded_payload: EncodedNoteRequest<UpdateNoteRequest>,
    ) -> PublicResult<EncodedNoteResponse<NoteResponse>> {
        let path = format!("/work-lists/{work_list_id}/notes/{note_id}");
        let response = self
            .send_json_bytes_bounded_body(
                reqwest::Method::PATCH,
                &path,
                encoded_payload.into_body(),
                MAX_NOTE_DECOMPRESSED_PAGE_BYTES,
            )
            .await?;
        Ok(EncodedNoteResponse::from_bounded_http(path, response))
    }

    pub async fn delete_note_encoded(
        &mut self,
        work_list_id: Uuid,
        note_id: Uuid,
        encoded_payload: EncodedNoteRequest<DeleteNoteRequest>,
    ) -> PublicResult<EncodedNoteResponse<DeleteNoteResponse>> {
        let path = format!("/work-lists/{work_list_id}/notes/{note_id}");
        let response = self
            .send_json_bytes_bounded_body(
                reqwest::Method::DELETE,
                &path,
                encoded_payload.into_body(),
                MAX_NOTE_DECOMPRESSED_PAGE_BYTES,
            )
            .await?;
        Ok(EncodedNoteResponse::from_bounded_http(path, response))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteResponse {
    pub id: Uuid,
    pub work_list_id: Uuid,
    pub created_by_membership_id: Uuid,
    pub title_ciphertext: SealedBlob,
    #[serde(default)]
    pub legacy_cbor_fields: Vec<String>,
    pub payload_ciphertext: SealedBlob,
    pub is_private: bool,
    pub note_key_ciphertext: Option<SealedBlob>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for NoteResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoteResponse")
            .field("id", &self.id)
            .field("work_list_id", &self.work_list_id)
            .field("created_by_membership_id", &self.created_by_membership_id)
            .field("title_ciphertext", &Redacted)
            .field("legacy_cbor_fields", &Redacted)
            .field("payload_ciphertext", &Redacted)
            .field("is_private", &self.is_private)
            .field("note_key_ciphertext", &Redacted)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePage {
    pub notes: Vec<NoteResponse>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

impl fmt::Debug for NotePage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NotePage")
            .field("note_count", &self.notes.len())
            .field("next_cursor", &Redacted)
            .finish()
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub idempotency_key: String,
    pub idempotency_commitment: String,
    pub title_ciphertext: String,
    pub title_ciphertext_proof: String,
    pub payload_ciphertext: String,
    pub payload_ciphertext_proof: String,
    pub is_private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_key_ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

impl fmt::Debug for CreateNoteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateNoteRequest")
            .field("idempotency_key", &self.idempotency_key)
            .field("idempotency_commitment", &Redacted)
            .field("title_ciphertext", &Redacted)
            .field("title_ciphertext_proof", &Redacted)
            .field("payload_ciphertext", &Redacted)
            .field("payload_ciphertext_proof", &Redacted)
            .field("is_private", &self.is_private)
            .field("note_key_ciphertext", &Redacted)
            .field("audit_patch", &Redacted)
            .finish()
    }
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_ciphertext_proof: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_ciphertext_proof: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_private: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_key_ciphertext: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

impl fmt::Debug for UpdateNoteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateNoteRequest")
            .field("expected_updated_at", &self.expected_updated_at)
            .field("title_ciphertext", &Redacted)
            .field("title_ciphertext_proof", &Redacted)
            .field("payload_ciphertext", &Redacted)
            .field("payload_ciphertext_proof", &Redacted)
            .field("is_private", &self.is_private)
            .field("note_key_ciphertext", &Redacted)
            .field("audit_patch", &Redacted)
            .finish()
    }
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteNoteRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

impl fmt::Debug for DeleteNoteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeleteNoteRequest")
            .field("audit_patch", &Redacted)
            .finish()
    }
}

macro_rules! impl_request_payload {
    ($($payload:ty),+ $(,)?) => {
        $(
            impl sealed::Sealed for $payload {}
            impl NoteRequestPayload for $payload {}
        )+
    };
}

impl_request_payload!(CreateNoteRequest, UpdateNoteRequest, DeleteNoteRequest);

macro_rules! impl_json_response_payload {
    ($($payload:ty),+ $(,)?) => {
        $(
            impl sealed::Sealed for $payload {}
            impl NoteResponsePayload for $payload {
                fn decode(response: EncodedNoteResponse<Self>) -> PublicResult<Self> {
                    decode_json_response(response)
                }
            }
        )+
    };
}

impl_json_response_payload!(NotePage, NoteResponse);

struct Redacted;

impl fmt::Debug for Redacted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

fn validate_notes_cursor(cursor: &str) -> PublicResult<()> {
    if cursor.is_empty()
        || cursor.len() > MAX_NOTE_CURSOR_BYTES
        || !cursor
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(sealtask_client_core::PublicError::unexpected(
            "server returned an invalid notes cursor",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_dto_debug_output_redacts_encrypted_payloads_proofs_and_cursors() {
        const TITLE: &str = "title-ciphertext-debug-canary";
        const TITLE_PROOF: &str = "title-proof-debug-canary";
        const PAYLOAD: &str = "payload-ciphertext-debug-canary";
        const PAYLOAD_PROOF: &str = "payload-proof-debug-canary";
        const NOTE_KEY: &str = "note-key-debug-canary";
        const LEGACY_FIELD: &str = "legacy-field-debug-canary";
        const CURSOR: &str = "cursor-debug-canary";
        const AUDIT_PAYLOAD: &str = "audit-payload-debug-canary";
        const AUDIT_PROOF: &str = "audit-proof-debug-canary";

        let now = Utc::now();
        let response = NoteResponse {
            id: Uuid::now_v7(),
            work_list_id: Uuid::now_v7(),
            created_by_membership_id: Uuid::now_v7(),
            title_ciphertext: TITLE.to_string(),
            legacy_cbor_fields: vec![LEGACY_FIELD.to_string()],
            payload_ciphertext: PAYLOAD.to_string(),
            is_private: true,
            note_key_ciphertext: Some(NOTE_KEY.to_string()),
            created_at: now,
            updated_at: now,
        };
        let audit_patch = AuditPatchRequest {
            fields: Vec::new(),
            payload_ciphertext: AUDIT_PAYLOAD.to_string(),
            payload_ciphertext_proof: AUDIT_PROOF.to_string(),
            payload_version: 1,
        };
        let page = NotePage {
            notes: vec![response.clone()],
            next_cursor: Some(CURSOR.to_string()),
        };
        let create = CreateNoteRequest {
            idempotency_key: "note-debug-key".to_string(),
            idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            title_ciphertext: TITLE.to_string(),
            title_ciphertext_proof: TITLE_PROOF.to_string(),
            payload_ciphertext: PAYLOAD.to_string(),
            payload_ciphertext_proof: PAYLOAD_PROOF.to_string(),
            is_private: true,
            note_key_ciphertext: Some(NOTE_KEY.to_string()),
            audit_patch: Some(audit_patch.clone()),
        };
        let update = UpdateNoteRequest {
            expected_updated_at: Some(now),
            title_ciphertext: Some(TITLE.to_string()),
            title_ciphertext_proof: Some(TITLE_PROOF.to_string()),
            payload_ciphertext: Some(PAYLOAD.to_string()),
            payload_ciphertext_proof: Some(PAYLOAD_PROOF.to_string()),
            is_private: Some(true),
            note_key_ciphertext: Some(Some(NOTE_KEY.to_string())),
            audit_patch: Some(audit_patch.clone()),
        };
        let delete = DeleteNoteRequest {
            audit_patch: Some(audit_patch),
        };
        let create_wire = serde_json::to_value(&create).expect("serialize create request");
        assert_eq!(create_wire["idempotencyKey"], "note-debug-key");
        assert_eq!(
            create_wire["idempotencyCommitment"],
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );

        let debug_values = [
            format!("{response:?}"),
            format!("{page:?}"),
            format!("{create:?}"),
            format!("{update:?}"),
            format!("{delete:?}"),
        ];
        for debug in debug_values {
            for canary in [
                TITLE,
                TITLE_PROOF,
                PAYLOAD,
                PAYLOAD_PROOF,
                NOTE_KEY,
                LEGACY_FIELD,
                CURSOR,
                AUDIT_PAYLOAD,
                AUDIT_PROOF,
            ] {
                assert!(
                    !debug.contains(canary),
                    "debug output exposed secret canary {canary}: {debug}"
                );
            }
            assert!(
                debug.contains("<redacted>"),
                "debug output must make redaction explicit: {debug}"
            );
        }
    }

    #[test]
    fn notes_cursor_rejects_query_delimiters_and_oversized_values() {
        assert!(validate_notes_cursor("valid_-Cursor09").is_ok());
        assert!(validate_notes_cursor("cursor&limit=100").is_err());
        assert!(validate_notes_cursor(&"a".repeat(MAX_NOTE_CURSOR_BYTES + 1)).is_err());
    }
}
