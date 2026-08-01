# Authoritative spatial task contract

Every visible task refers to authoritative world data. The client renders only snapshot task state;
it never infers a marker from a cat destination, job label, anchor, or fabricated fallback point.
`cat-sim` owns resolution and the world-scoped reservation ledger.

## Spatial roles and types

Every task distinguishes:

- `objective`: the authoritative resource, structure, plot, route, or complete footprint concerned;
- `work_position`: the reachable, reserved tile or slot for the current work stage;
- `delivery_endpoint`: the pinned structure, stockpile, Shrine, bed/site, or recipient accepting the
  result.

Target contracts are `SpatialObjective`, `SiteRef`, `TilePoint`, `TaskFootprint`, `WorkSlot`, and
`WorldReservationLedger`. `SiteRef` supports exact tiles; anchored rectangles; canonically ordered
tile sets; building ID/anchor/canonical footprint; stockpile ID/footprint; resource-source
ID/kind/footprint; ordered route/road segment; Shrine; and village/trade endpoint. Every reference
has a stable ID, kind, lifecycle stage, visibility, and optional bounded blocked reason.

Coordinates use authoritative world tile coordinates. Rectangles store north-west anchor, width,
and height. Tile sets and routes use canonical row-major or route order as appropriate. IDs, not a
selected contact tile, own capacity and exclusivity.

## Complete task mapping

| Visible category | Authoritative objective | Work position | Pinned delivery endpoint |
|---|---|---|---|
| Hunt | Exact revealed, reachable cave entrance or hunting-source identity and its canonical footprint | Reserved reachable entrance/perimeter slot | Exact compatible stockpile or Shrine selected before dispatch |
| Fetch Water | Actual revealed water-source tile/identity | Separate reachable dry bank adjacent to that source | Exact consumer, Water Bowl, or compatible stockpile selected before dispatch |
| Fish | Canonical fish-habitat identity/footprint, not a chosen shore tile | Reserved reachable shore/bank slot | Exact fishing destination or compatible stockpile |
| Quarry | Exact revealed deposit, cave, mountain, or rock-source identity/footprint | Reserved reachable quarry-face tile | Exact material stockpile or worksite |
| Logging | Tree identity plus complete canonical 2 × 3/six-tile footprint | Reserved reachable perimeter tile | Exact stockpile or downstream worksite |
| Replant | Exact stump/planting identity and footprint | Reserved reachable planting tile | Same objective; a separate haul task pins any seed/material source |
| Building Construction | Complete planned canonical building footprint | Reserved reachable entrance, perimeter, or scaffold slot | The same construction footprint/scaffold inventory |
| Road Construction | Full canonically ordered route tile set | The reserved current next road tile | Ordered route endpoint only when a material/result delivery is needed |
| Station Work | Complete canonical building footprint | Distinct reserved station work slot | The building's exact output compartment or compatible pinned stockpile |
| Workshop Work | Complete canonical Workshop 3 × 3/nine-tile footprint | Distinct reserved reachable Workshop slot | Workshop output compartment or compatible pinned stockpile |
| Farm Work | Complete designated field/plot footprint | Reserved current plot tile/slot | Exact food/input destination selected before dispatch |
| Haul/Delivery | Exact source cargo identity and source footprint, with the route and endpoint retained as separate refs | Reserved pickup tile, then current reachable route/contact tile | Exact precommitted recipient, structure, stockpile, or Shrine |
| Stockpile Transfer | Exact source pile/compartment and cargo quantity | Reserved source interaction tile, then route contact | Exact destination pile/compartment with reserved headroom |
| Fibre Forage | Exact revealed reachable fibre-source identity and canonical footprint | Reserved reachable source interaction/perimeter tile | Exact textile input pile or compatible stockpile |
| Scout | Exact selected frontier/contact objective and canonically ordered outbound/return route; never an arbitrary radial point | Reserved current route/contact tile | Colony Shrine contact where observations become reportable knowledge |
| Expansion | Exact proposed claim/expansion footprint and its access route | Reserved reachable boundary, survey, or scaffold slot | The approved expansion footprint; any cargo uses a separate pinned stockpile/scaffold endpoint |
| Offering/Ritual | Exact resource source/cargo location and Shrine represented as separate sites | Reserved haul position, then distinct Shrine ritual slot | Exact Shrine |
| Training | Complete canonical Barracks/training footprint | Distinct reserved reachable training slot | Same Barracks; the non-cargo result is applied to the assigned cat only after work completes |
| Accounting | Canonically ordered set of exact reachable stockpile IDs/footprints for the round | Current reserved pile interaction tile | Accounting Tent/report ledger after physical return |
| Eat | Exact reserved food serving at a pile/site | Reachable dining/interaction tile | Same serving site; consumption occurs only on arrival |
| Drink | Exact reserved water serving at a bowl/pile/site (not an invented source coordinate) | Reachable drinking interaction tile | Same water site; consumption occurs only on arrival |
| Sleep | Exact reserved bed ID and Den footprint | Reachable bed interaction tile | Same bed |

If a category later needs an additional physical stage, it adds another typed site without
collapsing objective, work position, and endpoint into one coordinate.

## Hunt, water, and Workshop are hard contracts

Hunt must resolve an actual revealed hunting-source/cave identity and a reachable perimeter/contact
tile. The old arbitrary seeded/radial coordinate is forbidden. No revealed reachable source means
`Blocked(SourceUnavailable)` with no objective marker and no cat assignment.

Fetch Water always carries three distinct facts: actual water source, reachable dry bank, and
pinned destination. A water tile is not itself a walkable work position. No reachable bank blocks
before assignment.

Workshop uses the existing canonical `footprint_for(BuildingType::Workshop)` authority, currently
3 × 3. The objective contains all nine tiles in row-major order; protocol JSON reports width 3 and
height 3; the client fills/outlines nine cells. Anchor or center alone is invalid. Multiple workers
share this objective only through distinct exclusive `WorkSlot`s. No duplicated Workshop-size
constant is allowed.

## Resolution and atomic validation

Before assignment and every activation, validate as one transaction:

1. The objective is known/revealed to the colony at the permitted knowledge boundary.
2. Stable source ID, source type, and canonical footprint match the task.
3. The source/structure still exists and is not depleted or destroyed.
4. Objective tiles and work slots are valid under current world occupancy.
5. A real route exists from worker or hub to the work position.
6. A real route and exact capacity exist to the pinned delivery endpoint.
7. Required quantities, cargo capacity, tools/equipment, source capacity, work slot, endpoint
   capacity, and worker can all be reserved.

Any failure rolls back every claim. The task may remain visible in the Plans panel with a bounded
reason, but it installs no worker destination and emits no world entity when it lacks a revealed
objective.

Generic straight-line movement after pathfinding failure, silent nearest-destination recomputation,
radial/seeded objective fallback, and marking a cat busy without a valid site are forbidden.

## Reservations

Exclusive reservations cover complete tree/stump footprints, scaffold slots, road tiles, unique
station/Workshop/training/care slots, beds, and other single-owner objectives. Capacity reservations
cover hunting sources, water-bank slots, canonical fish habitats, quarry quantity, farm plots,
stockpile headroom, transport cargo, and multi-user destinations.

The ledger is world-scoped, so overlapping colonies cannot exclusively reserve the same tree,
stump, road cell, scaffold, or unique slot. Fish capacity keys use habitat ID, never shore tile.
Source, work position, delivery endpoint, delivery/cargo capacity, resources/tools, and worker commit
together or not at all.

Revalidate at dispatch, every stage transition, restart, topology/route change, source removal, and
destination change. Reservation ordering uses stable site ID, task ID, then colony ID; collection
iteration cannot decide a conflict.

## Visible runtime and failure

The persisted task stage records stable task/intent IDs, category, assigned cats, objective,
work slots, endpoint, route, footprint, progress, reservations, cargo, blocked reason, and update
tick. A typical cargo task advances resolve → reserve → travel-to-source → pickup → travel-to-work
or endpoint → work/deposit → complete. The objective and endpoint stay pinned across stages and
restart.

Route closure before pickup releases/rematches and blocks. After pickup, the cat attempts the pinned
endpoint; when unsafe/impossible it salvages exact cargo to a validated safe owned stockpile, then
blocks. Plan change, refusal, death, cancellation, or reset cannot destroy cargo. Invalid legacy
site metadata migrates to an explicitly blocked legacy task; it never becomes an objective-less
task that keeps moving with a fallback.

## Wire and rendering

`VisibleTaskSnapshot` contains task/intent ID, category, stage, assigned cats, objective `SiteRef`,
work positions/slots, endpoint, footprint, progress, reservation state, bounded reason, and last
update tick. `CatSnapshot.active_task_id` links a cat without turning its destination into authority.

The Bevy client renders solely from snapshots: complete footprint fill/outline, current work-slot
highlight, and distinct endpoint marker. Coincident markers deduplicate by semantic site/stage;
removed/stale tasks despawn; unrevealed objectives are suppressed. Workshop renders nine cells and
Logging six. Objective-less blocked intents appear only in Plans and create zero map entities.

Required tests are listed in [testing-cutover.md](testing-cutover.md), including no-source Hunt,
no-bank Water, exact habitat capacity, six-cell trees, world-level conflicts, complete construction
and roads, nine-cell Workshop, stage pinning, route failure, restart, protocol round-trips, and
snapshot-only client despawn/deduplication.

## LAI.29 world-task footprint UI contract

`LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT` is the client-side rendering contract for
`VisibleTaskSnapshot`. The Bevy client must install one `VisibleTaskMarkerPlugin` that consumes
`VisibleTaskSnapshotMarkerSource` data and resolves every marker through `StrictSiteRefMarkerResolver`.
It may render only `TaskMarkerEntity` instances keyed by the snapshot task ID, site ID, marker kind,
stage, and cell index. `NoCatDestinationAuthorityForTaskMarkers` is a hard rule: cat destinations,
job names, active-task labels, animation targets, and old radial points can decorate an already
authorized marker, but they cannot create, move, or preserve one.

The marker kinds are `TaskMarkerKind::Objective`, `TaskMarkerKind::WorkSlot`,
`TaskMarkerKind::Endpoint`, and `TaskMarkerKind::FootprintCell`. Hunt uses
`render_hunt_objective_from_revealed_hunting_source` and emits `HuntObjectiveCaveOrSourceMarker`
only for the actual revealed reachable cave or hunting-source identity from the snapshot. Fetch
Water uses `render_fetch_water_source_bank_endpoint` and separately emits `FetchWaterSourceMarker`,
`FetchWaterDryBankWorkMarker`, and `FetchWaterPinnedDeliveryEndpointMarker`; the
`WaterSourceIsNotWalkableWorkPosition` guard prevents a water tile from doubling as the work marker.
`BlockedOrUnreachableSiteSuppressesWorldMarker` means blocked, missing, or unreachable authoritative
sites produce no world entity.

Workshop and tree tasks render complete canonical objective footprints. Workshop uses
`render_workshop_three_by_three_objective_cells`, `WorkshopObjectiveNineRowMajorCells`,
`WorkshopDistinctWorkSlotMarker`, and `WorkshopDistinctDeliveryEndpointMarker`; it obtains the
canonical 3 x 3 descriptor from the protocol snapshot and the simulation footprint authority, with
`NoDuplicatedWorkshopSizeConstant` forbidding a client-local Workshop dimension. Tree tasks use
`render_tree_six_canonical_footprint_cells` and `TreeObjectiveSixCanonicalCells`. Every cell carries a
`CanonicalFootprintCellIndex`, assigned in row-major order for rectangles and the authoritative
ordered-cell list for non-rectangular sites.

Runtime lifecycle is snapshot-ID keyed. `TaskSnapshotIdMarkerKey`,
`DedupeVisibleTaskMarkerBySnapshotId`, `UpdateVisibleTaskMarkerFromSnapshotVersion`,
`DespawnRemovedVisibleTaskMarkers`, `NoStaleTaskMarkerReuse`, `NoDuplicateCoincidentTaskMarker`,
`SemanticSiteStageDedupeKey`, and `VisibleTaskRemovalEvent` are required symbols for the production
slice. A later snapshot with the same key updates the existing marker; a missing snapshot despawns
it; coincident objective/work/endpoint sites dedupe only when their semantic site and stage match.

Redaction and colony filters run before entity creation. `RedactedVisibleTaskNoMarker`,
`ObjectiveLessBlockedTaskNoMapEntity`, `MissingSiteRefNoMarker`, `BlockedSiteRefNoMarker`,
`ForeignColonyVisibleTaskNoMarker`, `SelectedColonyTaskMarkerFilter`,
`MultiColonyTaskMarkerIsolation`, and `ReportSafeTaskMarkerVisibility` reject redacted, blocked,
missing-site, or non-selected-colony tasks without revealing whether the underlying private site
exists. Tooltips use `TaskMarkerReportSafeTooltip` and `TaskMarkerTooltipRedactionGuard`, with
`NoHiddenStockTooltipField`, `NoExactRegenerationBelowLevelFourTooltip`,
`NoPrivateBeliefOrPlanTooltip`, `NoRadialTaskMarkerFallback`, `NoGenericTaskDestinationFallback`,
and `NoClientSideSiteGuessing` as explicit leak/fallback guards.

Accessible IDs and labels must be stable enough for Playwright and the visible-browser gate:
`TASK_MARKER_OBJECTIVE_TEST_ID` renders as `task-marker:{task_id}:objective:{site_id}`;
`TASK_MARKER_WORK_SLOT_TEST_ID` renders as `task-marker:{task_id}:work:{slot_id}`;
`TASK_MARKER_ENDPOINT_TEST_ID` renders as `task-marker:{task_id}:endpoint:{site_id}`; and
`TASK_MARKER_CELL_TEST_ID` renders as `task-marker:{task_id}:cell:{index}:{site_id}`.
`ACCESSIBLE_TASK_OBJECTIVE_LABEL`, `ACCESSIBLE_TASK_WORK_SLOT_LABEL`, and
`ACCESSIBLE_TASK_ENDPOINT_LABEL` name the task category, marker role, report-safe site kind, and
bounded status, not hidden stock, exact regeneration, private beliefs, private plans, or auth
material. `RouteContactMarkerIsNotDeliveryEndpoint` keeps route/contact highlights visually and
semantically distinct from pinned delivery endpoints.

The marker renderer supports the same camera constraints as the map. `TaskMarkerSupportedZoomRange`
defines the tested zoom interval; `TaskMarkerViewportCullingKeepsAuthoritativeIds` and
`TaskMarkerScreenBoundsGuard` require viewport culling, hover hitboxes, and labels to preserve
stable marker IDs rather than re-keying by screen position. The production owner must publish
`PLAYWRIGHT_TASK_MARKER_LOCATOR_MANIFEST` plus visible-browser checkpoints
`VISIBLE_BROWSER_CHECKPOINT_LAI29_WORKSHOP_FOOTPRINT`,
`VISIBLE_BROWSER_CHECKPOINT_LAI29_HUNT_WATER`,
`VISIBLE_BROWSER_CHECKPOINT_LAI29_DESPAWN_DEDUPE`, and
`VISIBLE_BROWSER_CHECKPOINT_LAI29_REDACTION`.
