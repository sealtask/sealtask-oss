use super::{
    FlexibleValue, StrongBoxKeyRing, SymmetricKey, decrypt_sealed_bytes, deserialize_from_cbor,
    encrypt_sealed_payload, generate_symmetric_key, symmetric_key_from_bytes,
};
use sealtask_client_core::{PublicError, PublicResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use strong_box::StrongBox;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::attachment_transport_limits::MAX_ATTACHMENT_CIPHERTEXT_BYTES;

pub const ATTACHMENT_BLOB_CONTEXT: &[u8] = b"worklist.attachment.blob.v1";
pub const ATTACHMENT_REF_CONTEXT: &[u8] = b"worklist.attachment.ref.v1";
pub const ATTACHMENT_BLOB_CONTEXT_LABEL: &str = "worklist.attachment.blob.v1";
pub const ATTACHMENT_BLOB_REF_VERSION: u8 = 1;

/// Decrypted attachment blob reference that must not cross generic serialization paths.
///
/// ```compile_fail
/// use sealtask_client_crypto::AttachmentBlobRef;
///
/// let blob_ref = AttachmentBlobRef {
///     version: 1,
///     ciphertext_bytes: 42,
///     file_key: vec![7; 32],
///     enc_context: "worklist.attachment.blob.v1".to_string(),
/// };
/// let _ = serde_json::to_string(&blob_ref);
/// ```
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AttachmentBlobRef {
    pub version: u8,
    pub ciphertext_bytes: u64,
    pub file_key: Vec<u8>,
    #[serde(default = "default_attachment_blob_context_label")]
    pub enc_context: String,
}

#[derive(Serialize)]
struct AttachmentBlobRefWire<'a> {
    version: u8,
    ciphertext_bytes: u64,
    file_key: &'a [u8],
    enc_context: &'a str,
}

pub struct EncryptedAttachment {
    pub ciphertext: Vec<u8>,
    pub file_key: SymmetricKey,
    pub enc_context: String,
}

impl fmt::Debug for EncryptedAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedAttachment")
            .field("ciphertext_bytes", &self.ciphertext.len())
            .field("file_key", &"<redacted>")
            .field("enc_context", &self.enc_context)
            .finish()
    }
}

impl fmt::Debug for AttachmentBlobRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AttachmentBlobRef")
            .field("version", &self.version)
            .field("ciphertext_bytes", &self.ciphertext_bytes)
            .field("file_key", &"<redacted>")
            .field("enc_context", &self.enc_context)
            .finish()
    }
}

pub fn decode_attachment_blob_key(
    list_key: &SymmetricKey,
    blob_key: &[u8],
) -> PublicResult<AttachmentBlobRef> {
    let plaintext = Zeroizing::new(decrypt_sealed_bytes(
        list_key,
        blob_key,
        ATTACHMENT_REF_CONTEXT,
        "failed to decrypt attachment reference",
    )?);
    let blob_ref = deserialize_from_cbor::<AttachmentBlobRef>(&plaintext).map_err(|err| {
        PublicError::validation(format!(
            "failed to deserialize attachment key material: {err}"
        ))
    })?;
    validate_attachment_blob_ref(&blob_ref)?;
    Ok(blob_ref)
}

pub fn encrypt_attachment_bytes(plaintext: &[u8]) -> PublicResult<EncryptedAttachment> {
    let file_key = generate_symmetric_key()?;
    let ciphertext = StrongBoxKeyRing::new(file_key.clone())
        .strong_box()
        .encrypt(plaintext, ATTACHMENT_BLOB_CONTEXT)
        .map_err(|err| PublicError::crypto(format!("failed to encrypt attachment bytes: {err}")))?;
    Ok(EncryptedAttachment {
        ciphertext,
        file_key,
        enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL.to_string(),
    })
}

pub fn encode_attachment_blob_key(
    list_key: &SymmetricKey,
    blob_ref: &AttachmentBlobRef,
) -> PublicResult<Vec<u8>> {
    validate_attachment_blob_ref(blob_ref)?;
    let value = AttachmentBlobRefWire {
        version: blob_ref.version,
        ciphertext_bytes: blob_ref.ciphertext_bytes,
        file_key: &blob_ref.file_key,
        enc_context: &blob_ref.enc_context,
    };
    encrypt_sealed_payload(
        &value,
        list_key,
        ATTACHMENT_REF_CONTEXT,
        "failed to seal attachment reference",
    )
    .map(|payload| payload.bytes)
}

pub fn build_task_attachment_ref(
    id: uuid::Uuid,
    file_name: String,
    content_type: String,
    size_bytes: u64,
    blob_key: Vec<u8>,
    created_by_membership_id: uuid::Uuid,
) -> FlexibleValue {
    FlexibleValue::Map(vec![
        (
            FlexibleValue::Text("id".to_string()),
            FlexibleValue::Text(id.to_string()),
        ),
        (
            FlexibleValue::Text("file_name".to_string()),
            FlexibleValue::Text(file_name),
        ),
        (
            FlexibleValue::Text("content_type".to_string()),
            FlexibleValue::Text(content_type),
        ),
        (
            FlexibleValue::Text("size_bytes".to_string()),
            FlexibleValue::Integer(size_bytes.into()),
        ),
        (
            FlexibleValue::Text("blob_key".to_string()),
            FlexibleValue::Bytes(blob_key),
        ),
        (
            FlexibleValue::Text("created_by_membership_id".to_string()),
            FlexibleValue::Text(created_by_membership_id.to_string()),
        ),
    ])
}

pub fn decrypt_attachment_bytes(
    ciphertext: &[u8],
    file_key: &[u8],
    enc_context: Option<&str>,
) -> PublicResult<Vec<u8>> {
    let file_key = symmetric_key_from_bytes(file_key)?;
    let context = enc_context.unwrap_or(ATTACHMENT_BLOB_CONTEXT_LABEL);
    decrypt_raw_attachment_bytes(&file_key, ciphertext, context.as_bytes()).or_else(|raw_err| {
        decrypt_sealed_bytes(
            &file_key,
            ciphertext,
            context.as_bytes(),
            "failed to decrypt attachment bytes",
        )
        .map_err(|sealed_err| {
            PublicError::crypto(format!(
                "failed to decrypt attachment bytes as raw StrongBox ciphertext ({raw_err}); also failed wrapped payload fallback ({sealed_err})"
            ))
        })
    })
}

fn decrypt_raw_attachment_bytes(
    key: &SymmetricKey,
    ciphertext: &[u8],
    context: &[u8],
) -> PublicResult<Vec<u8>> {
    StrongBoxKeyRing::new(key.clone())
        .strong_box()
        .decrypt(ciphertext, context)
        .map_err(|err| PublicError::crypto(format!("failed to decrypt attachment bytes: {err}")))
}

fn default_attachment_blob_context_label() -> String {
    ATTACHMENT_BLOB_CONTEXT_LABEL.to_string()
}

fn validate_attachment_blob_ref(blob_ref: &AttachmentBlobRef) -> PublicResult<()> {
    if blob_ref.version != ATTACHMENT_BLOB_REF_VERSION {
        return Err(PublicError::validation(format!(
            "unsupported attachment blob reference version {}",
            blob_ref.version
        )));
    }
    if blob_ref.ciphertext_bytes == 0 || blob_ref.ciphertext_bytes > MAX_ATTACHMENT_CIPHERTEXT_BYTES
    {
        return Err(PublicError::validation(
            "attachment blob reference ciphertext size is invalid",
        ));
    }
    symmetric_key_from_bytes(&blob_ref.file_key)?;
    if blob_ref.enc_context.trim().is_empty() {
        return Err(PublicError::validation(
            "attachment blob reference encryption context cannot be empty",
        ));
    }
    Ok(())
}
