use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use sealtask_client_auth::{
    Credentials, config_dir, default_config_root, load_credentials_for_url, normalize_api_url,
};
use sealtask_client_core::{PublicError, PublicResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use uuid::Uuid;

const CONTEXT_FILE_NAME: &str = "context.json";
const CONTEXT_LOCK_FILE_NAME: &str = "context.lock";
const LOCAL_CONTEXT_DIRECTORY_NAME: &str = "project-contexts";
const LOCAL_CONTEXT_FILES_DIRECTORY_NAME: &str = "local";
const CONTEXT_SCHEMA_VERSION: u64 = 1;
pub(crate) const MAX_CONTEXT_FILE_BYTES: u64 = 16 * 1024;
const LOCAL_CONTEXT_KEY_DOMAIN: &[u8] = b"sealtask-local-project-context-v1\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ProjectContextScope {
    Local,
    Global,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedProjectContext {
    pub(crate) project_id: Uuid,
    pub(crate) scope: ProjectContextScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) directory: Option<PathBuf>,
    pub(crate) inherited: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectContextMutation {
    pub(crate) changed: bool,
    pub(crate) scope: ProjectContextScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) directory: Option<PathBuf>,
    pub(crate) inherited: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectContextDiagnosticTarget {
    pub(crate) path: PathBuf,
    pub(crate) scope: ProjectContextScope,
    pub(crate) inherited: bool,
    directory_key: Option<String>,
}

pub(crate) struct ProjectContextDiagnosticSnapshot {
    context: StoredProjectContext,
}

#[derive(Clone, Debug)]
struct ContextEnvironment {
    current_directory: PathBuf,
    home_directory: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredProjectContext {
    schema_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    directory_key: Option<String>,
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
            directory_key: None,
            api_url,
            user_id: credentials.user_id,
            project_id,
        })
    }

    fn for_directory(
        credentials: &Credentials,
        project_id: Uuid,
        directory_key: String,
    ) -> PublicResult<Self> {
        let mut context = Self::new(credentials, project_id)?;
        context.directory_key = Some(directory_key);
        Ok(context)
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
        self.validate_project_id()
    }

    fn validate_project_id(&self) -> PublicResult<()> {
        if self.project_id.is_nil() {
            return Err(corrupt_context_error());
        }
        Ok(())
    }

    fn validate_global_scope(&self) -> PublicResult<()> {
        if self.directory_key.is_some() {
            return Err(corrupt_context_error());
        }
        Ok(())
    }

    fn validate_local_scope(&self, expected_directory_key: &str) -> PublicResult<()> {
        if self.directory_key.as_deref() != Some(expected_directory_key) {
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

impl ContextEnvironment {
    fn from_process() -> PublicResult<Self> {
        let current_directory = std::env::current_dir().map_err(|err| {
            PublicError::unexpected(format!("failed to resolve the current directory: {err}"))
        })?;
        let default_config_root = default_config_root()?;
        let home_directory = default_config_root.parent().ok_or_else(|| {
            PublicError::unexpected("could not determine the home directory for project context")
        })?;
        Self::new(&current_directory, home_directory)
    }

    fn new(current_directory: &Path, home_directory: &Path) -> PublicResult<Self> {
        Ok(Self {
            current_directory: canonicalize_scope_directory(
                current_directory,
                "current directory",
            )?,
            home_directory: canonicalize_scope_directory(home_directory, "home directory")?,
        })
    }

    fn automatic_mutation_scope(&self) -> ProjectContextScope {
        if self.current_directory == self.home_directory {
            ProjectContextScope::Global
        } else {
            ProjectContextScope::Local
        }
    }

    fn local_search_directories(&self) -> Vec<PathBuf> {
        let current_is_within_home = self.current_directory.starts_with(&self.home_directory);
        let mut directories = Vec::new();
        for ancestor in self.current_directory.ancestors() {
            directories.push(ancestor.to_path_buf());
            if current_is_within_home && ancestor == self.home_directory {
                break;
            }
        }
        directories
    }
}

/// Loads the current project for the active profile and API target.
///
/// A saved context is usable only when credentials for `api_url` are present
/// and its account/API binding matches those credentials. The nearest local
/// directory binding takes precedence over the profile-global fallback.
pub(crate) fn load_current_project(api_url: &str) -> PublicResult<Option<Uuid>> {
    Ok(load_project_context(api_url, None)?.map(|context| context.project_id))
}

/// Loads a scoped project context for the active profile and API target.
///
/// `None` resolves the effective context (nearest local ancestor, then global).
/// An explicit local scope searches only the canonical current directory and
/// its eligible ancestors. An explicit global scope reads only `context.json`.
pub(crate) fn load_project_context(
    api_url: &str,
    scope: Option<ProjectContextScope>,
) -> PublicResult<Option<ResolvedProjectContext>> {
    let credentials = load_credentials_for_url(api_url)?.ok_or_else(|| {
        PublicError::validation(
            "not logged in for the current API; authenticate before using saved project context",
        )
    })?;
    let environment = ContextEnvironment::from_process()?;
    load_project_context_in(&config_dir()?, &credentials, &environment, scope)
}

/// Resolves the context file that runtime project selection would inspect.
///
/// This is a read-only diagnostic path: it applies the same canonical
/// nearest-ancestor local lookup and global fallback as `load_project_context`
/// without requiring credentials or creating the context lock.
pub(crate) fn resolve_project_context_diagnostic_target(
    config_directory: &Path,
) -> PublicResult<ProjectContextDiagnosticTarget> {
    let environment = ContextEnvironment::from_process()?;
    resolve_project_context_diagnostic_target_in(config_directory, &environment)
}

/// Reads and structurally validates a diagnostic target with the runtime
/// context decoder.
///
/// The returned opaque snapshot lets diagnostics validate account/API binding
/// against the same bytes without rereading a concurrently mutable file.
pub(crate) fn read_project_context_diagnostic_snapshot(
    target: &ProjectContextDiagnosticTarget,
) -> PublicResult<ProjectContextDiagnosticSnapshot> {
    let context = read_context_file_unlocked(&target.path)
        .map_err(ContextReadError::into_public_error)?
        .ok_or_else(|| {
            PublicError::unexpected(
                "the project context changed while diagnostics were inspecting it",
            )
        })?;
    match target.scope {
        ProjectContextScope::Local => {
            let directory_key = target
                .directory_key
                .as_deref()
                .ok_or_else(corrupt_context_error)?;
            context.validate_local_scope(directory_key)?;
        }
        ProjectContextScope::Global => context.validate_global_scope()?,
    }
    context.validate_project_id()?;
    Ok(ProjectContextDiagnosticSnapshot { context })
}

/// Validates account/API binding on a previously decoded diagnostic snapshot.
pub(crate) fn validate_project_context_diagnostic_binding(
    snapshot: &ProjectContextDiagnosticSnapshot,
    credentials: &Credentials,
) -> PublicResult<()> {
    snapshot.context.validate_binding(credentials)
}

/// Saves the current project for the active profile and API target.
///
/// Only the schema version, normalized API URL, account ID, and project ID are
/// persisted. Decrypted project names and other project content never reach
/// this file.
pub(crate) fn save_current_project(
    api_url: &str,
    project_id: Uuid,
    scope: Option<ProjectContextScope>,
) -> PublicResult<ProjectContextMutation> {
    let credentials = load_credentials_for_url(api_url)?.ok_or_else(|| {
        PublicError::validation(
            "not logged in for the current API; authenticate before selecting a project",
        )
    })?;
    let environment = ContextEnvironment::from_process()?;
    save_project_context_in(
        &config_dir()?,
        &credentials,
        project_id,
        &environment,
        scope,
    )
}

/// Clears a saved project context for the active profile.
///
/// `None` selects local outside the canonical home directory and global at
/// home. A local clear removes the nearest inherited local binding, if any.
pub(crate) fn clear_current_project(
    scope: Option<ProjectContextScope>,
) -> PublicResult<ProjectContextMutation> {
    let environment = ContextEnvironment::from_process()?;
    clear_project_context_in(&config_dir()?, &environment, scope)
}

#[cfg(test)]
fn load_current_project_in(dir: &Path, credentials: &Credentials) -> PublicResult<Option<Uuid>> {
    let context_lock = ContextFileLock::acquire(dir, LockMode::Shared)?;
    let result = load_global_project_unlocked(dir, credentials);
    finish_locked(context_lock, result)
}

fn load_project_context_in(
    dir: &Path,
    credentials: &Credentials,
    environment: &ContextEnvironment,
    scope: Option<ProjectContextScope>,
) -> PublicResult<Option<ResolvedProjectContext>> {
    let context_lock = ContextFileLock::acquire(dir, LockMode::Shared)?;
    let result = load_project_context_unlocked(dir, credentials, environment, scope);
    finish_locked(context_lock, result)
}

fn resolve_project_context_diagnostic_target_in(
    dir: &Path,
    environment: &ContextEnvironment,
) -> PublicResult<ProjectContextDiagnosticTarget> {
    if let Some((directory, path)) = nearest_local_context_path(dir, environment)? {
        return Ok(ProjectContextDiagnosticTarget {
            path,
            scope: ProjectContextScope::Local,
            inherited: directory != environment.current_directory,
            directory_key: Some(local_context_key(&directory)),
        });
    }
    Ok(ProjectContextDiagnosticTarget {
        path: dir.join(CONTEXT_FILE_NAME),
        scope: ProjectContextScope::Global,
        inherited: false,
        directory_key: None,
    })
}

fn load_project_context_unlocked(
    dir: &Path,
    credentials: &Credentials,
    environment: &ContextEnvironment,
    scope: Option<ProjectContextScope>,
) -> PublicResult<Option<ResolvedProjectContext>> {
    if scope != Some(ProjectContextScope::Global)
        && let Some(context) = load_nearest_local_project_unlocked(dir, credentials, environment)?
    {
        return Ok(Some(context));
    }
    if scope == Some(ProjectContextScope::Local) {
        return Ok(None);
    }

    Ok(
        load_global_project_unlocked(dir, credentials)?.map(|project_id| ResolvedProjectContext {
            project_id,
            scope: ProjectContextScope::Global,
            directory: None,
            inherited: false,
        }),
    )
}

fn load_global_project_unlocked(
    dir: &Path,
    credentials: &Credentials,
) -> PublicResult<Option<Uuid>> {
    let Some(context) =
        read_current_project_unlocked(dir).map_err(ContextReadError::into_public_error)?
    else {
        return Ok(None);
    };
    context.validate_global_scope()?;
    context.validate_binding(credentials)?;
    Ok(Some(context.project_id))
}

fn load_nearest_local_project_unlocked(
    dir: &Path,
    credentials: &Credentials,
    environment: &ContextEnvironment,
) -> PublicResult<Option<ResolvedProjectContext>> {
    if !inspect_local_context_storage(dir)? {
        return Ok(None);
    }
    for directory in environment.local_search_directories() {
        let directory_key = local_context_key(&directory);
        let path = local_context_file_path(dir, &directory_key);
        let Some(context) =
            read_context_file_unlocked(&path).map_err(ContextReadError::into_public_error)?
        else {
            continue;
        };
        context.validate_local_scope(&directory_key)?;
        context.validate_binding(credentials)?;
        return Ok(Some(ResolvedProjectContext {
            project_id: context.project_id,
            scope: ProjectContextScope::Local,
            inherited: directory != environment.current_directory,
            directory: Some(directory),
        }));
    }
    Ok(None)
}

fn read_current_project_unlocked(
    dir: &Path,
) -> Result<Option<StoredProjectContext>, ContextReadError> {
    read_context_file_unlocked(&dir.join(CONTEXT_FILE_NAME))
}

fn read_context_file_unlocked(
    path: &Path,
) -> Result<Option<StoredProjectContext>, ContextReadError> {
    let Some(file) = open_context_file(path).map_err(ContextReadError::Other)? else {
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

#[cfg(test)]
fn select_current_project_in(
    dir: &Path,
    credentials: &Credentials,
    project_id: Uuid,
) -> PublicResult<bool> {
    let context = StoredProjectContext::new(credentials, project_id)?;
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = select_project_context_file_unlocked(
        dir,
        &dir.join(CONTEXT_FILE_NAME),
        &context,
        |current| {
            current.validate_global_scope()?;
            current.validate_binding(credentials)
        },
    );
    finish_locked(context_lock, result)
}

fn save_project_context_in(
    dir: &Path,
    credentials: &Credentials,
    project_id: Uuid,
    environment: &ContextEnvironment,
    requested_scope: Option<ProjectContextScope>,
) -> PublicResult<ProjectContextMutation> {
    let scope = requested_scope.unwrap_or_else(|| environment.automatic_mutation_scope());
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = match scope {
        ProjectContextScope::Global => {
            let context = StoredProjectContext::new(credentials, project_id)?;
            select_project_context_file_unlocked(
                dir,
                &dir.join(CONTEXT_FILE_NAME),
                &context,
                |current| {
                    current.validate_global_scope()?;
                    current.validate_binding(credentials)
                },
            )
            .map(|changed| ProjectContextMutation {
                changed,
                scope,
                directory: None,
                inherited: false,
            })
        }
        ProjectContextScope::Local => {
            let directory = environment.current_directory.clone();
            let directory_key = local_context_key(&directory);
            let local_dir = prepare_local_context_storage(dir)?;
            let path = local_dir.join(local_context_file_name(&directory_key));
            let context = StoredProjectContext::for_directory(
                credentials,
                project_id,
                directory_key.clone(),
            )?;
            select_project_context_file_unlocked(&local_dir, &path, &context, |current| {
                current.validate_local_scope(&directory_key)?;
                current.validate_binding(credentials)
            })
            .map(|changed| ProjectContextMutation {
                changed,
                scope,
                directory: Some(directory),
                inherited: false,
            })
        }
    };
    finish_locked(context_lock, result)
}

fn select_project_context_file_unlocked(
    storage_dir: &Path,
    path: &Path,
    context: &StoredProjectContext,
    validate_current: impl Fn(&StoredProjectContext) -> PublicResult<()>,
) -> PublicResult<bool> {
    match read_context_file_unlocked(path) {
        Ok(Some(current))
            if validate_current(&current).is_ok() && current.project_id == context.project_id =>
        {
            Ok(false)
        }
        Err(ContextReadError::FutureSchema(schema_version)) => {
            Err(future_context_error(schema_version))
        }
        Ok(_) | Err(ContextReadError::Other(_)) => {
            save_project_context_file_unlocked(storage_dir, path, context).map(|()| true)
        }
    }
}

#[cfg(test)]
fn save_current_project_unlocked(dir: &Path, context: &StoredProjectContext) -> PublicResult<()> {
    save_project_context_file_unlocked(dir, &dir.join(CONTEXT_FILE_NAME), context)
}

fn save_project_context_file_unlocked(
    storage_dir: &Path,
    path: &Path,
    context: &StoredProjectContext,
) -> PublicResult<()> {
    let mut temporary = NamedTempFile::new_in(storage_dir).map_err(|err| {
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
    temporary.persist(path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to atomically replace the project context: {}",
            err.error
        ))
    })?;
    set_secret_file_permissions(path)?;
    sync_context_dir(storage_dir)
}

#[cfg(test)]
fn clear_current_project_in(dir: &Path) -> PublicResult<bool> {
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = clear_current_project_unlocked(dir);
    finish_locked(context_lock, result)
}

fn clear_project_context_in(
    dir: &Path,
    environment: &ContextEnvironment,
    requested_scope: Option<ProjectContextScope>,
) -> PublicResult<ProjectContextMutation> {
    let scope = requested_scope.unwrap_or_else(|| environment.automatic_mutation_scope());
    let context_lock = ContextFileLock::acquire(dir, LockMode::Exclusive)?;
    let result = match scope {
        ProjectContextScope::Global => {
            clear_current_project_unlocked(dir).map(|changed| ProjectContextMutation {
                changed,
                scope,
                directory: None,
                inherited: false,
            })
        }
        ProjectContextScope::Local => {
            let target = nearest_local_context_path(dir, environment)?;
            let (directory, path, inherited) = target.map_or_else(
                || {
                    let directory = environment.current_directory.clone();
                    let directory_key = local_context_key(&directory);
                    (
                        directory,
                        local_context_file_path(dir, &directory_key),
                        false,
                    )
                },
                |(directory, path)| {
                    let inherited = directory != environment.current_directory;
                    (directory, path, inherited)
                },
            );
            clear_project_context_file_unlocked(path.parent().unwrap_or(dir), &path).map(
                |changed| ProjectContextMutation {
                    changed,
                    scope,
                    directory: Some(directory),
                    inherited,
                },
            )
        }
    };
    finish_locked(context_lock, result)
}

fn nearest_local_context_path(
    dir: &Path,
    environment: &ContextEnvironment,
) -> PublicResult<Option<(PathBuf, PathBuf)>> {
    if !inspect_local_context_storage(dir)? {
        return Ok(None);
    }
    for directory in environment.local_search_directories() {
        let path = local_context_file_path(dir, &local_context_key(&directory));
        match fs::symlink_metadata(&path) {
            Ok(_) => return Ok(Some((directory, path))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(PublicError::unexpected(format!(
                    "failed to inspect the project context file: {err}"
                )));
            }
        }
    }
    Ok(None)
}

fn clear_current_project_unlocked(dir: &Path) -> PublicResult<bool> {
    clear_project_context_file_unlocked(dir, &dir.join(CONTEXT_FILE_NAME))
}

fn clear_project_context_file_unlocked(storage_dir: &Path, path: &Path) -> PublicResult<bool> {
    match fs::remove_file(path) {
        Ok(()) => {
            sync_context_dir(storage_dir)?;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(PublicError::unexpected(format!(
            "failed to clear the project context: {err}"
        ))),
    }
}

fn canonicalize_scope_directory(path: &Path, description: &str) -> PublicResult<PathBuf> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        PublicError::unexpected(format!("failed to resolve the {description}: {err}"))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|err| {
        PublicError::unexpected(format!("failed to inspect the {description}: {err}"))
    })?;
    if !metadata.is_dir() {
        return Err(PublicError::validation(format!(
            "the {description} is not a directory"
        )));
    }
    Ok(canonical)
}

fn local_context_key(directory: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LOCAL_CONTEXT_KEY_DOMAIN);
    update_directory_hash(&mut hasher, directory);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

#[cfg(unix)]
fn update_directory_hash(hasher: &mut Sha256, directory: &Path) {
    use std::os::unix::ffi::OsStrExt;

    hasher.update(b"unix\0");
    hasher.update(directory.as_os_str().as_bytes());
}

#[cfg(windows)]
fn update_directory_hash(hasher: &mut Sha256, directory: &Path) {
    use std::os::windows::ffi::OsStrExt;

    hasher.update(b"windows\0");
    for code_unit in directory.as_os_str().encode_wide() {
        hasher.update(code_unit.to_le_bytes());
    }
}

#[cfg(not(any(unix, windows)))]
fn update_directory_hash(hasher: &mut Sha256, directory: &Path) {
    hasher.update(b"other\0");
    hasher.update(directory.to_string_lossy().as_bytes());
}

fn local_context_file_name(directory_key: &str) -> String {
    format!("{directory_key}.json")
}

fn local_context_storage_path(dir: &Path) -> PathBuf {
    dir.join(LOCAL_CONTEXT_DIRECTORY_NAME)
        .join(LOCAL_CONTEXT_FILES_DIRECTORY_NAME)
}

fn local_context_file_path(dir: &Path, directory_key: &str) -> PathBuf {
    local_context_storage_path(dir).join(local_context_file_name(directory_key))
}

fn inspect_local_context_storage(dir: &Path) -> PublicResult<bool> {
    let contexts = dir.join(LOCAL_CONTEXT_DIRECTORY_NAME);
    let local = contexts.join(LOCAL_CONTEXT_FILES_DIRECTORY_NAME);
    for (path, description) in [
        (&contexts, "local project context directory"),
        (&local, "local project context files directory"),
    ] {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(err) => {
                return Err(PublicError::unexpected(format!(
                    "failed to inspect the {description}: {err}"
                )));
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(PublicError::validation(format!(
                "the {description} must not be a symbolic link"
            )));
        }
        if !metadata.is_dir() {
            return Err(PublicError::validation(format!(
                "the {description} is not a directory"
            )));
        }
        validate_secret_directory_permissions(&metadata, description)?;
    }
    Ok(true)
}

fn prepare_local_context_storage(dir: &Path) -> PublicResult<PathBuf> {
    let contexts = dir.join(LOCAL_CONTEXT_DIRECTORY_NAME);
    let local = contexts.join(LOCAL_CONTEXT_FILES_DIRECTORY_NAME);
    for (path, description, parent) in [
        (&contexts, "local project context directory", dir),
        (
            &local,
            "local project context files directory",
            contexts.as_path(),
        ),
    ] {
        reject_symlink(path, description)?;
        fs::create_dir(path)
            .or_else(|err| {
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(err)
                }
            })
            .map_err(|err| {
                PublicError::unexpected(format!("failed to create the {description}: {err}"))
            })?;
        let metadata = fs::symlink_metadata(path).map_err(|err| {
            PublicError::unexpected(format!("failed to inspect the {description}: {err}"))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(PublicError::validation(format!(
                "the {description} is not a secure directory"
            )));
        }
        set_context_dir_permissions(path)?;
        sync_context_dir(parent)?;
    }
    Ok(local)
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
fn validate_secret_directory_permissions(
    metadata: &fs::Metadata,
    description: &str,
) -> PublicResult<()> {
    use std::os::unix::fs::PermissionsExt;

    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::validation(format!(
            "the {description} permissions are too broad; expected mode 0700"
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_secret_directory_permissions(
    _metadata: &fs::Metadata,
    _description: &str,
) -> PublicResult<()> {
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

    fn context_environment(current_directory: &Path, home_directory: &Path) -> ContextEnvironment {
        ContextEnvironment::new(current_directory, home_directory)
            .expect("resolve context environment")
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
    fn automatic_mutation_scope_is_global_only_at_canonical_home() {
        let temp = TempDir::new().expect("temporary directory");
        let home = temp.path().join("home");
        let child = home.join("work/project");
        fs::create_dir_all(&child).expect("create project directory");

        let at_home = context_environment(&home, &home);
        assert_eq!(
            at_home.automatic_mutation_scope(),
            ProjectContextScope::Global
        );

        let in_child = context_environment(&child, &home);
        assert_eq!(
            in_child.automatic_mutation_scope(),
            ProjectContextScope::Local
        );
    }

    #[test]
    fn nearest_local_context_wins_over_ancestor_and_global_contexts() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("work/project");
        let nested = project.join("frontend/src");
        let sibling = project.join("backend");
        fs::create_dir_all(&nested).expect("create nested project directory");
        fs::create_dir_all(&sibling).expect("create sibling project directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(30));
        let global_project = Uuid::from_u128(31);
        let ancestor_project = Uuid::from_u128(32);
        let nested_project = Uuid::from_u128(33);

        let project_environment = context_environment(&project, &home);
        save_project_context_in(
            &config,
            &credentials,
            global_project,
            &project_environment,
            Some(ProjectContextScope::Global),
        )
        .expect("save global context");
        save_project_context_in(
            &config,
            &credentials,
            ancestor_project,
            &project_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save ancestor context");

        let nested_environment = context_environment(&nested, &home);
        let inherited = load_project_context_in(&config, &credentials, &nested_environment, None)
            .expect("load inherited context")
            .expect("inherited context");
        assert_eq!(inherited.project_id, ancestor_project);
        assert_eq!(inherited.scope, ProjectContextScope::Local);
        assert_eq!(
            inherited.directory.as_deref(),
            Some(project_environment.current_directory.as_path())
        );
        assert!(inherited.inherited);

        save_project_context_in(
            &config,
            &credentials,
            nested_project,
            &nested_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save nested context");
        let nearest = load_project_context_in(&config, &credentials, &nested_environment, None)
            .expect("load nearest context")
            .expect("nearest context");
        assert_eq!(nearest.project_id, nested_project);
        assert!(!nearest.inherited);

        let sibling_environment = context_environment(&sibling, &home);
        let sibling_context =
            load_project_context_in(&config, &credentials, &sibling_environment, None)
                .expect("load sibling context")
                .expect("sibling context");
        assert_eq!(sibling_context.project_id, ancestor_project);
        assert!(sibling_context.inherited);

        let global = load_project_context_in(
            &config,
            &credentials,
            &nested_environment,
            Some(ProjectContextScope::Global),
        )
        .expect("load explicit global context")
        .expect("global context");
        assert_eq!(global.project_id, global_project);
        assert_eq!(global.scope, ProjectContextScope::Global);
        assert!(global.directory.is_none());
        assert!(!global.inherited);
    }

    #[test]
    fn diagnostic_target_uses_inherited_local_context_then_global_fallback() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("work/project");
        let nested = project.join("src");
        fs::create_dir_all(&nested).expect("create nested project directory");
        let primary_credentials = credentials("https://api.example", Uuid::from_u128(60));
        let project_environment = context_environment(&project, &home);
        let nested_environment = context_environment(&nested, &home);

        save_project_context_in(
            &config,
            &primary_credentials,
            Uuid::from_u128(61),
            &project_environment,
            Some(ProjectContextScope::Global),
        )
        .expect("save global context");
        save_project_context_in(
            &config,
            &primary_credentials,
            Uuid::from_u128(62),
            &project_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save local context");

        let local = resolve_project_context_diagnostic_target_in(&config, &nested_environment)
            .expect("resolve inherited local diagnostic target");
        assert_eq!(local.scope, ProjectContextScope::Local);
        assert!(local.inherited);
        assert_eq!(
            local.path,
            local_context_file_path(
                &config,
                &local_context_key(&project_environment.current_directory),
            )
        );
        let local_snapshot =
            read_project_context_diagnostic_snapshot(&local).expect("read local snapshot");
        validate_project_context_diagnostic_binding(&local_snapshot, &primary_credentials)
            .expect("validate local binding");

        let other_credentials = credentials("https://other.example", Uuid::from_u128(63));
        let replacement = StoredProjectContext::for_directory(
            &other_credentials,
            Uuid::from_u128(64),
            local
                .directory_key
                .clone()
                .expect("local target directory key"),
        )
        .expect("build replacement context");
        save_project_context_file_unlocked(
            local.path.parent().expect("local context parent"),
            &local.path,
            &replacement,
        )
        .expect("replace local context after snapshot");
        validate_project_context_diagnostic_binding(&local_snapshot, &primary_credentials)
            .expect("snapshot binding must not reread the replacement");
        let replacement_snapshot =
            read_project_context_diagnostic_snapshot(&local).expect("read replacement snapshot");
        validate_project_context_diagnostic_binding(&replacement_snapshot, &primary_credentials)
            .expect_err("a fresh snapshot must observe the replacement binding");

        clear_project_context_in(
            &config,
            &nested_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("clear inherited local context");
        let global = resolve_project_context_diagnostic_target_in(&config, &nested_environment)
            .expect("resolve global diagnostic target");
        assert_eq!(global.scope, ProjectContextScope::Global);
        assert!(!global.inherited);
        assert_eq!(global.path, config.join(CONTEXT_FILE_NAME));
        let global_snapshot =
            read_project_context_diagnostic_snapshot(&global).expect("read global snapshot");
        validate_project_context_diagnostic_binding(&global_snapshot, &primary_credentials)
            .expect("validate global binding");

        let mut corrupt: serde_json::Value =
            serde_json::from_slice(&fs::read(&global.path).expect("read global diagnostic target"))
                .expect("parse global diagnostic target");
        corrupt["projectId"] = serde_json::Value::String(Uuid::nil().to_string());
        fs::write(
            &global.path,
            serde_json::to_vec_pretty(&corrupt).expect("serialize nil-project context"),
        )
        .expect("write nil-project context");
        let error = match read_project_context_diagnostic_snapshot(&global) {
            Ok(_) => panic!("credential-free validation must reject a nil project"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("corrupt"));
    }

    #[test]
    fn explicit_local_load_does_not_fall_back_to_global() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("project");
        fs::create_dir_all(&project).expect("create project directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(34));
        let environment = context_environment(&project, &home);

        save_project_context_in(
            &config,
            &credentials,
            Uuid::from_u128(35),
            &environment,
            Some(ProjectContextScope::Global),
        )
        .expect("save global context");

        assert!(
            load_project_context_in(
                &config,
                &credentials,
                &environment,
                Some(ProjectContextScope::Local),
            )
            .expect("load explicit local context")
            .is_none()
        );
    }

    #[test]
    fn default_save_and_clear_follow_home_boundary_and_report_scope() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("project");
        fs::create_dir_all(&project).expect("create project directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(36));
        let global_project = Uuid::from_u128(37);
        let local_project = Uuid::from_u128(38);
        let home_environment = context_environment(&home, &home);
        let project_environment = context_environment(&project, &home);

        let global = save_project_context_in(
            &config,
            &credentials,
            global_project,
            &home_environment,
            None,
        )
        .expect("save automatic global context");
        assert!(global.changed);
        assert_eq!(global.scope, ProjectContextScope::Global);
        assert!(global.directory.is_none());

        let local = save_project_context_in(
            &config,
            &credentials,
            local_project,
            &project_environment,
            None,
        )
        .expect("save automatic local context");
        assert!(local.changed);
        assert_eq!(local.scope, ProjectContextScope::Local);
        assert_eq!(
            local.directory.as_deref(),
            Some(project_environment.current_directory.as_path())
        );

        let cleared_local = clear_project_context_in(&config, &project_environment, None)
            .expect("clear automatic local context");
        assert!(cleared_local.changed);
        assert_eq!(cleared_local.scope, ProjectContextScope::Local);
        assert!(!cleared_local.inherited);
        let fallback = load_project_context_in(&config, &credentials, &project_environment, None)
            .expect("load global fallback")
            .expect("global fallback");
        assert_eq!(fallback.project_id, global_project);
        assert_eq!(fallback.scope, ProjectContextScope::Global);

        let cleared_global = clear_project_context_in(&config, &home_environment, None)
            .expect("clear automatic global context");
        assert!(cleared_global.changed);
        assert_eq!(cleared_global.scope, ProjectContextScope::Global);
    }

    #[test]
    fn local_clear_removes_the_nearest_inherited_binding() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("project");
        let nested = project.join("nested");
        fs::create_dir_all(&nested).expect("create nested directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(39));
        let project_environment = context_environment(&project, &home);
        let nested_environment = context_environment(&nested, &home);

        save_project_context_in(
            &config,
            &credentials,
            Uuid::from_u128(40),
            &project_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save ancestor local context");
        let cleared = clear_project_context_in(
            &config,
            &nested_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("clear inherited local context");
        assert!(cleared.changed);
        assert!(cleared.inherited);
        assert_eq!(
            cleared.directory.as_deref(),
            Some(project_environment.current_directory.as_path())
        );
        assert!(
            load_project_context_in(
                &config,
                &credentials,
                &nested_environment,
                Some(ProjectContextScope::Local),
            )
            .expect("load cleared local context")
            .is_none()
        );
    }

    #[test]
    fn local_context_storage_is_private_and_does_not_write_to_the_working_tree() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("private-client-name");
        fs::create_dir_all(&project).expect("create project directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(41));
        let project_id = Uuid::from_u128(42);
        let environment = context_environment(&project, &home);

        save_project_context_in(
            &config,
            &credentials,
            project_id,
            &environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save local context");

        assert_eq!(
            fs::read_dir(&project)
                .expect("read working directory")
                .count(),
            0
        );
        let directory_key = local_context_key(&environment.current_directory);
        let context_path = local_context_file_path(&config, &directory_key);
        let persisted = fs::read_to_string(&context_path).expect("read local context");
        let value: serde_json::Value =
            serde_json::from_str(&persisted).expect("parse local context");
        assert_eq!(value["directoryKey"], directory_key);
        assert_eq!(value["projectId"], project_id.to_string());
        assert!(!persisted.contains("private-client-name"));
        assert!(!persisted.contains("private-email@example.com"));
        assert!(!persisted.contains("private-data-key-ciphertext"));
    }

    #[test]
    fn invalid_nearest_local_binding_is_not_silently_replaced_by_global_fallback() {
        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("project");
        fs::create_dir_all(&project).expect("create project directory");
        let owner = credentials("https://api.example", Uuid::from_u128(43));
        let other_account = credentials("https://api.example", Uuid::from_u128(44));
        let environment = context_environment(&project, &home);

        save_project_context_in(
            &config,
            &other_account,
            Uuid::from_u128(45),
            &environment,
            Some(ProjectContextScope::Global),
        )
        .expect("save matching global fallback");
        save_project_context_in(
            &config,
            &owner,
            Uuid::from_u128(46),
            &environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save local context for another account");

        let error = load_project_context_in(&config, &other_account, &environment, None)
            .expect_err("invalid local binding must fail");
        assert!(error.to_string().contains("different account"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_and_physical_working_directories_share_one_local_scope() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let physical = home.join("physical-project");
        let alias = home.join("project-alias");
        fs::create_dir_all(&physical).expect("create physical project directory");
        symlink(&physical, &alias).expect("create project symlink");
        let credentials = credentials("https://api.example", Uuid::from_u128(47));
        let project_id = Uuid::from_u128(48);
        let alias_environment = context_environment(&alias, &home);
        let physical_environment = context_environment(&physical, &home);

        save_project_context_in(
            &config,
            &credentials,
            project_id,
            &alias_environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save through symlink");
        let loaded = load_project_context_in(&config, &credentials, &physical_environment, None)
            .expect("load through physical path")
            .expect("local context");
        assert_eq!(loaded.project_id, project_id);
        assert_eq!(
            loaded.directory,
            Some(physical_environment.current_directory)
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
    fn local_context_directories_and_file_use_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let home = temp.path().join("home");
        let project = home.join("project");
        fs::create_dir_all(&project).expect("create project directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(49));
        let environment = context_environment(&project, &home);

        save_project_context_in(
            &config,
            &credentials,
            Uuid::from_u128(50),
            &environment,
            Some(ProjectContextScope::Local),
        )
        .expect("save local context");

        let contexts = config.join(LOCAL_CONTEXT_DIRECTORY_NAME);
        let local = contexts.join(LOCAL_CONTEXT_FILES_DIRECTORY_NAME);
        let context =
            local_context_file_path(&config, &local_context_key(&environment.current_directory));
        let lock = config.join(CONTEXT_LOCK_FILE_NAME);
        for (path, expected_mode) in [
            (contexts.as_path(), 0o700),
            (local.as_path(), 0o700),
            (context.as_path(), 0o600),
            (lock.as_path(), 0o600),
        ] {
            let mode = fs::metadata(path)
                .expect("local context metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(
                mode,
                expected_mode,
                "unexpected mode for {}",
                path.display()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn loading_rejects_symlinked_local_context_storage() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().expect("temporary directory");
        let config = temp.path().join("config");
        let target = temp.path().join("redirected-contexts");
        let home = temp.path().join("home");
        let project = home.join("project");
        fs::create_dir_all(&config).expect("create config directory");
        fs::create_dir_all(&target).expect("create symlink target");
        fs::create_dir_all(&project).expect("create project directory");
        set_context_dir_permissions(&config).expect("secure config directory");
        symlink(&target, config.join(LOCAL_CONTEXT_DIRECTORY_NAME))
            .expect("symlink local context directory");
        let credentials = credentials("https://api.example", Uuid::from_u128(51));
        let environment = context_environment(&project, &home);

        let error = load_project_context_in(
            &config,
            &credentials,
            &environment,
            Some(ProjectContextScope::Local),
        )
        .expect_err("symlinked local storage must be rejected");
        assert!(error.to_string().contains("symbolic link"));
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
