# Extending the Leader Intelligence System

This is the contributor contract for extending the post-cutover system without reintroducing
parallel registries, hidden omniscience, collection-order behavior, or partial spatial truth. Read
[README.md](README.md) for precedence and subsystem ownership, then the domain design linked by the
recipe below. Current file names are listed explicitly; where an LAI card creates a focused leaf
module, that leaf becomes the behavior owner and the named current root remains integration-only.

## Canonical post-merge authority map

This section is the mandatory 2026-07-25 cutover correction for every recipe below. The older
step-by-step material is retained because its failure, conservation, spatial, redaction, and test
detail is still required; when an older paragraph names an interim LAI.24/25 type, a legacy root, a
compatibility alias, or an in-place migration, use the canonical owner and fresh-schema rule here.
Do not implement both paths.

| Extension concern | Canonical owner that must be changed |
|---|---|
| Content/resource/food/item/material/creature/station/recipe/capability/art identity | `crates/cat-sim/src/content_manifest.json` plus validation and typed IDs in `content_manifest.rs`; behavior-specific immutable definitions stay in `food_ecology.rs`, `cookhouse.rs`, `fishing.rs`, `hunting_lair.rs`, or `material_crafting.rs`. |
| Building type, complete footprint, level 1–10 bill, 20/60/20 work, permit, phase art, and miracle eligibility | Shared `BuildingType`, `spatial_tasks::footprint_for`, immutable `construction_catalog.rs`, mutable `construction_stages.rs`, and the closed manifest table documented in [construction-miracle-value-authority.md](construction-miracle-value-authority.md). A Workshop-like station carries its full 3×3 footprint once; the client never owns a second size. |
| Physical lot/item/container/storage truth | `quality_lots.rs` and `storage_authority.rs`; `physical_storage.rs` remains a descriptor leaf. Preserve stable identity, quality, age, provenance, reservation, exact location, and capacity. |
| Sites, routes, reservations, tasks, workers, outcomes, and XP | `spatial_tasks.rs`, `spatial_resolver.rs`, `world_reservations.rs`, `reservation_transaction.rs`, `task_runtime.rs`, `workforce_matcher.rs`, `cat_capability_authority.rs`, and the single `leader_ai_runtime.rs`/`world_tick.rs` transaction. |
| Leader/officer decision behavior | `leader_content_planner.rs`, `leader_planner.rs`, `officer_expertise.rs`, `officer_requests.rs`, and the canonical runtime. Add report observations, belief inputs, score/dependencies, omission/error behavior, exact site resolution, and one outcome path; never read executor truth. |
| Skills, families, governance, research, divine systems, and trade | Their canonical `*_authority.rs` owner and its immutable catalog leaf. Do not copy an aggregate or create a second ledger in `world_tick`. |
| Public snapshot/action/error | `crates/cat-protocol/src/lai64.rs`, protocol v3 and schema v2. Add a typed report-safe field and deep validation. Routine AI-owned work is not added to `CanonicalGodAction`. |
| Authentication, authorization, rate limit, replay, projection, and persistence | Canonical LAI.65 server boundary plus `cat-server/src/main.rs` and `persistence.rs` integration after the leaf is accepted. The authenticated session, not a submitted player ID, is authoritative. |
| Five-screen UI and world visualization | `leader_ai_ui/lai54`, `lai66`, `lai67`, the LAI.68 renderer/assets owner, and root registration only. Render canonical snapshot IDs/geometry; never reconstruct hidden values or placement. |
| Diagnostics and verification | `leader_ai_diagnostics.rs`, `diagnostics-and-debugging.md`, the owning board row, one focused check after a complete feature, and the final serialized integration/campaign/browser matrix. |

This repository is pre-production. The current cutover deliberately uses one fresh incompatible
database schema and regenerated fixtures. For this overhaul, every older reference below to “old
row defaults,” compatibility aliases, semantic conversion, or an in-place migration means:

1. bump the owned aggregate/schema/protocol version;
2. regenerate the fresh gameplay database and authoritative browser fixture, including the
   fixture identities/accounts, checksum, seed, protocol, and schema metadata; a gameplay reset
   preserves unrelated authentication/identity metadata required by the final reset contract;
3. accept only the exact known current schema;
4. reset a specifically recognized obsolete local development schema when the cutover procedure
   says to do so;
5. fail closed on unknown, future, or malformed schemas;
6. keep restart/replay/atomicity tests for the new schema;
7. add a real migration only after production deployment exists and the policy is explicitly
   changed.

## The extension transaction

Every recipe in this document is one indivisible extension transaction. A change is incomplete
unless its review records all of the following, even when a row is “not applicable” with a reason.

| Concern | Required decision and evidence |
|---|---|
| Stable identity | Declare permanent snake-case IDs for every type, instance, recipe, study, intent, task, report, ledger event, reservation, action, and snapshot field. Never derive identity from a display name or collection index. Reject duplicate IDs at load/catalog validation. |
| Determinism | Store integer/fixed-point values for game rules. Sort candidates by the documented semantic key and then stable ID. Use `BTreeMap`/`BTreeSet` or sort before observing a hash collection. Random choices use a named fork of `cat_sim::rng`, keyed by world seed, colony ID, subject ID, purpose, and decision epoch; adding an unrelated candidate must not advance another subsystem's stream. |
| Authority and visibility | Name the authoritative simulation owner, the report/belief projection, and the player-visible fields. Hidden truth never enters protocol, errors, logs, tooltips, or client components. A client receives only reports and authorized exact values and never reconstructs a site or quantity. |
| Complete spatial contract | For every visible task, specify the complete objective `SiteRef`/footprint, current work position or exclusive slot, pinned delivery endpoint, ordered route, and stage. Use the one canonical building footprint authority; enumerate anchored rectangles row-major. Missing or unrevealed truth produces an explicit blocked state, never a guessed coordinate. |
| Reservations | Reserve source quantity/identity, complete objective or capacity, route conflicts, work slot, cargo, and destination headroom atomically. Exclusive world resources and routes are keyed in the persisted world-scoped ledger, so colonies cannot reserve the same thing. Roll back every acquired claim on a failed transaction. |
| Persistence and compatibility | Under the current fresh-schema policy, add the field/table/aggregate and receipt atomically, regenerate fixtures/checksums, reject unknown/future/malformed schemas, and prove idempotent replay plus save/reload equality. Do not add semantic conversion or compatibility aliases. Protocol-breaking changes increment `PROTOCOL_VERSION` and reject old mutating clients with `UPDATE_REQUIRED`. |
| Failure and rollback | Enumerate pre-commit rejection, mid-stage cancellation, route closure, worker refusal/incapacity/death, source depletion, endpoint loss, restart, and duplicate action. Picked-up cargo is delivered or physically salvaged; consumed atomic work completes once; ledger debits and credits are idempotent. |
| Tests and evidence | Add focused red-before-green tests, order/tick-partition twins, negative authorization/redaction tests, persistence/restart tests, and protocol/UI tests where applicable. Record commands and results on [BOARD.md](BOARD.md); run the gates in [testing-cutover.md](testing-cutover.md). |

Do not solve an extension by matching the same ID in several unrelated roots. Prefer one typed
descriptor/manifest queried by planning, execution, protocol construction, and UI. If a new variant
still requires exhaustive matches, tests must enumerate every consumer and prove the registries
agree.

## Current architecture touchpoint map

| Contract | Current authoritative touchpoints | Post-cutover ownership rule |
|---|---|---|
| Closed game enums and cat/runtime state | `types.rs`, `entities.rs`, `content_manifest.rs`, `skill_catalog.rs`, `cat_capability_authority.rs` | Focused catalogs/authorities own behavior; `types.rs` carries only genuinely shared closed IDs. New content uses manifest IDs unless an exhaustive Rust enum is genuinely required. |
| Buildings, footprints, construction, tasks, matching | `construction_catalog.rs`, `construction_stages.rs`, `spatial_tasks.rs`, `spatial_resolver.rs`, `world_reservations.rs`, `reservation_transaction.rs`, `task_runtime.rs`, `workforce_matcher.rs` | These are the canonical rules. `world_tick.rs` invokes the one ordered LAI.63 transaction and must not duplicate geometry, bills, reservations, matching, or outcomes. |
| Station descriptors and execution | `content_manifest.json`, `station_recipes.rs`, behavior leaves such as `cookhouse.rs`/`fishing.rs`/`material_crafting.rs`, and canonical runtime adapters | Manifest/descriptor data owns inputs, outputs, capability, slot topology, timing, and art; one behavior authority owns each state machine; the runtime only composes them. |
| Resources, inventory, Hole and barter | `quality_lots.rs`, `physical_storage.rs`, `black_hole.rs`, `material_crafting.rs`, `moneyless_barter.rs` | Resource/item, Hole/Void, storage, diplomacy, and physical-barter leaves own truth and ledger transitions. Shrine, Favor, generic Food, coin, and settlement-price authorities are forbidden. |
| Planner, reports, officers, research | `leader_content_planner.rs`, `leader_planner.rs`, `beliefs.rs`, `officer_expertise.rs`, `officer_requests.rs`, `research_manifest.rs`, `progression_research.rs`, `research_authority.rs` | These leaves own report-limited decisions and two research lanes; the legacy director, Shrine/Favor purchases, and direct God officer controls are deleted at cutover. |
| Wire | `crates/cat-protocol/src/lai64.rs` with export/version only in `lib.rs` | One canonical v3/schema-v2 family owns strict DTOs, redaction shape, exact version lanes, real stable IDs, and the God-only action union. |
| Actions and projection | canonical LAI.65 server leaf, `crates/cat-server/src/main.rs`, `identity.rs`, and `rate_limit.rs` | Simulation validates domain rules; server authenticates, authorizes, orders, rate-limits, deduplicates, persists, and emits the only socket projection. |
| Save/load | `crates/cat-server/src/persistence.rs` | One persistence owner creates/loads/saves the fresh incompatible schema and regenerated fixtures atomically; no semantic conversion code remains. |
| Bevy | `crates/cat-client/src/lib.rs`, `leader_ai_ui/lai54`, `lai66`, `lai67`, and the LAI.68 renderer/assets owner | Focused five-screen and world-renderer slices own panels/markers; `lib.rs` only registers them. The deleted legacy `research_ui.rs` is not restored. |

## Decision tree: data-only addition or new behavior

Start here before choosing a recipe. “Data-only” does not mean “manifest-only”; it means the
existing authority, state machine, spatial contract, wire projection, persistence rows, UI
renderer, and art resolver already express the new definition without a new branch.

1. Can the canonical manifest and an existing immutable descriptor represent every rule using
   existing typed fields, units, stages, slots, permissions, failure reasons, art states, and
   report visibility?
   - **Yes:** continue to step 2.
   - **No:** this is a behavior extension. Name the new or extended authority before editing.
2. Does the addition use an existing objective/site kind with exactly the same footprint,
   work-slot topology, endpoint roles, route requirements, reservation modes, cargo lifecycle, and
   recovery behavior?
   - **Yes:** continue to step 3.
   - **No:** extend the spatial/task authority and use Recipe 4. Never squeeze a new site into a
     generic marker or center-point fallback.
3. Does it use an existing quality/lot/item/storage representation without a new identity,
   compatibility, durability, spoilage, capacity, or conservation rule?
   - **Yes:** continue to step 4.
   - **No:** extend the relevant quality, item, fixture, container, food, or storage authority.
4. Does it unlock through an existing capability/research rule and fit an existing
   Leader/officer domain, observation/report subject, score operator, omission policy, and command?
   - **Yes:** continue to step 5.
   - **No:** extend research and/or the report-driven planner; do not hard-code an exception in
     `world_tick`.
5. Can the canonical snapshot describe it with existing stable-ID catalog entries and fields, and
   can the client resolve its exact art and accessible label without interpreting the ID?
   - **Yes:** the simulation portion may be data-only. Add the manifest row, descriptor, assets,
     localization, persistence/catalog-version evidence, and all exhaustive-registry tests.
   - **No:** add the smallest typed wire/UI/art extension through Recipes 13, 16, and 27.
6. Does any step require a new God mutation, externally visible aggregate, database table/version,
   deterministic RNG decision, or diagnostic reason?
   - **Yes:** it is not a data-only extension; use the relevant behavior recipe and record the
     authority/version/evidence decisions.

Even a data-only entry still requires permanent IDs, duplicate/ordering validation, capability and
research classification, art/accessibility coverage, fresh-fixture regeneration where persisted,
and a board receipt. A behavior extension must have one authoritative owner and one cutover; it
must not land as a manifest row plus unrelated `match` arms in several roots.

## Recipe 1: add a Workshop or another building

1. Choose unused permanent content, building, blueprint, recipe, capability, slot, art, and
   localization IDs. Add the building variant to the one shared `BuildingType` only when exhaustive
   Rust dispatch is actually required; add content/station/recipe/capability/art definitions to the
   canonical manifest and behavior catalogs. Protocol uses stable IDs and typed snapshot state,
   not a second building enum. Keep every `ALL`/manifest order deterministic.
2. Add exactly one complete footprint entry through `spatial_tasks::footprint_for`. Add the
   corresponding immutable level 1–10 `ConstructionBlueprint` profiles in
   `construction_catalog.rs`: permit, duration, Logs versus Lumber/Planks, scaffold/structure/
   fit-out bills, exact 20/60/20 phases, and three phase art keys. A Workshop-like station is exactly
   3×3/nine tiles. Placement, occupancy, construction, visible tasks, protocol, and renderer must
   consume that one footprint; no client or feature-local width/height constant is allowed.
   Every new bill content ID also needs one sorted `construction_miracle_inputs` classification;
   follow [construction-miracle-value-authority.md](construction-miracle-value-authority.md) and
   never give an exact item or fixture bulk-lot generation semantics.
3. Define station/recipe descriptors, capabilities/research, officer domain, work-slot topology,
   linked non-overlapping input storage, output capacity, and Leader-planner topic in their
   canonical catalogs/leaves. Search `content_manifest.json`, `construction_catalog.rs`,
   `station_recipes.rs`, the owning behavior authority, `storage_authority.rs`,
   `leader_content_planner.rs`, and the LAI.63 runtime adapter. Update or explicitly disposition
   every exhaustive match; do not put a routine production/placement action in
   `CanonicalGodAction`.
4. Authority is the simulation building instance keyed by `(colony_id, building_id)`; protocol shows
   its authorized descriptor and report-safe state. The construction objective is the complete
   row-major canonical footprint, work is at reserved reachable perimeter/interior slots, and input
   and output endpoints are pinned station compartments or compatible stockpiles. The world ledger
   rejects overlap with any colony's cross-colony exclusive site/route claim.
5. Persist the stable type/instance, blueprint version, project stages/cargo, queue, slots, storage
   link, cycle receipts, and reservations in the fresh canonical schema; regenerate fixtures and
   schema checksum. Unknown/future/malformed state fails closed. Invalid type/footprint/slot state
   blocks or quarantines rather than shrinking to one tile. Before commit, release reservations and
   salvage cargo; after commit, cancellation preserves the scaffold or follows an explicit
   demolition transaction.
6. Tests cover manifest/wire round-trip, duplicate ID rejection, blueprint completeness for levels
   1–10, canonical footprint cardinality/order,
   overlap and route reachability, construction restart at each stage, station defaults, research
   unlock, projection, Bevy selection/rendering, and order/tick-partition twins. Run sim, protocol,
   server persistence/authorization, client rendering, smoke, Clippy, format, and whitespace gates.

## Recipe 2: change a footprint

1. Change only the canonical `footprint_for(BuildingType)` descriptor (or its LAI.3 successor), not
   a client or task constant. Keep width/height positive integers; the tile list is the north-west
   anchor's half-open rectangle in row-major `(y, x)` order with stable tile IDs. Footprint
   derivation and migration use no RNG; collision ties sort by `(colony_id, building_id)`.
2. Treat a footprint change as a save-schema change. Increment a spatial rules version, migrate all
   affected structures and active construction/tasks transactionally, and reject the migration if
   any expanded cell overlaps an authoritative structure, world-exclusive reservation, or invalid
   terrain. There is no one-tile or client-side fallback default: an old row derives the previous
   type footprint until the migration commits. Never silently move, rotate, truncate, or
   grandfather an illegal structure.
3. Rebuild objective footprints and world-scoped reservations from the canonical descriptor while
   preserving instance/task IDs. Work slots retain stable semantic slot IDs where still valid; a
   removed slot blocks and rematches its task after releasing the slot, never strands a busy cat.
   Picked cargo remains attached to the task and pinned endpoint.
4. Simulation owns the new footprint. Protocol exposes the complete authorized footprint; the
   server redacts unrevealed cells/objectives, and Bevy renders the snapshot cells rather than
   recomputing size. Rollback leaves the prior database and rules version intact; downgrade remains
   unsupported.
5. Touch `world_tick.rs`/the spatial leaf, `persistence.rs`, snapshot construction in
   `cat-sim/src/actions.rs`, protocol DTO tests in `cat-protocol/src/lib.rs`, and Bevy footprint/layout
   tests in `cat-client/src/lib.rs`/`station_layout.rs`. Test old/new fixtures, collisions, active
   tasks, cross-colony conflicts, nine/other-cell rendering, restart, failed migration rollback,
   deterministic ordering, and all quality gates.

## Recipe 3: add station/work slots and recipes

1. Allocate stable `recipe_id` and semantic `slot_id` strings. Add one
   `StationRecipeDescriptor`/`StationRecipeSet` entry in `cat-sim/src/station_recipes.rs`; planning,
   entitlements, queue validation, station-local domains, execution, and UI must query this registry.
   Keep descriptor iteration in stable recipe-ID order and slot iteration in stable slot-ID order.
2. State exact integer/fixed-point inputs, outputs or finite item identity, effective work duration,
   required study, skills/tools, station input/output capacity, and whether the recipe is available
   at founding. If a probabilistic output is essential, use a named recipe RNG keyed by station,
   slot, recipe, and cycle ID; otherwise use no RNG.
3. The complete building footprint is the objective; a cat reserves one exact reachable work slot;
   each input source, station compartment, cargo leg, output compartment, and final stockpile is
   pinned. Slot capacity and all world-exclusive route/source claims are committed atomically across
   colonies. Hidden stock and regeneration stay behind belief/report projection.
4. Persist queues, per-cat slots, cycle IDs, local inputs, outputs, and consumed-step marker. A
   rules-version migration adds new default slots/recipes once without reordering player queues.
   Invalid/locked recipes pause with a typed reason. Cancellation before consumption returns cargo;
   after atomic consumption it produces output once, including across restart/refusal/death.
5. Touch `station_recipes.rs`, the owning executor (`production.rs`, `processing.rs`, `smithy.rs`, or
   its successor), current `world_tick.rs` integration/default queue, `research_catalog.rs` and its
   JSON manifest when gated, protocol building/action types, persistence, server action validation,
   and `client/lib.rs` station controls/layout. Tests prove registry uniqueness/completeness,
   entitlement, capacity, exact conservation, queue edit idempotency, multi-worker slots,
   restart/cancel/refusal, wire/UI order, redaction, and the full quality gate.

## Recipe 4: add a visible task/site mapping

1. Allocate stable goal/intent/task category and stage IDs; add the task variant to the spatial
   descriptor/mapping table and, only when still needed for compatibility, `TaskType`/`JobKind` in
   `cat-sim/src/types.rs` and `cat-protocol/src/lib.rs`. Sort candidates by policy score, semantic
   site key, then stable site ID; any random omission/site choice uses its own named keyed stream.
2. Specify all fields: authoritative objective identity and complete footprint; reachable work tile
   or exclusive slot for every stage; pinned delivery endpoint and capacity; ordered route; cargo;
   discovery/report requirements; and reservation modes. Hunt means a real revealed hunting source,
   Fetch Water means actual water plus a separate dry bank, and a Workshop task means its canonical
   full 3 x 3/nine-tile footprint.
3. Simulation/spatial leaves own resolution; planners receive beliefs and typed block reasons, not
   hidden coordinates. Protocol carries the report-safe `SiteRef` and stages. Bevy draws only those
   fields. World-exclusive source/site/route claims conflict across colonies; shareable objectives
   still require distinct exclusive slots and capacity reservations.
4. Persist the objective, work position, endpoint, route, stage, reservation keys, and cargo. Legacy
   missing/malformed metadata defaults and migrates to `Blocked` without assigning a worker or
   destination; bump the spatial-task rules version for the new mapping. Revalidation failure
   releases unpicked reservations and rematches; picked cargo is delivered or salvaged. Duplicate
   completion and restart cannot repeat effects.
5. Touch spatial/task/reservation leaves (current `tasks.rs`, `movement.rs`, `pathfinding.rs`,
   `village_sites.rs`, `world_tick.rs`), snapshot/action types, persistence, server projection, and
   the LAI.29 client marker slice/current `client/lib.rs`. Add the named spatial tests in
   [testing-cutover.md](testing-cutover.md), every-variant round-trips, cross-colony conflicts,
   blocked/no-busy, route closure, restart, marker dedupe/despawn/redaction, and quality gates.

## Recipe 5: add a resource, source, or Hole-feed class

1. Give the resource, canonical source type, carrying kind, ledger account, and Hole-feed class
   stable IDs. Extend the current shared definitions in `cat-sim/src/stockpiles.rs`, `entities.rs`,
   `items.rs` when finite, and corresponding protocol enums. Append stable `ALL` registries or sort
   by ID; conversions must be total and duplicate-free.
2. Define integer/fixed-point units, capacity, depletion/regeneration truth, discovery observation,
   belief bounds/expiry, compatible storage, physical cargo, spoilage, production uses, valuation,
   and conservation. Source selection uses revealed report-safe candidates ordered by route cost
   then source ID; ecology randomness uses its named source RNG, never planner RNG.
3. The source's exact identity/footprint, work bank/slot, and destination are distinct. Reserve exact
   source quantity, cargo, route, and endpoint headroom in the world ledger before dispatch. Hidden
   quantity/regeneration never crosses the report boundary. Exact player currency is limited to the
   owning player's Research Notes and Void Insight balances.
4. For a Hole feed, add the canonical content gate, Darkness gate, base value, physical stage, and
   quality/processing/augmentation/condition value inputs to the manifest-owned rule. Demand remains
   endlessly eligible, but Width intake, Depth capacity, forty-game-minute openings, one active
   feed, scarcity mistakes, and report-limited Leader choice remain exact. Physical delivery is
   consumed once before one idempotent micro-Void credit; cancellation, death, refusal, and route
   loss recover the original identity rather than converting it to currency.
5. Persist balances/source state/cargo/ledger markers and version defaults in `persistence.rs`.
   Unknown required resource data quarantines; old saves default to zero/no source and migrate once.
   On failure, return or salvage physical cargo, release capacity, and never debit/credit twice.
6. Touch content/quality/storage/source leaves, `black_hole.rs`, station/research/barter consumers,
   protocol, persistence, report projection, and Stores/Hole UI. Tests cover catalog uniqueness,
   conservation, depletion/report redaction, source/site reachability, cross-colony contention,
   every feed/recovery stage, endless bounded operation, restart, and quality/value gates.

## Recipe 6: add a planner goal, operator, or intent

1. Assign permanent goal/operator/intent and evidence IDs. Register preconditions, effects,
   dependencies, authority domain, posture eligibility, score inputs in basis points, retry class,
   expiry, and maximum live/history counts in the planner leaf. Order by posture/score, creation
   tick, and stable intent ID; omission or bounded choice uses a named keyed planner RNG stream.
2. Operator inputs are only persisted beliefs/reports and explicit policy, never authoritative
   hidden resources or regeneration. Explanations list report provenance/confidence/bounds. The
   objective/work/delivery/route tuple is unresolved until the spatial resolver returns the full
   contract; resolution failure produces a typed block with no worker or marker.
3. Commit the intent plus source/site/route/slot/cargo/endpoint reservations atomically in the
   world-scoped ledger. Equivalent intents deduplicate by canonical semantic key across officers;
   dependency cycles fail. Cross-colony claims participate in the same conflict order.
4. Persist lifecycle, score inputs, evidence refs, retry count/deadline, reservations, and adoption
   provenance with a planner-state version. Migration defaults to no new intent. Cancellation/
   succession releases claims unless a validated successor adopts the unchanged intent; the exact
   retry sequence and terminal failure remain those in the planner design.
5. Touch the planner/intent/scheduler leaves created by LAI.2/LAI.10/LAI.11/LAI.15, current
   `leader_ai.rs` only for compatibility, LAI.23 `world_tick` phase registration, protocol plan/task
   DTOs, persistence, and Plans UI. Test hidden-truth twins, fixed-point/order twins, dedupe/cycles,
   reservation rollback, no-site blocking, retry/succession/restart, bounded queues, explanation
   redaction, and all gates.

## Recipe 7: add an officer domain, report, or belief

1. Add a stable officer/domain/subject/report/belief ID to the knowledge and officer registries; if
   it is a new office, extend both `OfficerRole` enums, `OfficerRole::ALL`, prerequisite mapping in
   `cat-sim/src/officers.rs`, authority mapping, and client labels. Use the established role order
   followed by stable subject ID; appointment ties use the isolated appointment RNG.
2. Define observation source, authority level, confidence/bounds, expiry class, contradiction key,
   report recipients, request types/budget, and whether the field is public, owning-player-only, or
   never projected. After expiry confidence decays by exactly 500 basis points per full
   subject-specific expiry interval to floor zero; direct invalidation sets it to zero.
3. Reports may refer to complete `SiteRef`s only when discovery permits. An officer never reserves
   hidden truth directly: its request goes through intent/spatial resolution and the world ledger,
   including cross-colony conflicts, complete objective, work slot, route, cargo, and endpoint.
4. Persist observations, belief provenance/version/confidence/expiry, reports, officer expertise,
   and outstanding requests. Old saves default to absent belief/office; migrations never synthesize
   knowledge. Vacancy, death, stale report, contradiction, or lost authority is an explicit failure
   that expires/reassigns work through typed states without leaking truth or discarding carried
   cargo.
5. Touch knowledge/officer/request leaves, current `officers.rs`, compatibility mappings in
   `leader_director.rs` until deletion, protocol officer/report types, persistence, server projection
   and authorization, and client officer/Plans panels. Test all-level error/expiry bands,
   contradiction precedence, hidden-truth twins, 3/5/8/12/all appointments, request aging/expiry,
   vacancy/succession/restart, complete sites, redaction, and gates.

## Recipe 8: add a cat attribute, personality axis, or acquired trait

1. Allocate a stable field/axis/trait ID and append it to the canonical cat-model registry. Touch
   current `cat-sim/src/entities.rs`, `genetics.rs`, `skills.rs` and the LAI.4/LAI.5 cat leaves;
   update `CatSnapshot` in protocol only for authorized visible/report fields.
2. Specify integer range/default, legacy conversion, inheritance/mutation if innate, deterministic
   population distribution if personality, exact fixed-point weight effects, acquisition/removal
   conditions if a trait, and every planner/matcher/care consumer. Tie by cat ID; mutation or trait
   rolls use a named cat/subject/event RNG so unrelated fields do not perturb outcomes.
3. Simulation owns exact cat state. Public/foreign projections expose only authorized summaries;
   server errors and UI never infer hidden aptitude or diagnoses. Any task effect still uses its
   complete objective/work/delivery contract and world reservations; the cat field cannot bypass
   eligibility, consent, cross-colony ownership, a route, or a slot.
4. Persist with an explicit serde/SQL default and cat-model rules version. Migration converts every
   old row deterministically once; malformed required values roll back/quarantine. Refusal or loss
   of eligibility releases unpicked reservations, preserves cargo, and completes an already-consumed
   atomic step once.
5. Update matcher/planner/care leaves and exact client inspector controls in `client/lib.rs`.
   Tests cover range/default/conversion, inheritance/distribution seed matrices, axis isolation,
   acquisition/removal, matching order, refusal/cargo, snapshot redaction, restart, and gates.

## Recipe 9: add an injury, body part, or prosthetic

1. Allocate stable anatomy part, side, injury, treatment, prosthetic item, and incident IDs. Extend
   the LAI.6/LAI.7 anatomy/care leaves and current finite-item types in `items.rs`; use semantic part
   order then side then stable ID. Incident rolls use the dedicated injury RNG keyed by cat, task,
   atomic work incident, and tick boundary.
2. Define severity, functional aggregation, eligibility exclusions, exact integer probability,
   treatment work-hours, medicine/tool requirements, consent, fitter skill, restoration basis
   points, durability in affected work-hours, adaptation, breakage, repair, death disposition, and
   report/redaction rules. Missing parts never regrow.
3. Treatment/fitting/repair needs a complete reachable care or canonical Workshop objective, exact
   work slot, prosthetic item identity, route/cargo, and endpoint. Reserve item/source/site/slot and
   destination atomically in the world ledger; cross-colony/foreign items or patients fail
   authorization.
4. Persist anatomy, injury/treatment progress, incident ID, item location, fit, wear, adaptation,
   and consumed-step marker under an anatomy/prosthetic rules version. Its one-time migration makes
   legacy cats intact/uninjured by default; no prosthetic is minted. Refusal/cancel/break/death/
   trade/restart moves the one item ID to one authoritative location and releases claims without
   regrowth or duplication.
5. Touch care/prosthetic/item leaves, matcher/task/spatial integration, protocol cat/item/actions,
   persistence, server consent/ownership/redaction, and client care/marker UI. Test probability and
   tick-partition matrices, paw/eye/tail aggregation, 12/48-hour treatment, 50/75/90% and 360/1080
   constants, one-ID conservation through every failure path, complete sites, restart, and gates.

## Recipe 10: add a research study or track

1. Assign stable study/track/payload/effect IDs. Add data to the canonical manifest owned by
   `research_manifest.rs` and the validated content/capability metadata it consumes. Its finite
   total is derived, never hard-coded in a planner, protocol, test, or UI; historical 531/556 totals
   are not current authority. Never add a second runtime catalog. Validate unique IDs, references,
   acyclic reachability, category counts, deterministic topology `(era, priority, layout,
   study_id)`, and live non-inert payloads.
2. State the exact integer Research Notes or Void cost, physical preparation work, prerequisites,
   purchaser authority, Leader-lane eligibility, repeatable-track behavior, effect, and scope.
   Ordinary studies consume Notes; the thirty Hole-axis studies and four player-only Divine Boosts
   consume Void. Selection and tie-break use belief-based utility then stable study ID; any choice
   randomness is a research-specific keyed stream. Committed price/duration never changes after
   later research.
3. Scholar work uses the full Research Hut/School objective, exclusive scholar slot, and pinned
   colony research endpoint; physical prerequisites use source/route/cargo reservations. Hidden
   research candidates or resource truth are not projected; the owning God sees exact Notes/Void.
   World-scoped sources/routes prevent a scholar job in one colony from stealing another colony's
   exclusive claim.
4. Persist manifest version, completion ledger, God queue/front, frozen cost/duration, preparation,
   Notes/Void balances, Leader cadence/decision reason, quotas/windows, repeatable levels, permits,
   and idempotency event IDs. The pre-production cutover uses a fresh schema rather than semantic
   currency conversion. Removing a queued node cascades descendants, refunds funded currency, and
   loses only elapsed labor; a Leader overtake refunds currency but cannot mint labor or preparation.
5. Touch the canonical progression/research and scholar leaves, content/research manifests, station
   unlock registry, protocol, persistence, server authorization, and the new full-screen Research
   slice under `leader_ai_ui/`. Tests enforce derived unique reachable studies, fourteen finite
   tracks plus repeatable level 11+, at least twenty-four AND junctions and the curated convergence
   junctions, deterministic graph/order, live payloads, two-lane collision avoidance and documented
   urgent/oopsie duplication, queue/refund/preparation, restart/offline catch-up, redaction, UI
   topology, and absence of Favor/Blessing/generic-research authorities.

## Recipe 11: add a divine boost

1. Allocate stable boost/effect/purchase/event IDs and register exact integer duration choices,
   base Void rate, fixed-point multiplier, eligibility, and stacking group in the divine-boost
   leaf. Order active boosts by expiry then boost ID; boosts are deterministic and use no RNG.
2. Simulation/Void ledger owns debit and effect. Protocol shows exact cost and the owning player's
   active authorized state, not hidden productivity inputs. A boost cannot reveal a task source and
   cannot bypass consent, eligibility, complete objective, work position/slot, delivery endpoint,
   or world-scoped cross-colony reservations.
3. Persist committed price, start, expiry, scope, purchase action ID, and ledger event ID under a
   boost-state version. Its migration defaults old saves to inactive. Purchase is one transaction:
   validate, calculate documented ceil cost, debit, install; any failure rolls all three back. Same-
   type active purchase rejects without debit; different permitted types overlap; expiry is exact
   through batched ticks/restart.
4. Touch boost/Void leaves, planner/matcher/task effect consumers, protocol action/snapshot/error,
   persistence, server auth/idempotency/redaction, and progression UI in the LAI.31 slice/current
   `client/lib.rs`. Test every duration/economy tier, +50% effect, ceil costs, overlap/rejection,
   duplicate/stale action, expiry partitions/restart, no site bypass, projection, and gates.

## Recipe 12: add diplomacy or trade behavior

1. Allocate stable personal stance, proposal, contract, actor, escrow, route, cargo, and ledger-event
   IDs. Extend `diplomacy.rs`, `moneyless_barter.rs`, and their runtime adapters; order due work by
   next event tick, contract ID, then actor ID. Any proposal variety uses a diplomacy/barter-specific
   keyed RNG and report inputs only. Do not add coins, purses, prices, settlement, debt, or a money
   compatibility field.
2. Define authenticated personal ownership plus exact `Alliance`, `Neutral`, and `Enemy` semantics.
   Alliance currently behaves as Neutral and must be labelled honestly; it promises no defense or
   migration. Enemy excludes the village from outbound candidates and makes the destination reject
   before dispatch, so no caravan or escrow is created. Global village stance remains Neutral.
   Define belief-based material valuation, exact cargo, escrow headroom, physical route/stages,
   deadlines, cancellation/stranding/recovery, and visible versus private contract fields.
3. Reserve both sides' source identities/quantities, destination capacities, routes, actors/slots,
   and finite item IDs atomically in the world ledger. Complete pickup/delivery endpoints and route
   are pinned as the contract's spatial objective and work stages; cross-colony contention follows
   the same stable conflict order. No party reads the other's hidden stock.
4. Persist relationship epochs, consent, contract stage, escrow, cargo location, route, next event,
   and idempotency markers under a diplomacy/trade rules version. Its migration defaults old saves
   to Neutral/no contracts. Failure before pickup releases escrow; after pickup cargo returns or
   becomes physically stranded. Restart and duplicate actions never duplicate/delete cargo or
   ledger effects.
5. Touch diplomacy/barter/reservation leaves, route runtime, protocol actions/snapshots, persistence,
   server ownership/redaction, and Village/Council UI. Test personal/global stance separation,
   honest Alliance behavior, Enemy pre-dispatch rejection, the complete possible-now versus
   better-trade score (need, offered utility/quality/value, distance, time, risk, carrying and
   opportunity cost), hidden-truth twins, stable ordering, cross-colony escrow/route conflicts,
   all cancellation/restart stages, physical conservation, money-identifier absence,
   authorization/redaction, and gates.

## Recipe 13: add a protocol snapshot field or action

1. Assign stable camel-case wire field or lower-camel action discriminator and typed error IDs in
   `crates/cat-protocol/src/lib.rs`. Preserve enum wire literals permanently. Additive snapshot
   fields require a documented serde default; incompatible shape/meaning increments
   `PROTOCOL_VERSION` and has no silent coercion. Wire encoding/decoding uses no RNG; collection
   order is the authoritative semantic order followed by stable ID.
2. Snapshot data originates in simulation/report projection (`cat-sim/src/actions.rs` and owning
   leaves), then passes server `project_snapshot`; it never originates in the client. Explicitly
   classify each field as public, owning-player exact, report-bounded, or server-only. Server-only
   data must not exist in the wire DTO.
3. Spatial fields carry complete `SiteRef`, row-major footprint, work position/slot, endpoint,
   route, stage, and only authorized reservation summary. Cross-colony private IDs and hidden source
   truth are redacted before serialization, including errors and logs.
4. Actions carry protocol version, authenticated identity/signature, selected colony, expected
   aggregate/entity version, client action ID, and typed payload. Server order is protocol,
   authentication, ownership, authority, expected version, idempotency, then simulation validation.
   Duplicate actions return the stored result; stale versions refresh without mutation.
5. Persist any new mutation and dedupe result in the same SQLite transaction; defaults/migration
   and rollback are defined by Recipes 14–15. Touch protocol root/tests, sim action/snapshot builder,
   server router/projection, persistence, and client decode/action sender/UI. Test JSON golden/round-
   trip/defaults, old-client `UPDATE_REQUIRED`, every enum/site variant, malformed/unauthorized/
   stale/duplicate actions, exhaustive leak scans, restart, and gates.

## Recipe 14: add persistence state or a schema migration

1. Choose a stable table/column/JSON field and monotonic fresh schema/subsystem rules version.
   Change `crates/cat-server/src/persistence.rs` in all paths together: current `CREATE TABLE`, save
   binding, strict load/parse, known-obsolete-schema reset detection, and future/malformed rejection.
   Do not reuse a retired field or add a semantic compatibility alias.
2. Preserve stable IDs and canonical ordering in serialized collections. Persist complete task
   objective/work position/delivery endpoint/route/cargo, world-scoped cross-colony reservations,
   ledger/dedupe markers, belief/report provenance, and rules versions whenever the extension owns
   them. Schema creation/reset uses no RNG beyond the documented deterministic fixture seed;
   conflict ties use stable persisted IDs.
3. This pre-production overhaul does not translate Shrine/Favor/generic-food/coin/legacy-research
   gameplay state. A recognized obsolete gameplay schema takes the signed two-step test/reset path
   or whole-application database recreation required by the deployment contract; unknown, future,
   or malformed state fails closed. Preserve only unrelated authentication metadata explicitly
   allowed by the reset contract. Regenerate fixtures, accounts/checksums, seed, protocol, and schema
   metadata together, transactionally and idempotently. Downgrade is unsupported.
4. Authority stays in sim state loaded from the database; migration must not invent reports from
   hidden truth or mark unrevealed sites known. Reconstructed reservations use full canonical
   footprints and stable conflict order across colonies. Collision blocks/quarantines rather than
   picking a winner by SQL row order.
5. Test fresh schema, recognized-obsolete reset, reset authentication and production rejection,
   fixture/checksum regeneration, malformed rollback, replay idempotency, newer-version rejection,
   save/load equality, active-stage restart, collision, and ledger/item conservation. Scan schema
   and fixtures for every forbidden legacy identifier. Run server focused tests plus sim/protocol
   consumers, smoke, Clippy, format, whitespace, and the signed restart journey where applicable.

## Recipe 15: add server authorization or redaction

1. Define a stable capability/authority/error ID and a complete actor-resource-action matrix. Touch
   `crates/cat-server/src/identity.rs`, `main.rs`, `rate_limit.rs` if rate policy changes, and the
   protocol capability/action/error DTO. Keep authentication and authorization independent of
   display name.
2. Enforce protocol, authentication, selected-colony ownership/control, domain authority/consent,
   expected version, idempotency, then simulation rules in that order. Order concurrent accepted
   actions by server sequence/action ID; authentication and redaction use no RNG.
3. Extend the single `main.rs::project_snapshot` path (or its post-cutover focused projection leaf)
   for every socket emission. Name public/owner/report/server-only fields. Strip hidden truth before
   JSON creation; bounded errors/logs never reveal quantity, regeneration, unseen source/site,
   threat, private colony ID, or reservation loser.
4. Authorization sees complete objective/work/delivery and world-scoped reservations only when the
   actor may act on that colony, including the exact work position/slot; a client-provided
   coordinate never substitutes for authoritative resolution. Persist accepted mutation plus
   action-result dedupe atomically; rejection mutates nothing. A new persisted capability defaults
   to deny; its one-time rules-version migration is transactional and idempotent, and a newer
   unsupported version fails closed. Restart retains dedupe, rollback returns the original typed
   failure, and downgrade is unsupported.
5. Test anonymous/owner/non-owner/shared-hub/officer/god cases, cross-colony isolation, every socket
   emission, snapshot/error/log/tooltip leak scans, malformed signature, replay/stale/concurrent
   actions, restart, `UPDATE_REQUIRED`, and gates. Never log signatures or raw identities in test
   artifacts.

## Recipe 16: add Bevy UI or world markers

1. Assign stable component/system/marker kind and accessibility label IDs. Put feature behavior in
   the relevant focused client slice; current roots are `crates/cat-client/src/lib.rs`,
   `station_layout.rs`, and `leader_ai_ui/`. `lib.rs` registers systems/resources/assets and must
   not become a second simulation registry.
2. Render only protocol snapshots and capability flags. Preserve server-provided stable order or
   sort by semantic priority then stable ID. Cosmetic animation may use Bevy time; it cannot affect
   actions or simulation and must not use simulation RNG. User actions carry exact IDs and expected
   versions, never display strings or inferred coordinates.
3. Markers render the complete authorized objective footprint, work position/slot, endpoint, and
   ordered route from the snapshot. Deduplicate by `(task_id, marker_role, cell_or_segment_id)`,
   update in place, and despawn absent/stale markers. Objective-less or unrevealed blocked tasks
   spawn zero world entities. A 3 x 3 Workshop-like site renders exactly nine cells.
4. Client state is disposable and not authoritative. Rejection/stale version restores controls from
   the refreshed snapshot; pending state has a timeout/failure presentation and never fabricates
   success. Cross-colony selection clears private/stale markers. Protocol/persistence defaults are
   handled upstream; the UI tolerates only documented additive defaults and shows update-required
   for incompatible versions.
5. Touch protocol DTO/action sender as needed, focused UI/marker module, `client/lib.rs` registration,
   `station_layout.rs` for station art/layout, and asset mappings. Test own-framebuffer native sizes,
   WASM where affected, marker cardinality/dedupe/despawn/redaction, selection changes, stale/reject/
   reconnect, keyboard/mouse controls, stable order, no hidden reconstruction, and client plus
   workspace quality gates.

## Recipe 17: add a skill, XP source, affinity, or refusal

1. Allocate a stable skill/activity/XP-source ID in the one data-owned capability catalog. Declare
   its primary and optional secondary skill, exact successful-work award, supervised award, haul-leg
   award, governing attributes, equipment/anatomy prerequisites, profession/domain grouping, and
   valid Loved/Preferred/Neutral/Disliked/Refused affinity states. Do not add a second skill enum or
   infer XP from a display label.
2. Grant XP only from a declared completed physical activity receipt: primary `1`, secondary `25%`,
   supervised `10%`, or the explicitly smaller haul award unless the descriptor says otherwise.
   Blocked, waiting, invalid, cancelled, and failed work grants zero. Level is
   `min(100, floor(sqrt(xp)))`; XP beyond 10,000 changes Mastery/legacy/teaching/reputation only and
   never increases the level-100 work effect.
3. Add the skill to the lexicographic matcher after urgency and personal priority. Emergency,
   Leader priorities 1–5, and Background choose the tier; Family Enterprise, Loved, Preferred,
   Neutral, Disliked then skill, attributes, continuity, route length, and stable IDs break ties.
   Refused is ineligible even in emergency. Personal flee/eat/drink remains bodily autonomy, not a
   forced work assignment.
4. Separate inherited attributes, learned skills, acquired traits, and office report clearance.
   Supervised professional work may teach a skill, but only completed office duty grants expertise
   or hidden-report capability. Persist source receipts and office duty independently so children,
   successors, restart, or cross-training cannot inherit security clearance accidentally.
5. Project only authorized skill/affinity/anatomy/match rationale. Add Cat/Council detail labels and
   textual blockers without hidden candidate truth. Tests cover every declared activity, exact
   awards, zero-XP failures, level/Mastery boundaries, shuffled ordering, Refused/anatomy exclusion,
   office-clearance separation, restart, strict catalog decoding, and no phantom task/log from
   ambient learning.

## Recipe 18: add family tradition, enterprise, housing, or mentorship

1. Give lineage, household, partnership, tradition, surname branch, enterprise, residence, teaching
   obligation, mentor relation, and social-event types stable IDs. Store both parental lineages and
   the deterministic family-seed key. Never derive identity or ownership from a localized surname.
2. Declare the exact occupational seed weights and transfer rules, inherited attribute/personality
   scope, XP cap, tradition learning bonus, apprenticeship bonus, maturity requirements, work-unit
   counters, station continuity, and bounded teaching cadence. Acquired traits and report clearance
   do not inherit. An enterprise influences work preference, mentoring, history, and signage but
   owns no colony item, stockpile, building, route, or currency.
3. For partnerships, score only non-kin eligible cats through declared report-safe attributes,
   skills, personality, family axes, traditions, and housing. Close kin are rejected and Gods cannot
   arrange a match. For housing, define exact adult/dependent/elder capacity, unlocks, move priority,
   empty-nest behavior, hazards, work effects, and bounded longevity; no upgrade grants immortality.
4. A parent teaching obligation is created after every three completed real tasks, persists through
   restart, and may be deferred—but not erased—by emergency work. Assigned mentoring runs before
   invisible ambient cleaning. Every visible Teach task uses the actual Home, Nursery, School,
   office, or enterprise footprint, reserved teacher/learner slots, route, and duration.
5. Persist lineage, household, partnership, residence, tradition, surname/branch, enterprise site,
   work counters, mentor/obligation, XP receipts, and move hazards. Project family trees, pressure,
   capacity, benefit, mentor, and work history without hidden compatibility scores. Tests cover kin
   rejection, deterministic pair/seed twins, maturity and surname rules, no enterprise ownership,
   capacity/moves/longevity, teaching cadence/defer/restart, exact-site markers, and conservation.

### Family institution and enterprise-sign extension checklist

Use this checklist when adding a Family Home/Elder Lodge-style institution or a visible family
enterprise identity. These are family institutions, not generic private property and not a second
production-station authority.

1. Add one stable building/content/blueprint/art/localization identity through Recipe 1. Declare the
   canonical full footprint, level bills, scaffold/structure/fit-out timing, permit/unlock, and
   operational state. Register the completed building as a `family_housing` institution; never
   infer housing kind from a localized name or sprite.
2. Declare exact occupants and eligibility. The built-in contracts are Family Home = two partnered
   adults plus up to four dependent Kitten/Young cats, Elder Lodge = eight elder beds, and Nursery
   = childcare/early teaching with zero permanent beds. State move priority, empty-nest fallback,
   dependent/guardian rules, elder eligibility, hazards, social recovery, mentoring effect, and
   bounded longevity. An upgrade may reduce old-age risk but cannot grant immortality.
3. Keep allocation in `FamilyAuthorityState` and the canonical housing leaf. A building instance
   supplies capacity only after construction reaches operational; cancellation, destruction,
   route loss, death, partnership change, or expulsion must trigger a persisted reallocation plan
   rather than silently deleting a residence. Protocol and Bevy consume authorized residence and
   pressure reports, not a client-side capacity calculation.
4. Teaching at the institution is a real visible task. Use the complete building footprint as its
   objective, reserve distinct teacher and learner slots plus routes and duration, and preserve the
   after-three-completed-work obligation through emergency deferral, worker refusal, death, and
   restart. Nursery never becomes a bed merely because a teaching task targets it.
5. A family enterprise sign is a presentation bound to a canonical enterprise record and real
   operational site. Give the sign instance, enterprise, family branch, site, art state, and
   localization key stable IDs. Render it only from the authorized snapshot, anchor it to the
   canonical site/footprint, provide an accessible text equivalent, and despawn or update it when
   the enterprise moves, matures, closes, loses the site, or changes branch.
6. The sign and enterprise may influence matcher preference, continuity, mentoring, history, and UI
   identity only. They never own colony goods, lots, items, storage, buildings, routes, currency,
   queues, or worker assignments. All physical work and cargo continue through the colony's shared
   task, reservation, storage, and station authorities.
7. Extend persistence, protocol projection, family/household panels, world selection/inspector,
   accessibility labels, and LAI.33A browser checkpoints together. Tests cover capacity boundaries,
   institution unlock/operation, deterministic allocation, kin/guardian rules, move/death/restart,
   exact-site teaching, sign anchor/dedupe/despawn, hidden-score redaction, and proof that the
   enterprise cannot mutate colony inventory or ownership.

## Recipe 19: add an election, office, or governance rule

1. Allocate stable election, candidate, ballot, voter, God-backing block, office, appointment,
   removal, succession, and expulsion-event IDs. Declare the complete merit formula and fixed-point
   Relational/Analytical interpolation. Candidate and tie ordering must end in merit, Governance,
   then stable cat ID; keyed variation uses an isolated election stream.
2. Every eligible Adult/Elder casts one cat ballot. The slate is the deterministic top five under
   the exact seven-term merit weights. Each eligible global player and the personal owner may keep
   one replaceable authenticated `+10` backing block; it advocates but does not replace cat voting.
   Scheduled elections and snap succession use the same persisted ballot and tie rules.
3. The elected Leader appoints/removes the seven officers using report-safe believed merit and may
   make a poor appointment. Cross-training and acting-office experience do not bypass report
   clearance. Leader death, vacancy, dismissal, and succession preserve or cancel plans through
   explicit adoption rules rather than transferring hidden truth.
4. Expulsion is a physical cleanup transaction. The personal owner may expel an adult or valid
   household, but dependents require a guardian. Resolve active job, office, election, residence,
   enterprise, carried cargo, reservations, equipment, and destination before departure; any
   unresolved identity aborts atomically.
5. Persist election epoch/slate/ballots/tally/backing blocks, office duty, appointment rationale,
   succession, expulsion plan, and idempotency receipts. Project bounded reasons and authenticated
   controls only. Tests cover slate/tally/ties, replacement blocks, scheduled/snap paths, poor/good
   appointment, vacancy/death, no inherited clearance, every expulsion cleanup stage, stale/replay,
   restart, authorization, and hidden-score redaction.

## Recipe 20: add staged construction, storage, or village works

1. A new building/workshop descriptor must include its stable type and instance IDs, complete
   canonical footprint, scaffold/structure/fit-out bill, exact work seconds, station slots, linked
   non-overlapping storage, research/permit gate, upgrades 1–10, stage sprites, and inspector labels.
   The founding/raw name is canonical Logs; developed scaffold/structure/fit-out bills use the
   manifest-owned Lumber/Planks/material IDs. Do not introduce a `wood` compatibility alias.
2. Every building, upgrade, and Hole-axis project follows
   reserve → deliver scaffold → 20% labor → deliver structure → 60% labor → deliver fit-out → 20%
   labor → operational. Inputs remain physical and identity-bearing at required/delivered/in-transit/
   consumed locations. Progress takes game time; cancellation, refusal, worker death, route loss,
   restart, demolition, and salvage conserve every unconsumed lot/item. Roads, walls, gates, farms,
   and other world works declare their own equally explicit stage sequence.
3. Divine click aid cannot bypass materials or create general inventory. A Log contributes exactly
   100 clicks; another eligible unit uses `ceil(100 × unit value / Log value)`. Rare materials,
   completed equipment, fixtures, and augmentations are ineligible. Accepted cargo is provenance-
   tagged and purpose-bound; each bounded authenticated click removes one labor second from the one
   global meter, respecting batching and per-player rate limits.
4. A storage-zone descriptor declares footprint and four loose slots per tile. One container uses
   one slot; Basket holds four compatible entries, Barrel/Crate eight of one kind, Chest sixteen,
   and Rack eight equipment items. Preserve lot quality/age/provenance/reservation and finite item
   IDs. Workshop linked stockpiles are adjacent, non-overlapping, capacity-checked endpoints—not
   hidden aggregate inventory.
5. Persist the exact stage, bill, delivered/in-transit/consumed cargo, progress, workers, reservations,
   click meter/contributions, zone/container slots, linked endpoint, farms/crops, authored roads,
   walls/gates, and maintenance plan. Project full footprints/routes/stages/fullness/blockers. Tests
   enumerate every building bill, 20/60/20 boundaries, stage restart/cancel/salvage, click ratios and
   anti-arbitrage, container compatibility/capacity, no-overlap linked storage, impassable walls,
   road/farm stages, Leader timing, complete markers, and conservation.

## Recipe 21: add food policy, divine aid, Inspiration, or a Void miracle

1. A food policy extension declares stable edible ID and exact `Allowed`, `Reserve`, or `Forbidden`
   Leader policy semantics. Gods may send only the documented broad conservation nudge; they cannot
   edit individual food permissions or see hidden stock/regeneration. A weak or stale report may
   produce a poor/late policy. Lethal starvation may consume Forbidden food only when no physically
   available permitted alternative remains.
2. Ordinary Divine Rations and Divine Water are nonexpiring physical 100%-need items created at the
   Hole apron, default Reserve, and require high-priority real hauling. The ordinary contribution
   meter is uncapped but idempotent; it does not directly fill a cat need or inventory balance.
3. Inspiration is a per-player temporary effective-stat source: `+10%` for fifteen real minutes,
   sixty-minute same-player cooldown, no same-player stacking, and additive independent-player
   sources without a shared cap. It never mutates genes, age, permanent attributes, traits, XP,
   office expertise, or report capability.
4. A one-Void construction press creates only the exact missing purpose-bound input bundle worth
   twice a one-Void Hole feed and removes ten percent of original duration earliest-stage-first.
   Generated cargo cannot overfill, return to general stock, trade, or feed the Hole. A one-Void
   population rescue creates exactly `2 × living residents` Rations or Water and requires
   report-safe evidence of dying need. New construction inputs must use the manifest-owned
   classification, shared Hole resolver, and typed-generation procedure in
   [construction-miracle-value-authority.md](construction-miracle-value-authority.md); caller,
   trader, and coin values are forbidden.
5. Persist/debit/generate/apply each aid in one checked idempotent transaction with player, colony,
   target, purpose, provenance, population snapshot, start/expiry/cooldown, and receipt IDs. Project
   only owning-player exact balance plus report-safe eligibility. Tests cover policy mistakes and
   starvation override, apron hauling, meter idempotency, Inspiration stacking/expiry/no mutation,
   press bundle/duration/binding, rescue count/gate, stale/replay/restart, and no hidden-truth leak.

## Recipe 22: add a food definition or renewable food source

1. Allocate permanent content, source/site, task, quality/provenance, permission, recipe, art,
   localization, capability, and research IDs. Classify the edible as raw/prepared/preserved and
   declare exact integer nutrition, spoilage, storage compatibility, founding availability, and
   whether cats may eat it directly. Do not add a generic `Food` or `Fish` fallback kind.
2. If it regenerates, define an authoritative persisted ecology state: capacity, current quantity,
   growth clock, depletion behavior, season/biome inputs, and a named deterministic update order.
   Regeneration is hidden truth. Officers/Gods receive only the report level, range, confidence,
   provenance, and expiry their knowledge permits.
3. Give gathering a real site and complete footprint. Apple work uses the complete Apple-tree
   footprint, fishing uses the Fishing Hut plus orientation-specific dock/shore/water habitat,
   hunting uses the specific Lair, farming uses the plot, and water uses a valid source plus dry
   bank and delivery endpoint. A new source must be equally explicit. Reserve source quantity,
   work slot, route, cargo, destination capacity, and exact lot identity atomically.
4. Route the output through the universal quality calculation and physical lot ledger. Quality,
   age, provenance, spoilage, permission, and location survive hauling, storage, cooking, barter,
   Hole consideration, cancellation, route loss, and restart. Spoiled or consumed units leave one
   idempotent terminal receipt.
5. Add all station recipes that consume the food, their ingredient bundles, Cookhouse/Fishing Hut
   ownership, capability/research prerequisites, AI replacement-cost inputs, weak-report mistake
   behavior, starvation exception, and Hole eligibility/value. A God receives only broad food
   conservation controls, never an individual food switch.
6. Add exact icon/source/state art, accessibility text, report-safe Stores/Food/Hole UI rows,
   canonical protocol projection, separate persistence rows, diagnostics, and a fixture entry.
   Tests cover ecology partition/restart twins, depletion/no-fallback, quality/spoilage/conservation,
   exact marker geometry, strong/weak AI choices, report redaction, recipes, storage, Hole/trade
   eligibility, art resolution, and browser inspection.

## Recipe 23: add an item, tool, equipment, or furniture definition

1. Allocate stable definition, instance, material, recipe, capability, research, slot, art, and
   localization IDs. State whether the result is a finite exact item or a divisible lot; tools,
   equipment, fixtures, augmentations, furniture, and rare creature products are exact identities,
   never anonymous bulk.
2. Declare compatible materials, required station/tier/skills/tools, input bundle, work duration,
   quality formula, durability/wear/repair rules, body/hand/building/container slot compatibility,
   and all gameplay effects. Effects use typed fixed-point fields; display copy does not become
   executable configuration.
3. Define every state transition: reserved inputs, in-transit cargo, station input, consumed
   inputs, produced item, equipped/installed/stored/carried, damaged/broken, repair input, salvaged,
   traded, recovered from death, and terminal destruction. One stable item ID survives every
   nonterminal move, and every transition has one idempotent receipt.
4. Add capability/research and planner/officer knowledge. The Leader may plan acquisition,
   production, assignment, repair, or replacement from reports; routine exact production and
   assignment do not become God actions. The matcher must enforce anatomy, willingness, clearance,
   slot, quality, and availability before reserving the item.
5. Persist definition version, instance, material, quality, durability, provenance, location,
   owner/equipper/fixture slot, reservation, effects, and transition receipt. Project only
   authorized exact identity/equipment facts and report-safe availability.
6. Add a native-size pixel icon/silhouette and every necessary equipped/installed/broken/repair
   state with transparent bounds and accessible fallback. Tests cover recipe conservation, quality,
   durability, slot rejection, death/refusal/cancel/restart, trade/Hole/miracle eligibility,
   exact-item UI, art lookup, order twins, and no duplicate instance.

## Recipe 24: add an augmentation or fixture

1. Allocate stable definition, exact instance, compatible target/slot, material, recipe, effect,
   capability, research, art, and localization IDs. An augmentation modifies one compatible exact
   item; a fixture occupies one typed station/building fixture slot. Neither is a divisible lot.
2. Define installation and removal as physical Workshop/station tasks with the complete canonical
   building footprint, exact work slot, source and target endpoints, routes, tools, cargo, and
   atomic reservations. State whether removal is reversible, consumes anything, changes quality or
   durability, or can fail; never silently replace an occupied slot.
3. Effects are typed fixed-point contributions keyed by effect ID and applied only by the owning
   item/station authority. State stacking, incompatibility, quality scaling, damage/breakage,
   repair, and inactive conditions. The planner and matcher consume the authority result rather
   than duplicating the formula.
4. Preserve both instance IDs, provenance, quality, location, target binding, reservation, and
   install/remove receipt through cancellation, worker refusal/death, route loss, target loss,
   restart, salvage, and trade. Rare materials, completed equipment, fixtures, and augmentations
   remain ineligible for construction-miracle bulk generation.
5. Add report-safe protocol detail and item/station inspector state. Routine installation/removal
   is Leader-owned; no new God action is added. Persist the exact binding in its own aggregate row
   and validate dangling/wrong-slot/duplicate bindings fail closed.
6. Add exact icons and installed/empty/broken slot visuals with accessible labels. Tests cover
   compatibility, occupied-slot rejection, effect math, quality, conservation, install/remove
   restart twins, death/cancel/salvage, trade restrictions, miracle rejection, redaction, and UI.

## Recipe 25: add a creature, Lair band, named drop, or portrait

1. Allocate stable species, generation/band, Lair, encounter, drop, material, portrait, world-art,
   capability, research, and localization IDs. Keep the public world-facing Lair visual band
   separate from the coarser encounter band; neither reveals an exact hidden level.
2. Declare deterministic ecology, capacity/regeneration, threat, health, anatomy/attack/defense,
   encounter composition, eligibility, and exact keyed roll purpose. Key encounter/drop randomness
   by world seed, Lair ID, generation, species, and clear index so input ordering and unrelated
   species additions cannot perturb an existing result.
3. A Hunt task targets the specific revealed Lair/cave, full objective footprint, reachable work
   slots, approach/return routes, hunters, equipment, cargo, and recovery endpoint. Missing,
   unrevealed, depleted, blocked, or unreachable Lairs produce a typed blocker and no marker,
   assignment, or fallback coordinate.
4. Define each ordinary and rare drop as a physical lot or exact item with quantity, quality,
   provenance, storage, recipe, trade, Hole, research, and miracle eligibility. The first-clear
   guarantee and any band floor are explicit and idempotent; death/retreat/route loss cannot
   duplicate a drop.
5. Persist Lair generation/ecology/clear index, encounter and hunter state, reservations, recovered
   cargo, drop receipts, and report knowledge. Project only report-safe species/bands, success
   ranges, and known equipment/health; exact hidden ecology and future rolls never cross the wire.
6. Add the exact native-size portrait, world Lair sprite/band state, material icons, accessibility
   labels, and manifest mappings. Tests cover deterministic encounter/drop twins, first clear,
   depletion/regrowth/restart, exact site/no-fallback, hunter injury/death/recovery, cargo
   conservation, report redaction, art dimensions/transparency, and browser inspection.

## Recipe 26: add a report or expose a formerly hidden field

1. First classify the value as authoritative hidden truth, an exact player-owned value, a direct
   observation, an officer report, a derived belief, a public fact, or a debug-only internal
   value. Write down who can produce it, who can receive it, report level, precision/range,
   confidence, expiry, contradiction, and whether it survives restart.
2. Add a stable subject/report/reason ID and a typed value/range. Report generation reads the
   authority once and emits only the permitted transformation; planners and Gods consume the same
   persisted report/belief projection. Never let the UI query the authority again, infer a hidden
   value from art/geometry/timing, or receive it in an error, `Debug`, diagnostic payload, tooltip,
   accessibility label, or test-only field.
3. State exact capability gates. For example, Hole regeneration is absent through officer level 3
   and level 4+ exposes only the permitted estimate range. A God has no broader visibility merely
   because an officer can report it.
4. Persist report identity, reporter, subject, observation tick, value/range, confidence, level,
   expiry, source provenance, supersession/contradiction, and delivery receipt separately from the
   hidden authority. Sorting and conflict precedence use semantic source priority then stable IDs.
5. Add the smallest canonical snapshot field and deep validation only if a player is authorized to
   receive it. No direct action is implied. Add bounded missing/stale/unavailable presentation and
   provenance text; never fill absence with an executor-derived default.
6. Tests must include hidden-truth twins with different authorities but byte-identical public
   output, every level boundary, expiry/decay/contradiction, restart, multi-colony redaction,
   unauthorized errors/logs/diagnostics/accessible text, and browser views.

## Recipe 27: add a sprite, icon, portrait, overlay, or state sheet

1. Inspect the shipped assets at native size and gameplay zoom before generating or drawing the
   addition. Record the reference files, palette, perspective, outline/shading rules, pixel scale,
   transparency convention, anchor, and semantic size class. Do not substitute a placeholder,
   recolored unrelated asset, smooth vector, generic AI style, or client-drawn fallback.
2. Allocate one stable `ArtKey` per semantically distinct authoritative state. Reuse an existing
   image only when the state is truly identical and record that alias in the manifest. Unknown
   keys fail closed with accessible text; they do not resolve to a generic image.
3. Produce the exact native dimensions required by its class and keep nearest-neighbor hard edges:
   current canonical classes include 16×16 icons, 32×32 detail/fixture assets, 48×48 ordinary
   building/state art, and 80×80 Hole/Lair/creature art. A new size must be justified by the
   renderer contract rather than an image generator's default.
4. Crop transparent bounds without clipping effects, define anchor/pivot, and enumerate every
   trigger state: construction scaffold/structure/fit-out/operational, orientation, idle/working,
   crop/ecology fullness, container fullness, quality badge, family/enterprise sign, damage,
   selection, or transport overlay as applicable.
5. Register the key in the canonical manifest/resolver, authoritative state-to-key mapping,
   preload path, native/WASM package, accessibility fallback, and visual inventory. The renderer
   consumes the server-projected state key and never recomputes hidden state.
6. Evidence includes deterministic key resolution, exact dimensions/color mode/transparency and
   nonempty bounds, missing-key failure, state/despawn/restart/zoom behavior, all required layouts
   and UI scales, screenshot examples at gameplay zoom, and independent visible-browser review.

## Recipe 28: add a diagnostic, blocker, rejection reason, or heartbeat field

1. Allocate a stable bounded reason/phase/probe ID and assign one owner. A diagnostic observes an
   already-defined phase or rejection; it never mutates simulation state, advances RNG, retries a
   task, repairs data, or becomes a second scheduler.
2. Define exact emission conditions, severity, aggregation window, cap/eviction order, counters,
   IDs, timestamps/ticks, task/colony correlation, and report visibility. Prefer one transition
   record or aggregate counter over per-tick log spam.
3. Redact hidden stock, regeneration, candidate scores, future rolls, auth material, filesystem
   paths, SQL text, and opaque executor payloads. Player-visible blockers use the same canonical
   report-safe reason as the planner/UI; internal probes stay behind explicit opt-in controls.
4. For long campaign probes include last completed phase, tick/colony/task/reservation counts,
   pending/running/blocked/recovery counts, last progress tick, and bounded memory/event counts so
   a liveness failure can be located without dumping the world.
5. Persist only diagnostics that must survive restart or support idempotent receipts; otherwise
   derive them from bounded canonical transition records. Protocol and UI expose only declared
   report-safe entries. Test-build probe APIs are compiled/authorized separately from production.
6. Tests cover cap/eviction, order/tick-partition twins, no state/RNG change, redaction, restart
   where applicable, liveness heartbeat cadence, no repeated spam, protocol bounds, and accessible
   presentation.

## Recipe 29: add a board card and evidence package

1. Read both locked plans, their requirement/visual/conflict registers, direct user notes, and the
   current line-1 cursor before defining scope. New cards are additive: never renumber, delete,
   compress, summarize away, or silently reassign an existing requirement.
2. Give the card one stable ID, title, dependency list, sole root owner, exact implementation
   contract, named source/test/doc/art/fixture deliverables, and acceptance evidence. Map every
   applicable P1/P2/GUI/conflict row directly; “covered by overhaul” is not traceability.
3. Record design, red, green, quality, QA, migration/reset, visual/browser, and legacy-disposition
   evidence separately. A missing category is explicitly pending or not applicable with a reason;
   no narrow check is used as proof for a broader requirement.
4. Worker dispatches record model/reasoning choice, task/dispatch IDs, owned paths, prohibited
   roots, resource limits, and completion receipt. Concurrent work never shares a hot root or runs
   heavy validation in parallel.
5. Append implementation discoveries, changed assumptions, failures, test results, screenshots,
   fixture/checksum metadata, and remaining work. Correct factual errors without deleting their
   historical evidence. Keep locked-plan checksums unchanged.
6. Move status only when the card's own evidence supports it. `dev` means implemented foundation,
   not integration; `qa` requires the named gates; `done` requires one live authority and every
   legacy/source/asset disposition. Final acceptance audits every requirement row against current
   source and runtime evidence.

## Real-browser QA for UI-bearing extensions

Unit, protocol, rendering, and headless WASM tests remain required, but none substitutes for
[LAI.33A real-browser acceptance](testing-cutover.md#real-browser-acceptance-lai33a). LAI.33A
requires both a Playwright browser play-test run and an independent visible-browser observation
run. Every extension that changes a snapshot-visible field, action, marker, panel, accessibility
label, persistence journey, or browser error path must add or update the scenario in both layers.
Use [browser-playtests/README.md](browser-playtests/README.md) for the extension checklist,
[browser-playtests/playwright-scenario-manifest.md](browser-playtests/playwright-scenario-manifest.md)
for checkpoint fields, and [browser-playtests/evidence-schema.md](browser-playtests/evidence-schema.md)
for artifact schema changes.

### Serve Rust and Trunk through Portless

Use the globally installed `portless` binary and stable base names. Portless assigns each child a
free port in `PORT`; the child must bind that exact value. In linked worktrees Portless may prepend a
worktree label, so the authoritative URL is the named `.localhost` route printed at startup and by
`portless list`/`portless get`, never a guessed numeric port.

From the repository root, start the Rust server in one terminal:

```bash
portless --name leader-ai-api cargo run -p cat-server
```

`cat-server` already reads the injected `PORT` through `ServerConfig`; do not set a fixed server
port. While it remains running, use another shell to run `portless get leader-ai-api`; record the
exact API route, replace its `https://` scheme with `wss://`, append `/ws`, and bake that value into
the browser client. In a second long-running terminal:

```bash
cd crates/cat-web
CAT_SERVER_URL=wss://<exact-named-api-host>/ws \
  portless --name leader-ai-browser sh -c \
  'exec trunk serve --release --address 127.0.0.1 --port "$PORT"'
```

While both services remain running, use another shell to run `portless list` and
`portless get leader-ai-browser`. The acceptance URL is the exact stable named `.localhost` browser
route returned by Portless.
`crates/cat-web/Trunk.toml`'s `8080` is only a raw default; the command-line `--port "$PORT"`
override is mandatory. A run fails setup if either process binds a different port, `portless list`
does not show both named routes, the browser uses a numeric localhost URL, or the WebSocket does not
connect through the named API route.

This workflow is Rust/Trunk only. Do not invoke Bun, `bunx`, a legacy Next.js command,
`scripts/portless.mjs`, or `scripts/build-web.sh --serve`; those paths either target the removed web
game or hard-code a port and cannot provide LAI.33A evidence. Do not bypass Portless with
`PORTLESS=0`/`PORTLESS=skip`.

### Run the Playwright browser play tests

Use the connected Playwright browser automation against the exact named Portless route. Exercise
only shipped player controls using accessible roles, labels, visible text, pointer/keyboard input,
and navigation. Record the locator/action/assertion transcript, before/after screenshots, console
messages, failed requests, current URL, simulation tick, and stable entity/action IDs for every
checkpoint. Do not use JavaScript evaluation or private endpoints to mutate state, bypass
authentication, manufacture inventory/Notes/Void, skip physical hauling, or advance the simulation
through an undocumented hook.

Every checkpoint in
[browser-playtests/playwright-scenario-manifest.md](browser-playtests/playwright-scenario-manifest.md)
runs in Playwright, including startup, reload/reconnect, and stale-action paths. The Playwright run
is a required automated play test, but it does not replace the visible desktop-browser evidence.

### Operate the actual browser

Use a visible desktop Chrome, Chromium, Edge, or Firefox window through `orca-ide computer`. Do not
replace this independent observation with the Playwright run, WebDriver, a headless-only browser,
`curl`, Bevy Remote Protocol, DOM script injection, or screenshots produced without browser
interaction. The browser and its DevTools are part of the observed product.

The operator loop is:

```text
orca-ide status --json
orca-ide computer capabilities --json
orca-ide computer list-apps --json
orca-ide computer list-windows --app <browser-app> --json
orca-ide computer get-app-state --app <browser-app> --window-id <id> \
  --restore-window --json
```

Choose the address-bar element from the fresh accessibility tree, set it to the exact Portless URL,
press Return, then refresh state. Use `set-value`, `click`, `press-key`, `scroll`, and `drag` with
element indexes from the latest state. Indexes are invalid after any navigation, scroll, focus
change, or Bevy rerender, so capture a new `get-app-state` before the next action. Coordinate clicks
are allowed only when the canvas has no semantic element and must use the latest screenshot's scale
and window-local coordinates.

For every checkpoint, save the complete `--json` result: its accessibility tree proves labels,
values, enabled/disabled controls, focus, and visible report text, while its `screenshot.path`
provides the pixel evidence. Copy that screenshot file into the checkpoint bundle immediately; a
temporary path alone is not reproducible evidence. A screenshot without the matching accessibility
state is not evidence, and an accessibility tree without the matching screenshot cannot prove
world-marker placement.

Open the real browser DevTools with:

```text
orca-ide computer hotkey --app <browser-app> --window-id <id> \
  --key CmdOrCtrl+Shift+I --json
orca-ide computer list-windows --app <browser-app> --json
orca-ide computer get-app-state --app <browser-app> \
  --window-id <devtools-id> --restore-window --json
```

Use fresh state to select Console, capture its accessibility tree and screenshot at startup, after
each scenario, after reconnect, and at the end. Uncaught exceptions, rejected promises, WebGL/WASM
failures, missing-asset 404s, repeated WebSocket failures, or any unclassified error fail the run.
Warnings must be enumerated in the evidence manifest with a disposition. DevTools is read-only for
this gate: do not type or execute JavaScript. If accessibility or screenshot permissions, the
browser window, or the DevTools console cannot be observed, the run is blocked rather than waived.

### Evidence bundle and extension scenarios

Save one immutable bundle under
`docs/leader-ai-overhaul/evidence/lai33a/<commit>-seed-<seed>/`. Its manifest records the full commit
SHA, dirty-tree status, deterministic world/test seed, exact browser and API Portless URLs, browser
name/version, OS, viewport, protocol/schema versions, SQLite fixture/checksum, server and Trunk
commands, start/end simulation ticks, scenario order, every Playwright locator/action/assertion,
Playwright screenshot, console entry and failed request, every `orca-ide computer` command, and each
visible-browser accessibility JSON, screenshot, DevTools console capture, warning disposition, and
PASS/FAIL result. A dirty tree must include the diff hash. Filenames use ordered
scenario/checkpoint IDs so a reviewer can replay the same route and seed.

At minimum, an affected extension updates and reruns the relevant checkpoint:

1. Workshop task: the inspector/accessibility state identifies one canonical 3 x 3 objective and
   nine ordered cells; the screenshot visibly covers all nine tiles, with work slot and delivery
   marker distinct and no duplicate/stale cell.
2. Hunt and Fetch Water: Hunt is visibly pinned to the revealed cave/hunting-source identity, not a
   radial fallback. Water shows the actual water-source tile, distinct reachable dry bank/work
   position, and delivery endpoint in both task state and pixels.
3. Plans and officers: operate top-eight Plan controls and observe refreshed score/reason/confidence;
   inspect officer reports, vacancy/authority behavior, and stale-action feedback through the
   accessible UI.
4. God secrecy: in owning-god and other authorized-god sessions at report levels below four,
   screenshots, accessibility trees, browser console/errors, tooltips, and inspectors contain no
   regeneration field/value or hidden-truth sentinel. The owning player's Notes/Void remain exact;
   report bounds and provenance remain visible.
5. Hole, Notes/Void, and research: complete a physical Hole feed through delivery/consumption,
   observe one micro-Void credit, inspect both research lanes, queue/front/preparation/refund state,
   and purchase an affordable ordinary study, Hole-axis study, or boost with the correct exact
   currency debit and refreshed state.
6. Cat care: inspect attributes/personality/stress/anatomy and exercise an available treatment,
   consent/refusal, prosthetic fit, or repair control without losing cargo/item identity.
7. Diplomacy and trade: use two authorized browser sessions to establish mutual consent, create and
   accept a belief-valued contract, and observe escrow plus physical pickup/delivery or its explicit
   block; neither session may reveal the other colony's hidden stock.
8. Save/restart: capture state during an active task/offer/research/fit/trade stage, restart the Rust
   server against the same SQLite file and named Portless route, let the actual browser reconnect,
   and prove IDs, cargo, reservations, Notes/Void, reports, stage, and controls match the pre-restart
   state without duplicate effects or stale markers.

Any UI-bearing implementation card remains below `done` until its automated coverage is green and
LAI.33A can execute these checkpoints through both Playwright and the independently observed visible
browser. The dedicated implementation/execution owner is orchestration task
`task_99e5e9fd0657`.

## Worked example: a second 3 x 3 Workshop-like building

This example is illustrative and intentionally names an unused hypothetical type. Before coding,
search the complete repository and persisted fixtures to confirm the IDs remain unused.

1. Add `BuildingType::CeramicsStudio` only to the shared sim enum, then add manifest entry
   `ceramics_studio`, blueprint IDs for new level 1 plus upgrades 2–10, art/localization keys, and
   recipe `ceramics_studio_tiles`. The recipe consumes the manifest Clay content ID and produces a
   finite Brick lot/item ID. Give the first work slot semantic ID `ceramics_studio:kiln_0`; instance
   IDs remain colony-scoped building IDs. Protocol projects these stable IDs through LAI.64 instead
   of growing a parallel building enum.
2. Do **not** add `CERAMICS_STUDIO_WIDTH`, `CERAMICS_STUDIO_TILES`, or a client size. Add
   `BuildingType::CeramicsStudio` to the same canonical 3×3 arm of
   `spatial_tasks::footprint_for`; add its exact phase bills/durations/permits/art to
   `construction_catalog.rs`. Construction, occupancy, spatial objective, protocol snapshot, and
   Bevy each call or receive that descriptor. The nine cells are row-major offsets `(0,0)` through
   `(2,2)`.
3. Add one manifest/station recipe descriptor for the studio, its Clay input domain, finite Brick
   output domain, founding availability or one stable research study, fixed work duration, local
   capacities, and default Leader-owned queue. Execution queries the descriptor; it does not copy
   input/output constants into planner, protocol, server, or UI. Gods may apply only an allowed
   broad construction/building-kind nudge, never place the studio or queue its tiles.
4. A production task's objective is the entire nine-cell anchored footprint. Work is the reserved
   reachable `kiln_0` slot; input endpoint is the studio's Clay compartment; output endpoint is its
   finite item compartment and then a pinned compatible stockpile. The atomic reservation includes
   Clay source/amount, routes, slot, cargo, output capacity, and destination headroom in the world
   ledger, conflicting stably with every colony.
5. The sim owns exact Clay and Brick state. Reports expose only authorized stock beliefs and the
   complete revealed task site. Persist type, instance, queue, slot/cycle, local stores, item ID,
   task stage, reservations, aggregate versions, and idempotency receipts in the fresh schema.
   Regenerate the authoritative fixture/checksum; do not add an old-save compatibility alias or
   semantic conversion.
6. Failure before Clay consumption returns it to the source/station compartment. Failure after the
   consumed-step marker completes one Brick output exactly once. Route closure blocks/revalidates;
   worker refusal releases the slot but preserves cargo; malformed fresh-schema state fails closed;
   duplicate commands/restart cannot duplicate the building or item.
7. Focused tests assert unique IDs, `(3,3)`, nine ordered cells, no duplicated footprint constant,
   placement collision, two-colony site conflict, distinct multi-worker slots, exact conservation,
   research entitlement, every stage across restart/cancel/refusal, protocol JSON, server redaction/
   authorization/idempotency, and exactly nine Bevy cells with dedupe/despawn. Then run every gate
   in [testing-cutover.md](testing-cutover.md).

## Overall subsystem extension checklist

- [ ] One authoritative descriptor/leaf owns the new semantics; roots only integrate it.
- [ ] Every type, instance, task, action, event, reservation, and catalog entry has a stable ID.
- [ ] All values are integer/fixed-point where rules observe them; ordering and tie-break keys are
      written down and tested against shuffled input.
- [ ] Every RNG decision has a named keyed stream and batching/order twin; deterministic behavior
      uses no incidental RNG.
- [ ] Authority, visibility, report level, confidence, expiry, and every forbidden hidden field are
      enumerated.
- [ ] Every visible task has complete objective/footprint, work position/slot, endpoint, route,
      cargo, stage, and typed blocked state.
- [ ] Source, site, route, slot, cargo, and destination reservations commit atomically and conflict
      across colonies where world-exclusive.
- [ ] Fresh-schema aggregate/version, fixture/checksum regeneration, replay, unknown/future-version
      rejection, downgrade policy, and malformed rollback/quarantine are covered; no semantic
      conversion or compatibility alias was added.
- [ ] Cancellation, refusal, death, depletion, closure, stale/duplicate action, and restart conserve
      physical cargo/items and idempotent ledger effects.
- [ ] Protocol version/defaults, authorization order, complete redaction, Bevy stale-state cleanup,
      and accessibility are covered where relevant.
- [ ] Every authoritative visual state has a game-style production `ArtKey`, exact native asset,
      deterministic resolver mapping, accessible fallback, trigger matrix, and zoom/layout
      evidence; no placeholder or generic fallback remains.
- [ ] The owning board card directly maps every applicable plan, Q&A, visual, conflict, and
      contributor-guide requirement and preserves append-only evidence.
- [ ] Every browser-visible change is represented in the LAI.33A workflow and its real browser is
      served through named Portless routes honoring injected `PORT`, operated with
      `orca-ide computer`, and evidenced by matching accessibility, screenshot, and console captures.
- [ ] Focused red/green, deterministic, persistence, protocol, server, UI, smoke, Clippy, format, and
      whitespace evidence is recorded on the owning board card.

## Exact wire-version and identifier extension rules

Every new player mutation needs its own authoritative concurrency lane. Do not reuse the aggregate
snapshot `stateVersion`, a display version, a queue length, or a planning epoch.

1. Add the lane to LAI.64 `VersionLane` only when no existing lane owns that exact aggregate.
   Update its fixed count, snapshot `versions`, the allowed action's exact `required_lanes`, deep
   validation, server `CanonicalVersionSource`, receipt validation, and persistence atomically.
2. Compute each published version from the same canonical server-owned aggregate that mutation
   preflight validates. Domain state remains outside the Leader fingerprint.
3. `CanonicalActionEnvelope.expectedVersions` must equal the action's required lane list exactly,
   already sorted in enum order—no missing or unrelated lane. Compare after trusted-session and
   colony authorization but before rate-limited/domain mutation. Replay lookup happens before
   version/rate checks so a lost response cannot consume twice.
4. The committed receipt carries the same exact lane identities at their post-commit values. Store
   aggregate mutation, replay receipt, rate state, and related session state in one SQLite
   transaction.
5. Add accepted, stale, malformed, unauthorized, duplicate replay, same-ID/different-payload
   conflict, restart, and unrelated-domain-change tests. Production rejects signed reset; test
   builds require the staged, signed, consuming two-step reset gate.
6. Routine construction, placement, routes, storage, production, food entries, worker assignment,
   officer appointment, standing orders, and trade decisions never gain a God action lane. They
   remain canonical Leader/officer commands behind the runtime transaction.

Canonical `StableId` is an opaque, nonempty, non-control UTF-8 value bounded to 512 bytes on the v3
wire. This losslessly carries colon/hyphen/dot IDs and nested length-prefixed `PlannerId` values such
as `planner:v1|…`; do not normalize, truncate, hash, alias, or parse them on the client. Family and
runtime authorities accept the same real lower-ASCII ID forms needed by their domain, including
nested Task/receipt IDs up to the same bound. Display labels remain separate bounded report text.

Catalog IDs exposed verbatim—such as a research study ID—must be resolved against the advertised
catalog entry. Do not derive an already-derived ID a second time.

For a detailed failure workflow and the opt-in phase/probe APIs, see
[diagnostics-and-debugging.md](diagnostics-and-debugging.md).

## Documentation update checklist

- [ ] Update the authoritative domain file in this directory and link the exact section from the
      implementation card's design evidence.
- [ ] Update this guide when a registry, module path, version rule, or extension recipe changes.
- [ ] Update [spatial-task-contract.md](spatial-task-contract.md) for every visible task/site mapping,
      including objective/work/delivery/route/reservation semantics.
- [ ] Update [planner-and-beliefs.md](planner-and-beliefs.md) for goals, operators, intents, officers,
      reports, beliefs, ordering, RNG, authority, cadence, or retry changes.
- [ ] Update [cats-and-care.md](cats-and-care.md),
      [hole-hunting-content-integration.md](hole-hunting-content-integration.md), or
      [diplomacy-barter.md](diplomacy-barter.md) for their domain constants and failure states.
- [ ] Update [wire-persistence-ui.md](wire-persistence-ui.md) for protocol, persistence,
      authorization, redaction, or Bevy contracts and version/default behavior.
- [ ] Add focused/campaign/journey cases to [testing-cutover.md](testing-cutover.md) and exact commands
      and artifacts to [BOARD.md](BOARD.md).
- [ ] Add or update the matching scenario and evidence manifest fields in
      [LAI.33A real-browser acceptance](testing-cutover.md#real-browser-acceptance-lai33a) for every
      browser-visible field, action, marker, error, redaction, or restart behavior.
- [ ] At direct cutover, synchronize every maintained root document listed in
      [README.md](README.md#cutover-documentation-synchronization); mark historical documents as
      superseded instead of copying a second source of truth.
- [ ] Run local-link, unique-card-ID, conflict-marker, Markdown structure, and whitespace checks and
      attach the results to the board evidence.
