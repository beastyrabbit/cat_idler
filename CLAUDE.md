# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cat Colony Idle Game — a real-time idle game where a shared cat colony runs autonomously. Players can boost jobs with clicks, but the colony self-sustains. Built with Next.js 16, Convex (real-time serverless backend), and TypeScript.

## Commands

```bash
# Development (run in separate terminals)
bun run convex:dev          # Terminal 1: Convex backend
bun run dev                 # Terminal 2: Next.js frontend (localhost:3000)
bun run dev:worker          # Terminal 3: Worker simulation loop (tsx watch)

# Testing
bun run test                # Run all unit tests once (vitest run)
bun run test:watch          # Vitest watch mode
bun test tests/unit/game/needs.test.ts   # Single test file
bun test -- --grep "pattern"             # Filter by test name
bun run test:coverage       # Coverage report (v8)
bun run test:e2e            # Selenium E2E tests

# Quality
bun run lint                # ESLint
bun run typecheck           # tsc --noEmit
bun run format              # Prettier
```

## Architecture

```
Browser (Next.js + React 19)
  ↕ real-time subscriptions (useQuery/useMutation)
Convex Backend (mutations, queries, schema)
  ↕ calls pure functions
lib/game/ (pure game logic, NO side effects)
  ↑ driven by
worker/index.ts (always-on tick loop via ConvexHttpClient)
```

**The worker drives the game, not Convex crons.** `convex/crons.ts` is intentionally empty. The worker (`bun run dev:worker`) calls `game.workerTick` every 1s (configurable via `WORKER_TICK_MS`).

### Dual Tick Systems

There are two independent tick codepaths:

- **`convex/game.ts:workerTick`** — Browser-idle v2 job system. Handles resource consumption, job completion, auto-queuing hunts/builds/rituals, specialization XP, critical state, and colony resets. Called by the worker process.
- **`convex/gameTick.ts:tickColony`** — Legacy system. Handles needs decay, autonomous cat AI, task assignment/progression, combat encounters, aging/death, breeding. Called as `internalMutation`.

### Key Layers

- **`lib/game/`** — All game logic as pure functions. No DB imports, no side effects. This is the core that gets unit tested.
- **`convex/`** — Mutations/queries that read/write DB and call into `lib/game/`. The `game.ts` file handles initialization, leader assignment, and the `workerTick` entry point.
- **`types/game.ts`** — All shared TypeScript types and constants (Cat, Colony, Building, Task, Encounter, etc.)
- **`worker/index.ts`** — Lightweight Node process that calls `game.workerTick` on an interval. Needs `CONVEX_URL` or `NEXT_PUBLIC_CONVEX_URL`.

### Browser Idle v2 Job System

The game operates on a **job-based system** (`convex/schema.ts:jobs` table, `lib/game/idleEngine.ts`):

- Short player actions: `supply_food` (20s), `supply_water` (15s)
- Long cat jobs: `hunt_expedition` (8h), `build_house` (8h), `ritual` (6h)
- Leader planning: `leader_plan_hunt` (30min), `leader_plan_house` (20h)
- Cat specializations reduce relevant job durations (50% for hunter/architect, 40% for ritualist)
- Click boosting reduces active job time (diminishing returns above 30 clicks/min)
- Global upgrades persist across colony resets (`globalUpgrades` table)
- Colonies auto-reset after extended critical state (configurable via `testCriticalMsOverride`)

### Test Acceleration

`lib/game/testAcceleration.ts` provides QA presets that scale time and resource decay for faster testing. Colony schema has `testTimeScale`, `testResourceDecayMultiplier`, and other override fields.

## Database Schema

11 tables in `convex/schema.ts`: `colonies`, `cats`, `buildings`, `worldTiles`, `tasks`, `encounters`, `events`, `players`, `jobs`, `globalUpgrades`, `runHistory`. Key indexes are defined inline. Cat lookup by colony uses `by_colony` and `by_colony_alive` (filters on `deathTime`).

## Testing Patterns

- **Vitest** with jsdom environment, globals enabled (no imports needed for `describe`/`it`/`expect`)
- Test factories in `tests/factories/` for building test data
- Path alias `@/` maps to repo root (matches tsconfig)
- Coverage targets: game logic 90%+, components 80%+
- TDD workflow: write failing tests first, then implement

## Git Hooks (lefthook)

- **pre-commit**: gitleaks (secret detection), lint, typecheck — all run in parallel
- **pre-push**: unit tests must pass

## Environment

Copy `.env.local` from an existing setup or create with:

```
CONVEX_DEPLOYMENT=dev:<your-deployment>
NEXT_PUBLIC_CONVEX_URL=https://<your-deployment>.convex.cloud
```

The worker reads `CONVEX_URL` or `NEXT_PUBLIC_CONVEX_URL`. In worktrees, `.env.local` may not exist — check before starting servers.

## Gotchas

- `convex/game.ts` uses `type Ctx = any` to bypass Convex's strict handler typing — this is intentional, not a code smell
- `bun run build` uses `next build --webpack` (not the default Turbopack)
- The `convex/` directory has auto-generated files in `_generated/` — never edit those
- Next.js dev server may already be running from another worktree or terminal — check `ss -tlnp | grep 300` before starting. If port 3000 is taken, kill the process or use `PORT=3002 bun run dev`
- If `next dev` fails with "Unable to acquire lock", delete `.next/dev/lock` first
- Convex backend (`bun run convex:dev`) must be running for the game UI to load data — without it, the page shows "Preparing Global Colony..."
- Greptile MCP `addressed: false` may persist even after GitHub review threads are resolved — verify with `pull_request_read get_review_comments` (check `IsResolved` field) as the source of truth

## Key Documentation

- `docs/plan.md` — Full game design document with architecture diagrams
- `docs/TASKS.md` — Development tasks with TDD instructions
- `docs/TESTING.md` — Testing guide, patterns, and mocking strategies

## Releases

- First release: `v0.1.0` (pre-1.0, semver)
- No CI/CD pipeline — tests enforced locally via lefthook
