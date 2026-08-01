# Construction-miracle Hole value authority

Status: canonical simulation leaf; server, protocol, client, persistence migration, and live action
routing remain integration work.

## Why this authority exists

The repeatable construction miracle spends exactly one Void Insight, creates an exact package of
still-missing construction inputs worth twice the Hole feed value required to earn one Void
Insight, and removes ten percent of the project's original labor duration. That package must be
derived from one deterministic authored value authority. A request, server adapter, trader,
coin-price table, UI, or caller must not supply an alternative economic table.

`content_manifest.json.construction_miracle_inputs` is that authority. It is a closed, sorted table
covering every content ID used by the canonical staged-construction catalog. The manifest
validator rejects missing, extra, duplicate, out-of-order, wrong-class, or numerically invalid
rows before the manifest can be used.

One Void Insight is `1,000,000` micro-units. The construction-miracle package target is therefore
exactly `2,000,000` micro-value. The package composer may use no more than each bill line's
currently missing quantity, and it succeeds only when eligible inputs can make the target exactly.
There is no rounding, overfill, stock return, trade, or Hole-feed fallback.

## Complete construction-input classification

| Construction content ID | Physical class | Authored base | Value stage | Darkness | Common unit value | Miracle generation |
|---|---|---:|---|---:|---:|---|
| `fixture_research` | fixture | 1,700 milli | raw, 100% | 7 | 1,700,000 micro | Eligible typed fixture item (`griffin_plume`) |
| `fixture_storage` | fixture | 1,700 milli | raw, 100% | 7 | 1,700,000 micro | Eligible typed fixture item (`bear_pelt`) |
| `fixture_workshop` | fixture | 1,700 milli | raw, 100% | 7 | 1,700,000 micro | Eligible typed fixture item (`stag_antler`) |
| `item_bowl` | exact item | 1,700 milli | raw, 100% | 4 | 1,700,000 micro | Eligible exact item (`boar_tusk`) |
| `item_furniture` | exact item | 1,700 milli | raw, 100% | 4 | 1,700,000 micro | Eligible exact item (`bear_pelt`) |
| `item_generic_tool` | exact item | 300 milli | raw, 100% | 4 | 300,000 micro | Eligible exact item (`warg_fang`) |
| `resource_blocks` | bulk lot | 240 milli | processed, 125% | 4 | 300,000 micro | Eligible |
| `resource_cloth` | bulk lot | 240 milli | processed, 125% | 6 | 300,000 micro | Eligible |
| `resource_gem` | bulk lot | 500 milli | raw, 100% | 7 | 500,000 micro | Eligible |
| `resource_logs` | bulk lot | 100 milli | raw, 100% | 2 | 100,000 micro | Eligible |
| `resource_lumber` | bulk lot | 240 milli | processed, 125% | 4 | 300,000 micro | Eligible |
| `resource_metal` | bulk lot | 240 milli | processed, 125% | 6 | 300,000 micro | Eligible |
| `resource_planks` | bulk lot | 240 milli | processed, 125% | 4 | 300,000 micro | Eligible |
| `resource_refined` | bulk lot | 240 milli | processed, 125% | 6 | 300,000 micro | Eligible |
| `resource_stone` | bulk lot | 100 milli | raw, 100% | 2 | 100,000 micro | Eligible |

The three physical classes are intentionally different:

- A **bulk lot** is divisible quantity with lot identity and may be materialized with
  `StorageCompatibility::BulkMaterial`.
- An **exact item** receives one stable `MaterialInstanceId` per unit and lives in the canonical
  item side of `QualityLotLedger`. It uses Common quality, full durability, its manifest item
  definition and augmentation compatibility, and its manifest-authored generated material.
- A **fixture** receives the same canonical exact-item identity shape, using the fixture definition,
  fixture material, and unique-item storage compatibility. It is purpose-bound construction input,
  not an already-installed station fixture.
- **Ineligible** is available for a future construction bill identity which is canonical content
  but must not participate in miracle generation. No current bill uses this disposition.

The current composer includes bulk, exact-item, and fixture rows in the same exact package search
and filters only explicitly ineligible rows. If missing quantities cannot compose exactly
`2,000,000` micro-value, the command fails closed. Basic Bowl/Furniture plus one Cloth and developed
Fixture/Furniture plus one Tool are deliberately exact `1,700,000 + 300,000` packages. It does not
create a different item, invent a request-time value, or materialize a typed input as bulk cargo.

## Value formula and protected-branch reconciliation

Both ordinary canonical Hole-feed callers and construction miracles resolve
`CatalogResolvedFeedPolicy` from the manifest, then use its existing fixed-point quality formula:

```text
micro value =
  (base milli + installed augmentation milli)
  × 1,000
  × stage percent
  × quality percent
  × current condition
  ÷ (100 × 100 × maximum condition)
```

Construction composition uses Common quality, no augmentation, and full `1/1` condition. This
produces the values in the table above.

The protected `the-shrine-upgrade` branch authored the bulk Common unit values and Darkness
thresholds: Logs/Stone `100,000` at Darkness 2; Lumber/Planks/Blocks `300,000` at Darkness 4;
Refined/Cloth/Metal `300,000` at Darkness 6; and Gem `500,000` at Darkness 7. The approved current
Hole model also applies a 125% processed-stage multiplier. Therefore the centralized manifest
authors processed inputs at base `240` milli so the shared resolver preserves the protected final
`300,000` value (`240 × 1,000 × 125%`) without double-counting processing. This base-240
reconciliation is the exact new assumption; it replaces no trader price and introduces no coin
authority.

The protected branch did not own values for purpose-bound exact construction items and fixtures.
The centralized authored assumption is therefore `1,700,000` micro for Bowl, Furniture, and each
fixture, and `300,000` micro for the generic Tool. These values make every current level-one fit-out
shape exactly two million without creating excess cargo. The generated material identities are
also centralized: Bowl→Boar Tusk, Furniture→Bear Pelt, Tool→Warg Fang, Research Fixture→Griffin
Plume, Storage Fixture→Bear Pelt, and Workshop Fixture→Stag Antler. Fixture materials match their
canonical fixture descriptors; the three ordinary item materials are explicit authored
construction-miracle assumptions, not trader prices.

## Runtime ownership

- `content_manifest.rs` owns strict decoding, closed coverage, physical classification, authored
  base/stage/Darkness values, and deterministic lookup.
- `black_hole.rs::resolve_manifest_hole_feed_policy` combines those authored fields with dynamic
  capability, ownership, reservation, route, quality, augmentation, and condition facts.
- `CatalogResolvedFeedPolicy::micro_void_for` remains the single fixed-point value calculation.
- `construction_miracle_runtime.rs` derives Common per-unit values from that resolver, composes the
  exact bounded package, and delegates each output to the manifest-classified materializer.
- `leader_ai_runtime.rs` deposits bulk as physical lots and deposits every exact item/fixture unit
  as a stable canonical `ItemInstance`; all are reserved and purpose-bound to one project/stage.
- `construction_runtime.rs` counts, stages, and consumes lots and items through the same
  `StorageAuthority::Consume` transaction, rejecting a purpose bound to any other project/stage.
- The Void ledger owns the one-Void debit. The construction project and its bound storage own
  missing quantities, stage identity, and labor credit. No module here creates a shadow ledger,
  bill, inventory, or price table.

## Adding a new construction input later

Adding a new building, workshop, upgrade, or bill line is incomplete until the new input follows
this sequence:

1. Give the content one canonical stable `ContentId` in the appropriate manifest catalog.
2. Add the ID to a canonical construction blueprint and keep stage bill ordering deterministic.
3. Add exactly one sorted `construction_miracle_inputs` row.
4. Select its real physical class: bulk lot, exact item, fixture, or explicitly ineligible.
5. For a bulk lot, exact item, or fixture, author positive `base_value_milli`, value stage, and
   Darkness `0..=10`. Derive the intended Common final value through the shared formula; do not
   copy a trader/coin price.
6. For an exact item or fixture, author one canonical `generated_material_id` and confirm the
   referenced item/fixture descriptor. Bulk rows have no generated material; ineligible rows have
   neither value policy nor generated material.
7. Extend the closed `CONSTRUCTION_MIRACLE_INPUT_IDS` coverage list. Manifest validation must fail
   until the catalog and classification set agree.
8. Confirm typed generation uses one stable item identity per unit, correct compatibility,
   provenance receipt, reservation, project/stage purpose binding, consumption, recovery,
   persistence, and report projection. Do not convert the row to a bulk lot.
9. Add focused manifest/resolver/composition tests, then restart, protocol, persistence, client,
   and browser evidence when the corresponding integration cards own those layers.
10. Update this table, the implementation board, and the general extension guide with the new
    identity, value reasoning, physical semantics, and remaining gaps.

## Remaining route gap

Typed lot/item/fixture materialization and matching-stage consumption now exist inside the atomic
simulation transaction and persisted aggregate. The remaining gap is the previously recorded
LAI.64–LAI.68 route: authenticated protocol action, server authorization/rate/replay adapter,
snapshot projection, client control/state, and browser evidence. The generic visible-task cargo
binding still describes scalar resource cargo and does not independently haul exact item
identities; miracle outputs bypass that ambiguity by being deposited directly into the matching
project's reserved construction cargo. Any future world-haul presentation for them must extend the
canonical typed task binding rather than reconstructing item identity from `resource_id`.
