use crate::operation_cancellation::OperationCancellation;
use sealtask_client_core::{PublicError, PublicResult};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, oneshot};

const MAX_CONCURRENT_BLOCKING_CRYPTO: usize = 2;
const MAX_BLOCKING_CRYPTO_WAITERS: usize = 8;
const MAX_CONCURRENT_LARGE_PAYLOADS: usize = 2;
const MAX_LARGE_PAYLOAD_WAITERS: usize = 4;

#[derive(Clone)]
pub(crate) struct BlockingCryptoAdmission {
    inner: Arc<BlockingCryptoAdmissionInner>,
}

pub(crate) struct BlockingCryptoPermit(OwnedSemaphorePermit);

#[derive(Debug)]
pub(crate) struct LargePayloadPermit {
    lease: Arc<OwnedSemaphorePermit>,
}

struct BlockingCryptoAdmissionInner {
    blocking: AdmissionLane,
    large_payloads: AdmissionLane,
    started: Arc<Notify>,
}

struct AdmissionLane {
    permits: Arc<Semaphore>,
    waiter_slots: Arc<Semaphore>,
    waiting: AtomicUsize,
}

struct WaitingForAdmission<'a> {
    waiting: &'a AtomicUsize,
    _slot: OwnedSemaphorePermit,
}

impl Default for BlockingCryptoAdmission {
    fn default() -> Self {
        Self::new_with_limits(
            MAX_CONCURRENT_BLOCKING_CRYPTO,
            MAX_BLOCKING_CRYPTO_WAITERS,
            MAX_CONCURRENT_LARGE_PAYLOADS,
            MAX_LARGE_PAYLOAD_WAITERS,
        )
    }
}

impl fmt::Debug for BlockingCryptoAdmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockingCryptoAdmission")
            .field(
                "available_blocking_permits",
                &self.inner.blocking.permits.available_permits(),
            )
            .field(
                "blocking_waiters",
                &self.inner.blocking.waiting.load(Ordering::Acquire),
            )
            .field(
                "available_large_payload_permits",
                &self.inner.large_payloads.permits.available_permits(),
            )
            .field(
                "large_payload_waiters",
                &self.inner.large_payloads.waiting.load(Ordering::Acquire),
            )
            .finish()
    }
}

impl BlockingCryptoAdmission {
    #[cfg(test)]
    pub(crate) fn new(max_concurrent: usize) -> Self {
        Self::new_with_limits(
            max_concurrent,
            MAX_BLOCKING_CRYPTO_WAITERS,
            max_concurrent,
            MAX_LARGE_PAYLOAD_WAITERS,
        )
    }

    fn new_with_limits(
        max_concurrent: usize,
        max_waiters: usize,
        max_large_payloads: usize,
        max_large_payload_waiters: usize,
    ) -> Self {
        assert!(
            max_concurrent > 0,
            "blocking crypto admission must have at least one permit"
        );
        assert!(
            max_large_payloads > 0,
            "large-payload admission must have at least one permit"
        );
        Self {
            inner: Arc::new(BlockingCryptoAdmissionInner {
                blocking: AdmissionLane::new(max_concurrent, max_waiters),
                large_payloads: AdmissionLane::new(max_large_payloads, max_large_payload_waiters),
                started: Arc::new(Notify::new()),
            }),
        }
    }

    pub(crate) async fn run<T, F>(&self, work: F, failure_message: &'static str) -> PublicResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        let permit = self.acquire().await?;
        await_supervised(
            start_supervised(permit, work, failure_message, self.inner.started.clone()),
            failure_message,
        )
        .await
    }

    pub(crate) async fn run_cancellable<T, F>(
        &self,
        cancellation: &OperationCancellation,
        work: F,
        failure_message: &'static str,
    ) -> PublicResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        let permit = self.admit_cancellable(cancellation).await?;
        self.run_admitted_cancellable(permit, cancellation, work, failure_message)
            .await
    }

    pub(crate) async fn admit_cancellable(
        &self,
        cancellation: &OperationCancellation,
    ) -> PublicResult<BlockingCryptoPermit> {
        self.acquire_cancellable(cancellation)
            .await
            .map(BlockingCryptoPermit)
    }

    pub(crate) async fn admit_large_payload(&self) -> PublicResult<LargePayloadPermit> {
        self.acquire_lane(
            &self.inner.large_payloads,
            "large-payload admission is unavailable",
            "too many large-payload operations are waiting; retry later",
        )
        .await
        .map(|permit| LargePayloadPermit {
            lease: Arc::new(permit),
        })
    }

    pub(crate) async fn admit_large_payload_cancellable(
        &self,
        cancellation: &OperationCancellation,
    ) -> PublicResult<LargePayloadPermit> {
        self.acquire_lane_cancellable(
            &self.inner.large_payloads,
            cancellation,
            "large-payload admission is unavailable",
            "too many large-payload operations are waiting; retry later",
        )
        .await
        .map(|permit| LargePayloadPermit {
            lease: Arc::new(permit),
        })
    }

    pub(crate) async fn run_with_large_payload<T, F>(
        &self,
        payload_permit: LargePayloadPermit,
        work: F,
        failure_message: &'static str,
    ) -> PublicResult<(LargePayloadPermit, T)>
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        let (payload_permit, result) = self
            .run_with_large_payload_preserving(payload_permit, work, failure_message)
            .await;
        result.map(|value| (payload_permit, value))
    }

    /// Runs blocking work while preserving the caller's large-payload lease
    /// across every recoverable outcome.
    ///
    /// The detached supervisor owns the shared lease while it waits for CPU
    /// admission and while the blocking task runs. The returned permit
    /// therefore refers to the same admission lease on success, work failure,
    /// admission rejection, worker panic, or supervisor-channel failure.
    pub(crate) async fn run_with_large_payload_preserving<T, F>(
        &self,
        payload_permit: LargePayloadPermit,
        work: F,
        failure_message: &'static str,
    ) -> (LargePayloadPermit, PublicResult<T>)
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        let recovery_permit = payload_permit.duplicate_lease();
        let result = start_large_payload_supervisor(
            self.clone(),
            payload_permit,
            work,
            failure_message,
            None,
        );
        receive_preserving_supervised(result.await, recovery_permit, failure_message)
    }

    pub(crate) async fn run_with_large_payload_cancellable<T, F>(
        &self,
        payload_permit: LargePayloadPermit,
        cancellation: &OperationCancellation,
        work: F,
        failure_message: &'static str,
    ) -> PublicResult<(LargePayloadPermit, T)>
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        let (payload_permit, result) = self
            .run_with_large_payload_cancellable_preserving(
                payload_permit,
                cancellation,
                work,
                failure_message,
            )
            .await;
        result.map(|value| (payload_permit, value))
    }

    pub(crate) async fn run_admitted_cancellable<T, F>(
        &self,
        permit: BlockingCryptoPermit,
        cancellation: &OperationCancellation,
        work: F,
        failure_message: &'static str,
    ) -> PublicResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        if cancellation.is_cancelled() {
            return Err(PublicError::cancelled("attachment upload cancelled"));
        }

        let mut result =
            start_supervised(permit.0, work, failure_message, self.inner.started.clone());
        tokio::select! {
            biased;
            result = &mut result => return receive_supervised(result, failure_message),
            () = cancellation.cancelled() => {}
        }

        // Once blocking work starts, its supervisor owns and joins it even if
        // the public upload waiter disappears. Cancellation is reported only
        // after the blocking operation has released its admission permit.
        let completed = receive_supervised(result.await, failure_message)?;
        if cancellation.is_cancelled() {
            Err(PublicError::cancelled("attachment upload cancelled"))
        } else {
            Ok(completed)
        }
    }

    async fn acquire(&self) -> PublicResult<OwnedSemaphorePermit> {
        self.acquire_lane(
            &self.inner.blocking,
            "blocking crypto admission is unavailable",
            "too many blocking crypto operations are waiting; retry later",
        )
        .await
    }

    async fn acquire_cancellable(
        &self,
        cancellation: &OperationCancellation,
    ) -> PublicResult<OwnedSemaphorePermit> {
        self.acquire_lane_cancellable(
            &self.inner.blocking,
            cancellation,
            "blocking crypto admission is unavailable",
            "too many blocking crypto operations are waiting; retry later",
        )
        .await
    }

    async fn run_with_large_payload_cancellable_preserving<T, F>(
        &self,
        payload_permit: LargePayloadPermit,
        cancellation: &OperationCancellation,
        work: F,
        failure_message: &'static str,
    ) -> (LargePayloadPermit, PublicResult<T>)
    where
        T: Send + 'static,
        F: FnOnce() -> PublicResult<T> + Send + 'static,
    {
        let recovery_permit = payload_permit.duplicate_lease();
        let result = start_large_payload_supervisor(
            self.clone(),
            payload_permit,
            work,
            failure_message,
            Some(cancellation.clone()),
        );
        receive_preserving_supervised(result.await, recovery_permit, failure_message)
    }

    async fn acquire_lane(
        &self,
        lane: &AdmissionLane,
        unavailable_message: &'static str,
        saturated_message: &'static str,
    ) -> PublicResult<OwnedSemaphorePermit> {
        if let Ok(permit) = lane.permits.clone().try_acquire_owned() {
            return Ok(permit);
        }
        let _waiting = WaitingForAdmission::new(lane, saturated_message)?;
        lane.permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| PublicError::unexpected(unavailable_message))
    }

    async fn acquire_lane_cancellable(
        &self,
        lane: &AdmissionLane,
        cancellation: &OperationCancellation,
        unavailable_message: &'static str,
        saturated_message: &'static str,
    ) -> PublicResult<OwnedSemaphorePermit> {
        if let Ok(permit) = lane.permits.clone().try_acquire_owned() {
            return Ok(permit);
        }
        let _waiting = WaitingForAdmission::new(lane, saturated_message)?;
        let permit = lane.permits.clone().acquire_owned();
        tokio::pin!(permit);
        tokio::select! {
            biased;
            () = cancellation.cancelled() => {
                Err(PublicError::cancelled("attachment upload cancelled"))
            }
            permit = &mut permit => {
                permit.map_err(|_| PublicError::unexpected(unavailable_message))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn waiting_count(&self) -> usize {
        self.inner.blocking.waiting.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn available_permits(&self) -> usize {
        self.inner.blocking.permits.available_permits()
    }

    #[cfg(test)]
    pub(crate) fn available_large_payload_permits(&self) -> usize {
        self.inner.large_payloads.permits.available_permits()
    }

    #[cfg(test)]
    pub(crate) fn large_payload_waiting_count(&self) -> usize {
        self.inner.large_payloads.waiting.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) async fn wait_for_start(&self) {
        self.inner.started.notified().await;
    }
}

impl LargePayloadPermit {
    fn duplicate_lease(&self) -> Self {
        Self {
            lease: self.lease.clone(),
        }
    }
}

impl AdmissionLane {
    fn new(max_active: usize, max_waiters: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_active)),
            waiter_slots: Arc::new(Semaphore::new(max_waiters)),
            waiting: AtomicUsize::new(0),
        }
    }
}

impl<'a> WaitingForAdmission<'a> {
    fn new(lane: &'a AdmissionLane, saturated_message: &'static str) -> PublicResult<Self> {
        let slot = lane
            .waiter_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| PublicError::rate_limited(saturated_message))?;
        lane.waiting.fetch_add(1, Ordering::AcqRel);
        Ok(Self {
            waiting: &lane.waiting,
            _slot: slot,
        })
    }
}

impl Drop for WaitingForAdmission<'_> {
    fn drop(&mut self) {
        self.waiting.fetch_sub(1, Ordering::AcqRel);
    }
}

fn start_supervised<T, F>(
    permit: OwnedSemaphorePermit,
    work: F,
    failure_message: &'static str,
    started: Arc<Notify>,
) -> oneshot::Receiver<PublicResult<T>>
where
    T: Send + 'static,
    F: FnOnce() -> PublicResult<T> + Send + 'static,
{
    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        started.notify_one();
        work()
    });
    let (result_tx, result_rx) = oneshot::channel();
    tokio::spawn(async move {
        let result = sanitize_blocking_result(task.await, failure_message);
        let _ = result_tx.send(result);
    });
    result_rx
}

fn start_large_payload_supervisor<T, F>(
    admission: BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
    work: F,
    failure_message: &'static str,
    cancellation: Option<OperationCancellation>,
) -> oneshot::Receiver<(LargePayloadPermit, PublicResult<T>)>
where
    T: Send + 'static,
    F: FnOnce() -> PublicResult<T> + Send + 'static,
{
    let (result_tx, result_rx) = oneshot::channel();
    tokio::spawn(async move {
        let blocking_permit = match cancellation.as_ref() {
            Some(cancellation) if cancellation.is_cancelled() => {
                Err(PublicError::cancelled("attachment upload cancelled"))
            }
            Some(cancellation) => admission.acquire_cancellable(cancellation).await,
            None => admission.acquire().await,
        };
        let result = match blocking_permit {
            Ok(blocking_permit) => {
                let started = admission.inner.started.clone();
                let task = tokio::task::spawn_blocking(move || {
                    let _blocking_permit = blocking_permit;
                    started.notify_one();
                    work()
                });
                let result = sanitize_blocking_result(task.await, failure_message);
                if cancellation
                    .as_ref()
                    .is_some_and(OperationCancellation::is_cancelled)
                {
                    Err(PublicError::cancelled("attachment upload cancelled"))
                } else {
                    result
                }
            }
            Err(error) => Err(error),
        };
        let _ = result_tx.send((payload_permit, result));
    });
    result_rx
}

async fn await_supervised<T>(
    result: oneshot::Receiver<PublicResult<T>>,
    failure_message: &'static str,
) -> PublicResult<T> {
    receive_supervised(result.await, failure_message)
}

fn receive_supervised<T>(
    result: Result<PublicResult<T>, oneshot::error::RecvError>,
    failure_message: &'static str,
) -> PublicResult<T> {
    result.map_err(|_| {
        PublicError::unexpected(format!(
            "{failure_message}: supervisor stopped unexpectedly"
        ))
    })?
}

fn receive_preserving_supervised<T>(
    result: Result<(LargePayloadPermit, PublicResult<T>), oneshot::error::RecvError>,
    recovery_permit: LargePayloadPermit,
    failure_message: &'static str,
) -> (LargePayloadPermit, PublicResult<T>) {
    match result {
        Ok((payload_permit, result)) => (payload_permit, result),
        Err(_) => (
            recovery_permit,
            Err(PublicError::unexpected(format!(
                "{failure_message}: supervisor stopped unexpectedly"
            ))),
        ),
    }
}

fn sanitize_blocking_result<T>(
    result: Result<PublicResult<T>, tokio::task::JoinError>,
    failure_message: &'static str,
) -> PublicResult<T> {
    match result {
        Ok(result) => result,
        Err(error) if error.is_panic() => Err(PublicError::unexpected(format!(
            "{failure_message}: worker panicked"
        ))),
        Err(_) => Err(PublicError::unexpected(format!(
            "{failure_message}: worker was cancelled"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shared_admission_bounds_concurrent_blocking_crypto() {
        let admission = BlockingCryptoAdmission::new(2);
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tasks = Vec::new();

        for _ in 0..4 {
            let admission = admission.clone();
            let gate = gate.clone();
            let active = active.clone();
            let maximum = maximum.clone();
            let started_tx = started_tx.clone();
            tasks.push(tokio::spawn(async move {
                admission
                    .run(
                        move || {
                            let now = active.fetch_add(1, Ordering::AcqRel) + 1;
                            maximum.fetch_max(now, Ordering::AcqRel);
                            started_tx.send(()).expect("observe blocking start");
                            let (lock, condition) = &*gate;
                            let mut released = lock
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            while !*released {
                                released = condition
                                    .wait(released)
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                            }
                            active.fetch_sub(1, Ordering::AcqRel);
                            Ok(())
                        },
                        "injected blocking crypto failed",
                    )
                    .await
            }));
        }

        started_rx.recv().await.expect("first blocking task starts");
        started_rx
            .recv()
            .await
            .expect("second blocking task starts");
        assert!(started_rx.try_recv().is_err());
        assert_eq!(maximum.load(Ordering::Acquire), 2);
        assert!(
            admission.waiting_count() >= 2,
            "remaining work must wait at the shared admission boundary"
        );

        let (lock, condition) = &*gate;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        condition.notify_all();
        for task in tasks {
            task.await
                .expect("blocking crypto caller joins")
                .expect("blocking crypto succeeds");
        }
        assert_eq!(maximum.load(Ordering::Acquire), 2);
    }

    #[tokio::test]
    async fn upload_cancellation_while_waiting_does_not_spawn_blocking_work() {
        let admission = BlockingCryptoAdmission::new(1);
        let held_permit = admission
            .inner
            .blocking
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("hold only blocking permit");
        let cancellation = OperationCancellation::new();
        let caller_cancellation = cancellation.clone();
        let work_started = Arc::new(AtomicBool::new(false));
        let worker_started = work_started.clone();
        let caller = tokio::spawn({
            let admission = admission.clone();
            async move {
                admission
                    .run_cancellable(
                        &caller_cancellation,
                        move || {
                            worker_started.store(true, Ordering::Release);
                            Ok(())
                        },
                        "injected blocking crypto failed",
                    )
                    .await
            }
        });

        while admission.waiting_count() == 0 {
            tokio::task::yield_now().await;
        }
        cancellation.cancel();
        let error = caller
            .await
            .expect("cancellable caller joins")
            .expect_err("waiting caller must cancel");
        assert!(matches!(error, PublicError::Cancelled(_)));
        assert!(!work_started.load(Ordering::Acquire));
        drop(held_permit);
    }

    #[tokio::test]
    async fn large_payload_wait_queue_rejects_work_over_its_cap() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let held = admission
            .admit_large_payload()
            .await
            .expect("hold active large-payload permit");
        let waiting = tokio::spawn({
            let admission = admission.clone();
            async move { admission.admit_large_payload().await }
        });
        while admission.large_payload_waiting_count() == 0 {
            tokio::task::yield_now().await;
        }

        let error = admission
            .admit_large_payload()
            .await
            .expect_err("work above the bounded wait queue must be rejected");
        assert!(matches!(error, PublicError::RateLimited(_)));

        drop(held);
        let admitted = waiting
            .await
            .expect("waiting caller joins")
            .expect("bounded waiter is eventually admitted");
        drop(admitted);
        assert_eq!(admission.available_large_payload_permits(), 1);
    }

    #[tokio::test]
    async fn cancelling_large_payload_wait_releases_its_queue_slot() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let held = admission
            .admit_large_payload()
            .await
            .expect("hold active large-payload permit");
        let cancellation = OperationCancellation::new();
        let waiting_cancellation = cancellation.clone();
        let waiting = tokio::spawn({
            let admission = admission.clone();
            async move {
                admission
                    .admit_large_payload_cancellable(&waiting_cancellation)
                    .await
            }
        });
        while admission.large_payload_waiting_count() == 0 {
            tokio::task::yield_now().await;
        }
        cancellation.cancel();
        let error = waiting
            .await
            .expect("waiting caller joins")
            .expect_err("waiting admission must cancel");
        assert!(matches!(error, PublicError::Cancelled(_)));
        assert_eq!(admission.large_payload_waiting_count(), 0);

        let replacement = tokio::spawn({
            let admission = admission.clone();
            async move { admission.admit_large_payload().await }
        });
        while admission.large_payload_waiting_count() == 0 {
            tokio::task::yield_now().await;
        }
        drop(held);
        drop(
            replacement
                .await
                .expect("replacement caller joins")
                .expect("released queue slot accepts another waiter"),
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_caller_does_not_release_large_payload_before_blocking_join() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_gate = gate.clone();
        let caller = tokio::spawn({
            let admission = admission.clone();
            async move {
                admission
                    .run_with_large_payload(
                        payload_permit,
                        move || {
                            let (lock, condition) = &*worker_gate;
                            let mut released = lock
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            while !*released {
                                released = condition
                                    .wait(released)
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                            }
                            Ok(())
                        },
                        "injected large-payload task failed",
                    )
                    .await
            }
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("blocking work starts");
        caller.abort();
        let _ = caller.await;
        assert_eq!(
            admission.available_large_payload_permits(),
            0,
            "the blocking supervisor must retain payload admission after caller cancellation"
        );

        let (lock, condition) = &*gate;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        condition.notify_all();
        tokio::time::timeout(Duration::from_secs(1), async {
            while admission.available_large_payload_permits() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("supervisor releases payload permit after blocking join");
        assert_eq!(admission.available_large_payload_permits(), 1);
    }

    #[tokio::test]
    async fn blocking_panic_is_observed_and_sanitized() {
        let admission = BlockingCryptoAdmission::new(1);
        let error = admission
            .run(
                || -> PublicResult<()> {
                    panic!("blocking crypto panic canary");
                },
                "attachment encryption task failed",
            )
            .await
            .expect_err("panic must be reported");

        assert!(matches!(
            error,
            PublicError::Unexpected(message)
                if message == "attachment encryption task failed: worker panicked"
                    && !message.contains("panic canary")
        ));
    }

    #[tokio::test]
    async fn preserving_large_payload_outcome_returns_lease_after_worker_panic() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");

        let (payload_permit, error) = admission
            .run_with_large_payload_preserving(
                payload_permit,
                || -> PublicResult<()> {
                    panic!("preserving panic canary");
                },
                "preserving payload task failed",
            )
            .await;

        assert!(matches!(
            error,
            Err(PublicError::Unexpected(message))
                if message == "preserving payload task failed: worker panicked"
                    && !message.contains("panic canary")
        ));
        assert_eq!(
            admission.available_large_payload_permits(),
            0,
            "the returned lease must still own payload admission"
        );
        drop(payload_permit);
        assert_eq!(admission.available_large_payload_permits(), 1);
    }

    #[tokio::test]
    async fn preserving_large_payload_outcome_returns_lease_after_work_error() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");

        let (payload_permit, error) = admission
            .run_with_large_payload_preserving(
                payload_permit,
                || -> PublicResult<()> { Err(PublicError::crypto("injected work failure")) },
                "preserving payload task failed",
            )
            .await;

        assert!(
            matches!(error, Err(PublicError::Crypto(message)) if message == "injected work failure")
        );
        assert_eq!(admission.available_large_payload_permits(), 0);
        drop(payload_permit);
        assert_eq!(admission.available_large_payload_permits(), 1);
    }

    #[tokio::test]
    async fn preserving_large_payload_outcome_returns_lease_after_admission_rejection() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let held_blocking = admission
            .inner
            .blocking
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("hold blocking permit");
        let queued = tokio::spawn({
            let admission = admission.clone();
            async move {
                admission
                    .run(|| Ok(()), "queued blocking task failed")
                    .await
            }
        });
        while admission.waiting_count() == 0 {
            tokio::task::yield_now().await;
        }
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");

        let (payload_permit, error) = admission
            .run_with_large_payload_preserving(
                payload_permit,
                || Ok(()),
                "rejected preserving task failed",
            )
            .await;

        assert!(matches!(error, Err(PublicError::RateLimited(_))));
        assert_eq!(admission.available_large_payload_permits(), 0);
        drop(payload_permit);
        assert_eq!(admission.available_large_payload_permits(), 1);

        drop(held_blocking);
        queued
            .await
            .expect("queued caller joins")
            .expect("queued work completes");
    }

    #[tokio::test]
    async fn pre_cancelled_large_payload_work_never_starts_and_releases_lease() {
        let admission = BlockingCryptoAdmission::new_with_limits(1, 1, 1, 1);
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let cancellation = OperationCancellation::new();
        cancellation.cancel();
        let work_started = Arc::new(AtomicBool::new(false));
        let work_started_in_worker = work_started.clone();

        let error = admission
            .run_with_large_payload_cancellable(
                payload_permit,
                &cancellation,
                move || {
                    work_started_in_worker.store(true, Ordering::Release);
                    Ok(())
                },
                "cancelled payload task failed",
            )
            .await
            .expect_err("pre-cancelled work must fail");

        assert!(matches!(error, PublicError::Cancelled(_)));
        assert!(!work_started.load(Ordering::Acquire));
        assert_eq!(admission.available_large_payload_permits(), 1);
    }
}
