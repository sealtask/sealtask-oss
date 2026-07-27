#![cfg_attr(test, allow(clippy::unwrap_used))]

mod attachment_transport_limits;
mod attachments;
mod notes;
mod read_cache;
mod task_references;

pub use attachment_transport_limits::{
    MAX_ATTACHMENT_CIPHERTEXT_BYTES, MAX_ATTACHMENT_PLAINTEXT_BYTES,
};
pub use attachments::{
    ATTACHMENT_BLOB_CONTEXT, ATTACHMENT_BLOB_CONTEXT_LABEL, ATTACHMENT_BLOB_REF_VERSION,
    ATTACHMENT_REF_CONTEXT, AttachmentBlobRef, EncryptedAttachment, build_task_attachment_ref,
    decode_attachment_blob_key, decrypt_attachment_bytes, encode_attachment_blob_key,
    encrypt_attachment_bytes,
};
pub use notes::{
    NOTE_KEY_CONTEXT, NOTE_PAYLOAD_CONTEXT, NOTE_TITLE_CONTEXT, NotePayloadBody,
    NotePayloadEnvelope, build_note_payload_envelope, decrypt_note_key, decrypt_note_payload,
    encrypt_note_key, encrypt_note_payload,
};
pub use read_cache::{
    MAX_READ_CACHE_CIPHERTEXT_BYTES, MAX_READ_CACHE_PLAINTEXT_BYTES, ReadCacheBinding,
    open_read_cache, seal_read_cache,
};
pub use task_references::{
    ParsedTaskReference, TASK_REFERENCE_ORDINARY_REVISION_MAX, TASK_REFERENCE_PREFIX_MAX_BYTES,
    TASK_REFERENCE_PREFIX_MIN_BYTES, TASK_REFERENCE_REPAIR_REVISION_MAX,
    TASK_REFERENCE_REVISION_MAX, TASK_REFERENCE_SAFE_INTEGER_MAX,
    TASK_REFERENCE_SCHEME_AEAD_CIPHERTEXT_BYTES, TASK_REFERENCE_SCHEME_CONTEXT,
    TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES, TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES,
    TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES, TaskReferenceSchemeV1, decrypt_task_reference_scheme,
    encrypt_task_reference_scheme, parse_project_reference_number, parse_task_reference,
};

use std::{fmt, io::Cursor};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::Sha256;
use strong_box::{Key as StrongBoxKey, StaticStrongBox, StrongBox};
use zeroize::Zeroizing;

use sealtask_client_core::{PublicError, PublicResult};

pub const USER_DATA_KEY_CONTEXT: &[u8] = b"worklist.user.data_key";
pub const USER_DATA_KEY_OPAQUE_CONTEXT: &[u8] = b"worklist.user.data_key.v2.opaque_export";
pub const USER_DATA_KEY_OPAQUE_WRAP_INFO: &[u8] = b"worklist.user.data_key.wrap.v2";
pub const WORK_LIST_PAYLOAD_CONTEXT: &[u8] = b"worklist.work_list.v1";
pub const WORK_LIST_MEMBERSHIP_CONTEXT: &[u8] = b"worklist.membership";
pub const TASK_PAYLOAD_CONTEXT: &[u8] = b"worklist.task.v1";
pub const TASK_TITLE_CONTEXT: &[u8] = b"worklist.task.title.v1";
pub const COMMENT_PAYLOAD_CONTEXT: &[u8] = b"worklist.comment.v1";
pub const DATA_KEY_SALT_BYTES: usize = 32;
pub const KEY_SIZE: usize = 32;
pub const OPAQUE_EXPORT_KEY_BYTES: usize = 64;
const LEGACY_DATA_KEY_PAYLOAD_VERSION: u8 = 1;
const OPAQUE_DATA_KEY_PAYLOAD_VERSION: u8 = 2;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DataKeyCiphertextVersion {
    LegacyPasswordV1,
    OpaqueExportKeyV2,
}

impl TryFrom<u8> for DataKeyCiphertextVersion {
    type Error = PublicError;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            LEGACY_DATA_KEY_PAYLOAD_VERSION => Ok(Self::LegacyPasswordV1),
            OPAQUE_DATA_KEY_PAYLOAD_VERSION => Ok(Self::OpaqueExportKeyV2),
            version => Err(PublicError::validation(format!(
                "unsupported data key ciphertext version {version}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoCapability {
    DataKeyUnwrap,
    WorkListKeyDecrypt,
    PayloadSeal,
    PayloadProof,
}

impl CryptoCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DataKeyUnwrap => "data_key_unwrap",
            Self::WorkListKeyDecrypt => "work_list_key_decrypt",
            Self::PayloadSeal => "payload_seal",
            Self::PayloadProof => "payload_proof",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SymmetricKey(Zeroizing<[u8; KEY_SIZE]>);

impl SymmetricKey {
    pub fn new(bytes: [u8; KEY_SIZE]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub fn from_slice(bytes: &[u8]) -> PublicResult<Self> {
        symmetric_key_from_bytes(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.0
    }

    fn to_strong_box_key(&self) -> StrongBoxKey {
        StrongBoxKey::from(Box::new(*self.as_bytes()))
    }

    fn from_zeroizing(bytes: Zeroizing<[u8; KEY_SIZE]>) -> Self {
        Self(bytes)
    }
}

impl fmt::Debug for SymmetricKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SymmetricKey").field(&"<redacted>").finish()
    }
}

#[derive(Clone, Default)]
pub struct KeyDerivationService {
    argon2: Argon2<'static>,
}

impl KeyDerivationService {
    pub fn new() -> Self {
        let params = Params::new(64 * 1024, 3, 1, None).expect("frontend-compatible argon2 params");
        Self {
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }

    pub fn derive_master_key(
        &self,
        secret: impl AsRef<[u8]>,
        salt: &[u8],
    ) -> PublicResult<SymmetricKey> {
        if salt.len() < 8 {
            return Err(PublicError::validation("salt must be at least 8 bytes"));
        }

        let mut output = Zeroizing::new([0u8; KEY_SIZE]);
        self.argon2
            .hash_password_into(secret.as_ref(), salt, &mut output[..])
            .map_err(|err| PublicError::crypto(format!("argon2id derivation failed: {err}")))?;

        Ok(SymmetricKey::from_zeroizing(output))
    }
}

#[derive(Clone, Debug)]
pub struct StrongBoxKeyRing {
    encryption: SymmetricKey,
    history: Vec<SymmetricKey>,
}

impl StrongBoxKeyRing {
    pub fn new(current: SymmetricKey) -> Self {
        Self {
            encryption: current,
            history: Vec::new(),
        }
    }

    pub fn strong_box(&self) -> StaticStrongBox {
        let enc = self.encryption.to_strong_box_key();
        let mut dec_keys: Vec<StrongBoxKey> = Vec::with_capacity(self.history.len() + 1);
        dec_keys.push(self.encryption.to_strong_box_key());
        dec_keys.extend(self.history.iter().map(SymmetricKey::to_strong_box_key));
        StaticStrongBox::new(enc, dec_keys)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedPayload {
    pub version: u8,
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
}

impl SealedPayload {
    pub const CURRENT_VERSION: u8 = 1;

    pub fn new(ciphertext: Vec<u8>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            ciphertext,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> PublicResult<Self> {
        deserialize_from_cbor(bytes)
    }

    pub fn to_bytes(&self) -> PublicResult<Vec<u8>> {
        serialize_to_cbor(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedBlobPayload {
    pub bytes: Vec<u8>,
    pub base64: String,
}

pub type FlexibleValue = strong_box::ciborium::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayloadEnvelope {
    pub kind: String,
    pub version: u8,
    pub body: TaskPayloadBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayloadBody {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rich_text: Option<TaskPayloadRichText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checklist: Option<Vec<ChecklistItemPayload>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<FlexibleValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<FlexibleValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_meta: Option<FlexibleValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_state: Option<FlexibleValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayloadRichText {
    pub format: String,
    pub version: u8,
    pub blocks: Vec<RichTextBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichTextBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPayloadEnvelope {
    pub kind: String,
    pub version: u8,
    pub body: CommentPayloadBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPayloadBody {
    pub content: TaskPayloadRichText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<FlexibleValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_meta: Option<FlexibleValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItemPayload {
    pub id: String,
    pub title: String,
    pub is_done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_user_ids: Option<Vec<String>>,
}

pub fn derive_child_key(
    parent: &SymmetricKey,
    purpose: impl AsRef<[u8]>,
) -> PublicResult<SymmetricKey> {
    let mut okm = Zeroizing::new([0u8; KEY_SIZE]);
    let hkdf = Hkdf::<Sha256>::new(None, parent.as_bytes());
    hkdf.expand(purpose.as_ref(), &mut okm[..])
        .map_err(|err| PublicError::crypto(format!("hkdf expansion failed: {err}")))?;
    Ok(SymmetricKey::from_zeroizing(okm))
}

pub fn derive_work_list_key(
    data_key: &SymmetricKey,
    work_list_id: &uuid::Uuid,
) -> PublicResult<SymmetricKey> {
    derive_child_key(data_key, format!("worklist:{work_list_id}"))
}

pub fn derive_payload_binding_key(list_key: &SymmetricKey) -> PublicResult<SymmetricKey> {
    derive_child_key(list_key, "member:payload-binding")
}

pub fn decrypt_user_data_key(
    password: &str,
    data_key_ciphertext_b64: &str,
) -> PublicResult<SymmetricKey> {
    let payload = decode_data_key_payload(data_key_ciphertext_b64)?;
    match DataKeyCiphertextVersion::try_from(payload.version)? {
        DataKeyCiphertextVersion::LegacyPasswordV1 => {}
        DataKeyCiphertextVersion::OpaqueExportKeyV2 => {
            return Err(PublicError::validation(
                "data key ciphertext version 2 requires an OPAQUE export key",
            ));
        }
    }

    if payload.ciphertext.len() <= DATA_KEY_SALT_BYTES {
        return Err(PublicError::validation("data key payload is truncated"));
    }

    let (salt, sealed) = payload.ciphertext.split_at(DATA_KEY_SALT_BYTES);
    let key_derivation = KeyDerivationService::new();
    let wrapping_key = key_derivation.derive_master_key(password.as_bytes(), salt)?;

    let strong_box = StrongBoxKeyRing::new(wrapping_key).strong_box();
    let data_key = Zeroizing::new(
        strong_box
            .decrypt(sealed, USER_DATA_KEY_CONTEXT)
            .map_err(|err| PublicError::crypto(format!("failed to decrypt data key: {err}")))?,
    );

    symmetric_key_from_bytes(&data_key)
}

pub fn data_key_ciphertext_version(
    data_key_ciphertext_b64: &str,
) -> PublicResult<DataKeyCiphertextVersion> {
    DataKeyCiphertextVersion::try_from(decode_data_key_payload(data_key_ciphertext_b64)?.version)
}

pub fn decrypt_user_data_key_with_opaque_export_key(
    export_key: &[u8],
    data_key_ciphertext_b64: &str,
) -> PublicResult<SymmetricKey> {
    let payload = decode_data_key_payload(data_key_ciphertext_b64)?;
    match DataKeyCiphertextVersion::try_from(payload.version)? {
        DataKeyCiphertextVersion::OpaqueExportKeyV2 => {}
        DataKeyCiphertextVersion::LegacyPasswordV1 => {
            return Err(PublicError::validation(
                "data key ciphertext version 1 requires legacy password migration",
            ));
        }
    }
    if payload.ciphertext.is_empty() {
        return Err(PublicError::validation("data key payload is empty"));
    }

    let wrapping_key = derive_opaque_data_key_wrapping_key(export_key)?;
    let strong_box = StrongBoxKeyRing::new(wrapping_key).strong_box();
    let data_key = Zeroizing::new(
        strong_box
            .decrypt(&payload.ciphertext, USER_DATA_KEY_OPAQUE_CONTEXT)
            .map_err(|err| PublicError::crypto(format!("failed to decrypt data key: {err}")))?,
    );

    symmetric_key_from_bytes(&data_key)
}

pub fn decrypt_work_list_key(
    data_key: &SymmetricKey,
    work_list_key_ciphertext: &[u8],
) -> PublicResult<SymmetricKey> {
    let plaintext = decrypt_sealed_bytes(
        data_key,
        work_list_key_ciphertext,
        WORK_LIST_MEMBERSHIP_CONTEXT,
        "failed to decrypt work list key",
    )?;
    let key_bytes = decode_work_list_key_bytes(&plaintext)?;
    symmetric_key_from_bytes(&key_bytes)
}

pub fn decrypt_work_list_payload<T: DeserializeOwned>(
    list_key: &SymmetricKey,
    payload_ciphertext: &[u8],
) -> PublicResult<T> {
    decrypt_sealed_payload(
        list_key,
        payload_ciphertext,
        WORK_LIST_PAYLOAD_CONTEXT,
        "failed to decrypt payload",
    )
}

pub fn decrypt_task_payload(
    list_key: &SymmetricKey,
    payload_ciphertext: &[u8],
) -> PublicResult<TaskPayloadEnvelope> {
    decrypt_sealed_payload(
        list_key,
        payload_ciphertext,
        TASK_PAYLOAD_CONTEXT,
        "failed to decrypt task payload",
    )
}

pub fn decrypt_comment_payload(
    list_key: &SymmetricKey,
    payload_ciphertext: &[u8],
) -> PublicResult<CommentPayloadEnvelope> {
    decrypt_sealed_payload(
        list_key,
        payload_ciphertext,
        COMMENT_PAYLOAD_CONTEXT,
        "failed to decrypt comment payload",
    )
}

pub fn generate_symmetric_key() -> PublicResult<SymmetricKey> {
    generate_symmetric_key_with_rng(&mut OsRng)
}

fn generate_symmetric_key_with_rng(rng: &mut impl RngCore) -> PublicResult<SymmetricKey> {
    let mut bytes = Zeroizing::new([0u8; KEY_SIZE]);
    rng.try_fill_bytes(&mut bytes[..])
        .map_err(|err| PublicError::crypto(format!("failed to generate symmetric key: {err}")))?;
    Ok(SymmetricKey::from_zeroizing(bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextValuePayload {
    value: String,
}

pub fn decrypt_text_value(payload_ciphertext: &[u8]) -> PublicResult<String> {
    let sealed = SealedPayload::from_bytes(payload_ciphertext)?;
    ensure_payload_version(sealed.version)?;
    let payload: TextValuePayload = deserialize_from_cbor(&sealed.ciphertext)?;
    Ok(payload.value)
}

pub fn encrypt_text_value(
    value: &str,
    key: &SymmetricKey,
    context: &[u8],
) -> PublicResult<SealedBlobPayload> {
    encrypt_sealed_payload(
        &TextValuePayload {
            value: value.to_string(),
        },
        key,
        context,
        "failed to seal text value",
    )
}

pub fn decrypt_encrypted_text_value(
    payload_ciphertext: &[u8],
    key: &SymmetricKey,
    context: &[u8],
) -> PublicResult<String> {
    let payload: TextValuePayload = decrypt_sealed_payload(
        key,
        payload_ciphertext,
        context,
        "failed to decrypt text value",
    )?;
    Ok(payload.value)
}

pub fn flexible_value_to_json(value: FlexibleValue) -> serde_json::Value {
    match value {
        FlexibleValue::Integer(value) => integer_to_json_value(value),
        FlexibleValue::Bytes(bytes) => {
            serde_json::Value::Array(bytes.into_iter().map(serde_json::Value::from).collect())
        }
        FlexibleValue::Float(value) => serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(value.to_string())),
        FlexibleValue::Text(value) => serde_json::Value::String(value),
        FlexibleValue::Bool(value) => serde_json::Value::Bool(value),
        FlexibleValue::Null => serde_json::Value::Null,
        FlexibleValue::Tag(tag, value) => serde_json::json!({
            "tag": tag,
            "value": flexible_value_to_json(*value),
        }),
        FlexibleValue::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(flexible_value_to_json).collect())
        }
        FlexibleValue::Map(entries) => serde_json::Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    (
                        flexible_map_key_to_string(key),
                        flexible_value_to_json(value),
                    )
                })
                .collect(),
        ),
        other => serde_json::Value::String(format!("{other:?}")),
    }
}

pub fn json_value_to_flexible(value: serde_json::Value) -> FlexibleValue {
    match value {
        serde_json::Value::Null => FlexibleValue::Null,
        serde_json::Value::Bool(value) => FlexibleValue::Bool(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                FlexibleValue::Integer(value.into())
            } else if let Some(value) = value.as_u64() {
                FlexibleValue::Integer(value.into())
            } else {
                FlexibleValue::Float(
                    value
                        .as_f64()
                        .expect("serde_json number should deserialize as f64"),
                )
            }
        }
        serde_json::Value::String(value) => FlexibleValue::Text(value),
        serde_json::Value::Array(values) => {
            FlexibleValue::Array(values.into_iter().map(json_value_to_flexible).collect())
        }
        serde_json::Value::Object(values) => FlexibleValue::Map(
            values
                .into_iter()
                .map(|(key, value)| (FlexibleValue::Text(key), json_value_to_flexible(value)))
                .collect(),
        ),
    }
}

pub fn build_task_payload_envelope(body: TaskPayloadBody, version: u8) -> TaskPayloadEnvelope {
    TaskPayloadEnvelope {
        kind: "task".to_string(),
        version,
        body,
    }
}

fn integer_to_json_value(value: strong_box::ciborium::value::Integer) -> serde_json::Value {
    let value = i128::from(value);
    if let Ok(value) = i64::try_from(value) {
        return serde_json::Value::from(value);
    }
    if let Ok(value) = u64::try_from(value) {
        return serde_json::Value::from(value);
    }

    serde_json::Value::String(value.to_string())
}

fn flexible_map_key_to_string(value: FlexibleValue) -> String {
    match value {
        FlexibleValue::Text(value) => value,
        other => flexible_value_to_json(other).to_string(),
    }
}

pub fn build_comment_payload_envelope(
    body: CommentPayloadBody,
    version: u8,
) -> CommentPayloadEnvelope {
    CommentPayloadEnvelope {
        kind: "comment".to_string(),
        version,
        body,
    }
}

pub fn plaintext_rich_text(markdown: &str) -> Option<TaskPayloadRichText> {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(TaskPayloadRichText {
        format: "markdown".to_string(),
        version: 1,
        blocks: vec![RichTextBlock {
            block_type: "paragraph".to_string(),
            text: trimmed.to_string(),
        }],
    })
}

pub fn encrypt_task_payload(
    envelope: &TaskPayloadEnvelope,
    list_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    encrypt_sealed_payload(
        envelope,
        list_key,
        TASK_PAYLOAD_CONTEXT,
        "failed to seal task payload",
    )
}

pub fn encrypt_comment_payload(
    envelope: &CommentPayloadEnvelope,
    list_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    encrypt_sealed_payload(
        envelope,
        list_key,
        COMMENT_PAYLOAD_CONTEXT,
        "failed to seal comment payload",
    )
}

pub fn seal_text_value(value: &str) -> PublicResult<SealedBlobPayload> {
    let ciphertext = serialize_to_cbor(&serde_json::json!({ "value": value }))?;
    sealed_blob_from_payload(SealedPayload::new(ciphertext))
}

pub fn compute_payload_proof(
    ciphertext: &[u8],
    binding_key: &SymmetricKey,
) -> PublicResult<String> {
    type PayloadMac = Hmac<Sha256>;

    let mut mac = PayloadMac::new_from_slice(binding_key.as_bytes())
        .map_err(|err| PublicError::crypto(format!("failed to create HMAC: {err}")))?;
    mac.update(ciphertext);
    let bytes = mac.finalize().into_bytes();
    Ok(STANDARD_NO_PAD.encode(bytes))
}

/// Computes an opaque, stable commitment for a logical task-create request.
///
/// StrongBox ciphertext changes on every encryption, so an idempotent retry
/// cannot fingerprint ciphertext bytes. Callers pass a canonical encoding of
/// the readable task semantics instead. A domain-separated key derived from
/// the project key keeps equal plaintext from becoming a cross-project
/// equality or dictionary oracle for the API.
pub fn compute_task_create_semantic_commitment(
    canonical_semantics: &[u8],
    list_key: &SymmetricKey,
) -> PublicResult<String> {
    type SemanticMac = Hmac<Sha256>;

    let commitment_key =
        derive_child_key(list_key, "sealtask.task-create.semantic-commitment.key.v1")?;
    let mut mac = SemanticMac::new_from_slice(commitment_key.as_bytes()).map_err(|err| {
        PublicError::crypto(format!(
            "failed to create task idempotency commitment HMAC: {err}"
        ))
    })?;
    mac.update(b"sealtask.task-create.semantic-commitment.payload.v1\0");
    mac.update(canonical_semantics);
    Ok(STANDARD_NO_PAD.encode(mac.finalize().into_bytes()))
}

/// Derives the server-visible idempotency key for an implicit batch task create.
///
/// The canonical input digest and operation ID are local equality-sensitive
/// material. A project-key-derived HMAC keeps them opaque to the API while
/// preserving deterministic retries for the same project and canonical batch.
pub fn derive_batch_task_create_idempotency_key(
    canonical_input_sha256: &[u8; 32],
    operation_id: &str,
    work_list_id: &uuid::Uuid,
    list_key: &SymmetricKey,
) -> PublicResult<String> {
    type IdempotencyMac = Hmac<Sha256>;

    let derivation_key = derive_child_key(
        list_key,
        "sealtask.batch.task-create.idempotency-key.key.v1",
    )?;
    let mut mac = IdempotencyMac::new_from_slice(derivation_key.as_bytes()).map_err(|err| {
        PublicError::crypto(format!(
            "failed to create batch task idempotency HMAC: {err}"
        ))
    })?;
    mac.update(b"sealtask.batch.task-create.idempotency-key.payload.v1\0");
    mac.update(work_list_id.as_bytes());
    mac.update(canonical_input_sha256);
    mac.update(&(operation_id.len() as u64).to_be_bytes());
    mac.update(operation_id.as_bytes());
    Ok(format!(
        "batch:v2:{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    ))
}

/// Computes an opaque, stable commitment for a logical note-create request.
///
/// The list-key-derived MAC lets a retry use freshly randomized ciphertext
/// while proving that its readable note semantics are unchanged.
pub fn compute_note_create_semantic_commitment(
    canonical_semantics: &[u8],
    list_key: &SymmetricKey,
) -> PublicResult<String> {
    type SemanticMac = Hmac<Sha256>;

    let commitment_key =
        derive_child_key(list_key, "sealtask.note-create.semantic-commitment.key.v1")?;
    let mut mac = SemanticMac::new_from_slice(commitment_key.as_bytes()).map_err(|err| {
        PublicError::crypto(format!(
            "failed to create note idempotency commitment HMAC: {err}"
        ))
    })?;
    mac.update(b"sealtask.note-create.semantic-commitment.payload.v1\0");
    mac.update(canonical_semantics);
    Ok(STANDARD_NO_PAD.encode(mac.finalize().into_bytes()))
}

pub fn decode_sealed_blob(b64: &str) -> PublicResult<Vec<u8>> {
    decode_base64(b64)
}

pub fn deserialize_from_cbor<T: DeserializeOwned>(bytes: &[u8]) -> PublicResult<T> {
    let mut cursor = Cursor::new(bytes);
    let value = strong_box::ciborium::de::from_reader(&mut cursor)
        .map_err(|err| PublicError::crypto(format!("failed to deserialize payload: {err}")))?;
    if cursor.position()
        != u64::try_from(bytes.len())
            .map_err(|_| PublicError::validation("CBOR payload is too large"))?
    {
        return Err(PublicError::validation(
            "CBOR payload must contain exactly one value",
        ));
    }
    Ok(value)
}

pub fn serialize_to_cbor<T: Serialize + ?Sized>(value: &T) -> PublicResult<Vec<u8>> {
    let mut buffer = Vec::new();
    strong_box::ciborium::ser::into_writer(value, &mut buffer)
        .map_err(|err| PublicError::crypto(format!("failed to serialize payload: {err}")))?;
    Ok(buffer)
}

fn decode_base64(value: &str) -> PublicResult<Vec<u8>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PublicError::validation("ciphertext cannot be empty"));
    }

    STANDARD_NO_PAD
        .decode(trimmed.as_bytes())
        .or_else(|_| STANDARD.decode(trimmed.as_bytes()))
        .or_else(|_| URL_SAFE_NO_PAD.decode(trimmed.as_bytes()))
        .or_else(|_| URL_SAFE.decode(trimmed.as_bytes()))
        .map_err(|err| PublicError::validation(format!("ciphertext must be base64: {err}")))
}

fn decrypt_sealed_payload<T: DeserializeOwned>(
    key: &SymmetricKey,
    payload_ciphertext: &[u8],
    context: &[u8],
    error_context: &str,
) -> PublicResult<T> {
    let plaintext = Zeroizing::new(decrypt_sealed_bytes(
        key,
        payload_ciphertext,
        context,
        error_context,
    )?);
    deserialize_from_cbor(&plaintext)
}

fn decrypt_sealed_bytes(
    key: &SymmetricKey,
    payload_ciphertext: &[u8],
    context: &[u8],
    error_context: &str,
) -> PublicResult<Vec<u8>> {
    let sealed = SealedPayload::from_bytes(payload_ciphertext)?;
    ensure_payload_version(sealed.version)?;

    StrongBoxKeyRing::new(key.clone())
        .strong_box()
        .decrypt(&sealed.ciphertext, context)
        .map_err(|err| PublicError::crypto(format!("{error_context}: {err}")))
}

fn encrypt_sealed_payload<T: Serialize + ?Sized>(
    value: &T,
    key: &SymmetricKey,
    context: &[u8],
    error_context: &str,
) -> PublicResult<SealedBlobPayload> {
    let plaintext = Zeroizing::new(serialize_to_cbor(value)?);
    let ciphertext = StrongBoxKeyRing::new(key.clone())
        .strong_box()
        .encrypt(plaintext, context)
        .map_err(|err| PublicError::crypto(format!("{error_context}: {err}")))?;
    sealed_blob_from_payload(SealedPayload::new(ciphertext))
}

fn encrypt_sealed_bytes(
    plaintext: &[u8],
    key: &SymmetricKey,
    context: &[u8],
    error_context: &str,
) -> PublicResult<SealedBlobPayload> {
    let ciphertext = StrongBoxKeyRing::new(key.clone())
        .strong_box()
        .encrypt(plaintext, context)
        .map_err(|err| PublicError::crypto(format!("{error_context}: {err}")))?;
    sealed_blob_from_payload(SealedPayload::new(ciphertext))
}

fn symmetric_key_from_bytes(bytes: &[u8]) -> PublicResult<SymmetricKey> {
    if bytes.len() != KEY_SIZE {
        return Err(PublicError::validation(format!(
            "expected {KEY_SIZE}-byte key, got {} bytes",
            bytes.len()
        )));
    }

    let mut array = Zeroizing::new([0u8; KEY_SIZE]);
    array.copy_from_slice(bytes);
    Ok(SymmetricKey::from_zeroizing(array))
}

fn decode_data_key_payload(data_key_ciphertext_b64: &str) -> PublicResult<SealedPayload> {
    let bytes = decode_base64(data_key_ciphertext_b64)?;
    SealedPayload::from_bytes(&bytes)
}

fn derive_opaque_data_key_wrapping_key(export_key: &[u8]) -> PublicResult<SymmetricKey> {
    if export_key.len() != OPAQUE_EXPORT_KEY_BYTES {
        return Err(PublicError::validation(format!(
            "OPAQUE export key must be {OPAQUE_EXPORT_KEY_BYTES} bytes"
        )));
    }

    let mut output = Zeroizing::new([0u8; KEY_SIZE]);
    Hkdf::<Sha256>::new(None, export_key)
        .expand(USER_DATA_KEY_OPAQUE_WRAP_INFO, &mut output[..])
        .map_err(|err| PublicError::crypto(format!("OPAQUE export-key HKDF failed: {err}")))?;
    Ok(SymmetricKey::from_zeroizing(output))
}

fn ensure_payload_version(version: u8) -> PublicResult<()> {
    if version != SealedPayload::CURRENT_VERSION {
        return Err(PublicError::validation(format!(
            "unsupported sealed payload version {version}"
        )));
    }
    Ok(())
}

fn sealed_blob_from_payload(payload: SealedPayload) -> PublicResult<SealedBlobPayload> {
    let bytes = payload.to_bytes()?;
    Ok(SealedBlobPayload {
        base64: STANDARD_NO_PAD.encode(&bytes),
        bytes,
    })
}

fn decode_work_list_key_bytes(plaintext: &[u8]) -> PublicResult<Vec<u8>> {
    let Some(bytes) = try_decode_envelope(plaintext)? else {
        return if plaintext.is_empty() {
            Err(PublicError::validation("work list key cannot be empty"))
        } else {
            Ok(plaintext.to_vec())
        };
    };

    if bytes.is_empty() {
        return Err(PublicError::validation("work list key cannot be empty"));
    }

    Ok(bytes)
}

fn try_decode_envelope(bytes: &[u8]) -> PublicResult<Option<Vec<u8>>> {
    if let Ok(envelope) = deserialize_from_cbor::<WorkListKeyEnvelope>(bytes) {
        let bytes = match envelope.key {
            WorkListKeyField::Bytes(data) => data,
            WorkListKeyField::Text(text) => decode_membership_key_string(&text)?,
        };
        return Ok(Some(bytes));
    }

    if let Ok(raw_bytes) = deserialize_from_cbor::<Vec<u8>>(bytes) {
        return Ok(Some(raw_bytes));
    }

    Ok(None)
}

fn decode_membership_key_string(value: &str) -> PublicResult<Vec<u8>> {
    let normalized = value.trim().replace('-', "+").replace('_', "/");
    if normalized.is_empty() {
        return Err(PublicError::validation(
            "membership key string cannot be empty",
        ));
    }

    STANDARD_NO_PAD
        .decode(normalized.as_bytes())
        .or_else(|_| STANDARD.decode(normalized.as_bytes()))
        .map_err(|err| PublicError::validation(format!("membership key must be base64: {err}")))
}

#[derive(Deserialize)]
struct WorkListKeyEnvelope {
    key: WorkListKeyField,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WorkListKeyField {
    Bytes(Vec<u8>),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GoldenSealedPayload {
        version: u8,
        ciphertext: Vec<u8>,
        cbor_base64_no_pad: String,
    }

    #[test]
    fn sealed_payload_matches_shared_canonical_cbor_vector() {
        let vector: GoldenSealedPayload =
            serde_json::from_str(include_str!("../testdata/sealed-payload-v1.json"))
                .expect("shared sealed-payload vector");
        let expected = STANDARD_NO_PAD
            .decode(vector.cbor_base64_no_pad)
            .expect("golden CBOR");
        let payload = SealedPayload {
            version: vector.version,
            ciphertext: vector.ciphertext.clone(),
        };

        assert_eq!(payload.to_bytes().expect("encode sealed payload"), expected);
        assert_eq!(
            SealedPayload::from_bytes(&expected)
                .expect("decode sealed payload")
                .ciphertext,
            vector.ciphertext
        );

        let value: FlexibleValue =
            deserialize_from_cbor(&expected).expect("inspect golden CBOR envelope");
        let ciphertext = match value {
            FlexibleValue::Map(fields) => fields.into_iter().find_map(|(key, value)| {
                matches!(key, FlexibleValue::Text(ref key) if key == "ciphertext").then_some(value)
            }),
            _ => None,
        };
        assert!(
            matches!(ciphertext, Some(FlexibleValue::Bytes(_))),
            "ciphertext must use CBOR major type 2"
        );
    }

    struct FailingRng;

    impl RngCore for FailingRng {
        fn next_u32(&mut self) -> u32 {
            0
        }

        fn next_u64(&mut self) -> u64 {
            0
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            dest.fill(0);
        }

        fn try_fill_bytes(&mut self, _dest: &mut [u8]) -> Result<(), rand_core::Error> {
            Err(rand_core::Error::new(std::io::Error::other(
                "entropy unavailable",
            )))
        }
    }

    #[test]
    fn symmetric_key_generation_maps_entropy_failure() {
        let error = generate_symmetric_key_with_rng(&mut FailingRng)
            .expect_err("entropy failure must propagate");
        assert!(
            matches!(error, PublicError::Crypto(message) if message.contains("generate symmetric key"))
        );
    }

    fn encode_opaque_data_key_ciphertext(export_key: &[u8], data_key: &SymmetricKey) -> String {
        let wrapping_key = derive_opaque_data_key_wrapping_key(export_key).expect("wrapping key");
        let ciphertext = StrongBoxKeyRing::new(wrapping_key)
            .strong_box()
            .encrypt(data_key.as_bytes(), USER_DATA_KEY_OPAQUE_CONTEXT)
            .expect("encrypt data key");
        let payload = SealedPayload {
            version: OPAQUE_DATA_KEY_PAYLOAD_VERSION,
            ciphertext,
        }
        .to_bytes()
        .expect("serialize data-key payload");
        STANDARD_NO_PAD.encode(payload)
    }

    #[test]
    fn test_should_redact_symmetric_and_attachment_keys_from_debug_output() {
        let symmetric_key = SymmetricKey::new([0xa5; KEY_SIZE]);
        assert_eq!(format!("{symmetric_key:?}"), "SymmetricKey(\"<redacted>\")");

        let blob_ref = AttachmentBlobRef {
            version: 1,
            ciphertext_bytes: 42,
            file_key: vec![0xf7; KEY_SIZE],
            enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL.to_string(),
        };
        let debug = format!("{blob_ref:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("247"));
    }

    #[test]
    fn test_decode_base64_standard() {
        let encoded = STANDARD.encode(b"hello");
        let decoded = decode_base64(&encoded).expect("decode");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_decode_base64_no_pad() {
        let encoded = STANDARD_NO_PAD.encode(b"hello");
        let decoded = decode_base64(&encoded).expect("decode");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_should_keep_task_create_semantic_commitment_stable_and_project_scoped() {
        let semantics = br#"{"title":"same task"}"#;
        let first_key = SymmetricKey::new([0x11; 32]);
        let second_key = SymmetricKey::new([0x22; 32]);
        let first = compute_task_create_semantic_commitment(semantics, &first_key)
            .expect("first commitment");
        let retry = compute_task_create_semantic_commitment(semantics, &first_key)
            .expect("retry commitment");
        let changed =
            compute_task_create_semantic_commitment(br#"{"title":"different task"}"#, &first_key)
                .expect("changed commitment");
        let other_project = compute_task_create_semantic_commitment(semantics, &second_key)
            .expect("other project commitment");

        assert_eq!(first, retry);
        assert_ne!(first, changed);
        assert_ne!(first, other_project);
        assert_eq!(STANDARD_NO_PAD.decode(first).expect("base64").len(), 32);
    }

    #[test]
    fn batch_task_create_idempotency_is_keyed_stable_and_project_scoped() {
        let digest = [0xab; 32];
        let operation_id = "plaintext-operation-id-canary";
        let first_project = uuid::Uuid::from_u128(1);
        let second_project = uuid::Uuid::from_u128(2);
        let data_key = SymmetricKey::new([0x41; KEY_SIZE]);
        let first_key = derive_work_list_key(&data_key, &first_project).expect("first list key");
        let second_key = derive_work_list_key(&data_key, &second_project).expect("second list key");
        let unrelated_key = SymmetricKey::new([0x42; KEY_SIZE]);

        let first = derive_batch_task_create_idempotency_key(
            &digest,
            operation_id,
            &first_project,
            &first_key,
        )
        .expect("first derivation");
        let retry = derive_batch_task_create_idempotency_key(
            &digest,
            operation_id,
            &first_project,
            &first_key,
        )
        .expect("retry derivation");
        let other_project = derive_batch_task_create_idempotency_key(
            &digest,
            operation_id,
            &second_project,
            &second_key,
        )
        .expect("other project derivation");
        let other_project_same_key = derive_batch_task_create_idempotency_key(
            &digest,
            operation_id,
            &second_project,
            &first_key,
        )
        .expect("other project with same key derivation");
        let other_operation = derive_batch_task_create_idempotency_key(
            &digest,
            "different-operation",
            &first_project,
            &first_key,
        )
        .expect("other operation derivation");
        let other_unlocked_key = derive_batch_task_create_idempotency_key(
            &digest,
            operation_id,
            &first_project,
            &unrelated_key,
        )
        .expect("other unlocked key derivation");

        assert_eq!(first, retry);
        assert_ne!(first, other_project);
        assert_ne!(
            first, other_project_same_key,
            "the project identifier must be part of the keyed payload"
        );
        assert_ne!(first, other_operation);
        assert_ne!(
            first, other_unlocked_key,
            "the server-visible value must depend on secret project key material"
        );
        assert!(first.starts_with("batch:v2:"));
        assert!(first.len() <= 128);
        assert!(first.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        }));
        assert!(!first.contains(operation_id));
        assert!(!first.contains(&"ab".repeat(32)));
        assert!(!first.contains(&URL_SAFE_NO_PAD.encode(digest)));
    }

    #[test]
    fn note_create_semantic_commitment_is_stable_semantic_and_project_scoped() {
        let semantics = br#"{"title":"same note"}"#;
        let first_key = SymmetricKey::new([0x31; 32]);
        let second_key = SymmetricKey::new([0x32; 32]);
        let first = compute_note_create_semantic_commitment(semantics, &first_key)
            .expect("first commitment");
        let retry = compute_note_create_semantic_commitment(semantics, &first_key)
            .expect("retry commitment");
        let changed =
            compute_note_create_semantic_commitment(br#"{"title":"different note"}"#, &first_key)
                .expect("changed commitment");
        let other_project = compute_note_create_semantic_commitment(semantics, &second_key)
            .expect("other project commitment");

        assert_eq!(first, retry);
        assert_ne!(first, changed);
        assert_ne!(first, other_project);
        assert_eq!(STANDARD_NO_PAD.decode(first).expect("base64").len(), 32);
    }

    #[test]
    fn opaque_data_key_hkdf_matches_webcrypto_vector() {
        let export_key = (0u8..OPAQUE_EXPORT_KEY_BYTES as u8).collect::<Vec<_>>();
        let wrapping_key =
            derive_opaque_data_key_wrapping_key(&export_key).expect("derive wrapping key");
        assert_eq!(
            wrapping_key.as_bytes(),
            &[
                0x48, 0x55, 0xbd, 0x65, 0x03, 0x54, 0x3d, 0x19, 0x3f, 0xbf, 0x07, 0x53, 0x22, 0xc8,
                0xbf, 0xcf, 0xd5, 0x23, 0x27, 0x8f, 0x51, 0x15, 0xe0, 0x72, 0xef, 0x00, 0x74, 0xac,
                0xc5, 0x96, 0xda, 0x14,
            ]
        );
    }

    #[test]
    fn opaque_data_key_ciphertext_round_trips_with_v2_context() {
        let export_key = [0x42; OPAQUE_EXPORT_KEY_BYTES];
        let data_key = SymmetricKey::new([0x24; KEY_SIZE]);
        let ciphertext = encode_opaque_data_key_ciphertext(&export_key, &data_key);

        assert_eq!(
            data_key_ciphertext_version(&ciphertext).expect("ciphertext version"),
            DataKeyCiphertextVersion::OpaqueExportKeyV2
        );
        assert_eq!(
            decrypt_user_data_key_with_opaque_export_key(&export_key, &ciphertext)
                .expect("decrypt v2 data key"),
            data_key
        );

        let url_safe = ciphertext
            .replace('+', "-")
            .replace('/', "_")
            .trim_end_matches('=')
            .to_string();
        assert_eq!(
            decrypt_user_data_key_with_opaque_export_key(&export_key, &url_safe)
                .expect("decrypt URL-safe v2 data key"),
            data_key
        );
    }

    #[test]
    fn test_should_reject_unknown_data_key_ciphertext_versions() {
        let payload = SealedPayload {
            version: u8::MAX,
            ciphertext: Vec::new(),
        }
        .to_bytes()
        .expect("serialize unknown-version data-key payload");
        let ciphertext = STANDARD_NO_PAD.encode(payload);

        let error = data_key_ciphertext_version(&ciphertext)
            .expect_err("unknown data-key ciphertext version must fail");

        assert_eq!(
            error.to_string(),
            "unsupported data key ciphertext version 255"
        );
    }

    #[test]
    fn opaque_data_key_ciphertext_decrypts_browser_wasm_fixture() {
        // Generated by frontend/src/crypto/data-key.ts using the production
        // StrongBox WASM bridge, export-key bytes 0..63, and a 0x07 data key.
        const BROWSER_CIPHERTEXT: &str = concat!(
            "uQACZ3ZlcnNpb24CamNpcGhlcnRleHTYQFhUsbj1g1Aue4S+jeDO0MxxVGnAhLyn",
            "TL/tx0msz1eNjbFex1gwawxU+h2qiJmX5bwbxErU6vKrC/EroaWmtmuapj6IN9Ia",
            "9wlfTF8w35B0wX9bKSG+"
        );
        let export_key = (0u8..OPAQUE_EXPORT_KEY_BYTES as u8).collect::<Vec<_>>();

        assert_eq!(
            decrypt_user_data_key_with_opaque_export_key(&export_key, BROWSER_CIPHERTEXT)
                .expect("decrypt browser v2 fixture"),
            SymmetricKey::new([0x07; KEY_SIZE])
        );
    }

    #[test]
    fn opaque_data_key_ciphertext_rejects_wrong_key_version_and_length() {
        let export_key = [0x31; OPAQUE_EXPORT_KEY_BYTES];
        let data_key = SymmetricKey::new([0x13; KEY_SIZE]);
        let ciphertext = encode_opaque_data_key_ciphertext(&export_key, &data_key);

        let wrong_key_error = decrypt_user_data_key_with_opaque_export_key(
            &[0x32; OPAQUE_EXPORT_KEY_BYTES],
            &ciphertext,
        )
        .expect_err("wrong export key must fail");
        assert!(
            wrong_key_error
                .to_string()
                .contains("failed to decrypt data key")
        );

        let short_key_error = decrypt_user_data_key_with_opaque_export_key(
            &[0x31; OPAQUE_EXPORT_KEY_BYTES - 1],
            &ciphertext,
        )
        .expect_err("short export key must fail");
        assert_eq!(
            short_key_error.to_string(),
            "OPAQUE export key must be 64 bytes"
        );

        let legacy_payload = SealedPayload::new(vec![0; DATA_KEY_SALT_BYTES + 1])
            .to_bytes()
            .expect("legacy payload");
        let legacy_ciphertext = STANDARD_NO_PAD.encode(legacy_payload);
        let version_error =
            decrypt_user_data_key_with_opaque_export_key(&export_key, &legacy_ciphertext)
                .expect_err("legacy wrapper must use legacy path");
        assert!(
            version_error
                .to_string()
                .contains("legacy password migration")
        );
    }

    #[test]
    fn test_derive_child_key_is_deterministic() {
        let key_derivation = KeyDerivationService::new();
        let master = key_derivation
            .derive_master_key("secret", b"salty-salt")
            .expect("master key");
        let child_a = derive_child_key(&master, "sealtask:test").expect("child");
        let child_b = derive_child_key(&master, "sealtask:test").expect("child");
        assert_eq!(child_a, child_b);
    }

    #[test]
    fn decrypt_attachment_bytes_supports_raw_strongbox_ciphertext() {
        let file_key = SymmetricKey::new([9; KEY_SIZE]);
        let plaintext = b"attachment body";
        let ciphertext = StrongBoxKeyRing::new(file_key.clone())
            .strong_box()
            .encrypt(plaintext.as_slice(), ATTACHMENT_BLOB_CONTEXT)
            .expect("encrypt attachment");

        let decrypted = decrypt_attachment_bytes(
            &ciphertext,
            file_key.as_bytes(),
            Some(ATTACHMENT_BLOB_CONTEXT_LABEL),
        )
        .expect("decrypt attachment");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_attachment_bytes_keeps_wrapped_payload_fallback() {
        let file_key = SymmetricKey::new([11; KEY_SIZE]);
        let plaintext = b"wrapped attachment body";
        let raw_ciphertext = StrongBoxKeyRing::new(file_key.clone())
            .strong_box()
            .encrypt(plaintext.as_slice(), ATTACHMENT_BLOB_CONTEXT)
            .expect("encrypt attachment");
        let wrapped_ciphertext = SealedPayload::new(raw_ciphertext)
            .to_bytes()
            .expect("wrap attachment");

        let decrypted = decrypt_attachment_bytes(
            &wrapped_ciphertext,
            file_key.as_bytes(),
            Some(ATTACHMENT_BLOB_CONTEXT_LABEL),
        )
        .expect("decrypt attachment");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn private_note_keys_payloads_and_titles_round_trip() {
        let data_key = SymmetricKey::new([0x21; KEY_SIZE]);
        let note_key = generate_symmetric_key().expect("generate note key");
        let wrapped_key = encrypt_note_key(&note_key, &data_key).expect("wrap note key");
        let unwrapped_key =
            decrypt_note_key(&wrapped_key.bytes, &data_key).expect("unwrap note key");
        assert_eq!(unwrapped_key, note_key);

        let body = NotePayloadBody {
            title: "Private note".to_string(),
            content: TaskPayloadRichText {
                format: "markdown".to_string(),
                version: 1,
                blocks: vec![RichTextBlock {
                    block_type: "paragraph".to_string(),
                    text: "Encrypted body".to_string(),
                }],
            },
            mentions: Some(Vec::new()),
            attachments: Some(Vec::new()),
            client_meta: Some(FlexibleValue::Map(Vec::new())),
        };
        let envelope = build_note_payload_envelope(body, 1);
        let encrypted = encrypt_note_payload(&envelope, &note_key).expect("encrypt note");
        let decrypted = decrypt_note_payload(&note_key, &encrypted.bytes).expect("decrypt note");
        assert_eq!(decrypted.body.title, "Private note");
        assert_eq!(decrypted.body.content.blocks[0].text, "Encrypted body");

        let title =
            encrypt_text_value("Private note", &note_key, NOTE_TITLE_CONTEXT).expect("title");
        assert_eq!(
            decrypt_encrypted_text_value(&title.bytes, &note_key, NOTE_TITLE_CONTEXT)
                .expect("decrypt title"),
            "Private note"
        );
    }

    #[test]
    fn attachment_encryption_and_wrapped_reference_round_trip() {
        let list_key = SymmetricKey::new([0x31; KEY_SIZE]);
        let plaintext = b"local plaintext attachment";
        let encrypted = encrypt_attachment_bytes(plaintext).expect("encrypt attachment");
        assert_ne!(encrypted.ciphertext, plaintext);
        assert_eq!(
            decrypt_attachment_bytes(
                &encrypted.ciphertext,
                encrypted.file_key.as_bytes(),
                Some(&encrypted.enc_context),
            )
            .expect("decrypt attachment"),
            plaintext
        );

        let blob_ref = AttachmentBlobRef {
            version: ATTACHMENT_BLOB_REF_VERSION,
            ciphertext_bytes: encrypted.ciphertext.len() as u64,
            file_key: encrypted.file_key.as_bytes().to_vec(),
            enc_context: encrypted.enc_context,
        };
        let blob_key =
            encode_attachment_blob_key(&list_key, &blob_ref).expect("encode attachment reference");
        let decoded =
            decode_attachment_blob_key(&list_key, &blob_key).expect("decode attachment reference");
        assert_eq!(decoded.file_key, blob_ref.file_key);
        assert_eq!(decoded.ciphertext_bytes, blob_ref.ciphertext_bytes);
    }
}
