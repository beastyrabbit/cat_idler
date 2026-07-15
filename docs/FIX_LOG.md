# Idle Cat Forest fix log

This is the maintained, evidence-backed log for fixes found during design review and player-guided
or unattended playtesting. The queue records reproduced problems before work begins. Move an item
to the verified log only after its behavior, persistence boundary, relevant Rust quality gates,
and any changed Bevy visuals have been verified.

## Open fix queue

| Finding | Required correction | State |
| --- | --- | --- |
| Carried cargo lacks semantic visual identity | The DF-readability pillar requires a player to recognize what a moving cat is hauling. Materials and Blessings have dedicated carry icons, but most other `CarryingKind` values still fall back to resource-colored squares, and the declared Stone mapping is not loaded. Give every physical cargo kind an exhaustive, tracked semantic glyph without borrowing terrain, farm, furniture, or world-prop art. Verify representative distinct loads in an exact 1024x768 client-owned framebuffer on native and WASM. | queued (high priority) |
| Six maintained benches still bypass the physical station contract | Wood Cutter, Stone Prep, Woodworking, Clothier, Tannery, and Smithy still consume colony-global aggregates and/or advance parallel hidden cycles. Their C2.0 descriptor scaffold is live: stable recipe IDs, canonical resource domains, deterministic default queues, exact catalog availability, selected-recipe block reasons, rules-v0 grandfathering, and generic snapshot/persistence fields exist while `aggregate_timer_compatibility` truthfully marks the unchanged behavior. Next give each station local input/output/transit state and one-worker/one-selected-recipe advancement. Preserve every existing `BuildingType`, queue identity, and open-top visual identity. | in progress |
| Functional equipment has two incomplete authorities | Stable `tools`, `weapons`, and `armor` scalar fields coexist with finite weighted item units, while finished functional equipment recipes are incomplete. Keep the scalar IDs readable for old saves during migration, make finite item instances the eventual identity/condition authority, and prevent crafting, wearing, repair, trade, or stockpile projection from double-counting one object. | queued |
| Most building-capacity studies still have no physical storage domain | Food Storage, Water Bowl, and Smithy capacity studies are live and target-correct. The other 22 generated `*_stores` studies remain deterministic no-ops because those buildings do not own a modeled storage domain. Mill, Sawmill, Workshop, and Smelter station-local input/output stores also remain fixed at 10 rather than consuming capacity research. Model a real physical domain before activating each remaining study. | queued |
| Research recipe/resource breadth remains incomplete | Eleven maintained recipe IDs now have data-owned station descriptors and exact catalog ownership metadata. The four physical queue recipes plus four existing aggregate textile/Smithy recipes enforce their rules-v1 entitlement before work; old/missing rules-v0 saves are grandfathered. Carpentry Staples, Stonecraft Preparation, and Toolmaking Preparation own the new Wood Cutter, Stone Prep, and Woodworking descriptors, but those behavior-neutral queue entries do not yet gate their compatibility aggregate timers. The other 93 generated recipe IDs and all 64 generated resource IDs still have no authoritative consumer. Add only sourced resource/recipe breadth. | in progress |
| Worker-slot studies have no staffing consumer | Twenty-five `worker_slots +1` effects resolve, but buildings, persistence, automation, protocol, and UI still support exactly one assigned cat. Implement real multi-worker ownership and physical work before presenting these studies as effective. | queued |
| Shared terrain is duplicated per colony | Terrain, ecology, roads, wear, depletion, and fish are colony-owned, so two villages at the same coordinates do not inhabit one authoritative mutable world. Move canonical spatial state to world scope while keeping fog and learned contact private. | queued |
| Inter-village trade is nonphysical | Contact summaries and atomic scalar barter exist, but cats do not meet, carry items, form caravans, or travel trade routes. Preserve knowledge-blind scouting and shrine-return discovery while adding physical exchange. | queued |
| Fine-biome resources and transport are incomplete | Bone now has a finite physical hunt source, storage/trade/persistence/HUD identity, and conserved final haul, but its crafting variants remain incomplete; Gem, clay, and sand still lack complete physical sources/chains. Rail and Shipping now grant blueprint entitlements only: they deliberately do not alter ordinary walking pathfinding or speed. Build tracks, rolling stock, docks, vessels, boarding, and staffed routes before activating transport effects. | queued |
| Authored road building is instant | `actions::build_road` validates a shrine-connected mapped path, paints every new tile immediately, and subtracts one aggregate Material per tile without a cat, cargo, or construction phase. Preserve the verified placement, surface, connectivity, and speed rules while adding a physical build route if roadwork is promoted to the full DF-style logistics contract. This is a P2 physical-consistency enhancement, not a blocker under the current P16 wording. | queued (P2) |
| Wall art needs a stronger top-down treatment | The staged palisade, closed perimeter, single-gate cutover, collision, and visual campaign are verified, but the current wall selection does not meet the player's requested quality bar. Compare the tracked public candidates in `docs/sprite-review.html`, select a coherent top-down wall and gate set, and preserve exact autotiling, staged-work color, gate placement, and authoritative collision. Re-verify native and WASM framebuffers before replacing the current art. | queued (low priority) |

## Verified fixes

## 2026-07-15 — All 25 HUD resources use semantic tracked icons

**Problem:** Thirteen resource rows still borrowed map terrain, farm stages, furniture, world props,
or the generic-goods glyph. Stone and Bone were especially misleading: raw Stone reused its map
pile and Bone reused a catch-all sack, despite the HUD contract requiring an icon readable before
the adjacent label.

**Fix:** Every maintained resource now resolves to a unique PNG under
`public/images/game/icons/`. Stone uses the tracked block/ingot glyph and Bone uses Kenney Fish
Pack's 128 px blue fish-skeleton glyph. The remaining substitutions were replaced with the
documented Board Game/Fish symbols for Fish, Catnip, Grain, Flour, Logs, Lumber, Fibre, Hide,
Cloth, Leather, Ore, and Metal. The client keeps the distinct labels, values, and resource-specific
tints while an exhaustive test locks the 25-entry protocol/HUD bijection and rejects paths outside
the semantic icon directory.

**Evidence:** All 135 `cat-client` tests, strict all-target Clippy, formatting, and diff checks
pass. The client's own exact 1024×768 framebuffer at `/tmp/semantic-hud-25-final.png` shows all
25 icon/value rows within the parchment panel without overlap or clipping.

## 2026-07-15 — Remaining stations own behavior-neutral recipe descriptors

**Problem:** Wood Cutter, Stone Prep, Woodworking, Clothier, Tannery, and Smithy had no common
recipe/queue/resource-domain authority to receive the physical station contract. Moving each route
directly would have repeated IDs, research maps, and block-reason logic while risking an accidental
change to established aggregate production.

**Fix:** One data module now owns all eleven maintained station recipe IDs and canonical resource
sets. The six remaining benches receive deterministic default queues, generic signed queue and
persistence state, exact catalog-derived rules-v1 availability, rules-v0 grandfathering, and
selected-recipe snapshot/block metadata. Stone Prep's descriptor consumes the new raw Stone kind;
Materials remain Supplies. The compatibility benches report `aggregate_timer_compatibility`, and
an exact six-bench twin proves queue order, empty state, and pause state do not alter their existing
aggregate timers. Additive trade-craft timers remain outside this bounded descriptor slice.

**Evidence:** Descriptor uniqueness/resource-domain tests, exact catalog reverse-index tests,
locked/owned/grandfathered block-reason tests, multi-recipe Smithy selection, signed snapshot and
protocol round trips, all 1,175 executed simulation tests (one intentional skip), all 44 protocol
tests, strict sim/protocol Clippy, formatting, and diff checks pass. This verifies C2.0 scaffolding
only; station-local cargo/work/output conversion remains open above.

## 2026-07-15 — Raw Stone and source byproducts are finite physical cargo

**Problem:** Quarry rock was still credited through the stable generic `materials` field, Stone
Prep dressed that same Supplies pool, and quarry/hunt byproducts could appear as aggregate stock
without a carried load. Adding Stone naively would also have stranded the renewable Supplies route
used by the Workshop and shrine offerings. Partial logging had a related conservation hole: the
tree was marked felled only on the final trip, so an interrupted early haul could target it again.

**Fix:** Stone is now its own defaulted save, wire, storage, trade, stockpile, cargo, HUD, and
Accountant-projection field. Missing legacy Stone defaults to zero while existing `materials`
remains bit-exact Supplies. Quarry workers carry three raw-Stone loads and one distinct positive
rubble/Supplies load; only a persisted Mountains site adds a fifth Ore load. Hunts now carry three
Food loads followed by separate Hide and Bone loads. Bone has its own defaulted save, wire,
storage, stockpile, cargo, trader, HUD, and private Accountant identity. Stone Prep consumes only
Stone. Every result credits aggregate stock only
after finite delivery, and the loaded-site Ore manifest avoids terrain generation in the active-job
hot path. The first positive logging extraction now writes the stump immediately, while an active
job reservation prevents premature replant/retarget until the job ends.

**Evidence:** Legacy JSON and SQLite migration, full persistence, trade depletion, stockpile
capacity, death/cancel/full-storage conservation, private Accountant reports, ordinary-versus-
mountain manifests, logging interruption/retry, and deterministic unattended and signed guided
campaigns cover the boundary. The guided fresh village begins at exactly zero Stone, physically
quarries it, and grows Blocks without debiting Supplies. The inspected client-owned framebuffer
`/tmp/raw-stone-bone-final.png` is exactly 1024×768, renders the top-down village normally, and
shows truthful counted Stone `~12/100` and Bone `~3/100` rows with distinct icons and no clipping.
The final gates pass 1,169 simulation, 43 protocol, 82 server, and 134 client tests plus strict
Clippy for all four crates. Bone item variants and downstream recipes remain open breadth, but the
raw hunt source and scalar are no longer future work.

## 2026-07-15 — Guided founding economy reaches farms, offerings, and bounded route planning

**Problem:** Broad player-guided and unattended runs exposed three integration failures hidden by
focused fixtures. The material-offering decision protected too much Supplies to ever reach its own
pickup threshold, the Farmer could treat a promised but unpaid Field as satisfying the essential
food floor, and the guided farm→Mill smoke sometimes appointed a Steward before demonstrating the
vacant-office manual road dependency. Repeated exact construction-road A* probes also dominated
long fishing campaigns even when village topology had not changed.

**Fix:** The offering decision bar is 20 Supplies: ten for the carried offering and ten retained for
operations. Essential Field demand now counts only completed physical Fields and reopens Supplies
for a fundable plank bill; comfort can stop only discretionary Fields after the minimum floor.
Non-sticky raw benches restaff while buffers are deficient without reserving dormant offerings.
The signed campaign now plans and manually paves a second Workshop while the Steward seat is vacant,
asserts the visible paved-road event immediately, appoints the Steward afterward, and continues to
physical Field harvest and Mill delivery. Construction access-road candidates are cached by a
deterministic topology signature, bounding unchanged exact route probes without caching mutable
claims or future sites.

**Evidence:** Five unattended 200-game-hour founding-economy seeds each produce tithes, one physical
offering, and blessings while the established population matrix retains its essential Field floor.
The signed pre-earned-research replay twin purchases its dependencies, lays the manual road, harvests
a staffed Field, and delivers both Flour and Food through the Mill bit-for-bit. Guided and unattended
fishing campaigns cover all maintained seeds with at most eight exact construction-road queries;
the authoritative full simulation gate and final touched-crate gates are recorded on P19.C1.

## 2026-07-15 — Visiting traders are physical and finite

**Problem:** Visiting merchants were a timer-backed menu with effectively infinite stock. They did
not walk into the village, own conserved cargo, wait behind a closed route, or visibly leave.

**Fix:** Each visit now owns a deterministic reachable exterior, follows ordinary obstacle-aware
A* through the retained gate to physical shrine contact, trades only while present, and returns to
a still-valid exterior before despawning. The merchant carries a finite deterministic resource
manifest, finite purse, 100 kg wagon, and exact stable item units. Purchases deplete exact stock and
show sold-out truth; sales preserve item identity and stop at purse or cargo capacity. Phase, route,
deadline, inventory, and cargo persist across restart. Expansion rehomes an exterior that becomes
claimed. Exact transition times agree across one-second, minute, hourly, and coarse tick partitions
without granting backdated travel after a blocked boundary. The bounded panel pages every offer and
uses only Accountant reports when explaining storage blocks.

**Evidence:** Focused route, passability, closure/reopen, shrine-contact, cadence, rehoming,
conservation, sold-out, SQLite restart, signed buy/sell/depletion/denial, protocol, pagination, and
privacy tests pass. A deterministic seed-41 live-cadence 60-hour passive twin observed arrival,
the complete shrine trading window, and physical departure without deaths or reset. The accepted
client-owned 1024×768 logical framebuffer `/tmp/trader-physical-1024.png` shows the merchant at the
shrine, page 2/2, finite quantities, Food sold out, bounded controls, and report-derived storage
guidance without exposing exact private headroom. The first broad gate exposed a blocked-route
timestamp regression that focused tests had not exercised: reopening could reuse elapsed time from
the unavailable window and collapse arrival, trading, and departure into one coarse call. A
defaulted persisted blocked-route marker now gives the reopened route only the new tick's movement
while timestamping contact/departure at that observation boundary. Both exact reopen guardrails,
all 23 trader tests, the full 1,153 simulation tests, all 80 server tests, strict touched-crate
Clippy, formatting, and diff checks pass.

## 2026-07-15 — Foresters physically replant felled trees

**Problem:** Logging converted every generated tree into a permanent stump. The maintained design
assigns felling, replanting, and lumber to the Forester, but there was no replant job, growth clock,
or visual/persisted route back to the deterministic forest.

**Fix:** `ReplantTree` is one signed manual order and one Forester-owned automation job. Manual
orders remain available while the office is vacant; the founding Leader never plants. A worker
must have a real route from the shrine, walk to the exact mapped/revealed stump, accept it, and
complete thirty game-minutes on site. The finite input is that stump's surviving coppice/root
stock: completion atomically changes the existing persisted tile from `stump` to `sapling` and
records `planted_at` in its existing `last_depleted` clock without inventing a seed resource.
After 24 game-hours the sapling restores the same seed-derived mature tree. Buildings, farms,
stockpiles, agriculture, water, mountains, roads, perimeter walls, village interiors, and rocks
delay growth without deleting it. Stumps and saplings cannot be logged as mature trees.

**Player visibility and evidence:** Snapshots expose separate stump/sapling anchor sets; Bevy hides
the mature canopy and renders tracked top-down stump/sprout art until growth completes. Manual and
officer campaign tests cover vacancy ownership, bounded automation, reachable-site denial,
arrival-controlled work timing, death/cancellation, finite mutation, delayed and obstructed retry,
reveal-clock idempotence, cadence partition determinism, SQLite job/tile restart, and the final
`overlay = None` becoming the exact original generated logging target again. The integrated full
gates pass 1,176 simulation, 43 protocol, 82 server, and 135 client tests. The later conflict-free
recipe-descriptor rebase passes 18 focused Forester/source-cargo/descriptor tests, all 44 protocol
tests, strict four-crate Clippy, formatting, and diff checks. The accepted own-framebuffer capture
used temporary render-only
lifecycle anchors inside the visible village to
isolate stump/sapling readability and canopy suppression; it does not imply that simulation-valid
replanting or regrowth may bypass the authoritative village-interior and occupancy safeguards.

## 2026-07-15 — Accountant reports are the player-wire inventory boundary

**Problem:** The client displayed the Accountant's stale aggregate and per-pile reports, but the
same owner WebSocket payload also carried authoritative colony resources, exact pile contents,
and aggregate/per-pile `accurate` comparisons. Reading JSON bypassed every physical counting round.

**Fix:** The server retains its exact completed snapshot cache, then applies one mandatory socket
projection for initial connections, broadcast ticks, and post-action/reconnect snapshots. Physical
`resources`, the duplicate threat weapon/armor totals, and every visible pile's `contents` now come
from the last report; uncounted piles project zero contents. Equality attestations are cleared and
omitted. Exact blessings remain visible because they are a non-stockpiled divine currency, not part
of the Accountant's physical inventory remit. New offer/block metadata must not copy an exact
resource total or reintroduce an equality oracle; whole-payload sentinel tests guard numeric leaks.

**Evidence:** Authenticated personal-owner tests cover vacant and blocked books, a wholly uncounted
pile, exact trusted-cache preservation, initial cached delivery, tick broadcast, signed post-action
refresh, and reconnect. Unique authoritative sentinels are absent from the complete serialized JSON,
not merely from the documented resource fields. Protocol compatibility defaults omitted accuracy
fields to conservative `false`; signed Accountant restart/release coverage remains green. The full
42 protocol, 76 server, and 130 client suites plus strict touched-crate Clippy and formatting pass.

## 2026-07-15 — Scaffold construction uses conserved physical inputs

**Problem:** Exact and autonomous construction subtracted Planks/Blocks from aggregate resources
when a scaffold appeared. No cat fetched the pinned bill, construction could progress without a
delivery, and restart/removal/death had no durable source→transit→scaffold accounting.

**Fix:** Placement now atomically reserves the exact type-local escalating bill from finite visible
piles, preferring Lumber and filling any shortfall with Planks. One living builder carries bounded
loads from each deterministic source into persisted transit and scaffold-input stores. The build
timer remains absent and progress stays zero until every pinned unit is physically delivered, at
which point the exact input is consumed once. Reservations protect the bill from crafting, repair,
trade, gather-spot movers, Steward balancing, station hauling, and the late-removal recovery
window. Reconciliation drains ordinary surplus before reserved sources or exact scaffold inputs;
aggregate restoration is limited to construction-local physical goods as legacy corruption safety.
Death spills at the cat's real tile; source loss replans to recovered visible goods; cancellation,
reassignment, scaffold removal, and restart conserve every unit. Legacy incomplete scaffolds with
no contract remain grandfathered as already funded.

**Evidence:** Player and Leader placement, split multi-pile Lumber/Planks fallback, partial loads,
no-early-progress, source loss, death/reassignment, removal and same-window spend protection,
blocked empty-paw source and loaded return routes, ready-input replacement arrival, pinned speed
across mid-build research/reassignment, deterministic one-/five-second cadence, SQLite
mid-haul/legacy restart, signed HMAC completion, and client inspector tests pass. The selected-
scaffold inspector always exposes required, delivered, in-transit, and blocked/building stage.
The final gates passed 1,139 simulation, 42 protocol, 75 server, and 130 client tests plus strict
Clippy for all four crates. The accepted live 2048×1152 own-framebuffer
`/tmp/scaffold-physical-inputs.png` shows the intact Grand Commons world and selected open
Woodworking scaffold at 0% with Lumber/Blocks required, partial Lumber delivered, no cargo in
transit, and its truthful blocked state.

## 2026-07-15 — Founding benches no longer require Basic Tools for placement

**Problem:** Wood Cutter, Stone Prep, and Woodworking are fixed P16 founding benches whose later
copies remain placement-available, but signed construction had a second hard-coded `basic_tools`
check after the catalog resolver. That study models hunt yield, not station placement, so a fresh
personal or communal village could see each bench in its blueprint yet could not request another
copy.

**Fix:** The duplicate action-layer gate is removed. The three exact building-family records now
declare `availableAtFounding` in catalog data, producing one non-purchase
`BuildingAvailableAtFounding` marker alongside each first durability modifier. The marker changes
neither study prerequisites, cost, order, category, modifier, nor daily Leader choice. Missing and
owned `basic_tools` states therefore have identical placement behavior; future research remains a
recipe entitlement, not permission to place another bench.

**Evidence:** Catalog validation pins one founding source and no `UnlockBuilding` competitor for
each bench while retaining the exact 167 Building / 167 RecipeResource / 166 Upgrade split and the
first durability payload. Deterministic signed personal/communal plans, exact scaffold payment,
reservation/overlap and mutation-free denial, authenticated server HMAC, and SQLite restart tests
cover placement without altering current aggregate production. The remaining six station-local
queues and finite-equipment authority work stays open below.

## 2026-07-15 — Prosperity migrants physically enter and leave through the gate

**Problem:** Prosperity migration created or removed cats directly in colony state. A new migrant
could count as a resident, consume stores, take work, vote, fight, or claim housing before walking
into the village; an expired unhoused migrant disappeared without releasing work or visibly
leaving. The client therefore showed a probation countdown without a truthful physical journey.

**Fix:** Each cohort now receives one deterministic dry/passable exterior origin near the south
gate. The origin and `Arriving`/`Probationary`/`Departing` phase persist in the existing migration
JSON, with legacy records defaulting to already-present probationers. Authoritative A* movement
alone carries cats through the current gate, so a blocked route waits and resumes without
consuming the cohort, Shipping never permits water walking, Rail remains neutral, and gate
relocation uses the new wall topology. Only physical entry emits `MigrationArrived` and starts the
36-game-hour housing clock. Expiry conserves cargo, cancels jobs and role ownership, then routes
the cat back to its persisted origin; only physical exit removes it and emits
`MigrationDeparted`. Arriving/departing cats are visible but excluded from needs, survival,
labor, research, rituals, officer dispatch, elections, combat, housing, and signed assignments.

**Evidence:** Focused migration, exact water/mountain passability, 120-second blocked-route
performance/reopen, moved-gate, cadence partition, cargo/job recovery, death/reset, legacy serde,
SQLite mid-arrival/mid-departure restart, protocol/client status, personal guided Den retention,
signed server guided-vs-unattended, and three-seed organic 150-game-hour campaigns pass on the
integrated transport/entitlement/Accountant base. The exact live-cadence 48-hour passive run kept
all 15 cats healthy while completing hunt, water, Leader scout, shrine-delivery, and fog-expansion
loops. The final touched-crate gate passed 1,363/1,363 tests with one intentional skip; strict
Clippy and formatting passed. An inspected 3,826×2,105 client framebuffer selected an arriving
cat walking through the south gate and showed the truthful `MIGRATION: ARRIVING THROUGH THE SOUTH
GATE — NOT YET A RESIDENT` inspector state with needs, skills, and controls intact.

## 2026-07-15 — Truthful catalog job and aggregate-recipe entitlements

**Problem:** Nine catalog job payloads were false: three founding/building capabilities were
already usable before their advertised studies, and six IDs were not runtime jobs at all. Only
Sawmill→Gather Logs matched live behavior, but both the signed action and Forester automation
duplicated that entitlement as hardcoded ownership checks. Textiles and the two Smithy design
studies also lacked recipe payloads for aggregate production that already existed.

**Fix:** Sawmill→Gather Logs is now the sole `UnlockJob`. A validated one-time catalog reverse
index drives both signed logging and Forester automation; denial is mutation-free and names
Sawmill. Fetch Water, Explore, manual research, and Barracks training remain founding/building
capabilities. Water Carriers, Textiles, and Den Insulation were reclassified without changing
stable identity, cost, prerequisites, order, or the live `housingPerDen` effect, preserving the
exact 167 Building / 167 RecipeResource / 166 Upgrade split. Textiles owns the Clothier and
Tannery aggregate recipes, while Weaponsmithing and Armorsmithing independently own the two
Smithy outputs. Fresh rules-v1 production cannot advance or spend inputs before its exact study;
rules-v0 saves remain grandfathered. These aggregate recipes do not appear in editable station
queues.

**Evidence:** Catalog uniqueness/runtime-ID/category and full 500-node purchase checks, signed
logging denial/success, Forester non-bypass, founding personal/communal capability regressions,
fresh-v1 versus rules-v0 Clothier/Tannery/Smithy neither/one/both cycles, both Smithy forge arms,
SQLite restart, Leader daily-choice determinism, client-copy/no-queue-leak tests, guided and
unattended campaigns, touched-crate gates, and strict Clippy cover the slice. The accepted
own-framebuffers `/tmp/research-job-payload-truth-sawmill.png` and
`/tmp/research-job-payload-truth-textiles.png` show the sole job line `Unlocks logging` and the
human recipe names `Cloth weaving` and `Leather tanning`; the temporary capture hook and processes
were removed.

## 2026-07-15 — Transport research no longer conjures vehicles

**Problem:** Owning Shipping made every water tile walkable by an ordinary cat, while owning Rail
gave any cat three-times speed whenever its remaining destination was at least 40 tiles away.
Neither effect required a dock, vessel, track, train, boarding step, or physical route, so research
ownership bypassed the game's physical-logistics contract.

**Fix:** The stable `water_travel` and `rail_logistics` capability IDs now mean blueprint
entitlements only. Water remains a hard obstacle for ordinary walking pathfinding regardless of
Shipping ownership. Rail ownership and remaining route length are neutral in physical movement
until tracks, vehicles, and routes exist. The two study descriptions and research-ledger payload
lines state that they grant blueprints rather than immediate travel.

**Evidence:** Ownership-on/off A* reaches exactly the same result across a full-width water
barrier; a real phase-34 cat advances exactly the same distance on a 50-tile route with or without
Rail; both comparisons have deterministic twins. A signed player campaign purchases the complete
prerequisite chain through Rail and Shipping, preserves the stable capability IDs, and rechecks
both physical denials. Existing fine-biome, stone-road, live-cadence founding, communal unattended,
and guided/unattended fishing campaigns remain green. Real rail and ship construction remains in
the open queue above. The accepted live own-framebuffers `/tmp/transport-rail-blueprint.png` and
`/tmp/transport-shipping-blueprint.png` visibly prove the two blueprint-only study descriptions
and payload labels; the temporary capture hook and isolated processes were removed.

## 2026-07-15 — Leader-owned daily research choice

**Problem:** The design assigns the once-per-rolling-real-day strategic study choice to the living
Leader, but runtime incorrectly required an appointed Loremaster and the ledger presented its
priority hint as an already active study.

**Fix:** A living Leader now deterministically spends existing research points on at most one
affordable study per rolling 24 hours. The colony-wide clock survives Leader replacement, run
reset, and SQLite restart through the legacy `lastLoremasterUnlockAt` column, so upgrades cannot
mint a free choice. Missing/null legacy values remain ready; zero points or no affordable target
never stamps the clock; clock rollback and multi-day offline jumps never create backlog. Signed
player purchases remain unlimited and outside that clock. Research labor/building automation,
comfort release, and every ritual path remain Loremaster-owned. The ledger now truthfully labels
the deterministic hint as `Leader priority`.

**Evidence:** Focused living/dead/bootstrap/succession/reset, exact-boundary/rollback/no-backlog,
positive-points/no-target, recipe/building entitlement, seven-role vacancy, ritual, signed manual
purchase, legacy-column restart, deterministic twin, and client-copy tests cover the authority
split without changing entitlement resolvers. The final touched-crate gate passed 1,334/1,334
tests (one intentionally skipped), strict Clippy, formatting, and diff checks; after removing the
temporary capture hook, all 126 client tests passed again. A live server/client framebuffer at
`/tmp/leader-daily-research.png` visibly proves the full ledger labels its hint `Leader priority`
and explains the Leader/Loremaster/player authority split in the selected-study inspector.

## 2026-07-15 — Truthful catalog-derived building placement research

**Problem:** The Research Hut was intentionally placeable before research could begin, but its
root study falsely claimed to unlock the hut. The generated `mill_foundations` study also claimed
to unlock the Mill even though the legacy `milling` study remained separately mandatory.

**Fix:** The catalog now marks the Research Hut explicitly available from founding. `milling` is
the sole Mill placement unlock, while `mill_foundations` is once again its declared durability
study with the same stable ID, cost, prerequisites, and daily-selection order. One catalog-derived
resolver drives signed placement acceptance and names the actual missing study in denial text;
catalog validation rejects competing founding/research placement sources.

**Evidence:** Exhaustive placement-source, catalog, signed deterministic, guided purchase/place,
unchanged daily-selection order/boundary, 48-hour unattended, server HMAC, SQLite restart, and client-copy tests
pass without changing the four live recipe entitlements or rules-v0 save grandfathering. The
accepted own-framebuffer `/tmp/research-building-unlock-truth.png` shows the full-page ledger's
Research Hut inspector saying `Available from founding: Research Hut`; the temporary capture hook
and processes were removed. Remaining daily Leader authority is tracked separately above.

## 2026-07-15 — Target-correct storage-capacity research

**Problem:** A capacity study on any completed building leaked its largest multiplier into every
resource. Simulation clamps, snapshots, physical pile routing, and village-trade checks also used
different capacity calculations, so the UI could advertise space that delivery could not use.

**Fix:** One authoritative calculation now drives the aggregate clamp, general-storehouse
headroom, snapshots, signed trade validation/deposit, and persistence. `food_storage_stores`
changes only Food/Fish/Herbs/Materials/Refined capacity, `water_bowl_stores` only Water, and
`smithy_stores` only Weapons/Armor. Unsupported building targets cannot leak globally. Trade
acceptance builds an exact physical deposit plan before mutation and therefore cannot lose goods
behind a debug-only assertion. Consumption refreshes the pile ledger before returning carriers;
source-less fresh expedition excess that genuinely cannot fit is explicitly abandoned and the
living worker released, while farm/gather/station cargo and death spills remain conserved.

**Evidence:** The capacity-only runtime resolver is bit-identical to full effect resolution across
all 500 catalog prefixes. Target-isolation, legacy shrine/designated migration, exact physical
headroom, signed purchase/trade, adversarial no-route trade, persistence, fresh-overflow
termination, physical-cargo conservation, deterministic twins, and the untouched no-cheat guided
farm→Mill campaign pass. Remaining scope is the 22 unsupported `*_stores` targets and the fixed
10-unit station-local stores listed in the open queue. The accepted 3826×2105 own-framebuffer
shows researched Food/Fish capacity at 680 (baseline 600 plus the targeted 80) while Water remains
independently 200. The final sim/server gate passes 1,130 tests with one explicit skip, plus strict
Clippy, formatting, and diff checks.

## 2026-07-15 — Physical Workshop and Smelter refining

**Problem:** Workshop Materials→Refined and Smelter Ore→Metal conversion mutated aggregate
resources without requiring the assigned cat to fetch inputs, work at the open station, or carry
finished output to finite storage.

**Fix:** Both refiners now use editable repeating queues, durable station-local input/output and
transit ledgers, physical source and destination routes, delivery-before-credit, per-completed-cycle
skill and tool wear, and deterministic partial-load handling. Removing a station leaves its local
goods at the former footprint for a living salvage trip; full storage, a filled destination while
en route, carrier death, and restart never teleport, create, or lose cargo.

**Player visibility and evidence:** The inspector exposes local and in-flight Materials/Refined and
Ore/Metal, queue state, progress, blocked reason, and worker travel. Signed player guidance,
cadence partitioning, legacy queue migration, SQLite restart, removal/death/full-storage recovery,
protocol/client coverage, 1,286 four-crate tests, strict Clippy, and formatting pass. Accepted
1920×1080 own-framebuffers show the selected roofless Workshop and Smelter with their real local
ledgers, outbound cargo, repeating recipes, and workers in transit.

## 2026-07-15 — Steward-managed physical station stockpiles

**Problem:** Physical processors still pulled inputs directly from the general storehouse. The
Steward did not create the limited local reserves promised by the role, and a naive implementation
could teleport goods, consume the player's stockpile budget, or lose cargo when an office, station,
route, carrier, or destination disappeared.

**Fix:** An appointed Steward now creates provenance-tracked, one-tile, exact-resource piles beside
every physical Mill, Sawmill, Workshop, and Smelter. Nine distinct piles cover one of each station;
the separate automation budget is sixteen. One durable source→transit→destination job balances
input deficits before output surplus, never rewrites player pile definitions, and never changes the
aggregate resource total. Vacating the office or removing a station leaves its pile and contents
dormant. Cancellation restores only available source headroom and persists any blocked remainder.

**Player visibility and evidence:** Active/dormant ownership, station provenance, carried resource,
route phase, and recovery blockage are visible in the stockpile inspector; unsafe removal is denied.
Deterministic all-four-station, priority, full-storage, partial-recovery, vacancy, station-removal,
death, signed HMAC, SQLite restart, protocol-compatibility, and client tests pass. Own-framebuffer
`/tmp/steward-stockpiles-final.png` shows a selected Sawmill Logs pile with resolved station
provenance; the other managed zones remain sprite/overlay-readable without persistent text plaques
over the open workshops. The unchanged no-cheat farm→Mill guided campaign passes in 87.793s after
topology-gated, minute-bounded, one-route-at-a-time planning.

## 2026-07-14 — Finite item wear, breakage, and material-backed repair

**Problem:** Material variants had value and quality, but individual units did not complete the
DF-like condition loop: no stable unit identity, physical load pressure, work wear, broken state,
or repair action was visible to a player.

**Fix:** Every item unit now has a stable ID, weight, and finite condition. Relevant work causes
wear and zero condition leaves the item broken. Signed repair requires the appropriate completed,
staffed workshop, a living worker, and one visible matching material; durability research scales
the repair. Traders respect a 20 kg item-load cap. State survives SQLite restart.

**Player visibility and evidence:** The Goods panel shows weight, condition range,
damaged/broken counts, and a repair control. Unit, migration, determinism, denial, research,
trader-cap, protocol, signed guided, persistence, client, and accepted own-framebuffer coverage
pass. This verifies condition behavior, not full material or recipe breadth.

## 2026-07-14 — Authoritative election timing between terms

**Problem:** The governance panel could show an open election but could not answer when the next
one would begin while the colony was between terms.

**Fix:** Snapshots now carry the authoritative term start, next election boundary, term length,
and remaining duration. The governance panel renders the server-derived countdown even when no
election is open, and the schedule survives persistence/restart.

**Evidence:** Boundary, protocol, persistence, client-formatting, and integrated governance UI
coverage pass.

## 2026-07-14 — Physical Mill production and truthful local inventory

**Problem:** Grain processing could mutate aggregate resources without proving that a worker
carried inputs to an open Mill, worked there, and returned its outputs.

**Fix:** The Mill now follows the same conserved physical contract as the Sawmill: finite-store
pickup, station-local input, on-site progress, station-local output, and finite-store delivery.
Its editable queue, cargo, progress, blocked reason, and local ledgers persist through restart;
death and cancellation do not create or lose goods.

**Player visibility and evidence:** The accepted 1920×1080 own-framebuffer shows a selected open
Mill with one worker, repeating grain processing at 50%, Grain 4.0 local input, Flour 2.0 local
output, and Flour 1.5 outbound. Simulation, protocol, server, client, persistence, determinism,
and strict lint gates pass. Temporary capture fixtures and processes were removed afterward.

## 2026-07-14 — Physical shrine material offerings

**Problem:** A material offering could be credited as a scalar threshold without a cat moving
visible stockpile goods to the shrine, so the player could receive a blessing before delivery.

**Fix:** The offering is now two conserved physical stages: reserve and carry material from a
reachable visible pile into shrine escrow, then perform the ritual. Cancellation, death, blocked
routes, and restart preserve inventory exactly; credit occurs once, only after both stages.

**Evidence:** Conservation, reachability, cancellation/death, determinism, restart, and integrated
shrine-faucet campaigns pass alongside the maintained full quality gate.

## 2026-07-14 — Physical Accountant rounds and truthful stale stock reports

**Problem:** A staffed Accounting Tent refreshed the colony-wide ledger without a cat visiting
the spatial stockpiles. The client could therefore present exact authoritative quantities that no
cat had physically counted, and separate or unreachable piles had no independent freshness.

**Fix:** Accounting is now a durable physical route. An assigned tent worker returns to the tent,
visits reachable piles in deterministic distance/ID order, counts each for five game-seconds, and
returns. Only the visited pile's report changes; blocked piles remain stale, topology changes are
replanned, and cancellation never changes physical inventory. Active routes and reports persist
through SQLite restart. Existing aggregate-only saves migrate additively.

**Vacancy correction (2026-07-15):** The first physical-round slice retained a hidden
30-game-second fallback that copied authoritative colony resources into every report whenever no
tent worker was assigned. That contradicted both the manual-before-officer design and the physical
bookkeeper contract. The fallback is removed: an unbuilt tent, a vacant Accountant office, or a
completed but unassigned tent may remain stale indefinitely. Founding and extinction recovery still
seed one exact baseline, and aggregate-only legacy JSON attributes its already-persisted historical
total to the founding general storehouse once without sampling current pile contents.

**Player visibility:** Resource HUD, Goods ledger, stockpile text, and inspectors use reported
values and mark stale estimates with `~` or `uncounted`. The Accounting Tent inspector shows the
active physical phase, target, reachable progress, and count dwell.

**Evidence:** Unit, determinism, protocol compatibility, signed assign/release server action,
persistence/restart, one-shot and hourly 24-game-hour vacancy partitions, and client display tests
pass across `cat-sim`, `cat-protocol`, `cat-server`, and `cat-client`. The booted client/server
own-framebuffer at 1920×1080 was inspected and accepted with an active counting round and visibly
stale HUD/ledger estimates; the 2026-07-15 recapture verifies the corrected indefinitely stale
vacancy behavior.
