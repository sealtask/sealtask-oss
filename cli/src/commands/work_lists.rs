use crate::args::{ProjectContextScopeArg, ProjectSectionsCommand, ProjectsCommand};
use crate::commands::audit_output::print_audit_page;
use crate::output::{
    CliResult, OutputFormat, mutation_output_enabled, print_json, print_simple_result,
};
use crate::project_context::{
    ProjectContextMutation, ProjectContextScope, ResolvedProjectContext, clear_current_project,
    load_project_context, save_current_project,
};
use crate::render::{
    print_empty_collection, print_project_sections, print_raw_work_list_detail,
    print_raw_work_lists, print_stats, print_user, print_work_list_detail, print_work_lists,
};
use crate::resolver::{ProjectLifecycle, ResolvedProject, list_sections, resolve_project};
use crate::table::sanitize_cell;
use crate::terminal::with_progress;
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
    scope: Option<ProjectContextScope>,
    directory: Option<String>,
    inherited: bool,
    profile: String,
    api_base_url: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectContextMutationResult {
    project_id: Option<Uuid>,
    changed: bool,
    scope: ProjectContextScope,
    directory: Option<String>,
    inherited: bool,
    profile: String,
}

pub(crate) async fn run_me(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let user = with_progress("Loading account…", runtime.get_me()).await?;
    print_user(&user, format)
}

pub(crate) async fn run_stats(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let stats = with_progress("Loading account statistics…", runtime.get_stats()).await?;
    print_stats(&stats, format)
}

pub(crate) async fn run_projects(
    runtime: &RuntimeClient,
    format: OutputFormat,
    legacy_verbose: bool,
    include_archived: bool,
    password_stdin: bool,
    raw: bool,
    command: Option<ProjectsCommand>,
) -> CliResult<()> {
    match command {
        Some(ProjectsCommand::List {
            details,
            include_archived: command_include_archived,
            password_stdin: command_password_stdin,
            raw: command_raw,
        }) => {
            list_projects(
                runtime,
                format,
                legacy_verbose || details,
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
                ("--verbose", legacy_verbose),
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
                ("--verbose", legacy_verbose),
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
                ("--verbose", legacy_verbose),
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
        Some(ProjectsCommand::Current { scope }) => {
            reject_project_options(&[
                ("--verbose", legacy_verbose),
                ("--include-archived", include_archived),
                ("--password-stdin", password_stdin),
                ("--raw", raw),
            ])?;
            let requested_scope = scope.map(context_scope);
            let context = load_project_context(runtime.api_url(), requested_scope)?;
            let project_id = context.as_ref().map(|context| context.project_id);
            let result = CurrentProjectResult {
                schema_version: 1,
                project_id,
                scope: context
                    .as_ref()
                    .map(|context| context.scope)
                    .or(requested_scope),
                directory: context_directory(context.as_ref()),
                inherited: context.as_ref().is_some_and(|context| context.inherited),
                profile: active_profile()?,
                api_base_url: runtime.api_url(),
            };
            match format {
                OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => print_json(
                    &result,
                    format,
                    "serializing current-project status should succeed",
                ),
                OutputFormat::Table => match project_id {
                    Some(project_id) => {
                        println!("Current project: {project_id}");
                        if let Some(context) = context.as_ref() {
                            println!("Scope: {}", context_scope_label(context));
                        }
                        Ok(())
                    }
                    None => {
                        let scope = requested_scope
                            .map_or("the local/global context hierarchy".to_string(), |scope| {
                                format!("the {} scope", scope_name(scope))
                            });
                        println!("No current project in {scope}.\nNext: sealtask pick project");
                        Ok(())
                    }
                },
            }
        }
        Some(ProjectsCommand::Clear { scope }) => {
            reject_project_options(&[
                ("--verbose", legacy_verbose),
                ("--include-archived", include_archived),
                ("--password-stdin", password_stdin),
                ("--raw", raw),
            ])?;
            let mutation = clear_current_project(scope.map(context_scope))?;
            let profile = active_profile()?;
            let scope_label = mutation_scope_label(&mutation, &profile);
            let outcome = if mutation.changed {
                format!("Cleared the current project from {scope_label}.")
            } else {
                format!("No current project was saved in {scope_label}.")
            };
            let message = format!(
                "{outcome}\nOther context layers remain unchanged.\nNext: sealtask projects current"
            );
            print_simple_result(
                format,
                &ProjectContextMutationResult {
                    project_id: None,
                    changed: mutation.changed,
                    scope: mutation.scope,
                    directory: mutation_directory(&mutation),
                    inherited: mutation.inherited,
                    profile,
                },
                "serializing current-project clear result should succeed",
                &message,
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
                ("--verbose", legacy_verbose),
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
        Some(ProjectsCommand::Audit {
            project,
            work_list_id,
            cursor,
            limit,
            password_stdin: command_password_stdin,
        }) => {
            reject_project_options(&[
                ("--verbose", legacy_verbose),
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
            let mut client = runtime.authenticated_api_client()?;
            let page = with_progress(
                "Loading project audit log…",
                client.get_work_list_audit_log(project.id, cursor, limit),
            )
            .await?;
            print_audit_page(project.id, &page, format)
        }
        None => {
            list_projects(
                runtime,
                format,
                legacy_verbose,
                include_archived,
                password_stdin,
                raw,
            )
            .await
        }
    }
}

pub(crate) async fn activate_project(
    runtime: &RuntimeClient,
    format: OutputFormat,
    project: ResolvedProject,
    scope: Option<ProjectContextScope>,
) -> CliResult<()> {
    let mut client = runtime.authenticated_api_client()?;
    let target = with_progress("Loading project…", client.get_work_list(project.id)).await?;
    if target.work_list.archived_at.is_some() {
        return Err(PublicError::validation(
            "an archived project cannot be selected as current; restore it first with 'sealtask projects unarchive <PROJECT>'",
        )
        .into());
    }

    let mutation = save_current_project(runtime.api_url(), project.id, scope)?;
    let profile = active_profile()?;
    let label = human_project_label(project.title.as_deref(), project.id);
    let selection = if mutation.changed {
        format!("Current project: {label}")
    } else {
        format!("Current project already selected: {label}")
    };
    let message = format!(
        "{selection}\nScope: {}\nNext: sealtask tasks list",
        mutation_scope_label(&mutation, &profile)
    );
    print_simple_result(
        format,
        &ProjectContextMutationResult {
            project_id: Some(project.id),
            changed: mutation.changed,
            scope: mutation.scope,
            directory: mutation_directory(&mutation),
            inherited: mutation.inherited,
            profile,
        },
        "serializing current-project result should succeed",
        &message,
    )
}

fn human_project_label(title: Option<&str>, project_id: Uuid) -> String {
    title.map_or_else(
        || project_id.to_string(),
        |title| format!("\"{}\" ({project_id})", sanitize_cell(title)),
    )
}

fn context_scope(scope: ProjectContextScopeArg) -> ProjectContextScope {
    match scope {
        ProjectContextScopeArg::Local => ProjectContextScope::Local,
        ProjectContextScopeArg::Global => ProjectContextScope::Global,
    }
}

fn context_directory(context: Option<&ResolvedProjectContext>) -> Option<String> {
    context
        .and_then(|context| context.directory.as_ref())
        .map(|directory| directory.display().to_string())
}

fn mutation_directory(mutation: &ProjectContextMutation) -> Option<String> {
    mutation
        .directory
        .as_ref()
        .map(|directory| directory.display().to_string())
}

fn context_scope_label(context: &ResolvedProjectContext) -> String {
    match context.scope {
        ProjectContextScope::Local => {
            let directory = context
                .directory
                .as_ref()
                .map(|directory| sanitize_cell(&directory.display().to_string()))
                .unwrap_or_else(|| "<unknown directory>".to_string());
            if context.inherited {
                format!("local ({directory}, inherited)")
            } else {
                format!("local ({directory})")
            }
        }
        ProjectContextScope::Global => "global (active profile fallback)".to_string(),
    }
}

fn mutation_scope_label(mutation: &ProjectContextMutation, profile: &str) -> String {
    match mutation.scope {
        ProjectContextScope::Local => mutation
            .directory
            .as_ref()
            .map(|directory| {
                format!(
                    "local ({})",
                    sanitize_cell(&directory.display().to_string())
                )
            })
            .unwrap_or_else(|| "local".to_string()),
        ProjectContextScope::Global => {
            format!("global (profile \"{}\")", sanitize_cell(profile))
        }
    }
}

fn scope_name(scope: ProjectContextScope) -> &'static str {
    match scope {
        ProjectContextScope::Local => "local",
        ProjectContextScope::Global => "global",
    }
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
        let lists = with_progress(
            "Loading projects…",
            client.list_work_lists_with_archived(include_archived),
        )
        .await?;
        if lists.is_empty() {
            return print_empty_collection(format, "No projects found.");
        }
        return print_raw_work_lists(&lists, format, verbose);
    }

    let lists = with_progress(
        "Loading and decrypting projects…",
        runtime.list_work_lists_with_archived(password_stdin, include_archived),
    )
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
        let result = if archive {
            with_progress("Archiving project…", client.archive_work_list(work_list_id)).await
        } else {
            with_progress(
                "Restoring project…",
                client.unarchive_work_list(work_list_id),
            )
            .await
        };
        runtime.invalidate_read_cache_for_mutation_result(&result);
        let work_list = result?;
        return if mutation_output_enabled(format) {
            print_raw_work_lists(std::slice::from_ref(&work_list), format, true)
        } else {
            Ok(())
        };
    }

    let work_list = if archive {
        with_progress(
            "Archiving project…",
            runtime.archive_work_list(work_list_id, password_stdin),
        )
        .await?
    } else {
        with_progress(
            "Restoring project…",
            runtime.unarchive_work_list(work_list_id, password_stdin),
        )
        .await?
    };
    if mutation_output_enabled(format) {
        print_work_lists(std::slice::from_ref(&work_list), format, true)
    } else {
        Ok(())
    }
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
        let detail = with_progress("Loading project…", client.get_work_list(work_list_id)).await?;
        return print_raw_work_list_detail(&detail, format);
    }

    let detail = with_progress(
        "Loading and decrypting project…",
        runtime.get_work_list(work_list_id, password_stdin),
    )
    .await?;
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
