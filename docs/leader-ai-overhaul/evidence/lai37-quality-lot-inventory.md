# LAI.37 Quality Lot Inventory

Date: 2026-07-25

Scope: read-only inventory for LAI.37. This document inventories current and source quality, lot, location, reservation, and conservation paths and defines the smallest LAI.37 module/test boundary. No production code, plans, boards, source manifests, Cargo files, protected source worktrees, tests, or builds were changed or run.

## Authorities Read

- Project operating rules: `AGENTS.md:1-62`.
- Restored Plan 1 source: `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:1-92`, `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:120-190`, `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:240-257`.
- Restored integrated plan: `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:1-260`.
- Main LAI board: `docs/leader-ai-overhaul/BOARD.md:1120-1196`, `docs/leader-ai-overhaul/BOARD.md:1248-1282`.
- Main migration board: `docs/migration/BOARD.md:1-168`.
- Branch merge board and receipt manifest: `docs/branch-plan-merge/BOARD.md:1-221`, `docs/branch-plan-merge/source-transfer-manifest.md:1-221`.
- Current target leaves:
  - `crates/cat-sim/src/content_manifest.rs:1-3321`.
  - `crates/cat-sim/src/items.rs:1-620`.
  - `crates/cat-sim/src/ledger.rs:1-202`.
  - `crates/cat-sim/src/storage.rs:1-665`.
  - `crates/cat-sim/src/stockpiles.rs:1-760`.
  - `crates/cat-sim/src/physical_storage.rs:1-393`.
  - `crates/cat-sim/src/autonomous_trade.rs:108-885`.
  - `crates/cat-sim/src/black_hole.rs:1-717`.
  - `crates/cat-sim/src/construction_stages.rs:1-744`.
  - `crates/cat-sim/src/task_runtime.rs:103-530`.
  - `crates/cat-sim/src/world_reservations.rs:40-614`.
  - `crates/cat-protocol/src/lib.rs:338-704`.
  - `crates/cat-protocol/src/lai24_snapshot.rs:536-560`.
  - `crates/cat-server/src/persistence.rs:870-1258`.
  - `crates/cat-server/src/leader_ai_persistence.rs:230-245`.
- Protected source worktree, read-only:
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/items.rs`.
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/stockpiles.rs`.
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/storage.rs`.
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/black_hole.rs`.
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/hunting_lair.rs`.
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs`.
  - `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-server/src/persistence/black_hole.rs`.

## P1 Rows Routed to LAI.37

The board register states that exact Plan 1 requirements may not be compressed (`docs/leader-ai-overhaul/BOARD.md:1191-1198`). Rows routed to LAI.37 and relevant to this inventory are:

| P1 row | Exact LAI.37 relevance | Origin |
| --- | --- | --- |
| P1.10 | Inventory art uses item silhouette plus material palette/texture; quality and augmentation remain detail text/badges/effects/provenance initially. | `docs/leader-ai-overhaul/BOARD.md:1208-1210` |
| P1.12 | Stable IDs include `PhysicalLotId` and `MaterialInstanceId`; `QualityBand` is Crude/Common/Fine/Superior/Masterwork 0-4; `BulkLotKey = content_id + quality`. | `docs/leader-ai-overhaul/BOARD.md:1211-1212` |
| P1.14 | Bulk stock is keyed by content+quality, physically located, exact items retain instance IDs, ItemInstance fields are fixed, typed slots exist, augmentation eligibility is constrained, and cancellation/death/route loss/restart conserve inputs/outputs. | `docs/leader-ai-overhaul/BOARD.md:1213-1214` |
| P1.15 | Quality applies from gathering to all listed physical stock classes and survives hauling/trade/reservations/Hole/persistence with exact multipliers. | `docs/leader-ai-overhaul/BOARD.md:1214-1215` |
| P1.16 | Production quality formula, skill buckets, tool/fixture/station/complexity/variation inputs, thresholds, gathering variant, fixed-point math, and affected-stat exposure are exact. | `docs/leader-ai-overhaul/BOARD.md:1215-1216` |
| P1.19 | Food complexity table is exact and quality applies after complexity. | `docs/leader-ai-overhaul/BOARD.md:1218-1219` |
| P1.22 | Apple harvest creates quality Apples and persisted regrowth remains report-limited. | `docs/leader-ai-overhaul/BOARD.md:1221-1222` |
| P1.24 | Hole rewards increase with processing, complexity, quality, value, augmentation, and condition; Darkness gates content and quality. | `docs/leader-ai-overhaul/BOARD.md:1223-1224` |
| P1.25 | Hole validation checks authoritative ownership, identity, quality, capability, Darkness, route, reservation, and amount. | `docs/leader-ai-overhaul/BOARD.md:1224-1225` |
| P1.27 | Hunting failure, injury/death, equipment wear, cache overflow, and respawn timing are exact downstream conservation inputs. | `docs/leader-ai-overhaul/BOARD.md:1226-1227` |
| P1.28 | Rare-drop quality bands and deterministic RNG key are exact. | `docs/leader-ai-overhaul/BOARD.md:1227-1228` |
| P1.29 | Every named drop has raw and processed physical state, exact quality/provenance, curated use, Hole Darkness/value, icon, and detail visualization. | `docs/leader-ai-overhaul/BOARD.md:1228-1229` |

## Exact LAI.37 Contract

LAI.37 is the universal quality and physical bulk-lot ledger card. The board requires that every physical stock type carries quality; exact five bands, multiplier table, production formula and thresholds, gathering variant, keyed fixed-point variation, physical locations, lots, instances, slots, no laundering, and cancellation/death/route/restart conservation are covered by red evidence before implementation (`docs/leader-ai-overhaul/BOARD.md:1139-1145`).

### Five bands

The restored Plan 1 stable-ID model defines exactly five quality bands: `Crude(0)`, `Common(1)`, `Fine(2)`, `Superior(3)`, and `Masterwork(4)` (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:145-148`). LAI.36 deliberately does not define `QualityBand`: `crates/cat-sim/tests/lai36_content_catalog.rs:1-5` states quality remains a downstream LAI.37 contract, and `crates/cat-sim/src/content_manifest.rs:212-222` defines stable ID newtypes without a quality enum.

### Exact multiplier table

Plan 1 requires these exact quality multipliers (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:178-185`):

| Band | Food hunger/nutrition | Trade/Hole value | Item effect/durability |
| --- | ---: | ---: | ---: |
| Crude | 80% | 75% | 80% |
| Common | 100% | 100% | 100% |
| Fine | 120% | 130% | 115% |
| Superior | 145% | 170% | 135% |
| Masterwork | 175% | 225% | 160% |

### Exact production score formula

Plan 1 requires the LAI.37 production score to be fixed-point integer and deterministic (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:187-190`, `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:240-257`):

- Weighted input quality: `quality * 1000`.
- Skill modifier:
  - `0..=19 -> -500`.
  - `20..=39 -> 0`.
  - `40..=59 -> +250`.
  - `60..=79 -> +500`.
  - `80..=94 -> +750`.
  - `95..=100 -> +1000`.
- Tool modifier: `(quality + 1) * 100`, or `0` when no tool applies.
- Fixture modifier: `(quality + 1) * 100`, or `0` when no fixture applies.
- Station modifier: `(tier - 1) * 125`.
- Complexity penalty:
  - `raw/simple -> 0`.
  - `prepared -> 250`.
  - `complex -> 500`.
  - `feast -> 750`.
- Keyed deterministic variation: integer range `-250..=250`.
- Thresholds:
  - `< 750 -> Crude`.
  - `750..=1749 -> Common`.
  - `1750..=2749 -> Fine`.
  - `2750..=3749 -> Superior`.
  - `>= 3750 -> Masterwork`.

Gathering substitutes source quality for input quality and omits the complexity penalty (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:187-190`).

### Eligible stock classes

Universal quality applies from gathering onward to Water, Apples, Fish, Meat, Bone, Hide, Logs, Stone, Grain, materials, intermediates, meals, tools, furniture, equipment, and creature drops (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:167-177`). It must survive hauling, trade, reservations, Hole feeding, and persistence (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:167-177`).

The current protocol still treats generic `Food`, `Fish`, and `Preserves` as physical stockpile goods, with only `Blessings` non-physical (`crates/cat-protocol/src/lib.rs:620-693`). Plan 1 instead makes generic stored Food/Fish/Preserves compatibility aliases for later deletion, not stable LAI.37 stock classes (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`, `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:167-177`).

### Location enum needs

Plan 1 requires all bulk physical stock to be keyed by `content_id + quality`, with physical locations covering stockpile, station input, station output, cargo, source, cache, and Hole (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:156-165`). LAI.37 should also allow exact instance locations for equipped, carried, reserved, and broken/incompatible checks because item instance eligibility depends on reservation and equipment state (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:156-165`).

### ItemInstance fields

Plan 1 requires exact equipment, furniture, tools, microscopes, augmentations, fixtures, and rare named drops to retain instance IDs. `ItemInstance` must reference definition, material, quality, durability, location, reservation, equipment slot, and optional augmentation (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:156-165`). LAI.37 should consume LAI.36 stable IDs: `ItemDefinitionId`, `MaterialId`, `PhysicalLotId`, `MaterialInstanceId`, `ContentId`, and `ArtKey` as applicable (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:145-148`; `crates/cat-sim/src/content_manifest.rs:212-222`).

### Augmentation and fixture eligibility

Plan 1 allows one typed augmentation slot per eligible item and one typed fixture slot per station or building. Reserved, equipped, carried, broken, and incompatible targets cannot be augmented (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:156-165`). Current LAI.36 manifest descriptors already model `AugmentationSlot` and `FixtureSlot`, augmentation descriptors, fixture descriptors, compatible classes, and station compatibility (`crates/cat-sim/src/content_manifest.rs:224-329`, `crates/cat-sim/src/content_manifest.rs:398-416`, `crates/cat-sim/src/content_manifest.rs:544-573`), but quality and physical-lot eligibility are intentionally absent.

### Conservation invariants

Plan 1 requires cancellation, death, route loss, and restart to conserve every input/output (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:156-165`). Current conservation paths are partial and scalar:

- Task cargo restart/cancel/death recovery preserves a single `resource_id + quantity` cargo object, but does not preserve lot ID or quality (`crates/cat-sim/src/task_runtime.rs:111-128`, `crates/cat-sim/src/task_runtime.rs:302-530`).
- World reservations atomically reserve cargo capacity by `resource_id + units`, but not quality or lot ID (`crates/cat-sim/src/world_reservations.rs:40-117`, `crates/cat-sim/src/world_reservations.rs:242-258`, `crates/cat-sim/src/world_reservations.rs:483-568`).
- Construction cancels report delivered/in-transit scalar content units and route loss decrements in-transit units without preserving lot identity (`crates/cat-sim/src/construction_stages.rs:357-443`, `crates/cat-sim/src/construction_stages.rs:485-495`, `crates/cat-sim/src/construction_stages.rs:539-563`).
- Autonomous trade escrow and delivery validate exact `resource_id + quantity`, but not quality or lot ID (`crates/cat-sim/src/autonomous_trade.rs:108-205`, `crates/cat-sim/src/autonomous_trade.rs:603-610`, `crates/cat-sim/src/autonomous_trade.rs:734-885`).

## Current Quality Representations

| Authority | Classification | Origin | Current shape | LAI.37 risk |
| --- | --- | --- | --- | --- |
| LAI.36 content manifest | Manifest data | `crates/cat-sim/src/content_manifest.rs:212-222`, `crates/cat-sim/src/content_manifest.rs:347-353`, `crates/cat-sim/src/content_manifest.rs:428-443`, `crates/cat-sim/src/content_manifest.rs:521-539` | Stable IDs include `PhysicalLotId` and `MaterialInstanceId`; acquisition descriptors have `produces_quality`; recipe descriptors have `complexity`; material descriptors have Hole value/darkness and `quality_effect`. | No `QualityBand` by design; LAI.37 must add one single enum rather than duplicating raw `u8` quality in each consumer. |
| LAI.36 content tests | Downstream LAI.37 marker | `crates/cat-sim/tests/lai36_content_catalog.rs:1-5`, `crates/cat-sim/tests/lai36_content_catalog.rs:56-69` | Tests explicitly state quality is downstream LAI.37 and assert stable IDs only. | This is the receipt that LAI.37 may depend on LAI.36 IDs without back-referencing a quality definition in LAI.36. |
| Item value model | Legacy/obsolete quality authority | `crates/cat-sim/src/items.rs:189-209` | `MAX_QUALITY: u8 = 4`; `QUALITY_FACTOR_PCT = [50,100,160,240,350]`; `item_value = base * material_pct * quality_pct / 10000`. | Conflicts with Plan 1 trade/Hole value multipliers `[75,100,130,170,225]`; must be deleted or cut over, not reused. |
| Item instance model | Legacy/obsolete instance authority | `crates/cat-sim/src/items.rs:211-285`, `crates/cat-sim/src/items.rs:287-353`, `crates/cat-sim/src/items.rs:390-467` | `Item { kind, material, quality: u8 }`; weight and durability use separate legacy quality arrays; `ItemInstance` stores item, durability, location, credited, auto_issued, active_job_id. | Competes with required `ItemDefinitionId + MaterialId/MaterialInstanceId + QualityBand + reservation + equipment slot + augmentation`; old weight/durability formulas also conflict with 80/100/115/135/160 item effect/durability multipliers. |
| Protocol item wire model | Handler/wire compatibility | `crates/cat-protocol/src/lib.rs:338-365`, `crates/cat-protocol/src/lib.rs:394-431` | Trader offers and item stack snapshots expose `quality: u8`; item instance snapshots expose kind/material/quality through stacks and location without reservations or augmentation. | Needed for compatibility until protocol LAI.47; LAI.37 should not define a protocol-only quality authority. |
| Physical storage lot model | Handler/prototype leaf | `crates/cat-sim/src/physical_storage.rs:1-67` | `StorageLot { lot_id, content_id: String, compatibility, units, quality_band: u8, produced_at_ms, expires_at_ms, provenance_id, reserved_units }`. | Closest existing lot leaf, but uses raw strings and raw `u8`; must cut over to `ContentId`, `PhysicalLotId`, and LAI.37 `QualityBand` or become an adapter. |
| Black Hole feed value model | Handler with legacy quality | `crates/cat-sim/src/black_hole.rs:230-264`, `crates/cat-sim/src/black_hole.rs:313-368`, `crates/cat-sim/src/black_hole.rs:393-452`, `crates/cat-sim/src/black_hole.rs:604-717` | Item feeds use `Item.quality` and `MAX_QUALITY`; resource feeds are scalar `ResourceKind`; value uses `item.value()` or resource fixed values; max quality gated by darkness axes. | Conflicts with universal trade/Hole value multipliers and lacks resource quality; should consume LAI.37 lot value and darkness eligibility after the leaf exists. |
| Stockpile resources | Legacy/obsolete scalar authority | `crates/cat-sim/src/stockpiles.rs:124-160`, `crates/cat-sim/src/stockpiles.rs:200-290`, `crates/cat-sim/src/stockpiles.rs:716-760` | `ResourceKind` variants map to scalar `Resources` f64 fields; stockpiles hold aggregate `Resources`, no quality. | Physical stock authority conflicts with `BulkLotKey = content_id + quality`; aliases `Food`, `Fish`, `Preserves`, `Blessings` require compatibility-only routing and later deletion. |
| Ledger reports | Handler/report leaf | `crates/cat-sim/src/ledger.rs:1-7`, `crates/cat-sim/src/ledger.rs:23-90`, `crates/cat-sim/src/ledger.rs:178-202` | Reports stock accounting from scalar `Resources`; explicitly says authoritative economy remains `Resources`. | Must become a consumer/report of LAI.37 lots, not a stock authority. |
| Storage capacities | Handler/capacity leaf | `crates/cat-sim/src/storage.rs:12-41`, `crates/cat-sim/src/storage.rs:179-238`, `crates/cat-sim/src/storage.rs:370-412`, `crates/cat-sim/src/storage.rs:621-665` | Scalar f64 capacities and accepted `ResourceKind` classes. | Capacity may stay scalar but must apply to physical lot units by content class; should not own quality. |
| Construction cargo | Handler with scalar physical flow | `crates/cat-sim/src/construction_stages.rs:75-129`, `crates/cat-sim/src/construction_stages.rs:357-443`, `crates/cat-sim/src/construction_stages.rs:485-563` | Cargo lines are `content_id: String` with required/delivered/in-transit/consumed units. | LAI.59 depends on LAI.37; construction cannot be final until it reserves and salvages exact lots. |
| Autonomous trade cargo | Handler with scalar physical flow | `crates/cat-sim/src/autonomous_trade.rs:108-205`, `crates/cat-sim/src/autonomous_trade.rs:603-610`, `crates/cat-sim/src/autonomous_trade.rs:734-885` | Trade cargo wraps `TaskCargo { resource_id, quantity, location }`; escrow validates scalar exactness. | LAI.62 depends on quality/physical barter; current scalar escrow can launder quality. |
| Persistence | Handler/persistence compatibility | `crates/cat-server/src/persistence.rs:870-930`, `crates/cat-server/src/persistence.rs:1113-1158`, `crates/cat-server/src/persistence.rs:1235-1258`; `crates/cat-server/src/leader_ai_persistence.rs:230-245` | Persists `resources`, stockpiles, stock ledger, and `ItemStore` as JSON; validates reservations and restarts but no lot ledger. | LAI.48+ must persist physical lots and item instance fields; current migration may normalize item locations and drop future lot-specific state if reused unchanged. |
| Research manifest | Manifest data / downstream router | `crates/cat-sim/src/research_manifest.rs:1-7`, `crates/cat-sim/src/research_manifest.rs:76-130`, `crates/cat-sim/src/research_manifest.rs:170-173`, `crates/cat-sim/src/research_manifest.rs:231-234` | Marks obsolete Shrine/Favor/generic-food/coin authority; capability families include `universal_quality` and `physical_lot_ledger`; Hole and construction depend on physical lots. | Confirms staged consumer order; should not define quality or lots. |

## Protected Source Quality-Related Inventory

The protected source worktree is covered by `docs/branch-plan-merge/source-transfer-manifest.md:1-221`. The manifest identifies `the-shrine-upgrade` as a frozen source branch with 13 tracked modifications, 69 untracked files, 53 assets, and digest `b1bcc2433d29d23f10167de07465f4c39a7164bc782d9ec292fa8cafe3a4bdaf` (`docs/branch-plan-merge/source-transfer-manifest.md:16-23`). It routes Shrine Hunting leaves and tests to LAI.36-37/42-48 and routes Shrine catalog/upgrade changes to LAI.36/43/44/58 (`docs/branch-plan-merge/source-transfer-manifest.md:213-221`).

Current target copies of `items.rs`, `stockpiles.rs`, `storage.rs`, and `black_hole.rs` are byte-identical to the protected source versions by SHA-256 inspection:

| Leaf | SHA-256 |
| --- | --- |
| `crates/cat-sim/src/items.rs` | `3478106f50810c4ba85e84e971494e838a6c70b84d52c3d2669b5218707e7051` |
| `crates/cat-sim/src/stockpiles.rs` | `ecf2e2b93e035420640ec5af2cfd964372a101c2c0d0715c8027175efe4949b2` |
| `crates/cat-sim/src/storage.rs` | `e137a6c141886f5a348a2ed4adf06b69059ac8d0eafde38fd98319535607b07d` |
| `crates/cat-sim/src/black_hole.rs` | `2fcc3412589d753601bd639ac2240386151b272a1899f8c8df2a52a297296525` |

Source-only hunting leaves are relevant downstream, not LAI.37 authorities:

- `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs:36-61` defines active parties, trophy records, and outcomes.
- `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs:306-355` applies XP, health, deaths, equipment wear, and credits rare material counts in `shared_spatial.hunting_materials` by `SpeciesMaterial`; it does not create quality-bearing physical lots.
- `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs:395-403` defines material darkness requirements for four species materials; no universal quality band is present.
- `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs:532-539` defines autonomous and player-nudge eligibility gates; `hunting_runtime.rs:595-610` releases party reservations.
- `the-shrine-upgrade/crates/cat-server/src/persistence/black_hole.rs:240-280` validates lifetime quantity and feed darkness requirements; for `Item` it also rejects item quality above the current axis maximum. It still does not provide resource quality or Plan 1 value multipliers.

Source transfer receipt: these source leaves are evidence inputs, not direct copy targets. LAI.37 should take exact quality/lot requirements from Plan 1 and the board, then consume source hunting/black-hole behavior later through staged consumers.

## Physical Lot, Location, Reservation, and Conservation Paths

| Path | Classification | Origin | Existing behavior | Gap to LAI.37 |
| --- | --- | --- | --- | --- |
| `PhysicalLotId` and `MaterialInstanceId` stable IDs | Manifest data | `crates/cat-sim/src/content_manifest.rs:212-222`; tested at `crates/cat-sim/tests/lai36_content_catalog.rs:56-69` | Stable ID newtypes exist in LAI.36 content manifest. | Need LAI.37 lot module to use these IDs, not local aliases. |
| `StorageLot` | Prototype handler | `crates/cat-sim/src/physical_storage.rs:13-67` | Local `StorageLotId = String`; `content_id: String`; `quality_band: u8`; `reserved_units`; produced/expiry/provenance metadata. | Missing LAI.36 stable newtypes, closed `QualityBand`, Plan 1 location enum, and persistence contract. |
| `PhysicalContainer` and storage slots | Handler | `crates/cat-sim/src/physical_storage.rs:70-206`, `crates/cat-sim/src/physical_storage.rs:208-291` | Typed containers with capacity, compatibility, mixed/same-kind validation, and visible storage tile slots. | Can become a consumer of LAI.37 lots; should not own lot identity or quality semantics. |
| Workshop input zone link | Handler | `crates/cat-sim/src/physical_storage.rs:293-354` | Validates 3x3 workshop and adjacent storage zones. | Good consumer for station input/output locations after LAI.37; no quality authority. |
| Stockpile aggregate resources | Legacy authority | `crates/cat-sim/src/stockpiles.rs:1-16`, `crates/cat-sim/src/stockpiles.rs:282-290` | Maintains scalar `Resources` aggregate invariant. | Directly competes with lot ledger as authority; needs compatibility aggregation after LAI.37. |
| Gather spots and fish populations | Handler/source | `crates/cat-sim/src/stockpiles.rs:60-93` | Source nodes produce scalar `ResourceKind`; no source quality. | LAI.37 gathering variant needs source quality and deterministic variation. |
| Item locations | Legacy instance authority | `crates/cat-sim/src/items.rs:390-434` | `LegacyTreasury`, stockpile, station input/output, carrier, equipped, trader, caravan. | Missing Plan 1 source, cache, Hole, explicit cargo, reservation, broken/incompatible eligibility state. |
| Item instances | Legacy instance authority | `crates/cat-sim/src/items.rs:440-467` | Instance ID, legacy item, durability, location, credited, auto-issued, active job. | Missing definition ID, material instance ID, `QualityBand`, reservation, equipment slot, augmentation. |
| Task cargo | Handler | `crates/cat-sim/src/task_runtime.rs:111-128`, `crates/cat-sim/src/task_runtime.rs:302-530` | Cargo has id, resource_id, quantity, and coarse location states. | Must carry exact lot IDs and quality; current restart/cancel conservation only proves scalar quantity. |
| World reservations | Handler | `crates/cat-sim/src/world_reservations.rs:40-117`, `crates/cat-sim/src/world_reservations.rs:242-258`, `crates/cat-sim/src/world_reservations.rs:483-568` | Atomic transaction over tool IDs and cargo resources. | Must reserve lot units atomically, preserving `PhysicalLotId + QualityBand`. |
| Construction cargo | Handler | `crates/cat-sim/src/construction_stages.rs:75-129`, `crates/cat-sim/src/construction_stages.rs:357-443`, `crates/cat-sim/src/construction_stages.rs:485-563` | String content IDs and scalar unit counters. | Must prevent quality laundering during reserve, transit, delivery, consumption, loss, and salvage. |
| Trade cargo | Handler | `crates/cat-sim/src/autonomous_trade.rs:108-205`, `crates/cat-sim/src/autonomous_trade.rs:603-610`, `crates/cat-sim/src/autonomous_trade.rs:734-885` | Scalar `resource_id + quantity` escrow and delivery. | Must price, escrow, move, cancel, and recover exact lots. |
| Hole feed orders | Handler | `crates/cat-sim/src/black_hole.rs:142-154`, `crates/cat-sim/src/black_hole.rs:251-264`, `crates/cat-sim/src/black_hole.rs:393-452` | Scalar resources and legacy quality-bearing items. | Must feed quality-bearing physical lots and apply exact trade/Hole multiplier. |
| Persistence | Handler | `crates/cat-server/src/persistence.rs:870-930`, `crates/cat-server/src/persistence.rs:1113-1158`, `crates/cat-server/src/persistence.rs:1235-1258` | JSON persists resources, stockpiles, ledgers, and item store; no physical-lot table/state. | LAI.48+ must persist/restart exact lots and reservations before scalar compatibility can be deleted. |

## Compatibility Aliases for Later Deletion

These identities must remain compatibility aliases until their consumers are cut over; LAI.37 should identify but not delete them.

| Alias | Origin | Current use | Deletion receipt |
| --- | --- | --- | --- |
| Shrine | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`; obsolete fragments in `crates/cat-sim/src/research_manifest.rs:76-108` | Historical domain name for Favor/Blessings/black-hole-adjacent content. | LAI.58 removes obsolete Shrine/Favor/generic-food/coin authority (`crates/cat-sim/src/research_manifest.rs:1-7`). |
| Favor | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`; `crates/cat-server/src/leader_ai_persistence.rs:230-245` | Persistence migration still validates/normalizes old Favor-related state. | Keep migration compatibility until fresh DB/fixtures and LAI.58 deletion receipts are complete. |
| Blessings | `crates/cat-sim/src/stockpiles.rs:124-160`, `crates/cat-sim/src/stockpiles.rs:388-407`; `crates/cat-protocol/src/lib.rs:620-693` | Stockpile resource variant; protocol marks it non-physical; stockpile capacity treats it specially/infinite. | Obsolete Shrine/Blessings references are removed by LAI.58, but LAI.37 must not model it as physical quality stock. |
| Generic Food | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`; `crates/cat-sim/src/stockpiles.rs:124-160`; `crates/cat-protocol/src/lib.rs:659-693` | Scalar physical stockpile good and protocol resource. | LAI.37 should route real food through stable typed food IDs; generic Food becomes compatibility only until LAI.58 deletion. |
| Generic Fish | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`; `crates/cat-sim/src/stockpiles.rs:124-160`; `crates/cat-protocol/src/lib.rs:659-693` | Scalar physical stockpile good and protocol resource. | LAI.37 should route fish species/products through stable IDs; generic Fish becomes compatibility only until LAI.58 deletion. |
| Generic Preserves | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`; `crates/cat-sim/src/stockpiles.rs:124-160`; `crates/cat-protocol/src/lib.rs:659-693` | Scalar physical stockpile good and protocol resource. | LAI.37 should route preserved foods through typed IDs; generic Preserves becomes compatibility only until LAI.58 deletion. |
| Scholar Insight | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:37-48`; obsolete fragments in `crates/cat-sim/src/research_manifest.rs:76-108` | Historical/research-domain compatibility identity. | LAI.58 deletion receipt; LAI.37 should not create quality lots for it. |

## Competing Authorities and Cutover Receipts

1. `items.rs` owns legacy item quality as `u8`, legacy value factors, durability factors, and weight factors (`crates/cat-sim/src/items.rs:189-353`). LAI.37 must replace it for all quality semantics because the factor tables do not match Plan 1.
2. `physical_storage.rs` owns a near-LAI.37 storage lot shape but uses `String` and `u8` (`crates/cat-sim/src/physical_storage.rs:13-67`). It should be a consumer or adapter after a new single authority exists.
3. `stockpiles.rs` and `ledger.rs` still declare scalar `Resources` as the economic authority (`crates/cat-sim/src/stockpiles.rs:1-16`; `crates/cat-sim/src/ledger.rs:1-7`). LAI.37 must not add another scalar aggregation path that can diverge from lots.
4. `black_hole.rs` prices legacy items through `Item::value()` and scalar resources through `resource_unit_value_micros` (`crates/cat-sim/src/black_hole.rs:251-264`, `crates/cat-sim/src/black_hole.rs:604-675`). This is a direct duplicate with Plan 1 trade/Hole value multipliers.
5. `construction_stages.rs`, `task_runtime.rs`, `world_reservations.rs`, and `autonomous_trade.rs` all have independent scalar cargo/reservation conservation paths (`crates/cat-sim/src/construction_stages.rs:75-129`; `crates/cat-sim/src/task_runtime.rs:111-128`; `crates/cat-sim/src/world_reservations.rs:40-117`; `crates/cat-sim/src/autonomous_trade.rs:108-205`). Cutover must stage them onto LAI.37 lot reservations before deleting compatibility scalars.
6. LAI.36 manifest receipts deliberately stop before quality: `crates/cat-sim/tests/lai36_content_catalog.rs:1-5`. This protects LAI.37 from duplicating a future `QualityBand` in the manifest layer.
7. Source-transfer receipts route source hunting and Shrine catalog work into LAI.36/37/42-48 rather than wholesale copying (`docs/branch-plan-merge/source-transfer-manifest.md:213-221`). Hunting rare drops should enter LAI.37 as consumer-produced lots, not as a source-only authority.

## Exact Red Cases for LAI.37

These are the smallest red cases that should fail before LAI.37 exists and should pass at the leaf boundary before consumers are cut over:

1. `QualityBand` is the only quality enum in `cat-sim`, has exactly `Crude`, `Common`, `Fine`, `Superior`, `Masterwork`, and maps to discriminants `0..=4`; no consumer uses raw `u8` as an authority.
2. Food hunger/nutrition multipliers return exactly `80, 100, 120, 145, 175` percent for the five bands.
3. Trade/Hole value multipliers return exactly `75, 100, 130, 170, 225` percent.
4. Item effect/durability multipliers return exactly `80, 100, 115, 135, 160` percent.
5. Production score threshold edges classify `749 -> Crude`, `750 -> Common`, `1749 -> Common`, `1750 -> Fine`, `2749 -> Fine`, `2750 -> Superior`, `3749 -> Superior`, `3750 -> Masterwork`.
6. Skill buckets classify exact edges: 19, 20, 39, 40, 59, 60, 79, 80, 94, 95, and 100.
7. Tool and fixture modifiers use `(quality + 1) * 100`; absent tool/fixture contributes zero.
8. Station tier modifier uses `(tier - 1) * 125`; tier 1 contributes zero.
9. Complexity penalties are exactly raw/simple 0, prepared 250, complex 500, feast 750.
10. Keyed variation is deterministic, fixed-point integer, stable for identical keys, and always in `-250..=250`.
11. Gathering classification uses source quality plus gathering modifiers and keyed variation, and omits the complexity penalty.
12. `BulkLotKey` equality uses exactly `(ContentId, QualityBand)`; two lots with same content but different quality never merge.
13. `PhysicalLotId` and `MaterialInstanceId` are consumed from LAI.36 stable ID newtypes, not redefined as local `String` aliases.
14. `PhysicalLotLocation` covers at least stockpile, station input, station output, cargo, source, cache, and Hole.
15. Lot reservation cannot exceed units and cancellation releases the same lot units with the same quality.
16. Death/recovery after pickup preserves exact lot ID, content ID, quality, units, and provenance into salvage or stranded state.
17. Route loss records exact lost lot units and does not silently decrement only scalar in-transit counters.
18. Restart revalidation preserves already-picked, deposited, salvaged, and stranded lots and blocks only invalid pre-pickup reservations.
19. Augmentation rejects reserved, equipped, carried, broken, incompatible, or already-augmented items; accepts only matching typed slots.
20. Fixture installation rejects reserved, incompatible, or already-fixtured stations/buildings; accepts only matching typed slots and eligible station classes.
21. Generic `Food`, `Fish`, `Preserves`, `Blessings`, `Favor`, `Shrine`, and scholar `Insight` are not valid new quality lot content IDs except through explicit compatibility adapters.
22. Persistence fixture eligibility: a fresh fixture can serialize and reload quality-bearing lots and reservations without losing `PhysicalLotId`, `QualityBand`, location, provenance, or reserved units.

## Smallest Single-Authority LAI.37 Boundary

Recommended leaf: `crates/cat-sim/src/quality_lots.rs`.

Recommended tests: `crates/cat-sim/tests/lai37_quality_lots.rs`.

The boundary should be a pure cat-sim module with no rendering, persistence, networking, filesystem, or clock access. It should import stable IDs from `crate::content_manifest` and define the single LAI.37 `QualityBand` authority locally in this leaf. It should not add `QualityBand` to LAI.36, should not import `items::MAX_QUALITY`, should not copy `physical_storage::StorageLot` raw `quality_band: u8`, and should not back-reference manifest descriptors to derive behavior.

Minimal data/API surface:

- `QualityBand`: closed enum with exactly five variants and explicit numeric mapping.
- `QualityMultipliers`: pure accessors for food/nutrition, trade/Hole value, and item effect/durability.
- `SkillQualityBucket` or pure function for exact skill modifiers.
- `ProductionQualityInput`: weighted input quality, skill, optional tool quality, optional fixture quality, station tier, complexity, keyed variation.
- `classify_production_quality(input) -> QualityBand`.
- `classify_gathered_quality(source_quality, skill/tool/station/keyed inputs) -> QualityBand` or equivalent that omits complexity.
- `BulkLotKey { content_id: ContentId, quality: QualityBand }`.
- `PhysicalLot { id: PhysicalLotId, key: BulkLotKey, units, location, provenance_id, reserved_units, produced_at_tick_or_ms, expires_at_tick_or_ms }`.
- `PhysicalLotLocation`: stockpile, station input, station output, cargo, source, cache, Hole, plus explicit legacy adapter locations only if needed for cutover.
- `LotReservation`: exact lot ID and units with validation `reserved <= units`.
- `QualityItemInstance` or a forward-compatible instance contract referencing `ItemDefinitionId`, optional `MaterialId`, optional `MaterialInstanceId`, `QualityBand`, durability, location, reservation, equipment slot, and optional augmentation.
- `AugmentationEligibility` and `FixtureEligibility` pure predicates consuming LAI.36 `AugmentationSlot`, `FixtureSlot`, `ItemClass`, and station/building IDs, without owning manifest data.

Initial LAI.37 should stop at leaf behavior and tests. Consumer rewrites should come later so the module remains the single authority and the red tests can lock the exact formulas before stockpiles, tasks, trade, Hole, construction, persistence, and protocol begin cutover.

## Staged Consumer Order

1. Add pure `quality_lots` leaf and LAI.37 tests for formulas, IDs, keys, locations, reservations, and eligibility.
2. Adapt `physical_storage.rs` to consume `QualityBand`, `BulkLotKey`, and `PhysicalLotId`, keeping existing visible storage/container validation as a consumer.
3. Adapt `stockpiles.rs`, `ledger.rs`, and `storage.rs` to aggregate/report/cap lots without owning quality; keep scalar compatibility until all downstream consumers read lots.
4. Adapt `task_runtime.rs` and `world_reservations.rs` so reservations and cargo carry exact lot IDs and quality-preserving units.
5. Adapt `construction_stages.rs` so stage bills, reserve/begin transit/deliver/lose/cancel/work-start paths conserve lots and quality; this feeds LAI.59.
6. Adapt `autonomous_trade.rs` so escrow, delivery, cancel, route failure, and barter pricing preserve and value exact lots; this feeds LAI.62.
7. Adapt `black_hole.rs` so feed candidates consume quality-bearing physical lots and apply Plan 1 trade/Hole multipliers; source black-hole persistence validation should become a downstream persistence consumer.
8. Adapt `items.rs` so exact instances consume `QualityBand`, LAI.36 definition/material IDs, reservations, equipment slots, augmentation slots, and new item durability/effect multipliers; delete or quarantine legacy `QUALITY_FACTOR_PCT` after consumers are moved.
9. Adapt protected-source hunting consumers so rare drops and trophies are produced as LAI.37 lots/material instances rather than source-only scalar material counts.
10. Adapt protocol snapshots/actions after sim contracts settle; avoid protocol becoming a second quality authority.
11. Adapt persistence/restart fixtures to save/load exact lots, item instances, locations, reservations, and compatibility aliases; only then delete generic Food/Fish/Preserves/Shrine/Favor/Blessings/scholar Insight compatibility paths under LAI.58.

## Top Risks

- Highest risk: there are already three quality authorities with incompatible semantics: `items.rs` raw `u8` quality and value arrays, `physical_storage.rs` raw `quality_band: u8`, and Plan 1 exact multipliers/formula. LAI.37 needs a single closed `QualityBand` or every downstream consumer can launder quality through a different formula.
- Stockpiles, ledger, task cargo, reservations, construction, autonomous trade, black-hole feeds, protocol, and persistence are still scalar `ResourceKind`/`Resources` paths. They preserve quantities in some routes, but not exact lot identity, quality, or provenance.
- Generic `Food`, `Fish`, `Preserves`, `Blessings`, Shrine/Favor, and scholar Insight still appear as compatibility identities. LAI.37 must not promote them into new quality-bearing manifest stock classes; deletion belongs to later cutover receipts.
- Protected source hunting code produces rare material counts and has death/reservation behavior, but no quality-bearing lots. Its transfer should be semantic and staged after LAI.37 so hunting outputs enter the unified lot authority rather than creating a second trophy/material ledger.
- Persistence currently serializes aggregate resources, stockpiles, ledger, and legacy item store JSON. Restart/cancel/route/death invariants cannot be considered LAI.37-complete until persistence fixtures prove exact lot IDs, quality, locations, reservations, and provenance survive reload.
