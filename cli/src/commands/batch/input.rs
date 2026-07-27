use crate::output::{CliError, CliResult};
use crate::selectors::EntitySelector;
use chrono::{DateTime, Utc};
use sealtask_client_core::PublicError;
use sealtask_client_crypto::ChecklistItemPayload;
use sealtask_client_runtime::{
    TaskCreateIdempotencyDerivation, TaskCreateInput, TaskFieldPatch, TaskUpdateInput,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

pub(super) const BATCH_SCHEMA_VERSION: u64 = 1;
const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;
const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
const MAX_OPERATIONS: usize = 10_000;
const MAX_OPERATION_ID_BYTES: usize = 128;

#[cfg(test)]
const ZEROIZE_TEST_DOMAIN: &[u8] = b"sealtask.batch.zeroize-test.v1\0";

#[cfg(test)]
thread_local! {
    static ZEROIZED_STRING_FINGERPRINTS: std::cell::RefCell<Option<Vec<String>>> =
        const { std::cell::RefCell::new(None) };
}

pub(super) struct BatchDocument {
    pub(super) input_sha256: String,
    pub(super) operations: Vec<BatchOperation>,
}

pub(super) struct BatchOperation {
    pub(super) index: usize,
    pub(super) operation_id: String,
    pub(super) project: EntitySelector,
    pub(super) kind: BatchOperationKind,
}

pub(super) enum BatchOperationKind {
    TaskCreate {
        input: TaskCreateInput,
        idempotency_derivation: Option<TaskCreateIdempotencyDerivation>,
    },
    TaskUpdate {
        task: EntitySelector,
        input: TaskUpdateInput,
    },
}

impl Drop for BatchOperationKind {
    fn drop(&mut self) {
        match self {
            Self::TaskCreate { input, .. } => zeroize_task_create_input(input),
            Self::TaskUpdate { input, .. } => zeroize_task_update_input(input),
        }
    }
}

struct SensitiveString(String);

impl SensitiveString {
    fn as_str(&self) -> &str {
        &self.0
    }

    fn into_inner(mut self) -> String {
        std::mem::take(&mut self.0)
    }
}

impl Serialize for SensitiveString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SensitiveString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self)
    }
}

impl Zeroize for SensitiveString {
    fn zeroize(&mut self) {
        zeroize_sensitive_string(&mut self.0);
    }
}

impl Drop for SensitiveString {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum WireOperation {
    #[serde(rename = "task.create")]
    TaskCreate {
        schema_version: u64,
        operation_id: SensitiveString,
        project: SensitiveString,
        input: WireTaskCreateInput,
    },
    #[serde(rename = "task.update")]
    TaskUpdate {
        schema_version: u64,
        operation_id: SensitiveString,
        project: SensitiveString,
        task: SensitiveString,
        input: WireTaskUpdateInput,
    },
}

#[derive(Clone, Copy, Deserialize)]
enum WireOperationType {
    #[serde(rename = "task.create")]
    TaskCreate,
    #[serde(rename = "task.update")]
    TaskUpdate,
}

#[derive(Deserialize)]
struct WireOperationTypeProbe {
    #[serde(rename = "type")]
    operation_type: WireOperationType,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireTaskCreateOperation {
    schema_version: u64,
    operation_id: SensitiveString,
    #[serde(rename = "type")]
    operation_type: WireOperationType,
    project: SensitiveString,
    input: WireTaskCreateInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireTaskUpdateOperation {
    schema_version: u64,
    operation_id: SensitiveString,
    #[serde(rename = "type")]
    operation_type: WireOperationType,
    project: SensitiveString,
    task: SensitiveString,
    input: WireTaskUpdateInput,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireTaskCreateInput {
    title: SensitiveString,
    #[serde(default)]
    body: Option<SensitiveString>,
    #[serde(default)]
    checklist: Option<Vec<WireChecklistItem>>,
    #[serde(default)]
    priority: Option<i8>,
    #[serde(default)]
    due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    start_at: Option<DateTime<Utc>>,
    #[serde(default)]
    section_id: Option<Uuid>,
    #[serde(default)]
    idempotency_key: Option<SensitiveString>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireTaskUpdateInput {
    #[serde(default)]
    title: Option<SensitiveString>,
    #[serde(default, skip_serializing_if = "WirePatch::is_unchanged")]
    body: WirePatch<SensitiveString>,
    #[serde(default, skip_serializing_if = "WirePatch::is_unchanged")]
    checklist: WirePatch<Vec<WireChecklistItem>>,
    #[serde(default, skip_serializing_if = "WirePatch::is_unchanged")]
    priority: WirePatch<i8>,
    #[serde(default, skip_serializing_if = "WirePatch::is_unchanged")]
    due_at: WirePatch<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "WirePatch::is_unchanged")]
    start_at: WirePatch<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "WirePatch::is_unchanged")]
    section_id: WirePatch<Uuid>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireChecklistItem {
    id: SensitiveString,
    title: SensitiveString,
    is_done: bool,
    #[serde(default)]
    completed_at: Option<i64>,
    #[serde(default)]
    assignee_user_ids: Option<Vec<SensitiveString>>,
}

#[derive(Default)]
enum WirePatch<T> {
    #[default]
    Unchanged,
    Set(T),
    Clear,
}

impl<T> WirePatch<T> {
    fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

impl<T: Serialize> Serialize for WirePatch<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Set(value) => value.serialize(serializer),
            Self::Clear | Self::Unchanged => serializer.serialize_none(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for WirePatch<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

pub(super) fn read_batch_input(path: &Path) -> CliResult<BatchDocument> {
    if path == Path::new("-") {
        let stdin = io::stdin();
        return read_batch(BufReader::new(stdin.lock()));
    }

    let file = open_input_file(path)
        .map_err(|error| public_validation(format!("failed to open batch input: {error}")))?;
    read_batch(BufReader::new(file))
}

fn read_batch<R: BufRead>(mut reader: R) -> CliResult<BatchDocument> {
    let mut wires = Vec::new();
    let mut operation_ids = HashSet::new();
    let mut explicit_idempotency_keys = HashSet::new();
    let mut total_bytes = 0usize;
    let mut line = Zeroizing::new(Vec::new());
    let mut line_number = 0usize;

    loop {
        line.zeroize();
        let Some(bytes_read) =
            read_bounded_line(&mut reader, &mut line, total_bytes, line_number + 1)?
        else {
            break;
        };
        line_number += 1;
        total_bytes += bytes_read;
        if line.last() == Some(&b'\n') {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            return Err(public_validation(format!(
                "batch input line {line_number} is empty"
            )));
        }
        if wires.len() == MAX_OPERATIONS {
            return Err(public_validation(format!(
                "batch input exceeds the {MAX_OPERATIONS}-operation limit"
            )));
        }

        let wire = deserialize_wire_operation(line.as_slice()).map_err(|error| {
            public_validation(format!(
                "batch input line {line_number} is not a valid strict JSONL v1 operation (column {})",
                error.column()
            ))
        })?;
        validate_wire(&wire, line_number)?;
        if let Some(key) = wire.explicit_idempotency_key() {
            let fingerprint = sensitive_fingerprint(b"sealtask.batch.idempotency.v1\0", key);
            if !explicit_idempotency_keys.insert(fingerprint) {
                return Err(public_validation(format!(
                    "batch input line {line_number} repeats an explicit task.create idempotencyKey"
                )));
            }
        }
        let operation_id = wire.operation_id();
        if !operation_ids.insert(operation_id.to_string()) {
            return Err(public_validation(format!(
                "batch input line {line_number} repeats an earlier operationId"
            )));
        }
        wires.push(wire);
    }

    if wires.is_empty() {
        return Err(public_validation(
            "batch input must contain at least one JSONL operation",
        ));
    }

    let canonical_input_sha256 = Zeroizing::new(canonical_input_sha256(&wires)?);
    let input_sha256 = hex_digest(canonical_input_sha256.as_slice());
    let operations = wires
        .into_iter()
        .enumerate()
        .map(|(index, wire)| wire.into_operation(index, &canonical_input_sha256))
        .collect::<CliResult<Vec<_>>>()?;
    Ok(BatchDocument {
        input_sha256,
        operations,
    })
}

fn deserialize_wire_operation(line: &[u8]) -> serde_json::Result<WireOperation> {
    // Serde's internally tagged enum decoder buffers the remaining map as
    // generic `Content`, whose owned strings are not zeroized on a late
    // decode error. Probe only the tag first, then decode directly into the
    // strict variant whose string fields are guarded as soon as they exist.
    let probe = serde_json::from_slice::<WireOperationTypeProbe>(line)?;
    match probe.operation_type {
        WireOperationType::TaskCreate => {
            let operation = serde_json::from_slice::<WireTaskCreateOperation>(line)?;
            debug_assert!(matches!(
                operation.operation_type,
                WireOperationType::TaskCreate
            ));
            Ok(WireOperation::TaskCreate {
                schema_version: operation.schema_version,
                operation_id: operation.operation_id,
                project: operation.project,
                input: operation.input,
            })
        }
        WireOperationType::TaskUpdate => {
            let operation = serde_json::from_slice::<WireTaskUpdateOperation>(line)?;
            debug_assert!(matches!(
                operation.operation_type,
                WireOperationType::TaskUpdate
            ));
            Ok(WireOperation::TaskUpdate {
                schema_version: operation.schema_version,
                operation_id: operation.operation_id,
                project: operation.project,
                task: operation.task,
                input: operation.input,
            })
        }
    }
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    line: &mut Vec<u8>,
    total_before_line: usize,
    line_number: usize,
) -> CliResult<Option<usize>> {
    let mut bytes_read = 0usize;
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| public_validation(format!("failed to read batch input: {error}")))?;
        if available.is_empty() {
            return Ok((bytes_read > 0).then_some(bytes_read));
        }
        let chunk_length = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let next_line_bytes = bytes_read.checked_add(chunk_length).ok_or_else(|| {
            public_validation(format!(
                "batch input line {line_number} exceeds the {MAX_LINE_BYTES}-byte limit"
            ))
        })?;
        if next_line_bytes > MAX_LINE_BYTES {
            return Err(public_validation(format!(
                "batch input line {line_number} exceeds the {MAX_LINE_BYTES}-byte limit"
            )));
        }
        let next_total = total_before_line
            .checked_add(next_line_bytes)
            .ok_or_else(|| {
                public_validation(format!(
                    "batch input exceeds the {MAX_INPUT_BYTES}-byte limit"
                ))
            })?;
        if next_total > MAX_INPUT_BYTES {
            return Err(public_validation(format!(
                "batch input exceeds the {MAX_INPUT_BYTES}-byte limit"
            )));
        }
        line.extend_from_slice(&available[..chunk_length]);
        reader.consume(chunk_length);
        bytes_read = next_line_bytes;
        if line.last() == Some(&b'\n') {
            return Ok(Some(bytes_read));
        }
    }
}

fn open_input_file(path: &Path) -> io::Result<File> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symlinks are not allowed",
        ));
    }
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "input must be a regular file",
        ));
    }
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("input exceeds the {MAX_INPUT_BYTES}-byte limit"),
        ));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "input must be a regular file",
        ));
    }
    Ok(file)
}

impl WireOperation {
    fn operation_id(&self) -> &str {
        match self {
            Self::TaskCreate { operation_id, .. } | Self::TaskUpdate { operation_id, .. } => {
                operation_id.as_str()
            }
        }
    }

    fn explicit_idempotency_key(&self) -> Option<&str> {
        match self {
            Self::TaskCreate { input, .. } => {
                input.idempotency_key.as_ref().map(SensitiveString::as_str)
            }
            Self::TaskUpdate { .. } => None,
        }
    }

    fn into_operation(
        self,
        index: usize,
        canonical_input_sha256: &[u8; 32],
    ) -> CliResult<BatchOperation> {
        match self {
            Self::TaskCreate {
                operation_id,
                project,
                input,
                ..
            } => {
                let project = parse_selector(project.as_str(), index + 1, "project")?;
                let input = input.into_runtime();
                let idempotency_derivation = input.idempotency_key.is_none().then(|| {
                    TaskCreateIdempotencyDerivation::new(
                        *canonical_input_sha256,
                        operation_id.as_str(),
                    )
                });
                Ok(BatchOperation {
                    index,
                    operation_id: operation_id.into_inner(),
                    project,
                    kind: BatchOperationKind::TaskCreate {
                        input,
                        idempotency_derivation,
                    },
                })
            }
            Self::TaskUpdate {
                operation_id,
                project,
                task,
                input,
                ..
            } => {
                let project = parse_selector(project.as_str(), index + 1, "project")?;
                let task = parse_selector(task.as_str(), index + 1, "task")?;
                Ok(BatchOperation {
                    index,
                    operation_id: operation_id.into_inner(),
                    project,
                    kind: BatchOperationKind::TaskUpdate {
                        task,
                        input: input.into_runtime(),
                    },
                })
            }
        }
    }
}

fn validate_wire(wire: &WireOperation, line_number: usize) -> CliResult<()> {
    let (schema_version, operation_id, project, task, create, update) = match wire {
        WireOperation::TaskCreate {
            schema_version,
            operation_id,
            project,
            input,
            ..
        } => (
            *schema_version,
            operation_id.as_str(),
            project.as_str(),
            None,
            Some(input),
            None,
        ),
        WireOperation::TaskUpdate {
            schema_version,
            operation_id,
            project,
            task,
            input,
            ..
        } => (
            *schema_version,
            operation_id.as_str(),
            project.as_str(),
            Some(task.as_str()),
            None,
            Some(input),
        ),
    };
    if schema_version != BATCH_SCHEMA_VERSION {
        return Err(public_validation(format!(
            "batch input line {line_number} uses unsupported schemaVersion {schema_version}; expected {BATCH_SCHEMA_VERSION}"
        )));
    }
    validate_operation_id(operation_id, line_number)?;
    parse_selector(project, line_number, "project")?;
    if let Some(task) = task {
        parse_selector(task, line_number, "task")?;
    }
    if let Some(input) = create {
        validate_create_input(input, line_number)?;
    }
    if let Some(input) = update {
        validate_update_input(input, line_number)?;
    }
    Ok(())
}

fn validate_operation_id(value: &str, line_number: usize) -> CliResult<()> {
    if value.is_empty()
        || value.len() > MAX_OPERATION_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(public_validation(format!(
            "batch input line {line_number} operationId must contain 1 to {MAX_OPERATION_ID_BYTES} ASCII letters, digits, '.', '_', ':', or '-'"
        )));
    }
    Ok(())
}

fn validate_create_input(input: &WireTaskCreateInput, line_number: usize) -> CliResult<()> {
    if input.title.as_str().trim().is_empty() {
        return Err(public_validation(format!(
            "batch input line {line_number} task.create title is required"
        )));
    }
    validate_priority(input.priority, line_number)?;
    if let Some(key) = input.idempotency_key.as_ref() {
        validate_idempotency_key(key.as_str(), line_number)?;
    }
    if let Some(items) = input.checklist.as_deref() {
        validate_checklist(items, line_number)?;
    }
    Ok(())
}

fn validate_update_input(input: &WireTaskUpdateInput, line_number: usize) -> CliResult<()> {
    let has_change = input.title.is_some()
        || !input.body.is_unchanged()
        || !input.checklist.is_unchanged()
        || !input.priority.is_unchanged()
        || !input.due_at.is_unchanged()
        || !input.start_at.is_unchanged()
        || !input.section_id.is_unchanged();
    if !has_change {
        return Err(public_validation(format!(
            "batch input line {line_number} task.update must change at least one field"
        )));
    }
    if input
        .title
        .as_ref()
        .is_some_and(|title| title.as_str().trim().is_empty())
    {
        return Err(public_validation(format!(
            "batch input line {line_number} task.update title cannot be empty"
        )));
    }
    if let WirePatch::Set(priority) = input.priority {
        validate_priority(Some(priority), line_number)?;
    }
    if let WirePatch::Set(items) = &input.checklist {
        validate_checklist(items, line_number)?;
    }
    Ok(())
}

fn validate_priority(priority: Option<i8>, line_number: usize) -> CliResult<()> {
    if priority.is_some_and(|value| ![1, 3, 5, 8].contains(&value)) {
        return Err(public_validation(format!(
            "batch input line {line_number} priority must be one of 1, 3, 5, or 8"
        )));
    }
    Ok(())
}

fn validate_idempotency_key(value: &str, line_number: usize) -> CliResult<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(public_validation(format!(
            "batch input line {line_number} idempotencyKey must contain 1 to 128 ASCII letters, digits, '.', '_', ':', or '-'"
        )));
    }
    Ok(())
}

fn validate_checklist(items: &[WireChecklistItem], line_number: usize) -> CliResult<()> {
    if items.len() > 200 {
        return Err(public_validation(format!(
            "batch input line {line_number} checklist cannot exceed 200 entries"
        )));
    }
    let mut ids = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let id = Uuid::parse_str(item.id.as_str()).map_err(|_| {
            public_validation(format!(
                "batch input line {line_number} checklist[{index}].id must be a UUID"
            ))
        })?;
        if !ids.insert(id) {
            return Err(public_validation(format!(
                "batch input line {line_number} checklist[{index}].id is duplicated"
            )));
        }
        let title_length = item.title.as_str().trim().chars().count();
        if !(1..=1_024).contains(&title_length) {
            return Err(public_validation(format!(
                "batch input line {line_number} checklist[{index}].title must contain 1 to 1024 characters"
            )));
        }
        if let Some(assignees) = item.assignee_user_ids.as_deref() {
            if assignees.len() > 16 {
                return Err(public_validation(format!(
                    "batch input line {line_number} checklist[{index}] cannot exceed 16 assignees"
                )));
            }
            let mut assignee_ids = HashSet::new();
            for (assignee_index, assignee) in assignees.iter().enumerate() {
                let id = Uuid::parse_str(assignee.as_str()).map_err(|_| {
                    public_validation(format!(
                        "batch input line {line_number} checklist[{index}].assignee_user_ids[{assignee_index}] must be a UUID"
                    ))
                })?;
                if !assignee_ids.insert(id) {
                    return Err(public_validation(format!(
                        "batch input line {line_number} checklist[{index}].assignee_user_ids[{assignee_index}] is duplicated"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn canonical_input_sha256(wires: &[WireOperation]) -> CliResult<[u8; 32]> {
    let mut digest = Sha256::new();
    digest.update(b"sealtask.batch.input.v1\0");
    for wire in wires {
        let record = Zeroizing::new(
            serde_json::to_vec(wire)
                .map_err(|_| public_validation("failed to canonicalize validated batch input"))?,
        );
        digest.update((record.len() as u64).to_be_bytes());
        digest.update(record.as_slice());
    }
    Ok(digest.finalize().into())
}

pub(super) fn operation_key(operation_id: &str) -> String {
    sensitive_fingerprint(b"sealtask.batch.operation.v1\0", operation_id)
}

fn sensitive_fingerprint(domain: &[u8], value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(value.as_bytes());
    hex_digest(digest.finalize().as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn parse_selector(value: &str, line_number: usize, field: &str) -> CliResult<EntitySelector> {
    let selector = EntitySelector::from_str(value).map_err(|error| {
        public_validation(format!(
            "batch input line {line_number} has an invalid {field} selector: {error}"
        ))
    })?;
    if selector.exact_id().is_some_and(|id| id.is_nil()) {
        return Err(public_validation(format!(
            "batch input line {line_number} {field} selector must not be a nil UUID"
        )));
    }
    Ok(selector)
}

impl WireTaskCreateInput {
    fn into_runtime(self) -> TaskCreateInput {
        TaskCreateInput {
            title: self.title.into_inner(),
            body: self.body.map(SensitiveString::into_inner),
            checklist: self
                .checklist
                .map(|items| items.into_iter().map(Into::into).collect()),
            priority: self.priority,
            due_at: self.due_at,
            start_at: self.start_at,
            section_id: self.section_id,
            idempotency_key: self.idempotency_key.map(SensitiveString::into_inner),
        }
    }
}

impl WireTaskUpdateInput {
    fn into_runtime(self) -> TaskUpdateInput {
        let checklist = match self.checklist {
            WirePatch::Unchanged => TaskFieldPatch::Unchanged,
            WirePatch::Set(items) => {
                TaskFieldPatch::Set(items.into_iter().map(Into::into).collect())
            }
            WirePatch::Clear => TaskFieldPatch::Clear,
        };
        TaskUpdateInput {
            title: self.title.map(SensitiveString::into_inner),
            body: self.body.into_runtime_string(),
            checklist,
            priority: self.priority.into_runtime(),
            due_at: self.due_at.into_runtime(),
            start_at: self.start_at.into_runtime(),
            section_id: self.section_id.into_runtime(),
        }
    }
}

impl WirePatch<SensitiveString> {
    fn into_runtime_string(self) -> TaskFieldPatch<String> {
        match self {
            Self::Unchanged => TaskFieldPatch::Unchanged,
            Self::Set(value) => TaskFieldPatch::Set(value.into_inner()),
            Self::Clear => TaskFieldPatch::Clear,
        }
    }
}

impl<T> WirePatch<T> {
    fn into_runtime(self) -> TaskFieldPatch<T> {
        match self {
            Self::Unchanged => TaskFieldPatch::Unchanged,
            Self::Set(value) => TaskFieldPatch::Set(value),
            Self::Clear => TaskFieldPatch::Clear,
        }
    }
}

fn public_validation(message: impl Into<String>) -> CliError {
    PublicError::validation(message).into()
}

pub(super) fn zeroize_task_create_input(input: &mut TaskCreateInput) {
    zeroize_sensitive_string(&mut input.title);
    if let Some(body) = input.body.as_mut() {
        zeroize_sensitive_string(body);
    }
    if let Some(checklist) = input.checklist.as_mut() {
        zeroize_checklist(checklist);
    }
    if let Some(idempotency_key) = input.idempotency_key.as_mut() {
        zeroize_sensitive_string(idempotency_key);
    }
}

pub(super) fn zeroize_task_update_input(input: &mut TaskUpdateInput) {
    if let Some(title) = input.title.as_mut() {
        zeroize_sensitive_string(title);
    }
    if let TaskFieldPatch::Set(body) = &mut input.body {
        zeroize_sensitive_string(body);
    }
    if let TaskFieldPatch::Set(checklist) = &mut input.checklist {
        zeroize_checklist(checklist);
    }
}

fn zeroize_checklist(items: &mut [ChecklistItemPayload]) {
    for item in items {
        zeroize_sensitive_string(&mut item.id);
        zeroize_sensitive_string(&mut item.title);
        if let Some(assignees) = item.assignee_user_ids.as_mut() {
            for assignee in assignees {
                zeroize_sensitive_string(assignee);
            }
        }
    }
}

fn zeroize_sensitive_string(value: &mut String) {
    #[cfg(test)]
    let fingerprint = ZEROIZED_STRING_FINGERPRINTS.with(|fingerprints| {
        (!value.is_empty() && fingerprints.borrow().is_some())
            .then(|| sensitive_fingerprint(ZEROIZE_TEST_DOMAIN, value))
    });

    value.zeroize();

    #[cfg(test)]
    if let Some(fingerprint) = fingerprint {
        ZEROIZED_STRING_FINGERPRINTS.with(|fingerprints| {
            if let Some(fingerprints) = fingerprints.borrow_mut().as_mut() {
                fingerprints.push(fingerprint);
            }
        });
    }
}

impl From<WireChecklistItem> for ChecklistItemPayload {
    fn from(value: WireChecklistItem) -> Self {
        let WireChecklistItem {
            id,
            mut title,
            is_done,
            completed_at,
            assignee_user_ids,
        } = value;
        let normalized_title = title.as_str().trim().to_string();
        title.zeroize();
        Self {
            id: id.into_inner(),
            title: normalized_title,
            is_done,
            completed_at: is_done.then_some(completed_at).flatten(),
            assignee_user_ids: assignee_user_ids.map(|assignees| {
                assignees
                    .into_iter()
                    .map(SensitiveString::into_inner)
                    .collect()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_line(operation_id: &str) -> String {
        format!(
            r#"{{"schemaVersion":1,"operationId":"{operation_id}","type":"task.create","project":"id:0197f52a-89f0-7b50-8000-000000000001","input":{{"title":"Ship it"}}}}"#
        )
    }

    fn begin_zeroize_tracking() {
        ZEROIZED_STRING_FINGERPRINTS.with(|fingerprints| {
            let previous = fingerprints.replace(Some(Vec::new()));
            assert!(previous.is_none(), "zeroize tracking must not be nested");
        });
    }

    fn finish_zeroize_tracking() -> Vec<String> {
        ZEROIZED_STRING_FINGERPRINTS.with(|fingerprints| {
            fingerprints
                .replace(None)
                .expect("zeroize tracking must be active")
        })
    }

    fn observed_zeroization(fingerprints: &[String], value: &str) -> bool {
        let expected = sensitive_fingerprint(ZEROIZE_TEST_DOMAIN, value);
        fingerprints.contains(&expected)
    }

    #[test]
    fn parses_strict_v1_and_defers_implicit_idempotency_until_project_unlock() {
        let input = format!("{}\n", create_line("release-1"));
        let first = read_batch(Cursor::new(input.as_bytes())).expect("valid batch");
        let second = read_batch(Cursor::new(input.as_bytes())).expect("valid batch");
        assert_eq!(first.input_sha256, second.input_sha256);
        let BatchOperationKind::TaskCreate {
            input: first_input,
            idempotency_derivation: first_derivation,
        } = &first.operations[0].kind
        else {
            panic!("expected create")
        };
        let BatchOperationKind::TaskCreate {
            input: second_input,
            idempotency_derivation: second_derivation,
        } = &second.operations[0].kind
        else {
            panic!("expected create")
        };
        assert_eq!(first_input.idempotency_key, second_input.idempotency_key);
        assert!(first_input.idempotency_key.is_none());
        assert!(first_derivation.is_some());
        assert!(second_derivation.is_some());
        let debug = format!("{first_derivation:?}");
        assert!(!debug.contains("release-1"));
        assert!(!debug.contains(&first.input_sha256));
    }

    #[test]
    fn zeroizes_wire_and_runtime_strings_after_successful_normalization() {
        const TITLE: &str = "wire-success-title-canary";
        const BODY: &str = "wire-success-body-canary";
        const PADDED_CHECKLIST_TITLE: &str = "  wire-success-checklist-canary  ";
        const NORMALIZED_CHECKLIST_TITLE: &str = "wire-success-checklist-canary";
        const IDEMPOTENCY_KEY: &str = "wire-success-idempotency-canary";
        let input = format!(
            concat!(
                r#"{{"schemaVersion":1,"operationId":"zeroize-success","type":"task.create","#,
                r#""project":"id:0197f52a-89f0-7b50-8000-000000000001","input":"#,
                r#"{{"title":"{title}","body":"{body}","checklist":["#,
                r#"{{"id":"0197f52a-89f0-7b50-8000-000000000002","title":"{checklist_title}","is_done":false}}"#,
                r#"],"idempotencyKey":"{idempotency_key}"}}}}"#
            ),
            title = TITLE,
            body = BODY,
            checklist_title = PADDED_CHECKLIST_TITLE,
            idempotency_key = IDEMPOTENCY_KEY,
        );

        begin_zeroize_tracking();
        let document = read_batch(Cursor::new(input)).expect("valid batch");
        let BatchOperationKind::TaskCreate {
            input: runtime_input,
            idempotency_derivation,
        } = &document.operations[0].kind
        else {
            panic!("expected create");
        };
        assert!(idempotency_derivation.is_none());
        assert_eq!(
            runtime_input.idempotency_key.as_deref(),
            Some(IDEMPOTENCY_KEY)
        );
        assert_eq!(
            runtime_input
                .checklist
                .as_ref()
                .and_then(|items| items.first())
                .map(|item| item.title.as_str()),
            Some(NORMALIZED_CHECKLIST_TITLE)
        );
        let before_document_drop =
            ZEROIZED_STRING_FINGERPRINTS.with(|fingerprints| fingerprints.borrow().clone());
        assert!(observed_zeroization(
            before_document_drop.as_deref().unwrap_or_default(),
            PADDED_CHECKLIST_TITLE
        ));

        drop(document);
        let fingerprints = finish_zeroize_tracking();
        for canary in [
            TITLE,
            BODY,
            PADDED_CHECKLIST_TITLE,
            NORMALIZED_CHECKLIST_TITLE,
            IDEMPOTENCY_KEY,
        ] {
            assert!(
                observed_zeroization(&fingerprints, canary),
                "{canary} allocation was not zeroized"
            );
        }
    }

    #[test]
    fn zeroizes_partial_wire_strings_on_unknown_and_typed_field_errors() {
        const TITLE: &str = "wire-error-title-canary";
        const BODY: &str = "wire-error-body-canary";
        const CHECKLIST_TITLE: &str = "wire-error-checklist-canary";
        for (invalid_input, expected_zeroized) in [
            (
                format!(
                    concat!(
                        r#"{{"schemaVersion":1,"operationId":"unknown-field","type":"task.create","#,
                        r#""project":"project","input":{{"title":"{title}","body":"{body}","#,
                        r#""password":"unknown-value-canary"}}}}"#
                    ),
                    title = TITLE,
                    body = BODY,
                ),
                &[TITLE, BODY][..],
            ),
            (
                format!(
                    concat!(
                        r#"{{"schemaVersion":1,"operationId":"typed-field","type":"task.create","#,
                        r#""project":"project","input":{{"title":"{title}","body":"{body}","#,
                        r#""priority":"not-a-number"}}}}"#
                    ),
                    title = TITLE,
                    body = BODY,
                ),
                &[TITLE, BODY][..],
            ),
            (
                format!(
                    concat!(
                        r#"{{"schemaVersion":1,"operationId":"typed-update","type":"task.update","#,
                        r#""project":"project","task":"task","input":{{"title":"{title}","body":"{body}","#,
                        r#""checklist":[{{"id":"0197f52a-89f0-7b50-8000-000000000002","#,
                        r#""title":"{checklist_title}","is_done":false}}],"priority":"not-a-number"}}}}"#
                    ),
                    title = TITLE,
                    body = BODY,
                    checklist_title = CHECKLIST_TITLE,
                ),
                &[TITLE, BODY, CHECKLIST_TITLE][..],
            ),
        ] {
            begin_zeroize_tracking();
            let error = read_batch(Cursor::new(invalid_input))
                .err()
                .expect("invalid strict operation");
            let fingerprints = finish_zeroize_tracking();
            assert!(error.to_string().contains("strict JSONL v1"));
            assert!(!error.to_string().contains(TITLE));
            assert!(!error.to_string().contains(BODY));
            for canary in expected_zeroized {
                assert!(
                    observed_zeroization(&fingerprints, canary),
                    "{canary} allocation was not zeroized"
                );
            }
        }
    }

    #[test]
    fn zeroizes_prior_successes_when_a_late_record_has_an_unknown_type() {
        const TITLE: &str = "late-wire-title-canary";
        const BODY: &str = "late-wire-body-canary";
        let first = create_line("first").replace(
            r#""title":"Ship it""#,
            &format!(r#""title":"{TITLE}","body":"{BODY}""#),
        );
        let unknown_type = r#"{"schemaVersion":1,"operationId":"future","type":"task.future","project":"project","input":{"title":"unknown-type-value-canary"}}"#;

        begin_zeroize_tracking();
        let error = read_batch(Cursor::new(format!("{first}\n{unknown_type}\n")))
            .err()
            .expect("unknown operation type");
        let fingerprints = finish_zeroize_tracking();

        assert!(error.to_string().contains("line 2"));
        assert!(error.to_string().contains("strict JSONL v1"));
        assert!(!error.to_string().contains(TITLE));
        assert!(!error.to_string().contains(BODY));
        assert!(observed_zeroization(&fingerprints, TITLE));
        assert!(observed_zeroization(&fingerprints, BODY));
    }

    #[test]
    fn zeroizes_deserialized_strings_on_late_schema_validation_failure() {
        const TITLE: &str = "schema-error-title-canary";
        const BODY: &str = "schema-error-body-canary";
        let invalid = create_line("unsupported-schema")
            .replace(r#""schemaVersion":1"#, r#""schemaVersion":2"#)
            .replace(
                r#""title":"Ship it""#,
                &format!(r#""title":"{TITLE}","body":"{BODY}""#),
            );

        begin_zeroize_tracking();
        let error = read_batch(Cursor::new(invalid))
            .err()
            .expect("unsupported schema");
        let fingerprints = finish_zeroize_tracking();

        assert!(error.to_string().contains("unsupported schemaVersion 2"));
        assert!(!error.to_string().contains(TITLE));
        assert!(!error.to_string().contains(BODY));
        assert!(observed_zeroization(&fingerprints, TITLE));
        assert!(observed_zeroization(&fingerprints, BODY));
    }

    #[test]
    fn canonical_hash_ignores_json_whitespace() {
        let compact = format!("{}\n", create_line("stable"));
        let spaced = concat!(
            "{ \"schemaVersion\": 1, \"operationId\": \"stable\",",
            " \"type\": \"task.create\", \"project\": ",
            "\"id:0197f52a-89f0-7b50-8000-000000000001\",",
            " \"input\": { \"title\": \"Ship it\" } }\n"
        );
        let compact = read_batch(Cursor::new(compact)).expect("compact");
        let spaced = read_batch(Cursor::new(spaced)).expect("spaced");
        assert_eq!(compact.input_sha256, spaced.input_sha256);
    }

    #[test]
    fn rejects_duplicate_ids_unknown_fields_and_late_invalid_records() {
        let duplicate = format!("{}\n{}\n", create_line("same"), create_line("same"));
        assert!(
            read_batch(Cursor::new(duplicate))
                .err()
                .expect("duplicate")
                .to_string()
                .contains("repeats an earlier operationId")
        );

        let unknown = create_line("unknown").replace(
            r#""title":"Ship it""#,
            r#""title":"Ship it","password":"canary""#,
        );
        assert!(
            read_batch(Cursor::new(unknown))
                .err()
                .expect("unknown")
                .to_string()
                .contains("strict JSONL v1")
        );

        let late = format!("{}\nnot-json\n", create_line("first"));
        assert!(
            read_batch(Cursor::new(late))
                .err()
                .expect("late invalid record")
                .to_string()
                .contains("line 2")
        );
    }

    #[test]
    fn rejects_oversized_line_before_deserializing() {
        let input = vec![b'x'; MAX_LINE_BYTES + 1];
        assert!(
            read_batch(Cursor::new(input))
                .err()
                .expect("oversized")
                .to_string()
                .contains("line 1 exceeds")
        );
    }

    #[test]
    fn enforces_total_and_operation_count_bounds() {
        let mut one_byte = Cursor::new(b"x".as_slice());
        let mut line = Vec::new();
        let total_error = read_bounded_line(&mut one_byte, &mut line, MAX_INPUT_BYTES, 1)
            .expect_err("total limit");
        assert!(total_error.to_string().contains("input exceeds"));

        let mut input = String::new();
        for index in 0..=MAX_OPERATIONS {
            input.push_str(&create_line(&format!("op-{index}")));
            input.push('\n');
        }
        let operation_error = read_batch(Cursor::new(input))
            .err()
            .expect("operation limit");
        assert!(operation_error.to_string().contains("operation limit"));
    }

    #[test]
    fn rejects_invalid_operation_id_and_noop_update() {
        let invalid_id = create_line("contains space");
        assert!(
            read_batch(Cursor::new(invalid_id))
                .err()
                .expect("invalid id")
                .to_string()
                .contains("operationId")
        );
        let noop = r#"{"schemaVersion":1,"operationId":"noop","type":"task.update","project":"project","task":"task","input":{}}"#;
        assert!(
            read_batch(Cursor::new(noop))
                .err()
                .expect("no-op")
                .to_string()
                .contains("at least one field")
        );
    }

    #[test]
    fn rejects_duplicate_explicit_idempotency_keys() {
        let first = create_line("one").replace(
            r#""title":"Ship it""#,
            r#""title":"Ship it","idempotencyKey":"shared-key""#,
        );
        let second = create_line("two").replace(
            r#""title":"Ship it""#,
            r#""title":"Another","idempotencyKey":"shared-key""#,
        );
        let error = read_batch(Cursor::new(format!("{first}\n{second}\n")))
            .err()
            .expect("duplicate idempotency key");
        assert!(error.to_string().contains("repeats an explicit"));
        assert!(!error.to_string().contains("shared-key"));
    }

    #[cfg(unix)]
    #[test]
    fn path_input_rejects_symlinks_and_non_regular_files_before_opening() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::TempDir::new().expect("temporary directory");
        let real = directory.path().join("real.jsonl");
        std::fs::write(&real, format!("{}\n", create_line("real"))).expect("write input");
        let link = directory.path().join("link.jsonl");
        symlink(&real, &link).expect("symlink");
        let symlink_error = read_batch_input(&link).err().expect("reject symlink");
        assert!(
            symlink_error
                .to_string()
                .contains("symlinks are not allowed")
        );

        let directory_error = read_batch_input(directory.path())
            .err()
            .expect("reject directory");
        assert!(
            directory_error
                .to_string()
                .contains("input must be a regular file")
        );
    }
}
