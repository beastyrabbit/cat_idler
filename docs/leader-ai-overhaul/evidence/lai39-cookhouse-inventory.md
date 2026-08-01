# LAI.39 Cookhouse Inventory

Read-only evidence for the Plan 1 Cookhouse dependency. This document inventories current and
protected-source definitions only; it does not implement Cookhouse runtime behavior.

## Required Plan And Board Receipts

| Requirement | Exact origin | LAI.39 implication |
|---|---|---|
| Cookhouse tasks are full 3x3, not center markers | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:99-107`; `docs/leader-ai-overhaul/BOARD.md:1208` | Every Cookhouse task must project all nine ordered cells through the existing spatial authority. |
| Stable IDs and manifest ownership | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:172-208`; `docs/leader-ai-overhaul/BOARD.md:1213-1214` | Recipes, foods, stations, complexity, tools, fixtures, outputs, art keys, and closed behavior handlers come from LAI.36 content IDs. |
| Physical lots and quality survive all movement | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:210-239`; `docs/leader-ai-overhaul/BOARD.md:1219-1220` | LAI.39 must consume LAI.37 lots/quality; no duplicate `QualityBand` or scalar laundering. |
| Recipe availability formula | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:288-299`; `docs/leader-ai-overhaul/BOARD.md:1219` | A recipe is available only when station+tier, ingredient capabilities, bundle owner, and physical ingredients/tools/capacity/workers are all satisfied. |
| Complexity table | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:301-312`; `docs/leader-ai-overhaul/BOARD.md:1220` | Raw/Simple/Prepared/Complex/Feast multipliers are monotonic and quality applies afterward. |
| Initial Cookhouse catalog | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:313-347`; `docs/leader-ai-overhaul/BOARD.md:1221` | Mill only converts Grain to Flour; cooking, baking, preserving, and brewing move to 3x3 Cookhouse. |
| Farmer/Cookhouse supply and shortages | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:506-518`; `docs/leader-ai-overhaul/BOARD.md:1231-1233` | Farmer owns supply planning; depletion creates located Cookhouse recovery work through report-safe planning. |
| Wire/UI/assets/cutover | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:530-595`, `623`; `docs/leader-ai-overhaul/BOARD.md:1234-1237` | Protocol v3 and UI expose Cookhouse queues, modifiers, tasks, quality, spoilage, and assets while deleting Shrine/Favor/generic-food aliases. |

## Existing Single Authorities To Consume

| Authority | Origin | Use in LAI.39 |
|---|---|---|
| LAI.36 content manifest | `crates/cat-sim/src/content_manifest.rs`; `crates/cat-sim/src/content_manifest.json` | Owns stable IDs, the one canonical recipe collection, explicit cutover receipts, station/recipe/bundle rows, art keys, and behavior handlers. There is no parallel active/compatibility recipe lifecycle. |
| LAI.37 quality lots | `crates/cat-sim/src/quality_lots.rs:50-105`, `107-138`, `139-220`, `249-340`, `334-410` | Owns `QualityBand`, multipliers, production score, lot locations/reservations, item/tool/fixture quality and augmentation eligibility. |
| LAI.38 food ecology | `docs/leader-ai-overhaul/evidence/lai38-food-ecology-inventory.md:21-31`, `80-102`, `253`, `273` | Owns food IDs, nutrition/hydration/spoilage/value, selection/consumption, trade/Hole food use, and God/report redaction. |
| Spatial task geometry | `crates/cat-sim/src/spatial_tasks.rs:70-133`, `228-248`, `686-727`; `crates/cat-sim/tests/spatial_tasks.rs:62-87` | Owns 3x3 ordered cells. LAI.39 should add Cookhouse to this authority instead of storing a second geometry table in recipe code. |
| Station-local scalar stores, pre-LAI.37 | `crates/cat-sim/src/stockpiles.rs:32-42` | Current station input/output persistence pattern and 10-unit local capacity receipt; LAI.39 should replace scalar recipe units with quality lots without losing restart conservation. |
| Physical containers | `crates/cat-sim/src/physical_storage.rs:1-5`, `20-47`, `70-120` | Baskets, barrels, crates, chests, racks and lot capacity/compatibility define storage/container invariants for Cookhouse ingredients and outputs. |

## Exact Complexity Table

Quality applies after these base complexity multipliers via LAI.37.

| Complexity | Ingredient count | Hunger/nutrition multiplier | Value multiplier | LAI.37 penalty origin |
|---|---:|---:|---:|---|
| Raw | 1 | 100% | 100% | `ProductionComplexity::Raw` penalty 0, `crates/cat-sim/src/quality_lots.rs:107-125` |
| Simple | 1-2 | 125% | 125% | `Simple` penalty 0, `crates/cat-sim/src/quality_lots.rs:107-125` |
| Prepared | 2-3 | 150% | 160% | `Prepared` penalty 250, `crates/cat-sim/src/quality_lots.rs:107-125` |
| Complex | 3-5 | 180% | 210% | `Complex` penalty 500, `crates/cat-sim/src/quality_lots.rs:107-125` |
| Feast | 5+ | 220% | 280% | `Feast` penalty 750, `crates/cat-sim/src/quality_lots.rs:107-125` |

The score formula is exact: weighted input quality + skill bonus + tool quality bonus + fixture quality bonus + `(station_tier - 1) * 125` - complexity penalty + keyed variation (`crates/cat-sim/src/quality_lots.rs:139-193`). Thresholds are Crude `<750`, Common `750..=1749`, Fine `1750..=2749`, Superior `2750..=3749`, Masterwork otherwise (`crates/cat-sim/src/quality_lots.rs:210-218`). Food multipliers are 80/100/120/145/175%, trade/Hole value multipliers are 75/100/130/170/225%, and item effect/durability multipliers are 80/100/115/135/160% (`crates/cat-sim/src/quality_lots.rs:91-104`).

## Mill-Only-Flour Rule

The active manifest Mill recipe is exactly `mill_flour`: `resource_grain` 2 -> `resource_flour` 2, no fuel, no container, no tools, no fixtures, bundle capability `grain_milling`, handler `mill_recipe` (`crates/cat-sim/src/content_manifest.json:3275-3307`). The manifest validation checks the active Cookhouse count (`crates/cat-sim/src/content_manifest.rs:1970-1985`), and the LAI.36 manifest test asserts the approved Cookhouse order plus Mill-only-Flour (`crates/cat-sim/tests/lai36_content_manifest.rs:164-187`).

Current runtime still conflicts with this rule: `station_recipes.rs` exposes Mill inputs `Grain/Flour/Food/Catnip/Herbs` and outputs `Food/Flour/Preserves/Brew` (`crates/cat-sim/src/station_recipes.rs:276-288`), has 20 Mill recipes including `flour_to_food`, baking, preserving, and brewing (`crates/cat-sim/src/station_recipes.rs:327-362`), and `world_tick` still routes Mill physical production through `ResourceKind::Food` plus `ResourceKind::Flour` (`crates/cat-sim/src/world_tick.rs:31173-31365`). Persistence also rewrites legacy combined Mill queues into `grain_to_flour` and `flour_to_food` (`crates/cat-server/src/persistence.rs:615-646`).

## Cookhouse Station And Bundle Data

| Definition | Current origin | Notes |
|---|---|---|
| Cookhouse station | `crates/cat-sim/src/content_manifest.json:2950-2972` | `content_id=station_cookhouse`, behavior `cookhouse`, art `art_station_cookhouse`, footprint 9, work geometry 3x3 origin `(0,0)`, min tier 1, task category `cooking`, fixture slot `cookhouse`, required capability `cookhouse`, handler `station_work`. |
| Cookhouse fixture | `crates/cat-sim/src/content_manifest.json:4168-4194` | `fixture_cookhouse`, art `art_fixture_cookhouse`, slot `cookhouse`, consumes `boar_tusk`, compatible with `cookhouse`, capability `station_fixtures`, handler `install_fixture`. |
| Bundle: grain milling | `crates/cat-sim/src/content_manifest.json:4029-4037` | Owner `resource_grain`, capability `grain_milling`, recipe `mill_flour`. |
| Bundle: apple Cookhouse | `crates/cat-sim/src/content_manifest.json:4038-4047` | Owner `resource_apple_tree`, capability `apple_gathering`, recipes `baked_apples`, `apple_porridge`, `apple_preserves`. |
| Bundle: fish Cookhouse | `crates/cat-sim/src/content_manifest.json:4048-4057` | Owner `resource_fish_habitat`, capability `hand_fishing`, recipes `grilled_fish`, `fish_stew`, `smoked_fish`. |
| Bundle: fuel Cookhouse | `crates/cat-sim/src/content_manifest.json:4058-4066` | Owner `resource_fuel`, capability `refined_processing`, recipes `roasted_meat`, `dried_meat`. This is valid current data but a reconciliation risk because raw meat itself requires `hunting_lairs`. |
| Bundle: flour Cookhouse | `crates/cat-sim/src/content_manifest.json:4067-4079` | Owner `resource_flour`, capability `grain_milling`, recipes `flatbread`, `apple_tart`, `herb_crusted_fish`, `meat_pie`, `travel_rations`, `festival_cake`. |
| Bundle: herb Cookhouse | `crates/cat-sim/src/content_manifest.json:4080-4088` | Owner `resource_herbs`, capability `herb_gathering`, recipes `meat_stew`, `surf_and_turf`, `hunters_feast`, `grand_lair_feast`. |

All eighteen initial meal rows and all five retained `brew_*` rows are canonical Cookhouse recipes, for twenty-three Cookhouse rows total. The initial meal rows use station `cookhouse`, tier 1, handler `cook_recipe`, and the manifest-owned `tools`/`fixtures` fields. LAI.39 must evaluate those fields generically even where the initial rows are empty because LAI.37 already owns item/fixture identity, quality, and eligibility.

## Exact 18 Cookhouse Recipes

| Recipe | Complexity | Inputs | Output | Fuel/container | Food stats | Bundle | Origins |
|---|---|---|---|---|---|---|---|
| `baked_apples` | Simple | `food_apple` x2 | `food_baked_apples` x2 | fuel yes, container no | nutrition 125, hydration 5, spoilage 72h, weight 1000, value 500 | apple | recipe `crates/cat-sim/src/content_manifest.json:3309-3338`; food `:560-580` |
| `grilled_fish` | Simple | `food_raw_fish` x1 | `food_grilled_fish` x1 | fuel yes, container no | nutrition 175, hydration 0, spoilage 48h, weight 1000, value 650 | fish | recipe `crates/cat-sim/src/content_manifest.json:3341-3370`; food `:581-602` |
| `roasted_meat` | Simple | `food_raw_meat` x1 | `food_roasted_meat` x1 | fuel yes, container no | nutrition 220, hydration 0, spoilage 48h, weight 1000, value 700 | fuel | recipe `crates/cat-sim/src/content_manifest.json:3373-3402`; food `:603-624` |
| `flatbread` | Simple | `resource_flour` x2, `food_water` x1 | `food_flatbread` x2 | fuel yes, container no | nutrition 150, hydration -5, spoilage 168h, weight 1000, value 550 | flour | recipe `crates/cat-sim/src/content_manifest.json:3405-3438`; food `:625-646` |
| `apple_porridge` | Prepared | `food_apple` x2, `resource_grain` x1, `food_water` x1 | `food_apple_porridge` x3 | fuel no, container no | nutrition 180, hydration 30, spoilage 72h, weight 1000, value 800 | apple | recipe `crates/cat-sim/src/content_manifest.json:3441-3478`; food `:647-668` |
| `fish_stew` | Prepared | `food_raw_fish` x2, `food_water` x1, `resource_herbs` x1 | `food_fish_stew` x3 | fuel no, container no | nutrition 240, hydration 40, spoilage 48h, weight 1000, value 1000 | fish | recipe `crates/cat-sim/src/content_manifest.json:3481-3518`; food `:669-690` |
| `meat_stew` | Prepared | `food_raw_meat` x2, `food_water` x1, `resource_herbs` x1 | `food_meat_stew` x3 | fuel no, container no | nutrition 275, hydration 35, spoilage 48h, weight 1000, value 1050 | herb | recipe `crates/cat-sim/src/content_manifest.json:3521-3558`; food `:691-712` |
| `apple_preserves` | Prepared | `food_apple` x3, `food_water` x1, `resource_clay` x1 | `food_apple_preserves` x3 | fuel no, container yes | nutrition 170, hydration 10, spoilage 720h, weight 1000, value 1000 | apple | recipe `crates/cat-sim/src/content_manifest.json:3561-3598`; food `:713-734` |
| `smoked_fish` | Prepared | `food_raw_fish` x2, `resource_herbs` x1 | `food_smoked_fish` x2 | fuel yes, container no | nutrition 220, hydration -10, spoilage 480h, weight 1000, value 1050 | fish | recipe `crates/cat-sim/src/content_manifest.json:3601-3634`; food `:735-756` |
| `dried_meat` | Prepared | `food_raw_meat` x2 | `food_dried_meat` x2 | fuel yes, container no | nutrition 260, hydration -15, spoilage 480h, weight 1000, value 1100 | fuel | recipe `crates/cat-sim/src/content_manifest.json:3637-3666`; food `:757-778` |
| `apple_tart` | Complex | `food_apple` x3, `resource_flour` x2, `food_water` x1 | `food_apple_tart` x4 | fuel no, container no | nutrition 300, hydration 5, spoilage 120h, weight 1000, value 1600 | flour | recipe `crates/cat-sim/src/content_manifest.json:3669-3706`; food `:779-800` |
| `herb_crusted_fish` | Complex | `food_raw_fish` x2, `resource_flour` x1, `resource_herbs` x1, `food_water` x1 | `food_herb_crusted_fish` x3 | fuel no, container no | nutrition 330, hydration 5, spoilage 72h, weight 1000, value 1700 | flour | recipe `crates/cat-sim/src/content_manifest.json:3709-3750`; food `:801-822` |
| `meat_pie` | Complex | `food_raw_meat` x2, `resource_flour` x2, `resource_herbs` x1, `food_water` x1 | `food_meat_pie` x4 | fuel no, container no | nutrition 390, hydration 0, spoilage 96h, weight 1000, value 1900 | flour | recipe `crates/cat-sim/src/content_manifest.json:3753-3794`; food `:823-844` |
| `surf_and_turf` | Complex | `food_raw_fish` x2, `food_raw_meat` x2, `resource_herbs` x1, `food_water` x1 | `food_surf_and_turf` x4 | fuel no, container no | nutrition 430, hydration 0, spoilage 72h, weight 1000, value 2100 | herb | recipe `crates/cat-sim/src/content_manifest.json:3797-3838`; food `:845-866` |
| `travel_rations` | Complex | `food_dried_meat` x1, `food_smoked_fish` x1, `food_flatbread` x1 | `food_travel_rations` x3 | fuel no, container no | nutrition 420, hydration -20, spoilage 960h, weight 1000, value 2000 | flour | recipe `crates/cat-sim/src/content_manifest.json:3841-3878`; food `:867-888` |
| `festival_cake` | Feast | `food_apple` x3, `resource_flour` x3, `food_water` x1, `food_brew` x1, `food_catnip` x1 | `food_festival_cake` x6 | fuel no, container no | nutrition 520, hydration 10, spoilage 120h, weight 1000, value 2800 | flour | recipe `crates/cat-sim/src/content_manifest.json:3881-3926`; food `:889-910` |
| `hunters_feast` | Feast | `food_raw_meat` x3, `food_raw_fish` x2, `food_apple` x2, `resource_herbs` x2, `food_water` x1 | `food_hunters_feast` x8 | fuel no, container no | nutrition 700, hydration 10, spoilage 72h, weight 1000, value 3500 | herb | recipe `crates/cat-sim/src/content_manifest.json:3929-3974`; food `:911-932` |
| `grand_lair_feast` | Feast | `food_raw_meat` x4, `food_raw_fish` x4, `food_apple` x3, `resource_flour` x3, `resource_herbs` x2, `food_brew` x1 | `food_grand_lair_feast` x12 | fuel no, container no | nutrition 980, hydration 0, spoilage 72h, weight 1000, value 5000 | herb | recipe `crates/cat-sim/src/content_manifest.json:3977-4026`; food `:933-954` |

Input food/resource capability receipts:
`food_water` requires `water_collection` (`crates/cat-sim/src/content_manifest.json:427-448`);
`food_apple` requires `apple_gathering` (`:449-470`);
`food_raw_fish` requires `hand_fishing` (`:471-492`);
`food_raw_meat` requires `hunting_lairs` (`:493-514`);
`food_catnip` requires `herb_gathering` (`:515-536`);
`food_brew` requires `cookhouse` (`:537-558`);
`resource_grain` and `resource_flour` require `grain_milling` (`:86-118`);
`resource_herbs`, `resource_clay`, and `resource_fuel` require `herb_gathering`, `material_processing`, and `refined_processing` (`:120-169`).

## Exact 108-runtime to 111-canonical recipe partition

The pre-cutover `station_recipes.rs` authority exposes 108 runtime recipe IDs. The settled manifest does **not** copy those into a compatibility-only lifecycle. It retains 92 current-runtime IDs as canonical recipes, removes or supersedes 16 current-runtime IDs, and adds nineteen exact replacements (`mill_flour` plus the eighteen initial meals), producing exactly 111 canonical recipes. The five stable brewing IDs are part of the 92 retained IDs and are reassigned to the Cookhouse, so they are neither deleted nor counted as new IDs. The manifest stores 17 cutover receipts: the 16 current-runtime dispositions plus removal of the persisted-only `grain_to_flour_and_food` alias. Constants `PRE_CUTOVER_RUNTIME_RECIPE_TOTAL=108`, `RETAINED_PRE_CUTOVER_RECIPE_TOTAL=92`, `CURRENT_RUNTIME_RECIPE_CUTOVER_TOTAL=16`, and `RECIPE_CUTOVER_RECEIPT_TOTAL=17` make that partition executable.

| Current station | Count | Current IDs and origin | LAI.39 disposition |
|---|---:|---|---|
| Mill | 20 | `grain_to_flour`, `flour_to_food`, `fine_grain_flour`, `stoneground_flour`, `masterwork_flour`, `bake_flatbread`, `bake_loaf`, `bake_biscuits`, `bake_festival_cake`, `bake_masterwork_pastry`, `dry_food`, `smoke_food`, `pickle_food`, `preserve_rations`, `preserve_masterwork_feast`, `brew_grain_small`, `brew_catnip_ale`, `brew_herbal_tonic`, `brew_spiced_ale`, `brew_masterwork` (`crates/cat-sim/src/station_recipes.rs:13-43`, `327-362`) | The first fifteen IDs have explicit supersession receipts. `grain_to_flour` becomes `mill_flour`; quality-named Flour variants collapse into LAI.37 quality; generic Food/bake/preserve rows map to exact typed meals. The five `brew_*` identities are retained, reassigned to Cookhouse, and remain the physical `food_brew` production path needed by Festival Cake and Grand Lair Feast. |
| Sawmill | 3 | `logs_to_lumber`, `carpentry_quality`, `carpentry_masterwork` (`crates/cat-sim/src/station_recipes.rs:363-374`) | Retained as canonical recipes outside LAI.39; not Cookhouse data. |
| Workshop | 20 pre-cutover / 19 retained | `materials_to_refined`, herbal/field-craft/expedition/gem/sand recipes (`crates/cat-sim/src/station_recipes.rs:375-522`) | `materials_to_refined` is explicitly removed because the canonical catalog has no distinct raw Materials input and therefore no valid non-self flow. The other nineteen IDs remain canonical outside LAI.39. |
| Smelter | 5 | `ore_to_metal`, metallurgy variants (`crates/cat-sim/src/station_recipes.rs:523-536`) | Retain outside LAI.39. |
| Wood Cutter | 2 | `logs_to_planks`, `carpentry_specialty` (`crates/cat-sim/src/station_recipes.rs:537-547`) | Retain outside LAI.39. |
| Stone Prep | 9 | `stone_to_blocks`, bone/stone/clay craft rows, `stonecraft_masterwork` (`crates/cat-sim/src/station_recipes.rs:548-620`) | Retain outside LAI.39. |
| Woodworking | 10 | `planks_and_blocks_to_tools`, bone tool, hunting/waterworks variants (`crates/cat-sim/src/station_recipes.rs:621-706`) | Retain outside LAI.39. |
| Clothier | 12 | `fibre_to_thread`, `fibre_to_cloth`, foraging/textile variants (`crates/cat-sim/src/station_recipes.rs:707-755`) | Retain outside LAI.39. |
| Tannery | 11 | `hide_to_leather`, animal-husbandry/leatherworking variants (`crates/cat-sim/src/station_recipes.rs:756-810`) | Retain outside LAI.39. |
| Smithy | 16 | `smithy_weapon`, `smithy_tool`, `smithy_armor`, metal mug, tool/weapon/armor variants (`crates/cat-sim/src/station_recipes.rs:811-849`) | Retain outside LAI.39. |

Cutover receipt: the dispatcher functions still treat `station_recipes.rs` as the runtime authority (`crates/cat-sim/src/station_recipes.rs:851-904`; `crates/cat-sim/src/world_tick.rs:810-899`). LAI.39 must route Cookhouse through the one canonical manifest, apply the explicit supersession/removal receipts, keep the five brew IDs at Cookhouse, and prevent the old Mill aliases from remaining executable.

## Runtime, Wire, Persistence, UI Consumers

| Consumer | Current origin | Required LAI.39 cutover |
|---|---|---|
| Building types | `crates/cat-sim/src/types.rs:100-138`; `crates/cat-protocol/src/lib.rs:1623-1655` | Add Cookhouse once to sim/protocol building enums. Current content manifest has `station_cookhouse`, but runtime/wire `BuildingType` does not. |
| Recipe availability and default queues | `crates/cat-sim/src/world_tick.rs:803-899` | Availability must evaluate the Plan recipe formula against canonical manifest recipes and LAI.36 capabilities, not `station_recipes` descriptors. Default Cookhouse and Mill queues must be deterministic and must not include any ID with a remove/supersede receipt. |
| Physical station execution | `crates/cat-sim/src/world_tick.rs:29791-29830`, `30853-30878`, `31173-31365`, `31728-31860` | Current execution is single-input/single-output, plus a Mill special-case for Food/Flour. LAI.39 needs a manifest-driven multi-ingredient cycle that preserves lot IDs, quality, station input/output/cargo locations, repeat queues, pause/progress, death/cancel/restart conservation. |
| Station local stores | `crates/cat-sim/src/stockpiles.rs:32-42`; `crates/cat-sim/src/quality_lots.rs:271-304` | Scalar station stores are a current restart receipt. LAI.39 should use `LotLocation::StationInput` and `StationOutput` as the conservation authority, not scalar `ResourceKind::Food`. |
| Persistence cutover | `crates/cat-server/src/persistence.rs:485-646`, `1495-1565` | The final P1.34 policy is a fresh schema, not semantic migration. New Cookhouse queues/stores persist strict canonical IDs, order/repeat/pause/progress, exact lots, and empty queues; the old Mill v7 split and `flour_to_food` resurrection path must be deleted. |
| Protocol station snapshots/actions | `crates/cat-protocol/src/lib.rs:1395-1458`, `1481-1510`, `2167-2174` | Existing station snapshots expose scalar input/output inventories and string recipe queues. Protocol v3/schema v2 must expose canonical content IDs, lot quality, spoilage, modifiers, capacity, and Cookhouse actions and reject the obsolete queue schema rather than retaining it as a semantic alias. |
| Resource wire aliases | `crates/cat-protocol/src/lib.rs:659-693`, `704-735` | `Food`, `Fish`, `Preserves`, `Blessings` remain wire aliases. LAI.39 must not add new generic-food behavior; deletion waits for protocol/report cutover with LAI.38/64. |
| Client UI | `crates/cat-client/src/leader_ai_ui/lai54/start_showcase.rs:19`, `101`, `259` | Current Cookhouse usage is a showcase marker only, not runtime UI. LAI.39 should feed later UI through protocol station snapshots and manifest metadata. |

## 3x3 Geometry Contract

Cookhouse task geometry should be the same ordered cells as the existing Workshop proof:
`(x,y)`, `(x+1,y)`, `(x+2,y)`, `(x,y+1)`, `(x+1,y+1)`, `(x+2,y+1)`, `(x,y+2)`, `(x+1,y+2)`, `(x+2,y+2)` (`crates/cat-sim/tests/spatial_tasks.rs:62-87`). `Rect::ordered_tiles` delegates to canonical row-major ordering (`crates/cat-sim/src/spatial_tasks.rs:70-133`), `TaskFootprint::rectangular` carries those tiles (`:228-248`), and `footprint_for` is the single size table for building footprints (`:686-704`). Cookhouse is not yet listed in `footprint_for`, so LAI.39's smallest spatial change is adding one `BuildingType::Cookhouse => (3, 3)` entry plus tests that mirror the Workshop exact nine cells.

Red geometry cases:

- Any Cookhouse task that reports only the center tile.
- Any recipe work endpoint that differs from the task footprint authority.
- Any UI/protocol projection that sorts cells differently from row-major.
- Any station obstruction/building placement that uses manifest `footprint_cells=9` while runtime still lacks Cookhouse in `BuildingType`.

## Physical Ingredients, Tools, Capacity, Workers, Reservations

- Station and tier gates: `station=cookhouse`, `station_tier=1`, station min tier 1, required station capability `cookhouse` (`crates/cat-sim/src/content_manifest.json:2950-2972`, `3309-4026`).
- Ingredient capability gates: all input content capabilities must be owned before processing; locked content may be found/stored/traded but not processed. Exact input capabilities are listed above from `content_manifest.json:86-169`, `427-558`.
- Bundle owner gate: recipe bundles identify the resource/material owner and capability (`crates/cat-sim/src/content_manifest.json:4029-4088`).
- Physical ingredient gate: every input must be an unreserved compatible lot in `LotLocation::Stockpile`, `StationInput`, or equivalent authorized source, then move through `StationInput -> StationOutput -> Cargo -> Stockpile/Hole` without scalar laundering (`crates/cat-sim/src/quality_lots.rs:249-304`).
- Tool and fixture gate: all twenty-three current Cookhouse rows have empty initial `tools`/`fixtures`, but the canonical descriptors still include both fields; LAI.39 evaluates them generically and LAI.37 owns item/fixture identity, quality, and installation eligibility.
- Capacity gate: current scalar station capacity is 10 per resource (`crates/cat-sim/src/stockpiles.rs:32-42`); LAI.39 must define how that maps to lot units without overflowing output or blocking reserved input.
- Worker gate: current station workers live in `BuildingRuntime` primary/additional work slots and durable queues (`crates/cat-sim/src/world_tick.rs:640-671`, `689-751`, `797-817`); Cookhouse should reuse this worker/queue model after adding a `Cooking`/Cookhouse labor route or agreed existing labor.
- Reservation invariant: `PhysicalLot.reservation` and `ItemInstance.reservation` forbid duplicate claims and invalid augmentation while reserved (`crates/cat-sim/src/quality_lots.rs:296-304`, `334-387`); death/cancel/route/restart must release or reassign claims without destroying ingredients/output.

## Obsolete aliases that must be deleted at cutover

LAI.39 must block new use immediately. LAI.47/48/52 own complete protocol, persistence, and root deletion, so no temporary adapter may become a supported compatibility surface:

- Generic food scalar: `ResourceKind::Food`, `ResourceAmounts.food`, Mill `flour_to_food`, baking-to-Food rows, and any recipe outputting generic Food (`crates/cat-protocol/src/lib.rs:659-693`, `704-735`; `crates/cat-sim/src/station_recipes.rs:327-362`).
- Generic fish scalar: `ResourceKind::Fish`; raw fish should be `food_raw_fish` lots from LAI.38 (`crates/cat-protocol/src/lib.rs:659-693`).
- Generic preserves scalar: `ResourceKind::Preserves`, Mill preserving rows, and storage/persistence fields that still expose preserves (`crates/cat-protocol/src/lib.rs:659-693`; `crates/cat-sim/src/station_recipes.rs:352-356`).
- Shrine/Favor/Blessings: `ResourceKind::Blessings` remains non-physical wire state (`crates/cat-protocol/src/lib.rs:691-693`) and must not backdoor Cookhouse quality/fuel/container behavior.
- Scholar/generic Insight: LAI.39 should consume LAI.36 capabilities and recipe bundles only; no per-recipe research nodes or hidden Insight gates.

## Source And Asset Receipts

The source-transfer manifest lists 53 exact untracked source assets from `the-shrine-upgrade`: black hole, crop stages, Apple overlays, sites, and transport (`docs/branch-plan-merge/source-transfer-manifest.md:126-141`). It explicitly says broader Plan 1 asset requirements absent from the list must be authored rather than dropped, and copied source assets must go through the asset owner with provenance (`docs/branch-plan-merge/source-transfer-manifest.md:140-141`, `213-221`).

Protected source scans found no Cookhouse or recipe implementation/assets. The protected source `station_recipes.rs` has the same legacy Mill constants and Mill recipe rows, including bake/preserve/brew aliases (`/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/station_recipes.rs:13-43`, `276-288`, `327-362`, `887-888`, `1171-1172`). Protected assets matching Cookhouse/food terms are only food storage, generic food/fish icons, and Apple tree overlays; no Cookhouse station, fixture, recipe icons, raw meat, typed cooked meal sprites, or quality badges were present.

Current target filesystem scans likewise found only `public/images/buildings/food_storage*.png`, `public/images/game/icons/{fish,food}.png`, `public/images/iso/buildings/food-storage.png`, and `public/images/resources/food.png`. The manifest already reserves planned art keys for all food icons (`crates/cat-sim/src/content_manifest.json:5959-6186`), recipe icons (`:6553-6717`), Cookhouse station (`:6967-6973`), and Cookhouse fixture (`:7147-7149`), but these are not delivered image files.

Missing sprite/icon inventory:

- Cookhouse 3x3 station state sheet and fixture icon: `art_station_cookhouse`, `art_fixture_cookhouse`.
- Active recipe icons: `art_recipe_mill_flour`, `art_recipe_baked_apples`, `art_recipe_grilled_fish`, `art_recipe_roasted_meat`, `art_recipe_flatbread`, `art_recipe_apple_porridge`, `art_recipe_fish_stew`, `art_recipe_meat_stew`, `art_recipe_apple_preserves`, `art_recipe_smoked_fish`, `art_recipe_dried_meat`, `art_recipe_apple_tart`, `art_recipe_herb_crusted_fish`, `art_recipe_meat_pie`, `art_recipe_surf_and_turf`, `art_recipe_travel_rations`, `art_recipe_festival_cake`, `art_recipe_hunters_feast`, `art_recipe_grand_lair_feast`.
- Typed food icons for all LAI.38 foods except legacy generic food/fish/water approximations; current generic icons are aliases only and should not satisfy final typed art.
- Quality badges for Crude/Common/Fine/Superior/Masterwork.

## Smallest LAI.39 Authority Boundary

Create one Cookhouse production authority that consumes existing authorities:

1. `content_manifest` remains the only recipe/station/food/bundle ID source.
2. `food_ecology` remains the only food stats, spoilage, consumption, trade/Hole, and report-redaction authority.
3. `quality_lots` remains the only quality/lot/item/fixture/reservation authority.
4. `spatial_tasks` remains the only footprint/cell-order authority.
5. LAI.39 owns only recipe execution orchestration: select an active manifest recipe for a Cookhouse building, validate station/tier/capability/bundle/physical ingredient/tool/capacity/worker gates, move quality lots deterministically through station input/output/cargo, compute output quality with LAI.37, and expose the queue/progress/block reason through existing protocol shape or a versioned extension.

Do not create a second catalog, second `QualityBand`, second food stat table, or second 3x3 geometry list.

## Staged Consumer Order

1. Content validation: assert the exact 111-recipe canonical partition, twenty-three Cookhouse rows (eighteen meals plus five retained brews), one Mill row (`mill_flour`), 92 retained pre-cutover IDs, 16 current-runtime dispositions, and 17 total cutover receipts.
2. Runtime type/spatial cutover: add Cookhouse to sim/protocol building types and `spatial_tasks::footprint_for` with exact nine-cell tests.
3. Queue availability cutover: source Cookhouse and Mill defaults from canonical manifest rows; block every superseded/removed ID.
4. Physical recipe engine: implement multi-ingredient lot reservation, station input/output movement, capacity, fuel/container consumption, output quality, spoilage start time, and queue repeat/progress.
5. Persistence restart: migrate existing scalar station stores/queues without resurrecting `flour_to_food`; prove death/cancel/route/restart conservation.
6. Protocol/UI cutover: expose Cookhouse station, typed lots, queue actions, quality/spoilage/modifiers, and report-safe fields.
7. Deletion cleanup: remove legacy Mill baking/preserving aliases and generic Food/Fish/Preserves UI/runtime paths after consumers are on typed IDs; preserve the five brew identities as Cookhouse work.

## Red Cases

- Mill can produce `Food`, `Preserves`, `Brew`, or any non-Flour output.
- `flour_to_food` appears in a fresh default queue or is resurrected by an obsolete loader.
- A Cookhouse recipe can run with station missing, tier too low, ingredient capability missing, bundle owner missing, no worker, no capacity, missing physical ingredients, or reserved input lots.
- A recipe consumes aggregate scalar resources instead of lot IDs, or output quality is recomputed from a duplicate enum/formula.
- Fuel/container flags are ignored; `apple_preserves` can run without the clay input and container rule.
- Multi-ingredient recipe cancellation, worker death, blocked route, or restart loses input, duplicates output, clears reservations incorrectly, or credits finished station output to aggregate resources before cargo deposit.
- Cookhouse task footprint is not exactly the nine row-major 3x3 cells.
- Generic `Food`, `Fish`, or `Preserves` is accepted as a compatibility alias for typed Cookhouse inputs/outputs after cutover.
- Locked content is processed or fed even though it is only allowed to be found/stored/traded.
- UI/God/report displays hidden exact spoilage/ecology/regrowth/quality source data beyond the LAI.38 report ladder.

## Highest-Risk Reconciliation Findings

1. The manifest has the correct active LAI.39 data, but live runtime selection and execution still use `station_recipes.rs` and do not have `BuildingType::Cookhouse`; this is the main duplicate-authority risk.
2. Persistence still migrates legacy Mill queues into `grain_to_flour` plus `flour_to_food`, directly conflicting with Mill-only-Flour and generic Food deletion.
3. Current physical station execution is single-input/single-output and scalar `ResourceKind` based; the 18 Cookhouse recipes require multi-ingredient, typed lot, quality-preserving movement.
4. Asset receipts are negative for Cookhouse: planned art keys exist, but no delivered Cookhouse, recipe, raw meat, typed meal, fixture, or quality badge sprites are present in current or protected source assets.
