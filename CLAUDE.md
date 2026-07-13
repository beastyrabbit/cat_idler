# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Idle Cat Forest** — "an idle version of Dwarf Fortress, played by cats, in a forest." A
top-down, single-level god-sim: a cat colony lives, works, breeds, ages, researches, and
fights on its own, driven by an authoritative server that ticks the simulation once a second
whether or not anyone is watching. Players are gods, not controllers — found villages, paint
zones, boost jobs, assign leadership roles, vote, and spend a slow tech tree.

Built as a Rust **Cargo workspace** under `crates/`:
- **cat-sim** — pure, deterministic simulation core (no I/O). `world_tick()` is the single
  source of truth, ~40 ordered phases per colony per tick.
- **cat-protocol** — `serde` wire DTOs (`WorldSnapshot`/`ColonySnapshot` + `ClientAction`)
  shared by server and client.
- **cat-server** — tokio + axum authoritative server: runs `world_tick` for every colony once
  a second, broadcasts snapshots over WebSocket, persists to SQLite (`rusqlite`).
- **cat-client** — Bevy 0.19 renderer/UI (library, native + wasm-targetable), top-down.
- **cat-desktop** / **cat-web** — thin native / wasm launcher binaries over `cat-client`.
- **cat-dev** — `cargo dev`, a local launcher that builds + runs `cat-server` + `cat-desktop`
  together.

**The rebuild is complete — this tree is now the Rust/Bevy game.** The game originally shipped
as a Next.js/TypeScript web app (a Victorian-newspaper-themed single shared colony, Drizzle ORM
+ `better-sqlite3`). That version was ported "same idea, not bit-identical" into this Rust +
Bevy workspace and then **retired at the P11 cutover** (2026-07-11): the TypeScript source
(`app/`, `lib/game/`, `server/`, `db/`, `types/`, `worker/`, `tests/`, and its JS build configs)
was removed from this tree. It remains fully preserved — runnable — on branch `archive/web-game`
(tag `web-final`, commit `8d3bc5a`); check that branch out if you need the original spec (e.g. to
regenerate a golden-master fixture). The Rust module doc-comments still cite the TS files they
were ported from (e.g. "ported from `lib/game/lifeSim.ts`") as historical provenance — those are
pointers into the archive branch, not files in this tree. See `README.md` and
`docs/ARCHITECTURE.md` for the full picture; `docs/HANDOFF.md` for hard-won build lessons;
`docs/migration/BOARD.md` for phase-by-phase status.

## Commands

```bash
# Run the game (native; requires a graphical session — Bevy opens a window)
cargo dev                    # builds + runs cat-server + cat-desktop together, one command.
                              # Refuses to start if something is already listening on the port
                              # (stale cat-server from an earlier run) — kill it first
                              # (e.g. `pkill -f target/debug/cat-server`) rather than silently
                              # connecting a fresh client to stale, pre-rebuild server data.

# Or run the two halves yourself in separate terminals:
cargo run -p cat-server                                                     # Terminal 1: the world
BEVY_ASSET_ROOT=$PWD CAT_SERVER_URL=ws://127.0.0.1:8787/ws cargo run -p cat-desktop  # Terminal 2: the window

# Reset to a fresh founding
rm data/cat.db                # SQLite is recreated + migrated automatically on next server start

# Testing
cargo nextest run -p cat-sim       # ~680 pure unit tests, no I/O — fast (preferred runner)
cargo nextest run --workspace      # everything (cat-sim, cat-protocol, cat-server)
cargo test -p cat-sim              # works too if cargo-nextest isn't installed

# Quality (must be green before any commit, per AGENTS.md)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check         # `cargo fmt --all` to fix

# WASM (compiles clean; not yet wired to a running in-browser build — see docs/migration/WASM.md)
cargo build -p cat-web --target wasm32-unknown-unknown
```

The TypeScript toolchain (`bun run dev`, `bun run db:generate`, `bun run test`, etc.) is gone
from this tree — it only ever drove the old web game, which now lives on branch
`archive/web-game`. Check that branch out if you need it.

## Architecture

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
they render whatever `WorldSnapshot` they last received, and a player action is only reflected
once the server's next tick includes its effect. No client-side prediction.

### The tick

- **`world_tick(&mut WorldState, now_ms) -> Vec<TickReport>`** (`crates/cat-sim/src/world_tick.rs`)
  is the single entry point, called once per colony per second by `cat-server`'s tokio loop.
  It runs ~40 ordered phases (`fn phase_*`) — life sim → consumption/spoilage →
  elections/zones → path decay/regrowth → job promotion → leader plan/direct/assign →
  production/research → survival → due-job completion → hauling → movement → roads → raids →
  status/persist-prep — mirroring the original TS `server/game.ts:workerTick`'s phase ordering
  (per `AGENTS.md` rule #1: "parity, not reinvention"). **Do not add a second tick path.**
- **World shape is multi-colony from the ground up**: `WorldState { colonies: Vec<ColonyRuntime>, .. }`.
  The old TS game's single global colony is now just `colonies[0]`; `found_colony(...)` is a
  first-class primitive for player-founded villages elsewhere on the map.

### Determinism

All randomness goes through a ported seeded LCG (`crates/cat-sim/src/rng.rs`:
`seed × 1_664_525 + 1_013_904_223 mod 2^32`, matching the original TS `seededRng.ts` bit for
bit). Independent subsystems fork the seed by a fixed offset so adding a roll to one subsystem
doesn't perturb another's sequence in the same tick: movement `+1_000_003`, life sim
(breeding/aging/mortality) `+2_000_003`, raids `+3_000_003`. `#![forbid(unsafe_code)]`, no
`rand` crate in `cat-sim`. This is what makes golden-master testing and "determinism twin"
tests possible (same seed → byte-identical trajectory across two independent runs).

### cat-sim module map (by concern)

| Concern | Modules |
| --- | --- |
| Foundation | `rng`, `types`, `entities`, `cost_constants`, `needs_constants`, `test_acceleration` |
| World generation | `noise`, `terrain_gen`, `world_gen`, `biomes`, `climate` |
| Cat AI | `pathfinding`, `movement`, `policy`, `tasks`, `cat_ai`, `leader_ai`, `leader_director`, `officers` |
| Life sim | `needs`, `age`, `breeding`, `genetics`, `life_sim`, `survival` |
| Economy | `idle_engine`, `idle_rules`, `production`, `smithy`, `storage`, `shrine`, `trips`, `depletion`, `spoilage`, `housing`, `roads`, `village_layout`, `village_area`, `stockpiles`, `skills`, `ledger` |
| Military & governance | `threat`, `warriors`, `combat`, `elections`, `zones`, `upgrade_tree` |
| Item economy | `items`, `recipes`, `trader` |
| Orchestration | `world_tick` (the tick loop), `actions` (pure `apply_action` + `build_snapshot`) |

Each module's doc comment cites the original TypeScript file it was ported from (e.g.
`leader_director.rs` ← `lib/game/leaderDirector.ts`) — the fastest way to find the ported
behavior's original spec/tests. Design detail beyond this table (leader director response
curves, pathfinding cost model, world_tick phase list, per-phase P12–P19 gameplay specs) lives
in `docs/migration/specs/` — read the relevant spec before touching a system, don't rely on
memory of the old TS code.

**Leader director** (`leader_director.rs`, per `docs/LEADER_AI_DESIGN.md` design intent) is a
utility-AI (IAUS-style) that scores colony goals on one [0,1] scale from response curves, then
hands a shared labor budget to the highest-scoring goals. It's the seed of a planned
role/officer system (Steward/Forester/Farmer/Captain/Loremaster, per `docs/GAME_VISION.md`)
that would split automation into assignable roles — `officers.rs` and
`AssignOfficer`/`UnassignOfficer` actions are scaffolded but the director is not yet fully
split; most labor allocation still runs through the single director.

### Known gaps (intentionally deferred, not bugs)

- **Ore/metal mining** is wired (mountain biomes, smelter → metal bars → better gear); the
  officer/role split described above is still partial — officer roles are an *additive*
  assignable automation layer, but the single leader director still runs most labor allocation
  (it is not yet fully split into per-role directors).
- **Cutover** (`docs/migration/BOARD.md` P11): the TypeScript reference tree is frozen but
  still physically present; retiring it is a pending, deliberate step done at merge time.

Recently closed (no longer gaps): `ResearchHut` and `School` are both fully ported — buildable,
staffable research faucets (`School` also adds a +50% research-rate multiplier via its upgrade
node). The WASM/browser build is done — `scripts/build-web.sh` produces a release bundle that
runs end-to-end in Chromium (WebGL2, live WS stream, 0 console errors); see
`docs/migration/WASM.md`.

### cat-protocol — the wire contract

One dependency (`serde`). `WorldSnapshot` (top-level, `Vec<ColonySnapshot>` + world fields) /
`ColonySnapshot` (one colony's full renderable state) / `ClientAction` (a large enum covering
everything a player can do — found/join village, request job, boost, purchase upgrade, vote,
zones, plan building, unlock node, assign worker/officer, train warrior, defend raid, build
road, designate stockpile/gather spot, sell/buy goods, boost cat, test-acceleration controls).
Field names are `camelCase` on the wire (matching the old TS API shape where it still matters).

### cat-server — the authoritative server

`GET /health` liveness probe; `GET /ws` WebSocket upgrade (each connection reads `ClientAction`
JSON frames, calls `apply_action` against the shared `Arc<Mutex<WorldState>>`, forwards the
broadcast `WorldSnapshot` stream). A `tokio::spawn`ed loop calls `world_tick` once per second
for the whole world (fixed `Duration::from_secs(1)` in `main.rs` — not currently
env-configurable), broadcasts the resulting snapshot, and saves to SQLite every 5 ticks plus
once on graceful shutdown. Identity (`identity.rs`) issues/verifies HMAC-signed sessions
(`SESSION_HMAC_SECRET`; refuses to boot in `NODE_ENV=production` without one, falls back to an
insecure dev secret otherwise). Rate limiting caps actions at 30 per 10-second window per
session.

### cat-client / cat-desktop / cat-web — the renderer

Bevy 0.19 library (`cat_client::run()`) shared by native (`cat-desktop`) and wasm (`cat-web`).
Connects to `cat-server` over WebSocket via `ewebsock`, deserializes `WorldSnapshot` on
receipt, stores it as a Bevy resource that render/UI systems read each frame. **Top-down**, not
isometric — a deliberate design pivot mid-migration (`docs/GAME_VISION.md`). Draws biome
terrain generated client-side from the shared `world_seed` (via `cat_sim::generate_terrain_chunk`
— no need for the server to stream tile data), fog of war, roads, cats (colored by
specialization, carried-item glyph), labelled buildings with craft-station sprites,
stockpiles/gather spots, raiders, zone overlays, cutaway top-down building interiors, and a
DF-Steam-styled HUD (resources w/ caps, census, event log, upgrade tree browse + purchase,
trade menu, cat/building inspectors).

Art: curated Kenney "Roguelike 16px" sprites under
`public/images/game/{terrain,nature,buildings,infra,props,farm,enemies}/` — see
`docs/assets/SELECTION.md` for asset provenance and selection rationale. Bevy-specific gotchas (camera Z-layering —
keep the camera at Z~1000, sprites below it or they get clipped/black-screened; `Sprite`/`Text`
API shapes; asset-root resolution via `BEVY_ASSET_ROOT`) are documented in `docs/HANDOFF.md` —
read it before touching client rendering code. There is no automated visual test suite; Bevy
rendering is verified manually by capturing the client's own framebuffer to a PNG and reading
it back (method in `docs/HANDOFF.md`), since "it compiles" has previously hidden a black-screen
regression.

## Persistence (cat-server)

`crates/cat-server/src/persistence.rs` uses `rusqlite` (bundled SQLite, not Drizzle) with
tables mirroring the old TS schema (`world`, `colonies`, `cats`, `jobs`, `buildings`,
`world_tiles`, `events`, `zones`, `elections`, `votes`, `raiders`). Colony resources are stored
as a JSON blob rather than one column per resource. Migrations are idempotent `ALTER
TABLE`/`ADD COLUMN`-style statements applied on open — same "migrate on connect" discipline as
the old `db/client.ts`, but hand-written Rust rather than generated SQL files (there is no
`db:generate` equivalent; add a new column by adding an `ADD COLUMN` migration statement plus
the corresponding struct field and read/write code).

## Testing Contract

- **`cat-sim`** is pure and deterministic (no `std::time`, no threads, no `rand`): plain
  `#[test]` unit tests (~680, sub-20s) plus golden-master fixtures under
  `docs/migration/fixtures/` for modules ported from TS (generated by a one-off `npx tsx`
  script run against the frozen, never-edited TS source — the sole permitted JS use in this
  codebase, per `AGENTS.md` rule #5). Parity bar is *behavioral* ("same idea"), not
  byte-identical `Math.random` output.
- **Determinism twins**: for long-horizon / RNG-sensitive behavior, pair a guardrail test with
  a twin that runs the same seed twice and asserts byte-identical trajectories (e.g.
  `founding_colony_sustains_its_population_over_a_long_horizon` /
  `founding_population_trajectory_is_deterministic_for_identical_seeds` in `world_tick.rs`) —
  this is the pattern to follow for any new long-running or seed-driven guardrail.
  `run_founding_population_trajectory` (`world_tick.rs`) is the current "survival proof": it
  runs a freshly founded colony unattended at a 5-game-minute tick cadence over 100+ game hours
  and asserts it never goes fully extinct, never dips near extinction after a 30h establishment
  window, and sustains a mean population at or above the founding count — a deliberately
  harsher proxy than the live 1s tick, so passing it is a conservative sustainability guarantee.
- **`cat-protocol`**: serde round-trip tests (serialize → deserialize → equal).
- **`cat-server`**: integration tests spin up the axum app in-process (no real socket needed)
  and drive it through `ClientAction` JSON (e.g. founding a village, asserting the shared
  snapshot updates).
- **`cat-client`**: no automated tests; verified manually via framebuffer capture (see above).
- Any new simulation constant/limit needs a boundary test in the owning `cat-sim` module, same
  discipline the old TS project enforced.

Quality gate before any commit (per `AGENTS.md`): `cargo nextest run -p <crate>`,
`cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo fmt`.

## Git Hooks (lefthook)

`lefthook.yml` (Rust-only since the P11 cutover removed the JS toolchain):
- **pre-commit**: `gitleaks` (secret detection), `cargo fmt --all -- --check` (`.rs` only).
- **pre-push**: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo nextest run
  --workspace` (`.rs` only).

Heavy Rust steps (clippy, tests) live on pre-push, not pre-commit, to keep commits fast while
Bevy is a dependency (slow incremental compiles).

## Environment

```
PORT=8787                              # cat-server listen port (both binaries agree via cat-dev)
GAME_DB_PATH=data/cat.db               # SQLite file (created + migrated automatically)
SESSION_HMAC_SECRET=...                # required in NODE_ENV=production; insecure dev default otherwise
CAT_SERVER_URL=ws://127.0.0.1:8787/ws  # cat-desktop/cat-web: which server to connect to
BEVY_ASSET_ROOT=$PWD                   # cat-desktop: resolve public/images/... from the workspace root
```

The world ticks once a second (fixed; not currently configurable via env var).

## Gotchas

- Requires a graphical session for the native client (Bevy opens a window).
- `cargo dev` refuses to start if something is already listening on the target port — that's a
  stale `cat-server` from an earlier run; kill it first rather than letting a fresh client
  silently attach to stale, pre-rebuild server data.
- `BEVY_ASSET_ROOT` must point at the workspace root (not `cat-desktop`'s `CARGO_MANIFEST_DIR`)
  for `public/images/...` to resolve — `cargo dev` handles this for you; set it yourself when
  running `cat-desktop` directly.
- Bevy 0.19: a default `Camera2d` sits at Z=0 and clips sprites at Z>0 — keep the camera at
  Z~1000, sprites below it, or you'll get a silent black screen. See `docs/HANDOFF.md` for more
  Bevy API-shape gotchas (`Sprite::from_color`, `Text` tuple access, `single_mut()` returning a
  `Result`, `Anchor` as its own component).
- The TypeScript reference tree (`app/`, `lib/game/`, `server/`, `db/`, `types/`, `worker/`,
  `tests/`) was **removed at the P11 cutover** — it lives on branch `archive/web-game` (tag
  `web-final`) only. If you need the original behavior spec, read it there; don't try to re-add
  it here. The Rust doc-comments' "ported from `lib/game/*.ts`" citations point into that branch.
- If you hit a spurious linker error (`undefined hidden symbol ... drop_in_place`), run `cargo
  clean -p cat-sim` and retest.
- Docs describing the old TypeScript/Next.js game (`docs/plan.md`, `docs/ROADMAP.md`,
  `docs/LEADER_AI_DESIGN.md`, `docs/TERRAIN_DESIGN.md`, `docs/ENGINE_PLATFORM.md`,
  `docs/ENGINE_FRONTEND.md`, `docs/TASKS.md`, `docs/TESTING.md`, `docs/UI_CONCEPTS.md`) are
  marked superseded and kept only as design-history reference — they don't describe how to
  build, run, or test this project anymore.

## Key Documentation

- `README.md` — quick start, environment, testing/determinism summary
- `docs/ARCHITECTURE.md` — the Rust workspace: crates, the tick loop, protocol, persistence,
  client rendering (start here for anything architectural)
- `docs/GAME_VISION.md` — design pillars for "Idle Cat Forest" (manual → role-automation,
  visible workplaces, production chains)
- `docs/HANDOFF.md` — migration status + hard-won Bevy/codex build lessons
- `docs/migration/BOARD.md` — phase-by-phase task board (P0–P9 tracked in detail; later phases
  tracked in `docs/migration/specs/` and the git log)
- `docs/migration/specs/` — design specs for pathfinding, leader director, `world_tick`, and
  the P12–P19 gameplay systems (skills/roles, spatial placement, biome generator, visual
  polish, item economy)
- `docs/migration/WASM.md` — browser/WASM build feasibility + remaining steps
- `docs/assets/SELECTION.md` — sprite pack selection and provenance for the Bevy client's art
- `AGENTS.md` — ground rules for the codex/Claude build team doing the port (parity discipline,
  determinism rules, the one permitted JS use, commit conventions)

## Status

Pre-release, migration effectively complete. Simulation core, server, and multi-colony founding
are done and live-verified. The Bevy client renders the full top-down world with a cohesive
"cozy ledger" UI kit (one visual language across every panel), and the browser/WASM build runs
end-to-end in Chromium. The one remaining step is the **cutover** (P11: retiring the frozen
TypeScript reference tree), done at merge time. See `docs/HANDOFF.md` for the living status and
`docs/migration/BOARD.md` for phase-by-phase detail. No CI/CD pipeline — tests enforced locally
via lefthook.
