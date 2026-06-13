# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cat Colony Idle Game — a real-time idle game where a shared cat colony runs autonomously. Players can boost jobs with clicks, but the colony self-sustains. Built with Next.js 16, Drizzle ORM + SQLite (better-sqlite3), and TypeScript.

## Commands

```bash
# Development (run in separate terminals)
bun run dev                 # Terminal 1: Next.js frontend (localhost:3000)
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

The game operates on a **job-based system** (`db/schema.ts:jobs` table, `lib/game/idleEngine.ts`):

- Short player actions: `supply_food` (20s), `supply_water` (15s)
- Long cat jobs: `hunt_expedition` (8h), `build_house` (8h), `ritual` (6h)
- Leader planning: `leader_plan_hunt` (30min), `leader_plan_house` (20h)
- Cat specializations reduce relevant job durations (50% for hunter/architect, 40% for ritualist)
- Click boosting reduces active job time (diminishing returns above 30 clicks/min)
- Global upgrades persist across colony resets (`globalUpgrades` table)
- Colonies auto-reset after extended critical state (configurable via `testCriticalMsOverride`)

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
- Any new simulation constant/limit must include boundary tests in `tests/unit/game/`.

## Frontend

- Production UI: `app/game/newspaper/page.tsx` (The Catford Examiner — broadsheet newspaper theme). `/game` redirects there.
- Shared game hook: `hooks/useGameDashboard.ts` — subscribes to the SSE stream and exposes actions; all UI variants import this for game state, actions, and session management
- README screenshots: `docs/screenshots/` — referenced by relative path from README.md
- 13 UI concept variants documented in `docs/UI_CONCEPTS.md` (archived on `archive/ui-concepts-all` branch)
- Subscriber identity: `app/api/subscriber-hash/route.ts` — IP-based anonymous hash, salt via `SUBSCRIBER_HASH_SALT` env var

## Database Schema

9 tables in `db/schema.ts`: `colonies`, `cats`, `buildings`, `worldTiles`, `events`, `players`, `jobs`, `globalUpgrades`, `runHistory`. Rows keep the `_id` property (TEXT nanoid, mapped to the `id` column) so API payloads match what the frontend expects. The legacy `tasks` and `encounters` tables were dropped with the retired `/colony/[id]` UI.

After editing `db/schema.ts`, run `bun run db:generate` and commit the new migration file — `db/client.ts` runs pending migrations on open.

## Testing Patterns

- **Vitest** with jsdom environment, globals enabled (no imports needed for `describe`/`it`/`expect`)
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
- Next.js dev server may already be running from another worktree or terminal — check `ss -tlnp | grep 300` before starting. If port 3000 is taken, kill the process or use `PORT=3002 bun run dev`
- If `next dev` fails with "Unable to acquire lock", delete `.next/dev/lock` first
- The worker (`bun run dev:worker`) must be running for the simulation to advance — without it the UI loads but nothing ticks
- `better-sqlite3` is a native module: route handlers using it must run in the Node runtime (default for route handlers; do not mark them `edge`)
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
