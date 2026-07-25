use crate::args::{
    CommentCreateArgsCli, CommentDeleteArgsCli, CommentUpdateArgsCli, CommentsCommand,
};
use crate::input::{resolve_comment_input, resolve_delete_input};
use crate::output::{CliResult, OutputFormat};
use crate::render::{print_comment, print_comments, print_delete_result, print_empty_collection};
use sealtask_client_api::DeleteCommentRequest;
use sealtask_client_runtime::{
    CreateCommentArgs, DeleteCommentArgs, RuntimeClient, UpdateCommentArgs,
};
use serde_json::json;
use uuid::Uuid;

pub(crate) async fn run_comments(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: CommentsCommand,
) -> CliResult<()> {
    match command {
        CommentsCommand::List {
            work_list_id,
            task_id,
            password_stdin,
        } => list_comments(runtime, format, work_list_id, task_id, password_stdin).await,
        CommentsCommand::Create(args) => create_comment(runtime, format, args).await,
        CommentsCommand::Update(args) => update_comment(runtime, format, args).await,
        CommentsCommand::Delete(args) => delete_comment(runtime, format, args).await,
    }
}

async fn list_comments(
    runtime: &RuntimeClient,
    format: OutputFormat,
    work_list_id: Uuid,
    task_id: Uuid,
    password_stdin: bool,
) -> CliResult<()> {
    let comments = runtime
        .list_comments(work_list_id, task_id, password_stdin)
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
    let input = resolve_comment_input(
        args.body.as_deref(),
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )?;
    let created = runtime
        .create_comment(CreateCommentArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_comment(&created, format)
}

async fn update_comment(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: CommentUpdateArgsCli,
) -> CliResult<()> {
    let input = resolve_comment_input(
        args.body.as_deref(),
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )?;
    let updated = runtime
        .update_comment(UpdateCommentArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            comment_id: args.comment_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_comment(&updated, format)
}

async fn delete_comment(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: CommentDeleteArgsCli,
) -> CliResult<()> {
    let input =
        resolve_delete_input::<DeleteCommentRequest>(args.input_file.as_deref(), args.input_stdin)?;
    runtime
        .delete_comment(DeleteCommentArgs {
            work_list_id: args.work_list_id,
            task_id: args.task_id,
            comment_id: args.comment_id,
            input,
        })
        .await?;
    print_delete_result(
        format,
        "comment",
        &json!({
            "deleted": true,
            "workListId": args.work_list_id,
            "taskId": args.task_id,
            "commentId": args.comment_id,
        }),
        &format!("Deleted comment {}.", args.comment_id),
    )
}
