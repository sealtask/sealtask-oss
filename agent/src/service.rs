use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use chrono::{DateTime, Utc};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _},
    process::Command,
    sync::watch,
};
use uuid::Uuid;
use zeroize::Zeroizing;

use sealtask_client_api::{
    AgentApiClient, AgentClaimResponse, AgentRunHeartbeatRequest, ApiTransportOptions,
    ClaimAgentAssignmentRequest, FinishAgentRunRequest,
};
use sealtask_client_auth::{
    AgentIdentity, AgentKeyMaterial, LocalAgentStatus, activate_agent_identity,
    canonicalize_agent_audience, canonicalize_agent_display_name, canonicalize_agent_handle,
    config_dir, list_agent_identities_with_failures, load_agent_identity, load_agent_key_material,
    mark_agent_identity_expired,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    AgentGrantAuthenticationInput, SymmetricKey, TASK_TITLE_CONTEXT, TaskPayloadEnvelope,
    decrypt_agent_instructions, decrypt_agent_project_key, decrypt_encrypted_text_value,
    decrypt_task_payload, encrypt_agent_run_result, verify_agent_grant,
};

use crate::harness::{
    Harness, HarnessOutput, ProcessTreeGuard, apply_git_environment, configure_process_tree,
    resume_process_tree, wait_for_process_tree,
};

const AGENT_PERMISSION_PRESET: &str = "assigned_task_worker";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const HEARTBEAT_RETRY_DELAY: Duration = Duration::from_secs(1);
const HEARTBEAT_MAX_ATTEMPTS: usize = 3;
const FINISH_RETRY_DELAY: Duration = Duration::from_secs(1);
const RUN_RETRY_MAX_DELAY: Duration = Duration::from_secs(30);
const RUN_RETRY_JITTER_MAX: Duration = Duration::from_millis(250);
const LEASE_RETRY_SAFETY_MARGIN: Duration = Duration::from_secs(1);
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(30);
const TOKEN_REFRESH_JITTER_MAX: Duration = Duration::from_secs(10);
const TOKEN_REFRESH_RETRY_INITIAL: Duration = Duration::from_secs(5);
const TOKEN_REFRESH_RETRY_MAX: Duration = Duration::from_secs(30);
const TOKEN_EXPIRY_SAFETY_MARGIN: Duration = Duration::from_secs(5);
const MAX_AGENT_TOKEN_LIFETIME: Duration = Duration::from_secs(5 * 60);
const ACTIVE_AUTH_RETRY_INITIAL: Duration = Duration::from_secs(60);
const PENDING_APPROVAL_RETRY_INITIAL: Duration = Duration::from_secs(60);
const EXPIRED_ENROLLMENT_RETRY_INITIAL: Duration = Duration::from_secs(60 * 60);
const IDENTITY_AUTH_RETRY_MAX: Duration = Duration::from_secs(60 * 60 * 24);
const IDENTITY_AUTH_RETRY_JITTER_MAX: Duration = Duration::from_secs(5);
const MAX_UNCACHED_AUTH_ATTEMPTS_PER_POLL: usize = 1;
const MAX_GIT_OUTPUT_BYTES: usize = 16 * 1024;
const REPOSITORY_PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(crate) struct CompletedRun {
    pub(crate) agent_id: Uuid,
    pub(crate) run_id: Uuid,
    pub(crate) status: String,
    pub(crate) worktree: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct PollFailure {
    pub(crate) agent_id: Uuid,
    pub(crate) message: String,
}

#[derive(Debug, Default)]
pub(crate) struct PollOutcome {
    pub(crate) configured_identities: usize,
    pub(crate) active_identities: usize,
    pub(crate) completed_runs: Vec<CompletedRun>,
    pub(crate) failures: Vec<PollFailure>,
    pub(crate) unconfirmed_terminal_runs: usize,
}

#[derive(Debug)]
pub(crate) struct AgentService {
    runner_instance_id: Uuid,
    selected_agent_id: Option<Uuid>,
    run_timeout: Duration,
    identity_retries: tokio::sync::Mutex<HashMap<Uuid, IdentityRetryState>>,
    last_uncached_auth_attempt: tokio::sync::Mutex<Option<Uuid>>,
    sessions: tokio::sync::Mutex<HashMap<Uuid, AgentSession>>,
    unconfirmed_finishes: tokio::sync::Mutex<HashMap<Uuid, DateTime<Utc>>>,
}

#[derive(Debug)]
struct AgentSession {
    client: AgentApiClient,
    refresh_at: Instant,
    expires_at: Instant,
    refresh_failures: u32,
    refresh_retry_not_before: Option<Instant>,
}

#[derive(Clone, Copy, Debug)]
struct IdentityRetryState {
    failures: u32,
    retry_at: Instant,
}

#[derive(Debug)]
struct RunPaths {
    run_directory: PathBuf,
    worktree: PathBuf,
}

#[derive(Debug)]
struct PreparedClaim {
    project_key: SymmetricKey,
    prompt: Zeroizing<String>,
}

impl AgentService {
    pub(crate) fn new(
        runner_instance_id: Uuid,
        selected_agent_id: Option<Uuid>,
        run_timeout: Duration,
    ) -> Self {
        Self {
            runner_instance_id,
            selected_agent_id,
            run_timeout,
            identity_retries: tokio::sync::Mutex::new(HashMap::new()),
            last_uncached_auth_attempt: tokio::sync::Mutex::new(None),
            sessions: tokio::sync::Mutex::new(HashMap::new()),
            unconfirmed_finishes: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn poll_once(
        &self,
        harness: &dyn Harness,
        shutdown: &mut watch::Receiver<bool>,
    ) -> PublicResult<PollOutcome> {
        let mut outcome = PollOutcome::default();
        let mut candidates = if let Some(agent_id) = self.selected_agent_id {
            outcome.configured_identities = 1;
            vec![load_agent_identity(agent_id)?.ok_or_else(|| {
                PublicError::not_found(format!("local agent identity {agent_id} was not found"))
            })?]
        } else {
            let listing = list_agent_identities_with_failures()?;
            outcome.configured_identities = listing.discovered_identities;
            outcome
                .failures
                .extend(listing.failures.into_iter().map(|failure| PollFailure {
                    agent_id: failure.agent_id,
                    message: failure.message,
                }));
            listing.identities
        };

        let cached_sessions = self
            .sessions
            .lock()
            .await
            .keys()
            .copied()
            .collect::<HashSet<_>>();
        candidates.sort_by_key(|identity| identity_poll_priority(identity, &cached_sessions));
        rotate_uncached_auth_candidates(
            &mut candidates,
            &cached_sessions,
            *self.last_uncached_auth_attempt.lock().await,
        );
        let mut uncached_auth_attempts = 0;

        for mut identity in candidates {
            if shutdown_requested(shutdown) {
                break;
            }
            if identity.status == LocalAgentStatus::Pending
                && identity
                    .enrollment_expires_at
                    .is_some_and(|expires_at| expires_at <= Utc::now())
            {
                identity = mark_agent_identity_expired(identity.agent_id)?;
            }
            if identity.status == LocalAgentStatus::Revoked
                || !self.identity_retry_is_due(identity.agent_id).await
            {
                continue;
            }
            let (keys, source_revision) = match prepare_identity_poll(&identity, shutdown).await {
                Ok(Some(prepared)) => prepared,
                Ok(None) => break,
                Err(error) => {
                    self.defer_identity_retry(
                        identity.agent_id,
                        identity.status,
                        error.retry_after(),
                    )
                    .await;
                    outcome.failures.push(PollFailure {
                        agent_id: identity.agent_id,
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let mut attempted_uncached_auth = false;
            let identity = match identity.status {
                LocalAgentStatus::Active => {
                    if !cached_sessions.contains(&identity.agent_id) {
                        if uncached_auth_attempts >= MAX_UNCACHED_AUTH_ATTEMPTS_PER_POLL {
                            continue;
                        }
                        uncached_auth_attempts += 1;
                        attempted_uncached_auth = true;
                        self.record_uncached_auth_attempt(identity.agent_id).await;
                    }
                    identity
                }
                LocalAgentStatus::Pending | LocalAgentStatus::Expired => {
                    if uncached_auth_attempts >= MAX_UNCACHED_AUTH_ATTEMPTS_PER_POLL {
                        continue;
                    }
                    uncached_auth_attempts += 1;
                    attempted_uncached_auth = true;
                    self.record_uncached_auth_attempt(identity.agent_id).await;
                    match reconcile_pending_identity(&identity, &keys).await {
                        Ok(Some((identity, session))) => {
                            self.clear_identity_retry(identity.agent_id).await;
                            self.sessions
                                .lock()
                                .await
                                .insert(identity.agent_id, session);
                            identity
                        }
                        Ok(None) => {
                            self.defer_identity_retry(identity.agent_id, identity.status, None)
                                .await;
                            continue;
                        }
                        Err(error) => {
                            self.defer_identity_retry(
                                identity.agent_id,
                                identity.status,
                                error.retry_after(),
                            )
                            .await;
                            outcome.failures.push(PollFailure {
                                agent_id: identity.agent_id,
                                message: error.to_string(),
                            });
                            continue;
                        }
                    }
                }
                LocalAgentStatus::Revoked => continue,
            };
            outcome.active_identities += 1;
            match self
                .poll_identity(&identity, &keys, &source_revision, harness, shutdown)
                .await
            {
                Ok(Some(completed)) => {
                    self.clear_identity_retry(identity.agent_id).await;
                    outcome.completed_runs.push(completed);
                }
                Ok(None) => self.clear_identity_retry(identity.agent_id).await,
                Err(error) => {
                    let session_was_established =
                        self.sessions.lock().await.contains_key(&identity.agent_id);
                    if error.http_status() == Some(401) {
                        // Authentication failures deliberately do not disclose whether
                        // the identity was revoked, its owner lost access, the assertion
                        // audience changed, or the local clock is outside policy. Drop
                        // the cached session, but do not irreversibly rewrite local
                        // identity state from that ambiguous signal.
                        self.sessions.lock().await.remove(&identity.agent_id);
                    }
                    if matches!(error.http_status(), Some(401 | 429))
                        || (attempted_uncached_auth && !session_was_established)
                    {
                        self.defer_identity_retry(
                            identity.agent_id,
                            identity.status,
                            error.retry_after(),
                        )
                        .await;
                    }
                    outcome.failures.push(PollFailure {
                        agent_id: identity.agent_id,
                        message: error.to_string(),
                    });
                }
            }
        }
        outcome.unconfirmed_terminal_runs =
            self.active_unconfirmed_terminal_run_count(Utc::now()).await;
        Ok(outcome)
    }

    async fn active_unconfirmed_terminal_run_count(&self, now: DateTime<Utc>) -> usize {
        let mut finishes = self.unconfirmed_finishes.lock().await;
        finishes.retain(|_, lease_expires_at| *lease_expires_at > now);
        finishes.len()
    }

    async fn identity_retry_is_due(&self, agent_id: Uuid) -> bool {
        self.identity_retries
            .lock()
            .await
            .get(&agent_id)
            .is_none_or(|retry| Instant::now() >= retry.retry_at)
    }

    async fn defer_identity_retry(
        &self,
        agent_id: Uuid,
        status: LocalAgentStatus,
        retry_after: Option<Duration>,
    ) {
        let base = match status {
            LocalAgentStatus::Pending => PENDING_APPROVAL_RETRY_INITIAL,
            LocalAgentStatus::Expired => EXPIRED_ENROLLMENT_RETRY_INITIAL,
            LocalAgentStatus::Active | LocalAgentStatus::Revoked => ACTIVE_AUTH_RETRY_INITIAL,
        };
        let mut retries = self.identity_retries.lock().await;
        let failures = retries
            .get(&agent_id)
            .map_or(1, |retry| retry.failures.saturating_add(1));
        let delay = identity_retry_delay(base, failures, retry_after, agent_id);
        retries.insert(
            agent_id,
            IdentityRetryState {
                failures,
                retry_at: Instant::now() + delay,
            },
        );
    }

    async fn clear_identity_retry(&self, agent_id: Uuid) {
        self.identity_retries.lock().await.remove(&agent_id);
    }

    async fn record_uncached_auth_attempt(&self, agent_id: Uuid) {
        *self.last_uncached_auth_attempt.lock().await = Some(agent_id);
    }

    async fn poll_identity(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        source_revision: &str,
        harness: &dyn Harness,
        shutdown: &mut watch::Receiver<bool>,
    ) -> PublicResult<Option<CompletedRun>> {
        let mut session = match self.sessions.lock().await.remove(&identity.agent_id) {
            Some(session) => session,
            None => mint_agent_session(identity, keys).await?,
        };
        let result = async {
            ensure_fresh_agent_session(&mut session, identity, keys).await?;
            self.poll_identity_with_session(
                identity,
                keys,
                &mut session,
                source_revision,
                harness,
                shutdown,
            )
            .await
        }
        .await;
        // The caller removes a cached session on an authoritative 401. Keep it
        // across all other errors so a transient poll failure does not force a
        // needless token mint and assertion reservation on the next pass.
        self.sessions
            .lock()
            .await
            .insert(identity.agent_id, session);
        result
    }

    async fn poll_identity_with_session(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        source_revision: &str,
        harness: &dyn Harness,
        shutdown: &mut watch::Receiver<bool>,
    ) -> PublicResult<Option<CompletedRun>> {
        if shutdown_requested(shutdown) {
            return Ok(None);
        }
        let Some(assignment) = session.client.next_assignment().await? else {
            return Ok(None);
        };
        if assignment.work_list_id != identity.project.work_list_id || !assignment.claimable {
            return Err(PublicError::validation(
                "next agent assignment did not match the local project binding",
            ));
        }
        if shutdown_requested(shutdown) {
            return Ok(None);
        }
        let claim = match session
            .client
            .claim_assignment(
                assignment.id,
                &ClaimAgentAssignmentRequest {
                    runner_instance_id: self.runner_instance_id,
                    source_revision: Some(source_revision.to_string()),
                },
            )
            .await
        {
            Ok(claim) => claim,
            Err(error) if error.code() == "assignment_not_available" => return Ok(None),
            Err(error) => return Err(error),
        };

        self.execute_claim(
            identity,
            keys,
            session,
            claim,
            source_revision,
            harness,
            shutdown,
        )
        .await
        .map(Some)
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_claim(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        mut claim: AgentClaimResponse,
        source_revision: &str,
        harness: &dyn Harness,
        shutdown: &mut watch::Receiver<bool>,
    ) -> PublicResult<CompletedRun> {
        let run_deadline = tokio::time::Instant::now() + self.run_timeout;
        let run_id = claim.run.id;
        let lease_token = Zeroizing::new(std::mem::take(&mut claim.lease_token));
        let initial_lease = RunLeaseState::from_claim(&claim);
        let PreparedClaim {
            project_key,
            prompt,
        } = match prepare_claim(identity, keys, &claim) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.finish_run(
                    identity,
                    keys,
                    session,
                    &claim,
                    lease_token.as_str(),
                    &initial_lease,
                    "failed",
                    None,
                    Some("grant_authentication"),
                )
                .await?;
                return Ok(CompletedRun {
                    agent_id: identity.agent_id,
                    run_id,
                    status: format!("failed ({})", error.code()),
                    worktree: None,
                });
            }
        };
        let paths = run_paths(identity.agent_id, run_id)?;

        let execution = if shutdown_requested(shutdown) {
            VersionedHarnessOutput::cancelled(initial_lease)
        } else {
            match self
                .prepare_worktree_with_heartbeats(
                    identity,
                    keys,
                    session,
                    &claim,
                    lease_token.as_str(),
                    source_revision,
                    &paths,
                    initial_lease,
                    run_deadline,
                    shutdown,
                )
                .await
            {
                WorktreePreparation::Ready(lease) => {
                    self.run_with_heartbeats(
                        identity,
                        keys,
                        session,
                        &claim,
                        lease_token.as_str(),
                        &paths,
                        prompt.as_str(),
                        harness,
                        lease,
                        run_deadline,
                        shutdown,
                    )
                    .await?
                }
                WorktreePreparation::Terminal(output) => output,
            }
        };
        let status = execution.status.as_api_value();
        let failure_code = execution.status.requires_failure_code().then(|| {
            execution
                .failure_code
                .as_deref()
                .unwrap_or("harness_failed")
        });
        let result = encrypt_agent_run_result(&execution.summary, run_id, &project_key)?;
        self.finish_run(
            identity,
            keys,
            session,
            &claim,
            lease_token.as_str(),
            &execution.lease,
            status,
            Some(result.base64.as_str()),
            failure_code,
        )
        .await?;
        let worktree = tokio::fs::try_exists(&paths.worktree)
            .await
            .unwrap_or(false)
            .then_some(paths.worktree);

        Ok(CompletedRun {
            agent_id: identity.agent_id,
            run_id,
            status: status.to_string(),
            worktree,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn prepare_worktree_with_heartbeats(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        claim: &AgentClaimResponse,
        lease_token: &str,
        source_revision: &str,
        paths: &RunPaths,
        initial_lease: RunLeaseState,
        run_deadline: tokio::time::Instant,
        shutdown: &mut watch::Receiver<bool>,
    ) -> WorktreePreparation {
        self.supervise_worktree_setup(
            identity,
            keys,
            session,
            claim,
            lease_token,
            initial_lease,
            run_deadline,
            shutdown,
            create_run_worktree(&identity.project.repository_root, source_revision, paths),
            HEARTBEAT_INTERVAL,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn supervise_worktree_setup<F>(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        claim: &AgentClaimResponse,
        lease_token: &str,
        initial_lease: RunLeaseState,
        run_deadline: tokio::time::Instant,
        shutdown: &mut watch::Receiver<bool>,
        setup: F,
        heartbeat_interval: Duration,
    ) -> WorktreePreparation
    where
        F: Future<Output = PublicResult<()>> + Send,
    {
        // Confirm and refresh the lease before starting any potentially slow
        // local setup. This closes the claim-to-first-heartbeat window.
        let mut lease = match tokio::time::timeout_at(
            run_deadline,
            self.heartbeat_with_retry(
                identity,
                keys,
                session,
                claim.run.id,
                lease_token,
                &initial_lease,
            ),
        )
        .await
        {
            Ok(Ok(run)) => RunLeaseState::from_run(&run),
            Ok(Err(_)) => {
                return WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                    "The agent run stopped because its initial lease heartbeat could not be confirmed.",
                    "heartbeat_failed",
                    initial_lease,
                ));
            }
            Err(_) => {
                return WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                    "Worktree setup exceeded the configured run timeout before the initial lease could be confirmed.",
                    "worktree_setup_timeout",
                    initial_lease,
                ));
            }
        };

        tokio::pin!(setup);
        let mut heartbeat = tokio::time::interval_at(
            tokio::time::Instant::now() + heartbeat_interval,
            heartbeat_interval,
        );
        let setup_timeout = tokio::time::sleep_until(run_deadline);
        tokio::pin!(setup_timeout);

        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || shutdown_requested(shutdown) {
                        return WorktreePreparation::Terminal(
                            VersionedHarnessOutput::cancelled(lease),
                        );
                    }
                }
                result = &mut setup => {
                    if let Err(error) = result {
                        return WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                            &format!("The isolated agent worktree could not be created: {error}"),
                            "worktree_setup",
                            lease,
                        ));
                    }
                    break;
                }
                _ = heartbeat.tick() => {
                    match tokio::time::timeout_at(
                        run_deadline,
                        self.heartbeat_with_retry(
                            identity,
                            keys,
                            session,
                            claim.run.id,
                            lease_token,
                            &lease,
                        ),
                    ).await {
                        Ok(Ok(run)) => lease = RunLeaseState::from_run(&run),
                        Ok(Err(_)) => {
                            return WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                                "The agent run stopped because its lease heartbeat could not be confirmed during worktree setup.",
                                "heartbeat_failed",
                                lease,
                            ));
                        }
                        Err(_) => {
                            return WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                                "Creating the isolated agent worktree exceeded the configured run timeout.",
                                "worktree_setup_timeout",
                                lease,
                            ));
                        }
                    }
                }
                () = &mut setup_timeout => {
                    return WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                        "Creating the isolated agent worktree exceeded the configured run timeout.",
                        "worktree_setup_timeout",
                        lease,
                    ));
                }
            }
        }

        // Synchronize once more after setup so Codex starts only with a freshly
        // confirmed lease, even when setup completed just before a scheduled
        // heartbeat.
        match tokio::time::timeout_at(
            run_deadline,
            self.heartbeat_with_retry(identity, keys, session, claim.run.id, lease_token, &lease),
        )
        .await
        {
            Ok(Ok(run)) => WorktreePreparation::Ready(RunLeaseState::from_run(&run)),
            Ok(Err(_)) => WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                "The agent run stopped because its post-setup lease heartbeat could not be confirmed.",
                "heartbeat_failed",
                lease,
            )),
            Err(_) => WorktreePreparation::Terminal(VersionedHarnessOutput::failed(
                "Worktree setup exceeded the configured run timeout before its final lease confirmation.",
                "worktree_setup_timeout",
                lease,
            )),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_with_heartbeats(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        claim: &AgentClaimResponse,
        lease_token: &str,
        paths: &RunPaths,
        prompt: &str,
        harness: &dyn Harness,
        mut lease: RunLeaseState,
        run_deadline: tokio::time::Instant,
        shutdown: &mut watch::Receiver<bool>,
    ) -> PublicResult<VersionedHarnessOutput> {
        let execution = harness.run(&paths.worktree, &paths.run_directory, prompt);
        tokio::pin!(execution);
        let mut heartbeat = tokio::time::interval_at(
            tokio::time::Instant::now() + HEARTBEAT_INTERVAL,
            HEARTBEAT_INTERVAL,
        );
        let timeout = tokio::time::sleep_until(run_deadline);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || shutdown_requested(shutdown) {
                        return Ok(VersionedHarnessOutput::cancelled(lease));
                    }
                }
                output = &mut execution => {
                    return Ok(match output {
                        Ok(output) => VersionedHarnessOutput::new(output, lease),
                        Err(_) => VersionedHarnessOutput::failed(
                            "The configured Codex harness failed before producing a result.",
                            "harness_error",
                            lease,
                        ),
                    });
                }
                _ = heartbeat.tick() => {
                    match tokio::time::timeout_at(
                        run_deadline,
                        self.heartbeat_with_retry(
                            identity,
                            keys,
                            session,
                            claim.run.id,
                            lease_token,
                            &lease,
                        ),
                    ).await {
                        Ok(Ok(run)) => lease = RunLeaseState::from_run(&run),
                        Ok(Err(_)) => {
                            return Ok(VersionedHarnessOutput::failed(
                                "The agent run stopped because its lease heartbeat could not be confirmed.",
                                "heartbeat_failed",
                                lease,
                            ));
                        }
                        Err(_) => {
                            return Ok(VersionedHarnessOutput::failed(
                                "Codex execution exceeded the configured run timeout.",
                                "run_timeout",
                                lease,
                            ));
                        }
                    }
                }
                () = &mut timeout => {
                    return Ok(VersionedHarnessOutput::failed(
                        "Codex execution exceeded the configured run timeout.",
                        "run_timeout",
                        lease,
                    ));
                }
            }
        }
    }

    async fn heartbeat_with_retry(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        run_id: Uuid,
        lease_token: &str,
        lease: &RunLeaseState,
    ) -> PublicResult<sealtask_client_api::AgentRunResponse> {
        let heartbeat_id = Uuid::now_v7();
        for attempt in 1..=HEARTBEAT_MAX_ATTEMPTS {
            let result = async {
                ensure_fresh_agent_session(session, identity, keys).await?;
                session
                    .client
                    .heartbeat_run(
                        run_id,
                        &AgentRunHeartbeatRequest {
                            lease_token,
                            expected_version: lease.version,
                            heartbeat_id,
                        },
                    )
                    .await
            }
            .await;
            match result {
                Ok(run) => return Ok(run),
                Err(error)
                    if attempt < HEARTBEAT_MAX_ATTEMPTS && is_retryable_heartbeat_error(&error) =>
                {
                    let delay = run_retry_delay(
                        HEARTBEAT_RETRY_DELAY,
                        u32::try_from(attempt).unwrap_or(u32::MAX),
                        error.retry_after(),
                        heartbeat_id,
                    );
                    if !retry_fits_before_lease(delay, &lease.lease_expires_at) {
                        return Err(error);
                    }
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("heartbeat retry loop always returns")
    }

    #[allow(clippy::too_many_arguments)]
    async fn finish_run(
        &self,
        identity: &AgentIdentity,
        keys: &AgentKeyMaterial,
        session: &mut AgentSession,
        claim: &AgentClaimResponse,
        lease_token: &str,
        lease: &RunLeaseState,
        status: &str,
        result_ciphertext: Option<&str>,
        failure_code: Option<&str>,
    ) -> PublicResult<()> {
        let completion_id = Uuid::now_v7();
        let mut retry_failures = 0_u32;
        loop {
            let result = async {
                ensure_fresh_agent_session(session, identity, keys).await?;
                session
                    .client
                    .finish_run(
                        claim.run.id,
                        &FinishAgentRunRequest {
                            lease_token,
                            expected_version: lease.version,
                            completion_id,
                            status,
                            result_ciphertext,
                            failure_code,
                        },
                    )
                    .await
            }
            .await;
            match result {
                Ok(_) => {
                    self.unconfirmed_finishes.lock().await.remove(&claim.run.id);
                    return Ok(());
                }
                Err(error) => {
                    if is_retryable_finish_error(&error) {
                        retry_failures = retry_failures.saturating_add(1);
                        let delay = run_retry_delay(
                            FINISH_RETRY_DELAY,
                            retry_failures,
                            error.retry_after(),
                            completion_id,
                        );
                        if retry_fits_before_lease(delay, &lease.lease_expires_at) {
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    }
                    self.unconfirmed_finishes
                        .lock()
                        .await
                        .insert(claim.run.id, lease.lease_expires_at);
                    return Err(error);
                }
            }
        }
    }
}

#[derive(Debug)]
struct VersionedHarnessOutput {
    status: HarnessRunStatus,
    summary: String,
    failure_code: Option<String>,
    lease: RunLeaseState,
}

#[derive(Debug)]
struct RunLeaseState {
    version: i64,
    lease_expires_at: DateTime<Utc>,
}

#[derive(Debug)]
enum WorktreePreparation {
    Ready(RunLeaseState),
    Terminal(VersionedHarnessOutput),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HarnessRunStatus {
    Succeeded,
    Failed,
    Cancelled,
}

impl HarnessRunStatus {
    const fn as_api_value(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    const fn requires_failure_code(self) -> bool {
        !matches!(self, Self::Succeeded)
    }
}

impl VersionedHarnessOutput {
    fn new(output: HarnessOutput, lease: RunLeaseState) -> Self {
        Self {
            status: if output.succeeded {
                HarnessRunStatus::Succeeded
            } else {
                HarnessRunStatus::Failed
            },
            summary: output.summary,
            failure_code: output.failure_code,
            lease,
        }
    }

    fn failed(summary: &str, failure_code: &str, lease: RunLeaseState) -> Self {
        Self {
            status: HarnessRunStatus::Failed,
            summary: summary.to_string(),
            failure_code: Some(failure_code.to_string()),
            lease,
        }
    }

    fn cancelled(lease: RunLeaseState) -> Self {
        Self {
            status: HarnessRunStatus::Cancelled,
            summary: "The agent service shut down before Codex execution completed.".to_string(),
            failure_code: Some("service_shutdown".to_string()),
            lease,
        }
    }
}

impl RunLeaseState {
    fn from_claim(claim: &AgentClaimResponse) -> Self {
        Self {
            version: claim.run.version,
            lease_expires_at: claim.run.lease_expires_at,
        }
    }

    fn from_run(run: &sealtask_client_api::AgentRunResponse) -> Self {
        Self {
            version: run.version,
            lease_expires_at: run.lease_expires_at,
        }
    }
}

fn shutdown_requested(shutdown: &watch::Receiver<bool>) -> bool {
    *shutdown.borrow()
}

fn identity_poll_priority(
    identity: &AgentIdentity,
    cached_sessions: &HashSet<Uuid>,
) -> (u8, DateTime<Utc>, Uuid) {
    let priority = match identity.status {
        LocalAgentStatus::Active if cached_sessions.contains(&identity.agent_id) => 0,
        LocalAgentStatus::Active => 1,
        LocalAgentStatus::Pending => 2,
        LocalAgentStatus::Expired => 3,
        LocalAgentStatus::Revoked => 4,
    };
    (priority, identity.created_at, identity.agent_id)
}

fn rotate_uncached_auth_candidates(
    candidates: &mut [AgentIdentity],
    cached_sessions: &HashSet<Uuid>,
    cursor: Option<Uuid>,
) {
    let Some(cursor) = cursor else {
        return;
    };
    let Some(start) = candidates
        .iter()
        .position(|identity| identity_requires_uncached_auth(identity, cached_sessions))
    else {
        return;
    };
    let priority = identity_poll_priority(&candidates[start], cached_sessions).0;
    let count = candidates[start..]
        .iter()
        .take_while(|identity| {
            identity_requires_uncached_auth(identity, cached_sessions)
                && identity_poll_priority(identity, cached_sessions).0 == priority
        })
        .count();
    let auth_candidates = &mut candidates[start..start + count];
    if let Some(position) = auth_candidates
        .iter()
        .position(|identity| identity.agent_id == cursor)
    {
        let rotation = (position + 1) % auth_candidates.len();
        auth_candidates.rotate_left(rotation);
    }
}

fn identity_requires_uncached_auth(
    identity: &AgentIdentity,
    cached_sessions: &HashSet<Uuid>,
) -> bool {
    match identity.status {
        LocalAgentStatus::Active => !cached_sessions.contains(&identity.agent_id),
        LocalAgentStatus::Pending | LocalAgentStatus::Expired => true,
        LocalAgentStatus::Revoked => false,
    }
}

fn exponential_backoff(base: Duration, maximum: Duration, failures: u32) -> Duration {
    let multiplier = 1_u32 << failures.saturating_sub(1).min(16);
    base.saturating_mul(multiplier).min(maximum)
}

fn run_retry_delay(
    base: Duration,
    failures: u32,
    retry_after: Option<Duration>,
    retry_id: Uuid,
) -> Duration {
    let local_backoff = exponential_backoff(base, RUN_RETRY_MAX_DELAY, failures);
    let jitter_range = RUN_RETRY_JITTER_MAX.as_millis() + 1;
    let mixed_seed = retry_id.as_u128() ^ u128::from(failures).wrapping_mul(0x9e37_79b9);
    let jitter_millis = u64::try_from(mixed_seed % jitter_range).unwrap_or(0);
    local_backoff
        .saturating_add(Duration::from_millis(jitter_millis))
        .max(retry_after.unwrap_or_default())
}

fn identity_retry_delay(
    base: Duration,
    failures: u32,
    retry_after: Option<Duration>,
    agent_id: Uuid,
) -> Duration {
    let local_backoff = exponential_backoff(base, IDENTITY_AUTH_RETRY_MAX, failures);
    let jitter_range = IDENTITY_AUTH_RETRY_JITTER_MAX.as_millis() + 1;
    let mixed_seed = agent_id.as_u128() ^ u128::from(failures).wrapping_mul(0x517c_c1b7);
    let jitter_millis = u64::try_from(mixed_seed % jitter_range).unwrap_or(0);
    local_backoff
        .saturating_add(Duration::from_millis(jitter_millis))
        .min(IDENTITY_AUTH_RETRY_MAX)
        .max(retry_after.unwrap_or_default())
}

fn retry_fits_before_lease(delay: Duration, lease_expires_at: &DateTime<Utc>) -> bool {
    let required_time = delay.saturating_add(LEASE_RETRY_SAFETY_MARGIN);
    lease_expires_at
        .signed_duration_since(Utc::now())
        .to_std()
        .is_ok_and(|remaining| required_time < remaining)
}

fn is_retryable_heartbeat_error(error: &PublicError) -> bool {
    error.transport_failure_kind().is_some()
        || error.response_failure_kind().is_some()
        || matches!(error.http_status(), Some(408 | 429 | 500..=599))
        || matches!(error.code(), "request_timeout" | "rate_limited")
}

fn is_retryable_session_refresh_error(error: &PublicError) -> bool {
    error.transport_failure_kind().is_some()
        || error.response_failure_kind().is_some()
        || matches!(error.http_status(), Some(408 | 429 | 500..=599))
        || matches!(error.code(), "request_timeout" | "rate_limited")
}

fn is_retryable_finish_error(error: &PublicError) -> bool {
    error.transport_failure_kind().is_some()
        || error.response_failure_kind().is_some()
        || matches!(error.http_status(), Some(408 | 429 | 500..=599))
        || matches!(error.code(), "request_timeout" | "rate_limited")
}

async fn reconcile_pending_identity(
    identity: &AgentIdentity,
    keys: &AgentKeyMaterial,
) -> PublicResult<Option<(AgentIdentity, AgentSession)>> {
    let session = match mint_agent_session(identity, keys).await {
        Ok(session) => session,
        Err(error) if error.http_status() == Some(401) => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let me = session.client.me().await?;
    if me.id != identity.agent_id {
        return Err(PublicError::crypto(
            "approved agent principal did not match the local identity",
        ));
    }
    if me.work_list_id != identity.project.work_list_id
        || me.permission_preset != identity.project.permission_preset
    {
        return Err(PublicError::validation(
            "approved agent grant does not match the local project binding",
        ));
    }
    let handle = canonicalize_agent_handle(&me.handle)?;
    let display_name = canonicalize_agent_display_name(&me.display_name)?;
    if handle != me.handle || display_name != me.display_name {
        return Err(PublicError::crypto(
            "approved agent identity metadata was not canonical",
        ));
    }
    verify_grant_material(
        identity,
        keys,
        &handle,
        &display_name,
        &me.permission_preset,
        me.instructions_revision,
        &me.key_ciphertext,
        &me.instructions_ciphertext,
        &me.grant_signature,
    )?;
    let identity = activate_agent_identity(
        identity.agent_id,
        handle,
        display_name,
        me.work_list_id,
        me.instructions_revision,
    )?;
    Ok(Some((identity, session)))
}

async fn mint_agent_session(
    identity: &AgentIdentity,
    keys: &AgentKeyMaterial,
) -> PublicResult<AgentSession> {
    let options = ApiTransportOptions::default().with_request_id(Uuid::now_v7());
    let api = AgentApiClient::unauthenticated(&identity.api_url, options)?;
    let audience = canonicalize_agent_audience(&identity.api_url)?;
    let assertion = keys.build_token_mint_assertion(identity.agent_id, &audience)?;
    let request_started_at = Instant::now();
    let mut token = api.mint_token(assertion).await?;
    let response_received_at = Instant::now();
    if token.agent_id != identity.agent_id {
        return Err(PublicError::crypto(
            "agent token response did not match the local identity",
        ));
    }
    let (refresh_at, expires_at) = agent_session_deadlines(
        request_started_at,
        response_received_at,
        token.expires_in,
        identity.agent_id,
    )?;
    Ok(AgentSession {
        client: AgentApiClient::authenticated(
            &identity.api_url,
            std::mem::take(&mut token.access_token),
            options,
        )?,
        refresh_at,
        expires_at,
        refresh_failures: 0,
        refresh_retry_not_before: None,
    })
}

fn agent_session_deadlines(
    request_started_at: Instant,
    response_received_at: Instant,
    expires_in: u64,
    agent_id: Uuid,
) -> PublicResult<(Instant, Instant)> {
    let lifetime = Duration::from_secs(expires_in);
    if lifetime.is_zero() || lifetime > MAX_AGENT_TOKEN_LIFETIME {
        return Err(PublicError::validation(
            "agent token response contained an invalid lifetime",
        ));
    }
    let expires_at = request_started_at.checked_add(lifetime).ok_or_else(|| {
        PublicError::validation("agent token lifetime exceeded the local monotonic clock range")
    })?;
    if !token_is_outside_expiry_margin(expires_at, response_received_at) {
        return Err(PublicError::request_timeout(
            "agent token response arrived too close to expiry",
        ));
    }
    let jitter_seconds =
        u64::from(agent_id.as_bytes()[15]) % (TOKEN_REFRESH_JITTER_MAX.as_secs() + 1);
    let refresh_margin = TOKEN_REFRESH_MARGIN + Duration::from_secs(jitter_seconds);
    let refresh_after = lifetime.saturating_sub(refresh_margin);
    let refresh_at = request_started_at
        .checked_add(refresh_after)
        .ok_or_else(|| PublicError::validation("agent token refresh deadline overflowed"))?;
    Ok((refresh_at, expires_at))
}

async fn ensure_fresh_agent_session(
    session: &mut AgentSession,
    identity: &AgentIdentity,
    keys: &AgentKeyMaterial,
) -> PublicResult<()> {
    let now = Instant::now();
    if let Some(retry_not_before) = session.refresh_retry_not_before {
        if now < retry_not_before {
            if token_is_outside_expiry_margin(session.expires_at, now) {
                return Ok(());
            }
            return Err(PublicError::rate_limited_with_retry_after(
                "agent token refresh remains deferred by server backoff",
                retry_not_before.saturating_duration_since(now),
            ));
        }
        session.refresh_retry_not_before = None;
    }
    if now < session.refresh_at && token_is_outside_expiry_margin(session.expires_at, now) {
        return Ok(());
    }
    let refresh = mint_agent_session(identity, keys).await;
    let refresh_finished_at = Instant::now();
    match refresh {
        Ok(refreshed) => *session = refreshed,
        Err(error) if is_retryable_session_refresh_error(&error) => {
            session.refresh_failures = session.refresh_failures.saturating_add(1);
            let delay = exponential_backoff(
                TOKEN_REFRESH_RETRY_INITIAL,
                TOKEN_REFRESH_RETRY_MAX,
                session.refresh_failures,
            )
            .max(error.retry_after().unwrap_or_default());
            let retry_not_before = refresh_finished_at
                .checked_add(delay)
                .unwrap_or(session.expires_at);
            session.refresh_at = retry_not_before;
            session.refresh_retry_not_before = Some(retry_not_before);
            if !token_is_outside_expiry_margin(session.expires_at, refresh_finished_at) {
                return Err(PublicError::rate_limited_with_retry_after(
                    "agent token refresh failed inside its expiry safety margin",
                    delay,
                ));
            }
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

async fn prepare_identity_poll(
    identity: &AgentIdentity,
    shutdown: &mut watch::Receiver<bool>,
) -> PublicResult<Option<(AgentKeyMaterial, String)>> {
    supervise_repository_preflight(
        async {
            let keys = load_agent_key_material(identity.agent_id)?
                .ok_or_else(|| PublicError::not_found("local agent key material not found"))?;
            validate_repository_binding(identity).await?;
            let source_revision = repository_revision(&identity.project.repository_root).await?;
            Ok((keys, source_revision))
        },
        REPOSITORY_PREFLIGHT_TIMEOUT,
        shutdown,
    )
    .await
}

async fn supervise_repository_preflight<T>(
    preflight: impl Future<Output = PublicResult<T>>,
    timeout: Duration,
    shutdown: &mut watch::Receiver<bool>,
) -> PublicResult<Option<T>> {
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    tokio::pin!(preflight);
    loop {
        tokio::select! {
            result = &mut preflight => return result.map(Some),
            () = &mut deadline => {
                return Err(PublicError::request_timeout(
                    "agent repository preflight exceeded its local deadline",
                ));
            }
            changed = shutdown.changed() => {
                if changed.is_err() || shutdown_requested(shutdown) {
                    return Ok(None);
                }
            }
        }
    }
}

fn token_is_outside_expiry_margin(expires_at: Instant, now: Instant) -> bool {
    expires_at
        .checked_duration_since(now)
        .is_some_and(|remaining| remaining > TOKEN_EXPIRY_SAFETY_MARGIN)
}

fn prepare_claim(
    identity: &AgentIdentity,
    keys: &AgentKeyMaterial,
    claim: &AgentClaimResponse,
) -> PublicResult<PreparedClaim> {
    if claim.run.work_list_id != identity.project.work_list_id
        || claim.permission_preset != AGENT_PERMISSION_PRESET
        || claim.run.instructions_revision != identity.project.instructions_revision
    {
        return Err(PublicError::validation(
            "agent claim does not match the local project grant",
        ));
    }
    let handle = identity
        .handle
        .as_deref()
        .ok_or_else(|| PublicError::validation("active agent identity is missing its handle"))?;
    let display_name = identity.display_name.as_deref().ok_or_else(|| {
        PublicError::validation("active agent identity is missing its display name")
    })?;
    let (key_ciphertext, instructions_ciphertext) = verify_grant_material(
        identity,
        keys,
        handle,
        display_name,
        &claim.permission_preset,
        claim.run.instructions_revision,
        &claim.key_ciphertext,
        &claim.instructions_ciphertext,
        &claim.grant_signature,
    )?;
    let title_ciphertext = decode_standard_base64("task title", &claim.task_title_ciphertext)?;
    let payload_ciphertext =
        decode_standard_base64("task payload", &claim.task_payload_ciphertext)?;
    let project_key = decrypt_agent_project_key(
        keys.recipient_private_key(),
        identity.agent_id,
        identity.project.work_list_id,
        claim.run.instructions_revision,
        &key_ciphertext,
    )?;
    let instructions = decrypt_agent_instructions(
        keys.recipient_private_key(),
        identity.agent_id,
        identity.project.work_list_id,
        claim.run.instructions_revision,
        &instructions_ciphertext,
    )?;
    let instructions = std::str::from_utf8(&instructions)
        .map_err(|_| PublicError::crypto("agent instructions are not valid UTF-8"))?;
    let title = decrypt_encrypted_text_value(&title_ciphertext, &project_key, TASK_TITLE_CONTEXT)?;
    let payload = decrypt_task_payload(&project_key, &payload_ciphertext)?;
    let prompt = Zeroizing::new(build_prompt(
        identity,
        claim,
        instructions,
        &title,
        &payload,
    ));
    Ok(PreparedClaim {
        project_key,
        prompt,
    })
}

#[allow(clippy::too_many_arguments)]
fn verify_grant_material(
    identity: &AgentIdentity,
    keys: &AgentKeyMaterial,
    handle: &str,
    display_name: &str,
    permission_preset: &str,
    instructions_revision: i64,
    key_ciphertext: &str,
    instructions_ciphertext: &str,
    grant_signature: &str,
) -> PublicResult<(Vec<u8>, Vec<u8>)> {
    if canonicalize_agent_handle(handle)? != handle
        || canonicalize_agent_display_name(display_name)? != display_name
    {
        return Err(PublicError::crypto(
            "agent grant identity metadata was not canonical",
        ));
    }
    let key_ciphertext = decode_standard_base64("agent project key", key_ciphertext)?;
    let instructions_ciphertext =
        decode_standard_base64("agent instructions", instructions_ciphertext)?;
    let enrollment_code = keys.enrollment_code()?;
    verify_agent_grant(
        &enrollment_code,
        grant_signature,
        AgentGrantAuthenticationInput {
            agent_id: identity.agent_id,
            work_list_id: identity.project.work_list_id,
            handle,
            display_name,
            permission_preset,
            instructions_revision,
            auth_public_key: keys.auth_public_key(),
            recipient_public_key: keys.recipient_public_key(),
            key_ciphertext: &key_ciphertext,
            instructions_ciphertext: &instructions_ciphertext,
        },
    )?;
    Ok((key_ciphertext, instructions_ciphertext))
}

fn build_prompt(
    identity: &AgentIdentity,
    claim: &AgentClaimResponse,
    instructions: &str,
    title: &str,
    payload: &TaskPayloadEnvelope,
) -> String {
    let mut body = String::new();
    if let Some(rich_text) = payload.body.rich_text.as_ref() {
        for block in &rich_text.blocks {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(&block.text);
        }
    }
    let mut checklist = String::new();
    if let Some(items) = payload.body.checklist.as_ref() {
        for item in items {
            checklist.push_str(if item.is_done { "\n- [x] " } else { "\n- [ ] " });
            checklist.push_str(&item.title);
        }
    }
    format!(
        "You are the SealTask agent principal {agent_id}, running assignment {assignment_id} as run {run_id}.\n\nManaged role instructions (revision {instructions_revision}):\n{instructions}\n\nAssigned task:\nTitle: {title}\n\n{body}{checklist}\n\nImplement only this assigned task in the isolated git worktree. Follow repository AGENTS.md instructions, keep changes focused, run proportionate verification, and finish with a concise summary of changes and tests. Do not commit, publish, merge, push, or access unrelated SealTask tasks.",
        agent_id = identity.agent_id,
        assignment_id = claim.run.delegation_id,
        run_id = claim.run.id,
        instructions_revision = claim.run.instructions_revision,
    )
}

fn decode_standard_base64(field: &str, value: &str) -> PublicResult<Vec<u8>> {
    STANDARD_NO_PAD
        .decode(value.trim())
        .or_else(|_| STANDARD.decode(value.trim()))
        .map_err(|_| PublicError::crypto(format!("invalid {field} ciphertext encoding")))
}

fn run_paths(agent_id: Uuid, run_id: Uuid) -> PublicResult<RunPaths> {
    let run_directory = config_dir()?
        .join("agent-runs")
        .join(agent_id.to_string())
        .join(run_id.to_string());
    Ok(RunPaths {
        worktree: run_directory.join("worktree"),
        run_directory,
    })
}

async fn validate_repository_binding(identity: &AgentIdentity) -> PublicResult<()> {
    let repository = identity
        .project
        .repository_root
        .canonicalize()
        .map_err(|error| {
            PublicError::validation(format!("failed to resolve bound repository: {error}"))
        })?;
    if repository != identity.project.repository_root {
        return Err(PublicError::validation(
            "bound agent repository path is no longer canonical",
        ));
    }
    let output = git_output(&repository, ["rev-parse", "--show-toplevel"]).await?;
    let top = PathBuf::from(output.trim())
        .canonicalize()
        .map_err(|error| {
            PublicError::validation(format!("failed to resolve git repository root: {error}"))
        })?;
    if top != repository {
        return Err(PublicError::validation(
            "bound agent repository must be the git worktree root",
        ));
    }
    Ok(())
}

async fn repository_revision(repository: &Path) -> PublicResult<String> {
    let object_format = git_output(repository, ["rev-parse", "--show-object-format"]).await?;
    let expected_length = git_object_id_length(object_format.trim())?;
    let revision = git_output(repository, ["rev-parse", "--verify", "HEAD"]).await?;
    let revision = revision.trim();
    if revision.len() != expected_length || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PublicError::unexpected(
            "git returned an invalid source revision",
        ));
    }
    Ok(revision.to_string())
}

async fn git_output<const N: usize>(repository: &Path, args: [&str; N]) -> PublicResult<String> {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("core.hooksPath=/dev/null")
        .arg("-C")
        .arg(repository)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    apply_git_environment(&mut command);
    command.kill_on_drop(true);
    let process_tree_configuration = configure_process_tree(&mut command).await?;
    let mut child = command.spawn().map_err(|error| {
        PublicError::unexpected(format!("failed to run git for agent repository: {error}"))
    })?;
    let mut process_tree = ProcessTreeGuard::new(&child, process_tree_configuration)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PublicError::unexpected("agent repository git stdout was unavailable"))?;
    let stdout_task = tokio::spawn(drain_git_output(stdout));
    resume_process_tree(&child)?;
    let status = wait_for_process_tree(&mut child, &mut process_tree)
        .await
        .map_err(|error| {
            PublicError::unexpected(format!("failed to wait for agent repository git: {error}"))
        })?;
    let output = stdout_task.await.map_err(|error| {
        PublicError::unexpected(format!("agent repository git output task failed: {error}"))
    })??;
    if !status.success() || output.len() > MAX_GIT_OUTPUT_BYTES {
        return Err(PublicError::validation(
            "bound agent repository is not a usable git worktree",
        ));
    }
    String::from_utf8(output)
        .map_err(|_| PublicError::unexpected("git returned non-UTF-8 repository metadata"))
}

async fn drain_git_output(mut output: impl AsyncRead + Unpin) -> PublicResult<Vec<u8>> {
    let mut captured = Vec::with_capacity(MAX_GIT_OUTPUT_BYTES + 1);
    let mut buffer = [0_u8; 4096];
    loop {
        let count = output.read(&mut buffer).await.map_err(|error| {
            PublicError::unexpected(format!(
                "failed to read agent repository git output: {error}"
            ))
        })?;
        if count == 0 {
            return Ok(captured);
        }
        let remaining = (MAX_GIT_OUTPUT_BYTES + 1).saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..count.min(remaining)]);
    }
}

async fn create_run_worktree(
    repository: &Path,
    source_revision: &str,
    paths: &RunPaths,
) -> PublicResult<()> {
    create_private_directory(&paths.run_directory).await?;
    if tokio::fs::try_exists(&paths.worktree)
        .await
        .map_err(|error| PublicError::unexpected(format!("failed to inspect worktree: {error}")))?
    {
        return Err(PublicError::conflict("agent run worktree already exists"));
    }
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("core.hooksPath=/dev/null")
        .arg("-C")
        .arg(repository)
        .arg("worktree")
        .arg("add")
        .arg("--detach")
        .arg(&paths.worktree)
        .arg(source_revision)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    apply_git_environment(&mut command);
    let process_tree_configuration = configure_process_tree(&mut command).await?;
    let mut child = command.spawn().map_err(|error| {
        PublicError::unexpected(format!("failed to create agent run worktree: {error}"))
    })?;
    let mut process_tree = ProcessTreeGuard::new(&child, process_tree_configuration)?;
    resume_process_tree(&child)?;
    let status = wait_for_process_tree(&mut child, &mut process_tree)
        .await
        .map_err(|error| {
            PublicError::unexpected(format!("failed to wait for agent worktree setup: {error}"))
        })?;
    if !status.success() {
        return Err(PublicError::unexpected(
            "git could not create the isolated agent run worktree",
        ));
    }
    Ok(())
}

fn git_object_id_length(object_format: &str) -> PublicResult<usize> {
    match object_format {
        "sha1" => Ok(40),
        "sha256" => Ok(64),
        _ => Err(PublicError::validation(format!(
            "git returned an unsupported object format: {object_format}"
        ))),
    }
}

async fn create_private_directory(path: &Path) -> PublicResult<()> {
    tokio::fs::create_dir_all(path).await.map_err(|error| {
        PublicError::unexpected(format!("failed to create agent run directory: {error}"))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(|error| {
                PublicError::unexpected(format!("failed to secure agent run directory: {error}"))
            })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use sealtask_client_auth::{
        AgentProjectBinding, SavePendingAgentIdentity, agent_key_material_from_seed,
        configure_local_state, save_pending_agent_identity,
    };
    use sealtask_client_crypto::{
        AgentGrantAuthenticationInput, TaskPayloadBody, build_task_payload_envelope,
        encrypt_agent_instructions, encrypt_agent_project_key, encrypt_task_payload,
        encrypt_text_value, sign_agent_grant,
    };
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    use super::*;

    struct FakeHarness {
        prompt: Mutex<Option<String>>,
    }

    impl Harness for FakeHarness {
        fn run<'a>(
            &'a self,
            _worktree: &'a Path,
            _run_directory: &'a Path,
            prompt: &'a str,
        ) -> Pin<Box<dyn Future<Output = PublicResult<HarnessOutput>> + Send + 'a>> {
            *self.prompt.lock().expect("prompt lock") = Some(prompt.to_string());
            Box::pin(async {
                Ok(HarnessOutput {
                    succeeded: true,
                    summary: "done".to_string(),
                    failure_code: None,
                })
            })
        }
    }

    #[tokio::test]
    async fn fake_harness_receives_only_the_composed_managed_task_prompt() {
        let harness = FakeHarness {
            prompt: Mutex::new(None),
        };
        let output = harness
            .run(Path::new("."), Path::new("."), "managed task")
            .await
            .expect("fake harness result");
        assert!(output.succeeded);
        assert_eq!(
            harness.prompt.lock().expect("prompt lock").as_deref(),
            Some("managed task")
        );
    }

    #[tokio::test]
    async fn invalid_active_identity_does_not_starve_pending_reconciliation() {
        let temporary = tempfile::tempdir().expect("identity scheduler fixture");
        configure_local_state(Some(temporary.path().join("config")), None)
            .expect("configure isolated agent state");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind pending reconciliation server");
        let api_url = format!(
            "http://{}",
            listener
                .local_addr()
                .expect("pending reconciliation server address")
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .expect("pending identity reaches reconciliation")
                .expect("accept pending token mint");
            let mut request = [0_u8; 8192];
            let bytes_read = stream.read(&mut request).await.expect("read token mint");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("POST /auth/agents/token ")
            );
            stream
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                )
                .await
                .expect("write pending reconciliation response");
        });

        let invalid_repository = temporary.path().join("invalid-active");
        std::fs::create_dir(&invalid_repository).expect("create invalid active repository");
        let active_keys = agent_key_material_from_seed([0xa1; 32]).expect("active agent keys");
        let active_agent_id = Uuid::from_u128(1);
        let active_auth_public_key = STANDARD_NO_PAD.encode(active_keys.auth_public_key());
        let active_recipient_public_key =
            STANDARD_NO_PAD.encode(active_keys.recipient_public_key());
        let active_fingerprint = active_keys.fingerprint();
        save_pending_agent_identity(
            SavePendingAgentIdentity {
                agent_id: active_agent_id,
                api_url: &api_url,
                proposed_handle: Some("stale-agent".to_string()),
                auth_public_key: &active_auth_public_key,
                recipient_public_key: &active_recipient_public_key,
                fingerprint: &active_fingerprint,
                enrollment_expires_at: Utc::now() + chrono::Duration::minutes(5),
                work_list_id: Uuid::now_v7(),
                repository_root: &invalid_repository,
            },
            &active_keys,
        )
        .expect("save invalid active identity");
        let active = load_agent_identity(active_agent_id)
            .expect("load active identity")
            .expect("active identity exists");
        activate_agent_identity(
            active_agent_id,
            "stale-agent".to_string(),
            "Stale Agent".to_string(),
            active.project.work_list_id,
            1,
        )
        .expect("activate invalid identity");

        let pending_repository = temporary.path().join("pending-repository");
        initialize_git_repository(&pending_repository).await;
        let pending_keys = agent_key_material_from_seed([0xa2; 32]).expect("pending agent keys");
        let pending_agent_id = Uuid::from_u128(2);
        let pending_auth_public_key = STANDARD_NO_PAD.encode(pending_keys.auth_public_key());
        let pending_recipient_public_key =
            STANDARD_NO_PAD.encode(pending_keys.recipient_public_key());
        let pending_fingerprint = pending_keys.fingerprint();
        save_pending_agent_identity(
            SavePendingAgentIdentity {
                agent_id: pending_agent_id,
                api_url: &api_url,
                proposed_handle: Some("pending-agent".to_string()),
                auth_public_key: &pending_auth_public_key,
                recipient_public_key: &pending_recipient_public_key,
                fingerprint: &pending_fingerprint,
                enrollment_expires_at: Utc::now() + chrono::Duration::minutes(5),
                work_list_id: Uuid::now_v7(),
                repository_root: &pending_repository,
            },
            &pending_keys,
        )
        .expect("save pending identity");

        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let harness = FakeHarness {
            prompt: Mutex::new(None),
        };
        let (_shutdown_sender, mut shutdown) = watch::channel(false);
        let outcome = service
            .poll_once(&harness, &mut shutdown)
            .await
            .expect("poll identities");

        server.await.expect("pending reconciliation server");
        assert!(
            outcome
                .failures
                .iter()
                .any(|failure| failure.agent_id == active_agent_id)
        );
        assert_eq!(
            *service.last_uncached_auth_attempt.lock().await,
            Some(pending_agent_id),
            "the failed local preflight must not consume the shared auth slot"
        );
    }

    #[tokio::test]
    async fn unconfirmed_finish_health_ages_out_after_the_lease_deadline() {
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let expired_run_id = Uuid::now_v7();
        let active_run_id = Uuid::now_v7();
        let now = Utc::now();
        {
            let mut finishes = service.unconfirmed_finishes.lock().await;
            finishes.insert(expired_run_id, now - chrono::Duration::seconds(1));
            finishes.insert(active_run_id, now + chrono::Duration::seconds(30));
        }

        assert_eq!(service.active_unconfirmed_terminal_run_count(now).await, 1);
        let finishes = service.unconfirmed_finishes.lock().await;
        assert!(!finishes.contains_key(&expired_run_id));
        assert!(finishes.contains_key(&active_run_id));
    }

    #[tokio::test]
    async fn slow_worktree_setup_is_heartbeated_before_during_and_after_setup() {
        let run_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let (api_url, heartbeat_count, server) = spawn_heartbeat_server(run_id).await;
        let keys = agent_key_material_from_seed([0x91; 32]).expect("agent keys");
        let identity = AgentIdentity {
            agent_id: Uuid::now_v7(),
            api_url: api_url.clone(),
            status: LocalAgentStatus::Active,
            proposed_handle: None,
            handle: Some("implementer".to_string()),
            display_name: Some("Implementation Agent".to_string()),
            fingerprint: keys.fingerprint(),
            auth_public_key: STANDARD_NO_PAD.encode(keys.auth_public_key()),
            recipient_public_key: STANDARD_NO_PAD.encode(keys.recipient_public_key()),
            enrollment_expires_at: None,
            project: AgentProjectBinding {
                work_list_id,
                repository_root: PathBuf::from("/tmp/project"),
                permission_preset: AGENT_PERMISSION_PRESET.to_string(),
                instructions_revision: 1,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut claim = claim_fixture(work_list_id, run_id);
        claim.run.lease_expires_at = Utc::now() + chrono::Duration::seconds(90);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "test-agent-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() + Duration::from_secs(300),
            expires_at: Instant::now() + Duration::from_secs(600),
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let (_shutdown_sender, mut shutdown) = watch::channel(false);
        let heartbeats_before_setup = Arc::new(AtomicUsize::new(0));
        let observed_before_setup = Arc::clone(&heartbeats_before_setup);
        let setup_heartbeat_count = Arc::clone(&heartbeat_count);
        let prepared = service
            .supervise_worktree_setup(
                &identity,
                &keys,
                &mut session,
                &claim,
                "lease-token",
                RunLeaseState::from_claim(&claim),
                tokio::time::Instant::now() + Duration::from_secs(5),
                &mut shutdown,
                async move {
                    observed_before_setup.store(
                        setup_heartbeat_count.load(Ordering::SeqCst),
                        Ordering::SeqCst,
                    );
                    tokio::time::sleep(Duration::from_millis(125)).await;
                    Ok(())
                },
                Duration::from_millis(25),
            )
            .await;

        let WorktreePreparation::Ready(lease) = prepared else {
            panic!("slow setup should retain its lease");
        };
        assert_eq!(heartbeats_before_setup.load(Ordering::SeqCst), 1);
        assert!(
            heartbeat_count.load(Ordering::SeqCst) >= 6,
            "setup should receive periodic heartbeats plus a post-setup confirmation"
        );
        assert_eq!(
            lease.version,
            i64::try_from(heartbeat_count.load(Ordering::SeqCst)).expect("heartbeat count")
        );
        server.abort();
    }

    #[tokio::test]
    async fn worktree_setup_stops_at_the_overall_run_deadline() {
        let run_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let (api_url, heartbeat_count, server) = spawn_heartbeat_server(run_id).await;
        let keys = agent_key_material_from_seed([0x93; 32]).expect("agent keys");
        let identity = active_identity_fixture(&api_url, work_list_id, &keys);
        let mut claim = claim_fixture(work_list_id, run_id);
        claim.run.lease_expires_at = Utc::now() + chrono::Duration::seconds(90);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "test-agent-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() + Duration::from_secs(300),
            expires_at: Instant::now() + Duration::from_secs(600),
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let (_shutdown_sender, mut shutdown) = watch::channel(false);
        let started_at = tokio::time::Instant::now();
        let prepared = service
            .supervise_worktree_setup(
                &identity,
                &keys,
                &mut session,
                &claim,
                "lease-token",
                RunLeaseState::from_claim(&claim),
                started_at + Duration::from_millis(100),
                &mut shutdown,
                std::future::pending::<PublicResult<()>>(),
                Duration::from_secs(1),
            )
            .await;

        let WorktreePreparation::Terminal(output) = prepared else {
            panic!("hung setup must terminate at the run deadline");
        };
        assert_eq!(output.status, HarnessRunStatus::Failed);
        assert_eq!(
            output.failure_code.as_deref(),
            Some("worktree_setup_timeout")
        );
        assert!(started_at.elapsed() < Duration::from_secs(1));
        assert_eq!(heartbeat_count.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn retryable_preemptive_refresh_honors_retry_after_while_token_is_still_valid() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind refresh server");
        let api_url = format!(
            "http://{}",
            listener.local_addr().expect("refresh server address")
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept token mint");
            let mut request = [0_u8; 8192];
            let bytes_read = stream.read(&mut request).await.expect("read token mint");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("POST /auth/agents/token ")
            );
            stream
                .write_all(
                    b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 20\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                )
                .await
                .expect("write refresh response");
            tokio::time::timeout(Duration::from_millis(200), listener.accept())
                .await
                .is_ok()
        });
        let keys = agent_key_material_from_seed([0x94; 32]).expect("agent keys");
        let identity = active_identity_fixture(&api_url, Uuid::now_v7(), &keys);
        let expires_at = Instant::now() + Duration::from_secs(10);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "still-valid-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() - Duration::from_secs(1),
            expires_at,
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let refresh_started_at = Instant::now();

        ensure_fresh_agent_session(&mut session, &identity, &keys)
            .await
            .expect("old token remains usable after retryable refresh failure");

        assert_eq!(session.refresh_failures, 1);
        assert_eq!(session.expires_at, expires_at);
        assert!(
            session.refresh_at >= refresh_started_at + Duration::from_secs(20),
            "server Retry-After must remain the earliest legal refresh time"
        );
        assert!(session.refresh_at > expires_at);
        assert_eq!(session.refresh_retry_not_before, Some(session.refresh_at));

        ensure_fresh_agent_session(&mut session, &identity, &keys)
            .await
            .expect("a second poll reuses the old token during server backoff");
        assert!(
            !server.await.expect("refresh server task"),
            "refresh retried before Retry-After elapsed"
        );
    }

    #[tokio::test]
    async fn refresh_backoff_fails_cleanly_inside_the_token_expiry_margin() {
        let keys = agent_key_material_from_seed([0x99; 32]).expect("agent keys");
        let identity = active_identity_fixture("http://127.0.0.1:9", Uuid::now_v7(), &keys);
        let retry_not_before = Instant::now() + Duration::from_secs(20);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &identity.api_url,
                "nearly-expired-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: retry_not_before,
            expires_at: Instant::now() + TOKEN_EXPIRY_SAFETY_MARGIN,
            refresh_failures: 1,
            refresh_retry_not_before: Some(retry_not_before),
        };

        let error = ensure_fresh_agent_session(&mut session, &identity, &keys)
            .await
            .expect_err("refresh must stop using a token inside its safety margin");

        assert_eq!(error.code(), "rate_limited");
        assert!(
            error
                .retry_after()
                .is_some_and(|delay| delay > Duration::from_secs(10))
        );
    }

    #[test]
    fn token_expiry_margin_forces_refresh_at_and_inside_the_boundary() {
        let now = Instant::now();

        assert!(token_is_outside_expiry_margin(
            now + TOKEN_EXPIRY_SAFETY_MARGIN + Duration::from_millis(1),
            now,
        ));
        assert!(!token_is_outside_expiry_margin(
            now + TOKEN_EXPIRY_SAFETY_MARGIN,
            now,
        ));
        assert!(!token_is_outside_expiry_margin(
            now + TOKEN_EXPIRY_SAFETY_MARGIN - Duration::from_millis(1),
            now,
        ));
    }

    #[test]
    fn token_deadlines_include_the_mint_request_round_trip() {
        let request_started_at = Instant::now();
        let response_received_at = request_started_at + Duration::from_secs(10);

        let (refresh_at, expires_at) = agent_session_deadlines(
            request_started_at,
            response_received_at,
            60,
            Uuid::from_u128(7),
        )
        .expect("token with sufficient remaining lifetime");

        assert_eq!(expires_at, request_started_at + Duration::from_secs(60));
        assert_eq!(expires_at - response_received_at, Duration::from_secs(50));
        assert!(refresh_at < expires_at);
    }

    #[test]
    fn token_response_inside_the_expiry_margin_is_rejected() {
        let request_started_at = Instant::now();
        let response_received_at = request_started_at + Duration::from_secs(55);

        let error = agent_session_deadlines(
            request_started_at,
            response_received_at,
            60,
            Uuid::from_u128(8),
        )
        .expect_err("near-expired token response must be rejected");

        assert_eq!(error.code(), "request_timeout");
    }

    #[tokio::test]
    async fn repository_preflight_stops_at_its_local_deadline() {
        let (_shutdown_sender, mut shutdown) = watch::channel(false);
        let started_at = Instant::now();

        let error = supervise_repository_preflight(
            std::future::pending::<PublicResult<()>>(),
            Duration::from_millis(25),
            &mut shutdown,
        )
        .await
        .expect_err("hung repository preflight must time out");

        assert_eq!(error.code(), "request_timeout");
        assert!(started_at.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn repository_preflight_stops_when_the_service_shuts_down() {
        let (shutdown_sender, mut shutdown) = watch::channel(false);
        let signal = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(25)).await;
            shutdown_sender.send(true).expect("send shutdown signal");
        });
        let started_at = Instant::now();

        let result = supervise_repository_preflight(
            std::future::pending::<PublicResult<()>>(),
            Duration::from_secs(5),
            &mut shutdown,
        )
        .await
        .expect("shutdown is not a preflight failure");

        assert!(result.is_none());
        assert!(started_at.elapsed() < Duration::from_secs(1));
        signal.await.expect("shutdown task");
    }

    #[test]
    fn identity_backoff_prioritizes_active_agents_and_rotates_within_status() {
        assert_eq!(
            exponential_backoff(Duration::from_secs(60), Duration::from_secs(600), 1),
            Duration::from_secs(60)
        );
        assert_eq!(
            exponential_backoff(Duration::from_secs(60), Duration::from_secs(600), 5),
            Duration::from_secs(600)
        );
        assert!(
            identity_retry_delay(
                Duration::from_secs(60),
                1,
                Some(Duration::from_secs(90)),
                Uuid::from_u128(1),
            ) >= Duration::from_secs(90)
        );

        let keys = agent_key_material_from_seed([0x95; 32]).expect("agent keys");
        let mut cached = active_identity_fixture("https://api.example.test", Uuid::now_v7(), &keys);
        cached.agent_id = Uuid::from_u128(1);
        let mut stale_a = cached.clone();
        stale_a.agent_id = Uuid::from_u128(2);
        let mut stale_b = cached.clone();
        stale_b.agent_id = Uuid::from_u128(3);
        let mut pending = cached.clone();
        pending.agent_id = Uuid::from_u128(4);
        pending.status = LocalAgentStatus::Pending;
        let mut expired = cached.clone();
        expired.agent_id = Uuid::from_u128(5);
        expired.status = LocalAgentStatus::Expired;
        let mut revoked = cached.clone();
        revoked.agent_id = Uuid::from_u128(6);
        revoked.status = LocalAgentStatus::Revoked;
        let sessions = HashSet::from([cached.agent_id]);
        let mut candidates = vec![expired, pending, stale_b, revoked, cached, stale_a];
        candidates.sort_by_key(|identity| identity_poll_priority(identity, &sessions));
        rotate_uncached_auth_candidates(&mut candidates, &sessions, Some(Uuid::from_u128(2)));
        assert_eq!(
            candidates
                .iter()
                .map(|identity| identity.agent_id)
                .collect::<Vec<_>>(),
            vec![
                Uuid::from_u128(1),
                Uuid::from_u128(3),
                Uuid::from_u128(2),
                Uuid::from_u128(4),
                Uuid::from_u128(5),
                Uuid::from_u128(6),
            ],
            "healthy identities precede enrollment recovery and rotation stays in its bucket"
        );
    }

    #[test]
    fn run_retry_delay_honors_server_backoff_and_stops_before_lease_expiry() {
        let retry_after = Duration::from_secs(17);
        let delay = run_retry_delay(
            Duration::from_secs(1),
            1,
            Some(retry_after),
            Uuid::from_u128(7),
        );

        assert_eq!(delay, retry_after);
        assert!(retry_fits_before_lease(
            delay,
            &(Utc::now() + chrono::Duration::seconds(30)),
        ));
        assert!(!retry_fits_before_lease(
            delay,
            &(Utc::now() + chrono::Duration::seconds(10)),
        ));
    }

    #[tokio::test]
    async fn heartbeat_retry_honors_retry_after_and_reuses_its_payload() {
        let run_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let (api_url, requests, server) = spawn_run_retry_after_server(run_id, "running").await;
        let keys = agent_key_material_from_seed([0x96; 32]).expect("agent keys");
        let identity = active_identity_fixture(&api_url, work_list_id, &keys);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "test-agent-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() + Duration::from_secs(300),
            expires_at: Instant::now() + Duration::from_secs(600),
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let lease = RunLeaseState {
            version: 4,
            lease_expires_at: Utc::now() + chrono::Duration::seconds(30),
        };
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let started_at = Instant::now();

        let run = service
            .heartbeat_with_retry(
                &identity,
                &keys,
                &mut session,
                run_id,
                "lease-token",
                &lease,
            )
            .await
            .expect("retry rate-limited heartbeat");

        assert!(started_at.elapsed() >= Duration::from_secs(2));
        assert_eq!(run.version, 5);
        server.await.expect("heartbeat retry server");
        let requests = requests.lock().expect("heartbeat requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], requests[1]);
        assert!(requests[0]["heartbeatId"].as_str().is_some());
    }

    #[tokio::test]
    async fn finish_retry_honors_retry_after_and_reuses_its_payload() {
        let run_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let (api_url, requests, server) = spawn_run_retry_after_server(run_id, "succeeded").await;
        let keys = agent_key_material_from_seed([0x97; 32]).expect("agent keys");
        let identity = active_identity_fixture(&api_url, work_list_id, &keys);
        let mut claim = claim_fixture(work_list_id, run_id);
        claim.run.version = 4;
        claim.run.lease_expires_at = Utc::now() + chrono::Duration::seconds(30);
        let lease = RunLeaseState::from_claim(&claim);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "test-agent-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() + Duration::from_secs(300),
            expires_at: Instant::now() + Duration::from_secs(600),
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let started_at = Instant::now();

        service
            .finish_run(
                &identity,
                &keys,
                &mut session,
                &claim,
                "lease-token",
                &lease,
                "succeeded",
                Some("encrypted-result"),
                None,
            )
            .await
            .expect("retry rate-limited finish");

        assert!(started_at.elapsed() >= Duration::from_secs(2));
        server.await.expect("finish retry-after server");
        let requests = requests.lock().expect("finish requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], requests[1]);
        assert!(requests[0]["completionId"].as_str().is_some());
    }

    #[test]
    fn git_object_formats_select_the_correct_revision_length() {
        assert_eq!(git_object_id_length("sha1").unwrap(), 40);
        assert_eq!(git_object_id_length("sha256").unwrap(), 64);
        assert!(git_object_id_length("unknown").is_err());
    }

    async fn run_git_fixture_command(mut command: Command, context: &str) {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        apply_git_environment(&mut command);
        let configuration = configure_process_tree(&mut command)
            .await
            .expect("configure Git fixture process tree");
        let mut child = command.spawn().expect(context);
        let mut process_tree =
            ProcessTreeGuard::new(&child, configuration).expect("own Git fixture process tree");
        resume_process_tree(&child).expect("resume Git fixture process tree");
        let status = wait_for_process_tree(&mut child, &mut process_tree)
            .await
            .expect(context);
        assert!(status.success(), "{context}");
    }

    async fn initialize_git_repository(repository: &Path) {
        let mut command = Command::new("git");
        command.arg("init").arg(repository);
        run_git_fixture_command(command, "initialize Git fixture").await;

        let mut command = Command::new("git");
        command.arg("-C").arg(repository).args([
            "-c",
            "user.name=SealTask Test",
            "-c",
            "user.email=agent@example.test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ]);
        run_git_fixture_command(command, "commit Git fixture").await;
    }

    #[tokio::test]
    async fn repository_revision_accepts_sha1_and_sha256_repositories() {
        let temporary = tempfile::tempdir().expect("git object-format fixture");
        for (object_format, expected_length) in [("sha1", 40), ("sha256", 64)] {
            let repository = temporary.path().join(object_format);
            let mut command = Command::new("git");
            command
                .arg("init")
                .arg(format!("--object-format={object_format}"))
                .arg(&repository);
            run_git_fixture_command(command, "initialize object-format fixture").await;

            let mut command = Command::new("git");
            command.arg("-C").arg(&repository).args([
                "-c",
                "user.name=SealTask Test",
                "-c",
                "user.email=agent@example.test",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ]);
            run_git_fixture_command(command, "commit object-format fixture").await;

            let revision = repository_revision(&repository)
                .await
                .expect("read repository revision");
            assert_eq!(revision.len(), expected_length);
            assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }

    #[tokio::test]
    async fn lost_finish_response_retries_the_same_completion_id_and_payload() {
        let run_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let (api_url, requests, server) = spawn_finish_retry_server(run_id).await;
        let keys = agent_key_material_from_seed([0x92; 32]).expect("agent keys");
        let identity = AgentIdentity {
            agent_id: Uuid::now_v7(),
            api_url: api_url.clone(),
            status: LocalAgentStatus::Active,
            proposed_handle: None,
            handle: Some("implementer".to_string()),
            display_name: Some("Implementation Agent".to_string()),
            fingerprint: keys.fingerprint(),
            auth_public_key: STANDARD_NO_PAD.encode(keys.auth_public_key()),
            recipient_public_key: STANDARD_NO_PAD.encode(keys.recipient_public_key()),
            enrollment_expires_at: None,
            project: AgentProjectBinding {
                work_list_id,
                repository_root: PathBuf::from("/tmp/project"),
                permission_preset: AGENT_PERMISSION_PRESET.to_string(),
                instructions_revision: 1,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut claim = claim_fixture(work_list_id, run_id);
        claim.run.version = 4;
        claim.run.lease_expires_at = Utc::now() + chrono::Duration::seconds(90);
        let lease = RunLeaseState::from_claim(&claim);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "test-agent-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() + Duration::from_secs(300),
            expires_at: Instant::now() + Duration::from_secs(600),
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        service
            .finish_run(
                &identity,
                &keys,
                &mut session,
                &claim,
                "lease-token",
                &lease,
                "succeeded",
                Some("encrypted-result"),
                None,
            )
            .await
            .expect("reconcile lost finish response");
        server.await.expect("finish retry server");

        let requests = requests.lock().expect("finish requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], requests[1]);
        assert!(requests[0]["completionId"].as_str().is_some());
        assert_eq!(requests[0]["expectedVersion"], 4);
        assert_eq!(requests[0]["resultCiphertext"], "encrypted-result");
    }

    #[test]
    fn prompt_keeps_identity_role_and_assignment_distinct_from_the_user() {
        let agent_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let identity = AgentIdentity {
            agent_id,
            api_url: "https://sealtask.example".to_string(),
            status: LocalAgentStatus::Active,
            proposed_handle: None,
            handle: Some("implementer".to_string()),
            display_name: Some("Implementation Agent".to_string()),
            fingerprint: "fingerprint".to_string(),
            auth_public_key: "auth".to_string(),
            recipient_public_key: "recipient".to_string(),
            enrollment_expires_at: None,
            project: AgentProjectBinding {
                work_list_id,
                repository_root: PathBuf::from("/tmp/project"),
                permission_preset: AGENT_PERMISSION_PRESET.to_string(),
                instructions_revision: 1,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let run_id = Uuid::now_v7();
        let claim = claim_fixture(work_list_id, run_id);
        let payload = build_task_payload_envelope(
            TaskPayloadBody {
                title: "Task".to_string(),
                rich_text: None,
                checklist: None,
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
            },
            1,
        );
        let prompt = build_prompt(
            &identity,
            &claim,
            "Implement, do not review.",
            "Task",
            &payload,
        );
        assert!(prompt.contains(&format!("agent principal {agent_id}")));
        assert!(prompt.contains("Managed role instructions"));
        assert!(prompt.contains("Assigned task"));
        assert!(prompt.contains(&run_id.to_string()));
        assert!(!prompt.contains("implementer"));
        assert!(!prompt.contains("Implementation Agent"));
        assert!(!prompt.contains("acting as the user"));
    }

    #[tokio::test]
    async fn rejected_grant_completion_does_not_report_an_uncreated_worktree() {
        let run_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind grant rejection server");
        let api_url = format!(
            "http://{}",
            listener.local_addr().expect("grant rejection address")
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept failed completion");
            let body = read_test_json_request(&mut stream).await;
            assert_eq!(body["status"], "failed");
            assert_eq!(body["failureCode"], "grant_authentication");
            write_test_agent_run_response(&mut stream, run_id, 1, "failed").await;
        });
        let keys = agent_key_material_from_seed([0x80; 32]).expect("agent keys");
        let identity = active_identity_fixture(&api_url, work_list_id, &keys);
        let mut claim = claim_fixture(work_list_id, run_id);
        claim.run.lease_expires_at = Utc::now() + chrono::Duration::seconds(90);
        let mut session = AgentSession {
            client: AgentApiClient::authenticated(
                &api_url,
                "test-agent-token".to_string(),
                ApiTransportOptions::default(),
            )
            .expect("agent API client"),
            refresh_at: Instant::now() + Duration::from_secs(300),
            expires_at: Instant::now() + Duration::from_secs(600),
            refresh_failures: 0,
            refresh_retry_not_before: None,
        };
        let service = AgentService::new(Uuid::now_v7(), None, Duration::from_secs(60));
        let harness = FakeHarness {
            prompt: Mutex::new(None),
        };
        let (_shutdown_sender, mut shutdown) = watch::channel(false);

        let completed = service
            .execute_claim(
                &identity,
                &keys,
                &mut session,
                claim,
                "unused-revision",
                &harness,
                &mut shutdown,
            )
            .await
            .expect("record rejected grant completion");

        server.await.expect("grant rejection server");
        assert!(completed.status.starts_with("failed"));
        assert!(completed.worktree.is_none());
        assert!(harness.prompt.lock().expect("prompt lock").is_none());
    }

    #[test]
    fn backend_forged_grant_is_rejected_before_a_prompt_is_prepared() {
        let keys = agent_key_material_from_seed([0x81; 32]).expect("agent keys");
        let agent_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let identity = AgentIdentity {
            agent_id,
            api_url: "https://sealtask.example".to_string(),
            status: LocalAgentStatus::Active,
            proposed_handle: None,
            handle: Some("implementer".to_string()),
            display_name: Some("Implementation Agent".to_string()),
            fingerprint: keys.fingerprint(),
            auth_public_key: STANDARD_NO_PAD.encode(keys.auth_public_key()),
            recipient_public_key: STANDARD_NO_PAD.encode(keys.recipient_public_key()),
            enrollment_expires_at: None,
            project: AgentProjectBinding {
                work_list_id,
                repository_root: PathBuf::from("/tmp/project"),
                permission_preset: AGENT_PERMISSION_PRESET.to_string(),
                instructions_revision: 1,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let project_key = SymmetricKey::new([0x82; 32]);
        let key_ciphertext = encrypt_agent_project_key(
            keys.recipient_public_key(),
            agent_id,
            work_list_id,
            1,
            &project_key,
        )
        .expect("encrypt project key");
        let instructions_ciphertext = encrypt_agent_instructions(
            keys.recipient_public_key(),
            agent_id,
            work_list_id,
            1,
            b"Implement the assigned task.",
        )
        .expect("encrypt instructions");
        let title_ciphertext =
            encrypt_text_value("Task", &project_key, TASK_TITLE_CONTEXT).expect("encrypt title");
        let payload = build_task_payload_envelope(
            TaskPayloadBody {
                title: "Task".to_string(),
                rich_text: None,
                checklist: None,
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
            },
            1,
        );
        let payload_ciphertext =
            encrypt_task_payload(&payload, &project_key).expect("encrypt payload");
        let authentication = AgentGrantAuthenticationInput {
            agent_id,
            work_list_id,
            handle: "implementer",
            display_name: "Implementation Agent",
            permission_preset: AGENT_PERMISSION_PRESET,
            instructions_revision: 1,
            auth_public_key: keys.auth_public_key(),
            recipient_public_key: keys.recipient_public_key(),
            key_ciphertext: &key_ciphertext.bytes,
            instructions_ciphertext: &instructions_ciphertext.bytes,
        };
        let enrollment_code = keys.enrollment_code().expect("owner enrollment code");
        let authenticated_signature =
            sign_agent_grant(&enrollment_code, authentication).expect("sign owner grant");
        let mut claim = claim_fixture(work_list_id, Uuid::now_v7());
        claim.key_ciphertext = key_ciphertext.base64.clone();
        claim.instructions_ciphertext = instructions_ciphertext.base64.clone();
        claim.grant_signature = authenticated_signature;
        claim.task_title_ciphertext = title_ciphertext.base64.clone();
        claim.task_payload_ciphertext = payload_ciphertext.base64.clone();
        let mut substituted_identity = identity.clone();
        substituted_identity.handle = Some("reviewer".to_string());
        let error = prepare_claim(&substituted_identity, &keys, &claim)
            .expect_err("reject substituted signed identity metadata");
        assert!(
            error
                .to_string()
                .contains("not authenticated by the project owner")
        );

        let attacker_keys = agent_key_material_from_seed([0x83; 32]).expect("attacker keys");
        let attacker_code = attacker_keys.enrollment_code().expect("attacker code");
        let forged_signature =
            sign_agent_grant(&attacker_code, authentication).expect("forge grant signature");
        claim.key_ciphertext = key_ciphertext.base64;
        claim.instructions_ciphertext = instructions_ciphertext.base64;
        claim.grant_signature = forged_signature;
        claim.task_title_ciphertext = title_ciphertext.base64;
        claim.task_payload_ciphertext = payload_ciphertext.base64;

        let error = prepare_claim(&identity, &keys, &claim).expect_err("reject forged grant");
        assert!(
            error
                .to_string()
                .contains("not authenticated by the project owner")
        );
    }

    async fn spawn_finish_retry_server(
        run_id: Uuid,
    ) -> (
        String,
        Arc<Mutex<Vec<serde_json::Value>>>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind finish server");
        let api_url = format!("http://{}", listener.local_addr().expect("finish address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let server = tokio::spawn(async move {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("accept finish");
                let body = read_test_json_request(&mut stream).await;
                server_requests
                    .lock()
                    .expect("finish requests")
                    .push(body.clone());
                if attempt == 0 {
                    // Simulate a completion that may have committed after the
                    // daemon lost its HTTP response.
                    continue;
                }
                let expected_version = body["expectedVersion"]
                    .as_i64()
                    .expect("expected finish version");
                write_test_agent_run_response(
                    &mut stream,
                    run_id,
                    expected_version + 1,
                    "succeeded",
                )
                .await;
            }
        });
        (api_url, requests, server)
    }

    async fn spawn_run_retry_after_server(
        run_id: Uuid,
        successful_status: &'static str,
    ) -> (
        String,
        Arc<Mutex<Vec<serde_json::Value>>>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind run retry server");
        let api_url = format!(
            "http://{}",
            listener.local_addr().expect("run retry address")
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let server = tokio::spawn(async move {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("accept run retry");
                let body = read_test_json_request(&mut stream).await;
                server_requests
                    .lock()
                    .expect("run retry requests")
                    .push(body.clone());
                if attempt == 0 {
                    stream
                        .write_all(
                            b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 2\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                        )
                        .await
                        .expect("write rate-limit response");
                    continue;
                }
                let expected_version = body["expectedVersion"]
                    .as_i64()
                    .expect("expected run version");
                write_test_agent_run_response(
                    &mut stream,
                    run_id,
                    expected_version + 1,
                    successful_status,
                )
                .await;
            }
        });
        (api_url, requests, server)
    }

    fn active_identity_fixture(
        api_url: &str,
        work_list_id: Uuid,
        keys: &AgentKeyMaterial,
    ) -> AgentIdentity {
        AgentIdentity {
            agent_id: Uuid::now_v7(),
            api_url: api_url.to_string(),
            status: LocalAgentStatus::Active,
            proposed_handle: None,
            handle: Some("implementer".to_string()),
            display_name: Some("Implementation Agent".to_string()),
            fingerprint: keys.fingerprint(),
            auth_public_key: STANDARD_NO_PAD.encode(keys.auth_public_key()),
            recipient_public_key: STANDARD_NO_PAD.encode(keys.recipient_public_key()),
            enrollment_expires_at: None,
            project: AgentProjectBinding {
                work_list_id,
                repository_root: PathBuf::from("/tmp/project"),
                permission_preset: AGENT_PERMISSION_PRESET.to_string(),
                instructions_revision: 1,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    async fn read_test_json_request(stream: &mut tokio::net::TcpStream) -> serde_json::Value {
        let mut request = Vec::new();
        let header_end = loop {
            let mut chunk = [0_u8; 4096];
            let read = stream.read(&mut chunk).await.expect("read test request");
            assert!(read > 0, "test request ended before its headers");
            request.extend_from_slice(&chunk[..read]);
            if let Some(position) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let headers = std::str::from_utf8(&request[..header_end]).expect("test request headers");
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("content length"))
            })
            .expect("test request content length");
        while request.len() < header_end + content_length {
            let mut chunk = [0_u8; 4096];
            let read = stream
                .read(&mut chunk)
                .await
                .expect("read test request body");
            assert!(read > 0, "test request body was truncated");
            request.extend_from_slice(&chunk[..read]);
        }
        serde_json::from_slice(&request[header_end..header_end + content_length])
            .expect("test request JSON")
    }

    async fn write_test_agent_run_response(
        stream: &mut tokio::net::TcpStream,
        run_id: Uuid,
        version: i64,
        status: &str,
    ) {
        let now = Utc::now();
        let response = serde_json::to_vec(&serde_json::json!({
            "id": run_id,
            "delegationId": Uuid::now_v7(),
            "workListId": Uuid::now_v7(),
            "taskId": Uuid::now_v7(),
            "assignmentRevision": 1,
            "attempt": 1,
            "runnerInstanceId": Uuid::now_v7(),
            "sourceRevision": null,
            "instructionsRevision": 1,
            "leaseExpiresAt": now + chrono::Duration::seconds(90),
            "status": status,
            "version": version,
            "failureCode": null,
            "claimedAt": now,
            "runningAt": now,
            "finishedAt": (status == "succeeded").then_some(now),
            "createdAt": now,
            "updatedAt": now,
        }))
        .expect("agent run response");
        let headers = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            response.len()
        );
        stream
            .write_all(headers.as_bytes())
            .await
            .expect("write agent run headers");
        stream
            .write_all(&response)
            .await
            .expect("write agent run body");
    }

    async fn spawn_heartbeat_server(
        run_id: Uuid,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind heartbeat server");
        let api_url = format!(
            "http://{}",
            listener.local_addr().expect("heartbeat address")
        );
        let heartbeat_count = Arc::new(AtomicUsize::new(0));
        let server_count = Arc::clone(&heartbeat_count);
        let server = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.expect("accept heartbeat");
                let mut request = Vec::new();
                let header_end = loop {
                    let mut chunk = [0_u8; 4096];
                    let read = stream.read(&mut chunk).await.expect("read heartbeat");
                    assert!(read > 0, "heartbeat request ended before its headers");
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(position) =
                        request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                    {
                        break position + 4;
                    }
                };
                let headers =
                    std::str::from_utf8(&request[..header_end]).expect("heartbeat request headers");
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("content length"))
                    })
                    .expect("heartbeat content length");
                while request.len() < header_end + content_length {
                    let mut chunk = [0_u8; 4096];
                    let read = stream.read(&mut chunk).await.expect("read heartbeat body");
                    assert!(read > 0, "heartbeat request body was truncated");
                    request.extend_from_slice(&chunk[..read]);
                }
                let body: serde_json::Value =
                    serde_json::from_slice(&request[header_end..header_end + content_length])
                        .expect("heartbeat JSON");
                let expected_version = body["expectedVersion"]
                    .as_i64()
                    .expect("expected heartbeat version");
                let count = server_count.fetch_add(1, Ordering::SeqCst) + 1;
                assert_eq!(expected_version + 1, i64::try_from(count).expect("count"));
                let now = Utc::now();
                let response = serde_json::to_vec(&serde_json::json!({
                    "id": run_id,
                    "delegationId": Uuid::now_v7(),
                    "workListId": Uuid::now_v7(),
                    "taskId": Uuid::now_v7(),
                    "assignmentRevision": 1,
                    "attempt": 1,
                    "runnerInstanceId": Uuid::now_v7(),
                    "sourceRevision": null,
                    "instructionsRevision": 1,
                    "leaseExpiresAt": now + chrono::Duration::seconds(90),
                    "status": "running",
                    "version": expected_version + 1,
                    "failureCode": null,
                    "claimedAt": now,
                    "runningAt": now,
                    "finishedAt": null,
                    "createdAt": now,
                    "updatedAt": now,
                }))
                .expect("heartbeat response");
                let headers = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    response.len()
                );
                stream
                    .write_all(headers.as_bytes())
                    .await
                    .expect("write heartbeat headers");
                stream
                    .write_all(&response)
                    .await
                    .expect("write heartbeat body");
            }
        });
        (api_url, heartbeat_count, server)
    }

    fn claim_fixture(work_list_id: Uuid, run_id: Uuid) -> AgentClaimResponse {
        use chrono::Utc;
        use sealtask_client_api::AgentRunResponse;

        AgentClaimResponse {
            run: AgentRunResponse {
                id: run_id,
                delegation_id: Uuid::now_v7(),
                work_list_id,
                task_id: Uuid::now_v7(),
                assignment_revision: 1,
                attempt: 1,
                runner_instance_id: Uuid::now_v7(),
                source_revision: None,
                instructions_revision: 1,
                lease_expires_at: Utc::now(),
                status: "claimed".to_string(),
                version: 0,
                failure_code: None,
                claimed_at: Utc::now(),
                running_at: None,
                finished_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            lease_token: "lease".to_string(),
            workspace_id: Uuid::now_v7(),
            task_title_ciphertext: String::new(),
            task_payload_ciphertext: String::new(),
            task_updated_at: Utc::now(),
            key_ciphertext: String::new(),
            instructions_ciphertext: String::new(),
            grant_signature: String::new(),
            permission_preset: AGENT_PERMISSION_PRESET.to_string(),
        }
    }
}
