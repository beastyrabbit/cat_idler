# Historical only: superseded Shrine/Favor research design

> This file preserves pre-integration branch evidence only. It is not an implementation authority.
> The approved combined plans delete Shrine, Favor, Blessings, generic research points, scholar
> Insight currency, semantic currency migration, and the legacy progression UI. Use
> [`hole-research-progression.md`](hole-research-progression.md) for the maintained Hole,
> Research Notes, Void Insight, two-lane research, boost, miracle, wire, persistence, UI, test, and
> future-extension contract. No current code, schema, fixture, protocol, UI, or contributor recipe
> may cite the historical rules below as desired behavior.

## Archived branch proposal (non-authoritative)

The Shrine is an endless strategic demand and the physical source of Favor. `cat-sim` owns the
offering state machine, Favor ledger, research manifest/progress, Insight/preparation, and active
boosts. Server actions provide authentication, expected-version checks, and idempotency.

## Endless demand

There is no Shrine establishment-complete state, offering cooldown, tithe scalar, hard production
cap, or supernatural punishment. If a Shrine is absent, establishing it is foundational. If it
exists, the planner continually considers another offering whenever survival and active defense
permit. Only one offering pipeline per Shrine may actively reserve/consume resources.

Each package produces one base Favor before existing Shrine/ritual mastery multipliers:

| Package | Base Favor |
|---|---:|
| 20 Food | 1 |
| 5 Herbs | 1 |
| 10 Materials | 1 |
| 5 Refined resources | 1 |

The planner compares beliefs, never hidden inventory:

`utility = expected_favor ÷ (1 + replacement_hours + labor_hours / 6) − reserve_risk − committed_use_penalty`

Good leadership generally chooses the cheapest safely replaceable package. Poor leadership may use
stale estimates, misjudge regeneration, choose scarce Food, overcommit workers, trigger replacement
hunting/farming, or omit an eligible Shrine review. These failures come from beliefs, horizon,
personality, and the documented omission roll—not authoritative cheating.

Default non-emergency strategic weights are Shrine progress 0.85, sustainable growth 0.80, and
prosperity/comfort 0.50. Survival and active defense override them.

## Physical offering state machine

1. Select a package from beliefs and persist its rationale/evidence.
2. Reserve exact uncommitted physical resources.
3. Resolve their authoritative source and the Shrine delivery site.
4. Reserve source, route, Shrine capacity/slot, cargo, and hauler atomically.
5. Pick up and move cargo physically.
6. Deposit at the Shrine.
7. Perform ritual work at a reserved Shrine position.
8. Consume the deposited package atomically.
9. Credit Favor once with a unique idempotent ledger event.

Before pickup, cancellation releases reservations. After pickup, cargo is delivered to the pinned
Shrine when safe or salvaged to a safe owned stockpile before blocking. Favor is never cargo and
cannot be stolen, stranded, or traded. Retry/restart may replay observations but cannot consume or
credit twice.

The offering pipeline persists the physical cargo disposition: `ReleasedBeforePickup`,
`DeliveredToShrine`, or `SalvagedToStockpile { stockpile_id }`. Cancelled pre-pickup pipelines must
record release-before-pickup and cannot later credit Favor. Deposited, ritual, and completed
pipelines must record Shrine delivery before ritual credit is legal. Blocked picked-up pipelines
must record a bounded reason plus either Shrine delivery or a non-empty safe stockpile salvage
target; a blocked or salvaged pipeline never credits Favor.

The old immediate tithe decision, daily/cooldown behavior, and duplicate scalar blessing path are
removed at cutover. Manual offerings use the same physical pipeline and warn from reports; hidden
truth cannot silently prohibit a risky but physically available choice.

## Favor ledger and migration

Favor is the sole spendable currency for research and divine boosts after cutover:

`favor = legacy_global_upgrade_points + legacy_unspent_research_points`

This transactional, versioned conversion executes exactly once while preserving owned studies. The
migration marker prevents replay/double minting. The spendable research-point balance and competing
research/blessing purchase paths are removed.

Every credit/debit has a unique stable event/action ID. Favor is exact, never negative, and debits
use compare-and-set/atomic semantics. Duplicate action IDs, stale versions, unaffordable studies,
rejected boosts, or failed physical offerings cannot debit/credit. Favor is not mirrored into a
presentation balance, inventory, cargo, stockpile, escrow, or trade contract.

## Research manifest and purchase

Preserve the current 487-study catalog and add four 11-stage tracks—Divine Duration, Divine Economy,
Rehabilitation, and Administration—for exactly 531 studies. Manifest validation proves:

- 531 unique stable IDs and unique display names;
- deterministic ordering;
- an acyclic prerequisite graph with every study reachable from a starting frontier;
- every effect references a live handler;
- no orphaned deprecated study.

The player may purchase any visible prerequisite-ready frontier study whenever Favor is sufficient.
The committed undiscounted/discounted price is frozen during the action; later research cannot
retroactively alter it.

Before effective Loremaster support, the Leader may purchase one affordable study per rolling
seven game-days. Effective Loremaster levels 1 through 5 allow 1, 2, 2, 3, and 4 total automatic
purchases per rolling seven game-days. Quota timestamps belong to the colony, survive succession,
and do not reset on restart. Only affordable, prerequisite-ready nodes are candidates. An
unaffordable selection consumes no quota; unused quota does not carry into another window.
Selection uses beliefs, posture, personality, active dependencies, and expected value. Automatic
purchases pay full Favor and never consume a player preparation discount.

## Scholars, Insight, and preparation

Scholars unlock in late midgame through the documented research/building prerequisite; they are not
required for baseline Leader research. Each active researcher produces 20 Insight per completed
game-week, modified only by documented skill/Scholarship effects. Insight becomes colony-owned only
after completed physical study work.

Preparing a study costs Insight equal to that study's current undiscounted Favor cost. Preparation
does not stack or expire. A prepared study receives a 25% Favor discount only when purchased by a
player, and the marker is consumed atomically with that purchase. If a scholar dies, stored Insight
remains and active preparation may be reassigned. Scholars prioritize dependencies requested by
approved plans before speculative studies.

`Seasoned Scholar` is earned after producing 200 Insight and adds 10% to future Insight production.

## New track effects

- Divine Duration has a one-hour base plus 11 stages unlocking the next maximum durations:
  `1, 2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24` game-hours.
- Divine Economy reduces the cost of a newly purchased boost by 3% per stage, capped at 33%.
- Rehabilitation adds 2 prosthetic-restoration percentage points per stage, under the global 90%
  restoration cap.
- Administration begins with three standing-order slots and four concurrent non-emergency
  strategic intents. Each stage adds one standing-order slot. Stages 2, 4, 6, 8, and 10 each add
  one strategic-intent slot.

Research acquired after a price/effect is committed never changes that committed purchase or active
effect.

## Player-only divine boosts

Leaders and officers never activate or reserve spending for boosts. Only an authenticated player
may buy one:

| Boost | Village-wide effect | Base cost/hour |
|---|---|---:|
| Bountiful Labor | +50% raw gathering, carrying, and harvesting | 2 Favor |
| Fleet Paws | +50% movement | 1 Favor |
| Inspired Work | +50% construction and production | 2 Favor |
| Restorative Grace | +50% healing | 2 Favor |

`economy_multiplier = 1 − min(0.03 × Divine Economy stage, 0.33)`

`cost = ceil(base_hourly_cost × selected_duration × economy_multiplier)`

The player selects an unlocked duration; base duration is one game-hour. Different types may
overlap. Buying an already active type is rejected without debit; same-type stacking, duration
reset, cancellation, and refund are forbidden. Activation tick, exact end tick, paid cost, selected
duration, and purchased research stages are persisted. Price/duration stay fixed, later research
does not alter an active boost, and expiry occurs exactly at the authoritative simulation tick under
fine ticks, batched ticks, and restart.

## LAI.31 progression UI contract

`LAI.31_PROGRESSION_UI_CONTRACT` defines the Shrine, Favor, research, scholar, and boost portions of
the post-cutover Bevy progression surface. LAI.31 depends on LAI.24 snapshots, LAI.25 actions, and
LAI.27 server authorization/redaction; this red contract does not add production client UI, protocol
DTOs, server routes, persistence fields, or `world_tick` integration.

The Shrine surface renders only report-safe state from the authoritative snapshot. It shows an
endless offering pipeline with package, belief-derived bounded rationale, source report provenance,
source stage, haul stage, ritual stage, cargo disposition, pinned Shrine endpoint, omission reason,
and block reason. It must not show exact hidden stock, source regeneration, unrevealed capacity,
private foreign-colony beliefs, or any client-derived nearest-source fallback. Regeneration remains
unavailable below effective report level 4 in visible labels, tooltips, accessibility trees,
screenshots, logs, and conflict feedback.

Favor is exact and singular. The UI shows the micro-Favor ledger balance, event IDs, credit/debit
direction, committed action/event reference, current resource version, and bounded conflict state.
It must not mirror Favor into inventory, cargo, escrow, research points, a local optimistic balance,
or a second presentation currency; duplicate or stale action feedback cannot imply a hidden debit or
credit.

The research surface shows the 531-study frontier with stable study IDs, prerequisites, reachable
frontier status, committed undiscounted price, committed discounted player price when preparation is
valid, rejection reason, and dependency report references. It shows automatic seven-day quota
window start/end, used count, limit, candidate affordability, and the no-carryover boundary.
Insight and scholar state include active researchers, weekly Insight progress, preparation target,
stored preparation, reassignment, scholar death/reassignment feedback, and the player-only 25%
discount marker.

The Divine Boost surface contains exactly four player-only boost controls: Bountiful Labor, Fleet
Paws, Inspired Work, and Restorative Grace. Each control shows current unlocked duration options,
selected duration, cost in micro-Favor, effect stage, active start/expiry tick, economy stage used
for the committed price, enabled/disabled state, and bounded rejection feedback. Same-type active
boost purchase is disabled without a debit. Leaders and officers cannot emit boost actions, and the
client must not expose a Leader/officer boost affordance.

All progression mutations use authenticated LAI.25 action envelopes with protocol version, stable
idempotency ID, colony/player identity, expected planner version, expected resource/Favor version,
and strict bounded payloads. Stale, duplicate, unauthorized, insufficient-Favor, same-type-active,
malformed, unknown-version, and precondition conflicts trigger refreshable typed feedback and never
mutate local state optimistically.

Playwright and visible-browser evidence use stable roles, labels, and test IDs for Shrine offering,
Favor ledger, research frontier, scholar preparation, boost controls, and restart checkpoints. The
browser tests operate shipped controls only, capture before/after accessibility trees and
screenshots, and do not inject DOM state, synthetic snapshots, private action endpoints, or hidden
test hooks.

## Projection and required evidence

The UI shows report-safe offering rationale/status, exact Favor ledger summary, 531-study frontier,
automatic quota/window, Insight, preparation and discount, boost cost/duration/expiry, and typed
failures. It must not disclose hidden stock or regeneration in warnings.

Required proofs cover endless repeated offerings, absence of cooldown/tithe/completion gates,
belief-driven good and bad package choices, physical consumption before single credit, exact-once
migration, nonnegative CAS ledger, 531-node validation, affordable quota behavior, succession,
at least four automatic purchases in a normal affordable 30-day campaign, scholar death/reassign,
the 25% discount, all duration/economy stages, same-type rejection, committed-price invariance, and
batch/restart-exact expiry.
