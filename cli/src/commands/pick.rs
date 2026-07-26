use crate::args::PickCommand;
use crate::output::{CliResult, write_stdout_line};
use crate::picker::{PickerCandidate, pick_candidate, selector_for};
use crate::resolver::{ProjectLifecycle, resolve_project};
use crate::terminal::with_progress;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::RuntimeClient;

pub(crate) async fn run_pick(runtime: &RuntimeClient, command: PickCommand) -> CliResult<()> {
    let selected = match command {
        PickCommand::Project {
            include_archived,
            password_stdin,
        } => pick_project(runtime, include_archived, password_stdin).await?,
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
                .map(|task| PickerCandidate::new(task.id, task.title))
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
            pick_candidate("task", candidates)?
        }
    };

    write_stdout_line(format_args!("{}", selector_for(selected)))
}

async fn pick_project(
    runtime: &RuntimeClient,
    include_archived: bool,
    password_stdin: bool,
) -> CliResult<uuid::Uuid> {
    let projects = with_progress(
        "Loading and decrypting projects to pick…",
        runtime.list_work_lists_with_archived(password_stdin, include_archived),
    )
    .await?;
    let candidates = projects
        .into_iter()
        .filter(|project| include_archived || project.archived_at.is_none())
        .map(|project| PickerCandidate::new(project.id, project.title))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        let discovery = if include_archived {
            "sealtask projects list --include-archived"
        } else {
            "sealtask pick project --include-archived"
        };
        return Err(PublicError::validation(format!(
            "no accessible projects are available to pick; run '{discovery}'"
        ))
        .into());
    }
    pick_candidate("project", candidates)
}
