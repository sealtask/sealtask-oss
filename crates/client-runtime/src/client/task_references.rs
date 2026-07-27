use super::RuntimeClient;
use crate::inputs::{QuarantineTaskReferenceSchemeArgs, RepairTaskReferenceSchemeArgs};
use crate::models::{
    AgentTaskReferenceHistoryStatus, AgentTaskReferenceSchemeStatus,
    TaskReferenceHistoryAvailability,
};
use crate::projections::validate_task_reference_scheme_history_metadata;
use sealtask_client_api::{
    TaskReferenceSchemeMutationRequest, TaskReferenceSchemeQuarantineRequest,
    TaskReferenceSchemeResponse,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    TASK_REFERENCE_REPAIR_REVISION_MAX, TASK_REFERENCE_REVISION_MAX, TaskReferenceSchemeV1,
    compute_payload_proof, decode_sealed_blob, decrypt_task_reference_scheme,
    derive_payload_binding_key, encrypt_task_reference_scheme,
};
use uuid::Uuid;

impl RuntimeClient {
    pub async fn inspect_task_reference_schemes(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentTaskReferenceHistoryStatus> {
        let (mut client, context) = self
            .load_unlocked_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to inspect encrypted task-reference history.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context.work_list)?;
        let current_revision = context.work_list.current_task_reference_scheme_revision;
        let current_revision_id = context.work_list.current_task_reference_scheme_revision_id;
        let Some(current_revision_value) = current_revision else {
            return Ok(AgentTaskReferenceHistoryStatus {
                work_list_id,
                current_revision: None,
                availability: TaskReferenceHistoryAvailability::Unchecked,
                ordinary_revision_count: None,
                repair_revision_count: None,
                schemes: Vec::new(),
            });
        };
        let history = client.get_task_reference_schemes(work_list_id).await?;
        let Some(metadata) = validate_task_reference_scheme_history_metadata(
            work_list_id,
            true,
            current_revision,
            current_revision_id,
            &history,
        ) else {
            return Ok(AgentTaskReferenceHistoryStatus {
                work_list_id,
                current_revision,
                availability: TaskReferenceHistoryAvailability::Unchecked,
                ordinary_revision_count: None,
                repair_revision_count: None,
                schemes: Vec::new(),
            });
        };

        let mut current_is_readable = false;
        let mut has_unreadable_history = false;
        let mut schemes = history
            .iter()
            .map(|row| {
                let state = if row.quarantined_at.is_some() {
                    "quarantined"
                } else if decrypt_response(row, list_key).is_ok() {
                    if row.revision == current_revision_value {
                        current_is_readable = true;
                    }
                    "readable"
                } else {
                    if row.revision < current_revision_value {
                        has_unreadable_history = true;
                    }
                    "unreadable"
                };
                AgentTaskReferenceSchemeStatus {
                    scheme_revision_id: row.scheme_revision_id,
                    revision: row.revision,
                    is_repair: row.is_repair,
                    state: state.to_string(),
                    retired_at: row.retired_at,
                    quarantined_at: row.quarantined_at,
                    quarantined_by_membership_id: row.quarantined_by_membership_id,
                }
            })
            .collect::<Vec<_>>();
        schemes.sort_by_key(|row| row.revision);
        let availability = if !current_is_readable {
            TaskReferenceHistoryAvailability::NeedsRepair
        } else if has_unreadable_history {
            TaskReferenceHistoryAvailability::NeedsQuarantine
        } else {
            TaskReferenceHistoryAvailability::Ready
        };
        Ok(AgentTaskReferenceHistoryStatus {
            work_list_id,
            current_revision,
            availability,
            ordinary_revision_count: Some(metadata.ordinary_revision_count),
            repair_revision_count: Some(metadata.repair_revision_count),
            schemes,
        })
    }

    pub async fn repair_task_reference_scheme(
        &self,
        args: RepairTaskReferenceSchemeArgs,
    ) -> PublicResult<TaskReferenceSchemeResponse> {
        let (mut client, context) = self
            .load_unlocked_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to install an encrypted task-reference repair.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context.work_list)?;
        let current_revision = context
            .work_list
            .current_task_reference_scheme_revision
            .ok_or_else(|| PublicError::validation("task references are not enabled"))?;
        let current_revision_id = context
            .work_list
            .current_task_reference_scheme_revision_id
            .ok_or_else(|| PublicError::validation("task reference metadata is incomplete"))?;
        let history = client.get_task_reference_schemes(args.work_list_id).await?;
        let metadata = validate_task_reference_scheme_history_metadata(
            args.work_list_id,
            true,
            Some(current_revision),
            Some(current_revision_id),
            &history,
        )
        .ok_or_else(|| {
            PublicError::unexpected("task reference public history is incomplete; refusing repair")
        })?;
        if metadata.current_revision >= TASK_REFERENCE_REVISION_MAX {
            return Err(PublicError::conflict(
                "task reference total revision capacity has been reached",
            ));
        }
        if i64::try_from(metadata.repair_revision_count).unwrap_or(i64::MAX)
            >= TASK_REFERENCE_REPAIR_REVISION_MAX
        {
            return Err(PublicError::conflict(
                "task reference repair capacity has been reached",
            ));
        }

        let scheme = TaskReferenceSchemeV1::new(
            args.work_list_id,
            Uuid::now_v7(),
            current_revision + 1,
            args.prefix.trim().to_ascii_uppercase(),
            args.minimum_digits,
        )?;
        let encrypted = encrypt_task_reference_scheme(&scheme, list_key)?;
        let binding_key = derive_payload_binding_key(list_key)?;
        let request = TaskReferenceSchemeMutationRequest {
            scheme_revision_id: scheme.scheme_revision_id,
            expected_scheme_revision: current_revision,
            payload_ciphertext: encrypted.base64,
            payload_ciphertext_proof: compute_payload_proof(&encrypted.bytes, &binding_key)?,
            audit_patch: None,
        };
        let response = client
            .repair_task_reference_scheme(args.work_list_id, &request)
            .await?;
        verify_repair_response(&response, &scheme, list_key)?;
        Ok(response)
    }

    pub async fn quarantine_task_reference_scheme(
        &self,
        args: QuarantineTaskReferenceSchemeArgs,
    ) -> PublicResult<TaskReferenceSchemeResponse> {
        let (mut client, context) = self
            .load_unlocked_work_list_context(
                args.work_list_id,
                args.password_stdin,
                "Password required to verify an unreadable historical task-reference scheme.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context.work_list)?;
        let current_revision = context
            .work_list
            .current_task_reference_scheme_revision
            .ok_or_else(|| PublicError::validation("task references are not enabled"))?;
        let current_revision_id = context
            .work_list
            .current_task_reference_scheme_revision_id
            .ok_or_else(|| PublicError::validation("task reference metadata is incomplete"))?;
        let history = client.get_task_reference_schemes(args.work_list_id).await?;
        validate_task_reference_scheme_history_metadata(
            args.work_list_id,
            true,
            Some(current_revision),
            Some(current_revision_id),
            &history,
        )
        .ok_or_else(|| {
            PublicError::unexpected(
                "task reference public history is incomplete; refusing quarantine",
            )
        })?;
        let target = history
            .iter()
            .find(|row| row.scheme_revision_id == args.scheme_revision_id)
            .ok_or_else(|| PublicError::not_found("task reference scheme revision not found"))?;
        let current = history
            .iter()
            .find(|row| {
                row.revision == current_revision && row.scheme_revision_id == current_revision_id
            })
            .ok_or_else(|| {
                PublicError::unexpected(
                    "current task reference scheme is missing; refusing quarantine",
                )
            })?;
        decrypt_response(current, list_key).map_err(|_| {
            PublicError::conflict(
                "install and verify a readable current repair before quarantining history",
            )
        })?;
        if target.revision >= current_revision
            || target.retired_at.is_none()
            || target.quarantined_at.is_some()
            || target.quarantined_by_membership_id.is_some()
        {
            return Err(PublicError::conflict(
                "only an unquarantined historical scheme may be quarantined",
            ));
        }
        if decrypt_response(target, list_key).is_ok() {
            return Err(PublicError::validation(
                "refusing to quarantine a readable task reference scheme",
            ));
        }

        let response = client
            .quarantine_task_reference_scheme(
                args.work_list_id,
                args.scheme_revision_id,
                &TaskReferenceSchemeQuarantineRequest {
                    expected_scheme_revision: current_revision,
                    audit_patch: None,
                },
            )
            .await?;
        verify_quarantine_response(&response, target, context.membership_id)?;
        Ok(response)
    }
}

fn decrypt_response(
    response: &TaskReferenceSchemeResponse,
    list_key: &sealtask_client_crypto::SymmetricKey,
) -> PublicResult<TaskReferenceSchemeV1> {
    let bytes = decode_sealed_blob(&response.payload_ciphertext)?;
    decrypt_task_reference_scheme(
        list_key,
        &bytes,
        response.work_list_id,
        response.scheme_revision_id,
        response.revision,
    )
}

fn verify_repair_response(
    response: &TaskReferenceSchemeResponse,
    expected: &TaskReferenceSchemeV1,
    list_key: &sealtask_client_crypto::SymmetricKey,
) -> PublicResult<()> {
    if response.work_list_id != expected.work_list_id
        || response.scheme_revision_id != expected.scheme_revision_id
        || response.revision != expected.revision
        || !response.is_repair
        || response.retired_at.is_some()
        || response.quarantined_at.is_some()
        || response.quarantined_by_membership_id.is_some()
        || decrypt_response(response, list_key)? != *expected
    {
        return Err(PublicError::unexpected(
            "task reference repair returned mismatched metadata or ciphertext",
        ));
    }
    Ok(())
}

fn verify_quarantine_response(
    response: &TaskReferenceSchemeResponse,
    expected: &TaskReferenceSchemeResponse,
    membership_id: Uuid,
) -> PublicResult<()> {
    if response.work_list_id != expected.work_list_id
        || response.scheme_revision_id != expected.scheme_revision_id
        || response.revision != expected.revision
        || response.payload_ciphertext != expected.payload_ciphertext
        || response.is_repair != expected.is_repair
        || response.created_at != expected.created_at
        || response.retired_at != expected.retired_at
        || response.quarantined_at.is_none()
        || response.quarantined_by_membership_id != Some(membership_id)
    {
        return Err(PublicError::unexpected(
            "task reference quarantine returned mismatched metadata",
        ));
    }
    Ok(())
}
