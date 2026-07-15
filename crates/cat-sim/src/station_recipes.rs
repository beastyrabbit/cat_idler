//! Data-owned production recipe descriptors for staffed stations.
//!
//! The authoritative target contract is P19's canonical production table. This
//! module describes recipe identity and resource domains only. Every maintained
//! processor now follows a finite physical route through station-local stores.

use crate::{stockpiles::ResourceKind, types::BuildingType};

pub const SAWMILL_RECIPE_ID: &str = "logs_to_lumber";
pub const MILL_RECIPE_ID: &str = "grain_to_flour_and_food";
pub const WORKSHOP_RECIPE_ID: &str = "materials_to_refined";
pub const SMELTER_RECIPE_ID: &str = "ore_to_metal";
pub const LOGS_TO_PLANKS_RECIPE_ID: &str = "logs_to_planks";
pub const STONE_TO_BLOCKS_RECIPE_ID: &str = "stone_to_blocks";
pub const PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID: &str = "planks_and_blocks_to_tools";
pub const FIBRE_TO_CLOTH_RECIPE_ID: &str = "fibre_to_cloth";
pub const HIDE_TO_LEATHER_RECIPE_ID: &str = "hide_to_leather";
pub const SMITHY_WEAPON_RECIPE_ID: &str = "smithy_weapon";
pub const SMITHY_ARMOR_RECIPE_ID: &str = "smithy_armor";

/// One stable queue recipe and the finite resource kinds it consumes and
/// produces through station-local stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationRecipeDescriptor {
    pub id: &'static str,
    pub building_type: BuildingType,
    pub input_resources: &'static [ResourceKind],
    pub output_resources: &'static [ResourceKind],
    /// Baseline recipe of a founding-placeable bench. Catalog studies may own
    /// later recipes without making this first survival chain unavailable.
    pub founding_available: bool,
}

/// Complete recipe and local-store domain owned by one station type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationRecipeSet {
    pub recipes: &'static [StationRecipeDescriptor],
    pub input_resources: &'static [ResourceKind],
    pub output_resources: &'static [ResourceKind],
}

const MILL_INPUTS: &[ResourceKind] = &[ResourceKind::Grain, ResourceKind::Flour];
const MILL_OUTPUTS: &[ResourceKind] = &[ResourceKind::Food, ResourceKind::Flour];
const SAWMILL_INPUTS: &[ResourceKind] = &[ResourceKind::Logs];
const SAWMILL_OUTPUTS: &[ResourceKind] = &[ResourceKind::Lumber];
const WORKSHOP_INPUTS: &[ResourceKind] = &[ResourceKind::Materials];
const WORKSHOP_OUTPUTS: &[ResourceKind] = &[ResourceKind::Refined];
const SMELTER_INPUTS: &[ResourceKind] = &[ResourceKind::Ore];
const SMELTER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Metal];
const WOOD_CUTTER_INPUTS: &[ResourceKind] = &[ResourceKind::Logs];
const WOOD_CUTTER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Planks];
const STONE_PREP_INPUTS: &[ResourceKind] = &[ResourceKind::Stone];
const STONE_PREP_OUTPUTS: &[ResourceKind] = &[ResourceKind::Blocks];
const WOODWORKING_INPUTS: &[ResourceKind] = &[ResourceKind::Planks, ResourceKind::Blocks];
const WOODWORKING_OUTPUTS: &[ResourceKind] = &[ResourceKind::Tools];
const CLOTHIER_INPUTS: &[ResourceKind] = &[ResourceKind::Fibre];
const CLOTHIER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Cloth];
const TANNERY_INPUTS: &[ResourceKind] = &[ResourceKind::Hide];
const TANNERY_OUTPUTS: &[ResourceKind] = &[ResourceKind::Leather];
const SMITHY_INPUTS: &[ResourceKind] = &[ResourceKind::Metal];
const SMITHY_OUTPUTS: &[ResourceKind] = &[ResourceKind::Weapons, ResourceKind::Armor];

const MILL_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: MILL_RECIPE_ID,
    building_type: BuildingType::Mill,
    input_resources: MILL_INPUTS,
    output_resources: MILL_OUTPUTS,
    founding_available: false,
}];
const SAWMILL_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: SAWMILL_RECIPE_ID,
    building_type: BuildingType::Sawmill,
    input_resources: SAWMILL_INPUTS,
    output_resources: SAWMILL_OUTPUTS,
    founding_available: false,
}];
const WORKSHOP_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: WORKSHOP_RECIPE_ID,
    building_type: BuildingType::Workshop,
    input_resources: WORKSHOP_INPUTS,
    output_resources: WORKSHOP_OUTPUTS,
    founding_available: false,
}];
const SMELTER_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: SMELTER_RECIPE_ID,
    building_type: BuildingType::Smelter,
    input_resources: SMELTER_INPUTS,
    output_resources: SMELTER_OUTPUTS,
    founding_available: false,
}];
const WOOD_CUTTER_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: LOGS_TO_PLANKS_RECIPE_ID,
    building_type: BuildingType::WoodCutter,
    input_resources: WOOD_CUTTER_INPUTS,
    output_resources: WOOD_CUTTER_OUTPUTS,
    founding_available: true,
}];
const STONE_PREP_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: STONE_TO_BLOCKS_RECIPE_ID,
    building_type: BuildingType::StonePrep,
    input_resources: STONE_PREP_INPUTS,
    output_resources: STONE_PREP_OUTPUTS,
    founding_available: true,
}];
const WOODWORKING_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID,
    building_type: BuildingType::Woodworking,
    input_resources: WOODWORKING_INPUTS,
    output_resources: WOODWORKING_OUTPUTS,
    founding_available: true,
}];
const CLOTHIER_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: FIBRE_TO_CLOTH_RECIPE_ID,
    building_type: BuildingType::Clothier,
    input_resources: CLOTHIER_INPUTS,
    output_resources: CLOTHIER_OUTPUTS,
    founding_available: false,
}];
const TANNERY_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: HIDE_TO_LEATHER_RECIPE_ID,
    building_type: BuildingType::Tannery,
    input_resources: TANNERY_INPUTS,
    output_resources: TANNERY_OUTPUTS,
    founding_available: false,
}];
const SMITHY_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: SMITHY_WEAPON_RECIPE_ID,
        building_type: BuildingType::Smithy,
        input_resources: SMITHY_INPUTS,
        output_resources: &[ResourceKind::Weapons],
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: SMITHY_ARMOR_RECIPE_ID,
        building_type: BuildingType::Smithy,
        input_resources: SMITHY_INPUTS,
        output_resources: &[ResourceKind::Armor],
        founding_available: false,
    },
];

/// The single data source for recipe IDs and station-local resource domains.
#[must_use]
pub const fn station_recipe_set(building_type: BuildingType) -> Option<StationRecipeSet> {
    let (recipes, input_resources, output_resources) = match building_type {
        BuildingType::Mill => (MILL_RECIPES, MILL_INPUTS, MILL_OUTPUTS),
        BuildingType::Sawmill => (SAWMILL_RECIPES, SAWMILL_INPUTS, SAWMILL_OUTPUTS),
        BuildingType::Workshop => (WORKSHOP_RECIPES, WORKSHOP_INPUTS, WORKSHOP_OUTPUTS),
        BuildingType::Smelter => (SMELTER_RECIPES, SMELTER_INPUTS, SMELTER_OUTPUTS),
        BuildingType::WoodCutter => (WOOD_CUTTER_RECIPES, WOOD_CUTTER_INPUTS, WOOD_CUTTER_OUTPUTS),
        BuildingType::StonePrep => (STONE_PREP_RECIPES, STONE_PREP_INPUTS, STONE_PREP_OUTPUTS),
        BuildingType::Woodworking => (WOODWORKING_RECIPES, WOODWORKING_INPUTS, WOODWORKING_OUTPUTS),
        BuildingType::Clothier => (CLOTHIER_RECIPES, CLOTHIER_INPUTS, CLOTHIER_OUTPUTS),
        BuildingType::Tannery => (TANNERY_RECIPES, TANNERY_INPUTS, TANNERY_OUTPUTS),
        BuildingType::Smithy => (SMITHY_RECIPES, SMITHY_INPUTS, SMITHY_OUTPUTS),
        _ => return None,
    };
    Some(StationRecipeSet {
        recipes,
        input_resources,
        output_resources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{stockpiles::ResourceKind, types::BuildingType};

    #[test]
    fn production_benches_have_stable_data_owned_recipes() {
        let cases = [
            (
                BuildingType::WoodCutter,
                &[LOGS_TO_PLANKS_RECIPE_ID][..],
                &[ResourceKind::Logs][..],
                &[ResourceKind::Planks][..],
            ),
            (
                BuildingType::StonePrep,
                &[STONE_TO_BLOCKS_RECIPE_ID][..],
                &[ResourceKind::Stone][..],
                &[ResourceKind::Blocks][..],
            ),
            (
                BuildingType::Woodworking,
                &[PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID][..],
                &[ResourceKind::Planks, ResourceKind::Blocks][..],
                &[ResourceKind::Tools][..],
            ),
            (
                BuildingType::Clothier,
                &[FIBRE_TO_CLOTH_RECIPE_ID][..],
                &[ResourceKind::Fibre][..],
                &[ResourceKind::Cloth][..],
            ),
            (
                BuildingType::Tannery,
                &[HIDE_TO_LEATHER_RECIPE_ID][..],
                &[ResourceKind::Hide][..],
                &[ResourceKind::Leather][..],
            ),
            (
                BuildingType::Smithy,
                &[SMITHY_WEAPON_RECIPE_ID, SMITHY_ARMOR_RECIPE_ID][..],
                &[ResourceKind::Metal][..],
                &[ResourceKind::Weapons, ResourceKind::Armor][..],
            ),
        ];

        for (building, expected_ids, expected_inputs, expected_outputs) in cases {
            let station = station_recipe_set(building).expect("maintained station descriptor");
            assert_eq!(
                station
                    .recipes
                    .iter()
                    .map(|recipe| recipe.id)
                    .collect::<Vec<_>>(),
                expected_ids,
                "{building:?} recipe order"
            );
            assert_eq!(station.input_resources, expected_inputs, "{building:?}");
            assert_eq!(station.output_resources, expected_outputs, "{building:?}");
            assert!(station.recipes.iter().all(|recipe| {
                recipe.building_type == building
                    && !recipe.input_resources.is_empty()
                    && !recipe.output_resources.is_empty()
            }));
        }
    }

    #[test]
    fn all_ten_queue_stations_have_unique_recipe_ids_and_resource_domains() {
        let stations = [
            BuildingType::Mill,
            BuildingType::Sawmill,
            BuildingType::Workshop,
            BuildingType::Smelter,
            BuildingType::WoodCutter,
            BuildingType::StonePrep,
            BuildingType::Woodworking,
            BuildingType::Clothier,
            BuildingType::Tannery,
            BuildingType::Smithy,
        ];
        let mut ids = std::collections::BTreeSet::new();
        for building in stations {
            let station = station_recipe_set(building).expect("queue station descriptor");
            assert!(!station.recipes.is_empty(), "{building:?}");
            assert!(!station.input_resources.is_empty(), "{building:?}");
            assert!(!station.output_resources.is_empty(), "{building:?}");
            for recipe in station.recipes {
                assert!(ids.insert(recipe.id), "duplicate recipe id {}", recipe.id);
            }
        }
        assert_eq!(ids.len(), 11);
        assert!(station_recipe_set(BuildingType::Den).is_none());
    }

    #[test]
    fn exactly_the_three_founding_bench_baselines_are_available_without_study() {
        let founding = [
            LOGS_TO_PLANKS_RECIPE_ID,
            STONE_TO_BLOCKS_RECIPE_ID,
            PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID,
        ];
        let actual = [
            BuildingType::Mill,
            BuildingType::Sawmill,
            BuildingType::Workshop,
            BuildingType::Smelter,
            BuildingType::WoodCutter,
            BuildingType::StonePrep,
            BuildingType::Woodworking,
            BuildingType::Clothier,
            BuildingType::Tannery,
            BuildingType::Smithy,
        ]
        .into_iter()
        .flat_map(|building| station_recipe_set(building).unwrap().recipes)
        .filter(|recipe| recipe.founding_available)
        .map(|recipe| recipe.id)
        .collect::<Vec<_>>();
        assert_eq!(actual, founding);
    }
}
