# Testing Guide

This document defines the maintained Rust/Bevy test workflow. The former TypeScript/Vitest game
is frozen on `archive/web-game`; do not use its test commands on `main`.

## Behavior-change discipline

Every production behavior change starts with the smallest focused failing test. Record the red
result, implement the behavior, make the focused test green, and then add or update the composed
chain that owns the user-visible consequence. Never delete, ignore, weaken, or broaden an assertion
to accommodate broken behavior. Test-only infrastructure, documentation, and CI maintenance do not
need a preceding behavioral red test, but still need proportional validation.

## Test tiers

### Focused regression — local

Run one exact test or integration binary while developing:

```bash
cargo nextest run -p cat-client focused_start_input_has_a_visible_cursor
cargo nextest run -p cat-client visible_command_registry_emits_the_expected_protocol_manifest_slice
cargo nextest run -p cat-sim regular_cat_walking_permanently_reveals_a_three_by_three_trail
cargo nextest run -p cat-server real_socket_auth_tick_save_restart_and_reconnect_is_deterministic
cargo nextest run -p cat-server real_socket_rate_limit_rejects_the_bounded_burst_and_recovers
```

Simulation changes also need a deterministic twin, conservation invariant, or exact boundary
assertion where appropriate.

### Stable smoke profile — local before commit

```bash
cargo nextest run --workspace --profile smoke
cargo clippy -p <touched-crate> --all-targets -- -D warnings
cargo fmt --all -- --check
```

The smoke filter in `.config/nextest.toml` is intentionally small and runs with two test threads.
Do not routinely run the full workspace suite locally. Long simulation campaigns, catalog sweeps,
and multi-seed playtests belong to the capped remote tier. When Forgejo reports a failure, reproduce
that exact test locally.

### Quick Forgejo gate — generic runner

For each push or pull request, `.forgejo/workflows/quality.yaml` cancels an obsolete run for the
same ref. Its `personal` quick job has a 45-minute timeout and uses `CARGO_BUILD_JOBS=2` for formatting,
dependency policy, workspace Clippy, complete test-inventory compilation, and the stable smoke
profile. The browser/WASM build begins after quick succeeds and is independent of whole-game
expectation failures.
The WASM job also enforces the maintained 12 MiB gzip transfer ceiling and uploads its measured
bundle size.

### Complete Forgejo gate — dedicated capped runner

After quick succeeds, one `cat-idler-heavy` job runs:

```bash
cargo nextest run --workspace --profile ci
```

There are no test archives or static hash shards. Nextest dynamically schedules one complete,
unpartitioned inventory with `test-threads=2` and `fail-fast=false`. Framebuffer/singleton tests use
the `singleton` serial group instead of forcing every test to one thread. The Kubernetes runner has
capacity one; its job containers are capped at 2 CPUs and 5 GiB, and the backing DinD sidecar at
2 CPUs and 6 GiB. The job timeout is 150 minutes, while the `ci` profile terminates any single test
that runs for 30 minutes.

The complete run always uploads JUnit, timing output, peak process-resource measurements, and
`target/playtest-traces/*.json`. A failing
scenario trace identifies its stable scenario/seed, last completed milestone, simulated time,
observed action results, projected cats/jobs/inventory/fog/events, and any restart difference.
Gameplay expectation tests remain normal, unignored tests and must pass with the rest of the full
gate. A red journey is a regression to diagnose, not an accepted baseline.

## Whole-game playtest contract

`cat-server` has a test-only `WsGameHarness` that binds the real Axum app to `127.0.0.1:0`, uses a
temporary SQLite database and normal HMAC/presence/session handling, disables the automatic ticker,
and advances deterministic monotonic time through the authoritative tick path. Fixture mutation is
allowed only before the listener starts. Once live, scenarios use WebSocket actions, action results,
ticks, and projected snapshots. The harness supports explicit save, shutdown, restart, reconnect,
and persistence comparison. Tick advancement drains any older action-triggered projection before
returning, so a milestone can never mistake a queued pre-tick snapshot for the tick it requested.

`playtest_scenarios` records stable IDs, design-document anchors, setup, trigger, ordered bounded
milestones, allowed chance outcomes, horizons, seed tiers, and persistence checkpoints. Catalog
sweeps aggregate errors so one bad option does not hide later entries. The catalogs and the `Full
playtest matrix` in `docs/IMPLEMENTATION_AUDIT.md` are the coverage contract; raw test count is not.
The current executable guards cover all 24 constructible buildings, 108 station recipes, three
crops, six finite-deposit/biome pairs, 32 resource kinds, 450 item kind/material/quality variants,
20 job kinds, 487 research studies, and 19 worker skills. Count assertions deliberately fail when a
typed catalog changes without corresponding journey coverage.
The action contract sends every public variant, every malformed counterpart, and every
invalid-authentication counterpart through a real socket without discarding rejected results.
Production queue and exact work-slot operations are likewise applied and checked in projected
state. The client command-registry guard serializes every visible non-inspect dock command and
compares the resulting action-tag set with its maintained protocol slice.

Direct integration contracts additionally pin the communal 30-adult workforce assignment after a
real world tick, physical survival-maintenance completion and renewal, and a signed shrine ritual
through blessing delivery and restart. Client tests exercise production right-click hit detection
across a building's entire footprint, deterministic cycling through stacked world targets, Den
roof/floor composition, and the actual animation-atlas frames selected for multiple moving cats.
These are ordinary expectations: a production gap is reported as a failing test, never ignored.

Seed cohorts are fixed:

- primary: `4242`;
- high risk: `7`, `42`, `99`, `4242`, `0xCA97_A111`;
- nightly: `0..=27`, `42`, `99`, `4242`, `0xCA97_A111`.

Lifecycle and conservation invariants apply to every selected seed. Chance-driven journeys assert
an explicit allowed result set and bounded physical completion, not one lucky outcome or an exact
completion tick.

## Scheduled and manual workflows

- Nightly at `08:30 UTC`: the 32-seed scenario cohort under `--profile nightly`, with a 180-minute
  job limit and a 120-minute per-test cap.
- Sunday at `10:30 UTC`: LLVM coverage, 230-minute limit.
- Manual dispatch: `quality.yaml` for normal full, `nightly-playtests.yaml` for the extended cohort,
  and `weekly-coverage.yaml` for coverage.

The one-capacity heavy runner serializes scheduled and manual work. Nightly and coverage runs keep
separate JUnit directories. Coverage always publishes JSON, LCOV, HTML, JUnit, timing, and traces,
even after test failures. The first measured run establishes
cached per-crate line baselines; later runs fail when a crate regresses by more than 0.5 percentage
points.

## Normal development sequence

1. Add the smallest focused failing test and retain its red result.
2. Implement the behavior and make that test green.
3. Add or update the composed causal-chain test.
4. Run focused tests, the smoke profile, touched-crate Clippy, and formatting locally.
5. Push and let the newest-ref quick/full/WASM workflow run remotely.
6. Reproduce only reported failures locally; preserve all assertions and push the correction.

## Inspection commands

```bash
cargo nextest list --workspace
cargo nextest list --workspace --profile smoke
fj actions tasks -R origin
```
