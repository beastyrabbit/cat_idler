# LAI.32C Sim Campaign Run Evidence

Task: LAI.32C full sim campaign execution and evidence.

Observed commit:

- `dfb5600da913e6e0db6bc55e733a6205cba320a6`

Owned source hashes after runner/harness edits:

- `crates/cat-sim/src/campaign_runner.rs`: `7e293c5af8cf3709e99a96d10d4cd167c0c3eb781eaf87a6698dee31320da3c1`
- `crates/cat-sim/tests/lai32_campaign_manifest.rs`: `935ebba23791f4f7430ead1af91d05e88d7f724636e32a99858c6630ddcdf181`

Dirty owned paths at evidence capture:

- `?? crates/cat-sim/src/campaign_runner.rs`
- `?? crates/cat-sim/tests/lai32_campaign_manifest.rs`
- `?? docs/leader-ai-overhaul/evidence/lai32-sim-campaign-run.md`

Shared pre-existing/touched-by-earlier-worker path visible but not edited by LAI.32C:

- ` M crates/cat-sim/src/lib.rs`

## Runner Correction

The first release execution used a one-call 30-day horizon shortcut added during LAI.32C to make the ignored gate tractable. That was a runner defect: current `world_tick` treats the single 30-day elapsed window as an unattended collapse/reset path, so it was measuring collapse behavior rather than campaign progression. LAI.32C removed the single-tick shortcut and restored documented cadence execution for full campaign state.

## Discarded Pre-Fix Horizon Evidence

These commands are recorded only to explain the runner correction above; they are not acceptance evidence.

Command:

```text
TIMEFMT=$'real %E\nuser %U\nsys %S'; time cargo test --release -p cat-sim --test lai32_campaign_manifest ignored_release_profile_full_campaign_matrix_meets_lai1_budget -- --ignored --nocapture --exact
```

Result:

- Exit: `101`
- Sets/seeds: `17` sets, `1700` seeds
- Threshold failures: `17`
- Invariant failure counts: `{NoShrineOnlyStarvation: 1700, AutomaticResearchPurchases: 1700}`
- Every set reported `0` successes, including `fresh_colony 0/85` and all established-style sets `0/97`.
- First failing seed: `fresh_colony` seed `320000`
- First failing details: `NoShrineOnlyStarvation` with `resetCount=1, liveJobs=0`; `AutomaticResearchPurchases` with `automaticPurchases=0`
- Timing: `real 443.28s`, `user 515.33s`, `sys 2.24s`
- Peak RSS: unavailable because neither `/usr/bin/time` nor `/bin/time` exists in this environment.

Command:

```text
TIMEFMT=$'real %E\nuser %U\nsys %S'; time cargo test --release -p cat-sim --test lai32_campaign_manifest ignored_restart_partition_matrix_is_byte_equal -- --ignored --nocapture --exact
```

Result:

- Exit: `101`
- Sets/seeds: `1` set, `100` seeds
- Threshold failures: `1`
- Invariant failure counts: `{NoShrineOnlyStarvation: 100, AutomaticResearchPurchases: 100, TickPartitionTwins: 100}`
- Set result: `restart_partition 0/97`
- First failing seed: `restart_partition` seed `336000`
- First failing details: `NoShrineOnlyStarvation` with `resetCount=1, liveJobs=0`; `AutomaticResearchPurchases` with `automaticPurchases=0`; `TickPartitionTwins` with `bounded partition twin fingerprint comparison`
- Timing: `real 163.65s`, `user 235.84s`, `sys 1.72s`
- Peak RSS: unavailable because neither `/usr/bin/time` nor `/bin/time` exists in this environment.

## Post-Fix Full Cadence Attempt

Command:

```text
TIMEFMT=$'real %E\nuser %U\nsys %S'; time cargo test --release -p cat-sim --test lai32_campaign_manifest ignored_release_profile_full_campaign_matrix_meets_lai1_budget -- --ignored --nocapture --exact
```

Observed output before interruption:

```text
Finished `release` profile [optimized] target(s) in 0.10s
Running tests/lai32_campaign_manifest.rs (target/release/deps/lai32_campaign_manifest-772afbbb1f1e6316)

running 1 test
test ignored_release_profile_full_campaign_matrix_meets_lai1_budget has been running for over 60 seconds
```

Result:

- Exit: `130` after manual interruption.
- Tool-observed runtime after the test body began: more than 30 minutes.
- Matrix summary: not emitted before interruption.
- Seed-set counts: not available from the cadence run because the test only prints after all 1700 outcomes have been assembled.
- Threshold/invariant failures: not available from the cadence run.
- Timing sample: no final `time` footer because the PTY was interrupted; use the observed `>30 minutes without matrix output` as the only cadence runtime evidence from this dispatch.
- Peak RSS: unavailable because neither `/usr/bin/time` nor `/bin/time` exists in this environment.

This means LAI.32C could not produce complete full-matrix acceptance evidence in the worker window once the runner used the real documented cadence. The current harness should add bounded progress/evidence streaming or shardable set execution before another full 17x100 cadence attempt.

## Restart/Partition Cadence Matrix

The corrected cadence restart/partition matrix was not run after the release matrix proved unbounded in this dispatch. The pre-fix horizon result above is retained only as discarded diagnostic evidence and must not be used as an acceptance result.

## Remaining Real Work

- Run the full 17-set x100-seed cadence matrix to completion after adding progress output or shard controls.
- Run the corrected restart/partition cadence matrix to completion.
- Capture peak RSS with an available measurement tool or a project-supported wrapper.
- If completed cadence evidence fails sim invariants, hand the exact seed/invariant pairs to the owning production sim workers without weakening LAI.32 thresholds.

## Focused Validation

Command:

```text
cargo test -p cat-sim --test lai32_campaign_manifest --no-fail-fast
```

Result:

- Exit: `0`
- Count: `12` tests discovered; `10` passed; `2` ignored
- Runtime: `4.91s`

Command:

```text
cargo nextest run -p cat-sim --test lai32_campaign_manifest --no-fail-fast
```

Result:

- Exit: `0`
- Count: `10` tests run; `10` passed; `2` skipped
- Runtime: `4.912s`

Command:

```text
cargo clippy -p cat-sim --tests -- -D warnings
```

Result:

- Exit: `0`
- Crates checked: `cat-protocol`, `cat-sim`

Command:

```text
rustfmt --edition 2024 --check crates/cat-sim/src/campaign_runner.rs crates/cat-sim/tests/lai32_campaign_manifest.rs
```

Result:

- Exit: `0`

Command:

```text
git diff --check -- crates/cat-sim/src/campaign_runner.rs crates/cat-sim/tests/lai32_campaign_manifest.rs docs/leader-ai-overhaul/evidence/lai32-sim-campaign-run.md
```

Result:

- Exit: `0`

Command:

```text
rg -n "[[:blank:]]$" crates/cat-sim/src/campaign_runner.rs crates/cat-sim/tests/lai32_campaign_manifest.rs docs/leader-ai-overhaul/evidence/lai32-sim-campaign-run.md
```

Result:

- Exit: `1`, meaning no trailing whitespace matches were found.

Workspace-format blocker:

```text
cargo fmt --all --check
```

Result:

- Exit: `1`
- The diffs were in files outside LAI.32C ownership: `crates/cat-server/src/main.rs`, `crates/cat-sim/src/research_purchase.rs`, and `crates/cat-sim/tests/player_directives.rs`.

## LAI.32D Sharded Evidence

LAI.32D added an independently selectable release-profile shard harness:

- Test: `ignored_release_profile_campaign_shard_from_env`
- Selector: `LAI32_CAMPAIGN_SET=<manifest-set-id>`
- Artifact directory: `LAI32_CAMPAIGN_ARTIFACT_DIR=<dir>`
- Real cadence is preserved: each seed runs the 30-game-day `world_tick` cadence path with `2880` ticks and no horizon shortcut, max-tick truncation, sampling reduction, synthetic success, ignored failure, or threshold change.
- Progress is visible: the shard prints one line after every completed seed with seed id, success flag, failed invariants, tick count, reset count, final tick, and deterministic fingerprint.

Environment:

- `rustc 1.96.1 (31fca3adb 2026-06-26)`
- `cargo 1.96.1 (356927216 2026-06-26)`
- `Linux bunux 7.1.3-arch1-3 #1 SMP PREEMPT_DYNAMIC Mon, 13 Jul 2026 20:15:15 +0000 x86_64 GNU/Linux`
- Peak RSS measurement remains unavailable in this container because neither `/usr/bin/time` nor `/bin/time` exists.

Source/artifact hashes:

- `crates/cat-sim/src/campaign_runner.rs`: `a5e799198546c11f9b1b8b68561a1ccf336c0e37836d94755eec00d684cf40cc`
- `crates/cat-sim/tests/lai32_campaign_manifest.rs`: `5f284374cacb3925fb3a28beaa8828bc92dee704ac78430d96643dbb95`
- `docs/leader-ai-overhaul/evidence/lai32-release-shard-fresh-colony.json`: `450f2f2d78d10e459df40fd04ed4b77dcf477382a29aa0a75b80aa419bd22d14`

Command:

```text
TIMEFMT=$'real %E\nuser %U\nsys %S'; time env LAI32_CAMPAIGN_SET=fresh_colony LAI32_CAMPAIGN_ARTIFACT_DIR=/tmp/lai32d cargo test --release -p cat-sim --test lai32_campaign_manifest ignored_release_profile_campaign_shard_from_env -- --ignored --nocapture --exact
```

Result:

- Exit: `101`
- Release build: `Finished release profile [optimized] target(s) in 0.06s`
- Runtime: `real 412.47s`, `user 409.88s`, `sys 0.06s`
- Shard: `fresh_colony`
- Seeds: `100` completed, `320000..=320099`
- Required successes: `85`
- Observed successes: `0`
- Threshold result: `fresh_colony 0/85`
- Invariant failures: `NoShrineOnlyStarvation: 100`, `AutomaticResearchPurchases: 100`
- Reset-count distribution bounds: minimum `53`, maximum `111`
- Automatic research purchase distribution: only `0`
- Artifact copied into the worktree as `docs/leader-ai-overhaul/evidence/lai32-release-shard-fresh-colony.json`

First reproducible failing seed:

- Scenario: `fresh_colony`
- Seed: `320000`
- Ticks: `2880`
- Final tick: `2592001000`
- Failed invariants: `NoShrineOnlyStarvation`, `AutomaticResearchPurchases`
- Details: `resetCount=62`, `liveJobs=0`, `automaticPurchases=0`

The remaining 16 release shards and the restart/partition matrix were not executed after this shard-wide behavioral failure, because LAI.32D requires either complete aggregate evidence or exact failing-seed escalation and forbids treating a known failure as green aggregate evidence. Escalation was sent with the exact first failing seed and artifact path. Follow-up production ownership should start with `fresh_colony` seed `320000` and determine why the 30-day campaign repeatedly resets and records no automatic research purchases.

## LAI.32D Focused Validation

Command:

```text
cargo test -p cat-sim --test lai32_campaign_manifest --no-fail-fast
```

Result:

- Exit: `0`
- Count: `13` tests discovered; `10` passed; `3` ignored
- Runtime: `5.01s`

Command:

```text
cargo nextest run -p cat-sim --test lai32_campaign_manifest --no-fail-fast
```

Result:

- Exit: `0`
- Count: `10` tests run; `10` passed; `3` skipped
- Runtime: `4.992s`

Command:

```text
cargo clippy -p cat-sim --tests -- -D warnings
```

Result:

- Exit: `0`
- Crates checked: `cat-protocol`, `cat-sim`
- The previous `clone_on_copy` warning is absent.

Command:

```text
rustfmt --edition 2024 --check crates/cat-sim/src/campaign_runner.rs crates/cat-sim/tests/lai32_campaign_manifest.rs
```

Result:

- Exit: `0`

Command:

```text
cargo fmt --all --check
```

Result:

- Exit: `0`

Command:

```text
git diff --check -- crates/cat-sim/src/campaign_runner.rs crates/cat-sim/tests/lai32_campaign_manifest.rs docs/leader-ai-overhaul/evidence/lai32-sim-campaign-run.md docs/leader-ai-overhaul/evidence/lai32-release-shard-fresh-colony.json docs/leader-ai-overhaul/BOARD.md
```

Result:

- Exit: `0`

Command:

```text
rg -n "[[:blank:]]$" crates/cat-sim/src/campaign_runner.rs crates/cat-sim/tests/lai32_campaign_manifest.rs docs/leader-ai-overhaul/evidence/lai32-sim-campaign-run.md docs/leader-ai-overhaul/evidence/lai32-release-shard-fresh-colony.json docs/leader-ai-overhaul/BOARD.md
```

Result:

- Exit: `1`, meaning no trailing whitespace matches were found.

## LAI.32E Focused Reset-Loop Follow-Up

Command:

```text
cargo test -p cat-sim --release --test lai32_campaign_manifest fresh_colony_seed_320000_no_longer_enters_reset_loop_and_progresses -- --nocapture
```

Result:

- Exit: `101`
- Count: `1` focused regression run, `0` passed, `1` failed
- Exact remaining failure after the first live-job/reservation cause was narrowed:
  `fresh_colony` seed `320000` reset at tick index `144` (`now_ms=129601000`) with
  `RunResetReason::UnattendedCollapse`, `live_job_count=0`,
  `visible_task_count=48`, `resolved_spatial_task_count=48`,
  `assigned_visible_task_count=0`, stages
  `FetchWater:Complete=24,Hunt:Complete=24`, `food=0`, `water=0`,
  `work_capable_cat_count=30`, `favor_balance_micro=0`, and
  `automatic_research_purchase_count=0`.
- Production integration changes made before this remaining failure: visible
  Hunt/Water tasks now materialize from ranked survival intents, use bounded
  shared spatial capacities, complete through the physical task runtime, credit
  Hunt/Water resources on completion, mark completed intents terminal, and add a
  pre-reset deterministic trace for this seed.
- Remaining work: identify why no new survival visible tasks are assigned after
  the first 24 Hunt/24 Water completions despite zero reported resources and 30
  work-capable cats, then rerun the full `fresh_colony` shard and the remaining
  LAI.32 matrix.

Validation:

- `cargo fmt` passed after edits.
- `git diff --check` passed after edits.
- `cargo clippy -p cat-sim --tests -- -D warnings` passed after edits.
- `cargo nextest run -p cat-sim --release --test lai32_campaign_manifest fresh_colony_seed_320000_no_longer_enters_reset_loop_and_progresses --no-fail-fast`
  reproduced the focused red failure with `1` test run, `0` passed, `1`
  failed, and `13` skipped.

## LAI.32E Follow-Up 2: Replacement-Task Materialization Blocker

Command:

```text
cargo test -p cat-sim --release --test lai32_campaign_manifest fresh_colony_seed_320000_no_longer_enters_reset_loop_and_progresses -- --nocapture
```

The deterministic regeneration and materialization changes now reproduce a later, independent
failure rather than the original no-intent handoff:

- Seed `320000`, real `900000` ms cadence, reset at tick `223`, `now_ms=200701000`.
- `RunResetReason::AllCatsDead`; `live_job_count=0`; `visible_task_count=132`;
  `resolved_spatial_task_count=132`; `assigned_visible_task_count=4`.
- Visible stages: `FetchWater:Complete=104`, `FetchWater:TravelToSource=4`,
  `Hunt:Complete=24`; `food=0`, `water=0`; `work_capable_cat_count=2`;
  cat summary `Eat=5,None=25`.
- The focused test remains red (`0` passed, `1` failed, `13` skipped), so no full shard or
  restart/partition matrix was accepted.

Production changes under review: survival review regenerates a terminalized goal immediately
when the report-derived need remains and no survival task is active; physical materialization
fills the existing population-scaled Hunt/Water caps with distinct deterministic occurrences;
legacy Hunt/FetchWater jobs are retired at the LAI.23 boundary. The exact remaining owner issue
is physical source/assignment starvation after the first Hunt wave: only four water tasks remain
assignable while the colony's work-capable roster collapses; thresholds and reset behavior were
not weakened, and no resources or hidden sites were injected.

## LAI.32F Follow-Up 3: Finite Hunt Source and Personal-Need Interaction

Focused regressions added in `crates/cat-sim/src/world_tick.rs`:

- `lai23_depleted_revealed_hunt_source_rejoins_phase12_regrowth` passes: a revealed non-forest
  Hunt source with zero stock and a persisted depletion timestamp regrows through phase 12.
- `lai23_report_safe_food_availability_reenables_hunt_after_depletion` passes: one remaining
  authoritative source unit is report-safe and makes Hunt a schedulable survival category.

Production changes: LAI.23 Hunt candidates now use available finite source units rather than a
fixed hidden capacity; visible Hunt completion clamps reward to source stock and drains the same
authoritative tile through the existing depletion/regrowth lifecycle; assigned visible survival
work is no longer preempted by personal-needs routing while a physical Hunt/FetchWater task is
active. The exact seed remains red at tick `212`, `now_ms=190801000`, `AllCatsDead`, with
`visibleTaskCount=148`, `assignedVisibleTaskCount=20`, `FetchWater:Complete=50`,
`FetchWater:TravelToEndpoint=8`, `Hunt:Complete=78`, `Hunt:TravelToEndpoint=12`,
`food=0`, `water=0`, `workCapableCats=2`, and cat task summary `FetchWater=8,Hunt=12,None=10`.
This is a new finite-source replenishment/arrival timing blocker; no threshold, hidden source,
or test shortcut was introduced. Full campaign and restart/partition gates remain blocked by the
focused red seed.

Validation: focused unit regressions pass `2/2`; `cargo check -p cat-sim --lib`, strict
`cargo clippy -p cat-sim --lib -- -D warnings`, rustfmt check for `world_tick.rs`, and
`git diff --check` pass.
