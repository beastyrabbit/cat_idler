# Integrated LAI.35–LAI.70 implementation map

This is the current cross-layer implementation map for both stored plans. It replaces the LAI.23–31
readiness maps as forward-looking authority; those files remain historical evidence for the first
Leader-AI cutover only. Exact behavior still comes from the two stored plans and their complete
P1/P2 board registers.

The detailed foundation-versus-runtime gaps for skills, families, governance, construction,
storage, research, food/divine systems, and barter are maintained in
[`authority-consolidation-audit.md`](authority-consolidation-audit.md). A pure leaf does not satisfy
this map until its state is live, its physical and report-safe paths are integrated, and the
superseded authority is retired.

## Precedence and completion rule

1. `final-hole-hunting-content-plan.md` and its P1.01–P1.45/P1-C01–C04 register.
2. `final-integrated-overhaul-plan.md` and its P2.01–P2.36/GUI-R/GUI-C registers.
3. Focused maintained domain docs, especially `planner-and-beliefs.md`,
   `hole-research-progression.md`, `diplomacy-barter.md`, `cats-and-care.md`, `spatial-task-contract.md`,
   `extending-the-system.md`, and this map.
4. Implemented leaf contracts that were reviewed against those sources.
5. Historical LAI.23–31 maps and discarded branch behavior.

A card reaches `done` only after its behavior, runtime, public wire, fresh persistence, server
authorization, client/visual surface, diagnostics, legacy absence, and required focused/final
evidence all agree. A pure type or passing unit test marks a foundation at most.

## Hot-root ownership

| Root | Sole integration owner | Rule |
|---|---|---|
| `cat-sim/src/world_tick.rs` and final runtime aggregate | LAI.46 + LAI.63 combined integration owner | One ordered runtime; no legacy/shadow mutation |
| `cat-protocol/src/lib.rs` and schema/version root | LAI.47 + LAI.64 protocol owner | All DTOs/actions/errors/version lanes in one cutover |
| `cat-server/src/main.rs` and routing/projection | LAI.48 + LAI.64 server owner | One header-first authorization/projection path |
| `cat-server/src/persistence.rs` and fixtures | LAI.48 + LAI.65 persistence owner | Fresh schema/reset/future-fail-closed only |
| `cat-client/src/lib.rs`, navigation, world renderer | LAI.49/50 + LAI.54/66–68 client owner | One five-screen UI and one world visualization authority |
| root docs, boards, diagnostics, acceptance | LAI.51/52 + LAI.69/70 owner | Full traceability; no partial closure |

Workers may own disjoint focused leaves. Only the coordinator runs Cargo, Clippy, builds, campaign
tests, Playwright, image generation, or browser sessions. Heavy processes are serialized.

## Simulation ownership map

| Domain | Focused authority | Runtime obligation |
|---|---|---|
| content and stable IDs | `content_manifest`, canonical embedded manifest | validate once; every planner/executor/wire/art lookup uses it |
| universal quality and physical identity | `quality_lots` | preserve content+quality lots and exact item/material identity through every location/recovery |
| food and ecology | `food_ecology`, Cookhouse/fishing sources | exact server truth; report-limited stock/production/regeneration; no generic Food |
| Cookhouse | `cookhouse` | exact 23 Cookhouse recipes, Mill only Flour, complete 3×3 task, physical batches |
| fishing | `fishing` | 3×3 Hut+dock/water geometry, finite habitat, hand/Rod/Hut profiles, wear |
| Hole | `black_hole` | fixed 5×5, physical feed/upgrade cargo, forty-minute cadence, Void credit, bounded histories |
| Hunting | `hunting_lair` | exact twenty creatures, encounter/risk/drop rules, physical loot/cache, absolute respawn |
| crafting | `material_crafting` | one-use exact material identity, processing, curated uses, augmentation/fixture identity |
| attributes/skills/refusal | `cat_capabilities`, skill catalog/matcher | declared XP only, Mastery, anatomy, affinities, urgency-first selection |
| families/housing/mentoring | `family_specialization`, `family_housing` and integrated family authority | lineage, partnership, enterprise, housing, teaching obligations, exact sites |
| governance | `cat_governance`, officer leaves | ballots/backing, appointments, clearance, succession, expulsion cleanup |
| research | unified progression authority over manifest/scholar/boost leaves | Notes/Void split, God queue, Leader lane, preparation/refund, permits/repeatables |
| construction | `construction_stages` plus canonical building bill catalog | exact scaffold/structure/fit-out cargo and 20/60/20 labor for every building/upgrade/Hole axis |
| storage/village works | `physical_storage`, `village_infrastructure` | zones/containers/linked stores, farms, roads, walls, gates, maintenance |
| food/divine policy | `food_divine_policy` and `divine_boosts` | permissions, aid, Inspiration, click contribution, Void miracles/rescue |
| diplomacy/barter | `diplomacy`, `moneyless_barter` | personal stance, honest Alliance label, Enemy rejection, physical escrow/cargo/recovery, no money |
| planner/officers | planner/belief/intent/scheduler/request leaves | report-safe goals, dependencies, omission/mistakes, recovery, bounded history |
| diagnostics | `leader_ai_diagnostics` | opt-in developer-only bounded trace, heartbeat and terminal cause; never public truth |

No leaf may define a second resource/recipe/quality/study/building/skill/art registry. Closed behavior
enums are allowed only when the executor genuinely requires exhaustive code.

## Ordered world-tick integration

The single runtime performs due work in semantic `(due time, domain order, stable ID)` order:

1. validate rules/schema versions and bound catch-up;
2. ecology, Apple/Fish/Hunt regrowth/respawn, spoilage, needs, hazards, injury, death, and emergency;
3. observations, officer rounds, report expiry/contradiction, and belief updates;
4. family partnership/housing/teaching obligations, elections, succession, vacancies, and office
   duty;
5. crossed Leader/officer planning boundaries, posture, goals, food permissions, research choices,
   construction/storage/village/Hole/barter planning, omissions, dependencies, and retries;
6. authoritative spatial resolution for full objective, footprint, work position/slot, endpoint,
   route, cargo, and stage;
7. one atomic world reservation transaction for sources, exact identities/quantities, sites,
   routes, capacities, slots, containers, construction stages, tools, cargo, and workers;
8. urgency-first workforce matching with enterprise/affinity/refusal/skill/anatomy/continuity/route
   ordering;
9. physical movement and task execution: gathering, water, apples, fishing, hunting, hauling,
   cooking/crafting, teaching/care, scholar work, construction, storage, farms/roads/walls, Hole,
   and caravans;
10. atomic completion/failure effects, XP/Mastery, cargo consumption/output, salvage, Notes/Void,
    refunds, research payloads, permits, boosts/divine aid/miracles, relationship/contract effects;
11. release/adopt/retry/terminal transitions, bounded history pruning and output drains;
12. report-safe snapshot/event projection and optional bounded developer trace.

Large-tick and partition twins must agree. A configured bound stops with an explicit non-pass
terminal cause rather than dropping due work or hanging silently.

## Spatial and physical transaction contract

Every visible task snapshot and persisted task state carries:

- stable task/intent/objective IDs and typed objective kind;
- complete row-major canonical footprint;
- current reachable work tile or exclusive semantic slot;
- pinned delivery endpoint and destination capacity;
- ordered route/segments and any world-exclusive claims;
- exact cargo lots/items/materials, quantities, quality, provenance, condition, reservation, and
  current physical location;
- stage, progress, worker, blockers, retry/recovery state, and report provenance.

Hunt uses the real revealed EnemyLair; Fetch Water uses water plus a distinct dry bank; Apple work
uses the tree footprint; Fishing uses shoreline habitat/Hut/dock attachment; Quarry uses its own
CaveEntrance type; a building/Workshop/Cookhouse/Hole task highlights the complete authoritative
footprint. There is no radius, center-point, nearest-source, or client-derived fallback.

All mutations preflight and stage the complete transaction, then commit once. Failure before pickup
releases reservations. Failure after pickup delivers or salvages the same identity to its origin, a
compatible stockpile, or typed last-land cache. Consumed atomic work completes once. Restart and
duplicate actions cannot duplicate or delete cargo.

## Protocol snapshot

The new protocol cutover publishes strict bounded DTOs for:

- report ladder, beliefs, evidence/provenance, officers, vacancies, planner goals/intents/tasks,
  dependencies, bounded reasons, and complete spatial roles;
- cats, attributes, skills/XP/Mastery, affinities/refusal, anatomy/prosthetics, office clearance,
  family/household/partnership/lineage/tradition/enterprise/housing/mentoring/elections;
- typed physical lots and exact items/materials/augmentations/fixtures with quality, condition,
  location, reservation, storage/container, cargo, stage, and provenance;
- food permissions/ecology reports, Cookhouse queues, Fishing, Apples, Hunting Lairs/parties/loot/
  respawn reports, Hole feeds/upgrades/axes/Void;
- canonical research graph, Notes/Void, God queue/front/progress/preparation/refunds/permits,
  Leader cadence/choice/collision, scholars, repeatables, boosts, Inspiration, divine aid/miracles;
- staged construction, building footprints/levels, stage bills/cargo/progress, click meter, storage
  zones/containers/linked stores, farms/crops, roads, walls/gates, maintenance;
- personal diplomacy stance and moneyless barter proposal/valuation/escrow/route/caravan/cargo/
  stage/recovery;
- presentation-safe five-screen navigation metadata, world art/state keys, and capability flags.

Exact authoritative stock, production, regeneration, replenishment, regrowth, respawn, hidden
candidate scores, unrevealed sites/routes, private foreign state, and developer trace never enter
the DTO. At report levels below four there is no regeneration field, value, sentinel, tooltip, or
error hint.

Every collection has a bounded maximum and documented canonical order. Unknown/future required
variants fail closed. Protocol/schema versions change once with the integrated root cutover.

## Public action surface

Every action has protocol version, authenticated principal, selected colony, stable bounded action
ID, exact domain expected-version lane, and strict payload. Allowed player actions are limited to:

- broad temporary encouragement/conservation nudges;
- God research queue/fund/prepare/reorder/remove actions;
- player-only boost, Inspiration, contribution, construction-click aid, Void press, and report-gated
  Ration/Water rescue;
- one replaceable election backing block;
- personal Alliance/Neutral/Enemy stance;
- authorized adult/valid-household expulsion with cleanup preview/commit;
- presentation navigation/selection that mutates no simulation.

Direct building/site/road/crop/storage/container/production/worker/food-permission/officer/Leader-
research/Hole-feed controls are absent and server-rejected.

Validation order is protocol, authentication/signature, selected-colony ownership, actor/domain
authority, expected version, idempotency replay, current simulation preconditions, staged commit.
Rejections are bounded typed results. Duplicate accepted or rejected actions replay the stored
result without reevaluating hidden state.

## Fresh SQLite persistence

The integrated database stores all authoritative aggregates and bounded receipts named above.
Collections serialize in stable ID order with strict versioned decoding. Runtime and action commit
use one SQLite transaction so state, physical ledgers, currency, reservations, and dedupe result
cannot diverge.

There is no semantic gameplay migration. A recognized obsolete schema uses the authenticated
two-step reset in test/development or whole application database recreation required by deployment.
The production reset endpoint is absent and rejected. Unknown, future, or malformed state fails
closed/quarantines without partial reset. Preserve only unrelated authentication metadata explicitly
allowed by the reset contract.

Fresh fixtures regenerate account/identity data as required, world seed, checksum, protocol/schema
metadata, and complete new state together. Fixture/schema/source scans must find no Shrine, Favor,
Blessing, generic stored Food/Fish/Preserves, scholar Insight, research points, coin, purse, price/
settlement, legacy director, direct-placement action, or legacy research UI authority.

## Server projection and commit

The server has one socket projection and one action router. Projection queries report-safe sim
aggregates and never serializes then hides truth. Every connection/event uses the same redaction
policy. Logs contain bounded stable IDs/categories but no signature, raw hidden quantity/rate,
unseen site/route, private colony state, or developer trace unless the local opt-in developer sink
is explicitly active.

The staged action commit clones or transactions the affected domain states, checks cross-domain
invariants, applies all physical/currency/reservation/dedupe effects, validates the candidate, and
commits once. A failed candidate leaves byte-equivalent authoritative state.

## Client and world renderer

The outer shell has exactly five primary routes: Log, Stores, Village, Research, and Council.
Center Village and session/account controls remain in the top bar. Council has exactly six approved
tabs. Map/Help/Dispatches/ticker/letter opener routes are removed. Escape behavior is centralized.

- Log: stable filtered colony events without hidden diagnostics.
- Stores: physical zones, containers, fullness, lots/items, compatibility, reservations, linked
  workshop stores, cargo and provenance.
- Village: cats, households/families/enterprises/housing, buildings/construction, production,
  farms/roads/walls, selected world details.
- Research: three-region graph, queue/front, preparation, Notes/Void, Leader lane, permits,
  repeatables, boosts and progression detail.
- Council: plans, officers, governance/elections, Hole, diplomacy/barter, divine/broad policy
  controls in the exact six-tab assignment.

The start screen is a non-authoritative off-map showcase. It creates no real IDs, snapshot, action,
tick, save, or selection and cannot auto-enter a colony.

World rendering uses snapshot-provided footprints/routes/stages and deterministic art keys. It
includes all Plan 1/2 Hole/lair/portrait/drop/food/item/fishing/apple/farm/construction/family/
enterprise/storage/container/road/wall/gate/task-marker assets and state sheets. The visual language
is parchment, wood, dark forest, solid pixel art—no glassmorphism, generic dashboard cards, pill
overuse, neon glow, or gradient drift.

Required viewports are 1024×768, 1280×800, 1920×1080, 2560×1440, and 3840×2160 at 100%, 115%, and
130% UI scale on native and WASM. Phones are out of scope. Every visual has native dimensions,
transparent bounds validation, accessible label/textual fallback, deterministic lookup, and a
gameplay-zoom screenshot.

## Diagnostics and verification

Developer diagnostics are disabled by default, bounded, strict/restart-safe, and never projected to
normal Log/protocol. They cover phase entry/exit, caller timing, planner candidates/omissions,
matching/rejections/tasks/reservations, skills/teaching/families, elections, research collisions/
refunds, construction/storage, Hole, divine accounting/rate rejection, barter/caravan, persistence/
actions, UI envelopes/rejections, last transition, and terminal cause.

The 120-tick probe emits periodic phase, task count, reservation count, last transition, and
terminal cause. Only `Completed` is pass; Timeout, Stalled, SimulationFailure, Panic, silence, or a
killed process is not.

Verification order is:

1. static/rustfmt/diff review during leaf waves;
2. one constrained focused test after a complete feature;
3. one serialized compile/Clippy/smoke integration ladder after root integration;
4. required deterministic/restart/partition/multi-colony and 30-game-day campaign scenarios with
   ≥85% fresh and ≥97% established success plus continued progression;
5. real Rust server + fresh SQLite + Rust/WASM client through named Portless routes;
6. one Playwright worker using shipped controls;
7. independent visible-browser operation with matching accessibility tree, screenshot, console, and
   network evidence at every required viewport/scale.

Automated tests never call a live AI provider. Browser acceptance never injects DOM/private state or
uses hidden endpoints.

## Final deletion gate

LAI.70 scans source, modules, exports, protocol JSON, schema/fixtures, docs, UI routes/labels, assets,
logs, tests, and browser artifacts. It deletes or archives every obsolete authority and proves
exactly one planner, report boundary, physical inventory, food model, Hole/Void ledger, research
completion ledger/two lanes, construction pipeline, storage model, task geometry authority,
diplomacy/barter system, protocol projection, persistence schema, server router, and client shell.

No compatibility alias, shadow projection, test-only production bypass, semantic save conversion,
or “temporarily unused” legacy root may remain to close the card.
