//! Multi-trip gathering rules ported from `lib/game/trips.ts`.

/// Total hauls per hunt: two mid-job trips plus the completion haul.
pub const HUNT_TRIP_COUNT: i32 = 3;

/// Integer share for one trip; earlier trips carry the remainder.
#[must_use]
pub fn split_yield(total: f64, trip_count: i32, trip_index: i32) -> f64 {
    let whole = total.floor();
    let count = f64::from(trip_count.max(1));
    let base = (whole / count).floor();
    let bonus_trips = whole % count;

    base + if f64::from(trip_index) < bonus_trips {
        1.0
    } else {
        0.0
    }
}

/// Yield still at the site after `trips_done` shares have been hauled.
#[must_use]
pub fn remaining_yield(total: f64, trip_count: i32, trips_done: i32) -> f64 {
    let mut hauled = 0.0;
    for trip_index in 0..trips_done.min(trip_count) {
        hauled += split_yield(total, trip_count, trip_index);
    }

    js_max(0.0, total.floor() - hauled)
}

/// When mid-trip `trip_index` departs for the shrine, using hunt trip count.
#[must_use]
pub fn trip_due_at(started_at: f64, ends_at: f64, trip_index: i32) -> f64 {
    trip_due_at_with_count(started_at, ends_at, trip_index, HUNT_TRIP_COUNT)
}

#[must_use]
pub fn trip_due_at_with_count(
    started_at: f64,
    ends_at: f64,
    trip_index: i32,
    trip_count: i32,
) -> f64 {
    let duration = js_max(1.0, ends_at - started_at);
    started_at + (duration * f64::from(trip_index)) / f64::from(trip_count.max(1))
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left >= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HUNT_TRIP_COUNT, remaining_yield, split_yield, trip_due_at, trip_due_at_with_count,
    };

    #[test]
    fn constants_match_ts() {
        assert_eq!(HUNT_TRIP_COUNT, 3);
    }

    #[test]
    fn split_yield_floors_total_and_gives_remainder_to_earlier_trips() {
        assert_eq!(split_yield(10.0, 3, 0), 4.0);
        assert_eq!(split_yield(10.0, 3, 1), 3.0);
        assert_eq!(split_yield(10.0, 3, 2), 3.0);

        assert_eq!(split_yield(2.9, 3, 0), 1.0);
        assert_eq!(split_yield(2.9, 3, 1), 1.0);
        assert_eq!(split_yield(2.9, 3, 2), 0.0);

        assert_eq!(split_yield(5.0, 0, 0), 5.0);
        assert_eq!(split_yield(5.0, -2, 1), 5.0);
    }

    #[test]
    fn split_yield_matches_js_for_negative_totals() {
        assert_eq!(split_yield(-1.0, 3, 0), -1.0);
        assert_eq!(split_yield(-1.0, 3, -2), 0.0);
    }

    #[test]
    fn remaining_yield_subtracts_completed_shares_and_clamps_to_zero() {
        assert_eq!(remaining_yield(10.0, 3, 0), 10.0);
        assert_eq!(remaining_yield(10.0, 3, 1), 6.0);
        assert_eq!(remaining_yield(10.0, 3, 2), 3.0);
        assert_eq!(remaining_yield(10.0, 3, 3), 0.0);
        assert_eq!(remaining_yield(10.0, 3, 10), 0.0);
        assert_eq!(remaining_yield(10.0, 3, -1), 10.0);
        assert_eq!(remaining_yield(2.9, 3, 2), 0.0);
    }

    #[test]
    fn trip_due_at_spaces_trips_evenly_across_duration() {
        assert_eq!(trip_due_at(1_000.0, 7_000.0, 1), 3_000.0);
        assert_eq!(trip_due_at(1_000.0, 7_000.0, 2), 5_000.0);
        assert_eq!(trip_due_at_with_count(1_000.0, 7_000.0, 3, 4), 5_500.0);
    }

    #[test]
    fn trip_due_at_uses_minimum_duration_and_count() {
        assert_eq!(trip_due_at(1_000.0, 1_000.0, 1), 1_000.0 + 1.0 / 3.0);
        assert_eq!(trip_due_at(1_000.0, 999.0, 2), 1_000.0 + 2.0 / 3.0);
        assert_eq!(trip_due_at_with_count(1_000.0, 1_010.0, 1, 0), 1_010.0);
    }
}
