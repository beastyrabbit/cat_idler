# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cat Colony Idle Game — a real-time idle game where a shared cat colony runs autonomously. Players can boost jobs with clicks, but the colony self-sustains. Built with Next.js 16, Drizzle ORM + SQLite (better-sqlite3), and TypeScript.

## Commands

```bash
# Development (run in separate terminals)
bun run dev                 # Terminal 1: Next.js frontend via portless (prints a *.localhost URL — port varies)
bun run dev:url             # Print the current worktree's dev URL without starting the server
bun run dev:worker          # Terminal 2: Worker simulation loop (tsx watch)

# Database
bun run db:generate         # Generate SQL migration after editing db/schema.ts
bun run db:studio           # Drizzle Studio (DB browser)

# Testing
bun run test                # Run all unit tests once (vitest run)
bun run test:watch          # Vitest watch mode
bun test tests/unit/game/needs.test.ts   # Single test file
bun test -- --grep "pattern"             # Filter by test name
bun run test:coverage       # Coverage report (v8)
bun run test:e2e            # Selenium E2E tests

# Quality
bun run lint                # biome + ESLint
bun run typecheck           # tsc --noEmit
bun run format              # Prettier
```

## Architecture

```
Browser (Next.js + React 19)
  ↕ SSE stream (/api/game/stream) + POST actions (/api/game/actions)
Next.js route handlers (app/api/game/*)
  ↕ calls server logic
server/ (game orchestration over Drizzle)  +  db/ (schema, client, migrations)
  ↕ calls pure functions
lib/game/ (pure game logic, NO side effects)
  ↑ driven by
worker/index.ts (always-on tick loop, writes SQLite directly)
```

**The worker drives the game.** The worker (`bun run dev:worker`) calls `server/game.ts:workerTick` every 1s (configurable via `WORKER_TICK_MS`). The web server and worker are separate processes sharing `data/game.db` — WAL mode + busy_timeout handle concurrency.

### Tick System

- **`server/game.ts:workerTick`** is the single source of truth for simulation.
- Do not introduce parallel tick paths (no crons, no per-request ticking).

### Key Layers

- **`lib/game/`** — All game logic as pure functions. No DB imports, no side effects. This is the core that gets unit tested.
- **`db/`** — Drizzle schema (`schema.ts`), client factory (`client.ts`, WAL + migrations on open), and generated SQL migrations (`migrations/`).
- **`server/`** — Functions that read/write the DB and call into `lib/game/`. `game.ts` handles initialization, leader assignment, jobs, upgrades, and the `workerTick` entry point. All synchronous (better-sqlite3); mutations wrapped in transactions.
- **`app/api/game/`** — Route handlers: `stream` (SSE, pushes the dashboard once per second), `actions` (POST `{action, ...payload}`), `dashboard` (one-shot GET).
- **`types/game.ts`** — All shared TypeScript types and constants (Cat, Colony, Building, etc.)
- **`worker/index.ts`** — Lightweight Node process that calls `workerTick` on an interval. Opens the DB itself; no env needed (optional `GAME_DB_PATH`).

### Browser Idle v2 Job System

The game operates on a **job-based system** (`db/schema.ts:jobs` table, `lib/game/idleEngine.ts`). Job `kind`s:

- Short player actions: `supply_food` (20s), `supply_water` (15s)
- Long cat jobs: `hunt_expedition` (8h), `build_house` (8h), `ritual` (6h), `quarry` (materials), `explore` (scout/reveal fog), `fetch_water` (colony water economy), `train_warrior` (barracks graduation)
- Leader planning: `leader_plan_hunt` (30min), `leader_plan_house` (20h)
- Cat specializations reduce relevant job durations (50% for hunter/architect, 40% for ritualist; `warrior` is the fourth specialization)
- Click boosting reduces active job time (diminishing returns above 30 clicks/min)
- Global upgrades persist across colony resets (`globalUpgrades` table)
- Colonies auto-reset after extended critical state (configurable via `testCriticalMsOverride`)

### Map-First God-Sim (all on the single `workerTick` path)

The game is a self-running map simulation: cats live, age, breed, work, research, fight, and die on their own; players are gods who nudge via zones, boosts, votes, roads, and a slow upgrade tree. `workerTick` runs ~30 ordered phases each tick (life sim → consumption → elections → jobs → leader director → assignment → production → research → survival → job completion → hauling → movement → road paving → raids → status). Do not add parallel tick paths.

- **Leader director** (`lib/game/leaderDirector.ts`, per `docs/LEADER_AI_DESIGN.md`): a pure utility AI (IAUS-style) that replaced the hand-ordered `leaderAI.ts` rule list. Scores every colony goal on one [0,1] scale from response curves over a `LeaderSnapshot` (deficit / projection / pressure curves + opportunity vetoes), then hands a shared employment budget (~0.7 of stage-weighted workforce, idle floor 0.8) to the highest-scoring goals. Emits labour goals (`hunt`, `fetch_water`, `quarry`, `scout`, `train_warrior`, `assign_workshop`, `assign_research`, `assign_smithy`) plus standalone decisions (`cancel_hunts`, `build_storage`, `build_den`, `tithe`). `matchCatsToSlots` is a deterministic greedy best-cat-per-slot matcher (skill fit × 1.5 specialization bonus, ties by stable cat id). The seeded policy-reliability roll stays at the executor call site.
- **Movement & pathfinding** (`lib/game/movement.ts`, `lib/game/pathfinding.ts`): cats travel to job sites, wander when idle; movement randomness runs on a forked seeded chain so policy-roll determinism is preserved. Cats walk *every* tile (no fog teleports); travel wears paths and reveals a fog halo (3x3, explorers 5x5). Pathing is bounded 4-directional A* over a `WalkGrid` — rivers and the palisade fence block (except the single gate), roads are cheap; a straight L-walk fast-path skips A* when unobstructed; failure falls back to a straight walk.
- **Roads** (`lib/game/roads.ts`): `selectRoadCorridor` deliberately paves the highest cumulative-wear trafficked corridor outside the fence when materials are spare (wear ≥ 70). Paved roads are exempt from path-wear decay and cheap to traverse.
- **Life simulation** (`lib/game/lifeSim.ts`, `age.ts`, `breeding.ts`, `genetics.ts`): aging through life stages in game-hours (kitten 0–6 / young 6–24 / adult 24–48 / elder 48+; kittens can't work, elders 0.7 weight), old-age mortality hazard, and breeding with lineages. Conception needs food+water above 0.35 capacity and population under housing cap; gestation 6 game-hours; stats blend 60/40 from parents with ±8 mutation (deterministic), visual traits inherit 50/50. `cats.ageHours` tracks the accelerated clock (responds to `advanceTime`). Population is a loop, not a fixed 20.
- **Upgrade tree & research** (`lib/game/upgradeTree.ts`): one tech tree, ~18 nodes across 3 eras (cost 5–25), persisted on the colony (`colonies.upgradeTree`, survives run resets). Two advancement paths — gods spend blessings for instant purchases; cats accrue research points from staffed research huts + schools (~10 pts/researcher/week) and auto-unlock the cheapest affordable node. Nodes unlock buildings (research_hut, sawmill, smithy, barracks, school, field), jobs, and effect multipliers (`resolveEffects` → flat modifier map the tick consumes).
- **Military, threat & raids** (`lib/game/threat.ts`, `server/raids.ts`, `lib/game/warriors.ts`, `smithy.ts`, `combat.ts`): threat pressure builds per game-hour after a 6h grace window from wealth/population/warriors/age; a raid launches at pressure 100 (HUD bands calm/rising/imminent). Smithies (staffed, 15min cycle) forge 2 refined + 3 materials → 1 weapon + 1 armor into the armory; warrior-specialized cats (and hunters, worse) muster to defend, drawing gear at raid time. `runRaidDirector` marches the warband to the gate on a forked roll chain and resolves combat there — loss loots stores and can cost a defender; a wipeout resets the run. Player defense clicks bank against the active raid (`colonies.raidClicks`).
- **Production, hauling & storage** (`lib/game/production.ts`, `storage.ts`, `shrine.ts`, `trips.ts`): workshops (village level 2+) refine 5 materials → 1 refined/10min; fields (level 4+) grow food passively; the leader auto-staffs idle workshops. Storage is per-resource with caps raised by granaries/water bowls/smithy scaled by the tree's `storagePerLevelMult`; resources are clamped to caps each tick. Hunt/quarry/water yields are hauled SC2-drone-style in 3 trips and credited on shrine arrival (Chebyshev radius 1), force-credited after a 60s grace window.
- **Terrain generator** (`lib/game/terrainGen.ts`): pure, deterministic, world-seeded value-noise terrain in 12x12 chunks — continuous elevation/moisture → quantized heights 0–3, biomes (lowland/grassland/forest/rocky/highland), auto-tiled oriented cliffs with stairs on straight runs, monotonic-descent rivers, and scattered decoration. Emits abstract *roles* (no sprite names); a guaranteed flat plateau surrounds the village anchor. Render integration is still in flight (`/dev/tiles` explorer).
- **Construction** (`lib/game/housing.ts`): `build_house` jobs place scaffolds via `nextBuildingSite`; the leader commissions dens when housing pressure ≥ 0.8 (shrine shelters 4, dens 2/level). `villageLevel` gates building unlocks.
- **Elections** (`lib/game/elections.ts`, `server/elections.ts`): NO per-tick leader auto-replace — interim pick only when the seat is empty. Term elections + vote-kick (5 effective identities) resolve in `runElectionLifecycle`. Voter identity requires a verified HMAC session with at least two presence records and 2 minutes of session age; ballots also dedupe on the server-derived salted subscriber/IP hash, so session rotation from one network identity counts once. Accepted caveat: NAT-shared households share one effective ballot, and proxy-pool Sybils are still possible at one warmed identity per distinct subscriber hash.
- **Zones** (`lib/game/zones.ts`, `server/zones.ts`): player-painted avoid/gather rects (max 2/player, 8x8, 10min–2h) steer hunts and wandering.

### Test Acceleration

`lib/game/testAcceleration.ts` provides QA presets that scale time and resource decay for faster testing. Colony schema has `testTimeScale`, `testResourceDecayMultiplier`, and other override fields.

`server/game.ts:advanceTime` is available for deterministic skip-time testing (advance last tick by N seconds). All test controls are reachable via POST `/api/game/actions` (`setTestAcceleration`, `advanceTime`, `setTestRngSeed`) and via the `?test=1` UI controls.

## Testing Contract

- Use deterministic tests for simulation logic:
  - Seed RNG via `setTestRngSeed`
  - Use `advanceTime` and `setTestAcceleration` for time-sensitive scenarios
- Server logic is integration-tested against in-memory SQLite (`createDb(':memory:')`) — see `tests/integration/serverGame.test.ts`. No running backend needed.
- Critical automated scenarios:
  - Water depletion crisis headline/event
  - Cat thirst decay after water depletion
  - Dehydration start, dehydration death, and recovery after water restoration
  - Build-request prerequisite chaining (`supply_water` / material gathering before construction)
  - Upgrade validation (insufficient points, max-level rejection, correct cost progression)
  - Leader-policy tier behavior (`simple`/`normal`/`excellent`) under seeded RNG
  - Leader director cross-axis trade-off (e.g. a water crisis pulls cats off hunting) via `leaderDirector` unit tests
  - Life-sim aging/breeding/mortality, upgrade-tree god-purchase and cat-auto-unlock paths, and raid spawn/resolution
- Any new simulation constant/limit must include boundary tests in `tests/unit/game/`.

## Frontend

- Main UI: `/game` renders `components/map/MapScreen.tsx` — full-screen 2.5D isometric world map (pure projection math in `lib/game/isoProjection.ts`, chunk culling via `visibleChunksIso`). The Catford Examiner newspaper lives at `/game/newspaper` (linked from the map HUD).
- Map art: curated Kenney "Isometric Miniature" sprites in `public/images/iso/` (256x512 bottom-anchored, 256x128 ground diamond). Source pack `public/Kenney Game Assets All-in-1 3.5.0/` is gitignored — copy what you need out of it; standalone tree sprites need a grass `base` underlay (see `TILE_SPRITES`).
- Shared game hook: `hooks/useGameDashboard.ts` — subscribes to the SSE stream and exposes actions; all UI variants import this for game state, actions, and session management
- README screenshots: `docs/screenshots/` — referenced by relative path from README.md
- 13 UI concept variants documented in `docs/UI_CONCEPTS.md` (archived on `archive/ui-concepts-all` branch)
- Subscriber identity: `app/api/subscriber-hash/route.ts` — IP-based anonymous hash, salt via `SUBSCRIBER_HASH_SALT` env var

## Database Schema

13 tables in `db/schema.ts`: `colonies`, `cats`, `buildings`, `worldTiles`, `events`, `players`, `jobs`, `globalUpgrades`, `runHistory`, `elections`, `votes`, `zones`, `raiders`. Rows keep the `_id` property (TEXT nanoid, mapped to the `id` column) so API payloads match what the frontend expects. The legacy `tasks` and `encounters` tables were dropped with the retired `/colony/[id]` UI.

Notable columns added since the map-first rework:
- `colonies`: `upgradeTree` (JSON tech-tree progress, survives resets), `threatPressure` / `lastRaidAt` / `activeRaidId` / `raidClicks` (military), `criticalSince` / `ritualRequestedAt`, plus the `test*` override fields.
- `cats`: `ageHours` / `pregnancyDueAgeHours` / `pregnancyMateId` (life sim), `specialization` and `roleXp` now include `warrior`, `carrying` / `activity` / `destination` (movement + shrine hauling).
- `buildings.type` now covers `mouse_farm`, `shrine`, `workshop`, `field`, `research_hut`, `school`, `smithy`, `barracks` alongside the original den/storage/needs buildings.
- `raiders`: enemy warband units for the active raid — position, target (village gate), strength/hp, and `advancing`/`engaging`/`retreating`/`dead` status.

After editing `db/schema.ts`, run `bun run db:generate` and commit the new migration file — `db/client.ts` runs pending migrations on open.

## Testing Patterns

- **Vitest** with jsdom environment, globals enabled (no imports needed for `describe`/`it`/`expect`)
- ~87 test files: pure `lib/game/` modules unit-tested in `tests/unit/game/`, server orchestration integration-tested against in-memory SQLite in `tests/integration/` (`serverGame`, `serverRaids`, `serverUpgradeTree`, `serverWorldMap`)
- Test factories in `tests/factories/` for building test data
- Path alias `@/` maps to repo root (matches tsconfig)
- Coverage targets (gated): simulation-core modules 99%+
- Avoid flaky assertions in browser E2E for simulation correctness; prefer unit/integration checks on deterministic modules.
- TDD workflow: write failing tests first, then implement

## Git Hooks (lefthook)

- **pre-commit**: gitleaks (secret detection), lint, typecheck — all run in parallel
- **pre-push**: unit tests must pass

## Environment

No environment variables are required for local dev. Optional:

```
GAME_DB_PATH=data/game.db     # SQLite file location (default shown)
WORKER_TICK_MS=1000           # Worker tick interval
SUBSCRIBER_HASH_SALT=...      # Salt for the anonymous subscriber hash
```

The database file is created (and migrations applied) automatically on first open.

## Gotchas

- `bun run build` uses `next build --webpack` (not the default Turbopack)
- `bun run dev` routes through portless (`scripts/portless.mjs`), so the dev URL and port **vary per worktree** — run `bun run dev:url` to print it rather than assuming `localhost:3000`. For raw port-based dev, use `PORTLESS=skip bun run dev`. A stale server may still be running from another worktree — check `ss -tlnp | grep -E '13|30'` before starting.
- If `next dev` fails with "Unable to acquire lock", delete `.next/dev/lock` first
- The worker (`bun run dev:worker`) must be running for the simulation to advance — without it the UI loads but nothing ticks
- `better-sqlite3` is a native module: route handlers using it must run in the Node runtime (default for route handlers; do not mark them `edge`)
- The one-shot `dashboard` and `chunks` GET routes are `export const dynamic = "force-dynamic"` — the live simulation must never be served from Next's static/data cache
- To skip a slow lefthook step on a commit, use lefthook's env exclude, e.g. `LEFTHOOK_EXCLUDE=lint git commit …` (the pre-commit `lint` step runs biome + eslint + typecheck + tests and is the slow one). Prefer fixing over skipping.
- Greptile MCP `addressed: false` may persist even after GitHub review threads are resolved — verify with `pull_request_read get_review_comments` (check `IsResolved` field) as the source of truth
- Repo-wide `bun run biome` currently reports pre-existing errors unrelated to recent work; scope biome checks to your changed files

## Key Documentation

- `docs/plan.md` — Full game design document with architecture diagrams
- `docs/TASKS.md` — Development tasks with TDD instructions
- `docs/TESTING.md` — Testing guide, patterns, and mocking strategies

## Releases

- Latest release: `v0.3.1` — Modernized README with screenshots & badges
- Pre-1.0, semver
- No CI/CD pipeline — tests enforced locally via lefthook
