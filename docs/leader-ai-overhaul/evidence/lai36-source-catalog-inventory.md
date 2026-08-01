# LAI.36 Source Catalog Inventory

Date: 2026-07-25

Scope: read-only inventory for LAI.36. I read the current dirty `feature-new-leader-ai` worktree,
the protected source worktree at `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade`, the
two restored plans from line 1, the LAI.35-LAI.36 board rows, the P1 register, and the source
transfer manifest. No tests or builds were run. The only file produced by this task is this
evidence document.

## Authorities Read

- `AGENTS.md:1-62`: Rust/Bevy workspace, cat-sim purity/determinism rules, read-only source-spec
  discipline, no routine full-suite local testing.
- `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:1-92`: Plan 1 requires semantic
  transfer, deletion of Shrine/Favor/Blessings/scholar Insight/generic Food/Fish/Preserves,
  BlackHole/Hole naming, fresh DBs/fixtures, one Leader planner, and validated stable-ID catalogs.
- `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:109-157`: stable ID public types,
  manifest-owned resources/foods/items/materials/creatures/recipes/augmentations/fixtures/
  capabilities/art, and closed behavior enums only.
- `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:1-21`: combined plan requires
  semantic, read-only source transfer; no cherry-pick/hot-root merge authority.
- `docs/leader-ai-overhaul/BOARD.md:1150-1152`: LAI.35 accepted; LAI.36 is the spec card for exact
  ID grammar/types, manifest-owned content classes, closed behavior enums, duplicate/dangling/
  cyclic/range/handler/art validation, strict decode, deterministic ordering, and additive-content
  tests.
- `docs/migration/BOARD.md:90-168`: P1 register shows the original closed enum taxonomy and
  constant-porting discipline; P1.2 is historical enum parity, not authority for new open content.
- `docs/branch-plan-merge/source-transfer-manifest.md:1-48`: semantic transfer rationale, frozen
  source snapshots, and `the-shrine-upgrade` snapshot digest.
- `docs/branch-plan-merge/source-transfer-manifest.md:98-100`: protected source tracked
  modifications include `research_catalog.rs`, `research_catalog_tracks.json`, and
  `upgrade_tree.rs`.
- `docs/branch-plan-merge/source-transfer-manifest.md:194-197`: bug/source catalog files to
  preserve as knowledge include the research catalog, junctions, legacy JSON, and tracks JSON.
- `docs/branch-plan-merge/source-transfer-manifest.md:213-221`: transfer matrix routes Shrine
  catalog/upgrade changes, Hunting leaves, and research graph knowledge into LAI.36/43/44/58.
- `docs/branch-plan-merge/source-transfer-manifest.md:232-243`: required receipt fields for later
  implementation evidence.

## Current-Tree Inventory

### LAI.36 Candidate Manifest

`crates/cat-sim/src/content_manifest.rs` is a current dirty-tree candidate left by a stopped
duplicate worker. It is 90,090 bytes / 2,337 lines, hash
`c36bb9f2af98d349916b76115e6100c8d0f0bcf40a0a78be4a4d9a7ab3a5d395`, and is not exported from
`crates/cat-sim/src/lib.rs:1-40`. I found no focused `content_manifest` test reference in
`crates/cat-sim/tests`.

Classification: manifest data candidate plus closed behavior enums plus validator implementation.
It is not yet a live exported manifest authority.

Exact LAI.36 requirement coverage by inspection:

| Requirement | Candidate origin | Classification | Status / risk |
|---|---|---|---|
| Stable ID grammar `[a-z][a-z0-9_]{0,63}` | `content_manifest.rs:20-123` | handler/validator | Present. Strict typed constructors and serde decode reject invalid IDs. |
| Public stable ID types | `content_manifest.rs:212-222` | manifest data types | Present: `ContentId`, `ResourceId`, `FoodId`, `ItemDefinitionId`, `MaterialId`, `CreatureId`, `RecipeId`, `CapabilityId`, `ArtKey`, `PhysicalLotId`, `MaterialInstanceId`. |
| Closed behavior enums only | `content_manifest.rs:224-359` | closed behavior enum | Present: `QualityBand`, `EquipmentSlot`, `ItemClass`, `TaskCategory`, `StationBehavior`, `AuthorityDomain`, `EffectOperation`, `AugmentationSlot`, `FixtureSlot`. |
| Descriptor classes | `content_manifest.rs:376-607` | manifest data | Present for resources, foods, item definitions/layers, materials/uses, creatures/loot, lair bands, stations, recipes, augmentations, fixtures, and research capabilities. |
| Duplicate ID validation | `content_manifest.rs:806-924` | handler/validator | Present for typed IDs, content IDs, and capability IDs. |
| Deterministic ordering | `content_manifest.rs:925-1014`, `1618-1689` | handler/validator | Present through monotonic `order` fields and builder increments. |
| Art validation | `content_manifest.rs:1016-1108` | handler/validator | Present only for nonempty/unique `ArtKey`; it does not validate file-backed assets or dimensions. |
| Handler validation | `content_manifest.rs:95-110`, `1110-1159` | handler/validator | Present as a string allow-list; no evidence these handlers map to callable runtime functions yet. |
| Dangling reference validation | `content_manifest.rs:1161-1368` | handler/validator | Present for capabilities, materials, loot, recipes, tools, fixtures, station links, and unlocked content. |
| Cyclic capability validation | `content_manifest.rs:1370-1407` | handler/validator | Present for capability prerequisite graph. |
| Range/numeric validation | `content_manifest.rs:1409-1508` | handler/validator | Present for food value/weight, material Hole gates/uses, creature levels/loot/stats, lair bands, footprints, recipe complexity, slots, augmentation/fixture compatibility. |
| Founding bootstrap validation | `content_manifest.rs:1510-1543` | handler/validator | Present for `water_collection`, `apple_gathering`, `hand_fishing`, `basic_food_handling`, and free `logs`/`stone`. |
| Canonical capability validation | `content_manifest.rs:1545-1609` | handler/validator | Present, but current candidate data appears internally inconsistent; see risks below. |
| Additive content tests | none found | downstream LAI.36 | Missing in current tree. No tests/builds run for this task. |

Current candidate stable IDs by manifest class:

- Resources (`content_manifest.rs:1697-1763`): `logs`, `stone`, `water_source`, `apple_tree`,
  `fish_habitat`, `grain`, `herbs`, `clay`, `fuel`, `planks`, `flour`.
- Foods (`content_manifest.rs:1765-1813`): `water`, `apple`, `raw_fish`, `raw_meat`, `brew`,
  `catnip`, `baked_apples`, `grilled_fish`, `roasted_meat`, `flatbread`, `apple_porridge`,
  `fish_stew`, `meat_stew`, `apple_preserves`, `smoked_fish`, `dried_meat`, `apple_tart`,
  `herb_crusted_fish`, `meat_pie`, `surf_and_turf`, `travel_rations`, `festival_cake`,
  `hunters_feast`, `grand_lair_feast`, `divine_ration`, `divine_water`.
- Item definitions (`content_manifest.rs:1815-1852`): `basket`, `barrel`, `crate`, `chest`,
  `rack`, `fishing_rod`, `lens`, `microscope`, `advanced_instrument`, `weapon`, `armor`,
  `treated_pelt_clothing`, `membrane_clothing`.
- Materials (`content_manifest.rs:1854-1922`): `bone`, `hide`, `bat_wing`, `fox_pelt`,
  `badger_pelt`, `boar_tusk`, `wolf_pelt`, `lynx_pelt`, `stag_antler`, `serpent_scale`,
  `bear_pelt`, `eagle_feather`, `moon_antler`, `warg_fang`, `cockatrice_eye`, `troll_hide`,
  `griffin_plume`, `basilisk_scale`, `manticore_barb`, `beast_core`, `wyvern_membrane`,
  `dragon_heart`.
- Curated material use routing (`content_manifest.rs:1925-1948`): pelt/hide/membrane to
  `clothier`; antler/tusk/feather/scale to `woodworking`; fang/barb to `smithy`;
  eye/core/heart/plume/wing to `research_hut`; fallback to `workshop`.
- Creatures (`content_manifest.rs:1950-2014`): `cave_bat`, `red_fox`, `badger`, `wild_boar`,
  `gray_wolf`, `lynx`, `great_stag`, `giant_serpent`, `brown_bear`, `great_eagle`, `moon_stag`,
  `warg`, `cockatrice`, `forest_troll`, `griffin`, `basilisk`, `manticore`, `chimera`, `wyvern`,
  `elder_dragon`.
- Lair bands (`content_manifest.rs:2016-2035`): `1-19`, `20-39`, `40-59`, `60-79`, `80-94`,
  `95-100` with synthetic `art_lair_band_*` keys.
- Stations (`content_manifest.rs:2037-2073`): `black_hole`, `mill`, `cookhouse`, `fishing_hut`,
  `workshop`, `tannery`, `clothier`, `woodworking`, `smithy`, `research_hut`, `school`.
- Recipes (`content_manifest.rs:2075-2168`): `mill_flour`, plus Plan 1 cookhouse recipes
  `baked_apples`, `grilled_fish`, `roasted_meat`, `flatbread`, `apple_porridge`, `fish_stew`,
  `meat_stew`, `apple_preserves`, `smoked_fish`, `dried_meat`, `apple_tart`,
  `herb_crusted_fish`, `meat_pie`, `surf_and_turf`, `travel_rations`, `festival_cake`,
  `hunters_feast`, `grand_lair_feast`.
- Augmentations (`content_manifest.rs:2170-2202`): `weapon_augmentation`,
  `armor_augmentation`, `tool_augmentation`, `research_instrument_augmentation`.
- Fixtures (`content_manifest.rs:2204-2228`): `cookhouse_fixture`, `fishing_hut_fixture`,
  `workshop_fixture`, `research_fixture`, `storage_fixture`, `black_hole_fixture`.
- Capabilities (`content_manifest.rs:2230-2336`): founding/food/craft/research/Hole base
  capabilities at `2260-2287`, one capability per rare material at `2289-2300`,
  `cookhouse_baked_apples_bundle` at `2302-2317`, and 30 Hole-axis capabilities
  `black_hole_{width,depth,darkness}_{01..10}` at `2319-2336`.

Highest current candidate risks:

1. The candidate is unexported and untested. It cannot satisfy LAI.36 completion until it is wired
   through `lib.rs`, has focused validation tests, and is used as the single content authority.
2. Canonical capability validation likely fails by inspection: most cookhouse foods and recipes
   use `canonical_capability: cookhouse`, but the `cookhouse` capability is canonical only for
   `station_cookhouse` (`content_manifest.rs:2270`); only baked apples gets a separate bundle
   capability (`2302-2317`), and even the baked-apples recipe still declares `cookhouse`
   (`2161`).
3. Recipe IDs do not reconcile with the live station recipe catalog: the candidate uses
   `mill_flour`/`recipe_mill_flour` (`content_manifest.rs:2075-2086`), while live station recipes
   use `grain_to_flour` and `flour_to_food` (`station_recipes.rs:13-15`).
4. Art keys are synthetic (`art_*`) and not mapped to source or current asset paths. LAI.36 can
   validate uniqueness, but LAI.49/68 still need asset receipt/path/dimension validation.
5. The candidate covers the Plan 1 initial 20-creature and 20-rare-material names, but the current
   live Hunting leaf still has only four source species; LAI.42 must reconcile behavior and roster.
6. The candidate has no 108-station-recipe coverage and no mapping for the full current
   `ResourceKind`/`ItemKind` runtime surface. It is a useful seed, not complete manifest coverage.

### Current Runtime Item, Resource, Recipe, and Research Catalogs

`crates/cat-sim/src/items.rs` is current runtime inventory behavior, hash
`3478106f50810c4ba85e84e971494e838a6c70b84d52c3d2669b5218707e7051`.

- `Material` enum (`items.rs:24-43`): `wood`, `stone`, `metal`, `bone`, `fibre`, `leather`,
  `gem`, `clay`, `sand`. Classification: closed behavior enum for current runtime, but LAI.36
  wants materials as manifest data.
- `ItemKind` enum (`items.rs:108-129`): `mug`, `bowl`, `furniture`, `tool`, `weapon`, `armor`,
  `clothing`, `trinket`, `toy`, `brick`. Classification: closed behavior enum/item class today;
  LAI.36 should split behavior class from manifest-owned item definitions.
- `MAX_QUALITY` and quality math (`items.rs:189-209`): legacy/current quality bands `0..=4`.
  Classification: current handler feeding LAI.37.
- `Item` compact wire key (`items.rs:211-285`) and `ItemStore`/`ItemInstance` (`items.rs:390-989`):
  current handler/ledger; not yet LAI.36 manifest data.

`crates/cat-sim/src/stockpiles.rs` is current physical resource enum and fish ecology.

- `ResourceKind` (`stockpiles.rs:124-160`): `Food`, `Fish`, `Water`, `Herbs`, `Catnip`, `Grain`,
  `Flour`, `Preserves`, `Medicine`, `Brew`, `Materials`, `Stone`, `Refined`, `Weapons`, `Armor`,
  `Logs`, `Lumber`, `Planks`, `Blocks`, `Tools`, `Fibre`, `Thread`, `Hide`, `Bone`, `Cloth`,
  `Leather`, `Ore`, `Gem`, `Clay`, `Sand`, `Metal`, `Blessings`. Classification: current closed
  runtime enum with explicit legacy/obsolete aliases for LAI.52 deletion: `Food`, `Fish`,
  `Preserves`, `Blessings`.
- Fish ecology (`stockpiles.rs:79-93`) is current handler/downstream LAI.40 input, not LAI.36
  manifest data yet.

`crates/cat-sim/src/recipes.rs`, hash
`de6c84fcd0b7006d0a2ba6ae48a2d5af99f02a734ca236a35c802bac64ab5221`, is current compact
trade-good crafting behavior.

- Recipe descriptors (`recipes.rs:57-104`): `WOOD_TRADE_RECIPE`, `STONE_TRADE_RECIPE`,
  `CLOTH_TRADE_RECIPE`, `LEATHER_TRADE_RECIPE`. Classification: handler/legacy compatibility
  descriptors; LAI.36 should turn the stable recipe/content IDs into manifest data.
- `CLOTH_TRADE_RECIPE` and `LEATHER_TRADE_RECIPE` are explicitly “reserved compatibility” and
  “No authoritative world-tick path invokes this descriptor” (`recipes.rs:81-104`). Classification:
  legacy/obsolete for later deletion or replacement by manifest-backed physical routes.

`crates/cat-sim/src/station_recipes.rs`, hash
`d8247e4c71a167e849492876bb42519b6d2574cc1be096cbfea5cd3c55ce6b9f`, owns the largest current
recipe stable-ID surface.

- Recipe constants (`station_recipes.rs:13-95`, `131-160`) include `logs_to_lumber`,
  `grain_to_flour`, `flour_to_food`, `fine_grain_flour`, `stoneground_flour`,
  `masterwork_flour`, `bake_flatbread`, `bake_loaf`, `bake_biscuits`, `bake_festival_cake`,
  `bake_masterwork_pastry`, `herbal_poultice`, `herbal_tonic`, `herbal_salve`,
  `herbal_remedy`, `herbal_masterwork_remedy`, `dry_food`, `smoke_food`, `pickle_food`,
  `preserve_rations`, `preserve_masterwork_feast`, `brew_grain_small`, `brew_catnip_ale`,
  `brew_herbal_tonic`, `brew_spiced_ale`, `brew_masterwork`, `materials_to_refined`,
  `ore_to_metal`, `logs_to_planks`, `stone_to_blocks`, `planks_and_blocks_to_tools`,
  `fibre_to_thread`, `fibre_to_cloth`, `hide_to_leather`, `smithy_weapon`, `smithy_armor`,
  `smithy_tool`, `bone_tool`, `bone_trinket`, `bone_toy`, `bone_mug`, `stone_mug`,
  `metal_mug`, `gem_jewelry`, `clay_mug`, `clay_bowl`, `clay_brick`, `sand_glass_mug`,
  `sand_glass_bowl`, `sand_glass_trinket`, the subsistence/frontier families, and industrial
  material families. Classification: current manifest data in code, plus handlers.
- Legacy aliases (`station_recipes.rs:39-43`): `grain_to_flour_and_food` and `MILL_RECIPE_ID`.
  Classification: legacy/obsolete compatibility aliases for later deletion.
- The module asserts 108 unique recipe IDs in tests (`station_recipes.rs:1048-1073`), but LAI.36
  still needs the IDs moved behind a unified content manifest.

`crates/cat-sim/src/research_catalog.rs`, hash
`c1dc71ab7d7f93bc245bcc0fcb7c3800b60b87caf5d8fed4b71411e1af8e05ea`, is current generated
research graph data.

- Approved building IDs include `shrine` (`research_catalog.rs:20-45`). Classification:
  legacy/obsolete alias under Plan 1; later deletion/replacement must use `black_hole`/Hole.
- Approved effect IDs include `shrineBlessingYield` (`research_catalog.rs:47-84`). Classification:
  legacy/obsolete Blessings alias.
- `ResearchPayload` (`research_catalog.rs:146-178`) is a closed behavior enum over string-keyed
  content IDs. Classification: closed behavior enum plus downstream LAI.44/58 handler.
- `RUNTIME_RESOURCE_UNLOCK_IDS` (`research_catalog.rs:240-300`) contains generated resource IDs,
  including `hunting_bulk`; current Plan 1 source says `hunting_bulk` is retained but renamed
  player-facing as Hunting Parties.

`crates/cat-sim/src/research_manifest.rs`, hash
`5676589ec14ec12feffb45bc4c372e7c733b5f802347c8f0cffc3a458436e8a3`, is already Plan 2/LAI.58
research manifest work, not LAI.36 content authority.

- It declares that it removes obsolete Shrine/Favor/generic-food/coin authority
  (`research_manifest.rs:1-7`).
- Deprecated IDs include `shrine_stores` (`research_manifest.rs:76-90`).
- Obsolete fragments include `shrine`, `favor`, `blessing`, `coin`, `purse`, `generic_food`,
  `food_storage` (`research_manifest.rs:92-100`).
- Obsolete exact identities include `flour_to_food`, `dry_food`, `smoke_food`, `pickle_food`,
  `preserve_rations`, `preserve_masterwork_feast` (`research_manifest.rs:101-108`).
- Required capability families include `typed_food_handling`, `apple_ecology`, `hand_fishing`,
  `universal_quality`, `physical_lot_ledger`, `cookhouse`, `fishing_hut`, `fishing_rods`,
  `hunting_lairs`, `hunting_parties`, `plank_processing`, `material_processing`,
  `augmentations`, `station_fixtures`, `research_instruments`, and Hole capability families
  (`research_manifest.rs:112-192`). Classification: downstream LAI.44/58 manifest data that
  LAI.36 must reconcile against, not replace silently.

`crates/cat-sim/src/black_hole.rs`, hash
`2fccf16ce8b92a1376857c92e262663ea390ab4604f0fa9e8fa03273dd60fb73`, is byte-identical to the
protected source copy.

- Axis constants and runtime schema (`black_hole.rs:16-22`), axis enum/state (`49-143`),
  feed/order/runtime (`144-230`), intake/candidate/credit/totals (`253-604`), value/gate functions
  (`605-689`), and upgrade recipe types (`690-785`) are current handlers for LAI.41.
- It still accepts current `ResourceKind::Food`, `Fish`, `Preserves`, and ignores
  `ResourceKind::Blessings` in value/gate functions (`black_hole.rs:610-673`). Classification:
  handler plus legacy/generic alias risk until LAI.38/41/52 replace those with typed food/content.

`crates/cat-sim/src/hunting_lair.rs`, hash
`3eea03a1f0cdc124608873fd85046b10aad257c81a5fc583ebf73a13b04bb111`, is byte-identical to the
protected source copy.

- Current species enum (`hunting_lair.rs:17-78`) has only `Fox`, `Badger`, `Bear`, `RivalBeast`
  and materials `FoxPelt`, `BadgerPelt`, `BearPelt`, `BeastCore`.
- Plan 1/LAI.36 candidate has 20 creature IDs. Classification: current handler/source behavior
  leaf is obsolete/partial relative to LAI.42 roster, but its combat/cache/respawn behavior remains
  source input.

Current obsolete/compatibility aliases explicitly identified for later deletion:

- Shrine identity: `shrine.rs:1-12`, `shrine_offerings.rs:1-14`, `stockpiles.rs:24-26`,
  `research_catalog.rs:20-45`, `research_catalog_tracks.json:21`, and source research catalog
  Shrine/Hole compatibility IDs.
- Favor currency: `favor.rs:1-12`, `favor.rs:69-76`, `shrine_offerings.rs:6-14`,
  `research_purchase.rs:1078-1154` (God research funds still support old lanes), and
  `scholar_research.rs:12-29` imports `Favor`.
- Blessings: `stockpiles.rs:124-160`, `research_catalog.rs:47-84`, `black_hole.rs:610-673`,
  source `upgrade_tree.rs:965-1040`.
- Generic Food/Fish/Preserves: `stockpiles.rs:124-160`, `station_recipes.rs:29-33`,
  `station_recipes.rs:276-288`, `black_hole.rs:610-673`.
- Scholar Insight compatibility: `scholar_research.rs:1-7`, `scholar_research.rs:31-79`.
- Compatibility recipe aliases: `station_recipes.rs:39-43` and reserved cloth/leather trade recipes
  `recipes.rs:81-104`.

## Protected Source Worktree Inventory

Source worktree: `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade`, read-only.

Source receipts from `docs/branch-plan-merge/source-transfer-manifest.md`:

- Source frozen snapshot digest: `b1bcc2433d29d23f10167de07465f4c39a7164bc782d9ec292fa8cafe3a4bdaf`
  (`source-transfer-manifest.md:36-48`).
- `the-shrine-upgrade` domain digests: `crates/cat-sim` digest
  `2094dbb2095ced51d11c09c9771487518281825ae1b70e103701e6543a25c59a`;
  `public/images` digest `f6752d02e2883aa4dc60d7a7483bea8c20e7e9368a52e3d001daccf6b9d5780b`
  (`source-transfer-manifest.md:51-60`).
- Exact source file inventory: tracked modifications at `source-transfer-manifest.md:98-100`;
  untracked sim/protocol/server/client leaves at `source-transfer-manifest.md:116-131`;
  53 source assets at `source-transfer-manifest.md:135-146`.

Source code hashes inspected:

- `the-shrine-upgrade/crates/cat-sim/src/black_hole.rs`:
  `2fccf16ce8b92a1376857c92e262663ea390ab4604f0fa9e8fa03273dd60fb73`; identical to current.
- `the-shrine-upgrade/crates/cat-sim/src/hunting_lair.rs`:
  `3eea03a1f0cdc124608873fd85046b10aad257c81a5fc583ebf73a13b04bb111`; identical to current.
- `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs`:
  `05985b2cd2d21903e18827fe8ac0c29816a916c59396b08e8b183e187167626d`; source-only current-tree
  missing handler.
- `the-shrine-upgrade/crates/cat-sim/src/research_catalog.rs`:
  `2eb1e7fef756a82d9c4091857a123474fed89d771b6e13bee2a7ff39830c6605`.
- `the-shrine-upgrade/crates/cat-sim/src/research_catalog_tracks.json`:
  `01be1f6bbaafb85f70150f0b9bc5a53732570c0fa137de77433be5ddd4e879ff`.
- `the-shrine-upgrade/crates/cat-sim/src/upgrade_tree.rs`:
  `b1190a5c1d2871528bfaed3a1a04516409e37dbbed244a3a4289b3151ca37779`.

Source stable IDs and classifications:

- Source Black Hole runtime (`the-shrine-upgrade/.../black_hole.rs:16-785`): handler source,
  already copied byte-identically to current; rename/currency/generic food reconciliation remains
  LAI.41/44/46/52.
- Source Hunting Lair (`the-shrine-upgrade/.../hunting_lair.rs:17-353`): handler source, copied
  byte-identically to current, but only four species; LAI.42 must expand to Plan 1 twenty-species
  manifest and quality model.
- Source Hunting runtime (`the-shrine-upgrade/.../hunting_runtime.rs:36-78`, `396-716`): source-only
  handler for active parties, trophy claims, outcomes, captain recommendation, attempt reports,
  start/release/validate/assemble/seed helpers. Missing in current; classify as downstream
  LAI.42/46+ source input.
- Source research catalog Black Hole axis compatibility:
  `the-shrine-upgrade/.../research_catalog_tracks.json:21` marks `buildingId: "shrine"` with
  `blackHoleTrack: true`; `38-41` defines axes `width`, `depth`, `darkness`.
- Source research catalog code:
  `the-shrine-upgrade/.../research_catalog.rs:143-170` defines `ResearchAxis` and
  `ResearchAxisEntitlement`; `735-811` expands Black Hole track; `794-802` maps early Hole tiers to
  persisted Shrine IDs (`shrine_foundations`, `shrine_workflow`, `shrine_timing`, `shrine_crews`,
  `shrine_reinforcement`). Classification: legacy/compatibility alias; extract ordering/axis
  knowledge only, delete Shrine names later.
- Source `hunting_bulk` compatibility: source merge doc says keep the existing `hunting_bulk`
  research ID but player-facing meaning becomes Hunting Parties and party cap three
  (`BLACK_HOLE_LEADER_AI_MERGE.md:87-89`). Current LAI.36 candidate still uses `hunting_bulk`
  (`content_manifest.rs:2274`), while current LAI.58 research manifest introduces
  `hunting_parties` (`research_manifest.rs:142-146`). Reconcile explicitly before implementation.

Source art/assets:

- Transfer manifest fixed the exact 53 new source assets:
  `black-hole.png`, `black-hole/base.png`, 30 axis layers
  `{width,depth,darkness}-01..10.png`, 12 crop stages, 3 oak apple overlays,
  `sites/{lair,quarry}.png`, and `transport/{boat,dock_land,dock_water,rail_cart}.png`
  (`source-transfer-manifest.md:135-146`).
- Source art tests prove quarry/lair distinction, oak apple overlay progression, crop-stage
  uniqueness, and transport uniqueness (`the-shrine-upgrade/crates/cat-client/tests/world_site_art.rs:4-73`).
- Source Black Hole art tests prove base plus ten layers per axis and transparent road ring
  (`the-shrine-upgrade/crates/cat-client/tests/black_hole_art.rs:21-66`).
- Current LAI.36 candidate uses synthetic `ArtKey` strings such as `art_station_black_hole`,
  `art_creature_*`, `art_portrait_*`, `art_lair_band_*`, and `art_recipe_*`
  (`content_manifest.rs:1748-2219`). These are not source asset paths and have no receipt/hash
  mapping yet.

## LAI.36 Initial Manifest Coverage Required

The minimum initial manifest for LAI.36 should cover:

1. ID grammar/newtypes and strict decode for all stable ID classes.
2. Closed behavior enums only for equipment slots, item classes, task categories, station behavior,
   authority domain, effect operation, augmentation slots, fixture slots, and quality bands.
3. Manifest-owned resources and acquisition/processing capabilities, including founding logs/stone,
   Water Source, Apple Tree, Fish Habitat, grain/herbs/clay/fuel/planks/flour, and all current
   live `ResourceKind` IDs that remain after generic aliases are deleted.
4. Manifest-owned foods: concrete water, apples, raw fish, raw meat, brewed/cooked/prepared/
   preserved foods, divine emergency supplies if retained, with nutrition/hydration/spoilage/
   value/quality/art.
5. Manifest-owned item definitions/materials: containers, fishing rod, research instruments,
   functional equipment, pelt/membrane clothing, common `bone`/`hide`, all twenty rare creature
   materials, typed augmentation slots, and typed station fixtures.
6. Manifest-owned creatures: all twenty Plan 1 creatures with levels, stats, loot, primary material,
   portrait art key, and lair-band/art references.
7. Manifest-owned stations and recipes: Hole, Mill, Cookhouse, Fishing Hut, Workshop, Tannery,
   Clothier, Woodworking, Smithy, Research Hut, School; full 108 current station recipes plus Plan
   1 cookhouse recipes or a documented superseding mapping; no duplicate scalar/finite output path.
8. Manifest-owned research capabilities/payloads: founding capabilities, physical lot/quality,
   Cookhouse, Fishing Hut/rods, Hunting Lairs/Parties, material processing, augmentations, fixtures,
   research instruments, Hole axes, and downstream LAI.44/58 research payloads.
9. Validators for duplicate/dangling/cyclic/range/handler/art errors, deterministic ordering, strict
   decode, file-backed art key resolution, and additive-content tests.

Current candidate coverage: strong type/validator skeleton and useful initial seed data; incomplete
runtime coverage, no export/tests, no file-backed art mapping, and unresolved canonical capability
ownership.

## Highest-Risk Reconciliation Findings

1. The unexported `content_manifest.rs` candidate is the right shape for LAI.36 but currently looks
   internally inconsistent around canonical capabilities and is not wired or tested.
2. There are three concurrent content authorities today: current runtime enums/constants
   (`ResourceKind`, `ItemKind`, `Material`, station recipe constants), the current LAI.58 research
   manifest, and the new LAI.36 candidate. LAI.36 must nominate one manifest authority and classify
   the rest as handlers or legacy aliases.
3. Shrine/Favor/Blessings/Insight/generic Food/Fish/Preserves are still present in current code and
   some current handlers. They are explicitly compatibility aliases for later deletion, not
   manifest data to preserve.
4. Source Black Hole/Hunting behavior has already been copied for two leaves, but source
   `hunting_runtime.rs` is not present in current. LAI.42/46 must either port/adapt it or record a
   stronger replacement.
5. The source research catalog uses Shrine IDs as Black Hole axis compatibility names. Current
   LAI.58 manifest marks Shrine fragments obsolete and introduces Hole capabilities. LAI.36/44
   must resolve `hunting_bulk` vs `hunting_parties` and `shrine_*` vs `black_hole_*` IDs before any
   protocol/persistence work.
6. Art keys are not receipts. LAI.36 can own `ArtKey` grammar and references, but LAI.49/68 must map
   every useful source image to a stable art key, source hash, dimensions, transparency/bounds, and
   final disposition.
