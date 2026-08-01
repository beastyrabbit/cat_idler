//! Seeded linear congruential RNG ported from `lib/game/seededRng.ts`.

const MODULUS: f64 = 4_294_967_296.0;
const MULTIPLIER: u32 = 1_664_525;
const INCREMENT: u32 = 1_013_904_223;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeededRoll {
    pub value: f64,
    pub next_seed: u32,
}

pub fn normalize_seed(seed: f64) -> u32 {
    if !seed.is_finite() {
        return 1;
    }

    let wrapped = (seed.abs().floor() % MODULUS) as u32;
    wrapped.max(1)
}

pub fn roll_seeded(seed: f64) -> SeededRoll {
    let normalized = normalize_seed(seed);
    let next_seed = normalized.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);

    SeededRoll {
        value: f64::from(next_seed) / MODULUS,
        next_seed,
    }
}

pub fn movement_seed(seed: u32) -> u32 {
    seed.wrapping_add(1_000_003)
}

pub fn life_seed(seed: u32) -> u32 {
    seed.wrapping_add(2_000_003)
}

/// Derive a restart-safe life-simulation fork for one semantic game-time boundary.
///
/// The original fork offset isolates life rolls from movement and raids, but reusing
/// that bare root on every world tick repeats the same conception decision forever.
/// Length-prefixed FNV mixing keeps the project LCG while making different colonies,
/// reset runs, and game seconds independent without storing call-order state.
#[must_use]
pub fn keyed_life_seed(seed: u32, colony_id: &str, run_number: u32, game_second: u64) -> u32 {
    const FNV_PRIME: u32 = 16_777_619;

    let mut hash = life_seed(seed) ^ 2_166_136_261;
    for bytes in [
        colony_id.as_bytes(),
        &run_number.to_le_bytes(),
        &game_second.to_le_bytes(),
    ] {
        for byte in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash.max(1)
}

pub fn raid_seed(seed: u32) -> u32 {
    seed.wrapping_add(3_000_003)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODULUS: f64 = 4_294_967_296.0;

    #[test]
    fn ts_seeded_rng_normalizes_invalid_seeds_to_stable_fallback() {
        assert_eq!(normalize_seed(f64::NAN), 1);
        assert_eq!(normalize_seed(f64::INFINITY), 1);
        assert_eq!(normalize_seed(f64::NEG_INFINITY), 1);
    }

    #[test]
    fn ts_seeded_rng_normalizes_to_unsigned_integer_space() {
        assert_eq!(normalize_seed(-42.8), 42);
        assert_eq!(normalize_seed(42.8), 42);
        assert_eq!(normalize_seed(u32::MAX as f64), u32::MAX);
    }

    #[test]
    fn ts_seeded_rng_normalizes_zero_and_wrapped_zero_to_one() {
        assert_eq!(normalize_seed(0.0), 1);
        assert_eq!(normalize_seed(-0.0), 1);
        assert_eq!(normalize_seed(4_294_967_296.0), 1);
    }

    #[test]
    fn ts_seeded_rng_wraps_large_seeds_with_js_unsigned_32_semantics() {
        assert_eq!(normalize_seed(4_294_967_301.9), 5);
        assert_eq!(normalize_seed(8_589_934_599.0), 7);
    }

    #[test]
    fn ts_seeded_rng_produces_deterministic_sequence_for_same_seed() {
        let first = roll_seeded(123.0);
        let second = roll_seeded(123.0);

        assert_eq!(first, second);
    }

    #[test]
    fn ts_seeded_rng_returns_values_in_zero_to_one_exclusive_range() {
        let roll = roll_seeded(999.0);

        assert!(roll.value >= 0.0);
        assert!(roll.value < 1.0);
    }

    #[test]
    fn ts_seeded_rng_produces_divergent_values_when_chaining() {
        let first = roll_seeded(123.0);
        let second = roll_seeded(first.next_seed as f64);

        assert_ne!(second.value, first.value);
        assert_ne!(second.next_seed, first.next_seed);
    }

    #[test]
    fn roll_seeded_matches_known_seed_123_multi_step_golden_vector() {
        let expected = [
            (1_218_640_798, 1_218_640_798.0 / MODULUS),
            (1_868_869_221, 1_868_869_221.0 / MODULUS),
            (166_005_888, 166_005_888.0 / MODULUS),
            (948_671_967, 948_671_967.0 / MODULUS),
            (1_543_727_538, 1_543_727_538.0 / MODULUS),
        ];

        let mut seed = 123.0;
        for (next_seed, value) in expected {
            let roll = roll_seeded(seed);
            assert_eq!(roll.next_seed, next_seed);
            assert_eq!(roll.value, value);
            seed = roll.next_seed as f64;
        }
    }

    #[test]
    fn worker_tick_rng_forks_use_movement_life_and_raid_offsets() {
        assert_eq!(movement_seed(123), 1_000_126);
        assert_eq!(life_seed(123), 2_000_126);
        assert_eq!(raid_seed(123), 3_000_126);
    }

    #[test]
    fn worker_tick_rng_forks_wrap_in_unsigned_32_space_before_rolling() {
        assert_eq!(movement_seed(u32::MAX), 1_000_002);
        assert_eq!(life_seed(u32::MAX), 2_000_002);
        assert_eq!(raid_seed(u32::MAX), 3_000_002);
    }

    #[test]
    fn keyed_life_forks_are_stable_and_change_across_semantic_boundaries() {
        let first = keyed_life_seed(42, "colony-1", 1, 3_600);
        assert_eq!(first, keyed_life_seed(42, "colony-1", 1, 3_600));
        assert_ne!(first, keyed_life_seed(42, "colony-2", 1, 3_600));
        assert_ne!(first, keyed_life_seed(42, "colony-1", 2, 3_600));
        assert_ne!(first, keyed_life_seed(42, "colony-1", 1, 3_601));
    }
}
