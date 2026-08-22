# AGENTS.md — shared context for the codex build team

> **Migration complete (P11 cutover, 2026-07-11).** The rebuild described below is done:
> `main` is now the Rust/Bevy game and the TypeScript source has been removed from this tree.
> The old web game (the behaviour spec referenced throughout this doc — `lib/game/*.ts`,
> `server/game.ts`, `types/game.ts`, `db/schema.ts`, `tests/`) lives on branch
> `archive/web-game` (tag `web-final`). Check that branch out to read the original spec or
> regenerate a golden-master fixture. The rest of this file is retained as the porting-era
> ground rules (determinism discipline, parity bar, the one permitted JS use), which still
> govern any further work on `cat-sim`.

> **Permanent project status:** Idle Cat Forest is a non-commercial game project. Treat this as
> established project context rather than an open product or asset-policy question.

This repository contains the completed **Cat Colony** god-sim rebuild from a
TypeScript/Next.js web app into a **Rust + Bevy** system. This file is the shared
ground truth for ongoing work.

## What we're building
One Cargo workspace (`crates/`):
- **cat-sim** — pure, deterministic simulation core. NO rendering, networking,
  filesystem, clock, threads, or `std::time`. `#![forbid(unsafe_code)]`.
- **cat-protocol** — `serde` wire types (world snapshot + client actions).
- **cat-server** — authoritative headless server: runs `cat-sim` for all colonies,
  WebSocket, SQLite persistence, identity/HMAC, rate-limit.
- **cat-client** — Bevy renderer + UI (native + WASM); talks to cat-server.
- **cat-desktop / cat-web** — thin native / wasm entry points over cat-client.

Plan: `~/.claude/plans/ok-then-lets-close-polished-quokka.md`.
Board: `docs/migration/BOARD.md`. Update your card's status when you finish.

## The source of truth for behaviour
The **existing TypeScript game is the spec**: `lib/game/*.ts` (pure rules),
`server/game.ts` (`workerTick`, ~lines 2677-5058), `server/raids.ts`,
`types/game.ts`, `db/schema.ts`. **READ these to port behaviour. NEVER modify
them** — the web game is frozen on branch `archive/web-game`; the TS files remain
only as the reference during migration.

## Non-negotiable rules
1. **Parity, not reinvention.** Reproduce the TS behaviour, constants, and phase
   ordering exactly. When unsure, read the TS + its test in `tests/unit/game/`.
2. **Determinism.** All randomness goes through the ported seeded LCG
   (`seededRng.ts`: MOD 2^32, MUL 1664525, INC 1013904223, `>>>0` overflow) and
   its forked chains (movement +1_000_003, life +2_000_003, raids +3_000_003).
   Reproduce the integer/overflow semantics exactly. No `rand` in cat-sim.
3. **TDD.** Tests exist before implementation. Prefer **golden-master** parity:
   fixtures under `docs/migration/fixtures/` (seed → N ticks → snapshot) generated
   from the TS sim; Rust asserts equality. Behavioural ("same idea") parity is the
   bar — not bit-identical `Math.random` (the TS uses raw `Math.random` in a few
   cosmetic spots; seed those or accept behavioural equivalence, and say which).
4. **Small & scoped.** One card = ≤1 module or slice. Don't touch unrelated files.
5. **Rust only — one exception.** Do not run `bun`, `vitest`, or `tsc`. The SOLE
   permitted JS use: to build a golden-master parity fixture you MAY run a *pure*
   `lib/game/*` module with `npx tsx` (a tiny throwaway script that imports the
   module, samples a seed/coordinate matrix, and writes JSON to
   `docs/migration/fixtures/`). Never modify the TS source. Then write Rust tests
   that assert against the committed fixture. Do not edit anything outside
   `crates/`, `docs/migration/`, `codex/` unless your card says so.
6. **Newest versions.** Add deps with `cargo add` (never hand-edit versions).
   Edition 2024. Bevy 0.19.
7. **Tiered quality gates.** Before commit, run the focused regression test plus
   `cargo nextest run --workspace --profile smoke`, touched-crate Clippy with `-D warnings`,
   and `cargo fmt`. Do not routinely run the complete workspace suite locally: broad aggregate
   and full-suite runs take tens of minutes, especially when failing or timing out. During local
   diagnosis and iteration, run the smallest focused named test or one `CAT_SYSTEM_SCENARIO_ID`
   instead of a broad aggregate or complete inventory. After push, Forgejo must run the full
   workspace inventory once on the capped `cat-idler-heavy` runner with two dynamically scheduled
   test threads and fail-fast disabled. See `docs/TESTING.md`.
8. **Out of scope:** the Catford Examiner newspaper and its ~35 flavor generators
   (horoscope, obituaries, gossip, sports…) are DROPPED. Don't port them. The
   client gets a dashboard + event-log page instead.

## New vs. old scope
The world is **multi-colony**: one shared world owns many colonies; today's single
global colony is colony #1. Design new sim/protocol/server code for N colonies and
player-founded villages from the start.

## Conventions
- Commits: imperative subject, scoped; end the body with the line
  `Powered by human calories and mass GPU cycles.`
- Rust: `snake_case` modules mirroring the TS file names where sensible
  (`seededRng.ts` → `rng.rs`, `leaderDirector.ts` → `leader_director.rs`).
- Put a short module doc comment citing the TS source file you ported from.

## Behavior-change test discipline

1. Every production behavior change starts with the smallest focused failing test.
2. Record the red result before implementing the behavior.
3. Make the focused test green, then add or update the composed causal-chain test.
4. Run focused tests and the smoke profile locally; leave exhaustive work to Forgejo.
5. Never delete, weaken, ignore, or broaden an assertion merely to match broken implementation.
6. Test-only infrastructure, documentation, and CI maintenance are exempt from requiring a
   preceding behavioral test, but still require appropriate validation.
