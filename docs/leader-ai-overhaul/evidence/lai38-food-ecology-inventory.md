# LAI.38 Food Ecology Inventory

Date: 2026-07-25

Scope: read-only inventory for LAI.38. This evidence consumes the current LAI.36 manifest contract and LAI.37 quality-lot contract, inventories current and protected source food/ecology definitions, and defines the smallest implementation boundary for the next dependency. No production code, tests, boards, plans, manifests, Cargo files, or protected source worktrees were edited; no builds or tests were run.

## Required Contract

LAI.38 is the Plan 1 dependency for typed food, Apples, founding sources, hunger, and spoilage. The active board row requires deleting generic stored `Food`/`Fish`/`Preserves`, guaranteeing reachable founding Water bank, Apple tree, and fish shoreline with no starter-stock substitute, defining concrete nutrition/hydration/spoilage/value/quality, deterministic consumption, exact Apple tile/states/depletion/slow persisted secret regrowth, and trade/Hole use (`docs/leader-ai-overhaul/BOARD.md:1175`).

P1 rows routed to LAI.38:

| Row | LAI.38 requirement |
| --- | --- |
| `P1.01` | Remove Shrine/Favor/Blessings/scholar Insight and generic stored `Food`/`Fish`/`Preserves` compatibility adapters during semantic integration (`docs/leader-ai-overhaul/BOARD.md:1200`). |
| `P1.04` | Report ladder: stock error +-40/25/12/5/2; ecology/regeneration hidden through level 3, +-25% at level 4, +-10% at level 5; exact Apple regrowth and fish replenishment stay server-only (`docs/leader-ai-overhaul/BOARD.md:1203`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:48-60`). |
| `P1.13` | Manifest owns foods and all food properties (`docs/leader-ai-overhaul/BOARD.md:1212`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:172-208`). |
| `P1.15` | Quality applies to Water, Apples, Fish, Meat, meals, and every physical stock class; it survives hauling, trade, reservations, Hole, and persistence (`docs/leader-ai-overhaul/BOARD.md:1214`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:210-239`). |
| `P1.16` | Production quality and gathering quality use the exact LAI.37 score formula, thresholds, and deterministic variation (`docs/leader-ai-overhaul/BOARD.md:1215`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:241-273`). |
| `P1.19` | Raw/Simple/Prepared/Complex/Feast food complexity affects nutrition and value before quality multipliers (`docs/leader-ai-overhaul/BOARD.md:1218`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:301-312`). |
| `P1.20` | Initial 3x3 Cookhouse catalog is fixed and manifest-owned; LAI.38 must leave recipe authority to LAI.39 but consume these food IDs (`docs/leader-ai-overhaul/BOARD.md:1219`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:313-347`). |
| `P1.21` | Founding guarantee must include reachable Water source plus valid bank, Apple tree, and fish habitat plus shoreline; starter reserve stock is not an acceptable substitute (`docs/leader-ai-overhaul/BOARD.md:1220`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:349-356`). |
| `P1.22` | Apple work occurs only at exact tree tiles; states are `empty`, `low`, `medium`, `full`; harvest lowers state and creates quality Apples; slow deterministic persisted regrowth is server-only and report-limited; Apples feed raw eating, Cookhouse, trade, and Hole (`docs/leader-ai-overhaul/BOARD.md:1221`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:357-367`). |
| `P1.23` | Founding hand-fishing and later Fishing Hut must use real shoreline/habitat with no fabricated stock (`docs/leader-ai-overhaul/BOARD.md:1222`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:109-125`). |
| `P1.24` | Hole rewards increase with processing, complexity, quality, value, augmentation, and condition (`docs/leader-ai-overhaul/BOARD.md:1223`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:378-389`). |
| `P1.25` | Hole validation checks ownership, identity, quality, capability, Darkness, route, reservation, and amount; no hidden survival-stock veto (`docs/leader-ai-overhaul/BOARD.md:1224`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:390-397`). |
| `P1.31` | Farmer owns Apples, fishing, food-days, and Cookhouse supply (`docs/leader-ai-overhaul/BOARD.md:1230`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:506-513`). |
| `P1.32` | Food depletion creates located Apple/Fish/Hunt/farm/Cookhouse recovery work; cargo delivery and salvage stay physical (`docs/leader-ai-overhaul/BOARD.md:1231`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:514-518`). |
| `P1.33` | Protocol v3 must remove Shrine/Favor/generic-food variants and add typed food/quality/Hole surfaces (`docs/leader-ai-overhaul/BOARD.md:1232`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:529-542`). |
| `P1.34` | Fresh DB/reset is acceptable; food/inventory must stay outside Leader fingerprint (`docs/leader-ai-overhaul/BOARD.md:1233`). |
| `P1.35` | UI Food/Cookhouse fields expose report-safe food-days, quality, nutrition, spoilage, source reports, queues, modifiers, and tasks without hidden truth (`docs/leader-ai-overhaul/BOARD.md:1234`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:587-595`). |
| `P1.36` | Assets must include four Apple states, farm stages, every raw/prepared food icon, and quality badges (`docs/leader-ai-overhaul/BOARD.md:1235`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:619-630`). |

Integrated Plan additions that affect LAI.38: God sees the same report projection as the Leader (`docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:56-68`); routine food permissions are Leader/officer decisions and divine rescue outputs physical purpose-bound Divine Rations/Water (`docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:422-493`); trade is physical barter and value-based comparison, not coin (`docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:517-529`); final cutover must remove `Shrine`, `Favor`, `Blessings`, generic `Food`/`Fish`/`Preserves`, scholar `Insight`, and duplicate authorities (`docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:663-664`).

## Upstream Contracts To Consume

### LAI.36 content manifest

Current `crates/cat-sim/src/content_manifest.rs` is the LAI.36 data-only authority. It states that stable identities and catalog data live in `content_manifest.json`, and explicitly excludes quality, physical-lot behavior, runtime mutation, research currency, and renderer file-existence checks (`crates/cat-sim/src/content_manifest.rs:1-7`). LAI.38 must consume the manifest instead of retyping food IDs or properties.

Manifest hooks already present:

| Definition | Origin | LAI.38 use |
| --- | --- | --- |
| Embedded manifest | `crates/cat-sim/src/content_manifest.rs:19-21`, `crates/cat-sim/src/content_manifest.json:1-2` | Runtime/tests can load versioned manifest data. |
| Founding capabilities | `crates/cat-sim/src/content_manifest.rs:23-28` | Required IDs are `water_collection`, `apple_gathering`, `hand_fishing`, `basic_food_handling`. |
| Plan 1 Cookhouse IDs | `crates/cat-sim/src/content_manifest.rs:30-49` | Exact 18 downstream recipe IDs; LAI.38 should consume foods, LAI.39 owns station production. |
| Founding resource rows | `crates/cat-sim/src/content_manifest.json:6-8` | `water_source`, `apple_tree`, and `fish_habitat` exist as manifest resources with founding acquisition. |
| `FoodDescriptor` schema | `crates/cat-sim/src/content_manifest.rs:546-563` | Single manifest owner for `id`, `content_id`, `art_key`, `nutrition`, `hydration`, `spoilage_hours`, `weight_milli`, `value_milli`, `raw_safe`, `ingredient_tags`, `recipe_bundle`, capability, and handler. |
| `ContentManifest.foods` and art registry | `crates/cat-sim/src/content_manifest.rs:832-850` | Food definitions and art keys come from manifest-owned arrays. |
| Planned food art keys | `crates/cat-sim/src/content_manifest.json:300-325` | All concrete food icons currently resolve to planned asset paths, not shipped sprites. |

### LAI.37 quality lots

Current `crates/cat-sim/src/quality_lots.rs` is the evolving LAI.37 authority. It must be consumed by LAI.38 rather than duplicated:

| Definition | Origin | LAI.38 use |
| --- | --- | --- |
| `QualityBand` | `crates/cat-sim/src/quality_lots.rs:50-59` | Exact five bands: `Crude`, `Common`, `Fine`, `Superior`, `Masterwork`. Do not introduce another `QualityBand` in LAI.38. |
| Food multipliers | `crates/cat-sim/src/quality_lots.rs:91-99` | Nutrition/hydration/value application uses 80/100/120/145/175 for food and 75/100/130/170/225 for trade/Hole. |
| Bulk lot identity | `crates/cat-sim/src/quality_lots.rs:249-259` | Stored food lots must key by manifest `ContentId` plus quality. |
| Locations | `crates/cat-sim/src/quality_lots.rs:268-277` | LAI.38 must route through `Source`, `Stockpile`, `StationInput`, `StationOutput`, `Cargo`, `Cache`, `Hole`. |
| Physical lot fields | `crates/cat-sim/src/quality_lots.rs:293-300` | Apple/fish/water/meal stock needs lot id, key, provenance, quantity, location, and reservation. |
| Item instance fields | `crates/cat-sim/src/quality_lots.rs:327-338` | Hole/trade food work should not duplicate item instance quality/augmentation behavior. |

The LAI.37 evidence records the exact score formula, thresholds, eligible stock classes, conservation invariants, and deletion aliases (`docs/leader-ai-overhaul/evidence/lai37-quality-lot-inventory.md:59-134`, `docs/leader-ai-overhaul/evidence/lai37-quality-lot-inventory.md:197-209`). LAI.38 red tests should import that authority and add no back-reference from LAI.37 to food ecology.

## Retained Concrete Food Manifest

These are the retained food definitions now present in `content_manifest.json`. Values are manifest data and must be the only LAI.38 source of exact nutrition, hydration, spoilage, weight, value, raw-safety, art key, and canonical capability.

| Food ID | Content ID | Nutrition | Hydration | Spoilage hours | Value milli | Art key | Raw safe | Capability | Origin |
| --- | --- | ---: | ---: | --- | ---: | --- | --- | --- | --- |
| `water` | `food_water` | 0 | 100 | none | 100 | `art_food_water` | true | `water_collection` | `crates/cat-sim/src/content_manifest.json:31` |
| `apple` | `food_apple` | 80 | 10 | 96 | 250 | `art_food_apple` | true | `apple_gathering` | `crates/cat-sim/src/content_manifest.json:32` |
| `raw_fish` | `food_raw_fish` | 140 | 0 | 24 | 400 | `art_food_raw_fish` | false | `hand_fishing` | `crates/cat-sim/src/content_manifest.json:33` |
| `raw_meat` | `food_raw_meat` | 180 | 0 | 18 | 450 | `art_food_raw_meat` | false | `hunting_lairs` | `crates/cat-sim/src/content_manifest.json:34` |
| `catnip` | `food_catnip` | 40 | 0 | 168 | 800 | `art_food_catnip` | false | `herb_gathering` | `crates/cat-sim/src/content_manifest.json:35` |
| `brew` | `food_brew` | 90 | 60 | 240 | 900 | `art_food_brew` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:36` |
| `baked_apples` | `food_baked_apples` | 125 | 5 | 72 | 500 | `art_food_baked_apples` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:37` |
| `grilled_fish` | `food_grilled_fish` | 175 | 0 | 48 | 650 | `art_food_grilled_fish` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:38` |
| `roasted_meat` | `food_roasted_meat` | 220 | 0 | 48 | 700 | `art_food_roasted_meat` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:39` |
| `flatbread` | `food_flatbread` | 150 | -5 | 168 | 550 | `art_food_flatbread` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:40` |
| `apple_porridge` | `food_apple_porridge` | 180 | 30 | 72 | 800 | `art_food_apple_porridge` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:41` |
| `fish_stew` | `food_fish_stew` | 240 | 40 | 48 | 1000 | `art_food_fish_stew` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:42` |
| `meat_stew` | `food_meat_stew` | 275 | 35 | 48 | 1050 | `art_food_meat_stew` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:43` |
| `apple_preserves` | `food_apple_preserves` | 170 | 10 | 720 | 1000 | `art_food_apple_preserves` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:44` |
| `smoked_fish` | `food_smoked_fish` | 220 | -10 | 480 | 1050 | `art_food_smoked_fish` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:45` |
| `dried_meat` | `food_dried_meat` | 260 | -15 | 480 | 1100 | `art_food_dried_meat` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:46` |
| `apple_tart` | `food_apple_tart` | 300 | 5 | 120 | 1600 | `art_food_apple_tart` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:47` |
| `herb_crusted_fish` | `food_herb_crusted_fish` | 330 | 5 | 72 | 1700 | `art_food_herb_crusted_fish` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:48` |
| `meat_pie` | `food_meat_pie` | 390 | 0 | 96 | 1900 | `art_food_meat_pie` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:49` |
| `surf_and_turf` | `food_surf_and_turf` | 430 | 0 | 72 | 2100 | `art_food_surf_and_turf` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:50` |
| `travel_rations` | `food_travel_rations` | 420 | -20 | 960 | 2000 | `art_food_travel_rations` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:51` |
| `festival_cake` | `food_festival_cake` | 520 | 10 | 120 | 2800 | `art_food_festival_cake` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:52` |
| `hunters_feast` | `food_hunters_feast` | 700 | 10 | 72 | 3500 | `art_food_hunters_feast` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:53` |
| `grand_lair_feast` | `food_grand_lair_feast` | 980 | 0 | 72 | 5000 | `art_food_grand_lair_feast` | false | `cookhouse` | `crates/cat-sim/src/content_manifest.json:54` |
| `divine_ration` | `food_divine_ration` | 1000 | 0 | none | 1 | `art_food_divine_ration` | true | `divine_rescue` | `crates/cat-sim/src/content_manifest.json:55` |
| `divine_water` | `food_divine_water` | 0 | 1000 | none | 1 | `art_food_divine_water` | true | `divine_rescue` | `crates/cat-sim/src/content_manifest.json:56` |

Recipe rows already encode the exact initial Cookhouse graph: `baked_apples` through `grand_lair_feast` with ingredients, outputs, complexity, fuel/container flags, art keys, and `cook_recipe` handlers (`crates/cat-sim/src/content_manifest.json:159-176`). LAI.38 should not implement Cookhouse production, but it must ensure all these output food definitions can be stored, spoiled, selected, traded, and fed to the Hole as typed lots.

Deletion aliases:

| Alias | Current origin | LAI.38 status |
| --- | --- | --- |
| Generic `Food` | `crates/cat-sim/src/stockpiles.rs:127-129`, `crates/cat-protocol/src/lib.rs:659-662`, `crates/cat-sim/src/world_tick.rs:10081-10086` | Legacy aggregate only; must not be a stored LAI.38 lot/content ID. |
| Generic `Fish` | `crates/cat-sim/src/stockpiles.rs:127-130`, `crates/cat-protocol/src/lib.rs:659-662`, `crates/cat-sim/src/world_tick.rs:10075-10080` | Legacy aggregate only; retained concrete food is `raw_fish`/fish meals. |
| Generic `Preserves` | `crates/cat-sim/src/stockpiles.rs:134-136`, `crates/cat-protocol/src/lib.rs:659-668`, `crates/cat-sim/src/world_tick.rs:10091-10093` | Legacy aggregate only; retained concrete food is `apple_preserves`. |
| `Blessings`, `Shrine`, `Favor`, scholar `Insight` | `docs/leader-ai-overhaul/BOARD.md:1200`; `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:663-664` | Compatibility names to delete during later cutover; LAI.38 must not add new dependencies on them. |

## Current Runtime Inventory

### Needs and consumption

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| Hunger/thirst scalar decay | `crates/cat-sim/src/needs.rs:10-29`, `crates/cat-sim/src/needs_constants.rs:13-17` | Existing behavior | Decay is independent of typed food quality/properties. |
| Scalar restore helpers | `crates/cat-sim/src/needs.rs:31-59`, `crates/cat-sim/src/needs_constants.rs:27-32` | Existing behavior | Uses fixed hunger/drink amounts, not manifest nutrition/hydration. |
| Passive survival tick | `crates/cat-sim/src/survival.rs:12-16`, `crates/cat-sim/src/survival.rs:96-127` | Legacy behavior | Consumes availability booleans/aggregate values; no food selection or lot decrement. |
| Personal serving completion | `crates/cat-sim/src/world_tick.rs:18991-19010` | Handler | Restores fixed `PERSONAL_MEAL_RESTORE`/`PERSONAL_DRINK_RESTORE` by carried fraction; ignores manifest nutrition, hydration, spoilage, and quality. |
| Critical survival accounting | `crates/cat-sim/src/world_tick.rs:22326-22341` | Handler | Treats carried `Food`/`Fish`/`Water` as aggregate survival resources; not exact typed lots. |
| Food permission policy | `crates/cat-sim/src/food_divine_policy.rs:30-36`, `crates/cat-sim/src/food_divine_policy.rs:80-105`, `crates/cat-sim/src/food_divine_policy.rs:140-176` | Downstream LAI.61 policy | Already models allowed/reserve/forbidden and lethal override by string `edible_id`; LAI.38 should feed it manifest IDs and physical availability, not duplicate permissions. |

Required LAI.38 selection authority: deterministic selection over physical food lots using manifest `FoodDescriptor`, LAI.37 quality multiplier, permissions, spoilage/expiry, nutrition/hydration need, stable food ID, and stable lot ID. The exact comparator should be test-owned by LAI.38 so `world_tick`, UI, Hole, and trade do not each invent their own food ordering.

### Spoilage

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| Stored/overflow scalar decay constants | `crates/cat-sim/src/spoilage.rs:7-18` | Existing behavior | Applies a TS-era food scalar formula, not per-food `spoilage_hours`. |
| `SpoilableResource` | `crates/cat-sim/src/spoilage.rs:20-26` | Legacy enum | Only `Food` and `Herbs`; no typed food or quality. |
| Storage report inputs/results | `crates/cat-sim/src/spoilage.rs:28-69` | Existing behavior | Reports per resource kind, not physical lot. |
| Current tick application | `crates/cat-sim/src/world_tick.rs:10066-10087` | Handler | Applies same scalar spoilage function independently to aggregate `fish` and `food`; water is only clamped. |

LAI.38 must cut over spoilage to physical lots. Each food lot needs an age/cursor sufficient for deterministic decay across restart, uses manifest `spoilage_hours`, preserves `content_id + quality + provenance`, and never launders spoiled or partially consumed stock back into generic aggregates.

### Founding ecology and source guarantees

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| Tile scalar resources | `crates/cat-sim/src/world_gen.rs:29-40` | Legacy data | World tiles have scalar `food`, `herbs`, and `water`, not Apple tree states or typed fish habitat resource IDs. |
| Biome food ranges | `crates/cat-sim/src/biomes.rs:170-187`, `crates/cat-sim/src/biomes.rs:207-245` | Legacy manifest-like data | Biomes seed generic `food` ranges and `max_resources.food`; they do not specify concrete Apples or raw fish ecology. |
| Starter water injection | `crates/cat-sim/src/world_gen.rs:80-103`, `crates/cat-sim/src/world_gen.rs:165-204` | Handler | Guarantees a nearby water tile only; it does not encode dry bank, Apple 3x3, or fish shoreline bundle in one authority. |
| Founding no natural interior | `crates/cat-sim/src/world_tick.rs:65436-65460` | Existing test | Confirms claimed interior has no natural resources/water and nearby revealed exterior water exists. |
| Building/road dryness and reachable water | `crates/cat-sim/src/world_tick.rs:65656-65682` | Existing test | Confirms buildings/roads avoid water and nearby reachable water source exists. |
| Dry water bank | `crates/cat-sim/src/world_tick.rs:66770-66786` | Existing test | Confirms one source has adjacent dry bank for water work. |
| Generic source report | `crates/cat-sim/src/world_tick.rs:5221-5256`, `crates/cat-sim/src/world_tick.rs:5350-5374` | Report handler | Counts generic stock and generic renewable food sources, not typed Water/Apple/fish ecology. |

Required founding bundle: every new colony must create, persist, and validate a revealed reachable Water source plus dry bank, an Apple tree with exact 3x3 obstruction footprint and trunk harvest tile, and a reachable fish habitat plus shoreline work tile. This must happen before any starter stock, and red tests must fail if survival is achieved only by initial `resources.food`, `resources.fish`, `resources.water`, or any generic reserve substitute.

### Apple source

Current target runtime has no Apple tree behavior authority. The manifest has `resource_apple_tree` with `apple_gathering` (`crates/cat-sim/src/content_manifest.json:7`) and `food_apple` with nutrition/hydration/spoilage/value/art (`crates/cat-sim/src/content_manifest.json:32`), but there is no runtime record for Apple tree state, regrowth cursor, trunk tile, or harvest quality.

Closest current code:

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| Generic non-forest food regrowth | `crates/cat-sim/src/world_tick.rs:10262-10312` | Legacy handler | Regrows scalar tile `resources.food`; no `empty`/`low`/`medium`/`full`, no Apple lot creation, no persisted secret regrowth state. |
| Sapling tree regrowth | `crates/cat-sim/src/world_tick.rs:10314-10345` | Legacy handler | Handles visual sapling overlay maturation only; not an Apple source. |
| Depletion helper | `crates/cat-sim/src/depletion.rs:1-17`, `crates/cat-sim/src/depletion.rs:33-57` | Legacy behavior | Defines generic forest exclusion, chopped forest food cap, and +1 food/hour regrowth; not typed Apple depletion. |
| Farm crops | `crates/cat-sim/src/farming.rs:17-34`, `crates/cat-sim/src/farming.rs:47-100` | Downstream farming behavior | Current persisted plots cover `Catnip`, `Grain`, and `Herb` stages/work; Apples are not farm plots and need a separate tree-source authority. |
| Food recovery tasks | `crates/cat-sim/src/world_tick.rs:6687-6696`, `crates/cat-sim/src/world_tick.rs:6705-6717` | Legacy handler | Shortage creates generic Hunt from cave food tiles; no Apple work category or exact tree tile. |

Required Apple authority:

- Geometry: exact 3x3 obstruction footprint around the Apple tree and one stable trunk/work tile; all harvest tasks must target that trunk, not arbitrary adjacent or center-only positions.
- States: `empty`, `low`, `medium`, `full`; harvest lowers state deterministically and creates quality `food_apple` lots.
- Quality: harvest uses LAI.37 gathering variant with source quality, worker skill/tool/fixture when applicable, and deterministic variation.
- Regrowth: slow, deterministic, persisted cursor once per world tick; no `Math.random`, no wall-clock drift, no lost fractional progress on restart.
- Report ladder: exact Apple regrowth truth is never protocol/God-visible; levels 1-3 hide ecology, level 4 shows +-25%, level 5 shows +-10%.

### Fish source

Current fish ecology is closer, but still scalar:

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| `FishPopulation` | `crates/cat-sim/src/stockpiles.rs:79-92` | Existing data | Persists stock/capacity/cursor, but stock is scalar and not `raw_fish` quality lots. |
| Replenishment | `crates/cat-sim/src/world_tick.rs:10349-10380` | Handler | Deterministic bounded fish replenish exists, but exact stock is currently visible through protocol snapshots. |
| Fishing job suspension/completion route | `crates/cat-sim/src/world_tick.rs:17317-17392`, `crates/cat-sim/src/world_tick.rs:17441-17450` | Handler | Work depends on valid designated bank/source jobs, but results still map into carried/resource scalars. |
| Protocol fish population | `crates/cat-protocol/src/lib.rs:590-609` | Wire | Exposes exact `stock`, `capacity`, and `last_replenished_at_ms`; conflicts with report-ladder secrecy. |

LAI.38 should define shared founding fish-source validation and report projection, but leave Fishing Hut station throughput to LAI.40. The founding guarantee still requires real fish habitat plus shoreline; `raw_fish` lots from hand-fishing must be physical, quality-bearing, and conservation-safe.

### Storage, stockpiles, persistence, and restart

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| Stockpile aggregate invariant | `crates/cat-sim/src/stockpiles.rs:1-16` | Legacy authority | `ColonyRuntime.resources` compatibility aggregate is reconciled to stockpiles; this competes with LAI.37 physical lots. |
| Gather spot kind | `crates/cat-sim/src/stockpiles.rs:60-77` | Existing data | Single `ResourceKind`; cannot specify food ID or quality. |
| Resource enum | `crates/cat-sim/src/stockpiles.rs:124-160` | Legacy enum | Includes generic `Food`, `Fish`, `Water`, `Preserves`, `Blessings`. |
| Protocol resources | `crates/cat-protocol/src/lib.rs:619-693`, `crates/cat-protocol/src/lib.rs:704-720` | Wire | Physical stockpile goods include generic food/fish/preserves/water and scalar amounts. |
| Persistence save | `crates/cat-server/src/persistence.rs:867-928` | Persistence | Saves scalar `resources`, stockpiles, farms, gather spots, stock ledger, and fish habitats; no typed food lot table/document. |
| Persistence load | `crates/cat-server/src/persistence.rs:1113-1158` | Persistence | Restores farm plots, gather spots, fish habitats, stock ledger, and items; no Apple source state or typed food lots. |

LAI.38 storage cutover should not mutate `resources.food/fish/preserves` directly. It should route typed food through LAI.37 physical lots, then let later protocol/persistence work decide how to serialize lot ledgers. Cancellation, death, route loss, spoilage, consumption, trade escrow, Hole delivery, and restart must conserve lot identity and quantity.

### Trade and Hole

| Current path | Origin | Classification | LAI.38 conflict |
| --- | --- | --- | --- |
| Hole generic value | `crates/cat-sim/src/black_hole.rs:609-645` | Legacy handler | Values generic `Food`, `Fish`, `Preserves`, and `Brew` as `ResourceKind`; water is zero value. |
| Hole generic Darkness | `crates/cat-sim/src/black_hole.rs:647-674` | Legacy handler | Darkness gates generic resources; no typed food value/quality/complexity. |
| Contribution kind | `crates/cat-sim/src/food_divine_policy.rs:201-221` | Downstream policy | Has `TypedFood` eligibility but does not yet validate lots or quality. |
| Trade valuation | `crates/cat-sim/src/trade_valuation.rs:41-53`, `crates/cat-sim/src/trade_valuation.rs:118-221` | Downstream evaluator | Values report-safe evidence but not typed food directly. |

LAI.38 must expose a reusable typed-food valuation surface: base `value_milli` from manifest, complexity/progression already in manifest values, LAI.37 quality/Hole multiplier, and physical lot eligibility. The Hole must later validate content ID, quality, reservation, route, capability, Darkness, and amount; scarcity may be misjudged by Leader reports but hidden exact survival stock must not veto a valid submitted offer.

## Protected Source and Asset Receipts

Protected source tree: `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade`, read-only.

Source-transfer receipt: the source manifest records `the-shrine-upgrade` frozen digest and transfer discipline (`docs/branch-plan-merge/source-transfer-manifest.md:16-23`). Its untracked asset receipt includes exactly 53 files, including 12 crop-stage sprites and Apple overlays `tree_oak_apples_{low,mid,full}.png` (`docs/branch-plan-merge/source-transfer-manifest.md:126-141`). The transfer matrix says source images copy only through asset-owner hash/provenance/transparency/bounds validation, while food ecology/leader/world tick code contributes ideas only (`docs/branch-plan-merge/source-transfer-manifest.md:213-221`).

Protected assets found:

| Asset | Source path | Target status |
| --- | --- | --- |
| Apple low overlay | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/nature/tree_oak_apples_low.png` | Missing in target. |
| Apple mid overlay | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/nature/tree_oak_apples_mid.png` | Missing in target. |
| Apple full overlay | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/nature/tree_oak_apples_full.png` | Missing in target. |
| Generic food/fish/water icons | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/icons/{food,fish,water}.png` | Target has same generic icons, not concrete manifest food icons. |
| Water terrain/edge | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/terrain/{water,water_edge}.png` | Target has water terrain/edge. |
| Farm dynamic stages | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/farm/dynamic/{catnip,grain,herb}-{sprout,growing,flowering,mature}.png` | Receipt exists; LAI.38 does not copy/generate art. |

Missing sprite/icon inventory for LAI.38:

- Apple tree `empty` state: no protected source overlay found; must be authored or represented from base tree plus no overlay.
- Apple tree 3x3 obstruction/trunk harvest indicator: no protected source-specific sprite found.
- Concrete food icons: target has only generic `public/images/game/icons/food.png`, `fish.png`, `water.png` and generic resource images; manifest points every retained concrete food icon to `assets/planned/content/*.png` (`crates/cat-sim/src/content_manifest.json:300-325`), but those planned sprites are not shipped.
- Quality badges: required by Plan 1 assets, not present in the found target/protected food asset scan.

Protected source code reviewed for Apple/farm/food work did not contain a complete Apple runtime authority; its farming model remains crop-oriented and the Apple evidence is asset/source-idea only. Therefore LAI.38 should not copy protected source code wholesale.

## Red Cases For LAI.38

These are the exact red cases needed before implementation:

1. Manifest coverage: every retained food ID above loads from LAI.36, has a unique `content_id`, art key, nutrition or hydration, positive weight/value, spoilage hours where applicable, and no generic `food`, `fish`, or `preserves` content ID. `apple_preserves` is allowed only as concrete `food_apple_preserves`.
2. No duplicate quality authority: LAI.38 consumes `quality_lots::QualityBand`, `BulkLotKey`, and `PhysicalLot`; any local `QualityBand`, raw quality `u8`, or parallel food-lot struct fails.
3. Founding sources: for a deterministic seed matrix, each founding colony has revealed reachable Water source plus dry bank, Apple tree with 3x3 obstruction and trunk work tile, and fish habitat plus shoreline work tile. Clearing starter `resources.food`, `resources.fish`, and `resources.water` must not break the test if sources are reachable; adding starter reserves must not satisfy it.
4. Apple geometry: construction/road/stockpile placement cannot occupy the 3x3 Apple footprint; harvest tasks target only the trunk/work tile; off-tree Apple work fails.
5. Apple state machine: `full`, `medium`, `low`, and `empty` are serializable states; harvest lowers state and mints `food_apple` lots with LAI.37 gathering quality; empty harvest does not mint stock.
6. Apple regrowth: regrowth is slow, deterministic, persisted, and once-per-world-tick; restart with partial progress produces the same future state as uninterrupted ticks. Protocol/God reports never expose the exact hidden cursor.
7. Fish founding and secrecy: founding fish habitat has a valid shoreline; hand-fishing mints `food_raw_fish` quality lots only from real habitat stock; exact stock/capacity/cursor are redacted by report level and no longer appear as raw protocol truth.
8. Deterministic consumption: selection uses only physical lots and report/permission inputs in a stable order. It applies manifest nutrition/hydration, LAI.37 food multiplier, spoilage status, `FoodPermission`, lethal emergency override, and stable tie-breaks by food ID and lot ID.
9. Physical spoilage: per-lot spoilage uses manifest `spoilage_hours`, lot age/cursor, storage/conservation modifiers, and restart-stable arithmetic. Spoilage preserves conservation and cannot become aggregate `Food`/`Fish`/`Preserves`.
10. Trade/Hole food valuation: typed food value derives from manifest `value_milli` and LAI.37 trade/Hole multiplier; raw food is least valuable, processed/complex food is worth more through manifest values, and validation checks identity, quality, reservation, route, capability, Darkness, and amount.
11. Recovery invariants: cancellation, carrier death, route loss, station cancel, trade cancel, Hole cancel, and restart conserve exact `content_id + quality + lot_id + quantity + provenance`.
12. God/report redaction: Food/Cookhouse/source UI and God actions use the same report projection as the Leader; levels 1-3 hide exact ecology/regrowth, level 4 shows +-25%, and level 5 shows +-10%.

## Smallest Single-Authority Boundary

Recommended new boundary: `crates/cat-sim/src/food_ecology.rs` plus `crates/cat-sim/tests/lai38_food_ecology.rs`.

The module should be a pure cat-sim leaf. It should consume LAI.36 `ContentManifest`, `FoodId`, `ContentId`, `ResourceId`, and `ArtKey`, and LAI.37 `QualityBand`, `BulkLotKey`, `PhysicalLot`, `LotLocation`, and `QualityLotLedger`. It should not import `world_tick`, protocol wire types, persistence, rendering, server code, or any Shrine/Favor/Blessings/scholar Insight compatibility state.

The boundary should own:

- Validation that LAI.36 has the retained concrete food/source IDs and no generic stored food IDs.
- `AppleTreeSource` data contract: source ID/content ID, trunk tile, 3x3 footprint, state, source quality, persisted regrowth cursor, last report projection tick.
- `FoundingFoodSources` validation contract: Water source/bank, Apple tree/trunk/footprint, fish habitat/shoreline, all reachable and revealed.
- Food serving/selection pure function over physical lots, manifest descriptors, permissions, need kind, current tick, and stable tie-breaks.
- Per-lot spoilage projection inputs/outputs; actual ledger mutation can be staged after the pure function is red.
- Report projection helpers for Apple/fish ecology using the Plan 1 report ladder.
- Typed food valuation helper for trade/Hole using manifest value and LAI.37 quality multiplier.

The boundary should not own:

- LAI.39 Cookhouse production recipes/station queue execution.
- LAI.40 Fishing Hut station throughput.
- LAI.41 Hole action/state machine.
- LAI.47/48/49 report UI/protocol rendering.
- LAI.61 food/divine permission policy, except consuming its decision result or manifest ID permissions.
- LAI.36 manifest parsing/validation or LAI.37 quality enum/ledger internals.

## Staged Consumer Order

1. Stabilize LAI.36 manifest consumption in red tests: food/source IDs, art keys, exact properties, and recipe outputs.
2. Stabilize LAI.37 lot consumption in red tests: no duplicate quality enum, physical lot keys/locations, conservation.
3. Add pure LAI.38 food ecology tests for manifest food table, founding-source validation, Apple state/regrowth, selection, spoilage projection, valuation, and report redaction.
4. Cut world generation/founding source placement to produce the Water/Apple/fish bundle, still without changing protocol.
5. Cut `world_tick` consumption/spoilage/regrowth from scalar `resources.food/fish/water/preserves` to typed lots and source records.
6. Cut physical cargo/recovery paths so Apple/fish/water/meal lots survive death/cancel/route/restart.
7. Cut Hole/trade consumers to typed-food value/eligibility and remove generic food/fish/preserves offering paths.
8. Cut persistence/protocol/report surfaces to lot/source/report projection data, then delete generic `Food`/`Fish`/`Preserves` aliases.
9. Attach renderer/UI assets after asset-owner receipt validation; do not block pure sim correctness on missing sprites.

## Highest-Risk Reconciliation Findings

1. `content_manifest.json` already defines concrete food IDs and exact food properties, but current runtime still consumes/spoils/stores generic scalar `Food`, `Fish`, `Water`, and `Preserves` through `Resources`, `ResourceKind`, protocol, and world tick. This is the largest duplicate-authority risk.
2. Founding currently proves nearby/reachable water and a dry bank in tests, but there is no single authority for the required Water bank + Apple 3x3/trunk + fish habitat/shoreline bundle. LAI.38 needs that source bundle before any consumption cutover.
3. Apple runtime behavior is absent: no persisted state, regrowth cursor, quality harvest, trunk-only task, 3x3 obstruction, or report-limited projection exists outside manifest/source-asset intent.
4. Fish ecology is persisted and deterministic but still exposes exact stock/capacity/cursor in protocol and mints scalar fish, which conflicts with report secrecy and LAI.37 lots.
5. Asset receipts cover only Apple low/mid/full overlays and farm dynamic sprites from the protected source; target lacks Apple overlays, Apple empty state, concrete food icons, and quality badges. Manifest art keys point to planned paths, so renderer cutover needs a separate asset delivery before production UI can be complete.
