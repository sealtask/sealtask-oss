use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::sync::Notify;

/// Cloneable cancellation signal for a supervised runtime operation.
///
/// Cancellation is cooperative: callers retain ownership of any cleanup that
/// must finish after the public waiter goes away.
#[derive(Clone, Debug, Default)]
pub struct OperationCancellation {
    inner: Arc<OperationCancellationInner>,
}

#[derive(Debug, Default)]
struct OperationCancellationInner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl OperationCancellation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        if !self.inner.cancelled.swap(true, Ordering::AcqRel) {
            self.inner.notify.notify_waiters();
        }
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    pub(crate) async fn cancelled(&self) {
        loop {
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}
