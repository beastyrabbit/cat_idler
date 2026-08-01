//! Bounded LAI.68 art-key lookup for assets that are present in this tree.
//!
//! The canonical simulation manifest intentionally describes more art than is
//! delivered yet. This resolver is therefore a positive allow-list: an art
//! key resolves only when its exact, category-compatible file is present.
//! Callers must keep an unknown key absent rather than substituting a nearby
//! building, a generic resource, or an invented state.

use super::recipe_art_assets::resolve_recipe_art_key;

/// Renderer-relevant category for a delivered art asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lai68ArtCategory {
    /// A public ten-band hunting-lair world sprite.
    LairBand,
    /// A report-gated portrait for one exact canonical Lair creature.
    CreaturePortrait,
    /// A non-lair world site, such as a Quarry cave entrance.
    Site,
    /// A complete station/building sprite.
    Building,
    /// One exact construction, activity, or blocked state of a building.
    BuildingState,
    /// The base of the separately layered Hole composition.
    HoleBase,
    /// A compact resource/item icon, not a world resource source.
    ResourceIcon,
    /// One exact named Hunting drop shown through its manifest art key.
    MaterialIcon,
    /// One exact typed raw, prepared, feast, or divine food icon.
    FoodIcon,
    /// One exact canonical production recipe icon.
    RecipeIcon,
    /// One exact item-definition silhouette shown in inventory/detail UI.
    ItemIcon,
    /// One exact station or Hole fixture icon.
    FixtureIcon,
    /// One exact item augmentation icon.
    AugmentationIcon,
    /// A physical storage container sprite.
    Container,
    /// A crop growth stage.
    CropStage,
    /// An Apple-bearing tree state overlay.
    AppleState,
    /// A transport vehicle or endpoint sprite.
    Transport,
}

/// A verified asset-root-relative image and its native dimensions.
///
/// Dimensions are source-image pixels, never a client-side scale decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai68ArtAsset {
    pub key: &'static str,
    pub path: &'static str,
    pub native_width_px: u16,
    pub native_height_px: u16,
    pub category: Lai68ArtCategory,
}

impl Lai68ArtAsset {
    const fn new(
        key: &'static str,
        path: &'static str,
        native_width_px: u16,
        native_height_px: u16,
        category: Lai68ArtCategory,
    ) -> Self {
        Self {
            key,
            path,
            native_width_px,
            native_height_px,
            category,
        }
    }
}

const LAIR_01_10: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_01_10",
    "assets/planned/lairs/art_lair_visual_01_10.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_11_20: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_11_20",
    "assets/planned/lairs/art_lair_visual_11_20.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_21_30: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_21_30",
    "assets/planned/lairs/art_lair_visual_21_30.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_31_40: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_31_40",
    "assets/planned/lairs/art_lair_visual_31_40.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_41_50: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_41_50",
    "assets/planned/lairs/art_lair_visual_41_50.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_51_60: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_51_60",
    "assets/planned/lairs/art_lair_visual_51_60.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_61_70: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_61_70",
    "assets/planned/lairs/art_lair_visual_61_70.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_71_80: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_71_80",
    "assets/planned/lairs/art_lair_visual_71_80.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_81_90: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_81_90",
    "assets/planned/lairs/art_lair_visual_81_90.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const LAIR_91_100: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_lair_visual_91_100",
    "assets/planned/lairs/art_lair_visual_91_100.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const ENCOUNTER_01_19: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_encounter_band_01_19",
    "assets/planned/lairs/art_encounter_band_01_19.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const ENCOUNTER_20_39: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_encounter_band_20_39",
    "assets/planned/lairs/art_encounter_band_20_39.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const ENCOUNTER_40_59: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_encounter_band_40_59",
    "assets/planned/lairs/art_encounter_band_40_59.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const ENCOUNTER_60_79: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_encounter_band_60_79",
    "assets/planned/lairs/art_encounter_band_60_79.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const ENCOUNTER_80_94: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_encounter_band_80_94",
    "assets/planned/lairs/art_encounter_band_80_94.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);
const ENCOUNTER_95_100: Lai68ArtAsset = Lai68ArtAsset::new(
    "art_encounter_band_95_100",
    "assets/planned/lairs/art_encounter_band_95_100.png",
    80,
    80,
    Lai68ArtCategory::LairBand,
);

/// Returns a delivered asset only for an exact canonical or documented state
/// key. Unknown, planned-only, and category-ambiguous keys return [`None`].
pub fn resolve_lai68_art_key(key: &str) -> Option<Lai68ArtAsset> {
    let asset = match key {
        "art_lair_visual_01_10" => LAIR_01_10,
        "art_lair_visual_11_20" => LAIR_11_20,
        "art_lair_visual_21_30" => LAIR_21_30,
        "art_lair_visual_31_40" => LAIR_31_40,
        "art_lair_visual_41_50" => LAIR_41_50,
        "art_lair_visual_51_60" => LAIR_51_60,
        "art_lair_visual_61_70" => LAIR_61_70,
        "art_lair_visual_71_80" => LAIR_71_80,
        "art_lair_visual_81_90" => LAIR_81_90,
        "art_lair_visual_91_100" => LAIR_91_100,
        "art_encounter_band_01_19" => ENCOUNTER_01_19,
        "art_encounter_band_20_39" => ENCOUNTER_20_39,
        "art_encounter_band_40_59" => ENCOUNTER_40_59,
        "art_encounter_band_60_79" => ENCOUNTER_60_79,
        "art_encounter_band_80_94" => ENCOUNTER_80_94,
        "art_encounter_band_95_100" => ENCOUNTER_95_100,
        "art_creature_cave_bat" => creature("art_creature_cave_bat"),
        "art_creature_red_fox" => creature("art_creature_red_fox"),
        "art_creature_badger" => creature("art_creature_badger"),
        "art_creature_wild_boar" => creature("art_creature_wild_boar"),
        "art_creature_gray_wolf" => creature("art_creature_gray_wolf"),
        "art_creature_lynx" => creature("art_creature_lynx"),
        "art_creature_great_stag" => creature("art_creature_great_stag"),
        "art_creature_giant_serpent" => creature("art_creature_giant_serpent"),
        "art_creature_brown_bear" => creature("art_creature_brown_bear"),
        "art_creature_great_eagle" => creature("art_creature_great_eagle"),
        "art_creature_moon_stag" => creature("art_creature_moon_stag"),
        "art_creature_warg" => creature("art_creature_warg"),
        "art_creature_cockatrice" => creature("art_creature_cockatrice"),
        "art_creature_forest_troll" => creature("art_creature_forest_troll"),
        "art_creature_griffin" => creature("art_creature_griffin"),
        "art_creature_basilisk" => creature("art_creature_basilisk"),
        "art_creature_manticore" => creature("art_creature_manticore"),
        "art_creature_chimera" => creature("art_creature_chimera"),
        "art_creature_wyvern" => creature("art_creature_wyvern"),
        "art_creature_elder_dragon" => creature("art_creature_elder_dragon"),
        "art_material_bat_wing" => material("art_material_bat_wing"),
        "art_material_fox_pelt" => material("art_material_fox_pelt"),
        "art_material_badger_pelt" => material("art_material_badger_pelt"),
        "art_material_boar_tusk" => material("art_material_boar_tusk"),
        "art_material_wolf_pelt" => material("art_material_wolf_pelt"),
        "art_material_lynx_pelt" => material("art_material_lynx_pelt"),
        "art_material_stag_antler" => material("art_material_stag_antler"),
        "art_material_serpent_scale" => material("art_material_serpent_scale"),
        "art_material_bear_pelt" => material("art_material_bear_pelt"),
        "art_material_eagle_feather" => material("art_material_eagle_feather"),
        "art_material_moon_antler" => material("art_material_moon_antler"),
        "art_material_warg_fang" => material("art_material_warg_fang"),
        "art_material_cockatrice_eye" => material("art_material_cockatrice_eye"),
        "art_material_troll_hide" => material("art_material_troll_hide"),
        "art_material_griffin_plume" => material("art_material_griffin_plume"),
        "art_material_basilisk_scale" => material("art_material_basilisk_scale"),
        "art_material_manticore_barb" => material("art_material_manticore_barb"),
        "art_material_beast_core" => material("art_material_beast_core"),
        "art_material_wyvern_membrane" => material("art_material_wyvern_membrane"),
        "art_material_dragon_heart" => material("art_material_dragon_heart"),
        "art_food_water" => food("art_food_water"),
        "art_food_apple" => food("art_food_apple"),
        "art_food_raw_fish" => food("art_food_raw_fish"),
        "art_food_raw_meat" => food("art_food_raw_meat"),
        "art_food_catnip" => food("art_food_catnip"),
        "art_food_brew" => food("art_food_brew"),
        "art_food_baked_apples" => food("art_food_baked_apples"),
        "art_food_grilled_fish" => food("art_food_grilled_fish"),
        "art_food_roasted_meat" => food("art_food_roasted_meat"),
        "art_food_flatbread" => food("art_food_flatbread"),
        "art_food_apple_porridge" => food("art_food_apple_porridge"),
        "art_food_fish_stew" => food("art_food_fish_stew"),
        "art_food_meat_stew" => food("art_food_meat_stew"),
        "art_food_apple_preserves" => food("art_food_apple_preserves"),
        "art_food_smoked_fish" => food("art_food_smoked_fish"),
        "art_food_dried_meat" => food("art_food_dried_meat"),
        "art_food_apple_tart" => food("art_food_apple_tart"),
        "art_food_herb_crusted_fish" => food("art_food_herb_crusted_fish"),
        "art_food_meat_pie" => food("art_food_meat_pie"),
        "art_food_surf_and_turf" => food("art_food_surf_and_turf"),
        "art_food_travel_rations" => food("art_food_travel_rations"),
        "art_food_festival_cake" => food("art_food_festival_cake"),
        "art_food_hunters_feast" => food("art_food_hunters_feast"),
        "art_food_grand_lair_feast" => food("art_food_grand_lair_feast"),
        "art_food_divine_ration" => food("art_food_divine_ration"),
        "art_food_divine_water" => food("art_food_divine_water"),
        "art_item_basket" => item("art_item_basket"),
        "art_item_chest" => item("art_item_chest"),
        "art_item_rack" => item("art_item_rack"),
        "art_item_fishing_rod" => item("art_item_fishing_rod"),
        "art_item_lens" => item("art_item_lens"),
        "art_item_microscope" => item("art_item_microscope"),
        "art_item_advanced_instrument" => item("art_item_advanced_instrument"),
        "art_item_weapon" => item("art_item_weapon"),
        "art_item_armor" => item("art_item_armor"),
        "art_item_treated_pelt_clothing" => item("art_item_treated_pelt_clothing"),
        "art_item_membrane_clothing" => item("art_item_membrane_clothing"),
        "art_item_mug" => item("art_item_mug"),
        "art_item_bowl" => item("art_item_bowl"),
        "art_item_furniture" => item("art_item_furniture"),
        "art_item_generic_tool" => item("art_item_generic_tool"),
        "art_item_trinket" => item("art_item_trinket"),
        "art_item_toy" => item("art_item_toy"),
        "art_item_brick" => item("art_item_brick"),
        "art_fixture_cookhouse" => small_icon(
            "art_fixture_cookhouse",
            "assets/planned/content/art_fixture_cookhouse.png",
            Lai68ArtCategory::FixtureIcon,
        ),
        "art_fixture_fishing_hut" => small_icon(
            "art_fixture_fishing_hut",
            "assets/planned/content/art_fixture_fishing_hut.png",
            Lai68ArtCategory::FixtureIcon,
        ),
        "art_fixture_workshop" => small_icon(
            "art_fixture_workshop",
            "assets/planned/content/art_fixture_workshop.png",
            Lai68ArtCategory::FixtureIcon,
        ),
        "art_fixture_research" => small_icon(
            "art_fixture_research",
            "assets/planned/content/art_fixture_research.png",
            Lai68ArtCategory::FixtureIcon,
        ),
        "art_fixture_storage" => small_icon(
            "art_fixture_storage",
            "assets/planned/content/art_fixture_storage.png",
            Lai68ArtCategory::FixtureIcon,
        ),
        "art_fixture_black_hole" => small_icon(
            "art_fixture_black_hole",
            "assets/planned/content/art_fixture_black_hole.png",
            Lai68ArtCategory::FixtureIcon,
        ),
        "art_augmentation_weapon" => small_icon(
            "art_augmentation_weapon",
            "assets/planned/content/art_augmentation_weapon.png",
            Lai68ArtCategory::AugmentationIcon,
        ),
        "art_augmentation_armor" => small_icon(
            "art_augmentation_armor",
            "assets/planned/content/art_augmentation_armor.png",
            Lai68ArtCategory::AugmentationIcon,
        ),
        "art_augmentation_tool" => small_icon(
            "art_augmentation_tool",
            "assets/planned/content/art_augmentation_tool.png",
            Lai68ArtCategory::AugmentationIcon,
        ),
        "art_augmentation_research" => small_icon(
            "art_augmentation_research",
            "assets/planned/content/art_augmentation_research.png",
            Lai68ArtCategory::AugmentationIcon,
        ),
        "art_site_quarry" => Lai68ArtAsset::new(
            "art_site_quarry",
            "public/images/game/sites/quarry.png",
            32,
            32,
            Lai68ArtCategory::Site,
        ),
        "art_station_black_hole" => Lai68ArtAsset::new(
            "art_station_black_hole",
            "public/images/game/buildings/black-hole/base.png",
            80,
            80,
            Lai68ArtCategory::HoleBase,
        ),
        "art_station_workshop" => Lai68ArtAsset::new(
            "art_station_workshop",
            "public/images/game/buildings/workshop.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_mill" => Lai68ArtAsset::new(
            "art_station_mill",
            "public/images/game/buildings/mill.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_tannery" => Lai68ArtAsset::new(
            "art_station_tannery",
            "public/images/game/buildings/tannery.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_clothier" => Lai68ArtAsset::new(
            "art_station_clothier",
            "public/images/game/buildings/clothier.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_woodworking" => Lai68ArtAsset::new(
            "art_station_woodworking",
            "public/images/game/buildings/woodworking.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_smithy" => Lai68ArtAsset::new(
            "art_station_smithy",
            "public/images/game/buildings/smithy.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_research_hut" => Lai68ArtAsset::new(
            "art_station_research_hut",
            "public/images/game/buildings/research_hut.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_school" => Lai68ArtAsset::new(
            "art_station_school",
            "public/images/game/buildings/school.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_wood_cutter" => Lai68ArtAsset::new(
            "art_station_wood_cutter",
            "public/images/game/buildings/wood_cutter.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_stone_prep" => Lai68ArtAsset::new(
            "art_station_stone_prep",
            "public/images/game/buildings/stone_prep.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_cookhouse" => Lai68ArtAsset::new(
            "art_station_cookhouse",
            "assets/planned/stations/art_station_cookhouse.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_fishing_hut" => Lai68ArtAsset::new(
            "art_station_fishing_hut",
            "assets/planned/stations/art_station_fishing_hut.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_sawmill" => Lai68ArtAsset::new(
            "art_station_sawmill",
            "assets/planned/stations/art_station_sawmill.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_smelter" => Lai68ArtAsset::new(
            "art_station_smelter",
            "assets/planned/stations/art_station_smelter.png",
            48,
            48,
            Lai68ArtCategory::Building,
        ),
        "art_station_cookhouse_scaffold" => building_state(
            "art_station_cookhouse_scaffold",
            "assets/planned/cookhouse/art_station_cookhouse_scaffold.png",
        ),
        "art_station_cookhouse_structure" => building_state(
            "art_station_cookhouse_structure",
            "assets/planned/cookhouse/art_station_cookhouse_structure.png",
        ),
        "art_station_cookhouse_fit_out" => building_state(
            "art_station_cookhouse_fit_out",
            "assets/planned/cookhouse/art_station_cookhouse_fit_out.png",
        ),
        "art_station_cookhouse_idle" => building_state(
            "art_station_cookhouse_idle",
            "assets/planned/cookhouse/art_station_cookhouse_idle.png",
        ),
        "art_station_cookhouse_working" => building_state(
            "art_station_cookhouse_working",
            "assets/planned/cookhouse/art_station_cookhouse_working.png",
        ),
        "art_station_cookhouse_blocked" => building_state(
            "art_station_cookhouse_blocked",
            "assets/planned/cookhouse/art_station_cookhouse_blocked.png",
        ),
        "art_station_fishing_hut_idle_north" => building_state(
            "art_station_fishing_hut_idle_north",
            "assets/planned/fishing_hut/art_station_fishing_hut_idle_north.png",
        ),
        "art_station_fishing_hut_idle_east" => building_state(
            "art_station_fishing_hut_idle_east",
            "assets/planned/fishing_hut/art_station_fishing_hut_idle_east.png",
        ),
        "art_station_fishing_hut_idle_south" => building_state(
            "art_station_fishing_hut_idle_south",
            "assets/planned/fishing_hut/art_station_fishing_hut_idle_south.png",
        ),
        "art_station_fishing_hut_idle_west" => building_state(
            "art_station_fishing_hut_idle_west",
            "assets/planned/fishing_hut/art_station_fishing_hut_idle_west.png",
        ),
        "art_station_fishing_hut_working_north" => building_state(
            "art_station_fishing_hut_working_north",
            "assets/planned/fishing_hut/art_station_fishing_hut_working_north.png",
        ),
        "art_station_fishing_hut_working_east" => building_state(
            "art_station_fishing_hut_working_east",
            "assets/planned/fishing_hut/art_station_fishing_hut_working_east.png",
        ),
        "art_station_fishing_hut_working_south" => building_state(
            "art_station_fishing_hut_working_south",
            "assets/planned/fishing_hut/art_station_fishing_hut_working_south.png",
        ),
        "art_station_fishing_hut_working_west" => building_state(
            "art_station_fishing_hut_working_west",
            "assets/planned/fishing_hut/art_station_fishing_hut_working_west.png",
        ),
        "art_resource_logs" => resource_icon("art_resource_logs"),
        "art_resource_stone" => resource_icon("art_resource_stone"),
        "art_resource_water_source" => resource_icon("art_resource_water_source"),
        "art_resource_apple_tree" => resource_icon("art_resource_apple_tree"),
        "art_resource_fish_habitat" => resource_icon("art_resource_fish_habitat"),
        "art_resource_grain" => resource_icon("art_resource_grain"),
        "art_resource_flour" => resource_icon("art_resource_flour"),
        "art_resource_herbs" => resource_icon("art_resource_herbs"),
        "art_resource_clay" => resource_icon("art_resource_clay"),
        "art_resource_fuel" => resource_icon("art_resource_fuel"),
        "art_resource_lumber" => resource_icon("art_resource_lumber"),
        "art_resource_planks" => resource_icon("art_resource_planks"),
        "art_resource_blocks" => resource_icon("art_resource_blocks"),
        "art_resource_fibre" => resource_icon("art_resource_fibre"),
        "art_resource_thread" => resource_icon("art_resource_thread"),
        "art_resource_cloth" => resource_icon("art_resource_cloth"),
        "art_resource_hide" => resource_icon("art_resource_hide"),
        "art_resource_bone" => resource_icon("art_resource_bone"),
        "art_resource_leather" => resource_icon("art_resource_leather"),
        "art_resource_ore" => resource_icon("art_resource_ore"),
        "art_resource_metal" => resource_icon("art_resource_metal"),
        "art_resource_gem" => resource_icon("art_resource_gem"),
        "art_resource_sand" => resource_icon("art_resource_sand"),
        "art_resource_refined" => resource_icon("art_resource_refined"),
        "art_resource_medicine" => resource_icon("art_resource_medicine"),
        "art_item_barrel" => container(
            "art_item_barrel",
            "assets/planned/content/art_item_barrel.png",
        ),
        "art_item_crate" => container(
            "art_item_crate",
            "assets/planned/content/art_item_crate.png",
        ),
        "art_crop_sprout" => crop("art_crop_sprout", "public/images/game/farm/crop_sprout.png"),
        "art_crop_growing" => crop(
            "art_crop_growing",
            "public/images/game/farm/crop_growing.png",
        ),
        "art_crop_flowering" => crop(
            "art_crop_flowering",
            "public/images/game/farm/crop_flowering.png",
        ),
        "art_crop_mature" => crop("art_crop_mature", "public/images/game/farm/crop_mature.png"),
        "art_apple_tree_low" => apple(
            "art_apple_tree_low",
            "public/images/game/nature/tree_oak_apples_low.png",
        ),
        "art_apple_tree_mid" => apple(
            "art_apple_tree_mid",
            "public/images/game/nature/tree_oak_apples_mid.png",
        ),
        "art_apple_tree_full" => apple(
            "art_apple_tree_full",
            "public/images/game/nature/tree_oak_apples_full.png",
        ),
        "art_transport_boat" => transport(
            "art_transport_boat",
            "public/images/game/transport/boat.png",
        ),
        "art_transport_dock_land" => transport(
            "art_transport_dock_land",
            "public/images/game/transport/dock_land.png",
        ),
        "art_transport_dock_water" => transport(
            "art_transport_dock_water",
            "public/images/game/transport/dock_water.png",
        ),
        "art_transport_rail_cart" => transport(
            "art_transport_rail_cart",
            "public/images/game/transport/rail_cart.png",
        ),
        _ => return resolve_recipe_art_key(key),
    };
    Some(asset)
}

fn resource_icon(key: &'static str) -> Lai68ArtAsset {
    let path = match key {
        "art_resource_logs" => "assets/planned/content/art_resource_logs.png",
        "art_resource_stone" => "assets/planned/content/art_resource_stone.png",
        "art_resource_water_source" => "assets/planned/content/art_resource_water_source.png",
        "art_resource_apple_tree" => "assets/planned/content/art_resource_apple_tree.png",
        "art_resource_fish_habitat" => "assets/planned/content/art_resource_fish_habitat.png",
        "art_resource_grain" => "assets/planned/content/art_resource_grain.png",
        "art_resource_flour" => "assets/planned/content/art_resource_flour.png",
        "art_resource_herbs" => "assets/planned/content/art_resource_herbs.png",
        "art_resource_clay" => "assets/planned/content/art_resource_clay.png",
        "art_resource_fuel" => "assets/planned/content/art_resource_fuel.png",
        "art_resource_lumber" => "assets/planned/content/art_resource_lumber.png",
        "art_resource_planks" => "assets/planned/content/art_resource_planks.png",
        "art_resource_blocks" => "assets/planned/content/art_resource_blocks.png",
        "art_resource_fibre" => "assets/planned/content/art_resource_fibre.png",
        "art_resource_thread" => "assets/planned/content/art_resource_thread.png",
        "art_resource_cloth" => "assets/planned/content/art_resource_cloth.png",
        "art_resource_hide" => "assets/planned/content/art_resource_hide.png",
        "art_resource_bone" => "assets/planned/content/art_resource_bone.png",
        "art_resource_leather" => "assets/planned/content/art_resource_leather.png",
        "art_resource_ore" => "assets/planned/content/art_resource_ore.png",
        "art_resource_metal" => "assets/planned/content/art_resource_metal.png",
        "art_resource_gem" => "assets/planned/content/art_resource_gem.png",
        "art_resource_sand" => "assets/planned/content/art_resource_sand.png",
        "art_resource_refined" => "assets/planned/content/art_resource_refined.png",
        "art_resource_medicine" => "assets/planned/content/art_resource_medicine.png",
        _ => unreachable!("resource_icon() is private and called only from exact match arms"),
    };
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::ResourceIcon)
}

fn creature(key: &'static str) -> Lai68ArtAsset {
    let path = match key {
        "art_creature_cave_bat" => "assets/planned/portraits/art_creature_cave_bat.png",
        "art_creature_red_fox" => "assets/planned/portraits/art_creature_red_fox.png",
        "art_creature_badger" => "assets/planned/portraits/art_creature_badger.png",
        "art_creature_wild_boar" => "assets/planned/portraits/art_creature_wild_boar.png",
        "art_creature_gray_wolf" => "assets/planned/portraits/art_creature_gray_wolf.png",
        "art_creature_lynx" => "assets/planned/portraits/art_creature_lynx.png",
        "art_creature_great_stag" => "assets/planned/portraits/art_creature_great_stag.png",
        "art_creature_giant_serpent" => "assets/planned/portraits/art_creature_giant_serpent.png",
        "art_creature_brown_bear" => "assets/planned/portraits/art_creature_brown_bear.png",
        "art_creature_great_eagle" => "assets/planned/portraits/art_creature_great_eagle.png",
        "art_creature_moon_stag" => "assets/planned/portraits/art_creature_moon_stag.png",
        "art_creature_warg" => "assets/planned/portraits/art_creature_warg.png",
        "art_creature_cockatrice" => "assets/planned/portraits/art_creature_cockatrice.png",
        "art_creature_forest_troll" => "assets/planned/portraits/art_creature_forest_troll.png",
        "art_creature_griffin" => "assets/planned/portraits/art_creature_griffin.png",
        "art_creature_basilisk" => "assets/planned/portraits/art_creature_basilisk.png",
        "art_creature_manticore" => "assets/planned/portraits/art_creature_manticore.png",
        "art_creature_chimera" => "assets/planned/portraits/art_creature_chimera.png",
        "art_creature_wyvern" => "assets/planned/portraits/art_creature_wyvern.png",
        "art_creature_elder_dragon" => "assets/planned/portraits/art_creature_elder_dragon.png",
        _ => unreachable!("creature() is private and called only from exact match arms"),
    };
    Lai68ArtAsset::new(key, path, 80, 80, Lai68ArtCategory::CreaturePortrait)
}

fn material(key: &'static str) -> Lai68ArtAsset {
    let path = match key {
        "art_material_bat_wing" => "assets/planned/content/art_material_bat_wing.png",
        "art_material_fox_pelt" => "assets/planned/content/art_material_fox_pelt.png",
        "art_material_badger_pelt" => "assets/planned/content/art_material_badger_pelt.png",
        "art_material_boar_tusk" => "assets/planned/content/art_material_boar_tusk.png",
        "art_material_wolf_pelt" => "assets/planned/content/art_material_wolf_pelt.png",
        "art_material_lynx_pelt" => "assets/planned/content/art_material_lynx_pelt.png",
        "art_material_stag_antler" => "assets/planned/content/art_material_stag_antler.png",
        "art_material_serpent_scale" => "assets/planned/content/art_material_serpent_scale.png",
        "art_material_bear_pelt" => "assets/planned/content/art_material_bear_pelt.png",
        "art_material_eagle_feather" => "assets/planned/content/art_material_eagle_feather.png",
        "art_material_moon_antler" => "assets/planned/content/art_material_moon_antler.png",
        "art_material_warg_fang" => "assets/planned/content/art_material_warg_fang.png",
        "art_material_cockatrice_eye" => "assets/planned/content/art_material_cockatrice_eye.png",
        "art_material_troll_hide" => "assets/planned/content/art_material_troll_hide.png",
        "art_material_griffin_plume" => "assets/planned/content/art_material_griffin_plume.png",
        "art_material_basilisk_scale" => "assets/planned/content/art_material_basilisk_scale.png",
        "art_material_manticore_barb" => "assets/planned/content/art_material_manticore_barb.png",
        "art_material_beast_core" => "assets/planned/content/art_material_beast_core.png",
        "art_material_wyvern_membrane" => "assets/planned/content/art_material_wyvern_membrane.png",
        "art_material_dragon_heart" => "assets/planned/content/art_material_dragon_heart.png",
        _ => unreachable!("material() is private and called only from exact match arms"),
    };
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::MaterialIcon)
}

fn food(key: &'static str) -> Lai68ArtAsset {
    let path = match key {
        "art_food_water" => "assets/planned/content/art_food_water.png",
        "art_food_apple" => "assets/planned/content/art_food_apple.png",
        "art_food_raw_fish" => "assets/planned/content/art_food_raw_fish.png",
        "art_food_raw_meat" => "assets/planned/content/art_food_raw_meat.png",
        "art_food_catnip" => "assets/planned/content/art_food_catnip.png",
        "art_food_brew" => "assets/planned/content/art_food_brew.png",
        "art_food_baked_apples" => "assets/planned/content/art_food_baked_apples.png",
        "art_food_grilled_fish" => "assets/planned/content/art_food_grilled_fish.png",
        "art_food_roasted_meat" => "assets/planned/content/art_food_roasted_meat.png",
        "art_food_flatbread" => "assets/planned/content/art_food_flatbread.png",
        "art_food_apple_porridge" => "assets/planned/content/art_food_apple_porridge.png",
        "art_food_fish_stew" => "assets/planned/content/art_food_fish_stew.png",
        "art_food_meat_stew" => "assets/planned/content/art_food_meat_stew.png",
        "art_food_apple_preserves" => "assets/planned/content/art_food_apple_preserves.png",
        "art_food_smoked_fish" => "assets/planned/content/art_food_smoked_fish.png",
        "art_food_dried_meat" => "assets/planned/content/art_food_dried_meat.png",
        "art_food_apple_tart" => "assets/planned/content/art_food_apple_tart.png",
        "art_food_herb_crusted_fish" => "assets/planned/content/art_food_herb_crusted_fish.png",
        "art_food_meat_pie" => "assets/planned/content/art_food_meat_pie.png",
        "art_food_surf_and_turf" => "assets/planned/content/art_food_surf_and_turf.png",
        "art_food_travel_rations" => "assets/planned/content/art_food_travel_rations.png",
        "art_food_festival_cake" => "assets/planned/content/art_food_festival_cake.png",
        "art_food_hunters_feast" => "assets/planned/content/art_food_hunters_feast.png",
        "art_food_grand_lair_feast" => "assets/planned/content/art_food_grand_lair_feast.png",
        "art_food_divine_ration" => "assets/planned/content/art_food_divine_ration.png",
        "art_food_divine_water" => "assets/planned/content/art_food_divine_water.png",
        _ => unreachable!("food() is private and called only from exact match arms"),
    };
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::FoodIcon)
}

fn item(key: &'static str) -> Lai68ArtAsset {
    let path = match key {
        "art_item_basket" => "assets/planned/content/art_item_basket.png",
        "art_item_chest" => "assets/planned/content/art_item_chest.png",
        "art_item_rack" => "assets/planned/content/art_item_rack.png",
        "art_item_fishing_rod" => "assets/planned/content/art_item_fishing_rod.png",
        "art_item_lens" => "assets/planned/content/art_item_lens.png",
        "art_item_microscope" => "assets/planned/content/art_item_microscope.png",
        "art_item_advanced_instrument" => "assets/planned/content/art_item_advanced_instrument.png",
        "art_item_weapon" => "assets/planned/content/art_item_weapon.png",
        "art_item_armor" => "assets/planned/content/art_item_armor.png",
        "art_item_treated_pelt_clothing" => {
            "assets/planned/content/art_item_treated_pelt_clothing.png"
        }
        "art_item_membrane_clothing" => "assets/planned/content/art_item_membrane_clothing.png",
        "art_item_mug" => "assets/planned/content/art_item_mug.png",
        "art_item_bowl" => "assets/planned/content/art_item_bowl.png",
        "art_item_furniture" => "assets/planned/content/art_item_furniture.png",
        "art_item_generic_tool" => "assets/planned/content/art_item_generic_tool.png",
        "art_item_trinket" => "assets/planned/content/art_item_trinket.png",
        "art_item_toy" => "assets/planned/content/art_item_toy.png",
        "art_item_brick" => "assets/planned/content/art_item_brick.png",
        _ => unreachable!("item() is private and called only from exact match arms"),
    };
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::ItemIcon)
}

fn container(key: &'static str, path: &'static str) -> Lai68ArtAsset {
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::Container)
}

fn crop(key: &'static str, path: &'static str) -> Lai68ArtAsset {
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::CropStage)
}

fn apple(key: &'static str, path: &'static str) -> Lai68ArtAsset {
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::AppleState)
}

fn transport(key: &'static str, path: &'static str) -> Lai68ArtAsset {
    Lai68ArtAsset::new(key, path, 16, 16, Lai68ArtCategory::Transport)
}

fn building_state(key: &'static str, path: &'static str) -> Lai68ArtAsset {
    Lai68ArtAsset::new(key, path, 48, 48, Lai68ArtCategory::BuildingState)
}

fn small_icon(key: &'static str, path: &'static str, category: Lai68ArtCategory) -> Lai68ArtAsset {
    Lai68ArtAsset::new(key, path, 32, 32, category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_exact_lair_band_with_native_size() {
        let asset = resolve_lai68_art_key("art_lair_visual_51_60").expect("delivered lair band");
        assert_eq!(asset.path, "assets/planned/lairs/art_lair_visual_51_60.png");
        assert_eq!((asset.native_width_px, asset.native_height_px), (80, 80));
        assert_eq!(asset.category, Lai68ArtCategory::LairBand);
    }

    #[test]
    fn resolves_coarse_encounter_art_without_aliasing_exact_visual_bands() {
        let encounter =
            resolve_lai68_art_key("art_encounter_band_60_79").expect("delivered encounter band");
        let visual = resolve_lai68_art_key("art_lair_visual_61_70").expect("delivered visual band");
        assert_eq!(
            (encounter.native_width_px, encounter.native_height_px),
            (80, 80)
        );
        assert_eq!(encounter.category, Lai68ArtCategory::LairBand);
        assert_ne!(encounter.key, visual.key);
        assert_ne!(encounter.path, visual.path);
    }

    #[test]
    fn refuses_planned_or_ambiguous_art() {
        assert!(resolve_lai68_art_key("art_station_unknown").is_none());
        assert!(resolve_lai68_art_key("art_item_unknown").is_none());
        assert!(resolve_lai68_art_key("art_resource_unknown").is_none());
        assert!(resolve_lai68_art_key("art_food_generic").is_none());
    }

    #[test]
    fn resolves_only_exact_cookhouse_and_fishing_hut_states() {
        let blocked = resolve_lai68_art_key("art_station_cookhouse_blocked")
            .expect("delivered Cookhouse state");
        let fishing = resolve_lai68_art_key("art_station_fishing_hut_working_west")
            .expect("delivered Fishing Hut state");
        assert_eq!(blocked.category, Lai68ArtCategory::BuildingState);
        assert_eq!(fishing.category, Lai68ArtCategory::BuildingState);
        assert_eq!(
            (blocked.native_width_px, blocked.native_height_px),
            (48, 48)
        );
        assert_eq!(
            (fishing.native_width_px, fishing.native_height_px),
            (48, 48)
        );
        assert_ne!(blocked.path, fishing.path);
    }

    #[test]
    fn resolves_each_report_gated_creature_to_a_unique_portrait() {
        let fox = resolve_lai68_art_key("art_creature_red_fox").expect("delivered portrait");
        let dragon =
            resolve_lai68_art_key("art_creature_elder_dragon").expect("delivered portrait");
        assert_eq!(fox.category, Lai68ArtCategory::CreaturePortrait);
        assert_eq!((fox.native_width_px, fox.native_height_px), (80, 80));
        assert_ne!(fox.path, dragon.path);
    }

    #[test]
    fn resolves_named_drop_icons_without_generic_material_fallback() {
        let scale =
            resolve_lai68_art_key("art_material_basilisk_scale").expect("delivered material");
        let membrane =
            resolve_lai68_art_key("art_material_wyvern_membrane").expect("delivered material");
        assert_eq!(scale.category, Lai68ArtCategory::MaterialIcon);
        assert_eq!((scale.native_width_px, scale.native_height_px), (16, 16));
        assert_ne!(scale.path, membrane.path);
        assert!(resolve_lai68_art_key("art_material_unknown_drop").is_none());
    }

    #[test]
    fn preserves_native_resource_icon_dimensions() {
        let stone = resolve_lai68_art_key("art_resource_stone").expect("delivered resource icon");
        let cloth = resolve_lai68_art_key("art_resource_cloth").expect("delivered cloth icon");
        let hide = resolve_lai68_art_key("art_resource_hide").expect("delivered hide icon");
        assert_eq!((stone.native_width_px, stone.native_height_px), (16, 16));
        assert_eq!((cloth.native_width_px, cloth.native_height_px), (16, 16));
        assert_eq!((hide.native_width_px, hide.native_height_px), (16, 16));
        assert!(stone.path.starts_with("assets/planned/content/"));
    }

    #[test]
    fn resolves_each_typed_food_without_generic_food_fallback() {
        let raw = resolve_lai68_art_key("art_food_raw_fish").expect("delivered raw food");
        let feast =
            resolve_lai68_art_key("art_food_grand_lair_feast").expect("delivered feast food");
        assert_eq!(raw.category, Lai68ArtCategory::FoodIcon);
        assert_eq!((raw.native_width_px, raw.native_height_px), (16, 16));
        assert_ne!(raw.path, feast.path);
        assert!(resolve_lai68_art_key("art_food_generic").is_none());
    }

    #[test]
    fn resolves_exact_item_silhouettes_without_definition_fallback() {
        let rod = resolve_lai68_art_key("art_item_fishing_rod").expect("delivered rod icon");
        let microscope =
            resolve_lai68_art_key("art_item_microscope").expect("delivered microscope icon");
        assert_eq!(rod.category, Lai68ArtCategory::ItemIcon);
        assert_eq!((rod.native_width_px, rod.native_height_px), (16, 16));
        assert_ne!(rod.path, microscope.path);
        assert!(resolve_lai68_art_key("art_item_unknown").is_none());
    }

    #[test]
    fn resolves_fixture_and_augmentation_icons_as_distinct_categories() {
        let fixture =
            resolve_lai68_art_key("art_fixture_black_hole").expect("delivered Hole fixture");
        let augmentation =
            resolve_lai68_art_key("art_augmentation_research").expect("delivered augmentation");
        assert_eq!(fixture.category, Lai68ArtCategory::FixtureIcon);
        assert_eq!(augmentation.category, Lai68ArtCategory::AugmentationIcon);
        assert_eq!(
            (augmentation.native_width_px, augmentation.native_height_px),
            (32, 32)
        );
    }
}
