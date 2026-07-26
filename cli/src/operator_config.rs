use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sealtask_client_api::{
    DEFAULT_API_CONNECT_TIMEOUT as DEFAULT_CONNECT_TIMEOUT,
    DEFAULT_API_READ_TIMEOUT as DEFAULT_READ_TIMEOUT,
    DEFAULT_API_REQUEST_TIMEOUT as DEFAULT_REQUEST_TIMEOUT,
};
use sealtask_client_auth::{default_config_root, normalize_api_url};
use sealtask_client_core::{PublicError, PublicResult};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tempfile::NamedTempFile;

const OPERATOR_SETTINGS_FILE_NAME: &str = "operator-settings.json";
const OPERATOR_SETTINGS_LOCK_FILE_NAME: &str = "operator-settings.lock";
const OPERATOR_SETTINGS_SCHEMA_VERSION: u64 = 1;
const PROFILES_DIRECTORY_NAME: &str = "profiles";
const DEFAULT_PROFILE: &str = "default";
const DEFAULT_API_URL: &str = "https://sealtask.com";
const MAX_PROFILE_NAME_BYTES: usize = 64;
const MAX_API_URL_BYTES: usize = 2_048;
const MAX_PROFILE_SETTINGS: usize = 512;
const MAX_OPERATOR_SETTINGS_BYTES: u64 = 64 * 1024;
const OPERATOR_SETTINGS_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const OPERATOR_SETTINGS_LOCK_POLL_INTERVAL: Duration = Duration::from_millis(20);
const MIN_TIMEOUT: Duration = Duration::from_millis(1);
const MAX_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);

const API_URL_ENV: &str = "SEALTASK_API_URL";
const PROFILE_ENV: &str = "SEALTASK_PROFILE";
const CONFIG_DIR_ENV: &str = "SEALTASK_CONFIG_DIR";
const CONNECT_TIMEOUT_ENV: &str = "SEALTASK_CONNECT_TIMEOUT";
const READ_TIMEOUT_ENV: &str = "SEALTASK_READ_TIMEOUT";
const REQUEST_TIMEOUT_ENV: &str = "SEALTASK_REQUEST_TIMEOUT";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ValueSource {
    Cli,
    Environment,
    Persisted,
    Default,
}

impl ValueSource {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Environment => "environment",
            Self::Persisted => "persisted",
            Self::Default => "default",
        }
    }
}

impl std::fmt::Display for ValueSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedValue<T> {
    pub(crate) value: T,
    pub(crate) source: ValueSource,
}

impl<T> ResolvedValue<T> {
    const fn new(value: T, source: ValueSource) -> Self {
        Self { value, source }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct OperatorOverrides {
    pub(crate) api_url: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) config_dir: Option<PathBuf>,
    pub(crate) connect_timeout: Option<Duration>,
    pub(crate) read_timeout: Option<Duration>,
    pub(crate) request_timeout: Option<Duration>,
}

impl OperatorOverrides {
    fn from_environment_for(cli: &Self) -> PublicResult<Self> {
        Ok(Self {
            api_url: cli
                .api_url
                .is_none()
                .then(|| read_unicode_environment(API_URL_ENV))
                .transpose()?
                .flatten(),
            profile: cli
                .profile
                .is_none()
                .then(|| read_unicode_environment(PROFILE_ENV))
                .transpose()?
                .flatten(),
            config_dir: cli
                .config_dir
                .is_none()
                .then(|| {
                    env::var_os(CONFIG_DIR_ENV)
                        .filter(|value| !value.is_empty())
                        .map(PathBuf::from)
                })
                .flatten(),
            connect_timeout: cli
                .connect_timeout
                .is_none()
                .then(|| read_timeout_environment(CONNECT_TIMEOUT_ENV))
                .transpose()?
                .flatten(),
            read_timeout: cli
                .read_timeout
                .is_none()
                .then(|| read_timeout_environment(READ_TIMEOUT_ENV))
                .transpose()?
                .flatten(),
            request_timeout: cli
                .request_timeout
                .is_none()
                .then(|| read_timeout_environment(REQUEST_TIMEOUT_ENV))
                .transpose()?
                .flatten(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedOperatorConfig {
    pub(crate) api_url: ResolvedValue<String>,
    pub(crate) profile: ResolvedValue<String>,
    pub(crate) config_dir: ResolvedValue<PathBuf>,
    pub(crate) connect_timeout: ResolvedValue<Duration>,
    pub(crate) read_timeout: ResolvedValue<Duration>,
    pub(crate) request_timeout: ResolvedValue<Duration>,
}

impl ResolvedOperatorConfig {
    pub(crate) fn uses_default_profile(&self) -> bool {
        self.profile.value == DEFAULT_PROFILE
    }

    pub(crate) fn profile_config_dir(&self) -> PathBuf {
        if self.uses_default_profile() {
            self.config_dir.value.clone()
        } else {
            self.config_dir
                .value
                .join(PROFILES_DIRECTORY_NAME)
                .join(&self.profile.value)
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) api_url: Option<String>,
    #[serde(
        default,
        rename = "connectTimeoutMs",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_duration_milliseconds",
        deserialize_with = "deserialize_optional_duration_milliseconds"
    )]
    pub(crate) connect_timeout: Option<Duration>,
    #[serde(
        default,
        rename = "readTimeoutMs",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_duration_milliseconds",
        deserialize_with = "deserialize_optional_duration_milliseconds"
    )]
    pub(crate) read_timeout: Option<Duration>,
    #[serde(
        default,
        rename = "requestTimeoutMs",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_duration_milliseconds",
        deserialize_with = "deserialize_optional_duration_milliseconds"
    )]
    pub(crate) request_timeout: Option<Duration>,
}

impl ProfileSettings {
    fn validate_and_normalize(&mut self) -> PublicResult<()> {
        if let Some(api_url) = self.api_url.take() {
            self.api_url = Some(validate_api_url(&api_url)?);
        }
        for (name, timeout) in [
            ("connect timeout", self.connect_timeout),
            ("read timeout", self.read_timeout),
            ("request timeout", self.request_timeout),
        ] {
            if let Some(timeout) = timeout {
                validate_timeout(timeout, name)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OperatorSettings {
    schema_version: u64,
    pub(crate) active_profile: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) profiles: BTreeMap<String, ProfileSettings>,
}

impl Default for OperatorSettings {
    fn default() -> Self {
        Self {
            schema_version: OPERATOR_SETTINGS_SCHEMA_VERSION,
            active_profile: DEFAULT_PROFILE.to_string(),
            profiles: BTreeMap::new(),
        }
    }
}

impl OperatorSettings {
    #[cfg(test)]
    pub(crate) const fn schema_version(&self) -> u64 {
        self.schema_version
    }

    pub(crate) fn profile_settings(&self, profile: &str) -> Option<&ProfileSettings> {
        self.profiles.get(profile)
    }

    fn validate_and_normalize(&mut self) -> PublicResult<()> {
        if self.schema_version != OPERATOR_SETTINGS_SCHEMA_VERSION {
            return Err(PublicError::validation(format!(
                "unsupported operator settings schema version {}",
                self.schema_version
            )));
        }
        if self.profiles.len() > MAX_PROFILE_SETTINGS {
            return Err(corrupt_settings_error());
        }

        let normalized_active_profile = validate_profile_name(&self.active_profile)?;
        if normalized_active_profile != self.active_profile {
            return Err(corrupt_settings_error());
        }

        for (profile, settings) in &mut self.profiles {
            let normalized_profile = validate_profile_name(profile)?;
            if normalized_profile != *profile {
                return Err(corrupt_settings_error());
            }
            settings.validate_and_normalize()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OperatorSettingsStore {
    base_dir: PathBuf,
}

impl OperatorSettingsStore {
    pub(crate) fn new(base_dir: impl Into<PathBuf>) -> PublicResult<Self> {
        let base_dir = base_dir.into();
        if base_dir.as_os_str().is_empty() {
            return Err(PublicError::validation(
                "the SealTask configuration directory cannot be empty",
            ));
        }
        let base_dir = std::path::absolute(&base_dir).map_err(|err| {
            PublicError::unexpected(format!(
                "failed to resolve configuration directory {}: {err}",
                base_dir.display()
            ))
        })?;
        Ok(Self { base_dir })
    }

    pub(crate) fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    #[cfg(test)]
    pub(crate) fn settings_path(&self) -> PathBuf {
        self.base_dir.join(OPERATOR_SETTINGS_FILE_NAME)
    }

    pub(crate) fn load(&self) -> PublicResult<OperatorSettings> {
        inspect_existing_base_directory(&self.base_dir)?;
        read_settings_unlocked(&self.base_dir).map(Option::unwrap_or_default)
    }

    pub(crate) fn update<T>(
        &self,
        mutate: impl FnOnce(&mut OperatorSettings) -> PublicResult<T>,
    ) -> PublicResult<T> {
        let settings_lock = SettingsFileLock::acquire(&self.base_dir)?;
        let result = (|| {
            let mut settings = read_settings_unlocked(&self.base_dir)?.unwrap_or_default();
            let output = mutate(&mut settings)?;
            settings.validate_and_normalize()?;
            save_settings_unlocked(&self.base_dir, &settings)?;
            Ok(output)
        })();
        finish_locked(settings_lock, result)
    }

    pub(crate) fn set_active_profile(&self, profile: &str) -> PublicResult<()> {
        let profile = validate_profile_name(profile)?;
        self.update(|settings| {
            let mut updated = settings.clone();
            updated.active_profile.clone_from(&profile);
            updated.profiles.entry(profile.clone()).or_default();
            updated.validate_and_normalize()?;
            self.ensure_profile_directory(&profile)?;
            *settings = updated;
            Ok(())
        })
    }

    #[cfg(test)]
    pub(crate) fn set_profile_settings(
        &self,
        profile: &str,
        mut profile_settings: ProfileSettings,
    ) -> PublicResult<()> {
        let profile = validate_profile_name(profile)?;
        profile_settings.validate_and_normalize()?;
        self.update(|settings| {
            let mut updated = settings.clone();
            updated.profiles.insert(profile.clone(), profile_settings);
            updated.validate_and_normalize()?;
            self.ensure_profile_directory(&profile)?;
            *settings = updated;
            Ok(())
        })
    }

    pub(crate) fn list_profiles(&self) -> PublicResult<Vec<String>> {
        if !inspect_existing_base_directory(&self.base_dir)? {
            return Ok(vec![DEFAULT_PROFILE.to_string()]);
        }
        let profiles_dir = self.base_dir.join(PROFILES_DIRECTORY_NAME);
        let metadata = match fs::symlink_metadata(&profiles_dir) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(vec![DEFAULT_PROFILE.to_string()]);
            }
            Err(err) => {
                return Err(PublicError::unexpected(format!(
                    "failed to inspect the SealTask profiles directory: {err}"
                )));
            }
        };
        validate_directory_metadata(&metadata, "SealTask profiles")?;

        let mut profiles = BTreeSet::from([DEFAULT_PROFILE.to_string()]);
        let entries = fs::read_dir(&profiles_dir).map_err(|err| {
            PublicError::unexpected(format!(
                "failed to read the SealTask profiles directory: {err}"
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to read an entry in the SealTask profiles directory: {err}"
                ))
            })?;
            let file_type = entry.file_type().map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to inspect profile entry {}: {err}",
                    entry.path().display()
                ))
            })?;
            if !file_type.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if let Ok(profile) = validate_profile_name(&name)
                && profile == name
            {
                profiles.insert(profile);
            }
        }
        Ok(profiles.into_iter().collect())
    }

    fn load_snapshot(&self) -> PublicResult<SettingsSnapshot> {
        inspect_existing_base_directory(&self.base_dir)?;
        read_settings_unlocked(&self.base_dir).map(|settings| match settings {
            Some(settings) => SettingsSnapshot {
                settings,
                persisted: true,
            },
            None => SettingsSnapshot {
                settings: OperatorSettings::default(),
                persisted: false,
            },
        })
    }

    fn ensure_profile_directory(&self, profile: &str) -> PublicResult<()> {
        prepare_private_directory(&self.base_dir, "SealTask configuration")?;
        if profile == DEFAULT_PROFILE {
            return Ok(());
        }
        let profiles_dir = self.base_dir.join(PROFILES_DIRECTORY_NAME);
        prepare_private_directory(&profiles_dir, "SealTask profiles")?;
        prepare_private_directory(
            &profiles_dir.join(profile),
            &format!("SealTask profile '{profile}'"),
        )
    }
}

struct SettingsFileLock {
    file: Option<File>,
}

impl SettingsFileLock {
    fn acquire(base_dir: &Path) -> PublicResult<Self> {
        prepare_private_directory(base_dir, "SealTask configuration")?;
        let path = base_dir.join(OPERATOR_SETTINGS_LOCK_FILE_NAME);
        reject_symlink(&path, "operator settings lock")?;

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|err| {
                PublicError::unexpected(format!("failed to open the operator settings lock: {err}"))
            })?;
        if !file
            .metadata()
            .map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to inspect the operator settings lock: {err}"
                ))
            })?
            .is_file()
        {
            return Err(PublicError::validation(
                "the operator settings lock is not a regular file",
            ));
        }
        set_secret_file_permissions(&file, &path, "operator settings lock")?;

        lock_exclusive_with_timeout(&file, OPERATOR_SETTINGS_LOCK_TIMEOUT)?;

        Ok(Self { file: Some(file) })
    }

    fn unlock(mut self) -> PublicResult<()> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        fs2::FileExt::unlock(&file).map_err(|err| {
            PublicError::unexpected(format!("failed to unlock the operator settings: {err}"))
        })
    }
}

fn lock_exclusive_with_timeout(file: &File, timeout: Duration) -> PublicResult<()> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| PublicError::unexpected("operator settings lock deadline overflowed"))?;
    loop {
        match fs2::FileExt::try_lock_exclusive(file) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                let now = Instant::now();
                if now >= deadline {
                    return Err(PublicError::request_timeout(
                        "timed out waiting for the operator settings lock; retry after the other SealTask command finishes",
                    ));
                }
                std::thread::sleep(
                    OPERATOR_SETTINGS_LOCK_POLL_INTERVAL
                        .min(deadline.saturating_duration_since(now)),
                );
            }
            Err(error) => {
                return Err(PublicError::unexpected(format!(
                    "failed to lock the operator settings: {error}"
                )));
            }
        }
    }
}

impl Drop for SettingsFileLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = fs2::FileExt::unlock(&file);
        }
    }
}

struct SettingsSnapshot {
    settings: OperatorSettings,
    persisted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsReadPolicy {
    Strict,
    FallBackToDefaults,
}

pub(crate) fn resolve_operator_config(
    cli: OperatorOverrides,
) -> PublicResult<ResolvedOperatorConfig> {
    resolve_operator_config_from_process(cli, SettingsReadPolicy::Strict)
}

pub(crate) fn resolve_operator_config_for_diagnostics(
    cli: OperatorOverrides,
) -> PublicResult<ResolvedOperatorConfig> {
    resolve_operator_config_from_process(cli, SettingsReadPolicy::FallBackToDefaults)
}

fn resolve_operator_config_from_process(
    cli: OperatorOverrides,
    settings_read_policy: SettingsReadPolicy,
) -> PublicResult<ResolvedOperatorConfig> {
    let environment = OperatorOverrides::from_environment_for(&cli)?;
    let default_config_dir = default_base_config_dir()?;
    resolve_operator_config_with(cli, environment, default_config_dir, settings_read_policy)
}

pub(crate) fn parse_timeout(raw: &str) -> Result<Duration, String> {
    parse_human_duration(raw, "timeout")
}

pub(crate) fn parse_human_duration(raw: &str, description: &str) -> Result<Duration, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(duration_format_error(description));
    }

    let bytes = normalized.as_bytes();
    let mut index = 0;
    let mut total_milliseconds = 0_u128;
    while index < bytes.len() {
        let number_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == number_start {
            return Err(duration_format_error(description));
        }
        let value = normalized[number_start..index]
            .parse::<u128>()
            .map_err(|_| duration_format_error(description))?;

        let multiplier = if normalized[index..].starts_with("ms") {
            index += 2;
            1_u128
        } else if index < bytes.len() {
            let multiplier = match bytes[index] {
                b's' => 1_000_u128,
                b'm' => 60 * 1_000_u128,
                b'h' => 60 * 60 * 1_000_u128,
                _ => return Err(duration_format_error(description)),
            };
            index += 1;
            multiplier
        } else {
            return Err(duration_format_error(description));
        };

        let segment = value
            .checked_mul(multiplier)
            .ok_or_else(|| duration_range_error(description))?;
        total_milliseconds = total_milliseconds
            .checked_add(segment)
            .ok_or_else(|| duration_range_error(description))?;
        if total_milliseconds > MAX_TIMEOUT.as_millis() {
            return Err(duration_range_error(description));
        }
    }

    let milliseconds =
        u64::try_from(total_milliseconds).map_err(|_| duration_range_error(description))?;
    let timeout = Duration::from_millis(milliseconds);
    validate_timeout(timeout, description).map_err(|error| error.to_string())?;
    Ok(timeout)
}

pub(crate) fn validate_profile_name(profile: &str) -> PublicResult<String> {
    let profile = profile.trim();
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

fn resolve_operator_config_with(
    cli: OperatorOverrides,
    environment: OperatorOverrides,
    default_config_dir: PathBuf,
    settings_read_policy: SettingsReadPolicy,
) -> PublicResult<ResolvedOperatorConfig> {
    let config_dir = resolve_value(
        cli.config_dir,
        environment.config_dir,
        None,
        default_config_dir,
    );
    let store = OperatorSettingsStore::new(&config_dir.value)?;
    let config_dir = ResolvedValue::new(store.base_dir().to_path_buf(), config_dir.source);
    let snapshot = match store.load_snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) if settings_read_policy == SettingsReadPolicy::FallBackToDefaults => {
            SettingsSnapshot {
                settings: OperatorSettings::default(),
                persisted: false,
            }
        }
        Err(error) => return Err(error),
    };

    let persisted_profile = snapshot
        .persisted
        .then(|| snapshot.settings.active_profile.clone());
    let mut profile = resolve_value(
        cli.profile,
        environment.profile,
        persisted_profile,
        DEFAULT_PROFILE.to_string(),
    );
    profile.value = validate_profile_name(&profile.value)?;

    let persisted = snapshot.settings.profile_settings(&profile.value);
    let persisted_api_url = persisted.and_then(|settings| settings.api_url.clone());
    let mut api_url = resolve_value(
        cli.api_url,
        environment.api_url,
        persisted_api_url,
        DEFAULT_API_URL.to_string(),
    );
    api_url.value = validate_api_url(&api_url.value)?;

    let connect_timeout = resolve_value(
        cli.connect_timeout,
        environment.connect_timeout,
        persisted.and_then(|settings| settings.connect_timeout),
        DEFAULT_CONNECT_TIMEOUT,
    );
    validate_timeout(connect_timeout.value, "connect timeout")?;

    let read_timeout = resolve_value(
        cli.read_timeout,
        environment.read_timeout,
        persisted.and_then(|settings| settings.read_timeout),
        DEFAULT_READ_TIMEOUT,
    );
    validate_timeout(read_timeout.value, "read timeout")?;

    let request_timeout = resolve_value(
        cli.request_timeout,
        environment.request_timeout,
        persisted.and_then(|settings| settings.request_timeout),
        DEFAULT_REQUEST_TIMEOUT,
    );
    validate_timeout(request_timeout.value, "request timeout")?;

    Ok(ResolvedOperatorConfig {
        api_url,
        profile,
        config_dir,
        connect_timeout,
        read_timeout,
        request_timeout,
    })
}

fn resolve_value<T>(
    cli: Option<T>,
    environment: Option<T>,
    persisted: Option<T>,
    default: T,
) -> ResolvedValue<T> {
    if let Some(value) = cli {
        ResolvedValue::new(value, ValueSource::Cli)
    } else if let Some(value) = environment {
        ResolvedValue::new(value, ValueSource::Environment)
    } else if let Some(value) = persisted {
        ResolvedValue::new(value, ValueSource::Persisted)
    } else {
        ResolvedValue::new(default, ValueSource::Default)
    }
}

fn default_base_config_dir() -> PublicResult<PathBuf> {
    default_config_root()
}

fn read_unicode_environment(name: &str) -> PublicResult<Option<String>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(PublicError::validation(format!(
            "{name} must contain valid Unicode"
        ))),
    }
}

fn read_timeout_environment(name: &str) -> PublicResult<Option<Duration>> {
    read_unicode_environment(name)?
        .map(|raw| {
            parse_timeout(&raw)
                .map_err(|error| PublicError::validation(format!("invalid {name}: {error}")))
        })
        .transpose()
}

fn validate_api_url(raw: &str) -> PublicResult<String> {
    let normalized = normalize_api_url(raw.trim());
    if normalized.is_empty() || normalized.len() > MAX_API_URL_BYTES {
        return Err(PublicError::validation(
            "API URL must be a non-empty HTTP(S) URL of at most 2048 bytes",
        ));
    }
    let parsed = reqwest::Url::parse(&normalized)
        .map_err(|_| PublicError::validation("API URL must be a valid HTTP(S) URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(PublicError::validation(
            "API URL must use HTTP or HTTPS and include a host",
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(PublicError::validation(
            "API URL must not contain embedded credentials",
        ));
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(PublicError::validation(
            "API URL must not contain a query string or fragment",
        ));
    }
    Ok(normalized)
}

fn validate_timeout(timeout: Duration, description: &str) -> PublicResult<()> {
    if timeout < MIN_TIMEOUT || timeout > MAX_TIMEOUT {
        return Err(PublicError::validation(format!(
            "{description} must be between 1ms and 24h"
        )));
    }
    Ok(())
}

fn duration_format_error(description: &str) -> String {
    format!(
        "{description} must be a positive duration using ms, s, m, or h (for example 500ms, 30s, or 1m30s)"
    )
}

fn duration_range_error(description: &str) -> String {
    format!("{description} must be between 1ms and 24h")
}

fn serialize_optional_duration_milliseconds<S>(
    value: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(duration) => {
            let milliseconds =
                u64::try_from(duration.as_millis()).map_err(serde::ser::Error::custom)?;
            serializer.serialize_some(&milliseconds)
        }
        None => serializer.serialize_none(),
    }
}

fn deserialize_optional_duration_milliseconds<'de, D>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let milliseconds = Option::<u64>::deserialize(deserializer)?;
    milliseconds
        .map(|milliseconds| {
            let duration = Duration::from_millis(milliseconds);
            validate_timeout(duration, "persisted timeout")
                .map(|()| duration)
                .map_err(serde::de::Error::custom)
        })
        .transpose()
}

fn read_settings_unlocked(base_dir: &Path) -> PublicResult<Option<OperatorSettings>> {
    let path = base_dir.join(OPERATOR_SETTINGS_FILE_NAME);
    let Some(mut file) = open_settings_file(&path)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_OPERATOR_SETTINGS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| {
            PublicError::unexpected(format!("failed to read operator settings: {err}"))
        })?;
    if bytes.len() as u64 > MAX_OPERATOR_SETTINGS_BYTES {
        return Err(PublicError::validation(
            "the operator settings file is too large and may be corrupt",
        ));
    }
    decode_settings(&bytes).map(Some)
}

fn open_settings_file(path: &Path) -> PublicResult<Option<File>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to inspect the operator settings file: {err}"
            )));
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(PublicError::validation(
            "the operator settings file must not be a symbolic link",
        ));
    }
    if !metadata.is_file() {
        return Err(PublicError::validation(
            "the operator settings path is not a regular file",
        ));
    }
    if metadata.len() > MAX_OPERATOR_SETTINGS_BYTES {
        return Err(PublicError::validation(
            "the operator settings file is too large and may be corrupt",
        ));
    }
    validate_secret_file_permissions(&metadata)?;

    File::open(path)
        .map(Some)
        .map_err(|err| PublicError::unexpected(format!("failed to open operator settings: {err}")))
}

fn decode_settings(bytes: &[u8]) -> PublicResult<OperatorSettings> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| corrupt_settings_error())?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(corrupt_settings_error)?;
    if schema_version > OPERATOR_SETTINGS_SCHEMA_VERSION {
        return Err(PublicError::validation(format!(
            "the operator settings use schema version {schema_version}, but this CLI supports version {OPERATOR_SETTINGS_SCHEMA_VERSION}; upgrade SealTask before changing these settings"
        )));
    }
    if schema_version != OPERATOR_SETTINGS_SCHEMA_VERSION {
        return Err(PublicError::validation(format!(
            "unsupported operator settings schema version {schema_version}"
        )));
    }

    let mut settings: OperatorSettings =
        serde_json::from_value(value).map_err(|_| corrupt_settings_error())?;
    settings.validate_and_normalize()?;
    Ok(settings)
}

fn save_settings_unlocked(base_dir: &Path, settings: &OperatorSettings) -> PublicResult<()> {
    let path = base_dir.join(OPERATOR_SETTINGS_FILE_NAME);
    validate_replaceable_settings_path(&path)?;

    let mut temporary = NamedTempFile::new_in(base_dir).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to create a temporary operator settings file: {err}"
        ))
    })?;
    set_secret_file_permissions(
        temporary.as_file(),
        temporary.path(),
        "temporary operator settings file",
    )?;
    serde_json::to_writer_pretty(&mut temporary, settings).map_err(|err| {
        PublicError::unexpected(format!("failed to serialize operator settings: {err}"))
    })?;
    temporary.write_all(b"\n").map_err(|err| {
        PublicError::unexpected(format!(
            "failed to finish the operator settings file: {err}"
        ))
    })?;
    temporary.as_file().sync_all().map_err(|err| {
        PublicError::unexpected(format!("failed to sync the operator settings file: {err}"))
    })?;
    let persisted = temporary.persist(&path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to atomically replace the operator settings: {}",
            err.error
        ))
    })?;
    set_secret_file_permissions(&persisted, &path, "operator settings file")?;
    persisted.sync_all().map_err(|err| {
        PublicError::unexpected(format!("failed to sync the operator settings file: {err}"))
    })?;
    sync_directory(base_dir)
}

fn validate_replaceable_settings_path(path: &Path) -> PublicResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(PublicError::validation(
            "the operator settings file must not be a symbolic link",
        )),
        Ok(metadata) if !metadata.is_file() => Err(PublicError::validation(
            "the operator settings path is not a regular file",
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(PublicError::unexpected(format!(
            "failed to inspect the operator settings file: {err}"
        ))),
    }
}

fn finish_locked<T>(settings_lock: SettingsFileLock, result: PublicResult<T>) -> PublicResult<T> {
    let unlock_result = settings_lock.unlock();
    match result {
        Err(error) => Err(error),
        Ok(value) => {
            unlock_result?;
            Ok(value)
        }
    }
}

fn corrupt_settings_error() -> PublicError {
    PublicError::validation(
        "the operator settings file is corrupt; repair or remove operator-settings.json",
    )
}

fn reject_symlink(path: &Path, description: &str) -> PublicResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(PublicError::validation(format!(
            "the {description} must not be a symbolic link"
        ))),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(PublicError::unexpected(format!(
            "failed to inspect the {description}: {err}"
        ))),
    }
}

fn inspect_existing_base_directory(path: &Path) -> PublicResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            validate_directory_metadata(&metadata, "SealTask configuration")?;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(PublicError::unexpected(format!(
            "failed to inspect the SealTask configuration directory: {err}"
        ))),
    }
}

fn prepare_private_directory(path: &Path, description: &str) -> PublicResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory_metadata(&metadata, description)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path).map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to create the {description} directory: {err}"
                ))
            })?;
            let metadata = fs::symlink_metadata(path).map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to inspect the {description} directory: {err}"
                ))
            })?;
            validate_directory_metadata(&metadata, description)?;
        }
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to inspect the {description} directory: {err}"
            )));
        }
    }
    set_private_directory_permissions(path, description)
}

fn validate_directory_metadata(metadata: &fs::Metadata, description: &str) -> PublicResult<()> {
    if metadata.file_type().is_symlink() {
        return Err(PublicError::validation(format!(
            "the {description} directory must not be a symbolic link"
        )));
    }
    if !metadata.is_dir() {
        return Err(PublicError::validation(format!(
            "the {description} path is not a directory"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path, description: &str) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to secure the {description} directory: {err}"
        ))
    })
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path, _description: &str) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_secret_file_permissions(file: &File, _path: &Path, description: &str) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|err| {
            PublicError::unexpected(format!("failed to secure the {description}: {err}"))
        })
}

#[cfg(not(unix))]
fn set_secret_file_permissions(_file: &File, _path: &Path, _description: &str) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn validate_secret_file_permissions(metadata: &fs::Metadata) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::validation(
            "the operator settings file permissions are too broad; expected mode 0600",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_secret_file_permissions(_metadata: &fs::Metadata) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> PublicResult<()> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|err| {
            PublicError::unexpected(format!(
                "failed to sync the SealTask configuration directory: {err}"
            ))
        })
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> PublicResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn overrides() -> OperatorOverrides {
        OperatorOverrides::default()
    }

    fn empty_environment() -> OperatorOverrides {
        OperatorOverrides::default()
    }

    fn store(temp: &TempDir) -> OperatorSettingsStore {
        OperatorSettingsStore::new(temp.path()).expect("settings store")
    }

    fn persisted_settings(
        api_url: &str,
        connect_timeout: Duration,
        read_timeout: Duration,
        request_timeout: Duration,
    ) -> ProfileSettings {
        ProfileSettings {
            api_url: Some(api_url.to_string()),
            connect_timeout: Some(connect_timeout),
            read_timeout: Some(read_timeout),
            request_timeout: Some(request_timeout),
        }
    }

    #[test]
    fn timeout_parser_accepts_supported_units_and_compounds() {
        assert_eq!(parse_timeout("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_timeout("10s").unwrap(), Duration::from_secs(10));
        assert_eq!(parse_timeout("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_timeout("1h").unwrap(), Duration::from_secs(3_600));
        assert_eq!(
            parse_timeout(" 1H30m5s250ms ").unwrap(),
            Duration::from_millis(5_405_250)
        );
    }

    #[test]
    fn timeout_parser_rejects_ambiguous_invalid_and_unsafe_values() {
        for invalid in [
            "", "0ms", "10", "1.5s", "1 second", "-1s", "ms", "1d", "24h1ms",
        ] {
            assert!(parse_timeout(invalid).is_err(), "{invalid} must be invalid");
        }
    }

    #[test]
    fn profile_validation_matches_auth_profile_rules() {
        for valid in ["default", "team-1", "local.dev", "snake_case", " A "] {
            assert!(
                validate_profile_name(valid).is_ok(),
                "{valid} must be valid"
            );
        }
        for invalid in ["", " ", ".", "..", "has/slash", "é", "has space"] {
            assert!(
                validate_profile_name(invalid).is_err(),
                "{invalid} must be invalid"
            );
        }
        assert!(validate_profile_name(&"a".repeat(64)).is_ok());
        assert!(validate_profile_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn settings_round_trip_is_versioned_normalized_and_private() {
        let temp = TempDir::new().expect("temporary directory");
        let store = store(&temp);
        store
            .set_profile_settings(
                "team",
                persisted_settings(
                    "https://api.example/",
                    Duration::from_secs(2),
                    Duration::from_secs(3),
                    Duration::from_secs(4),
                ),
            )
            .expect("persist profile settings");
        store
            .set_active_profile("team")
            .expect("persist active profile");

        let loaded = store.load().expect("load settings");
        assert_eq!(loaded.schema_version(), OPERATOR_SETTINGS_SCHEMA_VERSION);
        assert_eq!(loaded.active_profile, "team");
        assert_eq!(
            loaded.profile_settings("team").unwrap().api_url.as_deref(),
            Some("https://api.example")
        );
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(store.settings_path()).unwrap()).unwrap();
        assert_eq!(value["schemaVersion"], OPERATOR_SETTINGS_SCHEMA_VERSION);
        assert_eq!(value["profiles"]["team"]["connectTimeoutMs"], 2_000);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(store.settings_path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(store.base_dir()).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn resolution_obeys_cli_environment_persisted_default_precedence() {
        let temp = TempDir::new().expect("temporary directory");
        let store = store(&temp);
        store
            .set_profile_settings(
                "persisted",
                persisted_settings(
                    "https://persisted.example/",
                    Duration::from_secs(11),
                    Duration::from_secs(12),
                    Duration::from_secs(13),
                ),
            )
            .unwrap();
        store.set_active_profile("persisted").unwrap();

        let resolved = resolve_operator_config_with(
            overrides(),
            empty_environment(),
            temp.path().into(),
            SettingsReadPolicy::Strict,
        )
        .unwrap();
        assert_eq!(resolved.profile.value, "persisted");
        assert_eq!(resolved.profile.source, ValueSource::Persisted);
        assert_eq!(resolved.api_url.value, "https://persisted.example");
        assert_eq!(resolved.api_url.source, ValueSource::Persisted);
        assert_eq!(resolved.connect_timeout.value, Duration::from_secs(11));
        assert_eq!(resolved.connect_timeout.source, ValueSource::Persisted);
        assert_eq!(resolved.config_dir.source, ValueSource::Default);

        let environment = OperatorOverrides {
            api_url: Some("https://environment.example/".to_string()),
            profile: Some("persisted".to_string()),
            connect_timeout: Some(Duration::from_secs(21)),
            ..overrides()
        };
        let cli = OperatorOverrides {
            api_url: Some("https://cli.example/".to_string()),
            profile: Some("persisted".to_string()),
            read_timeout: Some(Duration::from_secs(22)),
            ..overrides()
        };
        let resolved = resolve_operator_config_with(
            cli,
            environment,
            temp.path().into(),
            SettingsReadPolicy::Strict,
        )
        .unwrap();
        assert_eq!(resolved.api_url.value, "https://cli.example");
        assert_eq!(resolved.api_url.source, ValueSource::Cli);
        assert_eq!(resolved.connect_timeout.value, Duration::from_secs(21));
        assert_eq!(resolved.connect_timeout.source, ValueSource::Environment);
        assert_eq!(resolved.read_timeout.value, Duration::from_secs(22));
        assert_eq!(resolved.read_timeout.source, ValueSource::Cli);
        assert_eq!(resolved.request_timeout.value, Duration::from_secs(13));
        assert_eq!(resolved.request_timeout.source, ValueSource::Persisted);
    }

    #[test]
    fn resolution_reports_defaults_when_no_settings_exist() {
        let temp = TempDir::new().expect("temporary directory");
        let resolved = resolve_operator_config_with(
            overrides(),
            empty_environment(),
            temp.path().into(),
            SettingsReadPolicy::Strict,
        )
        .unwrap();

        assert_eq!(resolved.api_url.value, DEFAULT_API_URL);
        assert_eq!(resolved.api_url.source, ValueSource::Default);
        assert_eq!(resolved.profile.value, DEFAULT_PROFILE);
        assert_eq!(resolved.profile.source, ValueSource::Default);
        assert_eq!(resolved.connect_timeout.value, DEFAULT_CONNECT_TIMEOUT);
        assert_eq!(resolved.read_timeout.value, DEFAULT_READ_TIMEOUT);
        assert_eq!(resolved.request_timeout.value, DEFAULT_REQUEST_TIMEOUT);
        assert_eq!(resolved.profile_config_dir(), store(&temp).base_dir());
    }

    #[test]
    fn profile_listing_is_sorted_deduplicated_and_directory_only() {
        let temp = TempDir::new().expect("temporary directory");
        let store = store(&temp);
        let profiles = store.base_dir().join(PROFILES_DIRECTORY_NAME);
        fs::create_dir_all(profiles.join("zeta")).unwrap();
        fs::create_dir_all(profiles.join("alpha")).unwrap();
        fs::create_dir_all(profiles.join("has space")).unwrap();
        fs::write(profiles.join("regular-file"), b"not a profile").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(profiles.join("alpha"), profiles.join("linked")).unwrap();

        assert_eq!(
            store.list_profiles().unwrap(),
            vec![
                "alpha".to_string(),
                "default".to_string(),
                "zeta".to_string()
            ]
        );
    }

    #[test]
    fn settings_lock_wait_is_bounded_and_actionable() {
        let temp = TempDir::new().expect("temporary directory");
        let path = temp.path().join("contended.lock");
        let first = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let second = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        fs2::FileExt::lock_exclusive(&first).unwrap();

        let started = Instant::now();
        let error = lock_exclusive_with_timeout(&second, Duration::from_millis(25))
            .expect_err("contended lock must time out");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(error.code(), "request_timeout");
        assert!(error.to_string().contains("retry"));
        fs2::FileExt::unlock(&first).unwrap();
    }

    #[test]
    fn future_schema_is_rejected_and_never_overwritten() {
        let temp = TempDir::new().expect("temporary directory");
        let store = store(&temp);
        prepare_private_directory(store.base_dir(), "test").unwrap();
        let future = br#"{"schemaVersion":2,"futureOnly":true}"#;
        fs::write(store.settings_path(), future).unwrap();
        secure_fixture(&store.settings_path());
        let before = fs::read(store.settings_path()).unwrap();

        let load_error = store.load().expect_err("future schema must fail");
        let update_error = store
            .set_active_profile("other")
            .expect_err("future schema must not be replaced");

        assert!(load_error.to_string().contains("upgrade SealTask"));
        assert!(update_error.to_string().contains("upgrade SealTask"));
        assert_eq!(fs::read(store.settings_path()).unwrap(), before);
        assert!(
            !store
                .base_dir()
                .join(PROFILES_DIRECTORY_NAME)
                .join("other")
                .exists()
        );
    }

    #[cfg(unix)]
    #[test]
    fn settings_symlinks_and_broad_permissions_are_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().expect("temporary directory");
        let store = store(&temp);
        prepare_private_directory(store.base_dir(), "test").unwrap();
        let target = temp.path().join("target.json");
        fs::write(&target, br#"{"schemaVersion":1,"activeProfile":"default"}"#).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        std::os::unix::fs::symlink(&target, store.settings_path()).unwrap();
        assert!(
            store
                .load()
                .unwrap_err()
                .to_string()
                .contains("symbolic link")
        );

        fs::remove_file(store.settings_path()).unwrap();
        fs::write(
            store.settings_path(),
            br#"{"schemaVersion":1,"activeProfile":"default"}"#,
        )
        .unwrap();
        fs::set_permissions(store.settings_path(), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            store
                .load()
                .unwrap_err()
                .to_string()
                .contains("permissions are too broad")
        );
    }

    #[cfg(unix)]
    fn secure_fixture(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    #[cfg(not(unix))]
    fn secure_fixture(_path: &Path) {}
}
