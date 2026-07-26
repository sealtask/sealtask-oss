#![cfg_attr(test, allow(clippy::unwrap_used))]

mod attachments;
pub mod note_transport;
mod note_transport_limits;
mod notes;
mod transport;

pub use attachments::{
    CompleteAttachmentUploadRequest, DownloadAttachmentResponse, InitiateAttachmentUploadRequest,
    InitiateAttachmentUploadResponse,
};
pub use note_transport_limits::{
    DEFAULT_NOTE_PAGE_ITEMS, MAX_NOTE_COLLECTION_ENCODED_BYTES, MAX_NOTE_COLLECTION_ITEMS,
    MAX_NOTE_COLLECTION_PAGES, MAX_NOTE_CURSOR_BYTES, MAX_NOTE_DECOMPRESSED_PAGE_BYTES,
    MAX_NOTE_ENCODED_PAGE_BYTES, MAX_NOTE_MUTATION_REQUEST_BYTES, MAX_NOTE_PAGE_ITEMS,
    MIN_NOTE_PAGE_ITEMS,
};
pub use notes::{CreateNoteRequest, DeleteNoteRequest, NotePage, NoteResponse, UpdateNoteRequest};
pub use transport::{
    ApiTransportOptions, CONTROL_PLANE_USER_AGENT, DEFAULT_API_CONNECT_TIMEOUT,
    DEFAULT_API_READ_TIMEOUT, DEFAULT_API_REQUEST_TIMEOUT, PublicApiClient, RequestCorrelation,
    build_control_plane_http_client,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sealtask_client_core::{PublicError, PublicResult};

pub(crate) use transport::{
    BoundedHttpResponse, decode_bounded_json, map_api_error_with_retry_after,
};

pub type SealedBlob = String;

const MY_TASKS_PAGE_LIMIT: i64 = 100;
const MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES: usize = 64 * 1024;

impl PublicApiClient {
    pub async fn get_me(&mut self) -> PublicResult<CurrentUserResponse> {
        self.get("/me").await
    }

    pub async fn list_work_lists(&mut self) -> PublicResult<Vec<WorkListResponse>> {
        self.list_work_lists_with_archived(false).await
    }

    pub async fn list_work_lists_with_archived(
        &mut self,
        include_archived: bool,
    ) -> PublicResult<Vec<WorkListResponse>> {
        let path = if include_archived {
            "/work-lists?includeArchived=true"
        } else {
            "/work-lists"
        };
        self.get(path).await
    }

    pub async fn get_work_list(&mut self, id: Uuid) -> PublicResult<WorkListDetailResponse> {
        self.get(&format!("/work-lists/{id}")).await
    }

    pub async fn get_task_reference_schemes(
        &mut self,
        work_list_id: Uuid,
    ) -> PublicResult<Vec<TaskReferenceSchemeResponse>> {
        let response: TaskReferenceSchemeListResponse = self
            .get(&format!(
                "/work-lists/{work_list_id}/task-reference-schemes"
            ))
            .await?;
        Ok(response.schemes)
    }

    pub async fn repair_task_reference_scheme(
        &mut self,
        work_list_id: Uuid,
        request: &TaskReferenceSchemeMutationRequest,
    ) -> PublicResult<TaskReferenceSchemeResponse> {
        self.post(
            &format!("/work-lists/{work_list_id}/task-reference-schemes/repairs"),
            request,
        )
        .await
    }

    pub async fn quarantine_task_reference_scheme(
        &mut self,
        work_list_id: Uuid,
        scheme_revision_id: Uuid,
        request: &TaskReferenceSchemeQuarantineRequest,
    ) -> PublicResult<TaskReferenceSchemeResponse> {
        self.post(
            &format!(
                "/work-lists/{work_list_id}/task-reference-schemes/{scheme_revision_id}/quarantine"
            ),
            request,
        )
        .await
    }

    pub async fn archive_work_list(&mut self, id: Uuid) -> PublicResult<WorkListResponse> {
        self.post(
            &format!("/work-lists/{id}/archive"),
            &ArchiveWorkListRequest::default(),
        )
        .await
    }

    pub async fn unarchive_work_list(&mut self, id: Uuid) -> PublicResult<WorkListResponse> {
        self.post(
            &format!("/work-lists/{id}/unarchive"),
            &UnarchiveWorkListRequest::default(),
        )
        .await
    }

    pub async fn get_tasks(
        &mut self,
        work_list_id: Uuid,
        include_archived: bool,
    ) -> PublicResult<TaskListResponse> {
        let path = if include_archived {
            format!("/work-lists/{work_list_id}/tasks?includeArchived=true")
        } else {
            format!("/work-lists/{work_list_id}/tasks")
        };
        self.get(&path).await
    }

    pub async fn get_my_tasks(
        &mut self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> PublicResult<MyTasksResponse> {
        self.get_my_tasks_page(limit, offset, false).await
    }

    pub async fn get_all_my_tasks(
        &mut self,
        include_completed: bool,
    ) -> PublicResult<Vec<MyTaskResponse>> {
        let mut tasks = Vec::new();
        let mut offset = 0;
        let mut target_total = None;

        loop {
            let page = self
                .get_my_tasks_page(Some(MY_TASKS_PAGE_LIMIT), Some(offset), include_completed)
                .await?;
            if page.offset != offset {
                return Err(PublicError::unexpected(format!(
                    "invalid /me/tasks page offset: requested {offset}, received {}",
                    page.offset
                )));
            }
            if page.limit <= 0 || page.total < 0 {
                return Err(PublicError::unexpected(
                    "invalid /me/tasks pagination metadata",
                ));
            }

            let fetched = i64::try_from(page.tasks.len()).map_err(|_| {
                PublicError::unexpected("/me/tasks page length exceeds the supported range")
            })?;
            let target_total = *target_total.get_or_insert(page.total);
            tasks.extend(page.tasks);

            let collected = i64::try_from(tasks.len()).map_err(|_| {
                PublicError::unexpected("/me/tasks result length exceeds the supported range")
            })?;
            if fetched == 0 || collected >= target_total {
                break;
            }

            offset = offset
                .checked_add(fetched)
                .ok_or_else(|| PublicError::unexpected("/me/tasks pagination offset overflowed"))?;
        }

        Ok(tasks)
    }

    async fn get_my_tasks_page(
        &mut self,
        limit: Option<i64>,
        offset: Option<i64>,
        include_completed: bool,
    ) -> PublicResult<MyTasksResponse> {
        let mut params = Vec::new();
        if let Some(limit) = limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={offset}"));
        }
        if include_completed {
            params.push("includeCompleted=true".to_string());
        }

        let path = if params.is_empty() {
            "/me/tasks".to_string()
        } else {
            format!("/me/tasks?{}", params.join("&"))
        };

        self.get(&path).await
    }

    pub async fn get_dashboard_stats(&mut self) -> PublicResult<DashboardStatsResponse> {
        self.get("/me/dashboard-stats").await
    }

    pub async fn start_opaque_export_key(
        &mut self,
        client_login_state: &str,
    ) -> PublicResult<OpaqueExportKeyStartResponse> {
        self.post(
            "/auth/opaque/export-key/start",
            &OpaqueExportKeyStartRequest { client_login_state },
        )
        .await
    }

    pub async fn get_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
    ) -> PublicResult<TaskDetailResponse> {
        self.get(&format!("/work-lists/{work_list_id}/tasks/{task_id}"))
            .await
    }

    pub async fn get_task_by_reference_number(
        &mut self,
        work_list_id: Uuid,
        reference_number: i64,
    ) -> PublicResult<TaskDetailResponse> {
        self.get(&format!(
            "/work-lists/{work_list_id}/tasks/by-reference-number/{reference_number}"
        ))
        .await
    }

    pub async fn create_task(
        &mut self,
        work_list_id: Uuid,
        payload: &CreateTaskRequest,
    ) -> PublicResult<TaskResponse> {
        self.post(&format!("/work-lists/{work_list_id}/tasks"), payload)
            .await
    }

    pub async fn update_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        payload: &UpdateTaskRequest,
    ) -> PublicResult<TaskResponse> {
        self.patch(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}"),
            payload,
        )
        .await
    }

    pub async fn move_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        payload: &MoveTaskRequest,
    ) -> PublicResult<TaskResponse> {
        self.post(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/move"),
            payload,
        )
        .await
    }

    pub async fn archive_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        payload: &ArchiveTaskRequest,
    ) -> PublicResult<TaskResponse> {
        self.post(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/archive"),
            payload,
        )
        .await
    }

    pub async fn unarchive_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        payload: &UnarchiveTaskRequest,
    ) -> PublicResult<TaskResponse> {
        self.post(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/unarchive"),
            payload,
        )
        .await
    }

    pub async fn delete_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        payload: &DeleteTaskRequest,
    ) -> PublicResult<()> {
        self.delete_no_content_with_body(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}"),
            payload,
        )
        .await
    }

    pub async fn list_comments(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
    ) -> PublicResult<Vec<CommentResponse>> {
        self.get(&format!(
            "/work-lists/{work_list_id}/tasks/{task_id}/comments"
        ))
        .await
    }

    pub async fn create_comment(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        payload: &CreateCommentRequest,
    ) -> PublicResult<CommentResponse> {
        self.post(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/comments"),
            payload,
        )
        .await
    }

    pub async fn update_comment(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        comment_id: Uuid,
        payload: &UpdateCommentRequest,
    ) -> PublicResult<CommentResponse> {
        self.patch(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/comments/{comment_id}"),
            payload,
        )
        .await
    }

    pub async fn delete_comment(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        comment_id: Uuid,
        payload: &DeleteCommentRequest,
    ) -> PublicResult<()> {
        self.delete_no_content_with_body(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/comments/{comment_id}"),
            payload,
        )
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicTaskRef {
    pub id: Uuid,
    pub work_list_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub timezone: String,
    pub avatar_color: String,
    pub data_key_ciphertext: String,
    pub workspace_lock_timeout_minutes: Option<i32>,
    pub theme_preference: String,
    pub email_verified: bool,
    pub last_accessed_work_list_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpaqueExportKeyStartRequest<'a> {
    client_login_state: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpaqueExportKeyStartResponse {
    pub server_login_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkListResponse {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub workspace_id: Uuid,
    pub title_ciphertext: SealedBlob,
    pub description_ciphertext: Option<SealedBlob>,
    pub payload_ciphertext: SealedBlob,
    pub timezone: String,
    pub section_snapshots: Vec<SectionSnapshotPayload>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub task_references_enabled_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub current_task_reference_scheme_revision: Option<i64>,
    #[serde(default)]
    pub current_task_reference_scheme_revision_id: Option<Uuid>,
    pub membership: MembershipResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkListDetailResponse {
    #[serde(flatten)]
    pub work_list: WorkListResponse,
    pub members: Vec<MembershipResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionSnapshotPayload {
    pub id: Uuid,
    pub position: i64,
    pub auto_archive_enabled: bool,
    pub auto_archive_after_days: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MembershipResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: String,
    pub user_name: String,
    pub user_avatar_color: String,
    pub role: String,
    pub status: String,
    pub work_list_key_ciphertext: SealedBlob,
    pub recipient_ciphertext: Option<SealedBlob>,
    pub invite_package_ciphertext: Option<SealedBlob>,
    pub salt_member: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub joined_at: DateTime<Utc>,
    pub payload_binding_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub id: Uuid,
    pub work_list_id: Uuid,
    pub created_by_membership_id: Uuid,
    pub title_ciphertext: SealedBlob,
    pub payload_ciphertext: SealedBlob,
    pub section_id: Option<Uuid>,
    pub priority: Option<i8>,
    pub position: String,
    pub due_at: Option<DateTime<Utc>>,
    pub start_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub is_completed: bool,
    pub recurrence_id: Option<Uuid>,
    pub recurrence_schedule: Option<String>,
    pub recurrence_iteration: Option<i64>,
    pub materialized_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comment_count: i64,
    #[serde(default)]
    pub reference_number: Option<i64>,
    #[serde(default)]
    pub delegations: Vec<DelegationResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetailResponse {
    #[serde(flatten)]
    pub task: TaskResponse,
    pub comments: Vec<CommentResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub membership_id: Uuid,
    pub role: String,
    pub status: String,
    pub note_ciphertext: Option<SealedBlob>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub author_membership_id: Uuid,
    pub body_ciphertext: SealedBlob,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    #[serde(default)]
    pub archived_counts: Vec<ArchivedTaskCountResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedTaskCountResponse {
    pub section_id: Option<Uuid>,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyTasksResponse {
    pub tasks: Vec<MyTaskResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyTaskResponse {
    pub id: Uuid,
    pub work_list_id: Uuid,
    pub work_list_title_ciphertext: SealedBlob,
    pub created_by_membership_id: Uuid,
    pub title_ciphertext: SealedBlob,
    pub payload_ciphertext: SealedBlob,
    pub section_id: Option<Uuid>,
    pub priority: Option<i8>,
    pub due_at: Option<DateTime<Utc>>,
    pub start_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comment_count: i64,
    #[serde(default)]
    pub reference_number: Option<i64>,
    #[serde(default)]
    pub delegations: Vec<DelegationResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReferenceSchemeResponse {
    pub scheme_revision_id: Uuid,
    pub work_list_id: Uuid,
    pub revision: i64,
    pub payload_ciphertext: SealedBlob,
    pub is_repair: bool,
    pub created_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
    pub quarantined_at: Option<DateTime<Utc>>,
    pub quarantined_by_membership_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReferenceSchemeListResponse {
    pub schemes: Vec<TaskReferenceSchemeResponse>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReferenceSchemeMutationRequest {
    pub scheme_revision_id: Uuid,
    pub expected_scheme_revision: i64,
    pub payload_ciphertext: SealedBlob,
    pub payload_ciphertext_proof: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

impl std::fmt::Debug for TaskReferenceSchemeMutationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaskReferenceSchemeMutationRequest")
            .field("scheme_revision_id", &self.scheme_revision_id)
            .field("expected_scheme_revision", &self.expected_scheme_revision)
            .field("payload_ciphertext", &"<redacted>")
            .field("payload_ciphertext_proof", &"<redacted>")
            .field(
                "audit_patch",
                &self.audit_patch.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReferenceSchemeQuarantineRequest {
    pub expected_scheme_revision: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatsResponse {
    pub tasks_overdue: i64,
    pub tasks_due_today: i64,
    pub tasks_due_this_week: i64,
    pub completed: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title_ciphertext: String,
    pub title_ciphertext_proof: String,
    pub payload_ciphertext: String,
    pub payload_ciphertext_proof: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_commitment: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_ciphertext_proof: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_ciphertext_proof: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Option<i8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<Option<DateTime<Utc>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<Option<DateTime<Utc>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_id: Option<Option<Uuid>>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTaskRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_task_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_boundary: Option<TaskSectionBoundary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskSectionBoundary {
    First,
    Last,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveWorkListRequest {}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnarchiveWorkListRequest {}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveTaskRequest {}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnarchiveTaskRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditPatchFieldRequest {
    pub field: String,
    pub change_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_scalar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_scalar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_ciphertext_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_ciphertext_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditPatchRequest {
    #[serde(default)]
    pub fields: Vec<AuditPatchFieldRequest>,
    pub payload_ciphertext: String,
    pub payload_ciphertext_proof: String,
    pub payload_version: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteTaskRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommentRequest {
    pub body_ciphertext: String,
    pub body_ciphertext_proof: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCommentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_ciphertext_proof: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteCommentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_patch: Option<AuditPatchRequest>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{MAX_RETRY_AFTER_SECONDS, map_api_error, parse_retry_after};
    use flate2::{Compression, write::GzEncoder};
    use sealtask_client_auth::Credentials;
    use sealtask_client_core::{ResponseFailureKind, TransportFailureKind};
    use std::io::Write as _;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Clone, Copy)]
    enum NoteMutationBodyFailure {
        Oversized,
        Truncated,
    }

    impl NoteMutationBodyFailure {
        const fn label(self) -> &'static str {
            match self {
                Self::Oversized => "oversized",
                Self::Truncated => "truncated",
            }
        }

        const fn expected_kind(self) -> ResponseFailureKind {
            match self {
                Self::Oversized => ResponseFailureKind::BodyTooLarge,
                Self::Truncated => ResponseFailureKind::BodyTruncated,
            }
        }
    }

    fn test_credentials(api_url: &str) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + chrono::Duration::hours(1),
            refresh_expires_at: Utc::now() + chrono::Duration::hours(2),
            user_id: Uuid::now_v7(),
            email: "agent@example.com".to_string(),
            data_key_ciphertext: "unused".to_string(),
        }
    }

    fn gzip_bytes(body: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(body).expect("compress test response");
        encoder.finish().expect("finish compressed response")
    }

    async fn serve_single_response(
        status: &'static str,
        content_encoding: Option<&'static str>,
        body: Vec<u8>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        serve_single_response_with_headers(status, content_encoding, "", body).await
    }

    async fn serve_single_response_with_headers(
        status: &'static str,
        content_encoding: Option<&'static str>,
        extra_headers: &'static str,
        body: Vec<u8>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).await.expect("request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let encoding_header = content_encoding
                .map(|encoding| format!("Content-Encoding: {encoding}\r\n"))
                .unwrap_or_default();
            let headers = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{encoding_header}{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).await.expect("headers");
            stream.write_all(&body).await.expect("body");
        });
        (api_url, server)
    }

    async fn serve_confirmed_note_mutation_body_failure(
        failure: NoteMutationBodyFailure,
    ) -> (String, tokio::task::JoinHandle<()>) {
        serve_bounded_body_failure("201 Created", "", failure, MAX_NOTE_DECOMPRESSED_PAGE_BYTES)
            .await
    }

    async fn serve_bounded_body_failure(
        status: &'static str,
        extra_headers: &'static str,
        failure: NoteMutationBodyFailure,
        max_decompressed_bytes: usize,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).await.expect("request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let (declared_len, body) = match failure {
                NoteMutationBodyFailure::Oversized => {
                    let body = vec![b'a'; max_decompressed_bytes + 1];
                    (body.len(), body)
                }
                NoteMutationBodyFailure::Truncated => (128, b"{".to_vec()),
            };
            let headers = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{extra_headers}Content-Length: {declared_len}\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(headers.as_bytes()).await.expect("headers");
            let _ = stream.write_all(&body).await;
        });
        (api_url, server)
    }

    fn assert_status_first_error(error: &PublicError, status: u16) {
        let expected_code = match status {
            400 => "validation",
            408 => "request_timeout",
            409 => "conflict",
            429 => "rate_limited",
            _ => panic!("unexpected status in status-first test: {status}"),
        };
        assert_eq!(error.code(), expected_code, "HTTP {status}");
        assert_eq!(error.http_status(), Some(status), "HTTP {status}");
        assert_eq!(
            error.retry_after(),
            (status == 429).then_some(Duration::from_secs(7)),
            "HTTP {status}"
        );
        assert_eq!(
            error.response_failure_kind(),
            None,
            "HTTP {status} must win over local body-read classification"
        );
    }

    #[test]
    fn test_should_map_json_conflicts_to_stable_public_error() {
        let error = map_api_error(
            409,
            r#"{"error":"conflict","message":"task changed"}"#,
            "/tasks/1",
        );

        assert_eq!(error.http_status(), Some(409));
        assert_eq!(error.backend_error_code(), Some("conflict"));
        assert_eq!(error.code(), "conflict");
        assert_eq!(
            error.to_string(),
            "request conflicted with current server state"
        );
        assert!(!format!("{error:?}").contains("task changed"));
    }

    #[test]
    fn backend_message_copy_does_not_change_http_classification() {
        let first = map_api_error(
            409,
            r#"{"error":"conflict","message":"first private copy"}"#,
            "/private/first",
        );
        let rewritten = map_api_error(
            409,
            r#"{"error":"conflict","message":"completely rewritten copy"}"#,
            "/private/second",
        );

        assert_eq!(first.http_status(), rewritten.http_status());
        assert_eq!(first.backend_error_code(), rewritten.backend_error_code());
        assert_eq!(first.code(), rewritten.code());
        for secret in [
            "first private copy",
            "completely rewritten copy",
            "/private/first",
            "/private/second",
        ] {
            assert!(!first.to_string().contains(secret));
            assert!(!format!("{first:?}").contains(secret));
            assert!(!rewritten.to_string().contains(secret));
            assert!(!format!("{rewritten:?}").contains(secret));
        }
    }

    #[test]
    fn test_should_map_non_json_conflicts_to_stable_public_error() {
        let error = map_api_error(409, "revision mismatch", "/tasks/1");

        assert_eq!(error.http_status(), Some(409));
        assert_eq!(error.backend_error_code(), None);
        assert_eq!(error.code(), "conflict");
    }

    #[test]
    fn test_should_map_plain_bad_request_and_unprocessable_entity_to_validation() {
        for status in [400, 422] {
            let error = map_api_error(status, "request schema rejected", "/work-lists/1/notes");
            assert_eq!(error.http_status(), Some(status));
            assert_eq!(error.code(), "validation");
            assert!(!error.to_string().contains("request schema rejected"));
        }
    }

    #[tokio::test]
    async fn bounded_note_transport_maps_plain_400_and_422_to_validation() {
        for (status, status_line) in [(400, "400 Bad Request"), (422, "422 Unprocessable Entity")] {
            let (api_url, server) =
                serve_single_response(status_line, None, b"plain schema rejection".to_vec()).await;
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let request = CreateNoteRequest {
                idempotency_key: "note-validation".to_string(),
                idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
                title_ciphertext: "title".to_string(),
                title_ciphertext_proof: "title-proof".to_string(),
                payload_ciphertext: "payload".to_string(),
                payload_ciphertext_proof: "payload-proof".to_string(),
                is_private: false,
                note_key_ciphertext: None,
                audit_patch: None,
            };

            let encoded =
                note_transport::EncodedNoteRequest::encode(&request).expect("encode note request");
            let error = client
                .create_note_encoded(Uuid::now_v7(), encoded)
                .await
                .expect("receive bounded validation response")
                .decode()
                .expect_err("plain validation response must fail");

            assert_eq!(error.http_status(), Some(status));
            assert_eq!(error.code(), "validation");
            assert!(!error.to_string().contains("plain schema rejection"));
            server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn bounded_note_mutation_preserves_known_201_across_body_failures() {
        for failure in [
            NoteMutationBodyFailure::Oversized,
            NoteMutationBodyFailure::Truncated,
        ] {
            let (api_url, server) = serve_confirmed_note_mutation_body_failure(failure).await;
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let request = CreateNoteRequest {
                idempotency_key: "note-confirmed".to_string(),
                idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
                title_ciphertext: "title".to_string(),
                title_ciphertext_proof: "title-proof".to_string(),
                payload_ciphertext: "payload".to_string(),
                payload_ciphertext_proof: "payload-proof".to_string(),
                is_private: false,
                note_key_ciphertext: None,
                audit_patch: None,
            };
            let encoded =
                note_transport::EncodedNoteRequest::encode(&request).expect("encode note request");

            let response = client
                .create_note_encoded(Uuid::now_v7(), encoded)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "known 201 status must survive the {} body failure: {error}",
                        failure.label()
                    )
                });
            assert!(
                response.is_success_status(),
                "known 201 must remain available after the {} body failure",
                failure.label()
            );
            let error = response
                .decode()
                .expect_err("the incomplete response body must not decode");
            assert_eq!(
                error.response_failure_kind(),
                Some(failure.expected_kind()),
                "{} response-body failure must remain a local processing error",
                failure.label()
            );
            assert!(
                !error.to_string().contains(&api_url),
                "response-body failures must not expose the request origin"
            );
            server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn bounded_note_error_statuses_survive_truncated_and_oversized_bodies() {
        let statuses = [
            (400, "400 Bad Request"),
            (408, "408 Request Timeout"),
            (409, "409 Conflict"),
            (429, "429 Too Many Requests"),
        ];
        for failure in [
            NoteMutationBodyFailure::Oversized,
            NoteMutationBodyFailure::Truncated,
        ] {
            for (status, status_line) in statuses {
                let (api_url, server) = serve_bounded_body_failure(
                    status_line,
                    "Retry-After: 7\r\n",
                    failure,
                    MAX_NOTE_DECOMPRESSED_PAGE_BYTES,
                )
                .await;
                let mut client =
                    PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                        .expect("API client");

                let error = client
                    .get_note_encoded(Uuid::now_v7(), Uuid::now_v7())
                    .await
                    .expect("receive status-bearing note response")
                    .decode()
                    .expect_err("non-success note status must fail");

                assert_status_first_error(&error, status);
                assert!(!format!("{error:?}").contains(&api_url));
                server.await.expect("server");
            }
        }
    }

    #[tokio::test]
    async fn bounded_attachment_error_statuses_survive_truncated_and_oversized_bodies() {
        let statuses = [
            (400, "400 Bad Request"),
            (408, "408 Request Timeout"),
            (409, "409 Conflict"),
            (429, "429 Too Many Requests"),
        ];
        for failure in [
            NoteMutationBodyFailure::Oversized,
            NoteMutationBodyFailure::Truncated,
        ] {
            for (status, status_line) in statuses {
                let (api_url, server) = serve_bounded_body_failure(
                    status_line,
                    "Retry-After: 7\r\n",
                    failure,
                    MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES,
                )
                .await;
                let mut client =
                    PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                        .expect("API client");
                let error = client
                    .get_attachment_download(Uuid::now_v7(), Uuid::now_v7())
                    .await
                    .expect_err("non-success attachment JSON status must fail");
                assert_status_first_error(&error, status);
                assert!(!format!("{error:?}").contains(&api_url));
                server.await.expect("server");

                let (api_url, server) = serve_bounded_body_failure(
                    status_line,
                    "Retry-After: 7\r\n",
                    failure,
                    MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES,
                )
                .await;
                let mut client =
                    PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                        .expect("API client");
                let error = client
                    .delete_attachment(Uuid::now_v7(), Uuid::now_v7())
                    .await
                    .expect_err("non-success attachment no-content status must fail");
                assert_status_first_error(&error, status);
                assert!(!format!("{error:?}").contains(&api_url));
                server.await.expect("server");
            }
        }
    }

    #[test]
    fn test_should_preserve_not_found_as_a_typed_public_error() {
        let json = map_api_error(
            404,
            r#"{"error":"not_found","message":"note not found"}"#,
            "/notes/1",
        );
        assert_eq!(json.http_status(), Some(404));
        assert_eq!(json.backend_error_code(), Some("not_found"));
        assert_eq!(json.code(), "not_found");

        let plain = map_api_error(404, "missing", "/notes/1");
        assert_eq!(plain.http_status(), Some(404));
        assert_eq!(plain.backend_error_code(), None);
        assert_eq!(plain.code(), "not_found");
    }

    #[test]
    fn test_should_map_capacity_statuses_to_stable_public_errors() {
        let cases = [
            (
                402,
                "payment required",
                "entitlement",
                "entitlement",
                r#"{"error":"entitlement","message":"upgrade required"}"#,
            ),
            (
                413,
                "request payload is too large",
                "payload_too_large",
                "payload_too_large",
                r#"{"error":"payload_too_large","message":"note is too large"}"#,
            ),
            (
                408,
                "request timed out before completion",
                "request_timeout",
                "request_timeout",
                r#"{"error":"request_timeout","message":"request body timed out before mutation execution"}"#,
            ),
            (
                429,
                "API rate limit exceeded",
                "rate_limited",
                "rate_limited",
                r#"{"error":"rate_limited","message":"try again later"}"#,
            ),
        ];

        for (status, expected_message, expected_code, backend_code, body) in cases {
            let json = map_api_error(status, body, "/work-lists/1/notes");
            assert_eq!(json.code(), expected_code);
            assert_eq!(json.http_status(), Some(status));
            assert_eq!(json.backend_error_code(), Some(backend_code));
            assert_eq!(json.to_string(), expected_message);

            let plain = map_api_error(status, "private response body", "/work-lists/1/notes");
            assert_eq!(plain.code(), expected_code);
            assert_eq!(plain.http_status(), Some(status));
            assert_eq!(plain.backend_error_code(), None);
            assert!(!plain.to_string().contains("private response body"));
        }
    }

    #[test]
    fn retry_after_delta_seconds_are_parsed_safely_and_capped() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("60"),
        );
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(60)));

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("not-a-delay"),
        );
        assert_eq!(parse_retry_after(&headers), None);

        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_static("999999999999999999999999999999999999999999"),
        );
        assert_eq!(
            parse_retry_after(&headers),
            Some(Duration::from_secs(MAX_RETRY_AFTER_SECONDS))
        );
    }

    #[tokio::test]
    async fn bounded_note_429_preserves_typed_retry_after_without_exposing_the_body() {
        let body_canary = "rate-limit-response-body-canary";
        let body = format!(
            r#"{{"error":"rate_limited","message":"retry later","private":"{body_canary}"}}"#
        )
        .into_bytes();
        let (api_url, server) = serve_single_response_with_headers(
            "429 Too Many Requests",
            None,
            "Retry-After: 1\r\n",
            body,
        )
        .await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let request = CreateNoteRequest {
            idempotency_key: "note-retry-after".to_string(),
            idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            title_ciphertext: "title".to_string(),
            title_ciphertext_proof: "title-proof".to_string(),
            payload_ciphertext: "payload".to_string(),
            payload_ciphertext_proof: "payload-proof".to_string(),
            is_private: false,
            note_key_ciphertext: None,
            audit_patch: None,
        };
        let encoded =
            note_transport::EncodedNoteRequest::encode(&request).expect("encode note request");

        let error = client
            .create_note_encoded(Uuid::now_v7(), encoded)
            .await
            .expect("receive bounded rate-limit response")
            .decode()
            .expect_err("429 must fail");

        assert_eq!(error.code(), "rate_limited");
        assert_eq!(error.retry_after(), Some(Duration::from_secs(1)));
        assert_eq!(error.http_status(), Some(429));
        assert_eq!(error.backend_error_code(), Some("rate_limited"));
        assert_eq!(error.to_string(), "API rate limit exceeded");
        assert!(!format!("{error:?}").contains(body_canary));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn bounded_note_and_general_json_errors_share_redacted_typed_classification() {
        let malformed_canary = "https://example.invalid/?token=malformed-secret";
        let malformed = format!(r#"{{"ciphertext":"{malformed_canary}""#).into_bytes();
        let (api_url, server) = serve_single_response("200 OK", None, malformed).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .get_note_encoded(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect("receive bounded malformed note response")
            .decode()
            .expect_err("malformed note JSON must fail");
        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::JsonMalformed)
        );
        assert!(!format!("{error:?}").contains(malformed_canary));
        assert!(!error.to_string().contains(&api_url));
        server.await.expect("server");

        let schema_canary = "ciphertext-schema-secret";
        let schema = format!(
            r#"{{"downloadUrl":"https://example.invalid/?token={schema_canary}","downloadHeaders":{{"authorization":"{schema_canary}"}},"expiresAt":false}}"#
        )
        .into_bytes();
        let (api_url, server) = serve_single_response("200 OK", None, schema).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .get_attachment_download(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect_err("schema-invalid attachment response must fail");
        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::JsonSchema)
        );
        assert!(!format!("{error:?}").contains(schema_canary));
        assert!(!error.to_string().contains(&api_url));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn bounded_request_transport_failures_are_typed_and_redacted() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        drop(listener);
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");

        let error = client
            .get_attachment_download(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect_err("closed listener must fail");

        assert_eq!(
            error.transport_failure_kind(),
            Some(TransportFailureKind::Connect)
        );
        assert!(!format!("{error:?}").contains(&api_url));
    }

    #[tokio::test]
    async fn bounded_note_delete_maps_structured_and_plain_408_as_definitive_request_timeout() {
        let cases = [
            (
                br#"{"error":"request_timeout","message":"body deadline elapsed; retry the request"}"#
                    .to_vec(),
                Some("request_timeout"),
            ),
            (
                b"request timed out before the mutation was admitted".to_vec(),
                None,
            ),
        ];

        for (body, expected_backend_code) in cases {
            let (api_url, server) = serve_single_response("408 Request Timeout", None, body).await;
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let encoded = note_transport::EncodedNoteRequest::encode(&DeleteNoteRequest::default())
                .expect("encode delete request");
            let error = client
                .delete_note_encoded(Uuid::now_v7(), Uuid::now_v7(), encoded)
                .await
                .expect("receive bounded timeout response")
                .decode()
                .expect_err("HTTP 408 must fail definitively");

            assert_eq!(error.code(), "request_timeout");
            assert_eq!(error.http_status(), Some(408));
            assert_eq!(error.backend_error_code(), expected_backend_code);
            assert_eq!(error.to_string(), "request timed out before completion");
            server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn bounded_note_response_limits_decompressed_bytes() {
        const NOTE_PAGE_LIMIT_BYTES: usize = 8 * 1024 * 1024;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder
            .write_all(&vec![b'a'; NOTE_PAGE_LIMIT_BYTES + 1])
            .expect("compress oversized response");
        let compressed = encoder.finish().expect("finish compressed response");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).await.expect("request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                compressed.len()
            );
            stream.write_all(headers.as_bytes()).await.expect("headers");
            stream.write_all(&compressed).await.expect("body");
        });
        let credentials = Credentials {
            api_url: api_url.clone(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + chrono::Duration::hours(1),
            refresh_expires_at: Utc::now() + chrono::Duration::hours(2),
            user_id: Uuid::now_v7(),
            email: "agent@example.com".to_string(),
            data_key_ciphertext: "unused".to_string(),
        };
        let mut client =
            PublicApiClient::with_credentials(&api_url, credentials).expect("API client");

        let error = client
            .list_notes_page_encoded(Uuid::now_v7(), None, MAX_NOTE_PAGE_ITEMS)
            .await
            .expect_err("decompressed response must be bounded");

        assert!(error.to_string().contains("8388608-byte limit"));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn bounded_note_raw_response_accepts_the_exact_decompressed_limit() {
        let body = vec![b' '; MAX_NOTE_DECOMPRESSED_PAGE_BYTES];
        let (api_url, server) = serve_single_response("200 OK", None, body).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");

        let response = client
            .list_notes_page_encoded(Uuid::now_v7(), None, MAX_NOTE_PAGE_ITEMS)
            .await
            .expect("exact-size raw note response");

        assert_eq!(response.encoded_len(), MAX_NOTE_DECOMPRESSED_PAGE_BYTES);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn bounded_single_note_success_and_error_paths_limit_decompressed_bytes() {
        let compressed = gzip_bytes(&vec![b'a'; MAX_NOTE_DECOMPRESSED_PAGE_BYTES + 1]);

        let (api_url, server) =
            serve_single_response("200 OK", Some("gzip"), compressed.clone()).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .get_note_encoded(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect_err("successful single-note response must enforce its decompressed limit");
        assert!(error.to_string().contains("exceeds the 8388608-byte limit"));
        assert!(!error.to_string().contains(&api_url));
        server.await.expect("server");

        let (api_url, server) =
            serve_single_response("500 Internal Server Error", Some("gzip"), compressed).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .get_note_encoded(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect("non-success status must remain available despite the oversized body")
            .decode()
            .expect_err("HTTP 500 must fail");
        assert_eq!(error.http_status(), Some(500));
        assert_eq!(
            error.to_string(),
            "API server could not complete the request"
        );
        assert!(!error.to_string().contains(&api_url));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn bounded_attachment_json_and_no_content_paths_reject_oversized_bodies() {
        let oversized = vec![b'a'; MAX_ATTACHMENT_CONTROL_PLANE_RESPONSE_BYTES + 1];

        let (api_url, server) = serve_single_response("200 OK", None, oversized.clone()).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .get_attachment_download(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect_err("attachment JSON success body must be bounded");
        assert!(error.to_string().contains("exceeds the 65536-byte limit"));
        server.await.expect("server");

        let (api_url, server) = serve_single_response("200 OK", None, oversized.clone()).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .complete_attachment_upload(
                Uuid::now_v7(),
                Uuid::now_v7(),
                &CompleteAttachmentUploadRequest {
                    ciphertext_bytes: 42,
                },
            )
            .await
            .expect_err("no-content success body must be bounded");
        assert!(error.to_string().contains("exceeds the 65536-byte limit"));
        server.await.expect("server");

        let compressed = gzip_bytes(&oversized);
        let (api_url, server) =
            serve_single_response("500 Internal Server Error", Some("gzip"), compressed).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let error = client
            .delete_attachment(Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect_err("no-content error status must survive an oversized body");
        assert_eq!(error.http_status(), Some(500));
        assert_eq!(
            error.to_string(),
            "API server could not complete the request"
        );
        assert!(
            !error.to_string().contains(&api_url),
            "bounded response failures must not expose the request URL"
        );
        server.await.expect("server");
    }
}
