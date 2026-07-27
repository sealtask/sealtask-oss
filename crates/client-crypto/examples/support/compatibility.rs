use std::error::Error;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chacha20poly1305::aead::{Aead as _, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit as _};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strong_box::StrongBox;
use strong_box::ciborium::Value as CborValue;
use uuid::Uuid;
use x25519_dalek::{X25519_BASEPOINT_BYTES, x25519};

use sealtask_client_crypto::{
    ATTACHMENT_BLOB_CONTEXT_LABEL, ATTACHMENT_BLOB_REF_VERSION, ATTACHMENT_REF_CONTEXT,
    COMMENT_PAYLOAD_CONTEXT, DATA_KEY_SALT_BYTES, KEY_SIZE, KeyDerivationService,
    NOTE_PAYLOAD_CONTEXT, SealedPayload, StrongBoxKeyRing, SymmetricKey, TASK_PAYLOAD_CONTEXT,
    USER_DATA_KEY_CONTEXT, USER_DATA_KEY_OPAQUE_CONTEXT, USER_DATA_KEY_OPAQUE_WRAP_INFO,
    compute_payload_proof, derive_payload_binding_key,
};

pub type FixtureResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const CORPUS_FILE: &str = "../../testdata/crypto-compat-v1.json";
const RECOVERY_DATA_KEY_CONTEXT: &[u8] = b"worklist.user.data_key.recovery.v1";
const RECOVERY_EXPORT_KEY_INFO: &[u8] = b"worklist.user.recovery_data_key.wrap.v1";
const ATTACHMENT_BLOB_CONTEXT: &[u8] = b"worklist.attachment.blob.v1";
const INVITE_PREVIEW_AUTH_SCHEME: &str = "x25519-hkdf-sha256-hmac-sha256";
const INVITE_MEMBER_KEY_INFO_PREFIX: &str = "member:";
const TRANSPARENCY_DOMAIN: &[u8] = b"worklist.transparency.v1";
const TRANSPARENCY_OWNER_KEY_INFO_PREFIX: &str = "transparency:owner-signing:v2";
const TRANSPARENCY_OWNER_IDENTITY_PUBLIC_KEY_B64: &str =
    "8q3Dhm9y8ioVUFX+Zo9p0qfBsxAQPmyPhhVzkEnDDII";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityCorpus {
    pub schema_version: u8,
    pub data_keys: DataKeyVectors,
    pub strong_box: StrongBoxVector,
    pub project_keys: ProjectKeyCompatibilityVector,
    pub payload_proof: PayloadProofVector,
    pub payloads: PayloadVectors,
    pub attachment: AttachmentVector,
    pub invite_bindings: InviteBindingVector,
    pub invite_preview_auth: InvitePreviewAuthVector,
    pub transparency: TransparencyVector,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataKeyVectors {
    pub data_key_b64: String,
    pub password_v1: PasswordDataKeyVector,
    pub opaque_v2: ExportDataKeyVector,
    pub recovery_v1: ExportDataKeyVector,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PasswordDataKeyVector {
    pub password: String,
    pub salt_b64: String,
    pub wrapping_key_b64: String,
    pub context_utf8: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportDataKeyVector {
    pub export_key_b64: String,
    pub wrapping_key_b64: String,
    pub wrapping_info_utf8: String,
    pub context_utf8: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StrongBoxVector {
    pub key_b64: String,
    pub context_utf8: String,
    pub plaintext_b64: String,
    pub nonce_b64: String,
    pub key_id_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectKeyCompatibilityVector {
    pub key_b64: String,
    pub legacy_bare_array_cbor_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PayloadProofVector {
    pub list_key_b64: String,
    pub binding_key_b64: String,
    pub ciphertext_b64: String,
    pub proof_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PayloadVectors {
    pub task: PayloadVector<TaskEnvelope>,
    pub comment: PayloadVector<CommentEnvelope>,
    pub note: PayloadVector<NoteEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PayloadVector<T> {
    pub context_utf8: String,
    pub envelope: T,
    pub plaintext_cbor_b64: String,
    pub sealed_payload_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskEnvelope {
    pub kind: String,
    pub version: u8,
    pub body: TaskBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskBody {
    pub title: String,
    pub rich_text: RichText,
    pub checklist: Vec<ChecklistItem>,
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChecklistItem {
    pub id: String,
    pub title: String,
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentEnvelope {
    pub kind: String,
    pub version: u8,
    pub body: CommentBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentBody {
    pub content: RichText,
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteEnvelope {
    pub kind: String,
    pub version: u8,
    pub body: NoteBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteBody {
    pub title: String,
    pub content: RichText,
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RichText {
    pub format: String,
    pub version: u8,
    pub blocks: Vec<RichTextBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RichTextBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentVector {
    pub plaintext_b64: String,
    pub file_key_b64: String,
    pub blob_context_utf8: String,
    pub blob_nonce_b64: String,
    pub blob_ciphertext_b64: String,
    pub list_key_b64: String,
    pub reference_context_utf8: String,
    pub reference_nonce_b64: String,
    pub blob_ref: AttachmentBlobRefJson,
    pub blob_ref_cbor_b64: String,
    pub blob_key_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentBlobRefJson {
    pub version: u8,
    pub ciphertext_bytes: u64,
    pub file_key_b64: String,
    pub enc_context: String,
}

#[derive(Serialize)]
struct AttachmentBlobRefWire<'a> {
    version: u8,
    ciphertext_bytes: u64,
    #[serde(with = "serde_bytes")]
    file_key: &'a [u8],
    enc_context: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InviteBindingVector {
    pub work_list_id: String,
    pub membership_id: String,
    pub user_id: String,
    pub role: String,
    pub key_fingerprint_b64: String,
    pub expires_at: Option<String>,
    pub invite_protocol_version: u8,
    pub reservation_revision: u64,
    pub recipient_context_cbor_b64: String,
    pub package_context_cbor_b64: String,
    pub list_key_b64: String,
    pub salt_b64: String,
    pub member_key_b64: String,
    pub issued_at: String,
    pub invite_package_digest_b64: String,
    pub recipient_plaintext_cbor_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvitePreviewAuthVector {
    pub package_version: u8,
    pub role: String,
    pub package_body: InviteAuthPackageBody,
    pub inviter_private_key_b64: String,
    pub inviter_public_key_b64: String,
    pub recipient_private_key_b64: String,
    pub recipient_public_key_b64: String,
    pub inviter_key_generation: u64,
    pub inviter_key_fingerprint_b64: String,
    pub recipient_key_fingerprint_b64: String,
    pub v1: InviteAuthenticatorVersionVector,
    pub v2: InviteAuthenticatorVersionVector,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InviteAuthPackageBody {
    pub work_list_id: String,
    pub membership_id: String,
    pub title: String,
    pub inviter: InviteAuthInviter,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub reservation_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InviteAuthInviter {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InviteAuthenticatorVersionVector {
    pub version: u8,
    pub key_context_cbor_b64: String,
    pub mac_message_cbor_b64: String,
    pub mac_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransparencyVector {
    pub owner_identity: TransparencyOwnerIdentityVector,
    pub statements: Vec<TransparencyStatementVector>,
    pub target_index: usize,
    pub log_size: usize,
    pub inclusion_proof_b64: Vec<String>,
    pub root_hash_b64: String,
    pub consistency: TransparencyConsistencyVector,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransparencyOwnerIdentityVector {
    pub data_key_b64: String,
    pub user_id: String,
    pub user_id_bytes_b64: String,
    pub hkdf_salt_b64: String,
    pub hkdf_info_utf8: String,
    pub identity_seed_b64: String,
    pub identity_public_key_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransparencyStatementVector {
    pub user_id: String,
    pub generation: u64,
    pub invite_key_b64: String,
    pub statement_digest_b64: String,
    pub leaf_hash_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransparencyConsistencyVector {
    pub from_size: usize,
    pub prefix_root_b64: String,
    pub proof_b64: Vec<String>,
}

#[derive(Serialize)]
struct RecipientBindingContext<'a> {
    kind: &'static str,
    version: u8,
    body: RecipientBindingBody<'a>,
}

#[derive(Serialize)]
struct RecipientBindingBody<'a> {
    work_list_id: &'a str,
    membership_id: &'a str,
    role: &'a str,
    key_fingerprint: &'a str,
}

#[derive(Serialize)]
struct PackageBindingContext<'a> {
    kind: &'static str,
    version: u8,
    body: PackageBindingBody<'a>,
}

#[derive(Serialize)]
struct PackageBindingBody<'a> {
    work_list_id: &'a str,
    membership_id: &'a str,
    role: &'a str,
    key_fingerprint: &'a str,
    expires_at: Option<&'a str>,
    reservation_revision: u64,
}

#[derive(Serialize)]
struct RecipientPlaintext<'a> {
    kind: &'static str,
    version: u8,
    body: RecipientPlaintextBody<'a>,
}

#[derive(Serialize)]
struct RecipientPlaintextBody<'a> {
    work_list_id: &'a str,
    membership_id: &'a str,
    role: &'a str,
    key: &'a str,
    invite_package_digest: &'a str,
    issued_at: &'a str,
}

#[derive(Serialize)]
struct InviteAuthMacMessage<'a> {
    kind: &'static str,
    version: u8,
    body: InviteAuthMacMessageBody<'a>,
}

#[derive(Serialize)]
struct InviteAuthMacMessageBody<'a> {
    package: &'a InviteAuthPackageBody,
    protocol: InviteAuthProtocol<'a>,
}

#[derive(Serialize)]
struct InviteAuthV1MacMessage<'a> {
    kind: &'static str,
    version: u8,
    body: InviteAuthV1MacMessageBody<'a>,
}

#[derive(Serialize)]
struct InviteAuthV1MacMessageBody<'a> {
    work_list_id: &'a str,
    membership_id: &'a str,
    title: &'a str,
    inviter: &'a InviteAuthInviter,
    issued_at: &'a str,
    expires_at: Option<&'a str>,
    reservation_revision: u64,
    invite_protocol_version: u8,
    role: &'a str,
    inviter_user_id: &'a str,
    inviter_key_generation: u64,
    inviter_key_fingerprint: &'a str,
    recipient_key_fingerprint: &'a str,
}

#[derive(Serialize)]
struct InviteAuthProtocol<'a> {
    invite_protocol_version: u8,
    role: &'a str,
    inviter_user_id: &'a str,
    inviter_key_generation: u64,
    inviter_key_fingerprint: &'a str,
    recipient_key_fingerprint: &'a str,
}

#[derive(Serialize)]
struct InviteAuthKeyContext<'a> {
    kind: &'static str,
    version: u8,
    body: InviteAuthKeyContextBody<'a>,
}

#[derive(Serialize)]
struct InviteAuthKeyContextBody<'a> {
    scheme: &'static str,
    work_list_id: &'a str,
    membership_id: &'a str,
    inviter_user_id: &'a str,
    inviter_key_generation: u64,
    inviter_key_fingerprint: &'a str,
    recipient_key_fingerprint: &'a str,
}

struct DeterministicStrongBox {
    key_id: [u8; 16],
    ciphertext: Vec<u8>,
}

pub fn corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(CORPUS_FILE)
}

pub fn generate_corpus_json() -> FixtureResult<String> {
    let mut json = serde_json::to_string_pretty(&generate_corpus()?)?;
    json.push('\n');
    Ok(json)
}

pub fn generate_corpus() -> FixtureResult<CompatibilityCorpus> {
    let list_key = byte_sequence(0x20, KEY_SIZE);
    let data_key = byte_sequence(0xa0, KEY_SIZE);

    let strong_box_key = byte_sequence(0x00, KEY_SIZE);
    let strong_box_context = "sealtask.compat.strongbox.v1";
    let strong_box_plaintext = b"SealTask StrongBox compatibility framing".to_vec();
    let strong_box_nonce = nonce(0x01);
    let strong_box_case = deterministic_strong_box(
        &strong_box_key,
        strong_box_context.as_bytes(),
        &strong_box_plaintext,
        strong_box_nonce,
    )?;
    verify_strong_box(
        &strong_box_key,
        strong_box_context.as_bytes(),
        &strong_box_plaintext,
        &strong_box_case.ciphertext,
    )?;

    let data_keys = generate_data_key_vectors(&data_key)?;
    let project_key = byte_sequence(0x00, KEY_SIZE);
    let legacy_bare_array = serialize_to_cbor_x(&project_key)?;
    let payloads = generate_payload_vectors(&list_key)?;
    let task_ciphertext = decode_b64(&payloads.task.sealed_payload_b64)?;
    let list_key_value = SymmetricKey::from_slice(&list_key)?;
    let binding_key = derive_payload_binding_key(&list_key_value)?;
    let proof = compute_payload_proof(&task_ciphertext, &binding_key)?;
    let attachment = generate_attachment_vector(&list_key)?;

    Ok(CompatibilityCorpus {
        schema_version: 1,
        data_keys,
        strong_box: StrongBoxVector {
            key_b64: b64(&strong_box_key),
            context_utf8: strong_box_context.to_string(),
            plaintext_b64: b64(&strong_box_plaintext),
            nonce_b64: b64(strong_box_nonce),
            key_id_b64: b64(strong_box_case.key_id),
            ciphertext_b64: b64(&strong_box_case.ciphertext),
        },
        project_keys: ProjectKeyCompatibilityVector {
            key_b64: b64(&project_key),
            legacy_bare_array_cbor_b64: b64(&legacy_bare_array),
        },
        payload_proof: PayloadProofVector {
            list_key_b64: b64(&list_key),
            binding_key_b64: b64(binding_key.as_bytes()),
            ciphertext_b64: b64(&task_ciphertext),
            proof_b64: proof,
        },
        payloads,
        attachment,
        invite_bindings: generate_invite_binding_vector(&list_key)?,
        invite_preview_auth: generate_invite_preview_auth_vector()?,
        transparency: generate_transparency_vector()?,
    })
}

fn generate_data_key_vectors(data_key: &[u8]) -> FixtureResult<DataKeyVectors> {
    let password = "correct horse battery staple";
    let salt = byte_sequence(0x40, DATA_KEY_SALT_BYTES);
    let wrapping_key = KeyDerivationService::new().derive_master_key(password, &salt)?;
    let password_nonce = nonce(0x10);
    let password_frame = deterministic_strong_box(
        wrapping_key.as_bytes(),
        USER_DATA_KEY_CONTEXT,
        data_key,
        password_nonce,
    )?;
    verify_strong_box(
        wrapping_key.as_bytes(),
        USER_DATA_KEY_CONTEXT,
        data_key,
        &password_frame.ciphertext,
    )?;
    let mut password_ciphertext = salt.clone();
    password_ciphertext.extend_from_slice(&password_frame.ciphertext);
    let password_payload = sealed_payload_b64(1, password_ciphertext)?;

    let opaque_export_key = byte_sequence(0x60, 64);
    let opaque_wrapping_key = hkdf(&opaque_export_key, USER_DATA_KEY_OPAQUE_WRAP_INFO, KEY_SIZE)?;
    let opaque_nonce = nonce(0x20);
    let opaque_frame = deterministic_strong_box(
        &opaque_wrapping_key,
        USER_DATA_KEY_OPAQUE_CONTEXT,
        data_key,
        opaque_nonce,
    )?;
    verify_strong_box(
        &opaque_wrapping_key,
        USER_DATA_KEY_OPAQUE_CONTEXT,
        data_key,
        &opaque_frame.ciphertext,
    )?;

    let recovery_export_key = byte_sequence(0xc0, 64);
    let recovery_wrapping_key = hkdf(&recovery_export_key, RECOVERY_EXPORT_KEY_INFO, KEY_SIZE)?;
    let recovery_nonce = nonce(0x30);
    let recovery_frame = deterministic_strong_box(
        &recovery_wrapping_key,
        RECOVERY_DATA_KEY_CONTEXT,
        data_key,
        recovery_nonce,
    )?;
    verify_strong_box(
        &recovery_wrapping_key,
        RECOVERY_DATA_KEY_CONTEXT,
        data_key,
        &recovery_frame.ciphertext,
    )?;

    Ok(DataKeyVectors {
        data_key_b64: b64(data_key),
        password_v1: PasswordDataKeyVector {
            password: password.to_string(),
            salt_b64: b64(&salt),
            wrapping_key_b64: b64(wrapping_key.as_bytes()),
            context_utf8: String::from_utf8(USER_DATA_KEY_CONTEXT.to_vec())?,
            nonce_b64: b64(password_nonce),
            ciphertext_b64: password_payload,
        },
        opaque_v2: ExportDataKeyVector {
            export_key_b64: b64(&opaque_export_key),
            wrapping_key_b64: b64(&opaque_wrapping_key),
            wrapping_info_utf8: String::from_utf8(USER_DATA_KEY_OPAQUE_WRAP_INFO.to_vec())?,
            context_utf8: String::from_utf8(USER_DATA_KEY_OPAQUE_CONTEXT.to_vec())?,
            nonce_b64: b64(opaque_nonce),
            ciphertext_b64: sealed_payload_b64(2, opaque_frame.ciphertext)?,
        },
        recovery_v1: ExportDataKeyVector {
            export_key_b64: b64(&recovery_export_key),
            wrapping_key_b64: b64(&recovery_wrapping_key),
            wrapping_info_utf8: String::from_utf8(RECOVERY_EXPORT_KEY_INFO.to_vec())?,
            context_utf8: String::from_utf8(RECOVERY_DATA_KEY_CONTEXT.to_vec())?,
            nonce_b64: b64(recovery_nonce),
            ciphertext_b64: sealed_payload_b64(2, recovery_frame.ciphertext)?,
        },
    })
}

fn generate_payload_vectors(list_key: &[u8]) -> FixtureResult<PayloadVectors> {
    let task = TaskEnvelope {
        kind: "task".to_string(),
        version: 1,
        body: TaskBody {
            title: "Compatibility task".to_string(),
            rich_text: rich_text("Task body"),
            checklist: vec![ChecklistItem {
                id: "11111111-1111-4111-8111-111111111111".to_string(),
                title: "Freeze formats".to_string(),
                is_done: false,
            }],
            mentions: vec!["22222222-2222-4222-8222-222222222222".to_string()],
        },
    };
    let comment = CommentEnvelope {
        kind: "comment".to_string(),
        version: 1,
        body: CommentBody {
            content: rich_text("Compatibility comment"),
            mentions: vec!["33333333-3333-4333-8333-333333333333".to_string()],
        },
    };
    let note = NoteEnvelope {
        kind: "note".to_string(),
        version: 1,
        body: NoteBody {
            title: "Compatibility note".to_string(),
            content: rich_text("Compatibility note body"),
            mentions: vec!["44444444-4444-4444-8444-444444444444".to_string()],
        },
    };

    Ok(PayloadVectors {
        task: payload_vector(task, list_key, TASK_PAYLOAD_CONTEXT, nonce(0x40))?,
        comment: payload_vector(comment, list_key, COMMENT_PAYLOAD_CONTEXT, nonce(0x50))?,
        note: payload_vector(note, list_key, NOTE_PAYLOAD_CONTEXT, nonce(0x60))?,
    })
}

fn payload_vector<T>(
    envelope: T,
    key: &[u8],
    context: &[u8],
    nonce: [u8; 12],
) -> FixtureResult<PayloadVector<T>>
where
    T: Serialize,
{
    let plaintext = serialize_to_cbor_x(&envelope)?;
    let frame = deterministic_strong_box(key, context, &plaintext, nonce)?;
    verify_strong_box(key, context, &plaintext, &frame.ciphertext)?;
    Ok(PayloadVector {
        context_utf8: String::from_utf8(context.to_vec())?,
        envelope,
        plaintext_cbor_b64: b64(&plaintext),
        sealed_payload_b64: sealed_payload_b64(1, frame.ciphertext)?,
    })
}

fn generate_attachment_vector(list_key: &[u8]) -> FixtureResult<AttachmentVector> {
    let plaintext = b"attachment compatibility bytes\x00\xff".to_vec();
    let file_key = byte_sequence(0x90, KEY_SIZE);
    let blob_nonce = nonce(0x70);
    let blob_frame =
        deterministic_strong_box(&file_key, ATTACHMENT_BLOB_CONTEXT, &plaintext, blob_nonce)?;
    verify_strong_box(
        &file_key,
        ATTACHMENT_BLOB_CONTEXT,
        &plaintext,
        &blob_frame.ciphertext,
    )?;

    let blob_ref = AttachmentBlobRefJson {
        version: ATTACHMENT_BLOB_REF_VERSION,
        ciphertext_bytes: blob_frame.ciphertext.len() as u64,
        file_key_b64: b64(&file_key),
        enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL.to_string(),
    };
    let blob_ref_cbor = serialize_to_cbor_x(&AttachmentBlobRefWire {
        version: blob_ref.version,
        ciphertext_bytes: blob_ref.ciphertext_bytes,
        file_key: &file_key,
        enc_context: &blob_ref.enc_context,
    })?;
    let reference_nonce = nonce(0x80);
    let reference_frame = deterministic_strong_box(
        list_key,
        ATTACHMENT_REF_CONTEXT,
        &blob_ref_cbor,
        reference_nonce,
    )?;
    verify_strong_box(
        list_key,
        ATTACHMENT_REF_CONTEXT,
        &blob_ref_cbor,
        &reference_frame.ciphertext,
    )?;

    Ok(AttachmentVector {
        plaintext_b64: b64(&plaintext),
        file_key_b64: b64(&file_key),
        blob_context_utf8: String::from_utf8(ATTACHMENT_BLOB_CONTEXT.to_vec())?,
        blob_nonce_b64: b64(blob_nonce),
        blob_ciphertext_b64: b64(&blob_frame.ciphertext),
        list_key_b64: b64(list_key),
        reference_context_utf8: String::from_utf8(ATTACHMENT_REF_CONTEXT.to_vec())?,
        reference_nonce_b64: b64(reference_nonce),
        blob_ref,
        blob_ref_cbor_b64: b64(&blob_ref_cbor),
        blob_key_b64: sealed_payload_b64(1, reference_frame.ciphertext)?,
    })
}

fn generate_invite_binding_vector(list_key: &[u8]) -> FixtureResult<InviteBindingVector> {
    let work_list_id = "55555555-5555-4555-8555-555555555555";
    let membership_id = "66666666-6666-4666-8666-666666666666";
    let user_id = "77777777-7777-4777-8777-777777777777";
    let role = "member";
    let fingerprint = Sha256::digest(byte_sequence(0x31, 32));
    let key_fingerprint_b64 = b64(fingerprint);
    let reservation_revision = 7;
    let recipient_context = serialize_to_cbor_x(&RecipientBindingContext {
        kind: "work_list.invite.binding",
        version: 1,
        body: RecipientBindingBody {
            work_list_id,
            membership_id,
            role,
            key_fingerprint: &key_fingerprint_b64,
        },
    })?;
    let package_context = serialize_to_cbor_x(&PackageBindingContext {
        kind: "work_list.invite.package.binding",
        version: 2,
        body: PackageBindingBody {
            work_list_id,
            membership_id,
            role,
            key_fingerprint: &key_fingerprint_b64,
            expires_at: None,
            reservation_revision,
        },
    })?;
    let salt = byte_sequence(0x51, 32);
    let salt_b64 = b64(&salt);
    let member_key_info = format!("{INVITE_MEMBER_KEY_INFO_PREFIX}{user_id}:{salt_b64}");
    let member_key = hkdf(list_key, member_key_info.as_bytes(), KEY_SIZE)?;
    let issued_at = "2026-07-25T12:00:00.000Z";
    let package_digest = Sha256::digest(b"fixed invite package fixture bytes");
    let package_digest_b64 = b64(package_digest);
    let list_key_b64 = b64(list_key);
    let recipient_plaintext = serialize_to_cbor_x(&RecipientPlaintext {
        kind: "work_list.invite.recipient",
        version: 1,
        body: RecipientPlaintextBody {
            work_list_id,
            membership_id,
            role,
            key: &list_key_b64,
            invite_package_digest: &package_digest_b64,
            issued_at,
        },
    })?;

    Ok(InviteBindingVector {
        work_list_id: work_list_id.to_string(),
        membership_id: membership_id.to_string(),
        user_id: user_id.to_string(),
        role: role.to_string(),
        key_fingerprint_b64,
        expires_at: None,
        invite_protocol_version: 2,
        reservation_revision,
        recipient_context_cbor_b64: b64(&recipient_context),
        package_context_cbor_b64: b64(&package_context),
        list_key_b64,
        salt_b64,
        member_key_b64: b64(&member_key),
        issued_at: issued_at.to_string(),
        invite_package_digest_b64: package_digest_b64,
        recipient_plaintext_cbor_b64: b64(&recipient_plaintext),
    })
}

fn generate_invite_preview_auth_vector() -> FixtureResult<InvitePreviewAuthVector> {
    let inviter_private_key = clamped_key(0x11);
    let recipient_private_key = clamped_key(0x71);
    let inviter_public_key = x25519(inviter_private_key, X25519_BASEPOINT_BYTES);
    let recipient_public_key = x25519(recipient_private_key, X25519_BASEPOINT_BYTES);
    let inviter_key_fingerprint_b64 = b64(Sha256::digest(inviter_public_key));
    let recipient_key_fingerprint_b64 = b64(Sha256::digest(recipient_public_key));
    let inviter_key_generation = 4;
    let package_body = InviteAuthPackageBody {
        work_list_id: "88888888-8888-4888-8888-888888888888".to_string(),
        membership_id: "99999999-9999-4999-8999-999999999999".to_string(),
        title: "Authenticated compatibility project".to_string(),
        inviter: InviteAuthInviter {
            id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string(),
            name: Some("Compatibility Owner".to_string()),
            email: Some("owner@example.com".to_string()),
        },
        issued_at: "2026-07-25T12:00:00.000Z".to_string(),
        expires_at: None,
        reservation_revision: 8,
    };
    let role = "member";
    let package_version = 2;
    let shared_secret = x25519(inviter_private_key, recipient_public_key);

    let v1 = invite_auth_version_vector(
        1,
        &package_body,
        role,
        package_version,
        inviter_key_generation,
        &inviter_key_fingerprint_b64,
        &recipient_key_fingerprint_b64,
        &shared_secret,
    )?;
    let v2 = invite_auth_version_vector(
        2,
        &package_body,
        role,
        package_version,
        inviter_key_generation,
        &inviter_key_fingerprint_b64,
        &recipient_key_fingerprint_b64,
        &shared_secret,
    )?;

    Ok(InvitePreviewAuthVector {
        package_version,
        role: role.to_string(),
        package_body,
        inviter_private_key_b64: b64(inviter_private_key),
        inviter_public_key_b64: b64(inviter_public_key),
        recipient_private_key_b64: b64(recipient_private_key),
        recipient_public_key_b64: b64(recipient_public_key),
        inviter_key_generation,
        inviter_key_fingerprint_b64,
        recipient_key_fingerprint_b64,
        v1,
        v2,
    })
}

#[allow(clippy::too_many_arguments)]
fn invite_auth_version_vector(
    version: u8,
    package_body: &InviteAuthPackageBody,
    role: &str,
    package_version: u8,
    inviter_key_generation: u64,
    inviter_key_fingerprint_b64: &str,
    recipient_key_fingerprint_b64: &str,
    shared_secret: &[u8],
) -> FixtureResult<InviteAuthenticatorVersionVector> {
    let inviter_user_id = package_body.inviter.id.as_str();
    let key_context = serialize_to_cbor_x(&InviteAuthKeyContext {
        kind: "work_list.invite.preview_auth.key",
        version,
        body: InviteAuthKeyContextBody {
            scheme: INVITE_PREVIEW_AUTH_SCHEME,
            work_list_id: &package_body.work_list_id,
            membership_id: &package_body.membership_id,
            inviter_user_id,
            inviter_key_generation,
            inviter_key_fingerprint: inviter_key_fingerprint_b64,
            recipient_key_fingerprint: recipient_key_fingerprint_b64,
        },
    })?;
    let protocol = InviteAuthProtocol {
        invite_protocol_version: package_version,
        role,
        inviter_user_id,
        inviter_key_generation,
        inviter_key_fingerprint: inviter_key_fingerprint_b64,
        recipient_key_fingerprint: recipient_key_fingerprint_b64,
    };
    let message = if version == 1 {
        serialize_to_cbor_x(&InviteAuthV1MacMessage {
            kind: "work_list.invite.preview_auth.message",
            version,
            body: InviteAuthV1MacMessageBody {
                work_list_id: &package_body.work_list_id,
                membership_id: &package_body.membership_id,
                title: &package_body.title,
                inviter: &package_body.inviter,
                issued_at: &package_body.issued_at,
                expires_at: package_body.expires_at.as_deref(),
                reservation_revision: package_body.reservation_revision,
                invite_protocol_version: protocol.invite_protocol_version,
                role: protocol.role,
                inviter_user_id: protocol.inviter_user_id,
                inviter_key_generation: protocol.inviter_key_generation,
                inviter_key_fingerprint: protocol.inviter_key_fingerprint,
                recipient_key_fingerprint: protocol.recipient_key_fingerprint,
            },
        })?
    } else {
        serialize_to_cbor_x(&InviteAuthMacMessage {
            kind: "work_list.invite.preview_auth.message",
            version,
            body: InviteAuthMacMessageBody {
                package: package_body,
                protocol,
            },
        })?
    };
    let mac_key = hkdf(shared_secret, &key_context, KEY_SIZE)?;
    let mac = hmac_sha256(&mac_key, &message)?;
    Ok(InviteAuthenticatorVersionVector {
        version,
        key_context_cbor_b64: b64(&key_context),
        mac_message_cbor_b64: b64(&message),
        mac_b64: b64(mac),
    })
}

fn generate_transparency_vector() -> FixtureResult<TransparencyVector> {
    let inputs = [
        (
            "11111111-1111-1111-1111-111111111111",
            1_u64,
            byte_sequence(0x21, 32),
        ),
        (
            "22222222-2222-2222-2222-222222222222",
            2_u64,
            byte_sequence(0x51, 32),
        ),
        (
            "33333333-3333-3333-3333-333333333333",
            3_u64,
            byte_sequence(0x81, 32),
        ),
    ];
    let mut statements = Vec::with_capacity(inputs.len());
    let mut leaves = Vec::with_capacity(inputs.len());
    for (user_id, generation, invite_key) in inputs {
        let digest = transparency_statement_digest(user_id, generation, &invite_key)?;
        let leaf = prefixed_hash(0, &[&digest]);
        statements.push(TransparencyStatementVector {
            user_id: user_id.to_string(),
            generation,
            invite_key_b64: b64(&invite_key),
            statement_digest_b64: b64(digest),
            leaf_hash_b64: b64(leaf),
        });
        leaves.push(leaf);
    }

    let prefix_root = prefixed_hash(1, &[&leaves[0], &leaves[1]]);
    let root = prefixed_hash(1, &[&prefix_root, &leaves[2]]);

    Ok(TransparencyVector {
        owner_identity: generate_transparency_owner_identity_vector()?,
        statements,
        target_index: 1,
        log_size: 3,
        inclusion_proof_b64: vec![b64(leaves[0]), b64(leaves[2])],
        root_hash_b64: b64(root),
        consistency: TransparencyConsistencyVector {
            from_size: 2,
            prefix_root_b64: b64(prefix_root),
            proof_b64: vec![b64(prefix_root), b64(leaves[2])],
        },
    })
}

fn generate_transparency_owner_identity_vector() -> FixtureResult<TransparencyOwnerIdentityVector> {
    let data_key = byte_sequence(1, KEY_SIZE);
    let user_id = "11111111-1111-1111-1111-111111111111";
    let parsed_user_id = Uuid::parse_str(user_id)?;
    let canonical_user_id = parsed_user_id.to_string();
    if canonical_user_id != user_id {
        return Err("transparency owner-identity fixture user id is not canonical".into());
    }
    let compact_user_id = canonical_user_id.replace('-', "");
    let hkdf_info_utf8 = format!("{TRANSPARENCY_OWNER_KEY_INFO_PREFIX}:{compact_user_id}");
    let identity_seed = hkdf(&data_key, hkdf_info_utf8.as_bytes(), KEY_SIZE)?;

    Ok(TransparencyOwnerIdentityVector {
        data_key_b64: b64(&data_key),
        user_id: canonical_user_id,
        user_id_bytes_b64: b64(parsed_user_id.as_bytes()),
        hkdf_salt_b64: b64([]),
        hkdf_info_utf8,
        identity_seed_b64: b64(&identity_seed),
        identity_public_key_b64: TRANSPARENCY_OWNER_IDENTITY_PUBLIC_KEY_B64.to_string(),
    })
}

fn transparency_statement_digest(
    user_id: &str,
    generation: u64,
    invite_key: &[u8],
) -> FixtureResult<[u8; 32]> {
    let user_id = Uuid::parse_str(user_id)?;
    let mut hasher = Sha256::new();
    hasher.update(TRANSPARENCY_DOMAIN);
    hasher.update(user_id.as_bytes());
    hasher.update(generation.to_be_bytes());
    hasher.update((invite_key.len() as u32).to_be_bytes());
    hasher.update(invite_key);
    Ok(hasher.finalize().into())
}

fn prefixed_hash(prefix: u8, parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([prefix]);
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn rich_text(text: &str) -> RichText {
    RichText {
        format: "plaintext".to_string(),
        version: 1,
        blocks: vec![RichTextBlock {
            block_type: "paragraph".to_string(),
            text: text.to_string(),
        }],
    }
}

fn deterministic_strong_box(
    key: &[u8],
    context: &[u8],
    plaintext: &[u8],
    nonce: [u8; 12],
) -> FixtureResult<DeterministicStrongBox> {
    let key: [u8; KEY_SIZE] = key.try_into()?;
    let mut key_id_material = [0_u8; KEY_SIZE];
    Hkdf::<Sha256>::from_prk(&key)
        .map_err(|_| "StrongBox key-id HKDF initialization failed")?
        .expand(b"key_id", &mut key_id_material)
        .map_err(|_| "StrongBox key-id HKDF expansion failed")?;
    let key_id: [u8; 16] = key_id_material[..16].try_into()?;
    let mut aad = Vec::with_capacity(context.len() + key_id.len() + nonce.len());
    aad.extend_from_slice(context);
    aad.extend_from_slice(&key_id);
    aad.extend_from_slice(&nonce);
    let cipher = ChaCha20Poly1305::new((&key).into());
    let encrypted = cipher
        .encrypt(
            (&nonce).into(),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| "deterministic StrongBox encryption failed")?;
    let mut framed = vec![0xb1, 0xb8, 0xf5, 0x83];
    encode_cbor_bytes(&mut framed, &key_id)?;
    encode_cbor_bytes(&mut framed, &nonce)?;
    encode_cbor_bytes(&mut framed, &encrypted)?;
    Ok(DeterministicStrongBox {
        key_id,
        ciphertext: framed,
    })
}

fn verify_strong_box(
    key: &[u8],
    context: &[u8],
    plaintext: &[u8],
    ciphertext: &[u8],
) -> FixtureResult<()> {
    let key = SymmetricKey::from_slice(key)?;
    let decrypted = StrongBoxKeyRing::new(key)
        .strong_box()
        .decrypt(ciphertext, context)?;
    if decrypted != plaintext {
        return Err("deterministic StrongBox frame did not decrypt to its plaintext".into());
    }
    Ok(())
}

fn encode_cbor_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> FixtureResult<()> {
    match bytes.len() {
        length @ 0..=23 => output.push(0x40 | length as u8),
        length @ 24..=255 => {
            output.push(0x58);
            output.push(length as u8);
        }
        length @ 256..=65_535 => {
            output.push(0x59);
            output.extend_from_slice(&(length as u16).to_be_bytes());
        }
        _ => return Err("fixture CBOR byte string is unexpectedly large".into()),
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn sealed_payload_b64(version: u8, ciphertext: Vec<u8>) -> FixtureResult<String> {
    Ok(b64(&serialize_to_cbor_x(&SealedPayload {
        version,
        ciphertext,
    })?))
}

fn serialize_to_cbor_x<T: Serialize + ?Sized>(value: &T) -> FixtureResult<Vec<u8>> {
    let value = CborValue::serialized(value)?;
    let mut encoded = Vec::new();
    encode_cbor_x_value(&mut encoded, &value)?;
    Ok(encoded)
}

fn encode_cbor_x_value(output: &mut Vec<u8>, value: &CborValue) -> FixtureResult<()> {
    match value {
        CborValue::Integer(value) => {
            let value = i128::from(*value);
            if value >= 0 {
                encode_cbor_argument(output, 0, value.try_into()?)
            } else {
                encode_cbor_argument(output, 1, (-1 - value).try_into()?)
            }
        }
        CborValue::Bytes(value) => {
            encode_cbor_argument(output, 6, 64);
            encode_cbor_argument(output, 2, value.len().try_into()?);
            output.extend_from_slice(value);
        }
        CborValue::Float(value) => {
            output.push(0xfb);
            output.extend_from_slice(&value.to_be_bytes());
        }
        CborValue::Text(value) => {
            encode_cbor_argument(output, 3, value.len().try_into()?);
            output.extend_from_slice(value.as_bytes());
        }
        CborValue::Bool(false) => output.push(0xf4),
        CborValue::Bool(true) => output.push(0xf5),
        CborValue::Null => output.push(0xf6),
        CborValue::Tag(tag, value) => {
            encode_cbor_argument(output, 6, *tag);
            encode_cbor_x_value(output, value)?;
        }
        CborValue::Array(values) => {
            encode_cbor_argument(output, 4, values.len().try_into()?);
            for value in values {
                encode_cbor_x_value(output, value)?;
            }
        }
        CborValue::Map(entries) => {
            let length: u16 = entries
                .len()
                .try_into()
                .map_err(|_| "fixture object has too many fields for cbor-x encoding")?;
            output.push(0xb9);
            output.extend_from_slice(&length.to_be_bytes());
            for (key, value) in entries {
                encode_cbor_x_value(output, key)?;
                encode_cbor_x_value(output, value)?;
            }
        }
        _ => return Err("fixture contains an unsupported CBOR value".into()),
    }
    Ok(())
}

fn encode_cbor_argument(output: &mut Vec<u8>, major: u8, value: u64) {
    let prefix = major << 5;
    match value {
        0..=23 => output.push(prefix | value as u8),
        24..=0xff => {
            output.push(prefix | 24);
            output.push(value as u8);
        }
        0x100..=0xffff => {
            output.push(prefix | 25);
            output.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            output.push(prefix | 26);
            output.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            output.push(prefix | 27);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn hkdf(parent: &[u8], info: &[u8], length: usize) -> FixtureResult<Vec<u8>> {
    let mut output = vec![0_u8; length];
    Hkdf::<Sha256>::new(None, parent)
        .expand(info, &mut output)
        .map_err(|_| "fixture HKDF expansion failed")?;
    Ok(output)
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> FixtureResult<[u8; 32]> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().into())
}

fn clamped_key(seed: u8) -> [u8; 32] {
    let mut key: [u8; 32] = byte_sequence(seed, 32)
        .try_into()
        .expect("fixed-size fixture key");
    key[0] &= 248;
    key[31] &= 127;
    key[31] |= 64;
    key
}

fn nonce(seed: u8) -> [u8; 12] {
    byte_sequence(seed, 12)
        .try_into()
        .expect("fixed-size fixture nonce")
}

fn byte_sequence(seed: u8, length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| seed.wrapping_add(index as u8))
        .collect()
}

fn b64(bytes: impl AsRef<[u8]>) -> String {
    STANDARD_NO_PAD.encode(bytes)
}

fn decode_b64(value: &str) -> FixtureResult<Vec<u8>> {
    Ok(STANDARD_NO_PAD.decode(value)?)
}
