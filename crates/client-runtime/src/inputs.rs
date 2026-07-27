use chrono::{DateTime, Utc};
use sealtask_client_api::{DeleteCommentRequest, DeleteNoteRequest, DeleteTaskRequest};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    ChecklistItemPayload, SymmetricKey, derive_batch_task_create_idempotency_key,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Default, PartialEq, Eq)]
pub enum TaskFieldPatch<T> {
    #[default]
    Unchanged,
    Set(T),
    Clear,
}

impl<T> TaskFieldPatch<T> {
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

impl<T> fmt::Debug for TaskFieldPatch<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unchanged => "Unchanged",
            Self::Set(_) => "Set(<redacted>)",
            Self::Clear => "Clear",
        })
    }
}

impl<T: Serialize> Serialize for TaskFieldPatch<T> {
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

impl<'de, T: Deserialize<'de>> Deserialize<'de> for TaskFieldPatch<T> {
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskCreateInput {
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub checklist: Option<Vec<ChecklistItemPayload>>,
    #[serde(default)]
    pub priority: Option<i8>,
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub start_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub section_id: Option<Uuid>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

impl fmt::Debug for TaskCreateInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskCreateInput")
            .field("title_present", &!self.title.is_empty())
            .field("body_present", &self.body.is_some())
            .field("checklist_present", &self.checklist.is_some())
            .field("priority", &self.priority)
            .field("due_at", &self.due_at)
            .field("start_at", &self.start_at)
            .field("section_id", &self.section_id)
            .field("idempotency_key_present", &self.idempotency_key.is_some())
            .finish()
    }
}

#[derive(Clone)]
pub struct TaskCreateIdempotencyDerivation {
    canonical_input_sha256: [u8; 32],
    operation_id: String,
}

impl TaskCreateIdempotencyDerivation {
    #[must_use]
    pub fn new(canonical_input_sha256: [u8; 32], operation_id: impl Into<String>) -> Self {
        Self {
            canonical_input_sha256,
            operation_id: operation_id.into(),
        }
    }

    pub(crate) fn derive_key(
        &self,
        work_list_id: &Uuid,
        list_key: &SymmetricKey,
    ) -> PublicResult<String> {
        derive_batch_task_create_idempotency_key(
            &self.canonical_input_sha256,
            &self.operation_id,
            work_list_id,
            list_key,
        )
    }

    fn zeroize_material(&mut self) {
        self.canonical_input_sha256.zeroize();
        self.operation_id.zeroize();
    }
}

impl fmt::Debug for TaskCreateIdempotencyDerivation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskCreateIdempotencyDerivation")
            .field("canonical_input_sha256", &"<redacted>")
            .field("operation_id", &"<redacted>")
            .finish()
    }
}

impl Drop for TaskCreateIdempotencyDerivation {
    fn drop(&mut self) {
        self.zeroize_material();
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskUpdateInput {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "TaskFieldPatch::is_unchanged")]
    pub body: TaskFieldPatch<String>,
    #[serde(default, skip_serializing_if = "TaskFieldPatch::is_unchanged")]
    pub checklist: TaskFieldPatch<Vec<ChecklistItemPayload>>,
    #[serde(default, skip_serializing_if = "TaskFieldPatch::is_unchanged")]
    pub priority: TaskFieldPatch<i8>,
    #[serde(default, skip_serializing_if = "TaskFieldPatch::is_unchanged")]
    pub due_at: TaskFieldPatch<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "TaskFieldPatch::is_unchanged")]
    pub start_at: TaskFieldPatch<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "TaskFieldPatch::is_unchanged")]
    pub section_id: TaskFieldPatch<Uuid>,
}

impl fmt::Debug for TaskUpdateInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskUpdateInput")
            .field("title_present", &self.title.is_some())
            .field("body", &self.body)
            .field("checklist", &self.checklist)
            .field("priority", &self.priority)
            .field("due_at", &self.due_at)
            .field("start_at", &self.start_at)
            .field("section_id", &self.section_id)
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentInput {
    pub body: String,
}

impl fmt::Debug for CommentInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommentInput")
            .field("body_present", &!self.body.is_empty())
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteCreateInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub is_private: bool,
    /// Stable user-scoped key for retrying one logical create after process
    /// loss or an ambiguous response.
    pub idempotency_key: String,
}

impl fmt::Debug for NoteCreateInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoteCreateInput")
            .field("title_present", &!self.title.is_empty())
            .field("body_present", &!self.body.is_empty())
            .field("is_private", &self.is_private)
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteUpdateInput {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

impl fmt::Debug for NoteUpdateInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoteUpdateInput")
            .field("title_present", &self.title.is_some())
            .field("body_present", &self.body.is_some())
            .finish()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTaskInput {
    pub section_id: Option<Uuid>,
    pub insert_before_task_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct CreateTaskArgs {
    pub work_list_id: Uuid,
    pub input: TaskCreateInput,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct UpdateTaskArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub input: TaskUpdateInput,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct MoveTaskArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub input: MoveTaskInput,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct TaskCompletionArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct ArchiveTaskArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct UnarchiveTaskArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct DeleteTaskArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub input: DeleteTaskRequest,
}

pub struct RepairTaskReferenceSchemeArgs {
    pub work_list_id: Uuid,
    pub prefix: String,
    pub minimum_digits: u8,
    pub password_stdin: bool,
}

impl fmt::Debug for RepairTaskReferenceSchemeArgs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RepairTaskReferenceSchemeArgs")
            .field("work_list_id", &self.work_list_id)
            .field("prefix", &"<redacted>")
            .field("minimum_digits", &self.minimum_digits)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Debug)]
pub struct QuarantineTaskReferenceSchemeArgs {
    pub work_list_id: Uuid,
    pub scheme_revision_id: Uuid,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct CreateCommentArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub input: CommentInput,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct UpdateCommentArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub comment_id: Uuid,
    pub input: CommentInput,
    pub password_stdin: bool,
}

#[derive(Debug)]
pub struct DeleteCommentArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub comment_id: Uuid,
    pub input: DeleteCommentRequest,
}

pub struct CreateNoteArgs {
    pub work_list_id: Uuid,
    pub input: NoteCreateInput,
    pub password_stdin: bool,
}

impl fmt::Debug for CreateNoteArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateNoteArgs")
            .field("work_list_id", &self.work_list_id)
            .field("input", &self.input)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

pub struct UpdateNoteArgs {
    pub work_list_id: Uuid,
    pub note_id: Uuid,
    pub input: NoteUpdateInput,
    pub password_stdin: bool,
}

impl fmt::Debug for UpdateNoteArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateNoteArgs")
            .field("work_list_id", &self.work_list_id)
            .field("note_id", &self.note_id)
            .field("input", &self.input)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Debug)]
pub struct DeleteNoteArgs {
    pub work_list_id: Uuid,
    pub note_id: Uuid,
    pub input: DeleteNoteRequest,
}

#[derive(Debug)]
pub struct UploadTaskAttachmentArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub path: PathBuf,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub password: Option<AttachmentUploadPassword>,
}

pub struct AttachmentUploadPassword(Zeroizing<String>);

impl AttachmentUploadPassword {
    pub fn new(password: impl Into<String>) -> PublicResult<Self> {
        let password = Zeroizing::new(password.into());
        if password.is_empty() {
            return Err(PublicError::validation("password is required"));
        }
        Ok(Self(password))
    }

    pub(crate) fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AttachmentUploadPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AttachmentUploadPassword(<redacted>)")
    }
}

#[derive(Debug)]
pub struct DeleteTaskAttachmentArgs {
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub attachment_id: Uuid,
    pub password_stdin: bool,
}

pub(crate) fn validate_priority(priority: Option<i8>) -> PublicResult<()> {
    if priority.is_some_and(|value| ![1, 3, 5, 8].contains(&value)) {
        return Err(PublicError::validation(
            "priority must be one of 1, 3, 5, or 8",
        ));
    }
    Ok(())
}

pub(crate) fn validate_idempotency_key(value: &str) -> PublicResult<String> {
    const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;

    if value.is_empty() || value.len() > MAX_IDEMPOTENCY_KEY_BYTES {
        return Err(PublicError::validation(
            "idempotencyKey must contain between 1 and 128 bytes",
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(PublicError::validation(
            "idempotencyKey may contain only ASCII letters, digits, '-', '_', '.', and ':'",
        ));
    }
    Ok(value.to_string())
}

pub(crate) fn normalize_checklist(
    mut items: Vec<ChecklistItemPayload>,
) -> PublicResult<Vec<ChecklistItemPayload>> {
    const MAX_CHECKLIST_ITEMS: usize = 200;
    const MAX_CHECKLIST_TITLE_CHARS: usize = 1_024;
    const MAX_CHECKLIST_ASSIGNEES: usize = 16;

    let result = (|| {
        if items.len() > MAX_CHECKLIST_ITEMS {
            return Err(PublicError::validation(format!(
                "checklist cannot exceed {MAX_CHECKLIST_ITEMS} entries"
            )));
        }

        let mut item_ids = HashSet::with_capacity(items.len());
        for (index, item) in items.iter_mut().enumerate() {
            let item_id = Uuid::parse_str(&item.id).map_err(|_| {
                PublicError::validation(format!("checklist[{index}].id must be a UUID"))
            })?;
            if !item_ids.insert(item_id) {
                return Err(PublicError::validation(format!(
                    "checklist[{index}].id is duplicated"
                )));
            }

            let normalized_title = item.title.trim();
            if normalized_title.is_empty()
                || normalized_title.chars().count() > MAX_CHECKLIST_TITLE_CHARS
            {
                return Err(PublicError::validation(format!(
                    "checklist[{index}].title must contain between 1 and {MAX_CHECKLIST_TITLE_CHARS} characters"
                )));
            }
            item.title = normalized_title.to_string();
            if !item.is_done {
                item.completed_at = None;
            }

            if let Some(assignees) = item.assignee_user_ids.as_ref() {
                if assignees.len() > MAX_CHECKLIST_ASSIGNEES {
                    return Err(PublicError::validation(format!(
                        "checklist[{index}].assignee_user_ids cannot exceed {MAX_CHECKLIST_ASSIGNEES} entries"
                    )));
                }
                let mut assignee_ids = HashSet::with_capacity(assignees.len());
                for (assignee_index, assignee) in assignees.iter().enumerate() {
                    let assignee_id = Uuid::parse_str(assignee).map_err(|_| {
                        PublicError::validation(format!(
                            "checklist[{index}].assignee_user_ids[{assignee_index}] must be a UUID"
                        ))
                    })?;
                    if !assignee_ids.insert(assignee_id) {
                        return Err(PublicError::validation(format!(
                            "checklist[{index}].assignee_user_ids[{assignee_index}] is duplicated"
                        )));
                    }
                }
            }
        }

        Ok(())
    })();
    if let Err(error) = result {
        for item in &mut items {
            item.id.zeroize();
            item.title.zeroize();
            if let Some(assignees) = item.assignee_user_ids.as_mut() {
                assignees.iter_mut().for_each(Zeroize::zeroize);
            }
        }
        return Err(error);
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_distinguish_missing_null_and_value_in_task_update_input() {
        let missing: TaskUpdateInput =
            serde_json::from_value(serde_json::json!({})).expect("missing fields");
        assert_eq!(missing.body, TaskFieldPatch::Unchanged);
        assert_eq!(missing.priority, TaskFieldPatch::Unchanged);

        let patched: TaskUpdateInput = serde_json::from_value(serde_json::json!({
            "body": null,
            "priority": 5,
            "dueAt": null
        }))
        .expect("patched fields");
        assert_eq!(patched.body, TaskFieldPatch::Clear);
        assert_eq!(patched.priority, TaskFieldPatch::Set(5));
        assert_eq!(patched.due_at, TaskFieldPatch::Clear);
    }

    #[test]
    fn test_should_reject_unknown_task_input_fields() {
        let error = serde_json::from_value::<TaskCreateInput>(serde_json::json!({
            "title": "Task",
            "priorty": 5
        }))
        .expect_err("typo must be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn attachment_upload_password_debug_is_redacted() {
        let password =
            AttachmentUploadPassword::new("correct horse battery staple").expect("valid password");

        let debug = format!("{password:?}");

        assert_eq!(debug, "AttachmentUploadPassword(<redacted>)");
        assert!(!debug.contains("correct horse battery staple"));
    }

    #[test]
    fn note_input_and_argument_debug_redact_plaintext() {
        let create_title = "runtime-create-title-canary";
        let create_body = "runtime-create-body-canary";
        let create = CreateNoteArgs {
            work_list_id: Uuid::now_v7(),
            input: NoteCreateInput {
                title: create_title.to_string(),
                body: create_body.to_string(),
                is_private: true,
                idempotency_key: "note:debug".to_string(),
            },
            password_stdin: false,
        };
        let create_debug = format!("{create:?}");
        assert!(!create_debug.contains(create_title));
        assert!(!create_debug.contains(create_body));
        assert!(create_debug.contains("title_present: true"));
        assert!(create_debug.contains("body_present: true"));
        assert!(create_debug.contains("is_private: true"));

        let update_title = "runtime-update-title-canary";
        let update_body = "runtime-update-body-canary";
        let update = UpdateNoteArgs {
            work_list_id: Uuid::now_v7(),
            note_id: Uuid::now_v7(),
            input: NoteUpdateInput {
                title: Some(update_title.to_string()),
                body: Some(update_body.to_string()),
            },
            password_stdin: false,
        };
        let update_debug = format!("{update:?}");
        assert!(!update_debug.contains(update_title));
        assert!(!update_debug.contains(update_body));
        assert!(update_debug.contains("title_present: true"));
        assert!(update_debug.contains("body_present: true"));
    }

    #[test]
    fn task_and_comment_debug_redact_plaintext_fields() {
        let title = "task-title-debug-canary";
        let body = "body-debug-canary";
        let checklist_title = "checklist-debug-canary";
        let task = TaskCreateInput {
            title: title.to_string(),
            body: Some(body.to_string()),
            checklist: Some(vec![ChecklistItemPayload {
                id: Uuid::now_v7().to_string(),
                title: checklist_title.to_string(),
                is_done: false,
                completed_at: None,
                assignee_user_ids: None,
            }]),
            priority: Some(5),
            due_at: None,
            start_at: None,
            section_id: None,
            idempotency_key: Some("task:debug".to_string()),
        };
        let patch = TaskUpdateInput {
            title: Some(title.to_string()),
            body: TaskFieldPatch::Set(body.to_string()),
            checklist: TaskFieldPatch::Set(task.checklist.clone().expect("checklist")),
            priority: TaskFieldPatch::Unchanged,
            due_at: TaskFieldPatch::Unchanged,
            start_at: TaskFieldPatch::Unchanged,
            section_id: TaskFieldPatch::Unchanged,
        };
        let comment = CommentInput {
            body: body.to_string(),
        };

        for debug in [
            format!("{task:?}"),
            format!("{patch:?}"),
            format!("{comment:?}"),
        ] {
            assert!(!debug.contains(title));
            assert!(!debug.contains(body));
            assert!(!debug.contains(checklist_title));
        }
        assert_eq!(
            format!("{:?}", TaskFieldPatch::Set(body.to_string())),
            "Set(<redacted>)"
        );
    }

    #[test]
    fn task_create_idempotency_derivation_redacts_and_zeroizes_material() {
        let operation_id = "batch-operation-debug-canary";
        let mut derivation = TaskCreateIdempotencyDerivation::new([0xab; 32], operation_id);

        let debug = format!("{derivation:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains(operation_id));
        assert!(!debug.contains("171"));

        derivation.zeroize_material();
        assert_eq!(derivation.canonical_input_sha256, [0; 32]);
        assert!(derivation.operation_id.is_empty());
    }
}
