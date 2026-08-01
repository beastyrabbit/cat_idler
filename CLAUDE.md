# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Idle Cat Forest** — "an idle version of Dwarf Fortress, played by cats, in a forest." A
top-down, single-level god-sim: a cat colony lives, works, breeds, ages, researches, and
fights on its own, driven by an authoritative server that ticks the simulation once a second
whether or not anyone is watching. Players are gods, not routine controllers: broad nudges,
God-lane research/preparation, temporary divine aid, one election backing block, personal stance,
and authorized expulsion are allowed; exact work/sites/buildings/storage/food/officers remain AI-owned.

Built as a Rust **Cargo workspace** under `crates/`:
- **cat-sim** — pure, deterministic simulation core (no I/O). `world_tick()` is the single
  source of truth; LAI.46/63 own the final integrated phase order per colony.
- **cat-protocol** — `serde` wire DTOs: the wider-world snapshot plus the final report-safe,
  authenticated, expected-versioned integrated snapshot/action surface.
- **cat-server** — tokio + axum authoritative server: runs `world_tick` for every colony once
  a second, broadcasts snapshots over WebSocket, persists to SQLite (`rusqlite`).
- **cat-client** — Bevy 0.19 renderer/UI (library, native + wasm-targetable), top-down.
- **cat-desktop** / **cat-web** — thin native / wasm launcher binaries over `cat-client`.
- **cat-dev** — `cargo dev`, a local launcher that builds + runs `cat-server` + `cat-desktop`
  together.

### Leader-intelligence status

The first leader planner is historical baseline. The active LAI.35–70 integration covers
report-limited planning, exact spatial tasks, skills/families/governance, staged construction/
storage, the Hole, Notes/Void two-lane research, divine policy, care, diplomacy/barter, fresh
persistence, server routing, and the five-screen Bevy surface. Read
`docs/leader-ai-overhaul/README.md` and its additive `BOARD.md` before changing them. Use
`extending-the-system.md` for all current new-content/system recipes,
`diagnostics-and-debugging.md` for bounded traces, and `browser-playtests/README.md` for the signed
Portless/Playwright journey.

LAI.34 is not final acceptance for the new plans. Shrine/Favor/Blessings, generic stored Food/Fish/
Preserves, scholar Insight, coins, direct routine base-world controls, semantic gameplay migration,
duplicate research/protocol/UI authorities, and the old navigation are deletion targets. Consult
the board for exact per-card status; do not claim campaign/browser acceptance before LAI.69–70.

**The platform rebuild, cutover, and maintained P12–P19 product contract are complete — this tree
is now the Rust/Bevy game.** The game originally shipped
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
cargo nextest run -p cat-sim <focused-filter>  # smallest relevant regression
cargo nextest run --workspace --profile smoke  # maintained local cross-crate gate
cargo test -p cat-sim <focused-filter>         # focused fallback

# Quality (must be green before any commit, per AGENTS.md)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check         # `cargo fmt --all` to fix

# Browser release bundle (live-verified; same-origin production image uses the Dockerfile)
scripts/build-web.sh
```

The TypeScript toolchain (`bun run dev`, `bun run db:generate`, `bun run test`, etc.) is gone
from this tree — it only ever drove the old web game, which now lives on branch
`archive/web-game`. Check that branch out if you need it.

## Architecture

```
cat-sim::world_tick() ─▶ WorldSnapshot + report-safe LAI.24 ─▶ cat-server ─▶ cat-client
        ▲                                                        │
        └──── pure mutation adapters ◀── authenticated/versioned LAI.25 action
```

The server is the single source of truth. Clients hold no authoritative state: the wider world
renders from `WorldSnapshot`, leader panels render from LAI.24, and an action is reflected only
after authoritative mutation and snapshot publication. Legacy Presence bootstraps identity;
authenticated non-overlapping base-world frames remain supported, while retired
leader/progression frames are rejected. No client-side prediction.

### The tick

- **`world_tick(&mut WorldState, now_ms) -> Vec<TickReport>`** (`crates/cat-sim/src/world_tick.rs`)
  is the single entry point, called once per colony per second by `cat-server`'s tokio loop.
  It runs 53 ordered phases (`fn phase_*`) — life sim → consumption/spoilage →
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
| World generation | `noise`, `terrain_gen`, `world_gen`, `biomes`, `climate`, `village_sites` |
| Cat AI/planning | `pathfinding`, `movement`, `policy`, `tasks`, `cat_ai`, `planner_core`, `beliefs`, `intent_graph`, `leader_planner`, `officer_expertise`, `officer_requests`, `scheduler`, `workforce_matcher`; old `leader_ai`/`leader_director` are compatibility history |
| Life sim | `needs`, `age`, `breeding`, `genetics`, `life_sim`, `survival` |
| Economy/village | `food_ecology`, `cookhouse`, `fishing`, `quality_lots`, `material_crafting`, `physical_storage`, `construction_stages`, `village_infrastructure`, plus integrated production/hauling adapters |
| Military/family/governance | `hunting_lair`, threat/combat leaves, `cat_capabilities`, `family_specialization`, `family_housing`, `cat_governance`, officer leaves |
| Hole/progression/divine/barter | `black_hole`, unified progression over research/scholar/boost leaves, `food_divine_policy`, `diplomacy`, `moneyless_barter`; `favor`/`shrine_offerings` are deletion targets |
| Orchestration | `world_tick` (the tick loop), `actions` (pure `apply_action` + `build_snapshot`) |

Each module's doc comment cites the original TypeScript file it was ported from (e.g.
`leader_director.rs` ← `lib/game/leaderDirector.ts`) — the fastest way to find the ported
behavior's original spec/tests. Design detail beyond this table (leader director response
curves, pathfinding cost model, world_tick phase list, per-phase P12–P19 gameplay specs) lives
in `docs/migration/specs/` — read the relevant spec before touching a system, don't rely on
memory of the old TS code.

The canonical research graph derives its finite total from content, contains fourteen level-1–10
tracks, repeatable level 11+, and real AND/convergence junctions. Ordinary scholar work creates
Notes; Hole feeds create Void. The God lane queues physical work/preparation; the free Leader lane
uses its rolling seven-day quota and normally avoids the funded God target.

**Leader and officers.** The persistent founding Leader plans across all essential domains from
beliefs and report-gated knowledge. Steward, Accountant, Forester, Farmer, Captain, Loremaster,
and Cloth Leader own specialist reviews and structured requests; expertise changes cadence,
candidate breadth, omission, and report detail. `docs/LEADER_AI_DESIGN.md` and the bounded utility
director are historical provenance. Current authority and verification live in
`docs/leader-ai-overhaul/`.

### Historical P12–P19 product status

Do not maintain a second detailed backlog in this file. The evidence-backed status source is
[`docs/IMPLEMENTATION_AUDIT.md`](docs/IMPLEMENTATION_AUDIT.md), and the concrete correction queue is
[`docs/FIX_LOG.md`](docs/FIX_LOG.md). In summary, seven-role manual-to-officer ownership, the
  15-adult/three-Den founding lifecycle, spatial stockpiles, physical farming/fishing, ten processor
  types with 108 physical recipes, the 487/487 live-study ledger, global/personal village routing,
  shared terrain and physical trade, exact roads/rail/shipping, all 25 building compositions, and
  native/optimized-WASM Adventure UI campaigns are live. Twelve `Crews` studies add real concurrent
  station slots; thirteen others are completed-building-scoped services rather than fake slots.
  Treat this as the pre-LAI baseline. The overhaul board governs the current Plan 1+2
  implementation and its still-open final gates.

### cat-protocol — the wire contract

One dependency (`serde`). The wider world uses `WorldSnapshot`/`ColonySnapshot`. Leader
intelligence uses report-safe LAI.24 snapshot envelopes and authenticated expected-versioned
LAI.25 action envelopes with typed conflicts and idempotent outcomes. The broad legacy
`ClientAction` enum remains for Presence, compatibility auditing, and pure adapters while old
controls are migrated; it is not a supported second production action protocol.
Field names are `camelCase` on the wire (matching the old TS API shape where it still matters).
`WorldSnapshot.protocolVersion` is serialized first. Increment `PROTOCOL_VERSION` before any
change that can make an older client reject a nested snapshot; the client then keeps its last
frame visibly stale and reports `UPDATE REQUIRED` instead of presenting a frozen live world.
Wire collection counts/indices use fixed-width integers, not target-width `usize`.

### cat-server — the authoritative server

`GET /health` liveness probe; `GET /ready` stateful readiness probe; `GET /ws` WebSocket upgrade
(each connection performs header-first protocol checks, bootstraps Presence, routes LAI.25
envelopes through ownership/version/idempotency guards, and forwards `WorldSnapshot` plus LAI.24).
A `tokio::spawn`ed loop schedules `world_tick` once per second
for the whole world (fixed `Duration::from_secs(1)` in `main.rs`). Simulation, snapshot building,
and synchronous SQLite work run on Tokio's blocking pool. New sockets clone a
startup-initialized last-completed snapshot; save ticks clone completed world state and release
the authoritative lock before disk I/O; missed intervals skip rather than burst. The server
broadcasts completed state, saves every 5 ticks, and saves once on graceful shutdown. Socket
state binds signed identity and selected-colony routing, while each snapshot still contains the
complete shared world. Identity (`identity.rs`) issues/verifies timestamped HMAC-signed v2 sessions
whose stable player token preserves village ownership across rotation. Ordinary action access
expires after 30 days; authentic legacy or expired credentials have a seven-day renewal window.
The development secret is loopback-only unless explicitly opted in;
public binds require both `SESSION_HMAC_SECRET` and an exact Origin allowlist. Per-session and
per-IP action, connection, issuance, village, and total-world caps bound abuse. Forwarding headers
are ignored unless the TCP peer is in `CAT_SERVER_TRUSTED_PROXY_IPS`, in which case exactly one
valid client IP is required.

### cat-client / cat-desktop / cat-web — the renderer

Bevy 0.19 library (`cat_client::run()`) shared by native (`cat-desktop`) and wasm (`cat-web`).
Connects to `cat-server` over WebSocket via `ewebsock`, deserializes `WorldSnapshot` on
receipt, stores it as a Bevy resource that render/UI systems read each frame. **Top-down**, not
isometric — a deliberate design pivot mid-migration (`docs/GAME_VISION.md`). Draws biome
terrain generated client-side from the shared `world_seed` (via `cat_sim::generate_terrain_chunk`
— no need for the server to stream tile data), fog of war, paved roads, cats (shape-and-color
specialization/officer badge, carrying marker), label-free roofed homes and typed open stations,
stockpiles/gather spots, crop stages, raiders, and zone overlays. The HUD covers resources,
census, events, trade, officers, persistent village selection, and inspectors. A full-page
  integrated client target is exactly Log/Stores/Village/Research/Council with six Council tabs.
  Research presents the canonical graph, physical God queue/preparation, free Leader lane,
  Notes/Void, permits, repeatables, and boosts. It never restores `research_ui.rs`.

Art: curated pixel sprites under
`public/images/game/{terrain,nature,buildings,interior,infra,props,farm,enemies}/` plus the
accepted cat/raider sheets under `public/images/cats/` — see `docs/assets/SELECTION.md` for the
runtime mapping. Bevy-specific gotchas (camera Z-layering —
keep the camera at Z~1000, sprites below it or they get clipped/black-screened; `Sprite`/`Text`
API shapes; asset-root resolution via `BEVY_ASSET_ROOT`) are documented in `docs/HANDOFF.md` —
read it before touching client rendering code. Logic/UI-shape tests supplement visual checks;
Bevy rendering is verified by capturing the client's own framebuffer to a PNG and reading
it back (method in `docs/HANDOFF.md`), since "it compiles" has previously hidden a black-screen
regression.

## Persistence (cat-server)

`crates/cat-server/src/persistence.rs` uses `rusqlite` (bundled SQLite, not Drizzle) with
tables mirroring the old TS schema (`world`, `colonies`, `cats`, `jobs`, `buildings`,
`world_tiles`, `shared_world_tiles`, `events`, `zones`, `elections`, `votes`, `raiders`). Colony resources are stored
as a JSON blob rather than one column per resource. Migrations are idempotent `ALTER
TABLE`/`ADD COLUMN`-style statements applied on open — same "migrate on connect" discipline as
the old `db/client.ts`, but hand-written Rust rather than generated SQL files (there is no
`db:generate` equivalent; add a new column by adding an `ADD COLUMN` migration statement plus
the corresponding struct field and read/write code).

## Testing Contract

- **`cat-sim`** is pure and deterministic (no `std::time`, no threads, no `rand`): 770+ pure
  unit/integration tests plus golden-master fixtures under
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
  window, and sustains a mean population at or above the 15-cat founding count — a deliberately
  harsher proxy than the live 1s tick, so passing it is a conservative sustainability guarantee.
  Housing work must additionally prove three five-bed Dens, pregnancy bed reservations,
  36-game-hour migrant probation/retention/departure, deterministic atomic reset, and real
  fetch/carry/deposit water recovery. Current old-age thresholds are deliberately 240 game-hours
  for ordinary cats and 288 for leaders/healers; archived 48/57.6-hour fixtures are history.
- **`cat-protocol`**: serde round-trip tests (serialize → deserialize → equal).
- **`cat-server`**: integration tests spin up the axum app in-process (no real socket needed)
  and drive current LAI.25 envelopes through the signed router; compatibility tests separately
  prove Presence bootstrap and old-client `UPDATE_REQUIRED` behavior.
- **`cat-client`**: logic/UI-shape tests plus manual own-framebuffer verification (see above).
- Any new simulation constant/limit needs a boundary test in the owning `cat-sim` module, same
  discipline the old TS project enforced.

Quality gate before any commit (per `AGENTS.md`): the smallest focused regression plus
`cargo nextest run --workspace --profile smoke`,
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
  `docs/ENGINE_FRONTEND.md`, `docs/TASKS.md`, `docs/UI_CONCEPTS.md`) are
  marked superseded and kept only as design-history reference — they don't describe how to
  build, run, or test this project anymore.
- `docs/TESTING.md` is maintained and authoritative for the Rust/Bevy local-smoke/remote-full
  workflow.

## Key Documentation

- `README.md` — quick start, environment, testing/determinism summary
- `docs/ARCHITECTURE.md` — the Rust workspace: crates, the tick loop, protocol, persistence,
  client rendering (start here for anything architectural)
- `docs/GAME_VISION.md` — design pillars for "Idle Cat Forest" (manual → role-automation,
  visible workplaces, production chains)
- `docs/HANDOFF.md` — migration status + hard-won Bevy/codex build lessons
- `docs/IMPLEMENTATION_AUDIT.md` — authoritative shipped/follow-up status and verification matrix
- `docs/FIX_LOG.md` — reproduced correction queue and evidence-backed verified fixes
- `docs/leader-ai-overhaul/README.md` — current planner/progression design, implementation board,
  extension recipes, diagnostics, and browser acceptance contracts
- `docs/TESTING.md` — maintained Rust/Bevy and leader-AI test workflow
- `docs/migration/BOARD.md` — phase-by-phase task board (P0–P9 tracked in detail; later phases
  tracked in `docs/migration/specs/` and the git log)
- `docs/migration/specs/` — design specs for pathfinding, leader director, `world_tick`, and
  the P12–P19 gameplay systems (skills/roles, spatial placement, biome generator, visual
  polish, item economy)
- `docs/migration/WASM.md` — verified browser/production build + optional optimization work
- `docs/assets/SELECTION.md` — sprite-family selection and runtime art mapping
- `AGENTS.md` — ground rules for the codex/Claude build team doing the port (parity discipline,
  determinism rules, the one permitted JS use, commit conventions)

## Status

Pre-release; the Rust/Bevy migration, P11 cutover, and maintained P12–P19 contract are complete. This file intentionally does not
duplicate fast-moving feature status. Use `docs/IMPLEMENTATION_AUDIT.md` for the authoritative
shipped/follow-up matrix and `docs/FIX_LOG.md` for the current correction queue. The Forgejo quality
workflow is committed; its first pushed run remains unverified.
