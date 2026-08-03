use std::{
    fmt,
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signer, SigningKey};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, Zeroizing};

use sealtask_client_core::{PublicError, PublicResult};

use crate::config_dir;

const AGENT_IDENTITY_SCHEMA_VERSION: u32 = 1;
const AGENT_ENROLLMENT_DRAFT_SCHEMA_VERSION: u32 = 1;
const AGENT_KEY_BYTES: usize = 32;
const MAX_AGENT_HANDLE_LEN: usize = 48;
const MAX_AGENT_DISPLAY_NAME_LEN: usize = 80;
const MAX_AGENT_IDENTITY_FILE_BYTES: u64 = 128 * 1024;
const MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES: u64 = 64 * 1024;
const AGENT_ASSERTION_TTL_SECONDS: i64 = 60;
const AGENT_ASSERTION_PURPOSE_TOKEN_MINT: &str = "token_mint";
const AGENT_FINGERPRINT_CONTEXT: &[u8] = b"sealtask.agent.fingerprint.v1\0";
const AGENT_ENROLLMENT_CODE_DERIVATION: &[u8] = b"sealtask.agent.enrollment-code.v1";
const AGENT_ENROLLMENT_DRAFT_CONTEXT: &[u8] = b"sealtask.agent.enrollment-draft.v1\0";
const AGENTS_DIRECTORY: &str = "agents";
const AGENT_ENROLLMENT_DRAFTS_DIRECTORY: &str = "enrollment-drafts";
const AGENT_ENROLLMENT_RECEIPT_SUFFIX: &str = ".registration.json";
const AGENT_IDENTITY_FILE: &str = "identity.json";
const AGENT_IDENTITY_LOCK_FILE: &str = "identity.lock";
const AGENT_IDENTITY_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const AGENT_IDENTITY_LOCK_RETRY: Duration = Duration::from_millis(20);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalAgentStatus {
    Pending,
    Expired,
    Active,
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentProjectBinding {
    pub work_list_id: Uuid,
    pub repository_root: PathBuf,
    pub permission_preset: String,
    pub instructions_revision: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub agent_id: Uuid,
    pub api_url: String,
    pub status: LocalAgentStatus,
    pub proposed_handle: Option<String>,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub fingerprint: String,
    pub auth_public_key: String,
    pub recipient_public_key: String,
    pub enrollment_expires_at: Option<DateTime<Utc>>,
    pub project: AgentProjectBinding,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentEnrollmentRegistration {
    pub agent_id: Uuid,
    pub proposed_handle: Option<String>,
    pub auth_public_key: String,
    pub recipient_public_key: String,
    pub fingerprint: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentIdentityLoadFailure {
    pub agent_id: Uuid,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentIdentityListing {
    pub discovered_identities: usize,
    pub identities: Vec<AgentIdentity>,
    pub failures: Vec<AgentIdentityLoadFailure>,
}

#[derive(Serialize, Deserialize)]
struct StoredAgentIdentity {
    schema_version: u32,
    #[serde(flatten)]
    identity: AgentIdentity,
    seed: String,
}

#[derive(Serialize, Deserialize)]
struct StoredAgentEnrollmentDraft {
    schema_version: u32,
    context_id: String,
    api_url: String,
    proposed_handle: Option<String>,
    work_list_id: Uuid,
    repository_root: PathBuf,
    seed: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct StoredAgentEnrollmentRegistration {
    schema_version: u32,
    context_id: String,
    registration: AgentEnrollmentRegistration,
}

struct AgentIdentityFileLock {
    file: File,
}

struct AgentEnrollmentDraftFileLock {
    file: File,
}

impl Drop for StoredAgentIdentity {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

impl Drop for StoredAgentEnrollmentDraft {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

impl fmt::Debug for StoredAgentEnrollmentRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredAgentEnrollmentRegistration")
            .field("schema_version", &self.schema_version)
            .field("context_id", &self.context_id)
            .field("registration", &self.registration)
            .finish()
    }
}

impl AgentIdentityFileLock {
    fn acquire(agent_id: Uuid) -> PublicResult<Self> {
        let root = agents_root()?;
        create_private_directory(&root)?;
        let directory = root.join(agent_id.to_string());
        create_private_directory(&directory)?;
        let path = directory.join(AGENT_IDENTITY_LOCK_FILE);

        let mut create_options = OpenOptions::new();
        create_options.create_new(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            create_options.mode(0o600);
        }
        let (file, created) = match create_options.open(&path) {
            Ok(file) => (file, true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                reject_symlink_if_present(&path)?;
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .map_err(|error| {
                        PublicError::unexpected(format!(
                            "failed to open agent identity lock {}: {error}",
                            path.display()
                        ))
                    })?;
                (file, false)
            }
            Err(error) => {
                return Err(PublicError::unexpected(format!(
                    "failed to create agent identity lock {}: {error}",
                    path.display()
                )));
            }
        };
        let metadata = file.metadata().map_err(|error| {
            PublicError::unexpected(format!("failed to inspect agent identity lock: {error}"))
        })?;
        if !metadata.is_file() {
            return Err(PublicError::unexpected(
                "agent identity lock must be a regular file",
            ));
        }
        validate_private_file_permissions(&metadata)?;
        if created {
            sync_directory(&directory)?;
        }

        let deadline = Instant::now() + AGENT_IDENTITY_LOCK_TIMEOUT;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    thread::sleep(AGENT_IDENTITY_LOCK_RETRY);
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(PublicError::conflict(
                        "agent identity is being updated by another process; retry the command",
                    ));
                }
                Err(TryLockError::Error(error)) => {
                    return Err(PublicError::unexpected(format!(
                        "failed to lock agent identity: {error}"
                    )));
                }
            }
        }
    }
}

impl Drop for AgentIdentityFileLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

impl AgentEnrollmentDraftFileLock {
    fn acquire(context_id: &str) -> PublicResult<Self> {
        let directory = enrollment_drafts_root()?;
        create_private_directory(&directory)?;
        let path = directory.join(format!("{context_id}.lock"));

        let mut create_options = OpenOptions::new();
        create_options.create_new(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            create_options.mode(0o600);
        }
        let (file, created) = match create_options.open(&path) {
            Ok(file) => (file, true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                reject_symlink_if_present(&path)?;
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .map_err(|error| {
                        PublicError::unexpected(format!(
                            "failed to open agent enrollment draft lock {}: {error}",
                            path.display()
                        ))
                    })?;
                (file, false)
            }
            Err(error) => {
                return Err(PublicError::unexpected(format!(
                    "failed to create agent enrollment draft lock {}: {error}",
                    path.display()
                )));
            }
        };
        let metadata = file.metadata().map_err(|error| {
            PublicError::unexpected(format!(
                "failed to inspect agent enrollment draft lock: {error}"
            ))
        })?;
        if !metadata.is_file() {
            return Err(PublicError::unexpected(
                "agent enrollment draft lock must be a regular file",
            ));
        }
        validate_private_file_permissions(&metadata)?;
        if created {
            sync_directory(&directory)?;
        }

        let deadline = Instant::now() + AGENT_IDENTITY_LOCK_TIMEOUT;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    thread::sleep(AGENT_IDENTITY_LOCK_RETRY);
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(PublicError::conflict(
                        "agent enrollment is being registered by another process; retry the command",
                    ));
                }
                Err(TryLockError::Error(error)) => {
                    return Err(PublicError::unexpected(format!(
                        "failed to lock agent enrollment draft: {error}"
                    )));
                }
            }
        }
    }
}

impl Drop for AgentEnrollmentDraftFileLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

impl fmt::Debug for StoredAgentIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredAgentIdentity")
            .field("schema_version", &self.schema_version)
            .field("identity", &self.identity)
            .field("seed", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for StoredAgentEnrollmentDraft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredAgentEnrollmentDraft")
            .field("schema_version", &self.schema_version)
            .field("context_id", &self.context_id)
            .field("api_url", &self.api_url)
            .field("proposed_handle", &self.proposed_handle)
            .field("work_list_id", &self.work_list_id)
            .field("repository_root", &self.repository_root)
            .field("seed", &"<redacted>")
            .field("created_at", &self.created_at)
            .finish()
    }
}

pub struct AgentKeyMaterial {
    seed: [u8; AGENT_KEY_BYTES],
    signing_key: SigningKey,
    auth_public_key: [u8; AGENT_KEY_BYTES],
    recipient_private_key: [u8; AGENT_KEY_BYTES],
    recipient_public_key: [u8; AGENT_KEY_BYTES],
}

pub struct PrepareAgentEnrollmentDraft<'a> {
    pub api_url: &'a str,
    pub proposed_handle: Option<String>,
    pub work_list_id: Uuid,
    pub repository_root: &'a Path,
}

pub struct AgentEnrollmentDraft {
    stored: StoredAgentEnrollmentDraft,
    key_material: AgentKeyMaterial,
    registration: Option<AgentEnrollmentRegistration>,
    resumed: bool,
    _lock: AgentEnrollmentDraftFileLock,
}

impl fmt::Debug for AgentKeyMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentKeyMaterial")
            .field("seed", &"<redacted>")
            .field("auth_public_key", &self.auth_public_key)
            .field("recipient_private_key", &"<redacted>")
            .field("recipient_public_key", &self.recipient_public_key)
            .finish()
    }
}

impl fmt::Debug for AgentEnrollmentDraft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentEnrollmentDraft")
            .field("context_id", &self.stored.context_id)
            .field("api_url", &self.stored.api_url)
            .field("proposed_handle", &self.stored.proposed_handle)
            .field("work_list_id", &self.stored.work_list_id)
            .field("repository_root", &self.stored.repository_root)
            .field("key_material", &"<redacted>")
            .field("registration", &self.registration)
            .field("resumed", &self.resumed)
            .finish()
    }
}

impl Drop for AgentKeyMaterial {
    fn drop(&mut self) {
        self.seed.zeroize();
        self.recipient_private_key.zeroize();
    }
}

impl AgentKeyMaterial {
    pub fn auth_public_key(&self) -> &[u8; AGENT_KEY_BYTES] {
        &self.auth_public_key
    }

    pub fn recipient_private_key(&self) -> &[u8; AGENT_KEY_BYTES] {
        &self.recipient_private_key
    }

    pub fn recipient_public_key(&self) -> &[u8; AGENT_KEY_BYTES] {
        &self.recipient_public_key
    }

    pub fn fingerprint(&self) -> String {
        agent_fingerprint(&self.auth_public_key, &self.recipient_public_key)
    }

    /// Returns the identity's high-entropy enrollment and grant-signing secret.
    /// It is stable for the lifetime of the local identity and must remain
    /// confidential. The API server receives only a one-way lookup token
    /// derived from this value.
    pub fn enrollment_code(&self) -> PublicResult<Zeroizing<String>> {
        let mut code = hkdf_expand(&self.seed, AGENT_ENROLLMENT_CODE_DERIVATION)?;
        let encoded = Zeroizing::new(URL_SAFE_NO_PAD.encode(code));
        code.zeroize();
        Ok(encoded)
    }

    pub fn build_token_mint_assertion(
        &self,
        agent_id: Uuid,
        audience: &str,
    ) -> PublicResult<String> {
        build_agent_assertion(agent_id, &self.signing_key, audience)
    }
}

impl AgentEnrollmentDraft {
    pub fn key_material(&self) -> &AgentKeyMaterial {
        &self.key_material
    }

    pub fn is_resumed(&self) -> bool {
        self.resumed
    }

    pub fn proposed_handle(&self) -> Option<&str> {
        self.stored.proposed_handle.as_deref()
    }

    pub fn work_list_id(&self) -> Uuid {
        self.stored.work_list_id
    }

    pub fn repository_root(&self) -> &Path {
        &self.stored.repository_root
    }

    pub fn registration(&self) -> Option<&AgentEnrollmentRegistration> {
        self.registration.as_ref()
    }

    /// Durably records the server-assigned identity before local identity
    /// persistence or terminal rendering. The receipt is immutable and an
    /// exact replay is idempotent.
    pub fn record_registration(
        &mut self,
        registration: AgentEnrollmentRegistration,
    ) -> PublicResult<()> {
        validate_enrollment_registration(&self.stored, &self.key_material, &registration)?;
        if let Some(existing) = self.registration.as_ref() {
            return if existing == &registration {
                Ok(())
            } else {
                Err(PublicError::conflict(
                    "agent enrollment already has a different registered identity",
                ))
            };
        }
        save_stored_enrollment_registration(&StoredAgentEnrollmentRegistration {
            schema_version: AGENT_ENROLLMENT_DRAFT_SCHEMA_VERSION,
            context_id: self.stored.context_id.clone(),
            registration: registration.clone(),
        })?;
        self.registration = Some(registration);
        Ok(())
    }

    /// Finds an identity already persisted from this exact draft. This is the
    /// recovery path for drafts written by older clients that crashed after
    /// saving identity.json but before removing the draft.
    pub fn matching_local_identity(&self) -> PublicResult<Option<AgentIdentity>> {
        find_identity_for_enrollment_draft(&self.stored, &self.key_material)
    }

    pub fn complete(self) -> PublicResult<()> {
        let directory = enrollment_drafts_root()?;
        // Remove the receipt first. If the process stops between these two
        // removals, the surviving draft is recovered through the persisted
        // identity scan and cannot register the same keys again.
        remove_enrollment_file_if_present(
            &enrollment_registration_path(&self.stored.context_id)?,
            "registration receipt",
        )?;
        remove_enrollment_file_if_present(
            &enrollment_draft_path(&self.stored.context_id)?,
            "enrollment draft",
        )?;
        sync_directory(&directory)
    }
}

pub struct SavePendingAgentIdentity<'a> {
    pub agent_id: Uuid,
    pub api_url: &'a str,
    pub proposed_handle: Option<String>,
    pub auth_public_key: &'a str,
    pub recipient_public_key: &'a str,
    pub fingerprint: &'a str,
    pub enrollment_expires_at: DateTime<Utc>,
    pub work_list_id: Uuid,
    pub repository_root: &'a Path,
}

pub fn canonicalize_agent_handle(value: &str) -> PublicResult<String> {
    let handle = value.trim().to_ascii_lowercase();
    if handle.len() < 2
        || handle.len() > MAX_AGENT_HANDLE_LEN
        || !handle
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        || !handle.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_')
        })
    {
        return Err(PublicError::validation(
            "agent handle must be 2-48 lowercase letters, numbers, hyphens, or underscores and start with a letter or number",
        ));
    }
    Ok(handle)
}

pub fn canonicalize_agent_display_name(value: &str) -> PublicResult<String> {
    let display_name = value.trim();
    if display_name.is_empty()
        || display_name.chars().count() > MAX_AGENT_DISPLAY_NAME_LEN
        || display_name
            .chars()
            .any(|character| character.is_control() || is_bidi_control(character))
    {
        return Err(PublicError::validation("invalid agent display name"));
    }
    Ok(display_name.to_string())
}

pub fn generate_agent_key_material() -> PublicResult<AgentKeyMaterial> {
    let mut seed = [0_u8; AGENT_KEY_BYTES];
    OsRng
        .try_fill_bytes(&mut seed)
        .map_err(|error| PublicError::crypto(format!("failed to generate agent seed: {error}")))?;
    agent_key_material_from_seed(seed)
}

pub fn agent_key_material_from_seed(seed: [u8; AGENT_KEY_BYTES]) -> PublicResult<AgentKeyMaterial> {
    let mut auth_seed = hkdf_expand(&seed, b"sealtask.agent.auth.v1")?;
    let mut recipient_seed = hkdf_expand(&seed, b"sealtask.agent.recipient.v1")?;
    let signing_key = SigningKey::from_bytes(&auth_seed);
    let auth_public_key = signing_key.verifying_key().to_bytes();
    let recipient_private = X25519StaticSecret::from(recipient_seed);
    let recipient_private_key = recipient_private.to_bytes();
    let recipient_public_key = X25519PublicKey::from(&recipient_private).to_bytes();
    auth_seed.zeroize();
    recipient_seed.zeroize();
    Ok(AgentKeyMaterial {
        seed,
        signing_key,
        auth_public_key,
        recipient_private_key,
        recipient_public_key,
    })
}

pub fn prepare_agent_enrollment_draft(
    input: PrepareAgentEnrollmentDraft<'_>,
) -> PublicResult<AgentEnrollmentDraft> {
    if let Some(proposed_handle) = input.proposed_handle.as_deref() {
        require_canonical_agent_handle(proposed_handle)?;
    }
    let api_url = canonicalize_agent_audience(input.api_url)?;
    let repository_root = canonicalize_repository_root(input.repository_root)?;
    let context_id = enrollment_draft_context_id(
        &api_url,
        input.proposed_handle.as_deref(),
        input.work_list_id,
        &repository_root,
    )?;
    let lock = AgentEnrollmentDraftFileLock::acquire(&context_id)?;

    let (stored, key_material, resumed) =
        if let Some(stored) = load_stored_enrollment_draft(&context_id)? {
            validate_enrollment_draft_context(
                &stored,
                &api_url,
                input.proposed_handle.as_deref(),
                input.work_list_id,
                &repository_root,
            )?;
            let seed = decode_seed(&stored.seed)?;
            let key_material = agent_key_material_from_seed(seed)?;
            (stored, key_material, true)
        } else {
            let key_material = generate_agent_key_material()?;
            let stored = StoredAgentEnrollmentDraft {
                schema_version: AGENT_ENROLLMENT_DRAFT_SCHEMA_VERSION,
                context_id,
                api_url,
                proposed_handle: input.proposed_handle,
                work_list_id: input.work_list_id,
                repository_root,
                seed: STANDARD_NO_PAD.encode(key_material.seed),
                created_at: Utc::now(),
            };
            save_stored_enrollment_draft(&stored)?;
            (stored, key_material, false)
        };

    let registration = load_stored_enrollment_registration(&stored.context_id)?
        .map(|receipt| {
            validate_enrollment_registration(&stored, &key_material, &receipt.registration)?;
            Ok(receipt.registration)
        })
        .transpose()?;

    Ok(AgentEnrollmentDraft {
        stored,
        key_material,
        registration,
        resumed,
        _lock: lock,
    })
}

pub fn save_pending_agent_identity(
    input: SavePendingAgentIdentity<'_>,
    key_material: &AgentKeyMaterial,
) -> PublicResult<AgentIdentity> {
    if let Some(proposed_handle) = input.proposed_handle.as_deref() {
        require_canonical_agent_handle(proposed_handle)?;
    }
    let api_url = canonicalize_agent_audience(input.api_url)?;
    let repository_root = canonicalize_repository_root(input.repository_root)?;
    let expected_auth = STANDARD_NO_PAD.encode(key_material.auth_public_key());
    let expected_recipient = STANDARD_NO_PAD.encode(key_material.recipient_public_key());
    if decode_standard_key("auth public key", input.auth_public_key)?
        != *key_material.auth_public_key()
        || decode_standard_key("recipient public key", input.recipient_public_key)?
            != *key_material.recipient_public_key()
        || input.fingerprint != key_material.fingerprint()
    {
        return Err(PublicError::crypto(
            "agent enrollment response does not match generated key material",
        ));
    }
    let now = Utc::now();
    let identity = AgentIdentity {
        agent_id: input.agent_id,
        api_url,
        status: LocalAgentStatus::Pending,
        proposed_handle: input.proposed_handle,
        handle: None,
        display_name: None,
        fingerprint: input.fingerprint.to_string(),
        auth_public_key: expected_auth,
        recipient_public_key: expected_recipient,
        enrollment_expires_at: Some(input.enrollment_expires_at),
        project: AgentProjectBinding {
            work_list_id: input.work_list_id,
            repository_root,
            permission_preset: "assigned_task_worker".to_string(),
            instructions_revision: 1,
        },
        created_at: now,
        updated_at: now,
    };
    let stored = StoredAgentIdentity {
        schema_version: AGENT_IDENTITY_SCHEMA_VERSION,
        identity: identity.clone(),
        seed: STANDARD_NO_PAD.encode(key_material.seed),
    };
    ensure_agent_keys_are_not_reused(&stored)?;
    let _lock = AgentIdentityFileLock::acquire(input.agent_id)?;
    if let Some(existing) = load_stored_identity(input.agent_id)? {
        return matching_pending_identity(existing, &stored);
    }
    save_stored_identity(&stored, true)?;
    Ok(identity)
}

pub fn activate_agent_identity(
    agent_id: Uuid,
    handle: String,
    display_name: String,
    work_list_id: Uuid,
    instructions_revision: i64,
) -> PublicResult<AgentIdentity> {
    require_canonical_agent_handle(&handle)?;
    require_canonical_agent_display_name(&display_name)?;
    update_agent_identity(agent_id, |identity| {
        if identity.status == LocalAgentStatus::Revoked {
            return Err(PublicError::conflict(
                "revoked local agent identity cannot be activated",
            ));
        }
        if identity.project.work_list_id != work_list_id {
            return Err(PublicError::conflict(
                "approved agent project does not match its local project binding",
            ));
        }
        if identity.status == LocalAgentStatus::Active
            && (identity.handle.as_deref() != Some(handle.as_str())
                || identity.display_name.as_deref() != Some(display_name.as_str())
                || identity.project.instructions_revision != instructions_revision)
        {
            return Err(PublicError::conflict(
                "active local agent identity metadata cannot be replaced",
            ));
        }
        identity.status = LocalAgentStatus::Active;
        identity.handle = Some(handle);
        identity.display_name = Some(display_name);
        identity.enrollment_expires_at = None;
        identity.project.instructions_revision = instructions_revision;
        Ok(())
    })
}

pub fn mark_agent_identity_revoked(agent_id: Uuid) -> PublicResult<AgentIdentity> {
    update_agent_identity(agent_id, |identity| {
        identity.status = LocalAgentStatus::Revoked;
        Ok(())
    })
}

pub fn mark_agent_identity_expired(agent_id: Uuid) -> PublicResult<AgentIdentity> {
    update_agent_identity(agent_id, |identity| {
        if identity.status == LocalAgentStatus::Pending {
            identity.status = LocalAgentStatus::Expired;
        }
        Ok(())
    })
}

pub fn load_agent_identity(agent_id: Uuid) -> PublicResult<Option<AgentIdentity>> {
    load_stored_identity(agent_id).map(|stored| stored.map(|stored| stored.identity.clone()))
}

pub fn load_agent_key_material(agent_id: Uuid) -> PublicResult<Option<AgentKeyMaterial>> {
    let Some(stored) = load_stored_identity(agent_id)? else {
        return Ok(None);
    };
    let seed = decode_seed(&stored.seed)?;
    let material = agent_key_material_from_seed(seed)?;
    validate_key_material(&stored.identity, &material)?;
    Ok(Some(material))
}

pub fn list_agent_identities() -> PublicResult<Vec<AgentIdentity>> {
    let listing = list_agent_identities_with_failures()?;
    if let Some(failure) = listing.failures.first() {
        return Err(PublicError::unexpected(format!(
            "failed to load agent identity {}: {}",
            failure.agent_id, failure.message
        )));
    }
    Ok(listing.identities)
}

pub fn list_agent_identities_with_failures() -> PublicResult<AgentIdentityListing> {
    let root = agents_root()?;
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AgentIdentityListing::default());
        }
        Err(error) => {
            return Err(PublicError::unexpected(format!(
                "failed to list agent identities in {}: {error}",
                root.display()
            )));
        }
    };
    let mut listing = AgentIdentityListing::default();
    for entry in entries {
        let entry = entry.map_err(|error| {
            PublicError::unexpected(format!("failed to inspect agent identity: {error}"))
        })?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Ok(agent_id) = Uuid::parse_str(&name) else {
            continue;
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                listing.discovered_identities += 1;
                listing.failures.push(AgentIdentityLoadFailure {
                    agent_id,
                    message: format!("failed to inspect identity directory: {error}"),
                });
                continue;
            }
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        listing.discovered_identities += 1;
        match load_agent_identity(agent_id) {
            Ok(Some(identity)) => listing.identities.push(identity),
            Ok(None) => listing.failures.push(AgentIdentityLoadFailure {
                agent_id,
                message: "identity metadata is missing".to_string(),
            }),
            Err(error) => listing.failures.push(AgentIdentityLoadFailure {
                agent_id,
                message: error.to_string(),
            }),
        }
    }
    listing
        .identities
        .sort_by_key(|identity| (identity.created_at, identity.agent_id));
    listing.failures.sort_by_key(|failure| failure.agent_id);
    Ok(listing)
}

pub fn agent_identity_path(agent_id: Uuid) -> PublicResult<PathBuf> {
    Ok(agents_root()?
        .join(agent_id.to_string())
        .join(AGENT_IDENTITY_FILE))
}

pub fn agent_fingerprint(auth_public_key: &[u8], recipient_public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(AGENT_FINGERPRINT_CONTEXT);
    hasher.update(auth_public_key);
    hasher.update(recipient_public_key);
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub fn canonicalize_agent_audience(api_url: &str) -> PublicResult<String> {
    let mut url = reqwest::Url::parse(api_url.trim()).map_err(|error| {
        PublicError::validation(format!("agent API URL must be absolute HTTP(S): {error}"))
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(PublicError::validation(
            "agent API URL must be an absolute HTTP(S) base URL without credentials, query, or fragment",
        ));
    }
    if matches!(
        (url.scheme(), url.port()),
        ("http", Some(80)) | ("https", Some(443))
    ) {
        url.set_port(None)
            .map_err(|_| PublicError::validation("failed to canonicalize agent API port"))?;
    }
    if url.path() == "/" {
        url.set_path("");
    } else {
        let path = url.path().trim_end_matches('/').to_string();
        url.set_path(&path);
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn build_agent_assertion(
    agent_id: Uuid,
    signing_key: &SigningKey,
    audience: &str,
) -> PublicResult<String> {
    let audience = canonicalize_agent_audience(audience)?;
    let now = Utc::now().timestamp();
    let header = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({"alg": "EdDSA", "typ": "JWT"})).map_err(
            |error| {
                PublicError::unexpected(format!("failed to encode agent assertion header: {error}"))
            },
        )?,
    );
    let claims = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "iss": agent_id,
            "aud": audience,
            "jti": Uuid::now_v7(),
            "iat": now,
            "exp": now + AGENT_ASSERTION_TTL_SECONDS,
            "purpose": AGENT_ASSERTION_PURPOSE_TOKEN_MINT,
        }))
        .map_err(|error| {
            PublicError::unexpected(format!("failed to encode agent assertion claims: {error}"))
        })?,
    );
    let signing_input = format!("{header}.{claims}");
    let signature = signing_key.sign(signing_input.as_bytes());
    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

fn update_agent_identity(
    agent_id: Uuid,
    update: impl FnOnce(&mut AgentIdentity) -> PublicResult<()>,
) -> PublicResult<AgentIdentity> {
    let _lock = AgentIdentityFileLock::acquire(agent_id)?;
    let mut stored = load_stored_identity(agent_id)?
        .ok_or_else(|| PublicError::not_found("local agent identity not found"))?;
    update(&mut stored.identity)?;
    stored.identity.updated_at = Utc::now();
    save_stored_identity(&stored, false)?;
    Ok(stored.identity.clone())
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn canonicalize_repository_root(repository_root: &Path) -> PublicResult<PathBuf> {
    let repository_root = repository_root.canonicalize().map_err(|error| {
        PublicError::validation(format!(
            "failed to resolve agent repository root {}: {error}",
            repository_root.display()
        ))
    })?;
    if !repository_root.is_dir() {
        return Err(PublicError::validation(
            "agent repository root must be a directory",
        ));
    }
    Ok(repository_root)
}

fn enrollment_draft_context_id(
    api_url: &str,
    proposed_handle: Option<&str>,
    work_list_id: Uuid,
    repository_root: &Path,
) -> PublicResult<String> {
    let context = serde_json::to_vec(&(api_url, proposed_handle, work_list_id, repository_root))
        .map_err(|error| {
            PublicError::validation(format!(
                "agent enrollment context cannot be persisted: {error}"
            ))
        })?;
    let mut hasher = Sha256::new();
    hasher.update(AGENT_ENROLLMENT_DRAFT_CONTEXT);
    hasher.update(context);
    Ok(URL_SAFE_NO_PAD.encode(hasher.finalize()))
}

fn validate_enrollment_draft_context(
    stored: &StoredAgentEnrollmentDraft,
    api_url: &str,
    proposed_handle: Option<&str>,
    work_list_id: Uuid,
    repository_root: &Path,
) -> PublicResult<()> {
    if stored.schema_version != AGENT_ENROLLMENT_DRAFT_SCHEMA_VERSION {
        return Err(PublicError::validation(format!(
            "unsupported agent enrollment draft schema version {}",
            stored.schema_version
        )));
    }
    let expected_context_id =
        enrollment_draft_context_id(api_url, proposed_handle, work_list_id, repository_root)?;
    if stored.context_id != expected_context_id
        || stored.api_url != api_url
        || stored.proposed_handle.as_deref() != proposed_handle
        || stored.work_list_id != work_list_id
        || stored.repository_root != repository_root
    {
        return Err(PublicError::crypto(
            "local agent enrollment draft does not match its registration context",
        ));
    }
    if let Some(handle) = stored.proposed_handle.as_deref() {
        require_canonical_agent_handle(handle)?;
    }
    Ok(())
}

fn validate_enrollment_registration(
    draft: &StoredAgentEnrollmentDraft,
    key_material: &AgentKeyMaterial,
    registration: &AgentEnrollmentRegistration,
) -> PublicResult<()> {
    if registration.agent_id.is_nil()
        || registration.proposed_handle != draft.proposed_handle
        || registration.auth_public_key != STANDARD_NO_PAD.encode(key_material.auth_public_key())
        || registration.recipient_public_key
            != STANDARD_NO_PAD.encode(key_material.recipient_public_key())
        || registration.fingerprint != key_material.fingerprint()
    {
        return Err(PublicError::crypto(
            "agent enrollment registration does not match its durable draft",
        ));
    }
    Ok(())
}

fn find_identity_for_enrollment_draft(
    draft: &StoredAgentEnrollmentDraft,
    key_material: &AgentKeyMaterial,
) -> PublicResult<Option<AgentIdentity>> {
    let expected_auth = STANDARD_NO_PAD.encode(key_material.auth_public_key());
    let expected_recipient = STANDARD_NO_PAD.encode(key_material.recipient_public_key());
    let expected_fingerprint = key_material.fingerprint();
    let mut matched = None;
    for identity in list_agent_identities()? {
        let shares_key = identity.auth_public_key == expected_auth
            || identity.recipient_public_key == expected_recipient;
        if !shares_key {
            continue;
        }
        let exact_context = identity.auth_public_key == expected_auth
            && identity.recipient_public_key == expected_recipient
            && identity.fingerprint == expected_fingerprint
            && identity.api_url == draft.api_url
            && identity.proposed_handle == draft.proposed_handle
            && identity.project.work_list_id == draft.work_list_id
            && identity.project.repository_root == draft.repository_root;
        if !exact_context || matched.is_some() {
            return Err(PublicError::conflict(
                "local agent key material is already bound to another identity or enrollment context",
            ));
        }
        matched = Some(identity);
    }
    Ok(matched)
}

fn ensure_agent_keys_are_not_reused(expected: &StoredAgentIdentity) -> PublicResult<()> {
    for identity in list_agent_identities()? {
        if identity.agent_id != expected.identity.agent_id
            && (identity.auth_public_key == expected.identity.auth_public_key
                || identity.recipient_public_key == expected.identity.recipient_public_key)
        {
            return Err(PublicError::conflict(
                "agent key material is already bound to a different local identity",
            ));
        }
    }
    Ok(())
}

fn matching_pending_identity(
    existing: StoredAgentIdentity,
    expected: &StoredAgentIdentity,
) -> PublicResult<AgentIdentity> {
    let existing_identity = &existing.identity;
    let expected_identity = &expected.identity;
    if existing.schema_version == expected.schema_version
        && existing.seed == expected.seed
        && existing_identity.agent_id == expected_identity.agent_id
        && existing_identity.api_url == expected_identity.api_url
        && matches!(
            existing_identity.status,
            LocalAgentStatus::Pending | LocalAgentStatus::Expired
        )
        && existing_identity.proposed_handle == expected_identity.proposed_handle
        && existing_identity.handle.is_none()
        && existing_identity.display_name.is_none()
        && existing_identity.fingerprint == expected_identity.fingerprint
        && existing_identity.auth_public_key == expected_identity.auth_public_key
        && existing_identity.recipient_public_key == expected_identity.recipient_public_key
        && existing_identity.enrollment_expires_at == expected_identity.enrollment_expires_at
        && existing_identity.project == expected_identity.project
    {
        return Ok(existing_identity.clone());
    }
    Err(PublicError::conflict(
        "local agent identity already exists with different enrollment metadata",
    ))
}

fn save_stored_enrollment_draft(stored: &StoredAgentEnrollmentDraft) -> PublicResult<()> {
    validate_enrollment_draft_context(
        stored,
        &stored.api_url,
        stored.proposed_handle.as_deref(),
        stored.work_list_id,
        &stored.repository_root,
    )?;
    let directory = enrollment_drafts_root()?;
    create_private_directory(&directory)?;
    let target = enrollment_draft_path(&stored.context_id)?;
    reject_symlink_if_present(&target)?;
    if target.exists() {
        return Err(PublicError::conflict(
            "local agent enrollment draft already exists",
        ));
    }
    let mut body = Zeroizing::new(serde_json::to_vec_pretty(stored).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to serialize agent enrollment draft: {error}"
        ))
    })?);
    body.push(b'\n');
    if body.len() as u64 > MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES {
        return Err(PublicError::validation(
            "agent enrollment draft file is too large",
        ));
    }
    let temporary = directory.join(format!(".draft-{}.tmp", Uuid::now_v7()));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to create agent enrollment draft file: {error}"
        ))
    })?;
    let result = (|| {
        file.write_all(&body).map_err(|error| {
            PublicError::unexpected(format!(
                "failed to write agent enrollment draft file: {error}"
            ))
        })?;
        file.sync_all().map_err(|error| {
            PublicError::unexpected(format!(
                "failed to sync agent enrollment draft file: {error}"
            ))
        })?;
        drop(file);
        fs::rename(&temporary, &target).map_err(|error| {
            PublicError::unexpected(format!(
                "failed to persist agent enrollment draft file: {error}"
            ))
        })?;
        sync_directory(&directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn save_stored_enrollment_registration(
    stored: &StoredAgentEnrollmentRegistration,
) -> PublicResult<()> {
    if stored.schema_version != AGENT_ENROLLMENT_DRAFT_SCHEMA_VERSION {
        return Err(PublicError::validation(
            "unsupported agent enrollment registration schema version",
        ));
    }
    let directory = enrollment_drafts_root()?;
    create_private_directory(&directory)?;
    let target = enrollment_registration_path(&stored.context_id)?;
    reject_symlink_if_present(&target)?;
    if target.exists() {
        return Err(PublicError::conflict(
            "agent enrollment registration receipt already exists",
        ));
    }
    let mut body = Zeroizing::new(serde_json::to_vec_pretty(stored).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to serialize agent enrollment registration receipt: {error}"
        ))
    })?);
    body.push(b'\n');
    if body.len() as u64 > MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES {
        return Err(PublicError::validation(
            "agent enrollment registration receipt is too large",
        ));
    }
    let temporary = directory.join(format!(".registration-{}.tmp", Uuid::now_v7()));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to create agent enrollment registration receipt: {error}"
        ))
    })?;
    let result = (|| {
        file.write_all(&body).map_err(|error| {
            PublicError::unexpected(format!(
                "failed to write agent enrollment registration receipt: {error}"
            ))
        })?;
        file.sync_all().map_err(|error| {
            PublicError::unexpected(format!(
                "failed to sync agent enrollment registration receipt: {error}"
            ))
        })?;
        drop(file);
        fs::rename(&temporary, &target).map_err(|error| {
            PublicError::unexpected(format!(
                "failed to persist agent enrollment registration receipt: {error}"
            ))
        })?;
        sync_directory(&directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn load_stored_enrollment_registration(
    context_id: &str,
) -> PublicResult<Option<StoredAgentEnrollmentRegistration>> {
    let path = enrollment_registration_path(context_id)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(PublicError::unexpected(format!(
                "failed to inspect agent enrollment registration receipt {}: {error}",
                path.display()
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PublicError::unexpected(
            "agent enrollment registration receipt must be a regular file and not a symlink",
        ));
    }
    validate_private_file_permissions(&metadata)?;
    if metadata.len() > MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES {
        return Err(PublicError::unexpected(
            "agent enrollment registration receipt is too large",
        ));
    }
    let file = OpenOptions::new().read(true).open(&path).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to open agent enrollment registration receipt: {error}"
        ))
    })?;
    let mut body = Zeroizing::new(Vec::with_capacity(metadata.len() as usize));
    file.take(MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to read agent enrollment registration receipt: {error}"
            ))
        })?;
    if body.len() as u64 > MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES {
        return Err(PublicError::unexpected(
            "agent enrollment registration receipt is too large",
        ));
    }
    let stored: StoredAgentEnrollmentRegistration =
        serde_json::from_slice(&body).map_err(|error| {
            PublicError::unexpected(format!(
                "failed to parse agent enrollment registration receipt: {error}"
            ))
        })?;
    if stored.schema_version != AGENT_ENROLLMENT_DRAFT_SCHEMA_VERSION
        || stored.context_id != context_id
    {
        return Err(PublicError::crypto(
            "agent enrollment registration receipt does not match its draft",
        ));
    }
    Ok(Some(stored))
}

fn load_stored_enrollment_draft(
    context_id: &str,
) -> PublicResult<Option<StoredAgentEnrollmentDraft>> {
    let path = enrollment_draft_path(context_id)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(PublicError::unexpected(format!(
                "failed to inspect agent enrollment draft {}: {error}",
                path.display()
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PublicError::unexpected(
            "agent enrollment draft path must be a regular file and not a symlink",
        ));
    }
    validate_private_file_permissions(&metadata)?;
    if metadata.len() > MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES {
        return Err(PublicError::unexpected(
            "agent enrollment draft file is too large",
        ));
    }
    let file = OpenOptions::new().read(true).open(&path).map_err(|error| {
        PublicError::unexpected(format!("failed to open agent enrollment draft: {error}"))
    })?;
    let mut body = Zeroizing::new(Vec::with_capacity(metadata.len() as usize));
    file.take(MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| {
            PublicError::unexpected(format!("failed to read agent enrollment draft: {error}"))
        })?;
    if body.len() as u64 > MAX_AGENT_ENROLLMENT_DRAFT_FILE_BYTES {
        return Err(PublicError::unexpected(
            "agent enrollment draft file is too large",
        ));
    }
    let stored: StoredAgentEnrollmentDraft = serde_json::from_slice(&body).map_err(|error| {
        PublicError::unexpected(format!("failed to parse agent enrollment draft: {error}"))
    })?;
    if stored.context_id != context_id {
        return Err(PublicError::crypto(
            "agent enrollment draft path does not match its context",
        ));
    }
    Ok(Some(stored))
}

fn save_stored_identity(stored: &StoredAgentIdentity, create_new: bool) -> PublicResult<()> {
    validate_stored_identity(stored)?;
    let root = agents_root()?;
    create_private_directory(&root)?;
    let directory = root.join(stored.identity.agent_id.to_string());
    create_private_directory(&directory)?;
    let target = directory.join(AGENT_IDENTITY_FILE);
    if create_new && target.exists() {
        return Err(PublicError::conflict("local agent identity already exists"));
    }
    reject_symlink_if_present(&target)?;
    let mut body = zeroize::Zeroizing::new(serde_json::to_vec_pretty(stored).map_err(|error| {
        PublicError::unexpected(format!("failed to serialize agent identity: {error}"))
    })?);
    body.push(b'\n');
    if body.len() as u64 > MAX_AGENT_IDENTITY_FILE_BYTES {
        return Err(PublicError::validation("agent identity file is too large"));
    }
    let temporary = directory.join(format!(".identity-{}.tmp", Uuid::now_v7()));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| {
        PublicError::unexpected(format!("failed to create agent identity file: {error}"))
    })?;
    let result = (|| {
        file.write_all(&body).map_err(|error| {
            PublicError::unexpected(format!("failed to write agent identity file: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            PublicError::unexpected(format!("failed to sync agent identity file: {error}"))
        })?;
        drop(file);
        fs::rename(&temporary, &target).map_err(|error| {
            PublicError::unexpected(format!("failed to replace agent identity file: {error}"))
        })?;
        sync_directory(&directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn load_stored_identity(agent_id: Uuid) -> PublicResult<Option<StoredAgentIdentity>> {
    let path = agent_identity_path(agent_id)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(PublicError::unexpected(format!(
                "failed to inspect agent identity {}: {error}",
                path.display()
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PublicError::unexpected(
            "agent identity path must be a regular file and not a symlink",
        ));
    }
    validate_private_file_permissions(&metadata)?;
    if metadata.len() > MAX_AGENT_IDENTITY_FILE_BYTES {
        return Err(PublicError::unexpected("agent identity file is too large"));
    }
    let file = OpenOptions::new().read(true).open(&path).map_err(|error| {
        PublicError::unexpected(format!("failed to open agent identity: {error}"))
    })?;
    let mut body = zeroize::Zeroizing::new(Vec::with_capacity(metadata.len() as usize));
    file.take(MAX_AGENT_IDENTITY_FILE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| {
            PublicError::unexpected(format!("failed to read agent identity: {error}"))
        })?;
    if body.len() as u64 > MAX_AGENT_IDENTITY_FILE_BYTES {
        return Err(PublicError::unexpected("agent identity file is too large"));
    }
    let stored: StoredAgentIdentity = serde_json::from_slice(&body).map_err(|error| {
        PublicError::unexpected(format!("failed to parse agent identity: {error}"))
    })?;
    if stored.identity.agent_id != agent_id {
        return Err(PublicError::unexpected(
            "agent identity path does not match its identity",
        ));
    }
    validate_stored_identity(&stored)?;
    Ok(Some(stored))
}

fn validate_stored_identity(stored: &StoredAgentIdentity) -> PublicResult<()> {
    if stored.schema_version != AGENT_IDENTITY_SCHEMA_VERSION {
        return Err(PublicError::validation(format!(
            "unsupported agent identity schema version {}",
            stored.schema_version
        )));
    }
    if stored.identity.project.instructions_revision <= 0
        || stored.identity.project.permission_preset != "assigned_task_worker"
    {
        return Err(PublicError::validation(
            "invalid local agent project binding",
        ));
    }
    if let Some(proposed_handle) = stored.identity.proposed_handle.as_deref() {
        require_canonical_agent_handle(proposed_handle)?;
    }
    match (
        &stored.identity.status,
        stored.identity.handle.as_deref(),
        stored.identity.display_name.as_deref(),
    ) {
        (LocalAgentStatus::Pending | LocalAgentStatus::Expired, None, None)
        | (LocalAgentStatus::Revoked, None, None) => {}
        (
            LocalAgentStatus::Active | LocalAgentStatus::Revoked,
            Some(handle),
            Some(display_name),
        ) => {
            require_canonical_agent_handle(handle)?;
            require_canonical_agent_display_name(display_name)?;
        }
        _ => {
            return Err(PublicError::validation(
                "invalid local agent identity metadata",
            ));
        }
    }
    let seed = decode_seed(&stored.seed)?;
    let material = agent_key_material_from_seed(seed)?;
    validate_key_material(&stored.identity, &material)
}

fn require_canonical_agent_handle(value: &str) -> PublicResult<()> {
    if canonicalize_agent_handle(value)? != value {
        return Err(PublicError::validation(
            "agent handle must use its canonical form",
        ));
    }
    Ok(())
}

fn require_canonical_agent_display_name(value: &str) -> PublicResult<()> {
    if canonicalize_agent_display_name(value)? != value {
        return Err(PublicError::validation(
            "agent display name must use its canonical form",
        ));
    }
    Ok(())
}

fn validate_key_material(
    identity: &AgentIdentity,
    material: &AgentKeyMaterial,
) -> PublicResult<()> {
    if identity.auth_public_key != STANDARD_NO_PAD.encode(material.auth_public_key())
        || identity.recipient_public_key != STANDARD_NO_PAD.encode(material.recipient_public_key())
        || identity.fingerprint != material.fingerprint()
    {
        return Err(PublicError::crypto(
            "local agent identity key material does not match its public identity",
        ));
    }
    Ok(())
}

fn decode_seed(value: &str) -> PublicResult<[u8; AGENT_KEY_BYTES]> {
    let bytes = zeroize::Zeroizing::new(
        STANDARD_NO_PAD
            .decode(value.trim())
            .or_else(|_| STANDARD.decode(value.trim()))
            .map_err(|_| PublicError::crypto("invalid local agent seed encoding"))?,
    );
    if bytes.len() != AGENT_KEY_BYTES {
        return Err(PublicError::crypto("invalid local agent seed length"));
    }
    let mut seed = [0_u8; AGENT_KEY_BYTES];
    seed.copy_from_slice(&bytes);
    Ok(seed)
}

fn decode_standard_key(field: &str, value: &str) -> PublicResult<[u8; AGENT_KEY_BYTES]> {
    let bytes = STANDARD_NO_PAD
        .decode(value.trim())
        .or_else(|_| STANDARD.decode(value.trim()))
        .map_err(|_| PublicError::validation(format!("invalid {field} encoding")))?;
    bytes
        .try_into()
        .map_err(|_| PublicError::validation(format!("invalid {field} length")))
}

fn hkdf_expand(seed: &[u8; AGENT_KEY_BYTES], label: &[u8]) -> PublicResult<[u8; AGENT_KEY_BYTES]> {
    let hkdf = Hkdf::<Sha256>::new(None, seed);
    let mut output = [0_u8; AGENT_KEY_BYTES];
    hkdf.expand(label, &mut output)
        .map_err(|error| PublicError::crypto(format!("agent key derivation failed: {error}")))?;
    Ok(output)
}

fn agents_root() -> PublicResult<PathBuf> {
    Ok(config_dir()?.join(AGENTS_DIRECTORY))
}

fn enrollment_drafts_root() -> PublicResult<PathBuf> {
    Ok(agents_root()?.join(AGENT_ENROLLMENT_DRAFTS_DIRECTORY))
}

fn enrollment_draft_path(context_id: &str) -> PublicResult<PathBuf> {
    Ok(enrollment_drafts_root()?.join(format!("{context_id}.json")))
}

fn enrollment_registration_path(context_id: &str) -> PublicResult<PathBuf> {
    Ok(enrollment_drafts_root()?.join(format!("{context_id}{AGENT_ENROLLMENT_RECEIPT_SUFFIX}")))
}

fn remove_enrollment_file_if_present(path: &Path, label: &str) -> PublicResult<()> {
    reject_symlink_if_present(path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PublicError::unexpected(format!(
            "failed to remove completed agent {label} {}: {error}",
            path.display()
        ))),
    }
}

fn create_private_directory(path: &Path) -> PublicResult<()> {
    reject_symlink_if_present(path)?;
    fs::create_dir_all(path).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to create agent identity directory {}: {error}",
            path.display()
        ))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
            PublicError::unexpected(format!(
                "failed to secure agent identity directory {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn reject_symlink_if_present(path: &Path) -> PublicResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(PublicError::unexpected(format!(
            "agent identity path must not be a symlink: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PublicError::unexpected(format!(
            "failed to inspect agent identity path {}: {error}",
            path.display()
        ))),
    }
}

fn sync_directory(path: &Path) -> PublicResult<()> {
    let directory = OpenOptions::new().read(true).open(path).map_err(|error| {
        PublicError::unexpected(format!("failed to open agent identity directory: {error}"))
    })?;
    directory.sync_all().map_err(|error| {
        PublicError::unexpected(format!("failed to sync agent identity directory: {error}"))
    })
}

#[cfg(unix)]
fn validate_private_file_permissions(metadata: &fs::Metadata) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt as _;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::unexpected(
            "agent identity file permissions are too broad; expected mode 0600",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_private_file_permissions(_metadata: &fs::Metadata) -> PublicResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_derivation_and_assertion_are_stable_and_bound() {
        let material = agent_key_material_from_seed([0x51; AGENT_KEY_BYTES]).unwrap();
        let same = agent_key_material_from_seed([0x51; AGENT_KEY_BYTES]).unwrap();
        assert_eq!(material.auth_public_key(), same.auth_public_key());
        assert_eq!(material.recipient_public_key(), same.recipient_public_key());
        assert_eq!(
            material.enrollment_code().unwrap(),
            same.enrollment_code().unwrap()
        );
        let assertion = material
            .build_token_mint_assertion(Uuid::now_v7(), "https://api.example.test/")
            .unwrap();
        let claims = assertion.split('.').nth(1).unwrap();
        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).unwrap()).unwrap();
        assert_eq!(claims["aud"], "https://api.example.test");
        assert_eq!(claims["purpose"], "token_mint");
    }
}
