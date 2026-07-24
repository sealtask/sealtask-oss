use sealtask_client_api::NoteResponse;
use sealtask_client_core::PublicResult;
use sealtask_client_crypto::{
    FlexibleValue, NOTE_TITLE_CONTEXT, TaskPayloadRichText, decode_sealed_blob,
    decrypt_encrypted_text_value, decrypt_note_payload, flexible_value_to_json,
};

use crate::models::AgentNote;
use crate::projections::{make_read_error, project_attachments, rich_text_to_markdown};

use super::super::{RuntimeClient, UnlockedWorkListContext};
use super::crypto::{resolve_note_key, validate_note_envelope};

struct DecryptedNote {
    title: String,
    content: TaskPayloadRichText,
    mentions: Option<Vec<String>>,
    attachments: Option<Vec<FlexibleValue>>,
    client_meta: Option<FlexibleValue>,
}

impl RuntimeClient {
    pub(super) fn project_note(
        &self,
        note: NoteResponse,
        context: &UnlockedWorkListContext,
    ) -> AgentNote {
        let decrypted = self.decrypt_note(&note, context);
        let (title, body_markdown, content, mentions, attachments, client_meta, read_error) =
            match decrypted {
                Ok(decrypted) => {
                    let body_markdown = rich_text_to_markdown(&decrypted.content);
                    let (attachments, read_error) = match project_attachments(decrypted.attachments)
                    {
                        Ok(attachments) => (attachments, None),
                        Err(err) => (None, Some(make_read_error("note_attachments", err))),
                    };
                    (
                        Some(decrypted.title),
                        body_markdown,
                        Some(decrypted.content),
                        decrypted.mentions,
                        attachments,
                        decrypted.client_meta.map(flexible_value_to_json),
                        read_error,
                    )
                }
                Err(err) => (
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(make_read_error("note_payload", err)),
                ),
            };

        AgentNote {
            id: note.id,
            work_list_id: note.work_list_id,
            created_by_membership_id: note.created_by_membership_id,
            is_private: note.is_private,
            title,
            body_markdown,
            content,
            mentions,
            attachments,
            client_meta,
            created_at: note.created_at,
            updated_at: note.updated_at,
            read_error,
        }
    }

    #[cfg(test)]
    pub(super) async fn project_note_page(
        &self,
        notes: Vec<NoteResponse>,
        context: UnlockedWorkListContext,
    ) -> PublicResult<Vec<AgentNote>> {
        let runtime = self.clone();
        self.blocking_crypto
            .run(
                move || {
                    Ok(notes
                        .into_iter()
                        .map(|note| runtime.project_note(note, &context))
                        .collect())
                },
                "note page decryption task failed",
            )
            .await
    }

    #[cfg(test)]
    pub(super) async fn project_single_note(
        &self,
        note: NoteResponse,
        context: UnlockedWorkListContext,
    ) -> PublicResult<AgentNote> {
        let runtime = self.clone();
        self.blocking_crypto
            .run(
                move || Ok(runtime.project_note(note, &context)),
                "note decryption task failed",
            )
            .await
    }

    fn decrypt_note(
        &self,
        note: &NoteResponse,
        context: &UnlockedWorkListContext,
    ) -> PublicResult<DecryptedNote> {
        let list_key = self.require_work_list_key(&context.work_list)?;
        let note_key = resolve_note_key(note, list_key, &context.data_key)?;
        let payload_bytes = decode_sealed_blob(&note.payload_ciphertext)?;
        let envelope = decrypt_note_payload(&note_key, &payload_bytes)?;
        validate_note_envelope(&envelope)?;
        let title = if envelope.body.title.trim().is_empty() {
            let title_bytes = decode_sealed_blob(&note.title_ciphertext)?;
            decrypt_encrypted_text_value(&title_bytes, &note_key, NOTE_TITLE_CONTEXT)?
        } else {
            envelope.body.title
        };
        Ok(DecryptedNote {
            title,
            content: envelope.body.content,
            mentions: envelope.body.mentions,
            attachments: envelope.body.attachments,
            client_meta: envelope.body.client_meta,
        })
    }
}
