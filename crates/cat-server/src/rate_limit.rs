//! Sliding-window action limiter ported from `lib/game/rateLimiter.ts`.
//!
//! The key store is deliberately bounded because keys are derived from untrusted
//! sessions and peer addresses on a long-lived public server.
//! At capacity, new keys are rejected until a window expires. Evicting live keys
//! would let identity churn reset an address's action or session-issuance budget.

use std::collections::HashMap;

const DEFAULT_MAX_KEYS: usize = 4096;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    max: usize,
    window_ms: i64,
    max_keys: usize,
    hits: HashMap<String, Vec<i64>>,
}

impl RateLimiter {
    #[must_use]
    pub fn new(max: usize, window_ms: i64) -> Self {
        Self::with_key_capacity(max, window_ms, DEFAULT_MAX_KEYS)
    }

    #[must_use]
    pub(crate) fn with_key_capacity(max: usize, window_ms: i64, max_keys: usize) -> Self {
        assert!(max_keys > 0, "rate limiter key capacity must be positive");
        Self {
            max,
            window_ms,
            max_keys,
            hits: HashMap::new(),
        }
    }

    pub fn check(&mut self, key: &str, now_ms: i64) -> bool {
        self.prune(now_ms);
        if !self.hits.contains_key(key) && self.hits.len() >= self.max_keys {
            return false;
        }
        let recent = self.hits.entry(key.to_owned()).or_default();
        if recent.len() >= self.max {
            return false;
        }
        recent.push(now_ms);
        true
    }

    fn prune(&mut self, now_ms: i64) {
        self.hits.retain(|_, hits| {
            hits.retain(|hit_ms| now_ms.saturating_sub(*hit_ms) < self.window_ms);
            !hits.is_empty()
        });
    }

    #[cfg(test)]
    fn key_count(&self) -> usize {
        self.hits.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_blocks_the_31st_action_in_10_seconds() {
        let mut limiter = RateLimiter::new(30, 10_000);

        for index in 0..30 {
            assert!(
                limiter.check("s:session-1", 1_000 + index),
                "action {index} should be allowed"
            );
        }
        assert!(!limiter.check("s:session-1", 2_000));
        assert!(limiter.check("s:session-1", 11_000));
    }

    #[test]
    fn extreme_clock_values_do_not_overflow_timestamp_arithmetic() {
        let mut limiter = RateLimiter::new(2, 10_000);

        assert!(limiter.check("session", i64::MAX));
        assert!(limiter.check("session", i64::MIN));
        assert!(!limiter.check("session", i64::MIN + 1));

        limiter.prune(i64::MIN);
        assert!(limiter.check("session", i64::MIN + 10_000));
    }

    #[test]
    fn full_key_store_rejects_new_keys_without_resetting_active_limits() {
        let mut limiter = RateLimiter::with_key_capacity(1, 10_000, 2);
        assert!(limiter.check("a", 0));
        assert!(limiter.check("b", 1));
        assert!(!limiter.check("a", 2));
        assert!(!limiter.check("c", 3));
        assert!(!limiter.check("a", 4));
        assert_eq!(limiter.key_count(), 2);
    }

    #[test]
    fn capacity_recovers_at_exact_expiry_without_forgetting_other_keys() {
        let mut limiter = RateLimiter::with_key_capacity(1, 100, 2);
        assert!(limiter.check("expired", 0));
        assert!(limiter.check("active", 50));
        assert!(!limiter.check("next", 99));
        assert!(limiter.check("next", 100));
        assert!(!limiter.check("active", 100));
        assert_eq!(limiter.key_count(), 2);

        assert!(limiter.check("next", 200));
        assert_eq!(limiter.key_count(), 1);
        assert!(!limiter.check("next", 201));
    }

    #[test]
    fn an_existing_key_can_use_its_remaining_budget_at_capacity() {
        let mut limiter = RateLimiter::with_key_capacity(2, 100, 1);
        assert!(limiter.check("known", 0));
        assert!(!limiter.check("new", 1));
        assert!(limiter.check("known", 2));
        assert!(!limiter.check("known", 3));
        assert_eq!(limiter.key_count(), 1);
    }
}
