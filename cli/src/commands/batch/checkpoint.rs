use super::input::BATCH_SCHEMA_VERSION;
use crate::output::{CliError, CliResult};
use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
#[cfg(unix)]
use cap_fs_ext::{OpenOptionsExt as _, OpenOptionsSyncExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
#[cfg(unix)]
use cap_std::fs::{DirBuilder, DirBuilderExt as _};
use chrono::{DateTime, Utc};
use fs2::FileExt;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use rustix::fs::{RenameFlags, renameat_with};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use rustix::io::Errno;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::ffi::{OsStr, OsString};
#[cfg(test)]
use std::fs::OpenOptions as StdOpenOptions;
use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(target_os = "macos")]
use std::os::fd::AsRawFd as _;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt as _, PermissionsExt};
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "macos")]
use std::ptr::NonNull;
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const MAX_CHECKPOINT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CHECKPOINT_OPERATIONS: usize = 10_000;
const CHECKPOINT_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const CHECKPOINT_LOCK_RETRY: Duration = Duration::from_millis(20);

pub(super) struct CheckpointStore {
    checkpoint: CheckpointFile,
    writer: Arc<StdMutex<CheckpointWriter>>,
}

struct CheckpointWriter {
    location: CheckpointLocation,
    file: Option<File>,
    lock: Option<File>,
    checkpoint: CheckpointFile,
    bytes_written: u64,
    failed: bool,
}

struct CheckpointLocation {
    directory: Dir,
    file_name: OsString,
    lock_name: OsString,
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OperationKind {
    TaskCreate,
    TaskUpdate,
}

#[derive(Clone)]
pub(super) struct StartedMetadata {
    pub(super) kind: OperationKind,
    pub(super) project_id: Uuid,
    pub(super) task_id: Option<Uuid>,
    pub(super) expected_updated_at: Option<DateTime<Utc>>,
    pub(super) change_commitment: Option<String>,
}

pub(super) enum ResumeState {
    Absent,
    Started(StartedMetadata),
    Succeeded {
        kind: OperationKind,
        project_id: Uuid,
        task_id: Uuid,
        updated_at: DateTime<Utc>,
    },
    Failed(StartedMetadata),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CheckpointFile {
    schema_version: u64,
    input_sha256: String,
    operations: BTreeMap<String, CheckpointEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum CheckpointEntry {
    Started {
        kind: OperationKind,
        project_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_updated_at: Option<DateTime<Utc>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        change_commitment: Option<String>,
    },
    Succeeded {
        kind: OperationKind,
        project_id: Uuid,
        task_id: Uuid,
        updated_at: DateTime<Utc>,
    },
    Failed {
        kind: OperationKind,
        project_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        task_id: Option<Uuid>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "recordType",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum CheckpointRecord {
    Header {
        schema_version: u64,
        input_sha256: String,
    },
    Operation {
        operation_key: String,
        entry: CheckpointEntry,
    },
}

impl CheckpointStore {
    pub(super) fn open(path: &Path, input_sha256: &str, resume: bool) -> CliResult<Self> {
        ensure_durable_checkpoint_platform()?;
        validate_hash(input_sha256, "input SHA-256")?;
        let file_name = path
            .file_name()
            .ok_or_else(|| CliError::checkpoint_conflict("checkpoint path must name a file"))?
            .to_os_string();
        let parent_path = absolute_normalized_path(&checkpoint_parent(path)?)?;
        let directory = prepare_checkpoint_parent(&parent_path)?;
        let location = CheckpointLocation {
            lock_name: lock_name(&file_name),
            directory,
            file_name,
        };
        let lock = acquire_lock(&location)?;

        let existing = read_checkpoint(&location)?;
        let replace_existing = existing.is_some();
        let checkpoint = match (resume, existing) {
            (true, Some(checkpoint)) => {
                validate_checkpoint(&checkpoint, input_sha256)?;
                checkpoint
            }
            (true, None) => {
                return Err(CliError::checkpoint_conflict(
                    "--resume requires an existing checkpoint",
                ));
            }
            (false, Some(_)) => {
                return Err(CliError::checkpoint_conflict(
                    "checkpoint already exists; pass --resume only with the exact original input, or choose a new checkpoint path",
                ));
            }
            (false, None) => CheckpointFile {
                schema_version: BATCH_SCHEMA_VERSION,
                input_sha256: input_sha256.to_string(),
                operations: BTreeMap::new(),
            },
        };
        validate_checkpoint(&checkpoint, input_sha256)?;
        write_compacted_checkpoint(&location, &checkpoint, replace_existing)?;
        let file = open_checkpoint_for_append(&location)?;
        let bytes_written = file
            .metadata()
            .map_err(|error| checkpoint_io(format!("failed to inspect checkpoint: {error}")))?
            .len();
        let writer_checkpoint = checkpoint.clone();

        Ok(Self {
            checkpoint,
            writer: Arc::new(StdMutex::new(CheckpointWriter {
                location,
                file: Some(file),
                lock: Some(lock),
                checkpoint: writer_checkpoint,
                bytes_written,
                failed: false,
            })),
        })
    }

    pub(super) fn resume_state(&self, operation_key: &str) -> CliResult<ResumeState> {
        validate_hash(operation_key, "operation checkpoint key")?;
        Ok(match self.checkpoint.operations.get(operation_key) {
            None => ResumeState::Absent,
            Some(CheckpointEntry::Started {
                kind,
                project_id,
                task_id,
                expected_updated_at,
                change_commitment,
            }) => ResumeState::Started(StartedMetadata {
                kind: *kind,
                project_id: *project_id,
                task_id: *task_id,
                expected_updated_at: *expected_updated_at,
                change_commitment: change_commitment.clone(),
            }),
            Some(CheckpointEntry::Succeeded {
                kind,
                project_id,
                task_id,
                updated_at,
            }) => ResumeState::Succeeded {
                kind: *kind,
                project_id: *project_id,
                task_id: *task_id,
                updated_at: *updated_at,
            },
            Some(CheckpointEntry::Failed {
                kind,
                project_id,
                task_id,
            }) => ResumeState::Failed(StartedMetadata {
                kind: *kind,
                project_id: *project_id,
                task_id: *task_id,
                expected_updated_at: None,
                change_commitment: None,
            }),
        })
    }

    pub(super) fn validate_operation_keys(
        &self,
        canonical_keys: &std::collections::HashSet<String>,
    ) -> CliResult<()> {
        if self
            .checkpoint
            .operations
            .keys()
            .any(|key| !canonical_keys.contains(key))
        {
            return Err(CliError::checkpoint_conflict(
                "checkpoint contains an operation that is not present in canonical batch input",
            ));
        }
        Ok(())
    }

    pub(super) async fn record_started(
        &self,
        operation_key: String,
        metadata: &StartedMetadata,
    ) -> CliResult<()> {
        validate_hash(&operation_key, "operation checkpoint key")?;
        let entry = CheckpointEntry::Started {
            kind: metadata.kind,
            project_id: metadata.project_id,
            task_id: metadata.task_id,
            expected_updated_at: metadata.expected_updated_at,
            change_commitment: metadata.change_commitment.clone(),
        };
        validate_entry(&entry)?;
        self.append(operation_key, entry).await
    }

    pub(super) async fn record_succeeded(
        &self,
        operation_key: String,
        kind: OperationKind,
        project_id: Uuid,
        task_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> CliResult<()> {
        validate_hash(&operation_key, "operation checkpoint key")?;
        let entry = CheckpointEntry::Succeeded {
            kind,
            project_id,
            task_id,
            updated_at,
        };
        validate_entry(&entry)?;
        self.append(operation_key, entry).await
    }

    pub(super) async fn record_failed(
        &self,
        operation_key: String,
        kind: OperationKind,
        project_id: Uuid,
        task_id: Option<Uuid>,
    ) -> CliResult<()> {
        validate_hash(&operation_key, "operation checkpoint key")?;
        let entry = CheckpointEntry::Failed {
            kind,
            project_id,
            task_id,
        };
        validate_entry(&entry)?;
        self.append(operation_key, entry).await
    }

    async fn append(&self, operation_key: String, entry: CheckpointEntry) -> CliResult<()> {
        let writer = Arc::clone(&self.writer);
        tokio::task::spawn_blocking(move || {
            let mut writer = writer
                .lock()
                .map_err(|_| checkpoint_io("checkpoint writer lock is unavailable"))?;
            writer.append(operation_key, entry)
        })
        .await
        .map_err(|_| checkpoint_io("checkpoint writer task did not complete"))?
    }
}

impl CheckpointWriter {
    fn append(&mut self, operation_key: String, entry: CheckpointEntry) -> CliResult<()> {
        if self.failed {
            return Err(checkpoint_io(
                "checkpoint writer is unavailable after an earlier durability failure",
            ));
        }
        let result = self.append_inner(operation_key, entry);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn append_inner(&mut self, operation_key: String, entry: CheckpointEntry) -> CliResult<()> {
        let record = CheckpointRecord::Operation {
            operation_key: operation_key.clone(),
            entry: entry.clone(),
        };
        let encoded = encode_record(&record)?;
        let next_len = self
            .bytes_written
            .checked_add(encoded.len() as u64)
            .ok_or_else(|| checkpoint_io("checkpoint journal size overflowed"))?;
        if next_len > MAX_CHECKPOINT_BYTES {
            self.checkpoint.operations.insert(operation_key, entry);
            drop(self.file.take());
            write_compacted_checkpoint(&self.location, &self.checkpoint, true)?;
            let file = open_checkpoint_for_append(&self.location)?;
            self.bytes_written = file
                .metadata()
                .map_err(|error| checkpoint_io(format!("failed to inspect checkpoint: {error}")))?
                .len();
            self.file = Some(file);
            return Ok(());
        }
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| checkpoint_io("checkpoint journal is not open"))?;
        file.write_all(&encoded)
            .map_err(|error| checkpoint_io(format!("failed to append checkpoint: {error}")))?;
        file.sync_data()
            .map_err(|error| checkpoint_io(format!("failed to sync checkpoint: {error}")))?;
        self.bytes_written = next_len;
        self.checkpoint.operations.insert(operation_key, entry);
        Ok(())
    }
}

impl Drop for CheckpointWriter {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            let _ = FileExt::unlock(&lock);
        }
    }
}

pub(super) fn reject_input_checkpoint_conflict(
    input: &Path,
    checkpoint: Option<&Path>,
) -> CliResult<()> {
    let Some(checkpoint) = checkpoint else {
        return Ok(());
    };
    if input == Path::new("-") {
        return Ok(());
    }
    let input = canonical_target(input)?;
    let checkpoint = canonical_target(checkpoint)?;
    if input == checkpoint {
        return Err(CliError::checkpoint_conflict(
            "batch input and checkpoint must be different files",
        ));
    }
    Ok(())
}

fn validate_checkpoint(checkpoint: &CheckpointFile, expected_hash: &str) -> CliResult<()> {
    if checkpoint.schema_version != BATCH_SCHEMA_VERSION {
        return Err(CliError::checkpoint_conflict(format!(
            "checkpoint schemaVersion {} is not supported by this CLI",
            checkpoint.schema_version
        )));
    }
    validate_hash(&checkpoint.input_sha256, "checkpoint input SHA-256")?;
    if checkpoint.input_sha256 != expected_hash {
        return Err(CliError::checkpoint_conflict(
            "checkpoint belongs to different canonical batch input",
        ));
    }
    if checkpoint.operations.len() > MAX_CHECKPOINT_OPERATIONS {
        return Err(CliError::checkpoint_conflict(
            "checkpoint contains more than 10000 operations",
        ));
    }
    for (key, entry) in &checkpoint.operations {
        validate_hash(key, "operation checkpoint key")?;
        validate_entry(entry)?;
    }
    Ok(())
}

fn validate_entry(entry: &CheckpointEntry) -> CliResult<()> {
    let (kind, project_id, task_id, expected_updated_at, change_commitment) = match entry {
        CheckpointEntry::Started {
            kind,
            project_id,
            task_id,
            expected_updated_at,
            change_commitment,
            ..
        } => (
            *kind,
            *project_id,
            *task_id,
            *expected_updated_at,
            change_commitment.as_deref(),
        ),
        CheckpointEntry::Succeeded {
            kind,
            project_id,
            task_id,
            ..
        } => (*kind, *project_id, Some(*task_id), None, None),
        CheckpointEntry::Failed {
            kind,
            project_id,
            task_id,
            ..
        } => (*kind, *project_id, *task_id, None, None),
    };
    if project_id.is_nil() || task_id.is_some_and(|id| id.is_nil()) {
        return Err(CliError::checkpoint_conflict(
            "checkpoint contains an invalid canonical entity ID",
        ));
    }
    let valid_target_shape = matches!(
        (entry, kind, task_id),
        (CheckpointEntry::Succeeded { .. }, _, Some(_))
            | (
                CheckpointEntry::Started { .. } | CheckpointEntry::Failed { .. },
                OperationKind::TaskCreate,
                None,
            )
            | (
                CheckpointEntry::Started { .. } | CheckpointEntry::Failed { .. },
                OperationKind::TaskUpdate,
                Some(_),
            )
    );
    if !valid_target_shape {
        return Err(CliError::checkpoint_conflict(
            "checkpoint operation kind does not match its canonical task target",
        ));
    }
    if let CheckpointEntry::Started { .. } = entry {
        let valid_started_shape = match kind {
            OperationKind::TaskCreate => expected_updated_at.is_none(),
            OperationKind::TaskUpdate => {
                expected_updated_at.is_some() == change_commitment.is_some()
            }
        };
        if !valid_started_shape {
            return Err(CliError::checkpoint_conflict(
                "checkpoint started metadata is incomplete or inconsistent",
            ));
        }
    }
    if let Some(commitment) = change_commitment {
        validate_change_commitment(commitment)?;
    }
    Ok(())
}

fn read_checkpoint(location: &CheckpointLocation) -> CliResult<Option<CheckpointFile>> {
    let mut options = CapOpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.nonblock(true);
    let file = match location
        .directory
        .open_with(Path::new(&location.file_name), &options)
    {
        Ok(file) => file.into_std(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(relative_open_error("checkpoint", error)),
    };
    let opened_metadata = file
        .metadata()
        .map_err(|error| checkpoint_io(format!("failed to inspect checkpoint: {error}")))?;
    if !opened_metadata.is_file() {
        return Err(CliError::checkpoint_conflict(
            "checkpoint must be a regular file",
        ));
    }
    validate_secret_file_handle(&file, &opened_metadata, "checkpoint")?;
    if opened_metadata.len() > MAX_CHECKPOINT_BYTES {
        return Err(CliError::checkpoint_conflict(format!(
            "checkpoint exceeds the {MAX_CHECKPOINT_BYTES}-byte limit"
        )));
    }

    let mut bytes = Vec::with_capacity(
        usize::try_from(opened_metadata.len())
            .map_err(|_| checkpoint_io("checkpoint size does not fit this platform"))?,
    );
    file.take(MAX_CHECKPOINT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| checkpoint_io(format!("failed to read checkpoint: {error}")))?;
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
        return Err(CliError::checkpoint_conflict(format!(
            "checkpoint exceeds the {MAX_CHECKPOINT_BYTES}-byte limit"
        )));
    }
    if let Ok(checkpoint) = serde_json::from_slice::<CheckpointFile>(&bytes) {
        return Ok(Some(checkpoint));
    }
    parse_checkpoint_journal(&bytes).map(Some)
}

fn parse_checkpoint_journal(bytes: &[u8]) -> CliResult<CheckpointFile> {
    let complete_len = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map(|index| index + 1)
        .ok_or_else(|| {
            CliError::checkpoint_conflict("checkpoint journal header is incomplete or corrupt")
        })?;
    let complete = &bytes[..complete_len - 1];
    let mut records = complete.split(|byte| *byte == b'\n');
    let header = records
        .next()
        .ok_or_else(|| CliError::checkpoint_conflict("checkpoint journal is empty"))?;
    let CheckpointRecord::Header {
        schema_version,
        input_sha256,
    } = decode_record(header)?
    else {
        return Err(CliError::checkpoint_conflict(
            "checkpoint journal must begin with its header",
        ));
    };
    let mut checkpoint = CheckpointFile {
        schema_version,
        input_sha256,
        operations: BTreeMap::new(),
    };
    for record in records {
        if record.is_empty() {
            return Err(CliError::checkpoint_conflict(
                "checkpoint journal contains an empty record",
            ));
        }
        let CheckpointRecord::Operation {
            operation_key,
            entry,
        } = decode_record(record)?
        else {
            return Err(CliError::checkpoint_conflict(
                "checkpoint journal contains an unexpected header",
            ));
        };
        validate_hash(&operation_key, "operation checkpoint key")?;
        validate_entry(&entry)?;
        checkpoint.operations.insert(operation_key, entry);
        if checkpoint.operations.len() > MAX_CHECKPOINT_OPERATIONS {
            return Err(CliError::checkpoint_conflict(
                "checkpoint contains more than 10000 operations",
            ));
        }
    }
    Ok(checkpoint)
}

fn decode_record(bytes: &[u8]) -> CliResult<CheckpointRecord> {
    serde_json::from_slice(bytes)
        .map_err(|_| CliError::checkpoint_conflict("checkpoint journal is corrupt"))
}

fn encode_record(record: &CheckpointRecord) -> CliResult<Vec<u8>> {
    let mut encoded = serde_json::to_vec(record)
        .map_err(|error| checkpoint_io(format!("failed to encode checkpoint record: {error}")))?;
    encoded.push(b'\n');
    Ok(encoded)
}

struct RelativeTemporaryFile<'a> {
    directory: &'a Dir,
    name: PathBuf,
    file: Option<File>,
    armed: bool,
}

impl<'a> RelativeTemporaryFile<'a> {
    fn create(directory: &'a Dir) -> CliResult<Self> {
        for _ in 0..8 {
            let name = PathBuf::from(format!(
                ".sealtask-checkpoint-{}.tmp",
                Uuid::now_v7().simple()
            ));
            let mut options = CapOpenOptions::new();
            options
                .create_new(true)
                .read(true)
                .write(true)
                .follow(FollowSymlinks::No);
            #[cfg(unix)]
            options.mode(0o600).nonblock(true);
            match directory.open_with(&name, &options) {
                Ok(file) => {
                    let file = file.into_std();
                    let metadata = file.metadata().map_err(|error| {
                        checkpoint_io(format!("failed to inspect temporary checkpoint: {error}"))
                    })?;
                    if !metadata.is_file() {
                        return Err(CliError::checkpoint_conflict(
                            "temporary checkpoint is not a regular file",
                        ));
                    }
                    set_secret_file_handle_permissions(&file, "temporary checkpoint")?;
                    return Ok(Self {
                        directory,
                        name,
                        file: Some(file),
                        armed: true,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(checkpoint_io(format!(
                        "failed to create temporary checkpoint: {error}"
                    )));
                }
            }
        }
        Err(checkpoint_io(
            "failed to allocate a unique temporary checkpoint",
        ))
    }

    fn file(&self) -> CliResult<&File> {
        self.file
            .as_ref()
            .ok_or_else(|| checkpoint_io("temporary checkpoint was closed before publication"))
    }

    fn file_mut(&mut self) -> CliResult<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| checkpoint_io("temporary checkpoint was closed before publication"))
    }

    fn publish(mut self, location: &CheckpointLocation, replace_existing: bool) -> CliResult<()> {
        drop(self.file.take());
        let target = Path::new(&location.file_name);
        if replace_existing {
            self.directory
                .rename(&self.name, self.directory, target)
                .map_err(|error| {
                    checkpoint_io(format!("failed to atomically replace checkpoint: {error}"))
                })?;
            self.armed = false;
            return Ok(());
        }

        publish_new_checkpoint(self.directory, &self.name, target)?;
        #[cfg(test)]
        exit_after_new_checkpoint_publication_if_requested();
        self.armed = false;
        Ok(())
    }
}

impl Drop for RelativeTemporaryFile<'_> {
    fn drop(&mut self) {
        drop(self.file.take());
        if self.armed {
            let _ = self.directory.remove_file(&self.name);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn publish_new_checkpoint(directory: &Dir, source: &Path, target: &Path) -> CliResult<()> {
    match renameat_with(directory, source, directory, target, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(()),
        Err(Errno::EXIST) => Err(CliError::checkpoint_conflict(
            "checkpoint appeared while the batch was acquiring its durable state",
        )),
        Err(error) if error == Errno::NOSYS || error == Errno::INVAL || error == Errno::NOTSUP => {
            Err(CliError::checkpoint_conflict(
                "this filesystem does not support atomic no-replace checkpoint publication",
            ))
        }
        Err(error) => Err(checkpoint_io(format!(
            "failed to atomically create checkpoint: {error}"
        ))),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn publish_new_checkpoint(_directory: &Dir, _source: &Path, _target: &Path) -> CliResult<()> {
    Err(CliError::checkpoint_conflict(
        "durable batch checkpoints require Linux or macOS",
    ))
}

#[cfg(test)]
fn exit_after_new_checkpoint_publication_if_requested() {
    if std::env::var_os("SEALTASK_TEST_EXIT_AFTER_NEW_CHECKPOINT_PUBLICATION").is_some() {
        std::process::exit(86);
    }
}

fn write_compacted_checkpoint(
    location: &CheckpointLocation,
    checkpoint: &CheckpointFile,
    replace_existing: bool,
) -> CliResult<()> {
    let mut temporary = RelativeTemporaryFile::create(&location.directory)?;
    let header = CheckpointRecord::Header {
        schema_version: checkpoint.schema_version,
        input_sha256: checkpoint.input_sha256.clone(),
    };
    temporary
        .file_mut()?
        .write_all(&encode_record(&header)?)
        .map_err(|error| checkpoint_io(format!("failed to write checkpoint header: {error}")))?;
    for (operation_key, entry) in &checkpoint.operations {
        temporary
            .file_mut()?
            .write_all(&encode_record(&CheckpointRecord::Operation {
                operation_key: operation_key.clone(),
                entry: entry.clone(),
            })?)
            .map_err(|error| {
                checkpoint_io(format!("failed to write compacted checkpoint: {error}"))
            })?;
    }
    let encoded_len = temporary
        .file()?
        .metadata()
        .map_err(|error| checkpoint_io(format!("failed to inspect checkpoint: {error}")))?
        .len();
    if encoded_len > MAX_CHECKPOINT_BYTES {
        return Err(CliError::checkpoint_conflict(format!(
            "checkpoint exceeds the {MAX_CHECKPOINT_BYTES}-byte limit"
        )));
    }
    temporary
        .file()?
        .sync_all()
        .map_err(|error| checkpoint_io(format!("failed to sync checkpoint: {error}")))?;
    temporary.publish(location, replace_existing)?;
    open_checkpoint_for_append(location)?
        .sync_all()
        .map_err(|error| checkpoint_io(format!("failed to sync saved checkpoint: {error}")))?;
    sync_directory_handle(&location.directory)
}

fn open_checkpoint_for_append(location: &CheckpointLocation) -> CliResult<File> {
    let mut options = CapOpenOptions::new();
    options.read(true).append(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    options.nonblock(true);
    let file = location
        .directory
        .open_with(Path::new(&location.file_name), &options)
        .map(cap_std::fs::File::into_std)
        .map_err(|error| relative_open_error("checkpoint", error))?;
    let metadata = file
        .metadata()
        .map_err(|error| checkpoint_io(format!("failed to inspect checkpoint: {error}")))?;
    if !metadata.is_file() {
        return Err(CliError::checkpoint_conflict(
            "checkpoint must be a regular file",
        ));
    }
    validate_secret_file_handle(&file, &metadata, "checkpoint")?;
    Ok(file)
}

fn checkpoint_parent(path: &Path) -> CliResult<PathBuf> {
    if path.file_name().is_none() {
        return Err(CliError::checkpoint_conflict(
            "checkpoint path must name a file",
        ));
    }
    Ok(path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf())
}

fn prepare_checkpoint_parent(parent: &Path) -> CliResult<Dir> {
    prepare_checkpoint_parent_with(parent, |_| Ok(()))
}

fn prepare_checkpoint_parent_with(
    parent: &Path,
    mut synchronized: impl FnMut(&Path) -> CliResult<()>,
) -> CliResult<Dir> {
    let parent = absolute_normalized_path(parent)?;
    let root = filesystem_root(&parent)?;
    // Walk from the filesystem root so every operator-supplied component is
    // opened relative to a held descriptor. This deliberately rejects even
    // system path aliases such as macOS `/tmp` and `/var`; canonicalizing the
    // complete path here would reintroduce a symlink traversal race.
    let mut directory = Dir::open_ambient_dir(&root, ambient_authority()).map_err(|error| {
        checkpoint_io(format!(
            "failed to open checkpoint filesystem root {}: {error}",
            root.display()
        ))
    })?;
    let mut walked = root;

    for component in parent.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        let child_path = walked.join(name);
        let (child, created) = match open_directory_component(&directory, name) {
            Ok(child) => (child, false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                validate_parent_acl_before_creation(&directory, &walked)?;
                let created = match create_directory_component(&directory, name) {
                    Ok(()) => true,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                    Err(error) => {
                        return Err(checkpoint_io(format!(
                            "failed to create checkpoint parent {}: {error}",
                            child_path.display()
                        )));
                    }
                };
                let child = open_directory_component(&directory, name)
                    .map_err(|error| parent_component_error(&child_path, error))?;
                (child, created)
            }
            Err(error) => return Err(parent_component_error(&child_path, error)),
        };

        if created {
            set_secret_directory_handle_permissions(&child)?;
            sync_directory_handle(&child)?;
            sync_directory_handle(&directory)?;
            synchronized(&child_path)?;
            synchronized(&walked)?;
        }
        directory = child;
        walked = child_path;
    }

    let metadata = directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .and_then(|directory| directory.metadata())
        .map_err(|error| {
            checkpoint_io(format!(
                "failed to inspect checkpoint parent {}: {error}",
                parent.display()
            ))
        })?;
    if !metadata.is_dir() {
        return Err(CliError::checkpoint_conflict(
            "checkpoint parent must be a directory",
        ));
    }
    validate_secret_directory_handle(&directory, &metadata)?;
    Ok(directory)
}

fn absolute_normalized_path(path: &Path) -> CliResult<PathBuf> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(CliError::checkpoint_conflict(
            "checkpoint paths must not contain `..`; use an absolute, symlink-free path",
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                checkpoint_io(format!("failed to resolve current directory: {error}"))
            })?
            .join(path)
    };
    let normalized = normalize_path(&absolute);
    if !normalized.is_absolute() {
        return Err(checkpoint_io(
            "failed to resolve checkpoint parent as an absolute path",
        ));
    }
    Ok(normalized)
}

fn filesystem_root(path: &Path) -> CliResult<PathBuf> {
    let mut root = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir | Component::Normal(_) => break,
        }
    }
    if root.as_os_str().is_empty() {
        return Err(checkpoint_io(
            "failed to resolve checkpoint filesystem root",
        ));
    }
    Ok(root)
}

fn open_directory_component(parent: &Dir, name: &OsStr) -> std::io::Result<Dir> {
    parent.open_dir_nofollow(Path::new(name))
}

#[cfg(unix)]
fn create_directory_component(parent: &Dir, name: &OsStr) -> std::io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    parent.create_dir_with(Path::new(name), &builder)
}

#[cfg(not(unix))]
fn create_directory_component(parent: &Dir, name: &OsStr) -> std::io::Result<()> {
    parent.create_dir(Path::new(name))
}

fn parent_component_error(path: &Path, error: std::io::Error) -> CliError {
    if error.kind() == std::io::ErrorKind::NotADirectory
        || error.kind() == std::io::ErrorKind::InvalidInput
        || is_symlink_loop_error(&error)
    {
        CliError::checkpoint_conflict(format!(
            "checkpoint parent must not traverse a symlink, reparse point, or non-directory component: {}",
            path.display()
        ))
    } else {
        checkpoint_io(format!(
            "failed to open checkpoint parent {}: {error}",
            path.display()
        ))
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

fn lock_name(file_name: &OsStr) -> OsString {
    let mut lock_name = OsString::from(file_name);
    lock_name.push(".lock");
    lock_name
}

fn acquire_lock(location: &CheckpointLocation) -> CliResult<File> {
    let lock_path = Path::new(&location.lock_name);
    let mut create_options = CapOpenOptions::new();
    create_options
        .create_new(true)
        .read(true)
        .write(true)
        .follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        create_options.mode(0o600).nonblock(true);
    }
    let (file, created) = match location.directory.open_with(lock_path, &create_options) {
        Ok(file) => (file.into_std(), true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut options = CapOpenOptions::new();
            options.read(true).write(true).follow(FollowSymlinks::No);
            #[cfg(unix)]
            options.nonblock(true);
            let file = location
                .directory
                .open_with(lock_path, &options)
                .map(cap_std::fs::File::into_std)
                .map_err(|error| relative_open_error("checkpoint lock", error))?;
            (file, false)
        }
        Err(error) => return Err(relative_open_error("checkpoint lock", error)),
    };
    let metadata = file
        .metadata()
        .map_err(|error| checkpoint_io(format!("failed to inspect checkpoint lock: {error}")))?;
    if !metadata.is_file() {
        return Err(CliError::checkpoint_conflict(
            "checkpoint lock must be a regular file",
        ));
    }
    if created {
        set_secret_file_handle_permissions(&file, "checkpoint lock")?;
        sync_directory_handle(&location.directory)?;
    } else {
        validate_secret_file_handle(&file, &metadata, "checkpoint lock")?;
    }

    let deadline = Instant::now() + CHECKPOINT_LOCK_TIMEOUT;
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(file),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(CliError::checkpoint_conflict(
                        "checkpoint is locked by another batch process",
                    ));
                }
                thread::sleep(CHECKPOINT_LOCK_RETRY);
            }
            Err(error) => {
                return Err(checkpoint_io(format!("failed to lock checkpoint: {error}")));
            }
        }
    }
}

fn canonical_target(path: &Path) -> CliResult<PathBuf> {
    if path.file_name().is_none() {
        return Err(CliError::checkpoint_conflict("batch path must name a file"));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                checkpoint_io(format!("failed to resolve current directory: {error}"))
            })?
            .join(path)
    };
    let mut cursor = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(cursor) {
            Ok(mut resolved) => {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return Ok(normalize_path(&resolved));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = cursor.file_name().ok_or_else(|| {
                    checkpoint_io("failed to resolve batch path through a missing filesystem root")
                })?;
                missing.push(name.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| checkpoint_io("failed to resolve batch path parent"))?;
            }
            Err(error) => {
                return Err(checkpoint_io(format!(
                    "failed to resolve batch path: {error}"
                )));
            }
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn validate_hash(value: &str, label: &str) -> CliResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CliError::checkpoint_conflict(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_change_commitment(value: &str) -> CliResult<()> {
    if value.len() != 43
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
    {
        return Err(CliError::checkpoint_conflict(
            "checkpoint change commitment is invalid",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn validate_secret_directory_handle(directory: &Dir, metadata: &fs::Metadata) -> CliResult<()> {
    validate_effective_owner(metadata, "checkpoint parent")?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(CliError::checkpoint_conflict(
            "checkpoint parent permissions are too broad; require mode 0700 or stricter",
        ));
    }
    let directory = directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .map_err(|error| {
            checkpoint_io(format!(
                "failed to inspect checkpoint parent security: {error}"
            ))
        })?;
    validate_no_allow_extended_acl(&directory, "checkpoint parent")
}

#[cfg(not(unix))]
fn validate_secret_directory_handle(_directory: &Dir, _metadata: &fs::Metadata) -> CliResult<()> {
    Ok(())
}

#[cfg(unix)]
fn validate_secret_file_handle(file: &File, metadata: &fs::Metadata, label: &str) -> CliResult<()> {
    validate_effective_owner(metadata, label)?;
    if metadata.nlink() != 1 {
        return Err(CliError::checkpoint_conflict(format!(
            "{label} must have exactly one hard link"
        )));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(CliError::checkpoint_conflict(format!(
            "{label} permissions are too broad; require mode 0600 or stricter"
        )));
    }
    validate_no_allow_extended_acl(file, label)
}

#[cfg(not(unix))]
fn validate_secret_file_handle(
    _file: &File,
    _metadata: &fs::Metadata,
    _label: &str,
) -> CliResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_secret_directory_handle_permissions(directory: &Dir) -> CliResult<()> {
    let directory = directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .map_err(|error| checkpoint_io(format!("failed to secure checkpoint parent: {error}")))?;
    directory
        .set_permissions(fs::Permissions::from_mode(0o700))
        .map_err(|error| checkpoint_io(format!("failed to secure checkpoint parent: {error}")))?;
    clear_extended_acl(&directory, "checkpoint parent")?;
    let metadata = directory.metadata().map_err(|error| {
        checkpoint_io(format!(
            "failed to inspect secured checkpoint parent: {error}"
        ))
    })?;
    validate_effective_owner(&metadata, "checkpoint parent")?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(checkpoint_io(
            "failed to restrict checkpoint parent to mode 0700 or stricter",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_secret_directory_handle_permissions(_directory: &Dir) -> CliResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_secret_file_handle_permissions(file: &File, label: &str) -> CliResult<()> {
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|error| checkpoint_io(format!("failed to secure {label}: {error}")))?;
    clear_extended_acl(file, label)?;
    let metadata = file
        .metadata()
        .map_err(|error| checkpoint_io(format!("failed to inspect secured {label}: {error}")))?;
    validate_secret_file_handle(file, &metadata, label)
}

#[cfg(not(unix))]
fn set_secret_file_handle_permissions(_file: &File, _label: &str) -> CliResult<()> {
    Ok(())
}

#[cfg(unix)]
fn validate_effective_owner(metadata: &fs::Metadata, label: &str) -> CliResult<()> {
    // SAFETY: `geteuid` has no preconditions and does not dereference pointers.
    let effective_uid = unsafe { libc::geteuid() };
    validate_effective_owner_ids(metadata.uid(), effective_uid, label)
}

#[cfg(unix)]
fn validate_effective_owner_ids(owner_uid: u32, effective_uid: u32, label: &str) -> CliResult<()> {
    if owner_uid != effective_uid {
        return Err(CliError::checkpoint_conflict(format!(
            "{label} must be owned by the current effective user"
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn validate_parent_acl_before_creation(directory: &Dir, path: &Path) -> CliResult<()> {
    let directory = directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .map_err(|error| {
            checkpoint_io(format!(
                "failed to inspect checkpoint parent {} security: {error}",
                path.display()
            ))
        })?;
    validate_no_allow_extended_acl(&directory, &format!("checkpoint parent {}", path.display()))
}

#[cfg(not(target_os = "macos"))]
fn validate_parent_acl_before_creation(_directory: &Dir, _path: &Path) -> CliResult<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExtendedAclState {
    Empty,
    DenyOnly,
    AllowsAdditionalAccess,
}

#[cfg(target_os = "macos")]
struct MacOsAclObject(NonNull<c_void>);

#[cfg(target_os = "macos")]
impl MacOsAclObject {
    fn as_ptr(&self) -> *mut c_void {
        self.0.as_ptr()
    }
}

#[cfg(target_os = "macos")]
impl Drop for MacOsAclObject {
    fn drop(&mut self) {
        // SAFETY: this object owns a non-null pointer returned by a macOS ACL
        // allocation API, and `acl_free` accepts each such pointer once.
        let _ = unsafe { macos_acl_free(self.as_ptr()) };
    }
}

#[cfg(target_os = "macos")]
fn inspect_extended_acl(file: &File, label: &str) -> CliResult<ExtendedAclState> {
    const ACL_FIRST_ENTRY: libc::c_int = 0;
    const ACL_NEXT_ENTRY: libc::c_int = -1;
    const ACL_EXTENDED_ALLOW: libc::c_int = 1;
    const ACL_EXTENDED_DENY: libc::c_int = 2;
    const ACL_TYPE_EXTENDED: libc::c_int = 0x100;

    // SAFETY: the raw descriptor remains valid for the duration of this call.
    let acl = unsafe { macos_acl_get_fd_np(file.as_raw_fd(), ACL_TYPE_EXTENDED) };
    let Some(acl) = NonNull::new(acl) else {
        let error = std::io::Error::last_os_error();
        // macOS reports ENOENT for a valid descriptor whose object has no
        // extended ACL. Other failures remain fatal so an inspection error
        // can never silently weaken the checkpoint boundary.
        if error.raw_os_error() == Some(libc::ENOENT) {
            return Ok(ExtendedAclState::Empty);
        }
        return Err(checkpoint_io(format!(
            "failed to inspect {label} extended ACL: {error}"
        )));
    };
    let acl = MacOsAclObject(acl);

    // Validate the opaque ACL before treating EINVAL from iteration as the
    // documented end-of-list signal.
    // SAFETY: `acl` owns a valid allocation returned by `acl_get_fd_np`.
    if unsafe { macos_acl_valid(acl.as_ptr()) } != 0 {
        return Err(checkpoint_io(format!(
            "failed to validate {label} extended ACL: {}",
            std::io::Error::last_os_error()
        )));
    }

    let mut text_len: libc::ssize_t = 0;
    // SAFETY: `acl` is valid and `text_len` points to writable storage.
    let text = unsafe { macos_acl_to_text(acl.as_ptr(), &mut text_len) };
    let _text = MacOsAclObject(NonNull::new(text.cast()).ok_or_else(|| {
        checkpoint_io(format!(
            "failed to inspect {label} extended ACL entries: {}",
            std::io::Error::last_os_error()
        ))
    })?);
    if text_len < 0 {
        return Err(checkpoint_io(format!(
            "failed to inspect {label} extended ACL entries"
        )));
    }
    if text_len == 0 {
        return Ok(ExtendedAclState::Empty);
    }

    let mut entry_id = ACL_FIRST_ENTRY;
    let mut saw_entry = false;
    loop {
        let mut entry = std::ptr::null_mut();
        // Avoid mistaking a stale errno value for the documented end-of-list
        // EINVAL after at least one successful entry.
        // SAFETY: `__error` returns this thread's writable errno location.
        unsafe { *libc::__error() = 0 };
        // SAFETY: `acl` is valid and `entry` points to writable storage.
        let result = unsafe { macos_acl_get_entry(acl.as_ptr(), entry_id, &mut entry) };
        if result == -1 {
            let error = std::io::Error::last_os_error();
            if saw_entry && error.raw_os_error() == Some(libc::EINVAL) {
                break;
            }
            return Err(checkpoint_io(format!(
                "failed to inspect {label} extended ACL entries: {error}"
            )));
        }
        if result != 0 || entry.is_null() {
            return Err(checkpoint_io(format!(
                "failed to inspect {label} extended ACL entries"
            )));
        }
        saw_entry = true;

        let mut tag = 0;
        // SAFETY: `entry` was returned by `acl_get_entry` for the live `acl`.
        if unsafe { macos_acl_get_tag_type(entry, &mut tag) } != 0 {
            return Err(checkpoint_io(format!(
                "failed to inspect {label} extended ACL entry: {}",
                std::io::Error::last_os_error()
            )));
        }
        match tag {
            ACL_EXTENDED_ALLOW => return Ok(ExtendedAclState::AllowsAdditionalAccess),
            ACL_EXTENDED_DENY => {}
            _ => {
                return Err(CliError::checkpoint_conflict(format!(
                    "{label} has an unsupported extended ACL entry"
                )));
            }
        }
        entry_id = ACL_NEXT_ENTRY;
    }
    Ok(ExtendedAclState::DenyOnly)
}

#[cfg(target_os = "macos")]
fn validate_no_allow_extended_acl(file: &File, label: &str) -> CliResult<()> {
    if inspect_extended_acl(file, label)? == ExtendedAclState::AllowsAdditionalAccess {
        return Err(CliError::checkpoint_conflict(format!(
            "{label} has an extended ACL that grants additional access"
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn validate_no_allow_extended_acl(_file: &File, _label: &str) -> CliResult<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn clear_extended_acl(file: &File, label: &str) -> CliResult<()> {
    const ACL_TYPE_EXTENDED: libc::c_int = 0x100;

    // SAFETY: a zero count is valid and initializes an empty ACL.
    let empty = unsafe { macos_acl_init(0) };
    let empty = MacOsAclObject(NonNull::new(empty).ok_or_else(|| {
        checkpoint_io(format!(
            "failed to initialize an empty ACL for {label}: {}",
            std::io::Error::last_os_error()
        ))
    })?);
    // SAFETY: the raw descriptor and empty ACL remain valid for this call.
    if unsafe { macos_acl_set_fd_np(file.as_raw_fd(), empty.as_ptr(), ACL_TYPE_EXTENDED) } != 0 {
        return Err(checkpoint_io(format!(
            "failed to clear {label} extended ACL: {}",
            std::io::Error::last_os_error()
        )));
    }
    if inspect_extended_acl(file, label)? != ExtendedAclState::Empty {
        return Err(checkpoint_io(format!(
            "failed to clear every {label} extended ACL entry"
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn clear_extended_acl(_file: &File, _label: &str) -> CliResult<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    #[link_name = "acl_free"]
    fn macos_acl_free(object: *mut c_void) -> libc::c_int;
    #[link_name = "acl_get_entry"]
    fn macos_acl_get_entry(
        acl: *mut c_void,
        entry_id: libc::c_int,
        entry: *mut *mut c_void,
    ) -> libc::c_int;
    #[link_name = "acl_get_fd_np"]
    fn macos_acl_get_fd_np(fd: libc::c_int, acl_type: libc::c_int) -> *mut c_void;
    #[link_name = "acl_get_tag_type"]
    fn macos_acl_get_tag_type(entry: *mut c_void, tag: *mut libc::c_int) -> libc::c_int;
    #[link_name = "acl_init"]
    fn macos_acl_init(count: libc::c_int) -> *mut c_void;
    #[link_name = "acl_set_fd_np"]
    fn macos_acl_set_fd_np(fd: libc::c_int, acl: *mut c_void, acl_type: libc::c_int)
    -> libc::c_int;
    #[link_name = "acl_to_text"]
    fn macos_acl_to_text(acl: *mut c_void, length: *mut libc::ssize_t) -> *mut libc::c_char;
    #[link_name = "acl_valid"]
    fn macos_acl_valid(acl: *mut c_void) -> libc::c_int;
}

#[cfg(unix)]
fn sync_directory_handle(directory: &Dir) -> CliResult<()> {
    directory
        .try_clone()
        .map(cap_std::fs::Dir::into_std_file)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| checkpoint_io(format!("failed to sync checkpoint directory: {error}")))
}

#[cfg(not(unix))]
fn sync_directory_handle(_directory: &Dir) -> CliResult<()> {
    // Windows opens capability directory handles without GENERIC_WRITE, while
    // FlushFileBuffers requires it. The checkpoint and journal files are still
    // synced before their handle-relative atomic publication.
    Ok(())
}

fn relative_open_error(label: &str, error: std::io::Error) -> CliError {
    if error.kind() == std::io::ErrorKind::NotADirectory
        || error.kind() == std::io::ErrorKind::InvalidInput
        || is_symlink_loop_error(&error)
    {
        CliError::checkpoint_conflict(format!(
            "{label} must be a regular file and must not be a symlink or reparse point"
        ))
    } else {
        checkpoint_io(format!("failed to open {label}: {error}"))
    }
}

#[cfg(test)]
fn set_secret_file_permissions(path: &Path) -> CliResult<()> {
    #[cfg(unix)]
    {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| checkpoint_io(format!("failed to secure checkpoint file: {error}")))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn checkpoint_io(message: impl Into<String>) -> CliError {
    CliError::checkpoint_io(message)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn ensure_durable_checkpoint_platform() -> CliResult<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn ensure_durable_checkpoint_platform() -> CliResult<()> {
    Err(CliError::checkpoint_conflict(
        "durable batch checkpoints require Linux or macOS; omit --checkpoint and --resume",
    ))
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use crate::commands::batch::input::operation_key;
    use tempfile::TempDir;

    fn input_hash() -> String {
        "a".repeat(64)
    }

    fn secure_dir() -> TempDir {
        // macOS exposes its temporary directory through `/var`, which is a
        // system symlink. Resolve only that trusted fixture root before any
        // test-controlled path components are added; hostile components below
        // remain visible to the no-follow walker.
        let temporary_root =
            fs::canonicalize(std::env::temp_dir()).expect("canonical temporary root");
        let directory = tempfile::Builder::new()
            .prefix("sealtask-checkpoint-")
            .tempdir_in(temporary_root)
            .expect("temporary directory");
        #[cfg(unix)]
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
            .expect("secure temporary directory");
        directory
    }

    #[test]
    fn rejects_parent_components_before_path_normalization() {
        let directory = secure_dir();
        let checkpoint = directory
            .path()
            .join("symlink-that-must-not-be-erased")
            .join("..")
            .join("checkpoint.json");

        let error = match CheckpointStore::open(&checkpoint, &input_hash(), false) {
            Ok(_) => panic!("parent component must be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.code(), "checkpoint_conflict");
        assert!(error.to_string().contains("must not contain `..`"));
        assert!(!directory.path().join("checkpoint.json").exists());
    }

    #[tokio::test]
    async fn append_journal_persists_only_safe_metadata_and_resumes_success() {
        let directory = secure_dir();
        let path = directory.path().join("run.checkpoint");
        let canary_operation_id = "operator-secret-canary";
        let key = operation_key(canary_operation_id);
        let project_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let updated_at = Utc::now();
        {
            let store = CheckpointStore::open(&path, &input_hash(), false).expect("new checkpoint");
            store
                .record_started(
                    key.clone(),
                    &StartedMetadata {
                        kind: OperationKind::TaskCreate,
                        project_id,
                        task_id: None,
                        expected_updated_at: None,
                        change_commitment: None,
                    },
                )
                .await
                .expect("started");
            store
                .record_succeeded(
                    key.clone(),
                    OperationKind::TaskCreate,
                    project_id,
                    task_id,
                    updated_at,
                )
                .await
                .expect("succeeded");
        }
        let contents = fs::read_to_string(&path).expect("checkpoint contents");
        assert!(!contents.contains(canary_operation_id));
        assert!(!contents.contains("title"));
        assert!(!contents.contains("idempotency"));

        let store = CheckpointStore::open(&path, &input_hash(), true).expect("resume");
        match store.resume_state(&key).expect("state") {
            ResumeState::Succeeded {
                kind: resumed_kind,
                project_id: resumed_project_id,
                task_id: resumed_id,
                updated_at: resumed_at,
            } => {
                assert!(resumed_kind == OperationKind::TaskCreate);
                assert_eq!(resumed_project_id, project_id);
                assert_eq!(resumed_id, task_id);
                assert_eq!(resumed_at, updated_at);
            }
            _ => panic!("expected success"),
        }
    }

    #[tokio::test]
    async fn rejects_input_mismatch_without_overwriting_checkpoint() {
        let directory = secure_dir();
        let path = directory.path().join("run.checkpoint");
        {
            let store = CheckpointStore::open(&path, &input_hash(), false).expect("new checkpoint");
            store
                .record_failed(
                    operation_key("failed"),
                    OperationKind::TaskCreate,
                    Uuid::now_v7(),
                    None,
                )
                .await
                .expect("persist");
        }
        let before = fs::read(&path).expect("before");
        let error = CheckpointStore::open(&path, &"b".repeat(64), true)
            .err()
            .expect("mismatch");
        assert_eq!(error.exit_code(), 4);
        assert_eq!(before, fs::read(&path).expect("after"));
    }

    #[test]
    fn rejects_corrupt_future_and_locked_checkpoints() {
        let directory = secure_dir();
        let corrupt = directory.path().join("corrupt");
        fs::write(&corrupt, b"not-json").expect("write");
        set_secret_file_permissions(&corrupt).expect("permissions");
        assert!(
            CheckpointStore::open(&corrupt, &input_hash(), true)
                .err()
                .expect("corrupt")
                .to_string()
                .contains("corrupt")
        );

        let future = directory.path().join("future");
        fs::write(
            &future,
            format!(
                r#"{{"schemaVersion":2,"inputSha256":"{}","operations":{{}}}}"#,
                input_hash()
            ),
        )
        .expect("write");
        set_secret_file_permissions(&future).expect("permissions");
        assert_eq!(
            CheckpointStore::open(&future, &input_hash(), true)
                .err()
                .expect("future")
                .exit_code(),
            4
        );

        let locked = directory.path().join("locked");
        let first = CheckpointStore::open(&locked, &input_hash(), false).expect("first lock");
        let second = CheckpointStore::open(&locked, &input_hash(), false)
            .err()
            .expect("second lock");
        assert_eq!(second.exit_code(), 4);
        drop(first);
    }

    #[test]
    fn rejects_oversized_sparse_checkpoints_before_allocating_their_declared_size() {
        let directory = secure_dir();
        let exact = directory.path().join("exact-bound");
        let exact_file = File::create(&exact).expect("exact-bound checkpoint");
        exact_file
            .set_len(MAX_CHECKPOINT_BYTES)
            .expect("size exact-bound checkpoint");
        set_secret_file_permissions(&exact).expect("exact-bound permissions");
        let exact_error = CheckpointStore::open(&exact, &input_hash(), true)
            .err()
            .expect("zero-filled exact-bound checkpoint is corrupt");
        assert_eq!(exact_error.code(), "checkpoint_conflict");
        assert!(
            !exact_error.to_string().contains("exceeds"),
            "the exact byte bound must be read and parsed"
        );

        let oversized = directory.path().join("oversized-sparse");
        let oversized_file = File::create(&oversized).expect("oversized checkpoint");
        oversized_file
            .set_len(MAX_CHECKPOINT_BYTES + 1)
            .expect("size oversized checkpoint");
        set_secret_file_permissions(&oversized).expect("oversized permissions");
        let oversized_error = CheckpointStore::open(&oversized, &input_hash(), true)
            .err()
            .expect("oversized checkpoint must fail");
        assert_eq!(oversized_error.code(), "checkpoint_conflict");
        assert!(
            oversized_error.to_string().contains("exceeds"),
            "the metadata bound must reject before allocating or reading"
        );
    }

    #[test]
    fn atomic_no_replace_publication_never_clobbers_an_existing_checkpoint() {
        let directory = secure_dir();
        let capability =
            prepare_checkpoint_parent(directory.path()).expect("checkpoint parent capability");
        let target_name = OsString::from("existing-target");
        let location = CheckpointLocation {
            directory: capability,
            file_name: target_name.clone(),
            lock_name: lock_name(&target_name),
        };
        let target = directory.path().join(&target_name);
        fs::write(&target, b"existing-checkpoint-canary").expect("existing target");
        set_secret_file_permissions(&target).expect("existing target permissions");

        let mut temporary =
            RelativeTemporaryFile::create(&location.directory).expect("temporary checkpoint");
        temporary
            .file_mut()
            .expect("temporary handle")
            .write_all(b"replacement")
            .expect("temporary content");
        let error = temporary
            .publish(&location, false)
            .expect_err("atomic no-replace must reject an existing target");

        assert_eq!(error.code(), "checkpoint_conflict");
        assert_eq!(
            fs::read(&target).expect("existing target contents"),
            b"existing-checkpoint-canary"
        );
        assert_eq!(
            fs::metadata(&target)
                .expect("existing target metadata")
                .nlink(),
            1
        );
        assert!(
            fs::read_dir(directory.path())
                .expect("checkpoint directory")
                .filter_map(Result::ok)
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sealtask-checkpoint-")),
            "failed publication must clean up its private temporary file"
        );
    }

    #[test]
    fn atomic_new_checkpoint_publication_survives_immediate_process_exit() {
        const CHILD_TEST_NAME: &str = "commands::batch::checkpoint::tests::atomic_new_checkpoint_publication_survives_immediate_process_exit";
        const EXIT_PATH_ENV: &str = "SEALTASK_TEST_EXIT_AFTER_NEW_CHECKPOINT_PUBLICATION";

        if let Some(path) = std::env::var_os(EXIT_PATH_ENV) {
            let path = PathBuf::from(path);
            let _store = CheckpointStore::open(&path, &input_hash(), false)
                .expect("child must reach the post-publication exit hook");
            panic!("post-publication exit hook did not terminate the child");
        }

        let directory = secure_dir();
        let checkpoint = directory.path().join("crash-window");
        let output = std::process::Command::new(
            std::env::current_exe().expect("current checkpoint test executable"),
        )
        .arg("--exact")
        .arg(CHILD_TEST_NAME)
        .env(EXIT_PATH_ENV, &checkpoint)
        .output()
        .expect("run checkpoint crash-window child");
        assert_eq!(
            output.status.code(),
            Some(86),
            "child stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let metadata = fs::metadata(&checkpoint).expect("atomically published checkpoint");
        assert_eq!(
            metadata.nlink(),
            1,
            "the target must never have a second crash-visible hard link"
        );
        assert!(
            fs::read_dir(directory.path())
                .expect("checkpoint directory")
                .filter_map(Result::ok)
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sealtask-checkpoint-")),
            "atomic rename must consume the temporary name before publication is visible"
        );

        let resumed = CheckpointStore::open(&checkpoint, &input_hash(), true)
            .expect("the checkpoint must remain resumable after immediate process exit");
        assert!(matches!(
            resumed
                .resume_state(&operation_key("not-started"))
                .expect("resume state"),
            ResumeState::Absent
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_and_broad_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let directory = secure_dir();
        let real = directory.path().join("real");
        fs::write(&real, b"{}").expect("real");
        fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).expect("mode");
        let link = directory.path().join("link");
        symlink(&real, &link).expect("symlink");
        assert_eq!(
            CheckpointStore::open(&link, &input_hash(), true)
                .err()
                .expect("symlink")
                .exit_code(),
            4
        );

        let broad = directory.path().join("broad");
        fs::write(&broad, b"{}").expect("broad");
        fs::set_permissions(&broad, fs::Permissions::from_mode(0o644)).expect("mode");
        assert_eq!(
            CheckpointStore::open(&broad, &input_hash(), true)
                .err()
                .expect("broad permissions")
                .exit_code(),
            4
        );
    }

    #[test]
    fn rejects_an_owner_other_than_the_effective_user() {
        let error = validate_effective_owner_ids(1000, 1001, "checkpoint")
            .expect_err("wrong owner must be rejected");

        assert_eq!(error.code(), "checkpoint_conflict");
        assert!(
            error
                .to_string()
                .contains("owned by the current effective user")
        );
        validate_effective_owner_ids(1000, 1000, "checkpoint")
            .expect("matching owner must be accepted");
    }

    #[test]
    fn rejects_hard_linked_checkpoint_and_lock_files() {
        let directory = secure_dir();
        let checkpoint = directory.path().join("hard-linked-checkpoint");
        {
            let store =
                CheckpointStore::open(&checkpoint, &input_hash(), false).expect("checkpoint");
            drop(store);
        }
        let checkpoint_alias = directory.path().join("checkpoint-alias");
        fs::hard_link(&checkpoint, &checkpoint_alias).expect("checkpoint hard link");
        let error = CheckpointStore::open(&checkpoint, &input_hash(), true)
            .err()
            .expect("hard-linked checkpoint must be rejected");
        assert_eq!(error.code(), "checkpoint_conflict");
        assert!(error.to_string().contains("exactly one hard link"));
        fs::remove_file(&checkpoint_alias).expect("remove checkpoint alias");

        let lock = directory.path().join("hard-linked-checkpoint.lock");
        let lock_alias = directory.path().join("lock-alias");
        fs::hard_link(&lock, &lock_alias).expect("lock hard link");
        let error = CheckpointStore::open(&checkpoint, &input_hash(), true)
            .err()
            .expect("hard-linked lock must be rejected");
        assert_eq!(error.code(), "checkpoint_conflict");
        assert!(error.to_string().contains("exactly one hard link"));
    }

    #[cfg(target_os = "macos")]
    fn change_acl(path: &Path, arguments: &[&str]) {
        let status = std::process::Command::new("/bin/chmod")
            .args(arguments)
            .arg(path)
            .status()
            .expect("run chmod");
        assert!(status.success(), "chmod failed for {}", path.display());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_allow_acls_on_checkpoint_state_and_before_parent_creation() {
        let directory = secure_dir();
        let checkpoint = directory.path().join("acl-checkpoint");
        {
            let store =
                CheckpointStore::open(&checkpoint, &input_hash(), false).expect("checkpoint");
            drop(store);
        }
        change_acl(&checkpoint, &["+a", "everyone allow read,write"]);
        let error = CheckpointStore::open(&checkpoint, &input_hash(), true)
            .err()
            .expect("ALLOW ACL on checkpoint must be rejected");
        assert_eq!(error.code(), "checkpoint_conflict");
        assert!(error.to_string().contains("grants additional access"));

        let acl_parent = directory.path().join("acl-parent");
        fs::create_dir(&acl_parent).expect("ACL parent");
        fs::set_permissions(&acl_parent, fs::Permissions::from_mode(0o700))
            .expect("secure ACL parent mode");
        change_acl(&acl_parent, &["+a", "everyone allow read,write"]);
        let missing_child = acl_parent.join("missing");
        let error = CheckpointStore::open(&missing_child.join("state"), &input_hash(), false)
            .err()
            .expect("ALLOW ACL must be rejected before child creation");
        assert_eq!(error.code(), "checkpoint_conflict");
        assert!(error.to_string().contains("grants additional access"));
        assert!(
            !missing_child.exists(),
            "ACL-bearing parent must not gain a newly created child"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn permits_deny_only_acls_and_clears_created_handle_acls() {
        let directory = secure_dir();
        change_acl(directory.path(), &["+a", "everyone deny delete"]);
        let checkpoint = directory.path().join("deny-only-parent");
        let store = CheckpointStore::open(&checkpoint, &input_hash(), false)
            .expect("deny-only parent ACL must remain usable");
        drop(store);
        change_acl(directory.path(), &["-N"]);

        let file_path = directory.path().join("acl-to-clear");
        fs::write(&file_path, b"test").expect("test file");
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600))
            .expect("secure file mode");
        change_acl(&file_path, &["+a", "everyone allow read,write"]);
        let file = StdOpenOptions::new()
            .read(true)
            .write(true)
            .open(&file_path)
            .expect("open ACL test file");
        assert_eq!(
            inspect_extended_acl(&file, "test file").expect("inspect added ACL"),
            ExtendedAclState::AllowsAdditionalAccess
        );
        set_secret_file_handle_permissions(&file, "test file").expect("clear ACL through handle");
        assert_eq!(
            inspect_extended_acl(&file, "test file").expect("inspect cleared ACL"),
            ExtendedAclState::Empty
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_an_existing_intermediate_parent_symlink_without_side_effects() {
        use std::os::unix::fs::symlink;

        let directory = secure_dir();
        let redirected_parent = directory.path().join("redirected-parent");
        fs::create_dir(&redirected_parent).expect("redirected parent");
        fs::set_permissions(&redirected_parent, fs::Permissions::from_mode(0o700))
            .expect("secure redirected parent");
        let existing_child = redirected_parent.join("existing-child");
        fs::create_dir(&existing_child).expect("existing child");
        fs::set_permissions(&existing_child, fs::Permissions::from_mode(0o700))
            .expect("secure existing child");

        let intermediate_link = directory.path().join("intermediate-link");
        symlink(&redirected_parent, &intermediate_link).expect("intermediate symlink");
        let checkpoint = intermediate_link.join("existing-child").join("state");

        let error = CheckpointStore::open(&checkpoint, &input_hash(), false)
            .err()
            .expect("intermediate symlink must be rejected");
        assert_eq!(error.exit_code(), 4);
        assert!(error.to_string().contains("symlink"));
        assert!(!existing_child.join("state").exists());
        assert!(!existing_child.join("state.lock").exists());
        assert!(
            fs::read_dir(&existing_child)
                .expect("existing child entries")
                .next()
                .is_none(),
            "rejected traversal must not leave a checkpoint temporary file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_dangling_intermediate_symlink_without_creating_missing_children() {
        use std::os::unix::fs::symlink;

        let directory = secure_dir();
        let missing_target = directory.path().join("missing-target");
        let intermediate_link = directory.path().join("missing-child-link");
        symlink(&missing_target, &intermediate_link).expect("dangling intermediate symlink");
        let checkpoint = intermediate_link.join("nested").join("state");

        let error = CheckpointStore::open(&checkpoint, &input_hash(), false)
            .err()
            .expect("dangling intermediate symlink must be rejected");
        assert_eq!(error.exit_code(), 4);
        assert!(error.to_string().contains("symlink"));
        assert!(
            !missing_target.exists(),
            "rejected traversal must not materialize the symlink target or its children"
        );
    }

    #[test]
    fn rejects_input_checkpoint_alias() {
        let directory = secure_dir();
        let input = directory.path().join("input.jsonl");
        fs::write(&input, b"input").expect("input");
        let error = reject_input_checkpoint_conflict(&input, Some(&input)).expect_err("same path");
        assert_eq!(error.exit_code(), 4);
    }

    #[test]
    fn accepts_a_new_checkpoint_beneath_missing_directories() {
        let directory = secure_dir();
        let input = directory.path().join("input.jsonl");
        fs::write(&input, b"input").expect("input");
        let checkpoint = directory.path().join("new").join("nested").join("state");
        reject_input_checkpoint_conflict(&input, Some(&checkpoint))
            .expect("missing checkpoint parents are valid");
    }

    #[test]
    fn materializes_and_syncs_each_nested_checkpoint_directory() {
        let directory = secure_dir();
        let first = directory.path().join("first");
        let second = first.join("second");
        let third = second.join("third");
        let mut synced = Vec::new();
        prepare_checkpoint_parent_with(&third, |path| {
            synced.push(path.to_path_buf());
            Ok(())
        })
        .expect("create nested checkpoint parent");

        for created in [&first, &second, &third] {
            assert!(created.is_dir());
            assert!(
                synced.contains(created),
                "new checkpoint directory was not synced: {}",
                created.display()
            );
            #[cfg(unix)]
            assert_eq!(
                fs::metadata(created)
                    .expect("directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_journal_writes_are_linear_and_resume_every_operation() {
        const JOBS: usize = 16;
        const OPERATIONS_PER_JOB: usize = 16;

        let directory = secure_dir();
        let path = directory.path().join("concurrent");
        let store = Arc::new(
            CheckpointStore::open(&path, &input_hash(), false).expect("new checkpoint journal"),
        );
        let project_id = Uuid::now_v7();
        let mut tasks = tokio::task::JoinSet::new();
        for job in 0..JOBS {
            let store = Arc::clone(&store);
            tasks.spawn(async move {
                for operation in 0..OPERATIONS_PER_JOB {
                    store
                        .record_failed(
                            operation_key(&format!("job-{job}-operation-{operation}")),
                            OperationKind::TaskCreate,
                            project_id,
                            None,
                        )
                        .await
                        .expect("append checkpoint record");
                }
            });
        }
        while let Some(result) = tasks.join_next().await {
            result.expect("checkpoint writer task");
        }
        drop(store);

        let bytes = fs::read(&path).expect("checkpoint journal");
        let transitions = JOBS * OPERATIONS_PER_JOB;
        assert_eq!(
            bytes.iter().filter(|byte| **byte == b'\n').count(),
            transitions + 1,
            "one header and one bounded append per transition"
        );
        assert!(
            bytes.len() < (transitions + 1) * 512,
            "journal growth must remain linear in transition count"
        );

        let resumed = CheckpointStore::open(&path, &input_hash(), true).expect("resume journal");
        for job in 0..JOBS {
            for operation in 0..OPERATIONS_PER_JOB {
                assert!(matches!(
                    resumed
                        .resume_state(&operation_key(&format!("job-{job}-operation-{operation}")))
                        .expect("resume state"),
                    ResumeState::Failed(_)
                ));
            }
        }
    }

    #[tokio::test]
    async fn journal_compacts_atomically_before_crossing_its_size_bound() {
        let directory = secure_dir();
        let path = directory.path().join("bounded");
        let store = CheckpointStore::open(&path, &input_hash(), false).expect("checkpoint");
        {
            let mut writer = store.writer.lock().expect("writer");
            writer.bytes_written = MAX_CHECKPOINT_BYTES;
        }
        let key = operation_key("compacted");
        store
            .record_failed(key.clone(), OperationKind::TaskCreate, Uuid::now_v7(), None)
            .await
            .expect("bounded compaction");
        drop(store);

        let bytes = fs::read(&path).expect("compacted checkpoint");
        assert!(bytes.len() as u64 <= MAX_CHECKPOINT_BYTES);
        assert_eq!(
            bytes.iter().filter(|byte| **byte == b'\n').count(),
            2,
            "compaction keeps one header and only the latest operation state"
        );
        let resumed = CheckpointStore::open(&path, &input_hash(), true).expect("resume compacted");
        assert!(matches!(
            resumed.resume_state(&key).expect("state"),
            ResumeState::Failed(_)
        ));
    }

    #[tokio::test]
    async fn resume_discards_only_an_incomplete_unacknowledged_tail() {
        let directory = secure_dir();
        let path = directory.path().join("torn-tail");
        let key = operation_key("durable-start");
        {
            let store = CheckpointStore::open(&path, &input_hash(), false).expect("new checkpoint");
            store
                .record_started(
                    key.clone(),
                    &StartedMetadata {
                        kind: OperationKind::TaskCreate,
                        project_id: Uuid::now_v7(),
                        task_id: None,
                        expected_updated_at: None,
                        change_commitment: None,
                    },
                )
                .await
                .expect("durable start");
        }
        {
            let mut file = StdOpenOptions::new()
                .append(true)
                .open(&path)
                .expect("append torn tail");
            file.write_all(br#"{"recordType":"operation","partial":"tail-canary""#)
                .expect("partial tail");
            file.sync_all().expect("sync partial tail");
        }

        let resumed = CheckpointStore::open(&path, &input_hash(), true)
            .expect("resume ignores unacknowledged tail");
        assert!(matches!(
            resumed.resume_state(&key).expect("started state"),
            ResumeState::Started(_)
        ));
        drop(resumed);
        let repaired = fs::read_to_string(&path).expect("compacted journal");
        assert!(!repaired.contains("tail-canary"));
        assert!(repaired.ends_with('\n'));
    }

    #[tokio::test]
    async fn rejects_extra_operation_keys_and_partial_started_metadata() {
        let directory = secure_dir();
        let path = directory.path().join("extra");
        {
            let store = CheckpointStore::open(&path, &input_hash(), false).expect("checkpoint");
            store
                .record_failed(
                    operation_key("extra"),
                    OperationKind::TaskCreate,
                    Uuid::now_v7(),
                    None,
                )
                .await
                .expect("persist extra");
        }
        let store = CheckpointStore::open(&path, &input_hash(), true).expect("resume");
        let allowed = std::collections::HashSet::from([operation_key("expected")]);
        assert_eq!(
            store
                .validate_operation_keys(&allowed)
                .expect_err("extra key")
                .exit_code(),
            4
        );
        drop(store);

        let partial = directory.path().join("partial");
        let update_key = operation_key("update");
        fs::write(
            &partial,
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": BATCH_SCHEMA_VERSION,
                "inputSha256": input_hash(),
                "operations": {
                    (update_key): {
                        "state": "started",
                        "kind": "task_update",
                        "projectId": Uuid::now_v7(),
                        "taskId": Uuid::now_v7(),
                        "expectedUpdatedAt": Utc::now(),
                    }
                }
            }))
            .expect("encode"),
        )
        .expect("write partial");
        set_secret_file_permissions(&partial).expect("permissions");
        assert_eq!(
            CheckpointStore::open(&partial, &input_hash(), true)
                .err()
                .expect("partial started")
                .exit_code(),
            4
        );
    }
}
