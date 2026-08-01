# Planner, beliefs, and authority

This document owns the colony planner, its knowledge boundary, officer authority, and deterministic
ordering. All quantities below use simulation game time and integer/fixed-point arithmetic unless a
formula explicitly says otherwise.

## Persisted state and phases

Every colony persists one versioned `ColonyAiState` containing the planner schema version, planning
clock/epoch, posture, Leader and officer identities and effective levels, belief store and report
versions, live and terminal intents, typed officer requests, standing orders, current-epoch nudge,
resource/spatial/delivery/workforce reservations, retries/deadlines, colony-owned research quota,
family/governance obligations, God/Leader research-lane state, staged construction/storage plans,
Hole planning state, food permissions, barter posture, and bounded rationale/event fingerprints.

The planner runs these phases in order:

1. Select the long-horizon posture from current beliefs.
2. Generate domain goals.
3. Turn goals and officer requests into persistent intents.
4. Expand dependencies into executable tasks; reject cycles.
5. Resolve authoritative objectives, work positions, delivery endpoints, and routes.
6. Atomically reserve source quantities, exclusive sites, endpoint capacity, tools, slots, and
   workers.
7. Match the whole colony workforce.
8. Execute multi-stage physical tasks.
9. Account for completion, failure, cancellation, refusal, injury, and retry.
10. Publish bounded feedback into beliefs/reports and report-safe snapshots.

Offline advances process every crossed planning boundary chronologically. One large advance and
equivalent 1-second, 1-minute, 15-minute, or 1-hour partitions must produce the same outcome where
batching is supported.

## Postures and priority

| Posture | Trigger |
|---|---|
| `Defend` | Active attack or immediately credible hostile threat |
| `Crisis` | Less than one forecast day of an essential resource, inaccessible food/water, dangerous untreated injury, or another survival failure |
| `Recover` | Emergency ended, but reserves remain below two forecast days or injury/housing damage is unresolved |
| `Establish` | Required Hole access, food, water, storage, shelter, family housing, or basic production infrastructure is missing |
| `Stabilize` | Essentials cover two to four days but a bottleneck or unstable chain remains |
| `Grow` | Essentials exceed four days and population, storage, production, or territory is capacity-constrained |
| `Prosper` | At least seven stable forecast days, core infrastructure complete, and no higher deficiency |

Precedence is active defense, crisis, recovery, establishment, stabilization, growth, then
prosperity. Emergencies can be injected under any posture and are never omitted.

## Founding Leader domain planner

The founding Leader domain planner is a leaf planner that emits bounded report-safe goal records;
it does not enqueue runtime jobs until the LAI.23 single-path integration. Inputs are domain signals
containing only report-safe urgency, confidence, cost, churn, temporary player bias, rationale keys,
criticality, optional specialist role, and stable target identity.

Domain priorities use the same fixed-point score formula as intents. Strategic weights are:

| Goal | Base strategic weight |
|---|---:|
| `Defense` | 20,000 |
| `Survival` | 18,000 |
| `Hole` | 14,000 |
| `Growth` | 10,000 |

Posture bonuses are +5,000 for `Defend`/`Defense`, +4,000 for `Crisis` or `Recover`/`Survival`,
+3,000 for `Establish`/`Hole`, and +2,000 for `Grow` or `Prosper`/`Growth`. Criticality bonuses
are +5,000 emergency, +3,000 self-preservation, +1,000 required, and +0 optional. Emergency and
self-preservation goals sort ahead of score, so active defense can be injected even when its belief
confidence is low.

Personality weighting is multiplicative and report-safe:

- Cautious Leaders favor `Defense` and `Survival`; Bold Leaders down-weight them.
- Devout Leaders favor optional `Hole` dependencies; Skeptical Leaders down-weight them.
- Ambitious Leaders favor `Growth`; Content Leaders down-weight it.

The domain planner never reads hidden truth. Confidence is the report confidence supplied by the
belief/report layer, and serialized plans must not contain authoritative hidden quantities.

When a domain signal names a specialist role but that office is vacant, the founding Leader owns the
goal as `FoundingNoSpecialistFallback`. This fallback is intentionally imperfect: it keeps essential
domains from deadlocking before offices exist, while preserving the same omission, scoring,
authority, reservation, and later runtime gates. When the specialist is filled, the goal is annotated
with that officer owner instead.

Every included or omitted goal carries no more than eight explanation keys. Built-in explanation
keys cover emergency injection, founding no-specialist fallback, officer-request omission reduction,
and optional omission. These keys are rationale identifiers, not free-form hidden diagnostics.

## Intents, score, ordering, and bounds

The lifecycle is `Proposed → Approved → Reserving → Active → Succeeded`, with `Blocked`,
`RetryWaiting`, `Cancelled`, and `Failed` alternate states.

Each intent stores its stable ID, colony, proposer, authority domain, kind, target, rationale key,
evidence/report IDs, belief version, creation/review/deadline/terminal ticks, urgency, strategic
weight, confidence, expected benefit/cost, dependencies/dependents, spatial objective, resource and
delivery reservations, assigned cats/tasks, retry count/next retry tick, bounded reason, temporary
player bias, and standing-order provenance.

The fixed-point score is:

`score = urgency × strategic_weight × personality_weight × confidence − opportunity_cost − churn_penalty + starvation_age + temporary_player_bias`

- Scores use basis points; platform-dependent floating-point comparison is forbidden.
- Active non-emergency work remains unless a replacement is at least 15% better.
- Emergencies, route invalidation, and worker incapacity bypass hysteresis.
- Starvation aging adds 1 percentage point per game-hour, capped at 25 points.
- Stable ties resolve by intent kind, creation tick, intent ID, then target ID.
- Intent IDs derive from colony ID, planning epoch, kind, target, and occurrence index.
- Equivalent intents merge evidence and urgency.
- A colony holds at most 128 live intents and 256 terminal history entries. Terminal eviction is
  oldest completion tick, then stable ID.

Retry delays are exactly 15 minutes, 30 minutes, 1 hour, 2 hours, and 4 hours. Five failed attempts
cause terminal failure unless a changed belief, route, resource, building, or dependency makes a
materially new intent. Permanent invalidity fails immediately. Reset, death, or succession never
rewinds retry history or duplicates an intent.

## Cadence, expertise, and omission

| Effective level | Leader cadence | Forecast horizon | Officer cadence |
|---:|---:|---:|---:|
| 1 | 12 hours | 6 hours | 6 hours |
| 2 | 6 hours | 12 hours | 3 hours |
| 3 | 3 hours | 24 hours | 1 hour |
| 4 | 1 hour | 48 hours | 30 minutes |
| 5 | 30 minutes | 72 hours | 15 minutes |

Personal office levels require 0, 24, 96, 240, and 480 completed duty hours. Only real completed
office work grants experience. Operational `Workflow` and `Reinforcement` research for the required
room/tool each add one effective level; `min(5, personal level + room/tool bonuses)` is authoritative.
Experience stays with the cat after removal.

Optional non-emergency omission per eligible review is 25%, 12%, 5%, 1%, and 0% at effective levels
1 through 5. It is rolled once per review using the dedicated omission RNG stream. Officers keep
domains visible through requests, reducing omission pressure, but no omission may suppress defense
or self-preservation. A valid non-expired officer request covering the same domain and goal advances
optional omission exactly one band: 12%, 5%, 1%, 0%, and 0% at effective levels 1 through 5. It does
not force approval or bypass authority, resources, reservations, or hidden knowledge; the review
still makes exactly one omission roll after selecting the applicable band.

## Deterministic RNG and collections

All randomness uses the project LCG. Planning omission/error, appointment sampling, personality,
and injury use isolated forks so additions do not perturb movement, life, or raid streams. A draw is
keyed from stable inputs—world seed, colony ID, cat ID when applicable, domain/kind, and review or
occurrence bucket—instead of call order. Appointment sampling is additionally keyed by role and
vacancy occurrence. Maps/sets that affect results must be ordered explicitly; no hash iteration,
wall clock, `rand`, or platform float ordering may influence state.

## Belief contract

Authoritative executor truth is private. A belief stores subject/domain, estimate or category,
lower/upper bounds where applicable, trend, confidence in basis points, observation/expiry ticks,
source type, reporter ID, evidence IDs, report level, and contradiction version. Production,
consumption, source capacity, depletion, and regeneration appear only when valid observations and
expertise permit them.

| Level | Stock/resource estimate | Flow information | Regeneration |
|---:|---|---|---|
| 1 | Broad band, about ±40% | None | Hidden |
| 2 | About ±25% | Rising/stable/falling | Hidden |
| 3 | About ±12% | Coarse observed inflow/consumption-rate range | Hidden |
| 4 | About ±5% | Numeric observed rate | Explicit estimate, about ±25% |
| 5 | About ±2% | High-confidence numeric rate | Estimate, about ±10% |

Level 3 flow is observed throughput, never terrain regeneration. Level 5 still does not reveal
unseen terrain or exact hidden truth.

Default expiry is:

- route and active threat: 1 hour;
- stock: 6 hours;
- production/consumption: 12 hours;
- regeneration: 24 hours;
- discovered static sites: persistent until physically invalidated.

Update precedence is newer direct observation, newer authorized officer report, older direct
observation, older report, then stable reporter ID for equal timestamps. After expiry, confidence
decays linearly by 500 basis points for each full subject-specific expiry interval elapsed, floored
at zero. Direct physical invalidation sets confidence to zero immediately. Newer contradictory
evidence replaces the estimate and marks prior evidence superseded.

Outside-source knowledge requires physical scouting, domain inspection, or another authorized
observation/report. Hole delivery proves only the delivered cargo and its accepted value; it never
reveals source stock or regeneration.
Stock knowledge comes from Accountant rounds; measured cycles produce flow reports. Hidden
regeneration changes cannot affect plan or UI before a valid level-4-or-higher report.

Execution may inspect truth only to enforce physical rules. Feedback is a bounded category such as
`SourceUnavailable`, `RouteBlocked`, or `DestinationFull`, never an exact hidden amount,
regeneration value, undiscovered site type, or unseen hostile fact. Two colonies with identical
beliefs and different hidden truth must plan identically until permitted evidence arrives.

The God/player consumes this same projection. Exact owning-player Research Notes and Void Insight
are visible because they are progression ledgers, not physical inventory. No snapshot, tooltip,
inspector, validation message, plan
explanation, research screen, trader hint, debug string, or client cache may contain greater truth.

## Authority and officer requests

| Actor | Authority |
|---|---|
| God/player | Broad temporary nudges, research queue/preparation, boost/Inspiration/miracle/aid actions, one election backing block, personal diplomacy stance, and authorized expulsion; never exact routine work control |
| Leader | Create, approve, reprioritize, cancel, and retry colony-wide intents; final labor/resource arbitration |
| Domain officer | Plan inside domain/budget and request cross-domain work |
| Acting Steward | Survival and evacuation only while the Leader office is vacant |
| Cat | Accept or refuse based on eligibility, stress, risk, and personality |
| Scheduler | Execute, block, and retry approved plans; never invent strategy |

The founding Leader can cover all essential domains imperfectly. Specialist offices are:

- Steward: reserves, storage/hauling/logistics, housing, population care, and administration.
- Accountant: counts, observed production/consumption, regeneration reports, budgets, valuation,
  and freshness.
- Forester: logging, replanting, woodland reserves, quarry/raw construction resources, and
  sustainable extraction.
- Farmer: fields, food, water, plant/animal supply, forecasts, and famine prevention.
- Captain: defense, raids, training, weapons/armor, patrols, dangerous-work policy, and recovery.
- Loremaster: Hole throughput, Notes/Void research lanes, scholars, preparation, technology advice, and knowledge
  coordination.
- Cloth Leader: fibre, thread, cloth, leather, clothing, and the related production/storage chain.

The Leader uses the free research lane/building permits, starts physical prerequisites, and
appoints a living candidate when an office first becomes available. Effective levels 1–5 inspect
3, 5, 8, 12, and all eligible cats respectively.
The dedicated appointment RNG samples without replacement, keyed by colony, role, and vacancy
occurrence; the Leader chooses the best believed candidate inside that sample. This makes weak
appointments deterministic without an execution coin flip. The AI does not casually replace a
filled office; death, vacancy, or invalidation triggers succession. Gods cannot directly appoint,
replace, or vacate an officer. The fresh-schema cutover carries no semantic appointment migration.

Officer requests persist ID, officer/domain, target/quantity, urgency, rationale, evidence/report
IDs, confidence, estimated resource/labor cost, dependencies, creation/expiry ticks, and one of
`Proposed`, `Accepted`, `Rejected`, `Fulfilled`, `Superseded`, or `Expired`. Equivalent requests
deduplicate. An unanswered request gains 1 percentage point of urgency for each full game-hour,
capped at +25 points; it uses the same integer full-hour calculation as intent starvation aging.
Default lifetime is 48 game-hours; survival/active-defense requests live 6 game-hours; research,
building, diplomacy, and trade strategic requests live 7 game-days. Cross-domain cycles fail. An
officer may escalate but never bypass another domain's reservations.

## Succession and player influence

On Leader death, safe active work and approved officer work continue. No new non-survival
colony-wide strategy is approved; the Steward may approve survival/evacuation. When an eligible cat
exists, election/succession completes within six game-hours. The successor adopts still-valid
intents, re-scores from current beliefs, and cancels only through the normal lifecycle. Notes/Void
ledgers, both research lanes, election/office history, standing orders, planner history, families,
construction/storage state, and reservations belong to the colony and cannot reset.

Officer requests remain attributable after death/vacancy and may be adopted. Invalid domain
reservations release atomically.

The Council Plans tab exposes the top eight report-safe intents, dependencies, task state, complete
site/route geometry, score factors, evidence, and bounded block/recovery reason. Gods may send only
the approved broad temporary encouragement/conservation nudges; they cannot move, dismiss, inject,
or retarget an exact intent, building, road, crop, storage zone, production queue, food permission,
officer, worker, site, or route. Standing policy remains Leader-owned and cannot bypass hidden
knowledge, eligibility, reservations, Notes/Void affordability, research-lane authority, refusal,
or physical rules.
