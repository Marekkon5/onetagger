//! Synthetic provider waits through the actual production scheduler. No live
//! service calls, credentials, audio dependencies, or provider quotas required.
#[path = "../crates/onetagger-autotag/src/scheduler.rs"]
mod scheduler;

use scheduler::{Attempt, Schedule};
use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex};
use std::time::{Duration, Instant};

fn matched(case: &str, stage: usize, item: usize) -> bool {
    match case {
        "first_provider_hits" | "multiplatform" => true,
        "all_miss" => false,
        "provider_errors" => stage == 2 && item % 4 != 0,
        _ => stage == 1 || item % 4 == 0,
    }
}

fn delay(case: &str, stage: usize, item: usize) -> u64 {
    match case {
        "slow_tail" if stage == 0 && item == 63 => 150,
        "limited_provider" if stage == 0 => 4,
        "limited_provider" => 16,
        "provider_errors" if stage == 0 => 8,
        _ => 3,
    }
}

fn trial(case: &str, schedule: Schedule, trial: usize) {
    let items: Vec<usize> = (0..64).collect();
    let limits = if case == "limited_provider" { [1, 4, 2] } else { [4, 4, 2] };
    let multiplatform = case == "multiplatform";
    let calls = AtomicUsize::new(0);
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let stage_active = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
    let mut histories = vec![vec![]; items.len()];
    let mut terminal_times = vec![];
    let started = Instant::now();
    let mut completed_before = 0;
    scheduler::run(&items, &limits, 4, multiplatform, schedule, || false,
        |stage, _, item| {
            calls.fetch_add(1, Ordering::SeqCst);
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            assert!(now <= 4);
            assert!(stage_active[stage].fetch_add(1, Ordering::SeqCst) < limits[stage]);
            std::thread::sleep(Duration::from_millis(delay(case, stage, *item)));
            stage_active[stage].fetch_sub(1, Ordering::SeqCst);
            active.fetch_sub(1, Ordering::SeqCst);
            Attempt { value: stage, matched: matched(case, stage, *item) }
        }, |stage, item, result, completed| {
            assert_eq!(result.unwrap(), stage);
            histories[item].push(stage);
            assert!(completed >= completed_before);
            if completed > completed_before { terminal_times.push(started.elapsed().as_secs_f64() * 1000.0); }
            completed_before = completed;
        });
    let elapsed = started.elapsed().as_secs_f64();
    let expected: Vec<Vec<usize>> = items.iter().map(|item| {
        let last = if multiplatform { 2 } else { (0..3).find(|s| matched(case, *s, *item)).unwrap_or(2) };
        (0..=last).collect()
    }).collect();
    assert_eq!(histories, expected, "provider order or first-match semantics changed");
    let expected_calls: usize = expected.iter().map(Vec::len).sum();
    assert_eq!(calls.load(Ordering::SeqCst), expected_calls, "speculative requests added");
    assert_eq!(terminal_times.len(), items.len());
    assert_eq!(active.load(Ordering::SeqCst), 0);
    println!("{case},{schedule:?},{trial},{:.3},{:.3},{:.3},{:.3},{expected_calls},{},0",
        elapsed * 1000.0, items.len() as f64 / elapsed, terminal_times[0], terminal_times[60],
        peak.load(Ordering::SeqCst));
}

fn cancellation(schedule: Schedule, trial: usize) {
    let stopped = AtomicBool::new(false);
    let cancelled_at = Mutex::new(None);
    let calls = AtomicUsize::new(0);
    let started = Instant::now();
    scheduler::run(&(0..64).collect::<Vec<_>>(), &[4, 4], 4, false, schedule,
        || stopped.load(Ordering::SeqCst), |stage, _, item| {
            assert_eq!(stage, 0, "a queued fallback ran after Stop");
            calls.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(if *item == 0 { 8 } else { 32 }));
            Attempt { value: (), matched: false }
        }, |_, _, value, _| {
            value.unwrap();
            if !stopped.swap(true, Ordering::SeqCst) { *cancelled_at.lock().unwrap() = Some(Instant::now()); }
        });
    assert!(calls.load(Ordering::SeqCst) <= 4);
    let drain = cancelled_at.lock().unwrap().unwrap().elapsed().as_secs_f64() * 1000.0;
    println!("cancellation,{schedule:?},{trial},{:.3},0,0,0,{},0,{drain:.3}",
        started.elapsed().as_secs_f64() * 1000.0, calls.load(Ordering::SeqCst));
}

fn main() {
    println!("case,schedule,trial,elapsed_ms,tracks_per_second,first_terminal_ms,p95_terminal_ms,requests,peak_active,cancel_drain_ms");
    // Alternate order to reduce systematic warm-up / runner-load bias.
    for trial_number in 0..5 {
        let modes = if trial_number % 2 == 0 { [Schedule::Batched, Schedule::Pipelined] }
            else { [Schedule::Pipelined, Schedule::Batched] };
        for mode in modes {
            for case in ["first_provider_hits", "slow_tail", "limited_provider", "all_miss", "provider_errors", "multiplatform"] {
                trial(case, mode, trial_number);
            }
            cancellation(mode, trial_number);
        }
    }
}
