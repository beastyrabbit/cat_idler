# Wire, persistence, authorization, and UI

> Historical LAI.24–31 cutover contract. The approved Plan 1+2 integration replaces its
> Shrine/Favor, semantic-migration, direct-action, and older client assumptions. Use
> [`integrated-implementation-map.md`](integrated-implementation-map.md) and
> [`hole-research-progression.md`](hole-research-progression.md) as current authority.

This document owns the public contract and atomic cutover. `cat-protocol` defines versioned wire
types, `cat-server` authorizes/actions/persists, and `cat-client` renders only the received
report-safe snapshot. Simulation types remain in `cat-sim` leaf modules.

## Target simulation and snapshot contracts

Versioned simulation state includes `ColonyAiState`, `PlannerState`, `Intent`/`IntentStatus`,
`BeliefStore`, `Observation`, `OfficerReport`, `OfficerRequest`, `SpatialObjective`, `SiteRef`,
`TaskFootprint`, `WorkSlot`, `WorldReservationLedger`, `VisibleTask`, cat personality/stress/traits/
anatomy/prosthetics, `FavorLedger`, `ResearchProgress`, `ScholarPreparation`, `DivineBoostState`,
`DiplomacyState`, and `TradeContract`.

Snapshots add only authorized projections:

- belief/report summaries, confidence, ranges, age, and provenance without hidden truth;
- top plan queue, dependencies, officer requests, rationale, expected cost/benefit, and reasons;
- `VisibleTaskSnapshot` and `CatSnapshot.active_task_id`;
- cat attributes, personality, stress/refusal, injuries/anatomy, prosthetics, and care status;
- physical Shrine offering pipeline and exact Favor ledger summary;
- 531-study frontier, automatic quota, Insight/preparation, and active boosts;
- diplomacy/consent and physical trade contracts.

No exact physical stock, rate, capacity, depletion, regeneration, unrevealed site, unseen threat, or
other authoritative field may be shipped and merely hidden by Bevy. Favor is exact by design.

`SiteRef` round-trips every supported variant and canonical tile order. Workshop JSON must encode
width 3, height 3, and all nine objective tiles. Unknown versions/variants fail closed.

## LAI.24 snapshot schema contract

LAI.24 owns the post-cutover report-safe snapshot schema in `cat-protocol`; LAI.23 must land the
single simulation path first, and LAI.25/LAI.27/LAI.26 own actions, UI, and persistence integration.
The production implementation must bump the protocol version away from the legacy value and expose a
versioned `LeaderAiSnapshotEnvelope` with a nested schema version. Unknown protocol versions,
unknown tagged variants, unknown object fields, malformed bounds, and invalid private-state fields
fail closed before any client or server compatibility path can use them.

The root envelope carries only authorized projections: `now`, `worldSeed`, selected colony, public
village summaries, and one `ColonyAiSnapshot` per visible colony. A colony snapshot contains
capabilities plus `reports`, `plans`, `officerRequests`, `visibleTasks`, `cats`, `shrine`, `favor`,
`research`, `boosts`, `diplomacy`, and `trade`. It never contains another colony's private beliefs,
hidden inventory, private plans, owner session identifiers, authentication material, unseen threats,
unrevealed sites, or exact stock/regeneration truth.

Belief/report DTOs use `BeliefReportSnapshot` with `ReportEstimateSnapshot` ranges, units,
confidence in 0..=10,000 basis points, report age, observation/expiry ticks, report level, source
provenance, contradiction/replacement metadata, and bounded unavailable reasons. Regeneration uses
an explicit `RegenerationReportSnapshot`: below effective report level 4 the only legal value is
`UnavailableBelowLevel4`; level 4 or higher may show a report-derived range with provenance, never
the authoritative regeneration timer or hidden source capacity.

Planning DTOs expose the bounded top queue: stable plan/intent IDs, lifecycle state, responsible
Leader/officer actor, dependencies, score bucket, rationale, expected cost/benefit, and bounded
reasons. `OfficerRequestSnapshot` includes office/domain, requested action, budget/priority,
source report IDs, expiry, merge/supersession status, and bounded block reason. `VisibleTaskSnapshot`
contains task ID, intent ID, category, stage, assigned cat IDs, objective `SiteRef`, work slots,
pinned endpoint, footprint, progress basis points, reservation summary, bounded block reason, cargo
summary, and last-update tick. `CatSnapshot.activeTaskId` links a cat to a visible task without
turning the cat's destination into authority.

`SiteRefSnapshot` is a strict tagged enum. It must round-trip exact tiles, anchored rectangles,
canonically ordered tile sets, building ID/anchor/canonical footprint, stockpile ID/footprint,
resource-source ID/kind/footprint, Hunt cave/source, Fetch Water source plus reachable dry bank,
ordered route/road segments, Shrine endpoint, village endpoint, and trade endpoint. Each reference
has a stable ID, kind, lifecycle stage, visibility, and optional bounded blocked reason. Workshop
references additionally include `width: 3`, `height: 3`, and all nine row-major objective tiles.

Cat DTOs add report-safe traits and care: migrated innate attributes, learned skills and office
experience references, personality axes, acquired traits, stress/recovery/refusal state, willingness
breakdown, anatomy parts, injury/treatment status, fitted prosthetic IDs and restoration/durability,
care-site/fitting/repair task references, and bounded eligibility reasons. They do not recalculate
capability from hidden truth in the client.

Shrine/Favor/research DTOs expose the endless physical offering pipeline, package, source report
IDs, cargo/ritual stage, pinned Shrine endpoint, cargo disposition, rationale, exact micro-Favor
ledger balance/events, 531-study manifest/frontier summary, automatic rolling seven-day quota
window/used/limit, Insight balance, scholar work/preparation/reassignment, committed discount, and
active Divine Boost type, price in micro-Favor, duration, effect stage, start, and expiry. Favor is
the only exact currency in this surface and is never physical cargo, stockpile contents, escrow, or
a second mirrored balance.

Diplomacy/trade DTOs expose public relationship, consent, proposal, contract, escrow, route, actor,
cargo, stage, next-event tick, reservation summary, bounded failure, and restart-visible recovery
state. Valuation evidence references reports and confidence; it never serializes hidden exact stock,
private destination headroom, source regeneration, route danger, or a rejected amount.

The focused LAI.24A red harness is `crates/cat-protocol/tests/lai24_snapshot_contract.rs`. Its
expected pre-implementation failures are the missing envelope, report DTOs, plan/request/task
projection, `SiteRef` variants, Workshop footprint validation, cat care projection, Shrine/Favor/
research/boost payloads, diplomacy/trade payloads, bounds validators, unknown-version/variant
rejection, and multi-colony private-state guard.

## Actions and concurrency

Add authenticated, versioned actions to:

- reprioritize or dismiss an intent;
- create, update, or delete a standing order;
- purchase a frontier study;
- activate a divine boost;
- change diplomacy or approve an alliance;
- accept/reject trade where consent is required;
- fit or repair a prosthetic when player-facing.

Every mutating action carries an idempotency ID and expected planner/domain state version. Server
ordering is authoritative. The server checks protocol version, authentication, colony/player
ownership, action authorization, expected version, duplicate ID/result, current state/preconditions,
then performs one atomic mutation. Stale concurrent actions return a typed conflict and the client
refreshes. A repeated accepted ID returns the original result without another debit, credit,
reservation, item move, or relationship change.

Old protocol clients receive a clear `UPDATE_REQUIRED` response and cannot enter a compatibility
mutation path.

## LAI.25 action protocol contract

LAI.25 owns the post-cutover mutation contract after LAI.24 establishes the snapshot DTOs. It must
replace direct legacy action payloads with a `LeaderAiActionEnvelope` that carries
`protocolVersion`, `idempotencyId`, selected `colonyId`, authenticated `playerId`, expected planner,
domain, resource/Favor, spatial, and reservation versions, and one strict tagged
`LeaderAiActionPayload`. Unknown action versions, variants, object fields, malformed IDs, out-of-
bounds numbers, and missing identity/version fields fail closed before compatibility code can route
the mutation.

The tagged action payload covers all player-facing mutations:

- plan nudges, standing-order create/update/delete, and one-epoch intent dismissal;
- officer appointment/removal and any explicit authority transfer or override;
- treatment, prosthetic fitting, and prosthetic repair;
- Favor-funded research purchase and scholar study preparation;
- player-only Divine Boost activation;
- diplomacy relationship change, alliance approval, and immediate blocking;
- consent-required trade acceptance and rejection; and
- the existing physical placement/designation domains: building placement, farms, stockpiles,
  gather/fishing spots, roads, bridges, rail, docks, and transport routes.

Every legacy physical placement domain moves under the same envelope rather than keeping a parallel
mutation path. Placement payloads include typed bounded site targets, expected spatial and
reservation versions, and action-specific bounds for rectangles, ordered paths, exact endpoints,
resource kinds, cargo amounts, and worker IDs.

The authoritative validation pipeline has exactly one order:

1. `check_protocol_compatibility`
2. `check_authentication`
3. `check_colony_ownership`
4. `check_action_authority`
5. `check_expected_versions`
6. `check_duplicate_replay`
7. `check_current_preconditions`
8. `commit_favor_or_reservation`

No action may debit Favor, reserve a site, move cargo, appoint an officer, alter diplomacy, or
return a duplicate result before the earlier checks pass. Duplicate accepted IDs return the original
result after expected-version matching and before current preconditions, so retrying a completed
mutation cannot fail merely because the world has advanced. Duplicate rejected IDs return the same
bounded rejection and also do not mutate.

Conflict DTOs are typed and bounded: `UpdateRequired`, `Unauthorized`, `OwnershipDenied`,
`AuthorityDenied`, `VersionMismatch`, `DuplicateReplay`, `PreconditionFailed`,
`InsufficientFavor`, `ReservationConflict`, `MalformedActionId`, and `UnknownActionVariant`.
Stale-version conflicts include the authoritative current version and a bounded state hint sufficient
for refresh, but never exact hidden stock, regeneration, rejected amount, unseen site, unseen threat,
private route danger, authentication material, or the identity of a competing colony/reservation
loser. `UpdateRequired` includes minimum supported and current protocol versions plus the stable
`UPDATE_REQUIRED` code used by old clients.

The focused LAI.25A red harness is `crates/cat-protocol/tests/lai25_action_contract.rs`. Its
expected pre-implementation failures are the missing action envelope, action payload variants,
physical-placement wrapping, validation pipeline symbols/order, typed conflict DTOs, strict bounds,
unknown-version/variant/malformed-ID rejection hooks, and explicit player-only versus
Leader/officer authority markers. No production protocol, server routing, persistence, or
world-tick work is part of LAI.25A.

## Authorization and redaction

- Gods may nudge queues, maintain standing orders, buy research/boosts, and approve/block diplomacy
  only for authorized villages.
- Leaders/officers act through simulation authority, never by forging a player action.
- Boost activation is player-only.
- Officer and Leader controls cannot bypass a cat's refusal, route/site eligibility, Favor CAS, or
  reservation conflicts.
- Multi-colony snapshots/actions expose public relationship/contract facts and that colony's
  permitted reports, not another colony's beliefs, hidden inventory, or private plans.
- Validation errors are bounded categories and never leak the authoritative amount or unseen fact
  that caused rejection.

Every API/UI field receives an explicit leak-audit owner under LAI.9/LAI.27.

## LAI.33A-SYS signed restart and multi-colony journey contract

`LAI.33A_SYS_SIGNED_SYSTEM_JOURNEY_CONTRACT` is the server and persistence acceptance contract for
the post-cutover signed system journeys. LAI.33 owns execution after LAI.24 snapshot DTOs, LAI.25
action envelopes, LAI.26 migration, LAI.27 server routing/redaction, and LAI.32 campaign fixtures
exist. The red harness is `crates/cat-server/tests/lai33_signed_system_journey_contract.rs`.

The production implementation must expose `Lai33SignedSystemJourneyHarness` with deterministic
fixtures for `run_lai33_fresh_startup_journey` and `run_lai33_migrated_startup_journey`. It records
`SignedJourneyActionOrder` entries for every signed player action, `Lai33RuntimeProtocolFingerprint`
for aggregate runtime/protocol equality, and `record_lai33_expected_ids_ticks_versions` plus
`record_lai33_sqlite_checksum_before_after` at every checkpoint. No journey may bypass
authentication, expected versions, idempotency, persistence, or server-side redaction.

Restart coverage must include `restart_at_every_visible_task_stage`,
`restart_at_offering_source_haul_ritual_stages`,
`restart_at_research_purchase_preparation_stages`,
`restart_at_prosthetic_fit_repair_stages`, and
`restart_at_boost_diplomacy_trade_stages`. These stages cover visible task resolve/reserve/travel/
work/deposit, offering source/haul/Shrine ritual/Favor credit, research purchase and scholar
preparation, prosthetic fitting and Workshop repair, boost purchase/expiry, diplomacy consent, trade
escrow/pickup/delivery, and trade failure/recovery.

Signed action journeys must provide `replay_authenticated_idempotency_ids`,
`assert_replayed_action_returns_identical_prior_result`,
`concurrent_stale_expected_versions_refresh_without_mutation`,
`assert_no_partial_mutation_on_stale_action`, `old_client_update_required_journey`, and
`assert_update_required_before_auth_or_decode`. Duplicate accepted and rejected action IDs return the
same bounded result recorded at the original action order, without another debit, credit,
reservation, item move, cargo move, relationship mutation, or error leak. Concurrent stale versions
return refreshable conflicts and no partial mutation. Old clients receive `UPDATE_REQUIRED` before
auth lookup or action decode.

Isolation and redaction journeys must include `multi_colony_reservation_site_id_isolation_journey`,
`multi_colony_trade_isolation_journey`, `server_side_regeneration_below_l4_redaction_journey`,
`server_side_hidden_inventory_plan_redaction_journey`, `malformed_row_rollback_quarantine_journey`,
`assert_opaque_existence_safe_errors`, and `assert_no_hidden_fields_in_snapshot_error_log`. These
journeys prove selected-colony ownership, world-scoped reservation/site ID isolation, public versus
private trade facts, regeneration hidden below effective report level 4, hidden inventory and plans
absent from snapshots/errors/logs, malformed required rows rolled back or quarantined atomically, and
existence-safe denial shapes.

Exact persistence equality requires `assert_exact_save_reload_equality`,
`assert_runtime_protocol_state_equality`, and
`assert_no_duplicate_ledger_reservation_cargo_effect`. The equality fingerprint covers the aggregate
runtime state embedded by LAI.23, the report-safe protocol snapshot, SQLite checksums, Favor ledger,
reservation ledger, visible tasks, cargo, offering pipeline, research/quota/preparation, boost
activation/expiry, cat care/prosthetic state, diplomacy/trade contracts, bounded idempotency
receipts, action-result replay records, and quarantine markers.

The focused red contract intentionally rejects legacy shortcuts named
`legacy_system_journey_pass`, `skip_lai33_signed_authentication`,
`manufacture_lai33_inventory_or_favor`, `undocumented_lai33_time_skip`,
`client_side_redaction_journey_only`, and `allow_partial_lai33_restart_reconstruction`.

## LAI.27 server authorization/routing/redaction contract

LAI.27 owns the server-side implementation of the LAI.24 snapshot envelope and LAI.25 action
envelope after those protocol DTOs and the LAI.26 persistence migration exist. It must replace the
legacy WebSocket mutation path with one `LeaderAiServerMutationPipeline`; no route, debug action,
compatibility handler, or test-only branch may debit Favor, reserve a site, move cargo, appoint an
officer, activate a boost, mutate diplomacy/trade, or emit a refresh snapshot outside this path.

Every mutation follows exactly this server order:

1. `check_protocol_compatibility`
2. `check_hmac_session_authentication`
3. `check_selected_colony_ownership`
4. `check_actor_action_authority`
5. `check_expected_state_versions`
6. `check_bounded_idempotent_replay`
7. `check_current_preconditions`
8. `commit_atomic_favor_reservation_state`

Protocol compatibility returns `UPDATE_REQUIRED` through an `UpdateRequiredResponse` before action
payload route selection or nested decode. The response includes minimum-supported/current action
protocol versions only. HMAC/session authentication produces a `VerifiedPlayerSession` using a
constant-time MAC check, rejects before route selection, and never returns session secrets, signed
token material, or player/session existence differences.

Ownership is checked against the selected colony before authority or current-state checks.
Unauthorized, malformed, unknown-colony, and foreign-colony requests use the same opaque
`OpaqueExistenceDenied` shape where revealing existence would leak private state. The selected-
colony ownership guard denies foreign colony mutation, while authorized public diplomacy/trade views
remain readable according to relationship rules.

Action authority is separate from ownership. `PlayerOnlyDivineBoostGuard` rejects Leader and officer
boost activation. `OfficerDomainAuthorityGuard` confines officer-originated actions to the office's
domain and denies out-of-domain mutation even when the player owns the colony. Leader and officer
actions never impersonate a player action; player actions cannot bypass cat refusal, site/route
eligibility, Favor CAS, reservation conflicts, or current preconditions.

Expected versions are checked before replay lookup so stale duplicate IDs cannot reveal whether a
newer action exists. Bounded idempotency stores accepted and rejected receipts; a duplicate accepted
ID returns `ReplayAcceptedPriorResult`, a duplicate rejected ID returns `ReplayRejectedPriorResult`,
and neither path performs another debit, reservation, cargo move, or state mutation. Current
preconditions run after replay and before commit. Stale or failed preconditions are proven by
`NoMutationBeforePreconditions`.

The final commit is one atomic `AtomicLeaderAiCommit` that applies Favor debits/credits, world and
local reservations, task/runtime state, officer state, research/scholar state, boosts, diplomacy,
and trade together. `commit_favor_debit_once`, `commit_reservation_once`, and
`commit_runtime_state_once` are the required audit markers for this transaction boundary. A failure
after any validation step leaves all ledgers unchanged.

`ServerActionConflict` and `ServerActionResult` are strict, bounded DTOs. Conflict categories are
`UpdateRequired`, `Unauthenticated`, `Unauthorized`, `OwnershipDenied`, `VersionMismatch`,
`DuplicateReplay`, `PreconditionFailed`, `RateLimited`, and refreshable domain conflicts such as
insufficient Favor or reservation contention. `RefreshSnapshotHint` may carry current public
versions and a server-redacted snapshot slice, but never hidden stock, exact regeneration below L4,
private beliefs/plans, auth material, competing reservation/colony identity, rejected hidden amount,
unseen sites, or another colony's private state.

Rate limiting remains before expensive work: `LeaderAiMutationRateLimit` runs before world locks,
database transactions, action payload fan-out, and refresh snapshot construction. Rejected rate-
limited requests cannot observe whether a target colony/action/entity exists.

Redaction is server-side. `ServerSideSnapshotRedactor` and `redact_snapshot_for_authenticated_colony`
run before every WebSocket send and every conflict refresh. They remove foreign private beliefs,
private plans, hidden stock, exact regeneration below effective report level 4, auth material,
unseen threats, and unrevealed sites. The Bevy client is a renderer only:
`client_is_not_redaction_authority` is a required implementation marker, and sending an unredacted
world snapshot for Bevy to hide is forbidden.

The focused LAI.27A red harness is `crates/cat-server/tests/lai27_server_contract.rs`. Its expected
pre-implementation failures are the missing server mutation pipeline and ordered check markers,
incompatible-client/HMAC fail-before-route symbols, selected-colony ownership and actor authority
guards, expected-version/idempotent-replay/current-precondition/atomic-commit markers, typed bounded
conflict DTOs, rate-limit-before-expensive-work markers, server-side redaction markers, and new
LAI.24/LAI.25 envelope routing. No `cat-server` production code, `world_tick`, protocol production
types, persistence, client code, or fake shims are part of LAI.27A.

## SQLite persistence and migration

Persist planner schema/version/clock, posture/epoch, beliefs and evidence, report versions, live and
terminal intents, officer requests, standing orders/nudges, task sites/stages/routes/cargo,
world-scoped reservations, attributes/personality/stress/traits, anatomy/injuries/treatment,
prosthetic items/fitting/wear, Shrine pipelines, Favor events/balance, research quota/manifest
ownership, Insight/preparation, boosts, diplomacy, trade/escrow/in-transit cargo, idempotency results,
and transition fingerprints.

The migration is transactional per world/save and has an idempotent version marker:

- convert legacy attributes with the formula in [cats-and-care.md](cats-and-care.md);
- deterministically backfill personality from stable IDs;
- initialize existing cats anatomically healthy unless authoritative injury data exists;
- convert `global_upgrade_points + unspent research_points` to Favor once and preserve owned nodes;
- preserve cats, appointments, jobs, buildings, villages, inventories, routes, and relationships;
- migrate legacy job metadata into typed sites;
- turn unknown/malformed required site metadata into an explicitly blocked legacy task;
- reconstruct and revalidate reservations, routes, endpoints, cargo, and workers on load.

One malformed required row rolls back or quarantines the complete save migration; partial migrated
worlds are forbidden. Re-running startup is a no-op. Downgrade to the old format is unsupported.

## LAI.26 SQLite migration and restart contract

LAI.26 owns the SQLite schema bump and startup migration after LAI.23-LAI.25 define the production
runtime, snapshot, and action contracts. The migration is one transaction per world/save with a
strict `LAI26_SCHEMA_VERSION` and a durable `leader_ai_migration_marker`. The marker records source
schema, target schema, world/save identity, migration fingerprint, conversion totals, and completion
tick; replaying startup after the marker is complete is a no-op, and replaying a partially completed
marker rejects or quarantines the save before any row is used.

Legacy currency conversion is exact and one-time:

- read legacy `global_upgrade_points` and unspent research points from the old upgrade/research
  state;
- convert their sum to `FavorLedger` micro-Favor exactly once;
- preserve every already owned study/node as research progress;
- reject negative Favor, duplicate conversion markers, and any row that would mint Favor twice; and
- remove legacy spendable research/blessing currency as an authority source after migration.

Cat migration converts old cat data into the new care model. Attributes use the 0-100 to 1-20
formula from [cats-and-care.md](cats-and-care.md), legacy skills remain learned-skill state,
personality is deterministically backfilled from stable IDs, anatomy defaults to healthy unless
authoritative injury data exists, injury/treatment state is validated, and prosthetic items keep one
finite item identity through inventory, fitting, breakage, repair, death recovery, and trade.
Duplicate fitted IDs, impossible anatomy/stress/treatment combinations, dangling cat/item
references, and invalid prosthetic stages abort the transaction.

Task migration converts legacy job metadata to typed sites and persisted runtime stages. Every
migrated task records objective `SiteRef`, work slot, endpoint, route, stage, progress, worker,
cargo, reservation IDs, blocked reason, and last transition tick. Invalid required legacy site
metadata becomes an explicitly blocked legacy task if it is bounded and recoverable; malformed,
dangling, negative-cargo, objective-less active, impossible-stage, or hidden-projection rows reject
the whole save. Routes, endpoints, reservations, cargo, and assigned workers revalidate on startup.

The LAI.26 schema persists every post-cutover leaf and its version clock: planner clock/posture/
epoch, planner/domain/resource/spatial/reservation versions, beliefs, evidence, reports, live and
terminal intents, officer institution and requests, standing orders/nudges, tasks, world
reservations, Shrine pipelines, Favor events/balance, research ownership/frontier/quota,
Insight/preparation, Divine Boost purchases/expiry, diplomacy relationships/proposals, trade
escrow/in-transit cargo, bounded idempotency results, and transition fingerprints. Fresh worlds get
valid empty/default records for each leaf and require no lazy null interpretation.

Failure is atomic. Unknown schema versions, unsupported downgrade attempts, duplicate stable IDs,
dangling references, negative Favor, hidden projection fields, impossible task stages, unbounded
idempotency payloads, malformed JSON, and cross-colony private references roll back the full
transaction and quarantine/reject the row with a bounded reason. Partial migrated worlds are
forbidden; no table may be half-upgraded while another still exposes legacy authority.

Restart equality is required at every stage: Shrine source/haul/deposit/ritual/cancel/salvage,
Favor event replay, research purchase/quota/Insight/preparation, boost active/expired boundaries,
treatment, prosthetic fit/repair/wear, task route/cargo/reservation transitions, diplomacy
proposals, trade escrow/outbound/return/stranded stages, idempotency replay, and quarantine recovery.
Transition fingerprints prove a pre-restart and post-restart world advance choose the same next
mutation and do not duplicate Favor, reservations, cargo, trade, injury, prosthetic, report, or
event changes.

Cross-colony persistence keeps IDs and reservations isolated. Colony-scoped IDs cannot reference
another colony's private beliefs, plans, inventory, cats, officers, or tasks. World-scoped
reservations keep exclusive conflicts authoritative without leaking the competing colony identity to
the wrong projection. Public relationship, diplomacy, and trade facts persist separately from
private planner/report state.

The focused LAI.26A red harness is
`crates/cat-server/tests/lai26_persistence_migration_contract.rs`. Its expected
pre-implementation failures are the missing schema/version marker, transactional migration
boundaries, exact Favor conversion, cat/care migration, task/site/cargo/reservation migration,
leaf-state persistence/defaults, fail-closed malformed-row handling, bounded idempotency replay,
cross-colony isolation, and every-stage restart equality/fingerprint APIs. No production
persistence, protocol, server routing, client, or `world_tick` work is part of LAI.26A.

## UI promises

The compact Plans panel shows at most eight live intents with goal/type, state, responsible actor,
rationale, report confidence, dependencies, assigned cats, objective/complete footprint, expected
cost/benefit, and blocked/retry reason.

`Move Up` and `Move Down` apply +0.15/−0.15 only in the current planning epoch; equal nudges do not
stack and opposite replaces prior. Dismiss lasts one epoch, though emergencies can regenerate.
Standing orders persist, occupy Administration slots, and express reserve/offering/posture/trade/
construction policy without bypassing knowledge or physical rules.

Other surfaces show:

- estimate bands, confidence, report age, and explicitly unavailable regeneration;
- cat capability breakdown, stress/refusal, anatomy, injuries, treatment, prosthetics, fitting, and
  repair;
- offering source/haul/ritual state and belief-based rationale;
- exact Favor, frontier/quota, Insight/preparation/discount, and boost price/duration/expiry;
- relationship consent and physical trade/escrow/transit;
- snapshot-only full task footprints, distinct work/endpoint markers, dedupe, and stale despawn.

The client does not derive a source from destination, recompute nearest delivery, display a hidden
debug value, retain a stale marker after its snapshot disappears, or emit a marker for an
objective-less/unrevealed task.

## LAI.28 Plans and standing-orders UI contract

`LAI.28_PLANS_UI_CONTRACT` is the post-cutover Bevy client contract for the Plans surface. LAI.28
depends on LAI.25 action DTOs and LAI.27 server routing/redaction; this card's red harness must not
add production UI shims, protocol payloads, server routes, persistence fields, or `world_tick`
integration. The client renders only the authoritative snapshot and never becomes redaction
authority.

The Plans panel renders at most eight authoritative live plans in the server-provided order. Each
row uses a stable plan ID and shows lifecycle/status, responsible Leader or officer, dependencies,
bounded rationale and reasons, score/confidence, estimate ranges, report age, and provenance. Rows
must not expose hidden truth: exact stock, exact regeneration below effective report level 4,
unreported source capacity, private foreign-colony plans, rejected hidden amounts, unseen threats,
or debug-only authoritative values are absent from labels, tooltips, inspectors, accessibility
trees, screenshots, logs, and conflict feedback.

The shipped controls are accessible through stable roles, labels, and test identifiers:

- `Move Up` applies exactly +0.15 for the current planning epoch;
- `Move Down` applies exactly -0.15 for the current planning epoch;
- `Dismiss` lasts one planning epoch and does not suppress emergency regeneration;
- standing-order create, edit, and remove controls persist policy without bypassing knowledge,
  authority, cat refusal, site eligibility, Favor CAS, or physical reservations; and
- domain nudge controls expose the target domain, current epoch, enabled state, and bounded
  rejection reason.

Equal nudges are deterministic: repeating the same nudge for the same plan and epoch does not stack,
opposite nudges replace the prior epoch value, and tie display order uses stable plan IDs rather
than Bevy entity creation order. Unknown, terminal, or removed plans despawn on refresh and cannot
leave stale buttons capable of sending an action. Stale actions preserve the user's panel focus and
standing-order draft, trigger a refresh through the authoritative conflict result, and keep all
feedback bounded.

Standing orders occupy Administration slots. The UI shows slot limit, used count, vacancy, and
typed feedback for full, unauthorized, stale, malformed, and precondition-failed edits. It does not
create a client-only order when the server rejects the action, and unused capacity never implies a
hidden future order or carryover.

Every mutation sent from this surface uses the LAI.25 action envelope: protocol version, stable
idempotency ID, authenticated colony/player identity, expected planner version, expected domain
version, expected resource/Favor version when relevant, expected spatial/reservation version when
relevant, and a strict bounded payload. The client displays `UPDATE_REQUIRED` and typed conflicts
as refreshable state, not as silent failures or optimistic hidden mutation.

Officer information appears alongside plans: Leader/officer responsibility, office/domain,
authority scope, vacancy, request reason, expiry, and bounded block reason. Regeneration remains
explicitly unavailable below effective report level 4; the client cannot fill this by querying
terrain, rerunning simulation helpers, or caching an older hidden value.

The focused LAI.28A red harness is `crates/cat-client/tests/lai28_plans_ui_contract.rs`. Its
expected pre-implementation failures are the missing Plans panel plugin/root, top-eight row
projection, report-safe field rendering, accessible nudge/dismiss/standing-order/domain controls,
Administration slot feedback, LAI.25 action-envelope construction, stale refresh/context
preservation, deterministic nudge/tie handling, officer report/vacancy/authority display,
regeneration L4 guard, and Playwright-stable accessibility/test-ID markers. No cat-client
production UI, server/protocol/persistence production code, or world tick changes are part of
LAI.28A.

## Exact action concurrency and public identity

LAI.24 publishes two different classes of version:

- `stateVersion` is an aggregate display/cache version for snapshot replacement; and
- `actionVersions` contains the exact server fingerprints used by LAI.25 optimistic concurrency for
  planner, domain, resource/Favor, spatial, reservation, research, scholar, boost, diplomacy, trade,
  prosthetic, care, officer, and standing-order mutations.

The client must use the matching `actionVersions` lane for each action. It must not substitute
`stateVersion` merely because both are present in the same snapshot. Adding an action domain requires
adding and projecting its exact lane, validating it in server preflight, and testing that unrelated
snapshot changes neither create false staleness nor allow a truly stale mutation.

`PlanQueueSnapshot.planningEpoch` is the policy lifetime for nudges and dismissal. It is not the plan
queue version and cannot be reconstructed from it.

Public IDs are bounded without losing canonical identity. Canonical planner IDs may contain `|`
between typed components. Action/principal IDs remain on the stricter alphabet; the shared stable
idempotency helper hashes an invalid or overlong candidate deterministically. Long runtime IDs may
appear as deterministic `wire:v1` aliases in LAI.24. LAI.25 routing resolves those aliases through
the shared `stable_id_matches` rule before applying standing-order or trade mutations. Human-readable
labels are presentation only and are never submitted as IDs.

After authentication and after every action response, the server sends an immediate authoritative
LAI.24 snapshot. This applies even when a test fixture freezes world ticks. It prevents valid
mutations from waiting indefinitely for a future simulation tick and gives browser controls a
precise action-response-snapshot completion boundary.

Operational tracing and redaction rules are defined in
[diagnostics-and-debugging.md](diagnostics-and-debugging.md).

## Single-path cutover

### LAI.24C strict runtime field closure (2026-07-23)

Protocol-v2 snapshots project officer appointments/expertise/vacancies, standing orders, and bounded
refresh hints from the selected colony's persisted runtime. Scholar week metadata is optional until a
real work event establishes its start tick; weekly Insight is persisted rather than reconstructed
from lifetime totals. Diplomacy records retain external colony IDs and mutation ticks through restart,
with a legacy string reader only for migration compatibility. Injury body-part state retains the
resolver's incident identity and completion tick, so server projection never synthesizes IDs or times.

After all new tests pass, one integration owner bumps protocol and persistence versions, installs
exactly one planner path in `world_tick`, and removes the legacy `leader_director` runtime,
per-action reliability misses, tithe/cooldown/immediate scalar path, duplicate research currency,
and conflicting scheduling/types/tests. Protocol, server, migration, simulation, and client change
atomically.

No runtime shadow planner, feature flag, dual mutation, or optional legacy mode is permitted.
Offline comparison fixtures are test-only. The roots `world_tick`, protocol exports, server routing,
persistence, and client orchestration each have a single owner and call focused leaf modules.

At LAI.34, synchronize the maintained root documentation listed in [README.md](README.md), add at
most a link from the historical migration board, and resolve the `docs/TESTING.md` maintained-versus-
superseded contradiction.
