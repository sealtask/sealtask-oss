use super::{RuntimeClient, WorkListContext};
use crate::inputs::{
    ArchiveTaskArgs, CreateTaskArgs, DeleteTaskArgs, MoveTaskArgs, TaskCompletionArgs,
    TaskCreateIdempotencyDerivation, TaskCreateInput, TaskFieldPatch, TaskUpdateInput,
    UnarchiveTaskArgs, UpdateTaskArgs, normalize_checklist, validate_idempotency_key,
    validate_priority,
};
use crate::models::{AgentTaskDetail, AgentTaskSummary};
use crate::read_cache::ReadCacheQuery;
use chrono::{DateTime, Utc};
use sealtask_client_api::{
    ArchiveTaskRequest, BoardEventStream, CreateTaskRequest, MoveTaskRequest, MyTaskResponse,
    PublicApiClient, TaskDetailResponse, TaskListResponse, TaskReferenceSchemeResponse,
    TaskSectionBoundary, UnarchiveTaskRequest, UpdateTaskRequest, WorkListResponse,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    ChecklistItemPayload, TASK_REFERENCE_REVISION_MAX, TASK_TITLE_CONTEXT, TaskPayloadBody,
    build_task_payload_envelope, compute_payload_proof, compute_task_create_semantic_commitment,
    decode_sealed_blob, decrypt_task_payload, derive_child_key, derive_payload_binding_key,
    encrypt_task_payload, encrypt_text_value, parse_task_reference, plaintext_rich_text,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const TASK_MUTATION_PLAN_SCHEMA_VERSION: u8 = 1;
const TASK_MUTATION_PLAN_TYPE: &str = "taskMutationPlan";
const TASK_CREATE_ACTION: &str = "task.create";
const TASK_UPDATE_ACTION: &str = "task.update";

/// A secret-safe projection of a fully prepared task mutation.
///
/// Plans deliberately contain no readable task content, durable idempotency
/// key, encryption key, ciphertext, or proof. `will_mutate` is always false:
/// inspecting or serializing a plan never performs the prepared request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMutationPlan {
    pub schema_version: u8,
    #[serde(rename = "type")]
    pub plan_type: &'static str,
    pub action: &'static str,
    pub project_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    pub section_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub changed_fields: Vec<String>,
    pub changed_field_count: usize,
    pub change_commitment: String,
    pub idempotency_protected: bool,
    pub would_change: bool,
    pub will_mutate: bool,
}

/// A task create that has already been resolved, normalized, and encrypted.
///
/// This type is intentionally opaque and does not implement `Debug` or
/// `Serialize` because it retains the exact encrypted request and unlocked
/// project context until consumed by execution or dropped.
pub struct PreparedTaskCreate {
    client: PublicApiClient,
    context: SensitiveWorkListContext,
    work_list_id: Uuid,
    request: SensitiveCreateTaskRequest,
    plan: TaskMutationPlan,
}

impl PreparedTaskCreate {
    /// Return the only safe, serializable projection of this prepared request.
    #[must_use]
    pub const fn plan(&self) -> &TaskMutationPlan {
        &self.plan
    }
}

/// A task update that has already fetched its base revision, normalized its
/// effective changes, and encrypted the exact request when a change is needed.
///
/// This type is intentionally opaque and does not implement `Debug` or
/// `Serialize`.
pub struct PreparedTaskUpdate {
    client: PublicApiClient,
    context: SensitiveWorkListContext,
    work_list_id: Uuid,
    task_id: Uuid,
    request: SensitiveUpdateTaskRequest,
    current: SensitiveTaskResponse,
    plan: TaskMutationPlan,
}

impl PreparedTaskUpdate {
    /// Return the only safe, serializable projection of this prepared request.
    #[must_use]
    pub const fn plan(&self) -> &TaskMutationPlan {
        &self.plan
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskCreateSemanticPlan<'a> {
    title: &'a str,
    body: Option<&'a str>,
    checklist: Option<&'a [ChecklistItemPayload]>,
    priority: Option<i8>,
    due_at: Option<&'a DateTime<Utc>>,
    start_at: Option<&'a DateTime<Utc>>,
    section_id: Option<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskUpdateSemanticPlan<'a> {
    project_id: Uuid,
    task_id: Uuid,
    expected_updated_at: &'a DateTime<Utc>,
    changed_fields: &'a [String],
    title: Option<&'a str>,
    body: Option<&'a sealtask_client_crypto::TaskPayloadRichText>,
    checklist: Option<&'a [ChecklistItemPayload]>,
    priority: Option<i8>,
    due_at: Option<&'a DateTime<Utc>>,
    start_at: Option<&'a DateTime<Utc>>,
    section_id: Option<Uuid>,
}

struct SensitiveTaskCreateInput(crate::inputs::TaskCreateInput);

impl Drop for SensitiveTaskCreateInput {
    fn drop(&mut self) {
        self.0.title.zeroize();
        if let Some(body) = self.0.body.as_mut() {
            body.zeroize();
        }
        if let Some(checklist) = self.0.checklist.as_mut() {
            zeroize_checklist(checklist);
        }
        if let Some(idempotency_key) = self.0.idempotency_key.as_mut() {
            idempotency_key.zeroize();
        }
    }
}

struct SensitiveTaskUpdateInput(TaskUpdateInput);

impl Drop for SensitiveTaskUpdateInput {
    fn drop(&mut self) {
        if let Some(title) = self.0.title.as_mut() {
            title.zeroize();
        }
        if let TaskFieldPatch::Set(body) = &mut self.0.body {
            body.zeroize();
        }
        if let TaskFieldPatch::Set(checklist) = &mut self.0.checklist {
            zeroize_checklist(checklist);
        }
    }
}

struct SensitiveTaskPayloadBody(TaskPayloadBody);

impl Drop for SensitiveTaskPayloadBody {
    fn drop(&mut self) {
        zeroize_task_payload_body(&mut self.0);
    }
}

struct SensitiveWorkListContext(Option<WorkListContext>);

impl SensitiveWorkListContext {
    fn get(&self) -> PublicResult<&WorkListContext> {
        self.0
            .as_ref()
            .ok_or_else(|| PublicError::unexpected("prepared project context was already consumed"))
    }
}

impl Drop for SensitiveWorkListContext {
    fn drop(&mut self) {
        if let Some(context) = self.0.as_mut() {
            zeroize_work_list_context(context);
        }
    }
}

struct SensitiveTaskResponse(Option<sealtask_client_api::TaskResponse>);

impl SensitiveTaskResponse {
    fn get(&self) -> PublicResult<&sealtask_client_api::TaskResponse> {
        self.0
            .as_ref()
            .ok_or_else(|| PublicError::unexpected("prepared current task was already consumed"))
    }

    fn take(&mut self) -> sealtask_client_api::TaskResponse {
        self.0
            .take()
            .expect("sensitive current task is consumed exactly once")
    }
}

impl Drop for SensitiveTaskResponse {
    fn drop(&mut self) {
        if let Some(task) = self.0.as_mut() {
            zeroize_task_response(task);
        }
    }
}

struct SensitiveTaskPayloadEnvelope(sealtask_client_crypto::TaskPayloadEnvelope);

impl Drop for SensitiveTaskPayloadEnvelope {
    fn drop(&mut self) {
        zeroize_task_payload_body(&mut self.0.body);
    }
}

struct SensitiveSealedBlob(sealtask_client_crypto::SealedBlobPayload);

impl Drop for SensitiveSealedBlob {
    fn drop(&mut self) {
        self.0.bytes.zeroize();
        self.0.base64.zeroize();
    }
}

struct SensitiveOptionalString(Option<String>);

impl SensitiveOptionalString {
    fn is_some(&self) -> bool {
        self.0.is_some()
    }

    fn take(&mut self) -> Option<String> {
        self.0.take()
    }

    fn zeroize(&mut self) {
        if let Some(value) = self.0.as_mut() {
            value.zeroize();
        }
    }
}

impl Drop for SensitiveOptionalString {
    fn drop(&mut self) {
        self.zeroize();
    }
}

struct SensitiveCreateTaskRequest(Option<CreateTaskRequest>);

impl SensitiveCreateTaskRequest {
    fn get(&self) -> &CreateTaskRequest {
        self.0
            .as_ref()
            .expect("sensitive create-task request is present until preparation completes")
    }

    fn get_mut(&mut self) -> &mut CreateTaskRequest {
        self.0
            .as_mut()
            .expect("sensitive create-task request is present until preparation completes")
    }

    fn zeroize(&mut self) {
        if let Some(request) = self.0.as_mut() {
            zeroize_create_task_request(request);
        }
    }
}

impl Drop for SensitiveCreateTaskRequest {
    fn drop(&mut self) {
        self.zeroize();
    }
}

struct SensitiveUpdateTaskRequest(Option<UpdateTaskRequest>);

impl SensitiveUpdateTaskRequest {
    fn get(&self) -> &UpdateTaskRequest {
        self.0
            .as_ref()
            .expect("sensitive update-task request is present until preparation completes")
    }

    fn get_mut(&mut self) -> &mut UpdateTaskRequest {
        self.0
            .as_mut()
            .expect("sensitive update-task request is present until preparation completes")
    }

    fn zeroize(&mut self) {
        if let Some(request) = self.0.as_mut() {
            zeroize_update_task_request(request);
        }
    }
}

impl Drop for SensitiveUpdateTaskRequest {
    fn drop(&mut self) {
        self.zeroize();
    }
}

struct SensitiveChecklist(Option<Vec<ChecklistItemPayload>>);

impl Drop for SensitiveChecklist {
    fn drop(&mut self) {
        if let Some(value) = self.0.as_mut() {
            zeroize_checklist(value);
        }
    }
}

struct SensitiveRichTextPatch(Option<Option<sealtask_client_crypto::TaskPayloadRichText>>);

impl Drop for SensitiveRichTextPatch {
    fn drop(&mut self) {
        if let Some(Some(value)) = self.0.as_mut() {
            zeroize_rich_text(value);
        }
    }
}

struct SensitiveChecklistPatch(Option<Option<Vec<ChecklistItemPayload>>>);

impl Drop for SensitiveChecklistPatch {
    fn drop(&mut self) {
        if let Some(Some(value)) = self.0.as_mut() {
            zeroize_checklist(value);
        }
    }
}

fn normalized_optional_title(title: Option<&str>) -> PublicResult<Option<&str>> {
    let Some(title) = title else {
        return Ok(None);
    };
    let title = title.trim();
    if title.is_empty() {
        return Err(PublicError::validation("title cannot be empty"));
    }
    Ok(Some(title))
}

fn effective_patch<T: Clone + PartialEq>(
    patch: &TaskFieldPatch<T>,
    current: &Option<T>,
) -> Option<Option<T>> {
    match patch {
        TaskFieldPatch::Unchanged => None,
        TaskFieldPatch::Set(value) if current.as_ref() == Some(value) => None,
        TaskFieldPatch::Set(value) => Some(Some(value.clone())),
        TaskFieldPatch::Clear if current.is_none() => None,
        TaskFieldPatch::Clear => Some(None),
    }
}

fn patched_value<T: Clone>(patch: &TaskFieldPatch<T>, current: &Option<T>) -> Option<T> {
    match patch {
        TaskFieldPatch::Unchanged => current.clone(),
        TaskFieldPatch::Set(value) => Some(value.clone()),
        TaskFieldPatch::Clear => None,
    }
}

fn sensitive_serialized_eq<T: Serialize>(left: &T, right: &T) -> PublicResult<bool> {
    let left = Zeroizing::new(serde_json::to_vec(left).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to encode normalized task field for comparison: {err}"
        ))
    })?);
    let right = Zeroizing::new(serde_json::to_vec(right).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to encode current task field for comparison: {err}"
        ))
    })?);
    Ok(left.as_slice() == right.as_slice())
}

fn task_create_idempotency_key(
    input: &TaskCreateInput,
    derivation: Option<&TaskCreateIdempotencyDerivation>,
    work_list_id: &Uuid,
    list_key: &sealtask_client_crypto::SymmetricKey,
) -> PublicResult<Option<String>> {
    if let Some(explicit) = input.idempotency_key.as_deref() {
        return validate_idempotency_key(explicit).map(Some);
    }
    derivation
        .map(|derivation| derivation.derive_key(work_list_id, list_key))
        .transpose()
}

fn task_mutation_change_commitment(
    action: &str,
    canonical_semantics: &[u8],
    list_key: &sealtask_client_crypto::SymmetricKey,
) -> PublicResult<String> {
    let commitment_key = derive_child_key(
        list_key,
        "sealtask.task-mutation-plan.change-commitment.key.v1",
    )?;
    let mut committed_bytes = Zeroizing::new(Vec::with_capacity(
        action.len() + canonical_semantics.len() + 1,
    ));
    committed_bytes.extend_from_slice(action.as_bytes());
    committed_bytes.push(0);
    committed_bytes.extend_from_slice(canonical_semantics);
    compute_payload_proof(&committed_bytes, &commitment_key)
}

fn zeroize_create_task_request(request: &mut CreateTaskRequest) {
    request.title_ciphertext.zeroize();
    request.title_ciphertext_proof.zeroize();
    request.payload_ciphertext.zeroize();
    request.payload_ciphertext_proof.zeroize();
    if let Some(idempotency_key) = request.idempotency_key.as_mut() {
        idempotency_key.zeroize();
    }
    if let Some(idempotency_commitment) = request.idempotency_commitment.as_mut() {
        idempotency_commitment.zeroize();
    }
}

fn zeroize_update_task_request(request: &mut UpdateTaskRequest) {
    if let Some(value) = request.title_ciphertext.as_mut() {
        value.zeroize();
    }
    if let Some(value) = request.title_ciphertext_proof.as_mut() {
        value.zeroize();
    }
    if let Some(value) = request.payload_ciphertext.as_mut() {
        value.zeroize();
    }
    if let Some(value) = request.payload_ciphertext_proof.as_mut() {
        value.zeroize();
    }
}

fn zeroize_task_response(task: &mut sealtask_client_api::TaskResponse) {
    task.title_ciphertext.zeroize();
    task.payload_ciphertext.zeroize();
    for delegation in &mut task.delegations {
        if let Some(note_ciphertext) = delegation.note_ciphertext.as_mut() {
            note_ciphertext.zeroize();
        }
    }
}

fn zeroize_task_comments(comments: &mut [sealtask_client_api::CommentResponse]) {
    for comment in comments {
        comment.body_ciphertext.zeroize();
    }
}

fn zeroize_work_list_context(context: &mut WorkListContext) {
    if let Some(title) = context.work_list_title.as_mut() {
        title.zeroize();
    }
}

fn zeroize_task_payload_body(body: &mut TaskPayloadBody) {
    body.title.zeroize();
    if let Some(rich_text) = body.rich_text.as_mut() {
        zeroize_rich_text(rich_text);
    }
    if let Some(checklist) = body.checklist.as_mut() {
        zeroize_checklist(checklist);
    }
    if let Some(attachments) = body.attachments.as_mut() {
        attachments.iter_mut().for_each(zeroize_flexible_value);
    }
    if let Some(references) = body.references.as_mut() {
        references.iter_mut().for_each(zeroize_flexible_value);
    }
    if let Some(mentions) = body.mentions.as_mut() {
        mentions.iter_mut().for_each(Zeroize::zeroize);
    }
    if let Some(client_meta) = body.client_meta.as_mut() {
        zeroize_flexible_value(client_meta);
    }
    if let Some(recurrence_state) = body.recurrence_state.as_mut() {
        zeroize_flexible_value(recurrence_state);
    }
}

fn zeroize_rich_text(rich_text: &mut sealtask_client_crypto::TaskPayloadRichText) {
    rich_text.format.zeroize();
    for block in &mut rich_text.blocks {
        block.block_type.zeroize();
        block.text.zeroize();
    }
}

fn zeroize_checklist(checklist: &mut [ChecklistItemPayload]) {
    for item in checklist {
        item.id.zeroize();
        item.title.zeroize();
        if let Some(assignees) = item.assignee_user_ids.as_mut() {
            assignees.iter_mut().for_each(Zeroize::zeroize);
        }
    }
}

fn zeroize_flexible_value(value: &mut sealtask_client_crypto::FlexibleValue) {
    use sealtask_client_crypto::FlexibleValue;

    match value {
        FlexibleValue::Bytes(bytes) => bytes.zeroize(),
        FlexibleValue::Text(text) => text.zeroize(),
        FlexibleValue::Tag(_, value) => zeroize_flexible_value(value),
        FlexibleValue::Array(values) => values.iter_mut().for_each(zeroize_flexible_value),
        FlexibleValue::Map(entries) => {
            for (key, value) in entries {
                zeroize_flexible_value(key);
                zeroize_flexible_value(value);
            }
        }
        _ => {}
    }
}

/// An authenticated, unlocked session for repeatedly reading one project's tasks.
///
/// The session intentionally has no `Debug` implementation because it retains
/// authenticated API credentials and the decrypted project context.
pub struct ProjectTaskSession {
    runtime: RuntimeClient,
    client: PublicApiClient,
    credentials: sealtask_client_auth::Credentials,
    data_key: sealtask_client_crypto::SymmetricKey,
    work_list_id: Uuid,
    context: WorkListContext,
    scheme_history: Vec<TaskReferenceSchemeResponse>,
}

impl ProjectTaskSession {
    #[must_use]
    pub const fn work_list_id(&self) -> Uuid {
        self.work_list_id
    }

    /// Fetch the authoritative task collection while reusing the unlocked
    /// project context retained by this session.
    pub async fn list_tasks(
        &mut self,
        include_completed: bool,
        include_archived: bool,
    ) -> PublicResult<Vec<AgentTaskSummary>> {
        self.refresh_context().await?;
        let cache_guard = self
            .runtime
            .read_cache
            .begin_online_read(&self.credentials)?;
        let response = self
            .client
            .get_tasks(self.work_list_id, include_archived)
            .await?;
        if self.runtime.read_cache.is_enabled() {
            self.runtime.read_cache.record_online(
                cache_guard.as_ref(),
                &self.data_key,
                &ReadCacheQuery::ProjectTasks {
                    work_list_id: self.work_list_id,
                    include_archived,
                },
                &response,
            )?;
        }
        let tasks = if include_completed {
            response.tasks
        } else {
            response
                .tasks
                .into_iter()
                .filter(|task| !task.is_completed)
                .collect()
        };

        Ok(tasks
            .into_iter()
            .map(|task| self.runtime.project_task_summary(task, Some(&self.context)))
            .collect())
    }

    async fn refresh_context(&mut self) -> PublicResult<()> {
        let cache_guard = self
            .runtime
            .read_cache
            .begin_online_read(&self.credentials)?;
        let work_list = self.client.get_work_list(self.work_list_id).await?;
        self.runtime.read_cache.record_online(
            cache_guard.as_ref(),
            &self.data_key,
            &ReadCacheQuery::WorkList {
                work_list_id: self.work_list_id,
            },
            &work_list,
        )?;

        self.scheme_history = if matches!(
            (
                work_list.work_list.task_references_enabled_at,
                work_list.work_list.current_task_reference_scheme_revision,
                work_list
                    .work_list
                    .current_task_reference_scheme_revision_id,
            ),
            (Some(_), Some(_), Some(_))
        ) {
            match self
                .client
                .get_task_reference_schemes(self.work_list_id)
                .await
            {
                Ok(history) => {
                    self.runtime.read_cache.record_online(
                        cache_guard.as_ref(),
                        &self.data_key,
                        &ReadCacheQuery::TaskReferenceSchemes {
                            work_list_id: self.work_list_id,
                        },
                        &history,
                    )?;
                    history
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };
        self.context = self.runtime.context_from_work_list_detail(
            &work_list,
            &self.scheme_history,
            Some(&self.data_key),
        );
        Ok(())
    }

    /// Issue a fresh one-time stream token and connect to this project's event
    /// feed without exposing the bearer token to callers.
    pub async fn connect_events(&self) -> PublicResult<BoardEventStream> {
        self.runtime.require_online("project event streams")?;
        let mut client = self.client.clone();
        let token = client.issue_project_sse_token(self.work_list_id).await?;
        client
            .connect_project_events(self.work_list_id, &token)
            .await
    }
}

impl RuntimeClient {
    pub async fn list_tasks(
        &self,
        work_list_id: Option<Uuid>,
        include_completed: bool,
        all: bool,
        password_stdin: bool,
    ) -> PublicResult<Vec<AgentTaskSummary>> {
        if !all && let Some(work_list_id) = work_list_id {
            return self
                .list_project_tasks(work_list_id, include_completed, false, password_stdin)
                .await;
        }

        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt task data.",
            )
            .await?;
        let work_lists_query = ReadCacheQuery::WorkLists {
            include_archived: false,
        };
        let my_tasks_query = ReadCacheQuery::MyTasks { include_completed };
        let (work_lists, tasks): (Vec<WorkListResponse>, Vec<MyTaskResponse>) = if self.is_offline()
        {
            (
                self.read_cache
                    .read_offline(&credentials, &data_key, &work_lists_query)?,
                self.read_cache
                    .read_offline(&credentials, &data_key, &my_tasks_query)?,
            )
        } else {
            let cached_work_lists = self.read_cache.memoized(&credentials, &work_lists_query)?;
            let cached_tasks = self.read_cache.memoized(&credentials, &my_tasks_query)?;
            let mut client = if cached_work_lists.is_none() || cached_tasks.is_none() {
                Some(self.api_client_with_credentials(credentials.clone())?)
            } else {
                None
            };
            let work_lists = match cached_work_lists {
                Some(work_lists) => work_lists,
                None => {
                    let cache_guard = self.read_cache.begin_online_read(&credentials)?;
                    let work_lists = client
                        .as_mut()
                        .expect("API client exists for an uncached snapshot")
                        .list_work_lists()
                        .await?;
                    self.read_cache.record_online(
                        cache_guard.as_ref(),
                        &data_key,
                        &work_lists_query,
                        &work_lists,
                    )?;
                    work_lists
                }
            };
            let tasks = match cached_tasks {
                Some(tasks) => tasks,
                None => {
                    let cache_guard = self.read_cache.begin_online_read(&credentials)?;
                    let tasks = client
                        .as_mut()
                        .expect("API client exists for an uncached snapshot")
                        .get_all_my_tasks(include_completed)
                        .await?;
                    self.read_cache.record_online(
                        cache_guard.as_ref(),
                        &data_key,
                        &my_tasks_query,
                        &tasks,
                    )?;
                    tasks
                }
            };
            (work_lists, tasks)
        };
        let mut scheme_histories = HashMap::new();
        for work_list in &work_lists {
            let history = self
                .load_read_task_reference_scheme_history(&credentials, &data_key, work_list)
                .await;
            if !history.is_empty() {
                scheme_histories.insert(work_list.id, history);
            }
        }
        let contexts =
            self.build_work_list_contexts(&work_lists, &scheme_histories, Some(&data_key));

        Ok(tasks
            .into_iter()
            .map(|task| {
                let context = contexts.get(&task.work_list_id);
                self.project_my_task_summary(task, context)
            })
            .collect())
    }

    /// List tasks from one project, with independent completed and archived filters.
    pub async fn list_project_tasks(
        &self,
        work_list_id: Uuid,
        include_completed: bool,
        include_archived: bool,
        password_stdin: bool,
    ) -> PublicResult<Vec<AgentTaskSummary>> {
        let (credentials, data_key, context) = self
            .load_read_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt task data.",
            )
            .await?;
        let query = ReadCacheQuery::ProjectTasks {
            work_list_id,
            include_archived,
        };
        let response: TaskListResponse = if self.is_offline() {
            self.read_cache
                .read_offline(&credentials, &data_key, &query)?
        } else if let Some(cached) = self.read_cache.memoized(&credentials, &query)? {
            cached
        } else {
            let cache_guard = self.read_cache.begin_online_read(&credentials)?;
            let mut client = self.api_client_with_credentials(credentials.clone())?;
            let response = client.get_tasks(work_list_id, include_archived).await?;
            self.read_cache
                .record_online(cache_guard.as_ref(), &data_key, &query, &response)?;
            response
        };
        Ok(response
            .tasks
            .into_iter()
            .filter(|task| include_completed || !task.is_completed)
            .map(|task| self.project_task_summary(task, Some(&context.work_list)))
            .collect())
    }

    /// Authenticate, unlock, and resolve the decrypted context for one project
    /// so repeated authoritative task refreshes can reuse it.
    pub async fn project_task_session(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<ProjectTaskSession> {
        self.require_online("live project task sessions")?;
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt task data.",
            )
            .await?;
        let client = self.api_client_with_credentials(credentials)?;
        self.project_task_session_with_client_and_data_key(work_list_id, client, &data_key)
            .await
    }

    async fn project_task_session_with_client_and_data_key(
        &self,
        work_list_id: Uuid,
        mut client: PublicApiClient,
        data_key: &sealtask_client_crypto::SymmetricKey,
    ) -> PublicResult<ProjectTaskSession> {
        let credentials = client.clone().into_credentials().ok_or_else(|| {
            PublicError::unexpected("project task session API client is not authenticated")
        })?;
        let cache_guard = self.read_cache.begin_online_read(&credentials)?;
        let work_list = client.get_work_list(work_list_id).await?;
        if self.read_cache.is_enabled() {
            self.read_cache.record_online(
                cache_guard.as_ref(),
                data_key,
                &ReadCacheQuery::WorkList { work_list_id },
                &work_list,
            )?;
        }
        let scheme_history = if matches!(
            (
                work_list.work_list.task_references_enabled_at,
                work_list.work_list.current_task_reference_scheme_revision,
                work_list
                    .work_list
                    .current_task_reference_scheme_revision_id,
            ),
            (Some(_), Some(_), Some(_))
        ) {
            match client.get_task_reference_schemes(work_list_id).await {
                Ok(history) => {
                    self.read_cache.record_online(
                        cache_guard.as_ref(),
                        data_key,
                        &ReadCacheQuery::TaskReferenceSchemes { work_list_id },
                        &history,
                    )?;
                    history
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };
        let context =
            self.context_from_work_list_detail(&work_list, &scheme_history, Some(data_key));
        Ok(ProjectTaskSession {
            runtime: self.clone(),
            client,
            credentials,
            data_key: data_key.clone(),
            work_list_id,
            context,
            scheme_history,
        })
    }

    pub async fn get_task(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentTaskDetail> {
        let (credentials, data_key, context) = self
            .load_read_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt task data.",
            )
            .await?;
        let query = ReadCacheQuery::Task {
            work_list_id,
            task_id,
        };
        let detail: TaskDetailResponse = if self.is_offline() {
            self.read_cache
                .read_offline(&credentials, &data_key, &query)?
        } else if let Some(cached) = self.read_cache.memoized(&credentials, &query)? {
            cached
        } else {
            let cache_guard = self.read_cache.begin_online_read(&credentials)?;
            let mut client = self.api_client_with_credentials(credentials.clone())?;
            let detail = client.get_task(work_list_id, task_id).await?;
            self.read_cache
                .record_online(cache_guard.as_ref(), &data_key, &query, &detail)?;
            detail
        };

        let task = self.project_task_summary(detail.task, Some(&context.work_list));
        let comments = detail
            .comments
            .into_iter()
            .map(|comment| self.project_comment(comment, context.work_list.list_key.as_ref()))
            .collect();
        Ok(AgentTaskDetail { task, comments })
    }

    pub async fn resolve_task_reference(
        &self,
        reference: &str,
        work_list_id: Option<Uuid>,
        password_stdin: bool,
    ) -> PublicResult<AgentTaskDetail> {
        if parse_task_reference(reference).is_none() {
            return Err(PublicError::validation(
                "task reference must be a full reference such as OPS-184",
            ));
        }
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to resolve an encrypted task reference.",
            )
            .await?;
        let work_lists = self
            .load_task_reference_lookup_work_lists(&credentials, &data_key, work_list_id)
            .await?;
        let contexts = self
            .load_task_reference_lookup_contexts(&credentials, &data_key, &work_lists)
            .await?;
        let (resolved_work_list_id, reference_number) =
            resolve_task_reference_candidate(reference, work_list_id, &contexts)?;
        let context = contexts.get(&resolved_work_list_id).ok_or_else(|| {
            PublicError::unexpected("resolved task reference lost its work list context")
        })?;
        let detail = self
            .load_task_by_reference_number(
                &credentials,
                &data_key,
                resolved_work_list_id,
                reference_number,
            )
            .await?;
        Ok(self.project_task_detail(detail, context))
    }

    pub async fn resolve_project_task_reference_number(
        &self,
        work_list_id: Uuid,
        reference_number: i64,
        password_stdin: bool,
    ) -> PublicResult<AgentTaskDetail> {
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to resolve an encrypted task reference.",
            )
            .await?;
        let work_lists = self
            .load_task_reference_lookup_work_lists(&credentials, &data_key, Some(work_list_id))
            .await?;
        let contexts = self
            .load_task_reference_lookup_contexts(&credentials, &data_key, &work_lists)
            .await?;
        let context = contexts.get(&work_list_id).ok_or_else(|| {
            PublicError::unexpected("task reference lost its selected work list context")
        })?;
        let detail = self
            .load_task_by_reference_number(&credentials, &data_key, work_list_id, reference_number)
            .await?;
        Ok(self.project_task_detail(detail, context))
    }

    async fn load_task_reference_lookup_work_lists(
        &self,
        credentials: &sealtask_client_auth::Credentials,
        data_key: &sealtask_client_crypto::SymmetricKey,
        work_list_id: Option<Uuid>,
    ) -> PublicResult<Vec<WorkListResponse>> {
        if let Some(work_list_id) = work_list_id {
            let query = ReadCacheQuery::WorkList { work_list_id };
            let detail: sealtask_client_api::WorkListDetailResponse = if self.is_offline() {
                self.read_cache
                    .read_offline(credentials, data_key, &query)?
            } else if let Some(cached) = self.read_cache.memoized(credentials, &query)? {
                cached
            } else {
                let cache_guard = self.read_cache.begin_online_read(credentials)?;
                let mut client = self.api_client_with_credentials(credentials.clone())?;
                let detail = client.get_work_list(work_list_id).await?;
                self.read_cache
                    .record_online(cache_guard.as_ref(), data_key, &query, &detail)?;
                detail
            };
            return Ok(vec![detail.work_list]);
        }

        // Archived projects remain valid lookup scopes and may own a
        // colliding private prefix. Excluding them could turn an incomplete
        // directory into a false miss or sole auto-resolution.
        let query = ReadCacheQuery::WorkLists {
            include_archived: true,
        };
        if self.is_offline() {
            self.read_cache.read_offline(credentials, data_key, &query)
        } else if let Some(cached) = self.read_cache.memoized(credentials, &query)? {
            Ok(cached)
        } else {
            let cache_guard = self.read_cache.begin_online_read(credentials)?;
            let mut client = self.api_client_with_credentials(credentials.clone())?;
            let work_lists = client.list_work_lists_with_archived(true).await?;
            self.read_cache
                .record_online(cache_guard.as_ref(), data_key, &query, &work_lists)?;
            Ok(work_lists)
        }
    }

    async fn load_task_reference_lookup_contexts(
        &self,
        credentials: &sealtask_client_auth::Credentials,
        data_key: &sealtask_client_crypto::SymmetricKey,
        work_lists: &[WorkListResponse],
    ) -> PublicResult<HashMap<Uuid, WorkListContext>> {
        let mut scheme_histories = HashMap::new();
        let mut reference_enabled_ids = HashSet::new();
        for work_list in work_lists {
            match (
                work_list.task_references_enabled_at.is_some(),
                work_list.current_task_reference_scheme_revision,
                work_list.current_task_reference_scheme_revision_id,
            ) {
                (false, None, None) => {}
                (true, Some(revision), Some(_))
                    if (1..=TASK_REFERENCE_REVISION_MAX).contains(&revision) =>
                {
                    let history = self
                        .load_read_task_reference_scheme_history_strict(
                            credentials,
                            data_key,
                            work_list,
                        )
                        .await?;
                    scheme_histories.insert(work_list.id, history);
                    reference_enabled_ids.insert(work_list.id);
                }
                _ => {
                    return Err(PublicError::unexpected(format!(
                        "task reference metadata is incomplete for work list {}",
                        work_list.id
                    )));
                }
            }
        }

        let contexts = self.build_work_list_contexts(work_lists, &scheme_histories, Some(data_key));
        ensure_task_reference_lookup_complete(&reference_enabled_ids, &contexts)?;
        Ok(contexts)
    }

    async fn load_task_by_reference_number(
        &self,
        credentials: &sealtask_client_auth::Credentials,
        data_key: &sealtask_client_crypto::SymmetricKey,
        work_list_id: Uuid,
        reference_number: i64,
    ) -> PublicResult<TaskDetailResponse> {
        let query = ReadCacheQuery::TaskByReferenceNumber {
            work_list_id,
            reference_number,
        };
        let detail: TaskDetailResponse = if self.is_offline() {
            self.read_cache
                .read_offline(credentials, data_key, &query)?
        } else if let Some(cached) = self.read_cache.memoized(credentials, &query)? {
            cached
        } else {
            let cache_guard = self.read_cache.begin_online_read(credentials)?;
            let mut client = self.api_client_with_credentials(credentials.clone())?;
            let detail = client
                .get_task_by_reference_number(work_list_id, reference_number)
                .await?;
            self.read_cache
                .record_online(cache_guard.as_ref(), data_key, &query, &detail)?;
            detail
        };
        if detail.task.work_list_id != work_list_id
            || detail.task.reference_number != Some(reference_number)
        {
            return Err(PublicError::unexpected(
                "task reference lookup returned mismatched public metadata",
            ));
        }
        Ok(detail)
    }

    fn project_task_detail(
        &self,
        detail: TaskDetailResponse,
        context: &WorkListContext,
    ) -> AgentTaskDetail {
        let task = self.project_task_summary(detail.task, Some(context));
        let comments = detail
            .comments
            .into_iter()
            .map(|comment| self.project_comment(comment, context.list_key.as_ref()))
            .collect();
        AgentTaskDetail { task, comments }
    }

    pub async fn create_task(&self, args: CreateTaskArgs) -> PublicResult<AgentTaskSummary> {
        let prepared = self.prepare_task_create(args).await?;
        self.execute_prepared_task_create(prepared).await
    }

    /// Resolve, normalize, and encrypt a task create without issuing a mutation.
    pub async fn prepare_task_create(
        &self,
        args: CreateTaskArgs,
    ) -> PublicResult<PreparedTaskCreate> {
        let (client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to create encrypted task payloads.",
            )
            .await?;
        self.prepare_task_create_with_context(args, client, context)
    }

    /// Resolve and prepare a task create whose idempotency key is derived only
    /// after the target project's key has been unlocked.
    pub async fn prepare_task_create_with_idempotency_derivation(
        &self,
        args: CreateTaskArgs,
        derivation: &TaskCreateIdempotencyDerivation,
    ) -> PublicResult<PreparedTaskCreate> {
        let (client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to create encrypted task payloads.",
            )
            .await?;
        self.prepare_task_create_with_context_and_idempotency_derivation(
            args,
            client,
            context,
            Some(derivation),
        )
    }

    fn prepare_task_create_with_context(
        &self,
        args: CreateTaskArgs,
        client: PublicApiClient,
        context: WorkListContext,
    ) -> PublicResult<PreparedTaskCreate> {
        self.prepare_task_create_with_context_and_idempotency_derivation(
            args, client, context, None,
        )
    }

    fn prepare_task_create_with_context_and_idempotency_derivation(
        &self,
        args: CreateTaskArgs,
        client: PublicApiClient,
        context: WorkListContext,
        idempotency_derivation: Option<&TaskCreateIdempotencyDerivation>,
    ) -> PublicResult<PreparedTaskCreate> {
        let context = SensitiveWorkListContext(Some(context));
        let list_key = self.require_work_list_key(context.get()?)?;
        let binding_key = derive_payload_binding_key(list_key)?;
        let input = SensitiveTaskCreateInput(args.input);

        let normalized_title = input.0.title.trim();
        if normalized_title.is_empty() {
            return Err(PublicError::validation("title is required"));
        }

        validate_priority(input.0.priority)?;
        let normalized_body = input
            .0
            .body
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let mut checklist = SensitiveChecklist(
            input
                .0
                .checklist
                .clone()
                .map(normalize_checklist)
                .transpose()?,
        );
        let mut idempotency_key = SensitiveOptionalString(task_create_idempotency_key(
            &input.0,
            idempotency_derivation,
            &args.work_list_id,
            list_key,
        )?);
        let semantics = TaskCreateSemanticPlan {
            title: normalized_title,
            body: normalized_body,
            checklist: checklist.0.as_deref(),
            priority: input.0.priority,
            due_at: input.0.due_at.as_ref(),
            start_at: input.0.start_at.as_ref(),
            section_id: input.0.section_id,
        };
        let canonical_semantics =
            Zeroizing::new(serde_json::to_vec(&semantics).map_err(|err| {
                PublicError::unexpected(format!("failed to encode task create semantics: {err}"))
            })?);
        let mut idempotency_commitment = SensitiveOptionalString(if idempotency_key.is_some() {
            Some(compute_task_create_semantic_commitment(
                &canonical_semantics,
                list_key,
            )?)
        } else {
            None
        });
        let idempotency_protected = idempotency_key.is_some();
        let mut request = SensitiveCreateTaskRequest(Some(CreateTaskRequest {
            title_ciphertext: String::new(),
            title_ciphertext_proof: String::new(),
            payload_ciphertext: String::new(),
            payload_ciphertext_proof: String::new(),
            attachment_ids: Vec::new(),
            priority: input.0.priority,
            due_at: input.0.due_at,
            start_at: input.0.start_at,
            section_id: input.0.section_id,
            idempotency_key: idempotency_key.take(),
            idempotency_commitment: idempotency_commitment.take(),
        }));
        let change_commitment =
            task_mutation_change_commitment(TASK_CREATE_ACTION, &canonical_semantics, list_key)?;

        let checklist_present = checklist.0.is_some();
        let envelope = SensitiveTaskPayloadEnvelope(build_task_payload_envelope(
            TaskPayloadBody {
                title: normalized_title.to_string(),
                rich_text: normalized_body.and_then(plaintext_rich_text),
                checklist: checklist.0.take(),
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
            },
            1,
        ));
        let mut payload_ciphertext =
            SensitiveSealedBlob(encrypt_task_payload(&envelope.0, list_key)?);
        request.get_mut().payload_ciphertext = std::mem::take(&mut payload_ciphertext.0.base64);
        request.get_mut().payload_ciphertext_proof =
            compute_payload_proof(&payload_ciphertext.0.bytes, &binding_key)?;
        let mut title_ciphertext = SensitiveSealedBlob(encrypt_text_value(
            normalized_title,
            list_key,
            TASK_TITLE_CONTEXT,
        )?);
        request.get_mut().title_ciphertext = std::mem::take(&mut title_ciphertext.0.base64);
        request.get_mut().title_ciphertext_proof =
            compute_payload_proof(&title_ciphertext.0.bytes, &binding_key)?;
        let mut changed_fields = vec!["title".to_string()];
        if normalized_body.is_some() {
            changed_fields.push("body".to_string());
        }
        if checklist_present {
            changed_fields.push("checklist".to_string());
        }
        if input.0.priority.is_some() {
            changed_fields.push("priority".to_string());
        }
        if input.0.due_at.is_some() {
            changed_fields.push("dueAt".to_string());
        }
        if input.0.start_at.is_some() {
            changed_fields.push("startAt".to_string());
        }
        if input.0.section_id.is_some() {
            changed_fields.push("sectionId".to_string());
        }
        let changed_field_count = changed_fields.len();
        let plan = TaskMutationPlan {
            schema_version: TASK_MUTATION_PLAN_SCHEMA_VERSION,
            plan_type: TASK_MUTATION_PLAN_TYPE,
            action: TASK_CREATE_ACTION,
            project_id: args.work_list_id,
            task_id: None,
            section_id: input.0.section_id,
            expected_updated_at: None,
            changed_fields,
            changed_field_count,
            change_commitment,
            idempotency_protected,
            would_change: true,
            will_mutate: false,
        };
        Ok(PreparedTaskCreate {
            client,
            context,
            work_list_id: args.work_list_id,
            request,
            plan,
        })
    }

    /// Execute the exact request retained by a prepared task create.
    pub async fn execute_prepared_task_create(
        &self,
        mut prepared: PreparedTaskCreate,
    ) -> PublicResult<AgentTaskSummary> {
        let result = prepared
            .client
            .create_task(prepared.work_list_id, prepared.request.get())
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let created = result?;
        Ok(self.project_task_summary(created, Some(prepared.context.get()?)))
    }

    pub async fn update_task(&self, args: UpdateTaskArgs) -> PublicResult<AgentTaskSummary> {
        let prepared = self.prepare_task_update(args).await?;
        self.execute_prepared_task_update(prepared).await
    }

    pub async fn update_task_if_unchanged(
        &self,
        args: UpdateTaskArgs,
        expected_updated_at: DateTime<Utc>,
    ) -> PublicResult<AgentTaskSummary> {
        let prepared = self
            .prepare_task_update_if_unchanged(args, expected_updated_at)
            .await?;
        self.execute_prepared_task_update(prepared).await
    }

    /// Fetch and prepare a task update against the authoritative current revision.
    pub async fn prepare_task_update(
        &self,
        args: UpdateTaskArgs,
    ) -> PublicResult<PreparedTaskUpdate> {
        self.prepare_task_update_with_expected_revision(args, None)
            .await
    }

    /// Fetch and prepare a conditional task update while retaining the caller's
    /// optimistic-concurrency revision in the exact request.
    pub async fn prepare_task_update_if_unchanged(
        &self,
        args: UpdateTaskArgs,
        expected_updated_at: DateTime<Utc>,
    ) -> PublicResult<PreparedTaskUpdate> {
        self.prepare_task_update_with_expected_revision(args, Some(expected_updated_at))
            .await
    }

    async fn prepare_task_update_with_expected_revision(
        &self,
        args: UpdateTaskArgs,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> PublicResult<PreparedTaskUpdate> {
        let (client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to update encrypted task payloads.",
            )
            .await?;
        self.prepare_task_update_with_loaded_context(args, expected_updated_at, client, context)
            .await
    }

    async fn prepare_task_update_with_loaded_context(
        &self,
        args: UpdateTaskArgs,
        expected_updated_at: Option<DateTime<Utc>>,
        mut client: PublicApiClient,
        context: WorkListContext,
    ) -> PublicResult<PreparedTaskUpdate> {
        let context = SensitiveWorkListContext(Some(context));
        let list_key = self.require_work_list_key(context.get()?)?;
        let binding_key = derive_payload_binding_key(list_key)?;
        let task_detail = client.get_task(args.work_list_id, args.task_id).await?;
        let sealtask_client_api::TaskDetailResponse { task, mut comments } = task_detail;
        zeroize_task_comments(&mut comments);
        let task = SensitiveTaskResponse(Some(task));
        let task_updated_at = task.get()?.updated_at;
        if expected_updated_at
            .as_ref()
            .is_some_and(|expected| *expected != task_updated_at)
        {
            return Err(PublicError::conflict(
                "task changed after the expected revision",
            ));
        }
        let expected_updated_at = expected_updated_at.unwrap_or(task_updated_at);
        self.prepare_task_update_with_context(
            args,
            expected_updated_at,
            client,
            context,
            task,
            &binding_key,
        )
    }

    fn prepare_task_update_with_context(
        &self,
        args: UpdateTaskArgs,
        expected_updated_at: DateTime<Utc>,
        client: PublicApiClient,
        context: SensitiveWorkListContext,
        current_guard: SensitiveTaskResponse,
        binding_key: &sealtask_client_crypto::SymmetricKey,
    ) -> PublicResult<PreparedTaskUpdate> {
        let list_key = self.require_work_list_key(context.get()?)?;
        let current = current_guard.get()?;
        let input = SensitiveTaskUpdateInput(args.input);

        let payload_requested = input.0.title.is_some()
            || !input.0.body.is_unchanged()
            || !input.0.checklist.is_unchanged();
        if !payload_requested
            && input.0.priority.is_unchanged()
            && input.0.due_at.is_unchanged()
            && input.0.start_at.is_unchanged()
            && input.0.section_id.is_unchanged()
        {
            return Err(PublicError::validation(
                "provide at least one task field to update",
            ));
        }
        if let TaskFieldPatch::Set(value) = &input.0.priority {
            validate_priority(Some(*value))?;
        }
        let normalized_title = normalized_optional_title(input.0.title.as_deref())?;
        let normalized_body = SensitiveRichTextPatch(match &input.0.body {
            TaskFieldPatch::Unchanged => None,
            TaskFieldPatch::Set(value) => Some(plaintext_rich_text(value)),
            TaskFieldPatch::Clear => Some(None),
        });
        let mut normalized_checklist = SensitiveChecklistPatch(match &input.0.checklist {
            TaskFieldPatch::Unchanged => None,
            TaskFieldPatch::Set(items) => Some(Some(normalize_checklist(items.clone())?)),
            TaskFieldPatch::Clear => Some(None),
        });

        let mut request = SensitiveUpdateTaskRequest(Some(UpdateTaskRequest {
            expected_updated_at: Some(expected_updated_at),
            ..UpdateTaskRequest::default()
        }));
        let mut changed_fields = Vec::new();
        let mut next_title_for_commitment = SensitiveOptionalString(None);
        let mut next_body_for_commitment = SensitiveRichTextPatch(None);
        let mut next_checklist_for_commitment = SensitiveChecklistPatch(None);
        if payload_requested {
            let existing_payload_bytes =
                Zeroizing::new(decode_sealed_blob(&current.payload_ciphertext)?);
            let existing_payload = decrypt_task_payload(list_key, &existing_payload_bytes)?;
            let existing_body = SensitiveTaskPayloadBody(existing_payload.body);
            let title_changed =
                normalized_title.is_some_and(|title| title != existing_body.0.title);
            let body_changed = match normalized_body.0.as_ref() {
                Some(body) => !sensitive_serialized_eq(body, &existing_body.0.rich_text)?,
                None => false,
            };
            let checklist_changed = match normalized_checklist.0.as_ref() {
                Some(checklist) => !sensitive_serialized_eq(checklist, &existing_body.0.checklist)?,
                None => false,
            };
            if title_changed {
                changed_fields.push("title".to_string());
            }
            if body_changed {
                changed_fields.push("body".to_string());
            }
            if checklist_changed {
                changed_fields.push("checklist".to_string());
            }
            if title_changed || body_changed || checklist_changed {
                let next_title = normalized_title
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| existing_body.0.title.clone());
                let next_rich_text = normalized_body
                    .0
                    .clone()
                    .unwrap_or_else(|| existing_body.0.rich_text.clone());
                let next_checklist = normalized_checklist
                    .0
                    .take()
                    .unwrap_or_else(|| existing_body.0.checklist.clone());
                let next_body = SensitiveTaskPayloadBody(TaskPayloadBody {
                    title: next_title,
                    rich_text: next_rich_text,
                    checklist: next_checklist,
                    attachments: existing_body.0.attachments.clone(),
                    references: existing_body.0.references.clone(),
                    mentions: existing_body.0.mentions.clone(),
                    client_meta: existing_body.0.client_meta.clone(),
                    recurrence_state: existing_body.0.recurrence_state.clone(),
                });
                let envelope = SensitiveTaskPayloadEnvelope(build_task_payload_envelope(
                    TaskPayloadBody {
                        title: next_body.0.title.clone(),
                        rich_text: next_body.0.rich_text.clone(),
                        checklist: next_body.0.checklist.clone(),
                        attachments: next_body.0.attachments.clone(),
                        references: next_body.0.references.clone(),
                        mentions: next_body.0.mentions.clone(),
                        client_meta: next_body.0.client_meta.clone(),
                        recurrence_state: next_body.0.recurrence_state.clone(),
                    },
                    1,
                ));
                let mut payload_ciphertext =
                    SensitiveSealedBlob(encrypt_task_payload(&envelope.0, list_key)?);
                request.get_mut().payload_ciphertext =
                    Some(std::mem::take(&mut payload_ciphertext.0.base64));
                request.get_mut().payload_ciphertext_proof = Some(compute_payload_proof(
                    &payload_ciphertext.0.bytes,
                    binding_key,
                )?);
                next_title_for_commitment.0 = title_changed.then(|| next_body.0.title.clone());
                next_body_for_commitment.0 = body_changed.then(|| next_body.0.rich_text.clone());
                next_checklist_for_commitment.0 =
                    checklist_changed.then(|| next_body.0.checklist.clone());
            }
        }

        if changed_fields.iter().any(|field| field == "title") {
            let normalized_title = normalized_title.ok_or_else(|| {
                PublicError::unexpected("changed task title lost its normalized value")
            })?;
            let mut title_ciphertext = SensitiveSealedBlob(encrypt_text_value(
                normalized_title,
                list_key,
                TASK_TITLE_CONTEXT,
            )?);
            request.get_mut().title_ciphertext =
                Some(std::mem::take(&mut title_ciphertext.0.base64));
            request.get_mut().title_ciphertext_proof = Some(compute_payload_proof(
                &title_ciphertext.0.bytes,
                binding_key,
            )?);
        }

        request.get_mut().priority = effective_patch(&input.0.priority, &current.priority);
        if request.get().priority.is_some() {
            changed_fields.push("priority".to_string());
        }
        request.get_mut().due_at = effective_patch(&input.0.due_at, &current.due_at);
        if request.get().due_at.is_some() {
            changed_fields.push("dueAt".to_string());
        }
        request.get_mut().start_at = effective_patch(&input.0.start_at, &current.start_at);
        if request.get().start_at.is_some() {
            changed_fields.push("startAt".to_string());
        }
        request.get_mut().section_id = effective_patch(&input.0.section_id, &current.section_id);
        if request.get().section_id.is_some() {
            changed_fields.push("sectionId".to_string());
        }
        let next_section_id = patched_value(&input.0.section_id, &current.section_id);
        let guarded_request = request.get();
        let semantic_plan = TaskUpdateSemanticPlan {
            project_id: args.work_list_id,
            task_id: args.task_id,
            expected_updated_at: &expected_updated_at,
            changed_fields: &changed_fields,
            title: next_title_for_commitment.0.as_deref(),
            body: next_body_for_commitment.0.as_ref().and_then(Option::as_ref),
            checklist: next_checklist_for_commitment
                .0
                .as_ref()
                .and_then(Option::as_deref),
            priority: guarded_request.priority.flatten(),
            due_at: guarded_request.due_at.as_ref().and_then(Option::as_ref),
            start_at: guarded_request.start_at.as_ref().and_then(Option::as_ref),
            section_id: guarded_request.section_id.flatten(),
        };
        let canonical_semantics =
            Zeroizing::new(serde_json::to_vec(&semantic_plan).map_err(|err| {
                PublicError::unexpected(format!("failed to encode task update semantics: {err}"))
            })?);
        let change_commitment =
            task_mutation_change_commitment(TASK_UPDATE_ACTION, &canonical_semantics, list_key)?;
        let changed_field_count = changed_fields.len();
        let would_change = changed_field_count != 0;
        let plan = TaskMutationPlan {
            schema_version: TASK_MUTATION_PLAN_SCHEMA_VERSION,
            plan_type: TASK_MUTATION_PLAN_TYPE,
            action: TASK_UPDATE_ACTION,
            project_id: args.work_list_id,
            task_id: Some(args.task_id),
            section_id: next_section_id,
            expected_updated_at: Some(expected_updated_at),
            changed_fields,
            changed_field_count,
            change_commitment,
            idempotency_protected: false,
            would_change,
            will_mutate: false,
        };
        Ok(PreparedTaskUpdate {
            client,
            context,
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            request,
            current: current_guard,
            plan,
        })
    }

    /// Execute the exact request retained by a prepared task update.
    ///
    /// A true no-op returns the already-fetched authoritative task without
    /// issuing a PATCH.
    pub async fn execute_prepared_task_update(
        &self,
        mut prepared: PreparedTaskUpdate,
    ) -> PublicResult<AgentTaskSummary> {
        if !prepared.plan.would_change {
            let context = prepared.context.get()?;
            let current = prepared.current.take();
            return Ok(self.project_task_summary(current, Some(context)));
        }

        let result = prepared
            .client
            .update_task(
                prepared.work_list_id,
                prepared.task_id,
                prepared.request.get(),
            )
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let updated = result?;
        Ok(self.project_task_summary(updated, Some(prepared.context.get()?)))
    }

    pub async fn move_task(&self, args: MoveTaskArgs) -> PublicResult<AgentTaskSummary> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to decrypt moved task data.",
            )
            .await?;
        let current = client.get_task(args.work_list_id, args.task_id).await?;
        let result = client
            .move_task(
                args.work_list_id,
                args.task_id,
                &MoveTaskRequest {
                    expected_updated_at: Some(current.task.updated_at),
                    section_id: args.input.section_id,
                    insert_before_task_id: args.input.insert_before_task_id,
                    section_boundary: None,
                },
            )
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let moved = result?;
        Ok(self.project_task_summary(moved, Some(&context)))
    }

    pub async fn complete_task(&self, args: TaskCompletionArgs) -> PublicResult<AgentTaskSummary> {
        self.set_task_completion(args, true).await
    }

    pub async fn reopen_task(&self, args: TaskCompletionArgs) -> PublicResult<AgentTaskSummary> {
        self.set_task_completion(args, false).await
    }

    async fn set_task_completion(
        &self,
        args: TaskCompletionArgs,
        complete: bool,
    ) -> PublicResult<AgentTaskSummary> {
        let prompt_message = if complete {
            "Password required to decrypt completed task data."
        } else {
            "Password required to decrypt reopened task data."
        };
        let (mut client, context) = self
            .load_work_list_context(args.work_list_id, args.password_stdin, prompt_message)
            .await?;
        let current = client.get_task(args.work_list_id, args.task_id).await?;
        let result = client
            .move_task(
                args.work_list_id,
                args.task_id,
                &MoveTaskRequest {
                    expected_updated_at: Some(current.task.updated_at),
                    section_id: None,
                    insert_before_task_id: None,
                    section_boundary: Some(if complete {
                        TaskSectionBoundary::Last
                    } else {
                        TaskSectionBoundary::First
                    }),
                },
            )
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let moved = result?;
        Ok(self.project_task_summary(moved, Some(&context)))
    }

    pub async fn archive_task(&self, args: ArchiveTaskArgs) -> PublicResult<AgentTaskSummary> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to decrypt archived task data.",
            )
            .await?;
        let result = client
            .archive_task(
                args.work_list_id,
                args.task_id,
                &ArchiveTaskRequest::default(),
            )
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let archived = result?;
        Ok(self.project_task_summary(archived, Some(&context)))
    }

    pub async fn unarchive_task(&self, args: UnarchiveTaskArgs) -> PublicResult<AgentTaskSummary> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to decrypt unarchived task data.",
            )
            .await?;
        let result = client
            .unarchive_task(
                args.work_list_id,
                args.task_id,
                &UnarchiveTaskRequest::default(),
            )
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        let unarchived = result?;
        Ok(self.project_task_summary(unarchived, Some(&context)))
    }

    pub async fn delete_task(&self, args: DeleteTaskArgs) -> PublicResult<()> {
        let mut client = self.authenticated_api_client()?;
        let result = client
            .delete_task(args.work_list_id, args.task_id, &args.input)
            .await;
        self.read_cache.invalidate_for_mutation_result(&result);
        result
    }
}

fn resolve_task_reference_candidate(
    reference: &str,
    requested_work_list_id: Option<Uuid>,
    contexts: &HashMap<Uuid, WorkListContext>,
) -> PublicResult<(Uuid, i64)> {
    let mut candidates = HashMap::new();
    for (work_list_id, context) in contexts {
        if requested_work_list_id.is_some_and(|requested| requested != *work_list_id)
            || context.current_task_reference_scheme().is_none()
        {
            continue;
        }
        for scheme in &context.task_reference_schemes {
            if let Some(reference_number) = scheme.parse_reference_number(reference) {
                candidates.insert(*work_list_id, reference_number);
            }
        }
    }

    match candidates.len() {
        0 => Err(PublicError::not_found(
            "task reference did not match an accessible work list",
        )),
        1 => candidates
            .into_iter()
            .next()
            .ok_or_else(|| PublicError::unexpected("task reference candidate disappeared")),
        _ => {
            let mut ids = candidates.keys().copied().collect::<Vec<_>>();
            ids.sort_unstable();
            Err(PublicError::conflict(format!(
                "task reference is ambiguous across work lists {}; pass --work-list-id",
                ids.iter()
                    .map(Uuid::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    }
}

fn ensure_task_reference_lookup_complete(
    reference_enabled_ids: &HashSet<Uuid>,
    contexts: &HashMap<Uuid, WorkListContext>,
) -> PublicResult<()> {
    let mut unchecked_work_list_ids = reference_enabled_ids
        .iter()
        .copied()
        .filter(|work_list_id| {
            contexts
                .get(work_list_id)
                .and_then(WorkListContext::current_task_reference_scheme)
                .is_none()
        })
        .collect::<Vec<_>>();
    unchecked_work_list_ids.sort_unstable();
    if unchecked_work_list_ids.is_empty() {
        return Ok(());
    }
    Err(PublicError::unexpected(format!(
        "task reference lookup is unchecked because scheme history is unavailable for work lists {}; no definitive miss or automatic resolution was attempted",
        unchecked_work_list_ids
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

#[cfg(test)]
mod task_reference_tests {
    use super::*;
    use sealtask_client_crypto::TaskReferenceSchemeV1;

    fn reference_context(work_list_id: Uuid, prefixes: &[&str]) -> WorkListContext {
        let schemes = prefixes
            .iter()
            .enumerate()
            .map(|(index, prefix)| {
                TaskReferenceSchemeV1::new(
                    work_list_id,
                    Uuid::from_u128(work_list_id.as_u128() + index as u128 + 1),
                    index as i64 + 1,
                    *prefix,
                    4,
                )
                .expect("valid scheme")
            })
            .collect::<Vec<_>>();
        let current = schemes.last().expect("at least one scheme");
        WorkListContext {
            work_list_title: None,
            work_list_timezone: "UTC".to_string(),
            list_key: None,
            task_reference_schemes: schemes.clone(),
            current_task_reference_scheme_revision: Some(current.revision),
            current_task_reference_scheme_revision_id: Some(current.scheme_revision_id),
            read_error: None,
        }
    }

    #[test]
    fn test_should_resolve_current_and_historical_prefixes_locally() {
        let work_list_id = Uuid::from_u128(0x1111_1111_1111_7111_8111_1111_1111_1111);
        let contexts = HashMap::from([(
            work_list_id,
            reference_context(work_list_id, &["OLD", "NEW"]),
        )]);

        assert_eq!(
            resolve_task_reference_candidate("old-0042", None, &contexts)
                .expect("historical prefix should resolve"),
            (work_list_id, 42)
        );
        assert_eq!(
            resolve_task_reference_candidate("NEW-42", None, &contexts)
                .expect("current prefix should resolve"),
            (work_list_id, 42)
        );
    }

    #[test]
    fn test_should_report_cross_project_prefix_ambiguity() {
        let first = Uuid::from_u128(0x1111_1111_1111_7111_8111_1111_1111_1111);
        let second = Uuid::from_u128(0x2222_2222_2222_7222_8222_2222_2222_2222);
        let contexts = HashMap::from([
            (first, reference_context(first, &["OPS"])),
            (second, reference_context(second, &["OPS"])),
        ]);

        assert!(matches!(
            resolve_task_reference_candidate("OPS-7", None, &contexts),
            Err(PublicError::Conflict(_))
        ));
        assert_eq!(
            resolve_task_reference_candidate("OPS-7", Some(second), &contexts)
                .expect("explicit work-list selection should disambiguate"),
            (second, 7)
        );
    }

    #[test]
    fn test_should_fail_closed_before_using_a_false_sole_candidate() {
        let checked = Uuid::from_u128(0x1111_1111_1111_7111_8111_1111_1111_1111);
        let unchecked = Uuid::from_u128(0x2222_2222_2222_7222_8222_2222_2222_2222);
        let mut unchecked_context = reference_context(unchecked, &["PRIVATE"]);
        unchecked_context.task_reference_schemes.clear();
        let contexts = HashMap::from([
            (checked, reference_context(checked, &["OPS"])),
            (unchecked, unchecked_context),
        ]);

        assert_eq!(
            resolve_task_reference_candidate("OPS-7", None, &contexts)
                .expect("the partial map would otherwise look definitive"),
            (checked, 7)
        );
        let error =
            ensure_task_reference_lookup_complete(&HashSet::from([checked, unchecked]), &contexts)
                .expect_err("an unchecked enabled project must prevent auto-resolution");
        assert!(matches!(error, PublicError::Unexpected(_)));
        assert!(error.to_string().contains(&unchecked.to_string()));
        assert!(!error.to_string().contains("PRIVATE"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use chrono::{TimeDelta, Utc};
    use sealtask_client_api::{
        CommentResponse, CreateTaskRequest, DelegationResponse, MembershipResponse,
        TaskDetailResponse, TaskListResponse, TaskResponse, UpdateTaskRequest,
        WorkListDetailResponse, WorkListResponse,
    };
    use sealtask_client_auth::Credentials;
    use sealtask_client_crypto::{
        KEY_SIZE, SealedPayload, SymmetricKey, build_task_payload_envelope, derive_work_list_key,
        encrypt_task_payload,
    };
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn project_task_session_refreshes_context_and_preserves_filtering() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let work_list_id = Uuid::now_v7();
        let data_key = SymmetricKey::new([0x61; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let active_id = Uuid::now_v7();
        let completed_id = Uuid::now_v7();
        let archived_id = Uuid::now_v7();
        let project_body =
            serde_json::to_vec(&work_list_fixture(work_list_id)).expect("project response");
        let active_body = serde_json::to_vec(&TaskListResponse {
            tasks: vec![
                task_fixture(
                    &list_key,
                    work_list_id,
                    active_id,
                    "Active task",
                    false,
                    false,
                ),
                task_fixture(
                    &list_key,
                    work_list_id,
                    completed_id,
                    "Completed task",
                    true,
                    false,
                ),
            ],
            archived_counts: Vec::new(),
        })
        .expect("active task response");
        let archived_body = serde_json::to_vec(&TaskListResponse {
            tasks: vec![
                task_fixture(
                    &list_key,
                    work_list_id,
                    active_id,
                    "Active task",
                    false,
                    false,
                ),
                task_fixture(
                    &list_key,
                    work_list_id,
                    completed_id,
                    "Completed task",
                    true,
                    false,
                ),
                task_fixture(
                    &list_key,
                    work_list_id,
                    archived_id,
                    "Archived task",
                    false,
                    true,
                ),
            ],
            archived_counts: Vec::new(),
        })
        .expect("archived task response");
        let server = tokio::spawn(async move {
            let mut request_targets = Vec::new();
            for _ in 0..9 {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_http_request(&mut stream).await;
                let target = request_target(&request);
                let response = if target == format!("/work-lists/{work_list_id}") {
                    &project_body
                } else if target == format!("/work-lists/{work_list_id}/tasks?includeArchived=true")
                {
                    &archived_body
                } else {
                    assert_eq!(target, format!("/work-lists/{work_list_id}/tasks"));
                    &active_body
                };
                request_targets.push(target);
                write_http_response(&mut stream, response).await;
            }
            request_targets
        });

        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let mut session = runtime
            .project_task_session_with_client_and_data_key(work_list_id, client, &data_key)
            .await
            .expect("task session");

        assert_eq!(session.work_list_id(), work_list_id);
        assert_eq!(
            task_titles(
                &session
                    .list_tasks(false, false)
                    .await
                    .expect("active refresh")
            ),
            ["Active task"]
        );
        assert_eq!(
            task_titles(
                &session
                    .list_tasks(true, false)
                    .await
                    .expect("completed refresh")
            ),
            ["Active task", "Completed task"]
        );
        assert_eq!(
            task_titles(
                &session
                    .list_tasks(false, true)
                    .await
                    .expect("archived refresh")
            ),
            ["Active task", "Archived task"]
        );
        assert_eq!(
            task_titles(
                &session
                    .list_tasks(true, true)
                    .await
                    .expect("complete refresh")
            ),
            ["Active task", "Completed task", "Archived task"]
        );

        let request_targets = server.await.expect("server");
        assert_eq!(
            request_targets
                .iter()
                .filter(|target| *target == &format!("/work-lists/{work_list_id}"))
                .count(),
            5,
            "the project context must be loaded initially and refreshed before every snapshot"
        );
    }

    #[tokio::test]
    async fn task_reference_transport_sends_only_project_and_numeric_suffix() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let data_key = SymmetricKey::new([0x62; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let mut task = task_fixture(
            &list_key,
            work_list_id,
            Uuid::now_v7(),
            "Referenced task",
            false,
            false,
        );
        task.reference_number = Some(184);
        let expected_task_id = task.id;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![MockHttpResponse::ok_json(&TaskDetailResponse {
                task,
                comments: Vec::new(),
            })],
        );

        let detail = runtime
            .load_task_by_reference_number(
                &test_credentials(&api_url),
                &data_key,
                work_list_id,
                184,
            )
            .await
            .expect("resolve task by numeric reference");
        assert_eq!(detail.task.id, expected_task_id);
        server.await.expect("server");

        let requests = requests.lock().expect("requests");
        let request = String::from_utf8_lossy(&requests[0]);
        assert!(
            request.starts_with(&format!(
                "GET /work-lists/{work_list_id}/tasks/by-reference-number/184 HTTP/"
            )),
            "unexpected request target: {request}"
        );
        assert!(!request.contains("OPS-184"));
    }

    #[tokio::test]
    async fn malformed_task_reference_is_rejected_before_authentication() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");

        let error = runtime
            .resolve_task_reference("not-a-reference", None, false)
            .await
            .expect_err("malformed reference must fail locally");

        assert!(matches!(error, PublicError::Validation(_)));
        assert_eq!(
            error.to_string(),
            "task reference must be a full reference such as OPS-184"
        );
    }

    #[tokio::test]
    async fn task_reference_scheme_history_is_invocation_memoized() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let directory = tempfile::tempdir().expect("cache directory");
        let runtime = RuntimeClient::new(&api_url)
            .expect("runtime")
            .with_read_cache_options(
                crate::ReadCacheOptions::online(directory.path(), "default")
                    .expect("cache options"),
            )
            .expect("runtime cache");
        let work_list_id = Uuid::now_v7();
        let scheme_revision_id = Uuid::now_v7();
        let mut work_list = work_list_fixture(work_list_id).work_list;
        work_list.task_references_enabled_at = Some(Utc::now());
        work_list.current_task_reference_scheme_revision = Some(1);
        work_list.current_task_reference_scheme_revision_id = Some(scheme_revision_id);
        let response = serde_json::json!({
            "schemes": [{
                "schemeRevisionId": scheme_revision_id,
                "workListId": work_list_id,
                "revision": 1,
                "payloadCiphertext": "opaque-scheme-ciphertext",
                "isRepair": false,
                "createdAt": Utc::now(),
                "retiredAt": null,
                "quarantinedAt": null,
                "quarantinedByMembershipId": null
            }]
        });
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![MockHttpResponse::ok_json(&response)],
        );
        let credentials = test_credentials(&api_url);
        let data_key = SymmetricKey::new([0x64; KEY_SIZE]);

        let first = runtime
            .load_read_task_reference_scheme_history_strict(&credentials, &data_key, &work_list)
            .await
            .expect("first scheme-history read");
        let second = runtime
            .load_read_task_reference_scheme_history_strict(&credentials, &data_key, &work_list)
            .await
            .expect("memoized scheme-history read");

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(first[0].scheme_revision_id, scheme_revision_id);
        assert_eq!(second[0].scheme_revision_id, scheme_revision_id);
        server.await.expect("server");
        let requests = requests.lock().expect("requests");
        assert_eq!(requests.len(), 1);
        assert!(String::from_utf8_lossy(&requests[0]).starts_with(&format!(
            "GET /work-lists/{work_list_id}/task-reference-schemes HTTP/"
        )));
    }

    #[tokio::test]
    async fn task_reference_transport_rejects_mismatched_public_metadata() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let data_key = SymmetricKey::new([0x63; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let mut wrong_project_task = task_fixture(
            &list_key,
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Wrong project",
            false,
            false,
        );
        wrong_project_task.reference_number = Some(184);
        let mut wrong_number_task = task_fixture(
            &list_key,
            work_list_id,
            Uuid::now_v7(),
            "Wrong number",
            false,
            false,
        );
        wrong_number_task.reference_number = Some(185);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests,
            vec![
                MockHttpResponse::ok_json(&TaskDetailResponse {
                    task: wrong_project_task,
                    comments: Vec::new(),
                }),
                MockHttpResponse::ok_json(&TaskDetailResponse {
                    task: wrong_number_task,
                    comments: Vec::new(),
                }),
            ],
        );
        let credentials = test_credentials(&api_url);

        for mismatch in ["project", "number"] {
            let error = runtime
                .load_task_by_reference_number(&credentials, &data_key, work_list_id, 184)
                .await
                .expect_err("mismatched public reference metadata must fail");
            assert!(
                matches!(error, PublicError::Unexpected(_)),
                "{mismatch} mismatch returned {error:?}"
            );
        }
        server.await.expect("server");
    }

    #[tokio::test]
    async fn prepared_create_resolves_once_plans_safely_and_executes_the_exact_request() {
        const TITLE_CANARY: &str = "create-title-plaintext-canary";
        const BODY_CANARY: &str = "create-body-plaintext-canary";
        const CHECKLIST_CANARY: &str = "create-checklist-plaintext-canary";
        const IDEMPOTENCY_CANARY: &str = "task:create-secret-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let data_key = SymmetricKey::new([0x71; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let created = task_fixture(
            &list_key,
            work_list_id,
            Uuid::now_v7(),
            TITLE_CANARY,
            false,
            false,
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![
                MockHttpResponse::ok_json(&work_list_fixture(work_list_id)),
                MockHttpResponse::ok_json(&created),
            ],
        );

        let (client, context) =
            resolve_test_context(&runtime, &api_url, work_list_id, &data_key).await;
        let prepared = runtime
            .prepare_task_create_with_context(
                CreateTaskArgs {
                    work_list_id,
                    input: crate::inputs::TaskCreateInput {
                        title: format!("  {TITLE_CANARY}  "),
                        body: Some(format!("  {BODY_CANARY}  ")),
                        checklist: Some(vec![checklist_item(CHECKLIST_CANARY)]),
                        priority: Some(5),
                        due_at: None,
                        start_at: None,
                        section_id: Some(Uuid::now_v7()),
                        idempotency_key: Some(IDEMPOTENCY_CANARY.to_string()),
                    },
                    password_stdin: false,
                },
                client,
                context,
            )
            .expect("prepare create");

        assert_eq!(
            requests.lock().expect("requests").len(),
            1,
            "preparation must resolve the project but issue no mutation"
        );
        assert_eq!(prepared.plan.schema_version, 1);
        assert_eq!(prepared.plan.plan_type, "taskMutationPlan");
        assert_eq!(prepared.plan.action, "task.create");
        assert!(prepared.plan.would_change);
        assert!(!prepared.plan.will_mutate);
        assert!(prepared.plan.idempotency_protected);
        assert_eq!(
            prepared.plan.changed_fields,
            ["title", "body", "checklist", "priority", "sectionId"]
        );
        assert_eq!(
            prepared.plan.changed_field_count,
            prepared.plan.changed_fields.len()
        );
        let plan_json = serde_json::to_string(prepared.plan()).expect("serialize plan");
        let plan_debug = format!("{:?}", prepared.plan());
        let prepared_request = prepared.request.get();
        for canary in [
            TITLE_CANARY,
            BODY_CANARY,
            CHECKLIST_CANARY,
            IDEMPOTENCY_CANARY,
            prepared_request.title_ciphertext.as_str(),
            prepared_request.payload_ciphertext.as_str(),
            prepared_request.title_ciphertext_proof.as_str(),
            prepared_request.payload_ciphertext_proof.as_str(),
        ] {
            assert!(!plan_json.contains(canary));
            assert!(!plan_debug.contains(canary));
        }
        let exact_request = serde_json::to_value(prepared_request).expect("prepared request");
        assert_eq!(
            prepared_request.idempotency_key.as_deref(),
            Some(IDEMPOTENCY_CANARY),
            "an explicit user key must reach the API unchanged"
        );

        let created = runtime
            .execute_prepared_task_create(prepared)
            .await
            .expect("execute prepared create");
        assert_eq!(created.title.as_deref(), Some(TITLE_CANARY));
        server.await.expect("server");

        let requests = requests.lock().expect("requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(request_method(&requests[0]), "GET");
        assert_eq!(request_method(&requests[1]), "POST");
        assert_eq!(request_json_body(&requests[1]), exact_request);
    }

    #[tokio::test]
    async fn prepared_update_detects_a_normalized_true_noop_without_a_patch() {
        const TITLE_CANARY: &str = "noop-title-plaintext-canary";
        const BODY_CANARY: &str = "noop-body-plaintext-canary";
        const CHECKLIST_CANARY: &str = "noop-checklist-plaintext-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let section_id = Uuid::now_v7();
        let revision = Utc::now();
        let data_key = SymmetricKey::new([0x72; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let checklist = vec![checklist_item(CHECKLIST_CANARY)];
        let mut requested_checklist = checklist.clone();
        requested_checklist[0].title = format!("  {CHECKLIST_CANARY}  ");
        let mut current = task_fixture_with_payload(
            &list_key,
            work_list_id,
            task_id,
            TaskPayloadBody {
                title: TITLE_CANARY.to_string(),
                rich_text: plaintext_rich_text(BODY_CANARY),
                checklist: Some(checklist.clone()),
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
            },
            revision,
        );
        current.priority = Some(5);
        current.section_id = Some(section_id);
        current.delegations.push(delegation_fixture(
            task_id,
            "noop-delegation-note-ciphertext-canary",
        ));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![
                MockHttpResponse::ok_json(&work_list_fixture(work_list_id)),
                MockHttpResponse::ok_json(&TaskDetailResponse {
                    task: current,
                    comments: vec![comment_fixture(task_id, "comment-ciphertext-canary")],
                }),
            ],
        );

        let (client, context) =
            resolve_test_context(&runtime, &api_url, work_list_id, &data_key).await;
        let prepared = runtime
            .prepare_task_update_with_loaded_context(
                UpdateTaskArgs {
                    work_list_id,
                    task_id,
                    input: TaskUpdateInput {
                        title: Some(format!("  {TITLE_CANARY}  ")),
                        body: TaskFieldPatch::Set(format!("  {BODY_CANARY}  ")),
                        checklist: TaskFieldPatch::Set(requested_checklist),
                        priority: TaskFieldPatch::Set(5),
                        due_at: TaskFieldPatch::Unchanged,
                        start_at: TaskFieldPatch::Unchanged,
                        section_id: TaskFieldPatch::Set(section_id),
                    },
                    password_stdin: false,
                },
                None,
                client,
                context,
            )
            .await
            .expect("prepare no-op update");
        server.await.expect("server");

        assert_eq!(requests.lock().expect("requests").len(), 2);
        assert!(!prepared.plan.would_change);
        assert!(!prepared.plan.will_mutate);
        assert!(prepared.plan.changed_fields.is_empty());
        assert_eq!(prepared.plan.changed_field_count, 0);
        assert_eq!(prepared.plan.expected_updated_at, Some(revision));
        assert!(prepared.request.get().title_ciphertext.is_none());
        assert!(prepared.request.get().payload_ciphertext.is_none());
        assert!(prepared.request.get().priority.is_none());
        assert!(prepared.request.get().section_id.is_none());
        let plan_json = serde_json::to_string(prepared.plan()).expect("serialize plan");
        let plan_debug = format!("{:?}", prepared.plan());
        for canary in [TITLE_CANARY, BODY_CANARY, CHECKLIST_CANARY] {
            assert!(!plan_json.contains(canary));
            assert!(!plan_debug.contains(canary));
        }

        let unchanged = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            runtime.execute_prepared_task_update(prepared),
        )
        .await
        .expect("no-op execution must not wait for a PATCH")
        .expect("execute no-op");
        assert_eq!(unchanged.id, task_id);
        assert_eq!(unchanged.title.as_deref(), Some(TITLE_CANARY));
        assert_eq!(requests.lock().expect("requests").len(), 2);
    }

    #[tokio::test]
    async fn prepared_update_executes_the_exact_encrypted_request_and_revision() {
        const NEXT_TITLE: &str = "exact-update-title-canary";
        const NEXT_BODY: &str = "exact-update-body-canary";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let revision = Utc::now();
        let data_key = SymmetricKey::new([0x73; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let mut current = task_fixture(&list_key, work_list_id, task_id, "Before", false, false);
        current.updated_at = revision;
        current.priority = Some(1);
        current.section_id = Some(Uuid::now_v7());
        let mut updated = task_fixture_with_payload(
            &list_key,
            work_list_id,
            task_id,
            TaskPayloadBody {
                title: NEXT_TITLE.to_string(),
                rich_text: plaintext_rich_text(NEXT_BODY),
                checklist: None,
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
            },
            revision + TimeDelta::seconds(1),
        );
        updated.priority = Some(8);
        updated.section_id = None;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![
                MockHttpResponse::ok_json(&work_list_fixture(work_list_id)),
                MockHttpResponse::ok_json(&TaskDetailResponse {
                    task: current,
                    comments: vec![comment_fixture(
                        task_id,
                        "exact-update-comment-ciphertext-canary",
                    )],
                }),
                MockHttpResponse::ok_json(&updated),
            ],
        );

        let (client, context) =
            resolve_test_context(&runtime, &api_url, work_list_id, &data_key).await;
        let prepared = runtime
            .prepare_task_update_with_loaded_context(
                UpdateTaskArgs {
                    work_list_id,
                    task_id,
                    input: TaskUpdateInput {
                        title: Some(format!("  {NEXT_TITLE}  ")),
                        body: TaskFieldPatch::Set(format!("  {NEXT_BODY}  ")),
                        checklist: TaskFieldPatch::Unchanged,
                        priority: TaskFieldPatch::Set(8),
                        due_at: TaskFieldPatch::Unchanged,
                        start_at: TaskFieldPatch::Unchanged,
                        section_id: TaskFieldPatch::Clear,
                    },
                    password_stdin: false,
                },
                Some(revision),
                client,
                context,
            )
            .await
            .expect("prepare update");

        assert_eq!(
            prepared.plan.changed_fields,
            ["title", "body", "priority", "sectionId"]
        );
        assert_eq!(prepared.plan.expected_updated_at, Some(revision));
        assert!(prepared.plan.would_change);
        let exact_request = serde_json::to_value(prepared.request.get()).expect("prepared request");
        assert_eq!(
            exact_request.get("expectedUpdatedAt"),
            Some(&serde_json::to_value(revision).expect("revision"))
        );

        let updated = runtime
            .execute_prepared_task_update(prepared)
            .await
            .expect("execute prepared update");
        assert_eq!(updated.title.as_deref(), Some(NEXT_TITLE));
        assert_eq!(updated.priority, Some(8));
        server.await.expect("server");

        let requests = requests.lock().expect("requests");
        assert_eq!(requests.len(), 3);
        assert_eq!(request_method(&requests[0]), "GET");
        assert_eq!(request_method(&requests[1]), "GET");
        assert_eq!(request_method(&requests[2]), "PATCH");
        assert_eq!(request_json_body(&requests[2]), exact_request);
    }

    #[tokio::test]
    async fn conditional_noop_with_a_stale_revision_still_conflicts_without_a_patch() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let revision = Utc::now();
        let stale_revision = revision - TimeDelta::seconds(1);
        let data_key = SymmetricKey::new([0x74; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let mut current = task_fixture(&list_key, work_list_id, task_id, "Unchanged", false, false);
        current.updated_at = revision;
        current.delegations.push(delegation_fixture(
            task_id,
            "stale-delegation-note-ciphertext-canary",
        ));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![
                MockHttpResponse::ok_json(&work_list_fixture(work_list_id)),
                MockHttpResponse::ok_json(&TaskDetailResponse {
                    task: current,
                    comments: vec![comment_fixture(task_id, "stale-comment-ciphertext-canary")],
                }),
            ],
        );

        let (client, context) =
            resolve_test_context(&runtime, &api_url, work_list_id, &data_key).await;
        let error = match runtime
            .prepare_task_update_with_loaded_context(
                UpdateTaskArgs {
                    work_list_id,
                    task_id,
                    input: TaskUpdateInput {
                        title: Some("Unchanged".to_string()),
                        body: TaskFieldPatch::Unchanged,
                        checklist: TaskFieldPatch::Unchanged,
                        priority: TaskFieldPatch::Unchanged,
                        due_at: TaskFieldPatch::Unchanged,
                        start_at: TaskFieldPatch::Unchanged,
                        section_id: TaskFieldPatch::Unchanged,
                    },
                    password_stdin: false,
                },
                Some(stale_revision),
                client,
                context,
            )
            .await
        {
            Ok(_) => panic!("stale conditional update must conflict during preparation"),
            Err(error) => error,
        };
        assert!(matches!(error, PublicError::Conflict(_)));
        server.await.expect("server");
        assert_eq!(requests.lock().expect("requests").len(), 2);
    }

    #[tokio::test]
    async fn prepared_conditional_update_preserves_backend_conflicts() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let revision = Utc::now();
        let data_key = SymmetricKey::new([0x78; KEY_SIZE]);
        let list_key = derive_work_list_key(&data_key, &work_list_id).expect("list key");
        let mut current = task_fixture(&list_key, work_list_id, task_id, "Conflict", false, false);
        current.updated_at = revision;
        current.priority = Some(1);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_http_sequence(
            listener,
            requests.clone(),
            vec![
                MockHttpResponse::ok_json(&work_list_fixture(work_list_id)),
                MockHttpResponse::ok_json(&TaskDetailResponse {
                    task: current,
                    comments: Vec::new(),
                }),
                MockHttpResponse {
                    status: "409 Conflict",
                    body: br#"{"error":"conflict"}"#.to_vec(),
                },
            ],
        );

        let (client, context) =
            resolve_test_context(&runtime, &api_url, work_list_id, &data_key).await;
        let prepared = runtime
            .prepare_task_update_with_loaded_context(
                UpdateTaskArgs {
                    work_list_id,
                    task_id,
                    input: TaskUpdateInput {
                        title: None,
                        body: TaskFieldPatch::Unchanged,
                        checklist: TaskFieldPatch::Unchanged,
                        priority: TaskFieldPatch::Set(8),
                        due_at: TaskFieldPatch::Unchanged,
                        start_at: TaskFieldPatch::Unchanged,
                        section_id: TaskFieldPatch::Unchanged,
                    },
                    password_stdin: false,
                },
                Some(revision),
                client,
                context,
            )
            .await
            .expect("prepare conditional update");
        let exact_request = serde_json::to_value(prepared.request.get()).expect("prepared request");
        let error = runtime
            .execute_prepared_task_update(prepared)
            .await
            .expect_err("backend conflict must be preserved");
        assert_eq!(error.code(), "conflict");
        server.await.expect("server");
        let requests = requests.lock().expect("requests");
        assert_eq!(requests.len(), 3);
        assert_eq!(request_json_body(&requests[2]), exact_request);
    }

    #[test]
    fn task_preparation_cleanup_zeroizes_every_owned_secret_buffer() {
        let mut create = SensitiveCreateTaskRequest(Some(CreateTaskRequest {
            title_ciphertext: "title-ciphertext-canary".to_string(),
            title_ciphertext_proof: "title-proof-canary".to_string(),
            payload_ciphertext: "payload-ciphertext-canary".to_string(),
            payload_ciphertext_proof: "payload-proof-canary".to_string(),
            attachment_ids: Vec::new(),
            priority: None,
            due_at: None,
            start_at: None,
            section_id: None,
            idempotency_key: Some("idempotency-key-canary".to_string()),
            idempotency_commitment: Some("idempotency-commitment-canary".to_string()),
        }));
        create.zeroize();
        let create = create.get();
        assert!(create.title_ciphertext.is_empty());
        assert!(create.title_ciphertext_proof.is_empty());
        assert!(create.payload_ciphertext.is_empty());
        assert!(create.payload_ciphertext_proof.is_empty());
        assert_eq!(create.idempotency_key.as_deref(), Some(""));
        assert_eq!(create.idempotency_commitment.as_deref(), Some(""));

        let mut update = SensitiveUpdateTaskRequest(Some(UpdateTaskRequest {
            title_ciphertext: Some("update-title-ciphertext-canary".to_string()),
            title_ciphertext_proof: Some("update-title-proof-canary".to_string()),
            payload_ciphertext: Some("update-payload-ciphertext-canary".to_string()),
            payload_ciphertext_proof: Some("update-payload-proof-canary".to_string()),
            ..UpdateTaskRequest::default()
        }));
        update.zeroize();
        let update = update.get();
        assert_eq!(update.title_ciphertext.as_deref(), Some(""));
        assert_eq!(update.title_ciphertext_proof.as_deref(), Some(""));
        assert_eq!(update.payload_ciphertext.as_deref(), Some(""));
        assert_eq!(update.payload_ciphertext_proof.as_deref(), Some(""));

        let mut idempotency_key =
            SensitiveOptionalString(Some("early-error-idempotency-key-canary".to_string()));
        idempotency_key.zeroize();
        assert_eq!(idempotency_key.0.as_deref(), Some(""));

        let list_key = SymmetricKey::new([0x75; KEY_SIZE]);
        let mut task = task_fixture(
            &list_key,
            Uuid::now_v7(),
            Uuid::now_v7(),
            "cleanup",
            false,
            false,
        );
        task.delegations.push(delegation_fixture(
            task.id,
            "delegation-note-ciphertext-canary",
        ));
        zeroize_task_response(&mut task);
        assert!(task.title_ciphertext.is_empty());
        assert!(task.payload_ciphertext.is_empty());
        assert_eq!(task.delegations[0].note_ciphertext.as_deref(), Some(""));
        let mut comments = vec![comment_fixture(task.id, "comment-body-ciphertext-canary")];
        zeroize_task_comments(&mut comments);
        assert!(comments[0].body_ciphertext.is_empty());
    }

    #[test]
    fn task_change_commitments_are_keyed_and_project_scoped() {
        let semantics = br#"{"title":"dictionary-attack-canary"}"#;
        let first = task_mutation_change_commitment(
            TASK_CREATE_ACTION,
            semantics,
            &SymmetricKey::new([0x76; KEY_SIZE]),
        )
        .expect("first commitment");
        let repeated = task_mutation_change_commitment(
            TASK_CREATE_ACTION,
            semantics,
            &SymmetricKey::new([0x76; KEY_SIZE]),
        )
        .expect("repeated commitment");
        let other_project = task_mutation_change_commitment(
            TASK_CREATE_ACTION,
            semantics,
            &SymmetricKey::new([0x77; KEY_SIZE]),
        )
        .expect("other project commitment");

        assert_eq!(first, repeated);
        assert_ne!(first, other_project);
        assert!(!first.contains("dictionary-attack-canary"));
    }

    #[test]
    fn batch_task_create_idempotency_derivation_is_opaque_scoped_and_explicit_safe() {
        let digest = [0xcd; 32];
        let operation_id = "runtime-batch-operation-canary";
        let derivation = TaskCreateIdempotencyDerivation::new(digest, operation_id);
        let first_project = Uuid::from_u128(1);
        let second_project = Uuid::from_u128(2);
        let data_key = SymmetricKey::new([0x52; KEY_SIZE]);
        let first_key = derive_work_list_key(&data_key, &first_project).expect("first list key");
        let second_key = derive_work_list_key(&data_key, &second_project).expect("second list key");
        let mut input = TaskCreateInput {
            title: "Task".to_string(),
            body: None,
            checklist: None,
            priority: None,
            due_at: None,
            start_at: None,
            section_id: None,
            idempotency_key: None,
        };

        let first =
            task_create_idempotency_key(&input, Some(&derivation), &first_project, &first_key)
                .expect("first derivation")
                .expect("derived key");
        let retry =
            task_create_idempotency_key(&input, Some(&derivation), &first_project, &first_key)
                .expect("retry derivation")
                .expect("derived key");
        let other_project =
            task_create_idempotency_key(&input, Some(&derivation), &second_project, &second_key)
                .expect("other project derivation")
                .expect("derived key");

        assert_eq!(first, retry);
        assert_ne!(first, other_project);
        assert!(!first.contains(operation_id));
        assert!(!first.contains(&"cd".repeat(32)));
        validate_idempotency_key(&first).expect("server-compatible key");

        input.idempotency_key = Some("user:key-unchanged".to_string());
        assert_eq!(
            task_create_idempotency_key(&input, Some(&derivation), &first_project, &first_key)
                .expect("explicit key"),
            Some("user:key-unchanged".to_string())
        );
    }

    struct MockHttpResponse {
        status: &'static str,
        body: Vec<u8>,
    }

    impl MockHttpResponse {
        fn ok_json(value: &impl Serialize) -> Self {
            Self {
                status: "200 OK",
                body: serde_json::to_vec(value).expect("mock response JSON"),
            }
        }
    }

    fn spawn_http_sequence(
        listener: tokio::net::TcpListener,
        requests: Arc<Mutex<Vec<Vec<u8>>>>,
        responses: Vec<MockHttpResponse>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            for response in responses {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_http_request(&mut stream).await;
                requests.lock().expect("requests").push(request);
                write_http_response_with_status(&mut stream, response.status, &response.body).await;
            }
        })
    }

    async fn resolve_test_context(
        runtime: &RuntimeClient,
        api_url: &str,
        work_list_id: Uuid,
        data_key: &SymmetricKey,
    ) -> (PublicApiClient, WorkListContext) {
        let client = PublicApiClient::with_credentials(api_url, test_credentials(api_url))
            .expect("API client");
        let session = runtime
            .project_task_session_with_client_and_data_key(work_list_id, client, data_key)
            .await
            .expect("resolved project context");
        (session.client, session.context)
    }

    fn checklist_item(title: &str) -> ChecklistItemPayload {
        ChecklistItemPayload {
            id: Uuid::now_v7().to_string(),
            title: title.to_string(),
            is_done: false,
            completed_at: None,
            assignee_user_ids: None,
        }
    }

    fn task_fixture_with_payload(
        list_key: &SymmetricKey,
        work_list_id: Uuid,
        id: Uuid,
        body: TaskPayloadBody,
        updated_at: DateTime<Utc>,
    ) -> TaskResponse {
        let payload = encrypt_task_payload(&build_task_payload_envelope(body, 1), list_key)
            .expect("task payload");
        TaskResponse {
            id,
            work_list_id,
            created_by_membership_id: Uuid::now_v7(),
            title_ciphertext: String::new(),
            payload_ciphertext: payload.base64,
            section_id: None,
            priority: None,
            position: id.simple().to_string(),
            due_at: None,
            start_at: None,
            completed_at: None,
            archived_at: None,
            is_completed: false,
            recurrence_id: None,
            recurrence_schedule: None,
            recurrence_iteration: None,
            materialized_at: None,
            created_at: updated_at,
            updated_at,
            comment_count: 0,
            reference_number: None,
            delegations: Vec::new(),
        }
    }

    fn comment_fixture(task_id: Uuid, body_ciphertext: &str) -> CommentResponse {
        let now = Utc::now();
        CommentResponse {
            id: Uuid::now_v7(),
            task_id,
            author_membership_id: Uuid::now_v7(),
            body_ciphertext: body_ciphertext.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn delegation_fixture(task_id: Uuid, note_ciphertext: &str) -> DelegationResponse {
        let now = Utc::now();
        DelegationResponse {
            id: Uuid::now_v7(),
            task_id,
            membership_id: Uuid::now_v7(),
            role: "watcher".to_string(),
            status: "accepted".to_string(),
            note_ciphertext: Some(note_ciphertext.to_string()),
            created_at: now,
            updated_at: now,
        }
    }

    fn task_titles(tasks: &[AgentTaskSummary]) -> Vec<&str> {
        tasks
            .iter()
            .map(|task| task.title.as_deref().expect("decrypted task title"))
            .collect()
    }

    fn task_fixture(
        list_key: &SymmetricKey,
        work_list_id: Uuid,
        id: Uuid,
        title: &str,
        is_completed: bool,
        is_archived: bool,
    ) -> TaskResponse {
        let payload = encrypt_task_payload(
            &build_task_payload_envelope(
                TaskPayloadBody {
                    title: title.to_string(),
                    rich_text: None,
                    checklist: None,
                    attachments: None,
                    references: None,
                    mentions: None,
                    client_meta: None,
                    recurrence_state: None,
                },
                1,
            ),
            list_key,
        )
        .expect("task payload");
        let now = Utc::now();
        TaskResponse {
            id,
            work_list_id,
            created_by_membership_id: Uuid::now_v7(),
            title_ciphertext: String::new(),
            payload_ciphertext: payload.base64,
            section_id: None,
            priority: None,
            position: id.simple().to_string(),
            due_at: None,
            start_at: None,
            completed_at: is_completed.then_some(now),
            archived_at: is_archived.then_some(now),
            is_completed,
            recurrence_id: None,
            recurrence_schedule: None,
            recurrence_iteration: None,
            materialized_at: None,
            created_at: now,
            updated_at: now,
            comment_count: 0,
            reference_number: None,
            delegations: Vec::new(),
        }
    }

    fn work_list_fixture(id: Uuid) -> WorkListDetailResponse {
        let now = Utc::now();
        WorkListDetailResponse {
            work_list: WorkListResponse {
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
                    user_email: "agent@example.com".to_string(),
                    user_name: "Agent".to_string(),
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
            },
            members: Vec::new(),
        }
    }

    fn test_credentials(api_url: &str) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + TimeDelta::hours(1),
            refresh_expires_at: Utc::now() + TimeDelta::hours(2),
            user_id: Uuid::now_v7(),
            email: "agent@example.com".to_string(),
            data_key_ciphertext: STANDARD_NO_PAD.encode(
                SealedPayload::new(vec![0x51; 48])
                    .to_bytes()
                    .expect("encode data-key binding fixture"),
            ),
        }
    }

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).await.expect("request bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4)
            .unwrap_or(request.len());
        let content_length = String::from_utf8_lossy(&request[..header_end])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = stream.read(&mut buffer).await.expect("request body bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        request
    }

    fn request_target(request: &[u8]) -> String {
        String::from_utf8_lossy(request)
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("request target")
            .to_string()
    }

    fn request_method(request: &[u8]) -> String {
        String::from_utf8_lossy(request)
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().next())
            .expect("request method")
            .to_string()
    }

    fn request_json_body(request: &[u8]) -> serde_json::Value {
        let body_start = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4)
            .expect("request body separator");
        serde_json::from_slice(&request[body_start..]).expect("request JSON body")
    }

    async fn write_http_response(stream: &mut tokio::net::TcpStream, body: &[u8]) {
        write_http_response_with_status(stream, "200 OK", body).await;
    }

    async fn write_http_response_with_status(
        stream: &mut tokio::net::TcpStream,
        status: &str,
        body: &[u8],
    ) {
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(head.as_bytes())
            .await
            .expect("response head");
        stream.write_all(body).await.expect("response body");
    }
}
