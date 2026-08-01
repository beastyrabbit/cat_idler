# Historical LAI.0–34 diplomacy/trade proposal

> This Friendly/Allied/Blocked contract is superseded by the approved personal
> Alliance/Neutral/Enemy and moneyless physical barter design. Use
> [`diplomacy-barter.md`](diplomacy-barter.md). Do not restore mutual-consent alliances, old
> valuation bands, money, settlement, or the LAI.31 progression surface from the archived text
> below.

## Archived proposal (non-authoritative)

Diplomacy and trade are world-level, deterministic, belief-driven systems. `cat-sim` owns state,
contracts, escrow, cargo, and ordering; `cat-server` owns authentication and consent authorization.
No trade decision may read hidden authoritative inventory through a side channel.

## Relationships and consent

Relationships are `Neutral`, `Friendly`, `Allied`, or `Blocked`.

- Friendly and Allied require mutual consent from the authorized parties.
- Either player may block immediately.
- NPC trade continues through the existing NPC contract layer.
- A player-founded village initiates autonomous trade only while Friendly or Allied.
- Neutral villages do not initiate autonomous trade.
- Blocked villages cannot create or accept contracts.
- AI may trade for a founded village but never silently found a player-owned village.

Relationship proposals, approvals, blocking, actor/owner authorization, expected versions, and
idempotency IDs persist. Restart or Leader succession cannot downgrade, duplicate, or auto-approve
a relationship.

## Belief-based valuation

Valuation uses the same reported scarcity, flow, confidence, and age available to leadership and
the god. Hidden exact stock, destination headroom, source regeneration, or route danger cannot be
serialized into a private trade hint.

- Friendly contracts stay within ±10% of believed fair value.
- Allied contracts may accept up to a 20% believed disadvantage for strategic survival or defense.
- Mercantile and Self-sufficient personality changes preferences only within those bounds.
- Accountant freshness and confidence influence valuation; an uncertain or expired report can
  block/request recount rather than becoming an oracle.

Authoritative validation may reject with bounded categories such as unavailable source,
insufficient escrow, route blocked, or destination full. It never returns the hidden exact amount.

## Contract lifecycle

1. Persist a globally stable proposal ID, parties, believed valuation/evidence, offered/requested
   cargo, source/destination, route requirements, and expiry.
2. Obtain mutual acceptance where policy requires it.
3. Atomically reserve source resources and exact destination capacity. Escrow removes the amount
   from all other spending and contracts without teleporting it.
4. Resolve authoritative pickup/work positions, delivery endpoint, and a valid physical route.
5. Match a hauler and pick up exact cargo.
6. Move cargo physically across the route.
7. Deposit at the pinned endpoint and verify the counter-delivery/contract terms.
8. Complete atomically and release all reservations.

The world orders due contracts by next-event tick, then globally stable contract ID. Colony vector,
map, or worker iteration order cannot grant advantage.

## Cancellation, failure, and recovery

- Before departure, cancellation releases both resource and destination-capacity escrow.
- After departure, a contract cannot be freely cancelled.
- If a route closes, cargo attempts a validated physical return.
- If return is impossible, cargo remains physically stranded and recoverable with stable ownership;
  it is not deleted, credited, or duplicated.
- Death/refusal salvages or strands the exact cargo and releases the worker without completing the
  contract.
- Destination removal/fullness blocks delivery without nearest-destination recomputation.
- Relationship blocking stops new creation/acceptance; existing in-transit cargo follows the
  persisted cancellation/salvage contract rather than disappearing.

Proposal, consent, escrow, route, actor, cargo IDs, current stage, next-event tick, reservations,
blocked reason, and recovery state survive restart exactly. Revalidation at acceptance, departure,
stage transition, route change, and restart is atomic.

## LAI.31 diplomacy and trade UI contract

`LAI.31_PROGRESSION_UI_CONTRACT` defines the diplomacy and trade portions of the post-cutover Bevy
progression surface. LAI.31 depends on LAI.24 snapshots, LAI.25 actions, and LAI.27 server
authorization/redaction; this red contract does not add production client UI, protocol DTOs, server
routes, persistence fields, or `world_tick` integration.

The diplomacy surface shows relationship state, consent requirement, proposal status, acting
player/colony authorization, expected diplomacy version, expiry, and bounded block reason. Friendly
and Allied controls must visibly require consent from authorized parties; Block remains immediate
and never exposes hidden reasons from the other colony. Player-founded villages and foreign
private colony state remain isolated: no private beliefs, hidden inventory, exact regeneration,
private plans, rejected hidden amounts, unseen sites, or route danger can appear in labels,
tooltips, accessibility trees, screenshots, logs, or conflict feedback.

Trade UI rows show report-safe proposal value, report references, confidence/age/provenance,
offered/requested cargo summaries, escrow state, physical route, pickup endpoint, delivery endpoint,
stage, next-event tick, reservation summary, blocked reason, and recovery state. Escrow and cargo
are rendered as physical contract state, not as Favor, mirrored inventory, or a local optimistic
balance. Route failure, stranded cargo, return attempt, cancellation boundary, refusal/death
recovery, and restart recovery are visible through bounded stages without revealing exact hidden
stock or the losing colony in a reservation conflict.

Consent-required accept/reject, alliance approval, relationship change, and block controls use
authenticated LAI.25 action envelopes with protocol version, stable idempotency ID, colony/player
identity, expected diplomacy/trade versions, and strict bounded payloads. Stale, duplicate,
unauthorized, ownership-denied, malformed, route-blocked, insufficient-escrow, destination-full,
and precondition conflicts trigger refreshable typed feedback and never mutate local state
optimistically.

Playwright and visible-browser evidence use stable roles, labels, and test IDs for diplomacy
relationship controls, consent state, trade proposals, valuation report references, escrow, route,
cargo stage, recovery, stale conflict handling, multi-colony privacy, and restart checkpoints. The
browser tests operate shipped controls only and do not inject DOM state, synthetic snapshots,
private action endpoints, or hidden test hooks.

## Wire/UI and evidence

Snapshots expose relationship/consent state and report-safe contract values, parties, stages,
routes, cargo, reservations, and bounded failures. Actions cover relationship change/alliance
approval and accept/reject where consent is required; every mutation carries expected state version
and idempotency ID. Multi-colony redaction prevents a player from inspecting private beliefs or
inventories of another village.

Required tests prove mutual Friendly/Allied consent, immediate blocking, Neutral/Blocked initiation
denial, ±10% Friendly and 20% Allied bounds, belief-only valuation, escrow against double spend,
physical route/delivery, no cargo loss/duplication on failure, deterministic global ordering,
authorization/isolation, and exact in-transit restart equivalence.
