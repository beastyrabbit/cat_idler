<div align="center">

# Idle Cat Forest

### An idle Dwarf Fortress, played by cats, in a forest.

_A self-running cat colony you nudge, not micromanage — now a native Rust + Bevy game._

**Project status: non-commercial.** Idle Cat Forest is developed and distributed solely as a
non-commercial game project.

![Rust](https://img.shields.io/badge/Rust-edition_2024-orange?style=flat-square&logo=rust)
![Bevy](https://img.shields.io/badge/Bevy-0.19-blue?style=flat-square)
![Tokio](https://img.shields.io/badge/tokio_+_axum-WebSocket-informational?style=flat-square)
![SQLite](https://img.shields.io/badge/SQLite_(rusqlite)-persistence-003b57?style=flat-square&logo=sqlite&logoColor=white)
![Tests](https://img.shields.io/badge/cat--sim_tests-770%2B_passing-brightgreen?style=flat-square)
![Status](https://img.shields.io/badge/status-pre--release%2C_migration_complete-yellow?style=flat-square)

</div>

---

## Leader intelligence overhaul

The colony now runs through a deterministic, report-limited strategic planner rather than a
perfect-information rule list. Leaders choose among survival, families/housing, construction and
storage, food/production, the Hole's endless physical feed demand, two-lane research, defense,
diplomacy, and material barter; seven specialist
officers improve decisions and may omit, delay, or choose poorly according to their experience.
Gods see the same reported knowledge the colony has, including regeneration only after an
effective level-4 officer report. Every visible job resolves to its real world site: Hunts to
caves, Water jobs to sources and banks/endpoints, and Workshop jobs to the complete 3×3 footprint.

The maintained design, implementation board, contributor extension guide, diagnostics, and
browser acceptance evidence are indexed in
[`docs/leader-ai-overhaul/README.md`](docs/leader-ai-overhaul/README.md).
Contributor entry points are the
[extension guide](docs/leader-ai-overhaul/extending-the-system.md),
[diagnostics guide](docs/leader-ai-overhaul/diagnostics-and-debugging.md), and
[browser play-test contract](docs/leader-ai-overhaul/browser-playtests/README.md).

**Delivery status (2026-07-25):** the first LAI.34 cutover is historical baseline; the exact
LAI.35–LAI.70 two-plan integration is in progress. The current board records implemented pure
foundations and every remaining runtime/protocol/persistence/server/client/art/diagnostic/deletion
gate. Shrine/Favor, generic stored Food/Fish/Preserves, scholar Insight, coin settlement, semantic
save conversion, direct routine micromanagement, and the old research UI are removal targets—not
restart compatibility requirements. See the
[integrated implementation map](docs/leader-ai-overhaul/integrated-implementation-map.md).

## What is this?

**"An idle version of Dwarf Fortress, played by cats, in a forest."** Idle Cat Forest is a
top-down, single-level god-sim: a cat colony lives, works, breeds, ages, researches, and
fights entirely on its own, driven by an authoritative server that ticks the simulation once
a second whether or not anyone is watching. The God influences broad priorities, player research,
temporary aid/boosts, one election backing block, personal diplomacy, and authorized expulsion;
the Leader owns exact routine jobs, sites, buildings, roads, crops, storage, production, food
permissions, officers, and workers. Cats keep bodily refusal and self-preservation.

The earlier P12 baseline used a **utility-AI leader director** to keep a fresh village alive with
bounded hunting, water, and scouting work, while seven officer roles automated specialist
categories. That historical behavior remains useful migration context; the production target is
the persistent, imperfect-knowledge planner described above and in the overhaul directory.
Cats walk every tile to get where they're going, lineages form through breeding, roads wear in
from traffic, stockpiles fill and empty, and raid pressure builds with your success. See
[`docs/GAME_VISION.md`](docs/GAME_VISION.md) for the full design pillars
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
  ├── broadcasts WorldSnapshot plus report-safe LAI.24 snapshots
  ├── applies authenticated/versioned LAI.25 actions and persists to SQLite (rusqlite)
  ├── keeps authenticated non-overlapping base-world ClientAction controls
  ├── rejects retired leader/progression ClientAction variants with UPDATE_REQUIRED
  └── HMAC-signed session identity, per-session rate limiting
        ↕ calls
cat-sim (pure, deterministic simulation core — no I/O, no rendering, no std::time)
```

Plus **cat-dev**, a small launcher bin (`cargo dev`) that builds and runs `cat-server` +
`cat-desktop` together for local development.

- **`cat-sim`** — the whole simulation as pure functions over plain data: life sim (aging,
  breeding, genetics, old-age/starvation death), movement + A* pathfinding, the persistent
  report-limited leader planner, jobs,
  production/hauling/physical storage, staged construction, families/housing/mentoring,
  governance/elections, Notes/Void research lanes, the Hole, divine policy, threat/raids/combat,
  zones, roads/walls/farms, terrain generation, and the DF-style item/material/barter economy.
  One `world_tick(&mut WorldState, now)` call
  runs the integrated ordered phases per colony per tick — the single source of truth, same discipline as
  the old TS `workerTick`. `#![forbid(unsafe_code)]`, no `rand` — all randomness goes through a
  ported seeded LCG (`rng.rs`) with forked chains for movement/life/raids so replay is
  deterministic. World state is multi-colony (`WorldState { colonies: Vec<ColonyRuntime> }`)
  from the ground up — the old game's single global colony is now colony `#1` of many, with
  player-founded villages (`found_colony`) as a first-class primitive.
- **`cat-protocol`** — `serde` wire types shared by client and server: the wider-world
  `WorldSnapshot`/`ColonySnapshot`, report-safe `LeaderAiSnapshotEnvelope` (LAI.24), and
  authenticated expected-versioned integrated action envelope. The final cutover removes direct
  base-world controls that bypass Leader authority; all remaining broad God actions have their own
  strict expected-version lane, idempotency, authorization, and typed rejection.
- **`cat-server`** — `axum` exposes `GET /health`, stateful `GET /ready`, and `GET /ws`
  (WebSocket). CPU-heavy simulation and synchronous persistence run on Tokio's blocking pool;
  new sockets receive a startup-initialized last-completed snapshot without waiting behind an
  in-progress tick. The loop ticks the shared world once a second, saves to SQLite every 5 ticks (plus a
  graceful-shutdown save), and broadcasts the new snapshot. Persistence
  (`crates/cat-server/src/persistence.rs`) uses a fresh strict schema for the integrated aggregates,
  physical identities, tasks/reservations/cargo, families/governance, Notes/Void research,
  construction/storage, divine effects, and barter. Known obsolete gameplay schemas reset/recreate;
  unknown/future/malformed state fails closed. Session identity is
  HMAC-signed (`SESSION_HMAC_SECRET`; the development fallback is loopback-only unless explicitly
  opted in). Public binds require an Origin allowlist. Actions and connections are bounded per
  authenticated session and effective client IP, including explicitly trusted reverse proxies.
- **`cat-client`** — a Bevy 0.19 app (`cat_client::run()`) shared by native and wasm: connects
  over WebSocket via `ewebsock`, deserializes `WorldSnapshot` every frame, and renders the
  world **top-down** (a design pivot away from the archived game's isometric map — see
  `docs/GAME_VISION.md`): terrain by biome, cats with shape-and-color specialization/officer
  badges and carried-item markers, label-free roofed homes and open craft stations, visible stockpiles/gather spots,
  fog of war, roads/walls/farms, construction/storage/family/enterprise states, and the exact
  five-screen Log/Stores/Village/Research/Council shell with six Council tabs. Research shows the
  canonical graph, physical God queue/preparation, free report-safe Leader lane, Notes/Void,
  permits, repeatables, and boosts. It renders only snapshot-provided truth and complete task
  geometry.
- **`cat-desktop`** / **`cat-web`** — thin binaries over `cat-client`. `cat-web` builds with
  Trunk, serves the selected assets, derives a same-origin WebSocket URL for deployment, and
  has been exercised end-to-end in Chromium. The production `Dockerfile` serves the optimized
  SPA, assets, probes, and WebSocket from one non-root server image with compression and exact
  Origin checks. See [`docs/migration/WASM.md`](docs/migration/WASM.md) for the build recipe and
  optional transfer-weight work.

For the full phase-by-phase build history and maintained rollup, see
[`docs/migration/BOARD.md`](docs/migration/BOARD.md). The post-cutover correctness pass and
P12–P19 design evidence are tracked in
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
BIND_ADDR=127.0.0.1                    # server bind IP; production image uses 0.0.0.0
GAME_DB_PATH=data/cat.db               # SQLite file (created + migrated automatically)
SESSION_HMAC_SECRET=...                # required for production and every public bind
CAT_SERVER_WEB_DIST_DIR=...            # optional Trunk dist served by cat-server
CAT_SERVER_PUBLIC_IMAGES_DIR=...       # optional image tree served at /public/images
CAT_SERVER_ALLOWED_ORIGINS=...         # required exact WS Origin allowlist for public binds
CAT_SERVER_TRUSTED_PROXY_IPS=...       # optional exact proxies allowed to supply one X-Forwarded-For IP
CAT_SERVER_URL=ws://127.0.0.1:8787/ws  # cat-desktop/cat-web: which server to connect to
BEVY_ASSET_ROOT=$PWD                   # cat-desktop: resolve public/images/... from the workspace root
```

The world ticks once a second (fixed; not currently configurable via env var).

### Browser / WASM build

The reproducible entry point is `scripts/build-web.sh` (or `scripts/build-web.sh --serve`). It
creates the Trunk release bundle under `crates/cat-web/dist/`; the browser client has been
live-verified in Chromium. Production packaging is the repository `Dockerfile` plus
[`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md). See
[`docs/migration/WASM.md`](docs/migration/WASM.md) for the smoke test and optional transfer and
performance work.

## Testing & determinism

`cat-sim` is pure and deterministic: no `std::time`, no threads, no `rand` — every random draw
goes through a ported seeded LCG with independently-forked chains for movement, life sim, and
raids, so a given seed reproduces the same run bit-for-bit. This is what makes the module
**unit-testable without a server or client**:

```bash
cargo nextest run --workspace --profile smoke # small local cross-crate safety net
cargo nextest run -p <crate> <test-filter>     # focused tests for the code being changed
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

The full workspace suite is intentionally a Forgejo responsibility: four `personal` runner jobs
execute deterministic Nextest hash partitions in parallel, so long simulation campaigns do not
consume the development workstation. The smoke profile is not a replacement for focused tests;
run the relevant module or regression test locally, then let the pushed Forgejo workflow provide
complete coverage.

Where a Rust module ports TS behavior, it's checked against **golden-master fixtures**
generated from the original TypeScript sim under `docs/migration/fixtures/` (seed → N ticks →
snapshot); parity is "same idea," not bit-identical `Math.random` output — see `AGENTS.md` for
the exact bar and rationale.

## Project structure

```
cat_idler/
├── crates/
│   ├── cat-sim/          # Pure deterministic simulation core (60+ modules, 53-phase world_tick)
│   ├── cat-protocol/     # WorldSnapshot plus LAI.24 snapshots and LAI.25 actions
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
| [`docs/migration/WASM.md`](docs/migration/WASM.md) | Verified browser/production build and optional optimization work |
| [`docs/assets/SELECTION.md`](docs/assets/SELECTION.md) | Sprite-family selection and runtime art mapping |
| [`AGENTS.md`](AGENTS.md) | Ground rules for the codex/Claude build team doing the port |

Docs describing the old TypeScript/Next.js game (`docs/plan.md`, `docs/ROADMAP.md`,
`docs/LEADER_AI_DESIGN.md`, `docs/TERRAIN_DESIGN.md`, `docs/ENGINE_PLATFORM.md`,
`docs/ENGINE_FRONTEND.md`, `docs/TASKS.md`, `docs/UI_CONCEPTS.md`) are
marked superseded at the top of each file and kept only as design-history reference for the
port — they no longer describe how to build, run, or test this project.
[`docs/TESTING.md`](docs/TESTING.md) is the maintained Rust/Bevy test workflow; the leader-AI
release additions are linked from its LAI section.

## Status

Pre-release, with the web→Rust/Bevy migration, P11 cutover, and maintained P12–P19 design complete. Verified product slices
include the responsive authoritative server, selected-village routing, a production browser
image, bounded world streaming, label-free roofed homes/open stations, the full-page 487-study
("about 500") ledger, exterior farming/logging with distinct Mill/Sawmill production, the
seven-role manual/officer split, physical local workshop logistics, exact road/rail/shipping
  routes, all 108 physical recipes, all 487 live studies, and exhaustive guided coverage of every
  public action. The global/personal village
model and founding housing/migration lifecycle are verified. The accepted founding contract is
15 adults in three five-bed Dens, slow
reserved-bed pregnancy, prosperity migration with 36 game-hours to house each arrival,
deterministic reset, physical emergency water hauling, and 240/288-game-hour ordinary versus
leader/healer old-age thresholds.
[`docs/IMPLEMENTATION_AUDIT.md`](docs/IMPLEMENTATION_AUDIT.md) is the living evidence ledger.
The tiered local-smoke/remote-full workflow is documented in
[`docs/TESTING.md`](docs/TESTING.md); newly reproduced defects go in
[`docs/FIX_LOG.md`](docs/FIX_LOG.md).

The paragraph above is the verified pre-LAI product baseline. Current leader-intelligence
completion and external release gates are tracked on the
[overhaul board](docs/leader-ai-overhaul/BOARD.md).

---

<div align="center">
  <sub>Built with human calories and mass GPU cycles.</sub>
</div>
