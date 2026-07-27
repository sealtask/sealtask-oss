use crate::project_context::load_current_project;
use crate::selectors::{
    EntityCandidate, EntitySelector, ProjectSection, ResolvedEntity, project_sections,
    resolve_entity, section_candidates,
};
use crate::terminal::with_progress;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{AgentNote, AgentTaskSummary, AgentWorkListSummary, RuntimeClient};
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

pub(crate) async fn resolve_task(
    runtime: &RuntimeClient,
    project_id: Uuid,
    selector: Option<&EntitySelector>,
    exact_id: Option<Uuid>,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> PublicResult<ResolvedEntity> {
    if let Some(id) = exact_id {
        return Ok(ResolvedEntity { id, name: None });
    }
    let selector = selector.ok_or_else(|| {
        PublicError::validation("a task target is required; pass TASK or --task-id")
    })?;
    if let Some(id) = selector.exact_id() {
        return Ok(ResolvedEntity { id, name: None });
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
