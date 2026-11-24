//! Bounded, short-lived reuse of identical successful provider detail lookups.
//! The map lock never covers network I/O; only callers for the same key wait.
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::sync::{Arc, Mutex, TryLockError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

type Slot<V> = Arc<Mutex<Option<(Instant, V)>>>;

struct Inner<K, V> {
    entries: Mutex<HashMap<K, Slot<V>>>,
    capacity: usize,
    lifetime: Duration,
    hits: AtomicUsize,
    fetches: AtomicUsize,
}

pub(crate) struct RequestCache<K, V>(Arc<Inner<K, V>>);

impl<K, V> Clone for RequestCache<K, V> {
    fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<K, V> fmt::Debug for RequestCache<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never expose cached provider data, keys, credentials or tokens.
        f.debug_struct("RequestCache")
            .field("hits", &self.0.hits.load(Ordering::Relaxed))
            .field("fetches", &self.0.fetches.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl<K, V> Default for RequestCache<K, V> {
    fn default() -> Self { Self::new(256, Duration::from_secs(600)) }
}

impl<K, V> RequestCache<K, V> {
    pub(crate) fn new(capacity: usize, lifetime: Duration) -> Self {
        Self(Arc::new(Inner { entries: Mutex::new(HashMap::new()), capacity, lifetime,
            hits: AtomicUsize::new(0), fetches: AtomicUsize::new(0) }))
    }
}

impl<K: Eq + Hash + Clone, V: Clone> RequestCache<K, V> {
    pub(crate) fn get_or_try_init<E>(&self, key: K, fetch: impl FnOnce() -> Result<V, E>) -> Result<V, E> {
        let slot = {
            let mut entries = self.0.entries.lock().unwrap_or_else(|error| error.into_inner());
            if let Some(slot) = entries.get(&key) { Some(slot.clone()) }
            else {
                if entries.len() >= self.0.capacity {
                    // Evict an idle entry only. In-flight keys keep a single owner
                    // even under pressure, and callers for other keys can progress.
                    let idle = entries.iter().find_map(|(key, slot)|
                        (Arc::strong_count(slot) == 1).then(|| key.clone()));
                    if let Some(key) = idle { entries.remove(&key); }
                }
                if entries.len() < self.0.capacity {
                    let slot = Arc::new(Mutex::new(None));
                    entries.insert(key, slot.clone()); Some(slot)
                } else { None }
            }
        };
        let Some(slot) = slot else {
            self.0.fetches.fetch_add(1, Ordering::Relaxed);
            return fetch();
        };
        // A panicking loader must not strand waiters or poison this key forever.
        let mut cached = match slot.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
            Err(TryLockError::WouldBlock) => slot.lock().unwrap_or_else(|error| error.into_inner()),
        };
        if let Some((since, value)) = cached.as_ref() {
            if since.elapsed() < self.0.lifetime {
                self.0.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(value.clone());
            }
        }
        *cached = None;
        self.0.fetches.fetch_add(1, Ordering::Relaxed);
        // Store only fully validated successes. Errors keep their original type
        // (including ProviderRateLimited), and the next call may retry normally.
        let value = fetch()?;
        *cached = Some((Instant::now(), value.clone()));
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Barrier, mpsc};
    use std::thread;

    #[test]
    fn concurrent_identical_release_requests_fetch_once_without_sharing_mutations() {
        let cache = RequestCache::new(8, Duration::from_secs(60));
        let gate = Barrier::new(16);
        let calls = AtomicUsize::new(0);
        thread::scope(|scope| {
            let mut workers = vec![];
            for _ in 0..16 {
                workers.push(scope.spawn(|| {
                    gate.wait();
                    let mut release = cache.get_or_try_init("release-1", || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(10));
                        Ok::<_, ()>(vec!["original metadata"])
                    }).unwrap();
                    release.push("per-track change"); release
                }));
            }
            for worker in workers { assert_eq!(worker.join().unwrap().len(), 2); }
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.0.hits.load(Ordering::Relaxed), 15);
    }

    #[test]
    fn unrelated_keys_are_not_blocked_by_a_slow_release() {
        let cache = RequestCache::new(8, Duration::from_secs(60));
        let (start_tx, start_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        let (done_tx, done_rx) = mpsc::channel();
        thread::scope(|scope| {
            scope.spawn(|| cache.get_or_try_init(1, || {
                start_tx.send(()).unwrap();
                release_rx.lock().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
                Ok::<_, ()>(1)
            }).unwrap());
            start_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            scope.spawn(|| done_tx.send(cache.get_or_try_init(2, || Ok::<_, ()>(2))).unwrap());
            let result = done_rx.recv_timeout(Duration::from_secs(2));
            release_tx.send(()).unwrap();
            assert_eq!(result.unwrap(), Ok(2));
        });
    }

    #[test]
    fn errors_expiration_and_overflow_do_not_reuse_failed_or_stale_values() {
        let cache = RequestCache::new(2, Duration::from_secs(60));
        assert_eq!(cache.get_or_try_init("one", || Err::<u8, _>("rate limited")), Err("rate limited"));
        assert_eq!(cache.get_or_try_init("one", || Ok::<_, ()>(1)), Ok(1));
        let slot = cache.0.entries.lock().unwrap()["one"].clone();
        slot.lock().unwrap().as_mut().unwrap().0 = Instant::now() - Duration::from_secs(61);
        drop(slot);
        assert_eq!(cache.get_or_try_init("one", || Ok::<_, ()>(2)), Ok(2));
        for key in ["two", "three", "four"] {
            cache.get_or_try_init(key, || Ok::<_, ()>(3)).unwrap();
            assert!(cache.0.entries.lock().unwrap().len() <= 2);
        }
        let disabled = RequestCache::new(0, Duration::ZERO);
        for value in [1, 2] { assert_eq!(disabled.get_or_try_init(1, || Ok::<_, ()>(value)), Ok(value)); }
        assert!(disabled.0.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn endpoint_and_context_keys_remain_separate_and_debug_is_redacted() {
        let cache = RequestCache::default();
        for key in [("release", 42, "en"), ("master", 42, "en"), ("release", 42, "de")] {
            assert_eq!(cache.get_or_try_init(key, || Ok::<_, ()>(key)), Ok(key));
        }
        let debug = format!("{cache:?}");
        assert!(!debug.contains("release")); assert!(!debug.contains("42"));
    }

    #[test]
    fn a_panicking_fetch_does_not_strand_the_key() {
        let cache = RequestCache::<u8, u8>::default();
        assert!(std::panic::catch_unwind(|| {
            cache.get_or_try_init(1, || -> Result<u8, ()> { panic!("loader failed") }).ok();
        }).is_err());
        assert_eq!(cache.get_or_try_init(1, || Ok::<_, ()>(9)), Ok(9));
    }
}
