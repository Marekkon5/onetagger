//! One request-start budget shared by a provider's workers and UI sessions.
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(crate) struct RequestPacer(Mutex<Option<Instant>>);

impl RequestPacer {
    pub(crate) const fn new() -> Self { Self(Mutex::new(None)) }

    /// Pace starts, allowing network time to consume part of the interval.
    /// A negative custom per-minute limit disables proactive pacing only.
    pub(crate) fn wait_per_minute(&self, requests: i16) {
        if requests == -1 { return; }
        self.wait(Duration::from_secs_f64(60.0 / f64::from(requests.max(1))));
    }

    pub(crate) fn wait(&self, interval: Duration) -> Instant {
        loop {
            let remaining = {
                let mut last = self.0.lock().unwrap_or_else(|error| error.into_inner());
                let remaining = last.map(|start| interval.saturating_sub(start.elapsed())).unwrap_or_default();
                if remaining.is_zero() {
                    let started = Instant::now(); *last = Some(started); return started;
                }
                remaining
            };
            // Do not hold a mutex during either sleeping or HTTP work.
            std::thread::sleep(remaining);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_starts_share_a_budget_and_response_time_is_not_added_twice() {
        let pacer = RequestPacer::new();
        let interval = Duration::from_millis(5);
        let mut starts = vec![];
        for _ in 0..3 {
            starts.push(pacer.wait(interval));
        }
        assert!(starts.windows(2).all(|pair| pair[1].duration_since(pair[0]) >= interval));
        *pacer.0.lock().unwrap() = Some(Instant::now() - Duration::from_secs(5));
        pacer.wait(interval);
        assert!(pacer.0.lock().unwrap().unwrap().elapsed() < Duration::from_secs(2));
        let before = *pacer.0.lock().unwrap();
        pacer.wait_per_minute(-1);
        assert_eq!(*pacer.0.lock().unwrap(), before);
    }

    #[test]
    fn parallel_workers_cannot_multiply_the_provider_request_budget() {
        let pacer = RequestPacer::new();
        let interval = Duration::from_millis(2);
        let mut starts = std::thread::scope(|scope| {
            (0..12).map(|_| scope.spawn(|| pacer.wait(interval)))
                .collect::<Vec<_>>().into_iter().map(|worker| worker.join().unwrap()).collect::<Vec<_>>()
        });
        starts.sort();
        assert!(starts.windows(2).all(|pair| pair[1].duration_since(pair[0]) >= interval));
    }
}
