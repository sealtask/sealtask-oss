#![cfg_attr(test, allow(clippy::unwrap_used))]

#[macro_use]
mod output;

mod args;
mod attachment_output;
mod commands;
mod input;
mod interaction;
mod output_models;
mod render;

use args::{Cli, Command};
use clap::Parser;
use commands::{
    run_auth, run_comments, run_info, run_lists, run_lists_get, run_me, run_notes, run_schema,
    run_stats, run_tasks,
};
use output::{CliError, CliResult, OutputFormat, print_clap_error, print_cli_error};
use sealtask_client_auth::configure_local_state;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::{RuntimeClient, serve};
use std::ffi::OsString;

const ROOT_QUICK_HELP: &str = "\
SealTask CLI — secure task management from your terminal

Get started:
  sealtask auth login        Sign in to SealTask
  sealtask auth unlock       Unlock workspace data
  sealtask lists             List projects
  sealtask tasks list --all  List your assigned tasks

Discover:
  sealtask --help             Show all commands and global options
  sealtask help <command>     Show help for one command";

#[tokio::main]
async fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let raw_format = OutputFormat::from_raw_args(&args);
    let cli = parse_cli_or_exit(&args, raw_format);
    let format = OutputFormat::from_cli(&cli);
    match run(cli, format).await {
        Ok(()) => {}
        Err(CliError::BrokenPipe) => std::process::exit(0),
        Err(err) => {
            let _ = print_cli_error(&err, format);
            std::process::exit(err.exit_code());
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
    if format.is_json() && err.use_stderr() {
        let _ = print_clap_error(&err, format);
        std::process::exit(err.exit_code());
    }

    err.exit()
}

async fn run(cli: Cli, format: OutputFormat) -> CliResult<()> {
    if cli.command.is_none() && cli.serve_unlock_daemon.is_none() {
        if format.is_json() {
            return Err(PublicError::validation(
                "a command is required; run 'sealtask --help' to list commands",
            )
            .into());
        }
        output::write_stdout_line(format_args!("{ROOT_QUICK_HELP}"))?;
        return Ok(());
    }

    configure_local_state(cli.config_dir.clone(), cli.profile.as_deref())?;

    if let Some(socket_path) = cli.serve_unlock_daemon.as_deref() {
        return serve(socket_path).await.map_err(Into::into);
    }

    let runtime = RuntimeClient::with_storage_origins(&cli.api_url, &cli.storage_origin)?;
    let command = cli
        .command
        .expect("a command is present after handling root guidance");

    match command {
        Command::Info => run_info(&runtime, format),
        Command::Schema { command } => run_schema(format, &command),
        Command::Auth { command } => run_auth(&runtime, format, cli.non_interactive, command).await,
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
        Command::Tasks { command } => {
            run_tasks(&runtime, format, cli.non_interactive, command).await
        }
        Command::Stats => run_stats(&runtime, format).await,
        Command::Inspect {
            work_list_id,
            password_stdin,
        } => run_lists_get(&runtime, format, work_list_id, password_stdin, false).await,
        Command::Comments { command } => {
            run_comments(&runtime, format, cli.non_interactive, command).await
        }
        Command::Notes { command } => {
            run_notes(&runtime, format, cli.non_interactive, command).await
        }
    }
}
