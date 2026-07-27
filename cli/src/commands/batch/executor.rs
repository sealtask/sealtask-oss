use super::input::{zeroize_task_create_input, zeroize_task_update_input};
use crate::output::{CliError, CliResult};
use chrono::{DateTime, Utc};
use sealtask_client_core::PublicResult;
use sealtask_client_runtime::{
    AgentTaskSummary, CreateTaskArgs, PreparedTaskCreate, PreparedTaskUpdate, RuntimeClient,
    TaskCreateIdempotencyDerivation, TaskCreateInput, TaskMutationPlan, TaskUpdateInput,
    UpdateTaskArgs,
};
use uuid::Uuid;

pub(super) enum MutationInput {
    TaskCreate {
        input: TaskCreateInput,
        idempotency_derivation: Option<TaskCreateIdempotencyDerivation>,
    },
    TaskUpdate {
        task_id: Uuid,
        input: TaskUpdateInput,
    },
}

impl Drop for MutationInput {
    fn drop(&mut self) {
        match self {
            Self::TaskCreate { input, .. } => zeroize_task_create_input(input),
            Self::TaskUpdate { input, .. } => zeroize_task_update_input(input),
        }
    }
}

pub(super) struct Mutation {
    pub(super) project_id: Uuid,
    pub(super) input: MutationInput,
}

pub(super) enum PreparedMutation {
    TaskCreate(Box<PreparedTaskCreate>),
    TaskUpdate(Box<PreparedTaskUpdate>),
}

impl Mutation {
    pub(super) fn task_id(&self) -> Option<Uuid> {
        match &self.input {
            MutationInput::TaskCreate { .. } => None,
            MutationInput::TaskUpdate { task_id, .. } => Some(*task_id),
        }
    }

    pub(super) async fn prepare(
        &self,
        runtime: &RuntimeClient,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> PublicResult<PreparedMutation> {
        match &self.input {
            MutationInput::TaskCreate {
                input,
                idempotency_derivation,
            } => {
                let args = CreateTaskArgs {
                    work_list_id: self.project_id,
                    input: input.clone(),
                    password_stdin: false,
                };
                let prepared = match idempotency_derivation {
                    Some(derivation) => {
                        runtime
                            .prepare_task_create_with_idempotency_derivation(args, derivation)
                            .await
                    }
                    None => runtime.prepare_task_create(args).await,
                };
                prepared.map(Box::new).map(PreparedMutation::TaskCreate)
            }
            MutationInput::TaskUpdate { task_id, input } => {
                let args = UpdateTaskArgs {
                    work_list_id: self.project_id,
                    task_id: *task_id,
                    input: input.clone(),
                    password_stdin: false,
                };
                match expected_updated_at {
                    Some(expected) => {
                        runtime
                            .prepare_task_update_if_unchanged(args, expected)
                            .await
                    }
                    None => runtime.prepare_task_update(args).await,
                }
                .map(Box::new)
                .map(PreparedMutation::TaskUpdate)
            }
        }
    }
}

impl PreparedMutation {
    pub(super) fn plan(&self) -> &TaskMutationPlan {
        match self {
            Self::TaskCreate(prepared) => prepared.plan(),
            Self::TaskUpdate(prepared) => prepared.plan(),
        }
    }

    pub(super) async fn execute(self, runtime: &RuntimeClient) -> PublicResult<AgentTaskSummary> {
        match self {
            Self::TaskCreate(prepared) => runtime.execute_prepared_task_create(*prepared).await,
            Self::TaskUpdate(prepared) => runtime.execute_prepared_task_update(*prepared).await,
        }
    }
}

pub(super) fn ensure_resume_plan_matches(
    stored_expected_updated_at: DateTime<Utc>,
    stored_change_commitment: Option<&str>,
    plan: &TaskMutationPlan,
) -> CliResult<ResumeDecision> {
    let current_revision = plan.expected_updated_at.ok_or_else(|| {
        CliError::checkpoint_conflict(
            "checkpointed task.update plan is missing its original revision",
        )
    })?;
    if current_revision != stored_expected_updated_at {
        return Err(CliError::checkpoint_conflict(
            "cannot safely resume task.update because its original revision is no longer current",
        ));
    }
    let stored_change_commitment = stored_change_commitment.ok_or_else(|| {
        CliError::checkpoint_conflict(
            "checkpointed task.update plan is missing its original change commitment",
        )
    })?;
    if stored_change_commitment != plan.change_commitment {
        return Err(CliError::checkpoint_conflict(
            "cannot safely resume task.update because its original prepared change no longer matches",
        ));
    }
    if !plan.would_change {
        return Ok(ResumeDecision::AlreadyApplied);
    }
    Ok(ResumeDecision::Execute)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResumeDecision {
    AlreadyApplied,
    Execute,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(
        expected_updated_at: Option<DateTime<Utc>>,
        change_commitment: &str,
        would_change: bool,
    ) -> TaskMutationPlan {
        TaskMutationPlan {
            schema_version: 1,
            plan_type: "taskMutationPlan",
            action: "task.update",
            project_id: Uuid::now_v7(),
            task_id: Some(Uuid::now_v7()),
            section_id: None,
            expected_updated_at,
            changed_fields: Vec::new(),
            changed_field_count: usize::from(would_change),
            change_commitment: change_commitment.to_string(),
            idempotency_protected: false,
            would_change,
            will_mutate: false,
        }
    }

    #[test]
    fn resume_update_never_rebases_or_accepts_a_changed_commitment_as_a_noop() {
        let original_revision = Utc::now();
        let moved_revision = original_revision + chrono::TimeDelta::seconds(1);
        let revision_error = ensure_resume_plan_matches(
            original_revision,
            Some("original"),
            &plan(Some(moved_revision), "original", false),
        )
        .expect_err("moved revision must fail closed");
        assert_eq!(revision_error.code(), "checkpoint_conflict");
        assert_eq!(revision_error.exit_code(), 4);

        let commitment_error = ensure_resume_plan_matches(
            original_revision,
            Some("original"),
            &plan(Some(original_revision), "changed", false),
        )
        .expect_err("changed commitment must fail before no-op reconciliation");
        assert_eq!(commitment_error.code(), "checkpoint_conflict");
        assert_eq!(commitment_error.exit_code(), 4);
    }

    #[test]
    fn resume_update_accepts_only_the_exact_original_plan() {
        let original_revision = Utc::now();
        assert!(matches!(
            ensure_resume_plan_matches(
                original_revision,
                Some("original"),
                &plan(Some(original_revision), "original", true),
            ),
            Ok(ResumeDecision::Execute)
        ));
        assert!(matches!(
            ensure_resume_plan_matches(
                original_revision,
                Some("original"),
                &plan(Some(original_revision), "original", false),
            ),
            Ok(ResumeDecision::AlreadyApplied)
        ));
    }
}
