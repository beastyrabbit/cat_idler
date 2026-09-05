//! Survival tick logic ported from `lib/game/survival.ts`.

use std::borrow::Borrow;

use crate::{
    entities::CatNeeds,
    needs::{restore_hunger, restore_thirst},
    needs_constants::NEEDS_DECAY_RATES,
    policy::PolicyConfig,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurvivalResources {
    pub food: f64,
    pub water: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurvivalConfig {
    pub needs_decay_multiplier: f64,
    pub needs_damage_multiplier: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurvivalTickResult {
    pub next_needs: CatNeeds,
    pub dehydrating_started: bool,
    pub recovered_from_dehydration: bool,
    pub died: bool,
}

impl From<PolicyConfig> for SurvivalConfig {
    fn from(config: PolicyConfig) -> Self {
        Self {
            needs_decay_multiplier: config.needs_decay_multiplier,
            needs_damage_multiplier: config.needs_damage_multiplier,
        }
    }
}

impl From<&PolicyConfig> for SurvivalConfig {
    fn from(config: &PolicyConfig) -> Self {
        (*config).into()
    }
}

impl From<&SurvivalConfig> for SurvivalConfig {
    fn from(config: &SurvivalConfig) -> Self {
        *config
    }
}

#[must_use]
pub fn apply_survival_tick(
    needs: impl Borrow<CatNeeds>,
    resources: impl Borrow<SurvivalResources>,
    elapsed_sec: f64,
    config: impl Into<SurvivalConfig>,
) -> SurvivalTickResult {
    apply_survival_tick_inner(needs, resources, elapsed_sec, config, true)
}

/// Apply the established fed-state decay and damage curve without granting any
/// scalar restoration. Physical-route callers use this between finite meals and
/// drinks: nourishment is represented by the slower decay curve, while actual
/// recovery remains exclusive to arrival at the dining/drinking destination.
#[must_use]
pub fn apply_physical_survival_tick(
    needs: impl Borrow<CatNeeds>,
    elapsed_sec: f64,
    config: impl Into<SurvivalConfig>,
) -> SurvivalTickResult {
    apply_survival_tick_inner(
        needs,
        SurvivalResources {
            food: 1.0,
            water: 1.0,
        },
        elapsed_sec,
        config,
        false,
    )
}

fn apply_survival_tick_inner(
    needs: impl Borrow<CatNeeds>,
    resources: impl Borrow<SurvivalResources>,
    elapsed_sec: f64,
    config: impl Into<SurvivalConfig>,
    passive_restore: bool,
) -> SurvivalTickResult {
    let needs = needs.borrow();
    let resources = resources.borrow();
    let config = config.into();

    let tick_units = js_max(0.0, elapsed_sec) / 600.0;
    let decay_scale = js_max(0.1, config.needs_decay_multiplier);
    let damage_scale = js_max(0.1, config.needs_damage_multiplier);

    let food_available = resources.food > 0.0;
    let water_available = resources.water > 0.0;

    let hunger_decay_per_unit = if food_available {
        NEEDS_DECAY_RATES.hunger * 0.25
    } else {
        NEEDS_DECAY_RATES.hunger
    };
    let thirst_decay_per_unit = if water_available {
        NEEDS_DECAY_RATES.thirst * 0.2
    } else {
        NEEDS_DECAY_RATES.thirst
    };

    let mut next_needs = CatNeeds {
        hunger: clamp_0_to_100(needs.hunger - hunger_decay_per_unit * tick_units * decay_scale),
        thirst: clamp_0_to_100(needs.thirst - thirst_decay_per_unit * tick_units * decay_scale),
        rest: clamp_0_to_100(needs.rest - NEEDS_DECAY_RATES.rest * tick_units),
        health: needs.health,
    };

    if passive_restore && food_available && next_needs.hunger < 90.0 {
        next_needs = restore_hunger(&next_needs, 5.0 * tick_units);
    }

    if passive_restore && water_available && next_needs.thirst < 90.0 {
        next_needs = restore_thirst(&next_needs, 8.0 * tick_units);
    }

    let mut damage = 0.0;
    if next_needs.hunger == 0.0 {
        damage += 5.0 * tick_units;
    }
    if next_needs.thirst == 0.0 {
        damage += 3.0 * tick_units;
    }

    if damage > 0.0 {
        next_needs.health = clamp_0_to_100(next_needs.health - damage * damage_scale);
    }

    SurvivalTickResult {
        next_needs: next_needs.clone(),
        dehydrating_started: needs.thirst > 0.0 && next_needs.thirst == 0.0,
        recovered_from_dehydration: needs.thirst == 0.0 && next_needs.thirst > 0.0,
        died: needs.health > 0.0 && next_needs.health == 0.0,
    }
}

fn clamp_0_to_100(value: f64) -> f64 {
    js_max(0.0, js_min(100.0, value))
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

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left <= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::{SurvivalResources, apply_survival_tick};
    use crate::{
        entities::CatNeeds, needs_constants::NEEDS_DECAY_RATES, policy::config_for_tier,
        types::PolicyTier,
    };

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_needs_bits(actual: &CatNeeds, expected: &CatNeeds, label: &str) {
        assert_f64_bits(actual.hunger, expected.hunger, &format!("{label} hunger"));
        assert_f64_bits(actual.thirst, expected.thirst, &format!("{label} thirst"));
        assert_f64_bits(actual.rest, expected.rest, &format!("{label} rest"));
        assert_f64_bits(actual.health, expected.health, &format!("{label} health"));
    }

    fn needs(hunger: f64, thirst: f64, rest: f64, health: f64) -> CatNeeds {
        CatNeeds {
            hunger,
            thirst,
            rest,
            health,
        }
    }

    #[test]
    fn normal_tick_decays_and_restores_available_food_and_water() {
        let result = apply_survival_tick(
            needs(80.0, 70.0, 60.0, 100.0),
            SurvivalResources {
                food: 10.0,
                water: 10.0,
            },
            600.0,
            config_for_tier(PolicyTier::Normal),
        );

        assert_needs_bits(
            &result.next_needs,
            &needs(
                80.0 - NEEDS_DECAY_RATES.hunger * 0.25 + 5.0,
                70.0 - NEEDS_DECAY_RATES.thirst * 0.2 + 8.0,
                60.0 - NEEDS_DECAY_RATES.rest,
                100.0,
            ),
            "normal tick",
        );
        assert!(!result.dehydrating_started);
        assert!(!result.recovered_from_dehydration);
        assert!(!result.died);
    }

    #[test]
    fn scarce_food_and_water_use_full_decay_scaled_by_policy() {
        let result = apply_survival_tick(
            needs(80.0, 70.0, 60.0, 100.0),
            SurvivalResources {
                food: 0.0,
                water: 0.0,
            },
            600.0,
            config_for_tier(PolicyTier::Simple),
        );

        assert_needs_bits(
            &result.next_needs,
            &needs(
                80.0 - NEEDS_DECAY_RATES.hunger * 1.25,
                70.0 - NEEDS_DECAY_RATES.thirst * 1.25,
                60.0 - NEEDS_DECAY_RATES.rest,
                100.0,
            ),
            "scarce resources",
        );
        assert!(!result.dehydrating_started);
        assert!(!result.recovered_from_dehydration);
        assert!(!result.died);
    }

    #[test]
    fn dehydration_starts_after_thirst_reaches_zero() {
        let result = apply_survival_tick(
            needs(100.0, 1.0, 100.0, 100.0),
            SurvivalResources {
                food: 10.0,
                water: 0.0,
            },
            600.0,
            config_for_tier(PolicyTier::Normal),
        );

        assert_needs_bits(
            &result.next_needs,
            &needs(98.75, 0.0, 98.0, 97.0),
            "dehydration start",
        );
        assert!(result.dehydrating_started);
        assert!(!result.recovered_from_dehydration);
        assert!(!result.died);
    }

    #[test]
    fn dehydration_damage_can_kill_with_policy_damage_multiplier() {
        let result = apply_survival_tick(
            needs(100.0, 0.0, 100.0, 3.5),
            SurvivalResources {
                food: 10.0,
                water: 0.0,
            },
            600.0,
            config_for_tier(PolicyTier::Simple),
        );

        assert_needs_bits(
            &result.next_needs,
            &needs(98.4375, 0.0, 98.0, 0.0),
            "dehydration death",
        );
        assert!(!result.dehydrating_started);
        assert!(!result.recovered_from_dehydration);
        assert!(result.died);
    }

    #[test]
    fn water_restoration_recovers_from_dehydration_before_damage() {
        let result = apply_survival_tick(
            needs(50.0, 0.0, 100.0, 42.0),
            SurvivalResources {
                food: 10.0,
                water: 10.0,
            },
            600.0,
            config_for_tier(PolicyTier::Normal),
        );

        assert_needs_bits(
            &result.next_needs,
            &needs(
                50.0 - NEEDS_DECAY_RATES.hunger * 0.25 + 5.0,
                8.0,
                100.0 - NEEDS_DECAY_RATES.rest,
                42.0,
            ),
            "dehydration recovery",
        );
        assert!(!result.dehydrating_started);
        assert!(result.recovered_from_dehydration);
        assert!(!result.died);
    }
}
