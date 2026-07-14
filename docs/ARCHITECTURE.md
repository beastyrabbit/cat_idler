# Architecture — Idle Cat Forest (Rust / Bevy)

This is the current architecture of the game as a Rust Cargo workspace. It supersedes
`docs/plan.md` (the TypeScript-era design doc, kept only as porting history — see the
superseded notice at its top). For *design/gameplay* intent (why the systems exist, where
they're headed) see `docs/GAME_VISION.md`; for the detailed per-system technical specs and
port status see `docs/migration/specs/` and `docs/migration/BOARD.md`; for hard-won
build/tooling lessons see `docs/HANDOFF.md`. This doc stays at the "how the pieces fit
together" altitude.

## The workspace

```
Cargo workspace (edition 2024, resolver "3")
├── crates/cat-sim       pure deterministic simulation core
├── crates/cat-protocol  serde wire types shared by server + client
├── crates/cat-server    tokio + axum authoritative server (WS + SQLite)
├── crates/cat-client    Bevy 0.19 renderer/UI (lib, native + wasm)
├── crates/cat-desktop   thin native binary over cat-client
├── crates/cat-web       thin wasm binary over cat-client
└── crates/cat-dev       `cargo dev` — local launcher, builds/runs server+desktop together
```

Data flow, end to end:

```
cat-sim::world_tick()  ──every colony, every 1s──▶  cat-server (owns the only WorldState)
        ▲                                                    │
        │ apply_action()                                     │ build_snapshot()
        │                                                     ▼
cat-server  ◀── ClientAction (JSON over WS) ──  cat-client  ◀── WorldSnapshot (JSON over WS)
                                                     │
                                                     ▼
                                          Bevy ECS: sprites, HUD, input
```

The server is the single source of truth. Clients hold no authoritative state — every frame
they render whatever `WorldSnapshot` they last received, and every player action is a
`ClientAction` sent to the server and only reflected once the server's next tick includes its
effect. There is no client-side prediction.

## cat-sim — the simulation core

`crates/cat-sim` is a `#![forbid(unsafe_code)]`, dependency-light crate (only `cat-protocol`,
`serde`, `serde_json`, and `ryu-js` for archived-behavior float formatting) with **no I/O**: no
filesystem, no networking, no clock, no threads, no `rand`. Every module is unit-tested in
place; `cargo test -p cat-sim` runs ~650 tests in well under 20 seconds because none of it
touches the outside world.

### Determinism

All randomness is a ported seeded LCG (`rng.rs`): `seed × 1_664_525 + 1_013_904_223 mod 2^32`,
matching the original TypeScript `seededRng.ts` bit for bit. Independent subsystems don't share
one RNG stream — they fork the seed by a fixed offset so that, say, adding a new movement roll
doesn't perturb the sequence of life-sim rolls in the same tick:

- movement: `seed.wrapping_add(1_000_003)`
- life sim (breeding/aging/mortality): `seed.wrapping_add(2_000_003)`
- raids: `seed.wrapping_add(3_000_003)`

This is what makes golden-master testing possible: seed → N ticks → deterministic snapshot,
comparable against fixtures generated from the original TS implementation
(`docs/migration/fixtures/`). The bar (per `AGENTS.md`) is *behavioral* parity — "same idea" —
not byte-identical output in the handful of spots the original TS used raw, unseeded
`Math.random()`.

### World shape

```rust
pub struct WorldState {
    pub colonies: Vec<ColonyRuntime>,
    // + world-level fields: world_seed, tick counters, etc.
}
```

The world is **multi-colony** from the ground up. The original TS game had exactly one global
colony; here that's just `colonies[0]` (`colony-1`), and `found_colony(...)` is a first-class
primitive for player-founded villages elsewhere on the same map. `ColonyRuntime` holds
everything that was previously spread across the `colonies`/`cats`/`jobs`/`buildings`/etc.
Drizzle tables: resources, cats, jobs, buildings, upgrade-tree state, threat/raid state,
elections, zones, claimed tiles/fence/gate, officers, stockpiles, gather spots, and the item
store.

### The tick

`world_tick(&mut WorldState, now_ms) -> Vec<TickReport>` is the single entry point, called once
per colony per second by `cat-server`. It runs roughly **40 ordered phases** (`fn phase_*` in
`crates/cat-sim/src/world_tick.rs`) — life sim → consumption/spoilage → elections/zones → path
decay/regrowth → job promotion → leader plan/direct/assign → production/research → survival →
due-job completion → hauling → movement → roads → raids → status/persist-prep — mirroring the
original `server/game.ts:workerTick`'s phase ordering exactly (per `AGENTS.md` rule #1:
"parity, not reinvention"). **Do not add a second tick path** — every simulated effect goes
through this one function, same discipline the TS game enforced for `workerTick`.

### Module map (by concern)

| Concern | Modules |
| --- | --- |
| Foundation | `rng`, `types`, `entities`, `cost_constants`, `needs_constants`, `test_acceleration` |
| World generation | `noise`, `terrain_gen`, `world_gen`, `biomes`, `climate` |
| Cat AI | `pathfinding`, `movement`, `policy`, `tasks`, `cat_ai`, `leader_ai`, `leader_director`, `officers` |
| Life sim | `needs`, `age`, `breeding`, `genetics`, `life_sim`, `survival` |
| Economy | `idle_engine`, `idle_rules`, `production`, `smithy`, `storage`, `shrine`, `trips`, `depletion`, `spoilage`, `housing`, `roads`, `village_layout`, `village_area`, `stockpiles`, `skills`, `ledger` |
| Military & governance | `threat`, `warriors`, `combat`, `elections`, `zones`, `upgrade_tree` |
| Item economy (P19) | `items`, `recipes`, `trader` |
| Orchestration | `world_tick` (the tick loop), `actions` (pure `apply_action` + `build_snapshot`) |

Each module's doc comment cites the original TypeScript file it was ported from (e.g.
`leader_director.rs` ← `lib/game/leaderDirector.ts`), per `AGENTS.md`'s commenting convention —
that's the fastest way to find the ported behavior's original spec and tests.

### Known product gaps

The core migration is complete, including ore/metal extraction and staffed Research Hut and
School buildings. Verified post-cutover slices include atomic placement/reservations, exterior
farm/logging production with Mill/Sawmill, label-free open stations, selected-village routing,
the complete purchasable and persistent 500-study runtime/client ledger, and a responsive blocking-pool server tick. The remaining work is
maintained product behavior rather than migration:

- **Officer/manual split.** The seven maintained offices now own distinct automation categories;
  vacant offices are manual-only, and appointment requires both the researched unlock and a
  completed role station. Signed client paths cover basic farm/gather/road designation, staffing,
  military, ritual, shrine, and production orders. Exact coordinate building placement,
  selectable farm/gather variants, election/vote-kick controls, and designation removal are live;
  exact per-cat labor and editable station production queues remain open.
- **Physical work and production breadth.** Workshop staffing is an id assignment rather than
  station travel; inputs/outputs still draw on colony-global resources, the shrine remains the
  fallback reservoir instead of the specified general storehouse, skills lack complete
  mill/farm/research gain-and-effect paths, and many recipes/material variants are absent. Tools
  now improve construction, crafting, quarrying, and hauling, and repeated building costs escalate
  per type, but neither closes the local-logistics gap.
- **Multi-village product model.** One canonical communal village and one personal village per
  stable signed identity are live. Ownership and selection persist; foreign private state stays
  server-filtered; explicit returned-scout delivery (never generic reveal state) creates mutual
  summary contact; configurable signed propose/accept/cancel actions perform capped atomic direct
  barter. Whole-world SQLite replacement is transactional, and colony-local runtime ids receive a
  storage-only colony namespace so simultaneous settlements cannot collide in legacy global-key
  tables. The communal village is not yet mechanically larger than a personal founding, however;
  each colony owns a duplicate mutable terrain map, meeting is a delivered summary rather than a
  physical encounter, and direct barter swaps scalar resources without caravans or item stacks.
- **Durable native identity.** The native client keeps its HMAC bearer and selected village in a
  mode-0600 file replaced through a synced same-directory temporary file and atomic rename; WASM
  uses the corresponding local-storage record.
- **Research and scouting depth.** Players can spend research points on all 500 studies, and a
  Loremaster may complete at most one affordable full-catalog node per rolling real-life day.
  Typed modeled effects and future-content unlock registries persist. Resource scouts preserve the
  shrine-return knowledge contract but currently select the nearest useful hidden terrain target
  rather than performing the deterministic random-walk search described by the P15 feedback spec.
- **Spatial, transport, and visual completeness.** Exact tree/rock occupancy and visible road
  surfaces are verified. Staged wall growth, a separate claimed agricultural-territory model,
  real rail/ship/fishing routes, and remaining menu/WASM skin states remain incomplete. Accounting
  Tent is snapshot-reachable and has an explicit open-station client composition; the maintained
  Adventure panel, button, progress, minimap, and cursor foundation is native-framebuffer verified.

The evidence and completion tests for each gap live in `docs/IMPLEMENTATION_AUDIT.md`.

### Maintained founding and life pacing

An ordinary village founding is a fixed simulation invariant, not client decoration: 15 adult
cat entities occupy three complete Dens with five beds apiece. Pregnancy reserves a permanent
bed before conception and gestates for 18 game-hours. After a 30-game-hour establishment window,
a prosperous settlement can receive deterministic migrant cohorts; unhoused arrivals participate
in the real economy during a 36-game-hour probation and leave if no bed opens. An extinction
reset reconstructs the whole founding state atomically and uses run-scoped identities so old
migrant or job records cannot leak into the new run.

Old-age pacing intentionally diverges from the archived TypeScript prototype: ordinary mortality
begins at 240 game-hours and leader/healer mortality at 288, rather than 48 and 57.6. Emergency
water is likewise an ordinary physical job: a selected cat travels to water, carries the yield,
and deposits it. No crisis phase may add free water directly to colony resources.

## cat-protocol — the wire contract

`crates/cat-protocol` has one dependency (`serde`) and defines the JSON shape both sides speak
over the WebSocket:

- **`WorldSnapshot`** — top-level, holds a `Vec<ColonySnapshot>` plus world-level fields.
- **`ColonySnapshot`** — one colony's full renderable state: resources + storage caps, cats,
  jobs, buildings, upgrade-tree progress, research, election/vote-kick state, zones, threat +
  raiders, claimed tiles/fence/gate, village radius/anchor, officers, stockpiles, gather spots,
  item store, road tiles, online count.
- **`ClientAction`** — a typed server contract: `Presence` (handshake),
  `RequestJob`, `Boost`, `PurchaseUpgrade`, `CastVote`,
  `RequestVoteKick`, `CreateZone`/`RemoveZone`, `PlanBuilding`, `UnlockNode`, `AssignWorker`,
  `TrainWarrior`, `DefendRaid`, `BuildRoad`, `FoundVillage`/`JoinVillage`,
  `AssignOfficer`/`UnassignOfficer`, `DesignateFarm`/`ClearFarm`,
  `DesignateStockpile`/`RemoveStockpile`,
  `DesignateGatherSpot`/`RemoveGatherSpot`, `SellGoods`, `BuyResource`, `BoostCat`, plus test
  controls (`SetTestAcceleration`, `AdvanceTime`, `SetTestRngSeed`).

Field names are `camelCase` on the wire (matching the old TS API shape where it still matters)
via `#[serde(rename_all = "camelCase")]`-style annotations; most actions carry `session_id` /
`nickname` / `sig` for HMAC-verified identity.

## cat-server — the authoritative server

`crates/cat-server` (tokio + axum) is a single binary:

- `GET /health` → `"ok"` liveness probe; `GET /ready` checks persisted-state readiness.
- `GET /ws` → WebSocket upgrade. Each connection is a task that reads `ClientAction` JSON
  frames, calls `apply_action` against the shared `Arc<Mutex<WorldState>>`, and forwards the
  broadcast `WorldSnapshot` stream.
- A `tokio::spawn`ed loop schedules `world_tick` **once per second** for the whole world (not
  currently configurable — the interval is `Duration::from_secs(1)` in `main.rs`). CPU-heavy
  simulation, snapshot construction, and synchronous SQLite work run on Tokio's blocking pool.
  A startup-initialized last-completed snapshot lets new sockets connect without waiting for an
  in-progress world lock; saves clone completed state and release that lock before disk I/O.
  Missed intervals skip rather than burst-replay. The server broadcasts after completed ticks,
  saves every 5 ticks, and saves once on graceful shutdown (`SIGTERM`/Ctrl-C).
- **Persistence** (`persistence.rs`) is `rusqlite` (bundled SQLite) with tables mirroring the
  old Drizzle schema (`world`, `colonies`, `cats`, `jobs`, `buildings`, `world_tiles`, `events`,
  `zones`, `elections`, `votes`, `raiders`) and additive `ALTER TABLE`-style migrations applied
  on open — same "migrate on connect" discipline as the old `db/client.ts`.
- **Identity** (`identity.rs`) issues and verifies HMAC-signed sessions
  (`SESSION_HMAC_SECRET`; refuses to boot in `NODE_ENV=production` without one, falls back to
  an insecure dev secret otherwise) — the hardening the old TS game's `docs/plan.md` flagged as
  a "forgeable sessionId, HMAC hardening is a flagged follow-up" is now implemented here.
- **Routing and security** bind signed identity and selected-colony state to each socket. A join
  reorders that socket's authorized shared-world projection so the selected colony is first while
  mutation context targets the same colony. Anonymous sockets see the global village read-only;
  authenticated sockets control it, owners additionally receive their personal village, and
  discovered foreign villages remain summary-only. Owner identity never enters the wire DTO.
- **Rate limiting** (`rate_limit.rs`) caps actions at 30 per 10-second window per session.
- **Production host** can serve the Trunk SPA and tracked images from the same process, with
  Brotli/gzip, cache headers, exact WebSocket Origin checks, and SPA fallback. The repository
  `Dockerfile` packages that mode as a non-root image.

Core env vars: `BIND_ADDR` (default `127.0.0.1`), `PORT` (default `8787`),
`GAME_DB_PATH` (default `data/cat.db`), and `SESSION_HMAC_SECRET`. Static production mode adds
`CAT_SERVER_WEB_DIST_DIR`, `CAT_SERVER_PUBLIC_IMAGES_DIR`, and
`CAT_SERVER_ALLOWED_ORIGINS`; see `docs/DEPLOYMENT.md`.

## cat-client / cat-desktop / cat-web — the renderer

`crates/cat-client` is a Bevy 0.19 **library** (`pub fn run()`), shared by two thin binaries:
`cat-desktop` (native) and `cat-web` (wasm, `wasm32-unknown-unknown`). It connects to
`cat-server` over WebSocket via `ewebsock` (a cross-platform WS crate with a browser backend),
deserializes `WorldSnapshot` on receipt, and stores it as a Bevy resource that render/UI
systems read each frame.

The renderer is **top-down**, not isometric — a deliberate pivot mid-migration (see
`docs/GAME_VISION.md`'s "design pivot" note and `docs/migration/BOARD.md` P9). It draws: biome
terrain generated client-side from the shared `world_seed` (via `cat_sim::generate_terrain_chunk`
— the client doesn't need the server to stream tile data, just the seed), fog of war, paved
roads, cats (colored by specialization, carrying marker, walk animation that interpolates toward
the latest snapshot tile rather than teleporting), label-free roofed homes and typed open
stations, stockpiles/gather spots, raiders, crop stages, and zone overlays. The HUD shows
resources with caps, colony census, event log, trade, officers, village selection, and
cat/building inspectors. Its full-page research screen renders and purchases the complete
500-study catalog with filter/search/pan/zoom.
The maintained P18 Adventure 9-patch/button/progress/minimap/cursor foundation is implemented and
native-framebuffer verified at 1024×768, 1280×800, and 1920×1080. New menus still require the same
responsive native and WASM interaction checks before they are called complete.

Art: curated pixel sprites under `public/images/game/{terrain,nature,buildings,interior,infra,
props,farm,enemies}/`, with accepted cat/raider sheets under `public/images/cats/` — see
`docs/assets/SELECTION.md` for the runtime mapping.

Bevy-specific gotchas (camera Z-layering, sprite/text API shapes, asset-root resolution) are
documented in `docs/HANDOFF.md` — read that before touching client rendering code.

### Native vs. browser

- **Native** (`cat-desktop`): `cargo run -p cat-desktop`, or `cargo dev` to launch server +
  client together. Uses `bevy`'s `multi_threaded`, `x11`, `wayland` features.
- **Browser** (`cat-web`): builds through Trunk for `wasm32-unknown-unknown`, serves the same
  tracked assets, uses `ewebsock`, and derives its production WebSocket URL from
  `window.location`. The bundle has been exercised end-to-end in Chromium and the combined
  server/WASM production image is verified. Optional transfer/performance work is tracked in
  `docs/migration/WASM.md`.

## cat-dev — the local dev launcher

`crates/cat-dev` is a tiny `std`-only binary (`cargo dev`, aliased in `.cargo/config.toml`)
that builds `cat-server` + `cat-desktop`, starts the server, waits for it to accept
connections, launches the desktop client pointed at it (`CAT_SERVER_URL`,
`BEVY_ASSET_ROOT=<workspace root>`), and kills the server when the client window closes. It
refuses to start if something is already listening on the target port, to avoid silently
attaching a fresh client to a stale, pre-rebuild server.

## Testing strategy

- **`cat-sim`**: 835+ pure unit/integration tests plus golden-master fixtures under
  `docs/migration/fixtures/` for modules ported from TS, generated by a one-off `npx tsx`
  script run against the *frozen, never-edited* TS source (`AGENTS.md` rule #5 — the sole
  permitted JS use in this codebase). Any new simulation constant needs a boundary test in the
  owning module, same discipline the old TS project enforced.
- **`cat-protocol`**: serde round-trip tests (serialize → deserialize → equal).
- **`cat-server`**: integration tests spin up the axum app in-process (no real socket needed)
  and drive it through `ClientAction` JSON, e.g. founding a village and asserting the shared
  snapshot updates.
- **`cat-client`**: logic/UI-shape tests supplement manual visual checks. Rendering is verified
  by capturing the client's own framebuffer to a PNG and reading it back (method documented in
  `docs/HANDOFF.md`), since "it compiles" has previously hidden a black-screen regression.

Quality gate before any commit (per `AGENTS.md`): `cargo nextest run -p <crate>`,
`cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo fmt`. Lefthook wires `cargo fmt`
on pre-commit and clippy + nextest on pre-push (`lefthook.yml`) — the JS lint/typecheck/test
hooks that used to gate the TypeScript game are no longer relevant to this workspace.

## What's not here yet

The P11 cutover is complete: the TypeScript implementation lives only on `archive/web-game`
(`web-final`), and the browser bundle plus combined production host run end-to-end. Remaining
work is the gameplay, client exposure, production breadth, and exhaustive QA backlog in
`docs/IMPLEMENTATION_AUDIT.md`; it is not unfinished migration work.
