# The Hole, Hunting, Food, and Extensible Content Integration

This document is the additive authority for integrating the design and art work from
`the-shrine-upgrade` into the completed Leader Intelligence overhaul. It preserves the new
report-limited Leader/officer planner as the only strategic AI. The source branch is a design,
domain, test, and asset source; its temporary legacy-Leader adapters and old protocol, persistence,
research UI, and root integration are not merge authorities.

Every explanation recorded here is an implementation requirement. It is not optional background.
Each behavior must have a domain rule, a real runtime path, report-safe projection, documentation,
and acceptance evidence before its board card can be `done`.

## Precedence and direct-cutover rule

This document supersedes the completed overhaul only for:

- the internal `Shrine` landmark, which is player-facing **The Hole**;
- Shrine offerings and Favor, which are replaced by Hole feeding and **Void Insight**;
- scholar Insight, which is replaced by **Research Notes** and labor-only preparation;
- generic scalar Food/Fish/Preserves storage, which is replaced by typed bulk food;
- fixed item-definition tables, which move to validated stable-ID content catalogs;
- Hunting Lairs, creature materials, related crafting, Fishing Hut, cooking, apples, and the
  supplied layered world art;
- protocol, persistence, UI, diagnostics, browser fixtures, and extension guides affected by those
  replacements.

All other Leader-AI authority, reports, beliefs, officers, task lifecycle, reservations, diplomacy,
trade, care, deterministic RNG, and multi-colony contracts remain in force.

This is a clean development cutover. There are no production players and no legacy-save promise.
The implementation must remove old Shrine/Favor/Insight/generic-food state and reset known
game-owned state on the exact old schema marker. It must not maintain dual behavior, shadow
currencies, compatibility adapters, or one-time semantic conversion.

## Integration method

The source branch has uncommitted work based on the same Git base as this worktree. Integration is
semantic, not a wholesale Git merge:

1. Preserve and import focused leaf domains, focused tests, and supplied art.
2. Rebuild root wiring against the current Leader-AI structures.
3. Never overwrite current `world_tick`, protocol root, server root/persistence, client root, or
   current research UI with their older source-branch versions.
4. Delete the source branch's temporary current-Leader Hole and Hunting adapters after their domain
   contracts have replacement commands.
5. Delete the current AI branch's `shrine_offerings` and Favor authorities after the Hole path is
   live.
6. Keep one implementation per behavior and one canonical registry per content class.

## Planner and officer ownership

The new Leader AI remains the sole decision-maker. Neither the Hole nor Hunting domain selects
strategic work.

- The Leader chooses strategic goals using reports and beliefs, not executor truth.
- The Loremaster owns Hole throughput, Hole axis research, Research Notes, scholars, and ordinary
  research proposals.
- The Captain owns dangerous Hunting Lair proposals and party recommendations.
- The Farmer owns apples, ordinary fishing, food production, and cooking supply requests.
- Workshop officers own material processing and craft queues within their existing domains.
- Cross-domain needs use typed officer requests and the existing bounded request budgets.
- A founding Leader may propose essential work while an office is vacant, using the existing
  reduced capability and higher omission rules.
- The Hole and Hunting domains validate commands and own all physical/runtime mutation.
- Player nudges affect the next ordinary planner review. They never directly feed the Hole, start a
  hunt, manufacture a task, bypass research, or override safety/authority.

The planner must behave like a local deterministic RTS director: it has many legal actions but must
choose when, where, in what order, and with which cats/resources. It maintains persistent goals,
decomposes dependencies, compares opportunity costs from beliefs, reserves physical inputs,
assigns a globally coherent workforce, monitors outcomes, and retries or revises at later reviews.

### Believable competence and mistakes

Competence is produced by better reports, effective Leader/officer levels, cadence, and scoring.
It is not produced by hidden omniscience.

- A good Leader normally selects the least damaging accepted Hole feed using believed replacement
  cost, supply confidence, downstream commitments, food-days, and expected Void Insight.
- A weak or poorly informed Leader may select scarce Apples, Fish, Meat, prepared food, or another
  expensive input because its report is stale or its valuation is wrong.
- An omission roll may cause a Leader or officer to forget to start a feed at a review. The next
  eligible review may recover; the Hole itself never stops demanding inputs.
- A bad choice is not vetoed merely because the authoritative stock is strategically scarce.
  Domain validation rejects only nonexistent, unowned, unaccepted, locked, already-reserved, or
  physically unreachable cargo. This preserves consequential poor leadership.
- If a poor feed depletes a food kind, the planner must observe the later shortage through reports,
  request recovery, and physically send cats to the correct Apple tree, fishing shoreline, Hunting
  Lair, field, or cooking station.
- Immediate defense and actual self-preservation retain the existing top priority and may preempt a
  feed before pickup. Picked-up cargo follows the explicit cancellation/salvage rules.

Village growth and the endless Hole loop are the two long-running strategic forces. The Hole has no
completion state, cooldown goal, or supernatural missed-feed punishment. Missing work loses Void
Insight, research, boosts, and momentum; it does not apply a hidden curse.

## Report parity and hidden regeneration

Gods/players see the same report projection used by the Leader and relevant officers.

- Exact authoritative regeneration, stock, respawn, ecology, catch probability, apple regrowth,
  fish habitat replenishment, and Hole timing are absent until the responsible report capability
  allows them.
- Report levels expose the already approved bands: unknown, directional state, wide range, narrower
  range, and then the allowed exact value. Exact regeneration remains unavailable before effective
  report level 4.
- The UI, action errors, logs, accessibility labels, debug strings, protocol snapshots, and browser
  fixtures must not leak hidden truth.
- Server/domain diagnostics may contain authoritative values only behind an explicit local debug
  switch and must never be serialized to a player.
- Hole feed explanations show the report references, confidence, believed cost, and rationale the
  planner actually used. They do not backfill the correct hidden answer after execution.

## The Hole domain

`cat_sim::black_hole` is the sole authority for axis state, acceptance, feed intake, reward
accounting, and physical axis-upgrade recipes. The player-facing name is **The Hole**. The stable
internal building identity remains `BuildingType::Shrine`.

### Permanent landmark and art

- The landmark is always a 5×5 footprint.
- The central 3×3 is the void/work area.
- The sixteen outer cells are a permanent paved road ring.
- Width, Depth, and Darkness never resize or relocate the footprint.
- Tasks targeting a Hole feed or upgrade project the complete central 3×3 work footprint plus the
  pinned delivery edge/slot; the road ring is route space, not an arbitrary task marker.
- The supplied `base.png` plus cumulative Width, Depth, and Darkness layers render from the
  authoritative snapshot. Axis level `n` renders layers `01..n`, not only the final layer.

### Axes

Each axis has levels 0 through 10 and ten stable research studies.

- **Width**: accepted intake units per opening are exactly `1 + width`.
- **Depth**: the maximum units in one feed order are exactly `10 × (1 + depth)`.
- **Darkness**: controls accepted resource, food, item, creature-material, and item-quality bands.
- The intake opening cadence is forty game minutes.
- One feed pipeline and one physical axis-upgrade project may be active per Hole.
- Research spends Void Insight and unlocks the next physical construction recipe. Construction
  consumes only its physical recipe; it does not charge Void Insight again.
- Upgrade recipes scale deterministically by level, require tools from level 2, Metal from level 7,
  and Gems at level 10, preserving the source-domain tables.

### Physical feed lifecycle

The Leader submits a narrow versioned command containing a believed candidate and ordered fallback
IDs. The domain:

1. revalidates the Hole, Darkness band, material/food/item processing study, amount, exact identity,
   ownership, current location, reservation state, route, endpoint capacity, and command version;
2. commits world-scoped source, route, endpoint, cargo, and cat reservations atomically;
3. creates visible child tasks with the real source, work position, delivery endpoint, route, and
   complete footprint;
4. moves exact cargo through pickup and travel;
5. consumes only cargo that reaches the Hole during an available intake opening;
6. credits one idempotent Void Insight event from exact accepted value;
7. releases or salvages every remaining reservation/cargo on cancellation, death, route loss, reset,
   or invalidation;
8. advances the opening index and publishes only report-safe state.

No scalar compatibility counter may stand in for an exact item, creature material, food kind, or
carried load.

### Reward rules

- Rewards use integer micro-Void-Insight and never floating currency.
- Raw resource value begins at 0.1 Void Insight per unit, processed value at 0.3, and Gem value at
  0.5, preserving the source branch's exact fixed-point tables.
- Items use their canonical content value, material, quality, augmentation, condition, and
  provenance value.
- Existing threat-scaled material values are: Fox Pelt 0.5, Badger Pelt 1, Bear Pelt 2, and Beast
  Core 5 Void Insight.
- Every new creature material defines an exact fixed-point Hole value in the material catalog.
- Higher drop quality multiplies value through a documented integer table; no runtime float
  calculation is allowed.
- Lifetime accepted value and lifetime Void Insight are monotonic idempotent ledgers.

## Currency and research

Favor, scalar Blessings, legacy research points, and scholar Insight are removed from the live
model.

- **Void Insight** is exact, colony-owned, nonphysical, nontradeable currency credited only by Hole
  intake. It pays for the thirty Hole-axis studies and all divine boosts.
- **Research Notes** are exact, colony-owned, nonphysical, nontradeable currency produced by
  completed scholar work. They pay for every ordinary study.
- Scholar preparation is labor-only. It does not debit Notes.
- A prepared ordinary study costs exactly 25% fewer Research Notes for a player purchase.
- Automatic Leader research never consumes preparation and pays full Notes cost.
- Automatic quotas retain the approved rolling seven-day limits and cannot purchase Hole-axis
  studies.
- Hole-axis studies cannot be prepared or discounted.
- Divine boosts retain player-only activation, exact duration/effects, same-type exclusion, and
  idempotency, but debit Void Insight instead of Favor.

### Material and production research

Research is a real capability gate:

- Raw Logs and raw Stone are the only universally available nonfood material sources.
- Concrete Apple gathering and shoreline Fish catching are founding capabilities.
- Basic food use, basic water collection, Apple gathering, and hand-fishing studies exist in the
  manifest and are pre-owned by a fresh colony.
- Every other stockpile resource, intermediate material, finished resource, creature material,
  cooking recipe, processing recipe, building, tool, fixture, and augmentation has one canonical
  study/capability.
- A material capability is global. For example, once Plank Processing is owned, the same physical
  Planks may satisfy any compatible building or station recipe; there are no shop-specific fake
  plank variants.
- A station recipe may require both the global input capability and a station/recipe study.
- Locked resources and creature drops may be found, received, or stored when their source permits
  it, but cannot be processed, crafted, installed, augmented, or fed to the Hole until their
  processing capability is owned.
- The catalog replaces the 556 interim target with a validator-derived exact total. The cutover
  test records the new total after all canonical resource, food, material, and creature studies are
  present; future content changes update the manifest and expected count together.

## Unified extensible content catalogs

All user-facing content identities use validated embedded data with stable IDs. Small Rust enums
remain only for closed behavioral classes such as equipment slot, item class, task category,
station class, effect operation, and authority domain.

The canonical catalog set contains:

- resources and processing capabilities;
- food kinds and nutrition/spoilage/value data;
- item definitions, base material definitions, equipment slots, tool functions, furniture/fixture
  functions, and art keys;
- creature species, encounter bands, stats, common loot, primary material, and art keys;
- creature materials, processing requirements, tags, Hole gates/values, and quality effects;
- station and cooking recipes;
- augmentations and building fixtures;
- research nodes and capability payloads.

Every content record has a stable lowercase ID, display name, bounded description, content class,
research/capability references, fixed-point values, deterministic ordering key, and protocol art
key where applicable. Cross-catalog validation rejects duplicates, dangling IDs, cycles,
unreachable studies, invalid numeric bands, missing art references, recipes without stations,
materials without uses, foods without nutrition, and behavior tags without a handler.

### Exact item and material identity

- `ItemInstance` retains a stable unit ID, definition ID, owner, physical location, quality,
  durability, equipped/reserved/cargo state, and optional single augmentation.
- An augmentation consumes one exact processed creature-material unit and attaches to one eligible
  exact item without replacing its item ID.
- Each eligible item has at most one typed augmentation slot.
- Buildings/stations have at most one typed fixture slot. An installed fixture references the exact
  crafted item ID, persists, wears on relevant cycles, and may be repaired, replaced, recovered, or
  destroyed through physical rules.
- Equipped, carried, reserved, broken, or incompatible items cannot be augmented.
- Cancellation never duplicates or silently deletes the base item, augmentation, fixture, or input.
- Common and creature materials are exact physical lots/units with stable identity once they enter a
  reservation/cargo/processing path; aggregate views are derived projections.

### Curated creature-material uses

The first integrated content line includes all of these:

- Tannery processing of raw pelts, hides, membranes, scales, eyes, hearts, wings, feathers, antlers,
  tusks, fangs, barbs, and cores into usable components;
- treated-pelt and membrane clothing at the Clothier;
- pelt, antler, tusk, bone, feather, and scale furniture/fixtures at Woodworking;
- weapon, armor, and tool augmentations at the Smithy/Workshop;
- advanced lenses, microscopes, and research instruments at the Workshop and Research Hut/School;
- Hole feeding after both the material-processing capability and Darkness gate permit it.

Existing stations are reused. No duplicate generic “cloth workshop” is added. A new Fishing Hut is
added because it owns a distinct shoreline workflow.

## Typed food, hunger, apples, fishing, and cooking

Generic scalar `Food`, `Fish`, and `Preserves` storage is removed. Food is a category over concrete
stable `FoodKind` records stored in a deterministic bulk ledger.

Each food kind defines:

- stable ID/display/art key;
- nutrition per unit in integer nutrition points;
- hydration contribution if any;
- spoilage lifetime and preserved state;
- unit weight, storage class, trade/Hole value, and ingredient tags;
- raw/cooked safety and required capability;
- recipe and output references.

Initial concrete foods include Apples, raw Fish, raw Meat, the existing five baking outputs, cooked
Fish and Meat, dried/smoked/pickled foods, travel rations, and a feast. Existing recipes that
formerly emitted generic Food/Preserves must emit named food kinds.

- Hunger drains nutrition points, not undifferentiated food units.
- Consumption chooses the earliest-spoiling usable food first, then the smallest unit that satisfies
  remaining hunger, then stable food ID. This minimizes waste deterministically.
- Food-days in reports are derived from believed nutrition, population, and observed drain.
- Stockpiles and Food Storage accept the food category or selected concrete kinds.
- Trade, cargo, Hole feeds, tasks, reports, and UI preserve the concrete food ID.
- Cooking is physical station work with real ingredients, tools, progress, output capacity, hauling,
  interruption, and spoilage.

### Apples

- Apple trees are real revealed world sources and use the supplied low/mid/full fruit overlays.
- A `Gather Apples` task targets the exact tree tile. It cannot appear on arbitrary ground.
- Harvest credits physical Apples and visibly lowers/removes the fruit layer.
- Regrowth is deliberately slow, deterministic, persisted, and advanced once per world tick.
- Exact regrowth remains hidden until the responsible officer report reaches the approved level.
- Apples may be eaten raw and used in baking, preserves, brewing, trade, and accepted Hole feeds.
- Apple source depletion can force a poor Leader to find Fish, Meat, or another food path.

### Fishing

- Fish are catchable from the start at a real revealed shoreline work tile adjacent to a persisted
  water habitat.
- A hand-fisher without a Fishing Hut or rod has a deliberately low deterministic catch efficiency
  and longer work cycle. It is difficult, not impossible.
- A Fishing Hut is a shoreline building with a canonical 3×3 land footprint and a pinned dock/water
  orientation. Invalid nonshore placement is rejected.
- Fishing jobs remain marked at the actual shoreline/water source, never at an arbitrary village
  tile or only at the Hut.
- An exact credited, unbroken Fishing Rod provides an independent catch/cycle modifier and wears
  only while fishing.
- An operational staffed Fishing Hut provides an independent coordination/storage modifier.
- Hut plus rod provides the full combined benefit; neither modifier is silently implied by the
  other.
- The supplied boat, land-dock, and water-dock art renders the authoritative orientation and
  activity state.
- Fish habitat stock, replenishment, difficulty, and catch chance use the existing finite ecology
  authority and report redaction.

## Hunting Lairs and creatures

`cat_sim::hunting_lair` owns deterministic rosters, danger, combat, respawn, loot, first-clear
guarantees, and XP. `attempt_hunting_lair` remains the sole dispatch boundary.

- `EnemyLair` is a Hunting Lair; `CaveEntrance` is the Quarry. They never alias.
- Hunt tasks target the exact revealed lair tile and use the lair art; Quarry tasks target the exact
  cave tile and use the quarry art.
- The Captain may recommend; the Leader submits a party command; the domain revalidates everything.
- Autonomous attempts require at least 70% predicted success and at least 70% health for every
  hunter.
- A player nudge lowers only predicted success to 45% and raises minimum health to 80%.
- `hunting_bulk` remains the stable ID and displays as **Hunting Parties**, allowing party cap
  three.
- Exact equipped weapon/armor/tool identities determine bonuses and wear once on resolution.
- Success awards Hunt and Fight XP. Failure may injure or kill and still conserves every item/cargo.
- Respawn persists one absolute `respawn_ready_at_ms` computed with the attacking colony's
  game-hour duration.
- Rosters, public outcomes, materials, trophies, nudges, and deadlines are world state outside the
  Leader fingerprint.
- Overflow uses a visible one-tile hunting cache at the lair.

### Creature roster and encounter bands

The initial validated registry contains ten normal animals and ten mystic creatures:

| Tier | Species | Encounter levels | Primary named drop |
|---|---|---:|---|
| Normal | Cave Bat | 1–8 | Bat Wing |
| Normal | Red Fox | 5–18 | Fox Pelt |
| Normal | Badger | 10–24 | Badger Pelt |
| Normal | Wild Boar | 16–30 | Boar Tusk |
| Normal | Gray Wolf | 22–36 | Wolf Pelt |
| Normal | Lynx | 28–42 | Lynx Pelt |
| Normal | Great Stag | 32–46 | Stag Antler |
| Normal | Giant Serpent | 36–50 | Serpent Scale |
| Normal | Brown Bear | 40–54 | Bear Pelt |
| Normal | Great Eagle | 44–60 | Eagle Feather |
| Mystic | Moon Stag | 40–60 | Moon Antler |
| Mystic | Warg | 46–66 | Warg Fang |
| Mystic | Cockatrice | 50–70 | Cockatrice Eye |
| Mystic | Forest Troll | 56–76 | Troll Hide |
| Mystic | Griffin | 62–82 | Griffin Plume |
| Mystic | Basilisk | 68–88 | Basilisk Scale |
| Mystic | Manticore | 74–92 | Manticore Barb |
| Mystic | Chimera | 80–96 | Chimera Heart |
| Mystic | Wyvern | 86–99 | Wyvern Membrane |
| Mystic | Elder Dragon | 95–100 | Dragon Heart |

Levels 1–39 use normal creatures. Levels 40–60 may mix normal and mystic creatures. Levels 61–100
contain at least one mystic creature but may include normal supporters. Deterministic roster size is
one at 1–19, one or two at 20–39, two at 40–59, two or three at 60–79, three at 80–94, and a boss
plus two supporters at 95–100.

Every species has exact body-size-scaled Meat, Bone, and optional Hide yield. Common yield is not a
flat shared number: an Elder Dragon must provide dramatically more Meat and Bone than a Cave Bat.
The registry owns exact per-species base amounts; encounter level and roster count use integer
scaling without allowing zero or overflow.

### Drop quality

Primary named drops use a deterministic keyed quality roll from 0 through 4:

- encounter 1–24: quality 0;
- 25–49: quality 0–1;
- 50–69: quality 1–2;
- 70–84: quality 2–3;
- 85–94: quality 3–4;
- 95–100: quality 4.

The key includes world seed, lair ID, roster generation, species ID, and clear index so input order,
restart, unrelated colonies, and batch size cannot change it. First clear guarantees the strongest
creature's primary material at the band floor if normal loot produced none.

Future creatures are additive catalog records. They must define level band, tier, body size, stats,
common loot, primary material, material study, recipes/uses, Hole gate/value, art, deterministic
tests, protocol projection, and documentation.

## Spatial and visible-task contract

The existing no-fallback contract remains absolute.

- Apple gathering is on the Apple tree.
- Fishing is on a valid shoreline work tile bound to an adjacent water habitat.
- Hunting is on the `EnemyLair`.
- Quarrying is on the distinct `CaveEntrance`.
- Fetch Water retains source water, dry-bank work position, route, and delivery endpoint.
- Hole work owns the central 3×3 and pinned delivery position within the permanent 5×5 landmark.
- Workshop/station work owns the complete canonical 3×3/nine ordered cells, not one center marker.
- Fishing Hut construction/operation owns its complete 3×3 footprint plus oriented dock attachment.
- Farm work renders the complete field footprint and correct crop stage.
- Tree work renders the exact occupied tree cells and fruit state.
- Missing site, route, footprint, capability, storage, tool, or worker blocks explicitly without a
  false marker or busy cat.

## Protocol, server, and persistence

- Bump the protocol from 2 to 3.
- Bump the Leader-AI snapshot schema from 1 to 2.
- Add versioned Hole, Void Insight, Research Notes, typed food, content-manifest, Hunting Lair,
  creature-material, item augmentation, fixture, Fishing Hut, and task projections.
- Add stable `NudgeBlackHole` and `NudgeHuntingSite` actions as planner hints.
- Add exact actions for ordinary research purchase, labor-only preparation, Void-Insight boost,
  material processing, cooking, augmenting an exact item, installing/removing a fixture, and
  Fishing Hut placement/queueing.
- Every mutation carries bounded idempotency plus exact affected version lanes.
- Action validation preserves authentication, colony ownership, authority, version, research,
  physical preconditions, reservation, mutation, persistence, and report-safe response ordering.

Because this is pre-production, persistence performs a known-version clean reset:

- retire/remove Favor, Blessings, Insight, old Shrine-offering, generic Food/Fish/Preserves, and
  source-branch legacy-adapter state;
- remove their old migration/conversion authority and tests;
- when the exact previous schema marker is found, transactionally clear only game-owned world,
  colony, aggregate, receipt, and fixture rows, install the new marker, and create a fresh world;
- do not delete the SQLite file, authentication identities, or unrelated server metadata;
- log the reset reason and old/new schema without secrets;
- fail closed on unknown future or malformed partial schemas;
- regenerate the committed browser SQLite fixture, manifest, checksum, and protocol/schema metadata.

Hole, Hunting, food, content, item, and fixture state remain separate versioned aggregates where
that preserves domain ownership. None is hidden inside the Leader fingerprint.

## Client and supplied art

The current Leader-AI Bevy UI remains the integration root. The source branch's deleted/older
`research_ui.rs` is not restored.

- Replace Shrine/Favor UI with a Hole panel showing report-safe axes, intake state, believed
  candidate/rationale, active physical feed, next accepted bands, Void Insight, lifetime totals,
  upgrade research/construction, and nudge.
- Show Research Notes, ordinary frontier, preparation labor, 25% prepared player price, automatic
  quota, and separate Void-Insight axis/boost spending.
- Add Hunting panels for revealed lairs, report-safe danger, party recommendations, health/success
  gates, roster when known, outcomes, common loot, named materials, quality, respawn band, and
  nudge.
- Add typed Food inventory/nutrition/spoilage, Apple sources, Fish habitat reports, cooking queues,
  Fishing Hut/rod effects, and food-days explanations.
- Add exact material inventory, processing study, recipes, augmentable targets, installed fixtures,
  and physical task/cargo state.
- Render the supplied Hole layers, lair/quarry distinction, dynamic farm stages, Apple tree
  low/mid/full overlays, Fishing Hut dock/boat, and rail cart through the reusable layered-sprite
  renderer.
- Preserve accessibility roles, bounded stable test IDs, keyboard/AccessKit controls, stale-action
  refresh, selected-colony context, and report secrecy.

## Diagnostics and serialized testing

Testing must not overload the workstation.

- Only one heavy build/test/browser command may run at a time.
- Use `CARGO_BUILD_JOBS=1`, `taskset -c 0-3`, and `--test-threads=1`.
- Workers may edit independent leaves in parallel, but only the coordinator grants the single test
  slot.
- Browser acceptance uses Portless and one Playwright worker after Rust gates.
- The full release campaign and Forgejo shards remain external serialized/publication gates.

Opt-in diagnostics add bounded records for:

- tick/phase enter and exit with elapsed time;
- planner review, officer report references, omissions, candidate scores, chosen/fallback IDs;
- Hole command validation, reservation, cargo stage, intake opening, credit event, salvage;
- Hunting roster key, recommendation, gate rejection class, resolution, wear, loot, respawn;
- typed-food acquisition, spoilage, nutrition selection, cooking, Apple regrowth, fishing effort;
- content ID and research/capability rejection;
- server version/idempotency/persistence boundaries;
- campaign progress and terminal/liveness cause.

Logs must never emit authentication material or player-hidden truth unless an explicit local
authoritative diagnostic switch is enabled. A slow probe must print periodic bounded progress so a
long computation is distinguishable from a deadlock.

### Acceptance groups

1. Catalog validation, stable IDs, order twins, future-extension fixtures.
2. Hole domain tables, exact identities, fixed-point rewards, upgrade recipes, interruption, and
   restart.
3. Planner hidden-truth twins, good/bad feed choice, omission, recovery, and no safety veto from
   hidden stock.
4. Report/god secrecy for regeneration, ecology, respawn, and rationale.
5. Twenty-species roster/band/mixing, quality, body-size loot, first-clear, safety gates, wear,
   injury/death, cache, and restart.
6. Typed food nutrition/spoilage/consumption, Apple depletion/slow regrowth, hand-fishing versus
   Hut/rod modifiers, cooking, and conservation.
7. Material processing, every locked capability, global Plank use across stations, full curated
   crafting, single augment/fixture slots, and cancellation conservation.
8. Exact spatial markers/footprints for Hole, lair, quarry, water, Apple, Fish, farms, Fishing Hut,
   and all 3×3 stations.
9. Protocol v3 strict round trips/old-client rejection, server action ordering, clean-reset
   persistence, restart, multi-colony isolation, and regenerated fixture.
10. Client projections, accessibility interactions, layered art, one-worker Playwright, and an
    independently operated visible browser after implementation.

## Extension documentation requirements

The maintained contributor guides must contain copyable procedures for:

- a new creature and encounter band;
- a new creature material, quality effect, processing study, recipe, augmentation, fixture, and
  Hole acceptance/value;
- a new food kind, nutrition/spoilage rule, source, cooking recipe, and art;
- a new resource/capability;
- a new item definition or behavior class;
- a new station/workshop, including canonical footprint, task roles, recipes, staffing, research,
  persistence, protocol, UI, art, and browser checkpoint;
- a new layered landmark or world source;
- a new planner goal/officer request without leaking executor truth.

Each procedure must include stable IDs, deterministic ordering/RNG, authority, reports/redaction,
physical identity/conservation, complete spatial metadata, research dependencies, version lanes,
persistence/default/reset behavior, tests, diagnostics, browser evidence, and board/document
touchpoints.
