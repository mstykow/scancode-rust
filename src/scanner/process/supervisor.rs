// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

//! Supervised per-file workers that enforce `--timeout` even when a detector
//! never polls its cancellation token.
//!
//! Each job runs on a long-lived worker thread while the calling scan thread
//! waits with a deadline. A worker that overruns is abandoned: its result can
//! never be received, its slot is replaced, and new work is refused once the
//! abandoned workers still running reach the cap. Jobs already in flight when the
//! cap is reached cannot be recalled, so the count is bounded by the cap plus the
//! number of concurrent callers.

use crate::cancel::CancelToken;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type Job = Box<dyn FnOnce() + Send + 'static>;

const RUNNING: u8 = 0;
const FINISHED: u8 = 1;
const ABANDONED: u8 = 2;

pub(super) enum Supervised<T> {
    Completed(T),
    TimedOut,
    Refused(String),
}

pub(super) struct WorkerPool {
    idle: Mutex<Vec<Sender<Job>>>,
    abandoned: Arc<AtomicUsize>,
    max_abandoned: usize,
}

struct SlotGuard {
    state: Arc<AtomicU8>,
    abandoned: Arc<AtomicUsize>,
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        if self.state.swap(FINISHED, Ordering::AcqRel) == ABANDONED {
            self.abandoned.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

impl WorkerPool {
    pub(super) fn new(max_abandoned: usize) -> Self {
        Self {
            idle: Mutex::new(Vec::new()),
            abandoned: Arc::new(AtomicUsize::new(0)),
            max_abandoned,
        }
    }

    pub(super) fn abandoned_workers(&self) -> usize {
        self.abandoned.load(Ordering::Acquire)
    }

    /// Runs `job` on a worker and waits at most `wait` for its result. `token`
    /// is cancelled when the job is abandoned so a polling worker stops early.
    pub(super) fn run<T, F>(&self, token: &CancelToken, wait: Duration, job: F) -> Supervised<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let abandoned = self.abandoned_workers();
        if abandoned >= self.max_abandoned {
            return Supervised::Refused(format!(
                "{abandoned} timed-out detection workers are still running (limit {})",
                self.max_abandoned
            ));
        }

        let (result_tx, result_rx) = sync_channel::<Result<T, Box<dyn Any + Send>>>(1);
        let state = Arc::new(AtomicU8::new(RUNNING));
        let guard_state = Arc::clone(&state);
        let guard_counter = Arc::clone(&self.abandoned);
        let boxed: Job = Box::new(move || {
            let _guard = SlotGuard {
                state: Arc::clone(&guard_state),
                abandoned: guard_counter,
            };
            let outcome = catch_unwind(AssertUnwindSafe(job));
            if guard_state
                .compare_exchange(RUNNING, FINISHED, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                let _ = result_tx.send(outcome);
            }
        });

        let worker = match self.dispatch(boxed) {
            Ok(worker) => worker,
            Err(reason) => return Supervised::Refused(reason),
        };

        match result_rx.recv_timeout(wait) {
            Ok(outcome) => self.finish(worker, outcome),
            Err(RecvTimeoutError::Timeout) => {
                self.abandoned.fetch_add(1, Ordering::AcqRel);
                if state
                    .compare_exchange(RUNNING, ABANDONED, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    token.cancel();
                    return Supervised::TimedOut;
                }
                // The worker finished between the timeout and the claim, so its
                // result is already on the way.
                self.abandoned.fetch_sub(1, Ordering::AcqRel);
                match result_rx.recv() {
                    Ok(outcome) => self.finish(worker, outcome),
                    Err(_) => Supervised::TimedOut,
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                resume_unwind(Box::new("detection worker exited without a result"))
            }
        }
    }

    fn finish<T>(
        &self,
        worker: Sender<Job>,
        outcome: Result<T, Box<dyn Any + Send>>,
    ) -> Supervised<T> {
        match outcome {
            Ok(value) => {
                if let Ok(mut idle) = self.idle.lock() {
                    idle.push(worker);
                }
                Supervised::Completed(value)
            }
            Err(payload) => resume_unwind(payload),
        }
    }

    /// Hands `job` to an idle worker, spawning one when none is free. Returns
    /// the sender that keeps the worker thread alive.
    fn dispatch(&self, mut job: Job) -> Result<Sender<Job>, String> {
        loop {
            let idle = self.idle.lock().ok().and_then(|mut idle| idle.pop());
            let Some(worker) = idle else { break };
            match worker.send(job) {
                Ok(()) => return Ok(worker),
                Err(returned) => job = returned.0,
            }
        }

        let (tx, rx) = channel::<Job>();
        thread::Builder::new()
            .name("provenant-detect".to_string())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    job();
                }
            })
            .map_err(|err| format!("could not start a detection worker: {err}"))?;
        tx.send(job)
            .map_err(|_| "detection worker exited before accepting work".to_string())?;
        Ok(tx)
    }
}

#[cfg(test)]
#[path = "supervisor_test.rs"]
mod tests;
