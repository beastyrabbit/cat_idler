# Hole, Research Notes, Void Insight, research lanes, and divine progression

This is the maintained progression contract for the combined Leader-AI and `bug-gui-design`
overhaul. It supersedes the pre-integration Shrine/Favor design. The historical
[`shrine-favor-research.md`](shrine-favor-research.md) remains only as evidence of the discarded
branch behavior; no implementation may restore its authorities, currencies, package conversion, or
semantic migration.

The exact sources remain
[`final-hole-hunting-content-plan.md`](final-hole-hunting-content-plan.md) and
[`final-integrated-overhaul-plan.md`](final-integrated-overhaul-plan.md). Their P1/P2 rows in
[`BOARD.md`](BOARD.md) and
[`bug-gui-design-BOARD.md`](../branch-plan-merge/bug-gui-design-BOARD.md) are additive acceptance,
not summaries that may replace this domain contract.

## One authority and three distinct progression concepts

The simulation owns one canonical completion ledger and two spendable currencies:

| Concept | Created by | Spent on | Exact visibility |
|---|---|---|---|
| Research Notes | completed physical scholar work | ordinary finite studies, technology families, convergence junctions, and repeatable tracks | exact to the owning player; report-safe rationale to Leader/officers |
| Void Insight | completed physical Hole feeds after authoritative validation and consumption | thirty Hole-axis studies, the four player-only Divine Boosts, and approved Void miracles | exact to the owning player; never physical cargo |
| Preparation | one completed physical scholar preparation job | one nonstacking 25% duration/cost reduction on a later player-funded ordinary study | target and completion are exact to the owning player; not a currency |

There is no Shrine, Favor, Blessing, generic research-point balance, scholar Insight currency,
research-credit conversion, or mirrored presentation wallet. `favor.rs`, `shrine_offerings.rs`, old
snapshot/action variants, legacy UI, fixtures, and schema columns are cutover inputs to delete, not
adapters to keep.

The two research lanes share the same canonical study IDs and completion ledger. A study completed
by either lane is complete for the colony. They do not share funding, timers, quota accounting, or
authority.

## Hole feed and Void boundary

The fixed Hole is the only source of Void Insight. Its geometry and physical feed authority live in
`black_hole.rs`; research consumes only the resulting idempotent micro-Void credits.

1. The Leader submits report-safe believed candidates and ordered fallbacks.
2. Authoritative validation checks content identity, physical item/lot identity, ownership,
   capability, Darkness gate, quality, reservation, route, location, amount, and Depth capacity.
3. The runtime reserves source, exact cargo, route, hauler, Hole delivery/work site, and capacity in
   one world-scoped transaction.
4. Cargo moves through queued, reserved, carried, and delivered physical stages.
5. A due forty-game-minute opening consumes only delivered authorized units, under Width intake
   `1 + width` and Depth capacity `10 × (1 + depth)`.
6. One checked final-floor value calculation creates one idempotent micro-Void credit.
7. Cancellation, refusal, death, preemption, route loss, or restart returns each unconsumed identity
   to its origin, a compatible stockpile, or a typed last-land cache.

The Hole is endlessly eligible. It has no completion gate or missed-feed punishment. Survival,
hydration, lethal danger, active defense, and already committed urgent work may outrank it. Strong
leadership normally feeds a low believed replacement-cost input; weak leadership may feed scarce
food or omit the review, causing truthful Apple/Fish/Hunt/Farm/Cookhouse recovery work. Hidden stock
never vetoes a legally available poor choice.

Gods and officers see the same report-safe resource/ecology information. Exact regeneration,
replenishment, regrowth, respawn, unreported stock, and unreported source capacity remain
server-only through report level 3.

## Canonical research graph

One validated manifest owns:

- every stable study, technology family, track, level, prerequisite, AND junction, payload, effect,
  building permit, capability unlock, queue/display metadata, and art key;
- all ordinary capabilities derived from content rather than an arbitrary hard-coded total;
- the thirty Hole-axis studies and their exact Width/Depth/Darkness prerequisites/effects;
- fourteen curated finite tracks at levels 1–10 and repeatable level 11+ behavior;
- at least twenty-four real AND junctions plus the eight curated convergence junctions;
- deterministic three-region graph placement and fixed pan behavior;
- all four Divine Boost definitions and researched duration/economy choices.

Validation rejects duplicate IDs/display identities, missing references, cycles, unreachable finite
nodes, inert or unknown payload handlers, mismatched capability ownership, invalid region/track
metadata, inconsistent repeatable cost growth, and a research node for a data-owned recipe bundle.
Ordering is topological, then `(region, track, level, priority, layout, study_id)`.

Founding Water, Apples, hand-fishing, basic food, raw Logs, and raw Stone capabilities are free.
Every other resource, processed material, food source, item class, rare material, station, tool,
fixture, and augmentation has one canonical ordinary study. Plank Processing is global. Locked
content may be found, stored, or bartered but cannot be processed, installed, augmented, crafted,
or fed to the Hole.

A curated recipe is unlocked only when its station and tier exist, every ingredient capability is
owned, the bundle-owner capability is owned, and physical ingredients, tools, capacity, and workers
exist. Per-recipe research nodes are forbidden.

## God lane: physical, queued, durable, and refundable

The authenticated player controls a topological path queue with maximum length 64.

- Only the queue front may freeze and reserve its exact Notes or Void cost.
- Ordinary studies use Notes; Hole-axis studies use Void.
- Work duration freezes with the front cost and is completed by staffed physical scholar work.
- Progress survives reorder, disconnect, restart, and the bounded offline-catch-up policy.
- A node may not be moved ahead of any prerequisite.
- Removing a node cascades queued descendants, refunds their funded currency, and discards only
  elapsed labor.
- If the Leader completes the funded/front study first, currency is refunded exactly once; elapsed
  God-lane labor is not converted to Notes, preparation, another study, or Leader quota.
- Completion is compare-and-set against the shared completion ledger, so concurrent or replayed
  completion cannot double-apply a payload.

One completed preparation applies a single 25% reduction to one later player-funded ordinary
study. It does not stack, expire, transfer, fund a Leader choice, or reduce a Hole study/boost/
miracle. The reduction is frozen and consumed atomically when the eligible front is funded. Removing
or overtaking the target follows the documented refund while never restoring a consumed preparation
twice.

Building levels 1–10 use research permits, but the Leader—not the God—decides when to start the
physical upgrade. A permit cannot place a building, choose a tile, assign a worker, select a storage
zone, or bypass three-stage construction.

## Leader lane: free, instant, report-safe, and imperfect

Leader research spends no Notes/Void, uses no scholar, queue, building, preparation, or timer, and
completes only a prerequisite-ready finite study through the shared completion ledger.

The rolling seven-game-day limit is:

| Effective Loremaster level | Total Leader completions per rolling seven days |
|---:|---:|
| vacancy/founding coverage | 1 |
| 1 | 1 |
| 2 | 2 |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |

Unused capacity does not carry over. Colony timestamps survive succession and restart. Selection
uses reports, active plan dependencies, believed village need, Intelligence, personality, relevant
skill, finite-first policy, and stable study ID. Hidden resources, hidden regeneration, private God
intent, and client-only graph state are not candidate inputs.

The Leader normally excludes the God queue/front and selects another useful eligible study. If a
funded God target becomes less attractive, it is down-ranked rather than silently cancelled.
Duplicate targeting is legal only when:

1. report-safe evidence marks the same study as a critical village need; or
2. the isolated deterministic research-error roll hits the exact leadership band
   `25/12/5/1/0%`.

The completion event records `ordinary`, `critical_need_override`, or `oopsie_duplicate` rationale.
If a duplicate wins the completion race, the God lane receives the exact currency refund and loses
elapsed labor; the payload applies once. An unaffordable God study does not affect Leader quota, and
an ineligible/blocked Leader choice consumes no quota.

## Scholar work and Notes

Scholars are physical workers, not passive currency generators. A research or preparation task
owns:

- the complete Research Hut/School footprint;
- one exact reserved scholar slot and work position;
- any physical prerequisite source, route, cargo identity, and endpoint;
- target study/preparation ID, completion marker, elapsed work, and block reason;
- declared Scholarship/secondary XP awards only after successful work.

Completed ordinary research labor credits Notes once through a stable event ID. Interrupted work
retains elapsed progress according to the queue contract; cancellation/refusal/death releases the
slot and returns unconsumed cargo. A replacement scholar may continue the same target without
inheriting office report clearance. Blocked, waiting, invalid, or failed work grants no Notes and no
XP.

Preparation is another explicit physical job. It yields the one target-bound 25% marker, not a
general Insight balance.

## Hole studies and Divine Boosts

The thirty Hole studies are the ten Width, ten Depth, and ten Darkness axis levels. Research effects
do not resize the 5×5 landmark. They unlock the corresponding physical upgrade project, whose exact
scaffold/structure/fit-out cargo and labor must complete before the axis changes.

Only authenticated players may buy the four specialized boosts:

| Boost | Village-wide effect | Base Void rate |
|---|---|---:|
| Bountiful Labor | +50% raw gathering, carrying, and harvesting | 2 per hour |
| Fleet Paws | +50% movement | 1 per hour |
| Inspired Work | +50% construction and production | 2 per hour |
| Restorative Grace | +50% healing | 2 per hour |

The base duration is one game-hour. Research unlocks the documented duration choices and cost
reductions. Cost uses checked integer/fixed-point ceil math and is frozen with start/expiry/effect
scope. Different boost types may overlap. Buying an already active type rejects without debit;
same-type stacking, reset, cancellation, and refund are forbidden. Leaders and officers never buy
or reserve currency for boosts.

Boosts modify only their declared effective operation. They cannot reveal a source, bypass consent,
make an ineligible cat work, skip a site/route/cargo/station reservation, grant XP, change genes/
traits/age, or expand report capability.

## Divine aid and Void miracles

Ordinary contribution and Inspiration are separate from research and boosts:

- ordinary contribution eventually creates one nonexpiring 100%-need Divine Ration or Divine Water
  at the Hole apron; it is a physical Reserve-policy item requiring real hauling;
- Inspiration is `+10%` effective stats for fifteen real minutes, with a sixty-minute per-player
  cooldown, no same-player stacking, and additive independent-player sources;
- neither permanently mutates cats or grants report knowledge.

One-Void construction press and population rescue actions are player-only:

- the press creates only the exact missing purpose-bound construction input bundle worth twice a
  one-Void Hole feed and removes 10% original duration earliest-stage-first;
- generated press cargo cannot overfill, return to general stock, barter, or feed the Hole;
- population rescue creates exactly `2 × living residents` Rations or Water;
- rescue controls require report-safe evidence that residents are dying from the matching need.

Debit, generated physical identity, purpose/provenance binding, stage change, population snapshot,
and action receipt commit once or not at all.

## Protocol and server boundary

The integrated snapshot projects:

- exact owning-player Notes and Void balances with stable ledger/event references;
- report-safe Hole feed rationale, stages, cargo identity summary, recovery, axis construction, and
  omission/block reason;
- canonical graph regions/tracks/nodes, prerequisites, completion, repeatable level, permits, and
  report-safe availability;
- God queue/front, frozen cost/duration, progress, preparation, descendants/refund preview;
- Leader cadence/quota, selected target, reason, collision/override/oopsie, and report provenance;
- scholar slots/tasks/progress/blockers and physical prerequisite route/cargo;
- active boost and divine-aid source/expiry/cooldown/provenance summaries.

Every mutation carries protocol version, authenticated player/colony, stable action ID, exact
domain expected-version lane, and strict bounded payload. Server order is protocol, authentication,
ownership/authority, expected version, idempotency, then simulation validation. Stale, duplicate,
unaffordable, ineligible, wrong-currency, locked, same-type-active, malformed, future-version, and
precondition failures are typed and reveal no hidden stock, regeneration, source, candidate, or
private colony state.

Forbidden direct actions include Leader target selection, research candidate injection, building/
upgrade start, exact site/worker/storage/route selection, food-list editing, or officer command.

## Fresh persistence and restart

Persist the canonical manifest/rules version, completion ledger, Notes/Void ledgers, Hole credits,
God queue/front/frozen values/progress, preparation, Leader rolling-window timestamps/decision
reason, scholar tasks, permits, repeatable levels, active boosts, divine aid/cooldowns, and bounded
idempotency receipts.

The pre-production cutover performs no currency or gameplay-state conversion. A known obsolete
Shrine/Favor/research schema takes the authorized reset/recreation path; unknown, future, or
malformed state fails closed. Fresh fixtures contain no Shrine, Favor, Blessing, generic research
points, scholar Insight, coin, purse, or legacy research UI identifier.

Fine ticks, large ticks, restart, reconnect, and bounded offline catch-up must agree on completion,
refund, quota windows, scholar progress, boost expiry, and physical cargo state. A crash between
validation and commit cannot debit twice, apply a payload twice, lose a funded refund, or mint a
preparation.

## UI and accessibility

Research is one of the five primary screens. It uses the three-region graph, queue/front, detailed
study inspector, preparation, repeatables, permits, Leader-lane explanation, and Notes/Void balances.
It does not restore `research_ui.rs` or a second progression surface.

The Hole inspector shows full 5×5 geometry, central 3×3 work objective, paved ring, physical feed/
upgrade pipeline, reported candidate rationale, Void credit history, axes, exact player-visible
costs, construction stages, nudges, and bounded blockers. Hidden stock/regeneration is absent from
labels, tooltips, accessibility trees, logs, errors, screenshots, and client state.

Controls are mouse/touch/keyboard accessible with stable roles, labels, focus, disabled reasons,
pending/stale/reconnect feedback, and centralized Escape behavior. Client state is disposable and
never calculates research availability, a refund, currency, Hole value, or hidden source.

## Required evidence

Focused and final serialized evidence must prove:

- unique reachable manifest graph, live payloads, fourteen tracks, repeatables, AND/convergence
  junctions, derived ordinary total, and exact thirty Hole studies;
- founding capability rules and locked-operation rejection;
- scholar physical task, Notes credit, preparation once, cancellation/death/restart, and zero award
  for blocked work;
- God queue topology, frozen front, reorder constraints, cascade/refund, offline/restart progress,
  and Leader overtake;
- Leader cadence, finite-first selection, God-target avoidance, critical override, exact
  `25/12/5/1/0` oopsie bands, refund, quota persistence, and hidden-truth twins;
- Notes/Void split and absence of every forbidden legacy authority in source, wire, schema,
  fixtures, UI, logs, and artifacts;
- all four boosts, duration/economy choices, checked ceil cost, overlap, same-type rejection, expiry,
  no permanent/report mutation, and restart;
- contribution/Ration/Water, Inspiration, construction press, population rescue, purpose binding,
  report gate, exact population count, idempotency, and physical conservation;
- protocol round trips/default rejection, server authorization/redaction/order, fresh persistence,
  multi-colony isolation, and save/load equality;
- one-worker Playwright plus an independent visible-browser run through the real Rust server, fresh
  SQLite, named Portless routes, shipped controls, accessibility trees, screenshots, console, and
  network evidence.

## Adding progression content later

Use [`extending-the-system.md`](extending-the-system.md), especially Recipes 5, 10, 11, 13–16, 20,
and 21. A progression extension is incomplete until it states:

1. stable manifest/study/effect/action/event/art IDs;
2. ordinary Notes versus Hole/Void authority and explicit reason;
3. prerequisites, region/track/level/junction/repeatable placement and deterministic order;
4. physical scholar/preparation/construction/site/route/cargo requirements;
5. Leader-lane eligibility, God-target avoidance, critical/oopsie behavior, and report inputs;
6. effect handler, scope, fixed-point math, stacking, expiry, and rollback;
7. public/owner/report/server-only fields and every forbidden hidden field;
8. expected-version lane, authentication, idempotency, and typed failures;
9. fresh-schema fields, restart/offline behavior, future/malformed rejection, and legacy absence;
10. graph/inspector/Hole visuals, art keys, accessibility labels, textual fallback, and browser
    checkpoints;
11. focused deterministic/restart/protocol/server/UI tests and final serialized campaign/browser
    evidence;
12. board rows, conflict receipts, maintained design links, and removal of any superseded path.
