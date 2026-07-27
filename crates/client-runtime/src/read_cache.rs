use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
#[cfg(unix)]
use cap_fs_ext::{OpenOptionsExt as _, OpenOptionsSyncExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
#[cfg(unix)]
use cap_std::fs::{DirBuilder, DirBuilderExt as _};
use chrono::{DateTime, Utc};
use fs2::FileExt as _;
use sealtask_client_api::{
    CommentResponse, MAX_COMMENTS, MAX_MEMBERS_PER_WORK_LIST, MAX_MY_TASKS,
    MAX_NOTE_COLLECTION_ITEMS, MAX_SECTIONS_PER_WORK_LIST, MAX_TASKS, MAX_WORK_LISTS,
    MyTaskResponse, NoteResponse, TaskDetailResponse, TaskListResponse,
    TaskReferenceSchemeResponse, WorkListDetailResponse, WorkListResponse,
};
use sealtask_client_auth::Credentials;
#[cfg(not(test))]
use sealtask_client_auth::with_current_credential_identity;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    MAX_READ_CACHE_CIPHERTEXT_BYTES, MAX_READ_CACHE_PLAINTEXT_BYTES, ReadCacheBinding,
    SymmetricKey, TASK_REFERENCE_REVISION_MAX, TASK_REFERENCE_SAFE_INTEGER_MAX, open_read_cache,
    seal_read_cache,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const CACHE_FILE_NAME: &str = "read-cache.strongbox";
const CACHE_LOCK_FILE_NAME: &str = "read-cache.lock";
const CACHE_SCHEMA_VERSION: u8 = 1;
const MAX_CACHE_ENTRIES: usize = 10_000;
const MAX_CACHE_KEY_BYTES: usize = 512;
const MAX_CACHE_NOTICES: usize = 32;
const CACHE_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const CACHE_LOCK_RETRY: Duration = Duration::from_millis(20);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadCacheMode {
    #[default]
    Online,
    Offline,
}

#[derive(Clone, Debug)]
pub struct ReadCacheOptions {
    pub(crate) mode: ReadCacheMode,
    pub(crate) profile_config_dir: Option<PathBuf>,
    pub(crate) active_profile: String,
}

impl ReadCacheOptions {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            mode: ReadCacheMode::Online,
            profile_config_dir: None,
            active_profile: "default".to_string(),
        }
    }

    pub fn online(
        profile_config_dir: impl Into<PathBuf>,
        active_profile: impl Into<String>,
    ) -> PublicResult<Self> {
        Self::enabled(
            ReadCacheMode::Online,
            profile_config_dir.into(),
            active_profile.into(),
        )
    }

    pub fn offline(
        profile_config_dir: impl Into<PathBuf>,
        active_profile: impl Into<String>,
    ) -> PublicResult<Self> {
        Self::enabled(
            ReadCacheMode::Offline,
            profile_config_dir.into(),
            active_profile.into(),
        )
    }

    fn enabled(
        mode: ReadCacheMode,
        profile_config_dir: PathBuf,
        active_profile: String,
    ) -> PublicResult<Self> {
        validate_profile_name(&active_profile)?;
        if profile_config_dir.as_os_str().is_empty() {
            return Err(PublicError::validation(
                "read-cache configuration directory cannot be empty",
            ));
        }
        if profile_config_dir
            .components()
            .any(|component| component == Component::ParentDir)
        {
            return Err(PublicError::validation(
                "read-cache configuration directory must not contain `..`",
            ));
        }
        Ok(Self {
            mode,
            profile_config_dir: Some(profile_config_dir),
            active_profile,
        })
    }

    #[must_use]
    pub const fn mode(&self) -> ReadCacheMode {
        self.mode
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.profile_config_dir.is_some()
    }

    #[must_use]
    pub fn active_profile(&self) -> &str {
        &self.active_profile
    }
}

impl Default for ReadCacheOptions {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadCacheStatus {
    pub enabled: bool,
    pub mode: ReadCacheMode,
    pub present: bool,
    pub ciphertext_bytes: Option<u64>,
    pub modified_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadCacheVerification {
    pub schema_version: u8,
    pub entry_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ciphertext_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadCacheNotice {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadCacheSnapshot {
    pub query: String,
    pub captured_at: DateTime<Utc>,
    pub age_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum ReadCacheQuery {
    WorkLists {
        include_archived: bool,
    },
    WorkList {
        work_list_id: Uuid,
    },
    TaskReferenceSchemes {
        work_list_id: Uuid,
    },
    ProjectTasks {
        work_list_id: Uuid,
        include_archived: bool,
    },
    MyTasks {
        include_completed: bool,
    },
    Task {
        work_list_id: Uuid,
        task_id: Uuid,
    },
    TaskByReferenceNumber {
        work_list_id: Uuid,
        reference_number: i64,
    },
    Comments {
        work_list_id: Uuid,
        task_id: Uuid,
    },
    Notes {
        work_list_id: Uuid,
    },
    Note {
        work_list_id: Uuid,
        note_id: Uuid,
    },
}

impl ReadCacheQuery {
    fn key(&self) -> String {
        match self {
            Self::WorkLists { include_archived } => {
                format!("work_lists?include_archived={include_archived}")
            }
            Self::WorkList { work_list_id } => format!("work_list/{work_list_id}"),
            Self::TaskReferenceSchemes { work_list_id } => {
                format!("work_list/{work_list_id}/task_reference_schemes")
            }
            Self::ProjectTasks {
                work_list_id,
                include_archived,
            } => format!("work_list/{work_list_id}/tasks?include_archived={include_archived}"),
            Self::MyTasks { include_completed } => {
                format!("my_tasks?include_completed={include_completed}")
            }
            Self::Task {
                work_list_id,
                task_id,
            } => format!("work_list/{work_list_id}/task/{task_id}"),
            Self::TaskByReferenceNumber {
                work_list_id,
                reference_number,
            } => format!("work_list/{work_list_id}/task_reference/{reference_number}"),
            Self::Comments {
                work_list_id,
                task_id,
            } => format!("work_list/{work_list_id}/task/{task_id}/comments"),
            Self::Notes { work_list_id } => format!("work_list/{work_list_id}/notes"),
            Self::Note {
                work_list_id,
                note_id,
            } => format!("work_list/{work_list_id}/note/{note_id}"),
        }
    }

    fn parse(key: &str) -> PublicResult<Self> {
        match key {
            "work_lists?include_archived=false" => {
                return Ok(Self::WorkLists {
                    include_archived: false,
                });
            }
            "work_lists?include_archived=true" => {
                return Ok(Self::WorkLists {
                    include_archived: true,
                });
            }
            "my_tasks?include_completed=false" => {
                return Ok(Self::MyTasks {
                    include_completed: false,
                });
            }
            "my_tasks?include_completed=true" => {
                return Ok(Self::MyTasks {
                    include_completed: true,
                });
            }
            _ => {}
        }
        let segments = key.split('/').collect::<Vec<_>>();
        match segments.as_slice() {
            ["work_list", work_list_id] => Ok(Self::WorkList {
                work_list_id: parse_cache_uuid(work_list_id)?,
            }),
            ["work_list", work_list_id, "task_reference_schemes"] => {
                Ok(Self::TaskReferenceSchemes {
                    work_list_id: parse_cache_uuid(work_list_id)?,
                })
            }
            ["work_list", work_list_id, tasks]
                if *tasks == "tasks?include_archived=false"
                    || *tasks == "tasks?include_archived=true" =>
            {
                Ok(Self::ProjectTasks {
                    work_list_id: parse_cache_uuid(work_list_id)?,
                    include_archived: *tasks == "tasks?include_archived=true",
                })
            }
            ["work_list", work_list_id, "task", task_id] => Ok(Self::Task {
                work_list_id: parse_cache_uuid(work_list_id)?,
                task_id: parse_cache_uuid(task_id)?,
            }),
            [
                "work_list",
                work_list_id,
                "task_reference",
                reference_number,
            ] => Ok(Self::TaskByReferenceNumber {
                work_list_id: parse_cache_uuid(work_list_id)?,
                reference_number: parse_cache_reference_number(reference_number)?,
            }),
            ["work_list", work_list_id, "task", task_id, "comments"] => Ok(Self::Comments {
                work_list_id: parse_cache_uuid(work_list_id)?,
                task_id: parse_cache_uuid(task_id)?,
            }),
            ["work_list", work_list_id, "notes"] => Ok(Self::Notes {
                work_list_id: parse_cache_uuid(work_list_id)?,
            }),
            ["work_list", work_list_id, "note", note_id] => Ok(Self::Note {
                work_list_id: parse_cache_uuid(work_list_id)?,
                note_id: parse_cache_uuid(note_id)?,
            }),
            _ => Err(PublicError::validation(
                "encrypted read cache contains an unknown snapshot query",
            )),
        }
    }
}

fn parse_cache_uuid(value: &str) -> PublicResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| {
        PublicError::validation("encrypted read cache contains an invalid query identifier")
    })
}

fn parse_cache_reference_number(value: &str) -> PublicResult<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|value| (1..=TASK_REFERENCE_SAFE_INTEGER_MAX).contains(value))
        .ok_or_else(|| {
            PublicError::validation(
                "encrypted read cache contains an invalid task reference number",
            )
        })
}

#[derive(Clone)]
pub(crate) struct ReadCacheRuntime {
    options: ReadCacheOptions,
    state: Arc<ReadCacheState>,
}

impl std::fmt::Debug for ReadCacheRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReadCacheRuntime")
            .field("mode", &self.options.mode)
            .field("enabled", &self.options.is_enabled())
            .field("active_profile", &self.options.active_profile)
            .field("profile_config_dir", &"<operator-local path>")
            .finish()
    }
}

struct ReadCacheState {
    invocation: Mutex<InvocationState>,
    notices: Mutex<Vec<ReadCacheNotice>>,
}

struct InvocationState {
    generation: u64,
    binding: Option<ReadCacheBinding>,
    memo: HashMap<String, Arc<MemoEntry>>,
    loaded: Option<LoadedDocument>,
    last_snapshot: Option<ReadCacheSnapshot>,
    snapshots: HashMap<String, ReadCacheSnapshot>,
}

pub(crate) struct ReadCacheWriteGuard {
    binding: ReadCacheBinding,
    credentials: Credentials,
    started_at: DateTime<Utc>,
    invocation_generation: u64,
    persistent_generation: Option<u64>,
}

struct MemoEntry {
    captured_at: DateTime<Utc>,
    payload_json: Vec<u8>,
}

impl Drop for MemoEntry {
    fn drop(&mut self) {
        self.payload_json.zeroize();
    }
}

struct LoadedDocument {
    binding: ReadCacheBinding,
    document: ReadCacheDocumentV1,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadCacheDocumentV1 {
    schema_version: u8,
    generation: u64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    entries: Vec<ReadCacheEntryV1>,
}

impl Drop for ReadCacheDocumentV1 {
    fn drop(&mut self) {
        for entry in &mut self.entries {
            entry.payload_json.zeroize();
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadCacheEntryV1 {
    key: String,
    captured_at: DateTime<Utc>,
    #[serde(with = "serde_bytes")]
    payload_json: Vec<u8>,
}

impl ReadCacheRuntime {
    pub(crate) fn new(options: ReadCacheOptions) -> Self {
        Self {
            options,
            state: Arc::new(ReadCacheState {
                invocation: Mutex::new(InvocationState {
                    generation: 0,
                    binding: None,
                    memo: HashMap::new(),
                    loaded: None,
                    last_snapshot: None,
                    snapshots: HashMap::new(),
                }),
                notices: Mutex::new(Vec::new()),
            }),
        }
    }

    pub(crate) fn mode(&self) -> ReadCacheMode {
        self.options.mode
    }

    pub(crate) fn is_offline(&self) -> bool {
        self.options.mode == ReadCacheMode::Offline
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.options.is_enabled()
    }

    pub(crate) fn binding(&self, credentials: &Credentials) -> PublicResult<ReadCacheBinding> {
        ReadCacheBinding::new(
            &credentials.api_url,
            credentials.user_id,
            &self.options.active_profile,
            &credentials.data_key_ciphertext,
        )
    }

    pub(crate) fn begin_online_read(
        &self,
        credentials: &Credentials,
    ) -> PublicResult<Option<ReadCacheWriteGuard>> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let binding = match self.binding(credentials) {
            Ok(binding) => binding,
            Err(error) => {
                let _ = self.push_notice(ReadCacheNotice {
                    code: "read_cache_unavailable",
                    message: format!(
                        "the encrypted read cache is unavailable for this session, so authoritative API reads will continue without caching: {error}"
                    ),
                });
                return Ok(None);
            }
        };
        let invocation_generation = {
            let mut invocation = self
                .state
                .invocation
                .lock()
                .map_err(|_| cache_state_error())?;
            invocation.select_identity(&binding, false)?;
            invocation.generation
        };
        let persistent_generation = if let Some(directory) =
            self.options.profile_config_dir.as_deref()
        {
            match self.capture_persistent_generation(directory) {
                Ok(generation) => Some(generation),
                Err(error) => {
                    let _ = self.push_notice(ReadCacheNotice {
                        code: "read_cache_generation_unavailable",
                        message: format!(
                            "authoritative data will be returned, but this read cannot update the encrypted offline cache safely: {error}"
                        ),
                    });
                    None
                }
            }
        } else {
            None
        };
        Ok(Some(ReadCacheWriteGuard {
            binding,
            credentials: credentials.clone(),
            started_at: Utc::now(),
            invocation_generation,
            persistent_generation,
        }))
    }

    pub(crate) fn memoized<T: DeserializeOwned>(
        &self,
        credentials: &Credentials,
        query: &ReadCacheQuery,
    ) -> PublicResult<Option<T>> {
        if !self.is_enabled() {
            return Ok(None);
        }
        let binding = match self.binding(credentials) {
            Ok(binding) => binding,
            Err(error) if !self.is_offline() => {
                let _ = self.push_notice(ReadCacheNotice {
                    code: "read_cache_unavailable",
                    message: format!(
                        "the encrypted read cache is unavailable for this session, so authoritative API reads will continue without caching: {error}"
                    ),
                });
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let key = query.key();
        let entry = {
            let mut invocation = self
                .state
                .invocation
                .lock()
                .map_err(|_| cache_state_error())?;
            invocation.select_identity(&binding, self.is_offline())?;
            let entry = invocation.memo.get(&key).cloned();
            if let Some(entry) = &entry {
                invocation.record_snapshot(&key, entry.captured_at)?;
            }
            entry
        };
        let Some(entry) = entry else {
            return Ok(None);
        };
        match decode_snapshot_json(&entry.payload_json, &key) {
            Ok(value) => Ok(Some(value)),
            Err(error) if !self.is_offline() => {
                if let Ok(mut invocation) = self.state.invocation.lock()
                    && invocation.binding.as_ref() == Some(&binding)
                    && invocation
                        .memo
                        .get(&key)
                        .is_some_and(|current| Arc::ptr_eq(current, &entry))
                {
                    invocation.memo.remove(&key);
                }
                let _ = self.push_notice(ReadCacheNotice {
                    code: "read_cache_snapshot_ignored",
                    message: format!(
                        "an incompatible invocation-cache snapshot was ignored and the authoritative API will be used: {error}"
                    ),
                });
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn record_online<T: Serialize>(
        &self,
        guard: Option<&ReadCacheWriteGuard>,
        data_key: &SymmetricKey,
        query: &ReadCacheQuery,
        value: &T,
    ) -> PublicResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        let Some(guard) = guard else {
            return Ok(());
        };
        if let Err(error) = self.record_online_inner(guard, data_key, query, value) {
            let _ = self.push_notice(ReadCacheNotice {
                code: "read_cache_update_failed",
                message: format!(
                    "authoritative data was returned, but the invocation cache could not be updated: {error}"
                ),
            });
        }
        Ok(())
    }

    fn record_online_inner<T: Serialize>(
        &self,
        guard: &ReadCacheWriteGuard,
        data_key: &SymmetricKey,
        query: &ReadCacheQuery,
        value: &T,
    ) -> PublicResult<()> {
        let key = query.key();
        let payload_json = encode_snapshot_json(value, &key)?;
        let captured_at = guard.started_at;
        {
            let mut invocation = self
                .state
                .invocation
                .lock()
                .map_err(|_| cache_state_error())?;
            if invocation.generation != guard.invocation_generation
                || invocation.binding.as_ref() != Some(&guard.binding)
            {
                drop(invocation);
                self.push_notice(ReadCacheNotice {
                    code: "read_cache_stale_response_ignored",
                    message:
                        "an authoritative response completed after cache invalidation or an account switch; it was returned but not cached"
                            .to_string(),
                })?;
                return Ok(());
            }
            if invocation
                .memo
                .get(&key)
                .is_some_and(|entry| entry.captured_at >= captured_at)
            {
                return Ok(());
            }
            invocation.memo.insert(
                key.clone(),
                Arc::new(MemoEntry {
                    captured_at,
                    payload_json: payload_json.clone(),
                }),
            );
            invocation.record_snapshot(&key, captured_at)?;
        }

        let Some(persistent_generation) = guard.persistent_generation else {
            return Ok(());
        };
        if let Err(error) = self.persist_entry(
            guard,
            persistent_generation,
            data_key,
            key,
            captured_at,
            payload_json,
        ) {
            self.push_notice(ReadCacheNotice {
                code: "read_cache_write_failed",
                message: format!(
                    "authoritative data was returned, but the encrypted offline cache could not be updated: {error}"
                ),
            })?;
        }
        Ok(())
    }

    pub(crate) fn invalidate_after_mutation(&self) {
        if let Err(error) = self.clear() {
            let _ = self.push_notice(ReadCacheNotice {
                code: "read_cache_invalidation_failed",
                message: format!(
                    "the mutation succeeded, but cached read snapshots could not be invalidated: {error}"
                ),
            });
        }
    }

    pub(crate) fn invalidate_for_mutation_result<T>(&self, result: &PublicResult<T>) {
        if result.is_ok()
            || result.as_ref().is_err_and(|error| {
                matches!(
                    error,
                    PublicError::CommittedButLocalProcessingFailed { .. }
                        | PublicError::OutcomeAmbiguous { .. }
                        | PublicError::CompensationFailed { .. }
                )
            })
        {
            self.invalidate_after_mutation();
        }
    }

    pub(crate) fn read_offline<T: DeserializeOwned>(
        &self,
        credentials: &Credentials,
        data_key: &SymmetricKey,
        query: &ReadCacheQuery,
    ) -> PublicResult<T> {
        if !self.is_enabled() {
            return Err(PublicError::validation(
                "offline mode requires an enabled encrypted read cache",
            ));
        }
        let binding = self.binding(credentials)?;
        if let Some(value) = self.memoized(credentials, query)? {
            return Ok(value);
        }
        self.ensure_loaded(binding.clone(), data_key)?;
        let key = query.key();
        let payload_json = {
            let mut invocation = self
                .state
                .invocation
                .lock()
                .map_err(|_| cache_state_error())?;
            invocation.select_identity(&binding, true)?;
            let document = &invocation
                .loaded
                .as_ref()
                .filter(|loaded| loaded.binding == binding)
                .ok_or_else(cache_state_error)?
                .document;
            let entry = document
                .entries
                .binary_search_by(|entry| entry.key.as_str().cmp(&key))
                .ok()
                .and_then(|index| document.entries.get(index))
                .ok_or_else(|| {
                    PublicError::validation(format!(
                        "the encrypted read cache has no snapshot for {key}; reconnect and run the command online first"
                    ))
                })?;
            let captured_at = entry.captured_at;
            let payload_json = entry.payload_json.clone();
            invocation.memo.insert(
                key.clone(),
                Arc::new(MemoEntry {
                    captured_at,
                    payload_json: payload_json.clone(),
                }),
            );
            invocation.record_snapshot(&key, captured_at)?;
            payload_json
        };
        decode_snapshot_json(&payload_json, &key)
    }

    pub(crate) fn status(&self) -> PublicResult<ReadCacheStatus> {
        let Some(directory) = self.options.profile_config_dir.as_deref() else {
            return Ok(ReadCacheStatus {
                enabled: false,
                mode: self.options.mode,
                present: false,
                ciphertext_bytes: None,
                modified_at: None,
            });
        };
        let Some(location) = CacheLocation::open(directory, false)? else {
            return Ok(ReadCacheStatus {
                enabled: true,
                mode: self.options.mode,
                present: false,
                ciphertext_bytes: None,
                modified_at: None,
            });
        };
        let metadata = location.cache_metadata()?;
        let Some(metadata) = metadata else {
            return Ok(ReadCacheStatus {
                enabled: true,
                mode: self.options.mode,
                present: false,
                ciphertext_bytes: None,
                modified_at: None,
            });
        };
        Ok(ReadCacheStatus {
            enabled: true,
            mode: self.options.mode,
            present: true,
            ciphertext_bytes: Some(metadata.len()),
            modified_at: metadata.modified().ok().map(DateTime::<Utc>::from),
        })
    }

    pub(crate) fn verify(
        &self,
        credentials: &Credentials,
        data_key: &SymmetricKey,
    ) -> PublicResult<ReadCacheVerification> {
        let binding = self.binding(credentials)?;
        let (document, ciphertext_bytes) = self
            .load_document_with_size(&binding, data_key)?
            .ok_or_else(|| PublicError::validation("encrypted read cache is missing"))?;
        Ok(ReadCacheVerification {
            schema_version: document.schema_version,
            entry_count: document.entries.len(),
            created_at: document.created_at,
            updated_at: document.updated_at,
            ciphertext_bytes,
        })
    }

    pub(crate) fn clear(&self) -> PublicResult<bool> {
        self.state
            .invocation
            .lock()
            .map_err(|_| cache_state_error())?
            .invalidate();
        let Some(directory) = self.options.profile_config_dir.as_deref() else {
            return Ok(false);
        };
        let Some(location) = CacheLocation::open(directory, false)? else {
            return Ok(false);
        };
        let mut lock = location.acquire_lock()?;
        lock.rotate_generation_for_clear()?;
        self.state
            .invocation
            .lock()
            .map_err(|_| cache_state_error())?
            .invalidate();
        location.remove_cache_for_clear()
    }

    pub(crate) fn take_notices(&self) -> Vec<ReadCacheNotice> {
        self.state
            .notices
            .lock()
            .map_or_else(|_| Vec::new(), |mut notices| std::mem::take(&mut *notices))
    }

    pub(crate) fn last_snapshot(&self) -> Option<ReadCacheSnapshot> {
        self.state
            .invocation
            .lock()
            .ok()
            .and_then(|invocation| invocation.last_snapshot.clone())
    }

    pub(crate) fn take_snapshots(&self) -> Vec<ReadCacheSnapshot> {
        let Ok(mut invocation) = self.state.invocation.lock() else {
            return Vec::new();
        };
        let mut snapshots = invocation
            .snapshots
            .drain()
            .map(|(_, snapshot)| snapshot)
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.query.cmp(&right.query));
        snapshots
    }

    fn capture_persistent_generation(&self, directory: &Path) -> PublicResult<u64> {
        let location = CacheLocation::open(directory, true)?
            .ok_or_else(|| PublicError::unexpected("failed to create read-cache directory"))?;
        let mut lock = location.acquire_lock()?;
        lock.generation()
    }

    fn persist_entry(
        &self,
        guard: &ReadCacheWriteGuard,
        expected_generation: u64,
        data_key: &SymmetricKey,
        key: String,
        captured_at: DateTime<Utc>,
        payload_json: Vec<u8>,
    ) -> PublicResult<()> {
        with_current_cache_identity(&guard.credentials, |_| {
            let directory = self
                .options
                .profile_config_dir
                .as_deref()
                .ok_or_else(|| PublicError::unexpected("read cache is not configured"))?;
            let location = CacheLocation::open(directory, true)?
                .ok_or_else(|| PublicError::unexpected("failed to create read-cache directory"))?;
            let mut lock = location.acquire_lock()?;
            if lock.generation()? != expected_generation {
                self.push_notice(ReadCacheNotice {
                    code: "read_cache_stale_response_ignored",
                    message:
                        "an authoritative response completed after another process invalidated the cache; it was returned but not cached"
                            .to_string(),
                })?;
                return Ok(());
            }
            let mut recovery_error = None;
            let mut document = match location.read_cache()? {
                Some(ciphertext) => {
                    match open_read_cache(data_key, &guard.binding, &ciphertext)
                        .and_then(|plaintext| decode_document(&plaintext))
                    {
                        Ok(document) => document,
                        Err(error) => {
                            recovery_error = Some(error.to_string());
                            ReadCacheDocumentV1::empty(expected_generation)
                        }
                    }
                }
                None => ReadCacheDocumentV1::empty(expected_generation),
            };
            if document.generation != expected_generation {
                recovery_error = Some(
                    "encrypted read-cache generation does not match its invalidation lock"
                        .to_string(),
                );
                document = ReadCacheDocumentV1::empty(expected_generation);
            }
            match document
                .entries
                .binary_search_by(|entry| entry.key.as_str().cmp(&key))
            {
                Ok(index) => {
                    if document.entries[index].captured_at >= captured_at {
                        return Ok(());
                    }
                    document.entries[index] = ReadCacheEntryV1 {
                        key,
                        captured_at,
                        payload_json,
                    };
                }
                Err(index) => {
                    if document.entries.len() >= MAX_CACHE_ENTRIES {
                        return Err(PublicError::unexpected(format!(
                            "encrypted read cache exceeds the {MAX_CACHE_ENTRIES}-entry limit"
                        )));
                    }
                    document.entries.insert(
                        index,
                        ReadCacheEntryV1 {
                            key,
                            captured_at,
                            payload_json,
                        },
                    );
                }
            }
            document.created_at = document.created_at.min(captured_at);
            document.updated_at = document.updated_at.max(captured_at);
            let encoded = encode_document(&document)?;
            let ciphertext = seal_read_cache(data_key, &guard.binding, &encoded)?;
            location.atomic_write(&ciphertext)?;
            if let Some(error) = recovery_error {
                self.push_notice(ReadCacheNotice {
                    code: "read_cache_recovered",
                    message: format!(
                        "the existing encrypted read cache failed cryptographic or schema verification and was durably replaced after an authoritative online read: {error}"
                    ),
                })?;
            }
            Ok(())
        })
    }

    fn ensure_loaded(
        &self,
        binding: ReadCacheBinding,
        data_key: &SymmetricKey,
    ) -> PublicResult<()> {
        let invocation_generation = {
            let mut invocation = self
                .state
                .invocation
                .lock()
                .map_err(|_| cache_state_error())?;
            invocation.select_identity(&binding, true)?;
            if invocation
                .loaded
                .as_ref()
                .is_some_and(|loaded| loaded.binding == binding)
            {
                return Ok(());
            }
            invocation.generation
        };
        let loaded = match self.load_document(&binding, data_key) {
            Ok(Some(document)) => document,
            Ok(None) => {
                return Err(PublicError::validation(
                    "encrypted read cache is missing; reconnect and run a read command online first",
                ));
            }
            Err(error) => return Err(error),
        };
        let mut invocation = self
            .state
            .invocation
            .lock()
            .map_err(|_| cache_state_error())?;
        if invocation.generation != invocation_generation
            || invocation.binding.as_ref() != Some(&binding)
        {
            return Err(PublicError::validation(
                "encrypted read cache was invalidated while it was being loaded; retry the command",
            ));
        }
        invocation.loaded = Some(LoadedDocument {
            binding,
            document: loaded,
        });
        Ok(())
    }

    fn load_document(
        &self,
        binding: &ReadCacheBinding,
        data_key: &SymmetricKey,
    ) -> PublicResult<Option<ReadCacheDocumentV1>> {
        self.load_document_with_size(binding, data_key)
            .map(|loaded| loaded.map(|(document, _)| document))
    }

    fn load_document_with_size(
        &self,
        binding: &ReadCacheBinding,
        data_key: &SymmetricKey,
    ) -> PublicResult<Option<(ReadCacheDocumentV1, u64)>> {
        let directory = self
            .options
            .profile_config_dir
            .as_deref()
            .ok_or_else(|| PublicError::validation("read cache is not configured"))?;
        let Some(location) = CacheLocation::open(directory, false)? else {
            return Ok(None);
        };
        let mut lock = location.acquire_lock()?;
        let generation = lock.generation()?;
        let Some(ciphertext) = location.read_cache()? else {
            return Ok(None);
        };
        let ciphertext_bytes = u64::try_from(ciphertext.len()).map_err(|_| {
            PublicError::validation("encrypted read-cache size does not fit this platform")
        })?;
        let plaintext = open_read_cache(data_key, binding, &ciphertext)?;
        let document = decode_document(&plaintext)?;
        if document.generation != generation {
            return Err(PublicError::validation(
                "encrypted read cache was invalidated and cannot be used offline",
            ));
        }
        Ok(Some((document, ciphertext_bytes)))
    }
}

impl InvocationState {
    fn select_identity(&mut self, binding: &ReadCacheBinding, strict: bool) -> PublicResult<()> {
        match self.binding.as_ref() {
            Some(active) if active != binding => {
                if strict {
                    return Err(PublicError::validation(
                        "the invocation read-cache identity changed; start a new command for the selected profile and account",
                    ));
                }
                self.invalidate();
                self.binding = Some(binding.clone());
            }
            Some(_) => {}
            None => self.binding = Some(binding.clone()),
        }
        Ok(())
    }

    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.binding = None;
        self.memo.clear();
        self.loaded = None;
        self.last_snapshot = None;
        self.snapshots.clear();
    }

    fn record_snapshot(&mut self, query: &str, captured_at: DateTime<Utc>) -> PublicResult<()> {
        let age_seconds = Utc::now()
            .signed_duration_since(captured_at)
            .num_seconds()
            .max(0) as u64;
        let snapshot = ReadCacheSnapshot {
            query: query.to_string(),
            captured_at,
            age_seconds,
        };
        self.last_snapshot = Some(snapshot.clone());
        if self.snapshots.len() >= MAX_CACHE_ENTRIES && !self.snapshots.contains_key(query) {
            return Err(PublicError::unexpected(
                "read-cache snapshot-report limit was exceeded",
            ));
        }
        self.snapshots.insert(query.to_string(), snapshot);
        Ok(())
    }
}

impl ReadCacheRuntime {
    fn push_notice(&self, notice: ReadCacheNotice) -> PublicResult<()> {
        let mut notices = self.state.notices.lock().map_err(|_| cache_state_error())?;
        if notices
            .iter()
            .any(|existing| existing.code == notice.code && existing.message == notice.message)
            || notices.len() >= MAX_CACHE_NOTICES
        {
            return Ok(());
        }
        if notices.len() == MAX_CACHE_NOTICES - 1 {
            notices.push(ReadCacheNotice {
                code: "read_cache_notices_suppressed",
                message:
                    "additional encrypted read-cache notices were suppressed for this invocation"
                        .to_string(),
            });
        } else {
            notices.push(notice);
        }
        Ok(())
    }
}

impl ReadCacheDocumentV1 {
    fn empty(generation: u64) -> Self {
        let now = Utc::now();
        Self {
            schema_version: CACHE_SCHEMA_VERSION,
            generation,
            created_at: now,
            updated_at: now,
            entries: Vec::new(),
        }
    }

    fn validate(&self) -> PublicResult<()> {
        if self.schema_version != CACHE_SCHEMA_VERSION {
            return Err(PublicError::validation(format!(
                "unsupported encrypted read-cache schema version {}",
                self.schema_version
            )));
        }
        if self.created_at > self.updated_at {
            return Err(PublicError::validation(
                "encrypted read-cache timestamps are inconsistent",
            ));
        }
        if self.entries.len() > MAX_CACHE_ENTRIES {
            return Err(PublicError::validation(format!(
                "encrypted read cache exceeds the {MAX_CACHE_ENTRIES}-entry limit"
            )));
        }
        let mut previous: Option<&str> = None;
        for entry in &self.entries {
            if entry.key.is_empty() || entry.key.len() > MAX_CACHE_KEY_BYTES {
                return Err(PublicError::validation(
                    "encrypted read cache contains an invalid snapshot key",
                ));
            }
            if previous.is_some_and(|previous| previous >= entry.key.as_str()) {
                return Err(PublicError::validation(
                    "encrypted read-cache snapshot keys are not strictly sorted",
                ));
            }
            if entry.captured_at < self.created_at || entry.captured_at > self.updated_at {
                return Err(PublicError::validation(
                    "encrypted read cache contains an invalid snapshot timestamp",
                ));
            }
            validate_snapshot_schema(&entry.key, &entry.payload_json)?;
            previous = Some(&entry.key);
        }
        Ok(())
    }
}

fn validate_snapshot_schema(key: &str, payload: &[u8]) -> PublicResult<()> {
    match ReadCacheQuery::parse(key)? {
        ReadCacheQuery::WorkLists { include_archived } => {
            let values: Vec<WorkListResponse> = decode_snapshot_json(payload, key)?;
            ensure_cache_count(values.len(), MAX_WORK_LISTS, "work lists")?;
            if !include_archived && values.iter().any(|value| value.archived_at.is_some()) {
                return Err(cache_snapshot_schema_error(
                    "active project collection contains an archived project",
                ));
            }
            for value in &values {
                ensure_cache_count(
                    value.section_snapshots.len(),
                    MAX_SECTIONS_PER_WORK_LIST,
                    "project sections",
                )?;
            }
        }
        ReadCacheQuery::WorkList { work_list_id } => {
            let value: WorkListDetailResponse = decode_snapshot_json(payload, key)?;
            if value.work_list.id != work_list_id {
                return Err(cache_snapshot_schema_error(
                    "project detail does not match its cache key",
                ));
            }
            ensure_cache_count(
                value.work_list.section_snapshots.len(),
                MAX_SECTIONS_PER_WORK_LIST,
                "project sections",
            )?;
            ensure_cache_count(value.members.len(), MAX_MEMBERS_PER_WORK_LIST, "members")?;
        }
        ReadCacheQuery::TaskReferenceSchemes { work_list_id } => {
            let values: Vec<TaskReferenceSchemeResponse> = decode_snapshot_json(payload, key)?;
            ensure_cache_count(
                values.len(),
                usize::try_from(TASK_REFERENCE_REVISION_MAX)
                    .expect("task-reference revision limit fits usize"),
                "task-reference schemes",
            )?;
            if values
                .iter()
                .any(|scheme| scheme.work_list_id != work_list_id)
            {
                return Err(cache_snapshot_schema_error(
                    "task-reference scheme history does not match its cache key",
                ));
            }
        }
        ReadCacheQuery::ProjectTasks {
            work_list_id,
            include_archived,
        } => {
            let value: TaskListResponse = decode_snapshot_json(payload, key)?;
            ensure_cache_count(value.tasks.len(), MAX_TASKS, "tasks")?;
            if value
                .tasks
                .iter()
                .any(|task| task.work_list_id != work_list_id)
            {
                return Err(cache_snapshot_schema_error(
                    "project task collection does not match its cache key",
                ));
            }
            if !include_archived && value.tasks.iter().any(|task| task.archived_at.is_some()) {
                return Err(cache_snapshot_schema_error(
                    "active task collection contains an archived task",
                ));
            }
        }
        ReadCacheQuery::MyTasks { include_completed } => {
            let values: Vec<MyTaskResponse> = decode_snapshot_json(payload, key)?;
            ensure_cache_count(values.len(), MAX_MY_TASKS, "assigned tasks")?;
            if !include_completed && values.iter().any(|task| task.is_completed) {
                return Err(cache_snapshot_schema_error(
                    "active assigned-task collection contains a completed task",
                ));
            }
        }
        ReadCacheQuery::Task {
            work_list_id,
            task_id,
        } => {
            let value: TaskDetailResponse = decode_snapshot_json(payload, key)?;
            if value.task.id != task_id || value.task.work_list_id != work_list_id {
                return Err(cache_snapshot_schema_error(
                    "task detail does not match its cache key",
                ));
            }
            ensure_cache_count(value.comments.len(), MAX_COMMENTS, "task comments")?;
            if value
                .comments
                .iter()
                .any(|comment| comment.task_id != task_id)
            {
                return Err(cache_snapshot_schema_error(
                    "task comments do not match their cached task",
                ));
            }
        }
        ReadCacheQuery::TaskByReferenceNumber {
            work_list_id,
            reference_number,
        } => {
            let value: TaskDetailResponse = decode_snapshot_json(payload, key)?;
            if value.task.work_list_id != work_list_id
                || value.task.reference_number != Some(reference_number)
            {
                return Err(cache_snapshot_schema_error(
                    "task-reference lookup does not match its cache key",
                ));
            }
            ensure_cache_count(value.comments.len(), MAX_COMMENTS, "task comments")?;
            if value
                .comments
                .iter()
                .any(|comment| comment.task_id != value.task.id)
            {
                return Err(cache_snapshot_schema_error(
                    "task comments do not match their cached task reference",
                ));
            }
        }
        ReadCacheQuery::Comments {
            work_list_id: _,
            task_id,
        } => {
            let values: Vec<CommentResponse> = decode_snapshot_json(payload, key)?;
            ensure_cache_count(values.len(), MAX_COMMENTS, "task comments")?;
            if values.iter().any(|comment| comment.task_id != task_id) {
                return Err(cache_snapshot_schema_error(
                    "comment collection does not match its cache key",
                ));
            }
        }
        ReadCacheQuery::Notes { work_list_id } => {
            let values: Vec<NoteResponse> = decode_snapshot_json(payload, key)?;
            ensure_cache_count(values.len(), MAX_NOTE_COLLECTION_ITEMS, "notes")?;
            if values.iter().any(|note| note.work_list_id != work_list_id) {
                return Err(cache_snapshot_schema_error(
                    "note collection does not match its cache key",
                ));
            }
        }
        ReadCacheQuery::Note {
            work_list_id,
            note_id,
        } => {
            let value: NoteResponse = decode_snapshot_json(payload, key)?;
            if value.id != note_id || value.work_list_id != work_list_id {
                return Err(cache_snapshot_schema_error(
                    "note detail does not match its cache key",
                ));
            }
        }
    }
    Ok(())
}

fn ensure_cache_count(count: usize, maximum: usize, label: &str) -> PublicResult<()> {
    if count > maximum {
        return Err(cache_snapshot_schema_error(&format!(
            "{label} exceed the {maximum}-item limit"
        )));
    }
    Ok(())
}

fn cache_snapshot_schema_error(message: &str) -> PublicError {
    PublicError::validation(format!(
        "encrypted read cache contains an incompatible snapshot: {message}"
    ))
}

fn encode_document(document: &ReadCacheDocumentV1) -> PublicResult<Zeroizing<Vec<u8>>> {
    document.validate()?;
    let mut encoded = Zeroizing::new(Vec::new());
    ciborium::ser::into_writer(document, &mut *encoded).map_err(|error| {
        PublicError::unexpected(format!("failed to encode encrypted read cache: {error}"))
    })?;
    if encoded.len() > MAX_READ_CACHE_PLAINTEXT_BYTES {
        return Err(PublicError::unexpected(format!(
            "encrypted read-cache plaintext exceeds the {MAX_READ_CACHE_PLAINTEXT_BYTES}-byte limit"
        )));
    }
    Ok(encoded)
}

fn decode_document(plaintext: &[u8]) -> PublicResult<ReadCacheDocumentV1> {
    if plaintext.len() > MAX_READ_CACHE_PLAINTEXT_BYTES {
        return Err(PublicError::validation(format!(
            "encrypted read-cache plaintext exceeds the {MAX_READ_CACHE_PLAINTEXT_BYTES}-byte limit"
        )));
    }
    let document: ReadCacheDocumentV1 = ciborium::de::from_reader(plaintext).map_err(|error| {
        PublicError::validation(format!("encrypted read cache is corrupt: {error}"))
    })?;
    document.validate()?;
    Ok(document)
}

fn encode_snapshot_json<T: Serialize>(value: &T, key: &str) -> PublicResult<Vec<u8>> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to encode read-cache snapshot {key}: {error}"
        ))
    })?;
    if encoded.len() > MAX_READ_CACHE_PLAINTEXT_BYTES {
        return Err(PublicError::unexpected(format!(
            "read-cache snapshot {key} exceeds the {MAX_READ_CACHE_PLAINTEXT_BYTES}-byte limit"
        )));
    }
    Ok(encoded)
}

fn decode_snapshot_json<T: DeserializeOwned>(bytes: &[u8], key: &str) -> PublicResult<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut deserializer).map_err(|error| {
        PublicError::validation(format!(
            "encrypted read-cache snapshot {key} has an incompatible schema: {error}"
        ))
    })?;
    deserializer.end().map_err(|_| {
        PublicError::validation(format!(
            "encrypted read-cache snapshot {key} contains trailing data"
        ))
    })?;
    Ok(value)
}

struct CacheLocation {
    directory: Dir,
}

impl CacheLocation {
    fn open(path: &Path, create: bool) -> PublicResult<Option<Self>> {
        let absolute = absolute_normalized_path(path)?;
        let root = filesystem_root(&absolute)?;
        let mut directory = Dir::open_ambient_dir(&root, ambient_authority()).map_err(|error| {
            cache_io(format!(
                "failed to open read-cache filesystem root {}: {error}",
                root.display()
            ))
        })?;
        let mut walked = root;
        for component in absolute.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            let next_path = walked.join(name);
            match directory.open_dir_nofollow(Path::new(name)) {
                Ok(child) => directory = child,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && !create => {
                    return Ok(None);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    match create_private_directory(&directory, name) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(error) => {
                            return Err(cache_io(format!(
                                "failed to create private read-cache directory {}: {error}",
                                next_path.display()
                            )));
                        }
                    }
                    directory = directory
                        .open_dir_nofollow(Path::new(name))
                        .map_err(|error| cache_path_error(&next_path, error))?;
                    secure_new_directory(&directory)?;
                    sync_directory(&directory)?;
                }
                Err(error) => return Err(cache_path_error(&next_path, error)),
            }
            walked = next_path;
        }
        validate_private_directory(&directory)?;
        Ok(Some(Self { directory }))
    }

    fn acquire_lock(&self) -> PublicResult<CacheLock> {
        let mut create_options = CapOpenOptions::new();
        create_options
            .create_new(true)
            .read(true)
            .write(true)
            .follow(FollowSymlinks::No);
        #[cfg(unix)]
        create_options.mode(0o600).nonblock(true);
        let (file, created) = match self
            .directory
            .open_with(CACHE_LOCK_FILE_NAME, &create_options)
        {
            Ok(file) => (file.into_std(), true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut options = CapOpenOptions::new();
                options.read(true).write(true).follow(FollowSymlinks::No);
                #[cfg(unix)]
                options.nonblock(true);
                let file = self
                    .directory
                    .open_with(CACHE_LOCK_FILE_NAME, &options)
                    .map(cap_std::fs::File::into_std)
                    .map_err(|error| cache_relative_open_error("read-cache lock", error))?;
                (file, false)
            }
            Err(error) => {
                return Err(cache_relative_open_error("read-cache lock", error));
            }
        };
        if created {
            secure_new_file(&file, "read-cache lock")?;
            self.sync_directory()?;
        } else {
            validate_private_file(&file, "read-cache lock")?;
        }
        let deadline = Instant::now() + CACHE_LOCK_TIMEOUT;
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(CacheLock(Some(file))),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(PublicError::unexpected(
                            "encrypted read cache is locked by another process",
                        ));
                    }
                    thread::sleep(CACHE_LOCK_RETRY);
                }
                Err(error) => {
                    return Err(cache_io(format!(
                        "failed to lock encrypted read cache: {error}"
                    )));
                }
            }
        }
    }

    fn cache_metadata(&self) -> PublicResult<Option<fs::Metadata>> {
        self.open_cache_file()?
            .map(|file| {
                file.metadata().map_err(|error| {
                    cache_io(format!("failed to inspect encrypted read cache: {error}"))
                })
            })
            .transpose()
    }

    fn open_cache_file(&self) -> PublicResult<Option<File>> {
        let mut options = CapOpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        #[cfg(unix)]
        options.nonblock(true);
        let file = match self.directory.open_with(CACHE_FILE_NAME, &options) {
            Ok(file) => file.into_std(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(cache_relative_open_error("encrypted read cache", error));
            }
        };
        validate_private_file(&file, "encrypted read cache")?;
        let metadata = file.metadata().map_err(|error| {
            cache_io(format!("failed to inspect encrypted read cache: {error}"))
        })?;
        if metadata.len() > MAX_READ_CACHE_CIPHERTEXT_BYTES as u64 {
            return Err(PublicError::validation(format!(
                "encrypted read cache exceeds the {MAX_READ_CACHE_CIPHERTEXT_BYTES}-byte limit"
            )));
        }
        Ok(Some(file))
    }

    fn read_cache(&self) -> PublicResult<Option<Zeroizing<Vec<u8>>>> {
        let Some(file) = self.open_cache_file()? else {
            return Ok(None);
        };
        let metadata = file.metadata().map_err(|error| {
            cache_io(format!("failed to inspect encrypted read cache: {error}"))
        })?;
        let capacity = usize::try_from(metadata.len()).map_err(|_| {
            PublicError::validation("encrypted read-cache size does not fit this platform")
        })?;
        let mut ciphertext = Zeroizing::new(Vec::with_capacity(capacity));
        file.take(MAX_READ_CACHE_CIPHERTEXT_BYTES as u64 + 1)
            .read_to_end(&mut ciphertext)
            .map_err(|error| cache_io(format!("failed to read encrypted read cache: {error}")))?;
        if ciphertext.len() > MAX_READ_CACHE_CIPHERTEXT_BYTES {
            return Err(PublicError::validation(format!(
                "encrypted read cache exceeds the {MAX_READ_CACHE_CIPHERTEXT_BYTES}-byte limit"
            )));
        }
        Ok(Some(ciphertext))
    }

    fn atomic_write(&self, ciphertext: &[u8]) -> PublicResult<()> {
        if ciphertext.len() > MAX_READ_CACHE_CIPHERTEXT_BYTES {
            return Err(PublicError::unexpected(format!(
                "encrypted read cache exceeds the {MAX_READ_CACHE_CIPHERTEXT_BYTES}-byte limit"
            )));
        }
        let mut temporary = TemporaryCacheFile::create(&self.directory)?;
        temporary
            .file_mut()?
            .write_all(ciphertext)
            .map_err(|error| {
                cache_io(format!(
                    "failed to write temporary encrypted read cache: {error}"
                ))
            })?;
        temporary.file()?.sync_all().map_err(|error| {
            cache_io(format!(
                "failed to synchronize temporary encrypted read cache: {error}"
            ))
        })?;
        temporary.publish()?;
        self.sync_directory()
    }

    fn remove_cache_for_clear(&self) -> PublicResult<bool> {
        match self.directory.remove_file(CACHE_FILE_NAME) {
            Ok(()) => {
                self.sync_directory()?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(cache_io(format!(
                "failed to remove encrypted read cache: {error}"
            ))),
        }
    }

    fn sync_directory(&self) -> PublicResult<()> {
        sync_directory(&self.directory)
    }
}

struct CacheLock(Option<File>);

impl CacheLock {
    fn generation(&mut self) -> PublicResult<u64> {
        let file = self
            .0
            .as_mut()
            .ok_or_else(|| PublicError::unexpected("read-cache lock was already released"))?;
        let length = file
            .metadata()
            .map_err(|error| cache_io(format!("failed to inspect read-cache lock: {error}")))?
            .len();
        match length {
            0 => {
                file.seek(SeekFrom::Start(0)).map_err(|error| {
                    cache_io(format!(
                        "failed to initialize read-cache generation: {error}"
                    ))
                })?;
                file.write_all(&0_u64.to_be_bytes()).map_err(|error| {
                    cache_io(format!(
                        "failed to initialize read-cache generation: {error}"
                    ))
                })?;
                file.sync_all().map_err(|error| {
                    cache_io(format!(
                        "failed to synchronize read-cache generation: {error}"
                    ))
                })?;
                Ok(0)
            }
            8 => {
                let mut bytes = [0_u8; 8];
                file.seek(SeekFrom::Start(0)).map_err(|error| {
                    cache_io(format!("failed to read read-cache generation: {error}"))
                })?;
                file.read_exact(&mut bytes).map_err(|error| {
                    cache_io(format!("failed to read read-cache generation: {error}"))
                })?;
                Ok(u64::from_be_bytes(bytes))
            }
            _ => Err(PublicError::validation(
                "read-cache lock contains invalid generation metadata",
            )),
        }
    }

    #[cfg(test)]
    fn rotate_generation(&mut self) -> PublicResult<u64> {
        let next = self.generation()?.checked_add(1).ok_or_else(|| {
            PublicError::unexpected("read-cache invalidation generation is exhausted")
        })?;
        let file = self
            .0
            .as_mut()
            .ok_or_else(|| PublicError::unexpected("read-cache lock was already released"))?;
        file.seek(SeekFrom::Start(0)).map_err(|error| {
            cache_io(format!("failed to rotate read-cache generation: {error}"))
        })?;
        file.write_all(&next.to_be_bytes()).map_err(|error| {
            cache_io(format!("failed to rotate read-cache generation: {error}"))
        })?;
        file.set_len(8).map_err(|error| {
            cache_io(format!("failed to rotate read-cache generation: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            cache_io(format!(
                "failed to synchronize read-cache generation: {error}"
            ))
        })?;
        Ok(next)
    }

    fn rotate_generation_for_clear(&mut self) -> PublicResult<u64> {
        let next = self
            .generation()
            .ok()
            .and_then(|generation| generation.checked_add(1))
            .unwrap_or_else(fresh_cache_generation);
        let file = self
            .0
            .as_mut()
            .ok_or_else(|| PublicError::unexpected("read-cache lock was already released"))?;
        file.seek(SeekFrom::Start(0)).map_err(|error| {
            cache_io(format!(
                "failed to recover read-cache invalidation generation: {error}"
            ))
        })?;
        file.write_all(&next.to_be_bytes()).map_err(|error| {
            cache_io(format!(
                "failed to recover read-cache invalidation generation: {error}"
            ))
        })?;
        file.set_len(8).map_err(|error| {
            cache_io(format!(
                "failed to recover read-cache invalidation generation: {error}"
            ))
        })?;
        file.sync_all().map_err(|error| {
            cache_io(format!(
                "failed to synchronize recovered read-cache generation: {error}"
            ))
        })?;
        Ok(next)
    }
}

fn fresh_cache_generation() -> u64 {
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&Uuid::now_v7().as_bytes()[..8]);
    u64::from_be_bytes(bytes).max(1)
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        if let Some(file) = self.0.take() {
            let _ = file.unlock();
        }
    }
}

struct TemporaryCacheFile<'a> {
    directory: &'a Dir,
    name: PathBuf,
    file: Option<File>,
    armed: bool,
}

impl<'a> TemporaryCacheFile<'a> {
    fn create(directory: &'a Dir) -> PublicResult<Self> {
        for _ in 0..8 {
            let name = PathBuf::from(format!(".read-cache-{}.tmp", Uuid::now_v7().simple()));
            let mut options = CapOpenOptions::new();
            options
                .create_new(true)
                .read(true)
                .write(true)
                .follow(FollowSymlinks::No);
            #[cfg(unix)]
            options.mode(0o600).nonblock(true).sync(true);
            match directory.open_with(&name, &options) {
                Ok(file) => {
                    let file = file.into_std();
                    secure_new_file(&file, "temporary encrypted read cache")?;
                    return Ok(Self {
                        directory,
                        name,
                        file: Some(file),
                        armed: true,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(cache_io(format!(
                        "failed to create temporary encrypted read cache: {error}"
                    )));
                }
            }
        }
        Err(cache_io(
            "failed to allocate a temporary encrypted read-cache file",
        ))
    }

    fn file(&self) -> PublicResult<&File> {
        self.file.as_ref().ok_or_else(|| {
            PublicError::unexpected("temporary encrypted read-cache file was closed")
        })
    }

    fn file_mut(&mut self) -> PublicResult<&mut File> {
        self.file.as_mut().ok_or_else(|| {
            PublicError::unexpected("temporary encrypted read-cache file was closed")
        })
    }

    fn publish(mut self) -> PublicResult<()> {
        drop(self.file.take());
        self.directory
            .rename(&self.name, self.directory, CACHE_FILE_NAME)
            .map_err(|error| {
                cache_io(format!(
                    "failed to atomically replace encrypted read cache: {error}"
                ))
            })?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for TemporaryCacheFile<'_> {
    fn drop(&mut self) {
        drop(self.file.take());
        if self.armed {
            let _ = self.directory.remove_file(&self.name);
        }
    }
}

fn validate_profile_name(profile: &str) -> PublicResult<()> {
    if profile.is_empty()
        || profile.len() > 64
        || profile == "."
        || profile == ".."
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(PublicError::validation(
            "read-cache profile must contain 1 to 64 ASCII letters, digits, '.', '_', or '-' and cannot be '.' or '..'",
        ));
    }
    Ok(())
}

fn absolute_normalized_path(path: &Path) -> PublicResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| cache_io(format!("failed to resolve read-cache directory: {error}")))?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(PublicError::validation(
                    "read-cache directory must not contain `..`",
                ));
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(cache_io(
            "failed to resolve read-cache directory as an absolute path",
        ));
    }
    canonicalize_trusted_macos_system_alias(normalized)
}

#[cfg(target_os = "macos")]
fn canonicalize_trusted_macos_system_alias(path: PathBuf) -> PublicResult<PathBuf> {
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
        cache_io(format!(
            "failed to inspect macOS read-cache system alias {}: {error}",
            alias.display()
        ))
    })?;
    if !alias_metadata.file_type().is_symlink() {
        return Ok(path);
    }
    let actual_target = fs::read_link(alias).map_err(|error| {
        cache_io(format!(
            "failed to resolve macOS read-cache system alias {}: {error}",
            alias.display()
        ))
    })?;
    let target_metadata = fs::symlink_metadata(absolute_target).map_err(|error| {
        cache_io(format!(
            "failed to inspect trusted macOS read-cache system target {}: {error}",
            absolute_target.display()
        ))
    })?;
    if alias_metadata.uid() != 0
        || (actual_target != expected_relative_target && actual_target != absolute_target)
        || target_metadata.uid() != 0
        || target_metadata.file_type().is_symlink()
        || !target_metadata.is_dir()
    {
        return Err(PublicError::validation(format!(
            "read-cache directory must not traverse an untrusted macOS system alias: {}",
            alias.display()
        )));
    }
    Ok(absolute_target.join(suffix))
}

#[cfg(not(target_os = "macos"))]
fn canonicalize_trusted_macos_system_alias(path: PathBuf) -> PublicResult<PathBuf> {
    Ok(path)
}

fn filesystem_root(path: &Path) -> PublicResult<PathBuf> {
    let mut root = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir | Component::Normal(_) => break,
        }
    }
    if root.as_os_str().is_empty() {
        return Err(cache_io("failed to resolve read-cache filesystem root"));
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

#[cfg(unix)]
fn secure_new_directory(directory: &Dir) -> PublicResult<()> {
    let file = open_operable_directory_handle(directory)
        .map_err(|error| cache_io(format!("failed to secure read-cache directory: {error}")))?;
    file.set_permissions(fs::Permissions::from_mode(0o700))
        .map_err(|error| cache_io(format!("failed to secure read-cache directory: {error}")))?;
    validate_private_directory(directory)
}

#[cfg(not(unix))]
fn secure_new_directory(directory: &Dir) -> PublicResult<()> {
    validate_private_directory(directory)
}

#[cfg(unix)]
fn validate_private_directory(directory: &Dir) -> PublicResult<()> {
    let file = open_operable_directory_handle(directory)
        .map_err(|error| cache_io(format!("failed to inspect read-cache directory: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| cache_io(format!("failed to inspect read-cache directory: {error}")))?;
    validate_effective_owner(&metadata, "read-cache directory")?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::validation(
            "read-cache directory permissions are too broad; require mode 0700 or stricter",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_private_directory(directory: &Dir) -> PublicResult<()> {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    let file = directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .map_err(|error| cache_io(format!("failed to inspect read-cache directory: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| cache_io(format!("failed to inspect read-cache directory: {error}")))?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(PublicError::validation(
            "read-cache directory must be a directory and not a reparse point",
        ));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn validate_private_directory(_directory: &Dir) -> PublicResult<()> {
    Ok(())
}

#[cfg(unix)]
fn secure_new_file(file: &File, label: &str) -> PublicResult<()> {
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|error| cache_io(format!("failed to secure {label}: {error}")))?;
    validate_private_file(file, label)
}

#[cfg(not(unix))]
fn secure_new_file(file: &File, label: &str) -> PublicResult<()> {
    validate_private_file(file, label)
}

#[cfg(unix)]
fn validate_private_file(file: &File, label: &str) -> PublicResult<()> {
    let metadata = file
        .metadata()
        .map_err(|error| cache_io(format!("failed to inspect {label}: {error}")))?;
    if !metadata.is_file() {
        return Err(PublicError::validation(format!(
            "{label} must be a regular file"
        )));
    }
    validate_effective_owner(&metadata, label)?;
    if metadata.nlink() != 1 {
        return Err(PublicError::validation(format!(
            "{label} must have exactly one hard link"
        )));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(PublicError::validation(format!(
            "{label} permissions are too broad; require mode 0600 or stricter"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_private_file(file: &File, label: &str) -> PublicResult<()> {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    let metadata = file
        .metadata()
        .map_err(|error| cache_io(format!("failed to inspect {label}: {error}")))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(PublicError::validation(format!(
            "{label} must be a regular file and not a reparse point"
        )));
    }
    if metadata.number_of_links() != Some(1) {
        return Err(PublicError::validation(format!(
            "{label} must have exactly one hard link"
        )));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn validate_private_file(file: &File, label: &str) -> PublicResult<()> {
    let metadata = file
        .metadata()
        .map_err(|error| cache_io(format!("failed to inspect {label}: {error}")))?;
    if !metadata.is_file() {
        return Err(PublicError::validation(format!(
            "{label} must be a regular file"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn validate_effective_owner(metadata: &fs::Metadata, label: &str) -> PublicResult<()> {
    // SAFETY: `geteuid` has no preconditions and does not dereference pointers.
    let effective_uid = unsafe { libc::geteuid() };
    if metadata.uid() != effective_uid {
        return Err(PublicError::validation(format!(
            "{label} must be owned by the current effective user"
        )));
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "redox")))]
fn open_operable_directory_handle(directory: &Dir) -> std::io::Result<File> {
    // `cap_std::fs::Dir` uses `O_PATH` on Linux. Cloning that descriptor and
    // calling `fchmod` or `fsync` fails with `EBADF`. `Dir::open_with(".")`
    // also preserves that descriptor because cap-std special-cases `.` during
    // capability resolution, so ask the kernel to reopen the held directory
    // directly. This remains handle-relative and cannot redirect through an
    // ambient path.
    let descriptor = rustix::fs::openat(
        directory,
        ".",
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?;
    Ok(File::from(descriptor))
}

#[cfg(target_os = "redox")]
fn open_operable_directory_handle(directory: &Dir) -> std::io::Result<File> {
    directory.try_clone().map(cap_std::fs::Dir::into_std_file)
}

#[cfg(unix)]
fn sync_directory(directory: &Dir) -> PublicResult<()> {
    open_operable_directory_handle(directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            cache_io(format!(
                "failed to synchronize read-cache directory: {error}"
            ))
        })
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Dir) -> PublicResult<()> {
    // Windows does not expose a portable directory-fsync primitive. The
    // temporary cache file itself is synchronized before atomic replacement.
    Ok(())
}

fn cache_path_error(path: &Path, error: std::io::Error) -> PublicError {
    if error.kind() == std::io::ErrorKind::NotADirectory
        || error.kind() == std::io::ErrorKind::InvalidInput
        || is_symlink_loop_error(&error)
    {
        PublicError::validation(format!(
            "read-cache directory must not traverse symlinks or non-directory components: {}",
            path.display()
        ))
    } else {
        cache_io(format!(
            "failed to open read-cache directory {}: {error}",
            path.display()
        ))
    }
}

fn cache_relative_open_error(label: &str, error: std::io::Error) -> PublicError {
    if error.kind() == std::io::ErrorKind::InvalidInput || is_symlink_loop_error(&error) {
        PublicError::validation(format!("{label} must not be a symlink"))
    } else {
        cache_io(format!("failed to open {label}: {error}"))
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

fn cache_io(message: impl Into<String>) -> PublicError {
    PublicError::unexpected(message.into())
}

fn cache_state_error() -> PublicError {
    PublicError::unexpected("read-cache invocation state lock is unavailable")
}

#[cfg(not(test))]
fn with_current_cache_identity<T>(
    expected: &Credentials,
    action: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> PublicResult<T> {
    with_current_credential_identity(expected, action)
}

#[cfg(test)]
fn with_current_cache_identity<T>(
    expected: &Credentials,
    action: impl FnOnce(&Credentials) -> PublicResult<T>,
) -> PublicResult<T> {
    action(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use chrono::TimeDelta;
    use sealtask_client_api::{MembershipResponse, TaskResponse};
    use sealtask_client_crypto::{KEY_SIZE, SealedPayload};
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn test_credentials(expired: bool) -> Credentials {
        let data_key_ciphertext = STANDARD_NO_PAD.encode(
            SealedPayload::new(vec![0x51; 48])
                .to_bytes()
                .expect("encode data-key binding fixture"),
        );
        Credentials {
            api_url: "https://api.example.test/".to_string(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            access_expires_at: if expired {
                Utc::now() - TimeDelta::hours(2)
            } else {
                Utc::now() + TimeDelta::hours(1)
            },
            refresh_expires_at: if expired {
                Utc::now() - TimeDelta::hours(1)
            } else {
                Utc::now() + TimeDelta::hours(2)
            },
            user_id: Uuid::now_v7(),
            email: "operator@example.test".to_string(),
            data_key_ciphertext,
        }
    }

    fn private_temp_dir() -> tempfile::TempDir {
        let directory = tempfile::Builder::new()
            .prefix(".sealtask-read-cache-test-")
            .tempdir_in(".")
            .expect("private test directory");
        #[cfg(unix)]
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
            .expect("secure test directory");
        directory
    }

    #[cfg(unix)]
    #[test]
    fn secures_validates_and_syncs_directory_through_capability_handle() {
        let root = tempfile::tempdir().expect("temporary root directory");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))
            .expect("secure root directory");
        let directory_path = root.path().join("cache");
        fs::create_dir(&directory_path).expect("create cache directory");
        fs::set_permissions(&directory_path, fs::Permissions::from_mode(0o755))
            .expect("broaden cache directory");
        let root_capability =
            Dir::open_ambient_dir(root.path(), ambient_authority()).expect("open root capability");
        let directory = root_capability
            .open_dir_nofollow(Path::new("cache"))
            .expect("open cache directory without following links");

        secure_new_directory(&directory).expect("secure cache directory");
        validate_private_directory(&directory).expect("validate cache directory");
        sync_directory(&directory).expect("sync cache directory");

        let mode = fs::metadata(&directory_path)
            .expect("inspect secured cache directory")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    fn record_fixture<T: Serialize>(
        runtime: &ReadCacheRuntime,
        credentials: &Credentials,
        data_key: &SymmetricKey,
        query: &ReadCacheQuery,
        value: &T,
    ) {
        let guard = runtime
            .begin_online_read(credentials)
            .expect("begin online read");
        runtime
            .record_online(guard.as_ref(), data_key, query, value)
            .expect("record online snapshot");
    }

    fn work_list_fixture(id: Uuid) -> WorkListResponse {
        let now = Utc::now();
        WorkListResponse {
            id,
            owner_user_id: Uuid::now_v7(),
            workspace_id: Uuid::now_v7(),
            title_ciphertext: String::new(),
            description_ciphertext: None,
            payload_ciphertext: String::new(),
            timezone: "UTC".to_string(),
            section_snapshots: Vec::new(),
            created_at: now,
            updated_at: now,
            archived_at: None,
            task_references_enabled_at: None,
            current_task_reference_scheme_revision: None,
            current_task_reference_scheme_revision_id: None,
            membership: MembershipResponse {
                id: Uuid::now_v7(),
                user_id: Uuid::now_v7(),
                user_email: "operator@example.test".to_string(),
                user_name: "Operator".to_string(),
                user_avatar_color: "#000000".to_string(),
                role: "owner".to_string(),
                status: "active".to_string(),
                work_list_key_ciphertext: String::new(),
                recipient_ciphertext: None,
                invite_package_ciphertext: None,
                salt_member: None,
                expires_at: None,
                joined_at: now,
                payload_binding_key: None,
            },
        }
    }

    fn task_reference_scheme_fixture(
        work_list_id: Uuid,
        revision: i64,
    ) -> TaskReferenceSchemeResponse {
        TaskReferenceSchemeResponse {
            scheme_revision_id: Uuid::now_v7(),
            work_list_id,
            revision,
            payload_ciphertext: format!("sealed-{revision}"),
            is_repair: revision > 32,
            created_at: Utc::now(),
            retired_at: None,
            quarantined_at: None,
            quarantined_by_membership_id: None,
        }
    }

    fn task_reference_detail_fixture(
        work_list_id: Uuid,
        reference_number: i64,
    ) -> TaskDetailResponse {
        let now = Utc::now();
        TaskDetailResponse {
            task: TaskResponse {
                id: Uuid::now_v7(),
                work_list_id,
                created_by_membership_id: Uuid::now_v7(),
                title_ciphertext: "sealed-title".to_string(),
                payload_ciphertext: "sealed-payload".to_string(),
                section_id: None,
                priority: None,
                position: "a".to_string(),
                due_at: None,
                start_at: None,
                completed_at: None,
                archived_at: None,
                is_completed: false,
                recurrence_id: None,
                recurrence_schedule: None,
                recurrence_iteration: None,
                materialized_at: None,
                created_at: now,
                updated_at: now,
                comment_count: 0,
                reference_number: Some(reference_number),
                delegations: Vec::new(),
            },
            comments: Vec::new(),
        }
    }

    #[test]
    fn disabled_options_are_online_and_do_not_touch_disk() {
        let runtime = ReadCacheRuntime::new(ReadCacheOptions::disabled());
        let status = runtime.status().expect("status");
        assert!(!status.enabled);
        assert!(!status.present);
        assert_eq!(status.mode, ReadCacheMode::Online);
    }

    #[test]
    fn document_rejects_unsorted_or_duplicate_keys() {
        let now = Utc::now();
        let document = ReadCacheDocumentV1 {
            schema_version: CACHE_SCHEMA_VERSION,
            generation: 0,
            created_at: now,
            updated_at: now,
            entries: vec![
                ReadCacheEntryV1 {
                    key: "z".to_string(),
                    captured_at: now,
                    payload_json: b"null".to_vec(),
                },
                ReadCacheEntryV1 {
                    key: "a".to_string(),
                    captured_at: now,
                    payload_json: b"null".to_vec(),
                },
            ],
        };
        assert!(document.validate().is_err());
    }

    #[test]
    fn document_rejects_a_known_query_with_an_incompatible_payload_schema() {
        let now = Utc::now();
        let document = ReadCacheDocumentV1 {
            schema_version: CACHE_SCHEMA_VERSION,
            generation: 0,
            created_at: now,
            updated_at: now,
            entries: vec![ReadCacheEntryV1 {
                key: ReadCacheQuery::WorkLists {
                    include_archived: false,
                }
                .key(),
                captured_at: now,
                payload_json: b"{}".to_vec(),
            }],
        };
        assert!(document.validate().is_err());
    }

    #[test]
    fn task_reference_scheme_query_is_project_bound_and_bounded() {
        let work_list_id = Uuid::now_v7();
        let query = ReadCacheQuery::TaskReferenceSchemes { work_list_id };
        assert_eq!(
            ReadCacheQuery::parse(&query.key()).expect("parse task-reference cache query"),
            query
        );

        let valid = vec![task_reference_scheme_fixture(work_list_id, 1)];
        validate_snapshot_schema(
            &query.key(),
            &serde_json::to_vec(&valid).expect("encode valid scheme history"),
        )
        .expect("validate matching scheme history");

        let wrong_project = vec![task_reference_scheme_fixture(Uuid::now_v7(), 1)];
        assert!(
            validate_snapshot_schema(
                &query.key(),
                &serde_json::to_vec(&wrong_project).expect("encode mismatched scheme history"),
            )
            .is_err()
        );

        let oversized = (1..=TASK_REFERENCE_REVISION_MAX + 1)
            .map(|revision| task_reference_scheme_fixture(work_list_id, revision))
            .collect::<Vec<_>>();
        assert!(
            validate_snapshot_schema(
                &query.key(),
                &serde_json::to_vec(&oversized).expect("encode oversized scheme history"),
            )
            .is_err()
        );
    }

    #[test]
    fn task_reference_scheme_history_round_trips_offline() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x5d; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let query = ReadCacheQuery::TaskReferenceSchemes { work_list_id };
        let expected = vec![task_reference_scheme_fixture(work_list_id, 1)];
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online cache options"),
        );
        record_fixture(&online, &credentials, &data_key, &query, &expected);

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline cache options"),
        );
        let actual: Vec<TaskReferenceSchemeResponse> = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("read cached task-reference history");
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].work_list_id, work_list_id);
        assert_eq!(actual[0].revision, 1);
    }

    #[test]
    fn task_reference_lookup_query_is_numeric_project_bound_and_schema_checked() {
        let work_list_id = Uuid::now_v7();
        let query = ReadCacheQuery::TaskByReferenceNumber {
            work_list_id,
            reference_number: 184,
        };
        let key = query.key();
        assert_eq!(key, format!("work_list/{work_list_id}/task_reference/184"));
        assert!(!key.contains("OPS"));
        assert_eq!(
            ReadCacheQuery::parse(&key).expect("parse task-reference lookup cache query"),
            query
        );
        assert!(
            ReadCacheQuery::parse(&format!(
                "work_list/{work_list_id}/task_reference/{}",
                TASK_REFERENCE_SAFE_INTEGER_MAX + 1
            ))
            .is_err()
        );

        let valid = task_reference_detail_fixture(work_list_id, 184);
        validate_snapshot_schema(
            &key,
            &serde_json::to_vec(&valid).expect("encode valid task-reference detail"),
        )
        .expect("validate matching task-reference detail");

        let wrong_project = task_reference_detail_fixture(Uuid::now_v7(), 184);
        assert!(
            validate_snapshot_schema(
                &key,
                &serde_json::to_vec(&wrong_project)
                    .expect("encode wrong-project task-reference detail"),
            )
            .is_err()
        );

        let wrong_number = task_reference_detail_fixture(work_list_id, 185);
        assert!(
            validate_snapshot_schema(
                &key,
                &serde_json::to_vec(&wrong_number)
                    .expect("encode wrong-number task-reference detail"),
            )
            .is_err()
        );
    }

    #[test]
    fn task_reference_lookup_round_trips_offline_without_a_prefix_key() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x6d; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let query = ReadCacheQuery::TaskByReferenceNumber {
            work_list_id,
            reference_number: 184,
        };
        let expected = task_reference_detail_fixture(work_list_id, 184);
        let expected_task_id = expected.task.id;
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online cache options"),
        );
        record_fixture(&online, &credentials, &data_key, &query, &expected);

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline cache options"),
        );
        let actual: TaskDetailResponse = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("read cached task-reference detail");
        assert_eq!(actual.task.id, expected_task_id);
        assert_eq!(actual.task.work_list_id, work_list_id);
        assert_eq!(actual.task.reference_number, Some(184));
    }

    #[test]
    fn older_same_query_response_cannot_overwrite_a_newer_snapshot() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x19; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let runtime = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online options"),
        );
        let mut older = runtime
            .begin_online_read(&credentials)
            .expect("older read guard")
            .expect("enabled cache guard");
        let mut newer = runtime
            .begin_online_read(&credentials)
            .expect("newer read guard")
            .expect("enabled cache guard");
        let base = Utc::now();
        older.started_at = base;
        newer.started_at = base + TimeDelta::seconds(1);
        let older_id = Uuid::now_v7();
        let newer_id = Uuid::now_v7();

        runtime
            .record_online(
                Some(&newer),
                &data_key,
                &query,
                &vec![work_list_fixture(newer_id)],
            )
            .expect("record newer snapshot");
        runtime
            .record_online(
                Some(&older),
                &data_key,
                &query,
                &vec![work_list_fixture(older_id)],
            )
            .expect("ignore older snapshot");

        let memoized: Vec<WorkListResponse> = runtime
            .memoized(&credentials, &query)
            .expect("memo lookup")
            .expect("memo snapshot");
        assert_eq!(memoized[0].id, newer_id);

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline options"),
        );
        let persisted: Vec<WorkListResponse> = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("persistent snapshot");
        assert_eq!(persisted[0].id, newer_id);
    }

    #[test]
    fn persistent_generation_rotation_rejects_an_old_cache_even_if_removal_failed() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x1a; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        let location = CacheLocation::open(directory.path(), false)
            .expect("open cache location")
            .expect("cache location exists");
        location
            .acquire_lock()
            .expect("lock cache")
            .rotate_generation()
            .expect("rotate generation");

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline options"),
        );
        assert!(
            offline
                .read_offline::<Vec<WorkListResponse>>(&credentials, &data_key, &query)
                .is_err()
        );
    }

    #[test]
    fn explicit_clear_repairs_a_truncated_generation_lock() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x1c; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let runtime = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online options"),
        );
        record_fixture(
            &runtime,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        fs::write(directory.path().join(CACHE_LOCK_FILE_NAME), [0x42; 3])
            .expect("truncate generation metadata");

        assert!(runtime.clear().expect("explicit clear repairs lock"));
        assert!(!runtime.status().expect("cache status").present);

        record_fixture(
            &runtime,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline options"),
        );
        let value: Vec<WorkListResponse> = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("repopulated cache after repaired clear");
        assert!(value.is_empty());
    }

    #[test]
    fn cache_notices_are_deduplicated_and_bounded() {
        let runtime = ReadCacheRuntime::new(ReadCacheOptions::disabled());
        for index in 0..(MAX_CACHE_NOTICES * 2) {
            runtime
                .push_notice(ReadCacheNotice {
                    code: "test_notice",
                    message: format!("notice {index}"),
                })
                .expect("push notice");
        }
        runtime
            .push_notice(ReadCacheNotice {
                code: "test_notice",
                message: "notice 0".to_string(),
            })
            .expect("push duplicate notice");

        let notices = runtime.take_notices();
        assert_eq!(notices.len(), MAX_CACHE_NOTICES);
        assert_eq!(
            notices.last().map(|notice| notice.code),
            Some("read_cache_notices_suppressed")
        );
    }

    #[test]
    fn ambiguous_mutation_result_invalidates_invocation_and_persistent_cache() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x1b; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let runtime = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online options"),
        );
        record_fixture(
            &runtime,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        let result: PublicResult<()> =
            Err(PublicError::outcome_ambiguous("test mutation", "unknown"));
        runtime.invalidate_for_mutation_result(&result);

        assert!(
            runtime
                .memoized::<Vec<WorkListResponse>>(&credentials, &query)
                .expect("memo lookup")
                .is_none()
        );
        assert!(!runtime.status().expect("cache status").present);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cache_accepts_root_owned_macos_var_and_tmp_aliases() {
        assert_eq!(
            absolute_normalized_path(Path::new("/var/example")).expect("normalize /var"),
            Path::new("/private/var/example")
        );
        assert_eq!(
            absolute_normalized_path(Path::new("/tmp/example")).expect("normalize /tmp"),
            Path::new("/private/tmp/example")
        );

        let directory = tempfile::TempDir::new().expect("standard macOS temporary directory");
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
            .expect("secure temporary directory");
        let config_directory = directory.path().join("config");
        assert!(
            CacheLocation::open(&config_directory, true)
                .expect("create cache through trusted system alias")
                .is_some()
        );
    }

    #[test]
    fn online_population_is_read_by_fresh_explicit_offline_runtime() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x27; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        assert!(online.take_notices().is_empty());

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline options"),
        );
        let value: Vec<WorkListResponse> = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("fresh runtime decrypts persisted snapshot without an HTTP client");
        assert!(value.is_empty());
        let snapshots = offline.take_snapshots();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].query, query.key());
    }

    #[test]
    fn fresh_online_runtime_never_loads_persistent_snapshot_as_a_fallback() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x28; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let first = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &first,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );

        let fresh_online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        let value: Option<Vec<WorkListResponse>> = fresh_online
            .memoized(&credentials, &query)
            .expect("memo lookup");
        assert!(
            value.is_none(),
            "online mode only consults invocation memory and must reach the API on a miss"
        );
    }

    #[test]
    fn disabled_options_bypass_invocation_memo_for_sdk_compatibility() {
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x29; KEY_SIZE]);
        let query = ReadCacheQuery::Task {
            work_list_id: Uuid::now_v7(),
            task_id: Uuid::now_v7(),
        };
        let runtime = ReadCacheRuntime::new(ReadCacheOptions::disabled());
        record_fixture(&runtime, &credentials, &data_key, &query, &vec![7_u8, 8]);
        let value: Option<Vec<u8>> = runtime.memoized(&credentials, &query).expect("memo");
        assert_eq!(value, None);
    }

    #[test]
    fn enabled_online_cache_binding_failure_is_a_warning_and_cache_miss() {
        let directory = private_temp_dir();
        let mut credentials = test_credentials(false);
        credentials.data_key_ciphertext = STANDARD_NO_PAD.encode(b"data-key-binding");
        let data_key = SymmetricKey::new([0x2f; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("online options"),
        );

        assert!(
            online
                .memoized::<Vec<WorkListResponse>>(&credentials, &query)
                .expect("malformed online binding degrades to a miss")
                .is_none()
        );
        let guard = online
            .begin_online_read(&credentials)
            .expect("malformed online binding does not block the API read");
        assert!(guard.is_none());
        online
            .record_online(
                guard.as_ref(),
                &data_key,
                &query,
                &Vec::<WorkListResponse>::new(),
            )
            .expect("missing guard is a deliberate cache no-op");

        let notices = online.take_notices();
        assert_eq!(
            notices.len(),
            1,
            "equivalent setup failures are deduplicated"
        );
        assert_eq!(notices[0].code, "read_cache_unavailable");

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline options"),
        );
        assert!(
            offline
                .memoized::<Vec<WorkListResponse>>(&credentials, &query)
                .is_err(),
            "offline cache identity failures remain strict"
        );
        assert!(
            offline.verify(&credentials, &data_key).is_err(),
            "explicit verification remains strict"
        );
    }

    #[test]
    fn invocation_memo_never_crosses_an_online_account_or_key_binding_change() {
        let directory = private_temp_dir();
        let first_credentials = test_credentials(false);
        let mut second_credentials = first_credentials.clone();
        second_credentials.user_id = Uuid::now_v7();
        let mut rotated_credentials = first_credentials.clone();
        rotated_credentials.data_key_ciphertext = STANDARD_NO_PAD.encode(
            SealedPayload::new(vec![0x77; 48])
                .to_bytes()
                .expect("encode rotated data-key binding fixture"),
        );
        let data_key = SymmetricKey::new([0x39; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let runtime = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &runtime,
            &first_credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );

        assert!(
            runtime
                .memoized::<Vec<WorkListResponse>>(&second_credentials, &query)
                .expect("second-account lookup")
                .is_none(),
            "a user-id change must clear invocation memory before an online lookup"
        );

        record_fixture(
            &runtime,
            &first_credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        assert!(
            runtime
                .memoized::<Vec<WorkListResponse>>(&rotated_credentials, &query)
                .expect("rotated-key lookup")
                .is_none(),
            "a data-key-ciphertext rotation must clear invocation memory"
        );
    }

    #[test]
    fn expired_credentials_are_valid_for_explicit_offline_cache_reads() {
        let directory = private_temp_dir();
        let mut credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x2a; KEY_SIZE]);
        let query = ReadCacheQuery::Notes {
            work_list_id: Uuid::now_v7(),
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<NoteResponse>::new(),
        );
        credentials.access_expires_at = Utc::now() - TimeDelta::hours(2);
        credentials.refresh_expires_at = Utc::now() - TimeDelta::hours(1);

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("options"),
        );
        let value: Vec<NoteResponse> = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("offline cache binding does not impose token expiry");
        assert!(value.is_empty());
    }

    #[test]
    fn cross_profile_and_cross_account_cache_open_are_rejected() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x2b; KEY_SIZE]);
        let query = ReadCacheQuery::Comments {
            work_list_id: Uuid::now_v7(),
            task_id: Uuid::now_v7(),
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<CommentResponse>::new(),
        );

        let other_profile = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "other").expect("options"),
        );
        assert!(
            other_profile
                .read_offline::<Vec<CommentResponse>>(&credentials, &data_key, &query)
                .is_err()
        );

        let other_account = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("options"),
        );
        let mut other_credentials = credentials;
        other_credentials.user_id = Uuid::now_v7();
        assert!(
            other_account
                .read_offline::<Vec<CommentResponse>>(&other_credentials, &data_key, &query)
                .is_err()
        );
    }

    #[test]
    fn cache_tampering_is_fatal_offline() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x2c; KEY_SIZE]);
        let query = ReadCacheQuery::Notes {
            work_list_id: Uuid::now_v7(),
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<NoteResponse>::new(),
        );
        let already_loaded = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("offline options"),
        );
        let _: Vec<NoteResponse> = already_loaded
            .read_offline(&credentials, &data_key, &query)
            .expect("load valid cache before tampering");
        fs::write(directory.path().join(CACHE_FILE_NAME), b"tampered").expect("tamper cache frame");
        assert!(
            already_loaded.verify(&credentials, &data_key).is_err(),
            "verification must re-read the persistent cache instead of trusting invocation memory"
        );

        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("options"),
        );
        assert!(
            offline
                .read_offline::<Vec<NoteResponse>>(&credentials, &data_key, &query)
                .is_err()
        );
    }

    #[test]
    fn authoritative_online_read_self_heals_safe_but_invalid_cache_content() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x3c; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        assert!(online.take_notices().is_empty());
        fs::write(
            directory.path().join(CACHE_FILE_NAME),
            b"invalid strongbox frame",
        )
        .expect("corrupt cache");

        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        assert!(
            online
                .take_notices()
                .iter()
                .any(|notice| notice.code == "read_cache_recovered")
        );
        let offline = ReadCacheRuntime::new(
            ReadCacheOptions::offline(directory.path(), "default").expect("options"),
        );
        let recovered: Vec<WorkListResponse> = offline
            .read_offline(&credentials, &data_key, &query)
            .expect("recovered cache is readable");
        assert!(recovered.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn cache_rejects_symlink_hardlink_and_broad_mode_files() {
        let directory = private_temp_dir();
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x2d; KEY_SIZE]);
        let query = ReadCacheQuery::WorkLists {
            include_archived: true,
        };
        let online = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(
            &online,
            &credentials,
            &data_key,
            &query,
            &Vec::<WorkListResponse>::new(),
        );
        let cache_path = directory.path().join(CACHE_FILE_NAME);
        let hardlink_path = directory.path().join("cache-hardlink");
        fs::hard_link(&cache_path, &hardlink_path).expect("hard link");
        assert!(online.status().is_err());
        fs::remove_file(&hardlink_path).expect("remove hard link");

        fs::set_permissions(&cache_path, fs::Permissions::from_mode(0o644))
            .expect("broaden cache mode");
        assert!(online.status().is_err());
        fs::set_permissions(&cache_path, fs::Permissions::from_mode(0o600))
            .expect("restore cache mode");
        fs::remove_file(&cache_path).expect("remove cache");
        symlink("/dev/null", &cache_path).expect("cache symlink");
        assert!(online.status().is_err());
        assert!(
            online
                .clear()
                .expect("explicit clear removes the unsafe cache entry")
        );
        assert!(!cache_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_and_invalidation_failures_are_notices_not_primary_failures() {
        let directory = private_temp_dir();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755))
            .expect("broaden directory mode");
        let credentials = test_credentials(false);
        let data_key = SymmetricKey::new([0x2e; KEY_SIZE]);
        let query = ReadCacheQuery::MyTasks {
            include_completed: false,
        };
        let runtime = ReadCacheRuntime::new(
            ReadCacheOptions::online(directory.path(), "default").expect("options"),
        );
        record_fixture(&runtime, &credentials, &data_key, &query, &vec![42_u8]);
        assert!(
            runtime
                .take_notices()
                .iter()
                .any(|notice| notice.code == "read_cache_generation_unavailable")
        );

        runtime.invalidate_after_mutation();
        assert!(
            runtime
                .take_notices()
                .iter()
                .any(|notice| notice.code == "read_cache_invalidation_failed")
        );
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
            .expect("restore directory mode");
    }
}
