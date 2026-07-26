use crate::args::{ProjectSectionsCommand, ProjectsCommand};
use crate::output::{CliResult, OutputFormat, print_json, print_simple_result};
use crate::project_context::{clear_current_project, load_current_project, save_current_project};
use crate::render::{
    print_empty_collection, print_project_sections, print_raw_work_list_detail,
    print_raw_work_lists, print_stats, print_user, print_work_list_detail, print_work_lists,
};
use crate::resolver::{ProjectLifecycle, list_sections, resolve_project};
use crate::table::sanitize_cell;
use sealtask_client_auth::active_profile;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::RuntimeClient;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentProjectResult<'a> {
    schema_version: u8,
    project_id: Option<Uuid>,
    profile: String,
    api_base_url: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectContextMutationResult {
    project_id: Option<Uuid>,
    changed: bool,
}

pub(crate) async fn run_me(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let user = runtime.get_me().await?;
    print_user(&user, format)
}

pub(crate) async fn run_stats(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let stats = runtime.get_stats().await?;
    print_stats(&stats, format)
}

pub(crate) async fn run_projects(
    runtime: &RuntimeClient,
    format: OutputFormat,
    verbose: bool,
    include_archived: bool,
    password_stdin: bool,
    raw: bool,
    command: Option<ProjectsCommand>,
) -> CliResult<()> {
    match command {
        Some(ProjectsCommand::List {
            verbose: command_verbose,
            include_archived: command_include_archived,
            password_stdin: command_password_stdin,
            raw: command_raw,
        }) => {
            list_projects(
                runtime,
                format,
                verbose || command_verbose,
                include_archived || command_include_archived,
                password_stdin || command_password_stdin,
                raw || command_raw,
            )
            .await
        }
        Some(ProjectsCommand::Get {
            project,
            password_stdin: command_password_stdin,
            raw: command_raw,
        }) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
            ])?;
            let password_stdin = password_stdin || command_password_stdin;
            let project = resolve_project(
                runtime,
                Some(&project),
                None,
                password_stdin,
                ProjectLifecycle::Any,
            )
            .await?;
            run_lists_get(
                runtime,
                format,
                project.id,
                password_stdin,
                raw || command_raw,
            )
            .await
        }
        Some(ProjectsCommand::Archive {
            project,
            password_stdin: command_password_stdin,
            raw: command_raw,
        }) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
            ])?;
            let password_stdin = password_stdin || command_password_stdin;
            let project = resolve_project(
                runtime,
                Some(&project),
                None,
                password_stdin,
                ProjectLifecycle::Active,
            )
            .await?;
            run_project_lifecycle(
                runtime,
                format,
                project.id,
                password_stdin,
                raw || command_raw,
                true,
            )
            .await
        }
        Some(ProjectsCommand::Unarchive {
            project,
            password_stdin: command_password_stdin,
            raw: command_raw,
        }) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
            ])?;
            let password_stdin = password_stdin || command_password_stdin;
            let project = resolve_project(
                runtime,
                Some(&project),
                None,
                password_stdin,
                ProjectLifecycle::Archived,
            )
            .await?;
            run_project_lifecycle(
                runtime,
                format,
                project.id,
                password_stdin,
                raw || command_raw,
                false,
            )
            .await
        }
        Some(ProjectsCommand::Use {
            project,
            password_stdin: command_password_stdin,
        }) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
                ("--raw", raw),
            ])?;
            let password_stdin = password_stdin || command_password_stdin;
            let project = resolve_project(
                runtime,
                Some(&project),
                None,
                password_stdin,
                ProjectLifecycle::Active,
            )
            .await?;
            let mut client = runtime.authenticated_api_client()?;
            let target = client.get_work_list(project.id).await?;
            if target.work_list.archived_at.is_some() {
                return Err(PublicError::validation(
                    "an archived project cannot be selected as current; restore it first with 'sealtask projects unarchive <PROJECT>'",
                )
                .into());
            }
            let changed = save_current_project(runtime.api_url(), project.id)?;
            let label = human_project_label(project.title.as_deref(), project.id);
            let message = if changed {
                format!("Current project: {label}")
            } else {
                format!("Current project already selected: {label}")
            };
            print_simple_result(
                format,
                &ProjectContextMutationResult {
                    project_id: Some(project.id),
                    changed,
                },
                "serializing current-project result should succeed",
                &message,
            )
        }
        Some(ProjectsCommand::Current) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
                ("--password-stdin", password_stdin),
                ("--raw", raw),
            ])?;
            let project_id = load_current_project(runtime.api_url())?;
            let result = CurrentProjectResult {
                schema_version: 1,
                project_id,
                profile: active_profile()?,
                api_base_url: runtime.api_url(),
            };
            match format {
                OutputFormat::Json | OutputFormat::JsonPretty => print_json(
                    &result,
                    format,
                    "serializing current-project status should succeed",
                ),
                OutputFormat::Table => match project_id {
                    Some(project_id) => {
                        println!("Current project: {project_id}");
                        Ok(())
                    }
                    None => {
                        println!(
                            "No current project. Run 'sealtask projects use <PROJECT>' to select one."
                        );
                        Ok(())
                    }
                },
            }
        }
        Some(ProjectsCommand::Clear) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
                ("--password-stdin", password_stdin),
                ("--raw", raw),
            ])?;
            let changed = clear_current_project()?;
            print_simple_result(
                format,
                &ProjectContextMutationResult {
                    project_id: None,
                    changed,
                },
                "serializing current-project clear result should succeed",
                if changed {
                    "Cleared the current project."
                } else {
                    "No current project was saved."
                },
            )
        }
        Some(ProjectsCommand::Sections {
            command:
                ProjectSectionsCommand::List {
                    project,
                    work_list_id,
                    password_stdin: command_password_stdin,
                },
        }) => {
            reject_project_options(&[
                ("--verbose", verbose),
                ("--include-archived", include_archived),
                ("--raw", raw),
            ])?;
            let password_stdin = password_stdin || command_password_stdin;
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Any,
            )
            .await?;
            let sections = list_sections(runtime, project.id, password_stdin).await?;
            if sections.is_empty() {
                return print_empty_collection(format, "No sections found in this project.");
            }
            print_project_sections(&sections, format)
        }
        None => {
            list_projects(
                runtime,
                format,
                verbose,
                include_archived,
                password_stdin,
                raw,
            )
            .await
        }
    }
}

fn human_project_label(title: Option<&str>, project_id: Uuid) -> String {
    title.map_or_else(
        || project_id.to_string(),
        |title| format!("\"{}\" ({project_id})", sanitize_cell(title)),
    )
}

fn reject_project_options(options: &[(&str, bool)]) -> PublicResult<()> {
    let invalid = options
        .iter()
        .filter_map(|(name, present)| present.then_some(*name))
        .collect::<Vec<_>>();
    if invalid.is_empty() {
        return Ok(());
    }
    Err(PublicError::validation(format!(
        "{} cannot be used with this projects subcommand",
        invalid.join(", ")
    )))
}

async fn list_projects(
    runtime: &RuntimeClient,
    format: OutputFormat,
    verbose: bool,
    include_archived: bool,
    password_stdin: bool,
    raw: bool,
) -> CliResult<()> {
    if raw {
        let mut client = runtime.authenticated_api_client()?;
        let lists = client
            .list_work_lists_with_archived(include_archived)
            .await?;
        if lists.is_empty() {
            return print_empty_collection(format, "No projects found.");
        }
        return print_raw_work_lists(&lists, format, verbose);
    }

    let lists = runtime
        .list_work_lists_with_archived(password_stdin, include_archived)
        .await?;
    if lists.is_empty() {
        return print_empty_collection(format, "No projects found.");
    }
    print_work_lists(&lists, format, verbose)
}

async fn run_project_lifecycle(
    runtime: &RuntimeClient,
    format: OutputFormat,
    work_list_id: Uuid,
    password_stdin: bool,
    raw: bool,
    archive: bool,
) -> CliResult<()> {
    if raw {
        let mut client = runtime.authenticated_api_client()?;
        let work_list = if archive {
            client.archive_work_list(work_list_id).await?
        } else {
            client.unarchive_work_list(work_list_id).await?
        };
        return print_raw_work_lists(std::slice::from_ref(&work_list), format, true);
    }

    let work_list = if archive {
        runtime
            .archive_work_list(work_list_id, password_stdin)
            .await?
    } else {
        runtime
            .unarchive_work_list(work_list_id, password_stdin)
            .await?
    };
    print_work_lists(std::slice::from_ref(&work_list), format, true)
}

pub(crate) async fn run_lists_get(
    runtime: &RuntimeClient,
    format: OutputFormat,
    work_list_id: Uuid,
    password_stdin: bool,
    raw: bool,
) -> CliResult<()> {
    if raw {
        let mut client = runtime.authenticated_api_client()?;
        let detail = client.get_work_list(work_list_id).await?;
        return print_raw_work_list_detail(&detail, format);
    }

    let detail = runtime.get_work_list(work_list_id, password_stdin).await?;
    print_work_list_detail(&detail, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_project_labels_strip_terminal_and_bidi_controls() {
        let project_id = Uuid::from_u128(42);
        let title = "safe\nforged\u{1b}[31m\u{202e}reversed\u{202c}";

        let label = human_project_label(Some(title), project_id);

        assert_eq!(label, format!("\"safe forged[31mreversed\" ({project_id})"));
    }
}
