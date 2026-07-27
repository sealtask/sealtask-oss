#![cfg_attr(test, allow(clippy::unwrap_used))]

#[macro_use]
mod output;

mod args;
mod attachment_output;
mod commands;
mod discovery;
mod doctor;
mod editor;
mod human_input;
mod input;
mod interaction;
mod interruption;
mod live_output;
mod operator_config;
mod output_models;
mod picker;
mod project_context;
mod render;
mod resolver;
mod selectors;
mod table;
mod task_list;
mod telemetry;
mod terminal;

use args::{Cli, Command};
use clap::FromArgMatches;
use commands::{
    run_activity, run_auth, run_batch, run_browse, run_cache, run_comments, run_config, run_info,
    run_lists_get, run_me, run_notes, run_pick, run_profile, run_projects, run_schema, run_stats,
    run_tasks,
};
use operator_config::{
    OperatorOverrides, parse_timeout, resolve_operator_config,
    resolve_operator_config_for_diagnostics,
};
use output::{
    CliError, CliResult, OutputFormat, finish_with_warnings, print_clap_error, print_cli_error,
    warning_result,
};
use sealtask_client_api::{ApiCancellationToken, ApiRetryPolicy, ApiTransportOptions};
use sealtask_client_auth::configure_local_state;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::{ReadCacheOptions, RuntimeClient, serve};
use std::ffi::{OsStr, OsString};
use std::time::Duration;
use telemetry::{Telemetry, TelemetryConfig, TelemetryLevel};
use terminal::{TerminalOptions, TerminalSession};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let mut args = std::env::args_os().collect::<Vec<_>>();
    if args.len() == 1 {
        args.push(OsString::from("--help"));
    }
    let raw_format = OutputFormat::from_raw_args(&args);
    let cli = parse_cli_or_exit(&args, raw_format);
    let format = OutputFormat::from_cli(&cli);
    let raw_discovery = matches!(
        cli.command.as_ref(),
        Some(Command::Completion { .. } | Command::Man { .. })
    ) || cli.serve_unlock_daemon.is_some();
    let composable_picker = matches!(
        cli.command.as_ref(),
        Some(Command::Pick { .. } | Command::Browse(_))
    );
    let composable_field = matches!(
        cli.command.as_ref(),
        Some(Command::Tasks { command }) if task_list::is_raw_field_output(command)
    );
    let streaming = command_is_streaming(cli.command.as_ref());
    let guarded_mutation = interruption::needs_mutation_supervision(cli.command.as_ref());
    let cancellation = ApiCancellationToken::new();
    let terminal = if raw_discovery {
        Ok(None)
    } else {
        TerminalSession::start(TerminalOptions {
            color: cli.color,
            pager: cli.pager,
            pager_explicit: long_option_present(&args, "--pager"),
            no_pager: cli.no_pager,
            progress: cli.progress,
            progress_explicit: long_option_present(&args, "--progress"),
            quiet: cli.quiet,
            format,
            pager_allowed: !(composable_picker || composable_field || streaming),
        })
        .map(Some)
    };
    let result = match terminal {
        Ok(terminal) => {
            let result = if guarded_mutation {
                interruption::supervise_mutation(
                    run(cli, format, &args, cancellation.clone()),
                    cancellation,
                    format,
                )
                .await
            } else {
                run(cli, format, &args, cancellation).await
            };
            let terminal_result = terminal.map_or(Ok(()), TerminalSession::finish);
            match (result, terminal_result) {
                (Err(error), _) => Err(error),
                (Ok(()), terminal_result) => terminal_result,
            }
        }
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => {}
        Err(CliError::BrokenPipe) => std::process::exit(0),
        Err(err) => {
            terminal::clear_active_progress();
            let _ = print_cli_error(&err, format);
            std::process::exit(err.exit_code());
        }
    }
}

fn parse_cli_or_exit(args: &[OsString], format: OutputFormat) -> Cli {
    let command = discovery::command().color(terminal::clap_color_choice(args, format));
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

async fn run(
    cli: Cli,
    format: OutputFormat,
    raw_args: &[OsString],
    cancellation: ApiCancellationToken,
) -> CliResult<()> {
    if cli.command.is_none() && cli.serve_unlock_daemon.is_none() {
        if format.is_json() {
            return Err(PublicError::validation(
                "a command is required; run 'sealtask --help' to list commands",
            )
            .into());
        }
        return discovery::print_root_help(terminal::clap_color_choice(raw_args, format));
    }

    if let Some(socket_path) = cli.serve_unlock_daemon.as_deref() {
        return serve(socket_path).await.map_err(Into::into);
    }

    validate_stream_output(cli.command.as_ref(), format)?;
    validate_offline_command(&cli)?;

    match cli.command.as_ref() {
        Some(Command::Completion { shell }) => {
            ensure_raw_discovery_output(&cli, format, "completion", raw_args)?;
            return discovery::print_completion(*shell);
        }
        Some(Command::Man {
            command,
            output_dir,
        }) => {
            ensure_raw_discovery_output(&cli, format, "man", raw_args)?;
            return output_dir.as_deref().map_or_else(
                || discovery::print_manpage(command),
                discovery::generate_manpages,
            );
        }
        _ => {}
    }

    if matches!(
        cli.command.as_ref(),
        Some(Command::Pick { .. } | Command::Browse(_))
    ) {
        if format.is_json() {
            let message = if matches!(cli.command.as_ref(), Some(Command::Pick { .. })) {
                "'sealtask pick' emits one raw reusable selector and cannot be combined with --json or any JSON --format value"
            } else {
                "'sealtask browse' displays decrypted content on the controlling terminal and cannot be combined with --json or any JSON --format value"
            };
            return Err(PublicError::validation(message).into());
        }
        if cli.non_interactive {
            let message = if matches!(cli.command.as_ref(), Some(Command::Pick { .. })) {
                "'sealtask pick' requires an interactive controlling terminal; use a UUID, id:<prefix>, or exact name directly when running non-interactively"
            } else {
                "'sealtask browse' requires an interactive controlling terminal and cannot be combined with --non-interactive"
            };
            return Err(PublicError::validation(message).into());
        }
        picker::ensure_picker_terminal()?;
    }

    if let Some(Command::Tasks {
        command:
            args::TasksCommand::List {
                columns,
                field,
                web_url,
                ..
            },
    }) = cli.command.as_ref()
    {
        task_list::validate_output_mode(
            format,
            columns,
            *field,
            long_option_present(raw_args, "--web-url"),
        )?;
        if *field == Some(args::TaskListFieldArg::Url) {
            task_list::resolve_web_origin(web_url.as_deref(), &cli.api_url)?;
        }
    }

    if command_uses_editor(cli.command.as_ref()) {
        if cli.non_interactive {
            return Err(PublicError::validation(
                "editor workflows require an interactive controlling terminal and cannot be combined with --non-interactive; use explicit field or input-file flags in automation",
            )
            .into());
        }
        editor::ensure_editor_available()?;
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
    .with_retry_policy(ApiRetryPolicy::new(cli.retry)?)
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
            retry_limit: cli.retry,
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
            | Command::Cache { .. }
    );
    let storage_origins = if local_only {
        &[][..]
    } else {
        cli.storage_origin.as_slice()
    };
    let read_cache_options = if cli.offline {
        ReadCacheOptions::offline(
            resolved_config.profile_config_dir(),
            resolved_config.profile.value.clone(),
        )
    } else {
        ReadCacheOptions::online(
            resolved_config.profile_config_dir(),
            resolved_config.profile.value.clone(),
        )
    }?;
    let runtime = RuntimeClient::with_storage_origins_and_transport(
        &resolved_config.api_url.value,
        storage_origins,
        transport_options,
    )
    .and_then(|runtime| runtime.with_read_cache_options(read_cache_options))
    .map(|runtime| runtime.with_api_cancellation_token(cancellation));
    let runtime_observer = runtime.as_ref().ok().cloned();

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
        (Command::Pick { command }, Ok(runtime)) => run_pick(&runtime, command).await,
        (
            Command::Projects {
                legacy_verbose,
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
                legacy_verbose,
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
        (Command::Activity { command }, Ok(runtime)) => {
            run_activity(&runtime, format, command).await
        }
        (Command::Browse(args), Ok(runtime)) => run_browse(&runtime, args).await,
        (Command::Cache { command }, Ok(runtime)) => run_cache(&runtime, format, command).await,
        (Command::Batch { command }, Ok(runtime)) => run_batch(&runtime, format, command).await,
        (
            Command::Doctor {
                strict,
                include_keychain,
            },
            Ok(runtime),
        ) => {
            let result = doctor::run_doctor(
                &runtime,
                &resolved_config.config_dir.value,
                doctor::DoctorOptions {
                    offline: cli.offline,
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
    let result = if let Some(runtime) = runtime_observer {
        let mut warnings = runtime
            .take_read_cache_notices()
            .into_iter()
            .map(|notice| warning_result(notice.code, notice.message))
            .collect::<Vec<_>>();
        if runtime.is_offline() {
            warnings.extend(
                runtime
                    .take_read_cache_snapshots()
                    .into_iter()
                    .map(|snapshot| {
                        warning_result(
                            "offline_snapshot",
                            format!(
                                "read encrypted snapshot '{}' captured at {} ({} seconds old); no network request was attempted",
                                snapshot.query,
                                snapshot.captured_at.to_rfc3339(),
                                snapshot.age_seconds
                            ),
                        )
                    }),
            );
        }
        finish_with_warnings(format, &warnings, result)
    } else {
        result
    };
    telemetry.finish(&result);
    result
}

fn ensure_raw_discovery_output(
    cli: &Cli,
    format: OutputFormat,
    command: &str,
    raw_args: &[OsString],
) -> CliResult<()> {
    if format.is_json() {
        return Err(PublicError::validation(format!(
            "'sealtask {command}' emits a raw terminal integration artifact and cannot be combined with --json or any JSON --format value"
        ))
        .into());
    }
    if cli.verbosity > 0 || cli.debug {
        return Err(PublicError::validation(format!(
            "'sealtask {command}' cannot be combined with -v, -vv, or --debug because diagnostics would corrupt generated output"
        ))
        .into());
    }
    if ["--color", "--pager", "--no-pager", "--progress", "--quiet"]
        .iter()
        .any(|option| long_option_present(raw_args, option))
        || raw_args
            .iter()
            .skip(1)
            .any(|argument| argument == OsStr::new("-q"))
    {
        return Err(PublicError::validation(format!(
            "'sealtask {command}' cannot be combined with --color, --pager, --no-pager, --progress, or --quiet because it emits an exact raw artifact"
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
        Command::Pick { .. } => "pick",
        Command::Projects { .. } => "projects",
        Command::Tasks { .. } => "tasks",
        Command::Stats => "stats",
        Command::Activity { .. } => "activity",
        Command::Browse(_) => "browse",
        Command::Cache { .. } => "cache",
        Command::Batch { .. } => "batch",
        Command::Doctor { .. } => "doctor",
        Command::Config { .. } => "config",
        Command::Profile { .. } => "profile",
        Command::Inspect { .. } => "inspect",
        Command::Comments { .. } => "comments",
        Command::Notes { .. } => "notes",
    }
}

fn validate_offline_command(cli: &Cli) -> CliResult<()> {
    if !cli.offline {
        return Ok(());
    }
    let allowed = match cli.command.as_ref() {
        Some(
            Command::Completion { .. }
            | Command::Man { .. }
            | Command::Info
            | Command::Schema { .. }
            | Command::Doctor { .. }
            | Command::Config { .. }
            | Command::Profile { .. }
            | Command::Cache { .. }
            | Command::Browse(_)
            | Command::Pick { .. },
        ) => true,
        Some(Command::Auth {
            command: args::AuthCommand::Status,
        }) => true,
        Some(Command::Projects {
            raw,
            command: None | Some(args::ProjectsCommand::Current),
            ..
        }) => !raw,
        Some(Command::Projects {
            raw,
            command:
                Some(
                    args::ProjectsCommand::List {
                        raw: command_raw, ..
                    }
                    | args::ProjectsCommand::Get {
                        raw: command_raw, ..
                    },
                ),
            ..
        }) => !raw && !command_raw,
        Some(Command::Projects {
            raw,
            command: Some(args::ProjectsCommand::Sections { .. }),
            ..
        }) => !raw,
        Some(Command::Tasks {
            command: args::TasksCommand::List { raw, .. } | args::TasksCommand::Get { raw, .. },
        }) => !raw,
        Some(Command::Comments {
            command: args::CommentsCommand::List { .. },
        }) => true,
        Some(Command::Notes {
            command: args::NotesCommand::List { .. } | args::NotesCommand::Get { .. },
        }) => true,
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(PublicError::validation(
            "--offline is read-only and supports cached project/task/comment/note reads, pick, browse, cache controls, auth status, and local discovery commands; remove --offline for this command (no network request was attempted)",
        )
        .into())
    }
}

fn command_is_streaming(command: Option<&Command>) -> bool {
    matches!(
        command,
        Some(Command::Tasks {
            command: args::TasksCommand::Watch { .. },
        }) | Some(Command::Activity {
            command: args::ActivityCommand::Follow { .. },
        }) | Some(Command::Batch { .. })
    )
}

fn validate_stream_output(command: Option<&Command>, format: OutputFormat) -> CliResult<()> {
    if command_is_streaming(command)
        && matches!(format, OutputFormat::Json | OutputFormat::JsonPretty)
    {
        return Err(PublicError::validation(
            "streaming commands do not produce one finite JSON document; use '--format jsonl' for machine-readable records or table output for humans",
        )
        .into());
    }
    Ok(())
}

fn command_uses_editor(command: Option<&Command>) -> bool {
    matches!(
        command,
        Some(Command::Tasks {
            command: args::TasksCommand::Create(args),
        }) if args.edit
    ) || matches!(
        command,
        Some(Command::Tasks {
            command: args::TasksCommand::Edit(_),
        })
    ) || matches!(
        command,
        Some(Command::Notes {
            command: args::NotesCommand::Edit(_),
        })
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(arguments: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("sealtask").chain(arguments.iter().copied()))
            .expect("parse CLI")
    }

    #[test]
    fn offline_allowlist_is_explicit_and_rejects_raw_or_remote_work() {
        for arguments in [
            vec!["--offline", "projects", "list"],
            vec!["projects", "current", "--offline"],
            vec!["--offline", "tasks", "list", "--all"],
            vec!["--offline", "notes", "list"],
            vec!["--offline", "pick", "project"],
            vec!["--offline", "browse"],
            vec!["cache", "status", "--offline"],
            vec!["doctor", "--offline"],
            vec!["--offline", "auth", "status"],
        ] {
            let cli = parse(&arguments);
            assert!(
                validate_offline_command(&cli).is_ok(),
                "offline command unexpectedly rejected: {arguments:?}"
            );
        }

        for arguments in [
            vec!["--offline", "me"],
            vec!["--offline", "stats"],
            vec!["--offline", "projects", "audit"],
            vec!["--offline", "projects", "list", "--raw"],
            vec!["--offline", "tasks", "watch"],
            vec![
                "--offline",
                "tasks",
                "create",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--title",
                "must not run",
            ],
            vec!["--offline", "auth", "login"],
            vec!["--offline", "batch", "run", "--input", "-"],
        ] {
            let cli = parse(&arguments);
            let error = validate_offline_command(&cli)
                .expect_err("remote or raw command must be rejected offline");
            assert_eq!(error.code(), "validation");
            assert!(
                error
                    .to_string()
                    .contains("no network request was attempted")
            );
        }
    }
}
