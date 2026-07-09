//! Sliding-window action limiter ported from `lib/game/rateLimiter.ts`.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    max: usize,
    window_ms: i64,
    hits: HashMap<String, Vec<i64>>,
}

impl RateLimiter {
    #[must_use]
    pub fn new(max: usize, window_ms: i64) -> Self {
        Self {
            max,
            window_ms,
            hits: HashMap::new(),
        }
    }

    pub fn check(&mut self, key: &str, now_ms: i64) -> bool {
        let recent = self.hits.entry(key.to_owned()).or_default();
        recent.retain(|hit_ms| now_ms - *hit_ms < self.window_ms);
        if recent.len() >= self.max {
            return false;
        }
        recent.push(now_ms);
        true
    }

    pub fn prune(&mut self, now_ms: i64) {
        self.hits.retain(|_, hits| {
            hits.retain(|hit_ms| now_ms - *hit_ms < self.window_ms);
            !hits.is_empty()
        });
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
}
