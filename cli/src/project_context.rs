use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;

use sealtask_client_auth::{Credentials, config_dir, load_credentials_for_url, normalize_api_url};
use sealtask_client_core::{PublicError, PublicResult};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use uuid::Uuid;

const CONTEXT_FILE_NAME: &str = "context.json";
const CONTEXT_LOCK_FILE_NAME: &str = "context.lock";
const CONTEXT_SCHEMA_VERSION: u64 = 1;
const MAX_CONTEXT_FILE_BYTES: u64 = 16 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredProjectContext {
    schema_version: u64,
    api_url: String,
    user_id: Uuid,
    project_id: Uuid,
}

impl StoredProjectContext {
    fn new(credentials: &Credentials, project_id: Uuid) -> PublicResult<Self> {
        let api_url = normalize_api_url(&credentials.api_url);
        if api_url.is_empty() {
            return Err(PublicError::validation(
                "cannot save the current project for an empty API URL",
            ));
        }
        if project_id.is_nil() {
            return Err(PublicError::validation(
                "cannot save a nil project ID as the current project",
            ));
        }

        Ok(Self {
            schema_version: CONTEXT_SCHEMA_VERSION,
            api_url,
            user_id: credentials.user_id,
            project_id,
        })
    }

    fn validate_binding(&self, credentials: &Credentials) -> PublicResult<()> {
        if normalize_api_url(&self.api_url) != normalize_api_url(&credentials.api_url) {
            return Err(PublicError::validation(
                "the saved current project belongs to a different API; select a project again",
            ));
        }
        if self.user_id != credentials.user_id {
            return Err(PublicError::validation(
                "the saved current project belongs to a different account; select a project again",
            ));
        }
        if self.project_id.is_nil() {
            return Err(corrupt_context_error());
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum LockMode {
    Shared,
    Exclusive,
}

enum ContextReadError {
    FutureSchema(u64),
    Other(PublicError),
}

impl ContextReadError {
    fn into_public_error(self) -> PublicError {
        match self {
            Self::FutureSchema(schema_version) => future_context_error(schema_version),
            Self::Other(error) => error,
        }
    }
}

struct ContextFileLock {
    file: Option<File>,
}

impl ContextFileLock {
    fn acquire(dir: &Path, mode: LockMode) -> PublicResult<Self> {
        prepare_context_dir(dir)?;
        let path = dir.join(CONTEXT_LOCK_FILE_NAME);
        reject_symlink(&path, "project context lock")?;

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|err| {
                PublicError::unexpected(format!("failed to open the project context lock: {err}"))
            })?;
        if !file
            .metadata()
            .map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to inspect the project context lock: {err}"
                ))
            })?
            .is_file()
        {
            return Err(PublicError::validation(
                "the project context lock is not a regular file",
            ));
        }
        set_secret_file_permissions(&path)?;

        let result = match mode {
            LockMode::Shared => fs2::FileExt::lock_shared(&file),
            LockMode::Exclusive => fs2::FileExt::lock_exclusive(&file),
        };
        result.map_err(|err| {
            PublicError::unexpected(format!("failed to lock the project context: {err}"))
        })?;

        Ok(Self { file: Some(file) })
    }

    fn unlock(mut self) -> PublicResult<()> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        fs2::FileExt::unlock(&file).map_err(|err| {
            PublicError::unexpected(format!("failed to unlock the project context: {err}"))
        })
    }
}

impl Drop for ContextFileLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = fs2::FileExt::unlock(&file);
        }
    }
}

/// Loads the current project for the active profile and API target.
///
/// A saved context is usable only when credentials for `api_url` are present
/// and its account/API binding matches those credentials.
pub(crate) fn load_current_project(api_url: &str) -> PublicResult<Option<Uuid>> {
    let credentials = load_credentials_for_url(api_url)?.ok_or_else(|| {
        PublicError::validation(
            "not logged in for the current API; authenticate before using saved project context",
        )
    })?;
    load_current_project_in(&config_dir()?, &credentials)
}

/// Saves the current project for the active profile and API target.
///
/// Only the schema version, normalized API URL, account ID, and project ID are
/// persisted. Decrypted project names and other project content never reach
/// this file.
pub(crate) fn save_current_project(api_url: &str, project_id: Uuid) -> PublicResult<bool> {
    let credentials = load_credentials_for_url(api_url)?.ok_or_else(|| {
        PublicError::validation(
            "not logged in for the current API; authenticate before selecting a project",
        )
    })?;
    select_current_project_in(&config_dir()?, &credentials, project_id)
}

/// Clears the active profile's saved current project.
///
/// Returning `false` means no context existed. Repeated clears are successful.
pub(crate) fn clear_current_project() -> PublicResult<bool> {
    clear_current_project_in(&config_dir()?)
}

fn load_current_project_in(dir: &Path, credentials: &Credentials) -> PublicResult<Option<Uuid>> {
    let context_lock = ContextFileLock::acquire(dir, LockMode::Shared)?;
    let result = load_current_project_unlocked(dir, credentials);
    finish_locked(context_lock, result)
}

fn load_current_project_unlocked(
    dir: &Path,
    credentials: &Credentials,
) -> PublicResult<Option<Uuid>> {
    let Some(context) =
        read_current_project_unlocked(dir).map_err(ContextReadError::into_public_error)?
    else {
        return Ok(None);
    };
    context.validate_binding(credentials)?;
    Ok(Some(context.project_id))
}

fn read_current_project_unlocked(
    dir: &Path,
) -> Result<Option<StoredProjectContext>, ContextReadError> {
    let path = dir.join(CONTEXT_FILE_NAME);
    let Some(file) = open_context_file(&path).map_err(ContextReadError::Other)? else {
        return Ok(None);
    };
    decode_context(BufReader::new(file)).map(Some)
}

#[cfg(test)]
fn save_current_project_in(
    dir: &Path,
    credentials: &Credentials,
    project_id: Uuid,
) -> PublicResult<()> {
    let context = StoredProjectContext::new(credentials, project_id)?;
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = save_current_project_unlocked(dir, &context);
    finish_locked(context_lock, result)
}

fn select_current_project_in(
    dir: &Path,
    credentials: &Credentials,
    project_id: Uuid,
) -> PublicResult<bool> {
    let context = StoredProjectContext::new(credentials, project_id)?;
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = match read_current_project_unlocked(dir) {
        Ok(Some(current))
            if current.validate_binding(credentials).is_ok()
                && current.project_id == project_id =>
        {
            Ok(false)
        }
        Err(ContextReadError::FutureSchema(schema_version)) => {
            Err(future_context_error(schema_version))
        }
        Ok(_) | Err(ContextReadError::Other(_)) => {
            save_current_project_unlocked(dir, &context).map(|()| true)
        }
    };
    finish_locked(context_lock, result)
}

fn save_current_project_unlocked(dir: &Path, context: &StoredProjectContext) -> PublicResult<()> {
    let path = dir.join(CONTEXT_FILE_NAME);
    let mut temporary = NamedTempFile::new_in(dir).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to create a temporary project context file: {err}"
        ))
    })?;
    set_secret_file_permissions(temporary.path())?;
    serde_json::to_writer_pretty(&mut temporary, context).map_err(|err| {
        PublicError::unexpected(format!("failed to serialize the project context: {err}"))
    })?;
    temporary.write_all(b"\n").map_err(|err| {
        PublicError::unexpected(format!("failed to finish the project context file: {err}"))
    })?;
    temporary.as_file().sync_all().map_err(|err| {
        PublicError::unexpected(format!("failed to sync the project context file: {err}"))
    })?;
    temporary.persist(&path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to atomically replace the project context: {}",
            err.error
        ))
    })?;
    set_secret_file_permissions(&path)?;
    sync_context_dir(dir)
}

fn clear_current_project_in(dir: &Path) -> PublicResult<bool> {
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = clear_current_project_unlocked(dir);
    finish_locked(context_lock, result)
}

fn clear_current_project_unlocked(dir: &Path) -> PublicResult<bool> {
    let path = dir.join(CONTEXT_FILE_NAME);
    match fs::remove_file(&path) {
        Ok(()) => {
            sync_context_dir(dir)?;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(PublicError::unexpected(format!(
            "failed to clear the project context: {err}"
        ))),
    }
}

fn open_context_file(path: &Path) -> PublicResult<Option<File>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to inspect the project context file: {err}"
            )));
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(PublicError::validation(
            "the project context file must not be a symbolic link",
        ));
    }
    if !metadata.is_file() {
        return Err(PublicError::validation(
            "the project context path is not a regular file",
        ));
    }
    if metadata.len() > MAX_CONTEXT_FILE_BYTES {
        return Err(PublicError::validation(
            "the saved project context is too large and may be corrupt",
        ));
    }
    validate_secret_file_permissions(&metadata)?;

    File::open(path)
        .map(Some)
        .map_err(|err| PublicError::unexpected(format!("failed to open project context: {err}")))
}

fn decode_context(reader: impl std::io::Read) -> Result<StoredProjectContext, ContextReadError> {
    let value: serde_json::Value = serde_json::from_reader(reader)
        .map_err(|_| ContextReadError::Other(corrupt_context_error()))?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ContextReadError::Other(corrupt_context_error()))?;
    if schema_version > CONTEXT_SCHEMA_VERSION {
        return Err(ContextReadError::FutureSchema(schema_version));
    }
    if schema_version != CONTEXT_SCHEMA_VERSION {
        return Err(ContextReadError::Other(PublicError::validation(format!(
            "unsupported project context schema version {schema_version}"
        ))));
    }

    serde_json::from_value(value).map_err(|_| ContextReadError::Other(corrupt_context_error()))
}

fn finish_locked<T>(context_lock: ContextFileLock, result: PublicResult<T>) -> PublicResult<T> {
    let unlock_result = context_lock.unlock();
    match result {
        Err(err) => Err(err),
        Ok(value) => {
            unlock_result?;
            Ok(value)
        }
    }
}

fn corrupt_context_error() -> PublicError {
    PublicError::validation(
        "the saved project context is corrupt; clear it and select a project again",
    )
}

fn future_context_error(schema_version: u64) -> PublicError {
    PublicError::validation(format!(
        "the saved project context uses schema version {schema_version}, but this CLI supports version {CONTEXT_SCHEMA_VERSION}; upgrade SealTask or clear the saved project"
    ))
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

fn prepare_context_dir(dir: &Path) -> PublicResult<()> {
    fs::create_dir_all(dir).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to create the project context directory: {err}"
        ))
    })?;
    set_context_dir_permissions(dir)
}

#[cfg(unix)]
fn set_context_dir_permissions(dir: &Path) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to set project context directory permissions: {err}"
        ))
    })
}

#[cfg(not(unix))]
fn set_context_dir_permissions(_dir: &Path) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_secret_file_permissions(path: &Path) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to set project context file permissions: {err}"
        ))
    })
}

#[cfg(not(unix))]
fn set_secret_file_permissions(_path: &Path) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn validate_secret_file_permissions(metadata: &fs::Metadata) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::validation(
            "the project context file permissions are too broad; expected mode 0600",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_secret_file_permissions(_metadata: &fs::Metadata) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn sync_context_dir(dir: &Path) -> PublicResult<()> {
    File::open(dir)
        .and_then(|file| file.sync_all())
        .map_err(|err| {
            PublicError::unexpected(format!(
                "failed to sync the project context directory: {err}"
            ))
        })
}

#[cfg(not(unix))]
fn sync_context_dir(_dir: &Path) -> PublicResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;

    fn credentials(api_url: &str, user_id: Uuid) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "access-token-secret".to_string(),
            refresh_token: "refresh-token-secret".to_string(),
            access_expires_at: Utc::now() + Duration::hours(1),
            refresh_expires_at: Utc::now() + Duration::hours(2),
            user_id,
            email: "private-email@example.com".to_string(),
            data_key_ciphertext: "private-data-key-ciphertext".to_string(),
        }
    }

    fn write_context_fixture(dir: &Path, contents: &[u8]) {
        prepare_context_dir(dir).expect("prepare context directory");
        let path = dir.join(CONTEXT_FILE_NAME);
        fs::write(&path, contents).expect("write project context fixture");
        set_secret_file_permissions(&path).expect("secure project context fixture");
    }

    #[test]
    fn context_round_trip_persists_only_binding_and_project_id() {
        let temp = TempDir::new().expect("temporary directory");
        let user_id = Uuid::from_u128(1);
        let project_id = Uuid::from_u128(2);
        let credentials = credentials("https://api.example/", user_id);

        save_current_project_in(temp.path(), &credentials, project_id)
            .expect("save current project");

        assert_eq!(
            load_current_project_in(temp.path(), &credentials).expect("load current project"),
            Some(project_id)
        );
        let persisted =
            fs::read_to_string(temp.path().join(CONTEXT_FILE_NAME)).expect("read context");
        let value: serde_json::Value =
            serde_json::from_str(&persisted).expect("parse persisted context");
        assert_eq!(value["schemaVersion"], CONTEXT_SCHEMA_VERSION);
        assert_eq!(value["apiUrl"], "https://api.example");
        assert_eq!(value["userId"], user_id.to_string());
        assert_eq!(value["projectId"], project_id.to_string());
        for secret in [
            "access-token-secret",
            "refresh-token-secret",
            "private-email@example.com",
            "private-data-key-ciphertext",
        ] {
            assert!(!persisted.contains(secret), "persisted secret {secret}");
        }
        assert!(value.get("projectName").is_none());
        assert!(value.get("name").is_none());
    }

    #[test]
    fn contexts_are_scoped_by_profile_directory() {
        let temp = TempDir::new().expect("temporary directory");
        let first_dir = temp.path().join("profiles/first");
        let second_dir = temp.path().join("profiles/second");
        let credentials = credentials("https://api.example", Uuid::from_u128(3));
        let first_project = Uuid::from_u128(4);
        let second_project = Uuid::from_u128(5);

        save_current_project_in(&first_dir, &credentials, first_project)
            .expect("save first profile context");
        save_current_project_in(&second_dir, &credentials, second_project)
            .expect("save second profile context");

        assert_eq!(
            load_current_project_in(&first_dir, &credentials).expect("load first profile"),
            Some(first_project)
        );
        assert_eq!(
            load_current_project_in(&second_dir, &credentials).expect("load second profile"),
            Some(second_project)
        );
    }

    #[test]
    fn context_binding_rejects_a_different_api_or_account() {
        let temp = TempDir::new().expect("temporary directory");
        let owner = credentials("https://api.example", Uuid::from_u128(6));
        save_current_project_in(temp.path(), &owner, Uuid::from_u128(7))
            .expect("save current project");

        let other_api = credentials("https://other.example", owner.user_id);
        let error = load_current_project_in(temp.path(), &other_api)
            .expect_err("different API must be rejected");
        assert!(error.to_string().contains("different API"));

        let other_account = credentials(&owner.api_url, Uuid::from_u128(8));
        let error = load_current_project_in(temp.path(), &other_account)
            .expect_err("different account must be rejected");
        assert!(error.to_string().contains("different account"));
    }

    #[test]
    fn explicit_selection_replaces_stale_binding_and_reports_idempotence() {
        let temp = TempDir::new().expect("temporary directory");
        let first = credentials("https://api.example", Uuid::from_u128(20));
        let second = credentials("https://api.example", Uuid::from_u128(21));
        let first_project = Uuid::from_u128(22);
        let second_project = Uuid::from_u128(23);

        assert!(
            select_current_project_in(temp.path(), &first, first_project)
                .expect("select first project")
        );
        assert!(
            select_current_project_in(temp.path(), &second, second_project)
                .expect("replace stale account binding")
        );
        assert_eq!(
            load_current_project_in(temp.path(), &second).expect("load replacement"),
            Some(second_project)
        );
        assert!(
            !select_current_project_in(temp.path(), &second, second_project)
                .expect("repeat selection")
        );
    }

    #[test]
    fn explicit_selection_repairs_a_corrupt_current_schema_context() {
        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(24));
        let project_id = Uuid::from_u128(25);
        write_context_fixture(
            temp.path(),
            br#"{"schemaVersion":1,"unexpectedFutureShape":true}"#,
        );

        assert!(
            select_current_project_in(temp.path(), &credentials, project_id)
                .expect("replace corrupt current-schema context")
        );
        assert_eq!(
            load_current_project_in(temp.path(), &credentials).expect("load repaired context"),
            Some(project_id)
        );
    }

    #[test]
    fn explicit_selection_preserves_a_future_schema_context() {
        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(26));
        let future_context = br#"{"schemaVersion":2,"futureOnlyShape":true}"#;
        write_context_fixture(temp.path(), future_context);
        let path = temp.path().join(CONTEXT_FILE_NAME);
        let before = fs::read(&path).expect("read future context before selection");
        let load_error =
            load_current_project_in(temp.path(), &credentials).expect_err("future context");

        let selection_error =
            select_current_project_in(temp.path(), &credentials, Uuid::from_u128(27))
                .expect_err("future context must not be replaced");

        assert_eq!(selection_error.to_string(), load_error.to_string());
        assert!(selection_error.to_string().contains("upgrade SealTask"));
        assert_eq!(
            fs::read(path).expect("read future context after selection"),
            before
        );
    }

    #[test]
    fn corrupt_and_future_contexts_have_distinct_errors() {
        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(9));

        write_context_fixture(temp.path(), br#"{"schemaVersion":1,"#);
        let error =
            load_current_project_in(temp.path(), &credentials).expect_err("corrupt context");
        assert!(error.to_string().contains("corrupt"));

        write_context_fixture(
            temp.path(),
            br#"{"schemaVersion":2,"apiUrl":"https://api.example","userId":"00000000-0000-0000-0000-000000000009","projectId":"00000000-0000-0000-0000-000000000010"}"#,
        );
        let error = load_current_project_in(temp.path(), &credentials).expect_err("future context");
        assert!(error.to_string().contains("schema version 2"));
        assert!(error.to_string().contains("upgrade SealTask"));
    }

    #[test]
    fn clear_is_idempotent_and_recovers_an_invalid_context() {
        let temp = TempDir::new().expect("temporary directory");
        write_context_fixture(temp.path(), b"not JSON");

        assert!(clear_current_project_in(temp.path()).expect("first clear"));
        assert!(!clear_current_project_in(temp.path()).expect("second clear"));
        assert!(!temp.path().join(CONTEXT_FILE_NAME).exists());
    }

    #[test]
    fn save_replaces_a_corrupt_context_atomically() {
        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(11));
        let project_id = Uuid::from_u128(12);
        write_context_fixture(temp.path(), b"corrupt");

        save_current_project_in(temp.path(), &credentials, project_id)
            .expect("replace corrupt context");
        assert_eq!(
            load_current_project_in(temp.path(), &credentials).expect("load replacement"),
            Some(project_id)
        );
    }

    #[test]
    fn schema_one_rejects_unknown_fields() {
        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(13));
        write_context_fixture(
            temp.path(),
            br#"{"schemaVersion":1,"apiUrl":"https://api.example","userId":"00000000-0000-0000-0000-000000000013","projectId":"00000000-0000-0000-0000-000000000014","unexpectedField":true}"#,
        );

        let error = load_current_project_in(temp.path(), &credentials).expect_err("unknown field");
        assert!(error.to_string().contains("corrupt"));
    }

    #[cfg(unix)]
    #[test]
    fn context_directory_file_and_lock_use_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(15));
        save_current_project_in(temp.path(), &credentials, Uuid::from_u128(16))
            .expect("save current project");

        let dir_mode = fs::metadata(temp.path())
            .expect("context directory metadata")
            .permissions()
            .mode()
            & 0o777;
        let context_mode = fs::metadata(temp.path().join(CONTEXT_FILE_NAME))
            .expect("context metadata")
            .permissions()
            .mode()
            & 0o777;
        let lock_mode = fs::metadata(temp.path().join(CONTEXT_LOCK_FILE_NAME))
            .expect("lock metadata")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(context_mode, 0o600);
        assert_eq!(lock_mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn loading_rejects_symlinked_or_overly_permissive_contexts() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let temp = TempDir::new().expect("temporary directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(17));
        let target = temp.path().join("target.json");
        write_context_fixture(
            temp.path(),
            br#"{"schemaVersion":1,"apiUrl":"https://api.example","userId":"00000000-0000-0000-0000-000000000017","projectId":"00000000-0000-0000-0000-000000000018"}"#,
        );
        fs::rename(temp.path().join(CONTEXT_FILE_NAME), &target).expect("move context target");
        symlink(&target, temp.path().join(CONTEXT_FILE_NAME)).expect("symlink context");
        let error =
            load_current_project_in(temp.path(), &credentials).expect_err("symlink context");
        assert!(error.to_string().contains("symbolic link"));

        fs::remove_file(temp.path().join(CONTEXT_FILE_NAME)).expect("remove symlink");
        fs::rename(&target, temp.path().join(CONTEXT_FILE_NAME)).expect("restore context");
        fs::set_permissions(
            temp.path().join(CONTEXT_FILE_NAME),
            fs::Permissions::from_mode(0o644),
        )
        .expect("make context overly permissive");
        let error =
            load_current_project_in(temp.path(), &credentials).expect_err("unsafe permissions");
        assert!(error.to_string().contains("permissions are too broad"));
    }
}
