#![cfg_attr(test, allow(clippy::unwrap_used))]

mod harness;
mod service;

use std::{path::PathBuf, time::Duration};

use clap::Parser;
use uuid::Uuid;

use sealtask_client_auth::configure_local_state;
use sealtask_client_core::{PublicError, PublicResult};

use crate::{
    harness::CodexHarness,
    service::{AgentService, PollOutcome},
};

#[derive(Debug, Parser)]
#[command(
    name = "sealtask-agent",
    version,
    about = "Run project-scoped SealTask agent identities with a local Codex harness"
)]
struct Args {
    /// Override the base directory used for local SealTask state.
    #[arg(long, env = "SEALTASK_CONFIG_DIR")]
    config_dir: Option<PathBuf>,

    /// Select the same isolated local profile used by the sealtask CLI.
    #[arg(long, env = "SEALTASK_PROFILE")]
    profile: Option<String>,

    /// Run only one locally enrolled agent identity.
    #[arg(long)]
    agent_id: Option<Uuid>,

    /// Poll once and exit, whether or not an assignment was claimable.
    #[arg(long)]
    once: bool,

    /// Seconds between assignment polls.
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(5..=3600))]
    poll_interval_seconds: u64,

    /// Maximum seconds for one claimed run, including Git setup and Codex execution.
    #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(60..=86400))]
    run_timeout_seconds: u64,

    /// Codex executable to launch.
    #[arg(long, default_value = "codex")]
    codex_bin: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(error) = run(args).await {
        eprintln!("sealtask-agent: {error}");
        std::process::exit(1);
    }
}

async fn run(args: Args) -> PublicResult<()> {
    configure_local_state(args.config_dir, args.profile.as_deref())?;
    let harness = CodexHarness::new(args.codex_bin);
    let service = AgentService::new(
        Uuid::now_v7(),
        args.agent_id,
        Duration::from_secs(args.run_timeout_seconds),
    );
    let poll_interval = Duration::from_secs(args.poll_interval_seconds);
    let (shutdown_sender, mut shutdown_receiver) = tokio::sync::watch::channel(false);
    let signal_task = tokio::spawn(async move {
        let result = shutdown_signal().await;
        let _ = shutdown_sender.send(true);
        result
    });

    let mut unconfirmed_terminal_runs;
    let shutdown_had_failures = loop {
        // The poll owns any claimed run until it records a terminal state.
        // Shutdown is cooperative inside the service so this future is never
        // dropped while a Codex harness or lease is active.
        let outcome = service.poll_once(&harness, &mut shutdown_receiver).await?;
        unconfirmed_terminal_runs = outcome.unconfirmed_terminal_runs;
        for completed in &outcome.completed_runs {
            if let Some(worktree) = &completed.worktree {
                eprintln!(
                    "sealtask-agent: agent={} run={} status={} worktree={}",
                    completed.agent_id,
                    completed.run_id,
                    completed.status,
                    worktree.display()
                );
            } else {
                eprintln!(
                    "sealtask-agent: agent={} run={} status={}",
                    completed.agent_id, completed.run_id, completed.status
                );
            }
        }
        for failure in &outcome.failures {
            eprintln!(
                "sealtask-agent: agent={} poll failed: {}",
                failure.agent_id, failure.message
            );
        }
        if *shutdown_receiver.borrow() {
            break unconfirmed_terminal_runs > 0;
        }
        if outcome.configured_identities == 0 {
            signal_task.abort();
            let _ = signal_task.await;
            return Err(PublicError::validation(
                "no local agent identities; register one with the sealtask CLI",
            ));
        }
        if args.once {
            signal_task.abort();
            let _ = signal_task.await;
            return validate_once_outcome(&outcome);
        }
        tokio::select! {
            () = tokio::time::sleep(poll_interval) => {}
            changed = shutdown_receiver.changed() => {
                if changed.is_err() || *shutdown_receiver.borrow() {
                    break unconfirmed_terminal_runs > 0;
                }
            }
        }
    };

    signal_task.await.map_err(|error| {
        PublicError::unexpected(format!(
            "agent signal listener stopped unexpectedly: {error}"
        ))
    })??;
    if shutdown_had_failures {
        return Err(PublicError::unexpected(
            "agent shutdown could not confirm terminal state for one or more active runs",
        ));
    }
    Ok(())
}

fn validate_once_outcome(outcome: &PollOutcome) -> PublicResult<()> {
    if outcome.unconfirmed_terminal_runs > 0 {
        return Err(PublicError::unexpected(
            "agent run completion could not be confirmed before its lease deadline",
        ));
    }
    if !outcome.failures.is_empty() {
        return Err(PublicError::unexpected(format!(
            "{} agent identity poll(s) failed",
            outcome.failures.len()
        )));
    }

    let unsuccessful_runs = outcome
        .completed_runs
        .iter()
        .filter(|run| run.status != "succeeded")
        .count();
    if unsuccessful_runs > 0 {
        return Err(PublicError::unexpected(format!(
            "{unsuccessful_runs} agent run(s) completed unsuccessfully"
        )));
    }

    Ok(())
}

async fn shutdown_signal() -> PublicResult<()> {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.map_err(|error| {
            PublicError::unexpected(format!("failed to listen for Ctrl-C: {error}"))
        })
    };

    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).map_err(
                |error| PublicError::unexpected(format!("failed to listen for SIGTERM: {error}")),
            )?;
        tokio::select! {
            result = ctrl_c => result,
            _ = terminate.recv() => Ok(()),
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{CompletedRun, PollFailure};

    #[test]
    fn once_outcome_accepts_success_and_no_assignment() {
        validate_once_outcome(&PollOutcome {
            configured_identities: 1,
            ..PollOutcome::default()
        })
        .expect("no assignment is a successful poll");
        validate_once_outcome(&PollOutcome {
            configured_identities: 1,
            completed_runs: vec![CompletedRun {
                agent_id: Uuid::now_v7(),
                run_id: Uuid::now_v7(),
                status: "succeeded".to_string(),
                worktree: None,
            }],
            ..PollOutcome::default()
        })
        .expect("a successful run is a successful poll");
    }

    #[test]
    fn once_outcome_rejects_identity_poll_failures() {
        let error = validate_once_outcome(&PollOutcome {
            configured_identities: 1,
            failures: vec![PollFailure {
                agent_id: Uuid::now_v7(),
                message: "authentication failed".to_string(),
            }],
            ..PollOutcome::default()
        })
        .expect_err("poll failures must make one-shot execution fail");

        assert_eq!(error.code(), "unexpected");
        assert!(
            error
                .to_string()
                .contains("1 agent identity poll(s) failed")
        );
    }

    #[test]
    fn once_outcome_rejects_unsuccessful_and_unconfirmed_runs() {
        for status in ["failed", "failed (grant_authentication)", "cancelled"] {
            let error = validate_once_outcome(&PollOutcome {
                configured_identities: 1,
                completed_runs: vec![CompletedRun {
                    agent_id: Uuid::now_v7(),
                    run_id: Uuid::now_v7(),
                    status: status.to_string(),
                    worktree: None,
                }],
                ..PollOutcome::default()
            })
            .expect_err("non-successful runs must make one-shot execution fail");
            assert!(error.to_string().contains("completed unsuccessfully"));
        }

        let error = validate_once_outcome(&PollOutcome {
            configured_identities: 1,
            unconfirmed_terminal_runs: 1,
            ..PollOutcome::default()
        })
        .expect_err("unconfirmed terminal results must make one-shot execution fail");
        assert!(error.to_string().contains("could not be confirmed"));
    }
}
