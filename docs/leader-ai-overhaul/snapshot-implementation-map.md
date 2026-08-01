# LAI.24 snapshot implementation readiness map

> Historical first-cutover readiness evidence only. Current full snapshot/redaction authority is in
> [`integrated-implementation-map.md`](integrated-implementation-map.md); obsolete Shrine/Favor and
> partial LAI.24 contracts below must not be restored.

This is a production-readiness map for LAI.24 only. It does not implement protocol
DTOs, does not change `world_tick`, does not change `cat-protocol` production
types, and does not update board status.

## Sources read

- LAI.24 red contract: [`../../crates/cat-protocol/tests/lai24_snapshot_contract.rs`](../../crates/cat-protocol/tests/lai24_snapshot_contract.rs)
- Current protocol DTO root: [`../../crates/cat-protocol/src/lib.rs`](../../crates/cat-protocol/src/lib.rs)
- Wire and persistence contract: [`wire-persistence-ui.md`](wire-persistence-ui.md)
- Spatial task contract: [`spatial-task-contract.md`](spatial-task-contract.md)
- Cat care contract: [`cats-and-care.md`](cats-and-care.md)
- Hole, Notes/Void, two-lane research, boost, and miracle contract:
  [`hole-research-progression.md`](hole-research-progression.md)
- Diplomacy and trade contract: [`diplomacy-trade.md`](diplomacy-trade.md)
- Runtime aggregate foundation: `crates/cat-sim/src/leader_ai_runtime.rs`
- Spatial/task leaves: `crates/cat-sim/src/spatial_tasks.rs` and
  `crates/cat-sim/src/task_runtime.rs`
- Belief/report leaf: `crates/cat-sim/src/beliefs.rs`
- Shrine/Favor/research/boost leaves: `crates/cat-sim/src/shrine_offerings.rs`,
  `crates/cat-sim/src/favor.rs`, `crates/cat-sim/src/research_manifest.rs`,
  `crates/cat-sim/src/research_purchase.rs`, `crates/cat-sim/src/scholar_research.rs`,
  and `crates/cat-sim/src/divine_boosts.rs`
- Cat care leaves: `crates/cat-sim/src/cat_traits.rs`, `crates/cat-sim/src/anatomy.rs`,
  `crates/cat-sim/src/injuries.rs`, and `crates/cat-sim/src/prosthetics.rs`
- Diplomacy/trade leaves: `crates/cat-sim/src/diplomacy.rs`,
  `crates/cat-sim/src/trade_valuation.rs`, and `crates/cat-sim/src/autonomous_trade.rs`

## Current readiness summary

`cat-sim` now has a persistence-ready aggregate source in
`LeaderAiRuntimeState`, with versioned leaf aggregates for planner/intents,
beliefs, officers, visible tasks, Shrine/Favor, research/scholars, boosts,
diplomacy, trade, cats, prosthetics, and idempotency receipts. LAI.24 should
consume that aggregate and its leaf types; it should not duplicate the leaf
validators or invent a second task/planner/shrine/trade model in protocol.

`cat-protocol` still exposes the legacy `WorldSnapshot`/`ColonySnapshot` root
with `PROTOCOL_VERSION: u32 = 1`. The LAI.24 red contract intentionally expects
missing DTOs such as `LeaderAiSnapshotEnvelope`, `SnapshotProtocolVersion`,
`BeliefReportSnapshot`, `VisibleTaskSnapshot`, `SiteRefSnapshot`,
`CatTraitsSnapshot`, `FavorLedgerSnapshot`, `ResearchFrontierSnapshot`,
`DivineBoostSnapshot`, `DiplomacySnapshot`, and `TradeContractSnapshot`. The
implementation owner should add these as a new protocol snapshot leaf, then
thread that leaf into the post-cutover snapshot builder in the later server
slice.

## Smallest implementation ownership slice

The narrow production slice is:

1. Add `crates/cat-protocol/src/leader_ai_snapshot.rs` with the LAI.24 DTOs,
   version constants, strict serde attributes, bounded string/ID/newtype
   validators, and JSON round-trip helpers used by tests.
2. Add a minimal export from `crates/cat-protocol/src/lib.rs` without changing
   unrelated legacy wire DTOs beyond exposing the new module and, when the
   protocol owner is ready, the new protocol version constant.
3. Add focused protocol tests beside `lai24_snapshot_contract.rs` for positive
   round trips, ordering, unknown-field rejection, and forbidden-token leak
   checks.
4. Defer actual server snapshot construction to the LAI.24/LAI.27 integration
   owner. That owner should read `LeaderAiRuntimeState` and current public world
   snapshot data, then construct `LeaderAiSnapshotEnvelope` at the existing
   authoritative snapshot boundary.

Do not change `world_tick.rs`, `cat-sim` mutation leaves, SQLite, client UI,
or server routing while implementing this DTO leaf. If a conversion needs a
field not present in `LeaderAiRuntimeState`, add the missing persisted source in
the appropriate sim leaf first; do not smuggle it into protocol as a shadow
runtime field.

## DTO conversion map

| Required DTO or field family | Exact sim source type | Conversion and redaction rule | Stable ordering | Validation and round trip | Forbidden hidden leak or current gap |
| --- | --- | --- | --- | --- | --- |
| `LeaderAiSnapshotEnvelope` root | `LeaderAiRuntimeState` plus existing public `WorldSnapshot` context | Serialize only selected-colony LAI state and public cross-colony facts. Include protocol version, nested LAI schema version, colony ID, snapshot tick/version, generated-at tick, and typed sections. | Sections appear in fixed struct field order; collection fields below use sorted IDs. | `#[serde(deny_unknown_fields)]`; reject unsupported protocol versions, unknown enum variants, malformed nested versions, and duplicate stable IDs. Golden JSON should round-trip byte-stably after canonical serialization. | Current `cat-protocol` has no envelope. Never include private state for another colony, HMAC/session material, hidden inventory, hidden regeneration truth, or private plans. |
| `SnapshotProtocolVersion` and schema guards | `cat-protocol::PROTOCOL_VERSION`, `LeaderAiRuntimeState::schema_version`, leaf `schema_version` fields | Protocol version is a wire compatibility guard; LAI schema version validates nested runtime compatibility. Runtime versions are evidence only, not client mutation authority. | Scalar. | Decode must fail closed on old/new incompatible versions and unknown snapshot variants. Positive tests should prove exact accepted version and rejected `PROTOCOL_VERSION > 1` when compiled against legacy root. | Current protocol root remains version 1. Production owner must avoid shadow version constants with names that conflict with legacy wire semantics. |
| `BeliefReportSnapshot` | `BeliefRuntimeAggregate` containing `BeliefState`, `ReportArchive`, and report freshness state | Project each `BeliefRecord`/`BeliefProjection` into report-safe estimate/category/trend fields only. Emit source report IDs, provenance, confidence basis points, observed/expiry age, and supersession/invalidated state. | Sort by stable belief key, then report/evidence ID. | Bounds: confidence 0..=10000, age nonnegative, expiry >= observed when present, report-safe strings bounded. Round-trip must preserve ranges and provenance IDs. | Never expose `hidden_truth`, authoritative quantity, exact stock for private stores, or a raw planner-only confidence calculation. |
| `ReportEstimateSnapshot` | `EstimateRange`, `ProjectedBeliefValue::StockRange`, `FlowRange`, `FlowRate`, `RegenerationRange` | Convert estimate/lower/upper into minimum/maximum plus optional midpoint display. Use bounded integer or basis-point units matching source quantity units. | Nested under sorted belief reports. | Validate minimum <= maximum, confidence bounds, unit enum known, no NaN/float fields. | Never serialize exact authoritative stock or exact regeneration under a range wrapper. |
| `RegenerationReportSnapshot` | `BeliefRecord.report_level`, `BeliefProjection.report_level`, `ReportLevel::regeneration_visible`, `ReportLevel::regeneration_error_basis_points` | Effective report level below 4 serializes only `UnavailableBelowLevel4`. Level 4+ serializes a range using the level-specific error basis points and provenance. | Nested under sorted report keys. | Test levels 1, 2, and 3 all reject any regen numeric field; levels 4 and 5 require a bounded range and source evidence. | God regeneration is unavailable below L4. Never leak `exact_regeneration`, exact hidden replenishment, or future source ecology. |
| `PlanQueueSnapshot` and `PlanSnapshot` | `PlannerCoreState`, `IntentGraph`, `Intent`, `IntentLifecycle`, `IntentReason`, `PlannerScore`, `IntentTieKey` | Emit top report-safe plans only: plan/intent IDs, lifecycle/status, domain/category, bounded rationale, visible dependencies, expected cost/benefit ranges, score/confidence ranges, and authority hints. | Planner queue order must use the existing deterministic score/tie order; equal-score twins sort by stable ID. | Validate top-N bound, unique plan and intent IDs, existing referenced visible task IDs, bounded rationale strings, and deterministic serialization under permutation twins. | Never expose private full candidate queues, hidden belief truth, omitted intents, RNG seeds, or exact planner weights beyond report-safe ranges. |
| `OfficerRequestSnapshot` | `OfficerRuntimeAggregate`, `OfficerRequestBook`, `OfficerRequest`, `OfficerRequestState`, `OfficerRole` | Emit visible officer requests with request ID, role/domain, state, bounded reason/block reason, created/expiry ticks, linked intent/task IDs when visible, and whether action requires player approval. | Sort by request book order if exposed by the leaf; otherwise `(role, request_id)`. | Reject duplicate request IDs, invalid role/domain enum, expired dangling task links, and cycles across requests. | Never expose private officer notes, hidden inventory estimates, or another colony's requests. |
| `VisibleTaskSnapshot` | `VisibleTaskRuntime`, `TaskCategory`, `TaskStage`, `SpatialObjective`, `TaskCargo`, `CargoLocation` | Emit task ID, occurrence, colony ID, intent ID, category, stage, assigned cat IDs, objective `SiteRefSnapshot`, work slots, endpoint, route IDs, reservation ID, progress basis points, cargo references, bounded block reason, and updated tick. | Tasks sort by `(updated_tick, task_id)` or explicit scheduler order; assigned cats, route IDs, and cargo IDs sort lexicographically if the source set has no order. | Validate unique task IDs, progress 0..=10000, assigned cats exist, task colony matches selected colony, cargo quantities nonnegative, and every visible reference resolves. Restart round-trip should preserve exact visible task identity and stage. | Do not expose invisible reservations, hidden source alternatives, private cargo candidates, or blocked/redacted sites. |
| `CatSnapshot.active_task_id` | `VisibleTaskRuntime.assigned_cat_ids`, `CatRuntimeState.cat_id` | For each authorized cat, set active task only when exactly one visible task assigns the cat; otherwise omit or report a bounded conflict according to DTO contract. | Cat snapshots sort by stable cat ID. | Validate active task references a visible task from same colony and the task assignment includes that cat. | Never infer active work from hidden scheduler reservations or movement targets not in `VisibleTaskRuntime`. |
| `SiteRefSnapshot::Tile` | `SiteRef::Tile { tile, metadata }` | Emit exact revealed tile coordinate and report-safe site metadata. | Scalar. | Validate coordinate bounds using public map dimensions and metadata stable ID. | No marker for unrevealed, blocked, redacted, or missing tile. |
| `SiteRefSnapshot::AnchoredRect` | `SiteRef::Rect { anchor, width, height, metadata }` | Emit anchor plus dimensions for revealed rectangular work areas. | Row-major derived order when converted to cells. | Width/height must be positive and bounded; generated cells must remain in map bounds. | Do not collapse physical rectangles into radial/generic client hints. |
| `SiteRefSnapshot::OrderedTileSet` | `SiteRef::OrderedTiles { tiles, metadata }`, `OrderedTiles`, `TaskFootprint` | Emit the exact ordered tile list from the spatial leaf. | Preserve source order; for canonical footprints the source must already be row-major. | Reject duplicates, empty sets, out-of-bounds cells, and noncanonical count for known footprints. | Do not reorder by client viewport or pathfinding distance. |
| `SiteRefSnapshot::BuildingFootprint` | `SiteRef::Building { building_id, building_type, anchor, footprint, metadata }` | Emit stable building ID/type and exact footprint cells. For Workshop, route to `WorkshopFootprintSnapshot`. | Preserve footprint order from `TaskFootprint`; Workshop is row-major 3 x 3. | Validate building ID, type enum, anchor consistency, footprint count, and no duplicate cells. | Do not expose planned-but-hidden buildings or unbuilt private future expansions. |
| `SiteRefSnapshot::StockpileFootprint` | `SiteRef::Stockpile { stockpile_id, footprint, metadata }` | Emit stockpile ID and visible footprint only, never hidden stock content. | Preserve footprint order. | Validate known stockpile ID and footprint cells. | Hidden inventory counts and exact headroom are forbidden. |
| `SiteRefSnapshot::ResourceSource` | `SiteRef::ResourceSource { source_id, resource_kind, footprint, metadata }` | Emit source identity, resource kind, and visible footprint for revealed reachable sources. | Preserve footprint order. | Validate source ID, kind enum, reveal status, and reachable/usable lifecycle state. | No redacted source, no hidden depletion/regeneration quantity, no generic fallback marker. |
| `SiteRefSnapshot::HuntSource` | `ResourceSource` SiteRef plus task `TaskCategory::Hunt` | Hunt objective must be the actual revealed reachable cave or hunting-source identity selected by the task, not a generic hunting radius. | Single source per visible task; stable by task ID/source ID. | Test source ID survives JSON restart round-trip and client marker contract can key to it. | Must not reveal caves not known to the selected colony or alternative prey/source candidates. |
| `SiteRefSnapshot::WaterSourceAndBank` | `ResourceSource`/`OrderedTiles`/`Tile` SiteRefs in `SpatialObjective` for `TaskCategory::FetchWater` | Emit three distinct refs: actual water source, reachable dry bank/work tile, and pinned delivery endpoint. The work tile must not be conflated with the source. | Source, bank/work, endpoint in fixed field order. | Validate all three refs exist, are same-colony visible, source is water kind, bank is reachable/dry, and endpoint is a delivery-capable site. | Do not render one combined marker, inferred route endpoint, or hidden water alternatives. |
| `WorkshopFootprintSnapshot` | `SiteRef::Building`, `TaskFootprint`, `SpatialObjective.work_positions` | Emit `width = 3`, `height = 3`, and exactly nine ordered row-major cells for the canonical Workshop objective, plus distinct work-slot and delivery markers. | Nine cells row-major from anchor; work slots sorted by `WorkSlot.stable_id`; delivery endpoint fixed after slots. | `validate_workshop_three_by_three` and `validate_nine_row_major_tiles`; reject missing/extra/duplicate cells and wrong order. | No 1-cell workshop shortcut, no radial work marker, no client-generated grid. |
| Tree/resource multi-cell footprints | `SiteRef::ResourceSource`, `OrderedTiles`, `TaskFootprint` | Emit all canonical cells where applicable, including the six-cell tree footprint required by LAI.29. | Preserve canonical source order. | Validate known footprint shape/count by resource kind when the spatial leaf exposes that kind. | No fallback to source center only. |
| `WorkSlotSnapshot` and endpoint markers | `SpatialObjective.work_positions`, `WorkSlot`, `WorkSlotReservation`, `SpatialObjective.delivery_endpoint` | Emit stable work-slot ID, role, site ref, optional visible reservation, and endpoint site ref. | Sort work slots by stable slot ID unless source order is semantically meaningful. | Reject dangling reservation/task/cat references and endpoint role/type mismatches. | Do not expose private reservation queues or hidden blocked alternatives. |
| `CatTraitsSnapshot` and attributes | `CatRuntimeState`, `CatTraits`, migrated innate `CatAttributes`, `CatPersonality`, acquired trait state | Emit stable cat identity, innate attribute breakdown, learned skills/office experience if present in the source leaf, personality axes, and acquired traits. | Cats by cat ID; traits/skills by stable key. | Validate bounded attribute ranges, known trait IDs, no duplicate skill/trait keys, and deterministic restart round-trip. | Do not recompute traits from hidden ancestry or expose breeding/private lifetime rolls. |
| `StressSnapshot` and `WillingnessSnapshot` | `CatRuntimeState.stress`, `cat_stress::StressState`, willingness/refusal calculation leaf | Emit stress/recovery/refusal state and bounded willingness reasons already available to the leader/report system. | Reasons sort by stable reason key or source priority. | Bounds on stress/recovery basis points and reason text; stale refresh must preserve selected cat ID. | Client must not recompute willingness from hidden exact needs, private stress causes, or regeneration. |
| `AnatomySnapshot` | `CatRuntimeState.anatomy`, `BodyPart`, `BodyPartState`, `BodyPartCondition` | Emit complete four-paw, two-eye, and tail anatomy with side/type/condition and treatment eligibility. | Canonical anatomy order: front-left paw, front-right paw, rear-left paw, rear-right paw, left eye, right eye, tail. | Reject missing body parts, duplicates, impossible side/type pairs, and unknown conditions. | Never omit injured parts for display convenience or expose hidden future healing rolls. |
| `InjurySnapshot` and `TreatmentSnapshot` | `injuries` incident/treatment state plus `anatomy` treatment transitions | Emit injury identity when persisted, affected body part, severity/status, active treatment task/site/cargo references, and bounded block/eligibility reasons. | Injuries by injury ID, then body-part canonical order. | Validate active care task/site/cargo references resolve through visible tasks and no cargo/item ID is duplicated. | No hidden incident RNG rolls, private prognosis truth, or regeneration below L4. |
| `ProstheticSnapshot` | `ProstheticLedger`, `ProstheticItem`, `ProstheticLocation`, `ProstheticMaterial`, anatomy fitted state | Emit prosthetic item ID, side/body-part, type/material, restoration, durability, wear, fitted/reserved/free location, and repair/removal eligibility. | Prosthetics by item ID; fitted prosthetics additionally in anatomy order. | Validate nonnegative durability/wear bounds, one item ID in one location, fitted side matches anatomy loss, and restart equality. | Do not manufacture replacement identity or duplicate cargo/item identity across fit/remove/repair/trade. |
| `CareStatusSnapshot` | `CatRuntimeState`, `VisibleTaskRuntime`, `TaskCargo`, prosthetic/anatomy leaves | Emit active care task, care site, cargo item IDs, treatment state, prosthetic action state, consent/refusal, and bounded eligibility/block reasons. | Nested under cat ID; care tasks by task ID. | Validate references resolve and state version/idempotency metadata can route future LAI.25 actions. | Client cannot infer hidden care tasks or hidden cargo from exact inventory. |
| `ShrineOfferingPipelineSnapshot` | `ShrineFavorRuntimeAggregate.shrine_offerings`, `ShrineOfferingState`, `OfferingPipeline`, `OfferingStage`, `OfferingPackage`, `OfferingCargoDisposition` | Emit one pipeline per Shrine, selected package, belief-based replacement-cost rationale/provenance, stage, source/haul/ritual credit, cargo disposition, pinned Shrine endpoint, omission/block reason, and restart/idempotency IDs. | Shrines by stable shrine/building ID; pipeline stages by shrine ID then stage enum order. | Validate one active pipeline per Shrine, known package, nonnegative exact credits, known stage, exact cargo IDs where visible, and no cooldown/tithe/completion gate fields. | Never expose hidden stock/replacement truth or hidden better package candidates; no implicit second pipeline. |
| `OfferingPackageSnapshot` | `OfferingPackage` | Emit exactly the four one-Favor physical packages from the sim leaf, including resource requirements and expected one-Favor credit. | Fixed package enum order from leaf. | Reject unknown package and malformed/negative resource quantities. | No cooldown, tithe, completion gate, or nonphysical offering shortcut. |
| `FavorLedgerSnapshot` | `FavorLedger`, `Favor`, `FavorEvent`, `FavorEventKind`, `FavorDirection` | Emit exact nonnegative micro-Favor balance, committed event IDs, direction/kind, linked source IDs, and idempotency receipt references. | Events by committed ledger order or `(tick, event_id)` if exposed; balance scalar. | Validate nonnegative balance and event amounts, duplicate event replay returns identical prior result, and sum(events) matches balance when full history is included. | No mirrored currency, client-side balance derivation, negative Favor, or refund/cancel fields not present in leaf rules. |
| `ResearchFrontierSnapshot` | `ResearchManifest`, `ResearchStudy`, `ResearchManifestError`, `RESEARCH_MANIFEST_STUDY_COUNT`, `ResearchPurchaseState`, `ResearchPurchaseEvent` | Emit the 531-study manifest frontier with study IDs, prerequisites, track/stage, effects, affordability/status, purchase events, and source Favor debits. | Manifest order from `ResearchManifest::studies()`; purchases by event ID/tick. | Assert manifest count 531, all referenced prerequisites/effect targets exist, unknown effect variants reject, and restart preserves committed purchase stages. | Red contract searches for `MANIFEST_STUDY_COUNT: usize = 531`; current sim constant is `RESEARCH_MANIFEST_STUDY_COUNT`. Protocol may expose a DTO constant/field but should not shadow or weaken the sim manifest source. |
| `AutomaticResearchQuotaSnapshot` | `ResearchPurchaseState` quota/window fields | Emit quota used, quota limit, window started tick/ms, reset horizon, and source evidence. | Scalar. | Validate used <= limit, nonnegative window, deterministic restart equality, and no hidden pending purchases. | Do not revive old research points or hidden automatic research pools. |
| `InsightSnapshot` and scholar/preparation DTOs | `ScholarResearchState`, `ScholarProgress`, `ScholarWorkEvent`, `ScholarWorkAuthorization`, `ResearchTrackStages`, `ResearchRuntimeEffects` | Emit scholar IDs, work progress, Insight/preparation state, assigned study/track, player discount state, and bounded block reason. | Scholars by `ScholarId`; work events by event ID/tick. | Validate scholar IDs unique, preparation references valid study IDs, progress bounds, and committed runtime effects match manifest stages. | No hidden research candidates, unbounded internal scores, or private scholar planning notes. |
| `DivineBoostSnapshot` | `DivineBoostState`, `DivineBoostType`, `DivineBoostResearchStages`, `UnlockedBoostDurations`, `DivineBoostPurchaseEvent` | Emit boost type, activation/end tick, cost, duration, committed duration/economy stages, effect percent, active/expired status, and player-only activation metadata. | Active boosts by `(boost_type, activation_id)`; expired/history by event ID if exposed. | Validate four boost types only, unlocked duration table, economy cap, ceil cost, same-type active rejection evidence, and exact fine/batched/restart expiry. | Later research must not alter active effects. No leader/officer activation/reservation, stacking, reset, cancel, or refund field. |
| `DiplomacySnapshot` | `DiplomacyLedger`, `DiplomacyPair`, `DiplomacyRecord`, `DiplomacyRelationship`, `DiplomacyAction`, `DiplomacyReceipt`, `DiplomacyAuthorization` | Emit public pair ID, counterpart colony public identity, relationship state/version, consent requirement/state, last public action/result, bounded block reason, and expected diplomacy version for future actions. | Pairs by canonical unordered `DiplomacyPairId`; receipts/actions by event ID/tick. | Validate canonical pair, known relationship enum, selected colony is one party, public counterpart exists, and action receipts are idempotent. | No private beliefs, private plans, hidden inventory, auth material, or another colony private state. Unauthorized/malformed snapshots must not reveal existence. |
| `TradeContractSnapshot` | `TradeLedger`, `TradeContract`, `TradeProposal`, `TradeParty`, `TradeStage`, `TradeCargoLeg`, `TradeRecoveryState`, `TradeReceipt`, `TradeAuthorization` | Emit public contract ID/version, parties, proposal status, valuation report references, escrow state, cargo IDs/kinds/quantities, route IDs, pickup/delivery endpoints, stage/recovery, bounded failure reason, and expected trade version. | Contracts by contract ID; cargo legs and route checkpoints by source order; receipts by event ID/tick. | Validate selected colony participates, diplomacy pair is visible, cargo quantities nonnegative, route/site refs resolve, escrow item IDs conserved, stale/concurrent replay is idempotent. | No direct hidden inventory valuation, no private trade hints, no other colony escrow internals, and no legacy NPC trader conflation. |
| `PrivateColonyStateSnapshot` rejection | N/A; negative test type only | The production DTO set should not contain a serializable private-state variant. Multi-colony public summaries must be explicit, typed, and bounded. | N/A. | Unknown/private variants fail closed; malformed rows in future persistence are quarantined before snapshot emission. | Current red contract explicitly rejects `owner_session_id`, `hmac`, `private_beliefs`, `hidden_inventory`, and `private_plans`. |
| Bounded strings, IDs, ages, and basis points | `PlannerId`, `IntentId`, `TaskId`, leaf IDs, `Confidence`, progress basis-point fields | Wrap all public strings and IDs in bounded DTO newtypes or constructors. Convert basis points with checked 0..=10000 validation and age/tick deltas with nonnegative checked arithmetic. | Each collection chooses one documented key; no `HashMap` iteration reaches the wire. | Positive and negative serde tests for empty IDs, overlong strings, unknown enum variants, duplicate IDs, invalid refs, and permutation twins. | No unbounded debug strings, panic-derived errors, memory-amplifying payloads, or map-order nondeterminism. |

## Required report-safe field details by feature

### Physical task objectives

`VisibleTaskSnapshot` must be the only source for client markers. Hunt uses the
actual `TaskCategory::Hunt` objective `SiteRef` for the revealed cave or hunting
source. Fetch Water uses three separate refs: a water source, a reachable dry
bank/work tile, and the delivery endpoint. Workshop uses the canonical building
footprint with `width = 3`, `height = 3`, and nine row-major cells, plus
separate work-slot and delivery markers. Tree/resource objectives must emit all
canonical cells available from `TaskFootprint`; if the spatial source cannot
prove the required six-cell tree shape, LAI.24 should expose no marker and log a
bounded missing-site conflict instead of falling back to a center tile.

### Beliefs and regeneration secrecy

Belief snapshots are report projections, not truth dumps. Regeneration below
effective report level 4 is serialized as unavailable only; levels 4 and 5 may
serialize ranges with provenance and confidence. The DTO should make the
unavailable state impossible to confuse with zero regeneration.

### Plans and officer requests

Plans and officer requests should publish bounded rationale, visible dependency
IDs, lifecycle, and action authority. The queue must preserve deterministic
planner ordering and cap the visible list. Hidden candidates, omitted intents,
private belief values, and exact planner weights are not part of LAI.24.

### Cats and care

Cat panels need stable identity, innate attribute breakdown, learned skills and
office experience where present, personality axes, acquired traits, stress and
recovery, refusal/willingness reasons, full anatomy, injury/treatment state,
prosthetics, and active care task/site/cargo references. The snapshot must
preserve item/cargo identity and must not let the client recompute treatment,
refusal, healing, or prosthetic eligibility from hidden truth.

### Shrine, Favor, research, and boosts

Shrine snapshots expose physical offering pipeline state and exact Favor ledger
state. Offering choice rationale may cite belief estimates and provenance but
not hidden replacement stock. Research snapshots expose the manifest frontier,
purchase/quota/scholar/preparation state, and committed effects. Boost snapshots
expose active type, duration, expiry, cost, and committed research stages; later
research never rewrites active boost economics or effects.

### Diplomacy and trade

Diplomacy snapshots show selected-colony public relationships, consent state,
versions, and bounded failure reasons. Trade snapshots show public physical
contract, escrow, cargo, route, endpoint, stage, and recovery state. Cross-colony
data is public relationship/trade fact only; selected colony private beliefs,
plans, inventory, auth material, and nonparty colony state remain absent.

## Compile and API gaps to close before production

- `cat-protocol` lacks the LAI.24 root envelope, DTO modules, strict bounded
  helpers, and serde fail-closed tests expected by the red contract.
- `cat-protocol::PROTOCOL_VERSION` remains `1`; the owner must decide whether
  LAI.24 introduces a new LAI-specific nested version first or bumps the global
  protocol version in the same PR as server routing compatibility.
- The red contract expects a manifest count token named
  `MANIFEST_STUDY_COUNT: usize = 531`; the sim source currently exposes
  `RESEARCH_MANIFEST_STUDY_COUNT`. The protocol DTO can expose a stable wire
  field for count, but production conversion should read the sim constant rather
  than duplicating the manifest.
- `LeaderAiRuntimeState` is the correct aggregate source, but no protocol
  conversion trait/function currently ties it to `WorldSnapshot` or server
  selected-colony authorization. That glue belongs in the LAI.24/LAI.27
  integration slice, not this documentation task.
- Some care details such as learned skills, office experience, and active
  treatment task linkage may require checking whether the data is already stored
  in `CatRuntimeState` or only derivable from adjacent leaves. If missing, add
  source fields to the relevant sim leaf before exposing protocol DTO fields.
- SiteRef validation can prove exact shape/order only when the spatial leaf
  carries the canonical footprint. LAI.24 should fail closed for malformed or
  absent footprints instead of inventing client-compatible approximations.

## Minimum focused tests for the implementation owner

1. Red contract green-up: all tests in `lai24_snapshot_contract.rs` compile and
   pass without fake shims.
2. Positive JSON round-trip for a synthetic envelope covering every DTO family.
3. Negative serde tests for unknown envelope version, unknown enum variant,
   unknown field, duplicate ID, empty/overlong ID, invalid basis points, invalid
   age/window, negative Favor/cargo, and dangling task/site/cargo/reservation
   reference.
4. Ordering tests for plans, visible tasks, SiteRef ordered tiles, Workshop nine
   cells, cats/anatomy, Favor events, research frontier, boost history,
   diplomacy pairs, and trade contracts.
5. Redaction tests that scan serialized JSON for forbidden tokens:
   `hidden_truth`, `authoritative_quantity`, `exact_regeneration`,
   `owner_session_id`, `hmac`, `private_beliefs`, `hidden_inventory`,
   `private_plans`, and any private endpoint/auth field.
6. Restart/permutation twins proving stable byte serialization after canonical
   sorting and stable equality across fine/batched/restart expiry for boosts,
   tasks, shrine offerings, and trade contracts.

## Implementation checklist

- Add the new protocol snapshot module and re-export it.
- Use strict serde on every DTO and strict tagged enums for every snapshot
  variant.
- Keep all ID/string/basis-point/age constructors checked and bounded.
- Convert from `LeaderAiRuntimeState` and leaf types; do not duplicate sim
  validation logic in protocol.
- Fail closed for malformed versions, duplicate IDs, dangling references,
  invalid Shrine/boost/research stages, negative Favor, and hidden-regeneration
  projection fields.
- Keep multi-colony data selected-colony scoped with explicit public
  relationship/trade summaries only.
- Leave `world_tick`, server routing, persistence, and client rendering for
  their owning LAI.24/LAI.27/LAI.29-LAI.31 slices.
