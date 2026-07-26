use super::{RuntimeClient, WorkListContext, load_task_reference_scheme_history};
use crate::inputs::{
    ArchiveTaskArgs, CreateTaskArgs, DeleteTaskArgs, MoveTaskArgs, TaskCompletionArgs,
    TaskFieldPatch, TaskUpdateInput, UnarchiveTaskArgs, UpdateTaskArgs, normalize_checklist,
    validate_idempotency_key, validate_priority,
};
use crate::models::{AgentTaskDetail, AgentTaskSummary};
use chrono::{DateTime, Utc};
use sealtask_client_api::{
    ArchiveTaskRequest, CreateTaskRequest, MoveTaskRequest, TaskSectionBoundary,
    UnarchiveTaskRequest, UpdateTaskRequest,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    ChecklistItemPayload, TASK_REFERENCE_REVISION_MAX, TASK_TITLE_CONTEXT, TaskPayloadBody,
    build_task_payload_envelope, compute_payload_proof, compute_task_create_semantic_commitment,
    decode_sealed_blob, decrypt_task_payload, derive_payload_binding_key, encrypt_task_payload,
    encrypt_text_value, plaintext_rich_text,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

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
        let mut client =
            sealtask_client_api::PublicApiClient::with_credentials(&self.api_url, credentials)?;

        let work_lists = client.list_work_lists().await?;
        let mut scheme_histories = HashMap::new();
        for work_list in &work_lists {
            let history = load_task_reference_scheme_history(&mut client, work_list).await;
            if !history.is_empty() {
                scheme_histories.insert(work_list.id, history);
            }
        }
        let contexts =
            self.build_work_list_contexts(&work_lists, &scheme_histories, Some(&data_key));
        let tasks = client.get_all_my_tasks(include_completed).await?;

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
        let mut credentials = self.require_logged_in_credentials()?;
        let data_key = self
            .load_data_key(
                &mut credentials,
                password_stdin,
                "Password required to decrypt task data.",
            )
            .await?;
        let mut client =
            sealtask_client_api::PublicApiClient::with_credentials(&self.api_url, credentials)?;
        let work_list = client.get_work_list(work_list_id).await?;
        let scheme_history =
            load_task_reference_scheme_history(&mut client, &work_list.work_list).await;
        let context =
            self.context_from_work_list_detail(&work_list, &scheme_history, Some(&data_key));
        let response = client.get_tasks(work_list_id, include_archived).await?;
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
            .map(|task| self.project_task_summary(task, Some(&context)))
            .collect())
    }

    pub async fn get_task(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentTaskDetail> {
        let (mut client, context) = self
            .load_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt task data.",
            )
            .await?;
        let detail = client.get_task(work_list_id, task_id).await?;

        let task = self.project_task_summary(detail.task, Some(&context));
        let comments = detail
            .comments
            .into_iter()
            .map(|comment| self.project_comment(comment, context.list_key.as_ref()))
            .collect();
        Ok(AgentTaskDetail { task, comments })
    }

    pub async fn resolve_task_reference(
        &self,
        reference: &str,
        work_list_id: Option<Uuid>,
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
        let mut client =
            sealtask_client_api::PublicApiClient::with_credentials(&self.api_url, credentials)?;

        let work_lists = if let Some(work_list_id) = work_list_id {
            vec![client.get_work_list(work_list_id).await?.work_list]
        } else {
            // Archived projects remain valid UUID/numeric lookup scopes and
            // may own a colliding private prefix. Excluding them would turn an
            // incomplete directory into a false miss or sole auto-resolution.
            client.list_work_lists_with_archived(true).await?
        };
        let mut scheme_histories = HashMap::new();
        let mut reference_enabled_ids = HashSet::new();
        for work_list in &work_lists {
            match (
                work_list.task_references_enabled_at.is_some(),
                work_list.current_task_reference_scheme_revision,
                work_list.current_task_reference_scheme_revision_id,
            ) {
                (false, None, None) => {}
                (true, Some(revision), Some(_))
                    if (1..=TASK_REFERENCE_REVISION_MAX).contains(&revision) =>
                {
                    let history = client.get_task_reference_schemes(work_list.id).await?;
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

        let contexts =
            self.build_work_list_contexts(&work_lists, &scheme_histories, Some(&data_key));
        let mut unchecked_work_list_ids = reference_enabled_ids
            .into_iter()
            .filter(|work_list_id| {
                contexts
                    .get(work_list_id)
                    .and_then(WorkListContext::current_task_reference_scheme)
                    .is_none()
            })
            .collect::<Vec<_>>();
        unchecked_work_list_ids.sort_unstable();
        if !unchecked_work_list_ids.is_empty() {
            return Err(PublicError::unexpected(format!(
                "task reference lookup is unchecked because scheme history is unavailable for work lists {}; no definitive miss or automatic resolution was attempted",
                unchecked_work_list_ids
                    .iter()
                    .map(Uuid::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        let (resolved_work_list_id, reference_number) =
            resolve_task_reference_candidate(reference, work_list_id, &contexts)?;
        let context = contexts.get(&resolved_work_list_id).ok_or_else(|| {
            PublicError::unexpected("resolved task reference lost its work list context")
        })?;

        let detail = client
            .get_task_by_reference_number(resolved_work_list_id, reference_number)
            .await?;
        if detail.task.work_list_id != resolved_work_list_id
            || detail.task.reference_number != Some(reference_number)
        {
            return Err(PublicError::unexpected(
                "task reference lookup returned mismatched public metadata",
            ));
        }
        let task = self.project_task_summary(detail.task, Some(context));
        let comments = detail
            .comments
            .into_iter()
            .map(|comment| self.project_comment(comment, context.list_key.as_ref()))
            .collect();
        Ok(AgentTaskDetail { task, comments })
    }

    pub async fn create_task(&self, args: CreateTaskArgs) -> PublicResult<AgentTaskSummary> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to create encrypted task payloads.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context)?;
        let binding_key = derive_payload_binding_key(list_key)?;

        let normalized_title = args.input.title.trim();
        if normalized_title.is_empty() {
            return Err(PublicError::validation("title is required"));
        }

        validate_priority(args.input.priority)?;
        let normalized_body = args
            .input
            .body
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let checklist = args
            .input
            .checklist
            .clone()
            .map(normalize_checklist)
            .transpose()?;
        let idempotency_key = args
            .input
            .idempotency_key
            .as_deref()
            .map(validate_idempotency_key)
            .transpose()?;
        let idempotency_commitment = if idempotency_key.is_some() {
            let semantics = TaskCreateSemanticPlan {
                title: normalized_title,
                body: normalized_body,
                checklist: checklist.as_deref(),
                priority: args.input.priority,
                due_at: args.input.due_at.as_ref(),
                start_at: args.input.start_at.as_ref(),
                section_id: args.input.section_id,
            };
            let canonical_semantics = serde_json::to_vec(&semantics).map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to encode task idempotency semantics: {err}"
                ))
            })?;
            Some(compute_task_create_semantic_commitment(
                &canonical_semantics,
                list_key,
            )?)
        } else {
            None
        };

        let task_body = TaskPayloadBody {
            title: normalized_title.to_string(),
            rich_text: normalized_body.and_then(plaintext_rich_text),
            checklist,
            attachments: None,
            references: None,
            mentions: None,
            client_meta: None,
            recurrence_state: None,
        };
        let envelope = build_task_payload_envelope(task_body, 1);
        let payload_ciphertext = encrypt_task_payload(&envelope, list_key)?;
        let title_ciphertext = encrypt_text_value(normalized_title, list_key, TASK_TITLE_CONTEXT)?;
        let payload_proof = compute_payload_proof(&payload_ciphertext.bytes, &binding_key)?;
        let title_proof = compute_payload_proof(&title_ciphertext.bytes, &binding_key)?;

        let created = client
            .create_task(
                args.work_list_id,
                &CreateTaskRequest {
                    title_ciphertext: title_ciphertext.base64,
                    title_ciphertext_proof: title_proof,
                    payload_ciphertext: payload_ciphertext.base64,
                    payload_ciphertext_proof: payload_proof,
                    attachment_ids: Vec::new(),
                    priority: args.input.priority,
                    due_at: args.input.due_at,
                    start_at: args.input.start_at,
                    section_id: args.input.section_id,
                    idempotency_key,
                    idempotency_commitment,
                },
            )
            .await?;

        Ok(self.project_task_summary(created, Some(&context)))
    }

    pub async fn update_task(&self, args: UpdateTaskArgs) -> PublicResult<AgentTaskSummary> {
        let (mut client, context) = self
            .load_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to update encrypted task payloads.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context)?;
        let binding_key = derive_payload_binding_key(list_key)?;
        let task_detail = client.get_task(args.work_list_id, args.task_id).await?;

        let TaskUpdateInput {
            title,
            body,
            checklist,
            priority,
            due_at,
            start_at,
            section_id,
        } = args.input;
        let payload_changed = title.is_some() || !body.is_unchanged() || !checklist.is_unchanged();
        if !payload_changed
            && priority.is_unchanged()
            && due_at.is_unchanged()
            && start_at.is_unchanged()
            && section_id.is_unchanged()
        {
            return Err(PublicError::validation(
                "provide at least one task field to update",
            ));
        }
        if let TaskFieldPatch::Set(value) = &priority {
            validate_priority(Some(*value))?;
        }

        let mut request = UpdateTaskRequest {
            expected_updated_at: Some(task_detail.task.updated_at),
            priority: priority.into_nested_option(),
            due_at: due_at.into_nested_option(),
            start_at: start_at.into_nested_option(),
            section_id: section_id.into_nested_option(),
            ..UpdateTaskRequest::default()
        };

        if payload_changed {
            let existing_payload_bytes = decode_sealed_blob(&task_detail.task.payload_ciphertext)?;
            let existing_payload = decrypt_task_payload(list_key, &existing_payload_bytes)?;
            let existing_body = existing_payload.body;
            let next_title = title
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| existing_body.title.clone());
            let next_rich_text = match body {
                TaskFieldPatch::Unchanged => existing_body.rich_text,
                TaskFieldPatch::Set(value) => plaintext_rich_text(&value),
                TaskFieldPatch::Clear => None,
            };
            let next_checklist = match checklist {
                TaskFieldPatch::Unchanged => existing_body.checklist,
                TaskFieldPatch::Set(items) => Some(normalize_checklist(items)?),
                TaskFieldPatch::Clear => None,
            };
            let next_body = TaskPayloadBody {
                title: next_title,
                rich_text: next_rich_text,
                checklist: next_checklist,
                attachments: existing_body.attachments,
                references: existing_body.references,
                mentions: existing_body.mentions,
                client_meta: existing_body.client_meta,
                recurrence_state: existing_body.recurrence_state,
            };
            let envelope = build_task_payload_envelope(next_body, 1);
            let payload_ciphertext = encrypt_task_payload(&envelope, list_key)?;
            let payload_proof = compute_payload_proof(&payload_ciphertext.bytes, &binding_key)?;
            request.payload_ciphertext = Some(payload_ciphertext.base64);
            request.payload_ciphertext_proof = Some(payload_proof);
        }

        if let Some(new_title) = title.as_deref() {
            let normalized_title = new_title.trim();
            if normalized_title.is_empty() {
                return Err(PublicError::validation("title cannot be empty"));
            }
            let title_ciphertext =
                encrypt_text_value(normalized_title, list_key, TASK_TITLE_CONTEXT)?;
            let title_proof = compute_payload_proof(&title_ciphertext.bytes, &binding_key)?;
            request.title_ciphertext = Some(title_ciphertext.base64);
            request.title_ciphertext_proof = Some(title_proof);
        }

        let updated = client
            .update_task(args.work_list_id, args.task_id, &request)
            .await?;
        Ok(self.project_task_summary(updated, Some(&context)))
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
        let moved = client
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
            .await?;
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
        let moved = client
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
            .await?;
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
        let archived = client
            .archive_task(
                args.work_list_id,
                args.task_id,
                &ArchiveTaskRequest::default(),
            )
            .await?;
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
        let unarchived = client
            .unarchive_task(
                args.work_list_id,
                args.task_id,
                &UnarchiveTaskRequest::default(),
            )
            .await?;
        Ok(self.project_task_summary(unarchived, Some(&context)))
    }

    pub async fn delete_task(&self, args: DeleteTaskArgs) -> PublicResult<()> {
        let mut client = self.authenticated_api_client()?;
        client
            .delete_task(args.work_list_id, args.task_id, &args.input)
            .await
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
}
