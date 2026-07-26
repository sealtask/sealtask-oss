#![cfg_attr(test, allow(clippy::unwrap_used))]

#[macro_use]
mod output;

mod args;
mod attachment_output;
mod commands;
mod discovery;
mod doctor;
mod human_input;
mod input;
mod interaction;
mod operator_config;
mod output_models;
mod project_context;
mod render;
mod resolver;
mod selectors;
mod table;
mod telemetry;

use args::{Cli, Command};
use clap::FromArgMatches;
use commands::{
    run_auth, run_comments, run_config, run_info, run_lists_get, run_me, run_notes, run_profile,
    run_projects, run_schema, run_stats, run_tasks,
};
use operator_config::{
    OperatorOverrides, parse_timeout, resolve_operator_config,
    resolve_operator_config_for_diagnostics,
};
use output::{CliError, CliResult, OutputFormat, print_clap_error, print_cli_error};
use sealtask_client_api::ApiTransportOptions;
use sealtask_client_auth::configure_local_state;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::{RuntimeClient, serve};
use std::ffi::{OsStr, OsString};
use std::time::Duration;
use telemetry::{Telemetry, TelemetryConfig, TelemetryLevel};
use uuid::Uuid;

const ROOT_QUICK_HELP: &str = "\
SealTask CLI — secure task management from your terminal

Get started:
  sealtask auth login        Sign in to SealTask
  sealtask auth unlock       Unlock workspace data
  sealtask projects list     List projects
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
    match run(cli, format, &args).await {
        Ok(()) => {}
        Err(CliError::BrokenPipe) => std::process::exit(0),
        Err(err) => {
            let _ = print_cli_error(&err, format);
            std::process::exit(err.exit_code());
        }
    }
}

fn parse_cli_or_exit(args: &[OsString], format: OutputFormat) -> Cli {
    let command = discovery::command();
    let matches = command
        .try_get_matches_from(args.iter().cloned())
        .unwrap_or_else(|err| exit_after_clap_error(err, format));
    Cli::from_arg_matches(&matches).unwrap_or_else(|err| exit_after_clap_error(err, format))
}

fn exit_after_clap_error(err: clap::Error, format: OutputFormat) -> ! {
    if format.is_json() && err.use_stderr() {
        let _ = print_clap_error(&err, format);
        std::process::exit(err.exit_code());
    }

    err.exit()
}

async fn run(cli: Cli, format: OutputFormat, raw_args: &[OsString]) -> CliResult<()> {
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

    if let Some(socket_path) = cli.serve_unlock_daemon.as_deref() {
        return serve(socket_path).await.map_err(Into::into);
    }

    match cli.command.as_ref() {
        Some(Command::Completion { shell }) => {
            ensure_raw_discovery_output(&cli, format, "completion")?;
            return discovery::print_completion(*shell);
        }
        Some(Command::Man {
            command,
            output_dir,
        }) => {
            ensure_raw_discovery_output(&cli, format, "man")?;
            return output_dir.as_deref().map_or_else(
                || discovery::print_manpage(command),
                discovery::generate_manpages,
            );
        }
        _ => {}
    }

    let telemetry_level = TelemetryLevel::from_flags(cli.verbosity, cli.debug);
    if format.is_json() && telemetry_level.enabled() {
        return Err(PublicError::validation(
            "-v, -vv, and --debug write diagnostic telemetry to stderr and cannot be combined with JSON output",
        )
        .into());
    }

    let operator_overrides = cli_overrides(&cli, raw_args)?;
    let resolved_config = if matches!(cli.command.as_ref(), Some(Command::Doctor { .. })) {
        resolve_operator_config_for_diagnostics(operator_overrides)?
    } else {
        resolve_operator_config(operator_overrides)?
    };
    configure_local_state(
        Some(resolved_config.config_dir.value.clone()),
        Some(&resolved_config.profile.value),
    )?;

    let invocation_id = Uuid::now_v7();
    let transport_options = ApiTransportOptions::new(
        resolved_config.connect_timeout.value,
        resolved_config.read_timeout.value,
        resolved_config.request_timeout.value,
    )?
    .with_request_id(invocation_id);
    let command = cli
        .command
        .expect("a command is present after handling root guidance");
    let command_name = command_name(&command);
    let telemetry = Telemetry::start(
        telemetry_level,
        invocation_id,
        command_name,
        TelemetryConfig {
            api_url: &resolved_config.api_url.value,
            profile_is_default: resolved_config.uses_default_profile(),
            profile_source: resolved_config.profile.source.as_str(),
            config_dir_source: resolved_config.config_dir.source.as_str(),
            timeouts: (
                resolved_config.connect_timeout.value,
                resolved_config.read_timeout.value,
                resolved_config.request_timeout.value,
            ),
        },
    );

    let local_only = matches!(
        &command,
        Command::Info
            | Command::Completion { .. }
            | Command::Man { .. }
            | Command::Schema { .. }
            | Command::Doctor { .. }
            | Command::Config { .. }
            | Command::Profile { .. }
    );
    let storage_origins = if local_only {
        &[][..]
    } else {
        cli.storage_origin.as_slice()
    };
    let runtime = RuntimeClient::with_storage_origins_and_transport(
        &resolved_config.api_url.value,
        storage_origins,
        transport_options,
    );

    let result = match (command, runtime) {
        (Command::Info, Ok(runtime)) => run_info(&runtime, format),
        (Command::Completion { .. } | Command::Man { .. }, Ok(_)) => {
            unreachable!("discovery commands return before operator configuration")
        }
        (Command::Schema { command }, _) => run_schema(format, &command),
        (Command::Auth { command }, Ok(runtime)) => {
            run_auth(&runtime, format, cli.non_interactive, command).await
        }
        (Command::Me, Ok(runtime)) => run_me(&runtime, format).await,
        (
            Command::Projects {
                verbose,
                include_archived,
                password_stdin,
                raw,
                command,
            },
            Ok(runtime),
        ) => {
            run_projects(
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
        (Command::Tasks { command }, Ok(runtime)) => {
            run_tasks(&runtime, format, cli.non_interactive, command).await
        }
        (Command::Stats, Ok(runtime)) => run_stats(&runtime, format).await,
        (
            Command::Doctor {
                offline,
                strict,
                include_keychain,
            },
            Ok(runtime),
        ) => {
            let result = doctor::run_doctor(
                &runtime,
                &resolved_config.config_dir.value,
                doctor::DoctorOptions {
                    offline,
                    strict,
                    include_keychain,
                },
            )
            .await;
            doctor::print_doctor_report(result.report(), format)?;
            result
                .status()
                .map_err(|failure| PublicError::validation(failure.to_string()).into())
        }
        (Command::Config { command }, _) => run_config(format, &resolved_config, command),
        (Command::Profile { command }, _) => run_profile(format, &resolved_config, command),
        (
            Command::Inspect {
                work_list_id,
                password_stdin,
            },
            Ok(runtime),
        ) => run_lists_get(&runtime, format, work_list_id, password_stdin, false).await,
        (Command::Comments { command }, Ok(runtime)) => {
            run_comments(&runtime, format, cli.non_interactive, command).await
        }
        (Command::Notes { command }, Ok(runtime)) => {
            run_notes(&runtime, format, cli.non_interactive, command).await
        }
        (_, Err(error)) => Err(error.into()),
    };
    telemetry.finish(&result);
    result
}

fn ensure_raw_discovery_output(cli: &Cli, format: OutputFormat, command: &str) -> CliResult<()> {
    if format.is_json() {
        return Err(PublicError::validation(format!(
            "'sealtask {command}' emits a raw terminal integration artifact and cannot be combined with --json, --format json, or --format json-pretty"
        ))
        .into());
    }
    if cli.verbosity > 0 || cli.debug {
        return Err(PublicError::validation(format!(
            "'sealtask {command}' cannot be combined with -v, -vv, or --debug because diagnostics would corrupt generated output"
        ))
        .into());
    }
    Ok(())
}

fn cli_overrides(cli: &Cli, raw_args: &[OsString]) -> CliResult<OperatorOverrides> {
    Ok(OperatorOverrides {
        api_url: long_option_present(raw_args, "--api-url").then(|| cli.api_url.clone()),
        profile: long_option_present(raw_args, "--profile")
            .then(|| cli.profile.clone())
            .flatten(),
        config_dir: long_option_present(raw_args, "--config-dir")
            .then(|| cli.config_dir.clone())
            .flatten(),
        connect_timeout: cli_timeout_override(
            raw_args,
            "--connect-timeout",
            cli.connect_timeout.as_deref(),
        )?,
        read_timeout: cli_timeout_override(
            raw_args,
            "--read-timeout",
            cli.read_timeout.as_deref(),
        )?,
        request_timeout: cli_timeout_override(
            raw_args,
            "--request-timeout",
            cli.request_timeout.as_deref(),
        )?,
    })
}

fn cli_timeout_override(
    raw_args: &[OsString],
    option: &str,
    value: Option<&str>,
) -> CliResult<Option<Duration>> {
    if !long_option_present(raw_args, option) {
        return Ok(None);
    }
    let value = value
        .ok_or_else(|| PublicError::validation(format!("{option} requires a duration value")))?;
    parse_timeout(value)
        .map(Some)
        .map_err(|error| PublicError::validation(format!("invalid {option}: {error}")).into())
}

fn long_option_present(raw_args: &[OsString], option: &str) -> bool {
    let option = OsStr::new(option);
    for argument in raw_args.iter().skip(1) {
        if argument == "--" {
            break;
        }
        if argument == option {
            return true;
        }
        if let Some(argument) = argument.to_str()
            && let Some((name, _)) = argument.split_once('=')
            && OsStr::new(name) == option
        {
            return true;
        }
    }
    false
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::Completion { .. } => "completion",
        Command::Man { .. } => "man",
        Command::Info => "info",
        Command::Schema { .. } => "schema",
        Command::Auth { .. } => "auth",
        Command::Me => "me",
        Command::Projects { .. } => "projects",
        Command::Tasks { .. } => "tasks",
        Command::Stats => "stats",
        Command::Doctor { .. } => "doctor",
        Command::Config { .. } => "config",
        Command::Profile { .. } => "profile",
        Command::Inspect { .. } => "inspect",
        Command::Comments { .. } => "comments",
        Command::Notes { .. } => "notes",
    }
}
