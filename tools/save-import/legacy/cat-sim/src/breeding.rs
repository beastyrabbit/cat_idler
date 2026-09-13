//! Fertility blessing helpers ported from `lib/game/breeding.ts`.

const FERTILITY_BONUS_PER_BLESSING: f64 = 0.02;
const MAX_FERTILITY_BONUS: f64 = 0.5;
const MAX_BREEDING_CHANCE: f64 = 0.8;

#[must_use]
pub fn calculate_fertility_bonus(blessings: f64) -> f64 {
    if blessings <= 0.0 {
        return 0.0;
    }

    let bonus = blessings * FERTILITY_BONUS_PER_BLESSING;
    if bonus > MAX_FERTILITY_BONUS {
        MAX_FERTILITY_BONUS
    } else {
        bonus
    }
}

#[must_use]
pub fn calculate_breeding_chance(base_chance: f64, blessings: f64) -> f64 {
    let clamped_base = if base_chance < 0.0 { 0.0 } else { base_chance };
    let chance = clamped_base + calculate_fertility_bonus(blessings);

    if chance > MAX_BREEDING_CHANCE {
        MAX_BREEDING_CHANCE
    } else {
        chance
    }
}

#[cfg(test)]
mod tests {
    use crate::breeding::{calculate_breeding_chance, calculate_fertility_bonus};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn fertility_bonus_is_two_percent_per_blessing() {
        assert_eq!(calculate_fertility_bonus(0.0), 0.0);
        assert_eq!(calculate_fertility_bonus(1.0), 0.02);
        assert_eq!(calculate_fertility_bonus(5.0), 0.1);
        assert_eq!(calculate_fertility_bonus(10.0), 0.2);
        assert_close(calculate_fertility_bonus(0.5), 0.01);
    }

    #[test]
    fn fertility_bonus_caps_at_fifty_percent() {
        assert_eq!(calculate_fertility_bonus(25.0), 0.5);
        assert_eq!(calculate_fertility_bonus(30.0), 0.5);
        assert_eq!(calculate_fertility_bonus(100.0), 0.5);
    }

    #[test]
    fn fertility_bonus_ignores_negative_blessings() {
        assert_eq!(calculate_fertility_bonus(-1.0), 0.0);
        assert_eq!(calculate_fertility_bonus(-100.0), 0.0);
    }

    #[test]
    fn breeding_chance_adds_bonus_to_non_negative_base() {
        assert_eq!(calculate_breeding_chance(0.3, 0.0), 0.3);
        assert_close(calculate_breeding_chance(0.3, 5.0), 0.4);
        assert_close(calculate_breeding_chance(0.0, 5.0), 0.1);
        assert_close(calculate_breeding_chance(0.3, 3.0), 0.36);
        assert_close(calculate_breeding_chance(0.3, 15.0), 0.6);
    }

    #[test]
    fn breeding_chance_clamps_negative_base_before_bonus() {
        assert_eq!(calculate_breeding_chance(-0.5, 0.0), 0.0);
        assert_close(calculate_breeding_chance(-0.5, 10.0), 0.2);
    }

    #[test]
    fn breeding_chance_caps_at_eighty_percent() {
        assert_eq!(calculate_breeding_chance(0.3, 100.0), 0.8);
        assert_eq!(calculate_breeding_chance(0.7, 10.0), 0.8);
        assert_eq!(calculate_breeding_chance(0.3, 9999.0), 0.8);
        assert_eq!(calculate_breeding_chance(0.3, 25.0), 0.8);
    }

    #[test]
    fn breeding_chance_ignores_negative_blessings() {
        assert_eq!(calculate_breeding_chance(0.3, -5.0), 0.3);
    }
}
