# Leader AI Design — a colony director that plays like a player

> **SUPERSEDED (design history).** Written for and implemented in the TypeScript
> `lib/game/leaderDirector.ts` (frozen on branch `archive/web-game`). The utility-AI design
> described here was ported behaviorally into Rust as `crates/cat-sim/src/leader_director.rs`
> — see `docs/migration/specs/leader_director.md` for the Rust port's spec and
> `docs/ARCHITECTURE.md` for how it fits into `world_tick`. Kept as the original design
> rationale.

Target (from `docs/ROADMAP.md` §6): near-zero idle cats, a portfolio balanced
across food / water / materials / research / defense, deliberate building, all
autonomous 24/7 and stable at `testTimeScale` up to 10000x. The current
`lib/game/leaderAI.ts` is a hand-ordered rule list with per-axis hysteresis. It
works but it does not *reason about trade-offs* — each axis is decided in
isolation against a fixed threshold, so it can't say "water is a crisis, defer
the storehouse and pull two hunters onto water." That comparison across
competing goals is exactly what utility AI gives us.

## 1. What the field actually uses

**Utility AI / Infinite Axis Utility System (Dave Mark, IAUS).** Each candidate
decision scores itself by mapping normalized world inputs through *response
curves* into per-consideration scores in [0,1], then multiplying them; the
highest-scoring decision wins. Multiplying clamped scores keeps the result in
[0,1] and lets a single veto consideration (score 0) kill a decision. This is
the canonical answer for "resource-heavy / survival gameplay" and is what we
want for the colony-level "what should the colony work on" question. Curves are
the design surface: linear, quadratic, logistic/sigmoid, and inverse forms let
you shape "how urgent is X as it drops." ([gameai.com/IAUS](https://www.gameai.com/iaus.php),
[GameAIPro Ch.9 — Utility Theory](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter09_An_Introduction_to_Utility_Theory.pdf),
[Shaggy Dev intro](https://shaggydev.com/2023/04/19/utility-ai/),
[Utility system — Wikipedia](https://en.wikipedia.org/wiki/Utility_system))

**RimWorld — work priorities + ThinkTree.** Per-pawn behavior is a prioritized
tree of JobGivers/WorkGivers; a pawn runs the *first valid* job its tree yields,
gated by a per-work-type priority (1–4) the player sets. Assignment uses a
`region` system (≤16×16 tiles, split by walls) so "nearest available work" is
cheap. Takeaway: **the per-agent layer is just prioritized filtering + nearest
match**, not planning — the intelligence lives in what work exists and its
priority. ([RimWorld Wiki — Work](https://rimworldwiki.com/wiki/Work),
[AI tutorial](https://github.com/CBornholdt/RimWorld-AI-Tutorial/wiki/Part-1---Introduction))

**Dwarf Fortress — labor switches + a job queue.** Jobs are *created* by
designations/zones/workshops/manager orders; any idle dwarf with the matching
labor enabled grabs one. Priority is weak/opaque; the manager (and DFHack's
`labormanager`) is what makes it feel directed — it *decides how many of each
labor to enable* based on outstanding work. Takeaway: **separate "how much work
to create" (director) from "who does it" (idle pull).**
([DF Wiki — Labor](https://dwarffortresswiki.org/index.php/DF2014:Labor),
[DFHack labormanager](https://docs.dfhack.org/en/stable/docs/tools/labormanager.html))

**Oxygen Not Included — errands + priority × proximity.** Each dupe picks the
highest-priority errand class it's allowed, breaking ties by sub-priority then
proximity; personal needs always preempt. Same shape as RimWorld: **priority
buckets, then distance.** ([ONI Wiki — Priority](https://oxygennotincluded.fandom.com/wiki/Priority))

**BT vs GOAP vs Utility, and the assignment problem.** Behavior trees are
reactive and great for *tactical, testable, scripted* NPC behavior but have no
notion of comparing competing goals. GOAP/HTN plan action *sequences* toward a
goal — overkill here; our "plans" are short and the chaining we need
(gather→build) is already modeled as prerequisite jobs. The consensus for
colony/survival sims is a **hybrid: utility picks priorities, a lightweight
executor carries them out.** Matching N idle cats to M open slots optimally is
the **assignment problem** (Hungarian/Kuhn–Munkres, O(n³)); a greedy
best-cat-per-slot pass is the cheap approximation. ([Tono — GOAP/Utility/BT](https://tonogameconsultants.com/game-ai-planning/),
[Aversa — BT vs GOAP](https://www.davideaversa.it/blog/choosing-behavior-tree-goap-planning/),
[Hungarian algorithm — Wikipedia](https://en.wikipedia.org/wiki/Hungarian_algorithm),
[cp-algorithms](https://cp-algorithms.com/graph/hungarian-algorithm.html))

## 2. Library verdict — write our own

**Don't adopt a BT/GOAP library.** The mature JS/TS option is
[mistreevous](https://github.com/nikkorn/mistreevous) (TS, MDSL/JSON, seedable
RNG, ~last publish 2024). It's fine software but it's the *wrong layer*: BTs
express per-agent reactive sequences, not cross-goal utility comparison, and
they'd fight our determinism model (we want the RNG only at policy-roll sites,
not woven through tree ticks). `behaviortree`/`ts-behavior-tree` are lighter but
same mismatch. No maintained JS utility-AI or GOAP lib is worth a dependency for
what is ~300 lines of pure functions.

**Do write a bespoke utility director** as pure functions in `lib/game/`,
matching the existing `leaderAI.ts` contract (snapshot in, decisions out, no
DB/RNG). It's small, fully unit-testable, and deterministic by construction. If
we ever need optimal multi-cat assignment, vendor a ~40-line Hungarian solver
rather than a framework. **Verdict: build, don't buy.**

## 3. Recommended architecture — two layers

```
snapshot ─▶ DIRECTOR (utility) ─▶ quotas ─▶ SLOTS ─▶ ASSIGNMENT ─▶ decisions
           score each goal,      per job    open      match idle    (existing
           needs→curves          type       job slots  cats→slots    executor)
```

**Layer A — Director (utility scoring).** For each *goal* (feed, water, house,
store, materials, research, defend, expand, tithe) compute a utility in [0,1]
from the snapshot, then convert utility to a **quota**: how many job slots of
that type the colony wants open right now. Goals are scored independently and
compared on one scale, so scarce labor flows to the most urgent axis
automatically — no hand-ordered `if` chain.

**Layer B — Assignment.** Open slots are ranked by their goal's utility; idle
cats are matched to slots greedily by a cost = `skillFit × availability`
(distance later, once pathing lands). This replaces the current per-decision
`sort(by stat)` loops with one global pass so a great hunter isn't burned on a
scout slot while a scrub takes the hunt.

### 3.1 State snapshot (superset of today's `LeaderSnapshot`)

Keep the existing fields; add per-resource and future-era inputs so goals can be
scored uniformly. Every field is a plain number/bool derived in the tick — no
behavior, so it stays trivially testable.

```ts
interface ColonySnapshot {
  population: number;
  idleCats: number;                 // free to take work this tick
  cats: CatBrief[];                  // {id, skills:{hunt,build,ritual,research,fight}, specialization}
  resources: {                       // amount + capacity per tracked resource
    food: Stock; water: Stock; materials: Stock; refined: Stock; research: Stock;
  };
  housing: { capacity: number; committed: number };
  inFlight: Record<GoalKind, number>;   // active+queued jobs per goal (replaces activeHunts etc.)
  sites: { quarry: boolean; water: boolean; frontier: boolean };
  threat: { level: number; incomingRaid: boolean; warriors: number };  // era 4+
  era: number;
}
type Stock = { amount: number; capacity: number; ratePerTick: number };  // rate enables lookahead
```

`ratePerTick` (net production/consumption) is the key addition: it lets a goal
score *projected* scarcity ("water hits zero in N ticks") instead of only
current level — essential at 10000x where a full-to-empty swing happens between
two ticks.

### 3.2 Scoring model (needs → utility curves)

Each goal is a `{ kind, score(snapshot) → [0,1], quota(score, snapshot) → number }`.
`score` multiplies 1–3 considerations, each a curve over a normalized input:

- **Deficit curve** (feed/water/store): input `ratio = amount/capacity`. Use an
  inverse-quadratic so urgency ramps hard near empty and flattens near full:
  `u = clamp01((1 - ratio)²)`. Hysteresis is baked in by scoring, not branching:
  add a small dead-band by subtracting the previous quota's satisfied fraction.
- **Projection curve** (water crisis): `ticksToEmpty = amount / max(ε, -rate)`;
  `u = clamp01(1 - ticksToEmpty / HORIZON)`. Multiply into the deficit score so
  a draining-but-full tank still scores high.
- **Pressure curve** (housing): `pressure = pop/(cap+committed)`; logistic around
  0.8 so a den is commissioned decisively, not linearly.
- **Opportunity gate** (quarry/scout/research): multiply by `sites.* ? 1 : 0` —
  a veto consideration, IAUS-style, so impossible goals score exactly 0.
- **Surplus curve** (tithe): only positive above `TITHE_RATIO`; inverse of the
  deficit curve.

Multiplying clamped considerations keeps every goal score in [0,1] and
comparable. Constants (`HORIZON`, curve exponents, dead-band width) live as named
exports like today's `HUNT_HOLD_RATIO`, each with a boundary test.

### 3.3 Quotas → slots

`quota(score, snapshot)` maps a goal's [0,1] score to a target count of open
job slots, capped by an **employment budget** so the colony can't over-commit:

```
budget         = floor(population * EMPLOYMENT_TARGET_RATIO)   // ~0.7 once eras land
goalTarget(g)  = round(score(g) * maxSlots(g))                // maxSlots caps e.g. hunters
openSlots(g)   = clamp(goalTarget(g) - inFlight[g], 0, ...)
```

Allocate the shared `budget` and `idleCats` to goals in **descending score
order** (the one global priority) — this is where cross-axis trade-off actually
happens: water at 0.9 gets cats before a storehouse at 0.3, without any explicit
"water first" rule. Long-horizon goals (research, roads) get a low floor so a
mouth is always spent on them once unlocked, matching the roadmap's "research
costs a mouth that gathers nothing."

### 3.4 Per-cat assignment

Collect all open slots across goals into one list, sorted by goal score. Greedy
pass: for each slot, pick the available idle cat maximizing
`fit = skill[goal] × (specializationMatch ? 1.5 : 1)` (add `× distanceFalloff`
when pathing exists), then remove that cat from the pool. Greedy is O(slots×cats)
and good enough; if profiling or "obviously wrong" assignments show up, swap in a
Hungarian solver over the `fit` matrix — the interface (`slots`, `cats` → pairs)
doesn't change. This subsumes the four separate `sort()` loops in the current
executor.

### 3.5 Determinism & testing

- Director + assignment stay **pure** (`lib/game/`): no DB, no `Date.now`, no
  RNG. Same snapshot ⇒ same decisions, so 1x and 10000x agree.
- Ties (equal score, equal fit) break by a **stable key** (cat id, goal kind
  order) — never by RNG — so runs are reproducible.
- The seeded policy-reliability roll (`canTakePolicyAction()`) stays at the
  executor call site exactly as today; the leader-tier tests keep passing.
- Unit tests per curve (boundary values 0/empty/full/over-cap), per goal
  (isolated snapshot → expected quota), and a few integration scenarios asserting
  cross-axis trade-off (water crisis pulls cats off hunting; research floor holds
  one cat when idle exists). Reuse `advanceTime` + `setTestRngSeed` for the
  time-sensitive crisis cases per the testing contract.

## 4. Migration path (3 increments, ship each on `main`)

**Increment 1 — utility core behind the existing contract (no behavior change
intended).** Add `lib/game/leaderDirector.ts` with the goal/curve/quota model for
the *goals that exist today* (feed, materials, scout, storage, den, workshop,
tithe). Have `planLeaderActions` delegate to it, tuning curves so its outputs
match the current rule list on today's integration tests (golden-master the
existing `tests/integration/serverGame.test.ts` decisions). No snapshot changes
yet. Deliverable: same decisions, new engine, full curve unit tests. *An
implementer can build this from §3.2–3.3 without more research.*

**Increment 2 — richer snapshot + global assignment.** Extend the snapshot with
`ratePerTick`, per-resource `Stock`, and `cats[]`; add the water goal (roadmap
§1) using the projection curve, and replace the executor's per-decision `sort()`
loops with the single greedy assignment pass (§3.4). Now labor genuinely
reallocates across food/water/materials by score. Add cross-axis trade-off
integration tests.

**Increment 3 — new goals as eras land.** As research (§3), military/defense
(§4), breeding/population, roads, and era progression ship, each is *one new
`Goal` object* — a score curve, a quota, a `maxSlots` cap — dropped into the
director's goal list; the budget allocator and assignment pass absorb them with
no structural change. Swap greedy → Hungarian only if assignment quality demands
it. This is the extensibility payoff: adding "defend against raid" is a curve,
not a new `if` branch in a growing chain.
