# Historical LAI.0–34 Content Authoring Guide

> This file preserves the first-cutover authoring workflow as evidence. Its Shrine/Favor, fixed
> 531-study, semantic-migration, direct-action, and older browser assumptions are superseded. Use
> [`extending-the-system.md`](extending-the-system.md) for the current 21 copyable Plan 1+2 recipes,
> [`hole-research-progression.md`](hole-research-progression.md) for progression, and
> [`integrated-implementation-map.md`](integrated-implementation-map.md) for current runtime/wire/
> persistence/server/client ownership. Do not implement the archived rules below.

This guide is for contributors who add a new building, task, resource, source,
planner goal, officer report, Shrine package, study, cat-care rule, diplomatic
relationship, trade route, snapshot field, action, persistence row, or client
surface to the Leader Intelligence overhaul. It is a companion to
[extending-the-system.md](extending-the-system.md), not a replacement for it.
The domain documents remain authoritative:

- [planner-and-beliefs.md](planner-and-beliefs.md) defines postures, reports,
  fallibility, cadence, officer authority, and deterministic planning.
- [spatial-task-contract.md](spatial-task-contract.md) defines legal anchors,
  objective/work/endpoint roles, reservations, and footprints.
- [hole-research-progression.md](hole-research-progression.md) defines physical Hole
  offerings, Favor, the 531-study manifest, scholars, and divine boosts.
- [cats-and-care.md](cats-and-care.md) defines attributes, personality, stress,
  anatomy, injuries, treatments, and prosthetics.
- [diplomacy-trade.md](diplomacy-trade.md) defines consent, valuation, escrow,
  physical routes, and recovery.
- [wire-persistence-ui.md](wire-persistence-ui.md) defines report-safe wire,
  SQLite, routing, and UI boundaries.
- [testing-cutover.md](testing-cutover.md) defines focused, restart, campaign,
  and browser acceptance gates.

When this guide and a domain contract appear to disagree, stop and resolve the
domain contract first. Do not encode an interpretation in a new registry or
client fallback.

## The authoring rule

Every new content item is one cross-layer transaction. A pull request is not
complete because a catalog entry renders. It is complete only when the item has
one stable identity, one authority owner, legal physical semantics, a report
projection, persistence behavior, authorization behavior, client behavior, and
tests that prove deterministic restart and redaction.

The required flow is:

1. Define the behavior and invariants in the relevant domain document.
2. Allocate stable IDs and decide ownership before editing code.
3. Implement the pure simulation/catalog leaf and focused tests.
4. Add persistence defaults, migration, validation, and quarantine behavior.
5. Add strict protocol snapshot/action fields and round-trip tests.
6. Add server routing, authentication, authorization, idempotency, and redaction.
7. Add client projection, entities, controls, accessibility IDs, and stale states.
8. Add system, campaign, restart, Playwright, and visible-browser evidence.
9. Run the definition-of-done matrix and record evidence on the appropriate card.

Never create a client-only content registry, a second Workshop size constant, a
second Favor balance, a display-name ID, or a hidden-truth debug field.

## File and module map

The current tree is split by authority. Roots orchestrate; focused leaves own
behavior.

| Concern | Current or target owner | Content-author responsibility |
|---|---|---|
| Building/resource/task truth | `crates/cat-sim/src/spatial_tasks.rs`, `spatial_resolver.rs`, `task_runtime.rs`, `tasks.rs`, and the relevant domain leaf | Add one canonical descriptor, legal anchors, stages, costs, reservations, and deterministic ordering. |
| World tick integration | `crates/cat-sim/src/world_tick.rs` | Add only the ordered phase hook owned by the cutover; do not add a shadow tick or second mutation path. |
| Planner and beliefs | `leader_planner.rs`, `leader_director.rs`, `leader_ai_runtime.rs`, `beliefs.rs`, `officer_requests.rs` | Add posture/domain score inputs and report-safe evidence, never hidden stock or exact regeneration. |
| Shrine/Favor | `shrine_offerings.rs`, `favor.rs`, `shrine.rs` | Add physical package descriptors and exactly-once ledger effects. |
| Research | `research_manifest.rs`, `research_purchase.rs`, `scholar_research.rs`, `research_catalog.rs` | Add a validated manifest node or track stage; preserve 531 manifest count and quota semantics. |
| Cats and care | `injuries.rs`, `prosthetics.rs`, `cat_stress.rs`, `cat_willingness.rs`, `anatomy.rs` | Add bounded report-safe state and physical treatment/fitting/repair transitions. |
| Diplomacy/trade | `diplomacy.rs`, `trade_valuation.rs`, `autonomous_trade.rs`, `village_trade_routes.rs` | Add consent and physical cargo/route transitions, not abstract currency exchange. |
| Snapshot DTO | `crates/cat-protocol/src/lai24_snapshot.rs` | Add strict report-safe fields, bounds, validation, and round-trip fixtures. |
| Action DTO | `crates/cat-protocol/src/lai25_action.rs` | Add tagged payload, expected versions, typed conflict, and decode bounds. |
| SQLite | `crates/cat-server/src/leader_ai_persistence.rs`, `persistence.rs` | Add schema/version, transactional save/load, old-row defaults, rollback, quarantine, and equality tests. |
| Server route | `crates/cat-server/src/leader_ai_action_routing.rs`, `leader_ai_snapshot_projection.rs`, `main.rs` | Keep one pipeline: version preflight, auth, rate limit, ownership, authority, versions, replay, preconditions, atomic mutation, persistence, projection. |
| Client UI | `crates/cat-client/src/leader_ai_ui/` and `leader_ai_live.rs` | Consume only snapshots, send real envelopes, preserve focus, and despawn stale entities. |
| Client marker geometry | `leader_ai_ui/task_footprints.rs`, `live_render.rs` | Render authoritative SiteRef cells, slots, endpoints, and routes; never infer coordinates. |
| Browser evidence | `docs/leader-ai-overhaul/browser-playtests/` | Extend paired Playwright and visible-browser checkpoints and immutable evidence schema. |

Do not edit `cat-server`, `cat-protocol`, `cat-sim`, `world_tick`, persistence,
or campaign files from a client-only card. Conversely, do not make protocol or
server fields merely to make a client mock convenient.

## Stable identity and registries

Allocate IDs before implementation. IDs are permanent semantic keys, not labels.

```text
building_kind: kiln
site:resource:clay:marsh-east
task:kiln:construction:001
intent:kiln:construction:001
report:forester:clay:001
offering:4_clay
study:craft:kiln:stage_01
action:player:kiln:purchase:001
snapshot:colony:one:state:42
```

Use lower-case ASCII snake-case segments separated by `:`. A display name may
change and a translated label must never change identity. IDs must be bounded by
the protocol's stable-ID type. Catalog validation must reject empty, oversized,
duplicate, or malformed IDs before any simulation state is mutated.

Each closed registry has one owner and one deterministic order. Prefer a stable
`ALL` list for a small closed enum or sort descriptors by semantic ID at every
boundary. If a collection is a `HashMap`, sort its output before score,
serialization, UI, or test comparison. Never rely on SQL row order, hash order,
or insertion order as a rule.

## Workshop and footprint authoring

### The exact Workshop 3x3 example

The canonical Workshop objective is a 3 by 3 rectangle anchored at its
north-west tile. Its nine cells are row-major: complete row `y`, then move east
through `x`.

For anchor `(10, 20)` the only valid ordered cells are:

```text
index 0: (10, 20)    index 1: (11, 20)    index 2: (12, 20)
index 3: (10, 21)    index 4: (11, 21)    index 5: (12, 21)
index 6: (10, 22)    index 7: (11, 22)    index 8: (12, 22)
```

The report-safe protocol representation is conceptually:

```json
{
  "kind": "building_footprint",
  "site": {
    "siteId": "site:workshop:001",
    "visibility": "visible",
    "lifecycleStage": "reserved"
  },
  "buildingId": "building:workshop:001",
  "buildingKind": "workshop",
  "anchor": {"x": 10, "y": 20},
  "width": 3,
  "height": 3,
  "orderedTiles": [
    {"x": 10, "y": 20}, {"x": 11, "y": 20}, {"x": 12, "y": 20},
    {"x": 10, "y": 21}, {"x": 11, "y": 21}, {"x": 12, "y": 21},
    {"x": 10, "y": 22}, {"x": 11, "y": 22}, {"x": 12, "y": 22}
  ]
}
```

The simulation, reservation ledger, persistence, snapshot DTO, server redactor,
and client marker renderer must all consume this same descriptor. A client must
not reconstruct the nine cells from `buildingKind`, width, and anchor; the
authoritative `orderedTiles` list is the report-safe source. There is no
one-cell fallback if the list is missing or malformed.

### Arbitrary multi-tile footprints

For a new building or resource source, decide which of these legal shapes it is:

- `Tile`: one exact objective tile.
- `AnchoredRect`: positive width and height, with a complete row-major rectangle.
- `OrderedTileSet`: an explicit finite set for an irregular footprint.
- `BuildingFootprint`: a typed building plus its complete ordered cells.
- `ResourceSource` or `StockpileFootprint`: source/stockpile identity plus all cells.
- `OrderedRoute`: a route's ordered contacts, separate from its endpoint.

Example irregular 5-cell drying rack anchored at `(4, 8)`:

```text
orderedTiles = [(4,8), (5,8), (6,8), (4,9), (6,9)]
```

This is not an anchored 3 by 2 rectangle because `(5,9)` is absent. It must be
an explicit ordered tile set, with a stable semantic meaning for each cell if a
work slot or delivery role depends on it. The resolver must validate reachability,
terrain, overlap, and ownership before the task exists.

Footprint checklist:

- [ ] The canonical descriptor has a stable type ID and no duplicate cells.
- [ ] Width and height are positive and within protocol bounds.
- [ ] Rectangles are exact row-major lists; irregular shapes use ordered sets.
- [ ] Objective cells, work slot, route, and delivery endpoint are distinct roles.
- [ ] Every cell is checked against terrain, occupancy, fog, and cross-colony claims.
- [ ] Expansion or rules changes have a migration and collision rollback plan.
- [ ] The client renders every authorized cell and no guessed cell.
- [ ] Tests assert cardinality, order, duplicate rejection, restart, and redaction.

## Task types and legal world anchors

A task category is not a coordinate convention. It is a contract that names the
objective, work slot, endpoint, route, cargo, and stage transitions.

| Task | Objective anchor | Work role | Endpoint/route rule |
|---|---|---|---|
| Hunt | Revealed reachable `HuntSource` with actual cave/source ID and source tile | Reachable hunt work tile/slot | Physical haul and pinned destination when applicable |
| Fetch Water | `WaterSourceAndBank` with separate water source and dry bank tiles | Dry bank work slot, never water geometry | Pinned delivery endpoint, source and endpoint are distinct |
| Workshop | Full canonical `BuildingFootprint`; Workshop is exactly nine 3x3 cells | Reserved reachable work slot | Pinned station/storage endpoint and route |
| Tree harvest | Six canonical tree cells when the source is revealed | Reachable trunk/work tile | Pinned stockpile or station endpoint |
| Road/route | `OrderedRoute` cells in semantic travel order | Route work slot/segment | Endpoint is not a route contact marker |
| Shrine offering | Shrine SiteRef and exact Shrine endpoint | Haul and ritual roles | Physical cargo reaches the Shrine endpoint before credit |
| Trade | Trade endpoint and ordered route | Hauler/cargo reservation | Partner endpoint, escrow, delivery, and recovery route |

Legal anchor rules:

1. Resolve from authoritative terrain/site state, not a task name or radial search.
2. A hidden, redacted, missing, blocked, or unreachable source creates a typed
   omission/block and zero world marker entities.
3. A work slot must be reachable and reserved; an objective tile is not an
   implicit work tile.
4. An endpoint must be pinned before cargo dispatch; a route contact is not an
   endpoint.
5. Cross-colony exclusive reservations use the world-scoped ledger and stable
   conflict ordering.

Worked Fetch Water example:

```text
objective: water source (31, 7), site:water:river-north
work slot: dry bank (32, 7), slot:river-north-bank
endpoint: village stores (4, 4), site:village:stores
route: ordered path [bank, bridge, lane, stores]
```

The client must show three markers and may show route cells: source, dry bank,
and delivery endpoint. It must not show a single generic water pin.

## Resources, sources, and physical conservation

Authoring a resource requires both an economic descriptor and a physical source
descriptor. Define:

- `resource_kind`, `carrying_kind`, unit scale, stack/capacity bounds, and storage;
- source ID/type, complete footprint, discovery requirements, depletion/regeneration;
- legal work tile/slot, route restrictions, endpoint types, and destination headroom;
- production, research, Shrine, trade, and treatment consumers;
- report estimate bounds, confidence, provenance, expiry, and omission behavior;
- cargo identity, reservation keys, cancellation salvage, and exactly-once effects.

Never project hidden stock or exact regeneration to the client. A report may say
“wood estimate 5-8 logs” with provenance; it must not expose the authoritative
`wood_count` merely for the client to hide. The only exact player-facing currency
is the owning colony's Favor ledger, which is not inventory, cargo, escrow, or
research points.

Resource invariant matrix:

| Transition | Required invariant |
|---|---|
| Reserve source | Exact source identity/quantity or bounded capacity is claimed atomically. |
| Pick up cargo | Every item identity has one owner/location and a task reference. |
| Cancel before pickup | Source reservation releases; no phantom cargo exists. |
| Cancel after pickup | Cargo is delivered or physically salvaged exactly once. |
| Restart | Cargo, reservation, stage, and endpoint are byte-equal after reload. |
| Cross-colony contention | One deterministic winner; loser has opaque bounded conflict and no mutation. |
| Report projection | Hidden source quantity/regeneration never enters DTO, log, tooltip, or error. |

## Leader, officer domains, and fallibility

The Leader is a colony-wide strategic authority and founding fallback. Officers
are domain specialists: Steward, Accountant, Forester, Farmer, Captain, Loremaster,
and Cloth Leader. A new domain must declare:

- which posture phases it serves and which emergency override can preempt it;
- officer role/domain, authority limits, report level, expertise/cadence, and vacancy;
- belief subjects, bounds, confidence, provenance, contradictions, expiry, and
  omission rate;
- request type, bounded budget, actor, target, dependency, and succession behavior;
- which fields are player-visible, officer-visible, owner-only, or never projected.

Leaders and officers reason over persisted reports, not hidden simulation truth.
An explanation should identify the report IDs and bounded rationale, not claim an
exact unseen stock count. Weak evidence can produce an omission, wrong candidate,
or typed blocked intent; that imperfection is part of the design. The client must
not “correct” a poor plan by looking up authoritative quantities.

Knowledge checklist:

- [ ] Report level 1-3 has regeneration unavailable, not an exact value.
- [ ] Level 4+ exposes only bounded/ranged regeneration with provenance.
- [ ] Confidence and age affect eligibility and scoring deterministically.
- [ ] Contradictions and expiry have a documented replacement/omission rule.
- [ ] Officer requests stay within domain; Leader can cover founding essentials.
- [ ] Player nudges change policy within bounded ±1500 basis points only.
- [ ] Omission rates are deterministic from the named keyed RNG stream.
- [ ] Explanations never leak hidden stock, private plans, or another colony.

## Standing orders and player policy

Standing orders are persistent player policy, distinct from temporary plan nudges.
Define `order_kind`, domain, optional target, bounded instruction text, priority
basis points, expiry, administration slot cost, and expected standing-order
version. Create/edit/delete must use authenticated LAI.25 envelopes and server
authority; the client must preserve an unsent draft through refresh and show typed
disabled/conflict feedback when the slot limit or domain is unavailable.

Worked standing-order payload intent:

```text
orderKind = gather
domain = workforce
targetId = site:resource:wood:west (optional)
instruction = "Prefer reachable reported wood sources"
priorityBasisPoints = 5000
expiresAtMs = 172800000
```

The instruction changes prioritization; it does not bypass knowledge, pathfinding,
worker willingness, reservations, or physical capacity. A server rejection does
not partially create an order, and duplicate replay returns the original result.

## Shrine offerings and Favor valuation

The Shrine demand is endless and physical. Add one of the four exact one-Favor
packages only through the canonical offering registry. For a new package, document
its physical input identity/quantity, replacement-cost valuation, source reports,
haul/ritual stages, endpoint, cancellation salvage, and exactly-once Favor event.

Rules that must not be invented:

- no cooldown, tithe, completion gate, supernatural penalty, or generic currency;
- one active physical pipeline per Shrine;
- package selection uses belief-based replacement cost and can omit when evidence
  is insufficient or no safe physical source exists;
- Favor ledger balances never become negative and each debit/credit is idempotent;
- physical completion precedes the exact ledger credit.

The player UI may show exact micro-Favor balance and ledger history for the owning
colony. It may show the leader/officer package rationale and bounded valuation, but
not hidden stock used to calculate it.

Offering test matrix:

| Case | Expected evidence |
|---|---|
| Poor report | Omission/block reason, no guessed source or generic package. |
| Good report | Lowest reported replacement-cost package, source provenance shown. |
| Concurrent Shrine request | One active pipeline; second request is bounded/no debit. |
| Haul cancellation | Cargo salvaged and no Favor credit. |
| Ritual restart | Same stage and carrier/ritualist; credit exactly once. |
| Duplicate credit | Ledger remains nonnegative and unchanged after replay. |

## Research nodes, scholars, and automatic purchases

The manifest has exactly 531 validated study nodes. A node needs a stable study
ID, display label, prerequisites, exact Favor price, prepared price if applicable,
track/stage, effect descriptor, and report-safe availability/block reason. Four
scholar tracks each have 11 stages; preparation consumes Insight and can receive
the player preparation discount defined by the domain contract.

Automatic purchases belong to the colony quota. Document quota window, used/limit,
purchase ordering, Favor debit, preparation selection, and rejection behavior.
Use stable semantic ordering `(frontier priority, study_id)`, never manifest file
line order alone. Later research must not rewrite an active boost's committed
duration/economy stages.

Research acceptance checklist:

- [ ] Manifest count remains 531 and all IDs/prerequisites validate.
- [ ] Purchased, prepared, owned, and in-progress states are distinct.
- [ ] Quota is colony-owned, bounded, and persisted across restart.
- [ ] Favor debit is atomic and idempotent with the action ID.
- [ ] Scholar preparation has track/stage/Insight/prepared-price evidence.
- [ ] New effects bind to existing manifest IDs; no shadow research IDs.
- [ ] Client controls include expected research/scholar versions and no hidden truth.

## Injuries, treatment, and prosthetics

Every cat retains stable identity and report-safe anatomy: four paws, two eyes,
and a tail, even when injured or fitted. Add an injury kind, severity bound,
sustained tick, treatment stage/site/task, and refusal/willingness reason. Add a
prosthetic with stable item identity, body-part side, type, restoration cap,
durability, wear, fit task, repair task, workshop, and input reservation.

Treatment, fit, remove, and repair are authenticated expected-version actions.
They must conserve item/cargo identity and never be inferred from hidden health.
The client can display bounded eligibility and disabled reasons; it cannot compute
regeneration, willingness, or treatment outcome locally. A stale response keeps
the selected cat and draft while replacing the report from the server.

## Diplomacy and physical trade

Relationships are Neutral, Friendly, Allied, or Blocked. Friendly/Allied consent
requires both sides where specified; Blocked is immediate and authoritative.
Diplomatic controls include relationship proposal/change, alliance approval, and
block with bounded public reason. Authorization is actor/domain/consent checked.

Trade is physical: proposal, report-based valuation range, mutual consent, escrow,
hauler/cargo reservation, ordered route, endpoint, delivery, cancellation, and
recovery. Define partner colony ID, contract ID/version, cargo IDs, valuation report
IDs/confidence, route cells, reservations, stage, and bounded failure/recovery.
Never implement trade as a silent numeric balance transfer. Cross-colony state is
redacted to the selected colony's authorized view.

## Snapshot and action protocol fields

Snapshot additions belong in `lai24_snapshot.rs` and must remain strict serde:

1. Add a report-safe field with a stable name and bounded type.
2. Add validation for range, collection size, duplicate IDs, references, and
   cross-field consistency.
3. Add a nonempty fixture and a malformed fixture that fails closed.
4. Add round-trip and permutation-twin tests.
5. Decide selected-colony ownership and server redaction before client code.

Action additions belong in `lai25_action.rs`:

1. Add one tagged payload variant with typed bounded IDs/amounts/text.
2. Require every affected domain version in `ExpectedStateVersions`.
3. Reject protocol incompatibility before nested decode.
4. Add authority, ownership, precondition, reservation, Favor, and consent
   conflict variants only when they are report-safe and bounded.
5. Add accepted/rejected/duplicate/stale refresh fixtures and exact replay tests.

The server pipeline is always:

```text
protocol preflight
  -> authenticated session/HMAC
  -> rate limit
  -> selected-colony ownership
  -> actor/action authority and consent
  -> expected versions
  -> bounded idempotency replay
  -> current preconditions and reservations
  -> atomic simulation mutation
  -> transactional persistence
  -> server-side redacted snapshot refresh
```

An unauthorized, malformed, stale, or foreign-colony request must not reveal
whether a private object exists through different errors or timing-sensitive work.

## SQLite migration and versioning

Every persisted addition declares:

- aggregate schema/persistence version and rules version;
- per-colony row ownership and canonical serialized ordering;
- fresh default at the one permitted migration boundary;
- old-row conversion, including exact once-only legacy currency/Favor rules;
- malformed required-row rollback/quarantine metadata and bounded diagnostics;
- unknown newer-version downgrade rejection;
- transaction order and save/reload equality fixture.

Save all related aggregate state in one transaction. Validate fully before replacing
the in-memory aggregate. A malformed task site must not leave a reservation, cargo,
Favor debit, or cat assignment partially applied. A migration rerun must be a no-op.
Never hand-edit schema versions to make a test pass; use the project's migration
helpers and record the resulting marker.

## Client panels, markers, and interactions

Client modules consume only `LeaderAiSnapshotEnvelope` and action responses:

- Plans: top eight stable rows, rationale/range/confidence, nudge/dismiss, standing orders.
- Task map: exact Hunt/Water/Workshop/tree/road markers, slots, endpoints, routes,
  assigned cats, stage, dedupe, despawn, and fog/redaction suppression.
- Cat Care: identity, bounded attributes/personality/stress/refusal, anatomy,
  injuries, prosthetics, care tasks, treatment and fit/repair controls.
- Progression: Shrine pipeline, exact Favor ledger, research/quota/Insight/scholars,
  four player-only boosts, diplomacy consent, physical trade.

Every control either sends a real authenticated expected-version envelope or is
explicitly disabled with a typed report-safe reason. Stable test IDs and accessible
labels use semantic IDs, not display text. Focused state preserves selected entity,
draft, and panel tab after stale refresh. Reconnect keeps the last snapshot marked
stale, clears unsafe pending mutation assumptions, and reconciles from the next
authoritative snapshot. `UPDATE_REQUIRED` blocks mutation and exposes only the
bounded compatibility message.

Marker rules are absolute: no generic pin, radial fallback, guessed coordinate,
private site, redacted objective, blocked missing site, stale duplicate, or hidden
regeneration tooltip. Workshop has nine cells; Water has source, dry bank, and
endpoint; Hunt has the actual revealed cave/source.

## Deterministic IDs and RNG

Use keyed deterministic RNG only in `cat-sim` rules. A new random decision gets a
named fork keyed by world seed, colony ID, subject/content ID, purpose, and epoch.
The fork must be independent of unrelated candidate count or collection order.
`cat-sim` must not use wall-clock, thread timing, `rand`, or external AI.

Action and snapshot IDs are deterministic semantic IDs where replay requires a
stable identity. UI test IDs are deterministic projections of those IDs. A client
must not generate an ID from array index, screen position, or current display label.

Permutation twin requirement:

```text
same seed + same semantic inputs + different input collection order
  => identical canonical state, IDs, snapshot JSON, and action results
```

## Test and evidence matrix

| Layer | Minimum tests for new content |
|---|---|
| Sim/catalog | Registry uniqueness, bounds, legal anchors, scoring, RNG/order twin, task stages, conservation, cancellation, refusal, and restart. |
| Protocol | Strict serde round-trip, unknown field/version/variant rejection, bounds, references, permutation JSON, and redaction fixture. |
| Persistence | Fresh default, migration rerun, exact legacy conversion once, malformed rollback/quarantine, newer-version rejection, multi-colony isolation, save/reload byte equality. |
| Server | Pipeline order, auth/HMAC, ownership, authority/consent, rate limit, stale expected version, duplicate replay, atomic mutation, opaque errors, and snapshot redaction. |
| Client headless | Entity cardinality, stable IDs, selection, focus/draft retention, button envelope exactness, disabled authority, accepted/rejected/duplicate/update-required, reconnect, and despawn. |
| Campaign | Required seed set, 30-day cadence, no-starvation, Favor/research/quota, spatial reservation, privacy, replay, restart, partition, and resource ceilings. |
| Playwright | Visible controls through shipped UI, locator contracts, action response, stale reload, screenshot, console/network acceptance, and cleanup. |
| Visible browser | Independent computer-use observation paired to each Playwright checkpoint, with screenshot and DevTools/network evidence. |

For a focused Rust change, use the narrow command first:

```bash
cargo test -p cat-sim --test <focused_test> --no-fail-fast
cargo nextest run -p cat-sim --test <focused_test> --no-fail-fast
cargo test -p cat-protocol --test <focused_test> --no-fail-fast
cargo test -p cat-server --test <focused_test> --no-fail-fast
cargo test -p cat-client --test <focused_test> --no-fail-fast
cargo clippy -p <touched-crate> --all-targets -- -D warnings
cargo fmt -p <touched-crate> -- --check
git diff --check
```

Then run the applicable [smoke profile](../../TESTING.md) and browser journey.
If a shared compile blocker prevents a gate, record the exact command, first
external error, and affected owner; do not weaken the focused assertion or fake
the missing API.

## Playwright and visible-browser journey

Add a checkpoint to
[browser-playtests/playwright-scenario-manifest.md](browser-playtests/playwright-scenario-manifest.md)
and pair it with the visible-browser layer. The checkpoint must name:

- deterministic seed/preconditions and selected colony;
- accessible locator/test ID and permitted user action;
- expected authoritative IDs, tick/version, and report-safe visible result;
- forbidden hidden values and privacy assertions;
- screenshot name, viewport, console/network acceptance, restart linkage, and cleanup.

Never inject DOM state, call private endpoints, bypass auth, manufacture Favor or
inventory, or skip time without a documented shipped control. A new Workshop,
task, source, panel, or action needs a fresh checkpoint and an extension note in
the browser README.

## Common failure modes

| Failure | Why it is wrong | Correct repair |
|---|---|---|
| Client derives a Workshop footprint from width | Duplicates authority and can hide malformed data | Render the validated ordered nine-cell snapshot list. |
| Water uses one generic pin | Erases source/bank/endpoint semantics | Emit three distinct authoritative roles. |
| Planner reads stock directly | Makes Leader omniscient | Feed persisted bounded reports and provenance. |
| Favor is mirrored in inventory | Enables double spend and ambiguous UI | Read one exact Favor ledger projection. |
| Study ID is an array index | Manifest reordering changes identity | Use stable manifest study ID. |
| Trade swaps numbers | Violates physical cargo conservation | Reserve, haul, escrow, deliver, or salvage items. |
| Rejected action changes UI state | Client simulated an outcome | Keep pending/stale feedback until snapshot reconciliation. |
| New SQL column defaults silently | Can erase or partially migrate state | Version, validate, transaction, rollback/quarantine. |
| Different unauthorized errors reveal object existence | Leaks private colony state | Return bounded opaque errors after common pipeline checks. |
| Browser test seeds DOM/local storage | Does not prove shipped behavior | Use authenticated visible controls and record trace/evidence. |
| Random choice uses collection order | Partition/restart can diverge | Sort semantic IDs and use a keyed RNG fork. |

## Adding a new player action or mutable panel

Use this recipe in addition to the domain-specific checklist:

1. Add a strict LAI.25 payload and bounded DTO fields. Keep player/action ID alphabets strict even
   when the target's canonical runtime ID uses typed `|` components.
2. Add an exact `CurrentVersionHint` lane to LAI.24 `actionVersions`. The aggregate `stateVersion`
   is for snapshot replacement and must not be used as the mutation CAS token.
3. Project that lane from the same canonical state fingerprint checked by the server. Carry it in
   the client envelope and test accepted, stale, duplicate, unrelated-change, and restart cases.
4. If the public entity ID can exceed wire bounds, project its deterministic `wire:v1` alias and
   resolve the action with shared `stable_id_matches`; never persist the alias as domain authority.
5. Generate the action idempotency ID with the shared stable helper. It validates an ordinary
   candidate and deterministically hashes a candidate containing a disallowed delimiter or excessive
   length.
6. Keep policy lifetime separate from mutation concurrency. Plan nudge/dismiss actions carry the
   projected `planningEpoch`, not the plan queue version.
7. After authentication and every action response, publish an immediate authoritative snapshot so
   fixed-tick fixtures and quiet worlds cannot strand a successful UI mutation.
8. Add the production Bevy control, bounded feedback, fixed-canvas checkpoint when required, and a
   Playwright action/response/snapshot trace.

See [wire-persistence-ui.md](wire-persistence-ui.md#exact-action-concurrency-and-public-identity) and
[diagnostics-and-debugging.md](diagnostics-and-debugging.md).

## Definition of done

An extension is ready for review only when every applicable box is checked:

- [ ] Domain document, stable IDs, owner, invariants, and migration decision are recorded.
- [ ] Canonical sim/catalog implementation has bounds, legal anchors, reservations, and deterministic order.
- [ ] Hidden truth and regeneration rules are explicitly projected or omitted.
- [ ] Snapshot/action DTOs have strict versions, bounds, references, and round-trip tests.
- [ ] SQLite save/load is transactional, versioned, idempotent, isolated, and restart-equal.
- [ ] Server pipeline authenticates, authorizes, checks versions/replay, mutates atomically, and redacts server-side.
- [ ] Client panels/markers use stable IDs, accessible labels, focus preservation, and no fallback geometry.
- [ ] Every enabled control sends a real action; unavailable controls are disabled with bounded reasons.
- [ ] Cargo/Nextest, strict Clippy, rustfmt, diff, and relevant smoke checks are recorded.
- [ ] Focused system/restart/campaign tests cover order twins, privacy, conservation, and idempotency.
- [ ] Playwright and independent visible-browser checkpoints are paired and evidence artifacts are immutable.
- [ ] Rollback, compatibility, old-save, old-client, and `UPDATE_REQUIRED` behavior are tested.
- [ ] The relevant board evidence is additive and states any remaining parent/cutover work honestly.

Do not mark the parent LAI card complete for a content-only leaf. Completion also
requires the production integration and the acceptance gates owned by the parent.
