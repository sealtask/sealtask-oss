use crate::args::{
    TaskArchiveArgsCli, TaskAttachmentsCommand, TaskCompletionArgsCli, TaskCreateArgsCli,
    TaskDeleteArgsCli, TaskEditArgsCli, TaskMoveArgsCli, TaskReferencesCommand,
    TaskUnarchiveArgsCli, TaskUpdateArgsCli, TasksCommand,
};
use crate::attachment_output::{resolve_attachment_output_path, write_attachment_file};
use crate::editor;
use crate::human_input::parse_due_input;
use crate::input::{
    read_required_password, resolve_body_input, resolve_delete_input, resolve_task_create_input,
    resolve_task_update_input, validate_body_input,
};
use crate::interaction::require_confirmation;
use crate::interruption::SignalMonitor;
use crate::output::{
    CliError, CliResult, OutputFormat, WarningResult, finish_with_warnings, print_json,
    warning_result,
};
use crate::render::{
    print_attachment, print_delete_result, print_download_result, print_empty_collection,
    print_raw_my_tasks, print_raw_task_detail, print_raw_tasks, print_readable_attachment,
    print_task, print_task_detail, print_task_reference_scheme_result,
};
use crate::resolver::{
    ProjectLifecycle, TaskLifecycle, load_project, resolve_optional_project, resolve_project,
    resolve_section, resolve_task,
};
use crate::selectors::{EntitySelector, IdSelector, ResolvedEntity, resolve_id_selector};
use crate::task_list::{TaskListOptions, TaskListScope, print_task_list};
use crate::terminal::{self, ProgressGuard, with_progress};
use chrono::Utc;
use sealtask_client_api::DeleteTaskRequest;
use sealtask_client_core::PublicError;
use sealtask_client_core::PublicResult;
use sealtask_client_runtime::{
    ArchiveTaskArgs, AttachmentUploadPassword, CreateTaskArgs, DeleteTaskArgs,
    DeleteTaskAttachmentArgs, MoveTaskArgs, MoveTaskInput, OperationCancellation,
    QuarantineTaskReferenceSchemeArgs, RepairTaskReferenceSchemeArgs, RuntimeClient,
    TaskCompletionArgs, TaskFieldPatch, TaskMutationPlan, UnarchiveTaskArgs, UpdateTaskArgs,
    UploadTaskAttachmentArgs,
};
use serde_json::json;
use std::future::Future;
use std::io;
use std::time::Duration;

const ATTACHMENT_UPLOAD_CANCELLATION_GRACE: Duration = Duration::from_secs(5);

pub(crate) async fn run_tasks(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    command: TasksCommand,
) -> CliResult<()> {
    match command {
        TasksCommand::List {
            project,
            work_list_id,
            include_completed,
            include_archived,
            all,
            columns,
            sort,
            field,
            web_url,
            password_stdin,
            raw,
        } => {
            let explicit_project = project.is_some() || work_list_id.is_some();
            let resolved_project = if all {
                None
            } else {
                resolve_optional_project(runtime, project.as_ref(), work_list_id, password_stdin)
                    .await?
            };
            let work_list_id = resolved_project.as_ref().map(|project| project.id);
            let scope = match resolved_project.as_ref() {
                Some(project) if explicit_project => TaskListScope::Selected(project.clone()),
                Some(project) => TaskListScope::Current(project.clone()),
                None => TaskListScope::AcrossProjects,
            };
            if include_archived && work_list_id.is_none() {
                return Err(PublicError::validation(
                    "--include-archived requires --project, --work-list-id, or a current project; run 'sealtask projects use <PROJECT>' first",
                )
                .into());
            }
            if raw {
                let mut client = runtime.authenticated_api_client()?;
                if all || work_list_id.is_none() {
                    let tasks =
                        with_progress("Loading tasks…", client.get_all_my_tasks(include_completed))
                            .await?;
                    if tasks.is_empty() {
                        return print_empty_collection(format, "No tasks found.");
                    }
                    return print_raw_my_tasks(&tasks, format);
                }

                let work_list_id = work_list_id.expect("validated work list id");
                let response = with_progress(
                    "Loading project tasks…",
                    client.get_tasks(work_list_id, include_archived),
                )
                .await?;
                let tasks: Vec<_> = if include_completed {
                    response.tasks
                } else {
                    response
                        .tasks
                        .into_iter()
                        .filter(|task| !task.is_completed)
                        .collect()
                };
                if tasks.is_empty() {
                    return print_empty_collection(format, "No tasks found in this project.");
                }
                return print_raw_tasks(&tasks, format);
            }

            let tasks = if let Some(work_list_id) = work_list_id {
                with_progress(
                    "Loading and decrypting project tasks…",
                    runtime.list_project_tasks(
                        work_list_id,
                        include_completed,
                        include_archived,
                        password_stdin,
                    ),
                )
                .await?
            } else {
                with_progress(
                    "Loading and decrypting tasks…",
                    runtime.list_tasks(None, include_completed, all, password_stdin),
                )
                .await?
            };
            print_task_list(
                tasks,
                format,
                TaskListOptions {
                    columns: &columns,
                    sort,
                    field,
                    web_url: web_url.as_deref(),
                    api_url: runtime.api_url(),
                    include_completed,
                    include_archived,
                    scope,
                },
            )
        }
        TasksCommand::Get {
            task,
            task_id,
            project,
            work_list_id,
            password_stdin,
            raw,
        } => {
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Any,
            )
            .await?;
            let task = resolve_task(
                runtime,
                project.id,
                task.as_ref(),
                task_id,
                password_stdin,
                TaskLifecycle::Any,
            )
            .await?;
            if raw {
                let mut client = runtime.authenticated_api_client()?;
                let detail =
                    with_progress("Loading task…", client.get_task(project.id, task.id)).await?;
                return print_raw_task_detail(&detail, format);
            }

            let detail = with_progress(
                "Loading and decrypting task…",
                runtime.get_task(project.id, task.id, password_stdin),
            )
            .await?;
            print_task_detail(&detail, format)
        }
        TasksCommand::Resolve(args) => {
            let detail = runtime
                .resolve_task_reference(&args.reference, args.work_list_id, args.password_stdin)
                .await?;
            print_task_detail(&detail, format)
        }
        TasksCommand::TaskReferences { command } => match command {
            TaskReferencesCommand::Status(args) => {
                let status = runtime
                    .inspect_task_reference_schemes(args.work_list_id, args.password_stdin)
                    .await?;
                crate::render::print_task_reference_history_status(&status, format)
            }
            TaskReferencesCommand::Repair(args) => {
                let response = runtime
                    .repair_task_reference_scheme(RepairTaskReferenceSchemeArgs {
                        work_list_id: args.work_list_id,
                        prefix: args.prefix,
                        minimum_digits: args.minimum_digits,
                        password_stdin: args.password_stdin,
                    })
                    .await?;
                print_task_reference_scheme_result(
                    &response,
                    format,
                    "Installed owner task-reference repair.",
                )
            }
            TaskReferencesCommand::Quarantine(args) => {
                if !args.confirm {
                    return Err(PublicError::validation(
                        "quarantine is irreversible; pass --confirm after verifying the exact historical scheme revision",
                    )
                    .into());
                }
                let response = runtime
                    .quarantine_task_reference_scheme(QuarantineTaskReferenceSchemeArgs {
                        work_list_id: args.work_list_id,
                        scheme_revision_id: args.scheme_revision_id,
                        password_stdin: args.password_stdin,
                    })
                    .await?;
                print_task_reference_scheme_result(
                    &response,
                    format,
                    "Quarantined unreadable historical task-reference scheme.",
                )
            }
        },
        TasksCommand::Watch {
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
            super::streams::watch_tasks(
                runtime,
                format,
                project.id,
                include_completed,
                include_archived,
                password_stdin,
            )
            .await
        }
        TasksCommand::Create(args) => create_task(runtime, format, non_interactive, args).await,
        TasksCommand::Edit(args) => edit_task(runtime, format, non_interactive, args).await,
        TasksCommand::Update(args) => update_task(runtime, format, args).await,
        TasksCommand::Move(args) => move_task(runtime, format, args).await,
        TasksCommand::Complete(args) => set_task_completion(runtime, format, args, true).await,
        TasksCommand::Reopen(args) => set_task_completion(runtime, format, args, false).await,
        TasksCommand::Archive(args) => archive_task(runtime, format, args).await,
        TasksCommand::Unarchive(args) => unarchive_task(runtime, format, args).await,
        TasksCommand::Delete(args) => delete_task(runtime, format, non_interactive, args).await,
        TasksCommand::Attachments { command } => {
            run_task_attachments(runtime, format, non_interactive, command).await
        }
    }
}

async fn run_task_attachments(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    command: TaskAttachmentsCommand,
) -> CliResult<()> {
    match command {
        TaskAttachmentsCommand::Upload(args) => {
            let selector_discovery = selector_needs_discovery(args.project.as_ref())
                || selector_needs_discovery(args.task.as_ref());
            let (project_id, task) = resolve_task_target(
                runtime,
                args.project.as_ref(),
                args.work_list_id,
                args.task.as_ref(),
                args.task_id,
                args.password_stdin,
                TaskLifecycle::Any,
            )
            .await?;
            let password = if selector_discovery {
                None
            } else {
                resolve_attachment_upload_password(args.password_stdin)?
            };
            let cancellation = OperationCancellation::new();
            let progress = ProgressGuard::start("Uploading and linking attachment…");
            let signal_monitor = SignalMonitor::start()?;
            let supervised = supervise_attachment_upload(
                runtime.upload_task_attachment_with_cancellation(
                    UploadTaskAttachmentArgs {
                        work_list_id: project_id,
                        task_id: task.id,
                        path: args.file,
                        file_name: args.file_name,
                        content_type: args.content_type,
                        password,
                    },
                    cancellation.clone(),
                ),
                signal_monitor.subscribe().wait_for(1),
                signal_monitor.subscribe().wait_for(2),
                cancellation,
                ATTACHMENT_UPLOAD_CANCELLATION_GRACE,
                Some(&progress),
            )
            .await;
            drop(progress);
            let result = match supervised.outcome {
                AttachmentUploadOutcome::Completed(Ok(attachment)) => {
                    print_attachment(&attachment, format)
                }
                AttachmentUploadOutcome::Completed(Err(error)) => Err(error.into()),
                AttachmentUploadOutcome::Interrupted {
                    message,
                    outcome_ambiguous,
                } => {
                    return Err(if outcome_ambiguous {
                        CliError::interrupted_ambiguous(message, &supervised.warnings)
                    } else {
                        CliError::interrupted(message, &supervised.warnings)
                    });
                }
            };
            finish_with_warnings(format, &supervised.warnings, result)
        }
        TaskAttachmentsCommand::Delete(args) => {
            let (project_id, task) = resolve_task_target(
                runtime,
                args.project.as_ref(),
                args.work_list_id,
                args.task.as_ref(),
                args.task_id,
                args.password_stdin,
                TaskLifecycle::Any,
            )
            .await?;
            let attachment_id = resolve_attachment_target(
                runtime,
                project_id,
                task.id,
                &args.attachment_id,
                args.password_stdin,
            )
            .await?;
            require_confirmation(
                format,
                non_interactive,
                args.yes,
                args.password_stdin,
                &format!(
                    "attachment {} from {} in project {}",
                    attachment_id,
                    entity_label("task", &task),
                    project_id
                ),
            )?;
            with_progress(
                "Deleting attachment…",
                runtime.delete_task_attachment(DeleteTaskAttachmentArgs {
                    work_list_id: project_id,
                    task_id: task.id,
                    attachment_id,
                    password_stdin: args.password_stdin,
                }),
            )
            .await?;
            print_delete_result(
                format,
                "attachment",
                &json!({
                    "deleted": true,
                    "workListId": project_id,
                    "taskId": task.id,
                    "attachmentId": attachment_id,
                }),
                &format!("Deleted attachment {attachment_id}."),
            )
        }
        TaskAttachmentsCommand::Read(args) => {
            let (project_id, task) = resolve_task_target(
                runtime,
                args.project.as_ref(),
                args.work_list_id,
                args.task.as_ref(),
                args.task_id,
                args.password_stdin,
                TaskLifecycle::Any,
            )
            .await?;
            let attachment_id = resolve_attachment_target(
                runtime,
                project_id,
                task.id,
                &args.attachment_id,
                args.password_stdin,
            )
            .await?;
            let attachment = with_progress(
                "Downloading and decrypting attachment…",
                runtime.read_task_attachment(
                    project_id,
                    task.id,
                    attachment_id,
                    args.password_stdin,
                ),
            )
            .await?;
            print_readable_attachment(&attachment, format)
        }
        TaskAttachmentsCommand::Download(args) => {
            let (project_id, task) = resolve_task_target(
                runtime,
                args.project.as_ref(),
                args.work_list_id,
                args.task.as_ref(),
                args.task_id,
                args.password_stdin,
                TaskLifecycle::Any,
            )
            .await?;
            let attachment_id = resolve_attachment_target(
                runtime,
                project_id,
                task.id,
                &args.attachment_id,
                args.password_stdin,
            )
            .await?;
            let attachment = with_progress(
                "Downloading and decrypting attachment…",
                runtime.download_task_attachment(
                    project_id,
                    task.id,
                    attachment_id,
                    args.password_stdin,
                ),
            )
            .await?;
            let output_path =
                resolve_attachment_output_path(&attachment.attachment.file_name, args.output);
            write_attachment_file(&output_path, &attachment.bytes, args.force)?;
            print_download_result(format, &attachment.attachment.file_name, &output_path)
        }
    }
}

async fn resolve_attachment_target(
    runtime: &RuntimeClient,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    selector: &IdSelector,
    password_stdin: bool,
) -> CliResult<uuid::Uuid> {
    if let Some(id) = selector.exact_id() {
        return Ok(id);
    }

    let detail = with_progress(
        "Resolving attachment ID…",
        runtime.get_task(project_id, task_id, password_stdin),
    )
    .await?;
    let ids = detail
        .task
        .attachments
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|attachment| attachment.id)
        .collect();
    resolve_id_selector(
        "attachment",
        &format!("task {task_id} in project {project_id}"),
        selector,
        ids,
        &format!(
            "sealtask tasks get id:{} --project id:{}",
            task_id.simple(),
            project_id.simple()
        ),
    )
    .map_err(Into::into)
}

async fn resolve_task_target(
    runtime: &RuntimeClient,
    project: Option<&EntitySelector>,
    work_list_id: Option<uuid::Uuid>,
    task: Option<&EntitySelector>,
    task_id: Option<uuid::Uuid>,
    password_stdin: bool,
    lifecycle: TaskLifecycle,
) -> CliResult<(uuid::Uuid, ResolvedEntity)> {
    let project = resolve_project(
        runtime,
        project,
        work_list_id,
        password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let task = resolve_task(
        runtime,
        project.id,
        task,
        task_id,
        password_stdin,
        lifecycle,
    )
    .await?;
    Ok((project.id, task))
}

fn selector_needs_discovery(selector: Option<&EntitySelector>) -> bool {
    selector.is_some_and(|selector| selector.exact_id().is_none())
}

fn resolve_attachment_upload_password(
    password_stdin: bool,
) -> CliResult<Option<AttachmentUploadPassword>> {
    resolve_attachment_upload_password_with(password_stdin, || read_required_password(true, None))
}

fn resolve_attachment_upload_password_with(
    password_stdin: bool,
    read_password: impl FnOnce() -> CliResult<String>,
) -> CliResult<Option<AttachmentUploadPassword>> {
    password_stdin
        .then(read_password)
        .transpose()?
        .map(AttachmentUploadPassword::new)
        .transpose()
        .map_err(Into::into)
}

struct SupervisedAttachmentUpload<T> {
    outcome: AttachmentUploadOutcome<T>,
    warnings: Vec<WarningResult>,
}

enum AttachmentUploadOutcome<T> {
    Completed(PublicResult<T>),
    Interrupted {
        message: String,
        outcome_ambiguous: bool,
    },
}

async fn supervise_attachment_upload<T, U, S1, S2>(
    upload: U,
    first_signal: S1,
    second_signal: S2,
    cancellation: OperationCancellation,
    cancellation_grace: Duration,
    progress: Option<&ProgressGuard>,
) -> SupervisedAttachmentUpload<T>
where
    U: Future<Output = PublicResult<T>>,
    S1: Future<Output = io::Result<()>>,
    S2: Future<Output = io::Result<()>>,
{
    tokio::pin!(upload);
    tokio::pin!(first_signal);
    let first_signal_result = tokio::select! {
        biased;
        result = &mut upload => {
            return SupervisedAttachmentUpload {
                outcome: AttachmentUploadOutcome::Completed(result),
                warnings: Vec::new(),
            };
        }
        signal_result = &mut first_signal => signal_result,
    };

    let mut warnings = Vec::new();
    if let Err(signal_error) = first_signal_result {
        warnings.push(warning_result(
            "signal_listener_failed",
            format!("failed to listen for process interruption: {signal_error}"),
        ));
        return SupervisedAttachmentUpload {
            outcome: AttachmentUploadOutcome::Completed(upload.await),
            warnings,
        };
    }

    cancellation.cancel();
    if let Some(progress) = progress {
        progress.set_message("Cancelling upload; cleaning up…");
    }
    warnings.push(warning_result(
        "attachment_upload_cancellation_requested",
        format!(
            "interrupt received; waiting up to {} seconds for attachment cleanup (interrupt again to stop waiting)",
            cancellation_grace.as_secs()
        ),
    ));

    tokio::pin!(second_signal);
    let grace = tokio::time::sleep(cancellation_grace);
    tokio::pin!(grace);
    let mut second_listener_active = true;
    loop {
        tokio::select! {
            biased;
            result = &mut upload => {
                let outcome = if matches!(result, Err(PublicError::Cancelled(_))) {
                    AttachmentUploadOutcome::Interrupted {
                        message: "attachment upload interrupted".to_string(),
                        outcome_ambiguous: false,
                    }
                } else {
                    AttachmentUploadOutcome::Completed(result)
                };
                return SupervisedAttachmentUpload { outcome, warnings };
            }
            signal_result = &mut second_signal, if second_listener_active => {
                match signal_result {
                    Ok(()) => {
                        warnings.push(warning_result(
                            "attachment_upload_cancellation_forced",
                            "second interrupt received before attachment cleanup completed; the backend may need to expire the pending upload".to_string(),
                        ));
                        return SupervisedAttachmentUpload {
                            outcome: AttachmentUploadOutcome::Interrupted {
                                message: "attachment upload interrupted before cleanup completed".to_string(),
                                outcome_ambiguous: true,
                            },
                            warnings,
                        };
                    }
                    Err(signal_error) => {
                        warnings.push(warning_result(
                            "signal_listener_failed",
                            format!("failed to listen for a second process interruption: {signal_error}"),
                        ));
                        second_listener_active = false;
                    }
                }
            }
            () = &mut grace => {
                warnings.push(warning_result(
                    "attachment_upload_cancellation_timed_out",
                    "attachment cleanup did not finish within the cancellation grace period; the backend may need to expire the pending upload".to_string(),
                ));
                return SupervisedAttachmentUpload {
                    outcome: AttachmentUploadOutcome::Interrupted {
                        message: "attachment upload interrupted before cleanup completed".to_string(),
                        outcome_ambiguous: true,
                    },
                    warnings,
                };
            }
        }
    }
}

async fn create_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    mut args: TaskCreateArgsCli,
) -> CliResult<()> {
    validate_body_input(
        args.body.as_deref(),
        args.body_file.as_deref(),
        args.password_stdin,
    )?;
    if non_interactive && args.edit {
        return Err(PublicError::validation(
            "--edit requires an interactive controlling terminal and cannot be combined with --non-interactive",
        )
        .into());
    }
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Active,
    )
    .await?;
    let edited = if args.edit {
        let seed_body = resolve_body_input(
            args.body.as_deref(),
            args.body_file.as_deref(),
            args.password_stdin,
        )?;
        Some(if args.title.is_some() || seed_body.is_some() {
            editor::edit_existing_document(
                "task",
                args.title.as_deref().unwrap_or_default(),
                seed_body.as_deref().unwrap_or_default(),
            )?
        } else {
            editor::edit_new_document("task")?
        })
    } else {
        None
    };
    if let Some(edited) = edited.as_ref() {
        args.title = Some(edited.title.clone());
        args.body = Some(edited.body.clone());
        args.body_file = None;
    }
    let mut input = resolve_task_create_input(&args)?;
    if let Some(due) = args.due.as_deref() {
        let project_detail = load_project(runtime, project.id, args.password_stdin).await?;
        input.due_at = Some(parse_due_input(due, &project_detail.timezone, Utc::now())?);
    }
    if let Some(section) = args.section.as_ref() {
        input.section_id = Some(
            resolve_section(runtime, project.id, section, args.password_stdin)
                .await?
                .id,
        );
    }
    if non_interactive && !args.dry_run && input.idempotency_key.is_none() {
        return Err(PublicError::validation(
            "--non-interactive tasks create requires --idempotency-key or input field idempotencyKey",
        )
        .into());
    }
    let prepared = with_progress(
        "Preparing task…",
        runtime.prepare_task_create(CreateTaskArgs {
            work_list_id: project.id,
            input,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    if args.dry_run {
        return print_task_mutation_plan(prepared.plan(), format);
    }
    let created = with_progress(
        "Creating task…",
        runtime.execute_prepared_task_create(prepared),
    )
    .await?;
    print_task(&created, format)
}

async fn edit_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    args: TaskEditArgsCli,
) -> CliResult<()> {
    if non_interactive {
        return Err(PublicError::validation(
            "'sealtask tasks edit' requires an interactive controlling terminal and cannot be combined with --non-interactive",
        )
        .into());
    }
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let current = with_progress(
        "Loading and decrypting task to edit…",
        runtime.get_task(project_id, task.id, args.password_stdin),
    )
    .await?;
    if current.task.read_error.is_some() {
        return Err(PublicError::validation(
            "the task cannot be edited because its encrypted content is unreadable; inspect it with 'sealtask tasks get' and resolve the read error first",
        )
        .into());
    }
    let current_title = current.task.title.as_deref().ok_or_else(|| {
        PublicError::validation(
            "the task cannot be edited because its decrypted title is unavailable",
        )
    })?;
    let current_body = current.task.body_markdown.as_deref().unwrap_or_default();
    let edited = editor::edit_existing_document("task", current_title, current_body)?;
    let title = (edited.title.trim() != current_title.trim()).then(|| edited.title.clone());
    let body = if edited.body.trim() == current_body.trim() {
        TaskFieldPatch::Unchanged
    } else if edited.body.trim().is_empty() {
        TaskFieldPatch::Clear
    } else {
        TaskFieldPatch::Set(edited.body.clone())
    };
    if title.is_none() && body.is_unchanged() {
        return print_task(&current.task, format);
    }

    let updated = with_progress(
        "Updating task…",
        runtime.update_task_if_unchanged(
            UpdateTaskArgs {
                work_list_id: project_id,
                task_id: task.id,
                input: sealtask_client_runtime::TaskUpdateInput {
                    title,
                    body,
                    checklist: TaskFieldPatch::Unchanged,
                    priority: TaskFieldPatch::Unchanged,
                    due_at: TaskFieldPatch::Unchanged,
                    start_at: TaskFieldPatch::Unchanged,
                    section_id: TaskFieldPatch::Unchanged,
                },
                password_stdin: args.password_stdin,
            },
            current.task.updated_at,
        ),
    )
    .await?;
    print_task(&updated, format)
}

async fn update_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskUpdateArgsCli,
) -> CliResult<()> {
    validate_body_input(
        args.body.as_deref(),
        args.body_file.as_deref(),
        args.password_stdin,
    )?;
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let mut input = resolve_task_update_input(&args)?;
    if let Some(due) = args.due.as_deref() {
        let project = load_project(runtime, project_id, args.password_stdin).await?;
        input.due_at = TaskFieldPatch::Set(parse_due_input(due, &project.timezone, Utc::now())?);
    }
    if let Some(section) = args.section.as_ref() {
        input.section_id = TaskFieldPatch::Set(
            resolve_section(runtime, project_id, section, args.password_stdin)
                .await?
                .id,
        );
    }
    let prepared = with_progress(
        "Preparing task update…",
        runtime.prepare_task_update(UpdateTaskArgs {
            work_list_id: project_id,
            task_id: task.id,
            input,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    if args.dry_run {
        return print_task_mutation_plan(prepared.plan(), format);
    }
    let updated = with_progress(
        "Updating task…",
        runtime.execute_prepared_task_update(prepared),
    )
    .await?;
    print_task(&updated, format)
}

fn print_task_mutation_plan(plan: &TaskMutationPlan, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(plan, format, "task mutation plan must serialize")
        }
        OutputFormat::Table => {
            println!(
                "{}",
                terminal::style_stdout(
                    "Task mutation dry run",
                    crate::terminal::StyleRole::Heading
                )
            );
            println!("Action: {}", plan.action);
            println!("Project: {}", plan.project_id);
            if let Some(task_id) = plan.task_id {
                println!("Task: {task_id}");
            }
            if let Some(section_id) = plan.section_id {
                println!("Section: {section_id}");
            }
            if let Some(expected_updated_at) = plan.expected_updated_at {
                println!("Expected revision: {expected_updated_at}");
            }
            let changed_fields = if plan.changed_fields.is_empty() {
                "(none)".to_string()
            } else {
                plan.changed_fields.join(", ")
            };
            println!(
                "Would change: {} ({} field(s): {changed_fields})",
                if plan.would_change { "yes" } else { "no" },
                plan.changed_field_count
            );
            println!(
                "Idempotency protected: {}",
                if plan.idempotency_protected {
                    "yes"
                } else {
                    "no"
                }
            );
            println!("Change commitment: {}", plan.change_commitment);
            println!("Mutation sent: no");
            Ok(())
        }
    }
}

async fn move_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskMoveArgsCli,
) -> CliResult<()> {
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let section_id = match (args.section_id, args.section.as_ref()) {
        (Some(section_id), None) => Some(section_id),
        (None, Some(section)) => Some(
            resolve_section(runtime, project_id, section, args.password_stdin)
                .await?
                .id,
        ),
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!("clap rejects conflicting section targets"),
    };
    let insert_before_task_id = match (args.insert_before_task_id, args.before.as_ref()) {
        (Some(task_id), None) => Some(task_id),
        (None, Some(before)) => Some(
            resolve_task(
                runtime,
                project_id,
                Some(before),
                None,
                args.password_stdin,
                TaskLifecycle::Any,
            )
            .await?
            .id,
        ),
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!("clap rejects conflicting task targets"),
    };
    let moved = with_progress(
        "Moving task…",
        runtime.move_task(MoveTaskArgs {
            work_list_id: project_id,
            task_id: task.id,
            input: MoveTaskInput {
                section_id,
                insert_before_task_id,
            },
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    print_task(&moved, format)
}

async fn set_task_completion(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskCompletionArgsCli,
    complete: bool,
) -> CliResult<()> {
    let lifecycle = if complete {
        TaskLifecycle::Incomplete
    } else {
        TaskLifecycle::Completed
    };
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        lifecycle,
    )
    .await?;
    let runtime_args = TaskCompletionArgs {
        work_list_id: project_id,
        task_id: task.id,
        password_stdin: args.password_stdin,
    };
    let task = if complete {
        with_progress("Completing task…", runtime.complete_task(runtime_args)).await?
    } else {
        with_progress("Reopening task…", runtime.reopen_task(runtime_args)).await?
    };
    print_task(&task, format)
}

async fn archive_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskArchiveArgsCli,
) -> CliResult<()> {
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Active,
    )
    .await?;
    let archived = with_progress(
        "Archiving task…",
        runtime.archive_task(ArchiveTaskArgs {
            work_list_id: project_id,
            task_id: task.id,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    print_task(&archived, format)
}

async fn unarchive_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskUnarchiveArgsCli,
) -> CliResult<()> {
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Archived,
    )
    .await?;
    let unarchived = with_progress(
        "Restoring task…",
        runtime.unarchive_task(UnarchiveTaskArgs {
            work_list_id: project_id,
            task_id: task.id,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    print_task(&unarchived, format)
}

async fn delete_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    args: TaskDeleteArgsCli,
) -> CliResult<()> {
    let (project_id, task) = resolve_task_target(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    require_confirmation(
        format,
        non_interactive,
        args.yes,
        args.input_stdin || args.password_stdin,
        &format!("{} in project {}", entity_label("task", &task), project_id),
    )?;
    let input =
        resolve_delete_input::<DeleteTaskRequest>(args.input_file.as_deref(), args.input_stdin)?;
    with_progress(
        "Deleting task…",
        runtime.delete_task(DeleteTaskArgs {
            work_list_id: project_id,
            task_id: task.id,
            input,
        }),
    )
    .await?;
    print_delete_result(
        format,
        "task",
        &json!({
            "deleted": true,
            "workListId": project_id,
            "taskId": task.id,
        }),
        &format!("Deleted task {}.", task.id),
    )
}

fn entity_label(kind: &str, entity: &ResolvedEntity) -> String {
    entity.name.as_deref().map_or_else(
        || format!("{kind} {}", entity.id),
        |name| format!("{kind} \"{name}\" ({})", entity.id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Notify;

    #[tokio::test]
    async fn signal_listener_failure_preserves_confirmed_upload_success_as_a_warning() {
        let cancellation = OperationCancellation::new();
        let observed_cancellation = cancellation.clone();
        let upload = async move {
            tokio::task::yield_now().await;
            assert!(!observed_cancellation.is_cancelled());
            Ok::<_, PublicError>(7_u8)
        };
        let supervised = supervise_attachment_upload(
            upload,
            async { Err(io::Error::other("signal registration failed")) },
            std::future::pending(),
            cancellation.clone(),
            Duration::from_millis(10),
            None,
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Completed(Ok(7))
        ));
        assert_eq!(supervised.warnings.len(), 1);
        assert_eq!(supervised.warnings[0].code(), "signal_listener_failed");
        assert!(
            supervised.warnings[0]
                .message()
                .contains("signal registration failed")
        );
        assert!(!cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn signal_listener_failure_preserves_ambiguous_cleanup_error_and_warning() {
        let cancellation = OperationCancellation::new();
        let observed_cancellation = cancellation.clone();
        let upload = async move {
            tokio::task::yield_now().await;
            assert!(!observed_cancellation.is_cancelled());
            Err::<(), _>(PublicError::compensation_failed(
                "attachment upload",
                "primary category",
                "cleanup category",
            ))
        };
        let supervised = supervise_attachment_upload(
            upload,
            async { Err(io::Error::other("signal registration failed")) },
            std::future::pending(),
            cancellation.clone(),
            Duration::from_millis(10),
            None,
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Completed(Err(PublicError::CompensationFailed {
                operation,
                primary,
                cleanup,
            })) if operation == "attachment upload"
                && primary == "primary category"
                && cleanup == "cleanup category"
        ));
        assert_eq!(supervised.warnings[0].code(), "signal_listener_failed");
        assert!(
            supervised.warnings[0]
                .message()
                .contains("signal registration failed")
        );
        assert!(!cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn signal_waits_for_bounded_upload_cleanup_before_returning() {
        let cancellation = OperationCancellation::new();
        let upload_cancellation = cancellation.clone();
        let cleanup_seen = Arc::new(AtomicBool::new(false));
        let upload_cleanup_seen = cleanup_seen.clone();
        let started = Arc::new(Notify::new());
        let upload_started = started.clone();
        let upload = async move {
            upload_started.notify_one();
            while !upload_cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
            upload_cleanup_seen.store(true, Ordering::Release);
            Err::<(), _>(PublicError::cancelled("attachment upload cancelled"))
        };
        let signal = async move {
            started.notified().await;
            Ok(())
        };

        let supervised = supervise_attachment_upload(
            upload,
            signal,
            std::future::pending(),
            cancellation,
            Duration::from_millis(50),
            None,
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted {
                outcome_ambiguous: false,
                ..
            }
        ));
        assert_eq!(
            supervised.warnings[0].code(),
            "attachment_upload_cancellation_requested"
        );
        assert!(cleanup_seen.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn upload_password_is_consumed_once_before_cancellable_supervision() {
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reader_reads = reads.clone();
        let password = resolve_attachment_upload_password_with(true, move || {
            reader_reads.fetch_add(1, Ordering::AcqRel);
            Ok("injected password".to_string())
        })
        .expect("injected password")
        .expect("password value");
        assert_eq!(reads.load(Ordering::Acquire), 1);
        assert!(!format!("{password:?}").contains("injected password"));

        let cancellation = OperationCancellation::new();
        let upload_cancellation = cancellation.clone();
        let upload = async move {
            while !upload_cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
            Err::<(), _>(PublicError::cancelled("attachment upload cancelled"))
        };
        let supervised = supervise_attachment_upload(
            upload,
            async { Ok(()) },
            std::future::pending(),
            cancellation,
            Duration::from_millis(50),
            None,
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted {
                outcome_ambiguous: false,
                ..
            }
        ));
        assert_eq!(reads.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn second_signal_stops_waiting_for_stalled_cleanup() {
        let cancellation = OperationCancellation::new();
        let upload_cancellation = cancellation.clone();
        let upload = async move {
            while !upload_cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
            std::future::pending::<PublicResult<()>>().await
        };
        let supervised = supervise_attachment_upload(
            upload,
            async { Ok(()) },
            async { Ok(()) },
            cancellation,
            Duration::from_secs(30),
            None,
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted {
                outcome_ambiguous: true,
                ..
            }
        ));
        assert!(
            supervised
                .warnings
                .iter()
                .any(|warning| warning.code() == "attachment_upload_cancellation_forced")
        );
    }

    #[tokio::test]
    async fn grace_timeout_stops_waiting_for_stalled_cleanup() {
        let cancellation = OperationCancellation::new();
        let upload_cancellation = cancellation.clone();
        let upload = async move {
            while !upload_cancellation.is_cancelled() {
                tokio::task::yield_now().await;
            }
            std::future::pending::<PublicResult<()>>().await
        };
        let supervised = supervise_attachment_upload(
            upload,
            async { Ok(()) },
            std::future::pending(),
            cancellation,
            Duration::from_millis(1),
            None,
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted {
                outcome_ambiguous: true,
                ..
            }
        ));
        assert!(
            supervised
                .warnings
                .iter()
                .any(|warning| warning.code() == "attachment_upload_cancellation_timed_out")
        );
    }
}
