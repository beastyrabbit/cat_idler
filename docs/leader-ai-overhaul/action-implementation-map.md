# LAI.25B action implementation readiness map

> Historical first-cutover readiness evidence only. Current LAI.35–70 action authority is
> [`integrated-implementation-map.md`](integrated-implementation-map.md) plus the exact P1/P2 board
> registers. Favor/Shrine/direct-placement/migration behavior below is not a current target.

This map is additive implementation guidance for LAI.25 production work. It does not mark LAI.25
complete and does not authorize edits to `world_tick.rs`; LAI.23 remains the single production
cutover owner.

## Source state read for this map

- `crates/cat-protocol/tests/lai25_action_contract.rs` is the complete LAI.25 red contract. It
  requires `LeaderAiActionEnvelope`, strict bounded action payloads, expected-version fields,
  typed conflicts, fail-closed unknown versions/variants, and an ordered validation pipeline.
- `docs/leader-ai-overhaul/wire-persistence-ui.md` defines the same post-cutover contract: every
  action checks protocol compatibility, authentication, colony ownership, action authority,
  expected versions, duplicate replay, current preconditions, then one atomic Favor/reservation
  commit.
- Current `crates/cat-protocol/src/lib.rs` still exposes `PROTOCOL_VERSION = 1`, legacy
  `ClientAction`, and legacy `ActionResult { ok, message, colony_id }`. It has no LAI.25 action
  envelope, no aggregate expected-version DTO, no typed action conflict/result DTO, and no bounded
  action ID newtypes.
- Current `crates/cat-server/src/main.rs` decodes WebSocket text directly as `ClientAction`,
  authenticates from embedded session fields, locks the world, calls `cat_sim::actions::apply_action`,
  and returns `ActionResult`. This is the legacy mutation path, not the post-cutover
  protocol/auth/version/idempotency pipeline.
- Current pure sim leaves already cover some deterministic mutation semantics:
  `favor.rs`, `research_purchase.rs`, `scholar_research.rs`, `divine_boosts.rs`,
  `diplomacy.rs`, `autonomous_trade.rs`, `prosthetics.rs`, `authority.rs`,
  `planner_core.rs`, `leader_planner.rs`, `officer_requests.rs`, and
  `leader_ai_runtime.rs`.

## Non-negotiable validation order

All LAI.25/LAI.27 server mutations must use one ordered path:

1. `check_protocol_compatibility`
2. `check_authentication`
3. `check_colony_ownership`
4. `check_action_authority`
5. `check_expected_versions`
6. `check_duplicate_replay`
7. `check_current_preconditions`
8. `commit_favor_or_reservation`

Implementation consequence: do not route by typed payload until an outer raw envelope version has
been checked. Incompatible clients must receive `UPDATE_REQUIRED` with minimum/current supported
versions before action decode, auth, world locking, database work, or simulation mutation.

Rate limiting should remain before expensive world/database/snapshot work, but it must not become a
substitute for protocol compatibility or authentication. Unknown action variants, unknown action
versions, malformed IDs, out-of-bounds numbers, and unknown object fields fail closed with bounded
protocol errors and no mutation.

## Envelope DTO

Add these protocol DTOs in `crates/cat-protocol/src/lib.rs`:

- `ActionProtocolVersion(u32)` and bumped action/snapshot protocol constants.
- `LeaderAiActionEnvelope` with `#[serde(deny_unknown_fields)]`:
  `protocol_version`, `idempotency_id`, `colony_id`, `player_id`, `expected_versions`, and
  `payload`.
- `ActionIdempotencyId`, `SelectedColonyId`, `AuthenticatedPlayerId`, `BoundedEntityId`, and
  action-specific stable IDs. All must reject empty, whitespace, delimiter-confusable,
  overlong, malformed, or cross-version IDs.
- `ExpectedStateVersions`:
  `expected_planner_version`, `expected_domain_version`, `expected_resource_version`, plus optional
  action-specific versions such as `expected_spatial_version`, `expected_reservation_version`,
  `expected_research_version`, `expected_scholar_version`, `expected_boost_version`,
  `expected_diplomacy_version`, `expected_trade_version`, and `expected_prosthetic_version`.
- `LeaderAiActionPayload`, a strict tagged enum. Unknown variants fail closed as
  `UnknownActionVariant`.

Do not put `session_id`, `sig`, or nickname fields inside each payload. Server authentication must
bind the WebSocket/session to `AuthenticatedPlayerId` before payload authority checks.

## Result and conflict DTO

Add one bounded result family:

- `LeaderAiActionResponse` with `protocol_version`, `idempotency_id`, `colony_id`, `result`, and
  optional `refresh`.
- `LeaderAiActionResult::Accepted(ActionAcceptedResult)`,
  `LeaderAiActionResult::Rejected(ActionConflict)`, and
  `LeaderAiActionResult::DuplicateReplay(ActionReplayResult)`.
- Accepted results include stable changed IDs, committed versions, and bounded public state hints.
  They never include hidden stock, hidden regeneration, unseen threats, foreign private state,
  auth material, exact rejected amounts, or reservation loser identity.
- Rejected results are persisted for idempotency replay when the request was well-formed,
  authenticated, owned, authorized, and past expected-version checks. Pre-decode, auth, ownership,
  malformed-ID, and unauthorized denials may be unreceipted to avoid storing attacker-chosen keys.
- Stale/refresh results include `CurrentVersionHint` and `CurrentStateHint` that are sufficient for
  UI refresh but report-safe. A stale client must refresh the authoritative snapshot and preserve
  local UI context where possible.

Conflict variants required by LAI.25:

- `UpdateRequired { code: UPDATE_REQUIRED, minimum_supported_version, current_protocol_version }`
- `Unauthorized`
- `OwnershipDenied`
- `AuthorityDenied`
- `VersionMismatch`
- `DuplicateReplay`
- `PreconditionFailed`
- `InsufficientFavor`
- `ReservationConflict`
- `MalformedActionId`
- `UnknownActionVariant`
- `MalformedPayload`
- `RateLimited`

For existence safety, `UnknownPlan`, `UnknownColony`, `UnknownTrade`, `UnknownProsthetic`, and
foreign ownership failures should normally collapse into an opaque bounded denial unless the selected
colony already has report-safe visibility of that object.

## Server pipeline ownership

Minimal server production slice:

- Add a LAI.27-owned router beside the current `handle_client_text` path. It should parse only the
  outer JSON object enough to check `protocol_version` and return `UPDATE_REQUIRED` before decoding
  `LeaderAiActionEnvelope`.
- Convert HMAC/session validation into a `VerifiedPlayerSession` or equivalent typed guard. Current
  `identity.rs` already has constant-time HMAC comparison and player ID derivation; LAI.27 needs to
  expose a post-auth guard rather than reusing embedded legacy action fields.
- Check `selected_colony_id` against server-owned village directory and owner data before any sim
  object lookup. Foreign colony mutation failures must be indistinguishable from missing-object
  failures unless the object is public and visible.
- Acquire the authoritative world lock only after compatibility, auth, ownership, authority, and
  bounded rate checks pass.
- Run expected-version checks before duplicate replay. This matches the LAI.25 red contract: a
  duplicate id with mismatched expected versions is not allowed to bypass stale-client detection.
- Replay stored accepted or rejected results before current preconditions so a retried action cannot
  fail merely because later ticks changed the world.
- Execute sim mutation through one atomic adapter. For multi-leaf mutations, clone candidate state,
  call leaf APIs, validate, then commit the whole candidate back once.
- Persist bounded idempotency receipts and updated world state transactionally under LAI.26; until
  then receipts must be held in the runtime aggregate and documented as non-restart-safe.

## Sim mutation adapter pattern

Every domain adapter should have this shape:

```text
fn apply_<domain>_action(world, selected_colony, verified_player, envelope) -> LeaderAiActionResponse
```

The adapter performs no protocol decode or HMAC work. It receives a typed envelope, a verified player,
and an ownership-checked colony handle. It must:

- derive all domain IDs from the envelope idempotency ID plus selected colony ID;
- check domain-specific expected versions against persisted state;
- check duplicate receipt after expected versions;
- perform current preconditions on a cloned candidate when the mutation spans more than one leaf;
- call deterministic leaf APIs with explicit expected versions;
- record one idempotency receipt with bounded result/error and expiry;
- return exact committed versions for changed domains;
- leave Favor, reservations, cargo, diplomacy, prosthetics, and planner state byte-identical on
  every stale, unaffordable, rejected, malformed, or duplicate-conflicting request.

`LeaderAiRuntimeState` already contains `idempotency_receipts`, but there is not yet a public
mutation API that owns this pattern.

## Action map

| Action | Payload and bounds | Authority and owner checks | Expected versions | Sim target | Idempotency and result | Stale/refresh/tests |
|---|---|---|---|---|---|---|
| `NudgePlan` | `plan_id`, signed delta exactly `+1500` or `-1500` basis points, optional bounded reason key. | Authenticated player for selected colony; action is `AuthorityOperation::PlayerNudge`; plan must belong to selected colony and be in current top/report-safe set. | planner version, domain version for the plan domain. | New planner nudge state is needed; `planner_core::IntentScoreInputs` already has `temporary_player_bias`. | Receipt records plan ID, delta, affected planning epoch, resulting planner/domain version, and bounded current rank. Duplicate same ID replays; conflicting same ID rejects. | Stale returns current planner/domain versions and whether the plan is still visible, without hidden score inputs. Tests: equal nudges deterministic; removed plan despawns stale controls; no stacking beyond one epoch. |
| `DismissIntent` | `intent_id`, current epoch, bounded dismissal reason enum. | Player authority for selected colony; cannot dismiss emergency/self-preservation intents unless the protocol defines an explicit override conflict. | planner version and intent/domain version. | `planner_core::IntentLifecycle` supports `Cancelled`; `IntentGraph` must expose a versioned cancel/dismiss API. | Receipt records terminal state, terminal tick, resulting planner version. Duplicate replay returns same terminal result. | Stale when lifecycle moved or epoch changed; refresh hint includes current lifecycle/status only. Tests: dismissal does not erase terminal history or dependencies. |
| `CreateStandingOrder` | bounded order kind/domain/target, bounded text/rationale IDs, priority/bias bounds, optional expiry. | Player authority with `AuthorityOperation::MaintainStandingOrder`; selected colony only. | planner version plus standing-order collection version. | New standing-order state is missing. Administration slot counts come from `scholar_research::ResearchTrackStages` / manifest effects. | Receipt records standing order ID, occupied slot count, resulting version. Duplicate replay returns same ID. | Stale includes current slot used/limit and no hidden eligibility facts. Tests: slot limits, duplicate create replay, no bypass of belief/report eligibility. |
| `UpdateStandingOrder` | standing order ID, patch with bounded fields; unknown fields denied. | Same as create; order must be owned by selected colony. | standing-order version and planner version. | New standing-order state. | Receipt records updated fields and resulting version. | Stale if order changed/removed; refresh preserves draft. Tests: no partial update on invalid patch. |
| `DeleteStandingOrder` | standing order ID. | Same as create; selected colony only. | standing-order version and planner version. | New standing-order state. | Receipt records tombstone/version. | Duplicate delete replays prior result; unknown/foreign ID is opaque. Tests: removed order stops contributing to future plans without mutating current hidden facts. |
| `AppointOfficer` | role, cat ID, optional authority scope if override is explicit. | Player owns selected colony; cat is living/eligible in same colony; office limits from `officers.rs`. | officer institution version and planner version. | Current legacy `ClientAction::AssignOfficer` calls `assign_officer`; LAI runtime has `OfficerInstitutionState`, but a post-cutover versioned appointment API is still needed. | Receipt records role, cat ID, previous occupant if report-safe, resulting officer/planner versions. | Stale if role/cat changed. Tests: foreign cat opaque, duplicate appointment replays, no cross-colony mutation. |
| `UnappointOfficer` | role. | Player owns selected colony; role exists in selected colony. | officer institution version and planner version. | `OfficerInstitutionState` / existing unassign behavior need a typed adapter. | Receipt records vacated role and resulting version. | Stale if already changed; refresh shows current role/vacancy only. Tests: succession-safe request reassignment and no hidden fallback. |
| `OfficerAuthorityOverride` | role, domain or request ID, bounded authority mode. | Player owns selected colony; cannot grant authority outside protocol-defined domains. | officer institution version, request/version if tied to a request. | Authority framework has `AuthorityDomain`, `AuthorityOperation`, and `officer_owns_domain`; no persisted override state exists. | Receipt records override ID, scope, resulting version. | Stale if institution changed. Tests: officer cannot mutate outside domain; override does not bypass cat refusal or reservations. |
| `RequestTreatment` | cat ID, treatment kind, bounded target injury/anatomy reference. | Player owns selected colony or authorized medical/care domain once defined; cat belongs to selected colony and visible report permits the action. | cat care/anatomy version, planner/domain version. | Cat stress/anatomy/injury leaves exist, but no player treatment action API was found in current protocol/server. | Receipt records treatment request/task ID and resulting care version. | Stale when injury/anatomy state changed; conflict hides exact unreported injury details. Tests: no hidden injury leak below report level; no mutation on invalid patient. |
| `FitProsthetic` | prosthetic ID, cat ID, body part, site/ref, fitter ID or task request. | Player owns selected colony; item and cat same colony; patient consent, fitter capability, reachable fitting site. | prosthetic version, cat anatomy version, reservation/spatial version. | `prosthetics::ProstheticLedger::begin_fitting` and `complete_fitting` validate item ownership, consent, fitter, site, and slot. The ledger does not expose a version counter or action receipt. | Adapter must wrap begin/complete in a versioned task/reservation mutation and record result. Duplicate replay cannot re-reserve or re-fit. | Stale returns current prosthetic/cat version hints only. Tests: wrong owner opaque, slot occupied no partial reservation, restart at fitting reservation. |
| `RepairProsthetic` | prosthetic ID, workshop ID, finite input reservation ID. | Player owns selected colony; item in selected colony inventory; reachable workshop; exact finite inputs authorized. | prosthetic version, reservation/resource version. | `prosthetics::ProstheticLedger::begin_repair` and `complete_repair` validate workshop reachability, broken state, inventory owner, and finite input authorization. Version/action receipt still missing. | Receipt records repair reservation or completion and resulting prosthetic/resource versions. | Stale if repaired/moved/broken state changed; no exact hidden inventory in conflict. Tests: finite inputs not consumed on stale/invalid repair. |
| `PurchaseResearchWithFavor` | study ID, optional `use_preparation`, committed displayed price fields from client are advisory only. | Player owns selected colony; `AuthorityOperation::PurchaseResearch`; selected study must be frontier and affordable. | research version, Favor version, scholar version if using preparation. | `research_purchase::ResearchPurchaseState::player_purchase` and `scholar_research::ScholarResearchState::player_purchase` already provide exact Favor CAS, frontier checks, preparation discount, cloned atomic commit for scholar+research+Favor. | Derive `ResearchPurchaseId` from action ID. Receipt records study ID, undiscounted price, charged price, discount, consumed preparation, committed research/Favor/scholar versions. | Stale returns current research/Favor/scholar versions and frontier-visible state; unaffordable consumes no Favor or quota. Tests: duplicate/replay, stale, already owned, not frontier, insufficient Favor, preparation consumed exactly once. |
| `PrepareScholarStudy` | study ID, scholar ID, bounded assignment. | Player owns selected colony or authorized research officer if later allowed; scholar alive and assigned to selected colony. | scholar version and research version. | `ScholarResearchState::prepare_study` covers Insight cost, already prepared, owned study, scholar alive, capacity, and version. | Receipt records preparation ID, study ID, scholar ID, Insight cost, resulting scholar version. | Stale includes scholar/research version and whether visible preparation still exists. Tests: insufficient Insight no mutation; scholar death/reassignment safe. |
| `ActivateDivineBoost` | boost type, duration hours from unlocked set. | Player-only. `divine_boosts::purchase` rejects non-God actors through `AuthorityOperation::ActivateBoost`; Leaders/officers must never be routed here. | boost version, Favor version, research-stage version or committed stage fingerprint. | `DivineBoostState::purchase` already validates duration/stages, same-type active rejection, expected boost/Favor versions, exact price, expiry, and replay. | Derive `DivineBoostPurchaseId` from action ID. Receipt records boost type, duration, paid cost, activation/expiry tick, committed boost/Favor versions. | Stale shows active same-type expiry and versions; no refund/cancel path. Tests: Leader/officer denied, same type active no debit, duplicate replay no second debit. |
| `ChangeDiplomacy` | pair/target colony, proposed relationship `friendly` or `allied`. | Player owns selected acting colony; pair includes selected colony; target is public/known enough to expose. | diplomacy pair version. | `DiplomacyLedger::apply` with `DiplomacyActionKind::Propose`. | Receipt records pair ID, proposed target, relationship version, pending consent. | Stale returns pair version and public relationship/consent summary only. Tests: same colony reject, blocked reject, pending proposal conflict, cross-colony opaque. |
| `ApproveAlliance` | pair/proposal ID. | Player owns selected acting colony; selected colony is party; approval is consent-required. | diplomacy pair version. | `DiplomacyLedger::apply` with `Approve`; activation requires both approvals. | Receipt records approval or activated relationship. Duplicate approval can replay or no-op consistently. | Stale when proposal changed/cleared; refresh public pair state only. Tests: two-party consent, replay, blocked proposal no hidden reason. |
| `BlockColony` | target colony/pair, optional bounded public reason. | Player owns selected acting colony; block is immediate and intentionally may bypass stale relationship version per current sim leaf. | diplomacy version should still be supplied by envelope; implementation must document whether Block bypasses exact stale rejection or returns a special winner result. | `DiplomacyLedger::apply` with `Block`. | Receipt records blocker and relationship version. | Refresh exposes blocked public state, not private target data. Tests: block wins approval race; replay stable; unblock ownership if later added. |
| `AcceptTradeContract` | contract ID. | Player owns selected colony; selected colony is party; relationship and consent state allow accept. | trade contract version, reservation/world version, diplomacy relationship version. | `TradeLedger::apply_action` with `TradeActionKind::Accept` validates version, relationship, expiry, and commits escrow reservations atomically on final acceptance. | Receipt records contract ID, resulting version, stage, escrow IDs as bounded summaries. Duplicate replay returns same receipt and does not re-escrow. | Stale returns current contract version/stage and public relationship hint. Tests: final accept escrow once, stale no reservation, expired no mutation, foreign contract opaque. |
| `RejectTradeContract` | contract ID and bounded reason enum. | Player owns selected colony; selected colony is party. | trade contract version. | Current trade leaf has `Cancel`, not an explicit consent reject. Production must decide whether `RejectTradeContract` maps to `Cancel` for proposed consent contracts or adds `Reject`. | Receipt records cancelled/rejected stage and resulting version. | Stale if stage already advanced. Tests: rejection releases any proposed-only state, cannot cancel in-transit without recovery path. |
| Existing physical placement actions | Typed wrapped payloads for buildings, farms, stockpiles, gather/fishing spots, roads, bridges, rail, docks, transport vehicles/routes. Bounds include rectangles, paths, endpoints, amounts, worker IDs, and site refs. | Player owns selected colony; action-specific map visibility/building/research/workforce authority. | spatial version, reservation version, resource/Favor if relevant, planner/domain version. | Current legacy `ClientAction` variants and `actions.rs` implementations exist, but they are not under LAI.25 envelope or typed conflicts. | Receipt records placed IDs/reservation IDs and resulting spatial/reservation versions. | Stale triggers refresh; placement conflicts hide private reservation loser. Tests: no parallel legacy/direct path, no partial resource/reservation on failure. |

## Minimal production ownership slices

1. Protocol DTO slice, owned by LAI.25:
   `crates/cat-protocol/src/lib.rs` only. Add the envelope, payload enum, bounded newtypes,
   expected-version block, result/conflict DTOs, and serde fail-closed tests. Keep old `ClientAction`
   only as an explicitly legacy compatibility type until cutover deletion.

2. Server pipeline slice, owned by LAI.27:
   `crates/cat-server/src/main.rs` plus small helper modules if needed. Add raw version preflight,
   typed auth guard, ownership/authority/version/replay/precondition/commit pipeline, and redacted
   refresh responses. Do not call the legacy `apply_action` for LAI.25 envelopes.

3. Persistence slice, owned by LAI.26:
   `crates/cat-server/src/persistence.rs`. Persist action protocol version, idempotency receipts,
   planner/domain/resource versions, and LAI runtime leaves transactionally. Restart must replay
   accepted/rejected receipts without duplicate mutation.

4. Sim adapter slice, owned by each leaf owner or LAI.23 cutover owner:
   expose small pure APIs that take typed requests and return typed outcomes for nudge/dismiss,
   standing orders, officer institution, treatment/prosthetics, research/preparation, boosts,
   diplomacy, trade, and physical placement. Existing Favor, research, scholar, boost, diplomacy,
   and trade leaves should be reused rather than rewritten.

5. Test slice:
   keep `crates/cat-protocol/tests/lai25_action_contract.rs` as the DTO red contract, add server
   routing/idempotency tests under `cat-server`, and add pure sim adapter tests under `cat-sim`.
   Required tests must cover update-required before decode/auth, malformed IDs, unknown variants,
   ownership opacity, authority denial, expected-version stale refresh, duplicate accepted replay,
   duplicate rejected replay, Favor conservation, no partial reservation/cargo/item mutation, and
   multi-colony isolation.

## Extension instructions

Future actions and workshop domains must follow the same pattern:

1. Add a strict payload variant under `LeaderAiActionPayload` with bounded IDs and quantities.
2. Add explicit expected-version fields for every domain the action reads before mutation.
3. Define the authority operation and domain in `authority.rs`; do not infer authority from UI.
4. Define accepted and rejected result shapes with bounded report-safe hints.
5. Add idempotency receipt matching that includes payload kind, selected colony, authenticated
   player, expected versions, and all mutation-defining fields.
6. Implement preconditions against the current world after duplicate replay and before commit.
7. Mutate cloned candidate state when more than one ledger/leaf is affected.
8. Persist the receipt and world update in one transaction.
9. Add protocol, server, sim, restart, stale-client, and multi-colony tests before exposing UI.
10. Confirm the action is absent from legacy direct `ClientAction` routing after cutover deletion.

The extension is not complete until an old client receives `UPDATE_REQUIRED` before payload decode,
a stale client receives a typed refresh conflict without hidden truth, and replaying the same
idempotency ID after restart returns the same bounded result without any additional mutation.
