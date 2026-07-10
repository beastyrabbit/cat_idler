//! Per-resource storage capacity ported from `lib/game/storage.ts`.

use serde::{Deserialize, Serialize};

use crate::types::BuildingType;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StorageCapacities {
    pub food: f64,
    pub water: f64,
    pub herbs: f64,
    pub materials: f64,
    pub refined: f64,
    pub weapons: f64,
    pub armor: f64,
    /// Refinement tier (P12.4b): planks/blocks/tools from the wood-cutter,
    /// stone-prep, and woodworking chains.
    pub planks: f64,
    pub blocks: f64,
    pub tools: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GranaryBonus {
    pub food: f64,
    pub herbs: f64,
    pub materials: f64,
    pub refined: f64,
}

/// Minimal building shape the capacity math needs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StorageBuilding {
    pub building_type: BuildingType,
    pub construction_progress: f64,
    pub level: Option<f64>,
}

impl StorageBuilding {
    #[must_use]
    pub const fn new(
        building_type: BuildingType,
        construction_progress: f64,
        level: Option<f64>,
    ) -> Self {
        Self {
            building_type,
            construction_progress,
            level,
        }
    }
}

/// Base capacity every settlement starts with, before any storehouses.
pub const BASE_CAPACITY: StorageCapacities = StorageCapacities {
    food: 200.0,
    water: 200.0,
    herbs: 100.0,
    materials: 100.0,
    refined: 100.0,
    weapons: 50.0,
    armor: 50.0,
    planks: 100.0,
    blocks: 100.0,
    tools: 100.0,
};

/// Dry goods a single finished granary adds per level.
pub const GRANARY_BONUS: GranaryBonus = GranaryBonus {
    food: 400.0,
    herbs: 100.0,
    materials: 100.0,
    refined: 50.0,
};

/// Extra water a single finished water bowl holds per level.
pub const WATER_BOWL_BONUS: f64 = 200.0;

/// Extra armory capacity a single finished smithy holds per level.
pub const SMITHY_ARMORY_BONUS: f64 = 50.0;

#[must_use]
pub fn storage_capacities_default(buildings: &[StorageBuilding]) -> StorageCapacities {
    storage_capacities(buildings, 1.0)
}

#[must_use]
pub fn storage_capacities(buildings: &[StorageBuilding], storage_mult: f64) -> StorageCapacities {
    let mut caps = BASE_CAPACITY;
    let mult = js_max(0.0, storage_mult);

    for building in buildings {
        if !is_finished(*building) {
            continue;
        }

        let level = level_of(*building);
        match building.building_type {
            BuildingType::FoodStorage => {
                caps.food += GRANARY_BONUS.food * level * mult;
                caps.herbs += GRANARY_BONUS.herbs * level * mult;
                caps.materials += GRANARY_BONUS.materials * level * mult;
                caps.refined += GRANARY_BONUS.refined * level * mult;
            }
            BuildingType::WaterBowl => {
                caps.water += WATER_BOWL_BONUS * level * mult;
            }
            BuildingType::Smithy => {
                caps.weapons += SMITHY_ARMORY_BONUS * level * mult;
                caps.armor += SMITHY_ARMORY_BONUS * level * mult;
            }
            _ => {}
        }
    }

    caps
}

#[must_use]
pub fn storage_capacities_with_mult(
    buildings: &[StorageBuilding],
    storage_mult: f64,
) -> StorageCapacities {
    storage_capacities(buildings, storage_mult)
}

#[must_use]
pub const fn storehouse_cap(population: u32) -> u32 {
    let cap = population / 6;
    if cap < 1 { 1 } else { cap }
}

#[must_use]
pub fn count_storehouses(buildings: &[StorageBuilding]) -> u32 {
    let count = buildings
        .iter()
        .filter(|building| {
            building.building_type == BuildingType::FoodStorage && is_finished(**building)
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn is_finished(building: StorageBuilding) -> bool {
    building.construction_progress >= 100.0
}

fn level_of(building: StorageBuilding) -> f64 {
    js_max(1.0, building.level.unwrap_or(1.0))
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
        BASE_CAPACITY, GRANARY_BONUS, SMITHY_ARMORY_BONUS, StorageBuilding, StorageCapacities,
        WATER_BOWL_BONUS, count_storehouses, storage_capacities, storage_capacities_default,
        storage_capacities_with_mult, storehouse_cap,
    };
    use crate::types::BuildingType;

    fn building(
        building_type: BuildingType,
        construction_progress: f64,
        level: Option<f64>,
    ) -> StorageBuilding {
        StorageBuilding {
            building_type,
            construction_progress,
            level,
        }
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_caps_bits(actual: StorageCapacities, expected: StorageCapacities, label: &str) {
        assert_f64_bits(actual.food, expected.food, &format!("{label} food"));
        assert_f64_bits(actual.water, expected.water, &format!("{label} water"));
        assert_f64_bits(actual.herbs, expected.herbs, &format!("{label} herbs"));
        assert_f64_bits(
            actual.materials,
            expected.materials,
            &format!("{label} materials"),
        );
        assert_f64_bits(
            actual.refined,
            expected.refined,
            &format!("{label} refined"),
        );
        assert_f64_bits(
            actual.weapons,
            expected.weapons,
            &format!("{label} weapons"),
        );
        assert_f64_bits(actual.armor, expected.armor, &format!("{label} armor"));
        assert_f64_bits(actual.planks, expected.planks, &format!("{label} planks"));
        assert_f64_bits(actual.blocks, expected.blocks, &format!("{label} blocks"));
        assert_f64_bits(actual.tools, expected.tools, &format!("{label} tools"));
    }

    #[test]
    fn constants_match_typescript_exports() {
        assert_caps_bits(
            BASE_CAPACITY,
            StorageCapacities {
                food: 200.0,
                water: 200.0,
                herbs: 100.0,
                materials: 100.0,
                refined: 100.0,
                weapons: 50.0,
                armor: 50.0,
                planks: 100.0,
                blocks: 100.0,
                tools: 100.0,
            },
            "base capacity",
        );
        assert_f64_bits(GRANARY_BONUS.food, 400.0, "granary food");
        assert_f64_bits(GRANARY_BONUS.herbs, 100.0, "granary herbs");
        assert_f64_bits(GRANARY_BONUS.materials, 100.0, "granary materials");
        assert_f64_bits(GRANARY_BONUS.refined, 50.0, "granary refined");
        assert_f64_bits(WATER_BOWL_BONUS, 200.0, "water bowl");
        assert_f64_bits(SMITHY_ARMORY_BONUS, 50.0, "smithy armory");
    }

    #[test]
    fn storage_capacities_match_hand_derived_vectors() {
        assert_caps_bits(
            storage_capacities_default(&[]),
            BASE_CAPACITY,
            "empty settlement",
        );

        assert_caps_bits(
            storage_capacities_default(&[
                building(BuildingType::FoodStorage, 100.0, Some(1.0)),
                building(BuildingType::FoodStorage, 100.0, Some(2.0)),
                building(BuildingType::WaterBowl, 100.0, Some(1.0)),
                building(BuildingType::Smithy, 100.0, Some(3.0)),
            ]),
            StorageCapacities {
                food: 1_400.0,
                water: 400.0,
                herbs: 400.0,
                materials: 400.0,
                refined: 250.0,
                weapons: 200.0,
                armor: 200.0,
                planks: 100.0,
                blocks: 100.0,
                tools: 100.0,
            },
            "mixed finished buildings",
        );

        assert_caps_bits(
            storage_capacities_default(&[
                building(BuildingType::FoodStorage, 40.0, Some(9.0)),
                building(BuildingType::WaterBowl, 99.99, Some(9.0)),
                building(BuildingType::Smithy, f64::NAN, Some(9.0)),
            ]),
            BASE_CAPACITY,
            "unfinished buildings",
        );
    }

    #[test]
    fn storage_multiplier_scales_only_building_bonus() {
        assert_caps_bits(
            storage_capacities(
                &[
                    building(BuildingType::FoodStorage, 100.0, None),
                    building(BuildingType::WaterBowl, 100.0, Some(2.0)),
                    building(BuildingType::Smithy, 100.0, Some(2.0)),
                ],
                1.25,
            ),
            StorageCapacities {
                food: 700.0,
                water: 700.0,
                herbs: 225.0,
                materials: 225.0,
                refined: 162.5,
                weapons: 175.0,
                armor: 175.0,
                planks: 100.0,
                blocks: 100.0,
                tools: 100.0,
            },
            "scaled building bonuses",
        );

        assert_caps_bits(
            storage_capacities_with_mult(
                &[building(BuildingType::FoodStorage, 100.0, Some(1.0))],
                -2.0,
            ),
            BASE_CAPACITY,
            "negative multiplier clamps to zero",
        );
    }

    #[test]
    fn level_defaults_and_clamps_like_typescript() {
        assert_caps_bits(
            storage_capacities_default(&[
                building(BuildingType::FoodStorage, 100.0, None),
                building(BuildingType::WaterBowl, 100.0, Some(0.0)),
                building(BuildingType::Smithy, 100.0, Some(-4.0)),
            ]),
            StorageCapacities {
                food: 600.0,
                water: 400.0,
                herbs: 200.0,
                materials: 200.0,
                refined: 150.0,
                weapons: 100.0,
                armor: 100.0,
                planks: 100.0,
                blocks: 100.0,
                tools: 100.0,
            },
            "default and minimum level",
        );
    }

    #[test]
    fn storehouse_cap_matches_population_floor() {
        assert_eq!(storehouse_cap(0), 1);
        assert_eq!(storehouse_cap(5), 1);
        assert_eq!(storehouse_cap(6), 1);
        assert_eq!(storehouse_cap(20), 3);
        assert_eq!(storehouse_cap(60), 10);
    }

    #[test]
    fn count_storehouses_counts_only_finished_granaries() {
        assert_eq!(
            count_storehouses(&[
                building(BuildingType::FoodStorage, 100.0, Some(1.0)),
                building(BuildingType::FoodStorage, 60.0, Some(1.0)),
                building(BuildingType::WaterBowl, 100.0, Some(1.0)),
                building(BuildingType::Den, 100.0, Some(1.0)),
            ]),
            1
        );
    }

    #[test]
    fn nan_inputs_follow_typescript_math_max_and_comparisons() {
        let caps = storage_capacities_with_mult(
            &[building(BuildingType::FoodStorage, 100.0, Some(1.0))],
            f64::NAN,
        );
        assert!(caps.food.is_nan());
        assert_f64_bits(caps.water, BASE_CAPACITY.water, "water unchanged");
        assert!(caps.herbs.is_nan());
        assert!(caps.materials.is_nan());
        assert!(caps.refined.is_nan());

        let caps =
            storage_capacities_default(&[building(BuildingType::WaterBowl, 100.0, Some(f64::NAN))]);
        assert!(caps.water.is_nan());
        assert_f64_bits(caps.food, BASE_CAPACITY.food, "food unchanged");
    }
}
