use crate::args::{
    TaskArchiveArgsCli, TaskAttachmentsCommand, TaskCompletionArgsCli, TaskCreateArgsCli,
    TaskDeleteArgsCli, TaskMoveArgsCli, TaskUnarchiveArgsCli, TaskUpdateArgsCli, TasksCommand,
};
use crate::input::{
    resolve_attachment_output_path, resolve_delete_input, resolve_task_create_input,
    resolve_task_update_input, write_attachment_file,
};
use crate::output::{CliResult, OutputFormat, print_pretty_json};
use crate::render::{
    print_delete_result, print_download_result, print_empty_collection, print_raw_my_tasks,
    print_raw_task_detail, print_raw_tasks, print_readable_attachment, print_task_detail,
    print_tasks,
};
use serde_json::json;
use worklist_client_api::DeleteTaskRequest;
use worklist_client_runtime::{
    ArchiveTaskArgs, CreateTaskArgs, DeleteTaskArgs, MoveTaskArgs, MoveTaskInput, RuntimeClient,
    TaskCompletionArgs, UnarchiveTaskArgs, UpdateTaskArgs,
};

pub(crate) async fn run_tasks(
    runtime: &RuntimeClient,
    format: OutputFormat,
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
        TasksCommand::Create(args) => create_task(runtime, args).await,
        TasksCommand::Update(args) => update_task(runtime, args).await,
        TasksCommand::Move(args) => move_task(runtime, args).await,
        TasksCommand::Complete(args) => set_task_completion(runtime, args, true).await,
        TasksCommand::Reopen(args) => set_task_completion(runtime, args, false).await,
        TasksCommand::Archive(args) => archive_task(runtime, args).await,
        TasksCommand::Unarchive(args) => unarchive_task(runtime, args).await,
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

async fn create_task(runtime: &RuntimeClient, args: TaskCreateArgsCli) -> CliResult<()> {
    let input = resolve_task_create_input(&args)?;
    let created = runtime
        .create_task(CreateTaskArgs {
            work_list_id: args.work_list_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_pretty_json(&created, "serializing created task should succeed")
}

async fn update_task(runtime: &RuntimeClient, args: TaskUpdateArgsCli) -> CliResult<()> {
    let input = resolve_task_update_input(&args)?;
    let updated = runtime
        .update_task(UpdateTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_pretty_json(&updated, "serializing updated task should succeed")
}

async fn move_task(runtime: &RuntimeClient, args: TaskMoveArgsCli) -> CliResult<()> {
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
    print_pretty_json(&moved, "serializing moved task should succeed")
}

async fn set_task_completion(
    runtime: &RuntimeClient,
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
    print_pretty_json(
        &task,
        if complete {
            "serializing completed task should succeed"
        } else {
            "serializing reopened task should succeed"
        },
    )
}

async fn archive_task(runtime: &RuntimeClient, args: TaskArchiveArgsCli) -> CliResult<()> {
    let archived = runtime
        .archive_task(ArchiveTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_pretty_json(&archived, "serializing archived task should succeed")
}

async fn unarchive_task(runtime: &RuntimeClient, args: TaskUnarchiveArgsCli) -> CliResult<()> {
    let unarchived = runtime
        .unarchive_task(UnarchiveTaskArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_pretty_json(&unarchived, "serializing unarchived task should succeed")
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
