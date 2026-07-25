use chrono::{DateTime, Utc};
use sealtask_client_api::{CurrentUserResponse, DashboardStatsResponse, SectionSnapshotPayload};
use sealtask_client_crypto::{ChecklistItemPayload, TaskPayloadRichText};
use sealtask_client_runtime::{
    AgentAttachment, AgentComment, AgentDelegation, AgentMembership, AgentNote, AgentTaskDetail,
    AgentTaskSummary, AgentWorkListDetail, AgentWorkListSummary, ReadError, ReadableAttachment,
    ReadableAttachmentContentFormat, ReadableAttachmentSourceKind,
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadErrorV1<'a> {
    code: &'a str,
    message: &'a str,
}

impl<'a> From<&'a ReadError> for ReadErrorV1<'a> {
    fn from(value: &'a ReadError) -> Self {
        Self {
            code: &value.code,
            message: &value.message,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MembershipV1<'a> {
    id: Uuid,
    user_id: Uuid,
    user_email: &'a str,
    user_name: &'a str,
    user_avatar_color: &'a str,
    role: &'a str,
    status: &'a str,
    expires_at: Option<&'a DateTime<Utc>>,
    joined_at: &'a DateTime<Utc>,
}

impl<'a> From<&'a AgentMembership> for MembershipV1<'a> {
    fn from(value: &'a AgentMembership) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            user_email: &value.user_email,
            user_name: &value.user_name,
            user_avatar_color: &value.user_avatar_color,
            role: &value.role,
            status: &value.status,
            expires_at: value.expires_at.as_ref(),
            joined_at: &value.joined_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkListSummaryV1<'a> {
    id: Uuid,
    owner_user_id: Uuid,
    workspace_id: Uuid,
    timezone: &'a str,
    section_snapshots: &'a [SectionSnapshotPayload],
    created_at: &'a DateTime<Utc>,
    updated_at: &'a DateTime<Utc>,
    archived_at: Option<&'a DateTime<Utc>>,
    membership: MembershipV1<'a>,
    title: Option<&'a str>,
    description: Option<&'a str>,
    payload: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_error: Option<ReadErrorV1<'a>>,
}

impl<'a> From<&'a AgentWorkListSummary> for WorkListSummaryV1<'a> {
    fn from(value: &'a AgentWorkListSummary) -> Self {
        Self {
            id: value.id,
            owner_user_id: value.owner_user_id,
            workspace_id: value.workspace_id,
            timezone: &value.timezone,
            section_snapshots: &value.section_snapshots,
            created_at: &value.created_at,
            updated_at: &value.updated_at,
            archived_at: value.archived_at.as_ref(),
            membership: (&value.membership).into(),
            title: value.title.as_deref(),
            description: value.description.as_deref(),
            payload: value.payload.as_ref(),
            read_error: value.read_error.as_ref().map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkListDetailV1<'a> {
    #[serde(flatten)]
    work_list: WorkListSummaryV1<'a>,
    members: Vec<MembershipV1<'a>>,
}

impl<'a> From<&'a AgentWorkListDetail> for WorkListDetailV1<'a> {
    fn from(value: &'a AgentWorkListDetail) -> Self {
        Self {
            work_list: (&value.work_list).into(),
            members: value.members.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DelegationV1<'a> {
    id: Uuid,
    task_id: Uuid,
    membership_id: Uuid,
    role: &'a str,
    status: &'a str,
    note_present: bool,
    created_at: &'a DateTime<Utc>,
    updated_at: &'a DateTime<Utc>,
}

impl<'a> From<&'a AgentDelegation> for DelegationV1<'a> {
    fn from(value: &'a AgentDelegation) -> Self {
        Self {
            id: value.id,
            task_id: value.task_id,
            membership_id: value.membership_id,
            role: &value.role,
            status: &value.status,
            note_present: value.note_present,
            created_at: &value.created_at,
            updated_at: &value.updated_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentV1<'a> {
    id: Uuid,
    file_name: &'a str,
    content_type: &'a str,
    size_bytes: u64,
}

impl<'a> From<&'a AgentAttachment> for AttachmentV1<'a> {
    fn from(value: &'a AgentAttachment) -> Self {
        Self {
            id: value.id,
            file_name: &value.file_name,
            content_type: &value.content_type,
            size_bytes: value.size_bytes,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskSummaryV1<'a> {
    id: Uuid,
    work_list_id: Uuid,
    work_list_title: Option<&'a str>,
    created_by_membership_id: Uuid,
    section_id: Option<Uuid>,
    priority: Option<i8>,
    position: Option<&'a str>,
    due_at: Option<&'a DateTime<Utc>>,
    start_at: Option<&'a DateTime<Utc>>,
    completed_at: Option<&'a DateTime<Utc>>,
    archived_at: Option<&'a DateTime<Utc>>,
    is_completed: bool,
    recurrence_id: Option<Uuid>,
    recurrence_schedule: Option<&'a str>,
    recurrence_iteration: Option<i64>,
    materialized_at: Option<&'a DateTime<Utc>>,
    created_at: &'a DateTime<Utc>,
    updated_at: &'a DateTime<Utc>,
    comment_count: i64,
    title: Option<&'a str>,
    body_markdown: Option<&'a str>,
    body_rich_text: Option<&'a TaskPayloadRichText>,
    checklist: Option<&'a [ChecklistItemPayload]>,
    attachments: Option<Vec<AttachmentV1<'a>>>,
    references: Option<&'a [Value]>,
    mentions: Option<&'a [String]>,
    client_meta: Option<&'a Value>,
    recurrence_state: Option<&'a Value>,
    delegations: Vec<DelegationV1<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_error: Option<ReadErrorV1<'a>>,
}

impl<'a> From<&'a AgentTaskSummary> for TaskSummaryV1<'a> {
    fn from(value: &'a AgentTaskSummary) -> Self {
        Self {
            id: value.id,
            work_list_id: value.work_list_id,
            work_list_title: value.work_list_title.as_deref(),
            created_by_membership_id: value.created_by_membership_id,
            section_id: value.section_id,
            priority: value.priority,
            position: value.position.as_deref(),
            due_at: value.due_at.as_ref(),
            start_at: value.start_at.as_ref(),
            completed_at: value.completed_at.as_ref(),
            archived_at: value.archived_at.as_ref(),
            is_completed: value.is_completed,
            recurrence_id: value.recurrence_id,
            recurrence_schedule: value.recurrence_schedule.as_deref(),
            recurrence_iteration: value.recurrence_iteration,
            materialized_at: value.materialized_at.as_ref(),
            created_at: &value.created_at,
            updated_at: &value.updated_at,
            comment_count: value.comment_count,
            title: value.title.as_deref(),
            body_markdown: value.body_markdown.as_deref(),
            body_rich_text: value.body_rich_text.as_ref(),
            checklist: value.checklist.as_deref(),
            attachments: value
                .attachments
                .as_ref()
                .map(|items| items.iter().map(Into::into).collect()),
            references: value.references.as_deref(),
            mentions: value.mentions.as_deref(),
            client_meta: value.client_meta.as_ref(),
            recurrence_state: value.recurrence_state.as_ref(),
            delegations: value.delegations.iter().map(Into::into).collect(),
            read_error: value.read_error.as_ref().map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommentV1<'a> {
    id: Uuid,
    task_id: Uuid,
    author_membership_id: Uuid,
    body_markdown: Option<&'a str>,
    content: Option<&'a TaskPayloadRichText>,
    mentions: Option<&'a [String]>,
    attachments: Option<Vec<AttachmentV1<'a>>>,
    client_meta: Option<&'a Value>,
    created_at: &'a DateTime<Utc>,
    updated_at: &'a DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_error: Option<ReadErrorV1<'a>>,
}

impl<'a> From<&'a AgentComment> for CommentV1<'a> {
    fn from(value: &'a AgentComment) -> Self {
        Self {
            id: value.id,
            task_id: value.task_id,
            author_membership_id: value.author_membership_id,
            body_markdown: value.body_markdown.as_deref(),
            content: value.content.as_ref(),
            mentions: value.mentions.as_deref(),
            attachments: value
                .attachments
                .as_ref()
                .map(|items| items.iter().map(Into::into).collect()),
            client_meta: value.client_meta.as_ref(),
            created_at: &value.created_at,
            updated_at: &value.updated_at,
            read_error: value.read_error.as_ref().map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoteV1<'a> {
    id: Uuid,
    work_list_id: Uuid,
    created_by_membership_id: Uuid,
    is_private: bool,
    title: Option<&'a str>,
    body_markdown: Option<&'a str>,
    content: Option<&'a TaskPayloadRichText>,
    mentions: Option<&'a [String]>,
    attachments: Option<Vec<AttachmentV1<'a>>>,
    client_meta: Option<&'a Value>,
    created_at: &'a DateTime<Utc>,
    updated_at: &'a DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_error: Option<ReadErrorV1<'a>>,
}

impl<'a> From<&'a AgentNote> for NoteV1<'a> {
    fn from(value: &'a AgentNote) -> Self {
        Self {
            id: value.id,
            work_list_id: value.work_list_id,
            created_by_membership_id: value.created_by_membership_id,
            is_private: value.is_private,
            title: value.title.as_deref(),
            body_markdown: value.body_markdown.as_deref(),
            content: value.content.as_ref(),
            mentions: value.mentions.as_deref(),
            attachments: value
                .attachments
                .as_ref()
                .map(|items| items.iter().map(Into::into).collect()),
            client_meta: value.client_meta.as_ref(),
            created_at: &value.created_at,
            updated_at: &value.updated_at,
            read_error: value.read_error.as_ref().map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskDetailV1<'a> {
    #[serde(flatten)]
    task: TaskSummaryV1<'a>,
    comments: Vec<CommentV1<'a>>,
}

impl<'a> From<&'a AgentTaskDetail> for TaskDetailV1<'a> {
    fn from(value: &'a AgentTaskDetail) -> Self {
        Self {
            task: (&value.task).into(),
            comments: value.comments.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadableAttachmentV1<'a> {
    attachment: AttachmentV1<'a>,
    text: &'a str,
    content_format: ReadableAttachmentContentFormat,
    source_kind: ReadableAttachmentSourceKind,
}

impl<'a> From<&'a ReadableAttachment> for ReadableAttachmentV1<'a> {
    fn from(value: &'a ReadableAttachment) -> Self {
        Self {
            attachment: (&value.attachment).into(),
            text: &value.text,
            content_format: value.content_format,
            source_kind: value.source_kind,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentUserV1<'a> {
    id: Uuid,
    email: &'a str,
    name: &'a str,
    timezone: &'a str,
    avatar_color: &'a str,
    data_key_ciphertext: &'a str,
    workspace_lock_timeout_minutes: Option<i32>,
    theme_preference: &'a str,
    email_verified: bool,
    last_accessed_work_list_id: Option<Uuid>,
}

impl<'a> From<&'a CurrentUserResponse> for CurrentUserV1<'a> {
    fn from(value: &'a CurrentUserResponse) -> Self {
        Self {
            id: value.id,
            email: &value.email,
            name: &value.name,
            timezone: &value.timezone,
            avatar_color: &value.avatar_color,
            data_key_ciphertext: &value.data_key_ciphertext,
            workspace_lock_timeout_minutes: value.workspace_lock_timeout_minutes,
            theme_preference: &value.theme_preference,
            email_verified: value.email_verified,
            last_accessed_work_list_id: value.last_accessed_work_list_id,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardStatsV1 {
    tasks_overdue: i64,
    tasks_due_today: i64,
    tasks_due_this_week: i64,
    completed: i64,
}

impl From<&DashboardStatsResponse> for DashboardStatsV1 {
    fn from(value: &DashboardStatsResponse) -> Self {
        Self {
            tasks_overdue: value.tasks_overdue,
            tasks_due_today: value.tasks_due_today,
            tasks_due_this_week: value.tasks_due_this_week,
            completed: value.completed,
        }
    }
}

pub(crate) fn work_list_summaries_v1(
    values: &[AgentWorkListSummary],
) -> Vec<WorkListSummaryV1<'_>> {
    values.iter().map(Into::into).collect()
}

pub(crate) fn task_summaries_v1(values: &[AgentTaskSummary]) -> Vec<TaskSummaryV1<'_>> {
    values.iter().map(Into::into).collect()
}

pub(crate) fn comments_v1(values: &[AgentComment]) -> Vec<CommentV1<'_>> {
    values.iter().map(Into::into).collect()
}

pub(crate) fn notes_v1(values: &[AgentNote]) -> Vec<NoteV1<'_>> {
    values.iter().map(Into::into).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_task_v1_dto_preserves_the_published_shape() {
        let now = Utc::now();
        let task = AgentTaskSummary {
            id: Uuid::now_v7(),
            work_list_id: Uuid::now_v7(),
            work_list_title: Some("Project".to_string()),
            created_by_membership_id: Uuid::now_v7(),
            section_id: Some(Uuid::now_v7()),
            priority: Some(5),
            position: Some("a0".to_string()),
            due_at: Some(now),
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
            comment_count: 2,
            title: Some("Contract task".to_string()),
            body_markdown: Some("Body".to_string()),
            body_rich_text: None,
            checklist: None,
            attachments: None,
            references: Some(vec![serde_json::json!({"kind": "test"})]),
            mentions: Some(vec!["user".to_string()]),
            client_meta: Some(serde_json::json!({"source": "test"})),
            recurrence_state: None,
            delegations: Vec::new(),
            read_error: None,
        };

        assert_eq!(
            serde_json::to_value(TaskSummaryV1::from(&task)).expect("v1 task JSON"),
            serde_json::to_value(&task).expect("legacy task JSON")
        );
    }

    #[test]
    fn explicit_work_list_v1_dto_preserves_the_published_shape() {
        let now = Utc::now();
        let list = AgentWorkListSummary {
            id: Uuid::now_v7(),
            owner_user_id: Uuid::now_v7(),
            workspace_id: Uuid::now_v7(),
            timezone: "Europe/Prague".to_string(),
            section_snapshots: Vec::new(),
            created_at: now,
            updated_at: now,
            archived_at: None,
            membership: AgentMembership {
                id: Uuid::now_v7(),
                user_id: Uuid::now_v7(),
                user_email: "agent@example.test".to_string(),
                user_name: "Agent".to_string(),
                user_avatar_color: "blue".to_string(),
                role: "owner".to_string(),
                status: "active".to_string(),
                expires_at: None,
                joined_at: now,
            },
            title: Some("Contract project".to_string()),
            description: None,
            payload: Some(serde_json::json!({"version": 1})),
            read_error: None,
        };

        assert_eq!(
            serde_json::to_value(WorkListSummaryV1::from(&list)).expect("v1 work-list JSON"),
            serde_json::to_value(&list).expect("legacy work-list JSON")
        );
    }
}
