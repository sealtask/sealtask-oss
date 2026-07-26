#![cfg_attr(test, allow(clippy::unwrap_used))]

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration as StdDuration, Instant};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
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
use tempfile::NamedTempFile;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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

impl CredentialsFileLock {
    fn acquire(dir: &Path) -> PublicResult<Self> {
        prepare_config_dir(dir)?;
        let lock_path = dir.join(CREDENTIALS_LOCK_FILE_NAME);
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|err| {
                PublicError::unexpected(format!("failed to open credentials lock file: {err}"))
            })?;
        set_secret_file_permissions(&lock_path)?;
        fs2::FileExt::lock_exclusive(&lock_file).map_err(|err| {
            PublicError::unexpected(format!("failed to lock credentials file: {err}"))
        })?;
        Ok(Self {
            file: Some(lock_file),
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
        fs2::FileExt::unlock(&file).map_err(|err| {
            PublicError::unexpected(format!("failed to unlock credentials file: {err}"))
        })
    }
}

impl Drop for CredentialsFileLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = fs2::FileExt::unlock(&file);
        }
    }
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
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to open credentials file: {err}"
            )));
        }
    };
    let reader = BufReader::new(file);
    let credentials: Credentials = serde_json::from_reader(reader).map_err(|err| {
        PublicError::unexpected(format!("failed to parse credentials file: {err}"))
    })?;
    Ok(Some(credentials))
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
    with_credentials_lock_in(&dir, || save_credentials_unlocked(&dir, credentials))
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
    with_credentials_lock_in(dir, || {
        let path = dir.join(CREDENTIALS_FILE_NAME);
        if load_credentials_unlocked(&path)?.as_ref() != Some(expected) {
            return Ok(false);
        }

        save_credentials_unlocked(dir, updated)?;
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
        let current = load_credentials_unlocked(&dir.join(CREDENTIALS_FILE_NAME))?
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
        save_credentials_unlocked(dir, &refreshed)?;
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
    with_credentials_lock_in(dir, || {
        let previous = load_credentials_unlocked(&dir.join(CREDENTIALS_FILE_NAME))?;
        before_replace(previous.as_ref())?;
        save_credentials_unlocked(dir, credentials)?;
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
    with_credentials_lock_in(dir, || {
        let current = load_credentials_unlocked(&dir.join(CREDENTIALS_FILE_NAME))?;
        let current = current
            .as_ref()
            .filter(|current| *current == expected)
            .ok_or_else(credentials_changed_error)?;
        action(current)
    })
}

pub fn clear_credentials_if_current(
    expected: &Credentials,
    before_clear: impl FnOnce(&Credentials) -> PublicResult<()>,
) -> PublicResult<()> {
    let dir = config_dir()?;
    with_credentials_lock_in(&dir, || {
        let path = dir.join(CREDENTIALS_FILE_NAME);
        let current = load_credentials_unlocked(&path)?;
        let current = current
            .as_ref()
            .filter(|current| *current == expected)
            .ok_or_else(credentials_changed_error)?;
        let cleanup_result = before_clear(current);
        clear_credentials_unlocked(&dir)?;
        cleanup_result
    })
}

pub fn clear_credentials() -> PublicResult<()> {
    let dir = config_dir()?;
    with_credentials_lock_in(&dir, || clear_credentials_unlocked(&dir))
}

fn clear_credentials_unlocked(dir: &Path) -> PublicResult<()> {
    let path = dir.join(CREDENTIALS_FILE_NAME);
    match fs::remove_file(&path) {
        Ok(()) => sync_config_dir(dir)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to remove credentials file: {err}"
            )));
        }
    }
    Ok(())
}

fn save_credentials_unlocked(dir: &Path, credentials: &Credentials) -> PublicResult<()> {
    prepare_config_dir(dir)?;
    let path = dir.join(CREDENTIALS_FILE_NAME);
    let mut temporary = NamedTempFile::new_in(dir).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to create temporary credentials file: {err}"
        ))
    })?;
    set_secret_file_permissions(temporary.path())?;
    serde_json::to_writer_pretty(&mut temporary, credentials).map_err(|err| {
        PublicError::unexpected(format!("failed to write credentials file: {err}"))
    })?;
    temporary.write_all(b"\n").map_err(|err| {
        PublicError::unexpected(format!("failed to finish credentials file: {err}"))
    })?;
    temporary.as_file().sync_all().map_err(|err| {
        PublicError::unexpected(format!("failed to sync credentials file: {err}"))
    })?;
    temporary.persist(&path).map_err(|err| {
        PublicError::unexpected(format!("failed to replace credentials file: {}", err.error))
    })?;
    sync_config_dir(dir)
}

fn with_credentials_lock_in<T>(
    dir: &Path,
    action: impl FnOnce() -> PublicResult<T>,
) -> PublicResult<T> {
    let credentials_lock = CredentialsFileLock::acquire(dir)?;
    let result = action();
    let unlock_result = credentials_lock.unlock();
    match result {
        Err(err) => Err(err),
        Ok(value) => {
            unlock_result?;
            Ok(value)
        }
    }
}

fn prepare_config_dir(dir: &Path) -> PublicResult<()> {
    fs::create_dir_all(dir).map_err(|err| {
        PublicError::unexpected(format!("failed to create config directory: {err}"))
    })?;
    set_config_dir_permissions(dir)
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

#[cfg(unix)]
fn sync_config_dir(dir: &Path) -> PublicResult<()> {
    File::open(dir)
        .and_then(|file| file.sync_all())
        .map_err(|err| PublicError::unexpected(format!("failed to sync config directory: {err}")))
}

#[cfg(not(unix))]
fn sync_config_dir(_dir: &Path) -> PublicResult<()> {
    Ok(())
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
    if err.is_timeout() {
        PublicError::transport(TransportFailureKind::Timeout)
    } else if err.is_connect() {
        PublicError::transport(TransportFailureKind::Connect)
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
    async fn refresh_should_classify_connection_failures_without_exposing_request_context() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        drop(listener);
        let base_url = format!("http://{address}");
        let client = reqwest::Client::builder()
            .connect_timeout(StdDuration::from_secs(1))
            .timeout(StdDuration::from_secs(1))
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

        let temp = TempDir::new().expect("temp dir");
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
        let temp = TempDir::new().expect("temp dir");
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

        let temp = TempDir::new().expect("temp dir");
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
        let temp = TempDir::new().expect("temp dir");
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
        let temp = TempDir::new().expect("temp dir");
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
        let temp = TempDir::new().expect("temp dir");
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
    fn test_credentials_store_should_serialize_concurrent_replacements() {
        let temp = TempDir::new().expect("temp dir");
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
        let temp = TempDir::new().expect("temp dir");
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
