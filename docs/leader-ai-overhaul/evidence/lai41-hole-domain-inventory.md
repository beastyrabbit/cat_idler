# LAI.41 — Hole-domain implementation inventory

**Status:** read-only inventory; no production cutover is present.

**Authority used:** the locked Plan 1, the complete Plan 1 register in `BOARD.md`,
the thread QA audit, the current Rust workspace, and the protected
`../the-shrine-upgrade` source, notes, tests, and art.  The protected branch is
a semantic/art source only, not a root to merge (P1.01); its current `black_hole`
leaf is identical to the candidate already present in this worktree.

## Decision boundary

The resulting domain must be named `BlackHole` internally and **The Hole** in
player-facing text.  It is a single physical authority: the planner selects
only report-supported candidates and ordered fallbacks, then the Hole validates
the authoritative content/lot/route/reservation state before it creates Void.
There is no compatibility route in this decision: `Shrine`, `Favor`,
`Blessings`, generic food storage, scholar Insight, and their migration aliases
must disappear rather than be renamed at the protocol or persistence boundary.

The plan fixes the following behavior, all of which is currently incomplete:

| Required behavior | Current evidence | Cutover finding |
|---|---|---|
| Fixed landmark | `BuildingType::Shrine` and `footprint_for` in `crates/cat-sim/src/types.rs` / `spatial_tasks.rs` model the old 3x3 shrine. | Replace with `BuildingType::BlackHole`, a fixed 5x5 reservation: central 3x3 work/upgrade/delivery area plus the permanent sixteen-cell paved ring. Axes must never affect geometry. |
| Axes and opening rules | Inert `black_hole.rs` has `BlackHoleAxes`, 0–10 validation, `1 + width`, and `10 * (1 + depth)`. | Reuse only those formulas and bounded-axis validation. Wire them to manifest-owned studies, physical construction, and the actual Hole location. |
| Cadence / concurrency | Candidate declares `INTAKE_COOLDOWN_GAME_MS = 40 * 60 * 1_000`, `active_feed`, and `active_upgrade`, but nothing calls it from `world_tick`. | Make the forty-game-minute absolute cursor persistent; one active feed pipeline and one active physical upgrade are independent allowed slots. Do not inherit the candidate's unrelated 12-hour review constant as a new rule. |
| Feed identity and cargo | Candidate accepts legacy `ResourceKind` / `Item` values and mutable quantities, including a `ChildLoad`; existing offering code has physical reservation, carry, deposit, cancellation, and salvage stages. | Replace candidate feed values with manifest `ContentId`, quality lots, exact item instances, provenance, capability, reservation, and route identity. Reuse the *shape* of explicit carry/deposit/recovery, not `OfferingPackage`, `Food`, or Shrine escrow. |
| Darkness and capability gates | Candidate has hand-written resource and item gate tables and quality ceiling. | Gates must come from the catalog and capability state for concrete foods, materials, recipe outputs, items, rare drops, and quality. A locked stored object is not feedable. Hand-written `ResourceKind` tables are not authority. |
| Reward value | Candidate credits a legacy resource price or `Item::value()` one-for-one in `VALUE_MICROS`; it does not represent recipe complexity, augmentations, condition, material provenance, or LAI.37 quality. | Replace it with checked integer micro-Void calculation over canonical content value, processing/recipe complexity, quality, item value, augmentation, and condition. The plan specifies direction but not the exact formula/rounding. |
| Upgrade inputs | Candidate emits deterministic legacy resource/tool recipes, physically free of its own reward currency, and includes tools at L2, Metal at L7, Gems at L10. | Preserve the separation—Void pays Hole-axis research, physical items/lots pay construction—but move all recipe/tool/material definitions to the catalog. The candidate's concrete counts and legacy tool quality are not yet plan authority. |
| Endless, imperfect choices | `shrine_offerings.rs` has belief-only selection, deterministic omission and an explicit physical state machine; it also has a hidden `survival_or_active_defense` block. | Reuse report-only candidate scoring, deterministic omission, physical stages, and later recovery. Remove the hidden-survival veto: a valid scarce-food feed must be legal, with physical recovery later. Defense/self-preservation may still preempt before pickup. |
| Report parity | The plan and audit require one report projection for God and Leader. Current client/server offering projection redacts some state but only for the Shrine/Favor flow. | Hole candidate selection, rationale, confidence, stage, omission, and error codes must derive from the same report-safe projection; no exact stock, ecology, regeneration, reservation internals, or hidden survival threshold may leak. |
| Cancellation, death, route loss | Existing offering pipeline explicitly distinguishes release before pickup, deposit, and stockpile salvage, and `world_tick` has reservation/cargo recovery paths. | Generalize this semantic pattern to physical lots and exact item instances. Every cancellation, death, route interruption, unreachable destination, or preemption must either preserve the existing reservation/lot or atomically recover it to a defined physical location; no Void credit without completed intake. |
| Idempotency / restart | Existing server actions, persistence, and the candidate branch provide idempotency and restart precedents. The candidate's isolated persistence still binds a `black_hole_runtime` row to `BuildingType::Shrine` / `building-shrine`. | New Hole command IDs, credit IDs, reservation IDs, axis project IDs, absolute opening cursor, route/cargo and recovery disposition must persist outside the Leader fingerprint and replay once. Do not retain the old seeded Shrine default or missing-row compatibility behavior. |
| Currency boundary | Current `favor.rs`, `shrine_offerings.rs`, research and boost paths are Favor-centric. | Physical feeds credit Void Insight only; Void alone funds thirty Hole-axis studies and player-only boosts. Ordinary research remains Notes; construction consumes only physical recipe inputs. |

## Exact source-to-target matrix

| Existing/source symbol or file | What exists now | Semantic reuse | Required target disposition |
|---|---|---|---|
| `crates/cat-sim/src/black_hole.rs::{BlackHoleAxes, intake_width, max_order}` | 0–10 axis validation and the two exact formulas; pure deterministic candidate ordering. | Reuse formulas, ordered-selection discipline, bounded error handling, and strict axis decode as implementation techniques. | Rebuild around content-manifest IDs, LAI.37 lots/items, real route/reservation validation, checked arithmetic, stable command/credit IDs, and Hole location. |
| `black_hole.rs::{FeedKind, FeedCandidate, resource_darkness_requirement, item_darkness_requirement, resource_unit_value_micros}` | Legacy `ResourceKind`, generic Food/Fish/Preserves, old `Item`, hard-coded gates/prices. | None of the legacy data model or tables. | Delete/replace with catalog-owned food/material/item/rare-material gates and canonical value rules. |
| `black_hole.rs::{BlackHoleRuntime, IntakeState, UpgradeRecipe}` | Inert schema v1 candidate; no world-tick integration. | One-feed/one-upgrade shape, absolute opening cursor concept, and checked persistence validation are useful. | New strict runtime state uses Hole identity, 5x5 spatial reference, content lots/items, physical cargo/feed queue, Void ledger and upgrade construction. Do not retain Shrine IDs/defaults. |
| `crates/cat-sim/src/shrine_offerings.rs` | Belief estimates, deterministic selection/omission, one active pipeline, reservation/haul/deposit/ritual/cancel states, and salvage disposition. | Report-only choice, deterministic omission, explicit physical state and salvage intent. | Replace all offering packages, Favor events, Shrine endpoint and survival veto with Hole candidates/fallbacks and lot/item-backed feed state. |
| `crates/cat-sim/src/favor.rs`, `divine_boosts.rs`, `scholar_research.rs`, old research purchase paths | Favor ledger, Favor debits, divine boost cost and scholar-Insight-era assumptions. | Only generic CAS/idempotent-ledger and checked-balance techniques. | Delete Favor/Blessing/Insight identities and conversion paths. LAI.44 must connect Void and Notes to the new research/boost manifest. |
| `crates/cat-sim/src/world_tick.rs` | Founding `BuildingType::Shrine`, `building-shrine`, shrine-centred roads, Shrine site refs, offering review/task/cargo/recovery, and many Shrine-related reachability assumptions. | Existing world tick’s phase discipline, tasks, reservations, physical haul, preemption, and recovery hooks. | Replace every Shrine branch with BlackHole-specific placement, 5x5/3x3/ring endpoint, feed/upgrade state advancement and diagnostics. Remove Shrine rituals/tithes/legacy resource mutation. |
| `crates/cat-sim/src/types.rs::BuildingType::Shrine`; `spatial_tasks.rs::SiteRef::Shrine`; `footprint_for` | Closed wire enum and typed canonical footprint machinery, but Shrine is a 3x3 building. | Reuse canonical footprint/site validation framework. | Introduce BlackHole-only enum/site/footprint and reject all old identity strings; central 3x3 task footprint and full 5x5 landmark must both be represented. |
| `crates/cat-sim/src/stockpiles.rs::{SHRINE_STOCKPILE_ID, shrine_rect, shrine_index}` | Legacy shrine reservoir and migration to a general storehouse. | General storage capacity/routing only. | Delete Shrine identifiers and migration branch. Hole feed queue must be a separate physical location in the LAI.37 ledger, not a generic stockpile alias. |
| `crates/cat-protocol/src/lai24_snapshot.rs` | `ShrineSnapshot`, offering pipeline, `FavorLedgerSnapshot`, generic resource presentation. | Strict snapshots, version checks and report-safe string wrappers. | Protocol v3/schema v2 must remove those variants and add strict Hole/Void/physical-cargo/axis/project/report snapshot fields. |
| `crates/cat-protocol/src/lai25_action.rs`, `lib.rs::ClientAction::{OfferMaterials,OfferResource}` | Favor research/boost actions and manual Shrine offering actions. | Bounded envelopes, expected-version and idempotency conventions. | Remove old actions; add the plan's `NudgeBlackHole` plus Void research/boost and the required physical Hole action surface, with exact domain-version lanes. |
| `crates/cat-server/src/leader_ai_snapshot_projection.rs`, `leader_ai_action_routing.rs`, `main.rs` | Projects/executes Shrine/Favor research/boost state; routes legacy offering actions. | Authorization, stale-version, report-safe conflict and projection plumbing. | Project the same Hole reports to God and planner; dispatch only typed Hole commands; validate no hidden truth leaks in errors/logs. |
| `crates/cat-server/src/persistence.rs`, `leader_ai_persistence.rs` | SQLite stores old Shrine/Favor migration/conversion state; source branch adds a separate `black_hole_runtime` table tied to `BuildingType::Shrine`. | Atomic save/load, bounded JSON, strict version/restart validation techniques. | Fresh schema only (P1.34): persist new Hole, Void, lots/items/cargo/routes/projects/cursors/idempotency; delete old tables/columns/conversions/fixtures, fail closed on malformed/future state. |
| `crates/cat-client/src/leader_ai_ui/progression.rs`, `accessibility.rs`, `station_layout.rs` | Shrine offering and Favor panels, badges, labels; `BuildingType::Shrine` open station art. | Accessibility/test-ID discipline and report provenance presentation patterns. | Replace with The Hole panel/wireframe, 5x5 map representation, central 3x3 task/pinned delivery, report provenance/rationale/physical status; remove shrine/favor labels, controls and accessibility entity kind. |
| `../the-shrine-upgrade/crates/cat-sim/tests/black_hole.rs`, protocol/persistence leaves | Baseline leaf tests cover axis tables, simple intake, legacy gates/value, upgrade recipe and restart checks. | Preserve test categories only: table bounds, cadence, ordering, restart/fail-closed. | Do not import assertions that bless generic `ResourceKind`, legacy `Item`, hard-coded legacy values, or Shrine-linked persistence. New red tests must cover the Plan 1 physical/capability/report contract. |

## What the present candidate proves—and what it does not

The candidate is useful evidence that an isolated, deterministic leaf can
validate `0..=10`, calculate `1 + Width` and `10 * (1 + Depth)`, and reject
some malformed persisted state.  It does **not** establish the domain required
by LAI.41 because it is deliberately inert, has no 5x5 spatial claim, does not
own physical quality lots/items/cargo, uses generic resources, lacks capability
checks, cannot validate a real route/reservation, has no report projection, and
is wired in neither the current server nor client.

It also contains concrete policy that must be re-derived rather than silently
adopted: legacy Darkness tables, generic item quality thresholds, resource
prices, upgrade recipe counts, child-load behavior, a 12-hour review interval,
and saturation in value/lifetime totals.  The plan only locks the axis formulas,
cadence, category gates, input milestones and qualitative reward factors; it
does not bless those candidate constants.

## Protocol, persistence, UI, and art gaps

### Protocol and server

- Bump to protocol v3 and Leader-AI snapshot schema v2; remove `ShrineSnapshot`,
  Favor ledger/events, generic food variants and legacy offering actions.
- Add strict, bounded Hole/axes/Void/feed candidate/fallback/physical-cargo/
  route/project/recovery/omission projections and `NudgeBlackHole`; preserve
  per-domain expected versions and idempotency IDs.
- Put all God-visible text, conflicts, reports and panel data through the same
  report projection consumed by the Leader. Exact stock, renewal, gate failures
  carrying hidden state and route internals remain server-only.
- Add server-side authoritative validation before reservation: canonical content,
  capability, Darkness, quality, ownership, provenance, physical location,
  amount, route, queue capacity and one-pipeline/one-upgrade invariants.

### Persistence

- The current persistence base contains old Shrine/Favor conversions and
  compatibility paths; P1.34 requires a fresh database/fixture reset, not a
  migration from them.
- Persist exactly once: Hole identity and fixed spatial state; axes plus study
  versus construction state; absolute next-opening cursor; active feed/project;
  lot/item reservations and cargo; destination/salvage recovery state; Void
  credits; and bounded idempotency outcomes.
- Restart must prove byte-equivalent or semantically identical continuation at
  pre-pickup, carried, deposited/waiting-to-open, interrupted, credited, and
  upgrade stages. A missing row may not fabricate `building-shrine` as the
  source-branch persistence code currently does.

### Client and visual presentation

- Current workspace assets contain only legacy `public/images/game/buildings/shrine.png`
  (48x48) plus blessing icons. They are deletion targets, not Hole art.
- The protected source supplies a reusable pixel-art pack: one 80x80
  `public/images/game/buildings/black-hole.png`, an 80x80 transparent
  `black-hole/base.png`, and ten 80x80 cumulative transparent layers for each
  of `width`, `depth`, and `darkness` (`01`–`10`). Its source art test verifies
  the 16-pixel transparent outer ring, cumulative pixels, and the fixed 80x80
  canvas; this directly matches the Plan's 5x5 landmark presentation.
- Reuse these files only after the target renderer has deterministic art-key
  lookup, native-dimension/alpha/bounds checks, central task and pinned-delivery
  markers, accessible textual fallback, and gameplay-zoom evidence. Missing in
  the current workspace are the Hole base and all thirty layers, a Hole icon/
  Void visual treatment, a central-3x3/ring task marker, complete panel states,
  and the plan-required presentation validation. Preserve the supplied crisp
  top-down pixel style; do not substitute the old shrine/altar/reliquary art.

## Diagnostics needed before campaigns

The Plan's 120-tick probe must include a bounded Hole record each progress
interval: world/tick and phase, Hole ID, active feed/project IDs and stage,
absolute opening/review deadline, report age/level, candidate and fallback
counts, chosen/omitted reason, reservation and cargo counts, route state,
physical lot/item quantities, recovery/salvage disposition, Void delta, and
terminal cause.  Values that would disclose hidden stock or ecology must be
recorded in server-only diagnostics, with the report-safe reason/projection
emitted separately.  This makes a blocked route, unavailable worker, missing
capability, unavailable slot, no eligible lot, stuck reservation, repeated
idempotency replay, or a missed cadence distinguishable from a hang without
turning silence into a pass.

## Red test matrix for the cutover

| Group | Required red evidence |
|---|---|
| Naming/deletion | No `Shrine`, `Favor`, `Blessing`, scholar `Insight`, old action/snapshot/table/id/asset reference or compatibility alias remains in the authoritative Hole path; fresh state begins with BlackHole only. |
| Geometry | Exact 25-cell landmark: nine canonical central work cells and sixteen permanent ring cells; placement/reservation/route/task projection cover every required cell; axes 0 and 10 produce identical geometry. |
| Axes/cadence | Reject every out-of-range axis; assert Width `1 + width`, Depth `10 * (1 + depth)`, one active feed and one active project, and exact forty-game-minute absolute deadlines across partition/restart twins. |
| Catalog/gates | Feed rejection is atomic for dangling/locked/unknown content, disallowed food/material/item/rare drop, insufficient Darkness, too-high quality, malformed item condition/augmentation, missing capability, wrong amount/provenance/location, absent reservation and unreachable route. |
| Value | Checked integer micro-Void calculation covers raw versus processed/recipe complexity, quality, item value, augmentation and condition; overflow rejects without consuming cargo or creating Void. Exact formula and rounding await a catalog decision. |
| Construction | Upgrade research debits Void only; construction consumes only canonical physical recipes. Assert tools at L2, Metal at L7, Gems at L10, unavailable materials/tool/route rejection, and no double debit or double consumption. |
| Physical feed | Source→reservation→carrier→Hole queue→central delivery→opening consumption conserves every lot/item identity and quality. Test cancellation, worker death, route loss, defense preemption and interruption before/after pickup with explicit release, delivery, or salvage and no duplicate credit. |
| Choice/visibility | Strong report-based choice prefers low believed replacement cost; weak/stale report choice may feed scarce Apple/Fish/Meat/meal; omission can skip one or more reviews; no hidden survival veto. God and Leader projections are identical, hide exact stock/ecology through level 3, and apply the P1.04 report ladder at levels 4–5. |
| Endless/idempotency | Endless eligible demand continues after accepted feeds; repeated command/credit/recovery IDs are no-ops with exact original outcome; conflict IDs fail closed; all stages round-trip/replay once across restart and partition twins. |
| Protocol/persistence/UI/art | Old protocol rejected; v3/v2 strict decode rejects unknown/future/malformed state; multi-colony isolation holds. Snapshot/action errors are report-safe. Asset key, 80x80 dimensions, transparency/ring, all 30 cumulative layers, deterministic composition, accessibility labels and Hole wireframe are covered. |
| Diagnostics | 120-tick slow/stalled scenarios produce bounded phase progress, state counts and terminal cause for feed, upgrade, reservation, route and recovery; no silence-based success. |

## Coordinator decision closure

These choices close gaps found by the inventory while preserving both the exact plan and the
non-conflicting design work in `the-shrine-upgrade`:

1. `10 * (1 + Depth)` is the maximum accepted **feed-order unit count**, exactly as Plan 1 section
   6 and P1.24 state. It is not a duration. Reserved, in-transit, delivered, and waiting units of
   one active order all count toward that fixed acceptance cap. Width consumes at most
   `1 + Width` delivered units at each opening.
2. The Hole leaf accepts catalog-resolved feed policy rather than reviving a `ResourceKind`
   switch. LAI.36/LAI.43 own each concrete content gate, capability, base value, processing stage,
   and augmentation value; LAI.41 validates and consumes that policy atomically.
3. Integer micro-Void is calculated once, with checked intermediates and one final floor:
   `(base_value_milli + installed_augmentation_value_milli) * 1000 * stage_value_percent *
   quality_hole_percent * current_condition / (100 * 100 * maximum_condition)`. Non-items use
   condition `1/1`. Stage value is Raw 100, Processed/Simple 125, Prepared 160, Complex 210, or
   Feast 280; quality uses LAI.37's exact 75/100/130/170/225 Hole percentages. Zero maximum
   condition and overflow reject without consuming cargo or crediting Void.
4. The protected branch's non-conflicting physical upgrade progression is retained and translated
   to canonical content/quality: every target level costs `5 * level` Refined Materials; Width
   adds `2 * level` Logs and, from level 4, `2 * (level - 3)` Planks; Depth analogously uses Stone
   and Blocks; Darkness uses Herbs and additional Refined Materials. Levels 7–10 add
   `2 * (level - 6)` Metal; level 10 adds four Gems. Tools are none at level 1; one Crude at 2–4;
   one Common at 5–6; two Fine at 7–8; two Superior at 9; three Masterwork at 10. Research pays
   Void separately; construction never charges Void again.
5. The cadence authority stores an absolute next-opening game-minute cursor and advances it by
   exactly forty minutes per due opening. A large tick processes every bounded due opening in
   order (limited by the finite active order), producing the same result as partitioned ticks and
   restart; elapsed wall-clock time is never used.
6. Hole reports use the already-locked P1.04 ladder, not a private visibility scheme: queue and
   capacity use stock precision; intake/output use the production ladder; cadence/ecology stays
   hidden through level 3 and estimated only at levels 4–5. Candidate rationale, confidence,
   omission, physical stage, and recovery disposition are report facts; authoritative rejected
   candidates and hidden stock never cross the projection. God and Leader receive identical
   projections.
7. Interruption before pickup releases the reservation in place. After pickup, cargo returns to
   its reachable origin, otherwise the nearest compatible reachable stockpile, otherwise a typed
   recovery cache on the carrier's last valid land tile. The same identity, quality, provenance,
   and quantity survives; silent deletion and remote teleporting are invalid.

## Implementation conclusion

LAI.41 is a replacement cutover, not an integration of an already working
feature.  The current candidate and protected source provide bounded leaf
patterns and a complete reusable 80x80 layer pack, while the live workspace
still has an active Shrine/Favor root across simulation, protocol, persistence,
server and UI.  Implementation should therefore establish catalog/quality-lot
contracts first, then add one Hole authority through the report→command→
validation→reservation→physical-task→outcome loop, while explicitly deleting
the legacy root rather than preserving it as a fallback.
