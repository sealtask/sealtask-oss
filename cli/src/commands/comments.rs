use crate::args::{
    CommentCreateArgsCli, CommentDeleteArgsCli, CommentUpdateArgsCli, CommentsCommand,
};
use crate::input::{resolve_comment_input, resolve_delete_input};
use crate::interaction::require_confirmation;
use crate::output::{CliResult, OutputFormat};
use crate::render::{print_comment, print_comments, print_delete_result, print_empty_collection};
use crate::resolver::{ProjectLifecycle, TaskLifecycle, resolve_project, resolve_task};
use crate::selectors::ResolvedEntity;
use crate::terminal::with_progress;
use sealtask_client_api::DeleteCommentRequest;
use sealtask_client_runtime::{
    CreateCommentArgs, DeleteCommentArgs, RuntimeClient, UpdateCommentArgs,
};
use serde_json::json;
use uuid::Uuid;

pub(crate) async fn run_comments(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    command: CommentsCommand,
) -> CliResult<()> {
    match command {
        CommentsCommand::List {
            project,
            work_list_id,
            task,
            task_id,
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
            let task = resolve_task(
                runtime,
                project.id,
                task.as_ref(),
                task_id,
                password_stdin,
                TaskLifecycle::Any,
            )
            .await?;
            list_comments(runtime, format, project.id, task.id, password_stdin).await
        }
        CommentsCommand::Create(args) => create_comment(runtime, format, args).await,
        CommentsCommand::Update(args) => update_comment(runtime, format, args).await,
        CommentsCommand::Delete(args) => {
            delete_comment(runtime, format, non_interactive, args).await
        }
    }
}

async fn list_comments(
    runtime: &RuntimeClient,
    format: OutputFormat,
    work_list_id: Uuid,
    task_id: Uuid,
    password_stdin: bool,
) -> CliResult<()> {
    let comments = with_progress(
        "Loading and decrypting comments…",
        runtime.list_comments(work_list_id, task_id, password_stdin),
    )
    .await?;
    if comments.is_empty() {
        return print_empty_collection(format, "No comments found.");
    }
    print_comments(&comments, format)
}

async fn create_comment(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: CommentCreateArgsCli,
) -> CliResult<()> {
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let task = resolve_task(
        runtime,
        project.id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let input = resolve_comment_input(
        args.body.as_deref(),
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )?;
    let created = with_progress(
        "Creating comment…",
        runtime.create_comment(CreateCommentArgs {
            work_list_id: project.id,
            task_id: task.id,
            input,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    print_comment(&created, format)
}

async fn update_comment(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: CommentUpdateArgsCli,
) -> CliResult<()> {
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let task = resolve_task(
        runtime,
        project.id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let input = resolve_comment_input(
        args.body.as_deref(),
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )?;
    let updated = with_progress(
        "Updating comment…",
        runtime.update_comment(UpdateCommentArgs {
            work_list_id: project.id,
            task_id: task.id,
            comment_id: args.comment_id,
            input,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    print_comment(&updated, format)
}

async fn delete_comment(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    args: CommentDeleteArgsCli,
) -> CliResult<()> {
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let task = resolve_task(
        runtime,
        project.id,
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
        &format!(
            "comment {} on {} in project {}",
            args.comment_id,
            entity_label("task", &task),
            project.id
        ),
    )?;
    let input =
        resolve_delete_input::<DeleteCommentRequest>(args.input_file.as_deref(), args.input_stdin)?;
    with_progress(
        "Deleting comment…",
        runtime.delete_comment(DeleteCommentArgs {
            work_list_id: project.id,
            task_id: task.id,
            comment_id: args.comment_id,
            input,
        }),
    )
    .await?;
    print_delete_result(
        format,
        "comment",
        &json!({
            "deleted": true,
            "workListId": project.id,
            "taskId": task.id,
            "commentId": args.comment_id,
        }),
        &format!("Deleted comment {}.", args.comment_id),
    )
}

fn entity_label(kind: &str, entity: &ResolvedEntity) -> String {
    entity.name.as_deref().map_or_else(
        || format!("{kind} {}", entity.id),
        |name| format!("{kind} \"{name}\" ({})", entity.id),
    )
}
