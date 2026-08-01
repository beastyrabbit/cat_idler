//! LAI.36 bounded, data-only production-content authority.
//!
//! Stable identities and catalog data live in `content_manifest.json`. This
//! module owns strict decoding, deterministic validation, canonical iteration,
//! and typed bindings to compiled behavior handlers. It deliberately does not
//! own quality, physical-lot behavior, runtime mutation, research currency, or
//! renderer file-existence checks.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::RangeInclusive,
    str::FromStr,
    sync::OnceLock,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

pub const CONTENT_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const STABLE_ID_MAX_LEN: usize = 64;
pub const EMBEDDED_MANIFEST_JSON: &str = include_str!("content_manifest.json");
pub const CONSTRUCTION_MIRACLE_INPUT_TOTAL: usize = 15;
pub const CONSTRUCTION_MIRACLE_INPUT_IDS: [&str; CONSTRUCTION_MIRACLE_INPUT_TOTAL] = [
    "fixture_research",
    "fixture_storage",
    "fixture_workshop",
    "item_bowl",
    "item_furniture",
    "item_generic_tool",
    "resource_blocks",
    "resource_cloth",
    "resource_gem",
    "resource_logs",
    "resource_lumber",
    "resource_metal",
    "resource_planks",
    "resource_refined",
    "resource_stone",
];

pub const REQUIRED_FOUNDING_CAPABILITIES: [&str; 4] = [
    "water_collection",
    "apple_gathering",
    "hand_fishing",
    "basic_food_handling",
];

pub const PLAN1_COOKHOUSE_RECIPE_IDS: [&str; 18] = [
    "baked_apples",
    "grilled_fish",
    "roasted_meat",
    "flatbread",
    "apple_porridge",
    "fish_stew",
    "meat_stew",
    "apple_preserves",
    "smoked_fish",
    "dried_meat",
    "apple_tart",
    "herb_crusted_fish",
    "meat_pie",
    "surf_and_turf",
    "travel_rations",
    "festival_cake",
    "hunters_feast",
    "grand_lair_feast",
];

pub const PLAN1_BREW_RECIPE_IDS: [&str; 5] = [
    "brew_grain_small",
    "brew_catnip_ale",
    "brew_herbal_tonic",
    "brew_spiced_ale",
    "brew_masterwork",
];

pub const PLAN1_CREATURE_IDS: [&str; 20] = [
    "cave_bat",
    "red_fox",
    "badger",
    "wild_boar",
    "gray_wolf",
    "lynx",
    "great_stag",
    "giant_serpent",
    "brown_bear",
    "great_eagle",
    "moon_stag",
    "warg",
    "cockatrice",
    "forest_troll",
    "griffin",
    "basilisk",
    "manticore",
    "chimera",
    "wyvern",
    "elder_dragon",
];

pub const PLAN1_RARE_MATERIAL_IDS: [&str; 20] = [
    "bat_wing",
    "fox_pelt",
    "badger_pelt",
    "boar_tusk",
    "wolf_pelt",
    "lynx_pelt",
    "stag_antler",
    "serpent_scale",
    "bear_pelt",
    "eagle_feather",
    "moon_antler",
    "warg_fang",
    "cockatrice_eye",
    "troll_hide",
    "griffin_plume",
    "basilisk_scale",
    "manticore_barb",
    "beast_core",
    "wyvern_membrane",
    "dragon_heart",
];

pub const HOLE_AXIS_COUNT: usize = 30;
pub const PRE_CUTOVER_RUNTIME_RECIPE_TOTAL: usize = 108;
pub const CURRENT_MILL_RECIPE_TOTAL: usize = 20;
pub const CURRENT_MILL_RECIPE_CUTOVER_TOTAL: usize = 15;
pub const RETAINED_PRE_CUTOVER_RECIPE_TOTAL: usize = 92;
pub const CURRENT_RUNTIME_RECIPE_CUTOVER_TOTAL: usize = 16;
pub const RECIPE_CUTOVER_RECEIPT_TOTAL: usize = 17;
pub const PERSISTED_COMBINED_MILL_RECIPE_ALIAS: &str = "grain_to_flour_and_food";
pub const UNCHANGED_RECIPE_FLOW_ALLOWLIST: [&str; 0] = [];
pub const PRE_CUTOVER_RUNTIME_RECIPE_IDS: [&str; PRE_CUTOVER_RUNTIME_RECIPE_TOTAL] = [
    "grain_to_flour",
    "flour_to_food",
    "fine_grain_flour",
    "stoneground_flour",
    "masterwork_flour",
    "bake_flatbread",
    "bake_loaf",
    "bake_biscuits",
    "bake_festival_cake",
    "bake_masterwork_pastry",
    "dry_food",
    "smoke_food",
    "pickle_food",
    "preserve_rations",
    "preserve_masterwork_feast",
    "brew_grain_small",
    "brew_catnip_ale",
    "brew_herbal_tonic",
    "brew_spiced_ale",
    "brew_masterwork",
    "logs_to_lumber",
    "carpentry_quality",
    "carpentry_masterwork",
    "materials_to_refined",
    "herbal_poultice",
    "herbal_tonic",
    "herbal_salve",
    "herbal_remedy",
    "herbal_masterwork_remedy",
    "field_craft_preparation",
    "field_craft_staples",
    "field_craft_quality",
    "field_craft_specialty",
    "field_craft_masterwork",
    "expedition_supplies_preparation",
    "expedition_supplies_staples",
    "expedition_supplies_quality",
    "expedition_supplies_specialty",
    "expedition_supplies_masterwork",
    "gem_jewelry",
    "sand_glass_mug",
    "sand_glass_bowl",
    "sand_glass_trinket",
    "ore_to_metal",
    "metallurgy_staples",
    "metallurgy_quality",
    "metallurgy_specialty",
    "metallurgy_masterwork",
    "logs_to_planks",
    "carpentry_specialty",
    "stone_to_blocks",
    "bone_trinket",
    "bone_toy",
    "bone_mug",
    "stone_mug",
    "clay_mug",
    "clay_bowl",
    "clay_brick",
    "stonecraft_masterwork",
    "planks_and_blocks_to_tools",
    "bone_tool",
    "hunting_quality",
    "hunting_specialty",
    "hunting_masterwork",
    "waterworks_preparation",
    "waterworks_staples",
    "waterworks_quality",
    "waterworks_specialty",
    "waterworks_masterwork",
    "fibre_to_thread",
    "fibre_to_cloth",
    "foraging_preparation",
    "foraging_staples",
    "foraging_quality",
    "foraging_specialty",
    "foraging_masterwork",
    "textile_work_preparation",
    "textile_work_staples",
    "textile_work_quality",
    "textile_work_specialty",
    "textile_work_masterwork",
    "hide_to_leather",
    "animal_husbandry_preparation",
    "animal_husbandry_staples",
    "animal_husbandry_quality",
    "animal_husbandry_specialty",
    "animal_husbandry_masterwork",
    "leatherworking_preparation",
    "leatherworking_staples",
    "leatherworking_quality",
    "leatherworking_specialty",
    "leatherworking_masterwork",
    "smithy_weapon",
    "smithy_tool",
    "smithy_armor",
    "metal_mug",
    "toolmaking_specialty",
    "toolmaking_masterwork",
    "weaponcraft_preparation",
    "weaponcraft_staples",
    "weaponcraft_quality",
    "weaponcraft_specialty",
    "weaponcraft_masterwork",
    "armorcraft_preparation",
    "armorcraft_staples",
    "armorcraft_quality",
    "armorcraft_specialty",
    "armorcraft_masterwork",
];

#[must_use]
pub fn is_valid_stable_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= STABLE_ID_MAX_LEN
        && bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

pub trait StableId: Clone + Ord + fmt::Debug + fmt::Display + FromStr<Err = StableIdError> {
    fn new(value: impl Into<String>) -> Result<Self, StableIdError>
    where
        Self: Sized;
    fn as_str(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableIdError {
    value: String,
}

impl StableIdError {
    fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl fmt::Display for StableIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "stable ID {:?} does not match [a-z][a-z0-9_]{{0,63}}",
            self.value
        )
    }
}

impl std::error::Error for StableIdError {}

macro_rules! stable_id_newtype {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, StableIdError> {
                <Self as StableId>::new(value)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                <Self as StableId>::as_str(self)
            }
        }

        impl StableId for $name {
            fn new(value: impl Into<String>) -> Result<Self, StableIdError> {
                let value = value.into();
                if is_valid_stable_id(&value) {
                    Ok(Self(value))
                } else {
                    Err(StableIdError::new(value))
                }
            }

            fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = StableIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(de::Error::custom)
            }
        }
    };
}

stable_id_newtype!(ContentId);
stable_id_newtype!(ResourceId);
stable_id_newtype!(FoodId);
stable_id_newtype!(ItemDefinitionId);
stable_id_newtype!(MaterialId);
stable_id_newtype!(CreatureId);
stable_id_newtype!(RecipeId);
stable_id_newtype!(CapabilityId);
stable_id_newtype!(ArtKey);
stable_id_newtype!(PhysicalLotId);
stable_id_newtype!(MaterialInstanceId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlot {
    Head,
    Body,
    MainHand,
    OffHand,
    Tool,
    Accessory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemClass {
    Container,
    Tool,
    Weapon,
    Armor,
    Clothing,
    Furniture,
    Tableware,
    Trinket,
    Toy,
    Brick,
    Fixture,
    Augmentation,
    ResearchInstrument,
    MaterialComponent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemFunction {
    LiquidStorage,
    FoodStorage,
    BulkStorage,
    SmallItemStorage,
    LongItemStorage,
    FishingBonus,
    ResearchPrecision,
    FightBonus,
    DefenseBonus,
    Warmth,
    Weatherproofing,
    CraftingBonus,
    Comfort,
    Eating,
    Drinking,
    Play,
    Construction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    Gathering,
    Fishing,
    Hunting,
    Cooking,
    Crafting,
    Processing,
    Construction,
    Research,
    HoleFeed,
    Storage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationBehavior {
    Mill,
    Sawmill,
    Smelter,
    WoodCutter,
    StonePrep,
    Cookhouse,
    FishingHut,
    Workshop,
    Tannery,
    Clothier,
    Woodworking,
    Smithy,
    ResearchHut,
    School,
    Hole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityDomain {
    Founding,
    Food,
    Craft,
    Hunting,
    Hole,
    Research,
    Storage,
    Construction,
    Divine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOperation {
    Unlock,
    Gather,
    Process,
    Produce,
    Craft,
    Feed,
    Augment,
    InstallFixture,
    Permit,
    Boost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentationSlot {
    Weapon,
    Armor,
    Tool,
    ResearchInstrument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureSlot {
    Cookhouse,
    FishingHut,
    Workshop,
    Research,
    Storage,
    Hole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreatureTier {
    Normal,
    Mystic,
    Boss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtLayer {
    Icon,
    Portrait,
    WorldBase,
    WorldOverlay,
    ItemMaterial,
    UiDetail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityBinding {
    ContentName,
    CreatureName,
    LairBand,
    Decorative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentOperation {
    Discover,
    Store,
    Trade,
    Process,
    Craft,
    InstallFixture,
    Augment,
    FeedHole,
}

impl ContentOperation {
    #[must_use]
    pub const fn requires_capability(self) -> bool {
        !matches!(self, Self::Discover | Self::Store | Self::Trade)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompiledHandler {
    AcquireResource,
    ConsumeFood,
    CraftItem,
    CookRecipe,
    GatherFounding,
    HoleAxis,
    HoleFeed,
    HuntCreature,
    InstallFixture,
    MillRecipe,
    ProcessMaterial,
    ResearchCapability,
    StationWork,
    UseAugmentation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledHandlerBinding {
    pub key: &'static str,
    pub handler: CompiledHandler,
}

pub const COMPILED_HANDLER_REGISTRY: &[CompiledHandlerBinding] = &[
    CompiledHandlerBinding {
        key: "acquire_resource",
        handler: CompiledHandler::AcquireResource,
    },
    CompiledHandlerBinding {
        key: "consume_food",
        handler: CompiledHandler::ConsumeFood,
    },
    CompiledHandlerBinding {
        key: "craft_item",
        handler: CompiledHandler::CraftItem,
    },
    CompiledHandlerBinding {
        key: "cook_recipe",
        handler: CompiledHandler::CookRecipe,
    },
    CompiledHandlerBinding {
        key: "gather_founding",
        handler: CompiledHandler::GatherFounding,
    },
    CompiledHandlerBinding {
        key: "hole_axis",
        handler: CompiledHandler::HoleAxis,
    },
    CompiledHandlerBinding {
        key: "hole_feed",
        handler: CompiledHandler::HoleFeed,
    },
    CompiledHandlerBinding {
        key: "hunt_creature",
        handler: CompiledHandler::HuntCreature,
    },
    CompiledHandlerBinding {
        key: "install_fixture",
        handler: CompiledHandler::InstallFixture,
    },
    CompiledHandlerBinding {
        key: "mill_recipe",
        handler: CompiledHandler::MillRecipe,
    },
    CompiledHandlerBinding {
        key: "process_material",
        handler: CompiledHandler::ProcessMaterial,
    },
    CompiledHandlerBinding {
        key: "research_capability",
        handler: CompiledHandler::ResearchCapability,
    },
    CompiledHandlerBinding {
        key: "station_work",
        handler: CompiledHandler::StationWork,
    },
    CompiledHandlerBinding {
        key: "use_augmentation",
        handler: CompiledHandler::UseAugmentation,
    },
];

#[must_use]
pub fn compiled_handler(key: &str) -> Option<CompiledHandler> {
    COMPILED_HANDLER_REGISTRY
        .iter()
        .find(|binding| binding.key == key)
        .map(|binding| binding.handler)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRequirement {
    Free,
    Required(CapabilityId),
}

impl CapabilityRequirement {
    #[must_use]
    pub fn required_id(&self) -> Option<&CapabilityId> {
        match self {
            Self::Free => None,
            Self::Required(id) => Some(id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionDescriptor {
    pub task: TaskCategory,
    pub founding_available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDescriptor {
    pub id: ResourceId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub acquisition: AcquisitionDescriptor,
    pub canonical_capability: CapabilityRequirement,
    pub processing_capability: Option<CapabilityId>,
    pub behavior_handler: String,
}

/// Physical representation required by the canonical construction-miracle
/// handoff. Bulk remains a lot; exact items and fixtures remain item identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionMiracleInputClass {
    BulkLot,
    ExactItem,
    Fixture,
    Ineligible,
}

/// Manifest spelling of the Hole's value-stage multiplier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestHoleValueStage {
    Raw,
    Processed,
    Simple,
    Prepared,
    Complex,
    Feast,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestHoleFeedPolicy {
    pub base_value_milli: u64,
    pub stage: ManifestHoleValueStage,
    pub required_darkness: u8,
}

/// Closed manifest row for every content identity appearing in a canonical
/// staged-construction bill.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructionMiracleInputDescriptor {
    pub content_id: ContentId,
    pub physical_class: ConstructionMiracleInputClass,
    pub hole_feed_policy: Option<ManifestHoleFeedPolicy>,
    pub generated_material_id: Option<MaterialId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoodDescriptor {
    pub id: FoodId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub nutrition: u16,
    pub hydration: i16,
    pub spoilage_hours: Option<u32>,
    pub weight_milli: u32,
    pub value_milli: u32,
    pub raw_safe: bool,
    pub ingredient_tags: Vec<String>,
    pub recipe_bundle: Option<CapabilityId>,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLayerDescriptor {
    pub layer_name: String,
    pub art_key: ArtKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDefinitionDescriptor {
    pub id: ItemDefinitionId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub class: ItemClass,
    pub equipment_slot: Option<EquipmentSlot>,
    pub augmentation_slot: Option<AugmentationSlot>,
    pub fixture_slot: Option<FixtureSlot>,
    pub base_materials: Vec<MaterialId>,
    pub functions: Vec<ItemFunction>,
    pub layers: Vec<ItemLayerDescriptor>,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialUseDescriptor {
    pub station: ItemDefinitionId,
    pub operation: EffectOperation,
    pub output: ContentId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDescriptor {
    pub id: MaterialId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub raw_state: ContentId,
    pub processed_state: ContentId,
    pub tags: Vec<String>,
    pub hole_darkness_gate: u8,
    pub hole_value_milli: u32,
    pub uses: Vec<MaterialUseDescriptor>,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatureStats {
    pub body_size: u16,
    pub attack: u16,
    pub defense: u16,
    pub danger: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootDescriptor {
    pub content_id: ContentId,
    pub units: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatureDescriptor {
    pub id: CreatureId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub tier: CreatureTier,
    pub level_min: u8,
    pub level_max: u8,
    pub stats: CreatureStats,
    pub common_loot: Vec<LootDescriptor>,
    pub primary_material: MaterialId,
    pub portrait: ArtKey,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LairBandDescriptor {
    pub band_min: u8,
    pub band_max: u8,
    pub public_art_key: ArtKey,
    /// First level in this roster-size band that requires a mystic creature.
    ///
    /// `None` means the band permits only the tier rules selected by the
    /// encounter authority without a mandatory mystic. The explicit threshold
    /// preserves Plan 1's level-60 mixed / level-61 mandatory boundary even
    /// though both levels share the 60–79 party-size band.
    pub mystic_required_from_level: Option<u8>,
    pub normal_allowed: bool,
    pub min_roster_size: u8,
    pub max_roster_size: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LairVisualBandDescriptor {
    pub band_min: u8,
    pub band_max: u8,
    pub art_key: ArtKey,
}

impl LairVisualBandDescriptor {
    #[must_use]
    pub fn level_range(&self) -> RangeInclusive<u8> {
        self.band_min..=self.band_max
    }

    #[must_use]
    pub fn art_key(&self) -> &ArtKey {
        &self.art_key
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridGeometry {
    pub width: u8,
    pub height: u8,
    pub origin_x: u8,
    pub origin_y: u8,
    pub occupied_cells: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationDescriptor {
    pub id: ItemDefinitionId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub behavior: StationBehavior,
    pub footprint_cells: u8,
    pub work_geometry: GridGeometry,
    pub landmark_geometry: Option<GridGeometry>,
    pub min_tier: u8,
    pub task_category: TaskCategory,
    pub fixture_slot: Option<FixtureSlot>,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeIngredient {
    pub content_id: ContentId,
    pub units: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDescriptor {
    pub id: RecipeId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub station: ItemDefinitionId,
    pub station_tier: u8,
    pub complexity: u8,
    pub ingredients: Vec<RecipeIngredient>,
    pub outputs: Vec<RecipeIngredient>,
    pub requires_fuel: bool,
    pub requires_container: bool,
    pub tools: Vec<ItemDefinitionId>,
    pub fixtures: Vec<ItemDefinitionId>,
    pub bundle_capability: CapabilityId,
    pub art_key: ArtKey,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutoverDisposition {
    Remove,
    SupersededBy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CutoverCard {
    #[serde(rename = "LAI.39")]
    Lai39,
    #[serde(rename = "LAI.43")]
    Lai43,
    #[serde(rename = "LAI.52")]
    Lai52,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeCutoverDescriptor {
    pub legacy_id: RecipeId,
    pub order: u32,
    pub disposition: CutoverDisposition,
    pub replacement_ids: Vec<RecipeId>,
    pub owning_cutover_card: CutoverCard,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeBundleDescriptor {
    pub id: ContentId,
    pub owner: ContentId,
    pub capability: CapabilityId,
    pub recipes: Vec<RecipeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentationDescriptor {
    pub id: ItemDefinitionId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub slot: AugmentationSlot,
    pub consumed_materials: Vec<MaterialId>,
    pub compatible_item_classes: Vec<ItemClass>,
    pub effect_operation: EffectOperation,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureDescriptor {
    pub id: ItemDefinitionId,
    pub content_id: ContentId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub art_key: ArtKey,
    pub slot: FixtureSlot,
    pub consumed_materials: Vec<MaterialId>,
    pub compatible_stations: Vec<ItemDefinitionId>,
    pub effect_operation: EffectOperation,
    pub canonical_capability: CapabilityRequirement,
    pub behavior_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchCapabilityPayload {
    pub domain: AuthorityDomain,
    pub operation: EffectOperation,
    pub effect_handler: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub display_name: String,
    pub description: String,
    pub order: u32,
    pub founding_owned: bool,
    pub prerequisites: Vec<CapabilityId>,
    /// The only serialized authority for capability-to-content grants.
    pub canonical_for: Vec<ContentId>,
    pub payload: ResearchCapabilityPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtAssetDescriptor {
    pub key: ArtKey,
    pub planned_asset_path: String,
    pub logical_key: String,
    pub native_width: u16,
    pub native_height: u16,
    pub layer: ArtLayer,
    pub accessibility: AccessibilityBinding,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentManifest {
    pub version: u32,
    pub resources: Vec<ResourceDescriptor>,
    pub construction_miracle_inputs: Vec<ConstructionMiracleInputDescriptor>,
    pub foods: Vec<FoodDescriptor>,
    pub item_definitions: Vec<ItemDefinitionDescriptor>,
    pub materials: Vec<MaterialDescriptor>,
    pub creatures: Vec<CreatureDescriptor>,
    pub lair_bands: Vec<LairBandDescriptor>,
    pub lair_visual_bands: Vec<LairVisualBandDescriptor>,
    pub stations: Vec<StationDescriptor>,
    pub recipes: Vec<RecipeDescriptor>,
    pub recipe_bundles: Vec<RecipeBundleDescriptor>,
    pub recipe_cutover: Vec<RecipeCutoverDescriptor>,
    pub augmentations: Vec<AugmentationDescriptor>,
    pub fixtures: Vec<FixtureDescriptor>,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub founding_capabilities: Vec<CapabilityId>,
    pub art_registry: Vec<ArtAssetDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalContentEntry {
    pub order: u32,
    pub class: &'static str,
    pub typed_id: String,
    pub content_id: ContentId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManifestSummary {
    pub resource_total: usize,
    pub construction_miracle_input_total: usize,
    pub food_total: usize,
    pub item_definition_total: usize,
    pub material_total: usize,
    pub creature_total: usize,
    pub station_total: usize,
    pub recipe_total: usize,
    pub augmentation_total: usize,
    pub fixture_total: usize,
    pub capability_total: usize,
    pub art_total: usize,
    pub recipe_cutover_total: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationPhase {
    Version,
    IdentityAndOrder,
    References,
    Cycles,
    NumericAndCardinality,
    HandlerRegistry,
    ArtRegistry,
    FoundingBootstrap,
    CanonicalCapability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationFailure {
    UnsupportedVersion,
    MalformedIdentity,
    DuplicateIdentity,
    NonMonotonicOrder,
    DuplicateVectorMember,
    DanglingReference,
    WrongReferenceClass,
    SlotMismatch,
    CapabilityCycle,
    NumericRange,
    Cardinality,
    MissingHandler,
    MissingArt,
    InvalidArt,
    FoundingBootstrap,
    CanonicalCapability,
    RecipeBundle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub phase: ValidationPhase,
    pub failure: ValidationFailure,
    pub path: String,
    pub message: String,
}

impl ValidationError {
    fn new(
        phase: ValidationPhase,
        failure: ValidationFailure,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            failure,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug)]
pub enum ManifestLoadError {
    Decode(serde_json::Error),
    Invalid(Vec<ValidationError>),
}

impl fmt::Display for ManifestLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => {
                write!(formatter, "content manifest strict decode failed: {error}")
            }
            Self::Invalid(errors) => {
                formatter.write_str("content manifest validation failed")?;
                for error in errors {
                    write!(formatter, "; {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ManifestLoadError {}

struct ContentRef<'a> {
    class: &'static str,
    typed_id: &'a str,
    content_id: &'a ContentId,
    order: u32,
    art_key: &'a ArtKey,
    requirement: &'a CapabilityRequirement,
    handler: &'a str,
}

impl ContentManifest {
    #[must_use]
    pub fn embedded() -> &'static Self {
        static MANIFEST: OnceLock<ContentManifest> = OnceLock::new();
        MANIFEST.get_or_init(|| {
            Self::decode_strict(EMBEDDED_MANIFEST_JSON).unwrap_or_else(|error| {
                panic!("embedded LAI.36 content manifest must strictly validate: {error}")
            })
        })
    }

    pub fn decode_strict(json: &str) -> Result<Self, ManifestLoadError> {
        let manifest = serde_json::from_str::<Self>(json).map_err(ManifestLoadError::Decode)?;
        manifest
            .validate()
            .map(|_| manifest)
            .map_err(ManifestLoadError::Invalid)
    }

    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        let mut canonical = self.clone();
        canonical.canonicalize();
        serde_json::to_string(&canonical)
            .expect("serializing the in-memory LAI.36 manifest is infallible")
    }

    #[must_use]
    pub fn public_lair_visual_bands(&self) -> &[LairVisualBandDescriptor] {
        &self.lair_visual_bands
    }

    #[must_use]
    pub fn derived_capability_total(&self) -> usize {
        self.capabilities.len()
    }

    #[must_use]
    pub fn summary(&self) -> ManifestSummary {
        ManifestSummary {
            resource_total: self.resources.len(),
            construction_miracle_input_total: self.construction_miracle_inputs.len(),
            food_total: self.foods.len(),
            item_definition_total: self.item_definitions.len(),
            material_total: self.materials.len(),
            creature_total: self.creatures.len(),
            station_total: self.stations.len(),
            recipe_total: self.recipes.len(),
            augmentation_total: self.augmentations.len(),
            fixture_total: self.fixtures.len(),
            capability_total: self.derived_capability_total(),
            art_total: self.art_registry.len(),
            recipe_cutover_total: self.recipe_cutover.len(),
        }
    }

    pub fn validate(&self) -> Result<ManifestSummary, Vec<ValidationError>> {
        let mut validator = ManifestValidator::new(self);
        validator.run();
        if validator.errors.is_empty() {
            Ok(self.summary())
        } else {
            Err(validator.errors)
        }
    }

    pub fn is_operation_permitted(
        &self,
        content_id: &ContentId,
        operation: ContentOperation,
        owned_capabilities: &BTreeSet<CapabilityId>,
    ) -> Result<bool, ValidationError> {
        let requirement = self
            .all_content()
            .into_iter()
            .find(|record| record.content_id == content_id)
            .map(|record| record.requirement)
            .ok_or_else(|| {
                ValidationError::new(
                    ValidationPhase::References,
                    ValidationFailure::DanglingReference,
                    "content_operation.content_id",
                    format!("unknown content {content_id}"),
                )
            })?;
        if !operation.requires_capability() {
            return Ok(true);
        }
        Ok(match requirement {
            CapabilityRequirement::Free => true,
            CapabilityRequirement::Required(required) => owned_capabilities.contains(required),
        })
    }

    #[must_use]
    pub fn canonical_content_entries(&self) -> Vec<CanonicalContentEntry> {
        let mut entries = self
            .all_content()
            .into_iter()
            .map(|record| CanonicalContentEntry {
                order: record.order,
                class: record.class,
                typed_id: record.typed_id.to_owned(),
                content_id: record.content_id.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            (left.order, left.content_id.as_str()).cmp(&(right.order, right.content_id.as_str()))
        });
        entries
    }

    #[must_use]
    pub fn art_asset(&self, key: &ArtKey) -> Option<&ArtAssetDescriptor> {
        self.art_registry.iter().find(|asset| &asset.key == key)
    }

    #[must_use]
    pub fn construction_miracle_input(
        &self,
        content_id: &ContentId,
    ) -> Option<&ConstructionMiracleInputDescriptor> {
        self.construction_miracle_inputs
            .binary_search_by(|entry| entry.content_id.cmp(content_id))
            .ok()
            .map(|index| &self.construction_miracle_inputs[index])
    }

    fn all_content(&self) -> Vec<ContentRef<'_>> {
        let mut records = Vec::new();
        for value in &self.resources {
            records.push(ContentRef {
                class: "resource",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.foods {
            records.push(ContentRef {
                class: "food",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.item_definitions {
            records.push(ContentRef {
                class: "item_definition",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.materials {
            records.push(ContentRef {
                class: "material",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.creatures {
            records.push(ContentRef {
                class: "creature",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.stations {
            records.push(ContentRef {
                class: "station",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.recipes {
            records.push(ContentRef {
                class: "recipe",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.augmentations {
            records.push(ContentRef {
                class: "augmentation",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        for value in &self.fixtures {
            records.push(ContentRef {
                class: "fixture",
                typed_id: value.id.as_str(),
                content_id: &value.content_id,
                order: value.order,
                art_key: &value.art_key,
                requirement: &value.canonical_capability,
                handler: &value.behavior_handler,
            });
        }
        records
    }

    fn canonicalize(&mut self) {
        self.resources
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.construction_miracle_inputs
            .sort_by_key(|record| record.content_id.clone());
        self.foods
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.item_definitions
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.materials
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.creatures
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.lair_bands.sort_by_key(|record| record.band_min);
        self.lair_visual_bands.sort_by_key(|record| record.band_min);
        self.stations
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.recipes
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.recipe_bundles.sort_by_key(|record| record.id.clone());
        self.recipe_cutover
            .sort_by_key(|record| (record.order, record.legacy_id.clone()));
        self.augmentations
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.fixtures
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.capabilities
            .sort_by_key(|record| (record.order, record.id.clone()));
        self.founding_capabilities.sort();
        self.art_registry.sort_by_key(|record| record.key.clone());

        for food in &mut self.foods {
            food.ingredient_tags.sort();
        }
        for item in &mut self.item_definitions {
            item.base_materials.sort();
            item.functions.sort();
            item.layers
                .sort_by(|left, right| left.layer_name.cmp(&right.layer_name));
        }
        for material in &mut self.materials {
            material.tags.sort();
            material
                .uses
                .sort_by_key(|use_record| (use_record.station.clone(), use_record.output.clone()));
        }
        for creature in &mut self.creatures {
            creature
                .common_loot
                .sort_by_key(|loot| loot.content_id.clone());
        }
        for recipe in &mut self.recipes {
            recipe
                .ingredients
                .sort_by_key(|ingredient| ingredient.content_id.clone());
            recipe
                .outputs
                .sort_by_key(|output| output.content_id.clone());
            recipe.tools.sort();
            recipe.fixtures.sort();
        }
        for bundle in &mut self.recipe_bundles {
            bundle.recipes.sort();
        }
        for receipt in &mut self.recipe_cutover {
            receipt.replacement_ids.sort();
        }
        for augmentation in &mut self.augmentations {
            augmentation.consumed_materials.sort();
            augmentation.compatible_item_classes.sort();
        }
        for fixture in &mut self.fixtures {
            fixture.consumed_materials.sort();
            fixture.compatible_stations.sort();
        }
        for capability in &mut self.capabilities {
            capability.prerequisites.sort();
            capability.canonical_for.sort();
        }
    }
}

struct ManifestValidator<'a> {
    manifest: &'a ContentManifest,
    errors: Vec<ValidationError>,
    content_classes: BTreeMap<ContentId, &'static str>,
    capability_ids: BTreeSet<CapabilityId>,
    material_ids: BTreeSet<MaterialId>,
    item_ids: BTreeSet<ItemDefinitionId>,
    item_classes: BTreeMap<ItemDefinitionId, ItemClass>,
    station_ids: BTreeSet<ItemDefinitionId>,
    recipe_ids: BTreeSet<RecipeId>,
    art_keys: BTreeSet<ArtKey>,
}

impl<'a> ManifestValidator<'a> {
    fn new(manifest: &'a ContentManifest) -> Self {
        Self {
            manifest,
            errors: Vec::new(),
            content_classes: BTreeMap::new(),
            capability_ids: BTreeSet::new(),
            material_ids: BTreeSet::new(),
            item_ids: BTreeSet::new(),
            item_classes: BTreeMap::new(),
            station_ids: BTreeSet::new(),
            recipe_ids: BTreeSet::new(),
            art_keys: BTreeSet::new(),
        }
    }

    fn run(&mut self) {
        self.validate_version();
        self.validate_identity_and_order();
        self.validate_references();
        self.validate_cycles();
        self.validate_numeric_and_cardinality();
        self.validate_handlers();
        self.validate_art_registry();
        self.validate_founding_bootstrap();
        self.validate_canonical_capabilities_and_bundles();
    }

    fn push(
        &mut self,
        phase: ValidationPhase,
        failure: ValidationFailure,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.errors
            .push(ValidationError::new(phase, failure, path, message));
    }

    fn validate_version(&mut self) {
        if self.manifest.version != CONTENT_MANIFEST_SCHEMA_VERSION {
            self.push(
                ValidationPhase::Version,
                ValidationFailure::UnsupportedVersion,
                "version",
                format!(
                    "unsupported content manifest version {}; expected {}",
                    self.manifest.version, CONTENT_MANIFEST_SCHEMA_VERSION
                ),
            );
        }
    }

    fn validate_identity_and_order(&mut self) {
        let records = self.manifest.all_content();
        let mut typed_by_class = BTreeSet::new();
        let mut global_orders = BTreeSet::new();
        let mut previous_by_class = BTreeMap::<&str, (u32, &str)>::new();
        for record in records {
            if !typed_by_class.insert((record.class, record.typed_id)) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateIdentity,
                    format!("{}.{}", record.class, record.typed_id),
                    format!("duplicate {} ID {}", record.class, record.typed_id),
                );
            }
            if self
                .content_classes
                .insert(record.content_id.clone(), record.class)
                .is_some()
            {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateIdentity,
                    format!("{}.content_id", record.class),
                    format!("duplicate content ID {}", record.content_id),
                );
            }
            if !global_orders.insert(record.order) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::NonMonotonicOrder,
                    format!("{}.order", record.class),
                    format!("non-monotonic stable order {}", record.order),
                );
            }
            if let Some(previous) =
                previous_by_class.insert(record.class, (record.order, record.content_id.as_str()))
            {
                if previous >= (record.order, record.content_id.as_str()) {
                    self.push(
                        ValidationPhase::IdentityAndOrder,
                        ValidationFailure::NonMonotonicOrder,
                        format!("{}.order", record.class),
                        format!("non-monotonic stable order at {}", record.content_id),
                    );
                }
            }
        }

        let mut miracle_inputs = BTreeSet::new();
        let mut previous_miracle_input = None;
        for entry in &self.manifest.construction_miracle_inputs {
            if !miracle_inputs.insert(&entry.content_id) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateIdentity,
                    "construction_miracle_inputs.content_id",
                    format!("duplicate construction-miracle input {}", entry.content_id),
                );
            }
            if previous_miracle_input
                .is_some_and(|previous: &ContentId| previous >= &entry.content_id)
            {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::NonMonotonicOrder,
                    "construction_miracle_inputs.content_id",
                    format!(
                        "non-monotonic construction-miracle input order at {}",
                        entry.content_id
                    ),
                );
            }
            previous_miracle_input = Some(&entry.content_id);
        }

        let mut previous_capability = None;
        for capability in &self.manifest.capabilities {
            if !self.capability_ids.insert(capability.id.clone()) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateIdentity,
                    "capabilities.id",
                    format!("duplicate capability ID {}", capability.id),
                );
            }
            if let Some(previous) = previous_capability {
                if previous >= (capability.order, capability.id.as_str()) {
                    self.push(
                        ValidationPhase::IdentityAndOrder,
                        ValidationFailure::NonMonotonicOrder,
                        "capabilities.order",
                        format!("non-monotonic stable order at {}", capability.id),
                    );
                }
            }
            previous_capability = Some((capability.order, capability.id.as_str()));
            self.duplicate_members(
                &capability.prerequisites,
                format!("capabilities.{}.prerequisites", capability.id),
            );
            self.duplicate_members(
                &capability.canonical_for,
                format!("capabilities.{}.canonical_for", capability.id),
            );
        }

        let mut cutover_ids = BTreeSet::new();
        let mut cutover_orders = BTreeSet::new();
        let mut previous_cutover = None;
        for record in &self.manifest.recipe_cutover {
            if !cutover_ids.insert(record.legacy_id.clone()) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateIdentity,
                    "recipe_cutover.legacy_id",
                    format!("duplicate recipe cutover ID {}", record.legacy_id),
                );
            }
            if !cutover_orders.insert(record.order) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::NonMonotonicOrder,
                    "recipe_cutover.order",
                    format!("duplicate recipe cutover order {}", record.order),
                );
            }
            if let Some(previous) = previous_cutover {
                if previous >= (record.order, record.legacy_id.as_str()) {
                    self.push(
                        ValidationPhase::IdentityAndOrder,
                        ValidationFailure::NonMonotonicOrder,
                        "recipe_cutover.order",
                        format!("non-monotonic recipe cutover order at {}", record.legacy_id),
                    );
                }
            }
            previous_cutover = Some((record.order, record.legacy_id.as_str()));
            self.duplicate_members(
                &record.replacement_ids,
                format!("recipe_cutover.{}.replacement_ids", record.legacy_id),
            );
        }

        for material in &self.manifest.materials {
            self.material_ids.insert(material.id.clone());
            self.duplicate_members(&material.tags, format!("materials.{}.tags", material.id));
        }
        for item in &self.manifest.item_definitions {
            self.item_ids.insert(item.id.clone());
            self.item_classes.insert(item.id.clone(), item.class);
            self.duplicate_members(
                &item.base_materials,
                format!("item_definitions.{}.base_materials", item.id),
            );
            self.duplicate_members(
                &item.functions,
                format!("item_definitions.{}.functions", item.id),
            );
        }
        for station in &self.manifest.stations {
            self.station_ids.insert(station.id.clone());
            self.item_ids.insert(station.id.clone());
        }
        for recipe in &self.manifest.recipes {
            self.recipe_ids.insert(recipe.id.clone());
            self.duplicate_members(&recipe.tools, format!("recipes.{}.tools", recipe.id));
            self.duplicate_members(&recipe.fixtures, format!("recipes.{}.fixtures", recipe.id));
            self.duplicate_ingredient_members(
                &recipe.ingredients,
                format!("recipes.{}.ingredients", recipe.id),
            );
            self.duplicate_ingredient_members(
                &recipe.outputs,
                format!("recipes.{}.outputs", recipe.id),
            );
        }
        self.duplicate_members(
            &self.manifest.founding_capabilities,
            "founding_capabilities",
        );
    }

    fn duplicate_members<T: Ord + fmt::Debug>(&mut self, values: &[T], path: impl Into<String>) {
        let path = path.into();
        let mut seen = BTreeSet::new();
        for value in values {
            if !seen.insert(value) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateVectorMember,
                    &path,
                    format!("duplicate vector member {value:?}"),
                );
            }
        }
    }

    fn duplicate_ingredient_members(&mut self, values: &[RecipeIngredient], path: String) {
        let mut seen = BTreeSet::new();
        for value in values {
            if !seen.insert(&value.content_id) {
                self.push(
                    ValidationPhase::IdentityAndOrder,
                    ValidationFailure::DuplicateVectorMember,
                    &path,
                    format!("duplicate vector member {}", value.content_id),
                );
            }
        }
    }

    fn validate_references(&mut self) {
        let capabilities = self.capability_ids.clone();
        for record in self.manifest.all_content() {
            if let Some(required) = record.requirement.required_id() {
                if !capabilities.contains(required) {
                    self.dangling(
                        format!("{}.canonical_capability", record.content_id),
                        format!("dangling capability {required}"),
                    );
                }
            }
        }
        for entry in &self.manifest.construction_miracle_inputs {
            let actual_class = self.content_classes.get(&entry.content_id).copied();
            let expected_class = match entry.physical_class {
                ConstructionMiracleInputClass::BulkLot => Some("resource"),
                ConstructionMiracleInputClass::ExactItem => Some("item_definition"),
                ConstructionMiracleInputClass::Fixture => Some("fixture"),
                ConstructionMiracleInputClass::Ineligible => None,
            };
            match (actual_class, expected_class) {
                (None, _) => self.dangling(
                    format!(
                        "construction_miracle_inputs.{}.content_id",
                        entry.content_id
                    ),
                    format!("dangling construction-miracle input {}", entry.content_id),
                ),
                (Some(actual), Some(expected)) if actual != expected => self.push(
                    ValidationPhase::References,
                    ValidationFailure::WrongReferenceClass,
                    format!(
                        "construction_miracle_inputs.{}.physical_class",
                        entry.content_id
                    ),
                    format!(
                        "{} is class {actual}, but {:?} requires {expected}",
                        entry.content_id, entry.physical_class
                    ),
                ),
                _ => {}
            }
            if let Some(material_id) = &entry.generated_material_id
                && !self.material_ids.contains(material_id)
            {
                self.dangling(
                    format!(
                        "construction_miracle_inputs.{}.generated_material_id",
                        entry.content_id
                    ),
                    format!("dangling generated material {material_id}"),
                );
            }
        }
        for resource in &self.manifest.resources {
            if let Some(required) = &resource.processing_capability {
                if !capabilities.contains(required) {
                    self.dangling(
                        format!("resources.{}.processing_capability", resource.id),
                        format!("dangling capability {required}"),
                    );
                }
            }
        }
        for food in &self.manifest.foods {
            if let Some(required) = &food.recipe_bundle {
                if !capabilities.contains(required) {
                    self.dangling(
                        format!("foods.{}.recipe_bundle", food.id),
                        format!("dangling capability {required}"),
                    );
                }
            }
        }
        for item in &self.manifest.item_definitions {
            for material in &item.base_materials {
                if !self.material_ids.contains(material) {
                    self.dangling(
                        format!("item_definitions.{}.base_materials", item.id),
                        format!("dangling material {material}"),
                    );
                }
            }
            let slot_valid = match item.class {
                ItemClass::Weapon => item.augmentation_slot == Some(AugmentationSlot::Weapon),
                ItemClass::Armor | ItemClass::Clothing => {
                    item.augmentation_slot == Some(AugmentationSlot::Armor)
                }
                ItemClass::Tool => item.augmentation_slot == Some(AugmentationSlot::Tool),
                ItemClass::ResearchInstrument => {
                    item.augmentation_slot == Some(AugmentationSlot::ResearchInstrument)
                }
                ItemClass::Augmentation => item.augmentation_slot.is_some(),
                ItemClass::Fixture => item.fixture_slot.is_some(),
                _ => item.augmentation_slot.is_none() && item.fixture_slot.is_none(),
            };
            if !slot_valid {
                self.push(
                    ValidationPhase::References,
                    ValidationFailure::SlotMismatch,
                    format!("item_definitions.{}.slot", item.id),
                    format!("{} without slot-compatible item class", item.id),
                );
            }
        }
        for material in &self.manifest.materials {
            for use_record in &material.uses {
                if !self.station_ids.contains(&use_record.station) {
                    self.dangling(
                        format!("materials.{}.uses.station", material.id),
                        format!("dangling station {}", use_record.station),
                    );
                }
                if !self.content_classes.contains_key(&use_record.output) {
                    self.dangling(
                        format!("materials.{}.uses.output", material.id),
                        format!("dangling output {}", use_record.output),
                    );
                }
            }
        }
        for creature in &self.manifest.creatures {
            if !self.material_ids.contains(&creature.primary_material) {
                self.dangling(
                    format!("creatures.{}.primary_material", creature.id),
                    format!("dangling material {}", creature.primary_material),
                );
            }
            for loot in &creature.common_loot {
                if !self.content_classes.contains_key(&loot.content_id) {
                    self.dangling(
                        format!("creatures.{}.common_loot", creature.id),
                        format!("dangling loot {}", loot.content_id),
                    );
                }
            }
        }
        for recipe in &self.manifest.recipes {
            if !self.station_ids.contains(&recipe.station) {
                self.dangling(
                    format!("recipes.{}.station", recipe.id),
                    format!("dangling station {}", recipe.station),
                );
            }
            if !capabilities.contains(&recipe.bundle_capability) {
                self.dangling(
                    format!("recipes.{}.bundle_capability", recipe.id),
                    format!("dangling capability {}", recipe.bundle_capability),
                );
            }
            for ingredient in &recipe.ingredients {
                if !self.content_classes.contains_key(&ingredient.content_id) {
                    self.dangling(
                        format!("recipes.{}.ingredients", recipe.id),
                        format!("dangling ingredient {}", ingredient.content_id),
                    );
                }
            }
            for output in &recipe.outputs {
                if !self.content_classes.contains_key(&output.content_id) {
                    self.dangling(
                        format!("recipes.{}.outputs", recipe.id),
                        format!("dangling output {}", output.content_id),
                    );
                }
            }
            let ingredient_ids = recipe
                .ingredients
                .iter()
                .map(|ingredient| &ingredient.content_id)
                .collect::<BTreeSet<_>>();
            if !UNCHANGED_RECIPE_FLOW_ALLOWLIST.contains(&recipe.id.as_str())
                && recipe
                    .outputs
                    .iter()
                    .any(|output| ingredient_ids.contains(&output.content_id))
            {
                self.push(
                    ValidationPhase::References,
                    ValidationFailure::WrongReferenceClass,
                    format!("recipes.{}.outputs", recipe.id),
                    format!(
                        "recipe {} emits an ingredient unchanged without an explicit allowlist entry",
                        recipe.id
                    ),
                );
            }
            for tool in &recipe.tools {
                if !matches!(
                    self.item_classes.get(tool),
                    Some(ItemClass::Tool | ItemClass::ResearchInstrument | ItemClass::Tableware)
                ) {
                    self.dangling(
                        format!("recipes.{}.tools", recipe.id),
                        format!("dangling or wrong-class tool {tool}"),
                    );
                }
            }
            for fixture in &recipe.fixtures {
                if !self
                    .manifest
                    .fixtures
                    .iter()
                    .any(|candidate| &candidate.id == fixture)
                {
                    self.dangling(
                        format!("recipes.{}.fixtures", recipe.id),
                        format!("dangling fixture {fixture}"),
                    );
                }
            }
        }
        for augmentation in &self.manifest.augmentations {
            for material in &augmentation.consumed_materials {
                if !self.material_ids.contains(material) {
                    self.dangling(
                        format!("augmentations.{}.consumed_materials", augmentation.id),
                        format!("dangling material {material}"),
                    );
                }
            }
        }
        for fixture in &self.manifest.fixtures {
            for material in &fixture.consumed_materials {
                if !self.material_ids.contains(material) {
                    self.dangling(
                        format!("fixtures.{}.consumed_materials", fixture.id),
                        format!("dangling material {material}"),
                    );
                }
            }
            for station in &fixture.compatible_stations {
                if !self.station_ids.contains(station) {
                    self.dangling(
                        format!("fixtures.{}.compatible_stations", fixture.id),
                        format!("dangling station {station}"),
                    );
                }
            }
        }
        for capability in &self.manifest.capabilities {
            for prerequisite in &capability.prerequisites {
                if !capabilities.contains(prerequisite) {
                    self.dangling(
                        format!("capabilities.{}.prerequisites", capability.id),
                        format!("dangling capability {prerequisite}"),
                    );
                }
            }
            for content_id in &capability.canonical_for {
                if !self.content_classes.contains_key(content_id) {
                    self.dangling(
                        format!("capabilities.{}.canonical_for", capability.id),
                        format!("dangling content {content_id}"),
                    );
                }
            }
        }
        for cutover in &self.manifest.recipe_cutover {
            if self.recipe_ids.contains(&cutover.legacy_id) {
                self.push(
                    ValidationPhase::References,
                    ValidationFailure::WrongReferenceClass,
                    format!("recipe_cutover.{}.legacy_id", cutover.legacy_id),
                    format!(
                        "cutover recipe {} must not also be canonical recipe content",
                        cutover.legacy_id
                    ),
                );
            }
            for replacement in &cutover.replacement_ids {
                if !self.recipe_ids.contains(replacement) {
                    self.dangling(
                        format!("recipe_cutover.{}.replacement_ids", cutover.legacy_id),
                        format!("superseding recipe {} is not canonical", replacement),
                    );
                }
            }
            match cutover.disposition {
                CutoverDisposition::SupersededBy if cutover.replacement_ids.is_empty() => {
                    self.push(
                        ValidationPhase::References,
                        ValidationFailure::WrongReferenceClass,
                        format!("recipe_cutover.{}.replacement_ids", cutover.legacy_id),
                        format!(
                            "superseded recipe {} must reference one or more canonical recipes",
                            cutover.legacy_id
                        ),
                    );
                }
                CutoverDisposition::Remove if !cutover.replacement_ids.is_empty() => self.push(
                    ValidationPhase::References,
                    ValidationFailure::WrongReferenceClass,
                    format!("recipe_cutover.{}.replacement_ids", cutover.legacy_id),
                    format!(
                        "removed recipe {} cannot reference superseding recipes",
                        cutover.legacy_id
                    ),
                ),
                CutoverDisposition::Remove | CutoverDisposition::SupersededBy => {}
            }
        }
    }

    fn dangling(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.push(
            ValidationPhase::References,
            ValidationFailure::DanglingReference,
            path,
            message,
        );
    }

    fn validate_cycles(&mut self) {
        let graph = self
            .manifest
            .capabilities
            .iter()
            .map(|capability| (&capability.id, &capability.prerequisites))
            .collect::<BTreeMap<_, _>>();
        let mut completed = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        for capability in &self.manifest.capabilities {
            if cycle_from(&capability.id, &graph, &mut visiting, &mut completed) {
                self.push(
                    ValidationPhase::Cycles,
                    ValidationFailure::CapabilityCycle,
                    format!("capabilities.{}.prerequisites", capability.id),
                    format!("capability prerequisite cycle involving {}", capability.id),
                );
                break;
            }
        }
    }

    fn validate_numeric_and_cardinality(&mut self) {
        let actual_miracle_input_ids = self
            .manifest
            .construction_miracle_inputs
            .iter()
            .map(|entry| entry.content_id.as_str())
            .collect::<Vec<_>>();
        if actual_miracle_input_ids != CONSTRUCTION_MIRACLE_INPUT_IDS {
            self.cardinality(
                "construction_miracle_inputs",
                "construction-miracle inputs must classify the exact fifteen canonical staged-construction bill identities",
            );
        }
        for entry in &self.manifest.construction_miracle_inputs {
            match (
                entry.physical_class,
                &entry.hole_feed_policy,
                &entry.generated_material_id,
            ) {
                (ConstructionMiracleInputClass::BulkLot, Some(policy), None)
                | (
                    ConstructionMiracleInputClass::ExactItem
                    | ConstructionMiracleInputClass::Fixture,
                    Some(policy),
                    Some(_),
                ) if policy.base_value_milli > 0 && policy.required_darkness <= 10 => {}
                (
                    ConstructionMiracleInputClass::BulkLot
                    | ConstructionMiracleInputClass::ExactItem
                    | ConstructionMiracleInputClass::Fixture,
                    _,
                    _,
                ) => self.numeric(
                    format!("construction_miracle_inputs.{}", entry.content_id),
                    "generatable construction input requires a positive Hole value policy, darkness in 0..=10, and class-correct generated material identity",
                ),
                (ConstructionMiracleInputClass::Ineligible, None, None) => {}
                _ => self.numeric(
                    format!("construction_miracle_inputs.{}", entry.content_id),
                    "ineligible construction input cannot define value or generated material",
                ),
            }
        }
        for food in &self.manifest.foods {
            if (food.nutrition == 0 && food.hydration == 0)
                || food.weight_milli == 0
                || food.value_milli == 0
            {
                self.numeric(
                    format!("foods.{}", food.id),
                    format!(
                        "food {} has no nutrition or hydration or a zero weight/value",
                        food.id
                    ),
                );
            }
        }
        for material in &self.manifest.materials {
            if material.hole_darkness_gate > 10
                || material.hole_value_milli == 0
                || material.uses.is_empty()
                || material.raw_state == material.processed_state
            {
                self.numeric(
                    format!("materials.{}", material.id),
                    format!(
                        "material {} has impossible range/state/use constraints",
                        material.id
                    ),
                );
            }
        }
        for creature in &self.manifest.creatures {
            if creature.level_min == 0
                || creature.level_min > creature.level_max
                || creature.level_max > 100
            {
                self.numeric(
                    format!("creatures.{}.levels", creature.id),
                    format!("creature {} has impossible level range", creature.id),
                );
            }
            if creature.common_loot.is_empty()
                || creature.common_loot.iter().any(|loot| loot.units == 0)
                || creature.stats.body_size == 0
                || creature.stats.attack == 0
                || creature.stats.defense == 0
                || creature.stats.danger == 0
            {
                self.numeric(
                    format!("creatures.{}.loot", creature.id),
                    format!("creature {} has impossible loot or stats", creature.id),
                );
            }
        }
        for station in &self.manifest.stations {
            if station.min_tier == 0
                || station.footprint_cells == 0
                || station.work_geometry.width == 0
                || station.work_geometry.height == 0
                || station.work_geometry.occupied_cells != station.footprint_cells
            {
                self.numeric(
                    format!("stations.{}", station.id),
                    format!("station {} has impossible geometry/tier", station.id),
                );
            }
        }
        if let Some(hole) = self
            .manifest
            .stations
            .iter()
            .find(|station| station.behavior == StationBehavior::Hole)
        {
            let work = &hole.work_geometry;
            let landmark = hole.landmark_geometry.as_ref();
            if (
                work.width,
                work.height,
                work.origin_x,
                work.origin_y,
                work.occupied_cells,
            ) != (3, 3, 1, 1, 9)
                || landmark.map(|geometry| {
                    (
                        geometry.width,
                        geometry.height,
                        geometry.origin_x,
                        geometry.origin_y,
                        geometry.occupied_cells,
                    )
                }) != Some((5, 5, 0, 0, 25))
            {
                self.numeric(
                    "stations.black_hole.geometry",
                    "Hole must keep central 3x3 work geometry separate from fixed 5x5 landmark geometry",
                );
            }
        } else {
            self.cardinality("stations", "exactly one Hole station is required");
        }
        for recipe in &self.manifest.recipes {
            if recipe.station_tier == 0
                || !(1..=5).contains(&recipe.complexity)
                || recipe.ingredients.is_empty()
                || recipe.outputs.is_empty()
                || recipe
                    .ingredients
                    .iter()
                    .chain(&recipe.outputs)
                    .any(|part| part.units == 0)
            {
                self.numeric(
                    format!("recipes.{}", recipe.id),
                    format!("recipe {} has impossible recipe constraints", recipe.id),
                );
            }
        }
        for augmentation in &self.manifest.augmentations {
            if augmentation.consumed_materials.len() != 1
                || augmentation.compatible_item_classes.is_empty()
            {
                self.cardinality(
                    format!("augmentations.{}", augmentation.id),
                    format!(
                        "augmentation {} must consume one material and name compatible classes",
                        augmentation.id
                    ),
                );
            }
        }
        for fixture in &self.manifest.fixtures {
            if fixture.consumed_materials.len() != 1 || fixture.compatible_stations.is_empty() {
                self.cardinality(
                    format!("fixtures.{}", fixture.id),
                    format!(
                        "fixture {} must consume one material and name compatible stations",
                        fixture.id
                    ),
                );
            }
        }
        if self.manifest.creatures.len() != PLAN1_CREATURE_IDS.len()
            || !PLAN1_CREATURE_IDS
                .iter()
                .zip(&self.manifest.creatures)
                .all(|(expected, actual)| *expected == actual.id.as_str())
        {
            self.cardinality(
                "creatures",
                "creature roster must contain the exact twenty Plan 1 identities in canonical order",
            );
        }
        if self.manifest.materials.len() != PLAN1_RARE_MATERIAL_IDS.len()
            || !PLAN1_RARE_MATERIAL_IDS.iter().all(|expected| {
                self.manifest
                    .materials
                    .iter()
                    .any(|material| material.id.as_str() == *expected)
            })
        {
            self.cardinality(
                "materials",
                "material catalog must contain exactly the twenty named Plan 1 materials",
            );
        }
        self.validate_bands();
        let cookhouse_count = self
            .manifest
            .recipes
            .iter()
            .filter(|recipe| recipe.station.as_str() == "cookhouse")
            .count();
        if cookhouse_count != PLAN1_COOKHOUSE_RECIPE_IDS.len() + PLAN1_BREW_RECIPE_IDS.len()
            || !PLAN1_COOKHOUSE_RECIPE_IDS.iter().all(|expected| {
                self.manifest
                    .recipes
                    .iter()
                    .any(|recipe| recipe.id.as_str() == *expected)
            })
            || !PLAN1_BREW_RECIPE_IDS.iter().all(|expected| {
                self.manifest.recipes.iter().any(|recipe| {
                    recipe.id.as_str() == *expected && recipe.station.as_str() == "cookhouse"
                })
            })
        {
            self.cardinality(
                "recipes",
                "Cookhouse must contain the approved eighteen recipes plus the five retained Plan 1 brewing recipes",
            );
        }
        let active_mill = self
            .manifest
            .recipes
            .iter()
            .filter(|recipe| recipe.station.as_str() == "mill")
            .map(|recipe| recipe.id.as_str())
            .collect::<Vec<_>>();
        if active_mill.len() != 1 || active_mill[0] != "mill_flour" {
            self.cardinality(
                "recipes",
                "Mill Flour must be the only canonical Mill recipe; predecessors belong only in the cutover ledger",
            );
        }
        if self.manifest.recipe_cutover.len() != RECIPE_CUTOVER_RECEIPT_TOTAL {
            self.cardinality(
                "recipe_cutover",
                format!(
                    "recipe cutover must contain {CURRENT_RUNTIME_RECIPE_CUTOVER_TOTAL} current-runtime dispositions, including {CURRENT_MILL_RECIPE_CUTOVER_TOTAL} Mill dispositions, plus the persisted combined-Mill alias receipt"
                ),
            );
        }
        let locked_runtime = PRE_CUTOVER_RUNTIME_RECIPE_IDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let canonical_runtime = self
            .manifest
            .recipes
            .iter()
            .map(|recipe| recipe.id.as_str())
            .filter(|recipe| locked_runtime.contains(recipe))
            .collect::<BTreeSet<_>>();
        let disposition_runtime = self
            .manifest
            .recipe_cutover
            .iter()
            .map(|receipt| receipt.legacy_id.as_str())
            .filter(|recipe| locked_runtime.contains(recipe))
            .collect::<BTreeSet<_>>();
        let covered_runtime = canonical_runtime
            .union(&disposition_runtime)
            .copied()
            .collect::<BTreeSet<_>>();
        if locked_runtime.len() != PRE_CUTOVER_RUNTIME_RECIPE_TOTAL
            || covered_runtime != locked_runtime
            || canonical_runtime.len() != RETAINED_PRE_CUTOVER_RECIPE_TOTAL
            || disposition_runtime.len() != CURRENT_RUNTIME_RECIPE_CUTOVER_TOTAL
        {
            self.cardinality(
                "recipe_cutover",
                format!(
                    "canonical recipes and cutover dispositions must partition the exact {PRE_CUTOVER_RUNTIME_RECIPE_TOTAL}-ID pre-cutover runtime inventory"
                ),
            );
        }
        let alias_receipt = self
            .manifest
            .recipe_cutover
            .iter()
            .find(|receipt| receipt.legacy_id.as_str() == PERSISTED_COMBINED_MILL_RECIPE_ALIAS);
        let valid_alias_receipt = alias_receipt.is_some_and(|receipt| {
            receipt.disposition == CutoverDisposition::Remove && receipt.replacement_ids.is_empty()
        });
        if !valid_alias_receipt
            || self.manifest.recipe_cutover.iter().any(|receipt| {
                !locked_runtime.contains(receipt.legacy_id.as_str())
                    && receipt.legacy_id.as_str() != PERSISTED_COMBINED_MILL_RECIPE_ALIAS
            })
        {
            self.cardinality(
                "recipe_cutover",
                "the persisted combined-Mill alias must have one explicit Remove receipt and no unowned compatibility IDs are permitted",
            );
        }
        for receipt in &self.manifest.recipe_cutover {
            if receipt.rationale.trim().is_empty() {
                self.cardinality(
                    format!("recipe_cutover.{}.rationale", receipt.legacy_id),
                    format!("cutover receipt {} requires a rationale", receipt.legacy_id),
                );
            }
        }
        let hole_axis_count = self
            .manifest
            .capabilities
            .iter()
            .filter(|capability| {
                [
                    "black_hole_width_",
                    "black_hole_depth_",
                    "black_hole_darkness_",
                ]
                .iter()
                .any(|prefix| capability.id.as_str().starts_with(prefix))
            })
            .count();
        if hole_axis_count != HOLE_AXIS_COUNT {
            self.cardinality(
                "capabilities",
                format!("Hole must expose exactly {HOLE_AXIS_COUNT} axis capabilities"),
            );
        }
    }

    fn validate_bands(&mut self) {
        let expected_encounter = [(1, 19), (20, 39), (40, 59), (60, 79), (80, 94), (95, 100)];
        let expected_mystic_thresholds = [None, None, None, Some(61), Some(80), Some(95)];
        if self.manifest.lair_bands.len() != expected_encounter.len()
            || !expected_encounter
                .iter()
                .zip(&self.manifest.lair_bands)
                .all(|(expected, band)| *expected == (band.band_min, band.band_max))
        {
            self.cardinality(
                "lair_bands",
                "lair encounter bands must be the exact six roster bands",
            );
        }
        if self.manifest.lair_bands.len() != expected_mystic_thresholds.len()
            || !expected_mystic_thresholds
                .iter()
                .zip(&self.manifest.lair_bands)
                .all(|(expected, band)| *expected == band.mystic_required_from_level)
        {
            self.cardinality(
                "lair_bands",
                "mystic thresholds must preserve mixed level 60 and mandatory mystic levels 61–100",
            );
        }
        let expected_visual = (0_u8..10)
            .map(|index| (index * 10 + 1, index * 10 + 10))
            .collect::<Vec<_>>();
        if self.manifest.lair_visual_bands.len() != expected_visual.len()
            || !expected_visual
                .iter()
                .zip(&self.manifest.lair_visual_bands)
                .all(|(expected, band)| *expected == (band.band_min, band.band_max))
        {
            self.cardinality(
                "lair_visual_bands",
                "public lair art bands must be the exact ten 1-10 through 91-100 bands",
            );
        }
    }

    fn numeric(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.push(
            ValidationPhase::NumericAndCardinality,
            ValidationFailure::NumericRange,
            path,
            message,
        );
    }

    fn cardinality(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.push(
            ValidationPhase::NumericAndCardinality,
            ValidationFailure::Cardinality,
            path,
            message,
        );
    }

    fn validate_handlers(&mut self) {
        for record in self.manifest.all_content() {
            if compiled_handler(record.handler).is_none() {
                self.push(
                    ValidationPhase::HandlerRegistry,
                    ValidationFailure::MissingHandler,
                    format!("{}.{}.behavior_handler", record.class, record.typed_id),
                    format!("missing live behavior handler {}", record.handler),
                );
            }
        }
        for capability in &self.manifest.capabilities {
            if compiled_handler(&capability.payload.effect_handler).is_none() {
                self.push(
                    ValidationPhase::HandlerRegistry,
                    ValidationFailure::MissingHandler,
                    format!("capabilities.{}.effect_handler", capability.id),
                    format!(
                        "missing live behavior handler {}",
                        capability.payload.effect_handler
                    ),
                );
            }
        }
    }

    fn validate_art_registry(&mut self) {
        let mut registry_keys = BTreeSet::new();
        let mut logical_keys = BTreeSet::new();
        let mut primary_keys = BTreeSet::new();
        for asset in &self.manifest.art_registry {
            if !registry_keys.insert(asset.key.clone()) {
                self.push(
                    ValidationPhase::ArtRegistry,
                    ValidationFailure::InvalidArt,
                    "art_registry.key",
                    format!("duplicated ArtKey {}", asset.key),
                );
            }
            if !logical_keys.insert(asset.logical_key.as_str()) {
                self.push(
                    ValidationPhase::ArtRegistry,
                    ValidationFailure::InvalidArt,
                    "art_registry.logical_key",
                    format!("duplicate art logical key {}", asset.logical_key),
                );
            }
            if asset.planned_asset_path.is_empty()
                || asset.logical_key.is_empty()
                || asset.native_width == 0
                || asset.native_height == 0
                || !asset.planned_asset_path.ends_with(".png")
            {
                self.push(
                    ValidationPhase::ArtRegistry,
                    ValidationFailure::InvalidArt,
                    format!("art_registry.{}", asset.key),
                    format!("invalid art registry metadata for {}", asset.key),
                );
            }
            let expected_square_size = match (asset.layer, asset.accessibility) {
                (ArtLayer::Icon | ArtLayer::ItemMaterial, AccessibilityBinding::ContentName) => {
                    Some(16)
                }
                (ArtLayer::Portrait, AccessibilityBinding::CreatureName)
                | (ArtLayer::WorldBase | ArtLayer::UiDetail, AccessibilityBinding::LairBand) => {
                    Some(80)
                }
                (ArtLayer::UiDetail, AccessibilityBinding::ContentName) => Some(32),
                (ArtLayer::WorldBase, AccessibilityBinding::ContentName)
                    if asset.key.as_str() == "art_station_black_hole" =>
                {
                    Some(80)
                }
                (ArtLayer::WorldBase, AccessibilityBinding::ContentName)
                    if asset.key.as_str().starts_with("art_station_") =>
                {
                    Some(48)
                }
                _ => None,
            };
            if expected_square_size.is_some_and(|expected| {
                asset.native_width != expected || asset.native_height != expected
            }) {
                self.push(
                    ValidationPhase::ArtRegistry,
                    ValidationFailure::InvalidArt,
                    format!("art_registry.{}.native_dimensions", asset.key),
                    format!(
                        "art registry dimensions for {} do not match its declared layer and accessibility role",
                        asset.key
                    ),
                );
            }
        }
        self.art_keys = registry_keys;

        let registered_order_ceiling = self
            .manifest
            .all_content()
            .into_iter()
            .filter(|record| self.art_keys.contains(record.art_key))
            .map(|record| record.order)
            .max()
            .unwrap_or(0);
        for record in self.manifest.all_content() {
            if !primary_keys.insert(record.art_key.clone()) {
                self.push(
                    ValidationPhase::ArtRegistry,
                    ValidationFailure::InvalidArt,
                    format!("{}.{}.art_key", record.class, record.typed_id),
                    format!("duplicated ArtKey {}", record.art_key),
                );
            }
            let conventional_additive_key = record.order > registered_order_ceiling
                && record.art_key.as_str().starts_with("art_")
                && record.art_key.as_str().ends_with(record.typed_id);
            if !conventional_additive_key {
                self.require_art(
                    record.art_key,
                    format!("{}.{}.art_key", record.class, record.typed_id),
                );
            }
        }
        for creature in &self.manifest.creatures {
            self.require_art(
                &creature.portrait,
                format!("creatures.{}.portrait", creature.id),
            );
        }
        for band in &self.manifest.lair_bands {
            self.require_art(&band.public_art_key, "lair_bands.public_art_key");
        }
        for band in &self.manifest.lair_visual_bands {
            self.require_art(&band.art_key, "lair_visual_bands.art_key");
        }
        for item in &self.manifest.item_definitions {
            for layer in &item.layers {
                self.require_art(
                    &layer.art_key,
                    format!("item_definitions.{}.layers", item.id),
                );
            }
        }
    }

    fn require_art(&mut self, key: &ArtKey, path: impl Into<String>) {
        if !self.art_keys.contains(key) {
            self.push(
                ValidationPhase::ArtRegistry,
                ValidationFailure::MissingArt,
                path,
                format!("ArtKey {key} is missing from the manifest art registry"),
            );
        }
    }

    fn validate_founding_bootstrap(&mut self) {
        let founding = self
            .manifest
            .founding_capabilities
            .iter()
            .map(CapabilityId::as_str)
            .collect::<BTreeSet<_>>();
        for required in REQUIRED_FOUNDING_CAPABILITIES {
            let capability = self
                .manifest
                .capabilities
                .iter()
                .find(|capability| capability.id.as_str() == required);
            if !founding.contains(required)
                || capability
                    .map(|capability| !capability.founding_owned)
                    .unwrap_or(true)
            {
                self.push(
                    ValidationPhase::FoundingBootstrap,
                    ValidationFailure::FoundingBootstrap,
                    "founding_capabilities",
                    format!("unavailable founding bootstrap capability {required}"),
                );
            }
        }
        for resource in ["logs", "stone"] {
            if !self.manifest.resources.iter().any(|candidate| {
                candidate.id.as_str() == resource
                    && candidate.acquisition.founding_available
                    && candidate.canonical_capability == CapabilityRequirement::Free
            }) {
                self.push(
                    ValidationPhase::FoundingBootstrap,
                    ValidationFailure::FoundingBootstrap,
                    "resources",
                    format!("unavailable founding bootstrap resource {resource}"),
                );
            }
        }
    }

    fn validate_canonical_capabilities_and_bundles(&mut self) {
        let mut grants = BTreeMap::<&ContentId, Vec<&CapabilityId>>::new();
        for capability in &self.manifest.capabilities {
            for content_id in &capability.canonical_for {
                grants.entry(content_id).or_default().push(&capability.id);
            }
        }
        for record in self.manifest.all_content() {
            let owners = grants.get(record.content_id).map_or(&[][..], Vec::as_slice);
            match record.requirement {
                CapabilityRequirement::Free if !owners.is_empty() => self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::CanonicalCapability,
                    format!("{}.{}.canonical_capability", record.class, record.typed_id),
                    format!(
                        "free content {} does not have exactly zero canonical capability grants",
                        record.content_id
                    ),
                ),
                CapabilityRequirement::Required(required)
                    if owners.len() != 1 || owners.first().copied() != Some(required) =>
                {
                    self.push(
                        ValidationPhase::CanonicalCapability,
                        ValidationFailure::CanonicalCapability,
                        format!("{}.{}.canonical_capability", record.class, record.typed_id),
                        format!(
                            "content {} does not have exactly one canonical capability",
                            record.content_id
                        ),
                    );
                }
                _ => {}
            }
        }

        let mut owners = BTreeMap::<&RecipeId, Vec<&RecipeBundleDescriptor>>::new();
        let resource_or_material = self
            .manifest
            .resources
            .iter()
            .map(|record| &record.content_id)
            .chain(
                self.manifest
                    .materials
                    .iter()
                    .map(|record| &record.content_id),
            )
            .collect::<BTreeSet<_>>();
        for bundle in &self.manifest.recipe_bundles {
            if !resource_or_material.contains(&bundle.owner) {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::WrongReferenceClass,
                    format!("recipe_bundles.{}.owner", bundle.id),
                    format!(
                        "recipe bundle owner {} is not a resource or material",
                        bundle.owner
                    ),
                );
            }
            if !self.capability_ids.contains(&bundle.capability) {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::RecipeBundle,
                    format!("recipe_bundles.{}.capability", bundle.id),
                    format!(
                        "recipe bundle has dangling capability {}",
                        bundle.capability
                    ),
                );
            }
            let owner_requirement = self
                .manifest
                .all_content()
                .into_iter()
                .find(|record| record.content_id == &bundle.owner)
                .map(|record| record.requirement);
            if owner_requirement.and_then(CapabilityRequirement::required_id)
                != Some(&bundle.capability)
            {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::RecipeBundle,
                    format!("recipe_bundles.{}.owner", bundle.id),
                    format!(
                        "recipe bundle owner {} does not reference capability {}",
                        bundle.owner, bundle.capability
                    ),
                );
            }
            self.duplicate_members(
                &bundle.recipes,
                format!("recipe_bundles.{}.recipes", bundle.id),
            );
            for recipe in &bundle.recipes {
                owners.entry(recipe).or_default().push(bundle);
                if !self.recipe_ids.contains(recipe) {
                    self.push(
                        ValidationPhase::CanonicalCapability,
                        ValidationFailure::RecipeBundle,
                        format!("recipe_bundles.{}.recipes", bundle.id),
                        format!("recipe bundle has dangling recipe {recipe}"),
                    );
                }
            }
        }
        for recipe in &self.manifest.recipes {
            let recipe_owners = owners.get(&recipe.id).map_or(&[][..], Vec::as_slice);
            if recipe_owners.len() != 1 || recipe_owners[0].capability != recipe.bundle_capability {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::RecipeBundle,
                    format!("recipes.{}.bundle_capability", recipe.id),
                    format!("recipe {} must have exactly one bundle owner", recipe.id),
                );
            }
            if self
                .manifest
                .capabilities
                .iter()
                .any(|capability| capability.id.as_str() == recipe.id.as_str())
            {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::RecipeBundle,
                    format!("recipes.{}", recipe.id),
                    format!(
                        "recipe {} may not have a per-recipe research node",
                        recipe.id
                    ),
                );
            }
            let station_requirement = self
                .manifest
                .stations
                .iter()
                .find(|station| station.id == recipe.station)
                .and_then(|station| station.canonical_capability.required_id());
            if station_requirement.is_none() {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::RecipeBundle,
                    format!("recipes.{}.station", recipe.id),
                    format!("recipe {} station has no capability reference", recipe.id),
                );
            }
            if self
                .manifest
                .stations
                .iter()
                .find(|station| station.id == recipe.station)
                .is_some_and(|station| recipe.station_tier < station.min_tier)
            {
                self.push(
                    ValidationPhase::CanonicalCapability,
                    ValidationFailure::RecipeBundle,
                    format!("recipes.{}.station_tier", recipe.id),
                    format!("recipe {} is below its station tier", recipe.id),
                );
            }
            for ingredient in &recipe.ingredients {
                if self
                    .manifest
                    .all_content()
                    .iter()
                    .find(|record| record.content_id == &ingredient.content_id)
                    .is_none()
                {
                    continue;
                }
            }
        }
    }
}

fn cycle_from<'a>(
    id: &'a CapabilityId,
    graph: &BTreeMap<&'a CapabilityId, &'a Vec<CapabilityId>>,
    visiting: &mut BTreeSet<CapabilityId>,
    completed: &mut BTreeSet<CapabilityId>,
) -> bool {
    if completed.contains(id) {
        return false;
    }
    if !visiting.insert(id.clone()) {
        return true;
    }
    if let Some(prerequisites) = graph.get(id) {
        for prerequisite in *prerequisites {
            if cycle_from(prerequisite, graph, visiting, completed) {
                return true;
            }
        }
    }
    visiting.remove(id);
    completed.insert(id.clone());
    false
}
