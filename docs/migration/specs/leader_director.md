# P3.2 Leader Director Port Spec

> **Historical parity spec.** This records the frozen TypeScript behavior used for the Rust port;
> it is not a claim about the maintained officer/manual-control product model. Current behavior
> and open role work are tracked in [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

Sources read:
- `lib/game/leaderDirector.ts`
- `lib/game/leaderAI.ts`
- `tests/unit/game/leaderDirector.test.ts`
- `tests/unit/game/leaderAI.test.ts`
- Narrow call-site check: `server/game.ts` leader snapshot and assignment execution.

## Purpose

Port the pure leader snapshot, decision contract, utility scoring curves, labor
slot director, and greedy cat-to-slot matcher. Target Rust modules:
`crates/cat-sim/src/leader_ai.rs` for the public snapshot/decision contract and
`crates/cat-sim/src/leader_director.rs` for the director logic.

The director must stay pure and deterministic: no DB, clock, filesystem, raw RNG,
or seeded RNG use. Policy fallibility remains at the executor call site.

## Public Surface

Use snake_case Rust APIs that preserve the TS exports. `leader_ai.rs` owns the
snapshot and decision types; `leader_director.rs` imports them and owns all scoring
and assignment behavior.

### `leader_ai.rs`

```rust
pub use crate::leader_director::{
    target_warriors, EMPLOYMENT_TARGET_RATIO, WARRIOR_MAX_RATIO,
    WARRIOR_TARGET_BY_BAND,
};

pub struct LeaderResources {
    pub food: f64,
    pub refined: f64,
}

pub struct LeaderHousing {
    pub capacity: u32,
    pub committed: u32,
}

pub enum ThreatBand {
    Calm,
    Rising,
    Imminent,
}

pub struct LeaderSnapshot {
    pub population: u32,
    pub workforce: Option<f64>,
    pub idle_cats: u32,
    pub employed_cats: u32,
    pub resources: LeaderResources,
    pub food_capacity: f64,
    pub food_drain_per_hour: Option<f64>,
    pub materials: f64,
    pub materials_capacity: f64,
    pub water: f64,
    pub water_capacity: f64,
    pub water_drain_per_hour: Option<f64>,
    pub housing: LeaderHousing,
    pub active_hunts: u32,
    pub active_quarries: u32,
    pub active_scouts: u32,
    pub active_water_fetchers: u32,
    pub has_quarry_site: bool,
    pub has_water_site: bool,
    pub has_frontier: bool,
    pub den_plans_in_flight: u32,
    pub storage_plans_in_flight: u32,
    pub storehouse_count: u32,
    pub storehouse_cap: u32,
    pub workshops_needing_workers: u32,
    pub research_huts_needing_workers: Option<u32>,
    pub smithies_needing_workers: Option<u32>,
    pub has_barracks: Option<bool>,
    pub warrior_count: Option<u32>,
    pub training_in_flight: Option<u32>,
    pub threat_band: Option<ThreatBand>,
    pub starving: Option<bool>,
}

pub enum LeaderDecision {
    Hunt { count: u32 },
    CancelHunts,
    FetchWater { count: u32 },
    Quarry { count: u32 },
    Scout { count: u32 },
    BuildDen,
    BuildStorage,
    AssignWorkshop { count: u32 },
    AssignResearch { count: u32 },
    AssignSmithy { count: u32 },
    TrainWarrior { count: u32 },
    CancelTraining,
    Tithe { food: u32, refined: u32, blessings: u32 },
}

pub fn plan_leader_actions(snapshot: &LeaderSnapshot) -> Vec<LeaderDecision>;
```

Wire names are the TS literals: `cancel_hunts`, `fetch_water`, `build_den`,
`assign_workshop`, etc. `ThreatBand` wire names are `calm`, `rising`, and
`imminent`.

Drain rates are resource units per game-hour. Deserialization continues to accept
the legacy `food_drain_per_tick` and `water_drain_per_tick` keys as aliases so
persisted snapshots and older clients remain compatible.

`LeaderSnapshot` optional defaults:

| field | omitted means |
| --- | --- |
| `workforce` | use `population` |
| `food_drain_per_hour` | `0` |
| `water_drain_per_hour` | `0` |
| `research_huts_needing_workers` | `0` |
| `smithies_needing_workers` | `0` |
| `has_barracks` | `false` |
| `warrior_count` | `0` |
| `training_in_flight` | `0` |
| `threat_band` | `ThreatBand::Calm` |
| `starving` | `false` |

`plan_leader_actions`:
1. Calls `direct_colony(snapshot)`.
2. Preserves only cancellation decisions first: `CancelHunts`, `CancelTraining`.
3. Converts every `plan.slots` entry, in slot order, into the matching count
   decision (`Hunt`, `FetchWater`, `Quarry`, `Scout`, `TrainWarrior`,
   `AssignWorkshop`, `AssignResearch`, or `AssignSmithy`).
4. Appends all non-cancellation standalone decisions in their `direct_colony`
   order.

### `leader_director.rs`

```rust
pub const EMPLOYMENT_TARGET_RATIO: f64 = 0.7;
pub const IDLE_EMPLOYMENT_FLOOR: f64 = 0.8;
pub const PROJECTION_HORIZON_HOURS: f64 = 4.0;
pub const HUNT_CANCEL_RATIO: f64 = 1.1;
pub const STORAGE_RATIO: f64 = 0.9;
pub const DEN_PRESSURE_THRESHOLD: f64 = 0.8;
pub const RESEARCH_COMFORT_RATIO: f64 = 0.5;
pub const TITHE_FOOD_RATIO: f64 = 0.6;
pub const TITHE_FOOD_AMOUNT: u32 = 20;
pub const TITHE_REFINED_AMOUNT: u32 = 5;
pub const HUNT_MAX_SLOTS_RATIO: f64 = 0.7;
pub const WATER_MAX_SLOTS: u32 = 4;
pub const QUARRY_MAX_SLOTS: u32 = 2;
pub const SCOUT_MAX_SLOTS: u32 = 2;
pub const SCOUT_BASE_SCORE: f64 = 0.3;
pub const STAFF_BASE_SCORE: f64 = 0.45;
pub const WARRIOR_BASE_SCORE: f64 = 0.5;
pub const WARRIOR_MAX_RATIO: f64 = 0.4;
pub const PROJECTION_GATE_RATIO: f64 = 0.9;

pub struct WarriorTargetByBand {
    pub calm: u32,
    pub rising: u32,
    pub imminent: u32,
}

pub const WARRIOR_TARGET_BY_BAND: WarriorTargetByBand =
    WarriorTargetByBand { calm: 2, rising: 4, imminent: 7 };

pub fn target_warriors(snapshot: &LeaderSnapshot) -> u32;

pub fn clamp01(x: f64) -> f64;
pub fn deficit_curve(ratio: f64) -> f64;
pub fn projection_curve(amount: f64, drain_per_hour: f64, horizon_hours: f64) -> f64;
pub fn pressure_curve(pressure: f64, center: f64, steepness: f64) -> f64;
pub fn surplus_curve(ratio: f64, threshold: f64) -> f64;
pub fn combine_or(a: f64, b: f64) -> f64;
pub fn projection_gate(fill_ratio: f64) -> f64;
pub fn survival_score(fill_ratio: f64, amount: f64, drain_per_hour: f64) -> f64;

pub enum LaborGoalKind {
    Hunt,
    FetchWater,
    Quarry,
    Scout,
    TrainWarrior,
    AssignWorkshop,
    AssignResearch,
    AssignSmithy,
}

pub enum GoalStat {
    Hunting,
    Building,
    Vision,
    Medicine,
    AttackDefense,
    Leadership,
}

pub struct GoalSkill {
    pub skill: GoalStat,
    pub prefer_specialization: Option<CatSpecialization>,
}

pub fn goal_skill(kind: LaborGoalKind) -> GoalSkill;

pub enum LaborGoalMode {
    Scaled,
    Fixed,
}

pub struct LaborGoal {
    pub kind: LaborGoalKind,
    pub score: f64,
    pub max_slots: u32,
    pub in_flight: u32,
    pub hard_cap: u32,
    pub vetoed: bool,
    pub mode: LaborGoalMode,
}

pub fn goal_open_slots(goal: &LaborGoal) -> u32;

pub struct OpenSlots {
    pub goal: LaborGoalKind,
    pub count: u32,
    pub score: f64,
}

pub struct CatBriefStats {
    pub hunting: f64,
    pub building: f64,
    pub vision: f64,
    pub medicine: f64,
    pub attack: f64,
    pub defense: f64,
    pub leadership: f64,
}

pub struct CatBrief {
    pub id: String,
    pub specialization: Option<CatSpecialization>,
    pub stats: CatBriefStats,
}

pub fn assignment_fit(cat: &CatBrief, goal: LaborGoalKind) -> f64;

pub struct Assignment {
    pub cat_id: String,
    pub goal: LaborGoalKind,
}

#[derive(Default)]
pub struct MatchOptions {
    pub exclude_warriors_from_training: bool,
}

pub fn match_cats_to_slots(
    slots: &[OpenSlots],
    cats: &[CatBrief],
    options: MatchOptions,
) -> Vec<Assignment>;

pub struct DirectorPlan {
    pub decisions: Vec<LeaderDecision>,
    pub slots: Vec<OpenSlots>,
}

pub fn direct_colony(snapshot: &LeaderSnapshot) -> DirectorPlan;
```

`labor_goals(snapshot)` is private in TS and may stay private in Rust, but its
result shape is parity-critical and should be unit-tested in-module if exposed as
`pub(crate)`.

## Constants

| TS name | value | notes |
| --- | ---: | --- |
| `EMPLOYMENT_TARGET_RATIO` | `0.7` | Used to size the core hunt budget. |
| `IDLE_EMPLOYMENT_FLOOR` | `0.8` | Near-zero idle fill target. |
| `PROJECTION_HORIZON_HOURS` | `4` | Default projection horizon in game-hours. |
| `HUNT_CANCEL_RATIO` | `1.1` | Strict `foodR > 1.1` cancellation threshold. |
| `STORAGE_RATIO` | `0.9` | Strict `foodR > 0.9` build-storage threshold. |
| `DEN_PRESSURE_THRESHOLD` | `0.8` | Inclusive `pressure >= 0.8` build-den threshold. |
| `RESEARCH_COMFORT_RATIO` | `0.5` | Food and water must both be at least this ratio. |
| `TITHE_FOOD_RATIO` | `0.6` | Food surplus threshold before tithing. |
| `TITHE_FOOD_AMOUNT` | `20` | Food spent for one blessing. |
| `TITHE_REFINED_AMOUNT` | `5` | Refined spent for one blessing. |
| `HUNT_MAX_SLOTS_RATIO` | `0.7` | Hunt max is `ceil(core_budget * 0.7)`. |
| `WATER_MAX_SLOTS` | `4` | Scaled water hard cap. |
| `QUARRY_MAX_SLOTS` | `2` | Scaled quarry hard cap. |
| `SCOUT_MAX_SLOTS` | `2` | Fixed scout hard cap before idle-floor fill. |
| `SCOUT_BASE_SCORE` | `0.3` | Fixed scout priority. |
| `STAFF_BASE_SCORE` | `0.45` | Fixed workshop/research/smithy priority. |
| `WARRIOR_BASE_SCORE` | `0.5` | Fixed training priority. |
| `WARRIOR_TARGET_BY_BAND.calm` | `2` | Base target. |
| `WARRIOR_TARGET_BY_BAND.rising` | `4` | Base target. |
| `WARRIOR_TARGET_BY_BAND.imminent` | `7` | Base target. |
| `WARRIOR_MAX_RATIO` | `0.4` | Workforce cap for target warriors. |
| `EPS` | `1e-9` | Internal projection divisor floor. |
| `PROJECTION_GATE_RATIO` | `0.9` | Fill ratio where projection is fully suppressed. |
| pressure steepness default | `10` | Default `pressure_curve` steepness. |

## Response Curves

`clamp01(x)` returns `0` when `x < 0`, `1` when `x > 1`, otherwise `x`. Preserve
TS NaN behavior if possible: comparisons with NaN are false, so NaN returns NaN.

`deficit_curve(ratio)`:
1. `r = clamp01(ratio)`.
2. Return `(1 - r) * (1 - r)`.

`projection_curve(amount, drain_per_hour, horizon_hours = 4)`:
1. If `drain_per_hour <= 0` or `horizon_hours <= 0`, return `0`.
2. `hours_to_empty = max(0, amount) / max(EPS, drain_per_hour)`.
3. Return `clamp01(1 - hours_to_empty / horizon_hours)`.

`pressure_curve(pressure, center = 0.8, steepness = 10)` returns:

```text
clamp01(1 / (1 + exp(-steepness * (pressure - center))))
```

`surplus_curve(ratio, threshold)` returns `0` when `ratio <= threshold`;
otherwise `clamp01((ratio - threshold) / (1 - threshold))`.

`combine_or(a, b)` returns:

```text
clamp01(1 - (1 - clamp01(a)) * (1 - clamp01(b)))
```

`projection_gate(fill_ratio)` returns:

```text
clamp01((PROJECTION_GATE_RATIO - fill_ratio) / PROJECTION_GATE_RATIO)
```

`survival_score(fill_ratio, amount, drain_per_hour)` returns:

```text
combine_or(
  deficit_curve(fill_ratio),
  projection_curve(amount, drain_per_hour, PROJECTION_HORIZON_HOURS)
    * projection_gate(fill_ratio)
)
```

## Labor Goals

Private helpers:

```text
ratio(amount, capacity):
  if capacity <= 0: return amount > 0 ? 1 : 0
  else: return amount / capacity

workforce_of(snapshot):
  snapshot.workforce.unwrap_or(snapshot.population as f64)

able_cats(snapshot):
  snapshot.idle_cats + snapshot.employed_cats
```

`labor_goals(snapshot)` computes:

```text
core_budget = floor(workforce_of(snapshot) * EMPLOYMENT_TARGET_RATIO)
foodR = ratio(resources.food, food_capacity)
waterR = ratio(water, water_capacity)
materialsR = ratio(materials, materials_capacity)
foodScore = survival_score(foodR, resources.food, food_drain_per_hour.unwrap_or(0))
waterScore = survival_score(waterR, water, water_drain_per_hour.unwrap_or(0))
materialsScore = deficit_curve(materialsR)
comfortable = foodR >= 0.5 && waterR >= 0.5
warriorGap = target_warriors(snapshot) - warrior_count.unwrap_or(0)
             - training_in_flight.unwrap_or(0)
```

Clamp `warriorGap` to zero for unsigned Rust slot counts after computing the signed
difference. The TS code uses signed numbers, then `Math.max(0, warriorGap)`.

Goal definitions, in TS construction order:

| kind | score | max_slots | in_flight | hard_cap | vetoed | mode |
| --- | --- | --- | --- | --- | --- | --- |
| `hunt` | `foodScore` | `ceil(core_budget * 0.7)` | `activeHunts` | same as max | `foodR >= 1` | `scaled` |
| `fetch_water` | `waterScore` | `4` | `activeWaterFetchers` | `4` | `!hasWaterSite || waterR >= 1` | `scaled` |
| `quarry` | `materialsScore` | `2` | `activeQuarries` | `2` | `!hasQuarrySite || materialsR >= 1` | `scaled` |
| `scout` | `0.3` | `2` | `activeScouts` | `2` | `!hasFrontier` | `fixed` |
| `assign_workshop` | `0.45` | `workshopsNeedingWorkers` | `0` | same | `workshopsNeedingWorkers <= 0 || starving` | `fixed` |
| `assign_research` | `0.45` | `researchHutsNeedingWorkers ?? 0` | `0` | same | `research <= 0 || !comfortable` | `fixed` |
| `assign_smithy` | `0.45` | `smithiesNeedingWorkers ?? 0` | `0` | same | `smithies <= 0 || starving` | `fixed` |
| `train_warrior` | `0.5` | `max(0, warriorGap)` | `0` | same | `warriorGap <= 0 || starving` | `fixed` |

Important: `assign_research` is not directly vetoed by `starving`; it is vetoed
only by `!comfortable`. With inconsistent snapshots where `starving=true` but
food/water ratios are comfortable, TS can still staff research. Replicate TS.

`goal_open_slots(goal)`:
1. If `goal.vetoed`, return `0`.
2. If `mode == fixed`, `target = max_slots`.
3. If `mode == scaled`, `target = Math.round(score * max_slots)`.
   For non-negative values, Rust can reproduce this with `floor(x + 0.5)`.
4. Return `max(0, min(target - in_flight, hard_cap - in_flight))`.

Water top-up edge case: with `waterR = 0.1`, score is `0.81`, target is
`round(0.81 * 4) = 3`; if `activeWaterFetchers = 3`, open slots are `0`, not `1`.
Only score `1` opens the final fourth slot.

## Goal Skills and Assignment Fit

`goal_skill(kind)` must return exactly:

| goal | stat | preferred specialization |
| --- | --- | --- |
| `hunt` | `hunting` | `Hunter` |
| `fetch_water` | `hunting` | `None` |
| `quarry` | `building` | `Architect` |
| `scout` | `vision` | `None` |
| `train_warrior` | `attack + defense` | `None` |
| `assign_workshop` | `building` | `None` |
| `assign_research` | `medicine` | `None` |
| `assign_smithy` | `building` | `Architect` |

`assignment_fit(cat, goal)`:
1. Select the base stat from the table above.
2. For `AttackDefense`, base is `cat.stats.attack + cat.stats.defense`.
3. If `prefer_specialization` is `Some(x)` and `cat.specialization == Some(x)`,
   multiply the base by `1.5`.
4. Return the raw score; do not clamp or round.

## `match_cats_to_slots`

Algorithm:
1. Expand `slots` in the input order into a flat list of one `LaborGoalKind` per
   requested slot. For each slot, repeat `slot.goal` exactly `slot.count` times.
2. Copy `cats` into a mutable pool preserving input order.
3. For every goal in the flat slot list:
   - Scan pool from first to last.
   - If goal is `train_warrior`, `exclude_warriors_from_training` is true, and the
     cat specialization is `Warrior`, skip that cat.
   - Compute `assignment_fit`.
   - Update best cat only on strict `fit > best_fit`.
   - If no cat is eligible, skip the slot.
   - Otherwise push `{ cat_id, goal }` and remove that cat from the pool.
4. Return assignments in slot expansion order.

Tie-break warning: comments and board text say "cat-id tie-break", but the TS
implementation never sorts by id. Equal fit keeps the first cat in the input pool
because the update is strictly greater. If the executor supplies cats sorted by
id, this becomes id order by convention; the pure matcher itself is input-order
tied. Do not add an internal sort unless the TS call-site is also changed.

## `target_warriors`

Algorithm:
1. If `has_barracks` is absent or false, return `0`.
2. `band = threat_band.unwrap_or(Calm)`.
3. `base = WARRIOR_TARGET_BY_BAND[band]`.
4. `workforce = workforce.unwrap_or(population as f64)`.
5. `cap = floor(workforce * 0.4)`.
6. Return `min(base, max(1, cap))`.

Parity edge case: with `has_barracks=true` and `workforce=0`, TS returns `1`, not
`0`, because of `max(1, cap)`.

## `direct_colony`

### Cancellations

Start with `decisions = []` and `foodR = ratio(resources.food, food_capacity)`.

1. If `foodR > 1.1` and `activeHunts > 0`, push `CancelHunts`.
2. If `starving == true` and `trainingInFlight > 0`, push `CancelTraining`.

Cancellations are emitted before every standalone build/tithe decision. They do
not directly increase `idle_cats` inside this pure function.

### Ranked labor allocation

`labourLeft` starts as `snapshot.idleCats`. Note the TS comment says "shared
employment budget", but the initial loop spends from the whole idle pool. The
`EMPLOYMENT_TARGET_RATIO` budget is used to size the hunt goal, not as a global
budget cap for all goals.

Build `goals = labor_goals(snapshot)`, then sort a copy by:
1. Descending `score`, using exact float inequality (`b.score !== a.score` in TS).
2. For equal scores, this fixed order:

```text
fetch_water
hunt
quarry
train_warrior
assign_smithy
assign_workshop
assign_research
scout
```

Then, for each ranked goal:
1. Stop if `labourLeft <= 0`.
2. `want = goal_open_slots(goal)`.
3. `give = min(want, labourLeft)`.
4. If `give > 0`, add it to the `granted` count for that kind and subtract it from
   `labourLeft`.

### Near-zero idle fill

After ranked grants:

```text
busySoFar = employedCats + sum(granted counts)
employTarget = ceil((idleCats + employedCats) * 0.8)
idleLeft = max(0, idleCats - (busySoFar - employedCats))
fillWanted = max(0, min(idleLeft, employTarget - busySoFar))
```

Fill work order is:

```text
[
  { kind: hunt, open: foodR < 1 },
  { kind: scout, open: hasFrontier },
  { kind: quarry, open: hasQuarrySite },
]
```

While `fillWanted > 0` and a pass makes progress, walk the fill order and grant
one slot to each open kind, decrementing `fillWanted`. This is round-robin.

Parity edge case: idle-floor grants ignore each goal's `max_slots`, `hard_cap`,
and `in_flight`. They can push `scout`, `quarry`, or `hunt` counts beyond the
normal `goal_open_slots` cap. Replicate this behavior.

### Slot emission

Emit `slots` in ranked priority order, not in fill-order. For each ranked goal
with a positive granted count, push:

```text
{ goal: goal.kind, count: granted_count, score: original_goal.score }
```

Counts include both ranked grants and idle-floor fill grants.

### Capital projects

After labor slots are computed, append standalone decisions:

1. `storehousesInPlay = storehouseCount + storagePlansInFlight`.
2. If `foodR > 0.9`, `storagePlansInFlight == 0`, and
   `storehousesInPlay < storehouseCap`, push `BuildStorage`.
3. `shelter = housing.capacity + housing.committed`.
4. `pressure = Infinity` if `shelter <= 0`, otherwise `population / shelter`.
5. If `pressure >= 0.8` and `denPlansInFlight == 0`, push `BuildDen`.

### Tithe

```text
titheFood =
  resources.food > foodCapacity * 0.6 + 20 ? 20 : 0

titheRefined =
  resources.refined >= 5 ? 5 : 0

blessings = (titheFood > 0 ? 1 : 0) + (titheRefined > 0 ? 1 : 0)
```

If `blessings > 0`, push `Tithe { food, refined, blessings }`.

Food tithe uses strict `>` against `foodCapacity * 0.6 + 20`. At capacity `200`,
food `140` does not tithe food; food above `140` does.

Final `DirectorPlan` is `{ decisions, slots }`.

## Determinism

This module does not use the seeded LCG, the movement/life/raid forked chains, or
raw `Math.random`. The same snapshot and cat input order must always produce the
same plan and assignments.

Determinism depends on preserving:
- Exact response formulas and `floor`/`ceil`/`round` behavior.
- Exact ranked tie order for labor goals.
- Strictly-greater best-fit updates in `match_cats_to_slots`.
- Cat input order for equal assignment fits.
- No sorting by id inside `match_cats_to_slots`.

No nondeterministic TS behavior was found in these two modules. The two behaviors
that look most bug-like but should be replicated for parity are:
- Idle-floor grants can exceed normal goal caps and in-flight limits.
- `target_warriors` returns `1` for a zero-workforce colony if it has barracks.

## Golden Fixtures to Generate

Recommended fixture path: `docs/migration/fixtures/p3/leader_director.json`.
Use a pure `npx tsx` script importing `lib/game/leaderDirector.ts` and
`lib/game/leaderAI.ts`; do not edit TS sources.

Unless stated otherwise, use this default snapshot:

```json
{
  "population": 20,
  "workforce": 20,
  "idleCats": 10,
  "employedCats": 0,
  "resources": { "food": 200, "refined": 0 },
  "foodCapacity": 200,
  "materials": 200,
  "materialsCapacity": 200,
  "water": 200,
  "waterCapacity": 200,
  "housing": { "capacity": 40, "committed": 0 },
  "activeHunts": 0,
  "activeQuarries": 0,
  "activeScouts": 0,
  "activeWaterFetchers": 0,
  "hasQuarrySite": false,
  "hasWaterSite": false,
  "hasFrontier": false,
  "denPlansInFlight": 0,
  "storagePlansInFlight": 0,
  "storehouseCount": 0,
  "storehouseCap": 3,
  "workshopsNeedingWorkers": 0
}
```

### Response Curve Vectors

The historical TS helper names and expected values below remain unchanged parity
evidence and use the legacy six-tick horizon. They do not describe the maintained
default: current drain arguments are resource units per game-hour, and the current
projection horizon is four game-hours.

| expression | expected |
| --- | ---: |
| `clamp01(-2)` | `0` |
| `clamp01(0.5)` | `0.5` |
| `clamp01(2)` | `1` |
| `deficitCurve(1)` | `0` |
| `deficitCurve(1.5)` | `0` |
| `deficitCurve(0)` | `1` |
| `deficitCurve(0.5)` | `0.25` |
| `deficitCurve(0.25)` | `0.5625` |
| `projectionCurve(100, 0)` | `0` |
| `projectionCurve(100, -5)` | `0` |
| `projectionCurve(10, 10)` | `0.8333333333333334` |
| `projectionCurve(600, 10)` | `0` |
| `projectionCurve(-5, 10)` | `1` |
| `projectionCurve(10, 10, 0)` | `0` |
| `projectionGate(1)` | `0` |
| `projectionGate(0.9)` | `0` |
| `projectionGate(0)` | `1` |
| `projectionGate(0.45)` | `0.5` |
| `survivalScore(1, 200, 9999)` | `0` |
| `survivalScore(0.3, 60, 40)` | `0.745` |
| `pressureCurve(0.8)` | `0.5` |
| `pressureCurve(0.4)` | `0.01798620996209156` |
| `pressureCurve(1.2)` | `0.9820137900379085` |
| `surplusCurve(0.5, 0.6)` | `0` |
| `surplusCurve(0.6, 0.6)` | `0` |
| `surplusCurve(1, 0.6)` | `1` |
| `surplusCurve(0.8, 0.6)` | `0.5000000000000001` |
| `combineOr(0, 0)` | `0` |
| `combineOr(1, 0)` | `1` |
| `combineOr(0, 1)` | `1` |
| `combineOr(0.5, 0.5)` | `0.75` |

Use `f64` and compare floats with a tight tolerance such as `1e-12` unless JSON
round-trips exactly in the local test helper.

### Target Warrior Vectors

| override | expected |
| --- | ---: |
| `{ "hasBarracks": false }` | `0` |
| `{ "hasBarracks": true, "threatBand": "calm", "workforce": 40 }` | `2` |
| `{ "hasBarracks": true, "threatBand": "rising", "workforce": 40 }` | `4` |
| `{ "hasBarracks": true, "threatBand": "imminent", "workforce": 40 }` | `7` |
| `{ "hasBarracks": true, "threatBand": "imminent", "workforce": 4 }` | `1` |
| `{ "hasBarracks": true, "threatBand": "calm", "workforce": 0 }` | `1` |

### `directColony` Snapshot Matrix

Each row means default snapshot plus override, then exact expected `DirectorPlan`.

```json
[
  {
    "name": "empty",
    "override": {
      "population": 0,
      "workforce": 0,
      "idleCats": 0,
      "resources": { "food": 100, "refined": 0 }
    },
    "expected": { "decisions": [], "slots": [] }
  },
  {
    "name": "famine_hunts",
    "override": {
      "resources": { "food": 0, "refined": 0 },
      "idleCats": 10
    },
    "expected": {
      "decisions": [],
      "slots": [{ "goal": "hunt", "count": 10, "score": 1 }]
    }
  },
  {
    "name": "water_crisis_and_storage",
    "override": {
      "water": 30,
      "hasWaterSite": true,
      "resources": { "food": 190, "refined": 0 },
      "idleCats": 6
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": [
        { "goal": "fetch_water", "count": 3, "score": 0.7224999999999999 },
        { "goal": "hunt", "count": 2, "score": 0.0025000000000000577 }
      ]
    }
  },
  {
    "name": "single_workshop_not_rounded_away",
    "override": { "workshopsNeedingWorkers": 1, "idleCats": 4 },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": [{ "goal": "assign_workshop", "count": 1, "score": 0.45 }]
    }
  },
  {
    "name": "vetoed_sites",
    "override": {
      "water": 0,
      "materials": 0,
      "hasWaterSite": false,
      "hasQuarrySite": false,
      "hasFrontier": false
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": []
    }
  },
  {
    "name": "water_in_flight_dry_tops_up_one",
    "override": {
      "water": 0,
      "hasWaterSite": true,
      "activeWaterFetchers": 3
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": [{ "goal": "fetch_water", "count": 1, "score": 1 }]
    }
  },
  {
    "name": "water_in_flight_10_percent_opens_zero",
    "override": {
      "water": 20,
      "hasWaterSite": true,
      "activeWaterFetchers": 3
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": []
    }
  },
  {
    "name": "idle_floor_frontier_exceeds_scout_cap",
    "override": {
      "resources": { "food": 200, "refined": 0 },
      "idleCats": 12,
      "employedCats": 0,
      "hasFrontier": true
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": [{ "goal": "scout", "count": 10, "score": 0.3 }]
    }
  },
  {
    "name": "cancel_threshold_1_05_no_cancel",
    "override": {
      "activeHunts": 4,
      "resources": { "food": 210, "refined": 0 }
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": []
    }
  },
  {
    "name": "cancel_threshold_1_2_cancel",
    "override": {
      "activeHunts": 4,
      "resources": { "food": 240, "refined": 0 }
    },
    "expected": {
      "decisions": [
        { "kind": "cancel_hunts" },
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": []
    }
  },
  {
    "name": "starving_cancels_training",
    "override": { "starving": true, "trainingInFlight": 2 },
    "expected": {
      "decisions": [
        { "kind": "cancel_training" },
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": []
    }
  },
  {
    "name": "den_at_threshold",
    "override": { "housing": { "capacity": 25, "committed": 0 } },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "build_den" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": []
    }
  },
  {
    "name": "storehouse_cap_stops_storage_but_idle_floor_hunts",
    "override": {
      "resources": { "food": 190, "refined": 0 },
      "storehouseCount": 3,
      "storehouseCap": 3
    },
    "expected": {
      "decisions": [
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": [{ "goal": "hunt", "count": 8, "score": 0.0025000000000000577 }]
    }
  },
  {
    "name": "tithe_food_and_refined",
    "override": { "resources": { "food": 200, "refined": 10 } },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 5, "blessings": 2 }
      ],
      "slots": []
    }
  },
  {
    "name": "fixed_order_staff_tie",
    "override": {
      "idleCats": 6,
      "resources": { "food": 120, "refined": 0 },
      "water": 120,
      "hasBarracks": true,
      "warriorCount": 1,
      "workshopsNeedingWorkers": 1,
      "researchHutsNeedingWorkers": 1,
      "smithiesNeedingWorkers": 1
    },
    "expected": {
      "decisions": [],
      "slots": [
        { "goal": "train_warrior", "count": 1, "score": 0.5 },
        { "goal": "assign_smithy", "count": 1, "score": 0.45 },
        { "goal": "assign_workshop", "count": 1, "score": 0.45 },
        { "goal": "assign_research", "count": 1, "score": 0.45 },
        { "goal": "hunt", "count": 2, "score": 0.16000000000000003 }
      ]
    }
  },
  {
    "name": "round_robin_idle_floor",
    "override": {
      "idleCats": 10,
      "employedCats": 0,
      "resources": { "food": 190, "refined": 0 },
      "water": 200,
      "materials": 200,
      "hasFrontier": true,
      "hasQuarrySite": true
    },
    "expected": {
      "decisions": [
        { "kind": "build_storage" },
        { "kind": "tithe", "food": 20, "refined": 0, "blessings": 1 }
      ],
      "slots": [
        { "goal": "scout", "count": 4, "score": 0.3 },
        { "goal": "hunt", "count": 2, "score": 0.0025000000000000577 },
        { "goal": "quarry", "count": 2, "score": 0 }
      ]
    }
  }
]
```

### `planLeaderActions` Flattening Vector

Use this snapshot:

```json
{
  "population": 20,
  "workforce": 20,
  "idleCats": 6,
  "employedCats": 0,
  "resources": { "food": 240, "refined": 10 },
  "foodCapacity": 200,
  "materials": 200,
  "materialsCapacity": 200,
  "water": 20,
  "waterCapacity": 200,
  "housing": { "capacity": 10, "committed": 0 },
  "activeHunts": 4,
  "activeQuarries": 0,
  "activeScouts": 0,
  "activeWaterFetchers": 0,
  "hasQuarrySite": false,
  "hasWaterSite": true,
  "hasFrontier": false,
  "denPlansInFlight": 0,
  "storagePlansInFlight": 0,
  "storehouseCount": 0,
  "storehouseCap": 3,
  "workshopsNeedingWorkers": 0
}
```

`directColony` should return decisions `[cancel_hunts, build_storage, build_den,
tithe]` and slots `[fetch_water x3 score 0.81]`.

`planLeaderActions` should flatten to:

```json
[
  { "kind": "cancel_hunts" },
  { "kind": "fetch_water", "count": 3 },
  { "kind": "build_storage" },
  { "kind": "build_den" },
  { "kind": "tithe", "food": 20, "refined": 5, "blessings": 2 }
]
```

### Assignment Matcher Vectors

Use this cat helper default stats:

```json
{
  "hunting": 30,
  "building": 30,
  "vision": 30,
  "medicine": 30,
  "attack": 30,
  "defense": 30,
  "leadership": 30
}
```

Specific expected fits:

| cat | goal | expected fit |
| --- | --- | ---: |
| `spec` hunting `50`, specialization `hunter` | `hunt` | `75` |
| `gen` hunting `70`, no specialization | `hunt` | `70` |
| `war` attack `50`, defense `60` | `train_warrior` | `110` |
| `arch` building `50`, specialization `architect` | `assign_smithy` | `75` |

Expected matches:

```json
[
  {
    "name": "best_hunter",
    "slots": [{ "goal": "hunt", "count": 1, "score": 1 }],
    "cats": [
      { "id": "a", "stats": { "hunting": 40 }, "specialization": null },
      { "id": "b", "stats": { "hunting": 90 }, "specialization": null }
    ],
    "expected": [{ "catId": "b", "goal": "hunt" }]
  },
  {
    "name": "specialization_bonus_beats_raw_skill",
    "slots": [{ "goal": "hunt", "count": 1, "score": 1 }],
    "cats": [
      { "id": "gen", "stats": { "hunting": 70 }, "specialization": null },
      { "id": "spec", "stats": { "hunting": 50 }, "specialization": "hunter" }
    ],
    "expected": [{ "catId": "spec", "goal": "hunt" }]
  },
  {
    "name": "priority_slots_first",
    "slots": [
      { "goal": "fetch_water", "count": 1, "score": 0.9 },
      { "goal": "scout", "count": 1, "score": 0.3 }
    ],
    "cats": [
      { "id": "sturdy", "stats": { "hunting": 90, "vision": 10 }, "specialization": null },
      { "id": "scout", "stats": { "hunting": 20, "vision": 90 }, "specialization": null }
    ],
    "expected": [
      { "catId": "sturdy", "goal": "fetch_water" },
      { "catId": "scout", "goal": "scout" }
    ]
  },
  {
    "name": "tie_keeps_input_order",
    "slots": [{ "goal": "hunt", "count": 1, "score": 1 }],
    "cats": [
      { "id": "z", "stats": { "hunting": 50 }, "specialization": null },
      { "id": "a", "stats": { "hunting": 50 }, "specialization": null }
    ],
    "expected": [{ "catId": "z", "goal": "hunt" }]
  },
  {
    "name": "exclude_existing_warriors_from_training",
    "options": { "excludeWarriorsFromTraining": true },
    "slots": [{ "goal": "train_warrior", "count": 1, "score": 1 }],
    "cats": [
      { "id": "vet", "stats": { "attack": 90, "defense": 90 }, "specialization": "warrior" },
      { "id": "rookie", "stats": { "attack": 40, "defense": 40 }, "specialization": null }
    ],
    "expected": [{ "catId": "rookie", "goal": "train_warrior" }]
  },
  {
    "name": "short_pool_assigns_each_cat_once",
    "slots": [
      { "goal": "hunt", "count": 2, "score": 1 },
      { "goal": "scout", "count": 2, "score": 0.3 }
    ],
    "cats": [
      { "id": "a", "stats": {}, "specialization": null },
      { "id": "b", "stats": {}, "specialization": null },
      { "id": "c", "stats": {}, "specialization": null }
    ],
    "expected": [
      { "catId": "a", "goal": "hunt" },
      { "catId": "b", "goal": "hunt" },
      { "catId": "c", "goal": "scout" }
    ]
  }
]
```

## Dependencies

Must exist before `leader_director.rs` implementation:
- `leader_ai.rs` snapshot and `LeaderDecision` contract.
- `types.rs::CatSpecialization` with variants `Hunter`, `Architect`,
  `Ritualist`, and `Warrior`, represented as `Option<CatSpecialization>` for TS
  `null`.

Useful but not strictly required:
- `entities.rs::CatStats` can be converted into `CatBriefStats`, but the matcher
  only needs the seven TS fields listed above and ignores `cleaning`.
- `policy.rs` is not a dependency. The seeded policy-reliability roll happens
  where decisions are executed, not inside the director.
- `rng.rs` is not a dependency. This module uses no randomness.

Implementation order:
1. `leader_ai.rs`: snapshot, decisions, wire names, and `plan_leader_actions`.
2. `leader_director.rs`: constants and response curves.
3. `target_warriors` and private `labor_goals`.
4. `goal_open_slots`, ranked allocation, idle-floor fill, capital projects, tithe.
5. `assignment_fit` and `match_cats_to_slots`.
6. Fixture-backed parity tests for `direct_colony`, `plan_leader_actions`, and
   matcher behavior.
