# Idle Cat Forest fix log

This is the maintained, evidence-backed log for fixes found during design review and player-guided
or unattended playtesting. The queue records reproduced problems before work begins. Move an item
to the verified log only after its behavior, persistence boundary, relevant Rust quality gates,
and any changed Bevy visuals have been verified.

## Open fix queue

| Finding | Required correction | State |
| --- | --- | --- |
| Production stations pull every input from general storage | Steward-managed limited piles must appear beside every physical processor and be filled by real source→destination balancing trips. Vacancy, removal, full storage, death, cancellation, and restart must preserve exact physical provenance and transit. | in progress |
| Most building-capacity studies still have no physical storage domain | Food Storage, Water Bowl, and Smithy capacity studies are live and target-correct. The other 22 generated `*_stores` studies remain deterministic no-ops because those buildings do not own a modeled storage domain. Mill, Sawmill, Workshop, and Smelter station-local input/output stores also remain fixed at 10 rather than consuming capacity research. Model a real physical domain before activating each remaining study. | queued |
| Research advertises recipes and resources that do not exist | All 100 generated recipe IDs and 64 generated resource IDs currently enter registries with no consumer, and none names a maintained queue recipe or `ResourceKind`. First bind research to real physical recipes, then add only physically sourced resource/recipe breadth. | queued |
| Research job unlocks do not gate jobs truthfully | All ten `UnlockJob` payloads are unread. Fetch Water and Explore work before their claimed unlocks, while six advertised IDs are not runtime job kinds. Align catalog wording/IDs and authoritative action gates. | queued |
| Some building unlock studies lie or duplicate another gate | Research Hut is intentionally available for bootstrap despite an unlock payload; `mill_foundations` cannot independently unlock the Mill because `milling` remains mandatory. Make the catalog and placement rules agree while preserving a playable bootstrap. | queued |
| Worker-slot studies have no staffing consumer | Twenty-five `worker_slots +1` effects resolve, but buildings, persistence, automation, protocol, and UI still support exactly one assigned cat. Implement real multi-worker ownership and physical work before presenting these studies as effective. | queued |
| Shared terrain is duplicated per colony | Terrain, ecology, roads, wear, depletion, and fish are colony-owned, so two villages at the same coordinates do not inhabit one authoritative mutable world. Move canonical spatial state to world scope while keeping fog and learned contact private. | queued |
| Inter-village trade is nonphysical | Contact summaries and atomic scalar barter exist, but cats do not meet, carry items, form caravans, or travel trade routes. Preserve knowledge-blind scouting and shrine-return discovery while adding physical exchange. | queued |
| Fine-biome resources and transport are incomplete | Gem, bone, clay, and sand lack complete physical sources/chains; rail and shipping have modifiers but no tracks, trains, vessels, or routes. | queued |

## Verified fixes

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
