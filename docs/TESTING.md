# Testing Guide

This document defines the maintained Rust/Bevy test workflow. The former TypeScript/Vitest game
is frozen on `archive/web-game`; do not use its test commands on `main`.

## Test tiers

Testing is deliberately split so development stays responsive while every pushed revision still
receives complete coverage.

### 1. Focused regression test — local

Run the smallest test that proves the behavior being changed. Prefer one exact test name or one
integration-test binary instead of an entire crate:

```bash
cargo nextest run -p cat-client focused_start_input_has_a_visible_cursor
cargo nextest run -p cat-sim regular_cat_walking_permanently_reveals_a_three_by_three_trail
cargo nextest run -p cat-server player_name_history_is_global_and_updates_last_seen
```

New behavior still follows TDD: reproduce the bug or boundary in a failing focused test, implement
the change, then make that test green. Deterministic simulation changes need a deterministic twin
or exact boundary assertion where appropriate.

### 2. Smoke profile — local before commit

Run the maintained cross-crate smoke profile before committing Rust changes:

```bash
cargo nextest run --workspace --profile smoke
```

The profile lives in `.config/nextest.toml`. It covers protocol compatibility plus selected client,
server, persistence, movement, road, fog, and scout regressions. It is capped at two test threads
to keep the workstation usable. `lefthook` runs the same smoke profile on pre-push.

Also run formatting and strict Clippy for the crates touched by the change:

```bash
cargo fmt --all -- --check
cargo clippy -p <crate> --all-targets -- -D warnings
```

Do not run `cargo nextest run --workspace` locally as a routine gate. The long simulation campaigns
belong to the remote tier. A local full-suite run should happen only when a user explicitly requests
it or when diagnosing a failure that cannot be reproduced with a focused test.

### 3. Complete suite — Forgejo after push

Every pushed commit triggers `.forgejo/workflows/quality.yaml`. Forgejo performs the exhaustive gate
on the Kubernetes runner pool:

- format and workspace Clippy run first;
- the complete workspace Nextest inventory is divided into four deterministic hash partitions;
- four `personal` runner jobs execute those partitions in parallel;
- the four partitions must contain every test exactly once;
- the browser/WASM build and dependency-policy check run as independent jobs.

The remote command used by shard `N` is:

```bash
cargo nextest run --workspace --partition "hash:N/4"
```

The workflow, not the local smoke profile, is the release-quality source of truth. A change is not
fully verified until all four remote test shards and the other required Forgejo jobs are green.

## Normal development sequence

1. Add or identify one focused regression test.
2. Run that focused test locally.
3. Implement the change and rerun the focused test.
4. Run the local smoke profile, formatting, and touched-crate Clippy.
5. Commit and push.
6. Let Forgejo run the complete suite in four parallel shards.
7. If a shard fails, reproduce only the reported test locally; do not rerun the whole workspace.
8. Push the fix and require a clean Forgejo run.

## Adding tests to a tier

- Every new behavior belongs in its owning crate regardless of tier.
- Add a test to the smoke filter only when it protects a short, foundational cross-cutting contract.
- Keep long-horizon economy, population, research, persistence, and campaign tests out of smoke;
  they remain part of the complete remote suite automatically.
- Never weaken or skip a long test merely to shorten CI. Optimize the test or rebalance shards if
  remote duration becomes unacceptable.

## Useful inspection commands

These commands list tests without executing them:

```bash
cargo nextest list --workspace
cargo nextest list --workspace --profile smoke
cargo nextest list --workspace --partition "hash:1/4"
```

Use `fj actions tasks -R origin` to inspect the current Forgejo run. Avoid continuous local polling;
the work is remote and can be checked when a result is needed.
