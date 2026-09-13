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
pub const FINE_GRAIN_FLOUR_RECIPE_ID: &str = "fine_grain_flour";
pub const STONEGROUND_FLOUR_RECIPE_ID: &str = "stoneground_flour";
pub const MASTERWORK_FLOUR_RECIPE_ID: &str = "masterwork_flour";
pub const BAKE_FLATBREAD_RECIPE_ID: &str = "bake_flatbread";
pub const BAKE_LOAF_RECIPE_ID: &str = "bake_loaf";
pub const BAKE_BISCUITS_RECIPE_ID: &str = "bake_biscuits";
pub const BAKE_FESTIVAL_CAKE_RECIPE_ID: &str = "bake_festival_cake";
pub const BAKE_MASTERWORK_PASTRY_RECIPE_ID: &str = "bake_masterwork_pastry";
pub const HERBAL_POULTICE_RECIPE_ID: &str = "herbal_poultice";
pub const HERBAL_TONIC_RECIPE_ID: &str = "herbal_tonic";
pub const HERBAL_SALVE_RECIPE_ID: &str = "herbal_salve";
pub const HERBAL_REMEDY_RECIPE_ID: &str = "herbal_remedy";
pub const HERBAL_MASTERWORK_REMEDY_RECIPE_ID: &str = "herbal_masterwork_remedy";
pub const DRY_FOOD_RECIPE_ID: &str = "dry_food";
pub const SMOKE_FOOD_RECIPE_ID: &str = "smoke_food";
pub const PICKLE_FOOD_RECIPE_ID: &str = "pickle_food";
pub const PRESERVE_RATIONS_RECIPE_ID: &str = "preserve_rations";
pub const PRESERVE_MASTERWORK_FEAST_RECIPE_ID: &str = "preserve_masterwork_feast";
pub const BREW_GRAIN_SMALL_RECIPE_ID: &str = "brew_grain_small";
pub const BREW_CATNIP_ALE_RECIPE_ID: &str = "brew_catnip_ale";
pub const BREW_HERBAL_TONIC_RECIPE_ID: &str = "brew_herbal_tonic";
pub const BREW_SPICED_ALE_RECIPE_ID: &str = "brew_spiced_ale";
pub const BREW_MASTERWORK_RECIPE_ID: &str = "brew_masterwork";
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
pub const FIBRE_TO_THREAD_RECIPE_ID: &str = "fibre_to_thread";
pub const FIBRE_TO_CLOTH_RECIPE_ID: &str = "fibre_to_cloth";
pub const HIDE_TO_LEATHER_RECIPE_ID: &str = "hide_to_leather";
pub const SMITHY_WEAPON_RECIPE_ID: &str = "smithy_weapon";
pub const SMITHY_ARMOR_RECIPE_ID: &str = "smithy_armor";
pub const SMITHY_TOOL_RECIPE_ID: &str = "smithy_tool";
pub const BONE_TOOL_RECIPE_ID: &str = "bone_tool";
pub const BONE_TRINKET_RECIPE_ID: &str = "bone_trinket";
pub const BONE_TOY_RECIPE_ID: &str = "bone_toy";
pub const BONE_MUG_RECIPE_ID: &str = "bone_mug";
pub const STONE_MUG_RECIPE_ID: &str = "stone_mug";
pub const METAL_MUG_RECIPE_ID: &str = "metal_mug";
pub const GEM_TRINKET_RECIPE_ID: &str = "gem_jewelry";
pub const CLAY_MUG_RECIPE_ID: &str = "clay_mug";
pub const CLAY_BOWL_RECIPE_ID: &str = "clay_bowl";
pub const CLAY_BRICK_RECIPE_ID: &str = "clay_brick";
pub const SAND_MUG_RECIPE_ID: &str = "sand_glass_mug";
pub const SAND_BOWL_RECIPE_ID: &str = "sand_glass_bowl";
pub const SAND_TRINKET_RECIPE_ID: &str = "sand_glass_trinket";
pub const HUNTING_QUALITY_RECIPE_ID: &str = "hunting_quality";
pub const HUNTING_SPECIALTY_RECIPE_ID: &str = "hunting_specialty";
pub const HUNTING_MASTERWORK_RECIPE_ID: &str = "hunting_masterwork";
pub const FORAGING_PREPARATION_RECIPE_ID: &str = "foraging_preparation";
pub const FORAGING_STAPLES_RECIPE_ID: &str = "foraging_staples";
pub const FORAGING_QUALITY_RECIPE_ID: &str = "foraging_quality";
pub const FORAGING_SPECIALTY_RECIPE_ID: &str = "foraging_specialty";
pub const FORAGING_MASTERWORK_RECIPE_ID: &str = "foraging_masterwork";
pub const WATERWORKS_PREPARATION_RECIPE_ID: &str = "waterworks_preparation";
pub const WATERWORKS_STAPLES_RECIPE_ID: &str = "waterworks_staples";
pub const WATERWORKS_QUALITY_RECIPE_ID: &str = "waterworks_quality";
pub const WATERWORKS_SPECIALTY_RECIPE_ID: &str = "waterworks_specialty";
pub const WATERWORKS_MASTERWORK_RECIPE_ID: &str = "waterworks_masterwork";
pub const ANIMAL_HUSBANDRY_PREPARATION_RECIPE_ID: &str = "animal_husbandry_preparation";
pub const ANIMAL_HUSBANDRY_STAPLES_RECIPE_ID: &str = "animal_husbandry_staples";
pub const ANIMAL_HUSBANDRY_QUALITY_RECIPE_ID: &str = "animal_husbandry_quality";
pub const ANIMAL_HUSBANDRY_SPECIALTY_RECIPE_ID: &str = "animal_husbandry_specialty";
pub const ANIMAL_HUSBANDRY_MASTERWORK_RECIPE_ID: &str = "animal_husbandry_masterwork";
pub const FIELD_CRAFT_PREPARATION_RECIPE_ID: &str = "field_craft_preparation";
pub const FIELD_CRAFT_STAPLES_RECIPE_ID: &str = "field_craft_staples";
pub const FIELD_CRAFT_QUALITY_RECIPE_ID: &str = "field_craft_quality";
pub const FIELD_CRAFT_SPECIALTY_RECIPE_ID: &str = "field_craft_specialty";
pub const FIELD_CRAFT_MASTERWORK_RECIPE_ID: &str = "field_craft_masterwork";
pub const EXPEDITION_SUPPLIES_PREPARATION_RECIPE_ID: &str = "expedition_supplies_preparation";
pub const EXPEDITION_SUPPLIES_STAPLES_RECIPE_ID: &str = "expedition_supplies_staples";
pub const EXPEDITION_SUPPLIES_QUALITY_RECIPE_ID: &str = "expedition_supplies_quality";
pub const EXPEDITION_SUPPLIES_SPECIALTY_RECIPE_ID: &str = "expedition_supplies_specialty";
pub const EXPEDITION_SUPPLIES_MASTERWORK_RECIPE_ID: &str = "expedition_supplies_masterwork";

/// Every recipe-bearing study in the six subsistence/frontier families. The two
/// starter Hunting variants retain their established stable runtime ids.
pub const SUBSISTENCE_FRONTIER_RECIPE_IDS: &[&str] = &[
    BONE_TRINKET_RECIPE_ID,
    BONE_TOY_RECIPE_ID,
    HUNTING_QUALITY_RECIPE_ID,
    HUNTING_SPECIALTY_RECIPE_ID,
    HUNTING_MASTERWORK_RECIPE_ID,
    FORAGING_PREPARATION_RECIPE_ID,
    FORAGING_STAPLES_RECIPE_ID,
    FORAGING_QUALITY_RECIPE_ID,
    FORAGING_SPECIALTY_RECIPE_ID,
    FORAGING_MASTERWORK_RECIPE_ID,
    WATERWORKS_PREPARATION_RECIPE_ID,
    WATERWORKS_STAPLES_RECIPE_ID,
    WATERWORKS_QUALITY_RECIPE_ID,
    WATERWORKS_SPECIALTY_RECIPE_ID,
    WATERWORKS_MASTERWORK_RECIPE_ID,
    ANIMAL_HUSBANDRY_PREPARATION_RECIPE_ID,
    ANIMAL_HUSBANDRY_STAPLES_RECIPE_ID,
    ANIMAL_HUSBANDRY_QUALITY_RECIPE_ID,
    ANIMAL_HUSBANDRY_SPECIALTY_RECIPE_ID,
    ANIMAL_HUSBANDRY_MASTERWORK_RECIPE_ID,
    FIELD_CRAFT_PREPARATION_RECIPE_ID,
    FIELD_CRAFT_STAPLES_RECIPE_ID,
    FIELD_CRAFT_QUALITY_RECIPE_ID,
    FIELD_CRAFT_SPECIALTY_RECIPE_ID,
    FIELD_CRAFT_MASTERWORK_RECIPE_ID,
    EXPEDITION_SUPPLIES_PREPARATION_RECIPE_ID,
    EXPEDITION_SUPPLIES_STAPLES_RECIPE_ID,
    EXPEDITION_SUPPLIES_QUALITY_RECIPE_ID,
    EXPEDITION_SUPPLIES_SPECIALTY_RECIPE_ID,
    EXPEDITION_SUPPLIES_MASTERWORK_RECIPE_ID,
];
pub const TEXTILE_WORK_PREPARATION_RECIPE_ID: &str = "textile_work_preparation";
pub const TEXTILE_WORK_STAPLES_RECIPE_ID: &str = "textile_work_staples";
pub const TEXTILE_WORK_QUALITY_RECIPE_ID: &str = "textile_work_quality";
pub const TEXTILE_WORK_SPECIALTY_RECIPE_ID: &str = "textile_work_specialty";
pub const TEXTILE_WORK_MASTERWORK_RECIPE_ID: &str = "textile_work_masterwork";
pub const LEATHERWORKING_PREPARATION_RECIPE_ID: &str = "leatherworking_preparation";
pub const LEATHERWORKING_STAPLES_RECIPE_ID: &str = "leatherworking_staples";
pub const LEATHERWORKING_QUALITY_RECIPE_ID: &str = "leatherworking_quality";
pub const LEATHERWORKING_SPECIALTY_RECIPE_ID: &str = "leatherworking_specialty";
pub const LEATHERWORKING_MASTERWORK_RECIPE_ID: &str = "leatherworking_masterwork";
pub const CARPENTRY_QUALITY_RECIPE_ID: &str = "carpentry_quality";
pub const CARPENTRY_SPECIALTY_RECIPE_ID: &str = "carpentry_specialty";
pub const CARPENTRY_MASTERWORK_RECIPE_ID: &str = "carpentry_masterwork";
pub const STONECRAFT_MASTERWORK_RECIPE_ID: &str = "stonecraft_masterwork";
pub const METALLURGY_STAPLES_RECIPE_ID: &str = "metallurgy_staples";
pub const METALLURGY_QUALITY_RECIPE_ID: &str = "metallurgy_quality";
pub const METALLURGY_SPECIALTY_RECIPE_ID: &str = "metallurgy_specialty";
pub const METALLURGY_MASTERWORK_RECIPE_ID: &str = "metallurgy_masterwork";
pub const TOOLMAKING_SPECIALTY_RECIPE_ID: &str = "toolmaking_specialty";
pub const TOOLMAKING_MASTERWORK_RECIPE_ID: &str = "toolmaking_masterwork";
pub const WEAPONCRAFT_PREPARATION_RECIPE_ID: &str = "weaponcraft_preparation";
pub const WEAPONCRAFT_STAPLES_RECIPE_ID: &str = "weaponcraft_staples";
pub const WEAPONCRAFT_QUALITY_RECIPE_ID: &str = "weaponcraft_quality";
pub const WEAPONCRAFT_SPECIALTY_RECIPE_ID: &str = "weaponcraft_specialty";
pub const WEAPONCRAFT_MASTERWORK_RECIPE_ID: &str = "weaponcraft_masterwork";
pub const ARMORCRAFT_PREPARATION_RECIPE_ID: &str = "armorcraft_preparation";
pub const ARMORCRAFT_STAPLES_RECIPE_ID: &str = "armorcraft_staples";
pub const ARMORCRAFT_QUALITY_RECIPE_ID: &str = "armorcraft_quality";
pub const ARMORCRAFT_SPECIALTY_RECIPE_ID: &str = "armorcraft_specialty";
pub const ARMORCRAFT_MASTERWORK_RECIPE_ID: &str = "armorcraft_masterwork";

/// Every newly activated recipe-bearing study in the industrial material
/// families. Trade Goods retains its established exact material-variant routes.
pub const INDUSTRIAL_MATERIAL_RECIPE_IDS: &[&str] = &[
    TEXTILE_WORK_PREPARATION_RECIPE_ID,
    TEXTILE_WORK_STAPLES_RECIPE_ID,
    TEXTILE_WORK_QUALITY_RECIPE_ID,
    TEXTILE_WORK_SPECIALTY_RECIPE_ID,
    TEXTILE_WORK_MASTERWORK_RECIPE_ID,
    LEATHERWORKING_PREPARATION_RECIPE_ID,
    LEATHERWORKING_STAPLES_RECIPE_ID,
    LEATHERWORKING_QUALITY_RECIPE_ID,
    LEATHERWORKING_SPECIALTY_RECIPE_ID,
    LEATHERWORKING_MASTERWORK_RECIPE_ID,
    CARPENTRY_QUALITY_RECIPE_ID,
    CARPENTRY_SPECIALTY_RECIPE_ID,
    CARPENTRY_MASTERWORK_RECIPE_ID,
    STONECRAFT_MASTERWORK_RECIPE_ID,
    METALLURGY_STAPLES_RECIPE_ID,
    METALLURGY_QUALITY_RECIPE_ID,
    METALLURGY_SPECIALTY_RECIPE_ID,
    METALLURGY_MASTERWORK_RECIPE_ID,
    TOOLMAKING_SPECIALTY_RECIPE_ID,
    TOOLMAKING_MASTERWORK_RECIPE_ID,
    WEAPONCRAFT_PREPARATION_RECIPE_ID,
    WEAPONCRAFT_STAPLES_RECIPE_ID,
    WEAPONCRAFT_QUALITY_RECIPE_ID,
    WEAPONCRAFT_SPECIALTY_RECIPE_ID,
    WEAPONCRAFT_MASTERWORK_RECIPE_ID,
    ARMORCRAFT_PREPARATION_RECIPE_ID,
    ARMORCRAFT_STAPLES_RECIPE_ID,
    ARMORCRAFT_QUALITY_RECIPE_ID,
    ARMORCRAFT_SPECIALTY_RECIPE_ID,
    ARMORCRAFT_MASTERWORK_RECIPE_ID,
];

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

macro_rules! scalar_recipe {
    ($id:ident, $building:ident, $input:ident, $output:ident) => {
        StationRecipeDescriptor {
            id: $id,
            building_type: BuildingType::$building,
            input_resources: &[ResourceKind::$input],
            output_resources: &[ResourceKind::$output],
            output_item: None,
            founding_available: false,
        }
    };
}

macro_rules! item_recipe {
    ($id:ident, $building:ident, $input:ident, $kind:ident, $material:ident, $quality:expr) => {
        StationRecipeDescriptor {
            id: $id,
            building_type: BuildingType::$building,
            input_resources: &[ResourceKind::$input],
            output_resources: &[],
            output_item: Some(StationItemOutput {
                kind: ItemKind::$kind,
                material: Material::$material,
                quality: $quality,
            }),
            founding_available: false,
        }
    };
}

macro_rules! equipment_recipe {
    ($id:ident, $kind:ident, $quality:literal) => {
        StationRecipeDescriptor {
            id: $id,
            building_type: BuildingType::Smithy,
            input_resources: &[ResourceKind::Metal],
            output_resources: &[],
            output_item: Some(StationItemOutput {
                kind: ItemKind::$kind,
                material: Material::Metal,
                quality: $quality,
            }),
            founding_available: false,
        }
    };
}

const MILL_INPUTS: &[ResourceKind] = &[
    ResourceKind::Grain,
    ResourceKind::Flour,
    ResourceKind::Food,
    ResourceKind::Catnip,
    ResourceKind::Herbs,
];
const MILL_OUTPUTS: &[ResourceKind] = &[
    ResourceKind::Food,
    ResourceKind::Flour,
    ResourceKind::Preserves,
    ResourceKind::Brew,
];
const SAWMILL_INPUTS: &[ResourceKind] = &[ResourceKind::Logs];
const SAWMILL_OUTPUTS: &[ResourceKind] = &[ResourceKind::Lumber];
const WORKSHOP_INPUTS: &[ResourceKind] = &[
    ResourceKind::Materials,
    ResourceKind::Gem,
    ResourceKind::Sand,
    ResourceKind::Herbs,
    ResourceKind::Cloth,
];
const WORKSHOP_OUTPUTS: &[ResourceKind] = &[ResourceKind::Refined, ResourceKind::Medicine];
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
const CLOTHIER_INPUTS: &[ResourceKind] = &[
    ResourceKind::Fibre,
    ResourceKind::Thread,
    ResourceKind::Cloth,
];
const CLOTHIER_OUTPUTS: &[ResourceKind] = &[ResourceKind::Thread, ResourceKind::Cloth];
const TANNERY_INPUTS: &[ResourceKind] = &[ResourceKind::Hide, ResourceKind::Leather];
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
    scalar_recipe!(FINE_GRAIN_FLOUR_RECIPE_ID, Mill, Grain, Flour),
    scalar_recipe!(STONEGROUND_FLOUR_RECIPE_ID, Mill, Grain, Flour),
    scalar_recipe!(MASTERWORK_FLOUR_RECIPE_ID, Mill, Grain, Flour),
    scalar_recipe!(BAKE_FLATBREAD_RECIPE_ID, Mill, Flour, Food),
    scalar_recipe!(BAKE_LOAF_RECIPE_ID, Mill, Flour, Food),
    scalar_recipe!(BAKE_BISCUITS_RECIPE_ID, Mill, Flour, Food),
    scalar_recipe!(BAKE_FESTIVAL_CAKE_RECIPE_ID, Mill, Flour, Food),
    scalar_recipe!(BAKE_MASTERWORK_PASTRY_RECIPE_ID, Mill, Flour, Food),
    scalar_recipe!(DRY_FOOD_RECIPE_ID, Mill, Food, Preserves),
    scalar_recipe!(SMOKE_FOOD_RECIPE_ID, Mill, Food, Preserves),
    scalar_recipe!(PICKLE_FOOD_RECIPE_ID, Mill, Food, Preserves),
    scalar_recipe!(PRESERVE_RATIONS_RECIPE_ID, Mill, Food, Preserves),
    scalar_recipe!(PRESERVE_MASTERWORK_FEAST_RECIPE_ID, Mill, Food, Preserves),
    scalar_recipe!(BREW_GRAIN_SMALL_RECIPE_ID, Mill, Grain, Brew),
    scalar_recipe!(BREW_CATNIP_ALE_RECIPE_ID, Mill, Catnip, Brew),
    scalar_recipe!(BREW_HERBAL_TONIC_RECIPE_ID, Mill, Herbs, Brew),
    scalar_recipe!(BREW_SPICED_ALE_RECIPE_ID, Mill, Catnip, Brew),
    scalar_recipe!(BREW_MASTERWORK_RECIPE_ID, Mill, Herbs, Brew),
];
const SAWMILL_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: SAWMILL_RECIPE_ID,
        building_type: BuildingType::Sawmill,
        input_resources: SAWMILL_INPUTS,
        output_resources: SAWMILL_OUTPUTS,
        output_item: None,
        founding_available: false,
    },
    scalar_recipe!(CARPENTRY_QUALITY_RECIPE_ID, Sawmill, Logs, Lumber),
    scalar_recipe!(CARPENTRY_MASTERWORK_RECIPE_ID, Sawmill, Logs, Lumber),
];
const WORKSHOP_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: WORKSHOP_RECIPE_ID,
        building_type: BuildingType::Workshop,
        input_resources: &[ResourceKind::Materials],
        output_resources: &[ResourceKind::Refined],
        output_item: None,
        founding_available: false,
    },
    scalar_recipe!(HERBAL_POULTICE_RECIPE_ID, Workshop, Herbs, Medicine),
    scalar_recipe!(HERBAL_TONIC_RECIPE_ID, Workshop, Herbs, Medicine),
    scalar_recipe!(HERBAL_SALVE_RECIPE_ID, Workshop, Herbs, Medicine),
    scalar_recipe!(HERBAL_REMEDY_RECIPE_ID, Workshop, Herbs, Medicine),
    scalar_recipe!(
        HERBAL_MASTERWORK_REMEDY_RECIPE_ID,
        Workshop,
        Herbs,
        Medicine
    ),
    item_recipe!(
        FIELD_CRAFT_PREPARATION_RECIPE_ID,
        Workshop,
        Materials,
        Tool,
        Wood,
        1
    ),
    item_recipe!(
        FIELD_CRAFT_STAPLES_RECIPE_ID,
        Workshop,
        Materials,
        Clothing,
        Wood,
        1
    ),
    item_recipe!(
        FIELD_CRAFT_QUALITY_RECIPE_ID,
        Workshop,
        Materials,
        Armor,
        Wood,
        1
    ),
    item_recipe!(
        FIELD_CRAFT_SPECIALTY_RECIPE_ID,
        Workshop,
        Materials,
        Furniture,
        Wood,
        1
    ),
    item_recipe!(
        FIELD_CRAFT_MASTERWORK_RECIPE_ID,
        Workshop,
        Materials,
        Toy,
        Wood,
        2
    ),
    item_recipe!(
        EXPEDITION_SUPPLIES_PREPARATION_RECIPE_ID,
        Workshop,
        Cloth,
        Bowl,
        Fibre,
        2
    ),
    item_recipe!(
        EXPEDITION_SUPPLIES_STAPLES_RECIPE_ID,
        Workshop,
        Cloth,
        Clothing,
        Fibre,
        2
    ),
    item_recipe!(
        EXPEDITION_SUPPLIES_QUALITY_RECIPE_ID,
        Workshop,
        Cloth,
        Tool,
        Fibre,
        2
    ),
    item_recipe!(
        EXPEDITION_SUPPLIES_SPECIALTY_RECIPE_ID,
        Workshop,
        Cloth,
        Armor,
        Fibre,
        2
    ),
    item_recipe!(
        EXPEDITION_SUPPLIES_MASTERWORK_RECIPE_ID,
        Workshop,
        Cloth,
        Furniture,
        Fibre,
        3
    ),
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
const SMELTER_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: SMELTER_RECIPE_ID,
        building_type: BuildingType::Smelter,
        input_resources: SMELTER_INPUTS,
        output_resources: SMELTER_OUTPUTS,
        output_item: None,
        founding_available: false,
    },
    scalar_recipe!(METALLURGY_STAPLES_RECIPE_ID, Smelter, Ore, Metal),
    scalar_recipe!(METALLURGY_QUALITY_RECIPE_ID, Smelter, Ore, Metal),
    scalar_recipe!(METALLURGY_SPECIALTY_RECIPE_ID, Smelter, Ore, Metal),
    scalar_recipe!(METALLURGY_MASTERWORK_RECIPE_ID, Smelter, Ore, Metal),
];
const WOOD_CUTTER_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: LOGS_TO_PLANKS_RECIPE_ID,
        building_type: BuildingType::WoodCutter,
        input_resources: WOOD_CUTTER_INPUTS,
        output_resources: WOOD_CUTTER_OUTPUTS,
        output_item: None,
        founding_available: true,
    },
    scalar_recipe!(CARPENTRY_SPECIALTY_RECIPE_ID, WoodCutter, Logs, Planks),
];
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
    item_recipe!(BONE_MUG_RECIPE_ID, StonePrep, Bone, Mug, Bone, 1),
    item_recipe!(STONE_MUG_RECIPE_ID, StonePrep, Stone, Mug, Stone, 1),
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
    scalar_recipe!(STONECRAFT_MASTERWORK_RECIPE_ID, StonePrep, Stone, Blocks),
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
    item_recipe!(
        HUNTING_QUALITY_RECIPE_ID,
        Woodworking,
        Bone,
        Weapon,
        Bone,
        1
    ),
    item_recipe!(
        HUNTING_SPECIALTY_RECIPE_ID,
        Woodworking,
        Bone,
        Armor,
        Bone,
        1
    ),
    item_recipe!(
        HUNTING_MASTERWORK_RECIPE_ID,
        Woodworking,
        Bone,
        Tool,
        Bone,
        2
    ),
    item_recipe!(
        WATERWORKS_PREPARATION_RECIPE_ID,
        Woodworking,
        Planks,
        Bowl,
        Wood,
        1
    ),
    item_recipe!(
        WATERWORKS_STAPLES_RECIPE_ID,
        Woodworking,
        Planks,
        Mug,
        Wood,
        1
    ),
    item_recipe!(
        WATERWORKS_QUALITY_RECIPE_ID,
        Woodworking,
        Planks,
        Toy,
        Wood,
        1
    ),
    item_recipe!(
        WATERWORKS_SPECIALTY_RECIPE_ID,
        Woodworking,
        Planks,
        Furniture,
        Wood,
        1
    ),
    item_recipe!(
        WATERWORKS_MASTERWORK_RECIPE_ID,
        Woodworking,
        Planks,
        Bowl,
        Wood,
        2
    ),
];
const CLOTHIER_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: FIBRE_TO_THREAD_RECIPE_ID,
        building_type: BuildingType::Clothier,
        input_resources: &[ResourceKind::Fibre],
        output_resources: &[ResourceKind::Thread],
        output_item: None,
        founding_available: false,
    },
    StationRecipeDescriptor {
        id: FIBRE_TO_CLOTH_RECIPE_ID,
        building_type: BuildingType::Clothier,
        input_resources: &[ResourceKind::Thread],
        output_resources: &[ResourceKind::Cloth],
        output_item: None,
        founding_available: false,
    },
    item_recipe!(
        FORAGING_PREPARATION_RECIPE_ID,
        Clothier,
        Fibre,
        Bowl,
        Fibre,
        1
    ),
    item_recipe!(
        FORAGING_STAPLES_RECIPE_ID,
        Clothier,
        Cloth,
        Clothing,
        Fibre,
        1
    ),
    item_recipe!(FORAGING_QUALITY_RECIPE_ID, Clothier, Fibre, Tool, Fibre, 1),
    item_recipe!(FORAGING_SPECIALTY_RECIPE_ID, Clothier, Fibre, Toy, Fibre, 2),
    item_recipe!(
        FORAGING_MASTERWORK_RECIPE_ID,
        Clothier,
        Fibre,
        Furniture,
        Fibre,
        2
    ),
    scalar_recipe!(TEXTILE_WORK_PREPARATION_RECIPE_ID, Clothier, Thread, Cloth),
    scalar_recipe!(TEXTILE_WORK_STAPLES_RECIPE_ID, Clothier, Thread, Cloth),
    scalar_recipe!(TEXTILE_WORK_QUALITY_RECIPE_ID, Clothier, Thread, Cloth),
    scalar_recipe!(TEXTILE_WORK_SPECIALTY_RECIPE_ID, Clothier, Thread, Cloth),
    scalar_recipe!(TEXTILE_WORK_MASTERWORK_RECIPE_ID, Clothier, Thread, Cloth),
];
const TANNERY_RECIPES: &[StationRecipeDescriptor] = &[
    StationRecipeDescriptor {
        id: HIDE_TO_LEATHER_RECIPE_ID,
        building_type: BuildingType::Tannery,
        input_resources: &[ResourceKind::Hide],
        output_resources: TANNERY_OUTPUTS,
        output_item: None,
        founding_available: false,
    },
    item_recipe!(
        ANIMAL_HUSBANDRY_PREPARATION_RECIPE_ID,
        Tannery,
        Leather,
        Clothing,
        Leather,
        1
    ),
    item_recipe!(
        ANIMAL_HUSBANDRY_STAPLES_RECIPE_ID,
        Tannery,
        Hide,
        Toy,
        Leather,
        1
    ),
    item_recipe!(
        ANIMAL_HUSBANDRY_QUALITY_RECIPE_ID,
        Tannery,
        Leather,
        Armor,
        Leather,
        1
    ),
    item_recipe!(
        ANIMAL_HUSBANDRY_SPECIALTY_RECIPE_ID,
        Tannery,
        Hide,
        Tool,
        Leather,
        1
    ),
    item_recipe!(
        ANIMAL_HUSBANDRY_MASTERWORK_RECIPE_ID,
        Tannery,
        Hide,
        Furniture,
        Leather,
        2
    ),
    scalar_recipe!(LEATHERWORKING_PREPARATION_RECIPE_ID, Tannery, Hide, Leather),
    scalar_recipe!(LEATHERWORKING_STAPLES_RECIPE_ID, Tannery, Hide, Leather),
    scalar_recipe!(LEATHERWORKING_QUALITY_RECIPE_ID, Tannery, Hide, Leather),
    scalar_recipe!(LEATHERWORKING_SPECIALTY_RECIPE_ID, Tannery, Hide, Leather),
    scalar_recipe!(LEATHERWORKING_MASTERWORK_RECIPE_ID, Tannery, Hide, Leather),
];
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
    item_recipe!(METAL_MUG_RECIPE_ID, Smithy, Metal, Mug, Metal, 1),
    equipment_recipe!(TOOLMAKING_SPECIALTY_RECIPE_ID, Tool, 3),
    equipment_recipe!(TOOLMAKING_MASTERWORK_RECIPE_ID, Tool, 4),
    equipment_recipe!(WEAPONCRAFT_PREPARATION_RECIPE_ID, Weapon, 0),
    equipment_recipe!(WEAPONCRAFT_STAPLES_RECIPE_ID, Weapon, 1),
    equipment_recipe!(WEAPONCRAFT_QUALITY_RECIPE_ID, Weapon, 2),
    equipment_recipe!(WEAPONCRAFT_SPECIALTY_RECIPE_ID, Weapon, 3),
    equipment_recipe!(WEAPONCRAFT_MASTERWORK_RECIPE_ID, Weapon, 4),
    equipment_recipe!(ARMORCRAFT_PREPARATION_RECIPE_ID, Armor, 0),
    equipment_recipe!(ARMORCRAFT_STAPLES_RECIPE_ID, Armor, 1),
    equipment_recipe!(ARMORCRAFT_QUALITY_RECIPE_ID, Armor, 2),
    equipment_recipe!(ARMORCRAFT_SPECIALTY_RECIPE_ID, Armor, 3),
    equipment_recipe!(ARMORCRAFT_MASTERWORK_RECIPE_ID, Armor, 4),
];

/// Whether a catalog recipe payload names a physical runtime recipe.
#[must_use]
pub fn is_runtime_recipe_id(recipe_id: &str) -> bool {
    BuildingType::ALL.iter().copied().any(|building_type| {
        station_recipe_set(building_type)
            .is_some_and(|station| station.recipes.iter().any(|recipe| recipe.id == recipe_id))
    })
}

/// Resolve any maintained physical recipe by its stable queue id.
#[must_use]
pub fn station_recipe(recipe_id: &str) -> Option<&'static StationRecipeDescriptor> {
    BuildingType::ALL
        .iter()
        .copied()
        .filter_map(station_recipe_set)
        .flat_map(|station| station.recipes)
        .find(|recipe| recipe.id == recipe_id)
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
                &[LOGS_TO_PLANKS_RECIPE_ID, CARPENTRY_SPECIALTY_RECIPE_ID][..],
                &[ResourceKind::Logs][..],
                &[ResourceKind::Planks][..],
            ),
            (
                BuildingType::StonePrep,
                &[
                    STONE_TO_BLOCKS_RECIPE_ID,
                    BONE_TRINKET_RECIPE_ID,
                    BONE_TOY_RECIPE_ID,
                    BONE_MUG_RECIPE_ID,
                    STONE_MUG_RECIPE_ID,
                    CLAY_MUG_RECIPE_ID,
                    CLAY_BOWL_RECIPE_ID,
                    CLAY_BRICK_RECIPE_ID,
                    STONECRAFT_MASTERWORK_RECIPE_ID,
                ][..],
                &[ResourceKind::Stone, ResourceKind::Bone, ResourceKind::Clay][..],
                &[ResourceKind::Blocks][..],
            ),
            (
                BuildingType::Woodworking,
                &[
                    PLANKS_AND_BLOCKS_TO_TOOLS_RECIPE_ID,
                    BONE_TOOL_RECIPE_ID,
                    HUNTING_QUALITY_RECIPE_ID,
                    HUNTING_SPECIALTY_RECIPE_ID,
                    HUNTING_MASTERWORK_RECIPE_ID,
                    WATERWORKS_PREPARATION_RECIPE_ID,
                    WATERWORKS_STAPLES_RECIPE_ID,
                    WATERWORKS_QUALITY_RECIPE_ID,
                    WATERWORKS_SPECIALTY_RECIPE_ID,
                    WATERWORKS_MASTERWORK_RECIPE_ID,
                ][..],
                &[
                    ResourceKind::Planks,
                    ResourceKind::Blocks,
                    ResourceKind::Bone,
                ][..],
                &[ResourceKind::Tools][..],
            ),
            (
                BuildingType::Clothier,
                &[
                    FIBRE_TO_THREAD_RECIPE_ID,
                    FIBRE_TO_CLOTH_RECIPE_ID,
                    FORAGING_PREPARATION_RECIPE_ID,
                    FORAGING_STAPLES_RECIPE_ID,
                    FORAGING_QUALITY_RECIPE_ID,
                    FORAGING_SPECIALTY_RECIPE_ID,
                    FORAGING_MASTERWORK_RECIPE_ID,
                    TEXTILE_WORK_PREPARATION_RECIPE_ID,
                    TEXTILE_WORK_STAPLES_RECIPE_ID,
                    TEXTILE_WORK_QUALITY_RECIPE_ID,
                    TEXTILE_WORK_SPECIALTY_RECIPE_ID,
                    TEXTILE_WORK_MASTERWORK_RECIPE_ID,
                ][..],
                &[
                    ResourceKind::Fibre,
                    ResourceKind::Thread,
                    ResourceKind::Cloth,
                ][..],
                &[ResourceKind::Thread, ResourceKind::Cloth][..],
            ),
            (
                BuildingType::Tannery,
                &[
                    HIDE_TO_LEATHER_RECIPE_ID,
                    ANIMAL_HUSBANDRY_PREPARATION_RECIPE_ID,
                    ANIMAL_HUSBANDRY_STAPLES_RECIPE_ID,
                    ANIMAL_HUSBANDRY_QUALITY_RECIPE_ID,
                    ANIMAL_HUSBANDRY_SPECIALTY_RECIPE_ID,
                    ANIMAL_HUSBANDRY_MASTERWORK_RECIPE_ID,
                    LEATHERWORKING_PREPARATION_RECIPE_ID,
                    LEATHERWORKING_STAPLES_RECIPE_ID,
                    LEATHERWORKING_QUALITY_RECIPE_ID,
                    LEATHERWORKING_SPECIALTY_RECIPE_ID,
                    LEATHERWORKING_MASTERWORK_RECIPE_ID,
                ][..],
                &[ResourceKind::Hide, ResourceKind::Leather][..],
                &[ResourceKind::Leather][..],
            ),
            (
                BuildingType::Smithy,
                &[
                    SMITHY_WEAPON_RECIPE_ID,
                    SMITHY_TOOL_RECIPE_ID,
                    SMITHY_ARMOR_RECIPE_ID,
                    METAL_MUG_RECIPE_ID,
                    TOOLMAKING_SPECIALTY_RECIPE_ID,
                    TOOLMAKING_MASTERWORK_RECIPE_ID,
                    WEAPONCRAFT_PREPARATION_RECIPE_ID,
                    WEAPONCRAFT_STAPLES_RECIPE_ID,
                    WEAPONCRAFT_QUALITY_RECIPE_ID,
                    WEAPONCRAFT_SPECIALTY_RECIPE_ID,
                    WEAPONCRAFT_MASTERWORK_RECIPE_ID,
                    ARMORCRAFT_PREPARATION_RECIPE_ID,
                    ARMORCRAFT_STAPLES_RECIPE_ID,
                    ARMORCRAFT_QUALITY_RECIPE_ID,
                    ARMORCRAFT_SPECIALTY_RECIPE_ID,
                    ARMORCRAFT_MASTERWORK_RECIPE_ID,
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
        assert_eq!(ids.len(), 108);
        assert!(station_recipe_set(BuildingType::Den).is_none());
    }

    #[test]
    fn all_new_industrial_recipes_have_one_finite_physical_authority() {
        let ids = [
            TEXTILE_WORK_PREPARATION_RECIPE_ID,
            TEXTILE_WORK_STAPLES_RECIPE_ID,
            TEXTILE_WORK_QUALITY_RECIPE_ID,
            TEXTILE_WORK_SPECIALTY_RECIPE_ID,
            TEXTILE_WORK_MASTERWORK_RECIPE_ID,
            LEATHERWORKING_PREPARATION_RECIPE_ID,
            LEATHERWORKING_STAPLES_RECIPE_ID,
            LEATHERWORKING_QUALITY_RECIPE_ID,
            LEATHERWORKING_SPECIALTY_RECIPE_ID,
            LEATHERWORKING_MASTERWORK_RECIPE_ID,
            CARPENTRY_QUALITY_RECIPE_ID,
            CARPENTRY_SPECIALTY_RECIPE_ID,
            CARPENTRY_MASTERWORK_RECIPE_ID,
            STONECRAFT_MASTERWORK_RECIPE_ID,
            METALLURGY_STAPLES_RECIPE_ID,
            METALLURGY_QUALITY_RECIPE_ID,
            METALLURGY_SPECIALTY_RECIPE_ID,
            METALLURGY_MASTERWORK_RECIPE_ID,
            TOOLMAKING_SPECIALTY_RECIPE_ID,
            TOOLMAKING_MASTERWORK_RECIPE_ID,
            WEAPONCRAFT_PREPARATION_RECIPE_ID,
            WEAPONCRAFT_STAPLES_RECIPE_ID,
            WEAPONCRAFT_QUALITY_RECIPE_ID,
            WEAPONCRAFT_SPECIALTY_RECIPE_ID,
            WEAPONCRAFT_MASTERWORK_RECIPE_ID,
            ARMORCRAFT_PREPARATION_RECIPE_ID,
            ARMORCRAFT_STAPLES_RECIPE_ID,
            ARMORCRAFT_QUALITY_RECIPE_ID,
            ARMORCRAFT_SPECIALTY_RECIPE_ID,
            ARMORCRAFT_MASTERWORK_RECIPE_ID,
        ];
        for id in ids {
            let recipe = station_recipe(id).unwrap_or_else(|| panic!("missing recipe {id}"));
            assert_eq!(recipe.input_resources.len(), 1, "{id}");
            assert_eq!(
                usize::from(!recipe.output_resources.is_empty())
                    + usize::from(recipe.output_item.is_some()),
                1,
                "{id} must have exactly one scalar or exact-item output authority"
            );
            assert!(!recipe.founding_available, "{id}");
        }

        for (kind, recipes) in [
            (
                ItemKind::Weapon,
                [
                    WEAPONCRAFT_PREPARATION_RECIPE_ID,
                    WEAPONCRAFT_STAPLES_RECIPE_ID,
                    WEAPONCRAFT_QUALITY_RECIPE_ID,
                    WEAPONCRAFT_SPECIALTY_RECIPE_ID,
                    WEAPONCRAFT_MASTERWORK_RECIPE_ID,
                ],
            ),
            (
                ItemKind::Armor,
                [
                    ARMORCRAFT_PREPARATION_RECIPE_ID,
                    ARMORCRAFT_STAPLES_RECIPE_ID,
                    ARMORCRAFT_QUALITY_RECIPE_ID,
                    ARMORCRAFT_SPECIALTY_RECIPE_ID,
                    ARMORCRAFT_MASTERWORK_RECIPE_ID,
                ],
            ),
        ] {
            for (quality, id) in recipes.into_iter().enumerate() {
                let recipe = station_recipe(id).unwrap();
                let output = recipe.output_item.expect("exact equipment output");
                assert_eq!(recipe.output_resources, &[], "{id} shadow scalar");
                assert_eq!(output.kind, kind, "{id}");
                assert_eq!(output.material, Material::Metal, "{id}");
                assert_eq!(output.quality, quality as u8, "{id}");
            }
        }
        for (id, quality) in [
            (TOOLMAKING_SPECIALTY_RECIPE_ID, 3),
            (TOOLMAKING_MASTERWORK_RECIPE_ID, 4),
        ] {
            let recipe = station_recipe(id).unwrap();
            let output = recipe.output_item.expect("exact tool output");
            assert_eq!(recipe.output_resources, &[], "{id} shadow scalar");
            assert_eq!(output.kind, ItemKind::Tool);
            assert_eq!(output.material, Material::Metal);
            assert_eq!(output.quality, quality);
        }
    }

    #[test]
    fn sourced_breadth_uses_explicit_mill_steps_and_metal_tools() {
        let mill = station_recipe_set(BuildingType::Mill).unwrap();
        assert_eq!(mill.recipes.len(), 20);
        assert_eq!(mill.recipes[0].id, GRAIN_TO_FLOUR_RECIPE_ID);
        assert_eq!(mill.recipes[1].id, FLOUR_TO_FOOD_RECIPE_ID);
        assert_eq!(mill.recipes.last().unwrap().id, BREW_MASTERWORK_RECIPE_ID);
        assert_eq!(mill.recipes[0].input_resources, &[ResourceKind::Grain]);
        assert_eq!(mill.recipes[0].output_resources, &[ResourceKind::Flour]);
        assert_eq!(mill.recipes[1].input_resources, &[ResourceKind::Flour]);
        assert_eq!(mill.recipes[1].output_resources, &[ResourceKind::Food]);
        assert!(mill.recipes.iter().any(|recipe| {
            recipe.input_resources == [ResourceKind::Herbs]
                && recipe.output_resources == [ResourceKind::Brew]
        }));

        let smithy = station_recipe_set(BuildingType::Smithy).unwrap();
        assert_eq!(smithy.recipes.len(), 16);
        assert_eq!(smithy.recipes[0].id, SMITHY_WEAPON_RECIPE_ID);
        assert_eq!(smithy.recipes[1].id, SMITHY_TOOL_RECIPE_ID);
        assert_eq!(smithy.recipes[2].id, SMITHY_ARMOR_RECIPE_ID);
        assert_eq!(smithy.recipes[15].id, ARMORCRAFT_MASTERWORK_RECIPE_ID);
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
    fn canonical_textile_intermediates_and_four_mug_materials_are_physical() {
        let spin = station_recipe(FIBRE_TO_THREAD_RECIPE_ID).expect("fibre spinning route");
        assert_eq!(spin.building_type, BuildingType::Clothier);
        assert_eq!(spin.input_resources, &[ResourceKind::Fibre]);
        assert_eq!(spin.output_resources, &[ResourceKind::Thread]);
        assert!(spin.output_item.is_none());

        let weave = station_recipe(FIBRE_TO_CLOTH_RECIPE_ID).expect("thread weaving route");
        assert_eq!(weave.input_resources, &[ResourceKind::Thread]);
        assert_eq!(weave.output_resources, &[ResourceKind::Cloth]);

        for (id, station, input, material) in [
            (
                WATERWORKS_STAPLES_RECIPE_ID,
                BuildingType::Woodworking,
                ResourceKind::Planks,
                Material::Wood,
            ),
            (
                STONE_MUG_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Stone,
                Material::Stone,
            ),
            (
                METAL_MUG_RECIPE_ID,
                BuildingType::Smithy,
                ResourceKind::Metal,
                Material::Metal,
            ),
            (
                BONE_MUG_RECIPE_ID,
                BuildingType::StonePrep,
                ResourceKind::Bone,
                Material::Bone,
            ),
        ] {
            let recipe = station_item_recipe(id).unwrap_or_else(|| panic!("missing {id}"));
            assert_eq!(recipe.building_type, station, "{id}");
            assert_eq!(recipe.input_resources, &[input], "{id}");
            let output = recipe.output_item.expect("exact mug");
            assert_eq!((output.kind, output.material), (ItemKind::Mug, material));
        }

        for (id, input) in [
            (FORAGING_STAPLES_RECIPE_ID, ResourceKind::Cloth),
            (
                ANIMAL_HUSBANDRY_PREPARATION_RECIPE_ID,
                ResourceKind::Leather,
            ),
            (ANIMAL_HUSBANDRY_QUALITY_RECIPE_ID, ResourceKind::Leather),
        ] {
            assert_eq!(
                station_item_recipe(id)
                    .expect("finished textile consumer")
                    .input_resources,
                &[input],
                "{id} must consume the maintained intermediate"
            );
        }
    }

    #[test]
    fn subsistence_frontier_recipes_all_create_exact_finite_items() {
        assert_eq!(SUBSISTENCE_FRONTIER_RECIPE_IDS.len(), 30);
        let mut outputs = std::collections::BTreeSet::new();
        for id in SUBSISTENCE_FRONTIER_RECIPE_IDS {
            let recipe = station_item_recipe(id).expect("frontier recipe is runtime-backed");
            assert_eq!(recipe.input_resources.len(), 1, "{id}");
            assert!(recipe.output_resources.is_empty(), "{id}");
            let output = recipe.output_item.expect("exact finite output");
            assert!(!recipe.founding_available, "{id}");
            outputs.insert((
                recipe.building_type.as_str(),
                output.kind,
                output.material,
                output.quality,
            ));
        }
        assert!(
            outputs.len() >= 25,
            "family recipes should not collapse into cosmetic aliases"
        );
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
