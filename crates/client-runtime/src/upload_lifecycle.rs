use crate::{models::AgentAttachment, operation_cancellation::OperationCancellation};
use sealtask_client_core::{PublicError, PublicResult};
use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, oneshot, watch};

const MAX_FAILURE_REPORTS: usize = 32;
const MAX_ACTIVE_UPLOAD_WORKFLOWS: usize = 4;
const MAX_UPLOAD_WORKFLOW_WAITERS: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachmentUploadFailureReport {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Default)]
pub(crate) struct UploadLifecycleManager {
    inner: Arc<UploadLifecycleInner>,
}

struct UploadLifecycleInner {
    state: Mutex<UploadLifecycleState>,
    admission: Arc<Semaphore>,
    waiter_slots: Arc<Semaphore>,
    waiting: AtomicUsize,
    closing: watch::Sender<bool>,
    idle: Notify,
    failures: Mutex<VecDeque<AttachmentUploadFailureReport>>,
}

#[derive(Debug, Default)]
struct UploadLifecycleState {
    active: usize,
    closing: bool,
}

struct ActiveUpload {
    owner: UploadLifecycleManager,
    _permit: OwnedSemaphorePermit,
    completed: bool,
}

struct WaitingUpload {
    owner: UploadLifecycleManager,
    _slot: OwnedSemaphorePermit,
}

impl fmt::Debug for UploadLifecycleManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (active, closing) = {
            let state = self.lock_state();
            (state.active, state.closing)
        };
        f.debug_struct("UploadLifecycleManager")
            .field("active_uploads", &active)
            .field(
                "waiting_uploads",
                &self.inner.waiting.load(Ordering::Acquire),
            )
            .field("closing", &closing)
            .field("retained_failure_reports", &self.failure_reports().len())
            .finish()
    }
}

impl Default for UploadLifecycleInner {
    fn default() -> Self {
        let (closing, _) = watch::channel(false);
        Self {
            state: Mutex::new(UploadLifecycleState::default()),
            admission: Arc::new(Semaphore::new(MAX_ACTIVE_UPLOAD_WORKFLOWS)),
            waiter_slots: Arc::new(Semaphore::new(MAX_UPLOAD_WORKFLOW_WAITERS)),
            waiting: AtomicUsize::new(0),
            closing,
            idle: Notify::new(),
            failures: Mutex::new(VecDeque::new()),
        }
    }
}

impl UploadLifecycleManager {
    pub(crate) async fn supervise(
        &self,
        cancellation: &OperationCancellation,
        workflow: impl Future<Output = PublicResult<AgentAttachment>> + Send + 'static,
    ) -> PublicResult<oneshot::Receiver<PublicResult<AgentAttachment>>> {
        let permit = self.acquire(cancellation).await?;
        // Registration and closing use the same mutex after admission. A
        // closing drain therefore either sees this upload as active or causes
        // registration to fail before any workflow task is spawned.
        let mut active_upload = self.register(permit)?;
        let (result_tx, result_rx) = oneshot::channel();
        let worker = tokio::spawn(workflow);

        tokio::spawn(async move {
            let result = match worker.await {
                Ok(result) => {
                    if let Err(error) = &result
                        && let Some(report) = failure_report(error)
                    {
                        active_upload.record(report);
                    }
                    result
                }
                Err(join_error) if join_error.is_panic() => {
                    active_upload.record(AttachmentUploadFailureReport {
                        code: "worker_panicked",
                        message: "attachment upload worker panicked",
                    });
                    Err(PublicError::unexpected("attachment upload worker panicked"))
                }
                Err(_) => {
                    active_upload.record(AttachmentUploadFailureReport {
                        code: "worker_cancelled",
                        message: "attachment upload worker stopped before completion",
                    });
                    Err(PublicError::unexpected(
                        "attachment upload worker stopped before completion",
                    ))
                }
            };

            active_upload.complete();
            let _ = result_tx.send(result);
        });

        Ok(result_rx)
    }

    pub(crate) async fn drain(&self, timeout: Duration) -> PublicResult<()> {
        let deadline = tokio::time::Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| {
                PublicError::validation(
                    "attachment upload drain timeout exceeds the supported duration",
                )
            })?;
        {
            let mut state = self.lock_state();
            state.closing = true;
        }
        self.inner.closing.send_replace(true);
        loop {
            let idle = self.inner.idle.notified();
            tokio::pin!(idle);
            idle.as_mut().enable();
            if self.lock_state().active == 0 {
                return Ok(());
            }
            if tokio::time::timeout_at(deadline, idle).await.is_err() {
                return Err(PublicError::outcome_ambiguous(
                    "attachment upload drain",
                    "timed out waiting for attachment upload cleanup; forced process termination relies on backend orphan cleanup",
                ));
            }
        }
    }

    pub(crate) fn take_failure_reports(&self) -> Vec<AttachmentUploadFailureReport> {
        self.lock_failures().drain(..).collect()
    }

    #[cfg(test)]
    pub(crate) fn available_admission_permits(&self) -> usize {
        self.inner.admission.available_permits()
    }

    #[cfg(test)]
    fn new_with_limits(max_active: usize, max_waiters: usize) -> Self {
        assert!(max_active > 0, "upload lifecycle must allow active work");
        let (closing, _) = watch::channel(false);
        Self {
            inner: Arc::new(UploadLifecycleInner {
                state: Mutex::new(UploadLifecycleState::default()),
                admission: Arc::new(Semaphore::new(max_active)),
                waiter_slots: Arc::new(Semaphore::new(max_waiters)),
                waiting: AtomicUsize::new(0),
                closing,
                idle: Notify::new(),
                failures: Mutex::new(VecDeque::new()),
            }),
        }
    }

    #[cfg(test)]
    fn waiting_count(&self) -> usize {
        self.inner.waiting.load(Ordering::Acquire)
    }

    fn failure_reports(&self) -> MutexGuard<'_, VecDeque<AttachmentUploadFailureReport>> {
        self.lock_failures()
    }

    async fn acquire(
        &self,
        cancellation: &OperationCancellation,
    ) -> PublicResult<OwnedSemaphorePermit> {
        if cancellation.is_cancelled() {
            return Err(PublicError::cancelled("attachment upload cancelled"));
        }
        if self.lock_state().closing {
            return Err(PublicError::cancelled(
                "attachment upload lifecycle is closing",
            ));
        }
        if let Ok(permit) = self.inner.admission.clone().try_acquire_owned() {
            return Ok(permit);
        }

        let waiting = self.register_waiter()?;
        let mut closing = self.inner.closing.subscribe();
        if *closing.borrow() {
            return Err(PublicError::cancelled(
                "attachment upload lifecycle is closing",
            ));
        }
        let permit = self.inner.admission.clone().acquire_owned();
        tokio::pin!(permit);
        let result = tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                Err(PublicError::cancelled("attachment upload cancelled"))
            }
            changed = closing.changed() => {
                match changed {
                    Ok(()) if *closing.borrow() => Err(PublicError::cancelled(
                        "attachment upload lifecycle is closing",
                    )),
                    Ok(()) => Err(PublicError::unexpected(
                        "attachment upload lifecycle close signal regressed",
                    )),
                    Err(_) => Err(PublicError::unexpected(
                        "attachment upload lifecycle close signal stopped",
                    )),
                }
            }
            permit = &mut permit => {
                permit.map_err(|_| PublicError::unexpected(
                    "attachment upload admission closed",
                ))
            }
        };
        drop(waiting);
        result
    }

    fn register_waiter(&self) -> PublicResult<WaitingUpload> {
        let slot = self
            .inner
            .waiter_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                PublicError::rate_limited(
                    "too many attachment upload workflows are waiting; retry later",
                )
            })?;
        {
            let state = self.lock_state();
            if state.closing {
                return Err(PublicError::cancelled(
                    "attachment upload lifecycle is closing",
                ));
            }
        }
        self.inner.waiting.fetch_add(1, Ordering::AcqRel);
        Ok(WaitingUpload {
            owner: self.clone(),
            _slot: slot,
        })
    }

    fn register(&self, permit: OwnedSemaphorePermit) -> PublicResult<ActiveUpload> {
        let mut state = self.lock_state();
        if state.closing {
            return Err(PublicError::cancelled(
                "attachment upload lifecycle is closing",
            ));
        }
        state.active = state
            .active
            .checked_add(1)
            .ok_or_else(|| PublicError::unexpected("active attachment upload count overflowed"))?;
        Ok(ActiveUpload {
            owner: self.clone(),
            _permit: permit,
            completed: false,
        })
    }

    fn lock_state(&self) -> MutexGuard<'_, UploadLifecycleState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn lock_failures(&self) -> MutexGuard<'_, VecDeque<AttachmentUploadFailureReport>> {
        self.inner
            .failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl ActiveUpload {
    fn record(&self, report: AttachmentUploadFailureReport) {
        let mut failures = self.owner.lock_failures();
        if failures.len() == MAX_FAILURE_REPORTS {
            failures.pop_front();
        }
        failures.push_back(report);
    }

    fn complete(&mut self) {
        if self.completed {
            return;
        }
        self.completed = true;
        let became_idle = {
            let mut state = self.owner.lock_state();
            debug_assert!(state.active > 0);
            state.active = state.active.saturating_sub(1);
            state.active == 0
        };
        if became_idle {
            self.owner.inner.idle.notify_waiters();
        }
    }
}

impl Drop for ActiveUpload {
    fn drop(&mut self) {
        self.complete();
    }
}

impl Drop for WaitingUpload {
    fn drop(&mut self) {
        self.owner.inner.waiting.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(crate) fn failure_report(error: &PublicError) -> Option<AttachmentUploadFailureReport> {
    let message = match error {
        PublicError::CompensationFailed { .. } => {
            "attachment upload failed and cleanup did not complete"
        }
        PublicError::OutcomeAmbiguous { .. } => {
            "attachment upload outcome could not be established"
        }
        PublicError::Unexpected(_) => "attachment upload failed unexpectedly",
        PublicError::Conflict(_) => "attachment upload failed with a conflict",
        PublicError::Crypto(_) => "attachment upload cryptography failed",
        PublicError::Validation(_) => "attachment upload validation failed",
        PublicError::MfaRequiredUseBeginLogin | PublicError::MfaInputRequired => {
            "attachment upload requires additional authentication"
        }
        PublicError::Cancelled(_) => return None,
        _ => "attachment upload failed",
    };
    Some(AttachmentUploadFailureReport {
        code: error.code(),
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    #[tokio::test]
    async fn failure_report_queue_discards_oldest_entries_at_its_bound() {
        let manager = UploadLifecycleManager::default();
        for _ in 0..(MAX_FAILURE_REPORTS + 5) {
            let cancellation = OperationCancellation::new();
            let result = manager
                .supervise(&cancellation, async {
                    Err(PublicError::unexpected(
                        "injected worker failure with sensitive detail",
                    ))
                })
                .await
                .expect("admit workflow")
                .await
                .expect("supervisor result");
            assert!(result.is_err());
        }

        let reports = manager.take_failure_reports();
        assert_eq!(reports.len(), MAX_FAILURE_REPORTS);
        assert!(
            reports.iter().all(|report| report.code == "unexpected"
                && !report.message.contains("sensitive detail"))
        );
    }

    #[tokio::test]
    async fn drain_rejects_timeout_overflow_while_work_is_active() {
        let manager = UploadLifecycleManager::default();
        let release = Arc::new(Notify::new());
        let worker_release = release.clone();
        let cancellation = OperationCancellation::new();
        let result = manager
            .supervise(&cancellation, async move {
                worker_release.notified().await;
                Err(PublicError::cancelled("test upload released"))
            })
            .await
            .expect("admit workflow");

        let error = manager
            .drain(Duration::MAX)
            .await
            .expect_err("overflowing timeout must be rejected");
        assert!(matches!(
            error,
            PublicError::Validation(message)
                if message == "attachment upload drain timeout exceeds the supported duration"
        ));

        release.notify_one();
        let _ = result.await.expect("supervised result");
        manager
            .drain(Duration::from_secs(1))
            .await
            .expect("released upload drains");
    }

    #[tokio::test]
    async fn closing_drain_atomically_rejects_new_uploads() {
        let manager = UploadLifecycleManager::default();
        let release = Arc::new(Notify::new());
        let worker_release = release.clone();
        let existing_cancellation = OperationCancellation::new();
        let existing = manager
            .supervise(&existing_cancellation, async move {
                worker_release.notified().await;
                Err(PublicError::cancelled("released"))
            })
            .await
            .expect("admit existing workflow");
        let drain_manager = manager.clone();
        let drain = tokio::spawn(async move { drain_manager.drain(Duration::from_secs(1)).await });
        while !manager.lock_state().closing {
            tokio::task::yield_now().await;
        }

        let rejected_cancellation = OperationCancellation::new();
        let rejected = manager
            .supervise(&rejected_cancellation, async {
                panic!("a closing lifecycle must not spawn a new upload");
                #[allow(unreachable_code)]
                Err(PublicError::unexpected("unreachable"))
            })
            .await
            .expect_err("new upload must be rejected");
        assert!(matches!(rejected, PublicError::Cancelled(_)));

        release.notify_one();
        let _ = existing.await.expect("existing upload response");
        drain
            .await
            .expect("drain task")
            .expect("existing upload drains");
    }

    #[tokio::test]
    async fn waiter_admission_fails_fast_at_its_bound() {
        let manager = UploadLifecycleManager::new_with_limits(1, 1);
        let release = Arc::new(AtomicBool::new(false));
        let active_release = Arc::clone(&release);
        let active_cancellation = OperationCancellation::new();
        let active = manager
            .supervise(&active_cancellation, async move {
                while !active_release.load(Ordering::Acquire) {
                    tokio::task::yield_now().await;
                }
                Err(PublicError::cancelled("active test upload released"))
            })
            .await
            .expect("admit active upload");

        let waiter_manager = manager.clone();
        let waiter_cancellation = OperationCancellation::new();
        let cancellation_for_waiter = waiter_cancellation.clone();
        let waiter = tokio::spawn(async move {
            waiter_manager
                .supervise(&cancellation_for_waiter, async {
                    panic!("bounded waiter must not start before active work releases");
                    #[allow(unreachable_code)]
                    Err(PublicError::unexpected("unreachable"))
                })
                .await
        });
        while manager.waiting_count() != 1 {
            tokio::task::yield_now().await;
        }

        let saturated = manager
            .supervise(&OperationCancellation::new(), async {
                panic!("saturated workflow must not be spawned");
                #[allow(unreachable_code)]
                Err(PublicError::unexpected("unreachable"))
            })
            .await
            .expect_err("waiter capacity must fail fast");
        assert!(matches!(
            saturated,
            PublicError::RateLimited(message)
                if message.message()
                    == "too many attachment upload workflows are waiting; retry later"
        ));

        waiter_cancellation.cancel();
        assert!(matches!(
            waiter.await.expect("waiter task"),
            Err(PublicError::Cancelled(_))
        ));
        release.store(true, Ordering::Release);
        let _ = active.await.expect("active upload result");
    }

    #[tokio::test]
    async fn cancelled_waiter_releases_capacity_for_the_next_workflow() {
        let manager = UploadLifecycleManager::new_with_limits(1, 1);
        let release = Arc::new(AtomicBool::new(false));
        let active_release = Arc::clone(&release);
        let active = manager
            .supervise(&OperationCancellation::new(), async move {
                while !active_release.load(Ordering::Acquire) {
                    tokio::task::yield_now().await;
                }
                Err(PublicError::cancelled("active test upload released"))
            })
            .await
            .expect("admit active upload");

        let first_manager = manager.clone();
        let first_cancellation = OperationCancellation::new();
        let first_waiter_cancellation = first_cancellation.clone();
        let first_waiter = tokio::spawn(async move {
            first_manager
                .supervise(&first_waiter_cancellation, async {
                    Err(PublicError::unexpected(
                        "cancelled waiter unexpectedly started",
                    ))
                })
                .await
        });
        while manager.waiting_count() != 1 {
            tokio::task::yield_now().await;
        }
        first_cancellation.cancel();
        assert!(matches!(
            first_waiter.await.expect("cancelled waiter task"),
            Err(PublicError::Cancelled(_))
        ));
        assert_eq!(manager.waiting_count(), 0);

        let started = Arc::new(AtomicBool::new(false));
        let started_by_workflow = Arc::clone(&started);
        let second_manager = manager.clone();
        let second_waiter = tokio::spawn(async move {
            second_manager
                .supervise(&OperationCancellation::new(), async move {
                    started_by_workflow.store(true, Ordering::Release);
                    Err(PublicError::cancelled("replacement waiter completed"))
                })
                .await
        });
        while manager.waiting_count() != 1 {
            tokio::task::yield_now().await;
        }

        release.store(true, Ordering::Release);
        let _ = active.await.expect("active upload result");
        let second_result = second_waiter
            .await
            .expect("replacement waiter task")
            .expect("replacement waiter admitted")
            .await
            .expect("replacement waiter result channel");
        assert!(matches!(second_result, Err(PublicError::Cancelled(_))));
        assert!(started.load(Ordering::Acquire));
        assert_eq!(manager.waiting_count(), 0);
    }

    #[tokio::test]
    async fn drain_rejects_queued_waiters_but_preserves_active_cleanup() {
        let manager = UploadLifecycleManager::new_with_limits(1, 1);
        let release = Arc::new(AtomicBool::new(false));
        let active_release = Arc::clone(&release);
        let active = manager
            .supervise(&OperationCancellation::new(), async move {
                while !active_release.load(Ordering::Acquire) {
                    tokio::task::yield_now().await;
                }
                Err(PublicError::cancelled("active cleanup completed"))
            })
            .await
            .expect("admit active upload");

        let waiter_manager = manager.clone();
        let waiter = tokio::spawn(async move {
            waiter_manager
                .supervise(&OperationCancellation::new(), async {
                    panic!("drain must not promote a queued upload");
                    #[allow(unreachable_code)]
                    Err(PublicError::unexpected("unreachable"))
                })
                .await
        });
        while manager.waiting_count() != 1 {
            tokio::task::yield_now().await;
        }

        let drain_manager = manager.clone();
        let drain = tokio::spawn(async move { drain_manager.drain(Duration::from_secs(1)).await });
        let waiter_result = tokio::time::timeout(Duration::from_millis(250), waiter)
            .await
            .expect("drain promptly wakes queued waiter")
            .expect("queued waiter task");
        assert!(matches!(waiter_result, Err(PublicError::Cancelled(_))));
        assert_eq!(manager.waiting_count(), 0);
        assert!(
            !drain.is_finished(),
            "drain must continue waiting for already-active cleanup"
        );

        release.store(true, Ordering::Release);
        let _ = active.await.expect("active cleanup result");
        drain
            .await
            .expect("drain task")
            .expect("active cleanup drains");
    }

    #[tokio::test]
    async fn end_to_end_admission_bounds_active_workflows() {
        let manager = UploadLifecycleManager::default();
        let started = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(AtomicBool::new(false));
        let mut callers = Vec::new();
        for _ in 0..(MAX_ACTIVE_UPLOAD_WORKFLOWS * 2) {
            let caller_manager = manager.clone();
            let started = started.clone();
            let release = release.clone();
            callers.push(tokio::spawn(async move {
                let cancellation = OperationCancellation::new();
                let result = caller_manager
                    .supervise(&cancellation, async move {
                        started.fetch_add(1, Ordering::AcqRel);
                        while !release.load(Ordering::Acquire) {
                            tokio::task::yield_now().await;
                        }
                        Err(PublicError::cancelled("test workflow finished"))
                    })
                    .await?;
                result.await.map_err(|_| {
                    PublicError::unexpected("test upload supervisor dropped its result")
                })?
            }));
        }

        tokio::time::timeout(Duration::from_secs(1), async {
            while started.load(Ordering::Acquire) < MAX_ACTIVE_UPLOAD_WORKFLOWS {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("initial admitted workflows start");
        tokio::task::yield_now().await;
        assert_eq!(
            started.load(Ordering::Acquire),
            MAX_ACTIVE_UPLOAD_WORKFLOWS,
            "waiters above the limit must not start a workflow task"
        );
        assert_eq!(
            manager.lock_state().active,
            MAX_ACTIVE_UPLOAD_WORKFLOWS,
            "waiters above the limit must not register as active"
        );

        release.store(true, Ordering::Release);
        for caller in callers {
            let _ = caller.await.expect("caller task");
        }
    }
}
