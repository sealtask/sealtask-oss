use crate::project_context::load_current_project;
use crate::selectors::{
    EntityCandidate, EntitySelector, ProjectSection, ResolvedEntity, TaskSelectorTarget,
    project_sections, resolve_entity, section_candidates,
};
use crate::terminal::with_progress;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{
    AgentNote, AgentTaskDetail, AgentTaskSummary, AgentWorkListSummary, RuntimeClient,
};
use std::fmt;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectLifecycle {
    Any,
    Active,
    Archived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskLifecycle {
    Any,
    Active,
    Archived,
    Incomplete,
    Completed,
}

#[derive(Clone)]
pub(crate) struct ResolvedProject {
    pub(crate) id: Uuid,
    pub(crate) title: Option<String>,
}

impl fmt::Debug for ResolvedProject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedProject")
            .field("id", &self.id)
            .field("title_present", &self.title.is_some())
            .finish()
    }
}

pub(crate) struct ResolvedTaskAndProject {
    pub(crate) project: ResolvedProject,
    pub(crate) task: ResolvedEntity,
    pub(crate) detail: Option<AgentTaskDetail>,
}

pub(crate) async fn resolve_project(
    runtime: &RuntimeClient,
    selector: Option<&EntitySelector>,
    exact_id: Option<Uuid>,
    password_stdin: bool,
    lifecycle: ProjectLifecycle,
) -> PublicResult<ResolvedProject> {
    if let Some(id) = exact_id {
        return Ok(ResolvedProject { id, title: None });
    }
    if let Some(selector) = selector {
        if let Some(id) = selector.exact_id() {
            return Ok(ResolvedProject { id, title: None });
        }
        let projects = with_progress(
            "Resolving project…",
            runtime.list_work_lists_with_archived(password_stdin, true),
        )
        .await?;
        let filtered = projects
            .iter()
            .filter(|project| project_matches_lifecycle(project, lifecycle))
            .collect::<Vec<_>>();
        let resolved = resolve_entity(
            "project",
            project_lifecycle_scope(lifecycle),
            selector,
            filtered.iter().map(project_candidate).collect(),
            "sealtask projects list --include-archived",
        )?;
        let project = filtered
            .into_iter()
            .find(|project| project.id == resolved.id);
        return Ok(ResolvedProject {
            id: resolved.id,
            title: project.and_then(|project| project.title.clone()),
        });
    }

    let Some(id) = load_current_project(runtime.api_url())? else {
        return Err(PublicError::validation(
            "no project was specified and neither this directory nor the active profile has a current project; pass --project/--work-list-id or run 'sealtask pick project'",
        ));
    };
    Ok(ResolvedProject { id, title: None })
}

pub(crate) async fn load_project(
    runtime: &RuntimeClient,
    project_id: Uuid,
    password_stdin: bool,
) -> PublicResult<AgentWorkListSummary> {
    Ok(with_progress(
        "Loading and decrypting project…",
        runtime.get_work_list(project_id, password_stdin),
    )
    .await?
    .work_list)
}

pub(crate) async fn resolve_optional_project(
    runtime: &RuntimeClient,
    selector: Option<&EntitySelector>,
    exact_id: Option<Uuid>,
    password_stdin: bool,
) -> PublicResult<Option<ResolvedProject>> {
    if selector.is_some() || exact_id.is_some() {
        return resolve_project(
            runtime,
            selector,
            exact_id,
            password_stdin,
            ProjectLifecycle::Any,
        )
        .await
        .map(Some);
    }
    Ok(load_current_project(runtime.api_url())?.map(|id| ResolvedProject { id, title: None }))
}

pub(crate) async fn resolve_task_and_project(
    runtime: &RuntimeClient,
    project_selector: Option<&EntitySelector>,
    project_id: Option<Uuid>,
    task_selector: Option<&EntitySelector>,
    task_id: Option<Uuid>,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> PublicResult<(ResolvedProject, ResolvedEntity)> {
    let resolved = resolve_task_and_project_with_detail(
        runtime,
        project_selector,
        project_id,
        task_selector,
        task_id,
        password_stdin,
        lifecycle,
    )
    .await?;
    Ok((resolved.project, resolved.task))
}

pub(crate) async fn resolve_task_and_project_with_detail(
    runtime: &RuntimeClient,
    project_selector: Option<&EntitySelector>,
    project_id: Option<Uuid>,
    task_selector: Option<&EntitySelector>,
    task_id: Option<Uuid>,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> PublicResult<ResolvedTaskAndProject> {
    let explicit_project = project_selector.is_some() || project_id.is_some();
    let current_project_id = if explicit_project {
        None
    } else {
        load_current_project(runtime.api_url())?
    };
    if explicit_project || current_project_id.is_some() {
        let project = resolve_project(
            runtime,
            project_selector,
            project_id.or(current_project_id),
            password_stdin,
            ProjectLifecycle::Any,
        )
        .await?;
        let (task, detail) = resolve_task_with_detail(
            runtime,
            project.id,
            task_selector,
            task_id,
            password_stdin,
            lifecycle,
        )
        .await?;
        return Ok(ResolvedTaskAndProject {
            project,
            task,
            detail,
        });
    }

    let selector = task_selector.ok_or_else(|| {
        PublicError::validation(
            "no project was selected; pass --project/--work-list-id, use a full task reference, or run 'sealtask pick project'",
        )
    })?;
    match selector.task_target()? {
        TaskSelectorTarget::FullReference {
            reference,
            reference_number,
        } if task_id.is_none() => {
            let detail = with_progress(
                "Resolving task reference across projects…",
                runtime.resolve_task_reference(reference, None, password_stdin),
            )
            .await?;
            verify_task_reference_response(
                detail.task.work_list_id,
                reference_number,
                detail.task.work_list_id,
                detail.task.reference_number,
            )?;
            if !task_matches_lifecycle(&detail.task, lifecycle) {
                return Err(PublicError::not_found(
                    "task reference resolved to a task outside the command's required lifecycle",
                ));
            }
            Ok(ResolvedTaskAndProject {
                project: ResolvedProject {
                    id: detail.task.work_list_id,
                    title: detail.task.work_list_title.clone(),
                },
                task: ResolvedEntity {
                    id: detail.task.id,
                    name: detail.task.title.clone(),
                },
                detail: Some(detail),
            })
        }
        TaskSelectorTarget::ProjectReferenceNumber(_) => Err(PublicError::validation(
            "a project-local #NUMBER task reference requires --project/--work-list-id or a current project",
        )),
        TaskSelectorTarget::Entity | TaskSelectorTarget::FullReference { .. } => {
            Err(PublicError::validation(
                "no project was selected; pass --project/--work-list-id or run 'sealtask pick project'",
            ))
        }
    }
}

pub(crate) async fn resolve_task(
    runtime: &RuntimeClient,
    project_id: Uuid,
    selector: Option<&EntitySelector>,
    exact_id: Option<Uuid>,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> PublicResult<ResolvedEntity> {
    resolve_task_with_detail(
        runtime,
        project_id,
        selector,
        exact_id,
        password_stdin,
        lifecycle,
    )
    .await
    .map(|(task, _)| task)
}

async fn resolve_task_with_detail(
    runtime: &RuntimeClient,
    project_id: Uuid,
    selector: Option<&EntitySelector>,
    exact_id: Option<Uuid>,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> PublicResult<(ResolvedEntity, Option<AgentTaskDetail>)> {
    if let Some(id) = exact_id {
        return Ok((ResolvedEntity { id, name: None }, None));
    }
    let selector = selector.ok_or_else(|| {
        PublicError::validation("a task target is required; pass TASK or --task-id")
    })?;
    if let Some(id) = selector.exact_id() {
        return Ok((ResolvedEntity { id, name: None }, None));
    }
    match selector.task_target()? {
        TaskSelectorTarget::FullReference {
            reference,
            reference_number,
        } => {
            let detail = with_progress(
                "Resolving task reference…",
                runtime.resolve_task_reference(reference, Some(project_id), password_stdin),
            )
            .await?;
            verify_task_reference_response(
                project_id,
                reference_number,
                detail.task.work_list_id,
                detail.task.reference_number,
            )?;
            if !task_matches_lifecycle(&detail.task, lifecycle) {
                return Err(task_reference_lifecycle_error());
            }
            return Ok((
                ResolvedEntity {
                    id: detail.task.id,
                    name: detail.task.title.clone(),
                },
                Some(detail),
            ));
        }
        TaskSelectorTarget::ProjectReferenceNumber(reference_number) => {
            return resolve_project_task_reference_number_with_detail(
                runtime,
                project_id,
                reference_number,
                password_stdin,
                lifecycle,
            )
            .await;
        }
        TaskSelectorTarget::Entity => {}
    }
    let tasks = with_progress(
        "Resolving task…",
        runtime.list_project_tasks(project_id, true, true, password_stdin),
    )
    .await?;
    let candidates = tasks
        .iter()
        .filter(|task| task_matches_lifecycle(task, lifecycle))
        .map(task_candidate)
        .collect();
    resolve_entity(
        "task",
        &format!("project {project_id}"),
        selector,
        candidates,
        &format!(
            "sealtask tasks list --project {project_id} --include-completed --include-archived"
        ),
    )
    .map(|task| (task, None))
}

async fn resolve_project_task_reference_number_with_detail(
    runtime: &RuntimeClient,
    project_id: Uuid,
    reference_number: i64,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> PublicResult<(ResolvedEntity, Option<AgentTaskDetail>)> {
    let detail = with_progress(
        "Resolving project task reference…",
        runtime.resolve_project_task_reference_number(project_id, reference_number, password_stdin),
    )
    .await?;
    verify_task_reference_response(
        project_id,
        reference_number,
        detail.task.work_list_id,
        detail.task.reference_number,
    )?;
    if !task_matches_lifecycle(&detail.task, lifecycle) {
        return Err(task_reference_lifecycle_error());
    }
    Ok((
        ResolvedEntity {
            id: detail.task.id,
            name: detail.task.title.clone(),
        },
        Some(detail),
    ))
}

fn task_reference_lifecycle_error() -> PublicError {
    PublicError::not_found("task reference resolved outside the command's required lifecycle")
}

fn verify_task_reference_response(
    expected_project_id: Uuid,
    expected_reference_number: i64,
    actual_project_id: Uuid,
    actual_reference_number: Option<i64>,
) -> PublicResult<()> {
    if actual_project_id != expected_project_id
        || actual_reference_number != Some(expected_reference_number)
    {
        return Err(PublicError::unexpected(
            "task reference lookup returned mismatched public metadata",
        ));
    }
    Ok(())
}

pub(crate) async fn resolve_note(
    runtime: &RuntimeClient,
    project_id: Uuid,
    selector: Option<&EntitySelector>,
    exact_id: Option<Uuid>,
    password_stdin: bool,
) -> PublicResult<ResolvedEntity> {
    if let Some(id) = exact_id {
        return Ok(ResolvedEntity { id, name: None });
    }
    let selector = selector.ok_or_else(|| {
        PublicError::validation("a note target is required; pass NOTE or --note-id")
    })?;
    if let Some(id) = selector.exact_id() {
        return Ok(ResolvedEntity { id, name: None });
    }
    let notes = with_progress(
        "Resolving note…",
        runtime.list_notes(project_id, password_stdin),
    )
    .await?;
    resolve_entity(
        "note",
        &format!("project {project_id}"),
        selector,
        notes.iter().map(note_candidate).collect(),
        &format!("sealtask notes list --project {project_id}"),
    )
}

pub(crate) async fn resolve_section(
    runtime: &RuntimeClient,
    project_id: Uuid,
    selector: &EntitySelector,
    password_stdin: bool,
) -> PublicResult<ResolvedEntity> {
    if let Some(id) = selector.exact_id() {
        return Ok(ResolvedEntity { id, name: None });
    }
    let project = load_project(runtime, project_id, password_stdin).await?;
    let sections = project_sections(&project)?;
    resolve_entity(
        "section",
        &format!("project {project_id}"),
        selector,
        section_candidates(&sections),
        &format!("sealtask projects sections list --project {project_id}"),
    )
}

pub(crate) async fn list_sections(
    runtime: &RuntimeClient,
    project_id: Uuid,
    password_stdin: bool,
) -> PublicResult<Vec<ProjectSection>> {
    let project = load_project(runtime, project_id, password_stdin).await?;
    project_sections(&project)
}

fn project_candidate(project: &&AgentWorkListSummary) -> EntityCandidate {
    EntityCandidate {
        id: project.id,
        name: project.title.clone(),
    }
}

fn task_candidate(task: &AgentTaskSummary) -> EntityCandidate {
    EntityCandidate {
        id: task.id,
        name: task.title.clone(),
    }
}

fn note_candidate(note: &AgentNote) -> EntityCandidate {
    EntityCandidate {
        id: note.id,
        name: note.title.clone(),
    }
}

fn project_matches_lifecycle(project: &AgentWorkListSummary, lifecycle: ProjectLifecycle) -> bool {
    match lifecycle {
        ProjectLifecycle::Any => true,
        ProjectLifecycle::Active => project.archived_at.is_none(),
        ProjectLifecycle::Archived => project.archived_at.is_some(),
    }
}

fn project_lifecycle_scope(lifecycle: ProjectLifecycle) -> &'static str {
    match lifecycle {
        ProjectLifecycle::Any => "accessible projects",
        ProjectLifecycle::Active => "active projects",
        ProjectLifecycle::Archived => "archived projects",
    }
}

fn task_matches_lifecycle(task: &AgentTaskSummary, lifecycle: TaskLifecycle) -> bool {
    match lifecycle {
        TaskLifecycle::Any => true,
        TaskLifecycle::Active => task.archived_at.is_none(),
        TaskLifecycle::Archived => task.archived_at.is_some(),
        TaskLifecycle::Incomplete => task.archived_at.is_none() && !task.is_completed,
        TaskLifecycle::Completed => task.archived_at.is_none() && task.is_completed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_reference_endpoint_metadata_must_match_requested_project_and_number() {
        let expected_project_id = Uuid::from_u128(1);
        let other_project_id = Uuid::from_u128(2);

        verify_task_reference_response(expected_project_id, 184, expected_project_id, Some(184))
            .expect("matching metadata");
        assert!(
            verify_task_reference_response(expected_project_id, 184, other_project_id, Some(184),)
                .is_err()
        );
        assert!(
            verify_task_reference_response(
                expected_project_id,
                184,
                expected_project_id,
                Some(185),
            )
            .is_err()
        );
        assert!(
            verify_task_reference_response(expected_project_id, 184, expected_project_id, None)
                .is_err()
        );
    }
}
