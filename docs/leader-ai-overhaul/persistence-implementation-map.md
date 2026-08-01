# LAI.26B SQLite migration and persistence implementation map

> Historical first-cutover readiness evidence only. Current persistence uses the fresh-schema/reset
> contract in [`integrated-implementation-map.md`](integrated-implementation-map.md) and performs no
> Shrine/Favor/generic-food/coin/research semantic migration.

This map is additive implementation guidance for LAI.26 production work. It does not mark LAI.26
complete, does not authorize `world_tick.rs` edits, and does not change protocol, server routing,
client, or persistence production code.

## Source state read for this map

- `crates/cat-server/tests/lai26_persistence_migration_contract.rs` is the LAI.26 red contract. It
  requires a strict `LAI26_SCHEMA_VERSION`, durable `leader_ai_migration_marker`, one transactional
  migration per world/save, exact one-time Favor conversion, full leaf persistence, quarantine or
  rejection for malformed rows, bounded idempotency replay, cross-colony isolation, and every-stage
  restart equality.
- `docs/leader-ai-overhaul/wire-persistence-ui.md` defines the same migration contract and lists the
  complete post-cutover state families: planner, beliefs, reports, officer requests, standing orders,
  tasks/reservations/cargo, Shrine/Favor, research/quota/Insight/preparation, boosts, diplomacy,
  trade, cat care/prosthetics, idempotency receipts, and transition fingerprints.
- Current `crates/cat-server/src/persistence.rs` uses `CREATE TABLE IF NOT EXISTS` plus
  `migrate_add_missing_columns`, deletes and rewrites the whole world inside `save_world`, and loads
  legacy `colonies.globalUpgradePoints`, `upgradeTree`, `lastTitheAt`, `lastOfferingAt`, jobs,
  buildings, cats, villages, trade offers/caravans, and transport state.
- Current `load_colony` hard-defaults `leader_ai_runtime: Default::default()` and
  `leader_ai_restart_validated: false`. It does not read or write a durable LAI runtime aggregate.
- `crates/cat-sim/src/world_tick.rs` already has `ColonyRuntime.leader_ai_runtime:
  LeaderAiRuntimeState` and several in-progress LAI.23 phase hooks, but LAI.26 must not depend on
  debug-only validation or runtime reconstruction to restore persisted state.
- `crates/cat-sim/src/leader_ai_runtime.rs` is the correct aggregate serialization source. It contains
  versioned planner, intent, belief, officer, scheduling, Shrine/Favor, research, boost, diplomacy,
  trade, cat-care, prosthetic, and idempotency receipt state with `serde(deny_unknown_fields)` on the
  aggregate boundary.
- Current `cat-protocol` still exposes legacy `ClientAction` and `ActionResult`, and current
  `cat-server` still routes WebSocket actions directly into `cat_sim::actions::apply_action`. LAI.26
  persistence must be ready for the LAI.25/LAI.27 envelope pipeline, but it should not implement that
  routing.

## Migration version table

| Stored state | Meaning | Startup behavior | Save behavior |
|---|---|---|---|
| no marker, no LAI columns | Legacy save before LAI.26. | Run the full LAI.26 migration inside one transaction before any row is used by simulation. | Never write this format again after a successful migration. |
| marker `in_progress` | Process died or was interrupted during migration. | Reject or quarantine the save as partial before loading any colony/runtime row. | Production code must not leave this state after commit. |
| marker `complete`, `targetSchema = LAI26_SCHEMA_VERSION` | Current LAI.26 save. | Validate every required row and runtime aggregate, then load. | Write LAI.26 state and bounded receipts transactionally. |
| marker complete with older target schema | Supported older LAI schema only if an explicit forward migrator exists. | Run the next one-step migrator in a transaction; otherwise reject. | Save only the current target version. |
| marker complete with newer target schema | Downgrade attempt or future save. | Fail closed with `UpdateRequired`/unsupported-save style error and no mutation. | Never downgrade. |
| malformed/duplicate marker | Corrupt save. | Roll back candidate work and quarantine or reject the world/save row with a bounded reason. | No partial save. |

`LAI26_SCHEMA_VERSION` should be a single persistence constant owned by `cat-server` persistence. It
must not be shadowed by protocol or sim schema constants, though the marker should record the nested
`LeaderAiRuntimeState.schema_version` and protocol versions that were current when the save was
written.

## Minimal schema shape

The current persistence layer already stores many complex values as JSON columns. The smallest
production slice that satisfies LAI.26 is:

| Boundary | Proposed storage | Required content |
|---|---|---|
| World schema marker | `leader_ai_migration_marker` table keyed by `worldId`/save identity. | Source schema, target schema, status, migration fingerprint, conversion totals, source row checksums, completion tick, and conversion event IDs. |
| World metadata | Add current persistence version columns to `world` or the marker table. | `LAI26_SCHEMA_VERSION`, runtime aggregate schema, action protocol version, snapshot protocol version, world seed, and shared spatial/reservation rules versions. |
| Per-colony runtime | Add `colonies.leaderAiRuntime TEXT NOT NULL` or a `leader_ai_colony_runtime(colonyId, runtimeJson, runtimeVersion, transitionFingerprint)` table. | Canonical JSON for `LeaderAiRuntimeState`, a runtime version/checksum, and a transition fingerprint for restart equality. |
| Bounded idempotency | `leader_ai_idempotency_receipts` table or canonical subdocument plus indexed receipt IDs. | Idempotency ID, colony ID, authenticated player hash/id, action protocol version, payload fingerprint, expected-version fingerprint, accepted/rejected result DTO, committed tick, expiry tick, and receipt schema. |
| Quarantine | `leader_ai_quarantine` table. | World/save ID, table, row ID, bounded reason code, source fingerprint, target schema, quarantine tick, and redacted diagnostic detail. Do not store hidden truth in a form that can leak to snapshots or conflicts. |
| Legacy conversion audit | Marker row or `leader_ai_legacy_currency_conversion` table. | Legacy `globalUpgradePoints`, legacy unspent research points, converted micro-Favor, owned study/node IDs preserved, event ID, and duplicate-conversion guard. |

If LAI.26 chooses one JSON runtime column, all nested collections must still serialize canonically:
sorted `BTreeMap` keys, stable ID order for arrays, no map keyed by process-local pointer/order, and
`deny_unknown_fields` validation on deserialization. If LAI.26 normalizes later, the JSON aggregate
should remain the restart-equality oracle until all tables prove byte-equivalent load/save behavior.

## State family map

| State family | Current source | Persistence requirement |
|---|---|---|
| Planner clock/posture/epoch/versions | `LeaderAiRuntimeState.planner` and future standing-order/nudge state. | Persist schema, planning clock, planning epoch, posture, domain/resource/spatial/reservation versions, live nudges, standing orders, and terminal history without lazy defaults. |
| Intents and officer requests | `IntentGraph`, `OfficerRuntimeAggregate`. | Persist live and terminal intents, dependencies, responsible leader/officer IDs, request status, expiry, supersession, and versions. |
| Beliefs, evidence, reports | `BeliefStore` and report leaf state. | Persist observations, report confidence/ranges/age/provenance, contradiction/replacement metadata, and report versions; reject hidden projection-only fields in authoritative storage. |
| Scheduler and reservations | `SchedulingRuntimeAggregate`. | Persist scheduler state, local `ReservationLedger`, world `WorldReservationLedger`, visible task runtime, resolved spatial tasks, world reservation IDs, known cargo site IDs, and last transition tick. |
| Physical tasks, sites, cargo | `VisibleTaskRuntime`, `ResolvedSpatialTask`, legacy `jobs.metadata`. | Migrate legacy metadata into typed `SiteRef`, work slot, endpoint, route, stage, progress, worker, cargo disposition, reservation IDs, blocked reason, and last-update tick. |
| Shrine/Favor | `ShrineFavorRuntimeAggregate`, `FavorLedger`, `ShrineOfferingState`. | Persist every Shrine pipeline, package, source/haul/deposit/ritual/cancel/salvage stage, pinned endpoint, cargo disposition, exact Favor events, balance, event IDs, and ledger version. |
| Research/quota/Insight/preparation | `ResearchRuntimeAggregate`. | Persist owned studies, purchase records, committed undiscounted/discounted prices, automatic rolling seven-day quota window/used/limit, Insight balance/events, scholar assignments, preparations, and consumed preparation IDs. |
| Divine boosts | `DivineBoostState`. | Persist purchase IDs, active boost type, exact Favor debit reference, start/expiry ticks, committed duration/effect, same-type active state, and boost version. |
| Diplomacy | `DiplomacyLedger`. | Persist relationship records, proposals, consent/approval state, blocked state, pair versions, action receipts, and only public cross-colony facts. |
| Trade | `TradeLedger` plus legacy village trade offers/caravans. | Persist proposals/contracts, consent state, escrow reservation IDs, in-transit cargo, route, carrier/stage, recovery/salvage state, contract versions, and action receipts. |
| Cat care | `CatRuntimeState`, `CatTraits`, `StressState`, `CatAnatomy`, `AcquiredTraitState`. | Persist migrated 1-20 attributes, deterministic personality seed results, learned skills, stress/recovery/refusal state, anatomy, injuries/treatment, acquired traits, death-processing tick, and cat-care versions. |
| Prosthetics | `ProstheticLedger`. | Persist finite item identity, inventory/fitted/repair/broken/wear state, fitted slots, restoration/durability, death recovery, trade ownership, and any required prosthetic version. |
| Transition fingerprints | New persistence/runtime helper. | Persist deterministic fingerprints for pre/post restart equality over runtime, protocol snapshot, Favor, reservations, tasks, cargo, trade, prosthetics, and idempotency receipts. |

Fresh worlds and fresh personal colonies must write valid empty/default records for every required
family at creation time. They should not rely on `NULL` meaning "legacy default" after LAI.26.

## Transaction ordering

One startup migration should use this exact order:

1. Open an immediate transaction and enable foreign keys.
2. Read the world row and the LAI.26 marker without loading the world into `WorldState`.
3. Reject newer/downgrade/partial markers before decoding nested state.
4. Snapshot all legacy rows and source fingerprints needed for migration.
5. Convert legacy currency, cats, tasks/sites/cargo/reservations, trade, and runtime defaults into an
   in-memory candidate.
6. Validate stable IDs, ownership, references, nonnegative values, schema versions, unknown fields,
   bounds, and cross-colony privacy on the complete candidate.
7. Write new schema columns/tables and canonical runtime JSON.
8. Write the complete marker and conversion audit with final fingerprints.
9. Commit once.
10. On any error, roll back the whole transaction; if the source is malformed but diagnosable, write a
    separate bounded quarantine record only through the quarantine path.

`save_world` should likewise write the world, colonies, runtime aggregate, receipts, transition
fingerprints, and marker/checksum in one transaction. No accepted server action should be considered
durable until the updated world state and its idempotency receipt have both committed.

## Exact one-time Favor conversion

LAI.26 must convert legacy `globalUpgradePoints` plus legacy unspent research points to exact
micro-Favor once. The conversion implementation should:

- read `colonies.globalUpgradePoints` and `UpgradeTreeState.research_points` from the legacy
  `upgradeTree` JSON before the runtime aggregate is authoritative;
- use an integer micro-Favor conversion with a documented scale and checked rounding policy; do not
  carry `f64` spendable authority into the new ledger;
- create one `FavorLedger` credit event with a stable migration event ID derived from world ID,
  colony ID, source schema, source fingerprint, and target schema;
- preserve every already owned legacy node/study in `ResearchPurchaseState.owned_studies` or the
  explicit legacy-owned-study mapping chosen by LAI.19;
- write a duplicate-conversion guard before commit and reject any future startup that sees an
  inconsistent marker/event pair;
- zero or retire legacy spendable authority after the marker is complete, while keeping old columns
  only as inert historical fields until the global cutover removes them; and
- reject negative, NaN, infinite, overflowed, duplicate, or stale legacy currency inputs without
  minting partial Favor.

After migration, `resources.blessings`, `globalUpgradePoints`, and `upgrade_tree.research_points` must
not be authoritative spendable paths. Existing columns may remain for rollback-free compatibility, but
production mutations must read/write only `FavorLedger` and research leaf state.

## Validation and quarantine rules

The migration/load validator should fail closed for:

- unknown schema versions, newer save versions, unsupported downgrades, and partial markers;
- malformed JSON or unknown fields in required LAI runtime documents;
- duplicate stable IDs in cats, tasks, sites, reservations, Favor events, research purchases,
  prosthetic items, diplomacy pairs, trade contracts, or idempotency receipts;
- dangling references between tasks, intents, workers, cats, sites, cargo, reservations, Shrines,
  prosthetics, trade contracts, and colonies;
- negative Favor, impossible ledger chains, unbounded receipt/result payloads, or duplicate conversion
  events;
- hidden projection fields such as exact hidden stock, exact regeneration below report level 4,
  unseen threat, private foreign beliefs/plans, or rejected hidden amounts;
- impossible task/site stages, objective-less active tasks, non-spatial Hunt/Water, non-3x3 Workshop
  footprints, negative cargo, invalid endpoints, and unrecoverable legacy site metadata; and
- cross-colony private references or reservation conflict rows that expose a competing colony to the
  wrong owner.

Recoverable malformed legacy site metadata may become an explicitly blocked legacy task only when the
blocked task is bounded, visible, non-authoritative for hidden truth, and restart-equal. Required rows
that cannot be safely bounded must roll back or quarantine the complete save migration.

## Restart equality tests

LAI.26 green evidence should include byte/exact save-load-save equality and partition twins at these
stages:

- fresh default world before and after first authoritative tick;
- migrated save before startup, after migration, and after marker replay;
- visible task resolve, reserve, travel, work, deposit, cancel, blocked, and salvage;
- Shrine offering source, haul, deposit, ritual credit, cancel, omission, and cargo salvage;
- Favor event replay and duplicate idempotency retry;
- research player purchase, automatic quota use, unaffordable rejection, Insight production,
  preparation assignment, discount consumption, and scholar reassignment/death;
- Divine Boost activation, same-type rejection, active tick, expiry boundary, and restart after
  expiry;
- treatment, injury recovery, prosthetic fitting, repair, wear, breakage, death recovery, and trade
  transfer;
- diplomacy proposal, approval, block, stale action, and public relationship projection;
- trade proposal, consent, escrow, pickup, delivery, failure, return, stranded cargo, and recovery;
- malformed-row rollback/quarantine and subsequent startup; and
- accepted and rejected idempotency replay after restart with no duplicate Favor, reservation, cargo,
  item, diplomacy, trade, report, event, or prosthetic mutation.

Use canonical JSON bytes and explicit transition fingerprints as assertions. A test that reconstructs
runtime state from legacy columns on every load is not sufficient because it can mask lost planner
history, receipts, stale choices, quota windows, or cargo stages.

## Multi-colony isolation

Persistence IDs need explicit scope:

- colony-scoped IDs remain unique inside `(worldId, colonyId)` and must not be interpreted globally;
- world-scoped reservation IDs must include the world/save identity and the owning colony internally,
  while conflict DTOs expose only bounded public hints;
- public diplomacy/trade relationship facts may be world-level, but each colony's beliefs, plans,
  hidden stock, reports, officers, cats, tasks, inventory, and private action receipts stay
  colony-owned;
- `ownerPlayerId` and authentication/session material remain server-owned persistence state and never
  enter protocol snapshots or quarantined public diagnostics; and
- selected-colony load/save equality should be proven with a foreign colony carrying private sentinel
  values that never appear in the selected colony runtime, snapshot, conflicts, or logs.

## SQL and API gap inventory

Observed production gaps to close in the LAI.26 slice:

- `persistence.rs` has no `LAI26_SCHEMA_VERSION` constant, no durable
  `leader_ai_migration_marker`, no in-progress/complete marker lifecycle, and no explicit newer-save
  rejection.
- `persistence.rs` has no `leaderAiRuntime` column or runtime table. `load_colony` currently assigns
  `LeaderAiRuntimeState::default()` for every loaded colony.
- `save_colony` does not serialize `leader_ai_runtime`, runtime transition fingerprints, or bounded
  idempotency receipts.
- There is no `leader_ai_quarantine` table or bounded quarantine API for malformed required rows.
- There is no exact `convert_legacy_upgrade_points_and_research_points_to_favor` migration, no
  duplicate conversion marker, and no post-migration assertion that legacy research/blessing currency
  is inert.
- Existing column-add migrations are not one versioned world/save migration boundary; they update rows
  directly during startup and do not record source/target fingerprints.
- Current server action routing does not persist LAI.25 accepted/rejected action results
  transactionally with mutations, so restart-safe idempotency is not yet possible.
- `LeaderAiRuntimeState::RuntimeIdempotencyReceipt` currently stores only ID and tick bounds. LAI.26
  needs a persisted bounded result/error DTO or a server-owned receipt table that can replay exact
  accepted and rejected LAI.25 results after restart.
- Prosthetic, standing-order, treatment, and some action-version APIs still need explicit version
  clocks before persistence can enforce expected-version conflicts for those domains.
- The migration validator needs a public persistence-facing API that can validate aggregate
  references against loaded legacy rows before committing.

## Minimal production ownership slices

1. Persistence schema slice, owned by LAI.26:
   `crates/cat-server/src/persistence.rs` plus focused cat-server tests. Add the schema constant,
   marker/quarantine/receipt storage, runtime serialization, exact migration transaction, and
   load/save validation.

2. Runtime aggregate slice, owned by the sim leaf owners when needed:
   add only missing version/result fields or validation helpers to `cat-sim` leaf modules. Do not add a
   second planner or mutation path.

3. Protocol/server action slice, owned by LAI.25/LAI.27:
   define bounded action result DTOs and route them through the authoritative server pipeline. LAI.26
   should persist the result shape but not invent protocol payloads.

4. World-tick cutover slice, owned by LAI.23:
   after persistence exists, remove reconstruction fallbacks and ensure the single ordered runtime path
   consumes the durable aggregate.

5. Journey/evidence slice, owned by LAI.33 and campaign owners:
   add deterministic SQLite fixtures, checksum checkpoints, and signed restart journeys that exercise
   every persisted stage.

## Safe extension steps

For future workshops, resources, task kinds, or action domains:

1. Add a stable ID namespace and schema version before production data exists.
2. Define whether the state is world-scoped, colony-scoped, task-scoped, or item-scoped.
3. Add strict serde bounds and `deny_unknown_fields` for any JSON boundary.
4. Add migration defaults for old saves that are valid records, not lazy `NULL` interpretation.
5. Add reference validation against cats, colonies, tasks, sites, reservations, cargo, items, and
   ownership before commit.
6. Add idempotency receipt fields for every accepted or rejected mutation effect.
7. Add byte/exact restart equality tests at active, blocked, completed, failed, and recovery stages.
8. Add downgrade rejection and newer-version tests.
9. Add multi-colony private sentinel tests if any state can mention another colony.
10. Update this map or the maintained extension docs with the table/column/serialization boundary.

## Focused validation commands

The LAI.26 production owner should eventually run:

```bash
cargo test -p cat-server --test lai26_persistence_migration_contract --no-fail-fast
cargo nextest run -p cat-server --test lai26_persistence_migration_contract --no-fail-fast
cargo clippy -p cat-server --tests -- -D warnings
cargo fmt --all --check
git diff --check
```

Before LAI.23-LAI.25 production lands, some of these are expected to stay red for missing DTOs,
schema constants, migration APIs, and durable runtime persistence. LAI.26 should not weaken those
assertions or add fake passing shims.
