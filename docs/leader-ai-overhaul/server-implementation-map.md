# LAI.27 server implementation readiness map

> Historical first-cutover readiness evidence only. Current server routing, authorization,
> redaction, atomic commit, and legacy-deletion rules are in
> [`integrated-implementation-map.md`](integrated-implementation-map.md).

This is an additive readiness map for LAI.27 production work. It does not
implement routing, does not edit `world_tick.rs`, does not edit production
protocol/server/persistence code, and does not update board status.

## Sources read

- LAI.27 red contract:
  [`../../crates/cat-server/tests/lai27_server_contract.rs`](../../crates/cat-server/tests/lai27_server_contract.rs)
- Current server entry point:
  [`../../crates/cat-server/src/main.rs`](../../crates/cat-server/src/main.rs)
- Current HMAC/session helpers:
  [`../../crates/cat-server/src/identity.rs`](../../crates/cat-server/src/identity.rs)
- Current rate limiter:
  [`../../crates/cat-server/src/rate_limit.rs`](../../crates/cat-server/src/rate_limit.rs)
- Current SQLite persistence:
  [`../../crates/cat-server/src/persistence.rs`](../../crates/cat-server/src/persistence.rs)
- Current protocol root:
  [`../../crates/cat-protocol/src/lib.rs`](../../crates/cat-protocol/src/lib.rs)
- LAI.24 snapshot readiness:
  [`snapshot-implementation-map.md`](snapshot-implementation-map.md)
- LAI.25 action readiness:
  [`action-implementation-map.md`](action-implementation-map.md)
- Wire/persistence/UI contract:
  [`wire-persistence-ui.md`](wire-persistence-ui.md)
- Spatial task contract:
  [`spatial-task-contract.md`](spatial-task-contract.md)
- Cat care contract:
  [`cats-and-care.md`](cats-and-care.md)
- Shrine/Favor/research contract:
  [`hole-research-progression.md`](hole-research-progression.md)
- Diplomacy/trade contract:
  [`diplomacy-trade.md`](diplomacy-trade.md)

## Current server state

The live WebSocket path still decodes incoming text directly as legacy
`ClientAction`, classifies authentication from fields embedded inside that
payload, rate-limits by IP/session, constructs `ActionCtx`, locks `WorldState`,
calls `cat_sim::actions::apply_action`, and returns legacy
`ActionResult { ok, message, colony_id }`. Successful legacy actions rebuild a
legacy `WorldSnapshot`, update `completed_snapshot`, and broadcast it.

Useful existing building blocks:

- `identity.rs` has HMAC signing, session issuance/renewal, player ID derivation,
  age validation, and constant-time signature comparison.
- `rate_limit.rs` has a deterministic sliding-window limiter; `main.rs` already
  has per-IP and per-session limiters.
- `ConnectionContext` tracks socket identity, selected colony, nickname, and
  rate-limit keys.
- `village_directory` and `project_snapshot` already keep private personal
  villages out of unauthenticated/foreign sockets and prioritize the socket
  selected colony.
- `project_reported_stock` and `redact_exact_functional_equipment` prove the
  server can redact before WebSocket send; they are legacy stock/equipment
  redactors, not the complete LAI.24/27 report-safe redaction layer.

Missing LAI.27 production surface:

- No `LeaderAiServerMutationPipeline`.
- No raw outer-envelope compatibility check that can return `UPDATE_REQUIRED`
  before nested action decode/auth.
- No `VerifiedPlayerSession` guard for the post-cutover action envelope.
- No selected-colony ownership guard before sim object lookup.
- No typed actor/action authority guard for player-only boosts or officer domain
  limits at the server routing layer.
- No aggregate expected-version block or stale refresh result.
- No bounded idempotency receipt store integrated with server routing and
  persistence.
- No atomic Favor/reservation/runtime-state transaction wrapper around sim leaf
  mutations.
- No LAI.24 `LeaderAiSnapshotEnvelope` projection or complete server-side
  redactor.
- No typed bounded conflict/result family; legacy failures are strings.

## Authoritative request pipeline

All post-cutover mutation requests must enter one server path in this exact
order:

1. `read_bounded_ws_frame`
2. `check_protocol_compatibility`
3. `decode_lai_action_envelope`
4. `check_hmac_session_authentication`
5. `check_mutation_rate_limit`
6. `check_selected_colony_ownership`
7. `check_actor_action_authority`
8. `check_expected_state_versions`
9. `check_bounded_idempotent_replay`
10. `check_current_preconditions`
11. `commit_atomic_favor_reservation_state`
12. `persist_world_and_receipt`
13. `refresh_authoritative_snapshot`
14. `redact_snapshot_for_authenticated_colony`
15. `send_bounded_action_response`

Compatibility is special: the server must parse only the minimal outer JSON
needed to read `protocol_version` and return `UPDATE_REQUIRED` before decoding
the nested action payload, authenticating, selecting a route, locking the world,
opening a transaction, or building a refresh snapshot. After compatibility
passes, strict envelope decode rejects unknown fields, unknown variants,
malformed IDs, malformed bounds, and oversized payloads with typed protocol
errors and no mutation.

Authentication precedes rate limiting in the LAI.27 action pipeline so the
dedicated limiter can key on the verified player/session when available. A
cheap unauthenticated/IP limiter may still reject abusive frames before
compatibility work, but that prefilter must return only a generic bounded
rate-limit denial and must not replace the authenticated mutation limiter.
Every limiter check must complete before world lock acquisition, database
transactions, sim mutation, snapshot refresh, or expensive cross-colony lookup.

## Pipeline map

| Step | Current source | LAI.27 production rule | Response/redaction rule | Tests |
| --- | --- | --- | --- | --- |
| Bounded frame read | `MAX_WEBSOCKET_MESSAGE_BYTES`, `handle_socket` | Reject oversized text/binary before JSON work. Preserve ping/pong/close behavior. | `RateLimited` or `MalformedPayload` without echoing payload. | Oversized frame never touches world/db/snapshot. |
| Protocol compatibility | Missing; legacy directly runs `serde_json::from_str::<ClientAction>` | Read only outer `protocol_version`; incompatible/old client receives `UpdateRequiredResponse { code: UPDATE_REQUIRED, minimum_supported_action_protocol_version, current_action_protocol_version }`. | No nested decode, no auth result, no existence hint. | `UPDATE_REQUIRED` before nested decode/auth and before route selection. |
| Strict envelope decode | LAI.25 DTOs missing in `cat-protocol` | Decode `LeaderAiActionEnvelope` with bounded IDs, selected colony, player ID, idempotency ID, expected versions, and strict tagged payload. | Unknown variants/fields are typed malformed conflicts. | Unknown action variant, unknown object field, malformed ID fail closed. |
| HMAC/session authentication | `verify_session`, `verify_session_at`, `session_signature_valid`, `ActionAuthentication` | Convert session/HMAC validation into `VerifiedPlayerSession { player_id, session_id }`; payload `player_id` must match the verified session. New LAI payloads should not carry raw `sig` inside each nested action. | `Unauthenticated` is opaque and never reveals whether a colony/action/object exists. Auth material never appears in result or snapshot. | Bad/missing MAC, mismatched player, expired session, and replay from another session fail before ownership/route. |
| Mutation rate limit | `RateLimiter`, `state.rate_limiter`, `state.ip_rate_limiter` | Add `LeaderAiMutationRateLimit` keyed by verified player/session plus peer fallback. Run before world lock, DB work, and refresh snapshots. | `RateLimited` with retry/window class only; no hidden state hints. | Rate limit before `world.lock`, database transaction, snapshot refresh, and sim route. |
| Selected-colony ownership | `ConnectionContext.colony_id`, `village_directory`, `owner_player_id`, `can_control_village` in sim actions | Create `SelectedColonyOwnershipGuard` from verified player plus selected colony. Global remains controllable by authenticated players only if that is still desired; personal villages require exact owner match. | Unknown, foreign, and unauthorized personal colonies collapse to `OpaqueExistenceDenied` unless public visibility already authorizes distinction. | Foreign selected colony cannot mutate or reveal existence through error differences. |
| Actor/action authority | `authority.rs`, `decide_authority`, `AuthorityActor`, `AuthorityOperation`, `AuthorityDomain`, legacy `action_authentication` exhaustiveness | Map every `LeaderAiActionPayload` to one authority operation and domain. Player-only Divine Boost activation must reject Leader/officer routes. Officer actions must pass `officer_owns_domain`; Leader/officer simulation authority cannot forge player actions. | `AuthorityDenied` includes bounded reason class only. No private eligibility, hidden inventory, or cat refusal details beyond report-safe reasons. | Boost player-only denial for Leader/officer; officer out-of-domain denial; all new payloads force explicit authority mapping. |
| Expected versions | Missing server aggregate DTO; leaf versions exist in sim leaves and LAI.23 aggregate | Check planner/domain/resource/Favor/spatial/reservation/research/scholar/boost/diplomacy/trade/prosthetic versions before replay. Stale versions stop before current preconditions and mutation. | `VersionMismatch` includes current version hints and a bounded refresh hint only. | Concurrent stale action returns refresh without debit/reservation/state change. |
| Idempotent replay | `LeaderAiRuntimeState.idempotency_receipts`, `DiplomacyLedger` receipts, `TradeLedger` receipts, boost/research leaf receipts | Use a bounded `IdempotencyReceiptStore` keyed by selected colony, verified player, protocol version, idempotency ID, payload kind, payload hash, and expected versions. Replay accepted/rejected prior results only after expected-version match and before current preconditions. | `DuplicateReplay` returns identical prior bounded result; conflicting reuse is opaque/malformed and does not mutate. | Duplicate accepted replay same result; duplicate rejected replay same rejection; mismatched payload same ID rejects; restart replay once LAI.26 persists receipts. |
| Current preconditions | Sim leaves such as `favor.rs`, `research_purchase.rs`, `scholar_research.rs`, `divine_boosts.rs`, `diplomacy.rs`, `autonomous_trade.rs`, `prosthetics.rs`, spatial/reservation leaves | Evaluate current availability, ownership, route/site, Favor, reservation, consent, same-type boost, cat willingness, and trade stage checks on candidate state after replay. | `PreconditionFailed`, `InsufficientFavor`, `ReservationConflict`, or domain conflict with report-safe state hint only. | No partial mutation on unaffordable, occupied, same-type boost, invalid route, missing public site, cat refusal, or expired trade. |
| Atomic sim mutation | Legacy `apply_action` mutates `WorldState` directly per `ClientAction`; pure leaves expose several CAS/idempotency APIs | Add one server adapter that clones or stages affected leaves, applies deterministic leaf APIs, validates cross-leaf invariants, then commits Favor/reservation/runtime state once. | Accepted result includes changed IDs and committed versions only. | Favor debit once, reservation once, runtime state once; stale and rejected paths byte-identical. |
| Persistence | `save_world`, `load_world`, current SQLite schema; LAI.26 owns migration | Persist updated world plus idempotency receipt in one transaction after commit and before success response where persistence is authoritative. Until LAI.26 lands, document runtime-only receipts as non-restart-safe and keep LAI.33 red. | Persistence failure returns bounded server conflict and does not claim acceptance unless durable policy says snapshot cache is source of truth. | Receipt/world transaction atomic; malformed rows rollback/quarantine under LAI.26; save failure cannot produce duplicate debit on retry. |
| Snapshot refresh | Legacy `build_snapshot`, `completed_snapshot`, broadcast channel | Build the authoritative post-mutation snapshot from committed state only, then pass through LAI.24 projection/redaction for the selected socket before sending refresh hints or broadcasts. | Refresh hints are report-safe and selected-colony scoped. | Accepted action refresh shows committed version; stale refresh preserves selected context without hidden truth. |
| Server-side redaction | `project_snapshot`, `project_reported_stock`, `redact_exact_functional_equipment` | Replace/extend with `ServerSideSnapshotRedactor` over `LeaderAiSnapshotEnvelope`. Redaction happens before WebSocket send and before refresh payload embedding. Bevy is never a privacy boundary. | Remove foreign private beliefs/plans, hidden stock, exact regen below L4, hidden inventory/escrow/private route danger, auth material, owner session IDs. | Serialized socket frames lack forbidden tokens; redaction is applied server-side for multi-colony sockets. |
| Bounded response send | `ServerActionResult`, `send_action_result` | Send `ServerActionResult`/`LeaderAiActionResponse` with strict typed conflicts and optional redacted refresh snapshot/hint. | Unauthorized/malformed/foreign errors are existence-safe and bounded. | Unauthorized and malformed requests cannot reveal object/colony existence by error shape or timing-sensitive expensive work. |

## Redaction contract

LAI.27 makes the server the privacy boundary. The client receives only
report-safe snapshot fields from LAI.24 and bounded conflict hints from LAI.25.
It must never receive a full canonical world or a private field and be trusted
to hide it.

Specific redaction rules:

- God/source regeneration below effective report level 4 is absent as a number
  and represented only by the LAI.24 unavailable state.
- Level 4+ regeneration is a report-derived range with provenance, not the
  authoritative timer/capacity.
- Hidden stock, exact headroom, hidden inventory, exact rejected amounts, unseen
  route danger, unseen threats, private plans, private beliefs, private officer
  notes, and omitted planner candidates are not serialized.
- Favor is exact and may be serialized only as Favor ledger state; it is not a
  physical stockpile/inventory leak.
- Foreign colony private state never appears in either snapshots or conflict
  hints. Public diplomacy/trade facts can appear only when selected-colony
  visibility/relationship permits them.
- Authentication material, session IDs, HMAC signatures, owner installation
  identifiers, and raw player peer/IP data never appear in snapshots, conflicts,
  logs intended for clients, or refresh hints.

## Multi-colony isolation

The server should derive `SelectedColonyOwnershipGuard` before any object lookup
inside a selected colony. This is stricter than the current legacy path, where
some actions reach `apply_action` and let sim action handlers reject ownership
after action-specific lookup.

Isolation requirements:

- selected colony must be visible and controllable by the verified player;
- global colony control remains an explicit policy branch, not a fallback caused
  by missing personal ownership;
- foreign personal colony IDs and foreign object IDs collapse into the same
  opaque denial shape as unknown IDs;
- trade/diplomacy lookups require that the selected colony is a party and that
  the relationship/contract is public enough to expose;
- all refreshed snapshots are projected for the connection's selected colony,
  not the colony referenced by an arbitrary payload field;
- multi-session sockets for the same player can select different owned colonies
  without sharing private plans or action receipts across colony boundaries.

## Concurrency and idempotency

Server ordering is authoritative. A mutation receives an expected version set
from the action envelope and compares it to the current committed state before
duplicate replay or preconditions.

Required behavior:

- stale expected versions return `VersionMismatch` and no partial mutation;
- duplicate accepted IDs return the original accepted result, including the same
  committed versions and changed IDs;
- duplicate rejected IDs return the identical bounded rejection when the
  original request reached receiptable validation;
- same idempotency ID with different payload, player, selected colony, protocol
  version, or expected versions is a conflict and never reuses a receipt;
- current preconditions run after replay so a retry cannot fail because later
  ticks changed the world;
- multi-leaf mutations stage candidate state and commit once to avoid half
  Favor debits, half reservations, cargo duplication, or item loss;
- persistence records the receipt and world update atomically once LAI.26 lands;
- snapshot refresh reads the committed state after persistence/commit, not a
  precommit candidate.

## Domain coverage

| Domain | Server authority and routing rule | Atomic state touched | Redaction and conflict rule | Focused tests |
| --- | --- | --- | --- | --- |
| Plans and standing orders | Authenticated player for selected colony; no officer/Leader forged player action. | Planner/intent/standing-order state and idempotency receipt. | Stale refresh shows visible plan lifecycle/rank hints only; no hidden scores or omitted candidates. | Nudge/dismiss/create/update/delete stale and duplicate replay; equal ordering deterministic. |
| Officer appointments and domains | Player can appoint/remove; officer mutations require `OfficerDomainAuthorityGuard`. | Officer institution, officer request book, planner/domain versions. | Denials show bounded domain reason only; no private officer notes. | Officer out-of-domain rejection and selected-colony ownership opacity. |
| Cat care/prosthetics | Player or explicitly authorized care domain for selected colony; patient/item/site must belong to same colony. | Cat anatomy/stress/care task, prosthetic ledger, reservations, cargo/item state. | Conflict hides hidden injury/prognosis/inventory; item identity remains conserved. | Treatment, fit, remove, repair stale/no partial mutation; foreign cat/item opaque. |
| Shrine/Favor/research | Player-owned selected colony for research purchase; Shrine automation acts through sim authority, not player mutation. | Favor ledger, research purchase state, scholar state, Shrine pipeline if action owns it. | Exact Favor allowed; hidden stock/replacement costs and hidden source regen denied. | Exact debit once, insufficient Favor no debit, research stage stale, Shrine offering conflict report-safe. |
| Divine Boost | `PlayerOnlyDivineBoostGuard`; Leader/officer actors rejected before sim purchase. | Boost state, Favor ledger, idempotency receipt. | Same-type active rejection includes visible expiry/version only; no refund/cancel. | Leader/officer denied, same-type no debit, duplicate accepted/rejected replay. |
| Diplomacy | Selected colony must be party; player approval/block only through diplomacy authority. | `DiplomacyLedger`, relationship version, idempotency receipt. | Foreign/nonparty pair opaque; consent state public only when relationship visible. | Two-session consent, block/approve race, stale pair version, duplicate replay. |
| Trade | Selected colony must be party; relationship/consent state must permit contract action. | `TradeLedger`, reservation/escrow/cargo state, diplomacy relationship version, idempotency receipt. | Valuation references report IDs and ranges only; no hidden stock/headroom/route danger or nonparty private escrow. | Final accept escrows once, reject/cancel releases once, stale no reservation, recovery stage refresh. |
| Physical placement/workshop extensions | New actions must enter the same envelope/pipeline; no legacy direct placement after cutover. | Spatial/reservation/resource/cargo/planner versions as declared by action. | Site conflicts hide reservation loser and hidden source alternatives. | Workshop/action extension tests prove explicit payload, authority, expected versions, redacted stale refresh, and no fallback route. |

## API gaps

- `cat-protocol` must first provide LAI.24 `LeaderAiSnapshotEnvelope` and LAI.25
  `LeaderAiActionEnvelope`/conflict DTOs. LAI.27 should not invent server-local
  shadow DTOs that differ from protocol.
- `main.rs` needs a LAI.27 router separate from legacy `handle_client_text` until
  cutover deletion. The old route can remain for legacy runtime until the planned
  deletion slice, but LAI.25 envelopes must never fall through to
  `serde_json::from_str::<ClientAction>` or `apply_action`.
- `identity.rs` needs a typed verified-session guard or constructor that hides
  raw `sig` and returns a verified player/session identity for the pipeline.
- `rate_limit.rs` can be reused, but `main.rs` needs a named LAI.27 mutation
  limiter/check point whose order is testable before world/db/snapshot work.
- `AppState` needs storage for bounded idempotency receipts or access to the
  LAI.23 aggregate store until LAI.26 persists receipts durably.
- `persistence.rs` does not yet persist LAI.26 idempotency receipts or aggregate
  runtime versions transactionally with post-cutover world state.
- `project_snapshot` only handles legacy `WorldSnapshot` and stock/equipment
  redaction. LAI.27 needs a redactor for LAI.24 `LeaderAiSnapshotEnvelope` and
  typed refresh hints.
- Existing sim `actions::apply_action` is legacy. Server adapters need typed
  calls into pure leaf APIs and staged multi-leaf commit semantics for Favor,
  reservations, research, boosts, diplomacy, trade, prosthetics, and physical
  placement.

## Minimal production slices

1. Protocol dependency slice, owned by LAI.24/LAI.25:
   add snapshot/action envelopes, conflict/result DTOs, version constants, and
   strict bounded serde tests. LAI.27 starts after these names are real.
2. Server router slice, owned by LAI.27:
   add `LeaderAiServerMutationPipeline`, raw version preflight,
   `VerifiedPlayerSession`, mutation limiter check, ownership/authority guards,
   expected-version/replay/precondition order, and typed bounded responses in a
   small `cat-server` module plus minimal `main.rs` hook.
3. Redaction slice, owned by LAI.27 with LAI.24 DTOs:
   add `ServerSideSnapshotRedactor` and selected-colony projection for
   `LeaderAiSnapshotEnvelope`; prove it runs before WebSocket send and refresh
   response embedding.
4. Persistence slice, owned by LAI.26:
   persist runtime state, state versions, idempotency receipts, and action
   receipt expiry transactionally; provide rollback/quarantine behavior for
   malformed rows.
5. Sim adapter slices, owned by LAI.23/domain owners:
   expose small pure mutation adapters with expected versions and candidate
   commit for planner, officers, care/prosthetics, research/scholars, boosts,
   diplomacy, trade, and physical placement/workshop extensions.
6. Cutover/deletion slice:
   only after LAI.24-27 are green, stop accepting legacy mutation payloads for
   the new surfaces and return `UPDATE_REQUIRED` for old clients.

## Required tests

Focused LAI.27 server tests should cover:

- `lai27_server_pipeline_is_single_authoritative_ordered_path`
- `compatibility_update_required_and_hmac_auth_fail_before_route_selection`
- `ownership_and_actor_authority_cover_player_only_boosts_and_officer_domains`
- `expected_versions_replay_preconditions_and_commit_are_atomic`
- `conflicts_are_typed_bounded_refreshable_and_existence_safe`
- `rate_limiting_runs_before_expensive_world_or_database_work`
- `multi_colony_isolation_and_server_side_snapshot_redaction_are_enforced`
- `protocol_contract_is_not_satisfied_by_legacy_action_result_or_snapshot_types`

Additional green implementation tests should prove:

- old protocol frames return `UPDATE_REQUIRED` before nested decode, auth,
  route, world lock, database transaction, or snapshot refresh;
- unauthenticated, malformed, unknown, and foreign requests do not reveal object
  or colony existence through conflict type, message, refresh hint, or timing of
  expensive work;
- rate limiting fires before world/db/snapshot work and returns a bounded
  response;
- duplicate accepted and rejected idempotency IDs return byte-identical prior
  results without another debit/reservation/state change;
- stale expected versions never partially mutate and return a redacted refresh
  hint;
- player-only boosts deny Leader/officer actors and do not debit on same-type
  active rejection;
- diplomacy consent works across two authenticated sessions with selected-colony
  isolation;
- trade accept/reject/escrow/pickup/delivery/recovery stages conserve cargo and
  reveal only report-safe public facts;
- server-side snapshot redaction removes hidden stock, private plans/beliefs,
  auth material, foreign private state, and exact regeneration below L4;
- future Workshop or physical-action payload additions fail to compile until
  they declare protocol bounds, authority domain, expected versions, idempotency
  receipt shape, redacted conflicts, and tests.

## Extension rule for future actions

Every new workshop/action extension must enter the same LAI.27 pipeline. The
extension is not accepted if it adds a direct `ClientAction` branch, bypasses
outer version preflight, reads selected-colony objects before ownership is
established, omits expected versions for a mutated domain, returns unbounded
string errors, or relies on the client to hide private state. Add protocol,
server, pure sim, persistence/restart, stale-client, duplicate replay,
multi-colony, redaction, and rate-limit-before-expensive-work tests before UI
controls are exposed.
