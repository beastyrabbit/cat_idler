# LAI.64–LAI.70 Plan 2 Delivery Audit

Recorded: 2026-07-25

This is a corrected Opus 5 source audit of Plan 2 delivery. The reviewer read
both locked plans and all boards from line 1, then traced P2.24–P2.36,
GUI-R01–GUI-R26, GUI-C01–GUI-C12, P2-G01–P2-G09, and LAI.63–LAI.70 through
protocol, server, SQLite, client, assets, diagnostics, source tests, and legacy
roots.

No repository file was edited by the reviewer. No Cargo, compiler, test,
build, lint, formatter, browser, Playwright, image-generation, or validation
command ran. All conclusions below are static findings, not acceptance
evidence.

## Status summary

| Card | Audited state |
|---|---|
| LAI.63 | Partial: the protected runtime is in the real tick, but major authorities and physical delivery state are not retained or advanced through it. |
| LAI.64 | Partial/blocking: the public God action set is strong; the canonical snapshot lacks research topology and has permanently empty/stubbed collections. |
| LAI.65 | Partial: all sixteen canonical action adapters exist, but the persistence/domain/reset/fixture cutover is incomplete and two authorities remain. |
| LAI.66 | Partial/blocking: the plugin and screen models exist, but the Log has no event source and other content projections are empty. |
| LAI.67 | Partial/blocking: all five routes and six Council tabs render, but Research cannot render real dependency edges/junctions/tracks. |
| LAI.68 | Partial: the canonical renderer exists, but the production world feed and multiple required asset families are missing. |
| LAI.69 | Foundation only: the diagnostics leaf exists, but its non-heartbeat domains have no production emitters and the campaign gate is not a campaign run. |
| LAI.70 | Not started: legacy roots, duplicate authorities, disabled blocks, and the retired browser suite remain. |

`dev` is the consistent status for partial implementation. It must never be
read as acceptance. `todo` remains correct for LAI.70.

## Confirmed strong public action boundary

`CanonicalGodAction` contains the intended sixteen Plan 2 actions with exact
version-lane requirements. Routine worker, tile, route, storage, food-list,
officer, standing-order, production, and direct trade controls are absent from
that canonical action union.

The action boundary is not the same as complete delivery. Snapshot projection,
production socket liveness, persistence, client cutover, diagnostics, and
acceptance remain separate obligations.

## Blocking delivery findings

### Production does not provide the base world to the shipped client

The production broadcast path sends the canonical envelope to authenticated
canonical connections. The alternate `WorldSnapshot` send and its helper are
compiled only for tests. On the client, terrain, cats, buildings, roads, fog,
zones, and stockpiles still consume `LatestSnapshot`, whose only writer is the
legacy non-canonical frame path.

Static consequence: a real canonical connection can populate the new shell
and canonical overlay while leaving the base world renderer without terrain,
cats, or buildings. Browser acceptance before this is corrected would test an
empty world rather than the shipped game.

The final implementation should project all required report-safe terrain,
cat, building, road, fog, zone, and stockpile state into the canonical
envelope and retire the legacy snapshot authority. Re-enabling the legacy
snapshot as a second live feed is not the preferred single-authority cutover.

### Canonical Research has no graph topology

`ResearchSnapshotV2` currently contains balances, the two queues/decisions,
and preparations. It does not contain canonical study definitions,
prerequisite edges, track/level membership, repeatable state, junction
membership, permits, or derived totals.

The client explicitly admits it will not invent dependency lines. Its three
visual regions therefore bucket queue entries but do not render the required
fixed research graph, fourteen tracks, curated AND junctions, level 1–10 plus
repeatable level 11, or permit dependencies.

LAI.64 must add a bounded, ordered topology projection from the canonical
research catalog and bump the snapshot schema. LAI.67 must render only those
reported nodes/edges/junctions/tracks and never infer missing dependencies.

### Seven canonical collections are hard-coded empty

The server projection emits empty:

- typed food stocks;
- Hunting sites;
- rare materials;
- fixtures;
- Cookhouse batches;
- Fishing Huts;
- event log.

The event log omission makes the Log primary screen data-less. The other six
remove major Plan 1 content from the wire even though client models exist.
Comments in the projection identify the root cause: the relevant Hunting,
Cookhouse, Fishing, material/fixture, and event authorities are not retained
by the canonical runtime.

LAI.63 must retain and advance each authority once. LAI.64 must project
report-safe bounded iterators instead of filling empty vectors.

### Task, construction, Hole, and trade projections are stubs

The current canonical projection includes:

- one constant generic task-objective sentence;
- empty task cargo, reservations, refusals, and anatomy requirements;
- empty construction delivered/in-transit/consumed cargo;
- constant Hole permission reasons and confidence;
- empty Hole contribution receipts;
- empty trade route tiles and escrow.

These are honest unavailable states, not fabricated values, but they do not
satisfy the observable acceptance rows. The production projection must use the
authoritative report-safe values added by LAI.63 and preserve explicit
unavailable/redacted states where reports truly cannot know.

### Diagnostics leaf has no production emitters

The diagnostics module defines phase, planner, matcher, skill/family, election,
research, construction, divine, trade, persistence, and action-outcome
domains. Outside the leaf and its focused source test, no production code
records those domain events.

The only live sink is the 120-tick heartbeat/count structure. That does not
cover the required per-domain progress, blocker, recovery, terminal,
persistence, and server-action diagnostics.

LAI.69 must wire bounded, opt-in emitters into the protected simulation phases
and canonical server action/rejection boundary. Player-facing logs must remain
quiet and report-safe.

### The 30-game-day campaign gate is only a symbol grep

The campaign threshold helpers for at least 85 of 100 fresh and 97 of 100
established seeds exist. The current manifest test only checks that the helper
symbol names appear in source text. It does not execute the campaign runner,
measure 30 game days, or evaluate growth/progression.

No campaign pass may be inferred from that test. The final serialized gate
must execute the actual 100-seed fresh and established matrices, prove the
thresholds, and verify continued village growth/progression rather than
survival-only idling.

### Browser scenarios target retired surfaces

The existing eight browser scenarios address Plans, Shrine/Favor,
progression-purchase, and care panels. They contain no LAI.54/66/67/68 shell
routes or Council-tab coverage.

The final browser suite must be rewritten against the five primary screens,
six Council tabs, both research lanes, construction phases, Stores containers,
Village families/elections, Hole/divine controls, exact world geometry, and
the 1024×768 through 4K at 100/115/130 percent matrix. The independent visible
browser pass comes after the one serialized automated run.

### Legacy client root remains live

The app still registers direct building placement, zone painting, station
queues, removal, crop/gather controls, tool/dock/command controls, manual
trade/labor/equipment controls, and officer appointment/vacancy controls.
Those controls emit legacy actions that the canonical server rejects, so the
shipped UI presents unavailable direct micromanagement instead of omitting it.

Map, Help, Dispatches, ticker, minimap, and old upgrade-tree entry points also
remain registered. The old HUD still exposes generic Food, Fish, Preserves,
and Blessings identities.

LAI.66–68 must cut the root over to the five-screen shell and authorized broad
actions only. LAI.70 then deletes the retired controls/surfaces after their
replacement behavior is observable.

### Hole and station building identities are incomplete

`BuildingType::Shrine` remains the live Hole identity; `BuildingType::BlackHole`
is absent. Cookhouse and Fishing Hut building variants are also absent at the
time of this audit, so their canonical construction materialization returns an
unsupported-target gap.

LAI.46/63 must add Cookhouse and Fishing Hut as real 3×3 buildings. The final
ordered cutover must rename the Hole building identity and delete remaining
Shrine vocabulary without creating a second Hole authority.

## Asset and accessibility findings

The planned asset tree contains the delivered Hole layers, Lair bands,
creatures, materials, foods, items, recipes, fixtures, augmentations,
Cookhouse, and Fishing Hut art described on LAI.49. The new screens use
AccessKit and stable semantic IDs.

Still absent at audit time:

- construction scaffold/structure/fit-out/operational sheets for every
  building and upgrade (only Cookhouse has the three intermediate keys);
- Basket/Barrel/Crate/Chest/Rack fullness states;
- quality badges/compositor;
- family enterprise signs;
- residence/household art.

The final render path also needs authoritative trigger, transparency/bounds,
native/WASM, zoom, despawn, restart, and screenshot-matrix evidence.

## Legacy deletion inventory

Still compiled in `cat-sim`:

- `favor.rs`;
- `shrine.rs`;
- `shrine_offerings.rs`;
- `research_purchase.rs`;
- `scholar_research.rs`;
- `leader_ai.rs`;
- `leader_director.rs`.

Still present in the client as dormant or legacy roots:

- `progression.rs`;
- `plans.rs`;
- `cat_care.rs`;
- `live_render.rs`;
- their superseded plugins and direct-control systems.

Legacy Shrine/Favor/research source tests remain. Coin references remain in
sim, protocol, client, and footprint/UI modules.

Twenty-six `#[cfg(any())]` blocks in server `main.rs` and three in
`world_tick.rs` hide rather than delete old behavior. One server block disables
the complete unit-test module. This is an explicit evidence regression, not a
successful legacy deletion.

## Dependency-ordered delivery and deletion plan

1. Finish LAI.46/63 runtime retention, physical tasks, and one-authority
   advancement, including Food/Hunting/Cookhouse/Fishing/material/event state.
2. Restore production world liveness through the canonical envelope so terrain,
   cats, buildings, roads, fog, zones, and stockpiles have one shipped feed.
3. Complete LAI.64 research topology and every empty/stubbed report-safe
   collection; bump/validate the canonical schema and projection.
4. Complete LAI.65's per-aggregate persistence/reset/fixture cutover described
   in `lai48-static-persistence-cutover-inventory.md`.
5. Cut the client root over to LAI.54/66/67/68 and delete live registration of
   forbidden direct controls, banned navigation, and generic resource HUD.
6. Generate/land the missing inspected-style construction, container,
   quality, family/enterprise, and residence sprites; complete their
   authoritative renderer triggers.
7. Wire all required bounded diagnostic emitters before spending the final
   campaign/browser budget.
8. Replace the symbol-presence campaign contract with the actual serialized
   100-seed fresh/established 30-day run.
9. Rewrite the browser suite for the five screens, six Council tabs, both
   research lanes, all visual matrices, and the real server/SQLite path; run
   the independent visible browser only after the automated suite.
10. Delete dormant client modules, legacy sim authorities/tests, Coin and
    generic resource identities, `Shrine`/Favor/Blessings/Insight vocabulary,
    duplicate persistence and wire paths, and every `#[cfg(any())]` staging
    block only after a verified replacement exists.
11. Reconcile every board copy so a partial `dev` card has the same status and
    exact remaining gaps everywhere.
12. Run the single final serialized focused → format/Clippy → smoke → restart/
    isolation → campaign → Playwright → visible-browser sequence. No timeout,
    source grep, authored test, or earlier focused pass may be relabeled as
    final acceptance.

## Acceptance state

LAI.64–69 remain partial `dev`; LAI.70 remains `todo`. The action surface alone
does not satisfy Plan 2 delivery. No runtime, compilation, layout, WASM,
viewport, screenshot, campaign, or browser claim is made by this audit.

## Static implementation progress after the audit

Orca task `task_52f8187f7e5d` / dispatch `ctx_08cc6077b373`
implemented the production legacy-action retirement gate in
`crates/cat-server/src/main.rs`:

- one exhaustive, wildcard-free `legacy_action_requires_lai_v2` classifier now
  names every legacy `ClientAction`;
- only Presence, Ensure, FoundVillage, and JoinVillage remain on the bootstrap/
  lifecycle legacy lane;
- every superseded gameplay mutation is rejected with the canonical
  update-required response after bounded legacy decoding and before
  `apply_action`;
- canonical schema-v2 actions still short-circuit to the existing strict
  authenticated canonical handler;
- stale source assertions now reference the real handler/gate ordering and
  require one classifier, one allowance group, and no wildcard fallback.

This closes the production fall-through identified in the simulation audit,
but not the client-side direct-control registrations or legacy action/type
deletion. Existing server unit tests that intentionally drive old gameplay
actions must be migrated to the canonical lane. No Cargo, compiler, test,
build, lint, formatter, or browser validation ran, so the implementation is
static/unverified and does not change LAI.65/70 acceptance.
