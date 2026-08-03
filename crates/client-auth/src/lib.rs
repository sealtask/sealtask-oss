#![cfg_attr(test, allow(clippy::unwrap_used))]

mod agent_identities;

pub use agent_identities::{
    AgentEnrollmentDraft, AgentEnrollmentRegistration, AgentIdentity, AgentIdentityListing,
    AgentIdentityLoadFailure, AgentKeyMaterial, AgentProjectBinding, LocalAgentStatus,
    PrepareAgentEnrollmentDraft, SavePendingAgentIdentity, activate_agent_identity,
    agent_fingerprint, agent_identity_path, agent_key_material_from_seed,
    canonicalize_agent_audience, canonicalize_agent_display_name, canonicalize_agent_handle,
    generate_agent_key_material, list_agent_identities, list_agent_identities_with_failures,
    load_agent_identity, load_agent_key_material, mark_agent_identity_expired,
    mark_agent_identity_revoked, prepare_agent_enrollment_draft, save_pending_agent_identity,
};

use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, TryLockError};
use std::io::{Read, Write};
#[cfg(all(unix, not(target_os = "redox")))]
use std::os::fd::{AsRawFd as _, FromRawFd as _};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration as StdDuration, Instant};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
#[cfg(unix)]
use cap_fs_ext::{OpenOptionsExt as _, OpenOptionsSyncExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
#[cfg(unix)]
use cap_std::fs::{DirBuilder, DirBuilderExt as _};
use chrono::{DateTime, Utc};
use generic_array::{ArrayLength, GenericArray};
use opaque_ke::{
    CipherSuite, ClientLogin, ClientLoginFinishParameters, ClientLoginStartResult,
    CredentialResponse, Identifiers, Ristretto255, errors::InternalError,
    key_exchange::tripledh::TripleDh, ksf::Ksf,
};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind, TransportFailureKind};

const OPAQUE_SERVER_ID: &[u8] = b"worklist.api";
const DATA_KEY_KEYCHAIN_SERVICE: &str = "sealtask.data-key";
const CONFIG_DIR_ENV: &str = "SEALTASK_CONFIG_DIR";
const PROFILE_ENV: &str = "SEALTASK_PROFILE";
const TEST_KEYCHAIN_DIR_ENV: &str = "SEALTASK_TEST_KEYCHAIN_DIR";
const MFA_CAPABILITIES_HEADER: &str = "X-Worklist-Auth-Capabilities";
const MFA_CAPABILITIES_VALUE: &str = "mfa-totp-v1";
const MFA_CHALLENGE_EXPIRED_MESSAGE: &str = "MFA challenge expired; restart sign-in";
const OPAQUE_EXPORT_KEY_BYTES: usize = 64;
const CREDENTIALS_FILE_NAME: &str = "credentials.json";
const CREDENTIALS_LOCK_FILE_NAME: &str = "credentials.lock";
const CREDENTIALS_CHANGED_MESSAGE: &str =
    "credentials changed while the command was running; retry the command";
const MAX_CREDENTIALS_FILE_BYTES: u64 = 64 * 1024;
const CREDENTIALS_LOCK_TIMEOUT: StdDuration = StdDuration::from_secs(2);
const CREDENTIALS_LOCK_RETRY: StdDuration = StdDuration::from_millis(20);
const CREDENTIAL_REFRESH_TIMEOUT: StdDuration = StdDuration::from_secs(30);
const MAX_RETRY_AFTER_SECONDS: u64 = 24 * 60 * 60;
const MAX_REFRESH_RESPONSE_BYTES: usize = 64 * 1024;
const DEFAULT_PROFILE: &str = "default";
const MAX_PROFILE_NAME_BYTES: usize = 64;

static LOCAL_STATE_OVERRIDE: OnceLock<LocalStateOverride> = OnceLock::new();

#[derive(Debug, Clone)]
struct LocalStateOverride {
    base_dir: Option<PathBuf>,
    profile: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnlockMode {
    SingleCommand,
    Daemon,
}

impl UnlockMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleCommand => "single_command",
            Self::Daemon => "daemon",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub user_id: Uuid,
    pub api_url: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Credentials {
    pub api_url: String,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub email: String,
    pub data_key_ciphertext: String,
}

struct CredentialsFileLock {
    file: Option<File>,
    store: CredentialStore,
}

struct CredentialStore {
    directory: Dir,
}

impl Credentials {
    pub fn is_access_expired(&self) -> bool {
        Utc::now() >= self.access_expires_at
    }

    pub fn is_refresh_expired(&self) -> bool {
        Utc::now() >= self.refresh_expires_at
    }

    pub fn access_expires_within(&self, seconds: i64) -> bool {
        Utc::now() + chrono::Duration::seconds(seconds) >= self.access_expires_at
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("api_url", &"<redacted>")
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .field("access_expires_at", &self.access_expires_at)
            .field("refresh_expires_at", &self.refresh_expires_at)
            .field("user_id", &self.user_id)
            .field("email", &"<redacted>")
            .field("data_key_ciphertext", &"<redacted>")
            .finish()
    }
}

impl CredentialsFileLock {
    fn acquire(dir: &Path) -> PublicResult<Self> {
        let store = CredentialStore::open(dir, true)?.ok_or_else(|| {
            PublicError::unexpected("failed to prepare credentials storage directory")
        })?;
        let mut create_options = secret_open_options();
        create_options.create_new(true).read(true).write(true);
        let (lock_file, created) = match store
            .directory
            .open_with(CREDENTIALS_LOCK_FILE_NAME, &create_options)
        {
            Ok(file) => (file.into_std(), true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut options = secret_open_options();
                options.read(true).write(true);
                let file = store
                    .directory
                    .open_with(CREDENTIALS_LOCK_FILE_NAME, &options)
                    .map(cap_std::fs::File::into_std)
                    .map_err(|error| secret_open_error("credentials lock file", error))?;
                (file, false)
            }
            Err(error) => {
                return Err(secret_open_error("credentials lock file", error));
            }
        };
        if created {
            set_secret_file_handle_permissions(&lock_file, "credentials lock file")?;
            store.sync()?;
        } else {
            validate_secret_file_handle(&lock_file, "credentials lock file")?;
        }

        let deadline = Instant::now() + CREDENTIALS_LOCK_TIMEOUT;
        loop {
            match lock_file.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(PublicError::conflict(
                            "credentials are locked by another process; retry the command",
                        ));
                    }
                    thread::sleep(CREDENTIALS_LOCK_RETRY);
                }
                Err(TryLockError::Error(error)) => {
                    return Err(PublicError::unexpected(format!(
                        "failed to lock credentials file: {error}"
                    )));
                }
            }
        }
        Ok(Self {
            file: Some(lock_file),
            store,
        })
    }

    async fn acquire_async(dir: &Path) -> PublicResult<Self> {
        let dir = dir.to_path_buf();
        tokio::task::spawn_blocking(move || Self::acquire(&dir))
            .await
            .map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to join credentials lock acquisition: {err}"
                ))
            })?
    }

    fn unlock(mut self) -> PublicResult<()> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        file.unlock().map_err(|err| {
            PublicError::unexpected(format!("failed to unlock credentials file: {err}"))
        })
    }

    fn store(&self) -> &CredentialStore {
        &self.store
    }
}

impl Drop for CredentialsFileLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

impl CredentialStore {
    fn open(path: &Path, create: bool) -> PublicResult<Option<Self>> {
        let Some((directory, created)) = open_directory_nofollow(path, create)? else {
            return Ok(None);
        };
        if created {
            set_secret_directory_handle_permissions(&directory)?;
        } else if create {
            // Preserve the historical login behavior of tightening an
            // owner-controlled config directory while refusing foreign-owned
            // storage above.
            restrict_secret_directory_handle_permissions(&directory)?;
        }
        validate_secret_directory_handle(&directory)?;
        Ok(Some(Self { directory }))
    }

    fn load(&self) -> PublicResult<Option<Credentials>> {
        let mut options = secret_open_options();
        options.read(true);
        let file = match self
            .directory
            .open_with(CREDENTIALS_FILE_NAME, &options)
            .map(cap_std::fs::File::into_std)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(secret_open_error("credentials file", error)),
        };
        validate_secret_file_handle(&file, "credentials file")?;
        let metadata = file.metadata().map_err(|error| {
            PublicError::unexpected(format!("failed to inspect credentials file: {error}"))
        })?;
        if metadata.len() > MAX_CREDENTIALS_FILE_BYTES {
            return Err(credentials_too_large_error());
        }

        let mut body = Zeroizing::new(Vec::with_capacity(metadata.len() as usize));
        file.take(MAX_CREDENTIALS_FILE_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|error| {
                PublicError::unexpected(format!("failed to read credentials file: {error}"))
            })?;
        if body.len() as u64 > MAX_CREDENTIALS_FILE_BYTES {
            return Err(credentials_too_large_error());
        }
        let credentials = serde_json::from_slice(&body).map_err(|error| {
            PublicError::unexpected(format!("failed to parse credentials file: {error}"))
        })?;
        Ok(Some(credentials))
    }

    fn save(&self, credentials: &Credentials) -> PublicResult<()> {
        if let Some(existing) = self.open_secret_file_if_present(CREDENTIALS_FILE_NAME)? {
            validate_secret_file_handle(&existing, "credentials file")?;
        }

        let credential_string_bytes = [
            &credentials.api_url,
            &credentials.access_token,
            &credentials.refresh_token,
            &credentials.email,
            &credentials.data_key_ciphertext,
        ]
        .into_iter()
        .try_fold(0_usize, |total, value| total.checked_add(value.len()))
        .ok_or_else(credentials_too_large_error)?;
        if credential_string_bytes as u64 > MAX_CREDENTIALS_FILE_BYTES {
            return Err(credentials_too_large_error());
        }
        let mut body = Zeroizing::new(serde_json::to_vec_pretty(credentials).map_err(|error| {
            PublicError::unexpected(format!("failed to encode credentials file: {error}"))
        })?);
        body.push(b'\n');
        if body.len() as u64 > MAX_CREDENTIALS_FILE_BYTES {
            return Err(credentials_too_large_error());
        }

        let temporary_name = format!(".credentials-{}.tmp", Uuid::now_v7());
        let mut options = secret_open_options();
        options.create_new(true).read(true).write(true);
        let temporary = self
            .directory
            .open_with(&temporary_name, &options)
            .map(cap_std::fs::File::into_std)
            .map_err(|error| secret_open_error("temporary credentials file", error))?;
        let result = (|| {
            set_secret_file_handle_permissions(&temporary, "temporary credentials file")?;
            (&temporary).write_all(&body).map_err(|error| {
                PublicError::unexpected(format!("failed to write credentials file: {error}"))
            })?;
            temporary.sync_all().map_err(|error| {
                PublicError::unexpected(format!("failed to sync credentials file: {error}"))
            })?;
            self.directory
                .rename(&temporary_name, &self.directory, CREDENTIALS_FILE_NAME)
                .map_err(|error| {
                    PublicError::unexpected(format!("failed to replace credentials file: {error}"))
                })?;
            self.sync()
        })();
        if result.is_err() {
            let _ = self.directory.remove_file(&temporary_name);
        }
        result
    }

    fn clear(&self) -> PublicResult<()> {
        let Some(file) = self.open_secret_file_if_present(CREDENTIALS_FILE_NAME)? else {
            return Ok(());
        };
        validate_secret_file_handle(&file, "credentials file")?;
        drop(file);
        self.directory
            .remove_file(CREDENTIALS_FILE_NAME)
            .map_err(|error| {
                PublicError::unexpected(format!("failed to remove credentials file: {error}"))
            })?;
        self.sync()
    }

    fn open_secret_file_if_present(&self, name: &str) -> PublicResult<Option<File>> {
        let mut options = secret_open_options();
        options.read(true);
        match self
            .directory
            .open_with(name, &options)
            .map(cap_std::fs::File::into_std)
        {
            Ok(file) => Ok(Some(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(secret_open_error("credentials file", error)),
        }
    }

    fn sync(&self) -> PublicResult<()> {
        sync_directory_handle(&self.directory)
    }
}

fn open_directory_nofollow(path: &Path, create: bool) -> PublicResult<Option<(Dir, bool)>> {
    let absolute = absolute_normalized_credentials_path(path)?;
    let root = credentials_filesystem_root(&absolute)?;
    let mut directory = Dir::open_ambient_dir(&root, ambient_authority()).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to open credentials storage filesystem root {}: {error}",
            root.display()
        ))
    })?;
    let mut walked = root;
    let mut final_directory_created = false;
    let normal_components = absolute
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    let mut normal_component_index = 0_usize;

    for component in absolute.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        normal_component_index += 1;
        let is_final = normal_component_index == normal_components;
        let next_path = walked.join(name);
        match directory.open_dir_nofollow(Path::new(name)) {
            Ok(child) => directory = child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !create => {
                return Ok(None);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let created_component = match create_private_directory(&directory, name) {
                    Ok(()) => true,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                    Err(error) => {
                        return Err(PublicError::unexpected(format!(
                            "failed to create credentials storage directory {}: {error}",
                            next_path.display()
                        )));
                    }
                };
                directory = directory
                    .open_dir_nofollow(Path::new(name))
                    .map_err(|error| credentials_directory_path_error(&next_path, error))?;
                if created_component {
                    set_secret_directory_handle_permissions(&directory)?;
                    sync_directory_handle(&directory)?;
                    final_directory_created |= is_final;
                }
            }
            Err(error) => {
                return Err(credentials_directory_path_error(&next_path, error));
            }
        }
        walked = next_path;
    }

    Ok(Some((directory, final_directory_created)))
}

fn absolute_normalized_credentials_path(path: &Path) -> PublicResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                PublicError::unexpected(format!(
                    "failed to resolve credentials storage directory: {error}"
                ))
            })?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(PublicError::unexpected(
                    "credentials storage path must not contain `..`",
                ));
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(PublicError::unexpected(
            "failed to resolve credentials storage directory as an absolute path",
        ));
    }
    canonicalize_trusted_macos_system_alias(normalized)
}

#[cfg(target_os = "macos")]
fn canonicalize_trusted_macos_system_alias(path: PathBuf) -> PublicResult<PathBuf> {
    use std::os::unix::fs::MetadataExt as _;

    let aliases = [
        (
            Path::new("/var"),
            Path::new("private/var"),
            Path::new("/private/var"),
        ),
        (
            Path::new("/tmp"),
            Path::new("private/tmp"),
            Path::new("/private/tmp"),
        ),
    ];
    let Some((alias, expected_relative_target, absolute_target, suffix)) =
        aliases.into_iter().find_map(|(alias, relative, target)| {
            path.strip_prefix(alias)
                .ok()
                .map(|suffix| (alias, relative, target, suffix.to_path_buf()))
        })
    else {
        return Ok(path);
    };

    let alias_metadata = fs::symlink_metadata(alias).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to inspect macOS credentials storage alias {}: {error}",
            alias.display()
        ))
    })?;
    if !alias_metadata.file_type().is_symlink() {
        return Ok(path);
    }
    let actual_target = fs::read_link(alias).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to resolve macOS credentials storage alias {}: {error}",
            alias.display()
        ))
    })?;
    let target_metadata = fs::symlink_metadata(absolute_target).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to inspect trusted macOS credentials storage target {}: {error}",
            absolute_target.display()
        ))
    })?;
    if alias_metadata.uid() != 0
        || (actual_target != expected_relative_target && actual_target != absolute_target)
        || target_metadata.uid() != 0
        || target_metadata.file_type().is_symlink()
        || !target_metadata.is_dir()
    {
        return Err(PublicError::unexpected(format!(
            "credentials storage path must not traverse an untrusted macOS system alias: {}",
            alias.display()
        )));
    }
    Ok(absolute_target.join(suffix))
}

#[cfg(not(target_os = "macos"))]
fn canonicalize_trusted_macos_system_alias(path: PathBuf) -> PublicResult<PathBuf> {
    Ok(path)
}

fn credentials_filesystem_root(path: &Path) -> PublicResult<PathBuf> {
    let mut root = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir | Component::Normal(_) => break,
        }
    }
    if root.as_os_str().is_empty() {
        return Err(PublicError::unexpected(
            "failed to resolve credentials storage filesystem root",
        ));
    }
    Ok(root)
}

#[cfg(unix)]
fn create_private_directory(parent: &Dir, name: &OsStr) -> std::io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    parent.create_dir_with(Path::new(name), &builder)
}

#[cfg(not(unix))]
fn create_private_directory(parent: &Dir, name: &OsStr) -> std::io::Result<()> {
    parent.create_dir(Path::new(name))
}

fn credentials_directory_path_error(path: &Path, error: std::io::Error) -> PublicError {
    if error.kind() == std::io::ErrorKind::InvalidInput
        || error.kind() == std::io::ErrorKind::NotADirectory
        || is_symlink_loop_error(&error)
    {
        PublicError::unexpected(format!(
            "credentials storage path must not traverse symlinks, reparse points, or non-directory components: {}",
            path.display()
        ))
    } else {
        PublicError::unexpected(format!(
            "failed to open credentials storage directory {}: {error}",
            path.display()
        ))
    }
}

fn validate_credentials_path(path: &Path) -> PublicResult<()> {
    if path.file_name().and_then(|name| name.to_str()) != Some(CREDENTIALS_FILE_NAME) {
        return Err(PublicError::unexpected(
            "credentials path does not name the expected credentials file",
        ));
    }
    Ok(())
}

fn credentials_parent(path: &Path) -> PublicResult<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| PublicError::unexpected("credentials path has no storage directory"))
}

fn secret_open_options() -> CapOpenOptions {
    let mut options = CapOpenOptions::new();
    options.follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.mode(0o600).nonblock(true);
    options
}

fn secret_open_error(label: &str, error: std::io::Error) -> PublicError {
    if error.kind() == std::io::ErrorKind::InvalidInput
        || error.kind() == std::io::ErrorKind::NotADirectory
        || is_symlink_loop_error(&error)
    {
        PublicError::unexpected(format!(
            "{label} must be a regular file and must not be a symlink or reparse point"
        ))
    } else {
        PublicError::unexpected(format!("failed to open {label}: {error}"))
    }
}

#[cfg(unix)]
fn is_symlink_loop_error(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::ELOOP)
}

#[cfg(not(unix))]
fn is_symlink_loop_error(_error: &std::io::Error) -> bool {
    false
}

fn credentials_too_large_error() -> PublicError {
    PublicError::unexpected(format!(
        "credentials file exceeds the {MAX_CREDENTIALS_FILE_BYTES}-byte limit"
    ))
}

#[cfg(unix)]
fn validate_secret_directory_handle(directory: &Dir) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let metadata = open_operable_directory_handle(directory)
        .and_then(|directory| directory.metadata())
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to inspect credentials storage directory: {error}"
            ))
        })?;
    if !metadata.is_dir() {
        return Err(PublicError::unexpected(
            "credentials storage path must be a directory",
        ));
    }
    validate_effective_owner(&metadata, "credentials storage directory")?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::unexpected(
            "credentials storage directory permissions are too broad; require mode 0700 or stricter",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_secret_directory_handle(directory: &Dir) -> PublicResult<()> {
    let metadata = directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .and_then(|directory| directory.metadata())
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to inspect credentials storage directory: {error}"
            ))
        })?;
    if !metadata.is_dir() {
        return Err(PublicError::unexpected(
            "credentials storage path must be a directory",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_secret_directory_handle_permissions(directory: &Dir) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let file = open_operable_directory_handle(directory).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to secure credentials storage directory: {error}"
        ))
    })?;
    let metadata = file.metadata().map_err(|error| {
        PublicError::unexpected(format!(
            "failed to inspect credentials storage directory: {error}"
        ))
    })?;
    validate_effective_owner(&metadata, "credentials storage directory")?;
    if metadata.permissions().mode() & 0o077 != 0 {
        file.set_permissions(fs::Permissions::from_mode(0o700))
            .map_err(|error| {
                PublicError::unexpected(format!(
                    "failed to secure credentials storage directory: {error}"
                ))
            })?;
        file.sync_all().map_err(|error| {
            PublicError::unexpected(format!(
                "failed to sync credentials storage directory: {error}"
            ))
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn restrict_secret_directory_handle_permissions(_directory: &Dir) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_secret_directory_handle_permissions(directory: &Dir) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let file = open_operable_directory_handle(directory).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to secure credentials storage directory: {error}"
        ))
    })?;
    file.set_permissions(fs::Permissions::from_mode(0o700))
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to secure credentials storage directory: {error}"
            ))
        })?;
    validate_secret_directory_handle(directory)
}

#[cfg(not(unix))]
fn set_secret_directory_handle_permissions(_directory: &Dir) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn validate_secret_file_handle(file: &File, label: &str) -> PublicResult<()> {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let metadata = file
        .metadata()
        .map_err(|error| PublicError::unexpected(format!("failed to inspect {label}: {error}")))?;
    if !metadata.is_file() {
        return Err(PublicError::unexpected(format!(
            "{label} must be a regular file"
        )));
    }
    validate_effective_owner(&metadata, label)?;
    if metadata.nlink() != 1 {
        return Err(PublicError::unexpected(format!(
            "{label} must have exactly one hard link"
        )));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::unexpected(format!(
            "{label} permissions are too broad; require mode 0600 or stricter"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_secret_file_handle(file: &File, label: &str) -> PublicResult<()> {
    use cap_fs_ext::MetadataExt as _;

    let metadata = cap_std::fs::Metadata::from_file(file)
        .map_err(|error| PublicError::unexpected(format!("failed to inspect {label}: {error}")))?;
    if !metadata.is_file() {
        return Err(PublicError::unexpected(format!(
            "{label} must be a regular file"
        )));
    }
    if metadata.nlink() != 1 {
        return Err(PublicError::unexpected(format!(
            "{label} must have exactly one hard link"
        )));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn validate_secret_file_handle(file: &File, label: &str) -> PublicResult<()> {
    let metadata = file
        .metadata()
        .map_err(|error| PublicError::unexpected(format!("failed to inspect {label}: {error}")))?;
    if !metadata.is_file() {
        return Err(PublicError::unexpected(format!(
            "{label} must be a regular file"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn set_secret_file_handle_permissions(file: &File, label: &str) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt as _;

    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|error| PublicError::unexpected(format!("failed to secure {label}: {error}")))?;
    validate_secret_file_handle(file, label)
}

#[cfg(not(unix))]
fn set_secret_file_handle_permissions(file: &File, label: &str) -> PublicResult<()> {
    validate_secret_file_handle(file, label)
}

#[cfg(unix)]
fn validate_effective_owner(metadata: &fs::Metadata, label: &str) -> PublicResult<()> {
    use std::os::unix::fs::MetadataExt as _;

    // SAFETY: `geteuid` has no preconditions and does not dereference pointers.
    let effective_uid = unsafe { libc::geteuid() };
    validate_effective_owner_ids(metadata.uid(), effective_uid, label)
}

#[cfg(unix)]
fn validate_effective_owner_ids(
    owner_uid: u32,
    effective_uid: u32,
    label: &str,
) -> PublicResult<()> {
    if owner_uid != effective_uid {
        return Err(PublicError::unexpected(format!(
            "{label} must be owned by the current effective user"
        )));
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "redox")))]
fn open_operable_directory_handle(directory: &Dir) -> std::io::Result<File> {
    // `cap_std::fs::Dir` uses `O_PATH` on Linux. Cloning that descriptor and
    // calling `fchmod` or `fsync` fails with `EBADF`. Open `.` directly
    // relative to the held capability with explicit read-only directory flags;
    // this cannot redirect through an ambient path or a symlink.
    const CURRENT_DIRECTORY: &[u8; 2] = b".\0";
    // SAFETY: `CURRENT_DIRECTORY` is NUL-terminated, the directory descriptor
    // remains borrowed for the call, and a successful descriptor is
    // transferred exactly once into `File`.
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            CURRENT_DIRECTORY.as_ptr().cast(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor == -1 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `openat` returned a new owned descriptor which has not been
    // transferred elsewhere.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(target_os = "redox")]
fn open_operable_directory_handle(directory: &Dir) -> std::io::Result<File> {
    directory.try_clone().map(cap_std::fs::Dir::into_std_file)
}

#[cfg(unix)]
fn sync_directory_handle(directory: &Dir) -> PublicResult<()> {
    open_operable_directory_handle(directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to sync credentials storage directory: {error}"
            ))
        })
}

#[cfg(not(unix))]
fn sync_directory_handle(_directory: &Dir) -> PublicResult<()> {
    // Windows capability directory handles do not necessarily carry the
    // access required by FlushFileBuffers. Credential files themselves are
    // still synced before handle-relative atomic replacement.
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PersistedDataKeyStatus {
    Available,
    Missing,
    Unavailable(String),
}

impl PersistedDataKeyStatus {
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStartResponse {
    pub server_login_state: String,
    pub session_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub refresh_expires_in: u64,
    pub token_type: String,
    pub user: UserResponse,
    pub data_key_ciphertext: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub timezone: String,
    pub avatar_color: String,
    pub theme_preference: String,
    pub email_verified: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub refresh_expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MfaMethod {
    Totp,
    BackupCode,
}

#[derive(Debug, Clone)]
pub struct MfaChallenge {
    pub methods: Vec<MfaMethod>,
    pub expires_in: u64,
    pub attempts_remaining: u8,
    pub requires_legacy_password: bool,
}

pub enum LoginOutcome {
    Authenticated(AuthResponse),
    MfaRequired {
        challenge: MfaChallenge,
        pending: PendingMfaLogin,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CompleteMfaLoginError {
    #[error("{message}")]
    Retryable {
        message: String,
        pending: PendingMfaLogin,
        attempts_remaining: Option<u8>,
        expires_in: Option<u64>,
        retry_after_seconds: Option<u64>,
    },
    #[error("{message}")]
    TotpLocked {
        message: String,
        pending: PendingMfaLogin,
    },
    #[error("{0}")]
    Terminal(String),
}

pub struct PendingMfaLogin {
    base_url: String,
    challenge_token: SecretString,
    challenge: MfaChallenge,
    expires_at: Instant,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct OpaqueExportKey([u8; OPAQUE_EXPORT_KEY_BYTES]);

impl OpaqueExportKey {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; OPAQUE_EXPORT_KEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for OpaqueExportKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

pub struct OpaqueLoginFinish {
    pub client_finish_message: String,
    pub export_key: OpaqueExportKey,
}

impl PendingMfaLogin {
    fn new(base_url: String, challenge_token: String, challenge: MfaChallenge) -> Self {
        Self::new_at(base_url, challenge_token, challenge, Instant::now())
    }

    fn new_at(
        base_url: String,
        challenge_token: String,
        challenge: MfaChallenge,
        now: Instant,
    ) -> Self {
        let expires_at = initial_mfa_deadline(now, challenge.expires_in);
        Self {
            base_url,
            challenge_token: SecretString::new(challenge_token),
            challenge,
            expires_at,
        }
    }

    #[must_use]
    pub fn challenge(&self) -> &MfaChallenge {
        &self.challenge
    }

    #[must_use]
    pub fn remaining_seconds(&self) -> u64 {
        self.remaining_seconds_at(Instant::now())
    }

    #[must_use]
    pub fn deadline(&self) -> Instant {
        self.expires_at
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn remaining_seconds_at(&self, now: Instant) -> u64 {
        remaining_mfa_seconds(self.expires_at, now)
    }

    fn remaining_duration_at(&self, now: Instant) -> StdDuration {
        self.expires_at.saturating_duration_since(now)
    }

    fn refresh_remaining_at(&mut self, now: Instant) -> u64 {
        let remaining = self.remaining_seconds_at(now);
        self.challenge.expires_in = remaining;
        remaining
    }

    fn apply_retry_metadata_at(
        &mut self,
        attempts_remaining: Option<u8>,
        server_expires_in: Option<u64>,
        now: Instant,
    ) -> u64 {
        if let Some(attempts_remaining) = attempts_remaining {
            self.challenge.attempts_remaining = attempts_remaining;
        }
        if let Some(server_expires_in) = server_expires_in
            && let Some(server_deadline) =
                now.checked_add(StdDuration::from_secs(server_expires_in))
            && server_deadline < self.expires_at
        {
            self.expires_at = server_deadline;
        }
        self.refresh_remaining_at(now)
    }
}

fn initial_mfa_deadline(now: Instant, expires_in: u64) -> Instant {
    now.checked_add(StdDuration::from_secs(expires_in))
        .unwrap_or(now)
}

fn remaining_mfa_seconds(expires_at: Instant, now: Instant) -> u64 {
    let remaining = expires_at.saturating_duration_since(now);
    remaining
        .as_secs()
        .saturating_add(u64::from(remaining.subsec_nanos() != 0))
}

impl Drop for PendingMfaLogin {
    fn drop(&mut self) {
        self.challenge_token.zeroize();
    }
}

impl fmt::Debug for PendingMfaLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingMfaLogin")
            .field("base_url", &self.base_url)
            .field("challenge_token", &"<redacted>")
            .field("challenge", &self.challenge)
            .finish()
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct SecretString(String);

impl SecretString {
    fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretMfaCode(String);

impl SecretMfaCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretMfaCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecondFactorRequiredResponse {
    status: MfaRequiredStatus,
    challenge_token: String,
    methods: Vec<MfaMethod>,
    expires_in: u64,
    attempts_remaining: u8,
    requires_legacy_password: bool,
}

#[derive(Clone, Copy, Deserialize)]
enum MfaRequiredStatus {
    #[serde(rename = "second_factor_required")]
    SecondFactorRequired,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MfaVerificationErrorResponse {
    error: String,
    message: String,
    attempts_remaining: Option<u8>,
    expires_in: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MfaLoginVerifyRequest<'a> {
    challenge_token: &'a str,
    code: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginStartRequest {
    email: String,
    client_login_state: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginFinishRequest {
    session_token: String,
    client_finish_message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    refresh_token: String,
}

pub struct ClientKsf {
    argon: Argon2<'static>,
}

impl Default for ClientKsf {
    fn default() -> Self {
        let params = Params::new(65536, 3, 4, None).expect("valid argon2 params");
        Self {
            argon: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }
}

impl Ksf for ClientKsf {
    fn hash<L: ArrayLength<u8>>(
        &self,
        input: GenericArray<u8, L>,
    ) -> Result<GenericArray<u8, L>, InternalError> {
        let mut output = GenericArray::default();
        self.argon
            .hash_password_into(&input, &[0; argon2::RECOMMENDED_SALT_LEN], &mut output)
            .map_err(|_| InternalError::KsfError)?;
        Ok(output)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClientCipherSuite;

impl CipherSuite for ClientCipherSuite {
    type OprfCs = Ristretto255;
    type KeyExchange = TripleDh<Ristretto255, Sha512>;
    type Ksf = ClientKsf;
}

pub fn configure_local_state(base_dir: Option<PathBuf>, profile: Option<&str>) -> PublicResult<()> {
    let profile = normalize_profile(profile)?;
    LOCAL_STATE_OVERRIDE
        .set(LocalStateOverride { base_dir, profile })
        .map_err(|_| PublicError::unexpected("local state configuration is already initialized"))
}

pub fn active_profile() -> PublicResult<String> {
    local_state_settings().map(|settings| settings.profile)
}

pub fn config_dir() -> PublicResult<PathBuf> {
    let settings = local_state_settings()?;
    let base_dir = match settings.base_dir {
        Some(path) => path,
        None => default_config_root()?,
    };
    if settings.profile == DEFAULT_PROFILE {
        Ok(base_dir)
    } else {
        Ok(base_dir.join("profiles").join(settings.profile))
    }
}

/// Returns the canonical base directory used for SealTask local state when no override is set.
pub fn default_config_root() -> PublicResult<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".sealtask"))
        .ok_or_else(|| PublicError::unexpected("could not determine home directory"))
}

pub fn credentials_path() -> PublicResult<PathBuf> {
    Ok(config_dir()?.join(CREDENTIALS_FILE_NAME))
}

pub fn normalize_api_url(api_url: &str) -> String {
    api_url.trim_end_matches('/').to_string()
}

fn local_state_settings() -> PublicResult<LocalStateOverride> {
    if let Some(settings) = LOCAL_STATE_OVERRIDE.get() {
        return Ok(settings.clone());
    }

    let base_dir = std::env::var_os(CONFIG_DIR_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let profile = std::env::var(PROFILE_ENV).ok();
    Ok(LocalStateOverride {
        base_dir,
        profile: normalize_profile(profile.as_deref())?,
    })
}

fn normalize_profile(profile: Option<&str>) -> PublicResult<String> {
    let profile = profile.unwrap_or(DEFAULT_PROFILE).trim();
    if profile.is_empty()
        || profile.len() > MAX_PROFILE_NAME_BYTES
        || profile == "."
        || profile == ".."
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(PublicError::validation(format!(
            "profile must contain 1 to {MAX_PROFILE_NAME_BYTES} ASCII letters, digits, '.', '_', or '-' and cannot be '.' or '..'"
        )));
    }
    Ok(profile.to_string())
}

pub fn load_credentials() -> PublicResult<Option<Credentials>> {
    load_credentials_unlocked(&credentials_path()?)
}

fn load_credentials_unlocked(path: &Path) -> PublicResult<Option<Credentials>> {
    validate_credentials_path(path)?;
    let Some(store) = CredentialStore::open(credentials_parent(path)?, false)? else {
        return Ok(None);
    };
    store.load()
}

pub fn load_credentials_for_url(api_url: &str) -> PublicResult<Option<Credentials>> {
    let normalized_api_url = normalize_api_url(api_url);
    match load_credentials()? {
        Some(credentials) if credentials.api_url == normalized_api_url => Ok(Some(credentials)),
        _ => Ok(None),
    }
}

pub fn save_credentials(credentials: &Credentials) -> PublicResult<()> {
    let dir = config_dir()?;
    with_credentials_lock_in(&dir, |store| store.save(credentials))
}

pub fn save_credentials_if_current(
    expected: &Credentials,
    updated: &Credentials,
) -> PublicResult<bool> {
    let dir = config_dir()?;
    save_credentials_if_current_in(&dir, expected, updated)
}

fn save_credentials_if_current_in(
    dir: &Path,
    expected: &Credentials,
    updated: &Credentials,
) -> PublicResult<bool> {
    with_credentials_lock_in(dir, |store| {
        if store.load()?.as_ref() != Some(expected) {
            return Ok(false);
        }

        store.save(updated)?;
        Ok(true)
    })
}

pub async fn refresh_credentials_if_needed(
    client: &reqwest::Client,
    base_url: &str,
    expected: &Credentials,
    access_expiry_window_seconds: i64,
) -> PublicResult<Credentials> {
    refresh_credentials_if_needed_with_timeout(
        client,
        base_url,
        expected,
        access_expiry_window_seconds,
        CREDENTIAL_REFRESH_TIMEOUT,
    )
    .await
}

pub async fn refresh_credentials_if_needed_with_timeout(
    client: &reqwest::Client,
    base_url: &str,
    expected: &Credentials,
    access_expiry_window_seconds: i64,
    refresh_timeout: StdDuration,
) -> PublicResult<Credentials> {
    if refresh_timeout.is_zero() {
        return Err(PublicError::validation(
            "credential refresh timeout must be greater than zero",
        ));
    }
    let dir = config_dir()?;
    refresh_credentials_if_needed_in(
        &dir,
        client,
        base_url,
        expected,
        access_expiry_window_seconds,
        refresh_timeout,
    )
    .await
}

async fn refresh_credentials_if_needed_in(
    dir: &Path,
    client: &reqwest::Client,
    base_url: &str,
    expected: &Credentials,
    access_expiry_window_seconds: i64,
    refresh_timeout: StdDuration,
) -> PublicResult<Credentials> {
    let credentials_lock = CredentialsFileLock::acquire_async(dir).await?;
    let result = async {
        let current = credentials_lock
            .store()
            .load()?
            .filter(|current| credentials_share_refresh_context(current, expected))
            .ok_or_else(credentials_changed_error)?;

        if !current.access_expires_within(access_expiry_window_seconds) {
            return Ok(current);
        }
        if current.is_refresh_expired() {
            return Err(PublicError::validation(
                "session expired, please login again",
            ));
        }

        let refresh_response = tokio::time::timeout(
            refresh_timeout,
            refresh_access_token(client, base_url, &current.refresh_token),
        )
        .await
        .map_err(|_| PublicError::transport(TransportFailureKind::Timeout))??;
        let mut refreshed = current;
        update_credentials_with_refresh(&mut refreshed, refresh_response);
        credentials_lock.store().save(&refreshed)?;
        Ok(refreshed)
    }
    .await;
    let unlock_result = credentials_lock.unlock();
    match result {
        Err(err) => Err(err),
        Ok(credentials) => {
            unlock_result?;
            Ok(credentials)
        }
    }
}

pub fn replace_credentials_atomically(
    credentials: &Credentials,
    before_replace: impl FnOnce(Option<&Credentials>) -> PublicResult<()>,
) -> PublicResult<Option<Credentials>> {
    let dir = config_dir()?;
    replace_credentials_atomically_in(&dir, credentials, before_replace)
}

fn replace_credentials_atomically_in(
    dir: &Path,
    credentials: &Credentials,
    before_replace: impl FnOnce(Option<&Credentials>) -> PublicResult<()>,
) -> PublicResult<Option<Credentials>> {
    with_credentials_lock_in(dir, |store| {
        let previous = store.load()?;
        before_replace(previous.as_ref())?;
        store.save(credentials)?;
        Ok(previous)
    })
}

pub fn with_current_credentials<T>(
    expected: &Credentials,
    action: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> PublicResult<T> {
    let dir = config_dir()?;
    with_current_credentials_in(&dir, expected, action)
}

fn with_current_credentials_in<T>(
    dir: &Path,
    expected: &Credentials,
    action: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> PublicResult<T> {
    with_credentials_lock_in(dir, |store| {
        let current = store.load()?;
        let current = current
            .as_ref()
            .filter(|current| *current == expected)
            .ok_or_else(credentials_changed_error)?;
        action(current)
    })
}

/// Run a local-state action while the authenticated account and encryption
/// identity still match `expected`.
///
/// Access and refresh tokens may rotate without changing this identity. The
/// credential-store lock remains held until `action` returns, preventing a
/// concurrent login from switching accounts while identity-bound state is
/// being written.
pub fn with_current_credential_identity<T>(
    expected: &Credentials,
    action: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> PublicResult<T> {
    let dir = config_dir()?;
    with_current_credential_identity_in(&dir, expected, action)
}

fn with_current_credential_identity_in<T>(
    dir: &Path,
    expected: &Credentials,
    action: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> PublicResult<T> {
    with_credentials_lock_in(dir, |store| {
        let current = store.load()?;
        let current = current
            .as_ref()
            .filter(|current| credentials_share_identity(current, expected))
            .ok_or_else(credentials_changed_error)?;
        action(current)
    })
}

pub fn clear_credentials_if_current(
    expected: &Credentials,
    before_clear: impl FnOnce(&Credentials) -> PublicResult<()>,
) -> PublicResult<()> {
    let dir = config_dir()?;
    with_credentials_lock_in(&dir, |store| {
        let current = store.load()?;
        let current = current
            .as_ref()
            .filter(|current| *current == expected)
            .ok_or_else(credentials_changed_error)?;
        let cleanup_result = before_clear(current);
        store.clear()?;
        cleanup_result
    })
}

pub fn clear_credentials() -> PublicResult<()> {
    let dir = config_dir()?;
    with_credentials_lock_in(&dir, CredentialStore::clear)
}

fn with_credentials_lock_in<T>(
    dir: &Path,
    action: impl FnOnce(&CredentialStore) -> PublicResult<T>,
) -> PublicResult<T> {
    let credentials_lock = CredentialsFileLock::acquire(dir)?;
    let result = action(credentials_lock.store());
    let unlock_result = credentials_lock.unlock();
    match result {
        Err(err) => Err(err),
        Ok(value) => {
            unlock_result?;
            Ok(value)
        }
    }
}

fn credentials_changed_error() -> PublicError {
    PublicError::conflict(CREDENTIALS_CHANGED_MESSAGE)
}

fn credentials_share_refresh_context(left: &Credentials, right: &Credentials) -> bool {
    left.api_url == right.api_url
        && left.user_id == right.user_id
        && left.email == right.email
        && left.data_key_ciphertext == right.data_key_ciphertext
}

fn credentials_share_identity(left: &Credentials, right: &Credentials) -> bool {
    normalize_api_url(&left.api_url) == normalize_api_url(&right.api_url)
        && left.user_id == right.user_id
        && left.data_key_ciphertext.trim() == right.data_key_ciphertext.trim()
}

pub fn load_persisted_data_key(credentials: &Credentials) -> PublicResult<Option<Vec<u8>>> {
    persisted_data_key_backend().load(credentials)
}

pub fn save_persisted_data_key(credentials: &Credentials, data_key: &[u8]) -> PublicResult<()> {
    persisted_data_key_backend().save(credentials, data_key)
}

pub fn clear_persisted_data_key(credentials: &Credentials) -> PublicResult<()> {
    persisted_data_key_backend().clear(credentials)
}

#[must_use]
pub fn persisted_data_key_status(credentials: &Credentials) -> PersistedDataKeyStatus {
    match load_persisted_data_key(credentials) {
        Ok(Some(_)) => PersistedDataKeyStatus::Available,
        Ok(None) => PersistedDataKeyStatus::Missing,
        Err(err) => PersistedDataKeyStatus::Unavailable(err.to_string()),
    }
}

#[cfg(unix)]
fn set_config_dir_permissions(dir: &Path) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to set config directory permissions on {}: {err}",
            dir.display()
        ))
    })
}

#[cfg(not(unix))]
fn set_config_dir_permissions(_dir: &Path) -> PublicResult<()> {
    Ok(())
}

pub fn opaque_login_start(
    password: &str,
) -> PublicResult<(ClientLogin<ClientCipherSuite>, String)> {
    let mut rng = OsRng;
    let ClientLoginStartResult { message, state } =
        ClientLogin::<ClientCipherSuite>::start(&mut rng, password.as_bytes())
            .map_err(|err| PublicError::crypto(format!("OPAQUE login start failed: {err}")))?;
    Ok((state, encode_bytes(message.serialize().as_slice())))
}

pub fn opaque_login_finish(
    state: ClientLogin<ClientCipherSuite>,
    email: &str,
    password: &str,
    server_response_b64: &str,
) -> PublicResult<String> {
    let OpaqueLoginFinish {
        client_finish_message,
        export_key: _,
    } = opaque_login_finish_with_export_key(state, email, password, server_response_b64)?;
    Ok(client_finish_message)
}

pub fn opaque_login_finish_with_export_key(
    state: ClientLogin<ClientCipherSuite>,
    email: &str,
    password: &str,
    server_response_b64: &str,
) -> PublicResult<OpaqueLoginFinish> {
    let mut rng = OsRng;
    let server_bytes = decode_bytes(server_response_b64)?;
    let credential_response = CredentialResponse::<ClientCipherSuite>::deserialize(&server_bytes)
        .map_err(|err| {
        PublicError::crypto(format!("failed to deserialize server response: {err}"))
    })?;

    let normalized_email = email.trim().to_lowercase();
    let identifiers = Identifiers {
        client: Some(normalized_email.as_bytes()),
        server: Some(OPAQUE_SERVER_ID),
    };
    let params = ClientLoginFinishParameters::new(None, identifiers, None);

    let mut finish_result = state
        .finish(&mut rng, password.as_bytes(), credential_response, params)
        .map_err(|err| PublicError::crypto(format!("OPAQUE login finish failed: {err}")))?;

    let client_finish_message = encode_bytes(finish_result.message.serialize().as_slice());
    let mut export_key = [0u8; OPAQUE_EXPORT_KEY_BYTES];
    export_key.copy_from_slice(finish_result.export_key.as_slice());
    finish_result.export_key.as_mut_slice().zeroize();

    Ok(OpaqueLoginFinish {
        client_finish_message,
        export_key: OpaqueExportKey(export_key),
    })
}

pub async fn login(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> PublicResult<AuthResponse> {
    match begin_login(client, base_url, email, password).await? {
        LoginOutcome::Authenticated(response) => Ok(response),
        LoginOutcome::MfaRequired { pending, .. } => {
            drop(pending);
            Err(PublicError::mfa_required_use_begin_login())
        }
    }
}

pub async fn begin_login(
    client: &reqwest::Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> PublicResult<LoginOutcome> {
    let normalized_base = normalize_api_url(base_url);
    let (opaque_state, client_login_state) = opaque_login_start(password)?;

    let start_response = client
        .post(format!("{normalized_base}/auth/opaque/login/start"))
        .header(MFA_CAPABILITIES_HEADER, MFA_CAPABILITIES_VALUE)
        .json(&LoginStartRequest {
            email: email.to_string(),
            client_login_state,
        })
        .send()
        .await
        .map_err(|err| map_reqwest_error(err, "login start"))?;
    let start_result: LoginStartResponse =
        parse_json_response(start_response, "login start response").await?;

    let client_finish_message = opaque_login_finish(
        opaque_state,
        email,
        password,
        &start_result.server_login_state,
    )?;

    let finish_response = client
        .post(format!("{normalized_base}/auth/opaque/login/finish"))
        .header(MFA_CAPABILITIES_HEADER, MFA_CAPABILITIES_VALUE)
        .json(&LoginFinishRequest {
            session_token: start_result.session_token,
            client_finish_message,
        })
        .send()
        .await
        .map_err(|err| map_reqwest_error(err, "login finish"))?;

    parse_login_finish_response(finish_response, &normalized_base).await
}

pub async fn complete_mfa_login(
    client: &reqwest::Client,
    pending: PendingMfaLogin,
    code: SecretMfaCode,
) -> Result<AuthResponse, CompleteMfaLoginError> {
    complete_mfa_login_with_clock(client, pending, code, Instant::now).await
}

async fn complete_mfa_login_with_clock<Now>(
    client: &reqwest::Client,
    mut pending: PendingMfaLogin,
    code: SecretMfaCode,
    now: Now,
) -> Result<AuthResponse, CompleteMfaLoginError>
where
    Now: Fn() -> Instant,
{
    let request_started_at = now();
    if pending.remaining_duration_at(request_started_at).is_zero() {
        return Err(CompleteMfaLoginError::Terminal(
            MFA_CHALLENGE_EXPIRED_MESSAGE.to_string(),
        ));
    }
    pending.refresh_remaining_at(request_started_at);

    let base_url = pending.base_url().to_string();
    let request_deadline = tokio::time::Instant::from_std(pending.deadline());
    let request = async {
        let response = client
            .post(format!("{base_url}/auth/mfa/login/verify"))
            .header(MFA_CAPABILITIES_HEADER, MFA_CAPABILITIES_VALUE)
            .json(&MfaLoginVerifyRequest {
                challenge_token: pending.challenge_token.expose(),
                code: code.expose(),
            })
            .send()
            .await?;
        let status = response.status();
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let body = response.text().await;
        Ok::<_, reqwest::Error>((status, retry_after_seconds, body))
    };
    let (status, retry_after_seconds, body) =
        match tokio::time::timeout_at(request_deadline, request).await {
            Err(_) => {
                return Err(CompleteMfaLoginError::Terminal(
                    MFA_CHALLENGE_EXPIRED_MESSAGE.to_string(),
                ));
            }
            Ok(Err(err)) => {
                let expires_in = pending.refresh_remaining_at(now());
                if expires_in == 0 {
                    return Err(CompleteMfaLoginError::Terminal(
                        MFA_CHALLENGE_EXPIRED_MESSAGE.to_string(),
                    ));
                }
                return Err(CompleteMfaLoginError::Retryable {
                    message: map_reqwest_error(err, "MFA login verify").to_string(),
                    pending,
                    attempts_remaining: None,
                    expires_in: Some(expires_in),
                    retry_after_seconds: None,
                });
            }
            Ok(Ok(response)) => response,
        };

    if status.is_success() {
        return match body {
            Ok(body) => serde_json::from_str::<AuthResponse>(&body).map_err(|err| {
                CompleteMfaLoginError::Terminal(format!("failed to parse auth response: {err}"))
            }),
            Err(err) => Err(CompleteMfaLoginError::Terminal(format!(
                "failed to parse auth response: {err}"
            ))),
        };
    }

    let error_text = body.unwrap_or_else(|_| "unknown error".to_string());

    if status.as_u16() == 409 && response_has_error_code(&error_text, "mfa_client_upgrade_required")
    {
        return Err(CompleteMfaLoginError::Terminal(
            "client upgrade required for MFA".to_string(),
        ));
    }

    if let Ok(mfa_error) = serde_json::from_str::<MfaVerificationErrorResponse>(&error_text) {
        return Err(map_mfa_verification_error(
            status.as_u16(),
            mfa_error,
            pending,
            &code,
            retry_after_seconds,
            now(),
        ));
    }

    if status.as_u16() == 429 || status.as_u16() == 503 {
        let expires_in = pending.refresh_remaining_at(now());
        if expires_in == 0 {
            return Err(CompleteMfaLoginError::Terminal(
                MFA_CHALLENGE_EXPIRED_MESSAGE.to_string(),
            ));
        }
        return Err(CompleteMfaLoginError::Retryable {
            message: redact_mfa_secrets(&error_text, &pending, &code),
            pending,
            attempts_remaining: None,
            expires_in: Some(expires_in),
            retry_after_seconds,
        });
    }

    Err(CompleteMfaLoginError::Terminal(redact_mfa_secrets(
        &error_text,
        &pending,
        &code,
    )))
}

async fn parse_login_finish_response(
    response: reqwest::Response,
    base_url: &str,
) -> PublicResult<LoginOutcome> {
    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        if status.as_u16() == 409
            && response_has_error_code(&error_text, "mfa_client_upgrade_required")
        {
            return Err(PublicError::validation(
                "this client must be upgraded before signing in to an MFA-enabled account",
            ));
        }
        return Err(map_api_error(status.as_u16(), &error_text));
    }

    let body = response.text().await.map_err(|err| {
        PublicError::unexpected(format!("failed to read login finish body: {err}"))
    })?;

    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|err| PublicError::unexpected(format!("invalid login finish JSON: {err}")))?;
    if value.get("status").and_then(serde_json::Value::as_str) == Some("second_factor_required") {
        let challenge: SecondFactorRequiredResponse =
            serde_json::from_value(value).map_err(|err| {
                PublicError::unexpected(format!("failed to parse MFA challenge response: {err}"))
            })?;
        let _validated_status = challenge.status;
        if challenge.expires_in == 0
            || challenge.attempts_remaining == 0
            || challenge.methods.first() != Some(&MfaMethod::Totp)
        {
            return Err(PublicError::unexpected("invalid MFA challenge metadata"));
        }
        let public_challenge = MfaChallenge {
            methods: challenge.methods.clone(),
            expires_in: challenge.expires_in,
            attempts_remaining: challenge.attempts_remaining,
            requires_legacy_password: challenge.requires_legacy_password,
        };
        return Ok(LoginOutcome::MfaRequired {
            challenge: public_challenge.clone(),
            pending: PendingMfaLogin::new(
                base_url.to_string(),
                challenge.challenge_token,
                public_challenge,
            ),
        });
    }

    let auth_response: AuthResponse = serde_json::from_value(value)
        .map_err(|err| PublicError::unexpected(format!("failed to parse auth response: {err}")))?;
    Ok(LoginOutcome::Authenticated(auth_response))
}

fn response_has_error_code(body: &str, expected: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(|code| code == expected)
        })
        .unwrap_or(false)
}

fn map_mfa_verification_error(
    status: u16,
    mut error: MfaVerificationErrorResponse,
    mut pending: PendingMfaLogin,
    code: &SecretMfaCode,
    retry_after_seconds: Option<u64>,
    now: Instant,
) -> CompleteMfaLoginError {
    error.message = redact_mfa_secrets(&error.message, &pending, code);
    let expires_in =
        pending.apply_retry_metadata_at(error.attempts_remaining, error.expires_in, now);
    match error.error.as_str() {
        "mfa_challenge_invalid_or_expired" | "mfa_enrollment_invalid_or_expired" => {
            CompleteMfaLoginError::Terminal(error.message)
        }
        "invalid_mfa_code" if expires_in > 0 => CompleteMfaLoginError::Retryable {
            message: error.message,
            pending,
            attempts_remaining: error.attempts_remaining,
            expires_in: Some(expires_in),
            retry_after_seconds,
        },
        "mfa_totp_locked" if expires_in > 0 => CompleteMfaLoginError::TotpLocked {
            message: error.message,
            pending,
        },
        _ if (status == 429 || status == 503) && expires_in > 0 => {
            CompleteMfaLoginError::Retryable {
                message: error.message,
                pending,
                attempts_remaining: error.attempts_remaining,
                expires_in: Some(expires_in),
                retry_after_seconds,
            }
        }
        "invalid_mfa_code" | "mfa_totp_locked" if expires_in == 0 => {
            CompleteMfaLoginError::Terminal(MFA_CHALLENGE_EXPIRED_MESSAGE.to_string())
        }
        _ if (status == 429 || status == 503) && expires_in == 0 => {
            CompleteMfaLoginError::Terminal(MFA_CHALLENGE_EXPIRED_MESSAGE.to_string())
        }
        _ => CompleteMfaLoginError::Terminal(error.message),
    }
}

fn redact_mfa_secrets(message: &str, pending: &PendingMfaLogin, code: &SecretMfaCode) -> String {
    [pending.challenge_token.expose(), code.expose()]
        .into_iter()
        .filter(|secret| !secret.is_empty())
        .fold(message.to_string(), |redacted, secret| {
            redacted.replace(secret, "<redacted>")
        })
}

pub async fn refresh_access_token(
    client: &reqwest::Client,
    base_url: &str,
    refresh_token: &str,
) -> PublicResult<RefreshResponse> {
    let response = client
        .post(format!("{}/auth/refresh", base_url.trim_end_matches('/')))
        .json(&RefreshRequest {
            refresh_token: refresh_token.to_string(),
        })
        .send()
        .await
        .map_err(map_refresh_transport_error)?;

    parse_refresh_response(response).await
}

pub async fn logout(
    client: &reqwest::Client,
    base_url: &str,
    refresh_token: &str,
) -> PublicResult<Option<String>> {
    let status = client
        .post(format!("{}/auth/logout", base_url.trim_end_matches('/')))
        .json(&RefreshRequest {
            refresh_token: refresh_token.to_string(),
        })
        .send()
        .await
        .map_err(|err| map_reqwest_error(err, "logout"))?
        .status();

    Ok((!status.is_success()).then(|| format!("server logout returned status {status}")))
}

pub fn auth_response_to_credentials(api_url: &str, response: AuthResponse) -> Credentials {
    let now = Utc::now();
    Credentials {
        api_url: normalize_api_url(api_url),
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        access_expires_at: expires_at_from(now, response.expires_in),
        refresh_expires_at: expires_at_from(now, response.refresh_expires_in),
        user_id: response.user.id,
        email: response.user.email,
        data_key_ciphertext: response.data_key_ciphertext,
    }
}

pub fn update_credentials_with_refresh(
    credentials: &mut Credentials,
    refresh_response: RefreshResponse,
) {
    let now = Utc::now();
    credentials.access_token = refresh_response.access_token;
    credentials.refresh_token = refresh_response.refresh_token;
    credentials.access_expires_at = expires_at_from(now, refresh_response.expires_in);
    credentials.refresh_expires_at = expires_at_from(now, refresh_response.refresh_expires_in);
}

fn expires_at_from(now: DateTime<Utc>, expires_in_seconds: u64) -> DateTime<Utc> {
    now + chrono::Duration::seconds(expires_in_seconds as i64)
}

enum PersistedDataKeyBackend {
    PlatformKeyring,
    TestDirectory(PathBuf),
}

impl PersistedDataKeyBackend {
    fn load(&self, credentials: &Credentials) -> PublicResult<Option<Vec<u8>>> {
        match self {
            Self::PlatformKeyring => {
                let entry = platform_keyring_entry(credentials)?;
                match entry.get_secret() {
                    Ok(secret) => Ok(Some(secret)),
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(err) => Err(map_keyring_error("read from the platform keychain", err)),
                }
            }
            Self::TestDirectory(dir) => load_test_persisted_data_key(dir, credentials),
        }
    }

    fn save(&self, credentials: &Credentials, data_key: &[u8]) -> PublicResult<()> {
        match self {
            Self::PlatformKeyring => {
                let entry = platform_keyring_entry(credentials)?;
                entry
                    .set_secret(data_key)
                    .map_err(|err| map_keyring_error("write to the platform keychain", err))
            }
            Self::TestDirectory(dir) => save_test_persisted_data_key(dir, credentials, data_key),
        }
    }

    fn clear(&self, credentials: &Credentials) -> PublicResult<()> {
        match self {
            Self::PlatformKeyring => {
                let entry = platform_keyring_entry(credentials)?;
                match entry.delete_credential() {
                    Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                    Err(err) => Err(map_keyring_error("clear the platform keychain entry", err)),
                }
            }
            Self::TestDirectory(dir) => clear_test_persisted_data_key(dir, credentials),
        }
    }
}

fn persisted_data_key_backend() -> PersistedDataKeyBackend {
    match std::env::var(TEST_KEYCHAIN_DIR_ENV) {
        Ok(dir) if !dir.trim().is_empty() => PersistedDataKeyBackend::TestDirectory(dir.into()),
        _ => PersistedDataKeyBackend::PlatformKeyring,
    }
}

fn platform_keyring_entry(credentials: &Credentials) -> PublicResult<keyring::Entry> {
    let entry_name = persisted_data_key_entry_name(credentials)?;
    keyring::Entry::new(DATA_KEY_KEYCHAIN_SERVICE, &entry_name)
        .map_err(|err| map_keyring_error("create the platform keychain entry", err))
}

fn persisted_data_key_entry_name(credentials: &Credentials) -> PublicResult<String> {
    let fingerprint = data_key_fingerprint(&credentials.data_key_ciphertext)?;
    let entry = format!(
        "{}::{}::{}",
        normalize_api_url(&credentials.api_url),
        credentials.user_id,
        fingerprint
    );
    let profile = active_profile()?;
    if profile == DEFAULT_PROFILE {
        Ok(entry)
    } else {
        Ok(format!("profile:{profile}::{entry}"))
    }
}

fn data_key_fingerprint(data_key_ciphertext: &str) -> PublicResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(decode_bytes(data_key_ciphertext)?);
    Ok(URL_SAFE_NO_PAD.encode(hasher.finalize()))
}

fn map_keyring_error(action: &str, err: keyring::Error) -> PublicError {
    PublicError::validation(format!("failed to {action}: {err}"))
}

fn load_test_persisted_data_key(
    dir: &Path,
    credentials: &Credentials,
) -> PublicResult<Option<Vec<u8>>> {
    let path = test_persisted_data_key_path(dir, credentials)?;
    if !path.exists() {
        return Ok(None);
    }

    fs::read(&path).map(Some).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to read the persisted test keychain secret {}: {err}",
            path.display()
        ))
    })
}

fn save_test_persisted_data_key(
    dir: &Path,
    credentials: &Credentials,
    data_key: &[u8],
) -> PublicResult<()> {
    fs::create_dir_all(dir).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to create the persisted test keychain directory {}: {err}",
            dir.display()
        ))
    })?;
    set_config_dir_permissions(dir)?;

    let path = test_persisted_data_key_path(dir, credentials)?;
    fs::write(&path, data_key).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to write the persisted test keychain secret {}: {err}",
            path.display()
        ))
    })?;
    set_secret_file_permissions(&path)?;
    Ok(())
}

fn clear_test_persisted_data_key(dir: &Path, credentials: &Credentials) -> PublicResult<()> {
    let path = test_persisted_data_key_path(dir, credentials)?;
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to remove the persisted test keychain secret {}: {err}",
            path.display()
        ))
    })
}

fn test_persisted_data_key_path(dir: &Path, credentials: &Credentials) -> PublicResult<PathBuf> {
    let entry_name = persisted_data_key_entry_name(credentials)?;
    let file_name = format!(
        "persisted-data-key-{}.bin",
        URL_SAFE_NO_PAD.encode(Sha256::digest(entry_name.as_bytes()))
    );
    Ok(dir.join(file_name))
}

fn encode_bytes(bytes: &[u8]) -> String {
    STANDARD_NO_PAD.encode(bytes)
}

fn decode_bytes(value: &str) -> PublicResult<Vec<u8>> {
    let trimmed = value.trim();
    STANDARD_NO_PAD
        .decode(trimmed)
        .or_else(|_| STANDARD.decode(trimmed))
        .or_else(|_| URL_SAFE_NO_PAD.decode(trimmed))
        .or_else(|_| URL_SAFE.decode(trimmed))
        .map_err(|err| PublicError::validation(format!("invalid base64: {err}")))
}

fn map_reqwest_error(err: reqwest::Error, context: &str) -> PublicError {
    if err.is_connect() {
        PublicError::unexpected(format!("failed to connect to API during {context}: {err}"))
    } else if err.is_timeout() {
        PublicError::unexpected(format!("API request timed out during {context}"))
    } else {
        PublicError::unexpected(format!("API request failed during {context}: {err}"))
    }
}

fn map_refresh_transport_error(err: reqwest::Error) -> PublicError {
    if err.is_connect() {
        PublicError::transport(TransportFailureKind::Connect)
    } else if err.is_timeout() {
        PublicError::transport(TransportFailureKind::Timeout)
    } else if err.is_body() {
        PublicError::transport(TransportFailureKind::Body)
    } else {
        PublicError::transport(TransportFailureKind::Other)
    }
}

async fn parse_refresh_response(response: reqwest::Response) -> PublicResult<RefreshResponse> {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    let body = read_bounded_refresh_body(response).await;
    if !status.is_success() {
        let backend_error_code = body
            .ok()
            .and_then(|body| serde_json::from_slice::<ApiError>(&body).ok())
            .map(|api_error| api_error.error);
        return Err(PublicError::http(
            status.as_u16(),
            backend_error_code,
            retry_after,
        ));
    }

    serde_json::from_slice(&body?).map_err(map_refresh_json_error)
}

async fn read_bounded_refresh_body(mut response: reqwest::Response) -> PublicResult<Vec<u8>> {
    if response.content_length().is_some_and(|length| {
        usize::try_from(length).map_or(true, |length| length > MAX_REFRESH_RESPONSE_BYTES)
    }) {
        return Err(refresh_body_too_large_error());
    }

    let mut body = Vec::new();
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(map_refresh_transport_error)?;
        let Some(chunk) = chunk else {
            return Ok(body);
        };
        let Some(next_len) = body.len().checked_add(chunk.len()) else {
            return Err(refresh_body_too_large_error());
        };
        if next_len > MAX_REFRESH_RESPONSE_BYTES {
            return Err(refresh_body_too_large_error());
        }
        body.extend_from_slice(&chunk);
    }
}

fn refresh_body_too_large_error() -> PublicError {
    PublicError::response(
        ResponseFailureKind::BodyTooLarge,
        "refresh response exceeds the client safety limit",
    )
}

fn map_refresh_json_error(err: serde_json::Error) -> PublicError {
    let (kind, message) = match err.classify() {
        serde_json::error::Category::Data => (
            ResponseFailureKind::JsonSchema,
            "refresh response JSON does not match the expected schema",
        ),
        serde_json::error::Category::Eof | serde_json::error::Category::Syntax => (
            ResponseFailureKind::JsonMalformed,
            "refresh response contains malformed JSON",
        ),
        serde_json::error::Category::Io => (
            ResponseFailureKind::BodyRead,
            "refresh response body could not be read",
        ),
    };
    PublicError::response(kind, message)
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<StdDuration> {
    let value = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let seconds = value.parse::<u64>().unwrap_or(MAX_RETRY_AFTER_SECONDS);
    Some(StdDuration::from_secs(seconds.min(MAX_RETRY_AFTER_SECONDS)))
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    context: &str,
) -> PublicResult<T> {
    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(map_api_error(status.as_u16(), &error_text));
    }

    response
        .json()
        .await
        .map_err(|err| PublicError::unexpected(format!("failed to parse {context}: {err}")))
}

fn map_api_error(status: u16, body: &str) -> PublicError {
    if let Ok(api_error) = serde_json::from_str::<ApiError>(body) {
        let message = api_error.message.unwrap_or(api_error.error);
        return match status {
            401 => PublicError::validation(format!("authentication failed: {message}")),
            403 => PublicError::validation(format!("access denied: {message}")),
            404 => PublicError::validation(format!("not found: {message}")),
            400 | 422 => PublicError::validation(message),
            _ => PublicError::unexpected(format!("API error ({status}): {message}")),
        };
    }

    match status {
        401 => PublicError::validation("authentication failed"),
        403 => PublicError::validation("access denied"),
        404 => PublicError::validation("resource not found"),
        _ => PublicError::unexpected(format!("API error ({status}): {body}")),
    }
}

#[cfg(unix)]
fn set_secret_file_permissions(path: &Path) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to set secret file permissions on {}: {err}",
            path.display()
        ))
    })
}

#[cfg(not(unix))]
fn set_secret_file_permissions(_path: &Path) -> PublicResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::process::{Child, Command, Output, Stdio};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, mpsc};

    use super::*;
    use axum::Json;
    use axum::Router;
    use axum::http::{HeaderValue, StatusCode, header};
    use axum::response::IntoResponse;
    use axum::routing::post;
    use chrono::Duration;
    use opaque_ke::{
        ClientRegistration, ClientRegistrationFinishParameters, CredentialRequest,
        RegistrationResponse, ServerLogin, ServerLoginParameters, ServerRegistration, ServerSetup,
    };
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::net::TcpListener;
    use tokio::sync::Notify;

    const REFRESH_RACE_BASE_URL_ENV: &str = "SEALTASK_REFRESH_RACE_BASE_URL";
    const REFRESH_RACE_CREDENTIALS_DIR_ENV: &str = "SEALTASK_REFRESH_RACE_CREDENTIALS_DIR";
    const REFRESH_RACE_READY_PATH_ENV: &str = "SEALTASK_REFRESH_RACE_READY_PATH";

    #[test]
    fn profile_names_are_path_safe_and_stable() {
        for valid in ["default", "agent-1", "work.account", "CI_runner"] {
            assert_eq!(
                normalize_profile(Some(valid)).expect("valid profile"),
                valid
            );
        }
        for invalid in ["", ".", "..", "with/slash", "with space", "équipe"] {
            assert!(
                normalize_profile(Some(invalid)).is_err(),
                "profile unexpectedly accepted: {invalid:?}"
            );
        }
    }

    async fn spawn_mfa_server(
        status: StatusCode,
        body: serde_json::Value,
        retry_after: Option<&'static str>,
    ) -> String {
        let app = Router::new().route(
            "/auth/mfa/login/verify",
            post(move || {
                let body = body.clone();
                async move {
                    let mut response = (status, axum::Json(body)).into_response();
                    if let Some(value) = retry_after {
                        response
                            .headers_mut()
                            .insert(header::RETRY_AFTER, HeaderValue::from_static(value));
                    }
                    response
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test API");
        });
        format!("http://{address}")
    }

    #[test]
    fn opaque_login_finish_returns_the_registration_export_key_and_redacts_debug() {
        const EMAIL: &str = "opaque-export@example.test";
        const PASSWORD: &str = "correct horse battery staple";

        let mut rng = OsRng;
        let setup = ServerSetup::<ClientCipherSuite>::new(&mut rng);
        let registration =
            ClientRegistration::<ClientCipherSuite>::start(&mut rng, PASSWORD.as_bytes())
                .expect("start registration");
        let registration_response = ServerRegistration::<ClientCipherSuite>::start(
            &setup,
            registration.message,
            EMAIL.as_bytes(),
        )
        .expect("start server registration");
        let identifiers = Identifiers {
            client: Some(EMAIL.as_bytes()),
            server: Some(OPAQUE_SERVER_ID),
        };
        let mut registration_finish = registration
            .state
            .finish(
                &mut rng,
                PASSWORD.as_bytes(),
                RegistrationResponse::<ClientCipherSuite>::deserialize(
                    &registration_response.message.serialize(),
                )
                .expect("deserialize registration response"),
                ClientRegistrationFinishParameters::new(identifiers, None),
            )
            .expect("finish registration");
        let expected_export_key = registration_finish.export_key.to_vec();
        registration_finish.export_key.as_mut_slice().zeroize();
        let password_file =
            ServerRegistration::<ClientCipherSuite>::finish(registration_finish.message);

        let (client_state, client_request) = opaque_login_start(PASSWORD).expect("start login");
        let credential_request = CredentialRequest::<ClientCipherSuite>::deserialize(
            &decode_bytes(&client_request).expect("decode client request"),
        )
        .expect("deserialize credential request");
        let login = ServerLogin::<ClientCipherSuite>::start(
            &mut rng,
            &setup,
            Some(password_file),
            credential_request,
            EMAIL.as_bytes(),
            ServerLoginParameters {
                context: None,
                identifiers,
            },
        )
        .expect("start server login");

        let finish = opaque_login_finish_with_export_key(
            client_state,
            EMAIL,
            PASSWORD,
            &encode_bytes(login.message.serialize().as_slice()),
        )
        .expect("finish login");

        assert!(!finish.client_finish_message.is_empty());
        assert_eq!(finish.export_key.as_bytes(), expected_export_key.as_slice());
        assert_eq!(format!("{:?}", finish.export_key), "<redacted>");
    }

    #[test]
    fn opaque_login_finish_matches_browser_registered_account_export_key() {
        // Generated with @serenity-kit/opaque 1.1.0 using the production
        // identifiers. This guards the browser-registration/CLI-unlock boundary.
        const EMAIL: &str = "oss-v2-browser-fixture@example.test";
        const PASSWORD: &str = "correct horse battery staple";
        const BROWSER_SERVER_SETUP: &str = concat!(
            "1oW8taI-3dYld7y5SN7_wH01dUZJx-04mcqsHz8bKs-X6gv3eSf-iDQZStvtITsz",
            "3vfDcBlAQDYj9eSUSuQQrN5LIjgbER1vXHKqfabynBdsUUxrga7xowyedxDafMsF",
            "KtcXWFJfQlHGVFf90oH0GEk8R8lMn2GvEa_NShqqhAw"
        );
        const BROWSER_PASSWORD_FILE: &str = concat!(
            "BAmBVNTcXHA9d0dVdffsepyIvyjl-d2e1U3DS4R173-yWM10EUdoTERIXYMujwzT",
            "HcNw-kRMrfpODV1GAXNR5oWmNe7IZ_qgMKgkA0mdLEAJSigpz-478fLdzi_H7tt0",
            "xpVN9a5KULYyfqEEZygHjLDZ0porWN1mtTchBqRN08YfQZKuqCVaq5GC2En7CPM",
            "Eq3Rhewr5Ogv8WTiixEXl3TCUCAD5RSwgZRjwiWiEH6NbGw5YBjbyc2bEzoP9xv",
            "rl"
        );
        const BROWSER_EXPORT_KEY: &str = concat!(
            "18Ifrzr4ncv62u1k2dSsRL1ZsD64msuxmSs_B91bwponDN7Vo7H3z9inGeUi0eqJ",
            "FMQQyyiZvPJ4FhZKZg1AEg"
        );

        let setup = ServerSetup::<ClientCipherSuite>::deserialize(
            &decode_bytes(BROWSER_SERVER_SETUP).expect("decode browser server setup"),
        )
        .expect("deserialize browser server setup");
        let password_file = ServerRegistration::<ClientCipherSuite>::deserialize(
            &decode_bytes(BROWSER_PASSWORD_FILE).expect("decode browser password file"),
        )
        .expect("deserialize browser password file");
        let (client_state, client_request) = opaque_login_start(PASSWORD).expect("start login");
        let credential_request = CredentialRequest::<ClientCipherSuite>::deserialize(
            &decode_bytes(&client_request).expect("decode client request"),
        )
        .expect("deserialize credential request");
        let login = ServerLogin::<ClientCipherSuite>::start(
            &mut OsRng,
            &setup,
            Some(password_file),
            credential_request,
            EMAIL.as_bytes(),
            ServerLoginParameters {
                context: None,
                identifiers: Identifiers {
                    client: Some(EMAIL.as_bytes()),
                    server: Some(OPAQUE_SERVER_ID),
                },
            },
        )
        .expect("start server login");

        let finish = opaque_login_finish_with_export_key(
            client_state,
            EMAIL,
            PASSWORD,
            &encode_bytes(login.message.serialize().as_slice()),
        )
        .expect("finish browser-account login");

        assert_eq!(
            finish.export_key.as_bytes().as_slice(),
            decode_bytes(BROWSER_EXPORT_KEY)
                .expect("decode browser export key")
                .as_slice()
        );
    }

    async fn spawn_hanging_mfa_server() -> String {
        let app = Router::new().route(
            "/auth/mfa/login/verify",
            post(|| async { std::future::pending::<StatusCode>().await }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test API");
        });
        format!("http://{address}")
    }

    async fn spawn_shortening_then_rate_limited_server(request_count: Arc<AtomicUsize>) -> String {
        let app = Router::new().route(
            "/auth/mfa/login/verify",
            post(move || {
                let request_count = Arc::clone(&request_count);
                async move {
                    let request_index = request_count.fetch_add(1, Ordering::SeqCst);
                    if request_index == 0 {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({
                                "error": "invalid_mfa_code",
                                "message": "invalid authenticator code",
                                "attemptsRemaining": 7,
                                "expiresIn": 5
                            })),
                        )
                            .into_response();
                    }

                    let mut response = (
                        StatusCode::TOO_MANY_REQUESTS,
                        Json(json!({
                            "error": "rate_limited",
                            "message": "try later",
                            "attemptsRemaining": 7,
                            "expiresIn": 300
                        })),
                    )
                        .into_response();
                    response
                        .headers_mut()
                        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
                    response
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test API");
        });
        format!("http://{address}")
    }

    async fn spawn_code_sensitive_mfa_server(
        expected_token: &'static str,
        accepted_code: &'static str,
    ) -> String {
        let app = Router::new().route(
            "/auth/mfa/login/verify",
            post(move |Json(request): Json<serde_json::Value>| async move {
                assert_eq!(request["challengeToken"], expected_token);
                if request["code"] == accepted_code {
                    (
                        StatusCode::OK,
                        Json(successful_auth_body("completed-access-token")),
                    )
                        .into_response()
                } else {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({
                            "error": "invalid_mfa_code",
                            "message": "invalid authenticator or MFA backup code",
                            "attemptsRemaining": 7,
                            "expiresIn": 240
                        })),
                    )
                        .into_response()
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test API");
        });
        format!("http://{address}")
    }

    async fn spawn_exact_mfa_request_server(expected_token: &str, expected_code: &str) -> String {
        let expected_token = expected_token.to_string();
        let expected_code = expected_code.to_string();
        let app = Router::new().route(
            "/auth/mfa/login/verify",
            post(move |Json(request): Json<serde_json::Value>| {
                let expected_token = expected_token.clone();
                let expected_code = expected_code.clone();
                async move {
                    assert_eq!(
                        request["challengeToken"].as_str(),
                        Some(expected_token.as_str())
                    );
                    assert_eq!(request["code"].as_str(), Some(expected_code.as_str()));
                    (
                        StatusCode::OK,
                        Json(successful_auth_body("raw-code-access-token")),
                    )
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test API");
        });
        format!("http://{address}")
    }

    async fn spawn_login_finish_server(status: StatusCode, body: serde_json::Value) -> String {
        let app = Router::new().route(
            "/auth/opaque/login/finish",
            post(move || {
                let body = body.clone();
                async move { (status, Json(body)) }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test API");
        });
        format!("http://{address}")
    }

    fn successful_auth_body(access_token: &str) -> serde_json::Value {
        json!({
            "accessToken": access_token,
            "refreshToken": "completed-refresh-token",
            "expiresIn": 900,
            "refreshExpiresIn": 2592000,
            "tokenType": "Bearer",
            "user": {
                "id": "01900000-0000-7000-8000-000000000001",
                "email": "mfa@example.test",
                "name": "MFA Test",
                "timezone": "UTC",
                "avatarColor": "blue",
                "themePreference": "system",
                "emailVerified": true
            },
            "dataKeyCiphertext": "encrypted-data-key"
        })
    }

    fn pending_for(base_url: String, token: &str) -> PendingMfaLogin {
        pending_for_at(base_url, token, Instant::now())
    }

    fn pending_for_at(base_url: String, token: &str, now: Instant) -> PendingMfaLogin {
        PendingMfaLogin::new_at(
            base_url,
            token.to_string(),
            MfaChallenge {
                methods: vec![MfaMethod::Totp, MfaMethod::BackupCode],
                expires_in: 300,
                attempts_remaining: 8,
                requires_legacy_password: false,
            },
            now,
        )
    }

    struct TestClock {
        origin: Instant,
        elapsed_seconds: AtomicU64,
    }

    impl TestClock {
        fn new() -> Self {
            Self {
                origin: Instant::now(),
                elapsed_seconds: AtomicU64::new(0),
            }
        }

        fn now(&self) -> Instant {
            self.origin + StdDuration::from_secs(self.elapsed_seconds.load(Ordering::SeqCst))
        }

        fn advance(&self, seconds: u64) {
            self.elapsed_seconds.fetch_add(seconds, Ordering::SeqCst);
        }
    }

    fn shorten_pending_at(pending: PendingMfaLogin, now: Instant) -> PendingMfaLogin {
        match map_mfa_verification_error(
            StatusCode::UNAUTHORIZED.as_u16(),
            MfaVerificationErrorResponse {
                error: "invalid_mfa_code".to_string(),
                message: "invalid authenticator code".to_string(),
                attempts_remaining: Some(7),
                expires_in: Some(5),
            },
            pending,
            &SecretMfaCode::new("000000"),
            None,
            now,
        ) {
            CompleteMfaLoginError::Retryable {
                pending,
                expires_in: Some(5),
                ..
            } => pending,
            _ => panic!("invalid code should shorten the live continuation to five seconds"),
        }
    }

    #[test]
    fn login_wrapper_returns_mfa_required_use_begin_login() {
        let err = PublicError::mfa_required_use_begin_login();
        assert!(matches!(err, PublicError::MfaRequiredUseBeginLogin));
    }

    #[test]
    fn response_error_code_matching_is_exact_and_top_level() {
        assert!(response_has_error_code(
            r#"{"error":"mfa_client_upgrade_required","message":"upgrade"}"#,
            "mfa_client_upgrade_required"
        ));
        assert!(!response_has_error_code(
            r#"{"error":"different","message":"mfa_client_upgrade_required"}"#,
            "mfa_client_upgrade_required"
        ));
    }

    #[test]
    fn second_factor_required_response_decodes_public_challenge_fields() {
        let body = r#"{"status":"second_factor_required","challengeToken":"challenge-token","methods":["totp","backup_code"],"expiresIn":300,"attemptsRemaining":8,"requiresLegacyPassword":false}"#;
        let challenge: SecondFactorRequiredResponse =
            serde_json::from_str(body).expect("decode challenge");
        assert_eq!(challenge.attempts_remaining, 8);
        assert!(challenge.methods.contains(&MfaMethod::Totp));
    }

    #[tokio::test]
    async fn totp_and_backup_codes_complete_to_final_auth_responses() {
        for code in ["012345", "ST2-00112233-44556677-8899AABB-CCDDEEFF"] {
            let challenge_token = "completion-challenge-token";
            let base_url = spawn_code_sensitive_mfa_server(challenge_token, code).await;
            let response = complete_mfa_login(
                &reqwest::Client::new(),
                pending_for(base_url.clone(), challenge_token),
                SecretMfaCode::new(code),
            )
            .await
            .expect("valid second factor should complete login");

            let credentials = auth_response_to_credentials(&base_url, response);
            let stored = serde_json::to_string(&credentials).expect("serialize credentials");
            assert!(stored.contains("completed-access-token"));
            assert!(!stored.contains(challenge_token));
            assert!(!stored.contains(code));
            assert!(!stored.contains("challenge"));
        }
    }

    #[tokio::test]
    async fn mfa_verify_request_preserves_factor_code_byte_for_byte() {
        for code in [
            "",
            " ",
            "\t",
            " 012345",
            "012345 ",
            "０１２３４５",
            "012345",
            "ST2-00112233-44556677-8899AABB-CCDDEEFF",
            "ST2-not-a-canonical-backup-code",
        ] {
            let challenge_token = "oss-raw-code-challenge";
            let base_url = spawn_exact_mfa_request_server(challenge_token, code).await;
            complete_mfa_login(
                &reqwest::Client::new(),
                pending_for(base_url, challenge_token),
                SecretMfaCode::new(code),
            )
            .await
            .expect("server should observe the exact factor code");
        }
    }

    #[tokio::test]
    async fn wrong_totp_retains_continuation_for_backup_completion() {
        let challenge_token = "wrong-then-backup-challenge";
        let backup_code = "ST2-FFEEDDCC-BBAA9988-77665544-33221100";
        let base_url = spawn_code_sensitive_mfa_server(challenge_token, backup_code).await;

        let wrong = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(base_url, challenge_token),
            SecretMfaCode::new("000000"),
        )
        .await;
        let pending = match wrong {
            Err(CompleteMfaLoginError::Retryable {
                pending,
                attempts_remaining: Some(7),
                expires_in: Some(240),
                ..
            }) => pending,
            _ => panic!("wrong TOTP should return a retryable continuation"),
        };

        let completed = complete_mfa_login(
            &reqwest::Client::new(),
            pending,
            SecretMfaCode::new(backup_code),
        )
        .await
        .expect("backup code should complete the retained challenge");
        assert_eq!(completed.access_token, "completed-access-token");
    }

    #[tokio::test]
    async fn shortened_deadline_cannot_be_extended_and_expires_locally() {
        let clock = TestClock::new();
        let request_count = Arc::new(AtomicUsize::new(0));
        let base_url = spawn_shortening_then_rate_limited_server(Arc::clone(&request_count)).await;
        let pending = pending_for_at(base_url, "deadline-challenge", clock.now());

        let first = complete_mfa_login_with_clock(
            &reqwest::Client::new(),
            pending,
            SecretMfaCode::new("000000"),
            || clock.now(),
        )
        .await;
        let pending = match first {
            Err(CompleteMfaLoginError::Retryable {
                pending,
                expires_in: Some(5),
                ..
            }) => pending,
            _ => panic!("invalid code should shorten the live continuation to five seconds"),
        };
        assert_eq!(pending.remaining_seconds_at(clock.now()), 5);

        clock.advance(2);
        let second = complete_mfa_login_with_clock(
            &reqwest::Client::new(),
            pending,
            SecretMfaCode::new("000001"),
            || clock.now(),
        )
        .await;
        let pending = match second {
            Err(CompleteMfaLoginError::Retryable {
                pending,
                expires_in: Some(3),
                retry_after_seconds: Some(1),
                ..
            }) => pending,
            _ => panic!("a later rate limit must retain the shortened deadline"),
        };
        assert_eq!(pending.challenge().expires_in, 3);

        clock.advance(3);
        let expired = complete_mfa_login_with_clock(
            &reqwest::Client::new(),
            pending,
            SecretMfaCode::new("000002"),
            || clock.now(),
        )
        .await;
        assert!(matches!(
            expired,
            Err(CompleteMfaLoginError::Terminal(message))
                if message == MFA_CHALLENGE_EXPIRED_MESSAGE
        ));
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            2,
            "local expiry must stop a third verification request"
        );
    }

    #[test]
    fn shortened_deadline_survives_service_outage_and_totp_lock() {
        let clock = TestClock::new();
        let pending = shorten_pending_at(
            pending_for_at("http://unused.test".to_string(), "deadline", clock.now()),
            clock.now(),
        );

        clock.advance(2);
        let outage = map_mfa_verification_error(
            StatusCode::SERVICE_UNAVAILABLE.as_u16(),
            MfaVerificationErrorResponse {
                error: "mfa_service_unavailable".to_string(),
                message: "temporarily unavailable".to_string(),
                attempts_remaining: Some(7),
                expires_in: Some(300),
            },
            pending,
            &SecretMfaCode::new("000001"),
            None,
            clock.now(),
        );
        let pending = match outage {
            CompleteMfaLoginError::Retryable {
                pending,
                expires_in: Some(3),
                ..
            } => pending,
            _ => panic!("a service outage must retain the shortened deadline"),
        };

        let locked = map_mfa_verification_error(
            StatusCode::CONFLICT.as_u16(),
            MfaVerificationErrorResponse {
                error: "mfa_totp_locked".to_string(),
                message: "use a backup code".to_string(),
                attempts_remaining: Some(7),
                expires_in: Some(300),
            },
            pending,
            &SecretMfaCode::new("000002"),
            None,
            clock.now(),
        );
        assert!(matches!(
            locked,
            CompleteMfaLoginError::TotpLocked { pending, .. }
                if pending.remaining_seconds_at(clock.now()) == 3
                    && pending.challenge().expires_in == 3
        ));
    }

    #[tokio::test]
    async fn shortened_deadline_survives_transport_failure_and_stops_later_request() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let unavailable_base_url = format!(
            "http://{}",
            listener.local_addr().expect("transport test address")
        );
        drop(listener);

        let clock = TestClock::new();
        let pending = shorten_pending_at(
            pending_for_at(unavailable_base_url, "network-deadline", clock.now()),
            clock.now(),
        );
        clock.advance(2);
        let failed = complete_mfa_login_with_clock(
            &reqwest::Client::new(),
            pending,
            SecretMfaCode::new("000001"),
            || clock.now(),
        )
        .await;
        let mut pending = match failed {
            Err(CompleteMfaLoginError::Retryable {
                pending,
                expires_in: Some(3),
                ..
            }) => pending,
            _ => panic!("a transport failure must retain the shortened deadline"),
        };

        let request_count = Arc::new(AtomicUsize::new(0));
        pending.base_url =
            spawn_shortening_then_rate_limited_server(Arc::clone(&request_count)).await;
        clock.advance(3);
        let expired = complete_mfa_login_with_clock(
            &reqwest::Client::new(),
            pending,
            SecretMfaCode::new("000002"),
            || clock.now(),
        )
        .await;
        assert!(matches!(
            expired,
            Err(CompleteMfaLoginError::Terminal(message))
                if message == MFA_CHALLENGE_EXPIRED_MESSAGE
        ));
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            0,
            "local expiry must stop the later verification request"
        );
    }

    #[tokio::test]
    async fn caller_http_timeout_remains_effective() {
        let base_url = spawn_hanging_mfa_server().await;
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_millis(20))
            .build()
            .expect("client");
        let result = tokio::time::timeout(
            StdDuration::from_secs(1),
            complete_mfa_login(
                &client,
                pending_for(base_url, "caller-timeout"),
                SecretMfaCode::new("000001"),
            ),
        )
        .await
        .expect("the caller's shorter HTTP timeout must remain effective");
        assert!(matches!(
            result,
            Err(CompleteMfaLoginError::Retryable { .. })
        ));
    }

    #[tokio::test]
    async fn exact_client_upgrade_error_is_terminal_with_fixed_message() {
        let base_url = spawn_mfa_server(
            StatusCode::CONFLICT,
            json!({
                "error": "mfa_client_upgrade_required",
                "message": "server-controlled text"
            }),
            None,
        )
        .await;
        let result = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(base_url, "upgrade-challenge"),
            SecretMfaCode::new("012345"),
        )
        .await;
        assert!(matches!(
            result,
            Err(CompleteMfaLoginError::Terminal(message))
                if message == "client upgrade required for MFA"
        ));

        let mismatched_url = spawn_mfa_server(
            StatusCode::CONFLICT,
            json!({
                "error": "different_error",
                "message": "ordinary conflict"
            }),
            None,
        )
        .await;
        let mismatched = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(mismatched_url, "not-upgrade-challenge"),
            SecretMfaCode::new("012345"),
        )
        .await;
        assert!(matches!(
            mismatched,
            Err(CompleteMfaLoginError::Terminal(message)) if message == "ordinary conflict"
        ));
    }

    #[tokio::test]
    async fn login_finish_classifies_only_exact_upgrade_error_code() {
        let base_url = spawn_login_finish_server(
            StatusCode::CONFLICT,
            json!({
                "error": "mfa_client_upgrade_required",
                "message": "server-controlled text"
            }),
        )
        .await;
        let response = reqwest::Client::new()
            .post(format!("{base_url}/auth/opaque/login/finish"))
            .send()
            .await
            .expect("request");
        let error = match parse_login_finish_response(response, &base_url).await {
            Err(error) => error,
            Ok(_) => panic!("upgrade response must fail"),
        };
        assert_eq!(
            error.to_string(),
            "this client must be upgraded before signing in to an MFA-enabled account"
        );

        let mismatched_url = spawn_login_finish_server(
            StatusCode::CONFLICT,
            json!({
                "error": "different_error",
                "message": "mfa_client_upgrade_required"
            }),
        )
        .await;
        let response = reqwest::Client::new()
            .post(format!("{mismatched_url}/auth/opaque/login/finish"))
            .send()
            .await
            .expect("request");
        let error = match parse_login_finish_response(response, &mismatched_url).await {
            Err(error) => error,
            Ok(_) => panic!("conflict response must fail"),
        };
        assert_ne!(
            error.to_string(),
            "this client must be upgraded before signing in to an MFA-enabled account"
        );
    }

    #[tokio::test]
    async fn no_mfa_login_finish_remains_authenticated() {
        let base_url =
            spawn_login_finish_server(StatusCode::OK, successful_auth_body("no-mfa-access")).await;
        let response = reqwest::Client::new()
            .post(format!("{base_url}/auth/opaque/login/finish"))
            .send()
            .await
            .expect("request");
        let outcome = parse_login_finish_response(response, &base_url)
            .await
            .expect("ordinary login response");
        assert!(matches!(
            outcome,
            LoginOutcome::Authenticated(response) if response.access_token == "no-mfa-access"
        ));
    }

    #[tokio::test]
    async fn server_echoes_of_mfa_secrets_are_redacted_from_errors() {
        let challenge_token = "never-log-this-challenge";
        let code = "098765";
        let base_url = spawn_mfa_server(
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "invalid_mfa_code",
                "message": format!("bad {code} for {challenge_token}"),
                "attemptsRemaining": 7,
                "expiresIn": 240
            }),
            None,
        )
        .await;
        let result = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(base_url, challenge_token),
            SecretMfaCode::new(code),
        )
        .await;
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("wrong code should fail"),
        };
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains(challenge_token));
        assert!(!rendered.contains(code));
        assert!(rendered.contains("<redacted>"));
    }

    #[tokio::test]
    async fn rate_limit_returns_original_redacted_pending_continuation() {
        let raw_token = "distinctive-pending-challenge-token";
        let base_url = spawn_mfa_server(
            StatusCode::TOO_MANY_REQUESTS,
            json!({"error":"rate_limited","message":"try later"}),
            Some("17"),
        )
        .await;

        let result = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(base_url, raw_token),
            SecretMfaCode::new("012345"),
        )
        .await;

        match result {
            Err(CompleteMfaLoginError::Retryable {
                pending,
                retry_after_seconds,
                ..
            }) => {
                assert_eq!(retry_after_seconds, Some(17));
                assert!(!format!("{pending:?}").contains(raw_token));
            }
            _ => panic!("expected retryable rate limit"),
        }
    }

    #[tokio::test]
    async fn verifier_outage_preserves_backup_capable_continuation() {
        let base_url = spawn_mfa_server(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error":"mfa_service_unavailable","message":"temporarily unavailable"}),
            None,
        )
        .await;

        let result = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(base_url, "outage-token"),
            SecretMfaCode::new("012345"),
        )
        .await;

        assert!(matches!(
            result,
            Err(CompleteMfaLoginError::Retryable { pending, .. })
                if pending.challenge().methods.contains(&MfaMethod::BackupCode)
        ));
    }

    #[tokio::test]
    async fn totp_lock_retains_continuation_but_expiry_is_terminal() {
        let locked_url = spawn_mfa_server(
            StatusCode::CONFLICT,
            json!({"error":"mfa_totp_locked","message":"use a backup code"}),
            None,
        )
        .await;
        let locked = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(locked_url, "locked-token"),
            SecretMfaCode::new("012345"),
        )
        .await;
        assert!(matches!(
            locked,
            Err(CompleteMfaLoginError::TotpLocked { .. })
        ));

        let expired_url = spawn_mfa_server(
            StatusCode::UNAUTHORIZED,
            json!({"error":"mfa_challenge_invalid_or_expired","message":"start again"}),
            None,
        )
        .await;
        let expired = complete_mfa_login(
            &reqwest::Client::new(),
            pending_for(expired_url, "expired-token"),
            SecretMfaCode::new("012345"),
        )
        .await;
        assert!(matches!(expired, Err(CompleteMfaLoginError::Terminal(_))));
    }

    fn test_credentials() -> Credentials {
        Credentials {
            api_url: "https://sealtask.example.test".to_string(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            access_expires_at: Utc::now() + Duration::hours(1),
            refresh_expires_at: Utc::now() + Duration::days(1),
            user_id: Uuid::now_v7(),
            email: "test@example.com".to_string(),
            data_key_ciphertext: STANDARD_NO_PAD.encode(b"ciphertext"),
        }
    }

    fn private_temp_dir() -> TempDir {
        let directory = tempfile::Builder::new()
            .prefix(".sealtask-client-auth-test-")
            .tempdir_in(".")
            .expect("private test directory");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
                .expect("secure test directory");
        }
        directory
    }

    fn write_credentials_fixture(path: &Path, size: usize) {
        set_config_dir_permissions(path.parent().expect("credentials fixture parent"))
            .expect("secure credentials fixture directory");
        let mut body = serde_json::to_vec(&test_credentials()).expect("serialize credentials");
        assert!(body.len() <= size, "fixture size must fit requested body");
        body.resize(size, b' ');
        fs::write(path, body).expect("write credentials fixture");
        set_secret_file_permissions(path).expect("secure credentials fixture");
    }

    #[test]
    fn credentials_debug_redacts_every_secret_and_identity_string() {
        let mut credentials = test_credentials();
        credentials.api_url = "https://private-api.example/tenant".to_string();
        credentials.access_token = "access-secret-value".to_string();
        credentials.refresh_token = "refresh-secret-value".to_string();
        credentials.email = "private-user@example.test".to_string();
        credentials.data_key_ciphertext = "private-data-key-ciphertext".to_string();

        let rendered = format!("{credentials:?}");

        for private in [
            &credentials.api_url,
            &credentials.access_token,
            &credentials.refresh_token,
            &credentials.email,
            &credentials.data_key_ciphertext,
        ] {
            assert!(!rendered.contains(private));
        }
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn credentials_file_size_bound_accepts_exact_limit_and_rejects_one_more() {
        let exact = private_temp_dir();
        let exact_path = exact.path().join(CREDENTIALS_FILE_NAME);
        write_credentials_fixture(&exact_path, MAX_CREDENTIALS_FILE_BYTES as usize);
        load_credentials_unlocked(&exact_path)
            .expect("exact file limit")
            .expect("credentials");

        const PRIVATE_OVERSIZED_SUFFIX: &str = "oversized-private-refresh-secret";
        let oversized = private_temp_dir();
        set_config_dir_permissions(oversized.path()).expect("secure oversized fixture directory");
        let oversized_path = oversized.path().join(CREDENTIALS_FILE_NAME);
        let mut body =
            serde_json::to_vec(&test_credentials()).expect("serialize oversized credentials");
        body.resize(
            MAX_CREDENTIALS_FILE_BYTES as usize + 1 - PRIVATE_OVERSIZED_SUFFIX.len(),
            b' ',
        );
        body.extend_from_slice(PRIVATE_OVERSIZED_SUFFIX.as_bytes());
        fs::write(&oversized_path, body).expect("write oversized credentials");
        set_secret_file_permissions(&oversized_path).expect("secure oversized credentials");

        let error =
            load_credentials_unlocked(&oversized_path).expect_err("one byte over the file limit");
        assert!(error.to_string().contains("65536-byte limit"));
        assert!(!error.to_string().contains(PRIVATE_OVERSIZED_SUFFIX));
        assert!(!format!("{error:?}").contains(PRIVATE_OVERSIZED_SUFFIX));
    }

    #[cfg(unix)]
    #[test]
    fn credentials_file_rejects_symlinks_hardlinks_fifos_and_broad_modes() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt as _;
        use std::os::unix::fs::{PermissionsExt as _, symlink};

        let symlink_dir = private_temp_dir();
        let symlink_target = symlink_dir.path().join("target.json");
        write_credentials_fixture(&symlink_target, 1_024);
        let symlink_path = symlink_dir.path().join(CREDENTIALS_FILE_NAME);
        symlink(&symlink_target, &symlink_path).expect("credentials symlink");
        let error =
            load_credentials_unlocked(&symlink_path).expect_err("credentials symlink rejected");
        assert!(error.to_string().contains("symlink"));

        let directory_link_root = private_temp_dir();
        let real_directory = directory_link_root.path().join("real-config");
        fs::create_dir(&real_directory).expect("real config directory");
        let real_credentials = real_directory.join(CREDENTIALS_FILE_NAME);
        write_credentials_fixture(&real_credentials, 1_024);
        let linked_directory = directory_link_root.path().join("linked-config");
        symlink(&real_directory, &linked_directory).expect("config directory symlink");
        let error = load_credentials_unlocked(&linked_directory.join(CREDENTIALS_FILE_NAME))
            .expect_err("credentials directory symlink rejected");
        assert!(error.to_string().contains("symlink"));

        let ancestor_link_root = private_temp_dir();
        let real_profiles = ancestor_link_root.path().join("real-profiles");
        let real_profile = real_profiles.join("operator");
        fs::create_dir_all(&real_profile).expect("real profile directory");
        let real_profile_credentials = real_profile.join(CREDENTIALS_FILE_NAME);
        write_credentials_fixture(&real_profile_credentials, 1_024);
        let linked_profiles = ancestor_link_root.path().join("profiles");
        symlink(&real_profiles, &linked_profiles).expect("profiles ancestor symlink");
        let error = load_credentials_unlocked(
            &linked_profiles.join("operator").join(CREDENTIALS_FILE_NAME),
        )
        .expect_err("credentials ancestor symlink rejected");
        assert!(error.to_string().contains("symlink"));
        let linked_new_profile = linked_profiles.join("new-operator");
        let error = match CredentialsFileLock::acquire(&linked_new_profile) {
            Ok(_) => panic!("credentials creation through ancestor symlink must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("symlink"));
        assert!(
            !real_profiles.join("new-operator").exists(),
            "rejected credential-store creation must not mutate the symlink target"
        );

        let hardlink_dir = private_temp_dir();
        let hardlink_target = hardlink_dir.path().join("original.json");
        write_credentials_fixture(&hardlink_target, 1_024);
        let hardlink_path = hardlink_dir.path().join(CREDENTIALS_FILE_NAME);
        fs::hard_link(&hardlink_target, &hardlink_path).expect("credentials hardlink");
        let error =
            load_credentials_unlocked(&hardlink_path).expect_err("credentials hardlink rejected");
        assert!(error.to_string().contains("exactly one hard link"));

        let broad_dir = private_temp_dir();
        let broad_path = broad_dir.path().join(CREDENTIALS_FILE_NAME);
        write_credentials_fixture(&broad_path, 1_024);
        fs::set_permissions(&broad_path, fs::Permissions::from_mode(0o644))
            .expect("broaden credentials permissions");
        let error =
            load_credentials_unlocked(&broad_path).expect_err("broad credentials mode rejected");
        assert!(error.to_string().contains("permissions are too broad"));

        let fifo_dir = private_temp_dir();
        set_config_dir_permissions(fifo_dir.path()).expect("secure FIFO fixture directory");
        let fifo_path = fifo_dir.path().join(CREDENTIALS_FILE_NAME);
        let fifo_c_path =
            CString::new(fifo_path.as_os_str().as_bytes()).expect("FIFO path without NUL");
        // SAFETY: `fifo_c_path` is a valid NUL-terminated path and `mkfifo`
        // does not retain the pointer.
        let result = unsafe { libc::mkfifo(fifo_c_path.as_ptr(), 0o600) };
        assert_eq!(result, 0, "create credentials FIFO");
        let error = load_credentials_unlocked(&fifo_path).expect_err("credentials FIFO rejected");
        assert!(error.to_string().contains("regular file"));
    }

    #[cfg(windows)]
    #[test]
    fn test_should_reject_hardlinked_credentials_file() {
        let directory = private_temp_dir();
        let original_path = directory.path().join("original.json");
        write_credentials_fixture(&original_path, 1_024);
        let credentials_path = directory.path().join(CREDENTIALS_FILE_NAME);
        fs::hard_link(&original_path, &credentials_path).expect("credentials hard link");

        let error = load_credentials_unlocked(&credentials_path)
            .expect_err("credentials hard link must be rejected");
        assert!(error.to_string().contains("exactly one hard link"));
    }

    #[cfg(unix)]
    #[test]
    fn credentials_file_rejects_a_foreign_owner_identity() {
        let error = validate_effective_owner_ids(42, 43, "credentials file")
            .expect_err("foreign owner rejected");
        assert!(error.to_string().contains("current effective user"));
        validate_effective_owner_ids(42, 42, "credentials file").expect("matching owner");
    }

    #[cfg(unix)]
    #[test]
    fn credentials_directory_security_operations_use_an_operable_capability_handle() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = TempDir::new().expect("temporary credentials directory");
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o750))
            .expect("broaden credentials directory for tightening");
        let (directory, created) = open_directory_nofollow(temp.path(), false)
            .expect("open credentials directory through capability traversal")
            .expect("credentials directory exists");
        assert!(!created);

        let operable =
            open_operable_directory_handle(&directory).expect("open operable directory handle");
        #[cfg(target_os = "linux")]
        {
            // SAFETY: `operable` owns a live descriptor and `F_GETFL` only
            // inspects its status flags.
            let flags = unsafe { libc::fcntl(operable.as_raw_fd(), libc::F_GETFL) };
            assert_ne!(flags, -1, "inspect operable directory flags");
            assert_eq!(
                flags & libc::O_PATH,
                0,
                "operable handle must not use O_PATH"
            );
        }
        operable.sync_all().expect("sync operable directory handle");

        restrict_secret_directory_handle_permissions(&directory)
            .expect("tighten and sync credentials directory");
        validate_secret_directory_handle(&directory)
            .expect("inspect secured credentials directory");
        sync_directory_handle(&directory).expect("sync credentials directory");

        let mode = fs::metadata(temp.path())
            .expect("inspect credentials directory")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn credentials_store_accepts_root_owned_macos_var_and_tmp_aliases() {
        assert_eq!(
            absolute_normalized_credentials_path(Path::new("/var/example"))
                .expect("normalize /var"),
            Path::new("/private/var/example")
        );
        assert_eq!(
            absolute_normalized_credentials_path(Path::new("/tmp/example"))
                .expect("normalize /tmp"),
            Path::new("/private/tmp/example")
        );

        let standard_temp = TempDir::new().expect("standard macOS temporary directory");
        set_config_dir_permissions(standard_temp.path())
            .expect("secure standard macOS temporary directory");
        let config_directory = standard_temp.path().join("config");
        assert!(
            CredentialStore::open(&config_directory, true)
                .expect("create config directory through trusted system alias")
                .is_some()
        );
        assert!(config_directory.is_dir());
    }

    #[test]
    fn credentials_lock_wait_is_bounded() {
        let temp = private_temp_dir();
        let first = CredentialsFileLock::acquire(temp.path()).expect("first credentials lock");
        let started = Instant::now();
        let error = match CredentialsFileLock::acquire(temp.path()) {
            Ok(_) => panic!("second credentials lock must time out"),
            Err(error) => error,
        };
        let elapsed = started.elapsed();
        assert!(matches!(error, PublicError::Conflict(_)));
        assert!(elapsed >= CREDENTIALS_LOCK_TIMEOUT);
        assert!(elapsed < CREDENTIALS_LOCK_TIMEOUT + StdDuration::from_secs(1));
        drop(first);
    }

    #[test]
    fn credentials_replacement_is_durable_and_leaves_no_temporary_file() {
        let temp = private_temp_dir();
        let credentials = test_credentials();
        replace_credentials_atomically_in(temp.path(), &credentials, |_| Ok(()))
            .expect("replace credentials");
        let mut replacement = credentials.clone();
        replacement.access_token = "replacement-access".to_string();
        replacement.refresh_token = "replacement-refresh".to_string();
        let previous = replace_credentials_atomically_in(temp.path(), &replacement, |_| Ok(()))
            .expect("replace existing credentials");
        assert_eq!(previous, Some(credentials));

        let names = fs::read_dir(temp.path())
            .expect("read credentials directory")
            .map(|entry| {
                entry
                    .expect("credentials directory entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        assert!(names.iter().any(|name| name == CREDENTIALS_FILE_NAME));
        assert!(
            !names
                .iter()
                .any(|name| name.starts_with(".credentials-") && name.ends_with(".tmp"))
        );
        assert_eq!(
            load_credentials_unlocked(&temp.path().join(CREDENTIALS_FILE_NAME))
                .expect("load durable credentials"),
            Some(replacement)
        );
    }

    #[tokio::test]
    async fn refresh_should_preserve_structured_rate_limit_metadata_without_backend_prose() {
        const PRIVATE_MESSAGE: &str =
            "retry refresh-secret at https://private.example.test/account/42";
        let app = Router::new().route(
            "/auth/refresh",
            post(|| async {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "error": "refresh_rate_limited",
                        "message": PRIVATE_MESSAGE
                    })),
                )
                    .into_response();
                response
                    .headers_mut()
                    .insert(header::RETRY_AFTER, HeaderValue::from_static("7"));
                response
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve refresh API");
        });

        let error = refresh_access_token(
            &reqwest::Client::new(),
            &format!("http://{address}"),
            "refresh-secret",
        )
        .await
        .expect_err("rate-limited refresh must fail");

        assert_eq!(error.http_status(), Some(429));
        assert_eq!(error.backend_error_code(), Some("refresh_rate_limited"));
        assert_eq!(error.retry_after(), Some(StdDuration::from_secs(7)));
        assert_eq!(error.code(), "rate_limited");
        assert_eq!(error.transport_failure_kind(), None);
        assert!(!error.to_string().contains(PRIVATE_MESSAGE));
        assert!(!format!("{error:?}").contains(PRIVATE_MESSAGE));
        assert!(!format!("{error:?}").contains("refresh-secret"));
    }

    #[tokio::test]
    async fn refresh_should_reject_oversized_response_bodies_without_exposing_them() {
        const PRIVATE_BODY_PREFIX: &str = "refresh-secret-private-response:";
        let app = Router::new().route(
            "/auth/refresh",
            post(|| async {
                format!(
                    "{PRIVATE_BODY_PREFIX}{}",
                    "x".repeat(MAX_REFRESH_RESPONSE_BYTES)
                )
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve refresh API");
        });

        let error = refresh_access_token(
            &reqwest::Client::new(),
            &format!("http://{address}"),
            "refresh-secret",
        )
        .await
        .expect_err("oversized refresh response must fail");

        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::BodyTooLarge)
        );
        assert_eq!(error.http_status(), None);
        assert!(!error.to_string().contains(PRIVATE_BODY_PREFIX));
        assert!(!format!("{error:?}").contains(PRIVATE_BODY_PREFIX));
        assert!(!format!("{error:?}").contains("refresh-secret"));
    }

    #[tokio::test]
    async fn refresh_should_classify_closed_listener_failure_without_exposing_request_context() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        drop(listener);
        let base_url = format!("http://{address}");
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(StdDuration::from_millis(250))
            .timeout(StdDuration::from_secs(3))
            .build()
            .expect("client");

        let error = refresh_access_token(&client, &base_url, "refresh-secret")
            .await
            .expect_err("connection to a closed listener must fail");

        assert_eq!(
            error.transport_failure_kind(),
            Some(TransportFailureKind::Connect)
        );
        assert_eq!(error.http_status(), None);
        assert!(!error.to_string().contains(&base_url));
        assert!(!format!("{error:?}").contains(&base_url));
        assert!(!format!("{error:?}").contains("refresh-secret"));
    }

    fn spawn_refresh_race_child(dir: &Path, base_url: &str, ready_path: &Path) -> Child {
        Command::new(std::env::current_exe().expect("current test executable"))
            .arg("--exact")
            .arg("tests::test_credentials_store_refresh_worker")
            .arg("--nocapture")
            .env(REFRESH_RACE_BASE_URL_ENV, base_url)
            .env(REFRESH_RACE_CREDENTIALS_DIR_ENV, dir)
            .env(REFRESH_RACE_READY_PATH_ENV, ready_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn refresh worker")
    }

    async fn wait_for_path(path: &Path) {
        tokio::time::timeout(StdDuration::from_secs(5), async {
            while !path.exists() {
                tokio::time::sleep(StdDuration::from_millis(10)).await;
            }
        })
        .await
        .expect("refresh worker should become ready");
    }

    async fn wait_for_refresh_race_child(child: Child) -> Output {
        tokio::time::timeout(
            StdDuration::from_secs(10),
            tokio::task::spawn_blocking(move || child.wait_with_output()),
        )
        .await
        .expect("refresh worker should finish")
        .expect("join refresh worker wait")
        .expect("wait for refresh worker")
    }

    fn assert_refresh_race_child_succeeded(output: &Output) {
        assert!(
            output.status.success(),
            "refresh worker failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[tokio::test]
    async fn test_credentials_store_refresh_worker() {
        let Some(dir) = std::env::var_os(REFRESH_RACE_CREDENTIALS_DIR_ENV) else {
            return;
        };
        let base_url = std::env::var(REFRESH_RACE_BASE_URL_ENV).expect("refresh API URL");
        let ready_path =
            PathBuf::from(std::env::var_os(REFRESH_RACE_READY_PATH_ENV).expect("ready path"));
        let dir = PathBuf::from(dir);
        let expected = load_credentials_unlocked(&dir.join(CREDENTIALS_FILE_NAME))
            .expect("load initial credentials")
            .expect("initial credentials");
        fs::write(&ready_path, b"ready").expect("mark refresh worker ready");

        let refreshed = refresh_credentials_if_needed_in(
            &dir,
            &reqwest::Client::new(),
            &base_url,
            &expected,
            60,
            StdDuration::from_secs(5),
        )
        .await
        .expect("refresh credentials");

        assert_eq!(refreshed.access_token, "refreshed-access");
        assert_eq!(refreshed.refresh_token, "refreshed-refresh");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_credentials_store_should_serialize_refresh_redemption_across_processes() {
        let request_count = Arc::new(AtomicUsize::new(0));
        let first_request_started = Arc::new(Notify::new());
        let release_first_response = Arc::new(Notify::new());
        let handler_request_count = Arc::clone(&request_count);
        let handler_first_request_started = Arc::clone(&first_request_started);
        let handler_release_first_response = Arc::clone(&release_first_response);
        let app = Router::new().route(
            "/auth/refresh",
            post(move |Json(body): Json<serde_json::Value>| {
                let request_count = Arc::clone(&handler_request_count);
                let first_request_started = Arc::clone(&handler_first_request_started);
                let release_first_response = Arc::clone(&handler_release_first_response);
                async move {
                    assert_eq!(body["refreshToken"], "refresh");
                    let request_number = request_count.fetch_add(1, Ordering::SeqCst) + 1;
                    if request_number == 1 {
                        first_request_started.notify_one();
                        release_first_response.notified().await;
                    }
                    Json(json!({
                        "accessToken": "refreshed-access",
                        "refreshToken": "refreshed-refresh",
                        "expiresIn": 3600,
                        "refreshExpiresIn": 86400,
                        "tokenType": "Bearer"
                    }))
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve refresh API");
        });
        let base_url = format!("http://{address}");

        let temp = private_temp_dir();
        let credentials_dir = temp.path().join("credentials");
        let mut original = test_credentials();
        original.api_url.clone_from(&base_url);
        original.access_expires_at = Utc::now() - Duration::seconds(1);
        replace_credentials_atomically_in(&credentials_dir, &original, |_| Ok(()))
            .expect("store original credentials");

        let first_ready = temp.path().join("first-ready");
        let first = spawn_refresh_race_child(&credentials_dir, &base_url, &first_ready);
        tokio::time::timeout(StdDuration::from_secs(5), first_request_started.notified())
            .await
            .expect("first refresh request should start");

        let second_ready = temp.path().join("second-ready");
        let second = spawn_refresh_race_child(&credentials_dir, &base_url, &second_ready);
        wait_for_path(&second_ready).await;
        tokio::time::sleep(StdDuration::from_millis(100)).await;
        let requests_before_release = request_count.load(Ordering::SeqCst);
        release_first_response.notify_one();

        let (first_output, second_output) = tokio::join!(
            wait_for_refresh_race_child(first),
            wait_for_refresh_race_child(second)
        );
        assert_refresh_race_child_succeeded(&first_output);
        assert_refresh_race_child_succeeded(&second_output);
        assert_eq!(
            requests_before_release, 1,
            "the second process must wait before redeeming the rotating token"
        );
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            1,
            "waiting processes must reuse the persisted refresh result"
        );
        let persisted = load_credentials_unlocked(&credentials_dir.join(CREDENTIALS_FILE_NAME))
            .expect("load refreshed credentials")
            .expect("refreshed credentials");
        assert_eq!(persisted.access_token, "refreshed-access");
        assert_eq!(persisted.refresh_token, "refreshed-refresh");
    }

    #[tokio::test]
    async fn test_credentials_store_should_reject_a_changed_crypto_context_before_refresh() {
        let temp = private_temp_dir();
        let expected = test_credentials();
        let mut current = expected.clone();
        current.access_expires_at = Utc::now() - Duration::seconds(1);
        current.data_key_ciphertext = STANDARD_NO_PAD.encode(b"replacement-ciphertext");
        replace_credentials_atomically_in(temp.path(), &current, |_| Ok(()))
            .expect("store changed credentials");

        let error = refresh_credentials_if_needed_in(
            temp.path(),
            &reqwest::Client::new(),
            "http://127.0.0.1:1",
            &expected,
            60,
            StdDuration::from_millis(50),
        )
        .await
        .expect_err("a changed crypto context must not reuse refreshed credentials");

        assert!(matches!(error, PublicError::Conflict(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_credentials_store_should_bound_refresh_and_release_the_lock_on_timeout() {
        let app = Router::new().route(
            "/auth/refresh",
            post(|| async { std::future::pending::<Json<serde_json::Value>>().await }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve refresh API");
        });
        let base_url = format!("http://{address}");

        let temp = private_temp_dir();
        let dir = temp.path().to_path_buf();
        let mut original = test_credentials();
        original.api_url.clone_from(&base_url);
        original.access_expires_at = Utc::now() - Duration::seconds(1);
        replace_credentials_atomically_in(&dir, &original, |_| Ok(()))
            .expect("store original credentials");

        let error = refresh_credentials_if_needed_in(
            &dir,
            &reqwest::Client::new(),
            &base_url,
            &original,
            60,
            StdDuration::from_millis(25),
        )
        .await
        .expect_err("a stalled refresh must time out");
        assert_eq!(
            error.transport_failure_kind(),
            Some(TransportFailureKind::Timeout)
        );
        assert_eq!(error.http_status(), None);

        let mut replacement = original;
        replacement.access_token = "replacement-access".to_string();
        tokio::time::timeout(
            StdDuration::from_secs(1),
            tokio::task::spawn_blocking(move || {
                replace_credentials_atomically_in(&dir, &replacement, |_| Ok(()))
            }),
        )
        .await
        .expect("the credential lock must be released after a refresh timeout")
        .expect("join replacement after timeout")
        .expect("replace credentials after timeout");
    }

    #[test]
    fn test_credentials_store_should_atomically_compare_and_swap() {
        let temp = private_temp_dir();
        let original = test_credentials();
        replace_credentials_atomically_in(temp.path(), &original, |_| Ok(()))
            .expect("store original credentials");

        let mut refreshed = original.clone();
        refreshed.access_token = "refreshed-access".to_string();
        refreshed.refresh_token = "refreshed-refresh".to_string();
        assert!(
            save_credentials_if_current_in(temp.path(), &original, &refreshed)
                .expect("compare and swap refreshed credentials")
        );

        let mut stale_update = original.clone();
        stale_update.access_token = "stale-access".to_string();
        assert!(
            !save_credentials_if_current_in(temp.path(), &original, &stale_update)
                .expect("reject stale compare and swap")
        );
        assert_eq!(
            load_credentials_unlocked(&temp.path().join(CREDENTIALS_FILE_NAME))
                .expect("load credentials"),
            Some(refreshed)
        );
    }

    #[test]
    fn test_credentials_store_should_clean_up_the_latest_snapshot_before_replace() {
        let temp = private_temp_dir();
        let original = test_credentials();
        replace_credentials_atomically_in(temp.path(), &original, |_| Ok(()))
            .expect("store original credentials");
        let mut rotated = original.clone();
        rotated.refresh_token = "rotated-refresh".to_string();
        assert!(
            save_credentials_if_current_in(temp.path(), &original, &rotated)
                .expect("rotate credentials")
        );

        let observed = RefCell::new(None);
        let replacement = test_credentials();
        let previous = replace_credentials_atomically_in(temp.path(), &replacement, |current| {
            observed.replace(current.cloned());
            Ok(())
        })
        .expect("replace credentials");

        assert_eq!(observed.into_inner(), Some(rotated.clone()));
        assert_eq!(previous, Some(rotated));
        assert_eq!(
            load_credentials_unlocked(&temp.path().join(CREDENTIALS_FILE_NAME))
                .expect("load replacement"),
            Some(replacement)
        );
    }

    #[test]
    fn test_credentials_store_should_not_run_guarded_writes_for_a_stale_snapshot() {
        let temp = private_temp_dir();
        let original = test_credentials();
        let mut current = original.clone();
        current.access_token = "newer-access".to_string();
        replace_credentials_atomically_in(temp.path(), &current, |_| Ok(()))
            .expect("store current credentials");
        let action_ran = Cell::new(false);

        let error = with_current_credentials_in(temp.path(), &original, |_| {
            action_ran.set(true);
            Ok(())
        })
        .expect_err("a stale snapshot must not authorize a local-secret write");

        assert!(matches!(error, PublicError::Conflict(_)));
        assert!(!action_ran.get());
    }

    #[test]
    fn credential_identity_guard_allows_token_refresh_but_returns_current_credentials() {
        let temp = private_temp_dir();
        let mut expected = test_credentials();
        expected.api_url.push('/');
        expected.data_key_ciphertext = format!(" {} \n", expected.data_key_ciphertext);
        let mut current = expected.clone();
        current.api_url = normalize_api_url(&current.api_url);
        current.data_key_ciphertext = current.data_key_ciphertext.trim().to_string();
        current.access_token = "refreshed-access".to_string();
        current.refresh_token = "refreshed-refresh".to_string();
        current.access_expires_at += Duration::hours(1);
        current.refresh_expires_at += Duration::hours(1);
        replace_credentials_atomically_in(temp.path(), &current, |_| Ok(()))
            .expect("store refreshed credentials");

        let observed_access_token = RefCell::new(None);
        with_current_credential_identity_in(temp.path(), &expected, |current| {
            observed_access_token.replace(Some(current.access_token.clone()));
            Ok(())
        })
        .expect("token rotation must preserve the credential identity");

        assert_eq!(
            observed_access_token.into_inner().as_deref(),
            Some("refreshed-access")
        );
    }

    #[test]
    fn credential_identity_guard_rejects_api_account_or_data_key_switches() {
        let expected = test_credentials();
        let mut different_api = expected.clone();
        different_api.api_url = "https://different-api.example".to_string();
        let mut different_account = expected.clone();
        different_account.user_id = Uuid::now_v7();
        let mut different_data_key = expected.clone();
        different_data_key.data_key_ciphertext =
            STANDARD_NO_PAD.encode(b"replacement-data-key-ciphertext");

        for current in [different_api, different_account, different_data_key] {
            let temp = private_temp_dir();
            replace_credentials_atomically_in(temp.path(), &current, |_| Ok(()))
                .expect("store switched credential identity");
            let action_ran = Cell::new(false);

            let error = with_current_credential_identity_in(temp.path(), &expected, |_| {
                action_ran.set(true);
                Ok(())
            })
            .expect_err("a switched credential identity must reject guarded state writes");

            assert!(matches!(error, PublicError::Conflict(_)));
            assert!(!action_ran.get());
        }
    }

    #[test]
    fn test_credentials_store_should_serialize_concurrent_replacements() {
        let temp = private_temp_dir();
        let dir = temp.path().to_path_buf();
        let original = test_credentials();
        replace_credentials_atomically_in(&dir, &original, |_| Ok(()))
            .expect("store original credentials");

        let mut first_replacement = original.clone();
        first_replacement.access_token = "first-replacement".to_string();
        let mut second_replacement = original;
        second_replacement.access_token = "second-replacement".to_string();
        let (first_entered_tx, first_entered_rx) = mpsc::channel();
        let (release_first_tx, release_first_rx) = mpsc::channel();
        let first_dir = dir.clone();
        let first_for_thread = first_replacement.clone();
        let first = std::thread::spawn(move || {
            replace_credentials_atomically_in(&first_dir, &first_for_thread, |_| {
                first_entered_tx.send(()).expect("report first lock");
                release_first_rx.recv().expect("release first lock");
                Ok(())
            })
            .expect("first replacement")
        });
        first_entered_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("first replacement should acquire the lock");

        let (second_entered_tx, second_entered_rx) = mpsc::channel();
        let second_dir = dir.clone();
        let second = std::thread::spawn(move || {
            replace_credentials_atomically_in(&second_dir, &second_replacement, |current| {
                second_entered_tx
                    .send(current.cloned())
                    .expect("report second lock");
                Ok(())
            })
            .expect("second replacement")
        });
        assert!(
            second_entered_rx
                .recv_timeout(StdDuration::from_millis(50))
                .is_err(),
            "the second replacement must wait for the first process lock"
        );

        release_first_tx
            .send(())
            .expect("release first replacement");
        let second_observed = second_entered_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("second replacement should proceed after unlock");
        first.join().expect("join first replacement");
        second.join().expect("join second replacement");

        assert_eq!(second_observed, Some(first_replacement));
    }

    #[test]
    fn test_persisted_data_key_round_trips_through_test_backend() {
        let temp = private_temp_dir();
        let credentials = test_credentials();
        unsafe {
            std::env::set_var(TEST_KEYCHAIN_DIR_ENV, temp.path());
        }

        save_persisted_data_key(&credentials, b"secret").expect("store key");
        let loaded = load_persisted_data_key(&credentials).expect("load key");

        assert_eq!(loaded.as_deref(), Some(b"secret".as_slice()));
        assert_eq!(
            persisted_data_key_status(&credentials),
            PersistedDataKeyStatus::Available
        );

        clear_persisted_data_key(&credentials).expect("clear key");
        assert_eq!(
            load_persisted_data_key(&credentials).expect("reload key"),
            None
        );
        assert_eq!(
            persisted_data_key_status(&credentials),
            PersistedDataKeyStatus::Missing
        );

        unsafe {
            std::env::remove_var(TEST_KEYCHAIN_DIR_ENV);
        }
    }
}
