// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use super::{Supervised, WorkerPool};
use crate::cancel::CancelToken;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let give_up = Instant::now() + Duration::from_secs(10);
    while Instant::now() < give_up {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn completed_job_returns_its_value_and_reuses_the_worker() {
    let pool = WorkerPool::new(4);
    let token = CancelToken::new(None);

    let first = pool.run(&token, Duration::from_secs(5), || thread::current().id());
    let second = pool.run(&token, Duration::from_secs(5), || thread::current().id());

    let (Supervised::Completed(first), Supervised::Completed(second)) = (first, second) else {
        panic!("both jobs should complete");
    };
    assert_eq!(first, second);
    assert_eq!(pool.abandoned_workers(), 0);
}

#[test]
fn job_that_never_polls_is_abandoned_within_the_wait() {
    let pool = WorkerPool::new(4);
    let token = CancelToken::new(None);
    let started = Instant::now();

    let outcome = pool.run(&token, Duration::from_millis(200), || {
        thread::sleep(Duration::from_secs(2));
        "late"
    });

    assert!(matches!(outcome, Supervised::TimedOut));
    assert!(started.elapsed() < Duration::from_millis(1500));
    assert!(token.is_cancelled());
    assert_eq!(pool.abandoned_workers(), 1);
    assert!(
        wait_until(|| pool.abandoned_workers() == 0),
        "the abandoned slot is released once the worker finishes"
    );
}

#[test]
fn abandoned_worker_is_replaced_and_does_not_block_later_jobs() {
    let pool = WorkerPool::new(4);
    let token = CancelToken::new(None);

    let stuck = pool.run(&token, Duration::from_millis(100), || {
        thread::sleep(Duration::from_secs(2));
    });
    assert!(matches!(stuck, Supervised::TimedOut));

    let started = Instant::now();
    let next = pool.run(&CancelToken::new(None), Duration::from_secs(5), || 7);

    assert!(matches!(next, Supervised::Completed(7)));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn polling_job_stops_promptly_once_abandoned() {
    let pool = WorkerPool::new(4);
    let token = CancelToken::new(None);
    let worker_token = token.clone();

    let outcome = pool.run(&token, Duration::from_millis(100), move || {
        while !worker_token.is_cancelled() {
            thread::sleep(Duration::from_millis(1));
        }
    });

    assert!(matches!(outcome, Supervised::TimedOut));
    assert!(wait_until(|| pool.abandoned_workers() == 0));
}

#[test]
fn abandoned_workers_are_capped_and_further_jobs_are_refused() {
    let pool = WorkerPool::new(2);

    for _ in 0..2 {
        let outcome = pool.run(&CancelToken::new(None), Duration::from_millis(50), || {
            thread::sleep(Duration::from_secs(2));
        });
        assert!(matches!(outcome, Supervised::TimedOut));
    }

    let refused = pool.run(&CancelToken::new(None), Duration::from_secs(5), || 1);
    let Supervised::Refused(reason) = refused else {
        panic!("the third job should be refused at the cap");
    };
    assert!(reason.contains("limit 2"), "{reason}");
    assert!(wait_until(|| pool.abandoned_workers() == 0));

    let recovered = pool.run(&CancelToken::new(None), Duration::from_secs(5), || 1);
    assert!(matches!(recovered, Supervised::Completed(1)));
}

#[test]
fn worker_panic_propagates_to_the_caller() {
    let pool = WorkerPool::new(4);

    let result = catch_unwind(AssertUnwindSafe(|| {
        pool.run(&CancelToken::new(None), Duration::from_secs(5), || {
            panic!("boom");
        })
    }));

    assert!(result.is_err());
    assert_eq!(pool.abandoned_workers(), 0);
}

#[test]
fn concurrent_overshoot_is_bounded_by_the_caller_count() {
    let pool = Arc::new(WorkerPool::new(2));
    let callers = 4;
    let start = Arc::new(Barrier::new(callers));
    let release = Arc::new(AtomicBool::new(false));

    let handles: Vec<_> = (0..callers)
        .map(|_| {
            let pool = Arc::clone(&pool);
            let start = Arc::clone(&start);
            let release = Arc::clone(&release);
            thread::spawn(move || {
                start.wait();
                pool.run(
                    &CancelToken::new(None),
                    Duration::from_millis(50),
                    move || {
                        while !release.load(Ordering::Acquire) {
                            thread::sleep(Duration::from_millis(5));
                        }
                    },
                );
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("caller thread");
    }

    assert!(pool.abandoned_workers() <= 2 + callers);
    let refused = pool.run(&CancelToken::new(None), Duration::from_secs(5), || 1);
    assert!(matches!(refused, Supervised::Refused(_)));

    release.store(true, Ordering::Release);
    assert!(wait_until(|| pool.abandoned_workers() == 0));
}
