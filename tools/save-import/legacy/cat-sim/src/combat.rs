//! Combat resolution rules ported from `lib/game/combat.ts`.

use serde::{Deserialize, Serialize};

use crate::types::BuildingType;

/// Result returned by `calculateCombatResult`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CombatResult {
    pub won: bool,
    pub damage: f64,
}

/// The random values consumed by `calculateCombatResult`.
///
/// The TypeScript source calls `Math.random()` once for the cat and once for the
/// enemy. The Rust port keeps the function deterministic by accepting those raw
/// random values as inputs and applying the original `floor(random * 20) + 1`
/// roll conversion internally.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CombatRolls {
    pub cat: f64,
    pub enemy: f64,
}

impl CombatRolls {
    #[must_use]
    pub const fn new(cat: f64, enemy: f64) -> Self {
        Self { cat, enemy }
    }
}

/// Minimal building shape the defense math needs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingForDefense {
    #[serde(rename = "type")]
    pub building_type: BuildingType,
    pub level: f64,
    pub construction_progress: f64,
}

impl BuildingForDefense {
    #[must_use]
    pub const fn new(building_type: BuildingType, level: f64, construction_progress: f64) -> Self {
        Self {
            building_type,
            level,
            construction_progress,
        }
    }
}

#[must_use]
pub fn calculate_combat_result(
    cat_attack: f64,
    cat_defense: f64,
    enemy_strength: f64,
    cat_random: f64,
    enemy_random: f64,
) -> CombatResult {
    calculate_combat_result_with_rolls(
        cat_attack,
        cat_defense,
        enemy_strength,
        CombatRolls::new(cat_random, enemy_random),
    )
}

#[must_use]
pub fn calculate_combat_result_with_rolls(
    cat_attack: f64,
    cat_defense: f64,
    enemy_strength: f64,
    rolls: CombatRolls,
) -> CombatResult {
    let cat_roll = cat_attack + cat_defense + d20_from_random(rolls.cat);
    let enemy_roll = enemy_strength + d20_from_random(rolls.enemy);
    let won = cat_roll > enemy_roll;

    if won {
        return CombatResult {
            won: true,
            damage: 0.0,
        };
    }

    let loss_margin = enemy_roll - cat_roll;
    let damage = js_min(70.0, js_max(30.0, 30.0 + loss_margin * 0.4));

    CombatResult {
        won: false,
        damage: js_round(damage),
    }
}

#[must_use]
pub fn get_clicks_needed(base_clicks: f64, colony_defense: f64, cat_vision: f64) -> f64 {
    let defense_multiplier = js_max(0.5, 1.0 - colony_defense / 100.0);
    let vision_multiplier = js_max(0.5, 1.0 - cat_vision / 100.0);
    let total_multiplier = defense_multiplier * vision_multiplier;
    let clicks = base_clicks * total_multiplier;

    js_max(1.0, js_round(clicks))
}

#[must_use]
pub fn calculate_colony_defense(buildings: &[BuildingForDefense]) -> f64 {
    let mut total = 0.0;

    for building in buildings {
        if building.building_type != BuildingType::Walls {
            continue;
        }

        total += (building.level * 10.0 * (building.construction_progress / 100.0)).floor();
    }

    js_min(100.0, total)
}

fn d20_from_random(random: f64) -> f64 {
    ((random * 20.0).floor() + 1.0).clamp(1.0, 20.0)
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

fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::{
        BuildingForDefense, CombatResult, CombatRolls, calculate_colony_defense,
        calculate_combat_result, calculate_combat_result_with_rolls, d20_from_random,
        get_clicks_needed,
    };
    use crate::types::BuildingType;

    fn building(
        building_type: BuildingType,
        level: f64,
        construction_progress: f64,
    ) -> BuildingForDefense {
        BuildingForDefense::new(building_type, level, construction_progress)
    }

    #[test]
    fn combat_result_uses_injected_rolls_for_a_win() {
        let result = calculate_combat_result(80.0, 80.0, 20.0, 0.9, 0.1);

        assert!(result.won);
        assert_eq!(result.damage, 0.0);
    }

    #[test]
    fn d20_roll_stays_in_range_at_closed_and_out_of_range_boundaries() {
        assert_eq!(d20_from_random(-1.0), 1.0);
        assert_eq!(d20_from_random(0.0), 1.0);
        assert_eq!(d20_from_random(0.999_999), 20.0);
        assert_eq!(d20_from_random(1.0), 20.0);
        assert_eq!(d20_from_random(f64::INFINITY), 20.0);
    }

    #[test]
    fn combat_result_tie_is_a_loss_with_minimum_damage() {
        let result =
            calculate_combat_result_with_rolls(50.0, 49.0, 99.0, CombatRolls::new(0.0, 0.0));

        assert_eq!(
            result,
            CombatResult {
                won: false,
                damage: 30.0,
            }
        );
    }

    #[test]
    fn combat_result_scales_and_caps_loss_damage() {
        let scaled = calculate_combat_result(40.0, 40.0, 100.0, 0.95, 0.2);
        assert!(!scaled.won);
        assert_eq!(scaled.damage, 32.0);

        let capped = calculate_combat_result(1.0, 1.0, 100.0, 0.0, 0.95);
        assert!(!capped.won);
        assert_eq!(capped.damage, 70.0);
    }

    #[test]
    fn clicks_needed_matches_combat_ts_vectors() {
        assert_eq!(get_clicks_needed(50.0, 0.0, 0.0), 50.0);
        assert_eq!(get_clicks_needed(50.0, 50.0, 0.0), 25.0);
        assert_eq!(get_clicks_needed(50.0, 0.0, 100.0), 25.0);
        assert_eq!(get_clicks_needed(100.0, 50.0, 100.0), 25.0);
        assert_eq!(get_clicks_needed(10.0, 90.0, 90.0), 3.0);
        assert_eq!(get_clicks_needed(1.0, 100.0, 100.0), 1.0);
    }

    #[test]
    fn colony_defense_uses_only_walls_and_caps_at_one_hundred() {
        let buildings = [
            building(BuildingType::Den, 5.0, 100.0),
            building(BuildingType::Walls, 2.0, 100.0),
            building(BuildingType::FoodStorage, 3.0, 100.0),
            building(BuildingType::Walls, 1.0, 100.0),
        ];
        assert_eq!(calculate_colony_defense(&buildings), 30.0);

        let capped = [
            building(BuildingType::Walls, 5.0, 100.0),
            building(BuildingType::Walls, 5.0, 100.0),
            building(BuildingType::Walls, 5.0, 100.0),
        ];
        assert_eq!(calculate_colony_defense(&capped), 100.0);
    }

    #[test]
    fn colony_defense_floors_each_wall_contribution() {
        let buildings = [
            building(BuildingType::Walls, 1.0, 33.0),
            building(BuildingType::Walls, 4.0, 75.0),
        ];

        assert_eq!(calculate_colony_defense(&buildings), 33.0);
    }
}
