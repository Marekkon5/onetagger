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
pub(crate) struct ProviderCooldown(Mutex<Option<(Instant, Duration)>>);

impl ProviderCooldown {
    pub(crate) const fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub(crate) fn check(&self) -> Result<(), ProviderRateLimited> {
        let pause = self.0.lock().unwrap();
        if let Some((since, delay)) = *pause {
            let remaining = delay.saturating_sub(since.elapsed());
            if !remaining.is_zero() {
                return Err(ProviderRateLimited { delay: remaining, deferred: true });
            }
        }
        Ok(())
    }

    pub(crate) fn pause(&self, delay: Duration) -> ProviderRateLimited {
        let mut pause = self.0.lock().unwrap();
        let remaining = pause.map(|(since, delay)| delay.saturating_sub(since.elapsed())).unwrap_or_default();
        // A response from another in-flight request can extend a pause.
        let delay = delay.max(Duration::from_secs(1)).max(remaining);
        *pause = Some((Instant::now(), delay));
        ProviderRateLimited { delay, deferred: false }
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
        *cooldown.0.lock().unwrap() = Some((Instant::now(), Duration::ZERO));
        assert!(cooldown.check().is_ok());
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
