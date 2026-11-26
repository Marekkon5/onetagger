//! Bounded, ordered fallback scheduling. No provider or audio dependencies so
//! the exact production scheduler can also run in CI's standalone benchmarks.
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Schedule {
    Pipelined,
    /// Successive files start at successive providers, then wrap in configured order.
    Rotating,
    /// Reference provider-wide barrier, used by the comparative CI benchmark.
    #[allow(dead_code)]
    Batched,
}

pub struct Attempt<R> {
    pub value: R,
    pub matched: bool,
}

struct Job {
    item: usize,
    stage: usize,
    slot: usize,
}

/// A file has at most one active job. Each provider owns `limits[stage]`
/// reusable source slots; all providers share at most `concurrency` workers.
/// `observe` runs serially, after each attempt, with a monotonic terminal-file
/// count. Err(()) means a provider panicked; it may still fall back normally.
/// Cancellation stops dispatch and joins in-flight calls before returning.
pub fn run<T, R, F, O, C>(
    items: &[T], limits: &[usize], concurrency: usize, multiplatform: bool,
    schedule: Schedule, cancelled: C, work: F, mut observe: O,
) where
    T: Sync,
    R: Send,
    F: Fn(usize, usize, &T) -> Attempt<R> + Sync,
    O: FnMut(usize, usize, Result<R, ()>, usize),
    C: Fn() -> bool + Sync,
{
    if items.is_empty() || limits.is_empty() || cancelled() { return; }
    let workers = concurrency.max(1).min(items.len());
    let stages = limits.len();
    let mut pending = vec![VecDeque::new(); stages];
    let first_stage = |item: usize| if schedule == Schedule::Rotating { item % stages } else { 0 };
    for item in 0..items.len() { pending[first_stage(item)].push_back(item); }
    let mut slots: Vec<Vec<usize>> = limits.iter()
        .map(|limit| (0..(*limit).max(1).min(workers)).collect()).collect();
    let mut free: Vec<usize> = (0..workers).collect();
    let mut active = vec![0usize; stages];
    let mut inflight = 0;
    let mut completed = 0;
    let mut cursor = 0;

    thread::scope(|scope| {
        let (done_tx, done_rx) = mpsc::channel();
        let mut senders = Vec::with_capacity(workers);
        for worker in 0..workers {
            // One mailbox per worker; dispatcher never queues speculative work.
            let (tx, rx) = mpsc::sync_channel::<Job>(1);
            senders.push(tx);
            let done_tx = done_tx.clone();
            let work = &work;
            let cancelled = &cancelled;
            scope.spawn(move || {
                while let Ok(job) = rx.recv() {
                    let result = if cancelled() { None } else {
                        Some(catch_unwind(AssertUnwindSafe(||
                            work(job.stage, job.slot, &items[job.item])
                        )).map_err(|_| ()))
                    };
                    if done_tx.send((worker, job, result)).is_err() { break; }
                }
            });
        }
        drop(done_tx);

        loop {
            while !free.is_empty() && !cancelled() {
                let stage = if schedule == Schedule::Batched {
                    // Do not enter the next provider until this one drains.
                    (0..stages).find(|&s| active[s] > 0 || !pending[s].is_empty())
                        .filter(|&s| !pending[s].is_empty() && !slots[s].is_empty())
                } else {
                    // Round robin prevents the initial queue from starving
                    // ready fallbacks, even when it contains thousands of files.
                    (0..stages).map(|offset| (cursor + offset) % stages)
                        .find(|&s| !pending[s].is_empty() && !slots[s].is_empty())
                };
                let Some(stage) = stage else { break; };
                let worker = free.pop().unwrap();
                let job = Job { item: pending[stage].pop_front().unwrap(),
                    stage, slot: slots[stage].pop().unwrap() };
                senders[worker].send(job).expect("scheduler worker exited without a result");
                active[stage] += 1;
                inflight += 1;
                cursor = (stage + 1) % stages;
            }
            if inflight == 0 { break; }
            let (worker, job, result) = done_rx.recv().expect("scheduler workers disconnected");
            free.push(worker);
            slots[job.stage].push(job.slot);
            active[job.stage] -= 1;
            inflight -= 1;
            if let Some(result) = result {
                let matched = result.as_ref().is_ok_and(|r| r.matched);
                let next = (job.stage + 1) % stages;
                let terminal = (matched && !multiplatform) || next == first_stage(job.item);
                if terminal { completed += 1; }
                observe(job.stage, job.item, result.map(|r| r.value), completed);
                if !terminal && !cancelled() {
                    pending[next].push_back(job.item);
                }
            }
        }
        // All active operations have returned. Closing mailboxes joins workers;
        // no source access or file writes can outlive the run's final report.
        drop(senders);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex, Condvar};
    use std::time::Duration;

    #[test]
    fn rotation_admits_another_provider_while_the_first_is_blocked() {
        let gate = (Mutex::new(false), Condvar::new());
        let mut events = Vec::new();
        run(&[0, 1], &[1, 1], 2, false, Schedule::Rotating, || false,
            |stage, _, item| {
                if *item == 0 {
                    let (open, _) = gate.1.wait_timeout_while(gate.0.lock().unwrap(), Duration::from_secs(3), |open| !*open).unwrap();
                    assert!(*open, "the other provider should complete independently");
                }
                Attempt { value: stage, matched: true }
            }, |stage, item, result, _| {
                events.push((item, result.unwrap()));
                if stage == 1 { *gate.0.lock().unwrap() = true; gate.1.notify_one(); }
            });
        assert_eq!(events, [(1, 1), (0, 0)]);
    }

    #[test]
    fn rotation_wraps_each_file_once_and_finishes_at_the_total() {
        let mut visits = vec![Vec::new(); 5];
        let mut progress = Vec::new();
        run(&[0; 5], &[1, 1, 1], 4, true, Schedule::Rotating, || false,
            |_, _, _| Attempt { value: (), matched: true },
            |stage, item, result, n| { result.unwrap(); visits[item].push(stage); progress.push(n); });
        for (item, stages) in visits.iter().enumerate() {
            assert_eq!(*stages, [item % 3, (item + 1) % 3, (item + 2) % 3]);
        }
        assert!(progress.is_sorted());
        assert_eq!(progress.last(), Some(&5));
    }

    #[test]
    fn fallback_starts_before_the_first_provider_finishes() {
        let gate = (Mutex::new(false), Condvar::new());
        let mut events = vec![];
        run(&[0, 1], &[2, 1], 2, false, Schedule::Pipelined, || false,
            |stage, _, item| {
                if stage == 0 && *item == 1 {
                    let (released, _) = gate.1.wait_timeout_while(gate.0.lock().unwrap(),
                        Duration::from_secs(5), |released| !*released).unwrap();
                    assert!(*released, "fallback was blocked by the provider batch barrier");
                }
                if stage == 1 {
                    *gate.0.lock().unwrap() = true;
                    gate.1.notify_all();
                }
                Attempt { value: (stage, *item), matched: stage == 1 }
            }, |_, _, value, _| events.push(value.unwrap()));
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn preserves_precedence_outcomes_limits_and_monotonic_progress() {
        for multiplatform in [false, true] {
            let mut reference = None;
            for mode in [Schedule::Batched, Schedule::Pipelined] {
                let active = AtomicUsize::new(0);
                let per_stage = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
                let limits = [1, 2, 3];
                let items: Vec<usize> = (0..30).collect();
                let paths = Mutex::new(vec![false; items.len()]);
                let mut histories = vec![vec![]; items.len()];
                let mut progress = 0;
                run(&items, &limits, 4, multiplatform, mode, || false,
                    |stage, _, item| {
                        assert!(active.fetch_add(1, Ordering::SeqCst) < 4);
                        assert!(per_stage[stage].fetch_add(1, Ordering::SeqCst) < limits[stage]);
                        { let mut paths = paths.lock().unwrap(); assert!(!paths[*item]); paths[*item] = true; }
                        thread::sleep(Duration::from_millis(1));
                        paths.lock().unwrap()[*item] = false;
                        per_stage[stage].fetch_sub(1, Ordering::SeqCst);
                        active.fetch_sub(1, Ordering::SeqCst);
                        Attempt { value: stage, matched: *item % 4 == stage }
                    }, |stage, item, value, completed| {
                        assert_eq!(value.unwrap(), stage);
                        assert!(completed >= progress && completed <= items.len());
                        progress = completed;
                        histories[item].push(stage);
                    });
                assert_eq!(progress, items.len());
                assert_eq!(active.load(Ordering::SeqCst), 0);
                for (item, history) in histories.iter().enumerate() {
                    let last = if multiplatform { 2 } else { (item % 4).min(2) };
                    assert_eq!(*history, (0..=last).collect::<Vec<_>>());
                }
                if let Some(reference) = &reference { assert_eq!(&histories, reference); }
                reference = Some(histories);
            }
        }
    }

    #[test]
    fn stop_drains_active_work_and_does_not_start_queued_fallbacks() {
        let stopped = AtomicBool::new(false);
        let active = AtomicUsize::new(0);
        let entered = AtomicUsize::new(0);
        run(&(0..100).collect::<Vec<_>>(), &[2, 2], 2, false, Schedule::Pipelined,
            || stopped.load(Ordering::SeqCst), |stage, _, _| {
                assert_eq!(stage, 0);
                entered.fetch_add(1, Ordering::SeqCst);
                active.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(5));
                active.fetch_sub(1, Ordering::SeqCst);
                Attempt { value: (), matched: false }
            }, |_, _, _, _| { stopped.store(true, Ordering::SeqCst); });
        assert!(entered.load(Ordering::SeqCst) <= 2);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn panic_does_not_lose_a_slot_or_hang_and_can_fall_back() {
        let mut successes = 0;
        let mut failures = 0;
        run(&[0, 1, 2], &[1, 1], 2, false, Schedule::Pipelined, || false,
            |stage, _, _| {
                if stage == 0 { panic!("mock provider panic"); }
                Attempt { value: (), matched: true }
            }, |stage, _, value, _| {
                if stage == 0 { assert!(value.is_err()); failures += 1; }
                else { assert!(value.is_ok()); successes += 1; }
            });
        assert_eq!((failures, successes), (3, 3));
    }

    #[test]
    fn empty_and_precancelled_runs_do_no_work_and_zero_threads_is_safe() {
        for (items, limits, stop) in [(vec![], vec![1], false), (vec![0], vec![], false),
            (vec![0], vec![1], true)] {
            run(&items, &limits, 1, false, Schedule::Pipelined, || stop,
                |_, _, _| -> Attempt<()> { panic!("unexpected work") }, |_, _, _, _| panic!("unexpected result"));
        }
        let mut count = 0;
        run(&[0], &[0], 0, false, Schedule::Pipelined, || false,
            |_, _, _| Attempt { value: (), matched: true }, |_, _, result, completed| {
                result.unwrap(); count = completed;
            });
        assert_eq!(count, 1);
    }
}
