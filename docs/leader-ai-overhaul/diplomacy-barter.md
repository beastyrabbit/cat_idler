# Personal diplomacy and moneyless physical barter

This is the maintained diplomacy/barter contract for the integrated plans. It supersedes the
historical [`diplomacy-trade.md`](diplomacy-trade.md) Friendly/Allied/Blocked proposal. There are no
coins, purses, prices, settlement, debt, or implied military/migration alliances.

## Stance authority and honest scope

Each authenticated player owns a personal stance toward each foreign village:

| Stance | Current behavior | UI wording requirement |
|---|---|---|
| `Alliance` | behaves exactly like Neutral in this release | explicitly says that defense and migration are not implemented |
| `Neutral` | village remains an eligible barter counterpart subject to reports, route, capacity, and utility | describes physical barter only |
| `Enemy` | excluded from outbound candidates; a destination with this personal stance rejects before dispatch | states that no caravan or escrow was created |

The colony/global village stance is always Neutral. It is separate from each personal player's
stance. One player cannot change another player's view, reveal their private state, or make a
global military decision.

The God may set only their authenticated personal stance. The Leader decides whether the colony
proposes or accepts a material barter from report-safe beliefs. There is no exact player-selected
offer, requested lot, route, caravan, worker, endpoint, or execution timing.

## Candidate and pre-dispatch gates

An autonomous proposal candidate must satisfy all of:

1. both villages are discovered through valid report-safe contact;
2. the origin personal stance does not exclude the destination;
3. the destination personal stance is not Enemy for the acting player;
4. the Leader has a report-safe belief about need and an offered physical surplus;
5. the offered/requested content is legal, unlocked for use where required, physically owned, and
   not already reserved, equipped, carried, consumed, purpose-bound, or spoiled;
6. source and destination storage compatibility/capacity are believed available;
7. a report-safe possible route and eligible carrying plan exist;
8. no active defense/survival/committed-work policy preempts departure;
9. one bounded proposal/contract limit and stable duplicate key permit creation.

The authoritative server/simulation revalidates current stance, ownership, exact identities,
headroom, route, actors, and capacity before any escrow or caravan is created. Destination Enemy,
unknown/future action versions, ownership failure, or forbidden content rejects before dispatch
with no partial reservation or visible fake caravan.

## Belief-based possible-now versus better-trade decision

The AI compares at least two explicit alternatives:

- accept/propose a possible trade using currently believed available materials;
- wait for a believed better barter or satisfy the need locally.

The fixed-point score records every input category:

- urgency and expected need relief;
- offered/requested utility and canonical value;
- quality, condition, age/spoilage, and provenance relevance;
- believed replacement cost and reserved/committed-use opportunity cost;
- distance, route time, carrying capacity, number of haul legs, and worker opportunity cost;
- report confidence/age and expected recount/scouting delay;
- route danger, stranding/recovery risk, destination reliability, and contract deadline;
- Leader personality, Intelligence, Trading/Logistics skill, officer report quality, and current
  posture;
- stable proposal/content/village IDs as final tie-breaks.

All values are report inputs. Hidden stock, regeneration, exact private headroom, unseen hazards,
private foreign plans, or authoritative future production cannot influence the score until a legal
observation/report exists.

A strong Leader tends to choose the lower expected total cost. A weak Leader may offer a poor mix,
wait too long for an unlikely better trade, accept unfavorable quality, omit a review, or discover
after authoritative validation that a believed source/route is unavailable. Those mistakes create
bounded blocked/recount/local-production/recovery work rather than a hidden corrective oracle.

## Physical proposal and contract identity

Every proposal and accepted contract has stable IDs for:

- origin/destination colony and authenticated personal stance context;
- proposal, acceptance/rejection, contract, action, and idempotency receipt;
- valuation report/evidence references and frozen bounded rationale;
- exact offered/requested content, quantities, quality/condition constraints, and finite item IDs
  where applicable;
- source locations, destination storage endpoints/capacity, routes/segments, caravan/hauler slots,
  cargo, reservations, deadlines, stages, and recovery;
- next due tick, rules version, and expected-version lanes.

Material barter has no price or settlement side. Both physical sides are terms of one contract.
Escrow means exact owned identities/quantities and destination headroom are unavailable to other
work; it is not currency or aggregate value.

## Atomic reservation and lifecycle

Acceptance performs one staged transaction:

1. revalidate stance, authority, reports required for public rationale, and current contract limits;
2. resolve exact offered/requested identities and compatible destination headroom;
3. resolve authoritative pickup/work positions, delivery endpoints, complete routes, and actors;
4. atomically reserve both sides' sources, quantities/items, destination capacity, route conflicts,
   caravan/hauler slots, and cargo;
5. persist the accepted terms, frozen valuation explanation, and next due transition;
6. create visible pickup tasks only after commit.

Execution uses explicit stages:

`Proposed → Accepted → Reserving → Pickup → Outbound → Exchange/CounterPickup → Return/Delivery → Completed`

Blocked/rejected/recovering terminal paths are typed. Due work orders by `(next_due_tick,
contract_id, actor_id)`. Colony vector, collection, map, or worker iteration order cannot choose a
winner.

Cargo moves through exact source, cargo, exchange, destination, return, or typed last-land-cache
locations. Quality, age, condition, provenance, lot/item identity, reservation, and contract
binding survive every stage.

## Cancellation, refusal, death, closure, and recovery

- Before pickup, cancellation releases both sides' identities, routes, slots, and headroom.
- After pickup, ordinary free cancellation is unavailable.
- Route closure triggers revalidation; if no permitted route exists, cargo attempts a validated
  physical return.
- Carrier refusal releases the worker after safe deposit/stranding rules; it does not complete the
  contract or erase cargo.
- Carrier death puts the same cargo into the declared recovery location and creates visible
  recovery work.
- Destination loss/fullness blocks at the pinned endpoint; it does not choose a new hidden
  destination.
- If return is impossible, the cargo remains owned, physically stranded, and recoverable at a
  typed last-land cache.
- A newly Enemy destination prevents only not-yet-dispatched work. An already physical caravan
  follows the persisted safe return/recovery contract; it never vanishes.
- Duplicate completion, acceptance, cancellation, or recovery replays the stored result and never
  transfers twice.

Every rejection/failure leaves a validated full candidate state or no mutation. No identity may
exist in escrow and world inventory simultaneously.

## Planner and officer ownership

The Leader owns final proposal/accept/wait/local-production choice and resource/labor arbitration.
The Accountant provides report-safe counts, quality/age/value, replacement estimates, and storage
pressure. The Steward provides hauling, route, capacity, and caravan logistics. Other officers may
request materials or report danger but cannot create a contract or reserve another domain.

God stance is an input gate, not a trade command. Officer death/vacancy, Leader succession, restart,
or report expiry cannot auto-approve or duplicate a proposal. A successor may adopt a still-valid
contract/intent with unchanged terms or cancel through its physical lifecycle.

## Protocol and server actions

Snapshots expose only authorized:

- personal Alliance/Neutral/Enemy stance and the honest present-scope explanation;
- report-safe foreign village contact identity;
- proposal/contract IDs, bounded valuation factor categories, report confidence/age/provenance;
- offered/requested public cargo summaries;
- escrow/reservation summary, route and complete pickup/delivery geometry, caravan/hauler,
  physical stage, next due tick, blocker, and recovery;
- own cargo identity/quality/condition/provenance where authorized.

They never expose foreign hidden stock/headroom/regeneration, private beliefs/plans/stance of
another player, unseen route danger, reservation loser, exact failed quantity, or developer trace.

The only direct diplomacy mutation is authenticated personal stance. Contract accept/reject actions
exist only where the final authority explicitly permits player participation; ordinary AI material
barter remains Leader-owned. Every action uses protocol version, principal/colony, stable action ID,
expected diplomacy/barter version, strict bounded payload, and stored accepted/rejected replay.

Server validation is protocol → authentication → personal ownership → selected-colony/domain
authority → expected version → idempotency → current simulation preconditions → staged commit.

## Persistence and fresh cutover

Persist personal stances separately from global Neutral, contact/report references, proposal/
contract terms, valuation explanation, escrow, exact cargo/locations, routes, actors, stages,
deadlines, next event, recovery, rules version, expected versions, and bounded action/transition
receipts.

Fine ticks, large ticks, restart, reconnect, and offline catch-up must agree on dispatch, movement,
exchange, return, delivery, and recovery. Unsupported future/malformed state fails closed.

Fresh schema/fixtures contain no coin, purse, price, settlement, debt, old relationship consent,
NPC merchant money, or compatibility translation. A known obsolete gameplay schema uses the
authorized reset/recreation path; there is no semantic conversion from money to physical barter.

## Village/Council UI and world visualization

Village and Council Trade/Diplomacy surfaces show:

- personal stance radio controls with honest Alliance text;
- contact/report provenance and stale/recount state;
- possible-now versus better-trade rationale factor rows;
- exact authorized offer/request quality and physical identities;
- escrow, storage headroom summary, caravan/hauler, route, pickup/delivery footprints, stage,
  deadline, blockers, safe return, stranded cache, and recovery;
- pending/stale/duplicate/unauthorized/Enemy/route/capacity feedback without hidden data.

World markers are created only for committed open physical tasks/contracts. They use the server-
supplied route, pickup, delivery, caravan, and cargo stages and despawn after terminal state.
Rejected Enemy work shows no fake caravan or marker.

Controls have stable roles/labels, keyboard/mouse/trackpad support, focus, centralized Escape,
disabled reasons, loading/empty/reconnect states, and textual fallbacks. The client never computes
a valuation, route, capacity, settlement, or success.

## Required evidence

Focused and final serialized evidence proves:

- personal stance isolation, global Neutral, Alliance=Neutral honesty, Enemy outbound exclusion and
  destination rejection before escrow/caravan;
- every score factor, belief-only hidden twins, weak/strong choices, omission, stable ordering, and
  no oracle fallback;
- proposal/contract uniqueness, atomic two-sided reservation, exact physical conservation, quality/
  condition/provenance, destination capacity, and cross-colony route conflicts;
- every lifecycle/failure/recovery stage, refusal/death/closure/Enemy transition, restart/partition,
  bounded receipts, and duplicate replay;
- protocol strict round trips, header-first authorization, redaction/leak scans, personal ownership,
  multi-colony isolation, fresh schema, future/malformed rejection, and complete money absence;
- Village/Council/world markers, honest text, accessibility, stale/reconnect/despawn behavior;
- real Rust server/fresh SQLite/Portless, one Playwright worker, then independent visible browser
  with two authorized sessions, screenshots, accessibility trees, console, network, and restart.

## Adding a stance or barter behavior later

Follow Recipes 12–16 in [`extending-the-system.md`](extending-the-system.md). Record stable IDs,
present-scope semantics, actor authority, report inputs, fixed-point scoring, physical terms,
complete sites/routes/cargo, reservation/conservation, failure/recovery, redaction, version lane,
fresh persistence, UI/art/accessibility, focused/restart/campaign/browser tests, and a board/conflict
receipt. Never add a UI promise—such as mutual defense—before its physical simulation, protocol,
persistence, and acceptance path exists.
