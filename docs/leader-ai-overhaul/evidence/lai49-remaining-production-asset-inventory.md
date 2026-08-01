# LAI.49 remaining production-asset inventory

Date: 2026-07-25

This is the mechanical gap inventory between:

- `crates/cat-sim/src/content_manifest.json`, including its closed
  `art_registry`;
- the positive allow-list in
  `crates/cat-client/src/leader_ai_ui/art_assets.rs`; and
- image files presently available in this worktree and the two integrated
  source worktrees.

The audit reads only the resolver function's real match arms. Test strings such
as “this key must remain unresolved” are deliberately excluded.

## Result and counting boundary

The closed art registry has 263 distinct keys: 247 definition keys, ten
world-facing Lair visual-band keys, and six separate coarse encounter-band
keys. All 263 have an exact delivered image and an exact positive resolver
entry. Two hundred fifty-two exist at their manifest-planned path; eleven
legacy Station keys deliberately resolve to semantically exact existing
project sources at a different path. There are therefore **zero remaining
image-creation or resolver gaps for registered manifest art**, while eleven
planned Station paths still need normalization if planned-path equality is
required.

| Family | Files delivered | Positively resolved | Resolver pending |
| --- | ---: | ---: | ---: |
| Definition art, including Recipe | 247 | 247 | 0 |
| World-facing Lair visual bands | 10 | 10 | 0 |
| Coarse encounter bands | 6 | 6 | 0 |
| **Total closed registry** | **263** | **263** | **0** |

The eleven planned-path normalization rows are
`art_station_black_hole`, `art_station_clothier`, `art_station_mill`,
`art_station_research_hut`, `art_station_school`, `art_station_smithy`,
`art_station_stone_prep`, `art_station_tannery`, `art_station_wood_cutter`,
`art_station_woodworking`, and `art_station_workshop`. Each has a positive
exact-source resolver mapping today; none is an image-generation gap.

The six coarse encounter sprites were generated only after this audit exposed
that their registry entries were not the same as the ten visual bands. Their
prompt, reference, post-processing, inspection contact, hashes, and
non-disclosure boundary are recorded in
`docs/branch-plan-merge/generated-encounter-band-art-receipt.md`.

The twenty-six food keys and eighteen item keys are **not** remaining gaps.
Their delivered files and generation provenance are in
`docs/branch-plan-merge/generated-food-icon-receipt.md` and
`docs/branch-plan-merge/generated-item-icon-receipt.md`. Both production
families now match the authoritative registry at exact `16×16`; their generated
`64×64` masters are preserved under `tmp/imagegen/foods/64px-masters/` and
`tmp/imagegen/items/64px-masters/`. Exact manifest-planned path copies are
present under `assets/planned/content/`.

The same native-size correction is complete for the previously delivered
creature and material families: creature production portraits are exact
registry-native `80×80` with `96×96` masters preserved, and named-material
production icons are exact registry-native `16×16` with `64×64` masters
preserved. Exact-final hashes and contacts are centralized in
`tmp/imagegen/native-size-final-hashes.sha256` and the four generated-asset
receipts.

Every recipe resolver row uses accessibility source `content_name`.

## Delivered item family — excluded from remaining gaps

The complete eighteen-key item family is delivered under
`assets/planned/items/` and positively resolved:

`art_item_advanced_instrument`, `art_item_armor`, `art_item_basket`,
`art_item_bowl`, `art_item_brick`, `art_item_chest`,
`art_item_fishing_rod`, `art_item_furniture`, `art_item_generic_tool`,
`art_item_lens`, `art_item_membrane_clothing`, `art_item_microscope`,
`art_item_mug`, `art_item_rack`, `art_item_toy`,
`art_item_treated_pelt_clothing`, `art_item_trinket`, and
`art_item_weapon`.

Their exact prompt, reference inputs, normalized atlas, contact, dimensions, and
hashes are recorded in
`docs/branch-plan-merge/generated-item-icon-receipt.md`. Do not regenerate
them. The item system should reuse these accepted silhouettes for material
palette layers and recipe composites rather than minting unrelated variants.

`art_item_barrel` and `art_item_crate` remain exact source reuse, copied from
`public/images/game/props/barrel.png` and
`public/images/game/props/crate.png` to their canonical
`assets/planned/content/` paths; they were never generation gaps.

## Batch B — resource icons

Delivered, exact-path, and positively resolved. Nineteen semantically exact
legacy sources were preserved and adapted; the six missing identities—Apple
Tree, Clay, Fuel, Gem, Sand, and Medicine—were generated as one exact atlas.
All twenty-five production icons are registry-native `16×16` under
`assets/planned/content/`.

Source provenance, the verbatim prompt, references, masters, final contact, and
all hashes are recorded in
`docs/branch-plan-merge/generated-resource-icon-receipt.md`.

## Batch C — fixture detail icons

Delivered and excluded from remaining gaps at exact-final `32×32 ui_detail`:
`art_fixture_black_hole`, `art_fixture_cookhouse`,
`art_fixture_fishing_hut`, `art_fixture_research`, `art_fixture_storage`, and
`art_fixture_workshop`.

## Batch D — augmentation detail icons

Delivered and excluded from remaining gaps at exact-final `32×32 ui_detail`:
`art_augmentation_armor`, `art_augmentation_research`,
`art_augmentation_tool`, and `art_augmentation_weapon`.

The exact prompt, retained atlas, `64×64` masters, exact-final contact, output
paths, and all hashes for Batch C/D are recorded in
`docs/branch-plan-merge/generated-fixture-augmentation-icon-receipt.md`.
These are construction fit-out or attachable-detail icons, not station world
sprites or duplicates of whole equipment.

## Batch E — station world bases

Delivered and positively resolved at exact `48×48 world_base`. Cookhouse is a
byte-identical reuse of its accepted idle state; Fishing Hut is a
byte-identical reuse of idle-north; Sawmill and Smelter are distinct generated
buildings. Exact sources, prompt, references, atlas/contact, and hashes are in
`docs/branch-plan-merge/generated-station-base-receipt.md`.

## Batch F — recipe progression emblems

Delivered as fifty-nine exact-key `16×16` recipe files in
`assets/planned/recipes/`, using each recipe's canonical first physical output.
Their filesystem delivery and all fifty-nine positive resolver arms are
complete.

### Masterwork (15)

`art_recipe_animal_husbandry_masterwork`,
`art_recipe_armorcraft_masterwork`, `art_recipe_brew_masterwork`,
`art_recipe_carpentry_masterwork`,
`art_recipe_expedition_supplies_masterwork`,
`art_recipe_field_craft_masterwork`, `art_recipe_foraging_masterwork`,
`art_recipe_hunting_masterwork`, `art_recipe_leatherworking_masterwork`,
`art_recipe_metallurgy_masterwork`, `art_recipe_stonecraft_masterwork`,
`art_recipe_textile_work_masterwork`, `art_recipe_toolmaking_masterwork`,
`art_recipe_waterworks_masterwork`, `art_recipe_weaponcraft_masterwork`.

### Preparation (9)

`art_recipe_animal_husbandry_preparation`,
`art_recipe_armorcraft_preparation`,
`art_recipe_expedition_supplies_preparation`,
`art_recipe_field_craft_preparation`, `art_recipe_foraging_preparation`,
`art_recipe_leatherworking_preparation`,
`art_recipe_textile_work_preparation`,
`art_recipe_waterworks_preparation`,
`art_recipe_weaponcraft_preparation`.

### Quality (12)

`art_recipe_animal_husbandry_quality`, `art_recipe_armorcraft_quality`,
`art_recipe_carpentry_quality`, `art_recipe_expedition_supplies_quality`,
`art_recipe_field_craft_quality`, `art_recipe_foraging_quality`,
`art_recipe_hunting_quality`, `art_recipe_leatherworking_quality`,
`art_recipe_metallurgy_quality`, `art_recipe_textile_work_quality`,
`art_recipe_waterworks_quality`, `art_recipe_weaponcraft_quality`.

### Specialty (13)

`art_recipe_animal_husbandry_specialty`,
`art_recipe_armorcraft_specialty`, `art_recipe_carpentry_specialty`,
`art_recipe_expedition_supplies_specialty`,
`art_recipe_field_craft_specialty`, `art_recipe_foraging_specialty`,
`art_recipe_hunting_specialty`, `art_recipe_leatherworking_specialty`,
`art_recipe_metallurgy_specialty`, `art_recipe_textile_work_specialty`,
`art_recipe_toolmaking_specialty`, `art_recipe_waterworks_specialty`,
`art_recipe_weaponcraft_specialty`.

### Staples (10)

`art_recipe_animal_husbandry_staples`, `art_recipe_armorcraft_staples`,
`art_recipe_expedition_supplies_staples`,
`art_recipe_field_craft_staples`, `art_recipe_foraging_staples`,
`art_recipe_leatherworking_staples`, `art_recipe_metallurgy_staples`,
`art_recipe_textile_work_staples`, `art_recipe_waterworks_staples`,
`art_recipe_weaponcraft_staples`.

## Batch G — concrete recipe icons

Delivered as fifty-two exact-key `16×16` panel/list icons. The registry puts
eighteen meal recipes plus `mill_flour` under `assets/planned/content/`; all
other keys below use `assets/planned/recipes/`. Their filesystem delivery and
all fifty-two positive resolver arms are complete.

### Exact prepared-food output adaptations (18)

`art_recipe_apple_porridge`, `art_recipe_apple_preserves`,
`art_recipe_apple_tart`, `art_recipe_baked_apples`,
`art_recipe_dried_meat`, `art_recipe_festival_cake`,
`art_recipe_fish_stew`, `art_recipe_flatbread`,
`art_recipe_grand_lair_feast`, `art_recipe_grilled_fish`,
`art_recipe_herb_crusted_fish`, `art_recipe_hunters_feast`,
`art_recipe_meat_pie`, `art_recipe_meat_stew`,
`art_recipe_roasted_meat`, `art_recipe_smoked_fish`,
`art_recipe_surf_and_turf`, `art_recipe_travel_rations`.

Each file is byte-identical to its corresponding canonical `art_food_*`
first-output source.

### Brewing identities (4)

`art_recipe_brew_catnip_ale`, `art_recipe_brew_grain_small`,
`art_recipe_brew_herbal_tonic`, `art_recipe_brew_spiced_ale`.

Each file uses the exact canonical `food_brew` first-output pixels.

### Bone, clay, glass, metal, gem, and smithy outputs (16)

`art_recipe_bone_mug`, `art_recipe_bone_tool`, `art_recipe_bone_toy`,
`art_recipe_bone_trinket`, `art_recipe_clay_bowl`,
`art_recipe_clay_brick`, `art_recipe_clay_mug`,
`art_recipe_gem_jewelry`, `art_recipe_metal_mug`,
`art_recipe_sand_glass_bowl`, `art_recipe_sand_glass_mug`,
`art_recipe_sand_glass_trinket`, `art_recipe_smithy_armor`,
`art_recipe_smithy_tool`, `art_recipe_smithy_weapon`,
`art_recipe_stone_mug`.

Each file uses the exact canonical first-output item pixels recorded for that
recipe.

### Resource transformations (9)

`art_recipe_fibre_to_cloth`, `art_recipe_fibre_to_thread`,
`art_recipe_hide_to_leather`, `art_recipe_logs_to_lumber`,
`art_recipe_logs_to_planks`, `art_recipe_mill_flour`,
`art_recipe_ore_to_metal`, `art_recipe_planks_and_blocks_to_tools`,
`art_recipe_stone_to_blocks`.

Each file is byte-identical to the exact canonical first-output resource or item
recorded for that recipe.

### Herbal preparations (5)

`art_recipe_herbal_masterwork_remedy`, `art_recipe_herbal_poultice`,
`art_recipe_herbal_remedy`, `art_recipe_herbal_salve`,
`art_recipe_herbal_tonic`.

Each file uses the canonical `resource_medicine` first-output pixels.

The complete 111-row per-recipe mapping, derivation rule, planned destination,
source path, checksums, and verification evidence are recorded in
`docs/branch-plan-merge/generated-recipe-output-art-receipt.md`,
`tmp/imagegen/recipe-output-art-map.tsv`, and
`tmp/imagegen/recipe-output-art-hashes.sha256`. This is exact output identity,
not a category-level or generic fallback.

## Required visual families that are not canonicalized yet

The integrated plans additionally require quality badges, construction phases,
container fullness, family/enterprise signs, and complete world states. None of
the following proposed keys exists in the manifest `art_registry`, so they are
not included in the now-zero canonical manifest file-gap count and cannot
truthfully be called resolved or unresolved canonical art. They need an
authority/registry card before image generation.

To make that card executable, the following native-size contract is proposed.
The dimensions are explicit production defaults, not a claim about current
manifest authority.

| Required family | Proposed exact native contract | Layer/state | Reuse requirement |
| --- | --- | --- | --- |
| Quality badges | five `32×16` sRGBA badges: Crude, Common, Fine, Superior, Masterwork | panel-detail badge; never a hidden quality inference | No source exists; generate one ordered family, then register exact keys. |
| Container fullness | Basket/Barrel/Crate/Chest/Rack × empty/partial/full, each `16×16` | world object state and Stores panel icon | Reuse existing `barrel.png` and `crate.png` silhouettes; use the delivered Basket/Chest/Rack bases. |
| Construction states | scaffold, partial structure, fit-out, operational at `48×48` per world building | `world_base` state selected from authoritative construction phase | Reuse every existing operational building sprite. Cookhouse already has all required phase states. Generate/adapt only missing phases. |
| Family-enterprise sign | one `16×16` sign overlay plus registered profession/lineage emblem variants | world overlay; enterprise identity, not private ownership | Reuse station palette and family identity data; no exact source exists. |
| Residence/household state | `48×48` operational building base plus `16×16` household overlay | world base + report-safe occupancy overlay | Reuse existing Den/Home/Lodge/Nursery building sources wherever exact; do not encode hidden kinship. |
| Apple empty | `16×16` Apple state | world resource state | **Reuse** `public/images/game/nature/tree_oak.png` as empty/no-fruit state. Low/mid/full already exist. |
| Task markers | retain existing `64×64` semantic source icons | world/UI marker at exact task geometry | **Reuse** all twelve files in `public/images/ui/tasks/`; do not generate generic markers or place them away from the authoritative site/footprint. |
| Roads, walls, gates, docks, boat, rail cart, crops | retain actual source-native dimensions | world base/edge/transport/crop state | Reuse delivered sources and resolver mappings; only missing canonical state keys need registry work. |

Construction coverage must not stop at Cookhouse. The canonical construction
catalog currently covers Den, Food Storage, Water Bowl, Beds, Herb Garden,
Nursery, Elder Corner, Workshop, Smithy, Barracks, Accounting Tent, Wood
Cutter, Stone Prep, Woodworking, Clothier, Tannery, Research Hut, Smelter,
Mill, Sawmill, and School. Each new build and upgrade has scaffold, structure,
and fit-out authority. The art registry needs a closed mapping from those
reported phases to either exact per-building state sprites or an explicitly
approved base-plus-overlay composition. A material icon in
`BlueprintPhasePresentation` is not, by itself, the authored world construction
state required by the visual plan.

Storage coverage likewise means visible lots and exact fullness on physical
tiles. A Barrel/Crate icon alone does not satisfy empty/partial/full Basket,
Barrel, Crate, Chest, and Rack states, nor the linked Workshop stockpile zone.

## Reuse findings from the integrated branches

The current worktree already contains the transferred Shrine branch assets and
the Bug-GUI-design integration outputs. A filename and semantic scan of
`../the-shrine-upgrade` and `../bug-gui-design` found no additional exact
Sawmill, Smelter, Fishing Rod, Microscope, quality-badge, augmentation, or
fullness-state production sprites to copy.

The following delivered families are therefore out of the remaining generation
inventory:

- Hole base and thirty cumulative layers;
- ten world-facing Lair visual bands and six distinct coarse encounter bands;
- twenty creature portraits;
- twenty named Hunting-drop material icons;
- all eighteen generated item/tool/equipment/container icons;
- all six fixture and four augmentation detail icons;
- all twenty-five canonical resource icons;
- all four formerly missing station bases;
- all 111 exact-key recipe icons with positive resolver integration;
- Cookhouse six-state sheet;
- Fishing Hut eight orientation/activity states;
- dynamic farm stages;
- Apple low/mid/full states;
- source-reused Barrel and Crate bases;
- Boat, docks, rail cart, and the already mapped station/resource sources;
- all twenty-six food icons.

Nearby pictures are not automatically valid substitutes. In particular:
Woodworking is not Sawmill, Smithy is not Smelter, Campfire is not stored Fuel,
Sack is not Basket, Herbs are not Medicine, Ore/Gold are not Gem, and an
interior Furnace is not a world-base building.

## Remaining integration ordering

1. Normalize the eleven legacy Station planned paths or deliberately update their
   registry paths; their current exact positive mappings remain usable and must
   not be replaced by generated lookalikes.
2. Canonicalize the still-unregistered quality/construction/fullness/family/
   world-state keys before generating those plan-required families.
3. Preserve all larger masters and exact-source copies as provenance only;
   runtime dimensions remain registry-owned.
4. Validate actual dimensions, sRGBA/alpha bounds, exact resolver key/path,
   accessibility fallback, authoritative trigger field, gameplay zoom,
   restart/despawn, native/WASM, and the required screenshot matrix.

There is no remaining image-generation or resolver work for the 263 registered
manifest art keys. The separate unregistered visual families remain
authority/design work.
