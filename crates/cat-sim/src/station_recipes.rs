//! Data-owned production recipe descriptors for staffed stations.
//!
//! The authoritative target contract is P19's canonical production table. This
//! module describes recipe identity and resource domains only. Every maintained
//! processor now follows a finite physical route through station-local stores.

use crate::{
    items::{ItemKind, Material},
    stockpiles::ResourceKind,
    types::BuildingType,
};

pub const SAWMILL_RECIPE_ID: &str = "logs_to_lumber";
pub const GRAIN_TO_FLOUR_RECIPE_ID: &str = "grain_to_flour";
pub const FLOUR_TO_FOOD_RECIPE_ID: &str = "flour_to_food";
/// Pre-split Mill queue ID written by rules through P19. Kept only so persisted
/// villages can be migrated to the two physically distinct operations.
pub const LEGACY_COMBINED_MILL_RECIPE_ID: &str = "grain_to_flour_and_food";
/// Compatibility name used by older call sites for the Mill's first selected recipe.
pub const MILL_RECIPE_ID: &str = GRAIN_TO_FLOUR_RECIPE_ID;
pub const WORKSHOP_RECIPE_ID: &str = "materials_to_refined";
pub const SMELTER_RECIPE_ID: &str = "ore_to_metal";
pub const LOGS_TO_PLANKS_RECIPE_ID: &str = "logs_to_planks";
pub const STONE_TO_BLOCKS_RECIPE_ID: &str = "stone_to_blocks";
pub const PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID: &str = "planks_and_blocks_to_tools";
pub const FIBRE_TO_CLOTH_RECIPE_ID: &str = "fibre_to_cloth";
pub const HIDE_TO_LEATHER_RECIPE_ID: &str = "hide_to_leather";
pub const SMITHY_WEAPON_RECIPE_ID: &str = "smithy_weapon";
pub const SMITHY_ARMOR_RECIPE_ID: &str = "smithy_armor";
pub const SMITHY_TOOL_RECIPE_ID: &str = "smithy_tool";
pub const BONE_TOOL_RECIPE_ID: &str = "bone_tool";
pub const BONE_TRINKET_RECIPE_ID: &str = "bone_trinket";
pub const BONE_TOY_RECIPE_ID: &str = "bone_toy";
pub const GEM_TRINKET_RECIPE_ID: &str = "gem_jewelry";
pub const CLAY_MUG_RECIPE_ID: &str = "clay_mug";
pub const CLAY_BOWL_RECIPE_ID: &str = "clay_bowl";
pub const CLAY_BRICK_RECIPE_ID: &str = "clay_brick";
pub const SAND_MUG_RECIPE_ID: &str = "sand_glass_mug";
pub const SAND_BOWL_RECIPE_ID: &str = "sand_glass_bowl";
pub const SAND_TRINKET_RECIPE_ID: &str = "sand_glass_trinket";

/// Exact finite output identity for a material-variant station recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationItemOutput {
    pub kind: ItemKind,
    pub material: Material,
    pub quality: u8,
}

/// One stable queue recipe and the finite resource kinds it consumes and
/// produces through station-local stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationRecipeDescriptor {
    pub id: &'static str,
    pub building_type: BuildingType,
    pub input_resources: &'static [ResourceKind],
    pub output_resources: &'static [ResourceKind],
    /// Exact item emitted into the station-local finite ledger. Such recipes
    /// deliberately have no fake scalar output resource.
    pub output_item: Option<StationItemOutput>,
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
const WORKSHOP_INPUTS: &[ResourceKind] = &[
    ResourceKind::Materials,
    ResourceKind::Gem,
    ResourceKind::Sand,
];
const WORKSHOP_OUTPUTS: &[ResourceKind] = &[ResourceKind::Refined];
const SMELTER_INPUTS: &[ResourceKind] = &[ResourceKind::Ore];
const SMELTER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Metal];
const WOOD_CUTTER_INPUTS: &[ResourceKind] = &[ResourceKind::Logs];
const WOOD_CUTTER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Planks];
const STONE_PREP_INPUTS: &[ResourceKind] =
    &[ResourceKind::Stone, ResourceKind::Bone, ResourceKind::Clay];
const STONE_PREP_OUTPUTS: &[ResourceKind] = &[ResourceKind::Blocks];
const WOODWORKING_INPUTS: &[ResourceKind] = &[
    ResourceKind::Planks,
    ResourceKind::Blocks,
    ResourceKind::Bone,
];
const WOODWORKING_OUTPUTS: &[ResourceKind] = &[ResourceKind::Tools];
const CLOTHIER_INPUTS: &[ResourceKind] = &[ResourceKind::Fibre];
const CLOTHIER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Cloth];
const TANNERY_INPUTS: &[ResourceKind] = &[ResourceKind::Hide];
const TANNERY_OUTPUTS: &[ResourceKind] = &[ResourceKind::Leather];
const SMITHY_INPUTS: &[ResourceKind] = &[ResourceKind::Metal];
const SMITHY_OUTPUTS: &[ResourceKind] = &[
    ResourceKind::Weapons,
    ResourceKind::Armor,
    ResourceKind::Tools,
];

const MILL_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: GRAIN_TO_FLOUR_RECIPE_ID,
        building_type: BuildingType::Mill,
        input_resources: &[ResourceKind::Grain],
        output_resources: &[ResourceKind::Flour],
        output_item: None,
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: FLOUR_TO_FOOD_RECIPE_ID,
        building_type: BuildingType::Mill,
        input_resources: &[ResourceKind::Flour],
        output_resources: &[ResourceKind::Food],
        output_item: None,
        founding_available: false,
    },
];
const SAWMILL_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: SAWMILL_RECIPE_ID,
    building_type: BuildingType::Sawmill,
    input_resources: SAWMILL_INPUTS,
    output_resources: SAWMILL_OUTPUTS,
    output_item: None,
    founding_available: false,
}];
const WORKSHOP_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: WORKSHOP_RECIPE_ID,
        building_type: BuildingType::Workshop,
        input_resources: &[ResourceKind::Materials],
        output_resources: WORKSHOP_OUTPUTS,
        output_item: None,
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: GEM_TRINKET_RECIPE_ID,
        building_type: BuildingType::Workshop,
        input_resources: &[ResourceKind::Gem],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Trinket,
            material: Material::Gem,
            quality: 2,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: SAND_MUG_RECIPE_ID,
        building_type: BuildingType::Workshop,
        input_resources: &[ResourceKind::Sand],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Mug,
            material: Material::Sand,
            quality: 1,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: SAND_BOWL_RECIPE_ID,
        building_type: BuildingType::Workshop,
        input_resources: &[ResourceKind::Sand],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Bowl,
            material: Material::Sand,
            quality: 1,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: SAND_TRINKET_RECIPE_ID,
        building_type: BuildingType::Workshop,
        input_resources: &[ResourceKind::Sand],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Trinket,
            material: Material::Sand,
            quality: 2,
        }),
        founding_available: false,
    },
];
const SMELTER_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: SMELTER_RECIPE_ID,
    building_type: BuildingType::Smelter,
    input_resources: SMELTER_INPUTS,
    output_resources: SMELTER_OUTPUTS,
    output_item: None,
    founding_available: false,
}];
const WOOD_CUTTER_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: LOGS_TO_PLANKS_RECIPE_ID,
    building_type: BuildingType::WoodCutter,
    input_resources: WOOD_CUTTER_INPUTS,
    output_resources: WOOD_CUTTER_OUTPUTS,
    output_item: None,
    founding_available: true,
}];
const STONE_PREP_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: STONE_TO_BLOCKS_RECIPE_ID,
        building_type: BuildingType::StonePrep,
        input_resources: &[ResourceKind::Stone],
        output_resources: STONE_PREP_OUTPUTS,
        output_item: None,
        founding_available: true,
    },
    StationRecipeDescriptor {
        id: BONE_TRINKET_RECIPE_ID,
        building_type: BuildingType::StonePrep,
        input_resources: &[ResourceKind::Bone],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Trinket,
            material: Material::Bone,
            quality: 1,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: BONE_TOY_RECIPE_ID,
        building_type: BuildingType::StonePrep,
        input_resources: &[ResourceKind::Bone],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Toy,
            material: Material::Bone,
            quality: 1,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: CLAY_MUG_RECIPE_ID,
        building_type: BuildingType::StonePrep,
        input_resources: &[ResourceKind::Clay],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Mug,
            material: Material::Clay,
            quality: 1,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: CLAY_BOWL_RECIPE_ID,
        building_type: BuildingType::StonePrep,
        input_resources: &[ResourceKind::Clay],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Bowl,
            material: Material::Clay,
            quality: 1,
        }),
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: CLAY_BRICK_RECIPE_ID,
        building_type: BuildingType::StonePrep,
        input_resources: &[ResourceKind::Clay],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Brick,
            material: Material::Clay,
            quality: 1,
        }),
        founding_available: false,
    },
];
const WOODWORKING_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID,
        building_type: BuildingType::Woodworking,
        input_resources: &[ResourceKind::Planks, ResourceKind::Blocks],
        output_resources: WOODWORKING_OUTPUTS,
        output_item: None,
        founding_available: true,
    },
    StationRecipeDescriptor {
        id: BONE_TOOL_RECIPE_ID,
        building_type: BuildingType::Woodworking,
        input_resources: &[ResourceKind::Bone],
        output_resources: &[],
        output_item: Some(StationItemOutput {
            kind: ItemKind::Tool,
            material: Material::Bone,
            quality: 1,
        }),
        founding_available: false,
    },
];
const CLOTHIER_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: FIBRE_TO_CLOTH_RECIPE_ID,
    building_type: BuildingType::Clothier,
    input_resources: CLOTHIER_INPUTS,
    output_resources: CLOTHIER_OUTPUTS,
    output_item: None,
    founding_available: false,
}];
const TANNERY_RECIPES: &[StationRecipeDescriptor] = &[StationRecipeDescriptor {
    id: HIDE_TO_LEATHER_RECIPE_ID,
    building_type: BuildingType::Tannery,
    input_resources: TANNERY_INPUTS,
    output_resources: TANNERY_OUTPUTS,
    output_item: None,
    founding_available: false,
}];
const SMITHY_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: SMITHY_WEAPON_RECIPE_ID,
        building_type: BuildingType::Smithy,
        input_resources: SMITHY_INPUTS,
        output_resources: &[ResourceKind::Weapons],
        output_item: None,
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: SMITHY_TOOL_RECIPE_ID,
        building_type: BuildingType::Smithy,
        input_resources: SMITHY_INPUTS,
        output_resources: &[ResourceKind::Tools],
        output_item: None,
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: SMITHY_ARMOR_RECIPE_ID,
        building_type: BuildingType::Smithy,
        input_resources: SMITHY_INPUTS,
        output_resources: &[ResourceKind::Armor],
        output_item: None,
        founding_available: false,
    },
];

/// Whether a catalog recipe payload names a physical runtime recipe.
#[must_use]
pub fn is_runtime_recipe_id(recipe_id: &str) -> bool {
    BuildingType::ALL.iter().copied().any(|building_type| {
        station_recipe_set(building_type)
            .is_some_and(|station| station.recipes.iter().any(|recipe| recipe.id == recipe_id))
    })
}

/// Resolve an exact finite material-variant output from the same descriptor
/// registry used by queue entitlement and station controls.
#[must_use]
pub fn station_item_recipe(recipe_id: &str) -> Option<&'static StationRecipeDescriptor> {
    BuildingType::ALL
        .iter()
        .copied()
        .filter_map(station_recipe_set)
        .flat_map(|station| station.recipes)
        .find(|recipe| recipe.id == recipe_id && recipe.output_item.is_some())
}

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
                &[
                    STONE_TO_BLOCKS_RECIPE_ID,
                    BONE_TRINKET_RECIPE_ID,
                    BONE_TOY_RECIPE_ID,
                    CLAY_MUG_RECIPE_ID,
                    CLAY_BOWL_RECIPE_ID,
                    CLAY_BRICK_RECIPE_ID,
                ][..],
                &[ResourceKind::Stone, ResourceKind::Bone, ResourceKind::Clay][..],
                &[ResourceKind::Blocks][..],
            ),
            (
                BuildingType::Woodworking,
                &[PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID, BONE_TOOL_RECIPE_ID][..],
                &[
                    ResourceKind::Planks,
                    ResourceKind::Blocks,
                    ResourceKind::Bone,
                ][..],
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
                &[
                    SMITHY_WEAPON_RECIPE_ID,
                    SMITHY_TOOL_RECIPE_ID,
                    SMITHY_ARMOR_RECIPE_ID,
                ][..],
                &[ResourceKind::Metal][..],
                &[
                    ResourceKind::Weapons,
                    ResourceKind::Armor,
                    ResourceKind::Tools,
                ][..],
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
                    && (!recipe.output_resources.is_empty() || recipe.output_item.is_some())
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
        assert_eq!(ids.len(), 23);
        assert!(station_recipe_set(BuildingType::Den).is_none());
    }

    #[test]
    fn sourced_breadth_uses_explicit_mill_steps_and_metal_tools() {
        let mill = station_recipe_set(BuildingType::Mill).unwrap();
        assert_eq!(
            mill.recipes
                .iter()
                .map(|recipe| recipe.id)
                .collect::<Vec<_>>(),
            [GRAIN_TO_FLOUR_RECIPE_ID, FLOUR_TO_FOOD_RECIPE_ID]
        );
        assert_eq!(mill.recipes[0].input_resources, &[ResourceKind::Grain]);
        assert_eq!(mill.recipes[0].output_resources, &[ResourceKind::Flour]);
        assert_eq!(mill.recipes[1].input_resources, &[ResourceKind::Flour]);
        assert_eq!(mill.recipes[1].output_resources, &[ResourceKind::Food]);

        let smithy = station_recipe_set(BuildingType::Smithy).unwrap();
        assert_eq!(
            smithy
                .recipes
                .iter()
                .map(|recipe| recipe.id)
                .collect::<Vec<_>>(),
            [
                SMITHY_WEAPON_RECIPE_ID,
                SMITHY_TOOL_RECIPE_ID,
                SMITHY_ARMOR_RECIPE_ID,
            ]
        );
        assert_eq!(smithy.recipes[1].output_resources, &[ResourceKind::Tools]);
        assert!(mill.recipes.iter().all(|recipe| !recipe.founding_available));
        assert!(!smithy.recipes[1].founding_available);
    }

    #[test]
    fn bone_gem_clay_and_sand_have_exhaustive_exact_item_routes() {
        let expected = [
            (
                BONE_TOOL_RECIPE_ID,
                BuildingType::Woodworking,
                ResourceKind::Bone,
                ItemKind::Tool,
                Material::Bone,
            ),
            (
                BONE_TRINKET_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Bone,
                ItemKind::Trinket,
                Material::Bone,
            ),
            (
                BONE_TOY_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Bone,
                ItemKind::Toy,
                Material::Bone,
            ),
            (
                GEM_TRINKET_RECIPE_ID,
                BuildingType::Workshop,
                ResourceKind::Gem,
                ItemKind::Trinket,
                Material::Gem,
            ),
            (
                CLAY_MUG_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Clay,
                ItemKind::Mug,
                Material::Clay,
            ),
            (
                CLAY_BOWL_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Clay,
                ItemKind::Bowl,
                Material::Clay,
            ),
            (
                CLAY_BRICK_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Clay,
                ItemKind::Brick,
                Material::Clay,
            ),
            (
                SAND_MUG_RECIPE_ID,
                BuildingType::Workshop,
                ResourceKind::Sand,
                ItemKind::Mug,
                Material::Sand,
            ),
            (
                SAND_BOWL_RECIPE_ID,
                BuildingType::Workshop,
                ResourceKind::Sand,
                ItemKind::Bowl,
                Material::Sand,
            ),
            (
                SAND_TRINKET_RECIPE_ID,
                BuildingType::Workshop,
                ResourceKind::Sand,
                ItemKind::Trinket,
                Material::Sand,
            ),
        ];
        for (id, building, input, kind, material) in expected {
            let recipe = station_item_recipe(id).expect("variant recipe is runtime-backed");
            assert_eq!(recipe.building_type, building, "{id}");
            assert_eq!(recipe.input_resources, &[input], "{id}");
            assert!(
                recipe.output_resources.is_empty(),
                "{id} has no fake scalar output"
            );
            let output = recipe.output_item.expect("exact output");
            assert_eq!((output.kind, output.material), (kind, material), "{id}");
            assert!(!recipe.founding_available, "{id}");
        }
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
