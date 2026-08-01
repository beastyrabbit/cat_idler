# LAI.46 static integration review

Status: working, non-acceptance inventory.

This file records the current source-level state of LAI.46 after Orca task
`task_66f03c3fe6df` / dispatch `ctx_8328670802af`. It is deliberately not proof
that LAI.46 is complete. The user-owned serialized Cargo/build/test/browser gate
has not been run for this pass, and the card remains `todo` until the complete
runtime path exists and that gate supplies evidence.

The two exact stored plans and every requirement row in `BOARD.md` remain
authoritative. This inventory may add evidence and implementation detail; it
must never replace, compress, reinterpret, or remove those requirements.

## Static implementation that exists

- `spatial_resolver::validate_truthful_task_geometry` rejects stable IDs
  containing `generic`, `fallback`, `center`, or `reported_work`.
- The truthful geometry validator requires:
  - a one-cell named source and a distinct adjacent work bank for Hunting,
    Water, Fishing, and Quarry work;
  - the complete ordered 3×3 Workshop work area and a Workshop building
    objective;
  - complete station, farm, and construction footprints;
  - a complete 3×3 Apple obstruction/work footprint;
  - the permanent 5×5 Hole landmark, its centered 3×3 work area, and a pinned
    delivery cell on the landmark edge;
  - a complete 3×3 Cookhouse work area;
  - the complete Fishing Hut land footprint plus its orientation-specific
    dock/shore/water attachment;
  - routes whose endpoints actually touch their declared objective, work area,
    and delivery endpoint.
- `VisibleTaskRuntime::emits_world_marker` hides completed, blocked, and
  cancelled tasks and requires resolved geometry/routes before ordinary work
  can render a marker.
- The protected runtime transaction invokes the canonical scheduling path at
  most once for the canonical runtime tick and mirrors the shared world
  reservation ledger back into every colony after stable colony-ID arbitration.
- A losing cross-colony reservation is returned to explicit recovery rather
  than silently retaining a local reservation.
- The focused LAI.46 source contains cases for exact footprints, all Fishing Hut
  orientations, terminal marker suppression, cross-colony arbitration,
  recovery, restart, shuffled ordering, and partitioned ticks. These cases are
  authored but are not evidence until the serialized test owner runs them.

## Critical end-to-end gaps

### 1. The canonical runtime does not own the live food/station authorities

`LeaderAiRuntimeState` currently persists planner, beliefs, scheduling, cats,
families, governance, research, construction, storage, Hole, divine policy,
boosts, trade, physical cats, directives, outcomes, and diagnostics. It does
not persist:

- `FoodEcology`;
- `FishingAuthority`;
- founding Apple-tree and Fish-habitat site instances through that ecology;
- Fishing Hut instances, orientation, dock, shoreline attachment, staffing, or
  operational state;
- Cookhouse station instances;
- `CookhouseQueue` and active `CookhouseBatch` state.

The strict leaf authorities already exist in `food_ecology.rs`, `fishing.rs`,
and `cookhouse.rs`, but the world-tick authority cannot call them from one
persisted aggregate. Exact geometry types without those live authorities are
contracts, not an end-to-end game path.

Smallest correct direction:

1. Add one canonical, versioned per-colony aggregate for these live instances
   inside the sole Leader-AI runtime authority.
2. Initialize it from the real founding sites and later construction
   materializations.
3. Validate and advance Apple regrowth and Fish replenishment exactly once per
   canonical world tick.
4. Bind all catch/harvest/cooking receipts to the existing physical lot and
   item authorities.
5. Persist strict bounds, version, restart, partition, and idempotency state.
6. Project only the report-safe ecology view to planners and Gods; exact
   regrowth/replenishment remains server-only.

### 2. The live task enum and materializer cannot express all required work

`task_runtime::TaskCategory` has Hunt, FetchWater, Fish, Quarry, Logging,
WorkshopWork, FarmWork, HaulDelivery, and legacy categories, but no exact
Apple-harvest, Cookhouse-work, Fishing-Hut-work, or Hole-work category.

The current planner-to-task mapping therefore aliases:

- Apple recovery to `FibreForage`;
- Cookhouse recovery/supply to `HaulDelivery`;
- Hole feeding to `HaulDelivery`.

The general spatial materializer converts only Hunt, FetchWater, Fish, and
WorkshopWork into `SpatialTaskCategory` values. Its candidate builder likewise
handles only those four categories, plus separately typed research preparation
and emergency supply paths. Farm recovery, Apple work, Cookhouse operation,
staffed Fishing Hut operation, Quarry, Logging, and other exact station work do
not pass through this bridge.

Smallest correct direction:

1. Add exact canonical runtime categories where behavior and spatial rules are
   materially different.
2. Preserve stable serde names and version the containing aggregate.
3. Map planner goals to those exact categories instead of a semantically
   unrelated legacy alias.
4. Resolve each category only from its live authoritative site instance.
5. Execute each accepted task through its existing domain authority and emit
   once-only physical lot/item/progress receipts.

### 3. Cookhouse and Fishing Hut have no closed live building identity

`types::BuildingType` contains legacy buildings and the Plan 2 Family Home and
Elder Lodge, but not Cookhouse or Fishing Hut. The manifest can name both
stations, and pure footprint functions exist, but construction cannot
materialize either as a closed live building type.

This also means the current Cookhouse geometry validator proves only that a
rectangle is 3×3; it cannot prove that the rectangle belongs to the persisted
Cookhouse instance. The Fishing Hut validator proves the geometry shape but
does not prove that it matches a persisted Hut ID and forged orientation.

Smallest correct direction:

1. Introduce the two stable building/station identities without changing the
   ordering or wire identity of existing variants.
2. Update every exhaustive building match and canonical footprint lookup.
3. Materialize completed Cookhouse and Fishing Hut construction into those
   exact types.
4. Bind spatial validation to the persisted station ID, anchor, orientation,
   dock edge, reserved water attachment, and operational state.

### 4. Hunting still resolves the wrong world tile type

The current Hunting candidate accepts `TileType::CaveEntrance`. The locked
visual and semantic contract requires an `EnemyLair` to remain distinct from a
Quarry `CaveEntrance`. A task that uses the Quarry cave tile for Hunting is
truthfully shaped but semantically false.

Smallest correct fix:

- resolve Hunting only at the specific revealed `EnemyLair` instance named by
  the report-safe planner candidate;
- resolve Quarry work only at `CaveEntrance`;
- keep their site IDs, markers, sprites, reports, ecology, and reservation keys
  distinct.

### 5. Cross-colony work-slot and delivery claims are colony-scoped

`world_reservations::build_claims` creates world-global objective-tile and route
keys, but prefixes WorkSlot and DeliveryEndpoint keys with `colony_id`. Two
colonies can therefore claim the same physical bank/work cell or delivery slot
under different colony-prefixed keys when the objective itself is capacity
shareable or the two tasks use different objective IDs.

The user requirement is world-shared physical exclusion/capacity with
colony-isolated ownership—not colony-scoped geometry.

Smallest correct fix:

- derive world objective, work-slot, and delivery keys from the physical
  site/slot identity without a colony prefix;
- emit world-tile claims for the complete work-slot site and exact delivery
  cell, using the work slot's own exclusivity/capacity instead of inheriting the
  objective's source-capacity mode;
- keep workers, tools, locally owned cargo, commands, and receipts partitioned
  by colony;
- make physical objective tiles, work cells, dock/water attachments, route
  segments, and storage slots collide globally according to their declared
  exclusive/capacity modes;
- recover deterministic losers through the existing rollback path.

### 6. Reservation admission can bypass the truthful geometry gate

The world-tick materializer calls `validate_truthful_task_geometry` before
inserting a newly resolved task. `WorldReservationTransaction::new`,
`WorldReservationTransaction::validate`, and `build_claims` call only the
lower-level `ResolvedSpatialTask::validate`.

Any caller outside the current materializer can therefore submit a structurally
valid center/generic/partial geometry directly to the world reservation API.
The truthful gate must be an invariant of reservation admission, not only one
call site.

Smallest correct fix:

- call `validate_truthful_task_geometry` inside world transaction construction,
  strict decode validation, and claim rebuilding;
- retain the earlier materializer check for a clear fail-closed boundary;
- add a source-level regression for direct transaction admission of each
  forbidden partial geometry.

## Major integration gaps

- The current Fishing candidate reads the legacy floating-point
  `ColonyRuntime::fish_habitats` stock rather than the canonical finite
  `FoodEcology`/`FishingAuthority` state.
- Apple work has no authoritative world-tick materializer, physical harvest,
  quality-lot receipt, or once-per-tick regrowth bridge.
- Staffed Fishing Hut bonuses, Rod identity/wear, finite-stock debit, catch
  receipt, travel, delivery, and recovery are not one atomic world-tick path.
- Cookhouse input reservation, travel, station delivery, work time, output lot
  creation, output pickup, queue advancement, cancellation, death, route-loss
  recovery, and backpressure are not one atomic world-tick path.
- FarmWork is produced by planner mapping but is not admitted by the general
  spatial-category bridge.
- Quarry exists in the task and spatial enums but is not admitted by the
  general spatial-category bridge.
- The focused test constructs several exact geometries directly. Direct
  construction proves the validator but does not prove that real world state
  can materialize and execute those tasks.
- The live world still contains legacy fishing, gathering, production, and
  survival phases outside the protected canonical transaction. Final one-path
  authority and deletion belong to LAI.52/LAI.63/LAI.70, but LAI.46 cannot be
  accepted while the new required categories exist only as validator fixtures.

## Additional independent static findings

A detached, lifecycle-rejected reviewer independently audited the same roots
and supplied exact file/line scenarios. Because its dispatch identity was
superseded, it is not accepted Orca completion evidence; its technical findings
are preserved here as non-acceptance input and must be confirmed by the
correctly tracked review and eventual serialized gate.

### Caller-asserted world validation is not authoritative validation

`WorldReservationValidation` is a public twelve-boolean input with
`all_valid()` and `Default = all_valid()`. Current production call sites
frequently pass that value directly:

- research preparation;
- canonical physical-task activation;
- autonomous trade escrow;
- persisted world-ledger mirror reconciliation.

The scheduler assignment path computes only source capacity. The remaining
objective-known/revealed/existence/occupancy, work-slot, route, quantity, cargo
capacity, tool, endpoint, and worker checks are caller assertions rather than
facts derived from the current world. Persisted reservations are consequently
recommitted as valid after restart even when a source, route, worker, or
endpoint changed while the server was down.

Smallest correct direction:

1. Make construction of successful validation evidence private.
2. Compute every field from the authoritative world, storage, worker, tool,
   cargo, route, and endpoint indexes.
3. Use that computation before every commit and stage transition.
4. Give persisted-mirror reconciliation an authoritative validation callback
   instead of hard-coding `all_valid()`.
5. Fail closed and invoke explicit task/cargo recovery on any failed field.

### The open-task marker helper is not used by live projection

`VisibleTaskRuntime::emits_world_marker` has only test references. The live
server task projection currently checks for an objective, footprint, and
projectable site kind, but does not suppress terminal stages through that
helper. Complete and cancelled tasks can therefore retain their full footprint
on the wire even though the helper itself returns false.

Smallest correct fix:

- make the live projection return no world marker/footprint when
  `emits_world_marker()` is false;
- test the real projection, not only the helper;
- keep non-spatial history in bounded task/event UI data if required.

### Local route claims are task-unique instead of route-unique

The local ledger derives its route claim from the ordered route IDs plus the
task ID. Two tasks using the same physical route therefore receive different
keys and never consume the same route capacity. A second runtime helper also
collapses both task routes into one capacity-one key, while the world ledger
claims two routes separately with their declared capacities.

Smallest correct fix:

- derive route claims from each physical route/segment stable ID without the
  task ID;
- reserve source-to-work and work-to-delivery separately;
- use the same capacity and granularity in local and world ledgers.

### The spatial/runtime contract hard-caps every task to one worker

`ResolvedSpatialTask::validate` requires exactly one work position and
`VisibleTaskRuntime::activate` requires exactly one assignment. That prevents
the documented multi-worker station/Workshop/construction behavior in which
workers share an objective through distinct exclusive work slots, including
crew research that adds real concurrent station slots.

Smallest correct direction:

- require one or more bounded work positions rather than exactly one;
- select and claim a specific position for each worker;
- reject duplicate slot assignment;
- cap assignments by both work-slot count and task/workforce policy;
- retain deterministic worker/slot ordering and restart identity.

### Logging geometry validation contains an unreachable branch

The truthful validator checks Logging work geometry only when its objective is
3×3, but category matching accepts the canonical logging footprint only at 2×3.
Logging therefore bypasses the intended work-position check.

Smallest correct fix:

- require the complete canonical 2×3 tree footprint;
- require a truthful reachable perimeter work tile adjacent to that footprint;
- claim both the footprint and the perimeter slot with their proper modes.

### Fishing Hut water attachment and delivery endpoint are conflated

The current Fishing Hut validator reads the reserved water tile from the
delivery endpoint. Route-role validation then requires the work-to-delivery
route to terminate at that water cell, and reservation code applies delivery
capacity to the water cell. The contract requires distinct roles:

- 3×3 Hut objective;
- land/dock/shore work position;
- oriented reserved water attachment;
- actual delivery/storage endpoint.

Smallest correct direction:

- add a distinct typed water-attachment role to the resolved task or to a
  category-specific extension;
- validate and world-claim it separately;
- leave the delivery endpoint as the exact storage/cargo destination.

### Local terminal transitions can leave a world reservation orphaned

Task completion, cancellation, pre-pickup blocking, post-pickup recovery, and
restart revalidation release the local reservation through
`VisibleTaskRuntime`, but that type does not own its world reservation ID.
World release is a separate caller obligation stored in
`SchedulingRuntimeAggregate::world_reservation_ids`. Nothing makes the two
releases atomic for every caller.

Smallest correct direction:

- make one runtime transaction own and release both reservation identities; or
- at minimum reject any persisted state in which a terminal/blocked task still
  owns a world reservation and route it through explicit recovery.

### Objective-tile capacity is repeated from caller-supplied site capacity

Every tile in a multi-cell objective receives the whole
`source_units/source_capacity` mode. This repeats site capacity per tile and
lets a caller-supplied capacity become the first committed capacity for every
cell. Differing declarations cause spurious mismatch conflicts; an inflated
declaration can monopolize the shared site.

Smallest correct direction:

- derive physical capacity from the authoritative site record;
- claim site capacity once on the site identity;
- use per-tile claims for truthful physical exclusivity/capacity only where
  that tile actually has such a role.

### Additional static correctness risks to preserve

- Persisted-mirror duplicate tie-breaking uses a serialization failure fallback
  of an empty byte vector; a failure must return an error rather than winning
  the deterministic comparison.
- Batch sorting calls an expectation-backed objective accessor before
  transaction validation; malformed hand-built state can panic before it is
  rejected.
- A task can call `complete()` without proving maximum progress.
- Local and world ledger versions use different overflow behavior, which can
  break restart/partition equality after rejected commits.
- Legacy `Shrine` and `OfferingRitual` variants remain reachable in the same
  roots and must be removed only in their ordered cutover card.
- Building construction is currently exempt from the two-route marker
  requirement, so the live projection must explicitly decide whether an
  unresolved construction footprint is allowed to render.
- Task cargo quantity is `u64` while reserved cargo units are `u32`; the
  transaction must range-check and prove equality instead of merely narrowing
  for the claim.
- Every colony mirrors the entire world reservation ledger. This is
  deterministic, but the 4,096-entry bound becomes a whole-world ceiling and
  clone-per-commit behavior becomes quadratic as the world grows.

## Confirmed-good static properties

The same independent review confirmed these implementation properties:

- colonies enter the world tick in stable colony-ID order;
- the protected runtime mutation is clone/validate/commit atomic;
- one shared world ledger is reconciled before colony processing and threaded
  through colonies in stable order;
- batch arbitration orders site, task, colony, then reservation ID;
- spatial candidate selection is deterministic, pinned, and fabrication-free;
- Hole geometry is the exact permanent 5×5/central-3×3/ring-delivery contract;
- forbidden marker IDs are rejected by the truthful validator;
- identical reservation replay is idempotent and conflicting replay fails;
- task cargo recovery preserves identity and never silently drops carried
  cargo;
- defense, village, and survival preemption are gated before pickup while
  worker death can interrupt any stage;
- colony partition ID derivation is consistent;
- dead workers are filtered before reservation;
- strict task and ledger deserialization validate persisted state.

## Required follow-up implementation slice

The next sole-editor LAI.46 slice should own only the simulation files needed
to close the above path. Its order is:

1. Add canonical live food/station aggregate fields and strict initialization.
2. Add Cookhouse/Fishing Hut live building identities and exhaustive footprint
   handling.
3. Add exact runtime task categories and planner mappings.
4. Materialize real Apple, Cookhouse, Fishing Hut, Farm, Quarry, Lair, Water,
   Workshop, Hole, and construction sites from authoritative instances.
5. Advance canonical ecology and station work once per world tick.
6. Route every result through physical lot/item/storage/cargo authorities.
7. Make objective/work/endpoint/tile/route claims world-global where the
   physical role is shared, while workers/tools/locally owned cargo remain
   colony-partitioned.
8. Make truthful geometry and computed twelve-field world validation admission
   invariants for commit, restart, and every stage transition.
9. Separate Fishing Hut water attachment from delivery, make local/world route
   claims identical, and release local/world reservations atomically.
10. Support bounded multiple work slots/assignments and repair the canonical
    2×3 Logging perimeter contract.
11. Suppress terminal task markers in the live server projection.
12. Add focused source cases for every live materialization, capacity,
    multi-worker, projection, restart, and recovery path.
13. Stop without running Cargo, builds, tests, formatters, or browser checks;
   the external serialized gate owns validation.

Protocol, server, SQLite, client, asset generation, UI, browser evidence, and
legacy deletion remain downstream cards and must not be folded into this hot
root patch.

## Validation state

- No Cargo command was run for this static review.
- No Rust compiler, test runner, Clippy, rustfmt, browser, Playwright, or image
  generation was run.
- Authored tests are not counted as passing evidence.
- LAI.46 remains open.

## Corrected Opus 5 review additions

A second supervised, dispatch-bound Opus 5 review completed after the inventory
above. It read the plans, board, spatial contract, hot roots, leaf authorities,
and focused source cases. It made no edits and ran no compiler, Cargo, test,
build, lint, formatter, browser, image, or validation command. The review
confirmed the findings above and added the following exact blockers.

### The live exact-task bridge is inert

`lai63_exact_task_cargo_binding` searches for an already existing, unreserved
lot whose provenance begins with the task's exact source identity:
`hunt:<source>`, `water:<source>`, `fish:<source>`, `quarry:<source>`, or
`tree:<source>`. No live authority produces those origins. The only matching
production-shaped code is inside the retired, never-compiled runtime described
below.

Therefore the four categories that currently resolve can remain in `Resolve`
forever: they receive geometry and routes, fail to find cargo, never call
`activate_physical_task`, never reserve a worker/world claim, and never produce
an outcome. The bridge's comment that it is a transport/work runtime rather
than a production source is accurate, but no live producer precedes it.

The correction must not fabricate a generic lot in the bridge. Apple work must
consume `FoodEcology`, fishing must consume `FishingAuthority`, Cookhouse work
must consume the Cookhouse batch authority, Hunting must consume the actual
Lair encounter/drop authority, and Quarry/Logging/Farm work must consume their
own physical source authorities. The emitted lot/item must use the exact source
site, stable task/command receipt, typed storage address, quantity, quality,
provenance, and conservation path.

### Four of fifteen spatial categories are resolved

The live `lai63_spatial_category` and candidate bridge cover only Hunt,
FetchWater, Fish, and WorkshopWork. Quarry, Logging, AppleHarvest, HoleWork,
CookhouseWork, FishingHutWork, Construction, RoadConstruction, StationWork,
FarmWork, OfferingRitual, and EmergencySupply have no complete matching
resolution path; Apple, Hole, Cookhouse, and Fishing Hut do not even have
truthful `TaskCategory` identities.

This makes the existing geometry source cases insufficient: most
hand-construct `ResolvedSpatialTask` values and validate the pure helper without
proving that `found_colony -> world_tick -> resolved_spatial_tasks` ever
materializes a live task.

### Hunting bypasses the Lair authority

The live Hunting candidate selects a revealed `TileType::CaveEntrance` with
legacy food, derives `hunt-source-x-y`, and creates a synthetic 1×1 objective.
It does not consume the specific `hunting_lair` identity, canonical footprint,
creature roster, encounter, or drop table. The resolver also requires the Hunt
objective to be 1×1, which prevents a real multi-tile Lair footprint.

Hunting must instead require an actual revealed `EnemyLair`/Hunting-Lair
record, preserve its stable ID and complete footprint, and validate that the
exclusive work slot is reachable and adjacent to the footprint. Quarry alone
uses `CaveEntrance`.

### Markers are emitted before activation

Resolution stores an objective and two routes while the task stays in
`Resolve`. `emits_world_marker` treats that as marker-ready because it only
checks geometry presence and a nonterminal stage. Combined with the missing
producer, these tasks can show permanent map markers for work no cat can start,
without a bounded blocked reason.

World markers must require an active local/world reservation and an open
physical execution stage. Unactivatable work belongs in the Plans/report
surface with a bounded reason, not as a live world marker.

### Delivery and route identities are conflated

All tasks in one resolution pass choose the same first vacant delivery slot
because the chooser does not track slots already pinned in that pass. With
capacity one, all but one task then lose admission.

The ordered-route helper also replaces the work-to-delivery route's stable ID
with the delivery-slot ID. The route world claim, persisted route ID, and
stranded-cargo recovery location consequently identify the stockpile slot
instead of the route. The route and endpoint must remain separate refs and
claims.

Route capacity is independently hard-coded to one in both local and world
claims. That turns shared paths into exclusive corridors. Route capacity must
come from the authoritative spatial/network rule and use identical granularity
in both ledgers.

### Shared objectives are accidentally single-worker

Workshop source capacity is one and that mode is stamped onto every tile in
its 3×3 objective. A second task conflicts on the first shared objective tile
before its distinct exclusive WorkSlot matters. Hole work similarly claims the
whole 5×5 landmark, so its apron/pinned edge can conflict with rescue or feed
deliveries.

Station/source concurrency must come from authoritative work-slot counts.
Objective sharing is allowed only through distinct exclusive slots, while the
Hole's central 3×3 work area and pinned delivery edge/apron carry their exact,
separate capacities.

### Restart losers and shared-ledger failure are not recovered

`reconcile_persisted_mirrors` returns losing world reservation IDs specifically
so their local reservations and carried cargo can be released/recovered. Its
only caller discards that vector. Restart can therefore normalize the world
mirror without executing the promised loser recovery.

During live ticking, the shared ledger is copied into each colony runtime and
then replaced from the staged colony on success. A failed colony transaction
is only guarded by `debug_assert!`; release builds can silently omit that
colony's claims from the world book for the tick. The world ledger must be the
single mutable authority, and any failed protected transaction must emit a
bounded diagnostic and preserve/recover all claims atomically.

### Placeholder geometry reaches persisted state

Unresolved non-Hole/non-Lair goals can receive a synthetic
`reported_work:<site>` 1×1 slot. The truthful resolver gate rejects that text
only after a task reaches resolution; the world-cutover validator does not
reject the placeholder stored on goals/intents/tasks. For the eleven categories
without a resolution path, placeholder geometry can persist indefinitely.

Missing authoritative geometry must yield a typed
`SpatialObjective::blocked(SourceUnavailable)` (or the exact bounded reason),
and the truthful geometry validator must be part of strict cutover/runtime
validation rather than a single resolution call site.

### Planner categories are semantically aliased

Apple recovery is mapped to fibre forage; Cookhouse supply is mapped to generic
haul delivery; defense to scout; self-preservation to eat; Notes/Void study to
generic station work. Category drives staffing, priority, risk, display, and
officer ownership. These aliases must be replaced with exact categories in
planner intent, visible task, reservation, marker, outcome, and report data.

### A retired 4,300-line runtime masks missing production behavior

`world_tick.rs` contains approximately 4,300 lines under
`#[cfg(any())] mod retired_lai23_runtime`. It is never compiled, linted, or
tested. It contains the only production-shaped source-origin lot minting,
computed world validation, per-stage revalidation, and visible task
movement/cargo phase. This makes static source inspection misleading and
leaves apparently complete code outside the product.

Do not revive it as a second authority. Port only the still-required behavior
into the sole canonical LAI.46 runtime, prove every consumer uses that path,
and delete the retired module in the ordered legacy cutover.

### Additional uncovered acceptance cases

The final focused source cases must directly prove:

- two colonies, different objectives, same physical work slot collide;
- two colonies, different objectives, same physical delivery endpoint respect
  exact capacity;
- every named live site is materialized from a founded world;
- admission and every stage transition recompute all twelve world facts;
- restart arbitration releases the losing local claim and conserves/re-homes
  carried cargo;
- once-per-tick equality covers every ecology/station/task mutation, not only
  phase receipts;
- unrevealed, unresolved, blocked, terminal, and placeholder objectives never
  emit live markers;
- a founding world contains reachable Water, Apple, fish habitat, shoreline
  work position, and their exact delivery paths;
- the shared station supports its bounded number of distinct worker slots;
- route identity and endpoint identity remain separate through restart and
  recovery.

These additions do not supersede or shrink the earlier inventory. LAI.46 stays
`todo` until the sole editor closes them and the external serialized owner
provides actual compile/test evidence.
