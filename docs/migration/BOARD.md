# Migration Task Board — Web → Bevy client + Rust sim

Source of truth for the codex-orchestrated rebuild. The orchestrator (Claude) keeps
this in sync with the in-session task tracker. Plan:
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
| P12 | Sim expansion: skills, officers, spatial stockpiles, workshop chains | in progress — seven strict manual/officer domains, role-station/unlock gates, active shrine faucets, useful tools, escalating costs, Accounting Tent, farming/logging/processing chains, and spatial container actions are verified; the shrine is still the all-resource fallback, and physical local workshop logistics, all-labor skills, and broader recipes remain |
| P13 | Client UI for P12: stockpile designation, officer assignment | in progress — designation/assignment, signed basic manual orders, the full-page 500-study ledger, legacy research-point purchase, crop/timber HUD state, visible farm stages, and distinct Mill/Sawmill stations shipped; coordinate building placement, selectable farm/gather variants, removal/election/queue tools, and live generated-study purchases remain |
| P14 | Spatial placement: footprints, tile occupancy, soft obstacles, road accessibility | in progress — atomic action validation, reservations, connectivity, linked expansion, scaffold recovery, rendered 2×3 tree/1×1 rock occupancy, and exact dirt/stone road surfaces are verified; wall expansion is not staged outer-before-inner |
| P15 | Playtest-feedback backlog: controls/feel, fog-of-war, booster, movement smoothing | in progress — movement/booster, visible roads, and resource/general shrine-return scouting are verified, including restart-safe in-flight notebooks and responsive controls; scout routing still uses nearest-hidden-target oracle selection instead of the specified random walk, and exact placement/election plus station-local inspector/queue paths remain partial |
| P16 | Founding village blueprint, gather spots, tile recalibration | in progress — the 15-adult/three-five-bed-Den lifecycle, reserved-bed pregnancy, migration that leaves the last real family vacancy, prosperity migration/36-hour probation, aging, deterministic reset, physical emergency water, authoritative interior clearing, exterior water, exact roads, and manual/Steward gather-spot movement are verified; selectable gather variants/removal and the outside-wall agricultural territory model remain |
| P17 | Climate-driven biome generator (~26 biomes), mining, crop fertility, transport upgrades | in progress — climate generation, crop fertility, ore/metal extraction, and exterior plots are live; fine-biome movement is unused, rail/shipping are global multipliers rather than built routes/vehicles, and fishing is absent |
| P18 | Visual polish: DF-Steam parchment UI, craft-station sprites | in progress — persistent map plaques are gone; all 25 current protocol variants have tested residential/open-station compositions, with the prior 24 plus Mill/Sawmill/crop stages framebuffer-verified; Accounting Tent is snapshot-reachable but still needs an integrated in-world capture. The Adventure skin is exact-size native-framebuffer verified, the release WASM bundle builds, and WASM visual interaction remains |
| P19 | Item/material economy: crafting chains, traders, coin | in progress — planks/blocks/tools, grain/flour/food, logs/lumber, fibre/cloth, hide/leather, ore/metal, protected useful tools, material trade goods, visiting traders, and coin are live; recipe/material breadth, physical local inventories, fishing, and complete controls remain |

**Notes on P12–P19**: these phases were decomposed and executed after this board's card
format fell out of active use for day-to-day tracking — the per-slice specs live in
`docs/migration/specs/p12-idle-cat-forest.md` through `p19-items-materials-trade.md`, and
the git log records what landed (commit subjects are tagged with phase/slice where applicable).
This table is a summary rollup, not a card-by-card log; see “P12–P19 — what actually landed”
below for the historical feature list and
`docs/IMPLEMENTATION_AUDIT.md` for the evidence-backed status and remaining acceptance matrix.

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
- **P12.1 skills** — proficiency/XP persists and is exposed for the four legacy labors
  (Hunter, Architect, Ritualist, Warrior). Mill, Farm, Research, and other maintained labors do
  not yet have complete gain/effect/UI paths.
- **P12.2 officers** — Steward, Accountant, Forester, Farmer, Captain, Loremaster, and Cloth
  Leader have strict automation ownership. Vacancies are manual-only; appointment requires the
  matching researched unlock and completed role station; assignment, replacement, automation
  provenance, and the rolling daily legacy-Loremaster timestamp persist.
- **P12.3 spatial stockpiles** — designated containers are places in the world, but the shrine
  remains the all-resource fallback reservoir; the specified seeded general storehouse/local
  capacity model and physical hauling contract are not complete.
- **P12.4a/b workshop chains + Accountant direction** — workshop crafting covers
  planks/blocks/tools, exterior catnip/grain/herb plots, logging, Mill grain→flour→food, Sawmill
  logs→lumber, fibre/hide→cloth/leather, and ore→metal. A staffed Accounting Tent keeps the
  aggregate ledger exact; tools give a bounded construction/crafting/quarrying/hauling bonus and
  repeated building costs escalate per type. Workers still do not path to their station and
  inputs/outputs use colony-global resources; station-local ledgers/queues remain absent.
- **P12.6 logistics** — general/limited stockpile designation, signed manual shrine orders, and
  Steward gather-spot automation landed. Population-relative tithe and carried-offering gates are
  reachable across five unattended seeds without consuming protected reserves. The seeded general
  storehouse and local physical logistics promised by the spec remain open.

### P13 — Client UI for P12
- Spatial stockpile designation + render (`b3d28fb`).
- Seven-role appointment/vacate UI and a manual-orders sheet with basic farm/gather/road, staffing,
  resource, building, military, ritual, shrine, hauling, and research actions.
- Crop/timber HUD state, visible farm growth stages, and distinct roofless Mill/Sawmill stations.
- A full-page 500-study ledger with dependencies, filter/search/pan/zoom. The original 24 nodes
  support research-point purchases and daily Loremaster automation; the other 476 studies are
  visibly read-only pending runtime purchase/effect/persistence integration.

### P14 — Spatial placement (spec: `docs/migration/specs/p14-spatial-placement.md`)
- **Verified slice:** atomic player/leader validation and commit, exclusive future footprints,
  collision-free building/stockpile/gather/road reservations, exact shrine/gate/exterior
  connectivity, linked expansion persistence, paid-scaffold recovery, rendered 2×3 tree and
  1×1 rock occupancy, soft-obstacle path costs, and disjoint authored-stone/traffic-dirt roads.
- **Still partial:** perimeter expansion replaces the derived wall immediately instead of
  building the outer wall before removing the inner wall.

### P15 — Playtest-feedback backlog (spec: `docs/migration/specs/p15-playtest-feedback.md`)
- Fog of war is verified: the exact 13×13 founding claim plus two-tile halo starts visible;
  ordinary walkers never reveal; signed resource/general scouts carry dim provisional knowledge
  and commit it only on physical shrine return. Death/cancellation drops notes, SQLite restarts
  preserve them, and the first wood scout has a deterministic three-live-minute bound.
- Scout targeting currently materializes a bounded hidden search area and chooses the nearest
  useful target; it does not implement this spec's deterministic random-walk search and remains an
  explicit design/implementation decision.
- Cat booster: a per-cat priority flag that biases the leader's job/role matcher, plus an
  inspector toggle and on-map priority marker.
- Control rebind + building inspector (real inbound-haul readout); smooth cat/raider movement
  (persisted + interpolated, no more teleport-to-tile snapping); final control scheme +
  constant-speed walking.

### P16 — Founding village blueprint (spec: `docs/migration/specs/p16-village-blueprint.md`)
- The fixed blueprint's older five-cat start is superseded. The active integration creates
  15 adult cats in three five-bed Dens, reserves a bed before an 18-game-hour pregnancy, opens
  prosperity migration after 30 game-hours, and gives an unhoused arrival 36 game-hours to gain
  a permanent bed before leaving. Extinction reset must restore that whole state atomically.
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
- Founding and ordinary expansion clear claimed natural deposits in authoritative state; the
  guaranteed water source sits outside the south wall. An explicit distinction between expanded
  walled settlement and claimed agricultural territory is still required.
- Tile recalibration (smaller render tile, footprint sizes tuned: house 2×3, workshop 3×3,
  shrine 3×3 with a road ring, tree 2×3).

### P17 — Climate-driven biome generator (spec: `docs/migration/specs/p17-biome-generator.md`)
- Climate-driven biome generator (~26 biomes) at the sim layer, rendered client-side as ground
  tint + per-biome tree density.
- Per-biome crop fertility + mining rules; **ore/metal mining is wired** (mountain-biome ore,
  a `Smelter` building refining ore → metal bars, metal-bars-for-better-gear via
  `smithy::advance_metal_forge`).
- Transport upgrade flags exist, but fine-biome movement factors are unused, rail is a
  distance-triggered global multiplier without tracks/trains, shipping makes water slow-walkable
  without routes/vessels, and maintained fishing paths are absent.

### P18 — Visual polish (spec: `docs/migration/specs/p18-visual-polish.md`)
- The maintained Adventure art is live through Bevy sliced images: parchment, dark, and ornate
  panels; button interaction/disabled states; progress bars; resource medallions; the minimap
  ring; and pointer/interact/pressed/target/disabled custom cursors. Native own-framebuffer
  captures are verified at exact 1024×768, 1280×800, and 1920×1080 sizes, and the release WASM
  bundle builds; WASM visual interaction remains in the final campaign.
- Persistent map-name plaques are removed. All 25 current protocol building variants have an
  explicit residential/open/infrastructure treatment. The prior 24 variants, Mill/Sawmill, and
  crop stages are framebuffer-verified; Accounting Tent is snapshot-reachable with a tested open
  layout, while its integrated in-world capture remains.

### P19 — Item/material economy (spec: `docs/migration/specs/p19-items-materials-trade.md`)
- Slice 1: item/material data model + per-colony item store; workshop crafting chains
  (planks/blocks/tools).
- Slice 1b: workshops go live — auto-staffing + wired resources + build cost.
- Production extension: logging + Sawmill (`logs→lumber`), crop plots + Mill
  (`grain→flour→food`), fibre/hide→cloth/leather, and ore→metal, with lumber-first construction,
  protected useful tools, type-local escalating costs, and persistence.
- Slice 2: workshops craft material-variant trade goods.
- Slice 3/4: visiting traders + a coin economy + sell/buy actions; the client renders the
  visiting trader (merchant cat + minimap mark), a goods/inventory panel, item glyphs, and an
  always-visible HUD treasury total.
- Remaining breadth includes bone/gem/clay/metal item variants, finished functional
  tool/weapon/armor chains, physical local inventories, fishing, and reachable exact client
  controls for every chain.

### Also shipped alongside P12–P19 (not tagged to a phase in commit subjects)
- **Multi-village founding and contact**: one communal global village; deterministic distant
  owner-only personal sites; restart-persistent secure socket routing; explicit returned-scout
  discovery provenance; summary-only foreign contact; configurable signed direct barter capped at
  32 open source offers; transactional whole-world persistence; and storage-scoped child ids for
  simultaneous villages. The global village still uses the personal 15-cat founding blueprint,
  each colony owns duplicated mutable terrain, and meeting/trade remain summary/scalar operations
  rather than physical shared-map encounters or caravans.
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
