# Authority consolidation audit

Status: maintained implementation audit for LAI.55–LAI.70.

This document records why the pure Plan 2 leaves are foundations rather than completed gameplay.
It is additive to both exact plans and both boards. The acceptance rule is stronger than “an API
exists”: a domain is integrated only when authoritative runtime entities own its state, the world
tick invokes it once in the protected phase order, all physical changes use the shared ledgers and
tasks, report-safe explanations reach the player, restart is exact, and the replaced legacy or
parallel authority is retired in the same cutover.

> **Cutover correction — 2026-07-25:** The sections from “Historical pre-cutover contradictions”
> through “Dependency-safe order” preserve the implementation audit taken before the LAI.63
> composition-root cutover. They are history, not the current authority map. In particular, live
> research is no longer Shrine/Favor, and capability, family, governance, Research, Hole/divine,
> storage, and trade authorities are now persisted fields of `LeaderAiRuntimeState`. The dated
> “Current live-authority audit” at the end of this file is authoritative for remaining work.

## Historical pre-cutover contradictions (superseded 2026-07-25)

1. The live research phase still invokes Shrine/Favor and automatic legacy purchasing. The
   Notes/Void God and Leader lanes exist only as additive leaves.
2. The canonical construction catalog now defines every retained building and level, but live
   construction still accepts caller-supplied bills instead of resolving and reserving those
   catalog requirements through one physical inventory authority.
3. Physical storage describes containers and slots without replacing the colony inventory used by
   production, hauling, spoilage, construction, barter, and recovery.
4. Moneyless barter and personal stances are separate from the live diplomacy and autonomous trade
   authorities.
5. Cat capabilities, anatomy adapters, family state, and governance state are not yet fields and
   lifecycle behavior of the authoritative cats.

These contradictions are intentionally visible. Temporary adapters may help one owned cutover, but
they may not survive LAI.70 as a second planner, catalog, currency, inventory, diplomacy model,
research ledger, or mutation path.

## LAI.55: cat capabilities, XP, affinity, refusal, and anatomy

### Foundation present

- `skill_catalog.rs` owns the data-defined skill and activity vocabulary.
- `cat_capabilities.rs` implements ten attributes, skill XP/level/Mastery arithmetic, declared XP
  helpers, affinity ranking, refusal, and anatomy eligibility.
- `cat_capability_authority.rs` attaches those facts to stable real-cat IDs, applies bounded
  idempotent successful-work and per-haul-leg receipts, preserves zero XP for failed/refused/
  unassigned work, records completed office duty separately from cross-training, computes exact
  assignment keys, and reads the existing anatomy/prosthetic authorities.
- Existing `anatomy.rs` and `prosthetics.rs` already own detailed physical body state.

### Missing live behavior

- The live `CatRuntimeState` does not yet store and persist the canonical capability authority.
- Productive task completion does not resolve a catalog activity and emit the declared primary,
  secondary, supervised, or haul XP receipt.
- The live workforce matcher does not yet use the exact priority → enterprise → affinity → skill →
  attributes → continuity → route → stable-ID order.
- The older `EffectiveAnatomy` adapter remains beside the active anatomy/prosthetics model; live
  matching must switch to the new authority's direct read and then delete that adapter.
- Office room/tool bonuses and held-office-only effective expertise are not wired to actual rooms,
  physical tools, or office duty.
- Ambient cleaning is a helper, not a bounded idle cadence. It must remain invisible—no task marker
  and no repetitive player log.

### Required cutover

LAI.63-A stores one capability state on real cats, adapts the existing anatomy/prosthetic authority,
adds task-to-XP declaration lookup and completion receipts, applies the exact matcher ordering, and
derives effective office expertise from held office plus physical room/tool state. Blocked,
waiting, refused, invalid, cancelled, and failed work grants zero XP.

## LAI.56: families, traditions, enterprises, housing, and mentoring

### Foundation present

- `family_specialization.rs` implements the exact birth distribution, parent transfers, caps,
  traditions, maturity, profession, surname, and enterprise records.
- `family_housing.rs` implements capacities, deterministic non-kin partnership scoring, teaching
  obligations, mentors, and Elder Lodge modifiers.
- `family_authority.rs` is the canonical strict versioned aggregate for real cat IDs, dual-parent
  lineage, autonomous kin-safe partnerships, completed-building residence, persisted teaching
  defer/resume, mature colony-owned enterprises, death cleanup, atomic receipts, restart, and
  bounded family reports. It stores authoritative references to attributes and the inherited
  Relational↔Analytical axis rather than copying acquired traits or office clearance.

### Missing live behavior

- Birth, death, partnership review, lineage, residence, and household transitions are not part of
  the real lifecycle.
- Cats do not carry the authoritative two-parent lineage needed for kinship and inherited-trait
  proofs.
- Teaching obligations do not create persisted work at the real Family Home, Nursery, School,
  office, or enterprise site after the third completed professional job.
- Enterprises are inert records rather than matcher preferences, continuity, visible signs, and
  physical business ownership.
- Profession-to-surname IDs require one reconciled catalog mapping before runtime use.
- Housing permits and completed buildings do not yet constrain placement and allocation.

### Required cutover

LAI.63-B integrates lineage and birth traits into the lifecycle phase, performs bounded
partnership/household review, binds enterprise preference into workforce matching, creates
spatially truthful teaching tasks, and allocates completed Family Homes, Elder Lodges, and
Nurseries. Families may specialize early and pass stronger traits/traditions to later generations,
but acquired traits are never inherited as genetics.

## LAI.57: elections, officers, backing, succession, and expulsion

### Foundation present

- `cat_governance.rs` owns the Relational↔Analytical axis, exact merit weights, deterministic
  ballots, one replaceable God backing block, triggers, tie-breaking, and ordered expulsion cleanup.
- `officer_expertise.rs` owns report-safe imperfect appointment, vacancies, death, and Leader
  succession.
- `governance_authority.rs` composes those leaves into one strict versioned lifecycle: real resident
  facts produce the exact top-five slate and all Adult/Elder ballots; scheduled/snap occurrences and
  replaceable authenticated +10 backing are durable; election results and report-safe imperfect
  appointments hand off once; expulsion requires acknowledgements from all ten cleanup domains plus
  a reachable physical departure before committing.

### Missing live behavior

- Slates and voters do not come from the real Adult/Elder resident roster.
- Scheduled and snap elections are not invoked from the authoritative lifecycle.
- Governance and officer expertise remain adjacent instead of one appointment/succession flow.
- Expulsion cleanup is a plan rather than exact item, cargo, task, reservation, workplace, housing,
  household, office, partnership, guardian, and departure movement.
- Player backing and expulsion lack authenticated, expected-version, idempotent actions and
  report-safe projections.

### Required cutover

LAI.63-C derives elections from real cats, runs them once in the lifecycle phase, uses
`officer_expertise` as the sole imperfect appointment evaluator, and executes physical cleanup plus
a reachable departure task. Protocol and server work later expose only the broad allowed God
actions; direct officer appointments remain forbidden.

## LAI.59: staged construction and canonical bills

### Foundation present

- `construction_stages.rs` owns site reserve → delivery → 20% scaffold → delivery → 60% structure
  → delivery → 20% fit-out → operational.
- It persists required, delivered, in-transit, and consumed units; owns the exact upgrade-duration
  table, clicks, cancellation salvage, full footprints, and strict stage invariants.
- `construction_catalog.rs` classifies every `BuildingType` as cataloged, delegated, or retired and
  defines stable level-1/new and level-2–10 blueprints with full footprints, permits, exact
  scaffold/structure/fit-out bills, durations, art keys, and inspector labels. Basic homes include
  Cloth bedding and Furniture woodwork; developed work uses Lumber/Planks; the Workshop is exactly
  3×3 and introduces fixtures/tools, Metal, and Gems; Hole bills remain delegated to the Hole
  authority.

### Missing live behavior

- Current projects still accept generic caller-supplied content and quantities instead of looking
  up the one catalog blueprint and freezing its bill.
- Bills do not reserve exact quality lots and item instances with provenance.
- Research permits, Leader target/timing choice, site reservation, builders, hauling, recovery, and
  stage rendering are not connected.
- The inspector has no authoritative stage/cargo/worker/duration/click/blocker DTO.

### Required cutover

LAI.59-A's canonical construction blueprint catalog is present. Every non-retired building has a
stable blueprint, full footprint, Basic or Developed scaffold, exact structural and fit-out bill,
duration, permit, stage art keys, and labels. Basic homes include bedding/cloth/woodwork; advanced
buildings introduce declared tools, fixtures, refined material, metal, and gems. Hole bills remain
owned by the Hole axis authority.

LAI.63-D converts a blueprint into exact `quality_lots` reservations and one live construction
project. Replacement builders inherit the project without paying twice; refusal releases only the
worker/slot; route loss and cancellation salvage the same identities; restart preserves stage,
cargo, progress, and site.

## LAI.60: one physical storage and village-infrastructure authority

### Foundation present

- `physical_storage.rs` models four loose slots, exact container capacities/compatibility,
  fullness, lot identity, and adjacent non-overlapping Workshop storage.
- `village_infrastructure.rs` models farms, road work, walls, gates, AI-only ownership, and the
  village-before-Hole priority helper.
- `storage_authority.rs` now combines exact zones, visible slots, typed containers, locations,
  reservations, command-only Workshop links, construction cargo, recovery, and replay indexes
  around the one `QualityLotLedger`; it does not mirror quantities or item payloads.

### Missing live behavior

- The canonical storage aggregate is not yet the live inventory used by gathering, production,
  spoilage, construction, divine cargo, barter, caches, and recovery.
- Live transfer/reserve/unreserve/consume/salvage paths do not yet issue its atomic command
  envelopes and still mutate shadow/scalar authorities.
- Farm, road, wall, and gate projects do not mutate real tiles, block routes, reserve material, or
  create physical work.
- God prohibition exists as a leaf guard rather than a server action rejection.

### Required cutover

LAI.60-A's storage-zone/quality-lot/location authority is present. LAI.63-E/F/G then makes every
producer and consumer use that ledger, maps every open intent to an exact objective, work slot,
endpoint, route, and cargo, and applies completed infrastructure to authoritative world tiles. No
generic fallback marker is allowed.

## LAI.58/LAI.44: one research graph, currency pair, and two lanes

### Foundation present

- `research_manifest.rs` validates a derived finite total, fourteen tracks, terminals, junctions,
  capabilities, and building permits.
- `research_purchase.rs` models the God queue and Leader lane.
- `scholar_research.rs` models physical preparation.
- LAI.44 owns canonical Research Notes, Void Insight, physical scholar output, thirty Hole-axis
  studies, and four specialized boosts.
- `research_authority.rs` is the chosen canonical aggregate: it embeds LAI.44's actual Notes and
  Void ledgers, the topological/frozen God queue, physical preparation/labor, the free Leader lane,
  collision/oopsie decisions, refunds, atomic receipts, strict restart, and bounded rolling
  cadence history. It does not persist a second currency balance. Its focused target passes 13/13.

### Missing live behavior

- World tick still invokes Shrine/Favor and legacy automatic purchase paths.
- Favor/Insight compatibility APIs remain in adjacent leaves.
- Live world/protocol/server/persistence roots do not yet route and persist the chosen
  `ResearchAuthority`; they still expose parallel completion paths despite the canonical aggregate.
- God research work and Leader cadence do not create or consume real tasks.
- The UI/protocol do not expose both lanes, preparation, frozen costs, duplicate reasons, refunds,
  and report-safe Leader rationale.

### Required cutover

LAI.58-A chooses the Notes/Void queue plus Leader lane as the sole completion authority and retires
Favor/Insight purchase/minting paths. Physical scholar preparation attaches only to one ordinary
God-lane front and grants one 25% discount. Leader selection normally excludes anything in the God
queue/front, chooses a different valuable eligible study, and duplicates only for an explicit
report-based critical village need or the deterministic oopsie band. Construction consumes permits
only; the Leader still chooses the physical building and time.

## LAI.61: food permissions and divine systems

### Foundation present

- `food_divine_policy.rs` models Allowed/Reserve/Forbidden, lethal-starvation exception, bounded
  click batches/rates, purpose-bound cargo, Inspiration, construction miracles, and report-gated
  Ration/Water rescue.
- `divine_boosts.rs` owns the four specialized boost definitions and duration/economy choices.
- `divine_hole_authority.rs` is the canonical pure coordinator. It binds real physical edible lot
  identities and the real Hole ID, shares the external `VoidInsightLedger` with boosts, uses typed
  miracle/rescue debit purposes, and emits provenance-tagged purpose-bound cargo without mirroring
  Hole axes, storage quantities, construction bills, or Void balances. Its focused target passes
  5/5.

### Missing live behavior

- World tick does not yet feed report-selected real edible lots into the canonical policy decision
  or execute its deliberately good/poor/omitted choice and visible recovery work.
- Click and miracle cargo does not enter the shared storage, hauling, and construction ledgers.
- Live Hole research, boosts, and miracles do not yet route through the already shared canonical
  Void ledger transaction.
- Server authentication, rate receipts, idempotency, and report-safe controls are absent.

### Required cutover

LAI.61-A binds food decisions to physical lots and the starvation phase, uses the shared Void ledger,
and translates aid into provenance-tagged, purpose-bound cargo that cannot be traded or fed to the
Hole. LAI.64/65/67 add authenticated actions, persistence, and truthful player explanations.

## LAI.62: personal diplomacy and moneyless barter

### Foundation present

- `moneyless_barter.rs` models directional personal Alliance/Neutral/Enemy, global Neutral,
  pre-escrow Enemy rejection, report-safe possible-now versus better-later scoring, physical
  contracts, and conservation.
- `autonomous_trade.rs` already has robust escrow, hauling, recovery, and restart mechanics.
- `trade_authority.rs` is the canonical pure composition boundary: it owns directional stances and
  one contract/escrow/route ledger, binds content directly to shared `StorageIdentity` values,
  rejects Enemy before proposal/receipt/reservation side effects, and preserves consent,
  dispatch/delivery, death salvage, cancellation, conservation, replay, partition, and restart.
  Its focused target passes 13/13.

### Missing live behavior

- `diplomacy.rs`, `moneyless_barter.rs`, and `autonomous_trade.rs` describe incompatible parallel
  relationship and contract authorities.
- Coin, purse, price settlement, and legacy NPC trading remain in production roots.
- The Leader planner does not select barter from beliefs or preserve the wait-for-better rationale.
- Protocol, persistence, server actions, and the Diplomacy/Trade panels are not cut over.

### Required cutover

LAI.62-A retains the proven physical escrow/task/recovery mechanics while replacing relationship
gates with `PersonalStance`. Enemy rejects before reservation, caravan, or escrow. Alliance and
Neutral remain behaviorally identical until a later explicit feature. LAI.63-I plans from reports;
LAI.64–67 expose the personal stance action and report-safe physical contract lifecycle.

## Cross-layer work

- **LAI.63 runtime:** integrate every domain once, in protected phase order, through exact spatial
  objectives, reservations, matcher assignment, physical outcomes, reports, and bounded recovery.
- **LAI.64 protocol/actions:** expose canonical typed snapshots and only the broad God actions
  allowed by the plans; reject exact construction, placement, routes, storage zones, queues,
  workers, food lists, and direct appointments.
- **LAI.65 persistence/server:** create the fresh schema, persist every new aggregate and receipt,
  enforce authentication/authorization/version/idempotency, restart exactly, and isolate colonies.
- **LAI.66–68 UI/art:** make skills, families, elections, construction cargo, storage lots,
  research lanes, Hole/divine choices, diplomacy, and barter inspectable with report-safe reasons
  and authoritative visual states.
- **LAI.69–70 diagnostics/acceptance:** emit bounded opt-in transition traces, map every plan/Q&A/
  branch-transfer row to behavior/docs/visual/evidence, delete all obsolete authorities, and run the
  serialized final campaign/browser ladder.

## Dependency-safe order

1. Complete LAI.44 and the LAI.58-A sole research/currency cutover.
2. Add the LAI.59-A construction catalog and LAI.60-A shared physical storage ledger.
3. Consolidate LAI.55 capability/anatomy/XP/matcher state.
4. Integrate LAI.56 family/housing/enterprise/mentoring lifecycle.
5. Integrate LAI.57 elections/officers/expulsion lifecycle.
6. Reconcile LAI.61 food/divine state and LAI.62 diplomacy/barter state.
7. Perform the one LAI.63 world-tick integration and delete replaced mutation paths.
8. Cut protocol/server/persistence, then UI/art, then diagnostics and final acceptance.

No step closes because a type or isolated unit test exists. Completion requires the live authority,
physical conservation, report-safe explanation, restart evidence, visual destination, and proof that
the superseded authority is absent.

## Current live-authority audit — 2026-07-25

This section replaces the historical status assertions above without deleting the reasoning that
led to the cutover. It was checked against the live composition root, protected world-tick
transaction, protocol action union, server dispatch/projection, and persistence root.

### Canonical state that is live now

- `LeaderAiRuntimeState` is the strict persisted composition root. It directly owns planner and
  beliefs, officer requests, exact intents/tasks/reservations, `CatCapabilityAuthority`,
  `FamilyAuthorityState`, `GovernanceAuthorityState`, `ResearchAuthority`, staged construction,
  `StorageAuthority`, the Hole, `DivineHoleAuthority`, purpose-bound cargo, boosts,
  `TradeAuthority`, physical cat/anatomy/prosthetic state, player directives, receipts, and bounded
  diagnostics. Former Shrine/Favor, purchase/scholar, and coin aggregates are deliberately absent
  from this schema.
- `phase_lai63_protected_runtime_transaction` is the live all-or-nothing world-tick gateway. Its
  persisted phase order is authority/needs, report observation, Leader/officer review, exact
  sites/reservations, workforce matching, physical movement/cargo, Hole/divine cargo, unified
  research, personal stance/barter, stress/injury, then projection/diagnostics. A failed phase drops
  the staged clone rather than committing a partial authority mutation.
- Real colony cats are reconciled into capability, family, and governance authorities. Physical
  deaths are projected into family and governance state once. Completed exact work issues
  once-only capability XP and optional family-professional receipts, and worker selection uses the
  canonical matcher rather than treating the pure leaves as detached data.
- Research is the live canonical `ResearchAuthority`: one Notes ledger, one shared Void ledger, the
  God queue, free Leader lane, collision/oopsie policy, preparation, receipts, and report-safe
  projection. Server queue, reorder, fund, remove, and preparation actions mutate this aggregate.
  Requested preparation creates an exact persisted Research Hut/School task, reserves the scholar,
  site, route, and work capacity, and credits only physical work at that footprint.
- Hole output is advanced into `ResearchAuthority::void`; boosts and emergency rescue debit that
  same ledger. Boost controls are opaque player-bound offers derived from canonical research
  entitlements. Rescue controls are opaque report-bound witnesses derived from the shared
  resident-needs report, and accepted rescue cargo materializes at the Hole delivery apron with
  purpose/provenance restrictions.
- Protocol v3/schema v2 and the canonical server boundary now expose typed Research, Candidate
  Backing, Personal Stance, Expulsion, broad domain nudge, Hole click, Inspiration, boost, and
  emergency-rescue actions with exact required version lanes. Snapshot and admission compute those
  lane versions from the same canonical aggregates.
- `cat-server::persistence` serializes and validates the canonical Leader AI runtime on save/load;
  it is no longer a leaf that exists only in unit tests.

### Why Shrine/Favor references still appear in searches

`world_tick.rs` contains an intentionally retained `#[cfg(any())] mod retired_lai23_runtime`.
That module is never compiled, invoked, serialized, or reachable from the production phase graph.
Its Shrine offering, Favor, automatic purchase, and legacy scholar code is historical implementation
evidence only. The few helpers immediately outside it are explicitly shared physical-world
compatibility helpers and are forbidden from reading or mutating Shrine/Favor state. A text search
that counts names inside the retired module is not evidence of a live parallel currency or research
authority.

The remaining `BuildingType::Shrine` and `SiteRef::Shrine` names are legacy display/spatial names
for the physical structure now designed as the Hole. They do not make Favor authoritative.
Renaming or deleting those compatibility names remains cleanup work, but no new behavior may route
through the retired module.

### Exact remaining integration gaps

1. **Family lifecycle:** registration, death, professional receipts, and the canonical aggregate are
   live, but the protected transaction does not yet perform the full partnership, residence,
   Family Home/Elder Lodge/Nursery allocation, after-three-work teaching, mentoring, mature
   enterprise continuity, or visible enterprise-sign lifecycle against real completed buildings.
2. **Governance lifecycle:** residents, death/succession state, officer institution, authenticated
   backing, and expulsion preview exist. Scheduled/snap elections, imperfect appointment handoff,
   and the complete physical expulsion cleanup/departure transaction still need one canonical
   protected-phase execution path; the older colony election phase is not accepted as that cutover.
3. **Research completion:** the God queue/actions and exact physical preparation are live. Funded
   God-study labor and the complete automatic Leader selection/labor/completion cadence still need
   to advance through the unified research phase with physical scholars, collision avoidance, and
   report-safe duplicate rationale.
4. **One physical inventory:** canonical storage is used by staged construction and divine cargo,
   but every legacy gather/production/spoilage/cache/recovery path has not yet been cut over from
   scalar colony resources to stable lot/item identities. Completion requires conservation through
   one storage authority, not synchronization between two inventories.
5. **Construction miracle action:** the sim owns manifest-classified bulk, exact-item, and fixture
   value/materialization rules, but `cat-server` still marks
   `ConstructionMiracleWitness` as an authority gap and fails the public action closed. The server
   must derive an opaque current project witness and call the canonical transaction; it must never
   accept a client bill, value table, stage, item identity, or destination.
6. **Divine rescue delivery:** authorization, debit, exact quantity, provenance, and apron
   materialization are live. The protected Hole/divine phase still needs the complete visible
   high-priority haul/consume/remainder/restart/death flow before rescue is accepted end to end.
7. **Trade execution:** personal stances and the canonical physical trade aggregate are persisted,
   projected, and mutable. The protected phase currently validates the aggregate; it does not yet
   originate every report-valued Leader contract or advance every escrow/hauler/recovery outcome
   while all legacy coin/NPC settlement paths are retired.
8. **Projection/UI/art and cleanup:** several canonical controls and panels are live, but the final
   family institutions/signs, construction phase visuals, container fullness, quality composition,
   and browser acceptance matrix remain. LAI.70 must delete or archive unreachable compatibility
   roots, rename remaining Shrine-facing labels to Hole, and prove no old protocol, currency,
   inventory, research, planner, or mutation path is reachable.

These are integration gaps, not permission to add another aggregate. Each closes only when the
existing canonical owner drives the physical world, persistence, report-safe projection, UI state,
restart/replay behavior, and serialized acceptance evidence.
