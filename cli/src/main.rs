#![cfg_attr(test, allow(clippy::unwrap_used))]

#[macro_use]
mod output;

mod args;
mod commands;
mod input;
mod render;

use args::{Cli, Command};
use clap::Parser;
use commands::{
    run_auth, run_comments, run_info, run_lists, run_lists_get, run_me, run_stats, run_tasks,
};
use output::{CliError, CliResult, OutputFormat, print_clap_error, print_cli_error};
use std::ffi::OsString;
use worklist_client_core::PublicError;
use worklist_client_runtime::{RuntimeClient, serve};

#[tokio::main]
async fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let format = OutputFormat::from_raw_args(&args);
    let cli = parse_cli_or_exit(&args, format);
    match run(cli, format).await {
        Ok(()) => {}
        Err(CliError::BrokenPipe) => std::process::exit(0),
        Err(err) => {
            let _ = print_cli_error(&err, format);
            std::process::exit(1);
        }
    }
}

fn parse_cli_or_exit(args: &[OsString], format: OutputFormat) -> Cli {
    match Cli::try_parse_from(args.iter().cloned()) {
        Ok(cli) => cli,
        Err(err) => exit_after_clap_error(err, format),
    }
}

fn exit_after_clap_error(err: clap::Error, format: OutputFormat) -> ! {
    if format == OutputFormat::Json && err.use_stderr() {
        let _ = print_clap_error(&err);
        std::process::exit(err.exit_code());
    }

    err.exit()
}

async fn run(cli: Cli, format: OutputFormat) -> CliResult<()> {
    if let Some(socket_path) = cli.serve_unlock_daemon.as_deref() {
        return serve(socket_path).await.map_err(Into::into);
    }

    let runtime = RuntimeClient::new(&cli.api_url);
    let Some(command) = cli.command else {
        return Err(PublicError::validation("a command is required").into());
    };

    match command {
        Command::Info => run_info(&runtime),
        Command::Auth { command } => run_auth(&runtime, format, command).await,
        Command::Me => run_me(&runtime, format).await,
        Command::Lists {
            verbose,
            include_archived,
            password_stdin,
            raw,
            command,
        } => {
            run_lists(
                &runtime,
                format,
                verbose,
                include_archived,
                password_stdin,
                raw,
                command,
            )
            .await
        }
        Command::Tasks { command } => run_tasks(&runtime, format, command).await,
        Command::Stats => run_stats(&runtime, format).await,
        Command::Inspect {
            work_list_id,
            password_stdin,
        } => run_lists_get(&runtime, format, work_list_id, password_stdin, false).await,
        Command::Comments { command } => run_comments(&runtime, format, command).await,
    }
}
