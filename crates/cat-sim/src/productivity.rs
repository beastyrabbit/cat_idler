//! Small deterministic productivity effect from accumulated reusable tools.
//!
//! Tools are non-consumable capital: each whole tool helps a little, up to a
//! deliberately low cap so tools improve work without replacing workers.

/// Each whole banked tool adds two percent throughput.
pub const TOOL_PRODUCTIVITY_PER_TOOL: f64 = 0.02;
/// Only the first ten tools contribute, capping the multiplier at 1.20.
pub const TOOL_PRODUCTIVITY_CAP: u32 = 10;

/// Deterministic throughput multiplier for construction, crafting, quarrying and
/// hauling. Fractional, negative and non-finite inventory never creates extra power.
#[must_use]
pub fn tool_productivity_multiplier(tools: f64) -> f64 {
    let whole = if tools.is_finite() {
        tools.max(0.0).floor() as u32
    } else {
        0
    };
    1.0 + f64::from(whole.min(TOOL_PRODUCTIVITY_CAP)) * TOOL_PRODUCTIVITY_PER_TOOL
}

/// Scale elapsed productive time while keeping the same deterministic boundary
/// semantics as the underlying cycle functions.
#[must_use]
pub fn productive_elapsed(elapsed: f64, tools: f64) -> f64 {
    elapsed * tool_productivity_multiplier(tools)
}

/// Shorten a job duration by tool throughput. Integer truncation is deliberate and
/// stable; all positive jobs retain at least one millisecond.
#[must_use]
pub fn productive_duration_ms(duration_ms: i64, tools: f64) -> i64 {
    if duration_ms <= 0 {
        return duration_ms;
    }
    ((duration_ms as f64 / tool_productivity_multiplier(tools)) as i64).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_have_exact_small_capped_boundaries() {
        assert_eq!(tool_productivity_multiplier(-1.0), 1.0);
        assert_eq!(tool_productivity_multiplier(0.999), 1.0);
        assert_eq!(tool_productivity_multiplier(1.0), 1.02);
        assert_eq!(tool_productivity_multiplier(9.999), 1.18);
        assert_eq!(tool_productivity_multiplier(10.0), 1.2);
        assert_eq!(tool_productivity_multiplier(10_000.0), 1.2);
        assert_eq!(tool_productivity_multiplier(f64::NAN), 1.0);
    }

    #[test]
    fn duration_and_elapsed_scaling_are_deterministic() {
        assert_eq!(productive_elapsed(100.0, 10.0), 120.0);
        assert_eq!(productive_duration_ms(1_200, 10.0), 1_000);
        assert_eq!(productive_duration_ms(1, 10.0), 1);
        assert_eq!(productive_duration_ms(0, 10.0), 0);
    }
}
