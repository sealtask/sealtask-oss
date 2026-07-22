use crate::args::ListsCommand;
use crate::output::{CliResult, OutputFormat};
use crate::render::{
    print_empty_collection, print_raw_work_list_detail, print_raw_work_lists, print_stats,
    print_user, print_work_list_detail, print_work_lists,
};
use sealtask_client_runtime::RuntimeClient;
use uuid::Uuid;

pub(crate) async fn run_me(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let user = runtime.get_me().await?;
    print_user(&user, format)
}

pub(crate) async fn run_stats(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let stats = runtime.get_stats().await?;
    print_stats(&stats, format)
}

pub(crate) async fn run_lists(
    runtime: &RuntimeClient,
    format: OutputFormat,
    verbose: bool,
    include_archived: bool,
    password_stdin: bool,
    raw: bool,
    command: Option<ListsCommand>,
) -> CliResult<()> {
    match command {
        Some(ListsCommand::Get {
            work_list_id,
            password_stdin,
            raw,
        }) => run_lists_get(runtime, format, work_list_id, password_stdin, raw).await,
        Some(ListsCommand::Archive {
            work_list_id,
            password_stdin,
            raw,
        }) => run_lists_lifecycle(runtime, format, work_list_id, password_stdin, raw, true).await,
        Some(ListsCommand::Unarchive {
            work_list_id,
            password_stdin,
            raw,
        }) => run_lists_lifecycle(runtime, format, work_list_id, password_stdin, raw, false).await,
        None => {
            list_work_lists(
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

async fn list_work_lists(
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
            return print_empty_collection(format, "No work lists found.");
        }
        return print_raw_work_lists(&lists, format, verbose);
    }

    let lists = if include_archived {
        runtime
            .list_work_lists_with_archived(password_stdin, true)
            .await?
    } else {
        runtime.list_work_lists(password_stdin).await?
    };
    if lists.is_empty() {
        return print_empty_collection(format, "No work lists found.");
    }
    print_work_lists(&lists, format, verbose)
}

async fn run_lists_lifecycle(
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
