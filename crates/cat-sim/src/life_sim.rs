//! Life-simulation rules ported from `lib/game/lifeSim.ts`, with deliberate
//! post-port idle-game population pacing.

pub use crate::age::get_life_stage;

use crate::{
    age::get_death_chance,
    breeding::calculate_fertility_bonus,
    entities::CatStats,
    types::{CatSpecialization, LifeStage},
};

pub const BREEDING_MIN_FOOD_RATIO: f64 = 0.35;
pub const BREEDING_MIN_WATER_RATIO: f64 = 0.35;
pub const BREEDING_FOOD_PER_CAT: f64 = 2.5;
pub const BREEDING_WATER_PER_CAT: f64 = 2.5;
/// Litters take eighteen game-hours after conception. Combined with the low
/// aggregate conception rate, even the luckiest roll cannot produce an instant
/// same-session population burst.
pub const GESTATION_GAME_HOURS: f64 = 18.0;
/// A fresh run spends its first day and a half establishing food, water and
/// prosperity migration before beginning pregnancies. This keeps migration the
/// visible fast-growth path without preventing already-pregnant cats from giving
/// birth after a reset.
pub const BREEDING_ESTABLISHMENT_GAME_HOURS: u64 = 36;
/// One eligible cat has a 0.1% chance per game-hour. Across the 15-cat founding
/// roster that is about a 1.5% colony-hour conception chance, before the one-
/// conception-per-tick cap: the first migration opportunity is reliably the
/// fast prosperity response, while breeding remains slow generational growth.
pub const BASE_BREEDING_CHANCE_PER_HOUR: f64 = 0.001;
pub const SPECIALIST_BREEDING_BONUS: f64 = 0.0002;
/// The recovery controller never raises an eligible cat above a five-percent
/// hourly conception chance. Housing, food, water, the one-conception-per-tick
/// limit, and gestation still bound the actual population response.
pub const LINEAGE_RECOVERY_MAX_CHANCE_PER_HOUR: f64 = 0.05;
/// A recovery conception is considered reliable only when its gestation can
/// finish before the ordinary adult-to-elder boundary. Older adults retain the
/// authored base chance, but are not used to promise replacement births that
/// old-age mortality may erase before delivery.
pub const RELIABLE_CONCEPTION_MAX_AGE_HOURS: f64 = 240.0 - GESTATION_GAME_HOURS;
/// Blessings remain useful without turning the old 2%-per-blessing helper into an
/// instant litter faucet. Its already-capped bonus is scaled to at most 0.035%/h,
/// preserving roughly the same proportion after the slower base-rate change.
pub const FERTILITY_BLESSING_RATE_SCALE: f64 = 0.0007;
pub const STAT_INHERIT_HIGH_WEIGHT: f64 = 0.6;
pub const STAT_MUTATION: f64 = 8.0;
pub const LEADERSHIP_GAIN_PER_HOUR: f64 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColonyBreedingState {
    pub food_ratio: f64,
    pub water_ratio: f64,
    pub population: f64,
    pub housing_capacity: f64,
    pub food: Option<f64>,
    pub water: Option<f64>,
}

#[must_use]
pub fn stage_work_effectiveness(stage: LifeStage) -> f64 {
    match stage {
        LifeStage::Kitten => 0.0,
        LifeStage::Young => 0.8,
        LifeStage::Adult => 1.0,
        LifeStage::Elder => 0.7,
    }
}

#[must_use]
pub fn can_work(stage: LifeStage) -> bool {
    stage != LifeStage::Kitten
}

#[must_use]
pub fn workforce_weight(stage: LifeStage) -> f64 {
    stage_work_effectiveness(stage)
}

#[must_use]
pub fn old_age_death_probability(
    age_hours: f64,
    is_leader_or_healer: bool,
    elapsed_game_hours: f64,
) -> f64 {
    if elapsed_game_hours <= 0.0 {
        return 0.0;
    }

    let per_hour = get_death_chance(age_hours, is_leader_or_healer);
    js_max(0.0, js_min(1.0, per_hour * elapsed_game_hours))
}

#[must_use]
pub fn colony_can_breed(state: &ColonyBreedingState) -> bool {
    let food_ok = state.food_ratio > BREEDING_MIN_FOOD_RATIO
        || state.food.unwrap_or(0.0) >= state.population * BREEDING_FOOD_PER_CAT;
    let water_ok = state.water_ratio > BREEDING_MIN_WATER_RATIO
        || state.water.unwrap_or(0.0) >= state.population * BREEDING_WATER_PER_CAT;

    food_ok && water_ok && state.population < state.housing_capacity
}

#[must_use]
pub fn cat_breeding_chance_per_hour(
    specialization: Option<CatSpecialization>,
    blessings: f64,
) -> f64 {
    BASE_BREEDING_CHANCE_PER_HOUR
        + specialization.map_or(0.0, |_| SPECIALIST_BREEDING_BONUS)
        + calculate_fertility_bonus(blessings) * FERTILITY_BLESSING_RATE_SCALE
}

#[must_use]
pub fn conception_probability(
    specialization: Option<CatSpecialization>,
    blessings: f64,
    elapsed_game_hours: f64,
) -> f64 {
    if elapsed_game_hours <= 0.0 {
        return 0.0;
    }

    let per_hour = cat_breeding_chance_per_hour(specialization, blessings);
    js_max(0.0, js_min(1.0, per_hour * elapsed_game_hours))
}

/// Preserve the authored slow conception rate while applying bounded
/// replacement pressure when a village falls below its founding lineage floor.
///
/// Dividing the missing future residents by the colony's remaining reliable
/// fertile cat-hours yields the per-cat hourly hazard required to replace that
/// generation in expectation. Existing pregnancies count toward the secured
/// lineage before this function is called.
#[must_use]
pub fn lineage_recovery_conception_probability(
    specialization: Option<CatSpecialization>,
    blessings: f64,
    elapsed_game_hours: f64,
    births_needed: usize,
    remaining_reliable_fertile_hours: f64,
    candidate_reliable_fertile_hours: f64,
) -> f64 {
    let authored = conception_probability(specialization, blessings, elapsed_game_hours);
    if births_needed == 0
        || elapsed_game_hours <= 0.0
        || remaining_reliable_fertile_hours <= 0.0
        || candidate_reliable_fertile_hours <= 0.0
    {
        return authored;
    }

    let recovery_per_hour = (births_needed as f64 / remaining_reliable_fertile_hours)
        .clamp(0.0, LINEAGE_RECOVERY_MAX_CHANCE_PER_HOUR);
    authored.max((recovery_per_hour * elapsed_game_hours).clamp(0.0, 1.0))
}

#[must_use]
pub fn inherit_stats<R>(parent1: &CatStats, parent2: Option<&CatStats>, mut roll: R) -> CatStats
where
    R: FnMut() -> f64,
{
    let parent2 = parent2.unwrap_or(parent1);

    CatStats {
        attack: inherit_stat(parent1.attack, parent2.attack, &mut roll),
        defense: inherit_stat(parent1.defense, parent2.defense, &mut roll),
        hunting: inherit_stat(parent1.hunting, parent2.hunting, &mut roll),
        medicine: inherit_stat(parent1.medicine, parent2.medicine, &mut roll),
        cleaning: inherit_stat(parent1.cleaning, parent2.cleaning, &mut roll),
        building: inherit_stat(parent1.building, parent2.building, &mut roll),
        leadership: inherit_stat(parent1.leadership, parent2.leadership, &mut roll),
        vision: inherit_stat(parent1.vision, parent2.vision, &mut roll),
    }
}

#[must_use]
pub fn trade_level(xp: f64) -> f64 {
    if xp <= 0.0 {
        return 0.0;
    }

    xp.sqrt().floor()
}

#[must_use]
pub fn trade_yield_multiplier(xp: f64) -> f64 {
    if xp <= 0.0 {
        return 1.0;
    }

    1.0 + 0.4 * (1.0 - 1.0 / (1.0 + xp / 30.0))
}

#[must_use]
pub fn trade_speed_multiplier(xp: f64) -> f64 {
    if xp <= 0.0 {
        return 1.0;
    }

    1.0 - 0.25 * (1.0 - 1.0 / (1.0 + xp / 25.0))
}

#[must_use]
pub fn leadership_after_tenure(leadership: f64, elapsed_game_hours: f64) -> f64 {
    js_min(
        100.0,
        leadership + LEADERSHIP_GAIN_PER_HOUR * elapsed_game_hours,
    )
}

fn inherit_stat<R>(left: f64, right: f64, roll: &mut R) -> f64
where
    R: FnMut() -> f64,
{
    let high = js_max(left, right);
    let low = js_min(left, right);
    let base = high * STAT_INHERIT_HIGH_WEIGHT + low * (1.0 - STAT_INHERIT_HIGH_WEIGHT);
    let mutation = (roll() * 2.0 - 1.0) * STAT_MUTATION;

    js_max(1.0, js_min(100.0, js_round(base + mutation)))
}

fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
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
    use super::{
        BASE_BREEDING_CHANCE_PER_HOUR, BREEDING_FOOD_PER_CAT, BREEDING_MIN_FOOD_RATIO,
        BREEDING_MIN_WATER_RATIO, BREEDING_WATER_PER_CAT, ColonyBreedingState,
        LEADERSHIP_GAIN_PER_HOUR, LINEAGE_RECOVERY_MAX_CHANCE_PER_HOUR,
        RELIABLE_CONCEPTION_MAX_AGE_HOURS, SPECIALIST_BREEDING_BONUS, can_work,
        cat_breeding_chance_per_hour, colony_can_breed, conception_probability, get_life_stage,
        inherit_stats, leadership_after_tenure, lineage_recovery_conception_probability,
        old_age_death_probability, stage_work_effectiveness, trade_level, trade_speed_multiplier,
        trade_yield_multiplier, workforce_weight,
    };
    use crate::{entities::CatStats, types::CatSpecialization, types::LifeStage};

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    // For the diminishing-return curve values, the impl reproduces the TS
    // operation order exactly; comparing against an algebraically-simplified
    // literal (e.g. 10/13) can differ by 1 ULP. The mathematical value is what
    // matters here, so compare within a tight tolerance.
    fn assert_f64_close(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= 1e-12,
            "{label}: {actual} vs {expected}"
        );
    }

    #[allow(clippy::too_many_arguments)] // one arg per CatStats field (8)
    fn stats(
        attack: f64,
        defense: f64,
        hunting: f64,
        medicine: f64,
        cleaning: f64,
        building: f64,
        leadership: f64,
        vision: f64,
    ) -> CatStats {
        CatStats {
            attack,
            defense,
            hunting,
            medicine,
            cleaning,
            building,
            leadership,
            vision,
        }
    }

    fn breeding_state(
        food_ratio: f64,
        water_ratio: f64,
        population: f64,
        housing_capacity: f64,
        food: Option<f64>,
        water: Option<f64>,
    ) -> ColonyBreedingState {
        ColonyBreedingState {
            food_ratio,
            water_ratio,
            population,
            housing_capacity,
            food,
            water,
        }
    }

    #[test]
    fn stage_capability_matches_life_sim_ts() {
        let cases = [
            (LifeStage::Kitten, 0.0, false),
            (LifeStage::Young, 0.8, true),
            (LifeStage::Adult, 1.0, true),
            (LifeStage::Elder, 0.7, true),
        ];

        for (stage, effectiveness, works) in cases {
            assert_f64_bits(
                stage_work_effectiveness(stage),
                effectiveness,
                stage.as_str(),
            );
            assert_eq!(can_work(stage), works, "{}", stage.as_str());
            assert_f64_bits(workforce_weight(stage), effectiveness, stage.as_str());
        }
    }

    #[test]
    fn old_age_death_probability_scales_and_clamps() {
        assert_f64_bits(old_age_death_probability(200.0, false, 1.0), 0.0, "adult");
        assert_f64_bits(
            old_age_death_probability(240.0, false, 2.0),
            0.02,
            "standard threshold two hours",
        );
        assert_f64_bits(
            old_age_death_probability(288.0, true, 1.5),
            0.015,
            "leader threshold one and a half hours",
        );
        assert_f64_bits(
            old_age_death_probability(500.0, false, 10_000.0),
            1.0,
            "skip-time cap",
        );
        assert_f64_bits(old_age_death_probability(300.0, false, 0.0), 0.0, "zero");
    }

    #[test]
    fn breeding_gates_match_ratio_and_per_capita_fallbacks() {
        assert_f64_bits(BREEDING_MIN_FOOD_RATIO, 0.35, "food ratio");
        assert_f64_bits(BREEDING_MIN_WATER_RATIO, 0.35, "water ratio");
        assert_f64_bits(BREEDING_FOOD_PER_CAT, 2.5, "food per cat");
        assert_f64_bits(BREEDING_WATER_PER_CAT, 2.5, "water per cat");

        let healthy = breeding_state(0.6, 0.6, 10.0, 14.0, None, None);
        assert!(colony_can_breed(&healthy));
        assert!(!colony_can_breed(&breeding_state(
            0.35, 0.6, 10.0, 14.0, None, None
        )));
        assert!(!colony_can_breed(&breeding_state(
            0.6, 0.35, 10.0, 14.0, None, None
        )));
        assert!(!colony_can_breed(&breeding_state(
            0.6, 0.6, 14.0, 14.0, None, None
        )));

        let subsistence = breeding_state(0.08, 0.08, 10.0, 14.0, Some(25.0), Some(25.0));
        assert!(colony_can_breed(&subsistence));
        assert!(!colony_can_breed(&breeding_state(
            0.08,
            0.08,
            10.0,
            14.0,
            Some(24.999),
            Some(25.0)
        )));
        assert!(!colony_can_breed(&breeding_state(
            0.08,
            0.08,
            10.0,
            14.0,
            Some(25.0),
            Some(24.999)
        )));
    }

    #[test]
    fn conception_uses_base_specialist_blessing_and_elapsed_time() {
        assert_f64_bits(BASE_BREEDING_CHANCE_PER_HOUR, 0.001, "base breeding chance");
        assert_f64_bits(
            SPECIALIST_BREEDING_BONUS,
            0.0002,
            "specialist breeding bonus",
        );

        assert_f64_bits(
            cat_breeding_chance_per_hour(None, 0.0),
            0.001,
            "plain per hour",
        );
        assert_f64_close(
            cat_breeding_chance_per_hour(Some(CatSpecialization::Hunter), 0.0),
            0.0012,
            "specialist per hour",
        );
        assert!(
            (cat_breeding_chance_per_hour(None, 5.0) - 0.00107).abs() < 1e-12,
            "plain with five blessings"
        );
        assert!(
            (conception_probability(None, 5.0, 2.5) - 0.002675).abs() < 1e-12,
            "elapsed scaling"
        );
        assert_f64_close(
            conception_probability(Some(CatSpecialization::Hunter), 100.0, 2.0),
            0.0031,
            "elapsed scaling remains slow even with many blessings",
        );
        assert_f64_bits(conception_probability(None, 0.0, 0.0), 0.0, "zero");
    }

    #[test]
    fn lineage_recovery_uses_the_finite_reproductive_opportunity_budget() {
        assert_f64_bits(
            RELIABLE_CONCEPTION_MAX_AGE_HOURS,
            222.0,
            "reliable conception age",
        );
        assert_f64_bits(
            LINEAGE_RECOVERY_MAX_CHANCE_PER_HOUR,
            0.05,
            "recovery hourly cap",
        );

        // Six missing future residents across 300 reliable fertile cat-hours
        // requires a two-percent per-cat hourly hazard.
        assert_f64_bits(
            lineage_recovery_conception_probability(None, 0.0, 1.0, 6, 300.0, 50.0),
            0.02,
            "replacement hazard",
        );
        // Once the floor is secured, and for a candidate whose reliable window
        // has closed, the authored slow probability remains untouched.
        assert_f64_bits(
            lineage_recovery_conception_probability(None, 0.0, 1.0, 0, 300.0, 50.0),
            0.001,
            "secured lineage",
        );
        assert_f64_bits(
            lineage_recovery_conception_probability(None, 0.0, 1.0, 6, 300.0, 0.0),
            0.001,
            "late adult",
        );
    }

    #[test]
    fn lineage_recovery_is_bounded_for_tiny_old_cohorts_and_coarse_ticks() {
        assert_f64_bits(
            lineage_recovery_conception_probability(None, 0.0, 1.0, 20, 10.0, 10.0),
            0.05,
            "hourly recovery cap",
        );
        assert_f64_bits(
            lineage_recovery_conception_probability(None, 0.0, 24.0, 20, 10.0, 10.0),
            1.0,
            "probability cap",
        );
    }

    #[test]
    fn inherit_stats_consumes_one_roll_per_stat_in_order() {
        let parent1 = stats(80.0, 30.0, 90.0, 2.0, 99.0, 5.0, 60.0, 10.0);
        let parent2 = stats(20.0, 70.0, 40.0, 90.0, 5.0, 100.0, 60.0, 30.0);
        let rolls = [0.5, 0.0, 1.0, 0.25, 0.75, 0.5, 0.1, 0.9];
        let mut roll_index = 0;

        let kitten = inherit_stats(&parent1, Some(&parent2), || {
            let roll = rolls[roll_index];
            roll_index += 1;
            roll
        });

        assert_eq!(roll_index, 8);
        assert_eq!(
            kitten,
            stats(56.0, 46.0, 78.0, 51.0, 65.0, 62.0, 54.0, 28.0)
        );
    }

    #[test]
    fn inherit_stats_falls_back_to_single_parent_and_clamps() {
        let low = stats(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let high = stats(100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0);

        assert_eq!(
            inherit_stats(&low, None, || 0.0),
            stats(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0)
        );
        assert_eq!(
            inherit_stats(&high, None, || 1.0),
            stats(100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0)
        );
    }

    #[test]
    fn trade_depth_uses_square_root_and_diminishing_curves() {
        assert_f64_bits(trade_level(0.0), 0.0, "level zero");
        assert_f64_bits(trade_level(1.0), 1.0, "level one");
        assert_f64_bits(trade_level(9.0), 3.0, "level nine");
        assert_f64_bits(trade_level(99.0), 9.0, "level ninety-nine");
        assert_f64_bits(trade_level(100.0), 10.0, "level one hundred");

        assert_f64_bits(trade_yield_multiplier(0.0), 1.0, "yield zero");
        assert_f64_bits(trade_yield_multiplier(30.0), 1.2, "yield thirty");
        assert_f64_close(
            trade_yield_multiplier(300.0),
            1.0 + 0.4 * (10.0 / 11.0),
            "yield three hundred",
        );

        assert_f64_bits(trade_speed_multiplier(0.0), 1.0, "speed zero");
        assert_f64_bits(trade_speed_multiplier(25.0), 0.875, "speed twenty-five");
        assert_f64_close(
            trade_speed_multiplier(300.0),
            10.0 / 13.0,
            "speed three hundred",
        );
    }

    #[test]
    fn leadership_tenure_adds_per_hour_and_caps_at_one_hundred() {
        assert_f64_bits(LEADERSHIP_GAIN_PER_HOUR, 0.35, "gain per hour");
        assert_f64_bits(leadership_after_tenure(50.0, 0.0), 50.0, "no time");
        assert_f64_bits(leadership_after_tenure(50.0, 10.0), 53.5, "ten hours");
        assert_f64_bits(leadership_after_tenure(99.0, 10_000.0), 100.0, "cap");
    }

    #[test]
    fn get_life_stage_reexport_uses_age_module() {
        assert_eq!(get_life_stage(0.0), LifeStage::Kitten);
        assert_eq!(get_life_stage(12.0), LifeStage::Young);
        assert_eq!(get_life_stage(30.0), LifeStage::Adult);
        assert_eq!(get_life_stage(239.999), LifeStage::Adult);
        assert_eq!(get_life_stage(240.0), LifeStage::Elder);
    }
}
