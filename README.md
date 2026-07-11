<div align="center">

# Idle Cat Forest

### An idle Dwarf Fortress, played by cats, in a forest.

_A self-running cat colony you nudge, not micromanage — now a native Rust + Bevy game._

![Rust](https://img.shields.io/badge/Rust-edition_2024-orange?style=flat-square&logo=rust)
![Bevy](https://img.shields.io/badge/Bevy-0.19-blue?style=flat-square)
![Tokio](https://img.shields.io/badge/tokio_+_axum-WebSocket-informational?style=flat-square)
![SQLite](https://img.shields.io/badge/SQLite_(rusqlite)-persistence-003b57?style=flat-square&logo=sqlite&logoColor=white)
![Tests](https://img.shields.io/badge/cat--sim_tests-650%2B_passing-brightgreen?style=flat-square)
![Status](https://img.shields.io/badge/status-pre--release%2C_migration_in_progress-yellow?style=flat-square)

</div>

---

## What is this?

**"An idle version of Dwarf Fortress, played by cats, in a forest."** Idle Cat Forest is a
top-down, single-level god-sim: a cat colony lives, works, breeds, ages, researches, and
fights entirely on its own, driven by an authoritative server that ticks the simulation once
a second whether or not anyone is watching. You don't control individual cats — you're a god
who shapes the world: found villages, paint zones, boost jobs, assign leadership roles, vote,
and spend a slow tech tree while the colony runs its own life.

A **utility-AI leader director** keeps almost every cat employed across a shared labor budget
(hunting, hauling, building, research, defense, farming…), so the colony reads as intentional
rather than random: cats walk every tile to get where they're going, lineages form through
breeding, roads wear in from traffic, stockpiles fill and empty, and raid pressure builds with
your success. See [`docs/GAME_VISION.md`](docs/GAME_VISION.md) for the full design pillars
(manual → role-automation, visible workplaces, production chains) and
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for how the Rust workspace implements it.

> **This is a rebuild.** The game shipped originally as a Next.js/TypeScript web app (a
> Victorian-newspaper-themed single shared colony). That version is frozen — reference only —
> on branch `archive/web-game` (tag `web-final`); it is being
> ported "same idea, not bit-identical" into this Rust + Bevy workspace. The TypeScript source
> files (`app/`, `lib/game/`, `server/`, `db/`, `types/`, `worker/`) still sit in this tree as
> the porting reference — they are not the running game and should not be edited or run as
> part of this project anymore. Full context in [`docs/HANDOFF.md`](docs/HANDOFF.md) and
> [`docs/migration/BOARD.md`](docs/migration/BOARD.md).

## Screenshots

The screenshots under [`docs/screenshots/`](docs/screenshots/) (newspaper UI, isometric map,
cat cards) are from the **archived TypeScript web version** — the Catford Examiner newspaper
and the isometric renderer were both dropped in the rebuild (see `AGENTS.md`: "the Catford
Examiner and its flavor generators are DROPPED"). They're kept for historical reference only
and are **not representative of the current top-down Bevy client**. No committed screenshot of
the Rust client exists yet; the standard way to see it is to build and run it yourself (below),
or see `docs/migration/HANDOFF.md`'s framebuffer-verification method for how the migration team
captures Bevy screenshots without a display.

## Architecture

One Cargo workspace under `crates/`:

```
cat-client (Bevy 0.19 renderer/UI, native + wasm-targetable)
  ├── cat-desktop   thin native launcher bin
  └── cat-web       thin wasm launcher bin (compiles; browser bundle not wired end-to-end yet)
        ↕ WebSocket (ewebsock), snapshot in / action out, JSON over cat-protocol
cat-server (tokio + axum, authoritative)
  ├── runs cat-sim's world_tick() once a second for every colony
  ├── broadcasts WorldSnapshot to all connected clients
  ├── receives ClientAction, applies it via cat-sim, persists to SQLite (rusqlite)
  └── HMAC-signed session identity, per-session rate limiting
        ↕ calls
cat-sim (pure, deterministic simulation core — no I/O, no rendering, no std::time)
```

Plus **cat-dev**, a small launcher bin (`cargo dev`) that builds and runs `cat-server` +
`cat-desktop` together for local development.

- **`cat-sim`** — the whole simulation as pure functions over plain data: life sim (aging,
  breeding, genetics, old-age/starvation death), movement + A* pathfinding, the leader
  director (a utility-AI that allocates a shared labor budget across colony goals), jobs,
  production/hauling/storage, an upgrade tree with god-purchase and cat-research paths,
  threat/raids/combat, elections, zones, roads, terrain generation, and the newer DF-style
  item/material economy (crafting, traders, coin). One `world_tick(&mut WorldState, now)` call
  runs ~40 ordered phases per colony per tick — the single source of truth, same discipline as
  the old TS `workerTick`. `#![forbid(unsafe_code)]`, no `rand` — all randomness goes through a
  ported seeded LCG (`rng.rs`) with forked chains for movement/life/raids so replay is
  deterministic. World state is multi-colony (`WorldState { colonies: Vec<ColonyRuntime> }`)
  from the ground up — the old game's single global colony is now colony `#1` of many, with
  player-founded villages (`found_colony`) as a first-class primitive.
- **`cat-protocol`** — `serde` wire types shared by client and server: `WorldSnapshot` /
  `ColonySnapshot` (resources, cats, jobs, buildings, upgrades, threat, raiders, zones, items,
  officers, gather spots, road tiles…) and a ~29-variant `ClientAction` enum (found/join
  village, request job, boost, purchase upgrade, vote, zones, plan building, unlock node,
  assign worker/officer, train warrior, defend raid, build road, designate stockpile/gather
  spot, sell/buy goods, boost cat, test-acceleration controls).
- **`cat-server`** — `axum` exposes `GET /health` and `GET /ws` (WebSocket). A `tokio::spawn`ed
  loop ticks every connected world once a second, saves to SQLite every 5 ticks (plus a
  graceful-shutdown save), and broadcasts the new snapshot. Persistence
  (`crates/cat-server/src/persistence.rs`) mirrors the old Drizzle schema in `rusqlite` tables
  (`world`, `colonies`, `cats`, `jobs`, `buildings`, `world_tiles`, `events`, `zones`,
  `elections`, `votes`, `raiders`) with additive migrations on open. Session identity is
  HMAC-signed (`SESSION_HMAC_SECRET`, falls back to an insecure dev secret outside
  `NODE_ENV=production`); actions are rate-limited (30 / 10s per session).
- **`cat-client`** — a Bevy 0.19 app (`cat_client::run()`) shared by native and wasm: connects
  over WebSocket via `ewebsock`, deserializes `WorldSnapshot` every frame, and renders the
  world **top-down** (a design pivot away from the TS game's isometric map — see
  `docs/GAME_VISION.md`): terrain by biome, cats colored by specialization with carried-item
  glyphs, labelled buildings with craft-station sprites, visible stockpiles/gather spots,
  fog of war, roads, raiders, a DF-Steam-styled HUD (resources, census, event log, upgrade
  tree, trade menu, cat inspector), and action buttons that round-trip over the WebSocket.
- **`cat-desktop`** / **`cat-web`** — thin binaries over `cat-client`. `cat-web` builds cleanly
  to `wasm32-unknown-unknown`; a running in-browser build (trunk bundling, asset serving,
  location-derived WS URL) is scouted but not fully wired — see
  [`docs/migration/WASM.md`](docs/migration/WASM.md) for exact status and next steps.

For the full phase-by-phase build history and current in-flight work, see
[`docs/migration/BOARD.md`](docs/migration/BOARD.md) (kept current through P9; later phases —
P12 sim expansion, P14–P19 spatial/biome/visual/economy work — are tracked in
[`docs/migration/specs/`](docs/migration/specs/) and the git log rather than the board).

## How to run it

Requires a graphical session for the native client (Bevy opens a window).

```bash
# One command: builds + runs cat-server and cat-desktop together, wires the client
# to the server's WebSocket, and stops the server when the client window closes.
cargo dev
```

Or run the two halves yourself in separate terminals:

```bash
# Terminal 1 — the authoritative server (the world; keeps ticking with no client attached)
cargo run -p cat-server
curl http://127.0.0.1:8787/health   # -> ok

# Terminal 2 — the Bevy client window
BEVY_ASSET_ROOT=$PWD CAT_SERVER_URL=ws://127.0.0.1:8787/ws cargo run -p cat-desktop
```

### Environment

```env
PORT=8787                              # cat-server listen port (both binaries agree on this via cat-dev)
GAME_DB_PATH=data/cat.db               # SQLite file (created + migrated automatically)
SESSION_HMAC_SECRET=...                # required in NODE_ENV=production; insecure dev default otherwise
CAT_SERVER_URL=ws://127.0.0.1:8787/ws  # cat-desktop/cat-web: which server to connect to
BEVY_ASSET_ROOT=$PWD                   # cat-desktop: resolve public/images/... from the workspace root
```

The world ticks once a second (fixed; not currently configurable via env var).

### Browser / WASM build

`cargo build -p cat-web --target wasm32-unknown-unknown` compiles clean today, but there is no
committed `trunk` bundle or in-browser smoke test yet. See
[`docs/migration/WASM.md`](docs/migration/WASM.md) for the concrete remaining steps (bundle,
asset serving, location-derived WS URL, canvas sizing) and known risks (bundle size, WebGL2
parity, non-localhost WS).

## Testing & determinism

`cat-sim` is pure and deterministic: no `std::time`, no threads, no `rand` — every random draw
goes through a ported seeded LCG with independently-forked chains for movement, life sim, and
raids, so a given seed reproduces the same run bit-for-bit. This is what makes the module
**unit-testable without a server or client**:

```bash
cargo test -p cat-sim        # ~650 unit tests, pure logic, no I/O — fast
cargo test -p cat-protocol   # wire-type round-trip tests
cargo test -p cat-server     # WS/action integration tests (in-process, no real network needed)
cargo nextest run -p <crate> # preferred runner if cargo-nextest is installed
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

Where a Rust module ports TS behavior, it's checked against **golden-master fixtures**
generated from the original TypeScript sim under `docs/migration/fixtures/` (seed → N ticks →
snapshot); parity is "same idea," not bit-identical `Math.random` output — see `AGENTS.md` for
the exact bar and rationale.

## Project structure

```
cat_idler/
├── crates/
│   ├── cat-sim/          # Pure deterministic simulation core (~40 modules, ~40-phase world_tick)
│   ├── cat-protocol/     # serde wire types: WorldSnapshot/ColonySnapshot + ClientAction
│   ├── cat-server/       # tokio + axum WS server, rusqlite persistence, identity, rate-limit
│   ├── cat-client/       # Bevy 0.19 renderer + UI (native + wasm)
│   ├── cat-desktop/      # native launcher bin over cat-client
│   ├── cat-web/          # wasm launcher bin over cat-client
│   └── cat-dev/          # `cargo dev` — builds/runs server + desktop client together
├── docs/
│   ├── ARCHITECTURE.md   # Rust workspace architecture (start here)
│   ├── GAME_VISION.md    # design pillars for the DF-style rebuild
│   ├── HANDOFF.md        # migration status + hard-won lessons for whoever picks this up
│   ├── migration/        # BOARD.md (task board), specs/ (design specs p2–p19), fixtures/
│   └── assets/           # sprite pack selection + catalogs for the Bevy client
├── public/images/game/   # Kenney Roguelike 16px sprites used by cat-client
├── app/, lib/game/, server/, db/, types/, worker/   # ARCHIVED TypeScript reference (do not run/edit)
└── tests/                # TypeScript test suite for the archived game (not run as part of this project)
```

## Documentation

| Doc | What it covers |
| --- | --- |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | The Rust workspace: crates, the tick loop, protocol, persistence, client rendering |
| [`docs/GAME_VISION.md`](docs/GAME_VISION.md) | Design pillars for "Idle Cat Forest" (manual → role-automation, visible workplaces) |
| [`docs/HANDOFF.md`](docs/HANDOFF.md) | Migration status, architecture, hard-won Bevy/codex lessons |
| [`docs/migration/BOARD.md`](docs/migration/BOARD.md) | Phase-by-phase task board (P0–P9 tracked in detail) |
| [`docs/migration/specs/`](docs/migration/specs/) | Design specs for pathfinding, leader director, world_tick, and P12–P19 (skills/roles, spatial placement, biomes, visual polish, item economy) |
| [`docs/migration/WASM.md`](docs/migration/WASM.md) | Browser/WASM build feasibility + remaining steps |
| [`docs/assets/SELECTION.md`](docs/assets/SELECTION.md) | Sprite pack selection + licensing notes for the Bevy client's art |
| [`AGENTS.md`](AGENTS.md) | Ground rules for the codex/Claude build team doing the port |

Docs describing the old TypeScript/Next.js game (`docs/plan.md`, `docs/ROADMAP.md`,
`docs/LEADER_AI_DESIGN.md`, `docs/TERRAIN_DESIGN.md`, `docs/ENGINE_PLATFORM.md`,
`docs/ENGINE_FRONTEND.md`, `docs/TASKS.md`, `docs/TESTING.md`, `docs/UI_CONCEPTS.md`) are
marked superseded at the top of each file and kept only as design-history reference for the
port — they no longer describe how to build, run, or test this project.

## Status

Pre-release. Simulation core, server, and multi-colony founding are done and live-verified.
The Bevy client renders the full top-down world with a working HUD and is mid-buildout on
newer sim systems (spatial stockpiles, gather spots, the item/material economy, transport
upgrades). Ore/metal mining is spec'd (`docs/migration/specs/p17-biome-generator.md`) but not
yet wired into `world_tick`. A browser/WASM build and the final cutover (retiring the
TypeScript reference tree) are still pending. See `docs/HANDOFF.md` for the living status.

---

<div align="center">
  <sub>Built with human calories and mass GPU cycles.</sub>
</div>
