use crate::args::{
    CommentCreateArgsCli, CommentDeleteArgsCli, CommentUpdateArgsCli, CommentsCommand,
};
use crate::input::{resolve_comment_input, resolve_delete_input, validate_body_input};
use crate::interaction::require_confirmation;
use crate::output::{CliResult, OutputFormat};
use crate::render::{print_comment, print_comments, print_delete_result, print_empty_collection};
use crate::resolver::{TaskLifecycle, resolve_task_and_project};
use crate::selectors::{IdSelector, ResolvedEntity, resolve_id_selector};
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
            let (project, task) = resolve_task_and_project(
                runtime,
                project.as_ref(),
                work_list_id,
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
    validate_body_input(
        args.body.as_deref(),
        args.body_file.as_deref(),
        args.password_stdin,
    )?;
    let (project, task) = resolve_task_and_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let input = resolve_comment_input(
        args.body.as_deref(),
        args.body_file.as_deref(),
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
    let (project, task) = resolve_task_and_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let comment_id = resolve_comment_target(runtime, project.id, task.id, &args.comment_id).await?;
    let input = resolve_comment_input(
        args.body.as_deref(),
        None,
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )?;
    let updated = with_progress(
        "Updating comment…",
        runtime.update_comment(UpdateCommentArgs {
            work_list_id: project.id,
            task_id: task.id,
            comment_id,
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
    let (project, task) = resolve_task_and_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.task.as_ref(),
        args.task_id,
        args.password_stdin,
        TaskLifecycle::Any,
    )
    .await?;
    let comment_id = resolve_comment_target(runtime, project.id, task.id, &args.comment_id).await?;
    require_confirmation(
        format,
        non_interactive,
        args.yes,
        args.input_stdin || args.password_stdin,
        &format!(
            "comment {} on {} in project {}",
            comment_id,
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
            comment_id,
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
            "commentId": comment_id,
        }),
        &format!("Deleted comment {comment_id}."),
    )
}

async fn resolve_comment_target(
    runtime: &RuntimeClient,
    project_id: Uuid,
    task_id: Uuid,
    selector: &IdSelector,
) -> CliResult<Uuid> {
    if let Some(id) = selector.exact_id() {
        return Ok(id);
    }

    let mut client = runtime.authenticated_api_client()?;
    let comments = with_progress(
        "Resolving comment ID…",
        client.list_comments(project_id, task_id),
    )
    .await?;
    resolve_id_selector(
        "comment",
        &format!("task {task_id} in project {project_id}"),
        selector,
        comments.into_iter().map(|comment| comment.id).collect(),
        &format!(
            "sealtask comments list id:{} --project id:{}",
            task_id.simple(),
            project_id.simple()
        ),
    )
    .map_err(Into::into)
}

fn entity_label(kind: &str, entity: &ResolvedEntity) -> String {
    entity.name.as_deref().map_or_else(
        || format!("{kind} {}", entity.id),
        |name| format!("{kind} \"{name}\" ({})", entity.id),
    )
}
