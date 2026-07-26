use crate::client::{RuntimeClient, WorkListContext};
use crate::models::{
    AgentAttachment, AgentComment, AgentDelegation, AgentMembership, AgentTaskSummary,
    AgentWorkListDetail, AgentWorkListSummary, ReadError,
};
use chrono::{DateTime, Utc};
use sealtask_client_api::{
    CommentResponse, MembershipResponse, MyTaskResponse, TaskReferenceSchemeResponse, TaskResponse,
    WorkListDetailResponse, WorkListResponse,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    CommentPayloadBody, FlexibleValue, SymmetricKey, TASK_REFERENCE_REVISION_MAX, TaskPayloadBody,
    TaskPayloadRichText, TaskReferenceSchemeV1, decode_sealed_blob, decrypt_comment_payload,
    decrypt_task_payload, decrypt_task_reference_scheme, decrypt_text_value, decrypt_work_list_key,
    decrypt_work_list_payload, derive_work_list_key, flexible_value_to_json,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug)]
struct TaskProjectionMetadata {
    id: Uuid,
    work_list_id: Uuid,
    work_list_title: Option<String>,
    work_list_timezone: Option<String>,
    created_by_membership_id: Uuid,
    section_id: Option<Uuid>,
    priority: Option<i8>,
    position: Option<String>,
    due_at: Option<DateTime<Utc>>,
    start_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    is_completed: bool,
    recurrence_id: Option<Uuid>,
    recurrence_schedule: Option<String>,
    recurrence_iteration: Option<i64>,
    materialized_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    comment_count: i64,
    reference_number: Option<i64>,
    reference: Option<String>,
}

#[derive(Debug)]
struct TaskProjectionInput<'a> {
    metadata: TaskProjectionMetadata,
    delegations: Vec<sealtask_client_api::DelegationResponse>,
    title_ciphertext: &'a str,
    payload_ciphertext: &'a str,
    list_key: Option<&'a SymmetricKey>,
    inherited_error: Option<ReadError>,
}

pub(crate) fn project_attachments(
    values: Option<Vec<FlexibleValue>>,
) -> PublicResult<Option<Vec<AgentAttachment>>> {
    values
        .map(|values| values.into_iter().map(project_attachment).collect())
        .transpose()
}

fn project_attachment(value: FlexibleValue) -> PublicResult<AgentAttachment> {
    let FlexibleValue::Map(entries) = value else {
        return Err(PublicError::validation("attachment must be an object"));
    };

    let mut id = None;
    let mut file_name = None;
    let mut content_type = None;
    let mut size_bytes = None;
    let mut blob_key = None;

    for (key, value) in entries {
        match flexible_key_to_string(key).as_str() {
            "id" => id = Some(value),
            "file_name" => file_name = Some(value),
            "content_type" => content_type = Some(value),
            "size_bytes" => size_bytes = Some(value),
            "blob_key" => blob_key = Some(value),
            _ => {}
        }
    }

    Ok(AgentAttachment {
        id: parse_attachment_uuid(id, "attachment.id")?,
        file_name: parse_attachment_text(file_name, "attachment.file_name")?,
        content_type: parse_attachment_text(content_type, "attachment.content_type")?,
        size_bytes: parse_attachment_u64(size_bytes, "attachment.size_bytes")?,
        blob_key: parse_attachment_bytes(blob_key, "attachment.blob_key")?,
    })
}

fn parse_attachment_uuid(value: Option<FlexibleValue>, field: &str) -> PublicResult<Uuid> {
    match value {
        Some(FlexibleValue::Text(value)) => Uuid::parse_str(&value).map_err(|err| {
            PublicError::validation(format!("{field} must be a UUID string: {err}"))
        }),
        Some(FlexibleValue::Bytes(value)) if value.len() == 16 => Uuid::from_slice(&value)
            .map_err(|err| PublicError::validation(format!("{field} must be a UUID: {err}"))),
        Some(_) => Err(PublicError::validation(format!(
            "{field} must be a UUID string or 16-byte UUID"
        ))),
        None => Err(PublicError::validation(format!("{field} is required"))),
    }
}

fn parse_attachment_text(value: Option<FlexibleValue>, field: &str) -> PublicResult<String> {
    match value {
        Some(FlexibleValue::Text(value)) if !value.trim().is_empty() => Ok(value),
        Some(FlexibleValue::Text(_)) => {
            Err(PublicError::validation(format!("{field} cannot be empty")))
        }
        Some(_) => Err(PublicError::validation(format!("{field} must be text"))),
        None => Err(PublicError::validation(format!("{field} is required"))),
    }
}

fn parse_attachment_u64(value: Option<FlexibleValue>, field: &str) -> PublicResult<u64> {
    let Some(value) = value else {
        return Err(PublicError::validation(format!("{field} is required")));
    };

    match value {
        FlexibleValue::Integer(value) => u64::try_from(i128::from(value)).map_err(|_| {
            PublicError::validation(format!("{field} must be a non-negative integer"))
        }),
        _ => Err(PublicError::validation(format!(
            "{field} must be a non-negative integer"
        ))),
    }
}

fn parse_attachment_bytes(value: Option<FlexibleValue>, field: &str) -> PublicResult<Vec<u8>> {
    match value {
        Some(FlexibleValue::Bytes(value)) if !value.is_empty() => Ok(value),
        Some(FlexibleValue::Bytes(_)) => {
            Err(PublicError::validation(format!("{field} cannot be empty")))
        }
        Some(_) => Err(PublicError::validation(format!("{field} must be bytes"))),
        None => Err(PublicError::validation(format!("{field} is required"))),
    }
}

fn flexible_key_to_string(value: FlexibleValue) -> String {
    match value {
        FlexibleValue::Text(value) => value,
        other => flexible_value_to_json(other).to_string(),
    }
}

impl RuntimeClient {
    pub(crate) fn build_work_list_contexts(
        &self,
        work_lists: &[WorkListResponse],
        scheme_histories: &HashMap<Uuid, Vec<TaskReferenceSchemeResponse>>,
        data_key: Option<&SymmetricKey>,
    ) -> HashMap<Uuid, WorkListContext> {
        work_lists
            .iter()
            .map(|work_list| {
                (
                    work_list.id,
                    self.context_from_work_list_response(
                        work_list,
                        scheme_histories
                            .get(&work_list.id)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                        data_key,
                    ),
                )
            })
            .collect()
    }

    pub(crate) fn context_from_work_list_detail(
        &self,
        work_list: &WorkListDetailResponse,
        scheme_history: &[TaskReferenceSchemeResponse],
        data_key: Option<&SymmetricKey>,
    ) -> WorkListContext {
        self.context_from_work_list_response(&work_list.work_list, scheme_history, data_key)
    }

    fn context_from_work_list_response(
        &self,
        work_list: &WorkListResponse,
        scheme_history: &[TaskReferenceSchemeResponse],
        data_key: Option<&SymmetricKey>,
    ) -> WorkListContext {
        match data_key {
            Some(data_key) => match resolve_list_key(
                data_key,
                work_list.id,
                &work_list.membership.work_list_key_ciphertext,
            ) {
                Ok(list_key) => {
                    let task_reference_schemes = decode_task_reference_schemes(
                        &list_key,
                        work_list.id,
                        work_list.task_references_enabled_at.is_some(),
                        work_list.current_task_reference_scheme_revision,
                        work_list.current_task_reference_scheme_revision_id,
                        scheme_history,
                    );
                    let payload =
                        decode_work_list_payload_value(&list_key, &work_list.payload_ciphertext);
                    let title = payload
                        .as_ref()
                        .ok()
                        .and_then(extract_work_list_title)
                        .or_else(|| decode_text_fallback(&work_list.title_ciphertext));
                    WorkListContext {
                        work_list_title: title,
                        work_list_timezone: work_list.timezone.clone(),
                        list_key: Some(list_key),
                        task_reference_schemes,
                        current_task_reference_scheme_revision: work_list
                            .current_task_reference_scheme_revision,
                        current_task_reference_scheme_revision_id: work_list
                            .current_task_reference_scheme_revision_id,
                        read_error: payload
                            .err()
                            .map(|err| make_read_error("work_list_payload", err)),
                    }
                }
                Err(err) => WorkListContext {
                    work_list_title: decode_text_fallback(&work_list.title_ciphertext),
                    work_list_timezone: work_list.timezone.clone(),
                    list_key: None,
                    task_reference_schemes: Vec::new(),
                    current_task_reference_scheme_revision: work_list
                        .current_task_reference_scheme_revision,
                    current_task_reference_scheme_revision_id: work_list
                        .current_task_reference_scheme_revision_id,
                    read_error: Some(make_read_error("work_list_key", err)),
                },
            },
            None => WorkListContext {
                work_list_title: decode_text_fallback(&work_list.title_ciphertext),
                work_list_timezone: work_list.timezone.clone(),
                list_key: None,
                task_reference_schemes: Vec::new(),
                current_task_reference_scheme_revision: work_list
                    .current_task_reference_scheme_revision,
                current_task_reference_scheme_revision_id: work_list
                    .current_task_reference_scheme_revision_id,
                read_error: Some(ReadError {
                    code: "data_key_missing".to_string(),
                    message: "could not load data key for work list decryption".to_string(),
                }),
            },
        }
    }

    pub(crate) fn project_work_list_summary(
        &self,
        work_list: WorkListResponse,
        data_key: Option<&SymmetricKey>,
    ) -> AgentWorkListSummary {
        let membership = project_membership(&work_list.membership);
        let (title, description, payload, read_error) = match data_key {
            Some(data_key) => match resolve_list_key(
                data_key,
                work_list.id,
                &work_list.membership.work_list_key_ciphertext,
            ) {
                Ok(list_key) => {
                    match decode_work_list_payload_value(&list_key, &work_list.payload_ciphertext) {
                        Ok(payload) => (
                            extract_work_list_title(&payload)
                                .or_else(|| decode_text_fallback(&work_list.title_ciphertext)),
                            extract_work_list_description(&payload).or_else(|| {
                                work_list
                                    .description_ciphertext
                                    .as_deref()
                                    .and_then(decode_text_fallback)
                            }),
                            Some(payload),
                            None,
                        ),
                        Err(err) => {
                            let (title, description) = decode_work_list_text_fallbacks(&work_list);
                            (
                                title,
                                description,
                                None,
                                Some(make_read_error("work_list_payload", err)),
                            )
                        }
                    }
                }
                Err(err) => {
                    let (title, description) = decode_work_list_text_fallbacks(&work_list);
                    (
                        title,
                        description,
                        None,
                        Some(make_read_error("work_list_key", err)),
                    )
                }
            },
            None => {
                let (title, description) = decode_work_list_text_fallbacks(&work_list);
                (
                    title,
                    description,
                    None,
                    Some(ReadError {
                        code: "data_key_missing".to_string(),
                        message: "could not load data key for work list decryption".to_string(),
                    }),
                )
            }
        };

        AgentWorkListSummary {
            id: work_list.id,
            owner_user_id: work_list.owner_user_id,
            workspace_id: work_list.workspace_id,
            timezone: work_list.timezone,
            section_snapshots: work_list.section_snapshots,
            created_at: work_list.created_at,
            updated_at: work_list.updated_at,
            archived_at: work_list.archived_at,
            task_references_enabled_at: work_list.task_references_enabled_at,
            current_task_reference_scheme_revision: work_list
                .current_task_reference_scheme_revision,
            current_task_reference_scheme_revision_id: work_list
                .current_task_reference_scheme_revision_id,
            membership,
            title,
            description,
            payload,
            read_error,
        }
    }

    pub(crate) fn project_work_list_detail(
        &self,
        work_list: WorkListDetailResponse,
        data_key: Option<&SymmetricKey>,
    ) -> AgentWorkListDetail {
        let members = work_list.members.iter().map(project_membership).collect();
        AgentWorkListDetail {
            work_list: self.project_work_list_summary(work_list.work_list, data_key),
            members,
        }
    }

    pub(crate) fn project_task_summary(
        &self,
        task: TaskResponse,
        context: Option<&WorkListContext>,
    ) -> AgentTaskSummary {
        project_task(TaskProjectionInput {
            metadata: TaskProjectionMetadata {
                id: task.id,
                work_list_id: task.work_list_id,
                work_list_title: context.and_then(|item| item.work_list_title.clone()),
                work_list_timezone: context.map(|item| item.work_list_timezone.clone()),
                created_by_membership_id: task.created_by_membership_id,
                section_id: task.section_id,
                priority: task.priority,
                position: Some(task.position),
                due_at: task.due_at,
                start_at: task.start_at,
                completed_at: task.completed_at,
                archived_at: task.archived_at,
                is_completed: task.is_completed,
                recurrence_id: task.recurrence_id,
                recurrence_schedule: task.recurrence_schedule,
                recurrence_iteration: task.recurrence_iteration,
                materialized_at: task.materialized_at,
                created_at: task.created_at,
                updated_at: task.updated_at,
                comment_count: task.comment_count,
                reference_number: task.reference_number,
                reference: task.reference_number.and_then(|number| {
                    context
                        .and_then(WorkListContext::current_task_reference_scheme)
                        .and_then(|scheme| scheme.format_reference(number).ok())
                }),
            },
            delegations: task.delegations,
            title_ciphertext: &task.title_ciphertext,
            payload_ciphertext: &task.payload_ciphertext,
            list_key: context.and_then(|item| item.list_key.as_ref()),
            inherited_error: context.and_then(|item| item.read_error.clone()),
        })
    }

    pub(crate) fn project_my_task_summary(
        &self,
        task: MyTaskResponse,
        context: Option<&WorkListContext>,
    ) -> AgentTaskSummary {
        let work_list_title = context
            .and_then(|item| item.work_list_title.clone())
            .or_else(|| decode_text_fallback(&task.work_list_title_ciphertext));
        let list_key = context.and_then(|item| item.list_key.as_ref());
        let read_error = context.and_then(|item| item.read_error.clone());

        project_task(TaskProjectionInput {
            metadata: TaskProjectionMetadata {
                id: task.id,
                work_list_id: task.work_list_id,
                work_list_title,
                work_list_timezone: context.map(|item| item.work_list_timezone.clone()),
                created_by_membership_id: task.created_by_membership_id,
                section_id: task.section_id,
                priority: task.priority,
                position: None,
                due_at: task.due_at,
                start_at: task.start_at,
                completed_at: task.completed_at,
                archived_at: None,
                is_completed: task.is_completed,
                recurrence_id: None,
                recurrence_schedule: None,
                recurrence_iteration: None,
                materialized_at: None,
                created_at: task.created_at,
                updated_at: task.updated_at,
                comment_count: task.comment_count,
                reference_number: task.reference_number,
                reference: task.reference_number.and_then(|number| {
                    context
                        .and_then(WorkListContext::current_task_reference_scheme)
                        .and_then(|scheme| scheme.format_reference(number).ok())
                }),
            },
            delegations: task.delegations,
            title_ciphertext: &task.title_ciphertext,
            payload_ciphertext: &task.payload_ciphertext,
            list_key,
            inherited_error: read_error,
        })
    }

    pub(crate) fn project_comment(
        &self,
        comment: CommentResponse,
        list_key: Option<&SymmetricKey>,
    ) -> AgentComment {
        match list_key {
            Some(list_key) => match decode_sealed_blob(&comment.body_ciphertext)
                .and_then(|bytes| decrypt_comment_payload(list_key, &bytes))
            {
                Ok(payload) => {
                    let CommentPayloadBody {
                        content,
                        mentions,
                        attachments,
                        client_meta,
                    } = payload.body;
                    let (attachments, read_error) = match project_attachments(attachments) {
                        Ok(attachments) => (attachments, None),
                        Err(err) => (None, Some(make_read_error("comment_attachments", err))),
                    };

                    AgentComment {
                        id: comment.id,
                        task_id: comment.task_id,
                        author_membership_id: comment.author_membership_id,
                        body_markdown: rich_text_to_markdown(&content),
                        content: Some(content),
                        mentions,
                        attachments,
                        client_meta: client_meta.map(flexible_value_to_json),
                        created_at: comment.created_at,
                        updated_at: comment.updated_at,
                        read_error,
                    }
                }
                Err(err) => AgentComment {
                    id: comment.id,
                    task_id: comment.task_id,
                    author_membership_id: comment.author_membership_id,
                    body_markdown: None,
                    content: None,
                    mentions: None,
                    attachments: None,
                    client_meta: None,
                    created_at: comment.created_at,
                    updated_at: comment.updated_at,
                    read_error: Some(make_read_error("comment_payload", err)),
                },
            },
            None => AgentComment {
                id: comment.id,
                task_id: comment.task_id,
                author_membership_id: comment.author_membership_id,
                body_markdown: None,
                content: None,
                mentions: None,
                attachments: None,
                client_meta: None,
                created_at: comment.created_at,
                updated_at: comment.updated_at,
                read_error: Some(ReadError {
                    code: "work_list_key_missing".to_string(),
                    message: "could not resolve work list key for comment decryption".to_string(),
                }),
            },
        }
    }
}

fn project_task(input: TaskProjectionInput<'_>) -> AgentTaskSummary {
    let TaskProjectionInput {
        metadata,
        delegations,
        title_ciphertext,
        payload_ciphertext,
        list_key,
        inherited_error,
    } = input;
    let projected_delegations = delegations.into_iter().map(project_delegation).collect();

    match list_key {
        Some(list_key) => match decode_sealed_blob(payload_ciphertext)
            .and_then(|bytes| decrypt_task_payload(list_key, &bytes))
        {
            Ok(payload) => {
                let TaskPayloadBody {
                    title,
                    rich_text,
                    checklist,
                    attachments,
                    references,
                    mentions,
                    client_meta,
                    recurrence_state,
                } = payload.body;
                let (attachments, read_error) = match project_attachments(attachments) {
                    Ok(attachments) => (attachments, None),
                    Err(err) => (None, Some(make_read_error("task_attachments", err))),
                };

                AgentTaskSummary {
                    id: metadata.id,
                    work_list_id: metadata.work_list_id,
                    work_list_title: metadata.work_list_title,
                    work_list_timezone: metadata.work_list_timezone,
                    created_by_membership_id: metadata.created_by_membership_id,
                    section_id: metadata.section_id,
                    priority: metadata.priority,
                    position: metadata.position,
                    due_at: metadata.due_at,
                    start_at: metadata.start_at,
                    completed_at: metadata.completed_at,
                    archived_at: metadata.archived_at,
                    is_completed: metadata.is_completed,
                    recurrence_id: metadata.recurrence_id,
                    recurrence_schedule: metadata.recurrence_schedule,
                    recurrence_iteration: metadata.recurrence_iteration,
                    materialized_at: metadata.materialized_at,
                    created_at: metadata.created_at,
                    updated_at: metadata.updated_at,
                    comment_count: metadata.comment_count,
                    reference_number: metadata.reference_number,
                    reference: metadata.reference,
                    title: Some(title),
                    body_markdown: rich_text.as_ref().and_then(rich_text_to_markdown),
                    body_rich_text: rich_text,
                    checklist,
                    attachments,
                    references: references
                        .map(|values| values.into_iter().map(flexible_value_to_json).collect()),
                    mentions,
                    client_meta: client_meta.map(flexible_value_to_json),
                    recurrence_state: recurrence_state.map(flexible_value_to_json),
                    delegations: projected_delegations,
                    read_error,
                }
            }
            Err(err) => AgentTaskSummary {
                id: metadata.id,
                work_list_id: metadata.work_list_id,
                work_list_title: metadata.work_list_title,
                work_list_timezone: metadata.work_list_timezone,
                created_by_membership_id: metadata.created_by_membership_id,
                section_id: metadata.section_id,
                priority: metadata.priority,
                position: metadata.position,
                due_at: metadata.due_at,
                start_at: metadata.start_at,
                completed_at: metadata.completed_at,
                archived_at: metadata.archived_at,
                is_completed: metadata.is_completed,
                recurrence_id: metadata.recurrence_id,
                recurrence_schedule: metadata.recurrence_schedule,
                recurrence_iteration: metadata.recurrence_iteration,
                materialized_at: metadata.materialized_at,
                created_at: metadata.created_at,
                updated_at: metadata.updated_at,
                comment_count: metadata.comment_count,
                reference_number: metadata.reference_number,
                reference: metadata.reference,
                title: decode_text_fallback(title_ciphertext),
                body_markdown: None,
                body_rich_text: None,
                checklist: None,
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
                delegations: projected_delegations,
                read_error: Some(make_read_error("task_payload", err)),
            },
        },
        None => AgentTaskSummary {
            id: metadata.id,
            work_list_id: metadata.work_list_id,
            work_list_title: metadata.work_list_title,
            work_list_timezone: metadata.work_list_timezone,
            created_by_membership_id: metadata.created_by_membership_id,
            section_id: metadata.section_id,
            priority: metadata.priority,
            position: metadata.position,
            due_at: metadata.due_at,
            start_at: metadata.start_at,
            completed_at: metadata.completed_at,
            archived_at: metadata.archived_at,
            is_completed: metadata.is_completed,
            recurrence_id: metadata.recurrence_id,
            recurrence_schedule: metadata.recurrence_schedule,
            recurrence_iteration: metadata.recurrence_iteration,
            materialized_at: metadata.materialized_at,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            comment_count: metadata.comment_count,
            reference_number: metadata.reference_number,
            reference: metadata.reference,
            title: decode_text_fallback(title_ciphertext),
            body_markdown: None,
            body_rich_text: None,
            checklist: None,
            attachments: None,
            references: None,
            mentions: None,
            client_meta: None,
            recurrence_state: None,
            delegations: projected_delegations,
            read_error: Some(inherited_error.unwrap_or(ReadError {
                code: "work_list_key_missing".to_string(),
                message: "could not resolve work list key for task decryption".to_string(),
            })),
        },
    }
}

fn project_membership(membership: &MembershipResponse) -> AgentMembership {
    AgentMembership {
        id: membership.id,
        user_id: membership.user_id,
        user_email: membership.user_email.clone(),
        user_name: membership.user_name.clone(),
        user_avatar_color: membership.user_avatar_color.clone(),
        role: membership.role.clone(),
        status: membership.status.clone(),
        expires_at: membership.expires_at,
        joined_at: membership.joined_at,
    }
}

fn project_delegation(delegation: sealtask_client_api::DelegationResponse) -> AgentDelegation {
    AgentDelegation {
        id: delegation.id,
        task_id: delegation.task_id,
        membership_id: delegation.membership_id,
        role: delegation.role,
        status: delegation.status,
        note_present: delegation.note_ciphertext.is_some(),
        created_at: delegation.created_at,
        updated_at: delegation.updated_at,
    }
}

fn resolve_list_key(
    data_key: &SymmetricKey,
    work_list_id: Uuid,
    membership_ciphertext: &str,
) -> PublicResult<SymmetricKey> {
    if membership_ciphertext.trim().is_empty() {
        return derive_work_list_key(data_key, &work_list_id);
    }

    let work_list_key_bytes = decode_sealed_blob(membership_ciphertext)?;
    decrypt_work_list_key(data_key, &work_list_key_bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskReferenceSchemeHistoryMetadata {
    pub(crate) current_revision: i64,
    pub(crate) ordinary_revision_count: usize,
    pub(crate) repair_revision_count: usize,
}

pub(crate) fn validate_task_reference_scheme_history_metadata(
    work_list_id: Uuid,
    references_enabled: bool,
    current_revision: Option<i64>,
    current_revision_id: Option<Uuid>,
    responses: &[TaskReferenceSchemeResponse],
) -> Option<TaskReferenceSchemeHistoryMetadata> {
    let (Some(current_revision), Some(current_revision_id)) =
        (current_revision, current_revision_id)
    else {
        return None;
    };
    if !references_enabled
        || responses.is_empty()
        || responses.len() > usize::try_from(TASK_REFERENCE_REVISION_MAX).unwrap_or(usize::MAX)
        || current_revision != i64::try_from(responses.len()).unwrap_or(i64::MAX)
        || !(1..=TASK_REFERENCE_REVISION_MAX).contains(&current_revision)
        || responses
            .iter()
            .any(|response| response.work_list_id != work_list_id)
    {
        return None;
    }

    let mut ordered = responses.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|response| response.revision);
    if ordered.first().is_none_or(|response| response.is_repair) {
        return None;
    }
    let mut response_ids = HashSet::with_capacity(responses.len());
    let mut ordinary_revision_count = 0;
    let mut repair_revision_count = 0;
    for (expected_revision, response) in (1..=current_revision).zip(ordered.iter().copied()) {
        if response.revision != expected_revision
            || !response_ids.insert(response.scheme_revision_id)
        {
            return None;
        }
        if response.is_repair {
            repair_revision_count += 1;
        } else {
            ordinary_revision_count += 1;
        }
        match (
            response.quarantined_at.as_ref(),
            response.quarantined_by_membership_id,
        ) {
            (None, None) => {}
            (Some(quarantined_at), Some(_))
                if response.revision < current_revision
                    && response
                        .retired_at
                        .as_ref()
                        .is_some_and(|retired_at| quarantined_at >= retired_at) => {}
            _ => return None,
        }
    }
    if ordinary_revision_count
        > usize::try_from(sealtask_client_crypto::TASK_REFERENCE_ORDINARY_REVISION_MAX)
            .unwrap_or(usize::MAX)
        || repair_revision_count
            > usize::try_from(sealtask_client_crypto::TASK_REFERENCE_REPAIR_REVISION_MAX)
                .unwrap_or(usize::MAX)
    {
        return None;
    }

    let active = responses
        .iter()
        .filter(|response| response.retired_at.is_none())
        .collect::<Vec<_>>();
    if active.len() != 1
        || active[0].revision != current_revision
        || active[0].scheme_revision_id != current_revision_id
        || active[0].quarantined_at.is_some()
        || active[0].quarantined_by_membership_id.is_some()
    {
        return None;
    }

    Some(TaskReferenceSchemeHistoryMetadata {
        current_revision,
        ordinary_revision_count,
        repair_revision_count,
    })
}

fn decode_task_reference_schemes(
    list_key: &SymmetricKey,
    work_list_id: Uuid,
    references_enabled: bool,
    current_revision: Option<i64>,
    current_revision_id: Option<Uuid>,
    responses: &[TaskReferenceSchemeResponse],
) -> Vec<TaskReferenceSchemeV1> {
    let Some(metadata) = validate_task_reference_scheme_history_metadata(
        work_list_id,
        references_enabled,
        current_revision,
        current_revision_id,
        responses,
    ) else {
        return Vec::new();
    };

    let mut schemes = Vec::with_capacity(responses.len());
    for response in responses
        .iter()
        .filter(|response| response.quarantined_at.is_none())
    {
        let Ok(bytes) = decode_sealed_blob(&response.payload_ciphertext) else {
            return Vec::new();
        };
        let Ok(scheme) = decrypt_task_reference_scheme(
            list_key,
            &bytes,
            work_list_id,
            response.scheme_revision_id,
            response.revision,
        ) else {
            return Vec::new();
        };
        schemes.push(scheme);
    }
    schemes.sort_by_key(|scheme| scheme.revision);
    if schemes.last().is_none_or(|scheme| {
        scheme.revision != metadata.current_revision
            || Some(scheme.scheme_revision_id) != current_revision_id
    }) {
        return Vec::new();
    }
    schemes
}

fn decode_work_list_payload_value(
    list_key: &SymmetricKey,
    payload_ciphertext: &str,
) -> PublicResult<Value> {
    let payload_bytes = decode_sealed_blob(payload_ciphertext)?;
    let payload: FlexibleValue = decrypt_work_list_payload(list_key, &payload_bytes)?;
    Ok(flexible_value_to_json(payload))
}

fn extract_work_list_title(payload: &Value) -> Option<String> {
    payload
        .get("body")
        .and_then(|body| body.get("title"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn extract_work_list_description(payload: &Value) -> Option<String> {
    payload
        .get("body")
        .and_then(|body| body.get("description"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn decode_text_fallback(ciphertext: &str) -> Option<String> {
    // Legacy scalar ciphertext is a best-effort display fallback. Canonical
    // payload/key failures are retained separately in `read_error`.
    decode_sealed_blob(ciphertext)
        .and_then(|bytes| decrypt_text_value(&bytes))
        .ok()
}

fn decode_work_list_text_fallbacks(
    work_list: &WorkListResponse,
) -> (Option<String>, Option<String>) {
    (
        decode_text_fallback(&work_list.title_ciphertext),
        work_list
            .description_ciphertext
            .as_deref()
            .and_then(decode_text_fallback),
    )
}

pub(crate) fn make_read_error(code: &str, err: PublicError) -> ReadError {
    ReadError {
        code: code.to_string(),
        message: err.to_string(),
    }
}

pub(crate) fn read_error_to_public_error(
    read_error: Option<&ReadError>,
    fallback: &str,
) -> PublicError {
    match read_error {
        Some(read_error) => PublicError::validation(read_error.message.clone()),
        None => PublicError::validation(fallback),
    }
}

pub(crate) fn rich_text_to_markdown(rich_text: &TaskPayloadRichText) -> Option<String> {
    let text = rich_text
        .blocks
        .iter()
        .map(|block| block.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.is_empty() { None } else { Some(text) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sealtask_client_crypto::encrypt_task_reference_scheme;

    #[test]
    fn test_should_join_non_empty_rich_text_blocks_as_markdown() {
        let rich_text = TaskPayloadRichText {
            format: "markdown".to_string(),
            version: 1,
            blocks: vec![
                sealtask_client_crypto::RichTextBlock {
                    block_type: "paragraph".to_string(),
                    text: "First".to_string(),
                },
                sealtask_client_crypto::RichTextBlock {
                    block_type: "paragraph".to_string(),
                    text: "Second".to_string(),
                },
            ],
        };

        assert_eq!(
            rich_text_to_markdown(&rich_text).as_deref(),
            Some("First\n\nSecond")
        );
    }

    fn task_reference_history(
        count: i64,
    ) -> (SymmetricKey, Uuid, Vec<TaskReferenceSchemeResponse>, Uuid) {
        let list_key = SymmetricKey::new([7; 32]);
        let work_list_id = Uuid::from_u128(0x018f_6b71_2b8a_7000_8000_0000_0000_0001);
        let mut current_revision_id = Uuid::nil();
        let responses = (1..=count)
            .map(|revision| {
                let scheme_revision_id =
                    Uuid::from_u128(0x018f_6b71_2b8a_7000_8000_0000_0000_1000 + revision as u128);
                current_revision_id = scheme_revision_id;
                let scheme = TaskReferenceSchemeV1::new(
                    work_list_id,
                    scheme_revision_id,
                    revision,
                    format!("WL{revision}"),
                    4,
                )
                .expect("valid test scheme");
                let payload = encrypt_task_reference_scheme(&scheme, &list_key)
                    .expect("scheme encryption should succeed");
                TaskReferenceSchemeResponse {
                    scheme_revision_id,
                    work_list_id,
                    revision,
                    payload_ciphertext: payload.base64,
                    is_repair: false,
                    created_at: Utc::now(),
                    retired_at: (revision != count).then(Utc::now),
                    quarantined_at: None,
                    quarantined_by_membership_id: None,
                }
            })
            .collect();
        (list_key, work_list_id, responses, current_revision_id)
    }

    #[test]
    fn test_should_accept_only_a_complete_task_reference_scheme_history() {
        let (list_key, work_list_id, responses, current_revision_id) = task_reference_history(2);

        let schemes = decode_task_reference_schemes(
            &list_key,
            work_list_id,
            true,
            Some(2),
            Some(current_revision_id),
            &responses,
        );

        assert_eq!(
            schemes
                .iter()
                .map(|scheme| scheme.revision)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            schemes.last().map(|scheme| scheme.scheme_revision_id),
            Some(current_revision_id)
        );
    }

    #[test]
    fn test_should_fail_closed_for_any_invalid_task_reference_scheme_row() {
        let (list_key, work_list_id, responses, current_revision_id) = task_reference_history(2);

        let mut corrupt = responses.clone();
        corrupt[0].payload_ciphertext = "not-base64".to_string();
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(2),
                Some(current_revision_id),
                &corrupt,
            )
            .is_empty()
        );

        let mut duplicate_revision = responses.clone();
        duplicate_revision[0].revision = 2;
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(2),
                Some(current_revision_id),
                &duplicate_revision,
            )
            .is_empty()
        );

        let mut duplicate_id = responses.clone();
        duplicate_id[0].scheme_revision_id = duplicate_id[1].scheme_revision_id;
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(2),
                Some(current_revision_id),
                &duplicate_id,
            )
            .is_empty()
        );

        let mut multiple_active = responses.clone();
        multiple_active[0].retired_at = None;
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(2),
                Some(current_revision_id),
                &multiple_active,
            )
            .is_empty()
        );
    }

    #[test]
    fn test_should_fail_closed_for_incomplete_or_mismatched_scheme_history() {
        let (list_key, work_list_id, responses, current_revision_id) = task_reference_history(2);

        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(3),
                Some(current_revision_id),
                &responses,
            )
            .is_empty()
        );
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(2),
                Some(Uuid::now_v7()),
                &responses,
            )
            .is_empty()
        );
        assert!(
            decode_task_reference_schemes(&list_key, work_list_id, true, None, None, &responses,)
                .is_empty()
        );

        let oversized = vec![responses[0].clone(); 37];
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(37),
                Some(responses[0].scheme_revision_id),
                &oversized,
            )
            .is_empty()
        );
    }

    #[test]
    fn test_should_exclude_only_explicitly_quarantined_rows_after_full_map_validation() {
        let (list_key, work_list_id, mut responses, current_revision_id) =
            task_reference_history(3);
        responses[0].payload_ciphertext = "opaque-poison".to_string();
        responses[0].quarantined_at = Some(Utc::now());
        responses[0].quarantined_by_membership_id = Some(Uuid::now_v7());

        let schemes = decode_task_reference_schemes(
            &list_key,
            work_list_id,
            true,
            Some(3),
            Some(current_revision_id),
            &responses,
        );

        assert_eq!(
            schemes
                .iter()
                .map(|scheme| scheme.revision)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );

        let mut incomplete_quarantine = responses;
        incomplete_quarantine[0].quarantined_by_membership_id = None;
        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(3),
                Some(current_revision_id),
                &incomplete_quarantine,
            )
            .is_empty()
        );
    }

    #[test]
    fn test_should_reject_a_repair_row_as_the_enablement_revision() {
        let (list_key, work_list_id, mut responses, current_revision_id) =
            task_reference_history(2);
        responses[0].is_repair = true;

        assert!(
            decode_task_reference_schemes(
                &list_key,
                work_list_id,
                true,
                Some(2),
                Some(current_revision_id),
                &responses,
            )
            .is_empty()
        );
    }
}
