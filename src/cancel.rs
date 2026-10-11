// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

//! Cheap-to-poll cancellation shared by every per-file detector.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Cancellation signal that trips when it is explicitly cancelled or when its
/// optional deadline passes. Clones share the same state.
#[derive(Clone, Debug)]
pub struct CancelToken {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    cancelled: AtomicBool,
    deadline: Option<Instant>,
}

impl CancelToken {
    pub fn new(deadline: Option<Instant>) -> Self {
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                deadline,
            }),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self::new(Instant::now().checked_add(timeout))
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
            || self
                .inner
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

/// Polls an optional token; `None` never cancels.
pub fn is_cancelled(token: Option<&CancelToken>) -> bool {
    token.is_some_and(CancelToken::is_cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_cancel_is_visible_to_clones() {
        let token = CancelToken::new(None);
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn expired_deadline_cancels() {
        assert!(CancelToken::with_timeout(Duration::ZERO).is_cancelled());
        assert!(!CancelToken::with_timeout(Duration::from_secs(3600)).is_cancelled());
        assert!(!is_cancelled(None));
    }
}
