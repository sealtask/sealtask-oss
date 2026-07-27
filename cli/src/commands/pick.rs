use super::work_lists::activate_project;
use crate::args::{PickCommand, ProjectContextScopeArg};
use crate::output::{CliResult, OutputFormat, write_stdout_line};
use crate::picker::{PickerCandidate, pick_candidate, selector_for};
use crate::project_context::ProjectContextScope;
use crate::render::task_reference_title_label;
use crate::resolver::{ProjectLifecycle, ResolvedProject, resolve_project};
use crate::terminal::with_progress;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::RuntimeClient;

pub(crate) async fn run_pick(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: PickCommand,
) -> CliResult<()> {
    match command {
        PickCommand::Project {
            project,
            include_archived,
            scope,
            print_selector,
            password_stdin,
        } => {
            if print_selector {
                let project = pick_project(runtime, include_archived, password_stdin).await?;
                write_stdout_line(format_args!("{}", selector_for(project.id)))
            } else {
                let project = match project.as_ref() {
                    Some(project) => {
                        resolve_project(
                            runtime,
                            Some(project),
                            None,
                            password_stdin,
                            ProjectLifecycle::Active,
                        )
                        .await?
                    }
                    None => pick_project(runtime, false, password_stdin).await?,
                };
                activate_project(runtime, format, project, scope.map(context_scope)).await
            }
        }
        PickCommand::Task {
            project,
            work_list_id,
            include_completed,
            include_archived,
            password_stdin,
        } => {
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Any,
            )
            .await?;
            let tasks = with_progress(
                "Loading and decrypting tasks to pick…",
                runtime.list_project_tasks(
                    project.id,
                    include_completed,
                    include_archived,
                    password_stdin,
                ),
            )
            .await?;
            let candidates = tasks
                .into_iter()
                .map(|task| {
                    let label = task_reference_title_label(&task);
                    PickerCandidate::new(task.id, Some(label))
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                let discovery = if include_completed && include_archived {
                    "sealtask tasks create --title <TITLE>"
                } else {
                    "sealtask pick task --include-completed --include-archived"
                };
                return Err(PublicError::validation(format!(
                    "no matching tasks are available to pick in project {}; run '{discovery}'",
                    project.id
                ))
                .into());
            }
            let selected = pick_candidate("task", candidates)?;
            write_stdout_line(format_args!("{}", selector_for(selected)))
        }
    }
}

fn context_scope(scope: ProjectContextScopeArg) -> ProjectContextScope {
    match scope {
        ProjectContextScopeArg::Local => ProjectContextScope::Local,
        ProjectContextScopeArg::Global => ProjectContextScope::Global,
    }
}

async fn pick_project(
    runtime: &RuntimeClient,
    include_archived: bool,
    password_stdin: bool,
) -> CliResult<ResolvedProject> {
    let projects = with_progress(
        "Loading and decrypting projects to pick…",
        runtime.list_work_lists_with_archived(password_stdin, include_archived),
    )
    .await?;
    let candidates = projects
        .iter()
        .filter(|project| include_archived || project.archived_at.is_none())
        .map(|project| PickerCandidate::new(project.id, project.title.clone()))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(PublicError::validation(
            "no accessible projects are available to pick; run 'sealtask projects list --include-archived'",
        )
        .into());
    }
    let id = pick_candidate("project", candidates)?;
    let title = projects
        .into_iter()
        .find(|project| project.id == id)
        .and_then(|project| project.title);
    Ok(ResolvedProject { id, title })
}
