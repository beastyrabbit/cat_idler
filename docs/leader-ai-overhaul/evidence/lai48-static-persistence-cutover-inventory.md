# LAI.48 / LAI.65 Static Persistence-Cutover Inventory

Recorded: 2026-07-25

This is a source-only inventory prepared while LAI.46 owns the simulation hot roots. It is not
acceptance evidence. No database was opened, mutated, migrated, reset, or regenerated, and no
Cargo, test, build, formatter, Clippy, server, browser, or image command was run.

## Foundation already present

- `leader_ai_persistence.rs` declares a canonical persistence schema version `2` and requires an
  exact runtime schema version.
- A persisted world with no canonical marker is rejected rather than semantically converted.
- Future persistence/runtime versions, malformed runtime JSON, wrong-colony partitions, dangling
  rows, and transition-fingerprint mismatches have fail-closed paths.
- Canonical replay, Hole-click rate, session, and signed test-reset rows use typed codecs and are
  committed through the canonical server boundary.
- `save_world_with_canonical_boundary` is intended to commit the simulation mutation and its
  boundary receipts in one SQLite transaction.
- Canonical reset challenges are typed, selected-colony-bound, expiring records rather than
  bearer secrets.

These are useful foundations, but they do not prove a fresh, complete, single-authority schema.

## Remaining contradictions and incomplete cutover

### One opaque runtime aggregate is still the persistence authority

`leader_ai_colony_runtime` stores one `runtimeJson` field and one whole-runtime fingerprint per
colony. The final plan explicitly requires all named aggregates to be persisted outside a Leader
fingerprint. The current row hides whether planner goals, beliefs/reports, officer expertise,
requests, task stages, reservations, cargo, Notes/Void, research lanes, quality lots, food,
Hunting, construction, families, governance, storage, divine state, diplomacy, and barter were
independently stored, bounded, versioned, and reconciled.

LAI.48/LAI.65 therefore still need explicit aggregate tables or equivalently explicit typed rows
whose ownership, version, partition, identity, and referential constraints are independently
inspectable. A hash of one JSON blob is integrity evidence, not the required storage model.

### The outer schema is still the historical compatibility schema

`persistence.rs` continues to create the old broad tables and legacy columns, including:

- `lastTitheAt`;
- `lastOfferingAt`;
- `globalUpgradePoints`;
- `ritualRequestedAt`;
- `coin`;
- legacy trader columns and JSON;
- generic resource/stock/item JSON surfaces;
- old upgrade-tree and upgrade-level state;
- compatibility `ALTER TABLE ... ADD COLUMN` repair logic.

These columns are not harmless history while they remain in the fresh production schema. They can
still be loaded, saved, or mistaken for authority. The pre-production cutover requires constructing
the new gameplay schema directly and deleting obsolete gameplay columns and compatibility
migrations, rather than creating the old schema and layering a canonical aggregate beside it.

### Legacy migration names and tests still encode the replaced design

- Public compatibility names such as `LAI26_SCHEMA_VERSION`,
  `begin_lai26_world_migration_transaction`, and `migrate_lai26_legacy_world` remain.
- `load_world` still branches through the LAI.26 migration-shaped API when the marker is missing,
  even though the implementation rejects conversion.
- Historical LAI.26 tests still require conversion of research/upgrade balances into Favor and
  migration of legacy Shrine-era systems.

Those tests describe the superseded cutover and cannot remain acceptance authority for the final
fresh schema. They need an explicit disposition: replace their reusable restart/fail-closed
assertions with canonical tests, then remove the semantic-migration expectations and production
compatibility API.

### Fresh fixture evidence is still missing

The final acceptance artifact must record and verify:

- fresh gameplay schema version;
- protocol version and snapshot/action schema versions;
- deterministic world seed;
- fixture account/identity scope;
- checksum of the generated SQLite fixture;
- selected global and personal colonies;
- all required canonical aggregates and receipts;
- restart equality and multi-colony isolation;
- production reset rejection;
- test-only signed two-step reset.

The fixture must be generated from empty gameplay state. It may not be a legacy database upgraded
in place.

## Reset and identity scope

The later integrated-plan conflict decision controls:

- Known gameplay state and known obsolete gameplay schema are recreated from empty state.
- Unknown, malformed, partial, or future gameplay state fails closed.
- Test-only signed reset may recreate the selected gameplay colony only after its two-step
  confirmation and idempotency/rate checks.
- Production hides and server-side rejects reset.
- Unrelated authentication/identity metadata required to preserve valid sessions is not silently
  deleted by a gameplay reset.
- Fixture identities and their metadata are regenerated as part of the authoritative test fixture,
  with the exact scope and checksum recorded.

This avoids both unsafe “delete every database table” behavior and compatibility migration of
obsolete gameplay meaning.

## Required aggregate inventory

The persistence owner must account for, version, partition, and restart-test at least:

1. world and colony identity/clock;
2. cats, anatomy, prosthetics, attributes, skills, XP, traits, affinities, and willingness;
3. partnerships, dual-parent lineage, homes, teaching, traditions, surnames, and enterprises;
4. Leader/officer appointments, expertise, goals, dependencies, reports, requests, standing
   orders, omissions, nudges, and receipts;
5. exact physical tasks, complete geometry, assignments, routes, reservation claims, cargo,
   salvage, blockers, and terminal receipts;
6. world-scoped cross-colony reservation authority;
7. content manifests and stable definition versions;
8. quality lots, exact items, provenance, durability, augmentations, fixtures, and storage
   locations/containers;
9. typed food, permissions, nutrition, spoilage, Apple trees, Fish habitats, Cookhouse batches,
   Fishing Huts, farms, and founding resources;
10. Hole footprint, axes, feed/upgrade pipelines, contribution receipts, and micro-Void;
11. Hunting Lairs, creature ecology, encounters, named materials, and recovery;
12. Notes, Void, God and Leader research lanes, preparations, permits, and boost entitlements;
13. construction blueprints, sites, stage bills, delivered/in-transit/missing material identity,
    work progress, miracles, and art state;
14. governance ballots, God backing, succession, expulsion cleanup, and departure work;
15. diplomacy stances, barter proposals, escrow, caravans, authorization, and recovery;
16. canonical server action replay, version lanes, rate-limit state, sessions, reset challenges,
    and receipts;
17. bounded report-safe event and diagnostic state needed across restart.

No row may use a legacy name as an alternate write/read authority for one of these aggregates.

## Ordered implementation boundary

1. LAI.46 finishes the physical simulation state and receipts.
2. LAI.47/LAI.64 freezes the one canonical wire projection/action surface.
3. LAI.48/LAI.65 replaces the historical gameplay schema with explicit canonical aggregates,
   routes canonical actions transactionally, and generates the fresh fixture.
4. LAI.50/LAI.66–LAI.68 consume only the canonical server boundary.
5. LAI.51/LAI.69 run serialized restart/isolation/fixture/browser evidence.
6. LAI.52/LAI.70 delete the obsolete schema, migration API, fixtures, tests, and runtime routes
   after the new path owns every consumer.

## Corrected Opus 5 source audit

The supervised Opus 5 audit on 2026-07-25 read both locked plans, the board from line 1, this
inventory, the server persistence/action/reset/fixture roots, the canonical runtime, and the
canonical plus legacy protocol roots. It made no repository edits and ran no Cargo, compiler,
test, build, lint, formatter, database, server, browser, image, or validation command. Its
conclusions are static findings, not acceptance evidence.

### What is genuinely implemented

- The canonical boundary tables for replay, Hole-click rate state, sessions, signed reset
  challenges, and quarantine exist in `leader_ai_persistence.rs:160-195`.
- The signed two-step selected-colony reset performs HMAC, session, ownership, expiry, rate, and
  idempotency checks before confirmation.
- Production-mode action handling has an explicit server-side
  `SignedTestResetDisabled` rejection.
- The intended save boundary wraps the simulation mutation and boundary receipts in one SQLite
  transaction.

This boundary layer is useful, but it is only one of the seventeen persistence families required
above.

### Confirmed domain-persistence violations

1. `leader_ai_colony_runtime` remains one `runtimeJson` row per colony
   (`leader_ai_persistence.rs:150-158`). The blob contains planner, beliefs, reports, officer
   requests, intents, scheduling, cats, families, governance, research, construction, storage,
   Hole, divine state, boosts, trade, physical state, prosthetics, directives, outcomes, receipts,
   and diagnostics (`leader_ai_runtime.rs:494-537`). Consequently all domain state remains inside
   the Leader runtime fingerprint, contrary to P1.34 and LAI.48.
2. The transition fingerprint is only a clone of the runtime fingerprint
   (`leader_ai_persistence.rs:676-682`). The stored world transition fingerprint is shape-checked,
   but not recomputed and compared on load (`:283-289`, `:699-715`).
3. Every domain shares a single 1 MiB JSON limit (`leader_ai_persistence.rs:154`), and every
   periodic save deletes and rewrites every runtime blob (`:326-348`; `persistence.rs:725-729`).
4. Required domain families have zero, two, or three authorities:
   - cats exist in legacy `cats` plus runtime physical/prosthetic state;
   - officers exist in `colonies.officers` plus runtime state;
   - tasks exist in legacy `jobs.metadata` plus runtime scheduling;
   - storage exists in legacy colony JSON plus runtime storage;
   - typed food/farms/fish exist in legacy colony/world fields plus runtime state;
   - research exists in legacy upgrade fields plus runtime research;
   - construction exists in legacy building fields plus runtime construction;
   - governance exists in legacy election/vote tables plus runtime governance;
   - trade exists in coin/trader/caravan fields plus runtime trade;
   - diagnostics exist in `events` plus runtime diagnostics.
5. No separate world-scoped reservation table exists. The world ledger is stored as per-colony
   mirrors and reconstructed, so restart equality is not backed by one persisted world authority.
6. Content manifests have no persistence aggregate, and the Hunting aggregate is absent from the
   runtime layout inspected by the audit.

### Confirmed schema-cutover violations

- Fresh schema creation still includes obsolete Shrine/Favor-era, generic-currency, generic-item,
  upgrade, trader, and station-progress columns, including `lastTitheAt`, `lastOfferingAt`,
  `globalUpgradePoints`, `ritualRequestedAt`, `coin`, trader JSON, `items`, upgrade trees/levels,
  and trade offers/caravans (`persistence.rs:87-152`).
- Compatibility `ALTER TABLE ... ADD COLUMN` repair code remains live
  (`persistence.rs:338-419`), as do semantic Mill, Workshop, Smelter, Wood Cutter, Stone Prep, and
  recipe migrations (`:424-611`).
- `open_database` initializes and mutates that historical schema before the canonical marker is
  classified (`persistence.rs:51-62`, `:315-316`, marker check at `:752`). Unknown or future data
  can therefore be altered before being rejected.
- Missing markers still route through `migrate_lai26_legacy_world`, which only returns an opaque
  error (`persistence.rs:825-830`; `leader_ai_persistence.rs:310-318`). There is no
  known-obsolete recreate path.
- LAI.26 compatibility names remain the public production persistence API, including marker,
  migration transaction, load, dangling-row, and quarantine functions.
- Future, older, malformed, bad-status, and bad-fingerprint states collapse to the same SQL error
  instead of distinct known-obsolete versus unknown/future/malformed outcomes.
- Strict exact-column enforcement covers only the marker and blob tables, not the gameplay
  tables.

### Reset and identity boundary

- The signed selected-colony reset is real, but the deletion batch does not clear that colony's
  canonical replay, Hole-rate, or quarantine rows (`persistence.rs:685-699` versus
  `leader_ai_persistence.rs:160-195`).
- `player_names` and canonical session rows survive only incidentally. No explicit preservation
  contract names them.
- Because identity/session rows share the gameplay database, GUI-C11 requires a table-scoped
  gameplay recreation with an explicit preserve set; a blind file deletion would destroy
  unrelated identity/session metadata.
- The production/test decision is currently driven by an environment-derived flag. The final
  boundary must ensure a publicly bound production server cannot expose destructive reset merely
  because a test environment variable is present.

### Version and fixture violations

- The canonical protocol defines thirteen lanes, but Hunting, content manifest,
  quality/inventory instance, care/anatomy, and family lanes are missing, and no lane is persisted
  across restart.
- The older LAI.25 expected-version structure and LAI.24 snapshot schema remain exported beside
  the canonical v3/schema-2 surface.
- `leader_ai_journey.rs` is not exported by `cat-server` and therefore its fixture writer is an
  orphan; the example that imports it is compile-shaped inconsistent with `lib.rs`.
- The fixture records persistence version `1` while the loader requires version `2`, gives the
  same checksum to its before/after fields, omits the required schema/action/snapshot/account/
  aggregate metadata, and still seeds Shrine/Favor migration scenarios.
- Historical LAI.26 tests require removed Favor conversion and migration symbols. Other server
  integration tests and `persistence.rs`'s own test module reference absent
  `shrine_favor`/idempotency symbols or private/unexported server modules. These are static
  compile-shaped failures that the narrow server-library check does not compile.
- The unused legacy LAI.25 text handler retains a second non-canonical save lane and must be
  deleted only after canonical consumers have cut over.

### Dependency-ordered implementation sequence

1. Export the real server library modules once, and make `main.rs` consume them rather than
   compiling separate private copies.
2. Replace the superseded LAI.26 test authority with LAI.48 tests that retain only reusable
   restart, isolation, and fail-closed obligations.
3. Classify the database before any DDL as empty, canonical, known obsolete, or
   unknown/future/malformed. Recreate only known obsolete gameplay tables while preserving the
   explicit GUI-C11 identity/session set; fail closed without mutation for all other invalid
   classes.
4. Extend and verify the canonical marker with gameplay, protocol, snapshot, action, persistence
   row versions, world seed, fixture scope, and a recomputed transition fingerprint.
5. Replace `runtimeJson` with strict per-family aggregate tables and an ordered aggregate
   fingerprint index; add the one world-scoped reservation table.
6. Persist all canonical version lanes, including Hunting, content, quality-instance, care, and
   family.
7. Rename the LAI.26 persistence API to canonical names and delete the legacy migration entry
   points.
8. Delete obsolete gameplay columns and duplicate sim fields only after each new aggregate owns
   every reader and writer.
9. Make selected-colony reset atomically clear its replay/rate/quarantine gameplay state while
   preserving the named unrelated identity/session set, and harden public production rejection.
10. Regenerate the fixture from an empty canonical schema with real versions, world seed,
    account/identity scope, aggregate inventory, distinct checksums, and no Shrine/Favor/migration
    scenario.
11. Add fresh-init, known-obsolete recreate, future/unknown/malformed fail-closed,
    per-aggregate restart, multi-colony sentinel isolation, reset preserve/clear, production
    rejection, and fixture-manifest contracts.
12. After every consumer is canonical, delete LAI.24/25 wire roots, the legacy text handler and
    second save lane, historical LAI.26 schema/tests, and every obsolete column.

LAI.48 and LAI.65 remain `todo`. The boundary tables do not satisfy the required domain cutover,
and none of the static findings above may be presented as compiled or runtime-tested evidence.
