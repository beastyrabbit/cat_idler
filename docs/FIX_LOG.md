# Idle Cat Forest fix log

This is the maintained, evidence-backed log for fixes found during design review and player-guided
or unattended playtesting. The queue records reproduced problems before work begins. Move an item
to the verified log only after its behavior, persistence boundary, relevant Rust quality gates,
and any changed Bevy visuals have been verified.

## Open fix queue

| Finding | Required correction | State |
| --- | --- | --- |
| Most building-capacity studies still have no physical storage domain | Food Storage, Water Bowl, and Smithy capacity studies are live and target-correct. The other 22 generated `*_stores` studies remain deterministic no-ops because those buildings do not own a modeled storage domain. Mill, Sawmill, Workshop, and Smelter station-local input/output stores also remain fixed at 10 rather than consuming capacity research. Model a real physical domain before activating each remaining study. | queued |
| Research recipe/resource breadth remains incomplete | Four data-owned preparation payloads now bind the maintained physical recipes: Grain Milling→`grain_to_flour_and_food`, Carpentry→`logs_to_lumber`, Metallurgy→`ore_to_metal`, and Trade Goods→`materials_to_refined`. Fresh rules-v1 villages require the exact study for snapshots, signed queue additions, block reasons, and physical station advance; old/missing rules-v0 saves retain their queues and are grandfathered. The other 96 generated recipe IDs and all 64 generated resource IDs still have no physical consumer. Add only physically sourced resource/recipe breadth. | in progress |
| Research job unlocks do not gate jobs truthfully | Nine `UnlockJob` payloads are false or unread: the three real `JobKind` claims (Fetch Water, Explore, and Train Warrior) intentionally work before their studies, while six advertised IDs have no runtime `JobKind`. Only Sawmill→Gather Logs agrees with live placement/action behavior. Remove the false claims without taking away founding survival or manual work. | queued |
| Worker-slot studies have no staffing consumer | Twenty-five `worker_slots +1` effects resolve, but buildings, persistence, automation, protocol, and UI still support exactly one assigned cat. Implement real multi-worker ownership and physical work before presenting these studies as effective. | queued |
| Shared terrain is duplicated per colony | Terrain, ecology, roads, wear, depletion, and fish are colony-owned, so two villages at the same coordinates do not inhabit one authoritative mutable world. Move canonical spatial state to world scope while keeping fog and learned contact private. | queued |
| Inter-village trade is nonphysical | Contact summaries and atomic scalar barter exist, but cats do not meet, carry items, form caravans, or travel trade routes. Preserve knowledge-blind scouting and shrine-return discovery while adding physical exchange. | queued |
| Fine-biome resources and transport are incomplete | Gem, bone, clay, and sand lack complete physical sources/chains; rail and shipping have modifiers but no tracks, trains, vessels, or routes. | queued |

## Verified fixes

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

**Player visibility:** Resource HUD, Goods ledger, stockpile text, and inspectors use reported
values and mark stale estimates with `~` or `uncounted`. The Accounting Tent inspector shows the
active physical phase, target, reachable progress, and count dwell.

**Evidence:** Unit, determinism, protocol compatibility, signed server action, persistence/restart,
and client display tests pass across `cat-sim`, `cat-protocol`, `cat-server`, and `cat-client`.
The booted client/server own-framebuffer at 1920×1080 was inspected and accepted with an active
counting round and visibly stale HUD/ledger estimates.
