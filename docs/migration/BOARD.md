# Migration Task Board — Web → Bevy client + Rust sim

Historical task board for the codex-orchestrated rebuild, plus a maintained phase rollup. Detailed
current evidence lives in `docs/IMPLEMENTATION_AUDIT.md`; dated card counts below are preserved as
the state at each slice boundary, not current backlog. Original plan:
`~/.claude/plans/ok-then-lets-close-polished-quokka.md`.

## Personas (see `codex/personas/*.md`)
`scrum-master` (decompose) · `researcher` (port specs) · `test-engineer` (tests first) ·
`developer` (implement to green) · `qa` (independent verify) · `integrator` (merge).

## Card format
```
### <PHASE>.<n> <title>   [status: todo|researching|red|dev|qa|blocked|done]
persona: <role>            depends_on: [<ids>]        parallel_group: <gid>
scope: <one-paragraph, ≤1 module or slice>
acceptance: <tests that must pass + parity criterion>
notes: <links, TS source paths, golden fixtures>
```
Status flow: `todo → researching → red (tests written, failing) → dev → qa → done`.
Nothing reaches `done` until `cargo nextest` + `clippy` are green and QA (a second
codex, plus a Claude review for high-value slices) signs off.

## Phase status
| Phase | Title | Status |
|-------|-------|--------|
| P0 | Foundation & safety | done |
| P1 | Sim foundation (rng, types, test-accel) | done |
| P2 | World generation | done |
| P3 | Cat AI (pathfinding, movement, leader director) | done |
| P4 | Life sim | done |
| P5 | Economy + housing + roads | done |
| P6 | Military + governance + upgrade tree | done |
| P7 | Master loop (`world_tick`, multi-colony) | done |
| P8 | Protocol + server (+ multi-village founding) | done |
| P9 | Client render + UI (top-down world, HUD, action buttons) | done — P9.1–P9.4 shipped and framebuffer-verified; P9.5 (`bevy_brp_extras` MCP screenshot tooling)/P9.gate were superseded rather than formally closed (manual framebuffer capture per `docs/HANDOFF.md` is the verification method actually used; P13/P18/P19 added far more client UI on top) |
| P10 | WASM/web + native packaging | done — release bundle builds via `scripts/build-web.sh`; a same-origin combined server/WASM image, compression, health/readiness probes, exact Origin checks, and deployment instructions are verified. Native ships as `cargo build --release -p cat-desktop` + `BEVY_ASSET_ROOT`/`CAT_SERVER_URL`. Transfer-weight optimization remains optional. See `docs/migration/WASM.md` |
| P11 | Cutover (retire the TS reference tree, big-bang) | done — 2026-07-11: fast-forwarded `main` → the Rust workspace and removed the TypeScript tree (`app/ components/ db/ hooks/ lib/ server/ tests/ types/ worker/` + JS build configs). Preserved on `archive/web-game` (tag `web-final`, `8d3bc5a`). `main` is now the Rust/Bevy game. |
| P12 | Sim expansion: skills, officers, spatial stockpiles, workshop chains | done — seven specialist manual/officer domains, the bounded Leader safety floor, all 19 skills, role gates, selectable physical shrine offerings, finite equipment/storage, physical confidential Accountant rounds, ten processor types, 108 recipes, Steward reserves, twelve real Crews multi-worker domains, thirteen completed-building services, physical Mouse Farm Food, source cargo, material families, and all 487 live research nodes pass the integrated correction gate |
| P13 | Client UI for P12: stockpile designation, officer assignment | done — designation/assignment, signed manual orders, exact construction/farm/gather/road/governance controls, election timing, per-cat labor preferences, multi-worker station and queue controls, all ten processor inspectors, Steward provenance, the zero-`FUTURE` 487-study ledger, crop/timber state, farm stages, and distinct open stations are verified |
| P14 | Spatial placement: footprints, tile occupancy, soft obstacles, road accessibility | done — atomic action validation, reservations, connectivity, scaffold recovery, resolved multi-cell decoration occupancy, exact occupancy/roads, persisted exterior agricultural claims, durable outer-before-inner wall construction with atomic one-gate cutover, and physical authored-road labor pass the generalized visual gate |
| P15 | Playtest-feedback backlog: controls/feel, fog-of-war, booster, movement smoothing | done — movement/booster, connected-road grammar, single-body cat rendering, exact controls, election timing, knowledge-blind shrine-return search, restart-safe notebooks, fast first wood, spatial personal needs, useful-labor pressure, exact 48-hour passive campaigns, all 52 public actions, guided play, and physical production pass the integrated correction campaign |
| P16 | Founding village blueprint, gather spots, tile recalibration | done — the 15-adult/three-five-bed-Den lifecycle, gate-routed migration/pregnancy/aging/reset, physical emergency water, interior clearing, exterior water, roads, gather controls, outside-wall agriculture, physical farming/fishing, founding benches, and production routes are verified |
| P17 | Climate-driven biome generator (~26 biomes), mining, crop fertility, transport upgrades | done — climate generation, crop fertility, finite Gem/Clay/Sand deposits and physical extraction, ore/metal, exterior plots, finite fish habitats, cached fine-biome movement, and exact constructed/staffed Rail plus Shipping routes are live |
| P18 | Visual polish: DF-Steam parchment UI, craft-station sprites | done — label-free building compositions, Adventure skin, research ledger, top-down palisades, staged walls, exterior agriculture, optimized-WASM interaction, unique tracked glyphs for all 32 resources, a four-store survival HUD, complete Stores menu, and category command dock pass the generalized framebuffer gate recorded in `docs/FIX_LOG.md` |
| P19 | Item/material economy: crafting chains, traders, coin | done — the canonical source/station/taxonomy contract preserves stable IDs and open-top buildings; raw Stone/Bone/Gem/Clay/Sand sources, Fibre→Thread→Cloth, four canonical Mug materials, scaffold inputs, all 108 recipes across ten processor types, every generated recipe/resource family, exact finite goods, equipment, visiting/village traders, coin, weight, wear, breakage, and repair pass the generalized production campaign |
| P20 | Comprehensive review hardening | done — all findings in `docs/reviews/` are dispositioned: hostile action/path bounds, finite wire projection and protocol versioning, network/session/origin limits, persistence diagnostics/readiness, transport status and onboarding, role cues, dependency policy, and documentation truth pass the consolidated gates recorded in `docs/FIX_LOG.md` |

**Notes on P12–P20**: these phases were decomposed and executed after this board's card
format fell out of active use for day-to-day tracking — the per-slice specs live in
`docs/migration/specs/p12-idle-cat-forest.md` through `p19-items-materials-trade.md`, and
P20's evidence and dispositions live in `docs/reviews/`. The git log records what landed (commit
subjects are tagged with phase/slice where applicable).
This table is a summary rollup, not a card-by-card log; see “P12–P19 — what actually landed”
below for the historical feature list and
`docs/IMPLEMENTATION_AUDIT.md` for the evidence-backed status and remaining acceptance matrix.

### P21.1 Playtest feedback backlog   [status: dev]
persona: test-engineer -> developer -> qa            depends_on: [P15, P18]        parallel_group: P21-feedback
scope: Implement the complete 2026-07-22 playtest-feedback request. The first partial slice compacts
Dens to 2x2, uses complete building and tree footprints for world interaction, labels paved streets
and worn paths, de-synchronizes actor walk cycles, and gives otherwise-idle work-capable cats a
low-priority maintenance job. Storage/barrels, shrine demand, job visualization, walls/pathfinding,
farm/fishing/road authoring, leader autonomy, and the full-screen Village/log/map UI remain open.
partial evidence: Regression tests cover the Den footprint, 3x3 workshop hover, multi-tile tree hover,
street/path labels, stable per-cat animation phases, 100% autonomous adult employment, explicit
maintenance job/task projection, wire literals, duration, and skill mapping. Focused Nextest,
workspace smoke (74/74), touched-crate Clippy with `-D warnings`, and rustfmt pass on 2026-07-22.
notes: The TypeScript-parity `direct_colony` scorer remains unchanged; evergreen maintenance is
appended only by `automated_plan`, after all survival, staffing, production, and scouting slots.
This card must not return to `done` until every item in the original feedback request has direct
tests where practical and has been exercised in a live graphical playtest.

---

## P0 — Foundation & safety (orchestrator does directly)
### P0.1 Commit Bevy 0.19 spike   [status: done]
### P0.2 Archive web game → `archive/web-game` + tag `web-final`   [status: done]
### P0.3 Close godot + love2d worktrees   [status: done]
### P0.4 Scaffold Cargo workspace (6 crates) + salvage spike   [status: done]
### P0.5 Rust toolchain + lefthook + this board   [status: done]
persona: orchestrator
scope: install cargo-nextest + cargo-llvm-cov; add glob-scoped Rust lefthook
steps (fmt on pre-commit; clippy + nextest on pre-push); write this board;
refresh the stale bevy README.
### P0.6 Stand up codex persona org   [status: done]
persona: orchestrator
scope: `codex/personas/*.md`, per-persona codex profiles, repo `AGENTS.md`, MCP
wired into codex, one end-to-end smoke card.
### P0.7 Golden-master fixture generator   [status: done (execution deferred to P7)]
persona: orchestrator
scope: `scripts/gen-golden.ts` drives the TS sim headlessly (pinned Math.random +
worldSeed + tick RNG) and emits per-tick AGGREGATE snapshots (resources, pop,
job/activity counts, threat, status) under `docs/migration/fixtures/`.
note: Tooling written + committed. First run is deferred to P7 (full-tick parity):
the archived app's `node_modules` is partial, so running it needs `bun install`.
P1–P6 use module-level fixtures / hand-computed vectors (see P1.1) instead, per the
"same idea" fidelity bar (bit-exact full-tick determinism is out of scope).

---

## P1+ cards
Generated by the `scrum-master` persona at the start of each phase, reviewed by the
orchestrator, then dispatched in `parallel_group` waves. Appended below as phases open.

### P1.1 Seeded RNG module   [status: done]
persona: test-engineer            depends_on: []        parallel_group: P1
scope: Write failing Rust tests and minimal skeleton for the deterministic seeded
LCG ported from `lib/game/seededRng.ts`.
acceptance: `cargo nextest run -p cat-sim` compiles and fails because RNG functions
are still unimplemented.
notes: TS source `lib/game/seededRng.ts`; TS tests `tests/unit/game/seededRng.test.ts`.

### P1.2 Core enum taxonomy   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P1.1]        parallel_group: P1-foundation
scope: Port the closed string-literal enums from `types/game.ts` into one
`cat-sim` type module: `LifeStage`, `BuildingType`, `TileType`, `TaskType`,
`EnemyType`, `JobKind`, `JobStatus`, `CatSpecialization`, `UpgradeKey`, and
`PolicyTier`.
acceptance: Add table-driven Rust tests proving every TS literal is represented
exactly once and round-trips through the chosen Rust string conversion API; then
`cargo nextest run -p cat-sim`, `cargo clippy -p cat-sim --all-targets -- -D warnings`,
and `cargo fmt` pass.
notes: TS source `types/game.ts`; parity criterion is exact variant coverage and
exact snake-case/wire-string spelling from the TS union definitions.

### P1.3 Core cat/colony state fields   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P1.2]        parallel_group: P1-after-types
scope: Add the minimal `cat-sim` state structs/value objects needed by later sim
modules, porting `Resources`, `CatStats`, `CatNeeds`, `Position`, `Colony`, `Cat`,
and carrying/activity support from `types/game.ts` plus schema-only fields in
`db/schema.ts`: `worldSeed`, `threatPressure`, `destination`, `carrying`,
`activity`, `ageHours`, `pregnancyDueAgeHours`, and `pregnancyMateId`.
acceptance: Add Rust tests that construct a colony and cat with every schema-only
field present and verify TS-compatible defaults/optionality for legacy rows:
`worldSeed` absent, `threatPressure` read as `0`, `destination`/`carrying` absent,
`activity` as `idle`, `ageHours` as `0`, and pregnancy extension fields absent
unless pregnant; then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS sources `types/game.ts`, `db/schema.ts`; this is state shape only, with
no SQLite/persistence implementation and no shrine deposit behavior.

### P1.4 Needs, life-stage, and leader constants   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P1.2]        parallel_group: P1-after-types
scope: Port the small scalar/range constant tables from `types/game.ts` into
`cat-sim`: `NEEDS_DECAY_RATES`, `NEEDS_RESTORE_AMOUNTS`, `LIFE_STAGE_HOURS`, and
`LEADER_QUALITY`.
acceptance: Add Rust unit tests with literal parity vectors for every table entry,
including `elder` ending at infinity and each leader tier's min/max/time/wrong
chance; then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `types/game.ts`; parity criterion is exact numeric values and
range boundaries.

### P1.5 Combat, building, and task mapping constants   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P1.2]        parallel_group: P1-after-types
scope: Port the enum-keyed constant tables from `types/game.ts` into `cat-sim`:
`ENEMY_STATS`, `BUILDING_COSTS`, and `TASK_TO_SKILL`.
acceptance: Add Rust unit tests with literal parity vectors for every enemy click
cost/damage range, every building material cost, and every task-to-stat mapping;
tests must also fail if a future enum variant lacks a table entry; then
`cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `types/game.ts`; parity criterion is exact table coverage and
exact numeric/string mapping values, including `shrine: 0` and `rest: defense`.

### P1.6 Test acceleration presets   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P1.1]        parallel_group: P1-foundation
scope: Port `lib/game/testAcceleration.ts` into a small `cat-sim` module covering
the `off`, `fast`, `turbo`, `hyper`, and `ludicrous` presets plus
`presetFromTimeScale`.
acceptance: Add Rust tests mirroring `tests/unit/game/testAcceleration.test.ts`
and extending coverage to `hyper`, `ludicrous`, `null`/missing-equivalent input,
scale `100`, scale `10_000`, and the `120 <= scale < 10_000 && scale != 100`
turbo rule; then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS sources `lib/game/testAcceleration.ts`,
`tests/unit/game/testAcceleration.test.ts`; parity criterion is exact config
values and branch ordering from the TS implementation.

---

## P2 — World generation
### P2.1 Predeclare world-generation modules   [status: done]
persona: orchestrator            depends_on: [P1.1, P1.2]        parallel_group: P2-scaffold
scope: Prepare the `cat-sim` module surface for parallel P2 work by declaring
`noise`, `biomes`, `terrain_gen`, and `world_gen` under `crates/cat-sim/src/`
without implementing terrain behavior.
acceptance: `cargo nextest run -p cat-sim` still compiles after the empty module
surface is in place; no TS files or product logic are changed.
notes: Rust module targets `crates/cat-sim/src/noise.rs`,
`crates/cat-sim/src/biomes.rs`, `crates/cat-sim/src/terrain_gen.rs`,
`crates/cat-sim/src/world_gen.rs`; this is a parallel-safety setup card.

### P2.2 Seeded noise utilities   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.1]        parallel_group: P2-foundation
scope: Port `lib/game/noise.ts` into `crates/cat-sim/src/noise.rs`: the local
`SeededRandom` API shape needed by world generation, `hashSeed`, `noise2D`, and
`fractalNoise2D`.
acceptance: Add Rust tests from a TS-generated vector fixture covering
`hashSeed` with mixed number/string inputs, `next`/`int`/`float`, `noise2D`, and
`fractalNoise2D` across positive and negative coordinates; repeated calls with
the same seed must be byte-identical, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/noise.ts`; fixture
`docs/migration/fixtures/p2/noise_vectors.json`; parity criterion is exact
32-bit hash/LCG semantics and JS-number-equivalent noise values.

### P2.3 Biome tables and calculators   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.1, P1.2]        parallel_group: P2-foundation
scope: Port `lib/game/biomes.ts` into `crates/cat-sim/src/biomes.rs`: biome and
overlay feature enums, `BIOME_PROPERTIES`, `OVERLAY_FEATURE_PROPERTIES`,
`calculateDangerLevel`, and `calculateTravelSpeed`.
acceptance: Add table-driven Rust tests proving every biome/overlay literal,
resource range, max resource, danger modifier, speed modifier, path-wear value,
and display name matches TS; include distance/danger clamp vectors and repeated
determinism checks, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/biomes.ts`; fixture
`docs/migration/fixtures/p2/biome_vectors.json`; parity criterion is exact table
coverage and exact calculator outputs. QA rerun 2026-07-13: all five fixture-backed
literal/property/calculator/NaN/determinism tests passed; strict `cat-sim` Clippy passed.

### P2.4 Terrain generator researcher spec   [status: done]
persona: researcher            depends_on: [P2.2]        parallel_group: P2-research
scope: Produce a port spec for `lib/game/terrainGen.ts` covering the copied
value-noise math, `TerrainOptions`, `WORLD_TERRAIN_OPTIONS`, height/moisture,
plateau behavior, cliff/stair roles, river source/path logic, biome/decoration
classification, and `generateTerrainChunk`.
acceptance: Write `docs/migration/p2-terrain-gen-spec.md` with exported Rust API
names, internal helper behavior that must be preserved, fixture seed/chunk
matrix, float-comparison policy, and hard edge cases for negative chunks and
chunk borders; no Rust product code changes.
notes: TS source `lib/game/terrainGen.ts`; recommended before P2.6-P2.10 because
this module is client-parity-critical.

### P2.5 World generation researcher spec   [status: done]
persona: researcher            depends_on: [P2.2, P2.3]        parallel_group: P2-research
scope: Produce a port spec for `lib/game/worldGen.ts` plus the distinct bridge
logic in `lib/game/terrainWorld.ts`, including chunk/tile mapping, colony anchor
constants, legacy Voronoi chunk generation, starter-water guarantees, and the
terrain-to-gameplay `WorldTile` mapping.
acceptance: Write `docs/migration/p2-world-gen-spec.md` with exported Rust API
names, exact dependency boundaries between `terrain_gen.rs` and `world_gen.rs`,
fixture seed/chunk matrix, and edge cases for safe-radius water suppression and
starter pond selection; no Rust product code changes.
notes: TS sources `lib/game/worldGen.ts`, `lib/game/terrainWorld.ts`;
recommended before P2.11-P2.14 because `terrainWorld.ts` is distinct gameplay
mapping logic.

### P2.6 Terrain options and scalar fields   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.4]        parallel_group: P2-terrain-core
scope: Port the foundational slice of `lib/game/terrainGen.ts` into
`crates/cat-sim/src/terrain_gen.rs`: public role/type vocabulary, constants
`TERRAIN_CHUNK_SIZE` and `DEFAULT_MAX_HEIGHT`, option resolution,
`WORLD_TERRAIN_OPTIONS`, copied `hashSeed`/`latticeValue`/`fade`/`valueNoise`/
`fractalNoise`, `terrainElevationAt`, `terrainMoistureAt`, and
`terrainHeightAt` with plateau behavior.
acceptance: Add Rust tests from TS vectors for resolved defaults, world terrain
options, elevation/moisture samples, height quantization, plateau override, and
negative coordinates; repeated calls with the same seed/options must be
identical, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/terrainGen.ts`; fixture
`docs/migration/fixtures/p2/terrain_fields.json`; parity criterion is exact
discrete height/options parity and JS-number-equivalent scalar fields.

### P2.7 Terrain cliff and stair roles   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.6]        parallel_group: P2-terrain-roles
scope: Port the cliff and stair slice of `lib/game/terrainGen.ts` into
`terrain_gen.rs`: direction constants, `classifyCliff`, terrain role lookup,
`stairEdgeDir`, run scanning, `deriveStairs`, and public `terrainStairAt`.
acceptance: Add direct fixture tests for every cliff mask family
(`flat`, `edge`, `corner`, `ridge`, `spur`, `pillar`), max-drop calculation,
single-floor stair eligibility, minimum-run midpoint placement, and deterministic
repeat calls, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/terrainGen.ts`; fixture
`docs/migration/fixtures/p2/terrain_cliffs_stairs.json`; parity criterion is
exact role/variant/facing/edge-mask output.

### P2.8 Terrain river roles   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.6]        parallel_group: P2-terrain-roles
scope: Port the river slice of `lib/game/terrainGen.ts` into `terrain_gen.rs`:
`regionRiverSources`, `traceRiver`, `classifyRiverSegment`, and the chunk-local
river collection behavior used by `generateTerrainChunk`.
acceptance: Add TS-vector tests for source selection by region, threshold
filtering, plateau avoidance, steepest-descent path tracing, segment
classification (`start`, `straight`, `bend`, `end`), max-length stopping, and
deterministic repeated calls, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/terrainGen.ts`; fixture
`docs/migration/fixtures/p2/terrain_rivers.json`; parity criterion is exact
source/path tile coordinates and river role fields.

### P2.9 Terrain biome and decoration roles   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.6]        parallel_group: P2-terrain-roles
scope: Port the biome and decoration slice of `lib/game/terrainGen.ts` into
`terrain_gen.rs`: `classifyBiome`, biome decoration densities, and deterministic
tree/rock decoration derivation.
acceptance: Add Rust tests for height/moisture biome thresholds, all decoration
density bands, tree species, rock size/resource selection, decoration absence,
and deterministic repeated calls, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/terrainGen.ts`; fixture
`docs/migration/fixtures/p2/terrain_biome_decor.json`; parity criterion is exact
biome role and optional decoration output for fixture coordinates.

### P2.10 Terrain chunk assembly   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.7, P2.8, P2.9]        parallel_group: P2-terrain-assembly
scope: Port `generateTerrainChunk` from `lib/game/terrainGen.ts` into
`terrain_gen.rs`, assembling 12x12 world-coordinate tiles with elevation,
moisture, height, biome, terrain, river, stairs, optional decoration, optional
river carving, and stable tile ordering.
acceptance: Add golden tests for multiple `(seed, chunkX, chunkY, opts)` cases,
including `(0,0)`, negative chunks, adjacent chunk borders, default options,
`WORLD_TERRAIN_OPTIONS`, `carveRivers`, and `decorate: false`; same seed+chunk
must produce tiles matching archived TS tile-for-tile and repeated Rust calls
must be identical, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/terrainGen.ts`; fixture
`docs/migration/fixtures/p2/terrain_chunks.json`; parity criterion is exact
tile order and exact discrete fields, with scalar float tolerance documented by
P2.4.

### P2.11 World chunk coordinate helpers   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.5]        parallel_group: P2-world-core
scope: Port the coordinate and colony-anchor slice of `lib/game/worldGen.ts` into
`crates/cat-sim/src/world_gen.rs`: chunk size usage, `COLONY_SAFE_RADIUS`,
`COLONY_WATER_RADIUS`, `tileToChunk`, `chunkToTile`, and `getColonyPosition`.
acceptance: Add Rust tests mirroring TS for positive, zero, and negative tile
coordinates, top-left chunk origin conversion, constants, colony position
`{ x: 6, y: 6 }`, and deterministic repeated calls, then
`cargo nextest run -p cat-sim`, `cargo clippy -p cat-sim --all-targets -- -D warnings`,
and `cargo fmt` pass.
notes: TS source `lib/game/worldGen.ts`; fixture
`docs/migration/fixtures/p2/world_coords.json`; parity criterion is exact
`Math.floor` chunk mapping, especially for negative coordinates.

### P2.12 Legacy world overlay generation   [status: skipped — dead code: worldGen.generateChunk (Voronoi) is superseded by terrainWorld.generateWorldChunk; server/worldMap.ts uses the latter]
persona: test-engineer -> developer -> qa            depends_on: [P2.2, P2.3, P2.5, P2.11]        parallel_group: P2-world-legacy
scope: Port the overlay-generation slice of `lib/game/worldGen.ts` into
`world_gen.rs`: Voronoi cell generation, nearest-cell lookup, biome-boundary
check, river/path predicates, and overlay priority in `getOverlayFeature`.
acceptance: Add TS-vector tests for cell placement, nearest biome selection,
boundary detection, river/path thresholds, overlay priority, orthogonal river
connection handling, and deterministic repeated calls, then
`cargo nextest run -p cat-sim`, `cargo clippy -p cat-sim --all-targets -- -D warnings`,
and `cargo fmt` pass.
notes: TS source `lib/game/worldGen.ts`; fixture
`docs/migration/fixtures/p2/world_overlays.json`; parity criterion is exact
biome/overlay decisions for fixture coordinates.

### P2.13 Legacy world chunk tiles   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.12]        parallel_group: P2-world-legacy
scope: Port the gameplay tile/chunk slice of `lib/game/worldGen.ts` into
`world_gen.rs`: `generateTile`, `generateChunk`, resource rolls, danger/path-wear
mapping, backward-compatible tile type selection, and `ensureWaterNearColony`.
acceptance: Add golden tests for same `(seed, chunkX, chunkY, colonyX, colonyY)`
fixtures against archived TS `generateChunk`, including colony chunk starter
water, non-colony chunks, safe-radius river suppression, and deterministic
repeated calls, then `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt` pass.
notes: TS source `lib/game/worldGen.ts`; fixture
`docs/migration/fixtures/p2/world_chunks_legacy.json`; parity criterion is exact
144-tile order, tile type, resources, max resources, danger, path wear,
`lastDepleted`, and overlay feature.

### P2.14 Terrain-driven world chunks   [status: done]
persona: test-engineer -> developer -> qa            depends_on: [P2.3, P2.10, P2.11, P2.13]        parallel_group: P2-world-bridge
scope: Port `lib/game/terrainWorld.ts` into `world_gen.rs`: `WorldTileData`,
terrain biome role to gameplay biome/tile mappings, `terrainToWorldTile`,
terrain-backed `ensureWaterNearColony`, and `generateWorldChunk`.
acceptance: Add golden tests for same `(seed, chunkX, chunkY, colonyX, colonyY)`
fixtures against archived TS `generateWorldChunk`, including river tiles,
non-river resource rolls, terrain biome mappings, starter pond forcing, and
deterministic repeated calls; same seed+chunk must match TS tile-for-tile, then
`cargo nextest run -p cat-sim`, `cargo clippy -p cat-sim --all-targets -- -D warnings`,
and `cargo fmt` pass.
notes: TS source `lib/game/terrainWorld.ts`; fixture
`docs/migration/fixtures/p2/world_chunks_terrain.json`; parity criterion is exact
gameplay `WorldTile` output generated from the shared terrain field.

### P2.15 P2 parity QA gate   [status: done (orchestrator gate: determinism + no-orphan-fixtures + water-guarantee spot-check; deep codex QA timed out at xhigh)]
persona: qa            depends_on: [P2.10, P2.13, P2.14]        parallel_group: P2-qa
scope: Independently verify the completed P2 world-generation surface in
`cat-sim` without adding new product behavior.
acceptance: Run the full P2 fixture suite plus `cargo nextest run -p cat-sim`,
`cargo clippy -p cat-sim --all-targets -- -D warnings`, and `cargo fmt --check`;
spot-check at least three seeds across origin, negative, and adjacent chunks for
terrain and world chunk parity, confirming deterministic reruns and no TS edits.
notes: TS sources `lib/game/noise.ts`, `lib/game/biomes.ts`,
`lib/game/terrainGen.ts`, `lib/game/worldGen.ts`, `lib/game/terrainWorld.ts`;
this closes the client-parity-critical P2 wave.

## P3 — Cat AI (decomposed by orchestrator; scrum-master timed out at xhigh)
### P3.1 Pathfinding researcher spec   [status: done]
persona: researcher            depends_on: []        parallel_group: P3-research
scope: Spec lib/game/pathfinding.ts -> docs/migration/specs/pathfinding.md: WalkGrid
interface, cost model (road .4/worn .6/open 1/forest 4/dense 8, MIN_STEP_COST .4),
X_FIRST_BIAS 1e-6 tie-break, ROAD_WEAR_THRESHOLD 70, DEFAULT_MAX_EXPANSIONS 6000,
deterministic min-heap insertion-order tie-break, buildColonyWalkGrid, findPath,
fence/gate/river blocking. Fixture matrix for byte-identical routes.
### P3.2 Leader director researcher spec   [status: done]
persona: researcher            depends_on: []        parallel_group: P3-research
scope: Spec lib/game/leaderDirector.ts + leaderAI.ts -> docs/migration/specs/leader_director.md:
response curves (deficit/projection/pressure/surplus/combineOr), all tunables,
LeaderSnapshot ~40 fields, laborGoals 8 kinds, directColony budget allocation +
fixed order[] tie-break, matchCatsToSlots greedy (skill fit x1.5 spec, id tie-break).
### P3.3 Pathfinding A* + WalkGrid   [status: done]
persona: developer -> qa            depends_on: [P3.1]        parallel_group: P3-core
scope: Port pathfinding.ts -> crates/cat-sim/src/pathfinding.rs. WalkGrid, cost model,
deterministic A* with byte-identical routes, buildColonyWalkGrid, findPath + straight-walk fallback.
acceptance: TS golden fixture (routes for a matrix of start/goal on varied grids incl. fence/gate/river/roads); identical path tiles; determinism.
### P3.4 Movement   [status: done]
persona: developer -> qa            depends_on: [P3.3]        parallel_group: P3-movement
scope: Port movement.ts -> crates/cat-sim/src/movement.rs (advanceMovement x-before-y,
pathTiles, walkPath, pickWanderTarget, destinationForJob; MOVE_SPEED .5, WANDER_RADIUS 3, etc).
acceptance: TS golden fixture; determinism (forked movement seed).
### P3.5 Policy tiers   [status: done]
persona: developer -> qa            depends_on: []        parallel_group: P3-independent
scope: Port policy.ts -> crates/cat-sim/src/policy.rs (bucketFromLeadership, weightsForLeadership,
pickPolicyTier, configForTier, PolicyConfig). acceptance: literal parity + tier boundaries.
### P3.6 Leader snapshot contract   [status: done]
persona: developer -> qa            depends_on: []        parallel_group: P3-independent
scope: Port leaderAI.ts LeaderSnapshot (~40 fields), LeaderDecision union, planLeaderActions
-> crates/cat-sim/src/leader_ai.rs. acceptance: struct shape + planLeaderActions flatten vs TS.
### P3.7 Leader director (IAUS)   [status: done]
persona: developer -> qa            depends_on: [P3.2, P3.6]        parallel_group: P3-director
scope: Port leaderDirector.ts -> crates/cat-sim/src/leader_director.rs: curves, tunables,
laborGoals, directColony (budget + fixed tie-break order), matchCatsToSlots, targetWarriors.
acceptance: TS golden fixture (snapshot -> decisions+slots identical); cross-axis trade-off cases.
### P3.8 Task assignment helpers   [status: done]
persona: developer -> qa            depends_on: []        parallel_group: P3-independent
scope: Port tasks.ts -> crates/cat-sim/src/tasks.rs (getOptimalCatForTask, getAssignmentTime,
getAssignedCat). Note Math.random usage — seed it or model behaviourally; document.
### P3.9 Autonomous needs behavior   [status: done]
persona: developer -> qa            depends_on: []        parallel_group: P3-independent
scope: Port catAI.ts -> crates/cat-sim/src/cat_ai.rs (getAutonomousAction priority chain).
acceptance: decision-table parity (return->eat<30->drink<40->sleep<20).
notes: Post-cutover physical-consistency pass (2026-07-16) wires these decisions to finite
Food/Fish/Water pickup and dining routes plus five-bed Den reservations. Persisted carrying
markers conserve cargo and interrupted work routes across blockage, death, and restart.
### P3.10 P3 parity QA gate   [status: done (orchestrator gate: fixture-backed exact-output parity, tie-break logic spot-verified, 125 tests deterministic; codex QA timed out at xhigh even focused)]
persona: qa (orchestrator gate if xhigh times out)        depends_on: [P3.3,P3.4,P3.5,P3.6,P3.7,P3.8,P3.9]
scope: determinism, no orphan fixtures, JS-trap audit across the cat-AI modules.

## P4 — Life sim (decomposed by orchestrator)
### P4.1 Needs   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P4a
scope: Port lib/game/needs.ts -> crates/cat-sim/src/needs.rs (decay/restore/damage/critical helpers). Fixture-backed.
### P4.2 Age   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P4a
scope: Port lib/game/age.ts -> crates/cat-sim/src/age.rs (getAgeInHours, getLifeStage, getDeathChance, getAgeSkillModifier, canPerformTask; shouldDieOfOldAge uses Math.random -> injected roll). Fixture-backed.
### P4.3 Breeding   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P4a
scope: Port lib/game/breeding.ts -> crates/cat-sim/src/breeding.rs (calculateFertilityBonus cap .5, calculateBreedingChance cap .8).
### P4.6 Genetics   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P4a
scope: Port lib/game/genetics.ts -> crates/cat-sim/src/genetics.rs (sprite-trait inheritance, traitsToSpriteParams). Math.random throughout -> injected rolls; document. Cosmetic but affects breeding output.
### P4.4 Survival   [status: done]
persona: developer -> qa   depends_on: [P4.1]   parallel_group: P4b
scope: Port lib/game/survival.ts -> crates/cat-sim/src/survival.rs (applySurvivalTick: 10-min unit normalization, availability-driven decay, damage, death; policy multipliers).
### P4.5 Life simulation   [status: done]
persona: developer -> qa   depends_on: [P4.1,P4.2,P4.3]   parallel_group: P4b
scope: Port lib/game/lifeSim.ts -> crates/cat-sim/src/life_sim.rs (stageWorkEffectiveness, canWork, workforceWeight, oldAgeDeathProbability, breeding gates+constants, colonyCanBreed, conceptionProbability, inheritStats 60/40+-8 deterministic, trade curves, leadershipAfterTenure). Fixture-backed.
### P4.7 P4 QA gate   [status: done (orchestrator gate: 170 tests, fixture/hand-vector exact-value parity, deterministic; QA times out at high on this many modules)]
persona: qa (orchestrator gate if timeout)   depends_on: [P4.1,P4.2,P4.3,P4.4,P4.5,P4.6]

## P5 — Economy + housing + roads (decomposed by orchestrator)
### P5.1 Idle engine   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5a
scope: lib/game/idleEngine.ts -> crates/cat-sim/src/idle_engine.rs (BASE_JOB_SECONDS, getDurationSeconds, getScaledDurationSeconds, getHuntReward, getResilienceHours, getUpgradeCost, nextSpecialization, applyClickBoostSeconds).
### P5.2 Idle rules   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5a
scope: lib/game/idleRules.ts -> crates/cat-sim/src/idle_rules.rs (consumptionForTick, nextColonyStatus, shouldTrackCritical/ResetFromCritical, auto-queue rules, ritualRequestIsFresh).
### P5.3 Production   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5a
scope: lib/game/production.ts -> crates/cat-sim/src/production.rs (workshop 5mat->1refined/600s architect x2; field 2food/hr; unlock levels).
### P5.4 Smithy   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5a
scope: lib/game/smithy.ts -> crates/cat-sim/src/smithy.rs (2refined+3mat->1weapon+1armor/900s, fast smith x2).
### P5.5 Storage   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5b
scope: lib/game/storage.ts -> crates/cat-sim/src/storage.rs (BASE_CAPACITY, granary/water-bowl/smithy bonuses, storageCapacities, storehouseCap, countStorehouses).
### P5.6 Shrine + trips   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5b
scope: lib/game/shrine.ts + trips.ts -> crates/cat-sim/src/shrine.rs + trips.rs (deposit rules DEPOSIT_GRACE_MS 60000, DEPOSIT_RADIUS 1; HUNT_TRIP_COUNT 3, splitYield, tripDueAt).
### P5.7 Depletion + spoilage   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5b
scope: lib/game/depletion.ts + spoilage.ts -> crates/cat-sim/src/depletion.rs + spoilage.rs (FOREST_TYPES, regrowthAmount, CHOPPED_FOREST_FOOD_CAP 5; spoilage report).
### P5.8 Housing + roads   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5c
scope: lib/game/housing.ts + roads.ts -> crates/cat-sim/src/housing.rs + roads.rs (housingCapacity/pressure, villageLevel thresholds [6,12,20,30]; ROAD_PAVE_WEAR 70, selectRoadCorridor).
### P5.9 Village layout   [status: done]
persona: developer -> qa   depends_on: []   parallel_group: P5c
scope: lib/game/villageLayout.ts -> crates/cat-sim/src/village_layout.rs (VILLAGE_ANCHOR, colonyToWorld/worldToColony, ringCells, nextBuildingSite, villageRadius).
### P5.10 Village area   [status: done]
persona: developer -> qa   depends_on: [P5.9]   parallel_group: P5d
scope: lib/game/villageArea.ts -> crates/cat-sim/src/village_area.rs (organic claimed-area set, fence perimeter/mask/segments, gatePlacement, fenceBlocksMove, expandVillage, shouldExpand).
### P5.11 P5 QA gate   [status: done (orchestrator gate: 259 tests hand-vector/exact-value parity, deterministic)]
persona: qa (orchestrator gate if timeout)   depends_on: [P5.1..P5.10]

## P6 — Military + governance + upgrade tree (decomposed by orchestrator)
### P6.1 Threat   [status: done]
scope: lib/game/threat.ts -> threat.rs (colonyWealth, threatRatePerHour grace 8h, accrueThreat, RAID_SPAWN_THRESHOLD 100, threatBand thirds, planRaid MAX_RAID_SIZE 12, resolveRaid +-25% MAX_LOOT .3 CASUALTY .6). depends_on:[] group:P6a
### P6.2 Warriors   [status: done]
scope: lib/game/warriors.ts -> warriors.rs (combat role/stage factors, WEAPON/ARMOR bonus 25, catCombatPower, musterDefense gear-to-strongest, canFight). depends_on:[] group:P6a
### P6.3 Combat   [status: done]
scope: lib/game/combat.ts -> combat.rs (calculateCombatResult [Math.random->injected roll], getClicksNeeded, calculateColonyDefense walls cap 100). depends_on:[] group:P6a
### P6.4 Elections   [status: done]
scope: lib/game/elections.ts -> elections.rs (KICK_THRESHOLD 5, CANDIDATE_COUNT 5, TERM_MS 24h, windows; candidatesFor, tallyVotes, electionWinner, shouldTriggerKick, electionDue). depends_on:[] group:P6a
### P6.5 Zones   [status: done]
scope: lib/game/zones.ts -> zones.rs (ZONE_MAX_PER_PLAYER 2, ZONE_MAX_EDGE 8, GATHER_MULTIPLIER 2, scoreTileWithZones, filterTargetsByZones, pickTargetWithZones, validateZone). depends_on:[] group:P6b
### P6.6 Upgrade tree + research   [status: done]
scope: lib/game/upgradeTree.ts -> upgrade_tree.rs (11 EffectKeys, resolveEffects, UPGRADE_NODES ~18 nodes 3 eras VERBATIM, state ser/de, isOwned/prerequisitesMet/canUnlock/unlockableNodes/godPurchase; research RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK 10, accrueResearch, nextResearchTarget cheapest+id-tiebreak, catAutoUnlock). depends_on:[] group:P6b
### P6.7 P6 QA gate   [status: done (orchestrator gate: 312 tests hand-vector exact-value parity, deterministic)]
persona: qa/orchestrator   depends_on:[P6.1..P6.6]

## P7 — Master loop (world_tick, multi-colony) (decomposed by orchestrator)
### P7.1 Runtime state + world_tick skeleton   [status: done]
persona: developer   depends_on:[]   scope: WorldState/ColonyRuntime/JobRuntime structs (per spec) + world_tick(state, now) iterating colonies, calling 37 phase fns as stubs; compiles + empty-world test. crates/cat-sim/src/world_tick.rs.
### P7.2..P7.N Phase ports   [status: done]
persona: developer   depends_on:[P7.1]   scope: port the 37 phases in small groups (elapsed gate/rng forks; life sim; consumption/spoilage/clamp; minute gate/elections/zones; path decay/regrowth; job promotion; leader plan/director/assignment; production/research; due-job completion; hauling; movement; roads; raids; status/persist). Validate vs golden fixtures (scripts/gen-golden.ts).
### P7.gate P7 parity gate   [status: done (integration: deterministic, 40-tick survival, multi-colony independence; exact aggregate match vs TS mulberry32 fixture not required per 'same idea')]
persona: qa/orchestrator   scope: seed -> N ticks aggregate trajectory matches worker-tick golden fixture; multi-colony independence.

### P7.followup BuildingType research_hut/school   [status: done]
note: closed by the ResearchHut/School ports (`aa938a3`-era + `133165e`): both variants
exist in `types.rs`, are buildable/staffable research faucets in `world_tick`, and School
adds a +50% research-rate multiplier via its upgrade node.

## P8 — Protocol + server (+ multi-village founding) (decomposed by orchestrator)
### P8.1 cat-protocol wire types   [status: done]
persona: developer   scope: crates/cat-protocol: WorldSnapshot (multi-colony generalization of getGlobalDashboard payload) + per-colony ColonySnapshot (resources+caps, leader, cats, jobs, upgrades, events, housing, research, election, voteKick, zones, threat, raiders, buildings, storage, claimedTiles, gate, villageRadius, anchor, worldSeed, onlineCount) + ClientAction enum (~19 actions from actions/route.ts + foundVillage/joinVillage). serde, round-trip tests.
### P8.2 apply_action (pure) in cat-sim   [status: done]
persona: developer   depends_on:[P8.1]   scope: cat-sim: apply a ClientAction to WorldState/ColonyRuntime (requestJob, boost, purchaseUpgrade, castVote, requestVoteKick, create/removeZone, planBuilding, unlockNode, assignWorker, trainWarrior, defendRaid, buildRoad, test controls, foundVillage->found_colony). Pure; validation + soft-fail results. Snapshot builder WorldState->WorldSnapshot.
### P8.3 cat-server tick loop + transport   [status: done; responsiveness follow-up verified in `daa75e8`]
persona: developer   depends_on:[P8.2]   scope: crates/cat-server: tokio loop running world_tick each 1s; WebSocket (axum or tokio-tungstenite) broadcasting WorldSnapshot + receiving ClientAction; presence/online tracking. CPU-heavy ticks and synchronous persistence run on the blocking pool; new sockets read a last-completed snapshot cache without waiting on the authoritative world lock.
### P8.4 cat-server persistence + identity   [status: done]
persona: developer   depends_on:[P8.3]   scope: SQLite (rusqlite) save/load WorldState (mirror db/schema tables); identity/HMAC session sig; rate-limit (30 actions/10s). Migrations on open.
### P8.gate P8 integration   [status: done (LIVE: boot+/health, prompt cached WS snapshot, foundVillage grows colonies to 2, load_world reloads; save-timing/responsiveness follow-up closed in `daa75e8`)]
persona: qa/orchestrator   scope: server boots, ticks, a client connects, founds a village, submits actions, receives snapshots; persistence round-trips.

### P8.followup persistence save timing   [status: done]
historical note: P8 gate found that a colony created immediately before an abrupt kill could
precede the next periodic save. The server now saves every five completed ticks and on graceful
shutdown; save/load round trips and responsiveness are verified. Abrupt process termination can
still lose the bounded interval since the previous save, which is the documented durability
contract rather than an open migration card.

## P9 — Client render + UI — TOP-DOWN (design pivot: see docs/GAME_VISION.md)
HISTORICAL CARD NOTE: P9 pivoted from isometric to a flat TOP-DOWN grid (single level), per the
"Idle Cat Forest = idle Dwarf Fortress, cats, forest" vision. Render the live
snapshot top-down: terrain grid, cats (+carried item), the original labelled workshop markers,
visible storage/stockpiles, camera, dashboard, manual action tools first. Deeper sim
(role/officer system, spatial stockpiles, more workshops + hauling chains) = new
phase P12 after the visible world is up.
The labelled-marker wording records the original P9 slice and is superseded by the current
label-free/open-workshop direction in `docs/GAME_VISION.md` and `docs/IMPLEMENTATION_AUDIT.md`.
### P9.1 cat-client foundation (Bevy app + WS + cats)   [status: done]
scope: HISTORICAL foundation — cat-client run() Bevy app; ewebsock WS client -> WorldSnapshot resource; the first spike's projection/camera + cat atlas; render cats from the snapshot. The current renderer is flat top-down. cat-desktop main -> cat_client::run().
### P9.2 terrain/buildings/raiders/zones render (TOP-DOWN)   [status: done]
scope: HISTORICAL P9 acceptance — flat top-down terrain regenerated from worldSeed via cat-sim generate_terrain_chunk (biome colours + blue rivers + tree/rock decoration dots), original labelled building markers, cats coloured by specialization + carried-item glyph, raiders, avoid/gather zone overlays, and an on-map stockpile readout. Current rendering supersedes the marker slice with label-free roofed homes/open stations and typed inspectors; see P18 and `docs/IMPLEMENTATION_AUDIT.md`.
### P9.3 input + camera   [status: done]
scope: WASD/arrow pan, middle-drag pan, wheel zoom, R reset — centred on the village anchor. (tool modes/selection inspector deferred.)
### P9.4 dashboard + action buttons   [status: done]
scope: HUD dashboard (resources w/ caps, status, leader, pop/housing, threat, jobs) + event-log panel from the snapshot; toolbar buttons (Supply food/water, Plan hunt, Found village) -> ClientAction over WS after a Presence handshake issues the signed session. Round-trip framebuffer-verified (Supply food -> supply_food job appears next snapshot).
### P9.5 bevy_brp_extras + polish   [status: todo — superseded, see P9 table note]
scope: add BrpExtrasPlugin (brp_status green + screenshot/input MCP tools); life-stage cat scale, hats/crown/badges.
### P9.gate   [status: todo — superseded, see P9 table note]
scope: client connects to cat-server, renders the live world, an action round-trips; screenshot via bevy_brp_mcp.

---

## P12–P19 — what actually landed (not a completion checklist)

These phases ran outside the card-by-card `todo → researching → red → dev → qa → done`
workflow used above — they're tracked as design specs in `docs/migration/specs/` plus the
git log, not as individual cards on this board. `docs/IMPLEMENTATION_AUDIT.md` is the maintained
completion ledger; this section is a rollup of
what actually landed and remains in `main`, grouped by phase, for anyone auditing status
without reading ~150 commits. It intentionally does not restate every commit — see `git log`
for exact commit hashes/messages, and the corresponding spec doc for the original design.

### P12 — Sim expansion (spec: `docs/migration/specs/p12-idle-cat-forest.md`)
- **P12.1 skills** — proficiency/XP persists for all 19 maintained labors. Only truthful work
  accrues continuous or completed-cycle XP; bounded effects apply to production, movement,
  research, and combat; protocol and the cat inspector expose the typed map while accepting
  legacy four-role snapshots. The original Foraging Lore and Sawmill yield effects now reach
  explicit fibre forage and the physical logging/quarry loads respectively, before ordinary
  trip splitting and capacity checks and without changing completion timing.
- **P12.2 officers** — Steward, Accountant, Forester, Farmer, Captain, Loremaster, and Cloth
  Leader have strict specialist automation ownership. Beyond the founding Leader's bounded
  hunt/water/scout safety floor, vacancies are manual-only. Appointment requires the matching
  researched unlock and completed role station; assignment, replacement, automation provenance,
  and the rolling daily Leader-choice timestamp persist through the legacy SQLite column name.
- **P12.3 spatial stockpiles** — founding seeds a finite general storehouse; designated containers
  determine real capacity, legacy shrine stores migrate into them, and persisted transit ledgers
  reserve carried cargo without blocking the map. The founding numbers are decided and verified:
  personal villages start with 50 Food, 100 Water, 16 Herbs, 60 Materials, 10 Planks, and 10
  Blocks; communal villages receive exactly twice that mix; all other maintained scalar resources
  start at zero. All ten maintained processors and exterior-farm routes use this model. Food Storage, Water Bowl, and Smithy research
  now expands only its owned physical domains through one clamp/routing/snapshot/trade authority.
  Processor-local capacity is now target-correct too: Workshop, Mill, Sawmill, Wood Cutter,
  Stone Prep, Woodworking, Smelter, Tannery, and Clothier `stores` studies expand their owning
  persisted input/output/transit reserve from 10 to 12 units per accepted resource. The wire and
  selected-building inspector expose that physical limit. Thirteen generic `stores` studies for
  buildings with no routed container were removed rather than sold as inert/global bonuses;
  the truthful graph contains 487 studies and an exhaustive guardrail names every remaining
  capacity payload's physical consumer.
- **P12.4a/b workshop chains + Accountant direction** — workshop crafting covers
  planks/blocks/tools, exterior catnip/grain/herb plots, logging, Mill grain→flour→food, Sawmill
  logs→lumber, fibre→thread→cloth, hide→leather, and ore→metal. A staffed Accounting Tent keeps the
  aggregate ledger from physical per-pile reports; vacant/unbuilt/unassigned accounting remains
  stale indefinitely with no authoritative background recount; tools give a bounded
  construction/crafting/quarrying/hauling bonus and repeated building costs escalate per type.
  Mill, Sawmill, Workshop, and Smelter workers now path stockpile→station→stockpile with persisted
  local input/output, real editable queues, and no aggregate output credit before delivery.
  The socket projection now makes those reports authoritative for player visibility: exact cached
  totals stay internal, uncounted piles project zero, duplicate threat equipment uses the books,
  and equality attestations are omitted. Blessings remain exact because they are not stockpile
  goods. Initial, tick, post-action, and reconnect JSON sentinels verify the boundary; future
  offer/block metadata must not recreate an exact-total oracle.
  Exterior farms likewise require physical plot work, bounded
  harvest baskets, local handoff, and final finite-storage delivery. Physical Fibre forage and
  Smithy's selected two-Metal→one-Weapon/Armor route follow the same delivery boundary.
  P19's canonical contract now fixes the target terminology and completion order. Raw Stone is
  defaulted without reinterpreting stable Materials saves; quarry Stone, rubble/Supplies, mountain
  Ore, hunt Hide, and foraged Fibre are carried cargo. Stone Prep, Woodworking, Tannery, Clothier,
  and Smithy are physical. P19.C3 now gives functional equipment one stable finite-item authority;
  compatibility scalars are derived projections rather than a second inventory. The complete
  generated recipe/resource graph is recorded in `docs/RECIPE_RESOURCE_MATRIX.md`.
- **P12.6 logistics** — general/limited stockpile designation, signed manual shrine orders, and
  Steward gather-spot automation landed. Population-relative tithe and carried-offering gates are
  reachable across five unattended seeds without consuming protected reserves. Tithes, rituals,
  and delivered offerings feed the one spendable blessing balance used by fertility and instant
  god purchases; spending lowers the fertility bonus, cat research points remain separate, reset
  preserves the remainder, and HUD/research snapshots agree without stockpile double-counting. The
  seeded finite general storehouse and complete physical Mill/Sawmill/Wood Cutter/Stone Prep/Woodworking/Workshop/Smelter/Tannery/Clothier/Smithy routes are
  verified. An appointed Steward creates ten provenance-distinct exact-resource piles for one of
  each processor within a separate sixteen-pile automation budget, then moves real conserved loads
  for input deficits before output surplus. Vacancy/removal leaves dormant physical contents;
  blocked recovery persists without overfill or fractional whole-gear cargo. Signed players can
  choose Food, Herbs, or Materials for the physical offering route; legacy `OfferMaterials`
  remains accepted as the Materials choice. The generalized signed playtest passes.

### P13 — Client UI for P12
- Spatial stockpile designation + render (`b3d28fb`).
- Seven-role appointment/vacate UI and a manual-orders sheet with basic farm/gather/road, staffing,
  resource, building, military, ritual, shrine, hauling, and research actions.
- Crop/timber HUD state, visible farm growth stages, and distinct roofless processing stations.
- A full-page 487-study ("about 500") ledger with dependencies, filter/search/pan/zoom. Every supported study permits
  research-point purchase and persistence; the living Leader selects at most one affordable node
  per rolling real-life day across the full
  catalog. Research Hut is explicitly available from founding; `milling` is the sole Mill
  placement unlock, generated Mill Foundations is durability only, Wood Cutter/Stone Prep/
  Woodworking are data-declared placement-available without Basic Tools, and one catalog-derived rule
  drives placement denial text. All 108 maintained station recipes have exact descriptor/catalog
  ownership and execute physically. Every one of the 487 studies is purchasable; no card remains
  disabled as `FUTURE`.
  Sawmill→Gather Logs is the sole validated research-gated job entitlement and drives signed plus
  Forester work; false founding/non-runtime job claims are removed. Physical manual/Forester
  replanting consumes each persisted stump/root stock into a visible sapling and restores the same
  deterministic logging tree after 24 unobstructed game-hours. Research labor/building automation and
  rituals remain Loremaster-owned; the daily strategic choice is Leader-owned.

### P14 — Spatial placement (spec: `docs/migration/specs/p14-spatial-placement.md`)
- **Implemented slice; generalized visual gate verified:** atomic player/leader validation and commit, exclusive future footprints,
  collision-free building/stockpile/gather/road reservations, exact shrine/gate/exterior
  connectivity, linked expansion persistence, paid-scaffold recovery, resolved 2×3 tree and
  1×1 rock occupancy across chunk boundaries, soft-obstacle path costs, and disjoint
  authored-stone/traffic-dirt roads. Focused occupancy tests pass; the combined canopy frame is in
  progress.
- **Physical-depth boundary:** scaffold costs are pinned at plan time and finite Lumber or Planks
  plus Blocks are carried from reserved visible sources into persisted scaffold-local input before
  progress. Authored stone roads now follow the same truth: one exact visible Material is reserved,
  carried, and worked by a living builder per ordered tile before paving and debit. Steward road
  automation queues that identical persisted job; death, restart, and placement races conserve it.
- **Staged expansion:** persisted exterior agricultural claims are excluded from wall derivation;
  a replacement perimeter is built segment by segment while the old closed enclosure remains
  authoritative, then all edges and the one south gate cut over atomically. Accepted native
  framebuffers show the complete old perimeter, one amber prospective face, retired shared edge,
  completed E/S/W outer faces, sole final south gate, and the same exterior 3×3 farm throughout.

### P15 — Playtest-feedback backlog (spec: `docs/migration/specs/p15-playtest-feedback.md`)
- Fog of war is verified: the exact 13×13 founding claim plus two-tile halo starts visible;
  ordinary walkers never reveal; signed resource/general scouts carry dim provisional knowledge
  and commit it only on physical shrine return. Death/cancellation drops notes, SQLite restarts
  preserve them, and the first wood scout has a deterministic three-live-minute bound.
- Scout targeting follows deterministic knowledge-blind wander legs with bounded alternate-heading
  retries. Targets remain absent until physically observed; missions change direction, give up, and
  return under survey/deadline/route exhaustion while preserving the shrine notebook contract.
- **Verified baseline:** the founding Leader now retains deficit-scaled Hunt/FetchWater/Scout allocation
  capped at six/two/one at 15 cats and scaled proportionally thereafter; vacancy cleanup preserves
  no more than those physical trips. Three personal seeds and the 30-cat communal village pass
  exact 48-hour one-second campaigns with fog growth and no research/ritual leakage. Optimized
  browser and signed fresh-native captures both confirm physical shrine return and permanent fog growth.
- **Verified labor pressure:** a read-only observed-state sample distinguishes useful assigned work,
  sourced processor/farm vacancies, true idle cats, personal needs, and intentionally manual offices.
  Three personal seeds fill all 15 founding paws through signed physical orders while a sixteenth
  sourced slot remains; passive personal/communal 48-hour proxy twins and guided 200-hour twins are
  deterministic and reset-free. The staged handoff observes every 7→0 office state, and the 52-action
  campaign covers production, research, roads, farms, scouts, Rail, and Shipping.
- **Failing baseline preserved:** a fresh personal village suffered eight deterministic
  unattended-collapse resets in 48 hours because the same strict filter removed primitive food
  work. This remains the accepted before-fix comparison for the verified repair.
- Cat booster: a per-cat priority flag that biases the leader's job/role matcher, plus an
  inspector toggle and on-map priority marker.
- Control rebind + building inspector (real inbound-haul readout); smooth cat/raider movement
  (persisted + interpolated, no more teleport-to-tile snapping); final control scheme +
  constant-speed walking.
- Automatic elections expose the authoritative resolved-term schedule between election windows;
  the governance panel shows the next boundary and countdown, while an open election still takes
  display precedence.

### P16 — Founding village blueprint (spec: `docs/migration/specs/p16-village-blueprint.md`)
- The fixed blueprint's older five-cat start is superseded. The active integration creates
  15 adult cats in three five-bed Dens, reserves a bed before an 18-game-hour pregnancy, opens
  prosperity migration after 30 game-hours, and gives an unhoused arrival 36 game-hours to gain
  a permanent bed before leaving. Migrants now begin at one persisted dry exterior origin, cross
  the authoritative south gate before the housing clock or resident simulation begins, and reuse
  that origin for a physical departure after releasing work and conserving cargo. Blocked routes
  wait and resume, gate relocation uses current wall topology, and SQLite restart preserves both
  in-flight directions. Extinction reset restores the whole state atomically.
- Ordinary old-age mortality is deliberately retuned from 48 to 240 game-hours, with the
  leader/healer threshold retuned from 57.6 to 288. Emergency water must be a physical
  source→carry→deposit job rather than a direct resource grant.
- The founding/housing slice is verified by all-seed long runs and determinism twins, signed
  server actions, persistence/restart, focused four-crate gates, independent review, and exact
  15/15 plus unhoused-probation framebuffers; see `docs/IMPLEMENTATION_AUDIT.md`.
- Gather spots (temporary drop points) + a gatherer/mover work split, with resource-typed
  markers rendered on the map.
- Farms and legacy fields stay beyond the permanent settlement core; logging ignores hidden
  interior trees, and linked field claims retain one-tile expansion without starving founders.
- Founding and ordinary expansion clear settlement deposits in authoritative state; the
  guaranteed water source sits outside the south wall. Expanded settlement and claimed
  agricultural territory are distinct persisted classes, and agricultural parcels stay exterior
  to both active and prospective walls.
- Tile recalibration (smaller render tile, footprint sizes tuned: house 2×3, workshop 3×3,
  shrine 3×3 with a road ring, tree 2×3). The settled authority is one integer simulation cell per
  16×16 source-art tile, scaled by the client camera/world transform rather than subdividing the
  pathfinding grid.

### P17 — Climate-driven biome generator (spec: `docs/migration/specs/p17-biome-generator.md`)
- Climate-driven biome generator (~26 biomes) at the sim layer, rendered client-side as ground
  tint + per-biome tree density.
- Per-biome crop fertility + mining rules; **ore/metal mining is wired** (mountain-biome ore,
  a `Smelter` building refining ore → metal bars, then a selected physical Smithy queue consuming
  two Metal for one whole Weapon or Armor).
- The physical fishing route is verified with persisted player-designated shore tiles,
  worker travel/work/return, finite fresh-Fish cargo transfers, Fishing/Haul skills, Farmer
  automation, signed controls, restart/death conservation, actual general-storehouse-footprint
  delivery with no village-anchor credit, and fixture-assisted guided/unattended campaigns
  (not a world-generation acceptance test). Each canonical water habitat starts with at most 24
  fish and replenishes deterministically by 0.5 per game-hour; only a successful on-site catch
  depletes it, and removing/repainting its designation cannot refill it.
  Fine-biome movement factors now drive both inverse-cost A* and physical elapsed-time travel,
  composed with dirt/stone roads and soft obstacles through a shared per-tick chunk cache.
  Focused generated-map/cadence tests, all 11 signed guided/manual campaign cases, and four
  live-cadence passive game-hours for seeds 7/42/20240712 as byte-identical twins verify the
  integration; fog grew 289→459/414/474 with no death or reset.
  Rail and Shipping preserve their stable capability IDs as blueprint entitlements, and ownership
  alone remains physically neutral. Signed exact-reservation projects now construct tracks,
  rolling stock, docks, and vessels; staffed finite-cargo routes board, load, travel, wait for
  storage, unload, and return with death/cancel/restart conservation.

### P18 — Visual polish (spec: `docs/migration/specs/p18-visual-polish.md`)
- The maintained Adventure art is live through Bevy sliced images: parchment, dark, and ornate
  panels; button interaction/disabled states; progress bars; resource medallions; the minimap
  ring; and pointer/interact/pressed/target/disabled custom cursors. The Adventure foundation has
  accepted native and optimized-WASM captures at exact 1024×768, 1280×800, and 1920×1080 sizes. A fresh personal
  browser village completed Explore and shrine return, growing permanent reveal from 289 to 394;
  the research ledger's clipped edge-pan state is also inspected at the supported width bounds.
- Persistent map-name plaques are removed. All 25 current protocol building variants have an
  explicit residential/open/infrastructure treatment. The prior 24 variants, Mill/Sawmill, and
  crop stages are framebuffer-verified; an integrated Accounting Tent retains all three
  founding Dens and renders as a separate open ledger/desk station. The staged wall/agricultural
  sequence is accepted at native resolution.
- The maintained compact presentation supersedes the exhaustive resource/button wall. The world
  HUD pins Food, Fish, Water, and Materials; Stores [G] derives all 32 resources from the protocol;
  and the category dock expands only the active Gather, Build, Territory, Scout, Village, or
  contextual controls. The 32 semantic resource paths/labels/tints are unique, including a
  byte-distinct Thread spool. Focused mapping and layout tests pass. Earlier decoded captures of
  the prior 31-resource exhaustive HUD remain dated evidence for the Adventure foundation, not
  proof of the redesigned layout. Generalized narrow/wide native interaction verification passes;
  the shared WASM target compiles cleanly.
- Visible hauling uses the same semantic authority: every one of the 32 `CarryingKind` values loads
  its exact tracked resource art, with no colored-square fallback. Fibre uses the tracked
  public-pack haystack silhouette, Thread the Generic Items spool, and Cloth its distinct finished
  textile glyph. Exhaustive uniqueness/file tests and the combined framebuffer gate pass.

### P19 — Item/material economy (spec: `docs/migration/specs/p19-items-materials-trade.md`)
- The spec's canonical production table is the resource/taxonomy authority. Logs are raw timber,
  Planks are fine boards, Lumber is structural, Stone is raw, Blocks are dressed, and the stable
  `materials`/`refined` IDs remain the generic Supplies/Crafted Supplies chain. P12 retains
  manual/officer logistics ownership and P16 retains its three founding benches; all existing
  building IDs and open-top station identities remain.
- Slice 1: item/material data model + per-colony item store; workshop crafting chains
  (planks/blocks/tools).
- Slice 1b: workshops go live — auto-staffing + wired resources + build cost.
- Production extension: logging + Sawmill (`logs→lumber`), crop plots + Mill
  (`grain→flour→food`), fibre→thread→cloth, hide→leather, and ore→metal, with lumber-first construction,
  protected useful tools, type-local escalating costs, and persistence.
- Slice 2: workshops craft material-variant trade goods.
- Slice 3/4: visiting traders + a coin economy + sell/buy actions; the client renders the
  visiting trader (merchant cat + minimap mark), a goods/inventory panel, item glyphs, and an
  always-visible HUD treasury total. The current correction gives every visit a bounded
  deterministic reachable exterior, ordinary A* travel through the retained gate to physical
  shrine contact, a finite manifest/purse/100 kg wagon, exact item-unit cargo, persisted
  phase/route/deadline/stock, sold-out truth, and a physical return to that same exterior.
- Finite-condition extension: stable item-unit IDs, physical weight, work-driven durability wear,
  persistent broken units, staffed material-backed repair with a live durability-research
  multiplier, a 20 kg trader item-load limit, signed/persisted controls, and truthful Goods-panel
  condition/repair visibility.
- Bone/Gem/Clay/Sand variants, climate-owned finite sources, exact functional equipment, all
  generated recipe/resource consumers, and reachable client controls are verified. Durability has
  a live repair consumer. The Accountant physically visits reachable piles and refreshes only the
  report it actually counted.

#### Completed canonical-production implementation cards

These cards preserve dated slice boundaries. Intermediate recipe/future counts are historical.

| Card | Status | Scope and acceptance |
| --- | --- | --- |
| P19.C1 — physical source taxonomy | done | Defaulted raw Stone preserves legacy Materials as distinct Supplies across saves, wire, storage, trade, HUD, and private Accountant reports. Quarry returns three Stone loads plus renewable rubble/Supplies; only persisted Mountains add an Ore load. Hunts return three Food loads followed by distinct Hide and Bone cargo; Bone is independently defaulted through save, wire, storage, stockpiles, trade, HUD, and Accountant projections. Aggregate credit waits for finite delivery, partial logging depletes on first extraction, and death/cancel/restart/full-storage cases conserve cargo. Five-seed unattended faucet/population campaigns, passive Bone hunts, a signed fresh-zero-Stone quarry, and the signed player road→farm→Mill replay verify both no-input and player-guided play. The 20-Supplies offering bar, completed-Field essential floor, and topology-signature construction-route cache are regression-covered. The final 1,169 sim + 43 protocol + 82 server + 134 client tests, strict four-crate Clippy, and accepted exact 1024×768 client-owned `/tmp/raw-stone-bone-final.png` showing counted Stone `~12/100` and Bone `~3/100` verify the slice; Bone item variants were open at this slice boundary and closed by later cards. |
| P19.C2 — remaining physical stations | done | C2.0 gives all six benches stable data-owned recipe descriptors, canonical resource sets, deterministic default queues, exact selected-recipe catalog/block metadata, rules-v0 grandfathering, and generic signed/persisted queue state. Across all maintained stations there are eleven runtime recipes: eight research-gated and three founding baselines. C2.1–C2.5 complete Wood Cutter, Stone Prep, Woodworking, Tannery, and Clothier. C2.6 completes the tenth physical processor: one Metalwork worker carries two Metal into Smithy, advances one selected 900-second `smithy_weapon` or `smithy_armor` batch, and carries one whole scalar Weapon or Armor to finite storage before aggregate credit. Rules-v6 is version-only; old aggregate forge timers remain bit-frozen; scalar gear does not mint a duplicate finite item. Deterministic 1s/5s/60s routes, whole-unit headroom, research/skill/comfort gates, death/removal/replacement, signed guided Ore→Smelter→Metal→Smithy→Weapon provenance, authentic HMAC SQLite restart, and passive 60s/5m Captain twins pass. The accepted exact 1024×768 RGB client-owned `/tmp/physical-smithy-c2.png` (SHA-256 `833082b06e6b95172bc1afe1e22a4d3e2e34787381538cce629f675466226429`) visibly proves its open sprite, assigned hauler, Metal input, whole Weapon output, progress, block reason, repeat queue, and controls. Earlier accepted Tannery and Clothier framebuffers remain recorded above. Final gates pass 1,233 simulation (one intentional skip), 44 protocol, 93 server, and 136 client tests plus strict four-crate Clippy, formatting, and diff checks. P19.C3 subsequently replaced the scalar-output boundary with one finite identity authority. |
| P19.C3 — finite functional equipment | done | Stable finite Tool/Weapon/Armor IDs are the identity, location, condition, job/combat wear, repair, and exact-sale authority. Woodworking/Smithy output travels local→carrier→stockpile before derived scalar credit; signed equip/unequip, Captain issue, capacity-safe death/departure/reset spill recovery, rules-v1/SQLite migration, Accountant-confidential wire projection, and nonduplicating Goods/loadout UI are verified. Passive 60s/5m Captain runs, signed guided actions, and one complete exact-ID craft→restart→equip→break→unequip→repair→sale campaign pass. The accepted exact 1024×768 client-owned `/tmp/finite-equipment-1024.png` (SHA-256 `cc18adfdab2d00b43fccaf95b44784a6142c47f043ec9eb6e26aea1f19c1ff9d`) visibly proves distinct locations, exact conditions, repair/unequip, and nonoverlapping responsive panels; capture staging was removed. |
| P19.C4 — physical scaffold inputs | done | Exact player and autonomous placement preserve pinned type-local cost, escalation, atomic footprint validation, and paid-scaffold recovery while reserving finite Lumber/Planks plus Blocks from visible sources. A living assigned builder carries bounded loads through persisted transit/input ledgers and progress waits for full delivery. Conservation, empty-paw/loaded blocked-route reopen, recovery, pinned speed, reservation safety, cadence, restart, signed HMAC, protocol/client, full touched-crate gates, strict Clippy, and the accepted 2048×1152 selected-scaffold own-framebuffer are verified. |
| P19.C5 — physical finite visiting trader | done | The merchant follows ordinary obstacle-aware A* exterior→gate→shrine→same-exterior travel, trades only after physical arrival, and owns finite deterministic resource stock, purse, wagon capacity, and exact item-unit cargo. Depletion, sold-out denial, blocked-route reopen observation, expansion rehoming, persistence/restart, signed guided actions, exact one-second/minute/hour/coarse transition times, and a passive 60h full visit pass are verified. The fixed-height panel exposes every craft offer through six-row pagination and uses only Accountant reports for storage guidance. The accepted client-owned 1024×768 logical framebuffer `/tmp/trader-physical-1024.png` shows the merchant at the shrine, page 2/2, finite quantities, and Food sold out without clipping or private exact-headroom leakage; 1,153 simulation and 80 server tests plus strict touched-crate Clippy pass. |
| P19.C6 — sourced recipe/resource breadth | done (first sourced slice) | The evidence boundary in `docs/RECIPE_RESOURCE_MATRIX.md` maps every catalog promise to a source, station, output, and entitlement. Mill grinding and baking are separate physical selected recipes (`grain_to_flour`, `flour_to_food`), and Smithy adds `smithy_tool`, producing one exact metal Tool from two Metal. The catalog exposes unsupported promises as disabled FUTURE content; tests pin 13 live runtime recipes and exactly 91 recipe plus 64 resource future payloads. Rules-v7 migrates old combined Mill queues without losing authored state. Signed farm→Mill and Ore→Smelter→Smithy→metal-Tool campaigns, passive one-/five-minute Captain twins, HMAC/SQLite restart, protocol/UI tests, and client-owned research-ledger framebuffer verify the slice. Further breadth was open at this slice boundary and was closed only after its physical source/station contracts landed. |
| P19.C7 — fine-biome raw source ecology | done (source boundary) | Mountains own finite Gem, wet/badlands biomes finite Clay, and beach/desert finite Sand. Exact physical quarry cargo drains persisted deposits on pickup, exhausts special-only sites, survives save/wire/storage/trade/HUD projection, and is erased from village interiors. Generic Quarry protects the founding Stone chain. Ten selected downstream variants are live under P19.C9; broader combinations were a later slice and are now closed. |
| P19.C8 — physical village caravans | done | Accepted village barter first computes one bounded deterministic shared-terrain land route through both current gates; water, mountains, and closed wall edges block it, and no-route rejection leaves the open offer plus both cargoes untouched. It then debits deterministic source piles or exact Tool/Weapon/Armor instances into persisted two-sided escrow and creates a visible actor that follows the exact outbound/reverse waypoints before atomic credit. Finite units retain material, quality, condition, and credit state while an injective origin-qualified identity prevents colony-local serial collisions. Detours, long-distance bounds, exact route restart, full-storage waits, signed cancellation, conservation/cadence/no-fog-leak, protocol/client position/route/manifest, and atomic failure are covered. |
| P19.C9 — physical material-variant goods | done | Ten research-owned recipes turn finite Bone into Tool/Trinket/Toy, Gem into jewelry, Clay into Mug/Bowl/Brick, and Sand into three glassy goods. Exact raw input travels to Woodworking, Stone Prep, or Workshop; staffed work creates one stable finite identity in local output; that same material/quality/condition-bearing ID travels to storage and remains visible to persistence and traders. Research stays non-purchasable until its descriptor exists. Exhaustive mapping, passive deterministic and signed guided campaigns, carrier-death conservation, SQLite mid-route restart, trader sale, reachable client recipe selection, full gates, and an inspected own-framebuffer verify the slice. The future boundary is now 81 generated recipes and 64 resources. |
| P19.C10 — food and plant research breadth | done | Twenty-three finite recipes activate the remaining Grain Milling stages and complete Baking, Herbalism, Food Preservation, and Brewing. Preserves, Medicine, and Brew have exact pile/cargo/storage/trade identities and bounded survival consumers. Seventeen generated resource stages change exact crop/batch yield, spoilage, family cycle time, or finite capacity. Exhaustive catalog/route/effect, passive deterministic, signed queue, SQLite audit, protocol/client, full gates, and inspected framebuffer evidence verify the slice. The future boundary is now 58 generated recipes and 47 resources. |
| P19.C11 — subsistence/frontier research breadth | done | Hunting, Foraging, Waterworks, Animal Husbandry, Field Craft, and Expedition Supplies activate all fifty catalog nodes through thirty exact finite item recipes (28 new routes plus the two retained Hunting goods) and twenty exact input/durability/cycle/capacity consumers. Every mapping conserves input and one stable output identity through station→carrier→storage; signed queue actions cover every recipe, default automation remains unchanged, and descriptions explicitly limit current use to equipment/trade rather than hidden farm/scout behavior. The future boundary is now 30 generated recipes and 27 resources. |
| P19.C12 — industrial material research breadth | done | Thirty finite recipes complete Textile Work, Leatherworking, Carpentry, Stonecraft, Metallurgy, Toolmaking, Weaponcraft, Armorcraft, and Trade Goods. Scalar batches follow the existing finite station route; twelve researched metal equipment recipes mint quality-mapped exact IDs with no shadow scalar authority. Twenty-seven generated resource stages change only exact family input/yield, physical capacity, or cycle time. Exhaustive catalog/descriptor/effect, exact-ID lifecycle, signed queue, SQLite restart, default-queue compatibility, and strict gates verify the slice. Combined with C11, the future recipe/resource boundary is now zero/zero. |

#### Physical-consistency enhancement outside P19 completion

| Card | Status | Scope and acceptance |
| --- | --- | --- |
| P16.R1 — physical authored-road labor | done | Signed and Steward routes preserve mapped-terrain, shrine-network, surface, and speed rules while reserving exact visible Materials. A living Build worker carries and works one unit per ordered tile; partial progress, death/reassignment, source/spill recovery, tool wear, map reservation, cadence, and SQLite restart are verified by passive and player-guided campaigns. |
| P12.R2 — truthful Crews worker slots and services | done | Twelve existing labor domains gain a deterministic second worker station from their exact `*_crews` study. Each station owns worker/provenance/queue/pause/progress, uses finite shared station stores, survives death/reassignment/restart, and is individually controlled/inspected. The thirteen passive/unsafe families keep their stable Crews IDs and graph positions but now grant completed-building-gated service effects instead of fake slots; Mouse Farm alone gains one base keeper whose Food remains station-local until physically hauled. Exhaustive catalog, consumer behavior, signed action, persistence, death/full-storage, determinism, and conservation tests verify the complete Building branch. |
| P16.R2 — shared mutable spatial authority | done | `WorldState` owns canonical mutable tiles and Fish ecology; colony maps are compatibility/view caches hydrated and published at signed-action/tick boundaries. Overlapping roads, wear, source depletion/regrowth, and habitat stock agree immediately; ecology ages once per coordinate, while fog/contact remain private and terrain overlay snapshots require committed reveal. Deterministic legacy merge, transactional SQLite whole-world persistence, restart replay, and signed overlap coverage are verified. |
| P12.R3 — one-third building research | done | The eleven stable `construction_*` studies are data-classified as Building research because their existing positive `constructionSpeed` payloads already drive authoritative physical-scaffold timing. The 487-node graph is 165 Building / 167 RecipeResource / 155 Upgrade without restoring inert stores, changing IDs or dependencies, or reducing recipe/resource breadth. All 165 Building studies are purchasable and have an authoritative consumer; sim and client guardrails require both product categories to remain at least one third. |

### Also shipped alongside P12–P19 (not tagged to a phase in commit subjects)
- **Multi-village founding and contact**: one larger durable communal global village (30 adults,
  six Dens, 19×19 core, doubled production/runway, civic buildings); exact deterministic distant
  owner-only personal sites; restart-persistent secure socket routing; explicit returned-scout
  discovery provenance; summary-only foreign contact; configurable signed direct barter capped at
  32 open source offers; transactional whole-world persistence; and storage-scoped child ids for
  simultaneous villages. Accepted offers now become persisted visible shrine-to-shrine caravans
  with exact pile or identity-bearing equipment escrow. Personal villages remain exact
  15-cat/three-Den foundings. Villages share one authoritative mutable terrain/ecology ledger:
  roads, wear, depletion, regrowth, and Fish populations propagate at overlapping coordinates
  while fog and contact stay private. Accepted barter now derives its durable waypoints from that
  shared obstacle authority without revealing terrain; deeper encounter behavior is optional
  future breadth.
- **Top-down building interiors**: cutaway (no-roof) interiors, then a second slice adding
  textured floors + furnace/altar props (`546d852`, `4b6a375`).
- **Life-sim breeding wired into the tick loop** — population is a loop, not a fixed roster
  (`b55637c`); old-age death made consistent with survival death (`2bba148`); a long-horizon
  "founding population boom-bust" fix admitted `Young` cats to the fertile pool (`b84c2a5`).
  That older pacing is historical implementation evidence: the maintained P16 lifecycle now
  requires reserved-bed slow pregnancy, migration-led early growth, and the longer 240/288-hour
  old-age thresholds described above.
- **Census**: a colony census panel (live demographics) and inspector surfacing cat pregnancy
  ("expecting") (`fe9161a`, `410bb70`).
- **Upgrade-tree UI**: read-only browse of the whole tech tree by era, plus a god-purchase
  button on affordable nodes (`f8f9822`, `678a289`).
- Two founding-economy death-spiral fixes: the leader never fetching water due to a
  veto/finder mismatch (`addb9a7`), and the same bug's twin in quarry assignment (`bde92d5`).
