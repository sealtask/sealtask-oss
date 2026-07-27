use crate::args::{
    Command, CommentsCommand, NotesCommand, ProjectsCommand, TaskAttachmentsCommand,
    TaskReferencesCommand, TasksCommand,
};
use crate::output::{
    CliError, CliResult, OutputFormat, emit_warnings_best_effort, finish_with_warnings,
    warning_result,
};
use sealtask_client_api::ApiCancellationToken;
use sealtask_client_core::{PublicError, TransportFailureKind};
use std::future::Future;
use std::io;
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub(crate) const MUTATION_INTERRUPT_GRACE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignalState {
    Listening,
    Received(u8),
    Failed,
}

pub(crate) struct SignalMonitor {
    receiver: watch::Receiver<SignalState>,
    task: JoinHandle<()>,
    #[cfg(unix)]
    registrations: UnixSignalRegistrations,
}

#[derive(Clone)]
pub(crate) struct SignalReceiver {
    receiver: watch::Receiver<SignalState>,
    observed_level: u8,
    #[cfg(unix)]
    delivered: Arc<AtomicUsize>,
}

impl Drop for SignalMonitor {
    fn drop(&mut self) {
        #[cfg(unix)]
        self.registrations.unregister();
        self.task.abort();
    }
}

/// Returns whether this command can cross a remote mutation boundary without
/// already owning a more specialized interruption supervisor.
pub(crate) fn needs_mutation_supervision(command: Option<&Command>) -> bool {
    match command {
        Some(Command::Projects {
            command: Some(ProjectsCommand::Archive { .. } | ProjectsCommand::Unarchive { .. }),
            ..
        }) => true,
        Some(Command::Tasks { command }) => match command {
            TasksCommand::Create(args) => !args.dry_run,
            TasksCommand::Update(args) => !args.dry_run,
            TasksCommand::Edit(_)
            | TasksCommand::Move(_)
            | TasksCommand::Complete(_)
            | TasksCommand::Reopen(_)
            | TasksCommand::Archive(_)
            | TasksCommand::Unarchive(_)
            | TasksCommand::Delete(_) => true,
            TasksCommand::Attachments {
                command: TaskAttachmentsCommand::Delete(_),
            } => true,
            TasksCommand::TaskReferences { command } => matches!(
                command,
                TaskReferencesCommand::Repair(_) | TaskReferencesCommand::Quarantine(_)
            ),
            TasksCommand::List { .. }
            | TasksCommand::Get { .. }
            | TasksCommand::Resolve(_)
            | TasksCommand::Watch { .. }
            | TasksCommand::Attachments {
                command:
                    TaskAttachmentsCommand::Upload(_)
                    | TaskAttachmentsCommand::Read(_)
                    | TaskAttachmentsCommand::Download(_),
            } => false,
        },
        Some(Command::Comments { command }) => matches!(
            command,
            CommentsCommand::Create(_) | CommentsCommand::Update(_) | CommentsCommand::Delete(_)
        ),
        Some(Command::Notes { command }) => matches!(
            command,
            NotesCommand::Create(_)
                | NotesCommand::Edit(_)
                | NotesCommand::Update(_)
                | NotesCommand::Delete(_)
        ),
        Some(
            Command::Completion { .. }
            | Command::Man { .. }
            | Command::Info
            | Command::Schema { .. }
            | Command::Auth { .. }
            | Command::Me
            | Command::Pick { .. }
            | Command::Browse(_)
            | Command::Cache { .. }
            | Command::Projects { .. }
            | Command::Stats
            | Command::Activity { .. }
            | Command::Batch { .. }
            | Command::Doctor { .. }
            | Command::Config { .. }
            | Command::Profile { .. }
            | Command::Inspect { .. },
        )
        | None => false,
    }
}

pub(crate) async fn supervise_mutation<F>(
    operation: F,
    cancellation: ApiCancellationToken,
    format: OutputFormat,
) -> CliResult<()>
where
    F: Future<Output = CliResult<()>>,
{
    let monitor = SignalMonitor::start()?;
    supervise_mutation_with_signals(
        operation,
        monitor.subscribe(),
        MUTATION_INTERRUPT_GRACE,
        format,
        &cancellation,
        || {
            cancellation.credential_refresh_in_flight()
                || cancellation.credential_refresh_may_have_rotated()
        },
        || cancellation.mutation_request_in_flight(),
    )
    .await
}

async fn supervise_mutation_with_signals<F, R, M>(
    operation: F,
    signals: SignalReceiver,
    grace: Duration,
    format: OutputFormat,
    cancellation: &ApiCancellationToken,
    credential_refresh_active: R,
    mutation_active: M,
) -> CliResult<()>
where
    F: Future<Output = CliResult<()>>,
    R: Fn() -> bool,
    M: Fn() -> bool,
{
    tokio::pin!(operation);
    let first_signal = signals.clone().wait_for(1);
    tokio::pin!(first_signal);
    let first_signal_result = tokio::select! {
        biased;
        result = &mut first_signal => result,
        result = &mut operation => {
            match signals.level() {
                Ok(0) => return result,
                Ok(_) => Ok(()),
                Err(signal_error) => {
                    let warning = warning_result(
                        "signal_listener_failed",
                        format!(
                            "failed to inspect process interruption state after the mutation completed: {signal_error}"
                        ),
                    );
                    return finish_with_warnings(format, &[warning], result);
                }
            }
        },
    };

    if let Err(signal_error) = first_signal_result {
        let warning = warning_result(
            "signal_listener_failed",
            format!(
                "failed to listen for process interruption; the mutation will continue to its normal timeout: {signal_error}"
            ),
        );
        return finish_with_warnings(format, &[warning], operation.await);
    }

    if credential_refresh_active() {
        cancellation.cancel();
        let deferred = warning_result(
            "credential_refresh_interruption_deferred",
            format!(
                "interrupt received while credentials are rotating; waiting up to {} seconds for durable persistence before cancelling the requested resource operation (interrupt again to stop waiting)",
                grace.as_secs()
            ),
        );
        if !format.is_json() {
            emit_warnings_best_effort(format, std::slice::from_ref(&deferred));
        }
        let second_signal = signals.wait_for(2);
        tokio::pin!(second_signal);
        let deadline = tokio::time::sleep(grace);
        tokio::pin!(deadline);
        return tokio::select! {
            biased;
            result = &mut operation => finish_after_credential_refresh(format, deferred, result),
            signal_result = &mut second_signal => {
                match signal_result {
                    Ok(()) => Err(ambiguous_credential_refresh_interruption(
                        format,
                        std::slice::from_ref(&deferred),
                        "credential refresh interrupted by a second signal before durable persistence",
                        "credential_refresh_interruption_forced",
                    )),
                    Err(signal_error) => {
                        let listener_failed = warning_result(
                            "signal_listener_failed",
                            format!(
                                "failed to listen for a second process interruption; credential refresh will continue to its normal timeout: {signal_error}"
                            ),
                        );
                        finish_after_credential_refresh_warnings(
                            format,
                            &[deferred, listener_failed],
                            operation.await,
                        )
                    }
                }
            }
            () = &mut deadline => {
                Err(ambiguous_credential_refresh_interruption(
                    format,
                    std::slice::from_ref(&deferred),
                    "credential refresh interruption timed out before durable persistence",
                    "credential_refresh_interruption_timed_out",
                ))
            }
        };
    }

    // A signal during local validation, prompting, encryption, discovery, or
    // final rendering can safely cancel the command. Only a currently active
    // durable mutation request creates an ambiguous remote outcome.
    if !mutation_active() {
        cancellation.cancel();
        return Err(CliError::interrupted(
            "command interrupted with no mutation request in flight; the remote outcome is not ambiguous",
            &[],
        ));
    }

    let deferred = warning_result(
        "mutation_interruption_deferred",
        format!(
            "interrupt received; waiting up to {} seconds for the in-flight mutation to reach a definitive response (interrupt again to stop waiting)",
            grace.as_secs()
        ),
    );
    if !format.is_json() {
        emit_warnings_best_effort(format, std::slice::from_ref(&deferred));
    }

    let second_signal = signals.wait_for(2);
    tokio::pin!(second_signal);
    let deadline = tokio::time::sleep(grace);
    tokio::pin!(deadline);

    tokio::select! {
        biased;
        result = &mut operation => finish_after_deferred_warning(format, deferred, result),
        signal_result = &mut second_signal => {
            match signal_result {
                Ok(()) if mutation_active() => Err(ambiguous_interruption(
                    format,
                    deferred,
                    "mutation interrupted by a second signal before a definitive response",
                    "mutation_interruption_forced",
                )),
                Ok(()) => Err(definitive_interruption(format, deferred)),
                Err(signal_error) => {
                    let listener_failed = warning_result(
                        "signal_listener_failed",
                        format!(
                            "failed to listen for a second process interruption; the mutation will continue to its normal timeout: {signal_error}"
                        ),
                    );
                    finish_after_deferred_warnings(
                        format,
                        &[deferred, listener_failed],
                        operation.await,
                    )
                }
            }
        }
        () = &mut deadline => {
            if mutation_active() {
                Err(ambiguous_interruption(
                    format,
                    deferred,
                    "mutation interruption timed out before a definitive response",
                    "mutation_interruption_timed_out",
                ))
            } else {
                Err(definitive_interruption(format, deferred))
            }
        }
    }
}

fn finish_after_credential_refresh(
    format: OutputFormat,
    deferred: crate::output::WarningResult,
    result: CliResult<()>,
) -> CliResult<()> {
    finish_after_credential_refresh_warnings(format, &[deferred], result)
}

fn finish_after_credential_refresh_warnings(
    format: OutputFormat,
    deferred: &[crate::output::WarningResult],
    result: CliResult<()>,
) -> CliResult<()> {
    if result
        .as_ref()
        .is_err_and(|error| error.code() == "cancelled")
    {
        let warnings = deferred_warnings_for_error(format, deferred);
        return Err(CliError::interrupted(
            "command interrupted after refreshed credentials were durably persisted; the requested resource mutation was not sent",
            &warnings,
        ));
    }
    if result
        .as_ref()
        .is_err_and(cli_error_may_follow_remote_credential_rotation)
    {
        return Err(ambiguous_credential_refresh_interruption(
            format,
            deferred,
            "credential refresh failed after interruption before durable local persistence could be confirmed",
            "credential_refresh_outcome_ambiguous",
        ));
    }
    finish_after_deferred_warnings(format, deferred, result)
}

fn ambiguous_credential_refresh_interruption(
    format: OutputFormat,
    deferred: &[crate::output::WarningResult],
    message: &str,
    warning_code: &'static str,
) -> CliError {
    let forced = warning_result(
        warning_code,
        "stopped waiting before rotating credentials were durably persisted; the remote session rotation may be ambiguous and signing in again may be required"
            .to_string(),
    );
    let mut warnings = deferred_warnings_for_error(format, deferred);
    warnings.push(forced);
    CliError::interrupted_session_ambiguous(
        format!(
            "{message}; the server may have rotated the session without a durable local replacement, so run 'sealtask auth login' if the current session no longer works"
        ),
        &warnings,
    )
}

fn deferred_warnings_for_error(
    format: OutputFormat,
    deferred: &[crate::output::WarningResult],
) -> Vec<crate::output::WarningResult> {
    if format.is_json() {
        deferred.to_vec()
    } else {
        deferred.iter().skip(1).cloned().collect()
    }
}

fn cli_error_may_follow_remote_credential_rotation(error: &CliError) -> bool {
    match error {
        CliError::Public(error) | CliError::PublicWithWarnings { error, .. } => {
            credential_refresh_failure_may_have_rotated(error)
        }
        CliError::BrokenPipe | CliError::BatchStatus { .. } | CliError::Interrupted { .. } => false,
    }
}

pub(crate) fn credential_refresh_failure_may_have_rotated(error: &PublicError) -> bool {
    match error {
        PublicError::Transport(failure) => failure.kind() != TransportFailureKind::Connect,
        PublicError::Response { .. }
        | PublicError::Unexpected(_)
        | PublicError::RequestTimeout(_)
        | PublicError::CompensationFailed { .. }
        | PublicError::OutcomeAmbiguous { .. }
        | PublicError::CommittedButLocalProcessingFailed { .. } => true,
        PublicError::Validation(_)
        | PublicError::NotFound(_)
        | PublicError::Conflict(_)
        | PublicError::Entitlement(_)
        | PublicError::PayloadTooLarge(_)
        | PublicError::RateLimited(_)
        | PublicError::Crypto(_)
        | PublicError::Cancelled(_)
        | PublicError::Http(_)
        | PublicError::MfaRequiredUseBeginLogin
        | PublicError::MfaInputRequired => false,
        _ => true,
    }
}

fn finish_after_deferred_warning(
    format: OutputFormat,
    deferred: crate::output::WarningResult,
    result: CliResult<()>,
) -> CliResult<()> {
    finish_after_deferred_warnings(format, &[deferred], result)
}

fn finish_after_deferred_warnings(
    format: OutputFormat,
    warnings: &[crate::output::WarningResult],
    result: CliResult<()>,
) -> CliResult<()> {
    if format.is_json() {
        finish_with_warnings(format, warnings, result)
    } else {
        if warnings.len() > 1 {
            emit_warnings_best_effort(format, &warnings[1..]);
        }
        result
    }
}

fn definitive_interruption(
    format: OutputFormat,
    deferred: crate::output::WarningResult,
) -> CliError {
    let warnings = format
        .is_json()
        .then_some(deferred)
        .into_iter()
        .collect::<Vec<_>>();
    CliError::interrupted(
        "command interrupted after the mutation request reached a definitive response; the remote outcome is not ambiguous",
        &warnings,
    )
}

fn ambiguous_interruption(
    format: OutputFormat,
    deferred: crate::output::WarningResult,
    message: &str,
    warning_code: &'static str,
) -> CliError {
    let forced = warning_result(
        warning_code,
        "stopped waiting before a definitive response; the remote mutation outcome may be ambiguous"
            .to_string(),
    );
    let warnings = if format.is_json() {
        vec![deferred, forced]
    } else {
        vec![forced]
    };
    CliError::interrupted_ambiguous(
        format!(
            "{message}; inspect the resource before retrying because the mutation may have committed"
        ),
        &warnings,
    )
}

impl SignalMonitor {
    pub(crate) fn start() -> CliResult<Self> {
        let (sender, receiver) = watch::channel(SignalState::Listening);

        #[cfg(unix)]
        {
            let delivered = Arc::new(AtomicUsize::new(0));
            // Register the authoritative counter first so Tokio's wakeup can
            // never publish an older generation from the same signal.
            let registrations = UnixSignalRegistrations::install(delivered.clone())?;
            let task = spawn_signal_task(sender, delivered.clone())?;
            Ok(Self {
                receiver,
                task,
                registrations,
            })
        }

        #[cfg(not(unix))]
        {
            let task = spawn_signal_task(sender)?;
            Ok(Self { receiver, task })
        }
    }

    pub(crate) fn subscribe(&self) -> SignalReceiver {
        SignalReceiver {
            receiver: self.receiver.clone(),
            observed_level: 0,
            #[cfg(unix)]
            delivered: self.registrations.delivered.clone(),
        }
    }
}

impl SignalReceiver {
    pub(crate) fn level(&self) -> io::Result<u8> {
        self.effective_level(*self.receiver.borrow())
    }

    pub(crate) async fn changed(&mut self) -> io::Result<u8> {
        loop {
            let state = *self.receiver.borrow_and_update();
            let level = self.effective_level(state)?;
            if level > self.observed_level {
                self.observed_level = level;
                return Ok(level);
            }
            self.receiver
                .changed()
                .await
                .map_err(|_| io::Error::other("the process interruption listener stopped"))?;
        }
    }

    pub(crate) async fn wait_for(mut self, minimum_level: u8) -> io::Result<()> {
        loop {
            let state = *self.receiver.borrow_and_update();
            let level = self.effective_level(state)?;
            self.observed_level = self.observed_level.max(level);
            if level >= minimum_level {
                return Ok(());
            }
            if self.receiver.changed().await.is_err() {
                return Err(io::Error::other(
                    "the process interruption listener stopped",
                ));
            }
        }
    }

    fn effective_level(&self, state: SignalState) -> io::Result<u8> {
        let published = signal_level(state)?;
        #[cfg(unix)]
        {
            Ok(published.max(delivered_signal_level(&self.delivered)))
        }
        #[cfg(not(unix))]
        {
            Ok(published)
        }
    }
}

fn signal_level(state: SignalState) -> io::Result<u8> {
    match state {
        SignalState::Listening => Ok(0),
        SignalState::Received(level) => Ok(level),
        SignalState::Failed => Err(io::Error::other(
            "the process interruption signal stream closed",
        )),
    }
}

#[cfg(unix)]
// Tokio publishes signal delivery from an async task. Keep a generation
// counter in the signal handler as the authoritative state so a concurrently
// ready operation cannot finish in the scheduler gap before that publication.
struct UnixSignalRegistrations {
    delivered: Arc<AtomicUsize>,
    interrupt: signal_hook_registry::SigId,
    terminate: signal_hook_registry::SigId,
    active: bool,
}

#[cfg(unix)]
impl UnixSignalRegistrations {
    fn install(delivered: Arc<AtomicUsize>) -> CliResult<Self> {
        let interrupt = register_unix_signal(libc::SIGINT, delivered.clone())?;
        let terminate = match register_unix_signal(libc::SIGTERM, delivered.clone()) {
            Ok(registration) => registration,
            Err(error) => {
                signal_hook_registry::unregister(interrupt);
                return Err(error);
            }
        };
        Ok(Self {
            delivered,
            interrupt,
            terminate,
            active: true,
        })
    }

    fn unregister(&mut self) {
        if !self.active {
            return;
        }
        signal_hook_registry::unregister(self.interrupt);
        signal_hook_registry::unregister(self.terminate);
        self.active = false;
    }
}

#[cfg(unix)]
impl Drop for UnixSignalRegistrations {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[cfg(unix)]
fn register_unix_signal(
    signal: libc::c_int,
    delivered: Arc<AtomicUsize>,
) -> CliResult<signal_hook_registry::SigId> {
    // SAFETY: The handler only performs a lock-free atomic update. It neither
    // allocates nor locks, and the captured counter remains alive until the
    // registration is removed.
    unsafe {
        signal_hook_registry::register(signal, move || {
            increment_signal_generation(&delivered);
        })
    }
    .map_err(signal_install_error)
}

#[cfg(unix)]
fn increment_signal_generation(delivered: &AtomicUsize) {
    let _ = delivered.fetch_update(Ordering::Release, Ordering::Relaxed, |level| {
        Some(level.saturating_add(1))
    });
}

#[cfg(unix)]
fn delivered_signal_level(delivered: &AtomicUsize) -> u8 {
    delivered.load(Ordering::Acquire).min(usize::from(u8::MAX)) as u8
}

#[cfg(unix)]
fn spawn_signal_task(
    sender: watch::Sender<SignalState>,
    delivered: Arc<AtomicUsize>,
) -> CliResult<JoinHandle<()>> {
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(signal_install_error)?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(signal_install_error)?;
    Ok(tokio::spawn(async move {
        loop {
            let received = tokio::select! {
                value = interrupt.recv() => value,
                value = terminate.recv() => value,
            };
            if received.is_none() {
                let _ = sender.send(SignalState::Failed);
                return;
            }
            if sender
                .send(SignalState::Received(delivered_signal_level(&delivered)))
                .is_err()
            {
                return;
            }
        }
    }))
}

#[cfg(windows)]
fn spawn_signal_task(sender: watch::Sender<SignalState>) -> CliResult<JoinHandle<()>> {
    let mut ctrl_c = tokio::signal::windows::ctrl_c().map_err(signal_install_error)?;
    let mut ctrl_break = tokio::signal::windows::ctrl_break().map_err(signal_install_error)?;
    Ok(tokio::spawn(async move {
        let mut level = 0_u8;
        loop {
            let received = tokio::select! {
                value = ctrl_c.recv() => value,
                value = ctrl_break.recv() => value,
            };
            if received.is_none() {
                let _ = sender.send(SignalState::Failed);
                return;
            }
            level = level.saturating_add(1);
            if sender.send(SignalState::Received(level)).is_err() {
                return;
            }
        }
    }))
}

#[cfg(not(any(unix, windows)))]
fn spawn_signal_task(sender: watch::Sender<SignalState>) -> CliResult<JoinHandle<()>> {
    Ok(tokio::spawn(async move {
        let mut level = 0_u8;
        loop {
            if tokio::signal::ctrl_c().await.is_err() {
                let _ = sender.send(SignalState::Failed);
                return;
            }
            level = level.saturating_add(1);
            if sender.send(SignalState::Received(level)).is_err() {
                return;
            }
        }
    }))
}

fn signal_install_error(error: io::Error) -> CliError {
    PublicError::unexpected(format!(
        "failed to install process interruption listener: {error}"
    ))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Cli;
    use clap::Parser;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::oneshot;

    #[test]
    fn only_unsupervised_remote_mutations_use_the_generic_guard() {
        for args in [
            vec![
                "sealtask",
                "tasks",
                "create",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--title",
                "ship",
            ],
            vec![
                "sealtask",
                "projects",
                "archive",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
            ],
            vec![
                "sealtask",
                "comments",
                "delete",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--task-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
                "--comment-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8eea",
                "--yes",
            ],
        ] {
            let cli = Cli::try_parse_from(args).expect("mutation arguments");
            assert!(needs_mutation_supervision(cli.command.as_ref()));
        }

        for args in [
            vec!["sealtask", "tasks", "list", "--all"],
            vec![
                "sealtask",
                "tasks",
                "create",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--title",
                "ship",
                "--dry-run",
            ],
            vec![
                "sealtask",
                "tasks",
                "attachments",
                "upload",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--task-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
                "--file",
                "artifact.bin",
            ],
        ] {
            let cli = Cli::try_parse_from(args).expect("non-generic arguments");
            assert!(!needs_mutation_supervision(cli.command.as_ref()));
        }
    }

    #[tokio::test]
    async fn signal_before_transport_boundary_cancels_without_ambiguity() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let operation = pending_operation(dropped.clone());
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || false,
            || false,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        let error = supervised.await.expect_err("ordinary interruption");
        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(!error.to_string().contains("may have committed"));
        assert!(dropped.load(Ordering::Acquire));
        assert!(cancellation.is_cancelled());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn signal_delivered_while_ready_operation_is_polled_wins_completion_race() {
        let (_sender, receiver) = test_signal_receiver();
        let delivered = receiver.delivered.clone();
        let cancellation = ApiCancellationToken::new();
        let operation = std::future::poll_fn(move |_| {
            increment_signal_generation(&delivered);
            std::task::Poll::Ready(Ok(()))
        });

        let error = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || false,
            || false,
        )
        .await
        .expect_err("the synchronously delivered signal must win");

        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn signal_during_credential_rotation_waits_then_cancels_before_mutation() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let cancellation_for_operation = cancellation.clone();
        let operation = async move {
            while !cancellation_for_operation.is_cancelled() {
                tokio::task::yield_now().await;
            }
            Err(PublicError::cancelled("cancelled at the safe boundary").into())
        };
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || true,
            || false,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        let error = supervised
            .await
            .expect_err("credential refresh interruption");
        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(error.to_string().contains("durably persisted"));
        assert!(error.to_string().contains("was not sent"));
        assert!(cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn second_signal_during_credential_rotation_reports_ambiguous_session_state() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let operation = pending_operation(dropped.clone());
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || true,
            || false,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        tokio::task::yield_now().await;
        assert!(sender.send(SignalState::Received(2)).is_ok());
        let error = supervised
            .await
            .expect_err("forced credential interruption");
        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(error.to_string().contains("rotated the session"));
        assert!(error.to_string().contains("auth login"));
        assert!(cancellation.is_cancelled());
        assert!(dropped.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn second_signal_after_refresh_guard_drops_without_a_result_stays_session_ambiguous() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let refresh_active = Arc::new(AtomicBool::new(true));
        let refresh_active_for_supervisor = refresh_active.clone();
        let dropped = Arc::new(AtomicBool::new(false));
        let operation = pending_operation(dropped.clone());
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            move || refresh_active_for_supervisor.load(Ordering::Acquire),
            || false,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        assert!(
            tokio::time::timeout(Duration::from_millis(1), &mut supervised)
                .await
                .is_err(),
            "the first signal must enter deferred credential supervision"
        );
        refresh_active.store(false, Ordering::Release);
        assert!(sender.send(SignalState::Received(2)).is_ok());

        let error = supervised
            .await
            .expect_err("a dropped guard is not proof that rotated credentials persisted");
        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(error.to_string().contains("rotated the session"));
        assert!(error.to_string().contains("auth login"));
        assert!(dropped.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn refresh_transport_failure_after_signal_reports_ambiguous_session_state() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let cancellation_for_operation = cancellation.clone();
        let operation = async move {
            while !cancellation_for_operation.is_cancelled() {
                tokio::task::yield_now().await;
            }
            Err(PublicError::transport(TransportFailureKind::Body).into())
        };
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || true,
            || false,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        let error = supervised
            .await
            .expect_err("lost refresh response may follow remote rotation");
        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(error.to_string().contains("rotated the session"));
        assert!(error.to_string().contains("auth login"));
    }

    #[test]
    fn only_post_delivery_refresh_failures_are_session_ambiguous() {
        assert!(!credential_refresh_failure_may_have_rotated(
            &PublicError::transport(TransportFailureKind::Connect)
        ));
        assert!(credential_refresh_failure_may_have_rotated(
            &PublicError::transport(TransportFailureKind::Timeout)
        ));
        assert!(credential_refresh_failure_may_have_rotated(
            &PublicError::request_timeout("refresh timed out after delivery")
        ));
        assert!(credential_refresh_failure_may_have_rotated(
            &PublicError::response(
                sealtask_client_core::ResponseFailureKind::JsonMalformed,
                "malformed refresh response",
            )
        ));
        assert!(credential_refresh_failure_may_have_rotated(
            &PublicError::unexpected("credential persistence failed")
        ));
    }

    #[tokio::test]
    async fn first_signal_waits_for_a_definitive_result() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let (release, operation) = oneshot::channel();
        let supervised = supervise_mutation_with_signals(
            async {
                operation.await.expect("release operation");
                Ok(())
            },
            receiver,
            Duration::from_secs(1),
            OutputFormat::Json,
            &cancellation,
            || false,
            || true,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        tokio::task::yield_now().await;
        assert!(release.send(()).is_ok());
        assert!(supervised.await.is_ok());
        assert!(!cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn second_signal_while_request_is_active_marks_the_outcome_ambiguous() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let operation = pending_operation(dropped.clone());
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || false,
            || true,
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        tokio::task::yield_now().await;
        assert!(sender.send(SignalState::Received(2)).is_ok());
        let error = supervised.await.expect_err("forced interruption");
        assert_eq!(error.code(), "interrupted");
        assert_eq!(error.exit_code(), 130);
        assert!(error.to_string().contains("may have committed"));
        assert!(dropped.load(Ordering::Acquire));
        assert!(!cancellation.is_cancelled());
    }

    #[tokio::test]
    async fn second_signal_after_request_finishes_is_not_ambiguous() {
        let (sender, receiver) = test_signal_receiver();
        let cancellation = ApiCancellationToken::new();
        let active = Arc::new(AtomicBool::new(true));
        let active_for_supervisor = active.clone();
        let dropped = Arc::new(AtomicBool::new(false));
        let operation = pending_operation(dropped.clone());
        let supervised = supervise_mutation_with_signals(
            operation,
            receiver,
            Duration::from_secs(60),
            OutputFormat::Json,
            &cancellation,
            || false,
            move || active_for_supervisor.load(Ordering::Acquire),
        );
        tokio::pin!(supervised);

        assert!(sender.send(SignalState::Received(1)).is_ok());
        assert!(
            tokio::time::timeout(Duration::from_millis(1), &mut supervised)
                .await
                .is_err(),
            "the first signal must enter deferred mutation supervision"
        );
        active.store(false, Ordering::Release);
        assert!(sender.send(SignalState::Received(2)).is_ok());
        let error = supervised.await.expect_err("definitive interruption");
        assert_eq!(error.code(), "interrupted");
        assert!(!error.to_string().contains("may have committed"));
        assert!(dropped.load(Ordering::Acquire));
        assert!(!cancellation.is_cancelled());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn receiver_observes_each_unpublished_unix_signal_generation() {
        let (_sender, mut receiver) = test_signal_receiver();
        let delivered = receiver.delivered.clone();

        increment_signal_generation(&delivered);
        assert_eq!(receiver.level().expect("first signal level"), 1);
        assert_eq!(
            tokio::time::timeout(Duration::from_millis(10), receiver.changed())
                .await
                .expect("unpublished first signal must be observed synchronously")
                .expect("first signal"),
            1
        );

        increment_signal_generation(&delivered);
        assert_eq!(
            tokio::time::timeout(Duration::from_millis(10), receiver.changed())
                .await
                .expect("unpublished second signal must be observed synchronously")
                .expect("second signal"),
            2
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(10), receiver.clone().wait_for(2))
                .await
                .expect("second signal wait must not require async publication")
                .is_ok()
        );
    }

    fn test_signal_receiver() -> (watch::Sender<SignalState>, SignalReceiver) {
        let (sender, receiver) = watch::channel(SignalState::Listening);
        (
            sender,
            SignalReceiver {
                receiver,
                observed_level: 0,
                #[cfg(unix)]
                delivered: Arc::new(AtomicUsize::new(0)),
            },
        )
    }

    fn pending_operation(dropped: Arc<AtomicBool>) -> impl Future<Output = CliResult<()>> {
        let guard = DropFlag(dropped);
        async move {
            let _guard = guard;
            std::future::pending::<()>().await;
            Ok(())
        }
    }

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
}
