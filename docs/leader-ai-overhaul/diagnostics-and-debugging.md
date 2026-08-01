# Leader AI Diagnostics and Debugging

This is the operational guide for diagnosing a stalled, slow, or apparently incorrect Leader AI
run. Diagnostics must make the deterministic cause visible without changing simulation ordering,
consuming RNG, exposing hidden truth to a player, or turning every normal tick into an expensive log
stream.

The default production path remains `world_tick`. Detailed simulation diagnostics are opt-in through
`world_tick_with_phase_observer`, focused campaign probes, server tracing, and browser wire evidence.
None of these records are part of the public integrated snapshot, normal Log, player report
projection, authoritative gameplay persistence, or planner input. The dedicated trace may persist
only inside an explicitly enabled developer/test artifact.

## Diagnostic layers

Use the least expensive layer that can answer the question:

1. **Browser wire evidence** identifies whether the client received an authoritative snapshot,
   queued the intended integrated action, received its typed response, and then received the
   post-mutation snapshot.
2. **Server action tracing** identifies authentication, selected-colony routing, optimistic
   concurrency, stable-ID resolution, authorization, simulation rejection, persistence, and
   immediate snapshot publication.
3. **Tick phase diagnostics** identify the exact deterministic phase whose entry was observed but
   whose exit was not.
4. **Focused tick-boundary probes** explain survival, task, reservation, belief, family/governance,
   construction/storage, Hole, research lanes, divine effects, barter, and demographic state
   immediately before and after a narrow tick range.
5. **Campaign summaries and restart twins** validate long-horizon outcomes only after the focused
   probe is bounded and live.

Do not start several campaign, Clippy, browser-build, or workspace-suite commands in parallel. On a
resource-constrained development machine, use one heavy command at a time with:

```sh
CARGO_BUILD_JOBS=1 taskset -c 0-3 <command>
```

Run tests with one test thread. A long command that produces no next phase or boundary record within
its expected window is liveness-red; stop it, retain the last record, and narrow the probe.

## World-tick phase records

`cat_sim::world_tick::world_tick_with_phase_observer` calls its observer synchronously at entry and
exit of each major authoritative phase. `WorldTickPhaseDiagnostic` contains:

- phase and entry/exit boundary;
- selected colony ID and requested/previous tick times;
- living cats, queued jobs, active jobs, and critical-since time;
- visible/resolved/terminal task totals plus stage and category counts;
- intent, local reservation, and world reservation totals;
- world/revealed tile, building, stockpile, and event counts; and
- current food, fish, and water.

The integrated `leader_ai_diagnostics` leaf generalizes this as a strict schema-v1 opt-in bounded
ring. It is disabled by default, evicts the oldest record deterministically at capacity, stores
canonical bounded stable strings/count maps, and checks monotonic sequence numbers. Caller-supplied
timing is diagnostic metadata only; the pure simulation leaf never reads a clock.

Typed records cover:

- phase entry/exit and last transition;
- planner candidates, scores, omissions, priorities, matching, rejections, tasks, blockers, and
  reservation counts;
- skill/XP, teaching, family, housing, partnership, election, ballot, appointment, and succession;
- God/Leader research selection, collision, refund, preparation, permits, and repeatables;
- construction stage/cargo/click contribution, storage pressure/container capacity, farms/roads/
  walls, and maintenance;
- Hole feed/upgrade/value/recovery, food permissions, contribution aid, Inspiration, boosts,
  miracles, rescue, and rate rejection;
- personal stance, barter valuation, escrow, caravan, cargo, route, stage, and recovery;
- persistence/action transaction and UI action-envelope/rejection boundaries.

If the final record is `Enter(X)` without `Exit(X)`, investigate phase `X`. Compare the last complete
tick with the failing tick. Collection growth is visible in the same record, so an accidental
quadratic scan or unbounded retained task/reservation set can be distinguished from a logic loop.

The observer must remain observational:

- it may format, serialize, or collect its supplied bounded record;
- it must not read the clock, call RNG, mutate the world, or run pathfinding;
- production uses the zero-observer wrapper;
- public protocol/UI must never receive the diagnostic record; and
- a new world-tick phase must add entry and exit records around the complete phase.

## Focused campaign probes

`cat_sim::campaign_runner` exposes explicit probes for the known LAI.32 fresh-colony seed. The
61–120 liveness probe emits:

- `BeforeWorldBuild` and `AfterWorldBuild`;
- `BeforeTick` and `AfterTick` with tick and simulation time;
- every world-tick phase entry/exit; and
- per-tick task, spatial-resolution, reservation, intent, cat, terminal/active-task, and task-stage
  counts.

Narrow boundary probes additionally expose the causal state needed for colony survival and growth:

- living/dead/pregnant cats, lifecycle stage, anatomy eligibility, willingness, Leader identity,
  level, duty time, review cadence, and forecast horizon;
- exact authoritative survival resources only inside the test diagnostic, report values separately,
  legal source/load counts, hunt-resolution reasoning, active task chains, assignments, and stages;
- intents, beliefs, critical duration, lifecycle events, migration departures, housing, fields,
  agricultural tiles, farms, expansion jobs, and food-chain inventory;
- Hole feed/upgrade pipeline and recovery, Notes/Void events, God queue/front/preparation/refund,
  Leader research candidate/collision/quota, construction/storage/family/election/divine/barter
  counts and blockers; and
- personal-need tasks and cats approaching a survival boundary.

The diagnostic-only ignored tests in
`crates/cat-sim/tests/lai32_campaign_manifest.rs` deliberately print these records with
`--nocapture`. Run exactly one:

```sh
CARGO_BUILD_JOBS=1 taskset -c 0-3 \
  cargo test -p cat-sim --test lai32_campaign_manifest \
  diagnostic_fresh_seed_320000_ticks_61_to_120 \
  -- --ignored --exact --nocapture --test-threads=1
```

Choose the smallest range containing the failure. Do not promote an ignored diagnostic to the smoke
profile or treat an incomplete/terminated probe as acceptance evidence.

## Exact 120-tick heartbeat and terminal causes

The bounded probe emits a heartbeat at the documented 120-tick cadence boundary and on terminal
state. Each heartbeat carries:

- current tick and sequence;
- current phase;
- live task count;
- active reservation count;
- last stable transition; and
- an explicit terminal cause when terminal.

Terminal causes are `Completed`, `Timeout`, `Stalled`, `SimulationFailure`, and `Panic`. Only
`Completed` is pass. A timeout, killed process, missing final heartbeat, silence after phase entry,
or output truncated before terminal cause is liveness-red. Large and partitioned advances must emit
the same due heartbeat boundaries without duplicates.

## Server action and snapshot tracing

Run the server with `RUST_LOG` targeted to the relevant subsystem. Useful targets include ordinary
`cat_server` routing plus `leader_ai_ui` for client-side control suppression. A rejected trade logs
its bounded contract/action context and internal typed failure before mapping it to a public,
report-safe reason. Authentication material and private foreign-colony state must never be logged.

Every authenticated request receives an immediate integrated snapshot after its action response. This is
required even when the deterministic browser fixture freezes simulation ticks: waiting for a future
tick would leave a valid mutation invisible and deadlock browser acceptance.

For an action failure, record this sequence:

1. authenticated player and selected colony (never the token);
2. action kind and bounded idempotency ID;
3. exact expected version lane from `actionVersions`;
4. canonical entity resolution, including a bounded `wire:v1` alias if presented;
5. authority and precondition result;
6. accepted/rejected/duplicate action response; and
7. the immediately following snapshot version.

Connection admission and new-session issuance are different limits. Server warnings name the peer,
bounded limit, and window when either rejects; they never log the session bearer or signature. A
serial browser journey must reuse its signed primary context. Creating a fresh anonymous context for
every checkpoint can correctly exhaust the new-session abuse limit even when every WebSocket closes.

An aggregate display `stateVersion` is not an action concurrency token. A planner dismissal also
uses the separately projected `planningEpoch`, not the queue version.

## Browser evidence

The production Playwright harness records console messages, page errors, failed requests,
integrated snapshots/actions, and action responses. Each click waits for:

1. the intended action kind;
2. a matching typed action response; and
3. a subsequent authoritative snapshot.

This makes a timeout diagnostic rather than ambiguous. The most recent trace shows whether the
failure was rendering, hit testing, envelope construction, socket routing, server rejection, or
snapshot refresh. The suite uses one worker and the committed SQLite fixture is copied to a
temporary runtime database, so source evidence is not mutated.

Keep action identifiers opaque in evidence. Human-readable planner labels are decoded only for
display. Never replace the submitted canonical ID with its display label.

## Adding diagnostics later

When adding a new planner phase, workshop, task stage, action domain, or persistent subsystem:

1. add bounded counts or stable IDs to the nearest existing diagnostic record;
2. add phase entry/exit observation if the work introduces a new major tick phase;
3. add a narrow deterministic probe around the first meaningful boundary;
4. keep expensive derivations conditional and outside ordinary traces;
5. add a focused test proving diagnostics do not mutate state or consume RNG;
6. redact public/server logs separately from test-only authoritative diagnostics;
7. document the exact one-thread command and expected last boundary; and
8. add the browser action/response/snapshot trace when the feature is player-visible.

Do not add a second simulation mode, a diagnostic RNG stream, unbounded debug strings to protocol
DTOs, or client access to authoritative hidden values.
