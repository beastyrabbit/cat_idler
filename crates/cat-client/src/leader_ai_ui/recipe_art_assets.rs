//! Exact LAI.68 allow-list for delivered canonical recipe icons.
//!
//! Keys and paths mirror `cat-sim/src/content_manifest.json`. This module must
//! stay a positive allow-list: a recipe-looking key is not evidence that an
//! asset exists or that it belongs to the canonical content manifest.

use super::art_assets::{Lai68ArtAsset, Lai68ArtCategory};

pub(super) fn resolve_recipe_art_key(key: &str) -> Option<Lai68ArtAsset> {
    match key {
        "art_recipe_apple_porridge" => Some(recipe(
            "art_recipe_apple_porridge",
            "assets/planned/content/art_recipe_apple_porridge.png",
        )),
        "art_recipe_apple_preserves" => Some(recipe(
            "art_recipe_apple_preserves",
            "assets/planned/content/art_recipe_apple_preserves.png",
        )),
        "art_recipe_apple_tart" => Some(recipe(
            "art_recipe_apple_tart",
            "assets/planned/content/art_recipe_apple_tart.png",
        )),
        "art_recipe_baked_apples" => Some(recipe(
            "art_recipe_baked_apples",
            "assets/planned/content/art_recipe_baked_apples.png",
        )),
        "art_recipe_dried_meat" => Some(recipe(
            "art_recipe_dried_meat",
            "assets/planned/content/art_recipe_dried_meat.png",
        )),
        "art_recipe_festival_cake" => Some(recipe(
            "art_recipe_festival_cake",
            "assets/planned/content/art_recipe_festival_cake.png",
        )),
        "art_recipe_fish_stew" => Some(recipe(
            "art_recipe_fish_stew",
            "assets/planned/content/art_recipe_fish_stew.png",
        )),
        "art_recipe_flatbread" => Some(recipe(
            "art_recipe_flatbread",
            "assets/planned/content/art_recipe_flatbread.png",
        )),
        "art_recipe_grand_lair_feast" => Some(recipe(
            "art_recipe_grand_lair_feast",
            "assets/planned/content/art_recipe_grand_lair_feast.png",
        )),
        "art_recipe_grilled_fish" => Some(recipe(
            "art_recipe_grilled_fish",
            "assets/planned/content/art_recipe_grilled_fish.png",
        )),
        "art_recipe_herb_crusted_fish" => Some(recipe(
            "art_recipe_herb_crusted_fish",
            "assets/planned/content/art_recipe_herb_crusted_fish.png",
        )),
        "art_recipe_hunters_feast" => Some(recipe(
            "art_recipe_hunters_feast",
            "assets/planned/content/art_recipe_hunters_feast.png",
        )),
        "art_recipe_meat_pie" => Some(recipe(
            "art_recipe_meat_pie",
            "assets/planned/content/art_recipe_meat_pie.png",
        )),
        "art_recipe_meat_stew" => Some(recipe(
            "art_recipe_meat_stew",
            "assets/planned/content/art_recipe_meat_stew.png",
        )),
        "art_recipe_mill_flour" => Some(recipe(
            "art_recipe_mill_flour",
            "assets/planned/content/art_recipe_mill_flour.png",
        )),
        "art_recipe_roasted_meat" => Some(recipe(
            "art_recipe_roasted_meat",
            "assets/planned/content/art_recipe_roasted_meat.png",
        )),
        "art_recipe_smoked_fish" => Some(recipe(
            "art_recipe_smoked_fish",
            "assets/planned/content/art_recipe_smoked_fish.png",
        )),
        "art_recipe_surf_and_turf" => Some(recipe(
            "art_recipe_surf_and_turf",
            "assets/planned/content/art_recipe_surf_and_turf.png",
        )),
        "art_recipe_travel_rations" => Some(recipe(
            "art_recipe_travel_rations",
            "assets/planned/content/art_recipe_travel_rations.png",
        )),
        "art_recipe_logs_to_lumber" => Some(recipe(
            "art_recipe_logs_to_lumber",
            "assets/planned/recipes/art_recipe_logs_to_lumber.png",
        )),
        "art_recipe_carpentry_quality" => Some(recipe(
            "art_recipe_carpentry_quality",
            "assets/planned/recipes/art_recipe_carpentry_quality.png",
        )),
        "art_recipe_carpentry_masterwork" => Some(recipe(
            "art_recipe_carpentry_masterwork",
            "assets/planned/recipes/art_recipe_carpentry_masterwork.png",
        )),
        "art_recipe_herbal_poultice" => Some(recipe(
            "art_recipe_herbal_poultice",
            "assets/planned/recipes/art_recipe_herbal_poultice.png",
        )),
        "art_recipe_herbal_tonic" => Some(recipe(
            "art_recipe_herbal_tonic",
            "assets/planned/recipes/art_recipe_herbal_tonic.png",
        )),
        "art_recipe_herbal_salve" => Some(recipe(
            "art_recipe_herbal_salve",
            "assets/planned/recipes/art_recipe_herbal_salve.png",
        )),
        "art_recipe_herbal_remedy" => Some(recipe(
            "art_recipe_herbal_remedy",
            "assets/planned/recipes/art_recipe_herbal_remedy.png",
        )),
        "art_recipe_herbal_masterwork_remedy" => Some(recipe(
            "art_recipe_herbal_masterwork_remedy",
            "assets/planned/recipes/art_recipe_herbal_masterwork_remedy.png",
        )),
        "art_recipe_field_craft_preparation" => Some(recipe(
            "art_recipe_field_craft_preparation",
            "assets/planned/recipes/art_recipe_field_craft_preparation.png",
        )),
        "art_recipe_field_craft_staples" => Some(recipe(
            "art_recipe_field_craft_staples",
            "assets/planned/recipes/art_recipe_field_craft_staples.png",
        )),
        "art_recipe_field_craft_quality" => Some(recipe(
            "art_recipe_field_craft_quality",
            "assets/planned/recipes/art_recipe_field_craft_quality.png",
        )),
        "art_recipe_field_craft_specialty" => Some(recipe(
            "art_recipe_field_craft_specialty",
            "assets/planned/recipes/art_recipe_field_craft_specialty.png",
        )),
        "art_recipe_field_craft_masterwork" => Some(recipe(
            "art_recipe_field_craft_masterwork",
            "assets/planned/recipes/art_recipe_field_craft_masterwork.png",
        )),
        "art_recipe_expedition_supplies_preparation" => Some(recipe(
            "art_recipe_expedition_supplies_preparation",
            "assets/planned/recipes/art_recipe_expedition_supplies_preparation.png",
        )),
        "art_recipe_expedition_supplies_staples" => Some(recipe(
            "art_recipe_expedition_supplies_staples",
            "assets/planned/recipes/art_recipe_expedition_supplies_staples.png",
        )),
        "art_recipe_expedition_supplies_quality" => Some(recipe(
            "art_recipe_expedition_supplies_quality",
            "assets/planned/recipes/art_recipe_expedition_supplies_quality.png",
        )),
        "art_recipe_expedition_supplies_specialty" => Some(recipe(
            "art_recipe_expedition_supplies_specialty",
            "assets/planned/recipes/art_recipe_expedition_supplies_specialty.png",
        )),
        "art_recipe_expedition_supplies_masterwork" => Some(recipe(
            "art_recipe_expedition_supplies_masterwork",
            "assets/planned/recipes/art_recipe_expedition_supplies_masterwork.png",
        )),
        "art_recipe_gem_jewelry" => Some(recipe(
            "art_recipe_gem_jewelry",
            "assets/planned/recipes/art_recipe_gem_jewelry.png",
        )),
        "art_recipe_sand_glass_mug" => Some(recipe(
            "art_recipe_sand_glass_mug",
            "assets/planned/recipes/art_recipe_sand_glass_mug.png",
        )),
        "art_recipe_sand_glass_bowl" => Some(recipe(
            "art_recipe_sand_glass_bowl",
            "assets/planned/recipes/art_recipe_sand_glass_bowl.png",
        )),
        "art_recipe_sand_glass_trinket" => Some(recipe(
            "art_recipe_sand_glass_trinket",
            "assets/planned/recipes/art_recipe_sand_glass_trinket.png",
        )),
        "art_recipe_ore_to_metal" => Some(recipe(
            "art_recipe_ore_to_metal",
            "assets/planned/recipes/art_recipe_ore_to_metal.png",
        )),
        "art_recipe_metallurgy_staples" => Some(recipe(
            "art_recipe_metallurgy_staples",
            "assets/planned/recipes/art_recipe_metallurgy_staples.png",
        )),
        "art_recipe_metallurgy_quality" => Some(recipe(
            "art_recipe_metallurgy_quality",
            "assets/planned/recipes/art_recipe_metallurgy_quality.png",
        )),
        "art_recipe_metallurgy_specialty" => Some(recipe(
            "art_recipe_metallurgy_specialty",
            "assets/planned/recipes/art_recipe_metallurgy_specialty.png",
        )),
        "art_recipe_metallurgy_masterwork" => Some(recipe(
            "art_recipe_metallurgy_masterwork",
            "assets/planned/recipes/art_recipe_metallurgy_masterwork.png",
        )),
        "art_recipe_logs_to_planks" => Some(recipe(
            "art_recipe_logs_to_planks",
            "assets/planned/recipes/art_recipe_logs_to_planks.png",
        )),
        "art_recipe_carpentry_specialty" => Some(recipe(
            "art_recipe_carpentry_specialty",
            "assets/planned/recipes/art_recipe_carpentry_specialty.png",
        )),
        "art_recipe_stone_to_blocks" => Some(recipe(
            "art_recipe_stone_to_blocks",
            "assets/planned/recipes/art_recipe_stone_to_blocks.png",
        )),
        "art_recipe_bone_trinket" => Some(recipe(
            "art_recipe_bone_trinket",
            "assets/planned/recipes/art_recipe_bone_trinket.png",
        )),
        "art_recipe_bone_toy" => Some(recipe(
            "art_recipe_bone_toy",
            "assets/planned/recipes/art_recipe_bone_toy.png",
        )),
        "art_recipe_bone_mug" => Some(recipe(
            "art_recipe_bone_mug",
            "assets/planned/recipes/art_recipe_bone_mug.png",
        )),
        "art_recipe_stone_mug" => Some(recipe(
            "art_recipe_stone_mug",
            "assets/planned/recipes/art_recipe_stone_mug.png",
        )),
        "art_recipe_clay_mug" => Some(recipe(
            "art_recipe_clay_mug",
            "assets/planned/recipes/art_recipe_clay_mug.png",
        )),
        "art_recipe_clay_bowl" => Some(recipe(
            "art_recipe_clay_bowl",
            "assets/planned/recipes/art_recipe_clay_bowl.png",
        )),
        "art_recipe_clay_brick" => Some(recipe(
            "art_recipe_clay_brick",
            "assets/planned/recipes/art_recipe_clay_brick.png",
        )),
        "art_recipe_stonecraft_masterwork" => Some(recipe(
            "art_recipe_stonecraft_masterwork",
            "assets/planned/recipes/art_recipe_stonecraft_masterwork.png",
        )),
        "art_recipe_planks_and_blocks_to_tools" => Some(recipe(
            "art_recipe_planks_and_blocks_to_tools",
            "assets/planned/recipes/art_recipe_planks_and_blocks_to_tools.png",
        )),
        "art_recipe_bone_tool" => Some(recipe(
            "art_recipe_bone_tool",
            "assets/planned/recipes/art_recipe_bone_tool.png",
        )),
        "art_recipe_hunting_quality" => Some(recipe(
            "art_recipe_hunting_quality",
            "assets/planned/recipes/art_recipe_hunting_quality.png",
        )),
        "art_recipe_hunting_specialty" => Some(recipe(
            "art_recipe_hunting_specialty",
            "assets/planned/recipes/art_recipe_hunting_specialty.png",
        )),
        "art_recipe_hunting_masterwork" => Some(recipe(
            "art_recipe_hunting_masterwork",
            "assets/planned/recipes/art_recipe_hunting_masterwork.png",
        )),
        "art_recipe_waterworks_preparation" => Some(recipe(
            "art_recipe_waterworks_preparation",
            "assets/planned/recipes/art_recipe_waterworks_preparation.png",
        )),
        "art_recipe_waterworks_staples" => Some(recipe(
            "art_recipe_waterworks_staples",
            "assets/planned/recipes/art_recipe_waterworks_staples.png",
        )),
        "art_recipe_waterworks_quality" => Some(recipe(
            "art_recipe_waterworks_quality",
            "assets/planned/recipes/art_recipe_waterworks_quality.png",
        )),
        "art_recipe_waterworks_specialty" => Some(recipe(
            "art_recipe_waterworks_specialty",
            "assets/planned/recipes/art_recipe_waterworks_specialty.png",
        )),
        "art_recipe_waterworks_masterwork" => Some(recipe(
            "art_recipe_waterworks_masterwork",
            "assets/planned/recipes/art_recipe_waterworks_masterwork.png",
        )),
        "art_recipe_fibre_to_thread" => Some(recipe(
            "art_recipe_fibre_to_thread",
            "assets/planned/recipes/art_recipe_fibre_to_thread.png",
        )),
        "art_recipe_fibre_to_cloth" => Some(recipe(
            "art_recipe_fibre_to_cloth",
            "assets/planned/recipes/art_recipe_fibre_to_cloth.png",
        )),
        "art_recipe_foraging_preparation" => Some(recipe(
            "art_recipe_foraging_preparation",
            "assets/planned/recipes/art_recipe_foraging_preparation.png",
        )),
        "art_recipe_foraging_staples" => Some(recipe(
            "art_recipe_foraging_staples",
            "assets/planned/recipes/art_recipe_foraging_staples.png",
        )),
        "art_recipe_foraging_quality" => Some(recipe(
            "art_recipe_foraging_quality",
            "assets/planned/recipes/art_recipe_foraging_quality.png",
        )),
        "art_recipe_foraging_specialty" => Some(recipe(
            "art_recipe_foraging_specialty",
            "assets/planned/recipes/art_recipe_foraging_specialty.png",
        )),
        "art_recipe_foraging_masterwork" => Some(recipe(
            "art_recipe_foraging_masterwork",
            "assets/planned/recipes/art_recipe_foraging_masterwork.png",
        )),
        "art_recipe_textile_work_preparation" => Some(recipe(
            "art_recipe_textile_work_preparation",
            "assets/planned/recipes/art_recipe_textile_work_preparation.png",
        )),
        "art_recipe_textile_work_staples" => Some(recipe(
            "art_recipe_textile_work_staples",
            "assets/planned/recipes/art_recipe_textile_work_staples.png",
        )),
        "art_recipe_textile_work_quality" => Some(recipe(
            "art_recipe_textile_work_quality",
            "assets/planned/recipes/art_recipe_textile_work_quality.png",
        )),
        "art_recipe_textile_work_specialty" => Some(recipe(
            "art_recipe_textile_work_specialty",
            "assets/planned/recipes/art_recipe_textile_work_specialty.png",
        )),
        "art_recipe_textile_work_masterwork" => Some(recipe(
            "art_recipe_textile_work_masterwork",
            "assets/planned/recipes/art_recipe_textile_work_masterwork.png",
        )),
        "art_recipe_hide_to_leather" => Some(recipe(
            "art_recipe_hide_to_leather",
            "assets/planned/recipes/art_recipe_hide_to_leather.png",
        )),
        "art_recipe_animal_husbandry_preparation" => Some(recipe(
            "art_recipe_animal_husbandry_preparation",
            "assets/planned/recipes/art_recipe_animal_husbandry_preparation.png",
        )),
        "art_recipe_animal_husbandry_staples" => Some(recipe(
            "art_recipe_animal_husbandry_staples",
            "assets/planned/recipes/art_recipe_animal_husbandry_staples.png",
        )),
        "art_recipe_animal_husbandry_quality" => Some(recipe(
            "art_recipe_animal_husbandry_quality",
            "assets/planned/recipes/art_recipe_animal_husbandry_quality.png",
        )),
        "art_recipe_animal_husbandry_specialty" => Some(recipe(
            "art_recipe_animal_husbandry_specialty",
            "assets/planned/recipes/art_recipe_animal_husbandry_specialty.png",
        )),
        "art_recipe_animal_husbandry_masterwork" => Some(recipe(
            "art_recipe_animal_husbandry_masterwork",
            "assets/planned/recipes/art_recipe_animal_husbandry_masterwork.png",
        )),
        "art_recipe_leatherworking_preparation" => Some(recipe(
            "art_recipe_leatherworking_preparation",
            "assets/planned/recipes/art_recipe_leatherworking_preparation.png",
        )),
        "art_recipe_leatherworking_staples" => Some(recipe(
            "art_recipe_leatherworking_staples",
            "assets/planned/recipes/art_recipe_leatherworking_staples.png",
        )),
        "art_recipe_leatherworking_quality" => Some(recipe(
            "art_recipe_leatherworking_quality",
            "assets/planned/recipes/art_recipe_leatherworking_quality.png",
        )),
        "art_recipe_leatherworking_specialty" => Some(recipe(
            "art_recipe_leatherworking_specialty",
            "assets/planned/recipes/art_recipe_leatherworking_specialty.png",
        )),
        "art_recipe_leatherworking_masterwork" => Some(recipe(
            "art_recipe_leatherworking_masterwork",
            "assets/planned/recipes/art_recipe_leatherworking_masterwork.png",
        )),
        "art_recipe_smithy_weapon" => Some(recipe(
            "art_recipe_smithy_weapon",
            "assets/planned/recipes/art_recipe_smithy_weapon.png",
        )),
        "art_recipe_smithy_tool" => Some(recipe(
            "art_recipe_smithy_tool",
            "assets/planned/recipes/art_recipe_smithy_tool.png",
        )),
        "art_recipe_smithy_armor" => Some(recipe(
            "art_recipe_smithy_armor",
            "assets/planned/recipes/art_recipe_smithy_armor.png",
        )),
        "art_recipe_metal_mug" => Some(recipe(
            "art_recipe_metal_mug",
            "assets/planned/recipes/art_recipe_metal_mug.png",
        )),
        "art_recipe_toolmaking_specialty" => Some(recipe(
            "art_recipe_toolmaking_specialty",
            "assets/planned/recipes/art_recipe_toolmaking_specialty.png",
        )),
        "art_recipe_toolmaking_masterwork" => Some(recipe(
            "art_recipe_toolmaking_masterwork",
            "assets/planned/recipes/art_recipe_toolmaking_masterwork.png",
        )),
        "art_recipe_weaponcraft_preparation" => Some(recipe(
            "art_recipe_weaponcraft_preparation",
            "assets/planned/recipes/art_recipe_weaponcraft_preparation.png",
        )),
        "art_recipe_weaponcraft_staples" => Some(recipe(
            "art_recipe_weaponcraft_staples",
            "assets/planned/recipes/art_recipe_weaponcraft_staples.png",
        )),
        "art_recipe_weaponcraft_quality" => Some(recipe(
            "art_recipe_weaponcraft_quality",
            "assets/planned/recipes/art_recipe_weaponcraft_quality.png",
        )),
        "art_recipe_weaponcraft_specialty" => Some(recipe(
            "art_recipe_weaponcraft_specialty",
            "assets/planned/recipes/art_recipe_weaponcraft_specialty.png",
        )),
        "art_recipe_weaponcraft_masterwork" => Some(recipe(
            "art_recipe_weaponcraft_masterwork",
            "assets/planned/recipes/art_recipe_weaponcraft_masterwork.png",
        )),
        "art_recipe_armorcraft_preparation" => Some(recipe(
            "art_recipe_armorcraft_preparation",
            "assets/planned/recipes/art_recipe_armorcraft_preparation.png",
        )),
        "art_recipe_armorcraft_staples" => Some(recipe(
            "art_recipe_armorcraft_staples",
            "assets/planned/recipes/art_recipe_armorcraft_staples.png",
        )),
        "art_recipe_armorcraft_quality" => Some(recipe(
            "art_recipe_armorcraft_quality",
            "assets/planned/recipes/art_recipe_armorcraft_quality.png",
        )),
        "art_recipe_armorcraft_specialty" => Some(recipe(
            "art_recipe_armorcraft_specialty",
            "assets/planned/recipes/art_recipe_armorcraft_specialty.png",
        )),
        "art_recipe_armorcraft_masterwork" => Some(recipe(
            "art_recipe_armorcraft_masterwork",
            "assets/planned/recipes/art_recipe_armorcraft_masterwork.png",
        )),
        "art_recipe_brew_grain_small" => Some(recipe(
            "art_recipe_brew_grain_small",
            "assets/planned/recipes/art_recipe_brew_grain_small.png",
        )),
        "art_recipe_brew_catnip_ale" => Some(recipe(
            "art_recipe_brew_catnip_ale",
            "assets/planned/recipes/art_recipe_brew_catnip_ale.png",
        )),
        "art_recipe_brew_herbal_tonic" => Some(recipe(
            "art_recipe_brew_herbal_tonic",
            "assets/planned/recipes/art_recipe_brew_herbal_tonic.png",
        )),
        "art_recipe_brew_spiced_ale" => Some(recipe(
            "art_recipe_brew_spiced_ale",
            "assets/planned/recipes/art_recipe_brew_spiced_ale.png",
        )),
        "art_recipe_brew_masterwork" => Some(recipe(
            "art_recipe_brew_masterwork",
            "assets/planned/recipes/art_recipe_brew_masterwork.png",
        )),
        _ => None,
    }
}

const fn recipe(key: &'static str, path: &'static str) -> Lai68ArtAsset {
    Lai68ArtAsset {
        key,
        path,
        native_width_px: 16,
        native_height_px: 16,
        category: Lai68ArtCategory::RecipeIcon,
    }
}
