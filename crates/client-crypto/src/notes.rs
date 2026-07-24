use super::{
    FlexibleValue, SealedBlobPayload, SymmetricKey, TaskPayloadRichText, decrypt_sealed_bytes,
    decrypt_sealed_payload, encrypt_sealed_bytes, encrypt_sealed_payload, symmetric_key_from_bytes,
};
use sealtask_client_core::PublicResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

pub const NOTE_PAYLOAD_CONTEXT: &[u8] = b"worklist.note.v1";
pub const NOTE_TITLE_CONTEXT: &[u8] = b"worklist.note.title.v1";
pub const NOTE_KEY_CONTEXT: &[u8] = b"worklist.note.key.v1";

#[derive(Serialize, Deserialize)]
pub struct NotePayloadEnvelope {
    pub kind: String,
    pub version: u8,
    pub body: NotePayloadBody,
}

impl fmt::Debug for NotePayloadEnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotePayloadEnvelope")
            .field("kind", &self.kind)
            .field("version", &self.version)
            .field("body", &"<redacted>")
            .finish()
    }
}

#[derive(Serialize, Deserialize)]
pub struct NotePayloadBody {
    pub title: String,
    pub content: TaskPayloadRichText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<FlexibleValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_meta: Option<FlexibleValue>,
}

impl fmt::Debug for NotePayloadBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotePayloadBody")
            .field("title", &"<redacted>")
            .field("content", &"<redacted>")
            .field("mentions", &"<redacted>")
            .field("attachments", &"<redacted>")
            .field("client_meta", &"<redacted>")
            .finish()
    }
}

pub fn decrypt_note_payload(
    note_key: &SymmetricKey,
    payload_ciphertext: &[u8],
) -> PublicResult<NotePayloadEnvelope> {
    decrypt_sealed_payload(
        note_key,
        payload_ciphertext,
        NOTE_PAYLOAD_CONTEXT,
        "failed to decrypt note payload",
    )
}

pub fn encrypt_note_key(
    note_key: &SymmetricKey,
    data_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    encrypt_sealed_bytes(
        note_key.as_bytes(),
        data_key,
        NOTE_KEY_CONTEXT,
        "failed to seal note key",
    )
}

pub fn decrypt_note_key(
    note_key_ciphertext: &[u8],
    data_key: &SymmetricKey,
) -> PublicResult<SymmetricKey> {
    let plaintext = Zeroizing::new(decrypt_sealed_bytes(
        data_key,
        note_key_ciphertext,
        NOTE_KEY_CONTEXT,
        "failed to decrypt note key",
    )?);
    symmetric_key_from_bytes(&plaintext)
}

pub fn build_note_payload_envelope(body: NotePayloadBody, version: u8) -> NotePayloadEnvelope {
    NotePayloadEnvelope {
        kind: "note".to_string(),
        version,
        body,
    }
}

pub fn encrypt_note_payload(
    envelope: &NotePayloadEnvelope,
    note_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    encrypt_sealed_payload(
        envelope,
        note_key,
        NOTE_PAYLOAD_CONTEXT,
        "failed to seal note payload",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SealedPayload, build_task_attachment_ref, deserialize_from_cbor, serialize_to_cbor,
    };

    #[derive(Deserialize)]
    struct CrossClientAttachmentRef {
        id: String,
        file_name: String,
        content_type: String,
        size_bytes: u64,
        #[serde(with = "serde_bytes")]
        blob_key: Vec<u8>,
        created_by_membership_id: String,
    }

    #[test]
    fn decrypted_note_payload_debug_is_redacted() {
        let body = NotePayloadBody {
            title: "private title".to_string(),
            content: TaskPayloadRichText {
                format: "markdown".to_string(),
                version: 1,
                blocks: Vec::new(),
            },
            mentions: Some(vec!["secret mention".to_string()]),
            attachments: None,
            client_meta: None,
        };
        let debug = format!("{:?}", build_note_payload_envelope(body, 1));

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("private title"));
        assert!(!debug.contains("secret mention"));
    }

    #[test]
    fn oss_note_attachment_reference_keeps_cross_client_cbor_byte_strings() {
        let attachment_id = uuid::Uuid::now_v7();
        let membership_id = uuid::Uuid::now_v7();
        let blob_key = SealedPayload::new(vec![0x10, 0x20, 0x30])
            .to_bytes()
            .expect("sealed attachment reference");
        let attachment = build_task_attachment_ref(
            attachment_id,
            "report.txt".to_string(),
            "text/plain".to_string(),
            3,
            blob_key.clone(),
            membership_id,
        );

        let attachment_wire = serialize_to_cbor(&attachment).expect("encode attachment reference");
        let decoded: CrossClientAttachmentRef =
            deserialize_from_cbor(&attachment_wire).expect("cross-client attachment decode");
        assert_eq!(decoded.id, attachment_id.to_string());
        assert_eq!(decoded.file_name, "report.txt");
        assert_eq!(decoded.content_type, "text/plain");
        assert_eq!(decoded.size_bytes, 3);
        assert_eq!(decoded.blob_key, blob_key);
        assert_eq!(decoded.created_by_membership_id, membership_id.to_string());

        let note_key = SymmetricKey::new([0x42; crate::KEY_SIZE]);
        let encrypted = encrypt_note_payload(
            &build_note_payload_envelope(
                NotePayloadBody {
                    title: "Attachment note".to_string(),
                    content: TaskPayloadRichText {
                        format: "markdown".to_string(),
                        version: 1,
                        blocks: Vec::new(),
                    },
                    mentions: None,
                    attachments: Some(vec![attachment]),
                    client_meta: None,
                },
                1,
            ),
            &note_key,
        )
        .expect("encrypt OSS note");
        let decoded_note =
            decrypt_note_payload(&note_key, &encrypted.bytes).expect("decrypt OSS note");
        let decoded_attachment = decoded_note
            .body
            .attachments
            .and_then(|attachments| attachments.into_iter().next())
            .expect("decoded attachment");
        let fields = match decoded_attachment {
            FlexibleValue::Map(fields) => fields,
            other => panic!("decoded attachment must be a map, got {other:?}"),
        };
        assert!(
            fields.into_iter().any(|(key, value)| {
                matches!(key, FlexibleValue::Text(ref key) if key == "blob_key")
                    && matches!(value, FlexibleValue::Bytes(_))
            }),
            "OSS-produced note attachment blob_key must remain CBOR major type 2"
        );
    }
}
