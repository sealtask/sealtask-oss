use crate::args::{
    TaskArchiveArgsCli, TaskAttachmentsCommand, TaskCompletionArgsCli, TaskCreateArgsCli,
    TaskDeleteArgsCli, TaskMoveArgsCli, TaskReferencesCommand, TaskUnarchiveArgsCli,
    TaskUpdateArgsCli, TasksCommand,
};
use crate::attachment_output::{resolve_attachment_output_path, write_attachment_file};
use crate::input::{
    read_required_password, resolve_delete_input, resolve_task_create_input,
    resolve_task_update_input,
};
use crate::output::{
    CliError, CliResult, OutputFormat, WarningResult, finish_with_warnings, warning_result,
};
use crate::render::{
    print_attachment, print_delete_result, print_download_result, print_empty_collection,
    print_raw_my_tasks, print_raw_task_detail, print_raw_tasks, print_readable_attachment,
    print_task, print_task_detail, print_task_reference_scheme_result, print_tasks,
};
use sealtask_client_api::DeleteTaskRequest;
use sealtask_client_core::PublicError;
use sealtask_client_core::PublicResult;
use sealtask_client_runtime::{
    ArchiveTaskArgs, AttachmentUploadPassword, CreateTaskArgs, DeleteTaskArgs,
    DeleteTaskAttachmentArgs, MoveTaskArgs, MoveTaskInput, OperationCancellation,
    QuarantineTaskReferenceSchemeArgs, RepairTaskReferenceSchemeArgs, RuntimeClient,
    TaskCompletionArgs, UnarchiveTaskArgs, UpdateTaskArgs, UploadTaskAttachmentArgs,
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
            work_list_id,
            include_completed,
            all,
            password_stdin,
            raw,
        } => {
            if raw {
                let mut client = runtime.authenticated_api_client()?;
                if all || work_list_id.is_none() {
                    let tasks = client.get_all_my_tasks(include_completed).await?;
                    if tasks.is_empty() {
                        return print_empty_collection(format, "No tasks found.");
                    }
                    return print_raw_my_tasks(&tasks, format);
                }

                let work_list_id = work_list_id.expect("validated work list id");
                let response = client.get_tasks(work_list_id, false).await?;
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
                    return print_empty_collection(format, "No tasks found in this work list.");
                }
                return print_raw_tasks(&tasks, format);
            }

            let tasks = runtime
                .list_tasks(work_list_id, include_completed, all, password_stdin)
                .await?;
            if tasks.is_empty() {
                let message = if all || work_list_id.is_none() {
                    "No tasks found."
                } else {
                    "No tasks found in this work list."
                };
                return print_empty_collection(format, message);
            }
            print_tasks(&tasks, format)
        }
        TasksCommand::Get {
            work_list_id,
            task_id,
            password_stdin,
            raw,
        } => {
            if raw {
                let mut client = runtime.authenticated_api_client()?;
                let detail = client.get_task(work_list_id, task_id).await?;
                return print_raw_task_detail(&detail, format);
            }

            let detail = runtime
                .get_task(work_list_id, task_id, password_stdin)
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
        TasksCommand::Create(args) => create_task(runtime, format, non_interactive, args).await,
        TasksCommand::Update(args) => update_task(runtime, format, args).await,
        TasksCommand::Move(args) => move_task(runtime, format, args).await,
        TasksCommand::Complete(args) => set_task_completion(runtime, format, args, true).await,
        TasksCommand::Reopen(args) => set_task_completion(runtime, format, args, false).await,
        TasksCommand::Archive(args) => archive_task(runtime, format, args).await,
        TasksCommand::Unarchive(args) => unarchive_task(runtime, format, args).await,
        TasksCommand::Delete(args) => delete_task(runtime, format, args).await,
        TasksCommand::Attachments { command } => {
            run_task_attachments(runtime, format, command).await
        }
    }
}

async fn run_task_attachments(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: TaskAttachmentsCommand,
) -> CliResult<()> {
    match command {
        TaskAttachmentsCommand::Upload(args) => {
            let password = resolve_attachment_upload_password(args.password_stdin)?;
            let cancellation = OperationCancellation::new();
            let supervised = supervise_attachment_upload(
                runtime.upload_task_attachment_with_cancellation(
                    UploadTaskAttachmentArgs {
                        work_list_id: args.work_list_id,
                        task_id: args.task_id,
                        path: args.file,
                        file_name: args.file_name,
                        content_type: args.content_type,
                        password,
                    },
                    cancellation.clone(),
                ),
                tokio::signal::ctrl_c(),
                tokio::signal::ctrl_c(),
                cancellation,
                ATTACHMENT_UPLOAD_CANCELLATION_GRACE,
            )
            .await;
            let result = match supervised.outcome {
                AttachmentUploadOutcome::Completed(Ok(attachment)) => {
                    print_attachment(&attachment, format)
                }
                AttachmentUploadOutcome::Completed(Err(error)) => Err(error.into()),
                AttachmentUploadOutcome::Interrupted { message } => {
                    return Err(CliError::interrupted(message, &supervised.warnings));
                }
            };
            finish_with_warnings(format, &supervised.warnings, result)
        }
        TaskAttachmentsCommand::Delete(args) => {
            runtime
                .delete_task_attachment(DeleteTaskAttachmentArgs {
                    work_list_id: args.work_list_id,
                    task_id: args.task_id,
                    attachment_id: args.attachment_id,
                    password_stdin: args.password_stdin,
                })
                .await?;
            print_delete_result(
                format,
                "attachment",
                &json!({
                    "deleted": true,
                    "workListId": args.work_list_id,
                    "taskId": args.task_id,
                    "attachmentId": args.attachment_id,
                }),
                &format!("Deleted attachment {}.", args.attachment_id),
            )
        }
        TaskAttachmentsCommand::Read(args) => {
            let attachment = runtime
                .read_task_attachment(
                    args.work_list_id,
                    args.task_id,
                    args.attachment_id,
                    args.password_stdin,
                )
                .await?;
            print_readable_attachment(&attachment, format)
        }
        TaskAttachmentsCommand::Download(args) => {
            let attachment = runtime
                .download_task_attachment(
                    args.work_list_id,
                    args.task_id,
                    args.attachment_id,
                    args.password_stdin,
                )
                .await?;
            let output_path =
                resolve_attachment_output_path(&attachment.attachment.file_name, args.output);
            write_attachment_file(&output_path, &attachment.bytes, args.force)?;
            print_download_result(format, &attachment.attachment.file_name, &output_path)
        }
    }
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
    Interrupted { message: String },
}

async fn supervise_attachment_upload<T, U, S1, S2>(
    upload: U,
    first_signal: S1,
    second_signal: S2,
    cancellation: OperationCancellation,
    cancellation_grace: Duration,
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
            "ctrl_c_listener_failed",
            format!("failed to listen for Ctrl-C: {signal_error}"),
        ));
        return SupervisedAttachmentUpload {
            outcome: AttachmentUploadOutcome::Completed(upload.await),
            warnings,
        };
    }

    cancellation.cancel();
    warnings.push(warning_result(
        "attachment_upload_cancellation_requested",
        format!(
            "Ctrl-C received; waiting up to {} seconds for attachment cleanup (press Ctrl-C again to stop waiting)",
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
                            "second Ctrl-C received before attachment cleanup completed; the backend may need to expire the pending upload".to_string(),
                        ));
                        return SupervisedAttachmentUpload {
                            outcome: AttachmentUploadOutcome::Interrupted {
                                message: "attachment upload interrupted before cleanup completed".to_string(),
                            },
                            warnings,
                        };
                    }
                    Err(signal_error) => {
                        warnings.push(warning_result(
                            "ctrl_c_listener_failed",
                            format!("failed to listen for a second Ctrl-C: {signal_error}"),
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
    args: TaskCreateArgsCli,
) -> CliResult<()> {
    let input = resolve_task_create_input(&args)?;
    if non_interactive && input.idempotency_key.is_none() {
        return Err(PublicError::validation(
            "--non-interactive tasks create requires --idempotency-key or input field idempotencyKey",
        )
        .into());
    }
    let created = runtime
        .create_task(CreateTaskArgs {
            work_list_id: args.work_list_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_task(&created, format)
}

async fn update_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskUpdateArgsCli,
) -> CliResult<()> {
    let input = resolve_task_update_input(&args)?;
    let updated = runtime
        .update_task(UpdateTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_task(&updated, format)
}

async fn move_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskMoveArgsCli,
) -> CliResult<()> {
    let moved = runtime
        .move_task(MoveTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            input: MoveTaskInput {
                section_id: args.section_id,
                insert_before_task_id: args.insert_before_task_id,
            },
            password_stdin: args.password_stdin,
        })
        .await?;
    print_task(&moved, format)
}

async fn set_task_completion(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskCompletionArgsCli,
    complete: bool,
) -> CliResult<()> {
    let args = TaskCompletionArgs {
        work_list_id: args.work_list_id,
        task_id: args.task_id,
        password_stdin: args.password_stdin,
    };
    let task = if complete {
        runtime.complete_task(args).await?
    } else {
        runtime.reopen_task(args).await?
    };
    print_task(&task, format)
}

async fn archive_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskArchiveArgsCli,
) -> CliResult<()> {
    let archived = runtime
        .archive_task(ArchiveTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_task(&archived, format)
}

async fn unarchive_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskUnarchiveArgsCli,
) -> CliResult<()> {
    let unarchived = runtime
        .unarchive_task(UnarchiveTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_task(&unarchived, format)
}

async fn delete_task(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: TaskDeleteArgsCli,
) -> CliResult<()> {
    let input =
        resolve_delete_input::<DeleteTaskRequest>(args.input_file.as_deref(), args.input_stdin)?;
    runtime
        .delete_task(DeleteTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            input,
        })
        .await?;
    print_delete_result(
        format,
        "task",
        &json!({
            "deleted": true,
            "workListId": args.work_list_id,
            "taskId": args.task_id,
        }),
        &format!("Deleted task {}.", args.task_id),
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
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Completed(Ok(7))
        ));
        assert_eq!(supervised.warnings.len(), 1);
        assert_eq!(supervised.warnings[0].code(), "ctrl_c_listener_failed");
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
        assert_eq!(supervised.warnings[0].code(), "ctrl_c_listener_failed");
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
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted { .. }
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
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted { .. }
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
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted { .. }
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
        )
        .await;

        assert!(matches!(
            supervised.outcome,
            AttachmentUploadOutcome::Interrupted { .. }
        ));
        assert!(
            supervised
                .warnings
                .iter()
                .any(|warning| warning.code() == "attachment_upload_cancellation_timed_out")
        );
    }
}
