mod checkpoint;
mod executor;
mod input;

use self::checkpoint::{
    CheckpointStore, OperationKind, ResumeState, StartedMetadata, reject_input_checkpoint_conflict,
};
use self::executor::{Mutation, MutationInput, ResumeDecision, ensure_resume_plan_matches};
use self::input::{
    BatchDocument, BatchOperation, BatchOperationKind, operation_key, read_batch_input,
};
use crate::args::{BatchCommand, BatchRunArgs};
use crate::interruption::{
    MUTATION_INTERRUPT_GRACE, SignalMonitor, SignalReceiver,
    credential_refresh_failure_may_have_rotated,
};
use crate::output::{CliError, CliResult, OutputFormat, print_jsonl, write_stdout_line_flushed};
use crate::resolver::{ProjectLifecycle, TaskLifecycle, resolve_project, resolve_task};
use chrono::{DateTime, Utc};
use sealtask_client_api::ApiCancellationToken;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{AgentTaskSummary, RuntimeClient, TaskMutationPlan};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};
use tokio::sync::oneshot;
use tokio::task::{Id as TaskId, JoinSet};
use tokio::time::{Duration, timeout};
use uuid::Uuid;
use zeroize::Zeroize;

const BATCH_OUTPUT_SCHEMA_VERSION: u8 = 1;
const FORCED_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const SESSION_OUTCOME_AMBIGUOUS: &str = "session_outcome_ambiguous";

pub(crate) async fn run_batch(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: BatchCommand,
) -> CliResult<()> {
    let BatchCommand::Run(args) = command;
    run_batch_file(runtime, format, args).await
}

async fn run_batch_file(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: BatchRunArgs,
) -> CliResult<()> {
    if matches!(format, OutputFormat::Json | OutputFormat::JsonPretty) {
        return Err(PublicError::validation(
            "batch execution is a record stream; use table output or '--format jsonl'",
        )
        .into());
    }
    if args.resume && args.checkpoint.is_none() {
        return Err(CliError::checkpoint_conflict(
            "--resume requires --checkpoint PATH",
        ));
    }
    reject_input_checkpoint_conflict(&args.input, args.checkpoint.as_deref())?;
    let signal_monitor = SignalMonitor::start()?;
    let mut signal = signal_monitor.subscribe();
    tokio::task::yield_now().await;
    let document = read_batch_input_interruptibly(&args.input, &mut signal).await?;
    let total = document.operations.len();
    if signal.level().map_err(signal_listener_error)? > 0 {
        emit_summary(
            format,
            &BatchSummary {
                total,
                succeeded: 0,
                failed: 0,
                skipped: 0,
                planned: 0,
                not_run: total,
                interrupted_count: 0,
                interrupted: true,
            },
        )?;
        return Err(CliError::interrupted(
            "batch interrupted before checkpoint or mutation processing",
            &[],
        ));
    }

    let checkpoint_result = match args.checkpoint.as_ref() {
        Some(path) => {
            let path = path.clone();
            let input_sha256 = document.input_sha256.clone();
            Some(
                tokio::task::spawn_blocking(move || {
                    CheckpointStore::open(&path, &input_sha256, args.resume)
                })
                .await
                .map_err(|_| CliError::checkpoint_io("checkpoint acquisition task failed"))??,
            )
        }
        None => None,
    };
    if signal.level().map_err(signal_listener_error)? > 0 {
        emit_summary(
            format,
            &BatchSummary {
                total,
                succeeded: 0,
                failed: 0,
                skipped: 0,
                planned: 0,
                not_run: total,
                interrupted_count: 0,
                interrupted: true,
            },
        )?;
        return Err(CliError::interrupted(
            "batch interrupted during checkpoint acquisition",
            &[],
        ));
    }
    let checkpoint = checkpoint_result;
    if let Some(checkpoint) = checkpoint.as_ref() {
        let canonical_keys = document
            .operations
            .iter()
            .map(|operation| operation_key(&operation.operation_id))
            .collect::<HashSet<_>>();
        checkpoint.validate_operation_keys(&canonical_keys)?;
    }
    let checkpoint = checkpoint.map(Arc::new);
    let cancellation = runtime
        .api_cancellation_token()
        .ok_or_else(|| PublicError::unexpected("batch API cancellation state is not configured"))?;
    let resolved = resolve_operations(
        runtime,
        document,
        checkpoint.as_deref(),
        args.continue_on_error,
        &mut signal,
        &cancellation,
    )
    .await;
    let mut pending = match resolved {
        Ok(resolved) => resolved,
        Err(error @ CliError::Interrupted { .. }) => {
            emit_summary(
                format,
                &BatchSummary {
                    total,
                    succeeded: 0,
                    failed: 0,
                    skipped: 0,
                    planned: 0,
                    not_run: total,
                    interrupted_count: 0,
                    interrupted: true,
                },
            )?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };

    schedule_operations(
        runtime,
        format,
        &mut pending,
        total,
        usize::from(args.jobs),
        args.continue_on_error,
        args.dry_run,
        checkpoint,
        &mut signal,
    )
    .await
}

async fn read_batch_input_interruptibly(
    path: &Path,
    signal: &mut SignalReceiver,
) -> CliResult<BatchDocument> {
    if path != Path::new("-") {
        return read_batch_input(path);
    }
    if signal.level().map_err(signal_listener_error)? > 0 {
        return Err(CliError::interrupted(
            "batch interrupted while waiting for stdin input",
            &[],
        ));
    }

    let (sender, receiver) = oneshot::channel();
    std::thread::Builder::new()
        .name("sealtask-batch-stdin".to_string())
        .spawn(move || {
            let result = read_batch_input(Path::new("-"));
            let _ = sender.send(result);
        })
        .map_err(|_| PublicError::unexpected("failed to start the batch stdin reader"))?;

    tokio::select! {
        biased;
        changed = signal.changed() => match changed {
            Ok(_) => Err(CliError::interrupted(
                "batch interrupted while waiting for stdin input",
                &[],
            )),
            Err(error) => Err(signal_listener_error(error)),
        },
        result = receiver => result
            .map_err(|_| PublicError::unexpected("the batch stdin reader stopped unexpectedly"))?,
    }
}

fn signal_listener_error(error: std::io::Error) -> CliError {
    PublicError::unexpected(format!(
        "batch process interruption listener failed: {error}"
    ))
    .into()
}

enum ResolvedItem {
    Immediate(OperationOutcome),
    Mutation(ResolvedOperation),
}

struct ResolvedOperation {
    index: usize,
    operation_id: String,
    operation_key: String,
    kind: OperationKind,
    mutation: Mutation,
    resume: Option<StartedMetadata>,
}

#[derive(Default)]
struct ResolutionCaches {
    projects: HashMap<String, Uuid>,
    tasks: HashMap<Uuid, HashMap<String, Uuid>>,
}

impl Drop for ResolutionCaches {
    fn drop(&mut self) {
        for (mut selector, _) in self.projects.drain() {
            selector.zeroize();
        }
        for (_, mut tasks) in self.tasks.drain() {
            for (mut selector, _) in tasks.drain() {
                selector.zeroize();
            }
        }
    }
}

impl ResolvedOperation {
    fn target(&self) -> Option<(Uuid, Uuid)> {
        self.mutation
            .task_id()
            .map(|task_id| (self.mutation.project_id, task_id))
    }
}

async fn resolve_operations(
    runtime: &RuntimeClient,
    document: BatchDocument,
    checkpoint: Option<&CheckpointStore>,
    continue_on_error: bool,
    signal: &mut SignalReceiver,
    cancellation: &ApiCancellationToken,
) -> CliResult<VecDeque<ResolvedItem>> {
    let mut resolved = VecDeque::with_capacity(document.operations.len());
    let mut caches = ResolutionCaches::default();

    for operation in document.operations {
        if signal.level().map_err(signal_listener_error)? > 0 {
            return Err(CliError::interrupted(
                "batch interrupted during target resolution",
                &[],
            ));
        }
        let expected_kind = operation_kind(&operation.kind);
        let checkpoint_state = match checkpoint {
            Some(checkpoint) => checkpoint.resume_state(&operation_key(&operation.operation_id))?,
            None => ResumeState::Absent,
        };

        let item = match checkpoint_state {
            ResumeState::Succeeded {
                kind,
                project_id,
                task_id,
                updated_at,
            } => {
                ensure_checkpoint_kind(expected_kind, kind)?;
                ResolvedItem::Immediate(OperationOutcome::skipped(
                    &operation, project_id, task_id, updated_at,
                ))
            }
            ResumeState::Started(metadata) => {
                ensure_checkpoint_kind(expected_kind, metadata.kind)?;
                resolved_from_metadata(operation, metadata, true)?
            }
            ResumeState::Failed(metadata) => {
                ensure_checkpoint_kind(expected_kind, metadata.kind)?;
                resolved_from_metadata(operation, metadata, false)?
            }
            ResumeState::Absent => {
                resolve_new_operation(runtime, operation, &mut caches, signal, cancellation).await?
            }
        };

        let failed = matches!(
            &item,
            ResolvedItem::Immediate(OperationOutcome {
                status: OutcomeStatus::Failed,
                ..
            })
        );
        resolved.push_back(item);
        if failed && !continue_on_error {
            break;
        }
    }
    Ok(resolved)
}

fn resolved_from_metadata(
    operation: BatchOperation,
    metadata: StartedMetadata,
    resume_started: bool,
) -> CliResult<ResolvedItem> {
    let input = match &operation.kind {
        BatchOperationKind::TaskCreate {
            input,
            idempotency_derivation,
        } => {
            if metadata.task_id.is_some() {
                return Err(CliError::checkpoint_conflict(
                    "checkpoint task.create unexpectedly contains a task target",
                ));
            }
            MutationInput::TaskCreate {
                input: input.clone(),
                idempotency_derivation: idempotency_derivation.clone(),
            }
        }
        BatchOperationKind::TaskUpdate { input, .. } => {
            let task_id = metadata.task_id.ok_or_else(|| {
                CliError::checkpoint_conflict(
                    "checkpoint task.update is missing its canonical task target",
                )
            })?;
            MutationInput::TaskUpdate {
                task_id,
                input: input.clone(),
            }
        }
    };
    Ok(ResolvedItem::Mutation(ResolvedOperation {
        index: operation.index,
        operation_key: operation_key(&operation.operation_id),
        operation_id: operation.operation_id,
        kind: metadata.kind,
        mutation: Mutation {
            project_id: metadata.project_id,
            input,
        },
        resume: resume_started.then_some(metadata),
    }))
}

async fn resolve_new_operation(
    runtime: &RuntimeClient,
    operation: BatchOperation,
    caches: &mut ResolutionCaches,
    signal: &mut SignalReceiver,
    cancellation: &ApiCancellationToken,
) -> CliResult<ResolvedItem> {
    let project_selector = operation.project.as_str();
    let project_id = if let Some(project_id) = caches.projects.get(project_selector) {
        Ok(*project_id)
    } else {
        match await_read_or_signal(
            signal,
            cancellation,
            resolve_project(
                runtime,
                Some(&operation.project),
                None,
                false,
                ProjectLifecycle::Active,
            ),
        )
        .await?
        {
            Some(Ok(project)) => {
                caches
                    .projects
                    .insert(project_selector.to_string(), project.id);
                Ok(project.id)
            }
            Some(Err(error)) => Err(error),
            None => {
                return Ok(ResolvedItem::Immediate(
                    OperationOutcome::interrupted_resolution(&operation),
                ));
            }
        }
    };
    let project_id = match project_id {
        Ok(project_id) => project_id,
        Err(error) => {
            return Ok(ResolvedItem::Immediate(OperationOutcome::failed_runtime(
                &operation, None, None, error,
            )));
        }
    };

    let (kind, input) = match &operation.kind {
        BatchOperationKind::TaskCreate {
            input,
            idempotency_derivation,
        } => (
            OperationKind::TaskCreate,
            MutationInput::TaskCreate {
                input: input.clone(),
                idempotency_derivation: idempotency_derivation.clone(),
            },
        ),
        BatchOperationKind::TaskUpdate { task, input } => {
            let task_selector = task.as_str();
            let task_id = if let Some(task_id) = caches
                .tasks
                .get(&project_id)
                .and_then(|tasks| tasks.get(task_selector))
            {
                Ok(*task_id)
            } else {
                match await_read_or_signal(
                    signal,
                    cancellation,
                    resolve_task(
                        runtime,
                        project_id,
                        Some(task),
                        None,
                        false,
                        TaskLifecycle::Any,
                    ),
                )
                .await?
                {
                    Some(Ok(task)) => {
                        caches
                            .tasks
                            .entry(project_id)
                            .or_default()
                            .insert(task_selector.to_string(), task.id);
                        Ok(task.id)
                    }
                    Some(Err(error)) => Err(error),
                    None => {
                        return Ok(ResolvedItem::Immediate(
                            OperationOutcome::interrupted_resolution_with_project(
                                &operation, project_id,
                            ),
                        ));
                    }
                }
            };
            let task_id = match task_id {
                Ok(task_id) => task_id,
                Err(error) => {
                    return Ok(ResolvedItem::Immediate(OperationOutcome::failed_runtime(
                        &operation,
                        Some(project_id),
                        None,
                        error,
                    )));
                }
            };
            (
                OperationKind::TaskUpdate,
                MutationInput::TaskUpdate {
                    task_id,
                    input: input.clone(),
                },
            )
        }
    };

    Ok(ResolvedItem::Mutation(ResolvedOperation {
        index: operation.index,
        operation_key: operation_key(&operation.operation_id),
        operation_id: operation.operation_id,
        kind,
        mutation: Mutation { project_id, input },
        resume: None,
    }))
}

async fn await_read_or_signal<T>(
    signal: &mut SignalReceiver,
    cancellation: &ApiCancellationToken,
    future: impl Future<Output = PublicResult<T>>,
) -> CliResult<Option<PublicResult<T>>> {
    tokio::pin!(future);
    tokio::select! {
        biased;
        changed = signal.changed() => {
            changed.map_err(signal_listener_error)?;
            if cancellation.credential_refresh_may_have_rotated() {
                cancellation.cancel();
                return Err(batch_session_ambiguous_interruption(
                    "batch credential refresh response was lost after the server may have rotated the session",
                ));
            }
            if !cancellation.credential_refresh_in_flight() {
                cancellation.cancel();
                return Ok(None);
            }
            match await_interrupted_credential_refresh(
                signal.clone(),
                cancellation,
                future.as_mut(),
            )
            .await
            {
                CredentialRefreshWait::ReachedDurableBoundary => Ok(None),
                CredentialRefreshWait::Failed(error) => Err(error.into()),
                CredentialRefreshWait::SessionAmbiguous(message) => {
                    Err(batch_session_ambiguous_interruption(message))
                }
            }
        },
        result = &mut future => {
            match observe_coincident_credential_refresh_signal(signal, cancellation) {
                Ok(true) => {
                    cancellation.cancel();
                    Err(batch_session_ambiguous_interruption(
                        "batch credential refresh response was lost while an interruption was being delivered",
                    ))
                }
                Ok(false) => Ok(Some(result)),
                Err(error) => Err(signal_listener_error(error)),
            }
        },
    }
}

fn observe_coincident_credential_refresh_signal(
    signal: &SignalReceiver,
    cancellation: &ApiCancellationToken,
) -> std::io::Result<bool> {
    Ok(cancellation.credential_refresh_may_have_rotated() && signal.level()? > 0)
}

enum CredentialRefreshWait {
    ReachedDurableBoundary,
    Failed(PublicError),
    SessionAmbiguous(&'static str),
}

async fn await_interrupted_credential_refresh<T, F>(
    signal: SignalReceiver,
    cancellation: &ApiCancellationToken,
    mut future: std::pin::Pin<&mut F>,
) -> CredentialRefreshWait
where
    F: Future<Output = PublicResult<T>>,
{
    cancellation.cancel();
    let second_signal = signal.wait_for(2);
    tokio::pin!(second_signal);
    let deadline = tokio::time::sleep(MUTATION_INTERRUPT_GRACE);
    tokio::pin!(deadline);

    tokio::select! {
        biased;
        result = future.as_mut() => classify_interrupted_credential_refresh(result),
        signal_result = &mut second_signal => match signal_result {
            Ok(()) => CredentialRefreshWait::SessionAmbiguous(
                "batch credential refresh was force-stopped by a second signal before durable local persistence was confirmed",
            ),
            Err(_) => CredentialRefreshWait::SessionAmbiguous(
                "batch credential refresh signal supervision failed before durable local persistence was confirmed",
            ),
        },
        () = &mut deadline => CredentialRefreshWait::SessionAmbiguous(
            "batch credential refresh did not reach a confirmed durable boundary within 30 seconds",
        ),
    }
}

fn classify_interrupted_credential_refresh<T>(result: PublicResult<T>) -> CredentialRefreshWait {
    match result {
        Ok(_) => CredentialRefreshWait::ReachedDurableBoundary,
        Err(error) if error.code() == "cancelled" => CredentialRefreshWait::ReachedDurableBoundary,
        Err(error) if credential_refresh_failure_may_have_rotated(&error) => {
            CredentialRefreshWait::SessionAmbiguous(
                "batch credential refresh failed after the server may have rotated the session",
            )
        }
        Err(error) => CredentialRefreshWait::Failed(error),
    }
}

fn batch_session_ambiguous_interruption(message: &str) -> CliError {
    CliError::interrupted_session_ambiguous(
        format!(
            "{message}; the server may have rotated the session without a durable local replacement, so run 'sealtask auth login' if authentication no longer works"
        ),
        &[],
    )
}

fn ensure_checkpoint_kind(expected: OperationKind, stored: OperationKind) -> CliResult<()> {
    if expected != stored {
        return Err(CliError::checkpoint_conflict(
            "checkpoint operation kind does not match canonical batch input",
        ));
    }
    Ok(())
}

fn operation_kind(kind: &BatchOperationKind) -> OperationKind {
    match kind {
        BatchOperationKind::TaskCreate { .. } => OperationKind::TaskCreate,
        BatchOperationKind::TaskUpdate { .. } => OperationKind::TaskUpdate,
    }
}

#[derive(Clone)]
struct OperationProgress(Arc<AtomicU8>);

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
enum OperationPhase {
    Preparing = 0,
    MutationInFlight = 1,
    ResponseReceived = 2,
}

impl OperationProgress {
    fn new() -> Self {
        Self(Arc::new(AtomicU8::new(OperationPhase::Preparing as u8)))
    }

    fn mark_mutation_in_flight(&self) {
        self.0
            .store(OperationPhase::MutationInFlight as u8, Ordering::Release);
    }

    fn mark_response_received(&self) {
        self.0
            .store(OperationPhase::ResponseReceived as u8, Ordering::Release);
    }

    fn mutation_may_have_committed(&self) -> bool {
        self.0.load(Ordering::Acquire) != OperationPhase::Preparing as u8
    }
}

struct RunningOperation {
    target: Option<(Uuid, Uuid)>,
    index: usize,
    operation_id: String,
    kind: OperationKind,
    project_id: Uuid,
    task_id: Option<Uuid>,
    progress: OperationProgress,
    cancellation: ApiCancellationToken,
    credential_refresh_interrupted: Arc<AtomicBool>,
}

struct OperationSupervision {
    signal: SignalReceiver,
    cancellation: ApiCancellationToken,
    credential_refresh_interrupted: Arc<AtomicBool>,
    progress: OperationProgress,
}

impl RunningOperation {
    fn new(
        operation: &ResolvedOperation,
        target: Option<(Uuid, Uuid)>,
        progress: OperationProgress,
        cancellation: ApiCancellationToken,
        credential_refresh_interrupted: Arc<AtomicBool>,
    ) -> Self {
        Self {
            target,
            index: operation.index,
            operation_id: operation.operation_id.clone(),
            kind: operation.kind,
            project_id: operation.mutation.project_id,
            task_id: operation.mutation.task_id(),
            progress,
            cancellation,
            credential_refresh_interrupted,
        }
    }

    fn observe_signal(&self) {
        if self.cancellation.credential_refresh_in_flight()
            || self.cancellation.credential_refresh_may_have_rotated()
        {
            self.credential_refresh_interrupted
                .store(true, Ordering::Release);
            self.cancellation.cancel();
        }
    }

    fn finalize_outcome(self, outcome: OperationOutcome) -> OperationOutcome {
        if !self.credential_refresh_interrupted.load(Ordering::Acquire) {
            return outcome;
        }
        if self.cancellation.credential_refresh_in_flight()
            || self.cancellation.credential_refresh_may_have_rotated()
        {
            return OperationOutcome::session_ambiguous_interrupted_parts(
                self.index,
                self.operation_id,
                self.kind,
                Some(self.project_id),
                self.task_id,
            );
        }
        OperationOutcome::interrupted_parts(
            self.index,
            self.operation_id,
            self.kind,
            Some(self.project_id),
            self.task_id,
        )
    }

    fn forced_outcome(self) -> OperationOutcome {
        if self.credential_refresh_interrupted.load(Ordering::Acquire) {
            if self.cancellation.credential_refresh_in_flight()
                || self.cancellation.credential_refresh_may_have_rotated()
            {
                return OperationOutcome::session_ambiguous_interrupted_parts(
                    self.index,
                    self.operation_id,
                    self.kind,
                    Some(self.project_id),
                    self.task_id,
                );
            }
            return OperationOutcome::interrupted_parts(
                self.index,
                self.operation_id,
                self.kind,
                Some(self.project_id),
                self.task_id,
            );
        }
        if self.progress.mutation_may_have_committed() {
            OperationOutcome::ambiguous_interrupted_parts(
                self.index,
                self.operation_id,
                self.kind,
                Some(self.project_id),
                self.task_id,
            )
        } else {
            OperationOutcome::interrupted_parts(
                self.index,
                self.operation_id,
                self.kind,
                Some(self.project_id),
                self.task_id,
            )
        }
    }

    fn join_failure_outcome(self) -> OperationOutcome {
        if self.credential_refresh_interrupted.load(Ordering::Acquire) {
            return self.forced_outcome();
        }
        OperationOutcome::failed_runtime_parts(
            self.index,
            self.operation_id,
            self.kind,
            Some(self.project_id),
            self.task_id,
            PublicError::unexpected("batch operation worker did not complete"),
        )
    }
}

#[allow(clippy::too_many_arguments)]
async fn schedule_operations(
    runtime: &RuntimeClient,
    format: OutputFormat,
    pending: &mut VecDeque<ResolvedItem>,
    total: usize,
    jobs: usize,
    continue_on_error: bool,
    dry_run: bool,
    checkpoint: Option<Arc<CheckpointStore>>,
    signal: &mut SignalReceiver,
) -> CliResult<()> {
    let mut running = JoinSet::new();
    let mut running_operations = HashMap::<TaskId, RunningOperation>::new();
    let mut active_targets = HashSet::new();
    let mut counters = OutcomeCounters::default();
    let mut stop_scheduling = false;
    let mut interrupted = signal.level().map_err(signal_listener_error)? > 0;
    let mut forced = false;
    let mut ambiguous_interruption = false;
    let mut session_ambiguous_interruption = false;
    let mut first_output_error = None;
    let mut signal_error = None;
    let mut fatal_exit_code = None;
    let mut checkpoint_io_failed = false;

    loop {
        while !stop_scheduling && !interrupted && running.len() < jobs {
            if signal.level().map_err(signal_listener_error)? > 0 {
                interrupted = true;
                stop_scheduling = true;
                break;
            }
            let Some(index) = next_schedulable(pending, &active_targets) else {
                break;
            };
            let item = pending
                .remove(index)
                .expect("next_schedulable returned an existing item");
            match item {
                ResolvedItem::Immediate(outcome) => {
                    stop_scheduling |= record_scheduled_outcome(
                        format,
                        None,
                        outcome,
                        continue_on_error,
                        &mut active_targets,
                        &mut counters,
                        &mut interrupted,
                        &mut ambiguous_interruption,
                        &mut session_ambiguous_interruption,
                        &mut first_output_error,
                        &mut fatal_exit_code,
                        &mut checkpoint_io_failed,
                    );
                }
                ResolvedItem::Mutation(operation) => {
                    let target = operation.target();
                    if let Some(target) = target {
                        active_targets.insert(target);
                    }
                    let (runtime, operation_cancellation) =
                        runtime_for_scheduled_operation(runtime);
                    let checkpoint = checkpoint.clone();
                    let operation_signal = signal.clone();
                    let progress = OperationProgress::new();
                    let credential_refresh_interrupted = Arc::new(AtomicBool::new(false));
                    let tracked = RunningOperation::new(
                        &operation,
                        target,
                        progress.clone(),
                        operation_cancellation.clone(),
                        credential_refresh_interrupted.clone(),
                    );
                    let abort_handle = running.spawn(async move {
                        let supervision = OperationSupervision {
                            signal: operation_signal,
                            cancellation: operation_cancellation,
                            credential_refresh_interrupted,
                            progress,
                        };
                        run_operation(&runtime, operation, dry_run, checkpoint, supervision).await
                    });
                    running_operations.insert(abort_handle.id(), tracked);
                }
            }
        }

        if running.is_empty() {
            break;
        }

        tokio::select! {
            biased;
            changed = signal.changed() => {
                match changed {
                    Ok(level) => {
                        interrupted = true;
                        stop_scheduling = true;
                        for operation in running_operations.values() {
                            operation.observe_signal();
                        }
                        if level >= 2 {
                            forced = true;
                            running.abort_all();
                            let _ = timeout(FORCED_CLEANUP_TIMEOUT, async {
                                while let Some(joined) = running.join_next_with_id().await {
                                    if let Ok((task_id, outcome)) = joined {
                                        let (target, outcome) = finalize_joined_outcome(
                                            &mut running_operations,
                                            task_id,
                                            outcome,
                                        );
                                        record_scheduled_outcome(
                                            format,
                                            target,
                                            outcome,
                                            continue_on_error,
                                            &mut active_targets,
                                            &mut counters,
                                            &mut interrupted,
                                            &mut ambiguous_interruption,
                                            &mut session_ambiguous_interruption,
                                            &mut first_output_error,
                                            &mut fatal_exit_code,
                                            &mut checkpoint_io_failed,
                                        );
                                    }
                                }
                            }).await;

                            let mut cancelled = running_operations
                                .drain()
                                .map(|(_, operation)| operation)
                                .collect::<Vec<_>>();
                            cancelled.sort_by_key(|operation| operation.index);
                            for operation in cancelled {
                                let target = operation.target;
                                let outcome = operation.forced_outcome();
                                record_scheduled_outcome(
                                    format,
                                    target,
                                    outcome,
                                    continue_on_error,
                                    &mut active_targets,
                                    &mut counters,
                                    &mut interrupted,
                                    &mut ambiguous_interruption,
                                    &mut session_ambiguous_interruption,
                                    &mut first_output_error,
                                    &mut fatal_exit_code,
                                    &mut checkpoint_io_failed,
                                );
                            }
                            break;
                        }
                    }
                    Err(error) => {
                        stop_scheduling = true;
                        signal_error.get_or_insert_with(|| signal_listener_error(error));
                    }
                }
            }
            joined = running.join_next_with_id() => {
                let Some(joined) = joined else {
                    continue;
                };
                match joined {
                    Ok((task_id, outcome)) => {
                        let (target, outcome) = finalize_joined_outcome(
                            &mut running_operations,
                            task_id,
                            outcome,
                        );
                        stop_scheduling |= record_scheduled_outcome(
                            format,
                            target,
                            outcome,
                            continue_on_error,
                            &mut active_targets,
                            &mut counters,
                            &mut interrupted,
                            &mut ambiguous_interruption,
                            &mut session_ambiguous_interruption,
                            &mut first_output_error,
                            &mut fatal_exit_code,
                            &mut checkpoint_io_failed,
                        );
                    }
                    Err(error) => {
                        if let Some((target, outcome)) =
                            finalize_join_error(&mut running_operations, error.id())
                        {
                            stop_scheduling |= record_scheduled_outcome(
                                format,
                                target,
                                outcome,
                                continue_on_error,
                                &mut active_targets,
                                &mut counters,
                                &mut interrupted,
                                &mut ambiguous_interruption,
                                &mut session_ambiguous_interruption,
                                &mut first_output_error,
                                &mut fatal_exit_code,
                                &mut checkpoint_io_failed,
                            );
                        } else {
                            counters.failed += 1;
                            stop_scheduling = true;
                            fatal_exit_code.get_or_insert(1);
                        }
                    }
                }
            }
        }
    }

    if !forced {
        while let Some(joined) = running.join_next_with_id().await {
            match joined {
                Ok((task_id, outcome)) => {
                    let (target, outcome) =
                        finalize_joined_outcome(&mut running_operations, task_id, outcome);
                    record_scheduled_outcome(
                        format,
                        target,
                        outcome,
                        continue_on_error,
                        &mut active_targets,
                        &mut counters,
                        &mut interrupted,
                        &mut ambiguous_interruption,
                        &mut session_ambiguous_interruption,
                        &mut first_output_error,
                        &mut fatal_exit_code,
                        &mut checkpoint_io_failed,
                    );
                }
                Err(error) => {
                    if let Some((target, outcome)) =
                        finalize_join_error(&mut running_operations, error.id())
                    {
                        record_scheduled_outcome(
                            format,
                            target,
                            outcome,
                            continue_on_error,
                            &mut active_targets,
                            &mut counters,
                            &mut interrupted,
                            &mut ambiguous_interruption,
                            &mut session_ambiguous_interruption,
                            &mut first_output_error,
                            &mut fatal_exit_code,
                            &mut checkpoint_io_failed,
                        );
                    } else {
                        counters.failed += 1;
                        fatal_exit_code.get_or_insert(1);
                    }
                }
            }
        }
    }

    if let Some(error) = first_output_error {
        return Err(error);
    }
    let summary = BatchSummary {
        total,
        succeeded: counters.succeeded,
        failed: counters.failed,
        skipped: counters.skipped,
        planned: counters.planned,
        not_run: total.saturating_sub(counters.processed()),
        interrupted_count: counters.interrupted,
        interrupted,
    };
    emit_summary(format, &summary)?;

    if let Some(error) = signal_error {
        return Err(error);
    }
    if interrupted {
        let message = if session_ambiguous_interruption {
            "batch interruption stopped waiting while credentials were rotating before durable local persistence was confirmed"
        } else if forced {
            forced_interruption_message(checkpoint.is_some(), ambiguous_interruption)
        } else {
            "batch interrupted; in-flight operations reached a durable boundary and can be resumed"
        };
        return Err(if session_ambiguous_interruption {
            batch_session_ambiguous_interruption(message)
        } else if ambiguous_interruption {
            CliError::interrupted_ambiguous(message, &[])
        } else {
            CliError::interrupted(message, &[])
        });
    }
    if fatal_exit_code == Some(4) {
        return Err(if checkpoint_io_failed {
            CliError::checkpoint_io(
                "batch stopped because checkpoint durability could not be guaranteed",
            )
        } else {
            CliError::checkpoint_conflict(
                "batch stopped because checkpoint state could not be safely continued",
            )
        });
    }
    if counters.failed > 0 {
        let completed = counters.succeeded + counters.skipped + counters.planned;
        if continue_on_error && completed > 0 {
            return Err(CliError::batch_partial_failure(format!(
                "{} batch operation(s) failed and {} completed",
                counters.failed, completed
            )));
        }
        return Err(PublicError::validation(format!(
            "{} batch operation(s) failed",
            counters.failed
        ))
        .into());
    }
    Ok(())
}

fn finalize_joined_outcome(
    running_operations: &mut HashMap<TaskId, RunningOperation>,
    task_id: TaskId,
    outcome: OperationOutcome,
) -> (Option<(Uuid, Uuid)>, OperationOutcome) {
    let Some(operation) = running_operations.remove(&task_id) else {
        return (None, outcome);
    };
    let target = operation.target;
    (target, operation.finalize_outcome(outcome))
}

fn finalize_join_error(
    running_operations: &mut HashMap<TaskId, RunningOperation>,
    task_id: TaskId,
) -> Option<(Option<(Uuid, Uuid)>, OperationOutcome)> {
    let operation = running_operations.remove(&task_id)?;
    let target = operation.target;
    Some((target, operation.join_failure_outcome()))
}

#[allow(clippy::too_many_arguments)]
fn record_scheduled_outcome(
    format: OutputFormat,
    target: Option<(Uuid, Uuid)>,
    outcome: OperationOutcome,
    continue_on_error: bool,
    active_targets: &mut HashSet<(Uuid, Uuid)>,
    counters: &mut OutcomeCounters,
    interrupted: &mut bool,
    ambiguous_interruption: &mut bool,
    session_ambiguous_interruption: &mut bool,
    first_output_error: &mut Option<CliError>,
    fatal_exit_code: &mut Option<i32>,
    checkpoint_io_failed: &mut bool,
) -> bool {
    if let Some(target) = target {
        active_targets.remove(&target);
    }
    counters.record(&outcome);
    *fatal_exit_code = merge_fatal_exit_code(*fatal_exit_code, outcome.fatal_exit_code);
    *checkpoint_io_failed |= outcome
        .error
        .as_ref()
        .is_some_and(|error| error.code == "checkpoint_io");
    let outcome_error_code = outcome.error.as_ref().map(|error| error.code.as_str());
    *session_ambiguous_interruption |= outcome_error_code == Some(SESSION_OUTCOME_AMBIGUOUS);
    *ambiguous_interruption |= outcome_error_code
        .is_some_and(|code| matches!(code, "outcome_ambiguous" | SESSION_OUTCOME_AMBIGUOUS));
    let mut should_stop = outcome.fatal_exit_code.is_some();
    if outcome.status == OutcomeStatus::Interrupted {
        *interrupted = true;
        should_stop = true;
    }
    if let Err(error) = emit_outcome(format, &outcome) {
        first_output_error.get_or_insert(error);
        should_stop = true;
    }
    should_stop || (outcome.status == OutcomeStatus::Failed && !continue_on_error)
}

fn forced_interruption_message(
    checkpoint_exists: bool,
    ambiguous_interruption: bool,
) -> &'static str {
    match (ambiguous_interruption, checkpoint_exists) {
        (true, true) => {
            "batch force-stopped after a second signal; one or more sent mutations may have committed, so resume from the checkpoint before retrying"
        }
        (true, false) => {
            "batch force-stopped after a second signal; one or more sent mutations may have committed, so inspect the affected resources before retrying"
        }
        (false, true) => {
            "batch force-stopped after a second signal before the interrupted mutations were sent; resume from the checkpoint before retrying"
        }
        (false, false) => {
            "batch force-stopped after a second signal before the interrupted mutations were sent"
        }
    }
}

fn merge_fatal_exit_code(current: Option<i32>, next: Option<i32>) -> Option<i32> {
    match (current, next) {
        (Some(4), _) | (_, Some(4)) => Some(4),
        (Some(current), _) => Some(current),
        (None, next) => next,
    }
}

fn next_schedulable(
    pending: &VecDeque<ResolvedItem>,
    active_targets: &HashSet<(Uuid, Uuid)>,
) -> Option<usize> {
    pending.iter().position(|item| match item {
        ResolvedItem::Immediate(_) => true,
        ResolvedItem::Mutation(operation) => operation
            .target()
            .is_none_or(|target| !active_targets.contains(&target)),
    })
}

fn runtime_for_scheduled_operation(
    runtime: &RuntimeClient,
) -> (RuntimeClient, ApiCancellationToken) {
    let cancellation = ApiCancellationToken::new();
    (
        runtime
            .clone()
            .with_api_cancellation_token(cancellation.clone()),
        cancellation,
    )
}

async fn run_operation(
    runtime: &RuntimeClient,
    operation: ResolvedOperation,
    dry_run: bool,
    checkpoint: Option<Arc<CheckpointStore>>,
    supervision: OperationSupervision,
) -> OperationOutcome {
    let OperationSupervision {
        mut signal,
        cancellation,
        credential_refresh_interrupted,
        progress,
    } = supervision;
    let ResolvedOperation {
        index,
        operation_id,
        operation_key,
        kind,
        mutation,
        resume,
    } = operation;
    let project_id = mutation.project_id;
    let target_task_id = mutation.task_id();
    macro_rules! stop_if_signalled {
        () => {
            match signal.level() {
                Ok(0) => {}
                Ok(_) => {
                    return OperationOutcome::interrupted_parts(
                        index,
                        operation_id.clone(),
                        kind,
                        Some(project_id),
                        target_task_id,
                    );
                }
                Err(error) => {
                    return OperationOutcome::failed_cli(
                        index,
                        operation_id.clone(),
                        kind,
                        Some(project_id),
                        target_task_id,
                        signal_listener_error(error),
                    );
                }
            }
        };
    }
    stop_if_signalled!();

    if !dry_run && resume.is_none() {
        let metadata = StartedMetadata {
            kind,
            project_id,
            task_id: target_task_id,
            expected_updated_at: None,
            change_commitment: None,
        };
        if let Err(error) =
            checkpoint_started(checkpoint.as_deref(), operation_key.clone(), &metadata).await
        {
            return OperationOutcome::failed_cli(
                index,
                operation_id,
                kind,
                Some(project_id),
                target_task_id,
                error,
            );
        }
    }
    stop_if_signalled!();

    let original_update_revision = if kind == OperationKind::TaskUpdate {
        resume
            .as_ref()
            .and_then(|started| started.expected_updated_at)
    } else {
        None
    };
    let preparation = mutation.prepare(runtime, original_update_revision);
    tokio::pin!(preparation);
    let preparation = tokio::select! {
        biased;
        changed = signal.changed() => {
            match changed {
                Ok(_) if cancellation.credential_refresh_may_have_rotated() => {
                    credential_refresh_interrupted.store(true, Ordering::Release);
                    cancellation.cancel();
                    return OperationOutcome::session_ambiguous_interrupted_parts(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                    );
                }
                Ok(_) if cancellation.credential_refresh_in_flight() => {
                    credential_refresh_interrupted.store(true, Ordering::Release);
                    match await_interrupted_credential_refresh(
                        signal.clone(),
                        &cancellation,
                        preparation.as_mut(),
                    )
                    .await
                    {
                        CredentialRefreshWait::ReachedDurableBoundary => {
                            credential_refresh_interrupted.store(false, Ordering::Release);
                            None
                        }
                        CredentialRefreshWait::Failed(error) => {
                            credential_refresh_interrupted.store(false, Ordering::Release);
                            Some(Err(error))
                        }
                        CredentialRefreshWait::SessionAmbiguous(_) => {
                            return OperationOutcome::session_ambiguous_interrupted_parts(
                                index,
                                operation_id,
                                kind,
                                Some(project_id),
                                target_task_id,
                            );
                        }
                    }
                }
                Ok(_) => {
                    cancellation.cancel();
                    None
                }
                Err(error) => {
                    return OperationOutcome::failed_cli(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                        signal_listener_error(error),
                    );
                }
            }
        },
        result = &mut preparation => {
            match observe_coincident_credential_refresh_signal(&signal, &cancellation) {
                Ok(true) => {
                    credential_refresh_interrupted.store(true, Ordering::Release);
                    cancellation.cancel();
                    return OperationOutcome::session_ambiguous_interrupted_parts(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                    );
                }
                Ok(false) => Some(result),
                Err(error) => {
                    return OperationOutcome::failed_cli(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                        signal_listener_error(error),
                    );
                }
            }
        },
    };
    let prepared = match preparation {
        None => {
            return OperationOutcome::interrupted_parts(
                index,
                operation_id,
                kind,
                Some(project_id),
                target_task_id,
            );
        }
        Some(Ok(prepared)) => prepared,
        Some(Err(error)) => {
            if let Some(checkpoint_error) =
                resume_prepare_safety_error(kind, original_update_revision, &error)
            {
                return OperationOutcome::failed_cli(
                    index,
                    operation_id,
                    kind,
                    Some(project_id),
                    target_task_id,
                    checkpoint_error,
                );
            }
            if !dry_run
                && resume.is_none()
                && let Err(checkpoint_error) = checkpoint_failed(
                    checkpoint.as_deref(),
                    operation_key,
                    kind,
                    project_id,
                    target_task_id,
                )
                .await
            {
                return OperationOutcome::failed_cli(
                    index,
                    operation_id,
                    kind,
                    Some(project_id),
                    target_task_id,
                    checkpoint_error,
                );
            }
            return OperationOutcome::failed_runtime_parts(
                index,
                operation_id,
                kind,
                Some(project_id),
                target_task_id,
                error,
            );
        }
    };
    let plan = prepared.plan().clone();

    if dry_run {
        return OperationOutcome::planned(
            index,
            operation_id,
            kind,
            project_id,
            target_task_id,
            plan,
        );
    }
    stop_if_signalled!();

    let mut reconciled = false;
    if kind == OperationKind::TaskUpdate
        && let Some(started) = resume.as_ref()
        && let Some(expected) = started.expected_updated_at
    {
        match ensure_resume_plan_matches(expected, started.change_commitment.as_deref(), &plan) {
            Ok(ResumeDecision::AlreadyApplied) => reconciled = true,
            Ok(ResumeDecision::Execute) => {}
            Err(error) => {
                return OperationOutcome::failed_cli(
                    index,
                    operation_id,
                    kind,
                    Some(project_id),
                    target_task_id,
                    error,
                );
            }
        }
    }
    if kind == OperationKind::TaskCreate
        && let Some(stored_commitment) = resume
            .as_ref()
            .and_then(|started| started.change_commitment.as_deref())
        && stored_commitment != plan.change_commitment
    {
        return OperationOutcome::failed_cli(
            index,
            operation_id,
            kind,
            Some(project_id),
            target_task_id,
            CliError::checkpoint_conflict(
                "checkpointed task.create plan no longer matches canonical input",
            ),
        );
    }

    let enriched = StartedMetadata {
        kind,
        project_id,
        task_id: target_task_id,
        expected_updated_at: plan.expected_updated_at,
        change_commitment: Some(plan.change_commitment.clone()),
    };
    let already_checkpointed = resume.as_ref().is_some_and(|started| {
        started.expected_updated_at == enriched.expected_updated_at
            && started.change_commitment == enriched.change_commitment
    });
    if !already_checkpointed
        && let Err(error) =
            checkpoint_started(checkpoint.as_deref(), operation_key.clone(), &enriched).await
    {
        return OperationOutcome::failed_cli(
            index,
            operation_id,
            kind,
            Some(project_id),
            target_task_id,
            error,
        );
    }
    stop_if_signalled!();

    if plan.would_change {
        progress.mark_mutation_in_flight();
    }
    let execution = prepared.execute(runtime);
    tokio::pin!(execution);
    let execution = tokio::select! {
        biased;
        changed = signal.changed() => {
            match changed {
                Ok(_) if cancellation.credential_refresh_may_have_rotated() => {
                    credential_refresh_interrupted.store(true, Ordering::Release);
                    cancellation.cancel();
                    return OperationOutcome::session_ambiguous_interrupted_parts(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                    );
                }
                Ok(_) if cancellation.credential_refresh_in_flight() => {
                    credential_refresh_interrupted.store(true, Ordering::Release);
                    match await_interrupted_credential_refresh(
                        signal.clone(),
                        &cancellation,
                        execution.as_mut(),
                    )
                    .await
                    {
                        CredentialRefreshWait::ReachedDurableBoundary => {
                            credential_refresh_interrupted.store(false, Ordering::Release);
                            return OperationOutcome::interrupted_parts(
                                index,
                                operation_id,
                                kind,
                                Some(project_id),
                                target_task_id,
                            );
                        }
                        CredentialRefreshWait::Failed(error) => {
                            credential_refresh_interrupted.store(false, Ordering::Release);
                            Err(error)
                        }
                        CredentialRefreshWait::SessionAmbiguous(_) => {
                            return OperationOutcome::session_ambiguous_interrupted_parts(
                                index,
                                operation_id,
                                kind,
                                Some(project_id),
                                target_task_id,
                            );
                        }
                    }
                }
                Ok(_) if cancellation.mutation_request_in_flight() => execution.await,
                Ok(_) => {
                    cancellation.cancel();
                    return OperationOutcome::interrupted_parts(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                    );
                }
                Err(error) => {
                    return OperationOutcome::failed_cli(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                        signal_listener_error(error),
                    );
                }
            }
        },
        result = &mut execution => {
            match observe_coincident_credential_refresh_signal(&signal, &cancellation) {
                Ok(true) => {
                    credential_refresh_interrupted.store(true, Ordering::Release);
                    cancellation.cancel();
                    return OperationOutcome::session_ambiguous_interrupted_parts(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                    );
                }
                Ok(false) => result,
                Err(error) => {
                    return OperationOutcome::failed_cli(
                        index,
                        operation_id,
                        kind,
                        Some(project_id),
                        target_task_id,
                        signal_listener_error(error),
                    );
                }
            }
        },
    };
    progress.mark_response_received();
    match execution {
        Ok(task) => {
            if let Err(error) = checkpoint_succeeded(
                checkpoint.as_deref(),
                operation_key,
                kind,
                project_id,
                &task,
            )
            .await
            {
                return OperationOutcome::failed_cli(
                    index,
                    operation_id,
                    kind,
                    Some(project_id),
                    Some(task.id),
                    error,
                );
            }
            OperationOutcome::succeeded(
                index,
                operation_id,
                kind,
                project_id,
                task,
                reconciled || resume.is_some(),
            )
        }
        Err(error) => {
            if execution_failure_is_definitive(&error)
                && let Err(checkpoint_error) = checkpoint_failed(
                    checkpoint.as_deref(),
                    operation_key,
                    kind,
                    project_id,
                    target_task_id,
                )
                .await
            {
                return OperationOutcome::failed_cli(
                    index,
                    operation_id,
                    kind,
                    Some(project_id),
                    target_task_id,
                    checkpoint_error,
                );
            }
            OperationOutcome::failed_runtime_parts(
                index,
                operation_id,
                kind,
                Some(project_id),
                target_task_id,
                error,
            )
        }
    }
}

async fn checkpoint_started(
    checkpoint: Option<&CheckpointStore>,
    operation_key: String,
    metadata: &StartedMetadata,
) -> CliResult<()> {
    match checkpoint {
        Some(checkpoint) => checkpoint.record_started(operation_key, metadata).await,
        None => Ok(()),
    }
}

async fn checkpoint_succeeded(
    checkpoint: Option<&CheckpointStore>,
    operation_key: String,
    kind: OperationKind,
    project_id: Uuid,
    task: &AgentTaskSummary,
) -> CliResult<()> {
    match checkpoint {
        Some(checkpoint) => {
            checkpoint
                .record_succeeded(operation_key, kind, project_id, task.id, task.updated_at)
                .await
        }
        None => Ok(()),
    }
}

async fn checkpoint_failed(
    checkpoint: Option<&CheckpointStore>,
    operation_key: String,
    kind: OperationKind,
    project_id: Uuid,
    task_id: Option<Uuid>,
) -> CliResult<()> {
    match checkpoint {
        Some(checkpoint) => {
            checkpoint
                .record_failed(operation_key, kind, project_id, task_id)
                .await
        }
        None => Ok(()),
    }
}

fn execution_failure_is_definitive(error: &PublicError) -> bool {
    matches!(
        error.code(),
        "validation"
            | "not_found"
            | "entitlement"
            | "payload_too_large"
            | "authentication"
            | "forbidden"
    )
}

fn resume_prepare_safety_error(
    kind: OperationKind,
    original_update_revision: Option<DateTime<Utc>>,
    error: &PublicError,
) -> Option<CliError> {
    (kind == OperationKind::TaskUpdate
        && original_update_revision.is_some()
        && error.code() == "conflict")
    .then(|| {
        CliError::checkpoint_conflict(
            "cannot safely resume task.update because its original revision is no longer current",
        )
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum OutcomeStatus {
    Succeeded,
    Failed,
    Skipped,
    Planned,
    Interrupted,
}

struct OperationOutcome {
    index: usize,
    operation_id: String,
    kind: OperationKind,
    status: OutcomeStatus,
    project_id: Option<Uuid>,
    task_id: Option<Uuid>,
    task_reference_number: Option<i64>,
    task_reference: Option<String>,
    updated_at: Option<DateTime<Utc>>,
    resumed: bool,
    plan: Option<TaskMutationPlan>,
    error: Option<OperationError>,
    fatal_exit_code: Option<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationError {
    code: String,
    message: String,
}

impl OperationOutcome {
    fn succeeded(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Uuid,
        task: AgentTaskSummary,
        resumed: bool,
    ) -> Self {
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Succeeded,
            project_id: Some(project_id),
            task_id: Some(task.id),
            task_reference_number: task.reference_number,
            task_reference: task.reference,
            updated_at: Some(task.updated_at),
            resumed,
            plan: None,
            error: None,
            fatal_exit_code: None,
        }
    }

    fn skipped(
        operation: &BatchOperation,
        project_id: Uuid,
        task_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            index: operation.index,
            operation_id: operation.operation_id.clone(),
            kind: operation_kind(&operation.kind),
            status: OutcomeStatus::Skipped,
            project_id: Some(project_id),
            task_id: Some(task_id),
            task_reference_number: None,
            task_reference: None,
            updated_at: Some(updated_at),
            resumed: true,
            plan: None,
            error: None,
            fatal_exit_code: None,
        }
    }

    fn planned(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Uuid,
        task_id: Option<Uuid>,
        plan: TaskMutationPlan,
    ) -> Self {
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Planned,
            project_id: Some(project_id),
            task_id,
            task_reference_number: None,
            task_reference: None,
            updated_at: None,
            resumed: false,
            plan: Some(plan),
            error: None,
            fatal_exit_code: None,
        }
    }

    fn failed_runtime(
        operation: &BatchOperation,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
        error: PublicError,
    ) -> Self {
        Self::failed_runtime_parts(
            operation.index,
            operation.operation_id.clone(),
            operation_kind(&operation.kind),
            project_id,
            task_id,
            error,
        )
    }

    fn failed_runtime_parts(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
        error: PublicError,
    ) -> Self {
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Failed,
            project_id,
            task_id,
            task_reference_number: None,
            task_reference: None,
            updated_at: None,
            resumed: false,
            plan: None,
            error: Some(OperationError {
                code: error.code().to_string(),
                message: sanitized_operation_error(error.code()).to_string(),
            }),
            fatal_exit_code: None,
        }
    }

    fn failed_cli(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
        error: CliError,
    ) -> Self {
        let fatal_exit_code = Some(error.exit_code());
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Failed,
            project_id,
            task_id,
            task_reference_number: None,
            task_reference: None,
            updated_at: None,
            resumed: false,
            plan: None,
            error: Some(OperationError {
                code: error.code().to_string(),
                message: sanitized_operation_error(error.code()).to_string(),
            }),
            fatal_exit_code,
        }
    }

    fn interrupted_resolution(operation: &BatchOperation) -> Self {
        Self::interrupted_resolution_parts(operation, None)
    }

    fn interrupted_resolution_with_project(operation: &BatchOperation, project_id: Uuid) -> Self {
        Self::interrupted_resolution_parts(operation, Some(project_id))
    }

    fn interrupted_resolution_parts(operation: &BatchOperation, project_id: Option<Uuid>) -> Self {
        Self::interrupted_parts(
            operation.index,
            operation.operation_id.clone(),
            operation_kind(&operation.kind),
            project_id,
            None,
        )
    }

    fn interrupted_parts(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
    ) -> Self {
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Interrupted,
            project_id,
            task_id,
            task_reference_number: None,
            task_reference: None,
            updated_at: None,
            resumed: false,
            plan: None,
            error: None,
            fatal_exit_code: Some(130),
        }
    }

    fn ambiguous_interrupted_parts(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
    ) -> Self {
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Interrupted,
            project_id,
            task_id,
            task_reference_number: None,
            task_reference: None,
            updated_at: None,
            resumed: false,
            plan: None,
            error: Some(OperationError {
                code: "outcome_ambiguous".to_string(),
                message: sanitized_operation_error("outcome_ambiguous").to_string(),
            }),
            fatal_exit_code: Some(130),
        }
    }

    fn session_ambiguous_interrupted_parts(
        index: usize,
        operation_id: String,
        kind: OperationKind,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
    ) -> Self {
        Self {
            index,
            operation_id,
            kind,
            status: OutcomeStatus::Interrupted,
            project_id,
            task_id,
            task_reference_number: None,
            task_reference: None,
            updated_at: None,
            resumed: false,
            plan: None,
            error: Some(OperationError {
                code: SESSION_OUTCOME_AMBIGUOUS.to_string(),
                message: sanitized_operation_error(SESSION_OUTCOME_AMBIGUOUS).to_string(),
            }),
            fatal_exit_code: Some(130),
        }
    }
}

#[derive(Default)]
struct OutcomeCounters {
    succeeded: usize,
    failed: usize,
    skipped: usize,
    planned: usize,
    interrupted: usize,
}

impl OutcomeCounters {
    fn record(&mut self, outcome: &OperationOutcome) {
        match outcome.status {
            OutcomeStatus::Succeeded => self.succeeded += 1,
            OutcomeStatus::Failed => self.failed += 1,
            OutcomeStatus::Skipped => self.skipped += 1,
            OutcomeStatus::Planned => self.planned += 1,
            OutcomeStatus::Interrupted => self.interrupted += 1,
        }
    }

    fn processed(&self) -> usize {
        self.succeeded + self.failed + self.skipped + self.planned + self.interrupted
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationRecord<'a> {
    schema_version: u8,
    #[serde(rename = "type")]
    record_type: &'static str,
    input_index: usize,
    operation_id: &'a str,
    operation: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_reference_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_reference: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "is_false")]
    resumed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<&'a TaskMutationPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a OperationError>,
}

fn operation_record(outcome: &OperationOutcome) -> OperationRecord<'_> {
    OperationRecord {
        schema_version: BATCH_OUTPUT_SCHEMA_VERSION,
        record_type: "batch.operation",
        input_index: outcome.index + 1,
        operation_id: &outcome.operation_id,
        operation: operation_name(outcome.kind),
        status: outcome_status_name(outcome.status),
        project_id: outcome.project_id,
        task_id: outcome.task_id,
        task_reference_number: outcome.task_reference_number,
        task_reference: outcome.task_reference.as_deref(),
        updated_at: outcome.updated_at,
        resumed: outcome.resumed,
        plan: outcome.plan.as_ref(),
        error: outcome.error.as_ref(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchSummary {
    total: usize,
    succeeded: usize,
    failed: usize,
    skipped: usize,
    planned: usize,
    not_run: usize,
    interrupted_count: usize,
    interrupted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SummaryRecord<'a> {
    schema_version: u8,
    #[serde(rename = "type")]
    record_type: &'static str,
    #[serde(flatten)]
    summary: &'a BatchSummary,
}

fn emit_outcome(format: OutputFormat, outcome: &OperationOutcome) -> CliResult<()> {
    match format {
        OutputFormat::Jsonl => print_jsonl(
            &operation_record(outcome),
            "serializing batch operation result should succeed",
        ),
        OutputFormat::Table => {
            let target = match (outcome.task_reference.as_deref(), outcome.task_id) {
                (Some(reference), Some(task_id)) => format!("{reference} ({task_id})"),
                (_, Some(task_id)) => task_id.to_string(),
                _ => "-".to_string(),
            };
            let suffix = outcome
                .error
                .as_ref()
                .map_or_else(String::new, |error| format!(" error={}", error.code));
            write_stdout_line_flushed(format_args!(
                "{:>5}  {:<24}  {:<11}  task={}{}",
                outcome.index + 1,
                outcome.operation_id,
                outcome_status_name(outcome.status),
                target,
                suffix
            ))
        }
        OutputFormat::Json | OutputFormat::JsonPretty => unreachable!("validated batch format"),
    }
}

fn emit_summary(format: OutputFormat, summary: &BatchSummary) -> CliResult<()> {
    match format {
        OutputFormat::Jsonl => print_jsonl(
            &SummaryRecord {
                schema_version: BATCH_OUTPUT_SCHEMA_VERSION,
                record_type: "batch.summary",
                summary,
            },
            "serializing batch summary should succeed",
        ),
        OutputFormat::Table => write_stdout_line_flushed(format_args!(
            "summary  total={} succeeded={} failed={} skipped={} planned={} not-run={} interrupted-count={} interrupted={}",
            summary.total,
            summary.succeeded,
            summary.failed,
            summary.skipped,
            summary.planned,
            summary.not_run,
            summary.interrupted_count,
            summary.interrupted
        )),
        OutputFormat::Json | OutputFormat::JsonPretty => unreachable!("validated batch format"),
    }
}

fn operation_name(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::TaskCreate => "task.create",
        OperationKind::TaskUpdate => "task.update",
    }
}

fn outcome_status_name(status: OutcomeStatus) -> &'static str {
    match status {
        OutcomeStatus::Succeeded => "succeeded",
        OutcomeStatus::Failed => "failed",
        OutcomeStatus::Skipped => "skipped",
        OutcomeStatus::Planned => "planned",
        OutcomeStatus::Interrupted => "interrupted",
    }
}

fn sanitized_operation_error(code: &str) -> &'static str {
    match code {
        "validation" => "operation input or target is invalid",
        "not_found" => "operation target was not found",
        "conflict" => "operation conflicts with current resource state",
        "authentication" => "authentication is required",
        "forbidden" => "operation is not permitted",
        "entitlement" => "operation requires a different entitlement",
        "rate_limited" => "operation was rate limited",
        "request_timeout" | "transport_timeout" => "operation timed out",
        "outcome_ambiguous" => {
            "operation outcome is ambiguous; inspect the affected resource before retrying"
        }
        SESSION_OUTCOME_AMBIGUOUS => {
            "credential rotation outcome is ambiguous; sign in again if authentication no longer works"
        }
        "checkpoint_conflict" => "checkpoint state conflicts with this batch",
        "checkpoint_io" => "checkpoint durability operation failed",
        _ => "operation failed",
    }
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn immediate(index: usize) -> ResolvedItem {
        ResolvedItem::Immediate(OperationOutcome {
            index,
            operation_id: format!("op-{index}"),
            kind: OperationKind::TaskCreate,
            status: OutcomeStatus::Skipped,
            project_id: Some(Uuid::now_v7()),
            task_id: Some(Uuid::now_v7()),
            task_reference_number: None,
            task_reference: None,
            updated_at: Some(Utc::now()),
            resumed: true,
            plan: None,
            error: None,
            fatal_exit_code: None,
        })
    }

    #[test]
    fn scheduler_skips_busy_task_targets_but_keeps_unrelated_work_moving() {
        let project = Uuid::now_v7();
        let task = Uuid::now_v7();
        let busy = ResolvedItem::Mutation(ResolvedOperation {
            index: 0,
            operation_id: "busy".to_string(),
            operation_key: "a".repeat(64),
            kind: OperationKind::TaskUpdate,
            mutation: Mutation {
                project_id: project,
                input: MutationInput::TaskUpdate {
                    task_id: task,
                    input: sealtask_client_runtime::TaskUpdateInput {
                        title: Some("one".to_string()),
                        body: sealtask_client_runtime::TaskFieldPatch::Unchanged,
                        checklist: sealtask_client_runtime::TaskFieldPatch::Unchanged,
                        priority: sealtask_client_runtime::TaskFieldPatch::Unchanged,
                        due_at: sealtask_client_runtime::TaskFieldPatch::Unchanged,
                        start_at: sealtask_client_runtime::TaskFieldPatch::Unchanged,
                        section_id: sealtask_client_runtime::TaskFieldPatch::Unchanged,
                    },
                },
            },
            resume: None,
        });
        let queue = VecDeque::from([busy, immediate(1)]);
        assert_eq!(
            next_schedulable(&queue, &HashSet::from([(project, task)])),
            Some(1)
        );
    }

    #[test]
    fn scheduled_operations_have_isolated_cancellation_tokens() {
        let root_cancellation = ApiCancellationToken::new();
        let runtime = RuntimeClient::new("https://api.example")
            .expect("runtime")
            .with_api_cancellation_token(root_cancellation.clone());
        let (first_runtime, first_cancellation) = runtime_for_scheduled_operation(&runtime);
        let (second_runtime, second_cancellation) = runtime_for_scheduled_operation(&runtime);

        assert_eq!(
            runtime.api_cancellation_token(),
            Some(root_cancellation.clone())
        );
        assert_eq!(
            first_runtime.api_cancellation_token(),
            Some(first_cancellation.clone())
        );
        assert_eq!(
            second_runtime.api_cancellation_token(),
            Some(second_cancellation.clone())
        );
        assert_ne!(first_cancellation, second_cancellation);
        assert_ne!(first_cancellation, root_cancellation);
        assert_ne!(second_cancellation, root_cancellation);

        first_cancellation.cancel();
        assert!(first_cancellation.is_cancelled());
        assert!(!second_cancellation.is_cancelled());
        assert!(!root_cancellation.is_cancelled());
    }

    #[test]
    fn definitive_failure_classification_preserves_ambiguous_started_state() {
        assert!(!execution_failure_is_definitive(&PublicError::conflict(
            "revision conflict"
        )));
        assert!(!execution_failure_is_definitive(
            &PublicError::request_timeout("ambiguous timeout")
        ));
        assert!(!execution_failure_is_definitive(
            &PublicError::outcome_ambiguous("task update", "response lost")
        ));
    }

    #[test]
    fn resume_revision_conflict_is_a_typed_checkpoint_safety_failure() {
        let error = resume_prepare_safety_error(
            OperationKind::TaskUpdate,
            Some(Utc::now()),
            &PublicError::conflict("task changed"),
        )
        .expect("checkpoint safety error");
        assert_eq!(error.code(), "checkpoint_conflict");
        assert_eq!(error.exit_code(), 4);

        assert!(
            resume_prepare_safety_error(
                OperationKind::TaskUpdate,
                None,
                &PublicError::conflict("fresh preparation conflict"),
            )
            .is_none()
        );
    }

    #[test]
    fn operation_error_records_do_not_echo_selector_plaintext() {
        let canary = "selector-plaintext-canary";
        let outcome = OperationOutcome::failed_runtime_parts(
            0,
            "op-1".to_string(),
            OperationKind::TaskUpdate,
            None,
            None,
            PublicError::validation(format!("no task matched {canary}")),
        );
        let encoded =
            serde_json::to_string(outcome.error.as_ref().expect("error")).expect("serialize");
        assert!(!encoded.contains(canary));
        assert!(encoded.contains("\"code\":\"validation\""));
    }

    #[test]
    fn successful_operation_records_add_reference_without_replacing_canonical_task_id() {
        let project_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let outcome = OperationOutcome {
            index: 0,
            operation_id: "op-1".to_string(),
            kind: OperationKind::TaskUpdate,
            status: OutcomeStatus::Succeeded,
            project_id: Some(project_id),
            task_id: Some(task_id),
            task_reference_number: Some(184),
            task_reference: Some("OPS-0184".to_string()),
            updated_at: Some(Utc::now()),
            resumed: false,
            plan: None,
            error: None,
            fatal_exit_code: None,
        };

        let record =
            serde_json::to_value(operation_record(&outcome)).expect("serialize operation record");
        assert_eq!(record["projectId"], project_id.to_string());
        assert_eq!(record["taskId"], task_id.to_string());
        assert_eq!(record["taskReferenceNumber"], 184);
        assert_eq!(record["taskReference"], "OPS-0184");
    }

    #[test]
    fn forced_in_flight_mutation_retains_identity_and_is_counted_as_ambiguous() {
        let project_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let progress = OperationProgress::new();
        let operation = RunningOperation {
            target: Some((project_id, task_id)),
            index: 4,
            operation_id: "stable-operation-id".to_string(),
            kind: OperationKind::TaskUpdate,
            project_id,
            task_id: Some(task_id),
            progress: progress.clone(),
            cancellation: ApiCancellationToken::new(),
            credential_refresh_interrupted: Arc::new(AtomicBool::new(false)),
        };
        progress.mark_mutation_in_flight();

        let outcome = operation.forced_outcome();
        let mut counters = OutcomeCounters::default();
        counters.record(&outcome);

        assert_eq!(outcome.index, 4);
        assert_eq!(outcome.operation_id, "stable-operation-id");
        assert_eq!(outcome.project_id, Some(project_id));
        assert_eq!(outcome.task_id, Some(task_id));
        assert!(outcome.status == OutcomeStatus::Interrupted);
        assert_eq!(
            outcome.error.as_ref().map(|error| error.code.as_str()),
            Some("outcome_ambiguous")
        );
        assert_eq!(counters.interrupted, 1);
        assert_eq!(counters.processed(), 1);
    }

    #[test]
    fn forced_mutation_with_a_received_response_stays_ambiguous_if_its_result_is_lost() {
        let project_id = Uuid::now_v7();
        let progress = OperationProgress::new();
        let operation = RunningOperation {
            target: None,
            index: 0,
            operation_id: "response-received".to_string(),
            kind: OperationKind::TaskCreate,
            project_id,
            task_id: None,
            progress: progress.clone(),
            cancellation: ApiCancellationToken::new(),
            credential_refresh_interrupted: Arc::new(AtomicBool::new(false)),
        };
        progress.mark_mutation_in_flight();
        progress.mark_response_received();

        let outcome = operation.forced_outcome();

        assert_eq!(
            outcome.error.as_ref().map(|error| error.code.as_str()),
            Some("outcome_ambiguous")
        );
    }

    #[test]
    fn forced_preparation_with_a_terminal_refresh_does_not_claim_session_ambiguity() {
        let project_id = Uuid::now_v7();
        let credential_refresh_interrupted = Arc::new(AtomicBool::new(true));
        let operation = RunningOperation {
            target: None,
            index: 2,
            operation_id: "credential-refresh".to_string(),
            kind: OperationKind::TaskCreate,
            project_id,
            task_id: None,
            progress: OperationProgress::new(),
            cancellation: ApiCancellationToken::new(),
            credential_refresh_interrupted,
        };

        let outcome = operation.forced_outcome();

        assert!(outcome.status == OutcomeStatus::Interrupted);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.fatal_exit_code, Some(130));
    }

    #[test]
    fn forced_interruption_only_mentions_a_checkpoint_when_one_exists() {
        let without_checkpoint = forced_interruption_message(false, true);
        assert!(!without_checkpoint.contains("checkpoint"));
        assert!(without_checkpoint.contains("inspect the affected resources"));

        let with_checkpoint = forced_interruption_message(true, true);
        assert!(with_checkpoint.contains("resume from the checkpoint"));
    }
}
