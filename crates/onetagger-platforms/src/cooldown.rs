use chrono::{DateTime, Utc};
use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ProviderRateLimited {
    delay: Duration,
    deferred: bool,
}

impl ProviderRateLimited {
    /// Signal a provider response with a retry delay, including from custom sources.
    pub fn retry_after(delay: Duration) -> Self {
        Self { delay, deferred: false }
    }

    pub fn retry_after_seconds(&self) -> u64 {
        self.delay.as_secs().saturating_add(u64::from(self.delay.subsec_nanos() > 0))
    }

    /// An existing cooldown stopped this lookup before it could make a request.
    pub fn deferred(&self) -> bool {
        self.deferred
    }
}

impl fmt::Display for ProviderRateLimited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider rate limited. Retry in {} s.", self.retry_after_seconds())
    }
}

impl std::error::Error for ProviderRateLimited {}

/// Share the requested pause across this provider's sources and worker slots.
struct CooldownState {
    pause: Option<(Instant, Duration)>,
    failures: u32,
}

pub(crate) struct ProviderCooldown(Mutex<CooldownState>);

impl ProviderCooldown {
    pub(crate) const fn new() -> Self {
        Self(Mutex::new(CooldownState { pause: None, failures: 0 }))
    }

    pub(crate) fn check(&self) -> Result<(), ProviderRateLimited> {
        let pause = self.0.lock().unwrap();
        if let Some((since, delay)) = pause.pause {
            let remaining = delay.saturating_sub(since.elapsed());
            if !remaining.is_zero() {
                return Err(ProviderRateLimited { delay: remaining, deferred: true });
            }
        }
        Ok(())
    }

    pub(crate) fn pause(&self, delay: Duration) -> ProviderRateLimited {
        let mut pause = self.0.lock().unwrap();
        let remaining = pause.pause.map(|(since, delay)| delay.saturating_sub(since.elapsed())).unwrap_or_default();
        // Repeated throttling must slow down further, including when a server
        // omits Retry-After. Never shorten a longer explicit server delay.
        let backoff = delay.max(Duration::from_secs(1))
            .saturating_mul(1 << pause.failures.min(6)).min(Duration::from_secs(600));
        pause.failures = pause.failures.saturating_add(1);
        // A response from another in-flight request can extend a pause.
        let delay = delay.max(backoff).max(Duration::from_secs(1)).max(remaining);
        pause.pause = Some((Instant::now(), delay));
        ProviderRateLimited::retry_after(delay)
    }

    pub(crate) fn record_success(&self) {
        let mut state = self.0.lock().unwrap();
        // An older in-flight success cannot undo another worker's active pause.
        if state.pause.is_none_or(|(since, delay)| since.elapsed() >= delay) {
            state.failures = 0;
        }
    }

    pub(crate) fn check_response(&self, response: &reqwest::blocking::Response, fallback: Duration)
        -> Result<(), ProviderRateLimited> {
        if matches!(response.status(), reqwest::StatusCode::TOO_MANY_REQUESTS | reqwest::StatusCode::SERVICE_UNAVAILABLE) {
            return Err(self.pause(retry_delay(response.headers().get(reqwest::header::RETRY_AFTER)
                .and_then(|header| header.to_str().ok()), fallback)));
        }
        if response.status().is_success() { self.record_success(); }
        Ok(())
    }
}

pub(crate) fn retry_delay(header: Option<&str>, fallback: Duration) -> Duration {
    let Some(header) = header.map(str::trim) else { return fallback; };
    if let Ok(seconds) = header.parse::<u64>() {
        return Duration::from_secs(seconds);
    }
    DateTime::parse_from_rfc2822(header).ok()
        .and_then(|date| date.signed_duration_since(Utc::now()).to_std().ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_defers_work_and_never_shortens_an_existing_pause() {
        let cooldown = ProviderCooldown::new();
        assert!(cooldown.check().is_ok());
        let initial = cooldown.pause(Duration::from_secs(60));
        assert!(!initial.deferred());
        assert_eq!(initial.retry_after_seconds(), 60);
        assert!(cooldown.check().unwrap_err().deferred());
        assert!(cooldown.pause(Duration::from_secs(2)).retry_after_seconds() > 2);

        // Move the deadline into the past without sleeping.
        cooldown.0.lock().unwrap().pause = Some((Instant::now(), Duration::ZERO));
        assert!(cooldown.check().is_ok());
    }

    #[test]
    fn repeated_throttling_backs_off_and_a_late_success_cannot_clear_a_pause() {
        let cooldown = ProviderCooldown::new();
        for seconds in [15, 30, 60] {
            assert_eq!(cooldown.pause(Duration::from_secs(15)).retry_after_seconds(), seconds);
            cooldown.record_success();
            assert!(cooldown.check().is_err());
        }
        assert_eq!(cooldown.pause(Duration::from_secs(1800)).retry_after_seconds(), 1800);
        cooldown.0.lock().unwrap().pause = None;
        cooldown.record_success();
        assert_eq!(cooldown.pause(Duration::from_secs(15)).retry_after_seconds(), 15);
    }

    #[test]
    fn retry_after_accepts_seconds_and_http_dates_with_a_fallback() {
        let fallback = Duration::from_secs(2);
        assert_eq!(retry_delay(Some("120"), fallback), Duration::from_secs(120));
        assert_eq!(retry_delay(None, fallback), fallback);
        assert_eq!(retry_delay(Some("invalid"), fallback), fallback);
        let later = (Utc::now() + chrono::Duration::minutes(2)).format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        assert!(retry_delay(Some(&later), fallback) >= Duration::from_secs(110));
    }
}
