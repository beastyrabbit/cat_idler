//! Housing and village growth rules ported from `lib/game/housing.ts`.

use serde::{Deserialize, Serialize};

use crate::types::BuildingType;

/// Minimal building shape the housing math needs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HousingBuilding {
    pub building_type: BuildingType,
    pub level: f64,
    pub construction_progress: f64,
}

impl HousingBuilding {
    #[must_use]
    pub const fn new(building_type: BuildingType, level: f64, construction_progress: f64) -> Self {
        Self {
            building_type,
            level,
            construction_progress,
        }
    }
}

/// Cats sheltered by the shrine itself.
pub const SHRINE_CAPACITY: f64 = 4.0;

/// Cats sheltered per den level.
pub const DEN_CAPACITY_PER_LEVEL: f64 = 2.0;

/// Leader plans a new den when pressure reaches this.
pub const HOUSE_PRESSURE_THRESHOLD: f64 = 0.8;

/// Completed non-shrine buildings needed for each village level past 1.
pub const VILLAGE_LEVEL_THRESHOLDS: [usize; 4] = [6, 12, 20, 30];

#[must_use]
pub fn housing_capacity_default(buildings: &[HousingBuilding]) -> f64 {
    housing_capacity(buildings, 0.0)
}

#[must_use]
pub fn housing_capacity(buildings: &[HousingBuilding], extra_per_den: f64) -> f64 {
    let mut capacity = 0.0;

    for building in buildings {
        if !is_complete(*building) {
            continue;
        }

        match building.building_type {
            BuildingType::Shrine => {
                capacity += SHRINE_CAPACITY;
            }
            BuildingType::Den => {
                capacity += DEN_CAPACITY_PER_LEVEL * js_max(1.0, building.level)
                    + js_max(0.0, extra_per_den);
            }
            _ => {}
        }
    }

    capacity
}

#[must_use]
pub fn housing_pressure(population: f64, capacity: f64) -> f64 {
    if population <= 0.0 {
        return 0.0;
    }
    if capacity <= 0.0 {
        return f64::INFINITY;
    }
    population / capacity
}

#[must_use]
pub fn should_queue_house(pressure: f64) -> bool {
    pressure >= HOUSE_PRESSURE_THRESHOLD
}

#[must_use]
pub fn village_level(buildings: &[HousingBuilding]) -> u32 {
    let completed = buildings
        .iter()
        .filter(|building| {
            building.building_type != BuildingType::Shrine && is_complete(**building)
        })
        .count();

    let mut level = 1;
    for threshold in VILLAGE_LEVEL_THRESHOLDS {
        if completed >= threshold {
            level += 1;
        }
    }
    level
}

fn is_complete(building: HousingBuilding) -> bool {
    building.construction_progress >= 100.0
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
        DEN_CAPACITY_PER_LEVEL, HOUSE_PRESSURE_THRESHOLD, HousingBuilding, SHRINE_CAPACITY,
        VILLAGE_LEVEL_THRESHOLDS, housing_capacity, housing_capacity_default, housing_pressure,
        should_queue_house, village_level,
    };
    use crate::types::BuildingType;

    fn building(
        building_type: BuildingType,
        level: f64,
        construction_progress: f64,
    ) -> HousingBuilding {
        HousingBuilding {
            building_type,
            level,
            construction_progress,
        }
    }

    fn den() -> HousingBuilding {
        building(BuildingType::Den, 1.0, 100.0)
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    #[test]
    fn constants_match_typescript_exports() {
        assert_f64_bits(SHRINE_CAPACITY, 4.0, "shrine capacity");
        assert_f64_bits(DEN_CAPACITY_PER_LEVEL, 2.0, "den capacity per level");
        assert_f64_bits(HOUSE_PRESSURE_THRESHOLD, 0.8, "house pressure threshold");
        assert_eq!(VILLAGE_LEVEL_THRESHOLDS, [6, 12, 20, 30]);
    }

    #[test]
    fn capacity_counts_finished_shrine_and_den_levels() {
        let buildings = [
            building(BuildingType::Shrine, 1.0, 100.0),
            den(),
            building(BuildingType::Den, 2.0, 100.0),
        ];

        assert_f64_bits(
            housing_capacity_default(&buildings),
            4.0 + 2.0 + 4.0,
            "shrine plus dens",
        );
    }

    #[test]
    fn capacity_ignores_unfinished_and_non_housing_buildings() {
        let buildings = [
            building(BuildingType::Shrine, 1.0, 100.0),
            building(BuildingType::Den, 1.0, 40.0),
            building(BuildingType::FoodStorage, 3.0, 100.0),
        ];

        assert_f64_bits(
            housing_capacity_default(&buildings),
            4.0,
            "finished housing",
        );
        assert_f64_bits(housing_capacity_default(&[]), 0.0, "empty building list");
    }

    #[test]
    fn capacity_adds_upgrade_bonus_flat_per_finished_den_only() {
        let buildings = [
            building(BuildingType::Shrine, 1.0, 100.0),
            den(),
            building(BuildingType::Den, 2.0, 100.0),
        ];

        assert_f64_bits(
            housing_capacity(&buildings, 1.0),
            4.0 + (2.0 + 1.0) + (4.0 + 1.0),
            "per-den bonus",
        );
        assert_f64_bits(
            housing_capacity(&[building(BuildingType::Shrine, 1.0, 100.0)], 3.0),
            4.0,
            "shrine receives no per-den bonus",
        );
        assert_f64_bits(
            housing_capacity(&[den()], -9.0),
            2.0,
            "negative bonus clamps to zero",
        );
    }

    #[test]
    fn capacity_clamps_den_level_to_at_least_one_like_typescript() {
        let buildings = [
            building(BuildingType::Den, 0.0, 100.0),
            building(BuildingType::Den, -4.0, 100.0),
            building(BuildingType::Den, 2.5, 100.0),
        ];

        assert_f64_bits(
            housing_capacity_default(&buildings),
            2.0 + 2.0 + 5.0,
            "levels",
        );
    }

    #[test]
    fn capacity_preserves_typescript_nan_edges() {
        assert!(housing_capacity_default(&[building(BuildingType::Den, f64::NAN, 100.0)]).is_nan());
        assert!(housing_capacity(&[den()], f64::NAN).is_nan());

        assert_f64_bits(
            housing_capacity_default(&[building(BuildingType::Den, 9.0, f64::NAN)]),
            0.0,
            "nan construction progress is incomplete",
        );
    }

    #[test]
    fn pressure_is_population_over_capacity_with_empty_and_zero_capacity_cases() {
        assert_f64_bits(housing_pressure(10.0, 20.0), 0.5, "half full");
        assert_f64_bits(housing_pressure(20.0, 14.0), 20.0 / 14.0, "over capacity");
        assert!(housing_pressure(5.0, 0.0).is_infinite());
        assert_f64_bits(housing_pressure(0.0, 0.0), 0.0, "empty with no capacity");
        assert_f64_bits(housing_pressure(0.0, 10.0), 0.0, "empty with capacity");
    }

    #[test]
    fn should_queue_house_uses_inclusive_threshold() {
        assert!(should_queue_house(HOUSE_PRESSURE_THRESHOLD));
        assert!(!should_queue_house(HOUSE_PRESSURE_THRESHOLD - 0.001));
        assert!(should_queue_house(HOUSE_PRESSURE_THRESHOLD + 0.5));
        assert!(!should_queue_house(f64::NAN));
    }

    #[test]
    fn village_level_counts_finished_non_shrine_buildings() {
        assert_eq!(
            village_level(&[building(BuildingType::Shrine, 1.0, 100.0)]),
            1
        );

        let mut starter = vec![building(BuildingType::Shrine, 1.0, 100.0)];
        starter.extend([den(); 5]);
        starter.push(building(BuildingType::FoodStorage, 1.0, 100.0));
        assert_eq!(village_level(&starter), 2);

        let mut grown = vec![building(BuildingType::Shrine, 1.0, 100.0)];
        grown.extend([den(); 12]);
        assert_eq!(village_level(&grown), 3);

        let mut city = vec![building(BuildingType::Shrine, 1.0, 100.0)];
        city.extend([den(); 20]);
        assert_eq!(village_level(&city), 4);
    }

    #[test]
    fn village_level_boundaries_include_all_thresholds_and_ignore_scaffolds() {
        for (completed_count, expected_level) in [
            (5, 1),
            (6, 2),
            (11, 2),
            (12, 3),
            (19, 3),
            (20, 4),
            (29, 4),
            (30, 5),
        ] {
            let buildings = vec![den(); completed_count];
            assert_eq!(
                village_level(&buildings),
                expected_level,
                "{completed_count} completed buildings"
            );
        }

        let scaffolds = vec![building(BuildingType::Den, 1.0, 10.0); 8];
        assert_eq!(village_level(&scaffolds), 1);
    }
}
