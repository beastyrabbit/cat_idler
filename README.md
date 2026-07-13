<div align="center">

# Idle Cat Forest

### An idle Dwarf Fortress, played by cats, in a forest.

_A self-running cat colony you nudge, not micromanage — now a native Rust + Bevy game._

![Rust](https://img.shields.io/badge/Rust-edition_2024-orange?style=flat-square&logo=rust)
![Bevy](https://img.shields.io/badge/Bevy-0.19-blue?style=flat-square)
![Tokio](https://img.shields.io/badge/tokio_+_axum-WebSocket-informational?style=flat-square)
![SQLite](https://img.shields.io/badge/SQLite_(rusqlite)-persistence-003b57?style=flat-square&logo=sqlite&logoColor=white)
![Tests](https://img.shields.io/badge/cat--sim_tests-650%2B_passing-brightgreen?style=flat-square)
![Status](https://img.shields.io/badge/status-pre--release%2C_migration_complete-yellow?style=flat-square)

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
> on branch `archive/web-game` (tag `web-final`). The "same idea, not bit-identical" Rust +
> Bevy migration is complete, and the TypeScript source was removed from `main` at the P11
> cutover. Full context is in [`docs/HANDOFF.md`](docs/HANDOFF.md) and
> [`docs/migration/BOARD.md`](docs/migration/BOARD.md).

## Screenshots

The screenshots under [`docs/screenshots/`](docs/screenshots/) (newspaper UI, isometric map,
cat cards) are from the **archived TypeScript web version** — the Catford Examiner newspaper
and the isometric renderer were both dropped in the rebuild (see `AGENTS.md`: "the Catford
Examiner and its flavor generators are DROPPED"). They're kept for historical reference only
and are **not representative of the current top-down Bevy client**. No committed screenshot of
the Rust client exists yet; the standard way to see it is to build and run it yourself (below),
or see `docs/HANDOFF.md`'s framebuffer-verification method for how the migration team
captures Bevy screenshots without a display.

## Architecture

One Cargo workspace under `crates/`:

```
cat-client (Bevy 0.19 renderer/UI, native + wasm-targetable)
  ├── cat-desktop   thin native launcher bin
  └── cat-web       thin wasm launcher bin (Trunk bundle live-verified in Chromium)
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
- **`cat-desktop`** / **`cat-web`** — thin binaries over `cat-client`. `cat-web` builds with
  Trunk, serves the selected assets, derives a same-origin WebSocket URL for deployment, and
  has been exercised end-to-end in Chromium. See [`docs/migration/WASM.md`](docs/migration/WASM.md)
  for the build recipe and remaining transfer-weight/hosting work.

For the full phase-by-phase build history and current in-flight work, see
[`docs/migration/BOARD.md`](docs/migration/BOARD.md). The post-cutover correctness pass and
partial P12–P19 design promises are tracked in
[`docs/IMPLEMENTATION_AUDIT.md`](docs/IMPLEMENTATION_AUDIT.md).

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

The reproducible entry point is `scripts/build-web.sh` (or `scripts/build-web.sh --serve`). It
creates the Trunk release bundle under `crates/cat-web/dist/`; the browser client has been
live-verified in Chromium. See [`docs/migration/WASM.md`](docs/migration/WASM.md) for the smoke
test and remaining hosting, caching, and transfer-weight work.

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
└── public/images/game/   # Kenney Roguelike 16px sprites used by cat-client
```

The original Next.js/TypeScript web game was retired at the P11 cutover — it lives, fully
runnable, on branch `archive/web-game` (tag `web-final`).

## Documentation

| Doc | What it covers |
| --- | --- |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | The Rust workspace: crates, the tick loop, protocol, persistence, client rendering |
| [`docs/GAME_VISION.md`](docs/GAME_VISION.md) | Design pillars for "Idle Cat Forest" (manual → role-automation, visible workplaces) |
| [`docs/HANDOFF.md`](docs/HANDOFF.md) | Migration status, architecture, hard-won Bevy/codex lessons |
| [`docs/IMPLEMENTATION_AUDIT.md`](docs/IMPLEMENTATION_AUDIT.md) | Current design-to-code gaps, active fixes, and full playtest matrix |
| [`docs/migration/BOARD.md`](docs/migration/BOARD.md) | Phase-by-phase task board (P0–P9 tracked in detail) |
| [`docs/migration/specs/`](docs/migration/specs/) | Design specs for pathfinding, leader director, world_tick, and P12–P19 (skills/roles, spatial placement, biomes, visual polish, item economy) |
| [`docs/migration/WASM.md`](docs/migration/WASM.md) | Browser/WASM build feasibility + remaining steps |
| [`docs/assets/SELECTION.md`](docs/assets/SELECTION.md) | Sprite pack selection and provenance for the Bevy client's art |
| [`AGENTS.md`](AGENTS.md) | Ground rules for the codex/Claude build team doing the port |

Docs describing the old TypeScript/Next.js game (`docs/plan.md`, `docs/ROADMAP.md`,
`docs/LEADER_AI_DESIGN.md`, `docs/TERRAIN_DESIGN.md`, `docs/ENGINE_PLATFORM.md`,
`docs/ENGINE_FRONTEND.md`, `docs/TASKS.md`, `docs/TESTING.md`, `docs/UI_CONCEPTS.md`) are
marked superseded at the top of each file and kept only as design-history reference for the
port — they no longer describe how to build, run, or test this project.

## Status

Pre-release, but the web→Rust/Bevy migration is complete. Simulation core, server, and
multi-colony founding are done and live-verified. The Bevy client renders the full top-down
world with a cohesive "cozy ledger" UI (spatial stockpiles, gather spots, the item/material
economy, ore/metal mining, research/School buildings — all wired). The browser/WASM build runs
end-to-end in Chromium, and the P11 cutover is done: `main` is now the Rust game and the old
TypeScript tree lives on `archive/web-game`. The one deliberately-partial area is the
officer/role split (assignable roles exist; the single leader director still does most labor
allocation). See `docs/IMPLEMENTATION_AUDIT.md` for the living, evidence-backed status.

---

<div align="center">
  <sub>Built with human calories and mass GPU cycles.</sub>
</div>
