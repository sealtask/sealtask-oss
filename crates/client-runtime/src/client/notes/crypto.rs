use sealtask_client_api::note_transport::EncodedNoteRequest;
use sealtask_client_api::{CreateNoteRequest, DeleteNoteRequest, NoteResponse, UpdateNoteRequest};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    FlexibleValue, NOTE_TITLE_CONTEXT, NotePayloadBody, NotePayloadEnvelope, RichTextBlock,
    SymmetricKey, TaskPayloadRichText, build_note_payload_envelope,
    compute_note_create_semantic_commitment, compute_payload_proof, decode_sealed_blob,
    decrypt_note_key, decrypt_note_payload, derive_payload_binding_key, encrypt_note_key,
    encrypt_note_payload, encrypt_text_value, generate_symmetric_key,
};
use serde::Serialize;

use crate::inputs::{NoteCreateInput, NoteUpdateInput, validate_idempotency_key};

use super::super::{RuntimeClient, UnlockedWorkListContext};

const MAX_NOTE_TITLE_CHARS: usize = 256;
pub(super) const MAX_NOTE_PLAINTEXT_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_RICH_TEXT_BLOCKS: usize = 500;
pub(super) const MAX_RICH_TEXT_BLOCK_CHARS: usize = 8_192;

pub(super) struct PreparedNoteCreate {
    pub(super) request: CreateNoteRequest,
    pub(super) encoded: EncodedNoteRequest<CreateNoteRequest>,
}

pub(super) struct PreparedNoteUpdate {
    pub(super) current: NoteResponse,
    pub(super) request: UpdateNoteRequest,
    pub(super) encoded: EncodedNoteRequest<UpdateNoteRequest>,
}

pub(super) struct PreparedNoteDelete {
    pub(super) current: NoteResponse,
    pub(super) encoded: EncodedNoteRequest<DeleteNoteRequest>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteCreateSemanticPlan<'a> {
    title: &'a str,
    body: &'a str,
    is_private: bool,
}

impl RuntimeClient {
    pub(super) fn prepare_create_note_request(
        &self,
        input: NoteCreateInput,
        context: &UnlockedWorkListContext,
    ) -> PublicResult<CreateNoteRequest> {
        let list_key = self.require_work_list_key(&context.work_list)?;
        let title = normalize_note_title(&input.title)?;
        let content = markdown_note_content(&input.body)?;
        validate_note_plaintext_size(&title, &content)?;
        let idempotency_key = validate_idempotency_key(&input.idempotency_key)?;
        let canonical_semantics = serde_json::to_vec(&NoteCreateSemanticPlan {
            title: &title,
            body: &input.body,
            is_private: input.is_private,
        })
        .map_err(|err| {
            PublicError::unexpected(format!(
                "failed to encode note idempotency semantics: {err}"
            ))
        })?;
        let idempotency_commitment =
            compute_note_create_semantic_commitment(&canonical_semantics, list_key)?;
        let body = NotePayloadBody {
            title: title.clone(),
            content,
            mentions: Some(Vec::new()),
            attachments: Some(Vec::new()),
            client_meta: Some(FlexibleValue::Map(Vec::new())),
        };
        let envelope = build_note_payload_envelope(body, 1);
        let note_key = if input.is_private {
            generate_symmetric_key()?
        } else {
            list_key.clone()
        };
        let note_key_ciphertext = if input.is_private {
            Some(encrypt_note_key(&note_key, &context.data_key)?.base64)
        } else {
            None
        };
        let binding_key = derive_payload_binding_key(list_key)?;
        let payload_ciphertext = encrypt_note_payload(&envelope, &note_key)?;
        let title_ciphertext = encrypt_text_value(&title, &note_key, NOTE_TITLE_CONTEXT)?;
        Ok(CreateNoteRequest {
            idempotency_key,
            idempotency_commitment,
            title_ciphertext_proof: compute_payload_proof(&title_ciphertext.bytes, &binding_key)?,
            title_ciphertext: title_ciphertext.base64,
            payload_ciphertext_proof: compute_payload_proof(
                &payload_ciphertext.bytes,
                &binding_key,
            )?,
            payload_ciphertext: payload_ciphertext.base64,
            is_private: input.is_private,
            note_key_ciphertext,
            audit_patch: None,
        })
    }

    pub(super) fn prepare_update_note_request(
        &self,
        current: &NoteResponse,
        input: NoteUpdateInput,
        context: &UnlockedWorkListContext,
    ) -> PublicResult<UpdateNoteRequest> {
        let list_key = self.require_work_list_key(&context.work_list)?;
        let note_key = resolve_note_key(current, list_key, &context.data_key)?;
        let payload_bytes = decode_sealed_blob(&current.payload_ciphertext)?;
        let existing = decrypt_note_payload(&note_key, &payload_bytes)?;
        validate_note_envelope(&existing)?;
        let title = match input.title.as_deref() {
            Some(title) => normalize_note_title(title)?,
            None => existing.body.title,
        };
        let content = match input.body.as_deref() {
            Some(body) => markdown_note_content(body)?,
            None => existing.body.content,
        };
        validate_note_plaintext_size(&title, &content)?;
        let next = build_note_payload_envelope(
            NotePayloadBody {
                title: title.clone(),
                content,
                mentions: existing.body.mentions,
                attachments: existing.body.attachments,
                client_meta: existing.body.client_meta,
            },
            existing.version,
        );
        let binding_key = derive_payload_binding_key(list_key)?;
        let payload_ciphertext = encrypt_note_payload(&next, &note_key)?;
        let title_ciphertext = encrypt_text_value(&title, &note_key, NOTE_TITLE_CONTEXT)?;
        Ok(UpdateNoteRequest {
            expected_updated_at: Some(current.updated_at),
            title_ciphertext_proof: Some(compute_payload_proof(
                &title_ciphertext.bytes,
                &binding_key,
            )?),
            title_ciphertext: Some(title_ciphertext.base64),
            payload_ciphertext_proof: Some(compute_payload_proof(
                &payload_ciphertext.bytes,
                &binding_key,
            )?),
            payload_ciphertext: Some(payload_ciphertext.base64),
            is_private: None,
            note_key_ciphertext: None,
            audit_patch: None,
        })
    }
}

pub(super) fn resolve_note_key(
    note: &NoteResponse,
    list_key: &SymmetricKey,
    data_key: &SymmetricKey,
) -> PublicResult<SymmetricKey> {
    if !note.is_private {
        return Ok(list_key.clone());
    }
    let ciphertext = note
        .note_key_ciphertext
        .as_deref()
        .ok_or_else(|| PublicError::validation("private note is missing its encrypted note key"))?;
    let bytes = decode_sealed_blob(ciphertext)?;
    decrypt_note_key(&bytes, data_key)
}

pub(super) fn validate_note_envelope(envelope: &NotePayloadEnvelope) -> PublicResult<()> {
    if envelope.kind != "note" || envelope.version != 1 {
        return Err(PublicError::validation(
            "unsupported encrypted note payload envelope",
        ));
    }
    Ok(())
}

fn normalize_note_title(title: &str) -> PublicResult<String> {
    let title = title.trim();
    let chars = title.chars().count();
    if chars == 0 || chars > MAX_NOTE_TITLE_CHARS {
        return Err(PublicError::validation(format!(
            "note title must contain between 1 and {MAX_NOTE_TITLE_CHARS} characters"
        )));
    }
    Ok(title.to_string())
}

pub(super) fn markdown_note_content(body: &str) -> PublicResult<TaskPayloadRichText> {
    let blocks = if body.is_empty() {
        vec![RichTextBlock {
            block_type: "paragraph".to_string(),
            text: String::new(),
        }]
    } else {
        body.split("\n\n")
            .map(|text| {
                if text.chars().count() > MAX_RICH_TEXT_BLOCK_CHARS {
                    return Err(PublicError::validation(format!(
                        "note body paragraphs cannot exceed {MAX_RICH_TEXT_BLOCK_CHARS} characters"
                    )));
                }
                Ok(RichTextBlock {
                    block_type: "paragraph".to_string(),
                    text: text.to_string(),
                })
            })
            .collect::<PublicResult<Vec<_>>>()?
    };
    if blocks.len() > MAX_RICH_TEXT_BLOCKS {
        return Err(PublicError::validation(format!(
            "note body cannot exceed {MAX_RICH_TEXT_BLOCKS} paragraphs"
        )));
    }
    Ok(TaskPayloadRichText {
        format: "markdown".to_string(),
        version: 1,
        blocks,
    })
}

pub(super) fn validate_note_plaintext_size(
    title: &str,
    content: &TaskPayloadRichText,
) -> PublicResult<()> {
    let bytes = content
        .blocks
        .iter()
        .try_fold(title.len(), |bytes, block| {
            bytes
                .checked_add(block.text.len())
                .ok_or_else(|| PublicError::validation("note plaintext size overflowed"))
        })?;
    if bytes > MAX_NOTE_PLAINTEXT_BYTES {
        return Err(PublicError::payload_too_large(format!(
            "note plaintext exceeds the {MAX_NOTE_PLAINTEXT_BYTES}-byte limit"
        )));
    }
    Ok(())
}
