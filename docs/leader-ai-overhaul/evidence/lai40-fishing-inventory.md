# LAI.40 Fishing Hut, Rods, Shoreline Work, And Finite Ecology Inventory

This is read-only evidence for LAI.40. It inventories current `feature-new-leader-ai`, the protected `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade` source worktree, and the evolving LAI.36-LAI.39 contracts; it does not implement behavior, edit production code, edit plans/boards, or run tests.

## Required Contract

| Requirement | Origin |
|---|---|
| LAI.40 card | `docs/leader-ai-overhaul/BOARD.md:1181`: founding hand-fishing is slow/unreliable; exact Rod-only, Hut-only, and combined improvements; Rod identity/wear; 3x3 Hut plus oriented dock/water attachment; real shoreline task/route/cargo; finite persisted report-limited habitat; reject nonshore placement. |
| P1 fishing geometry | `docs/leader-ai-overhaul/BOARD.md:1211`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:120-125`: full 3x3 land footprint, dock-facing land cell, reserved oriented water attachment; construction visualizes footprint+dock; operation visualizes real shoreline task, assigned fisher, rod, route, cargo. |
| P1 quality and lots | `docs/leader-ai-overhaul/BOARD.md:1215-1219`: stable IDs and `BulkLotKey = content_id + quality`; Fish and tools carry quality; exact quality formula is LAI.37-owned. |
| Founding guarantee | `docs/leader-ai-overhaul/BOARD.md:1224`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:351-357`: every new colony must reveal reachable Water+bank, Apple tree, and reachable fish habitat+shoreline; no starter reserve substitute. |
| Rod/Hut modifiers | `docs/leader-ai-overhaul/BOARD.md:1226`; `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:370-376`: founding hand-fishing slow/unreliable; Rod and staffed Hut independently improve catch/cycle; Hut+Rod gives full combined improvement; neither fabricates fish or replaces finite ecology; nonshore Hut placement rejected. |
| Report secrecy | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:48-60`; `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md:56-58`: God gets the same report projection as leadership; exact fish replenishment stays server-only and must not cross protocol for client-side hiding. |
| UI and assets | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:587-610`, `:615-641`; `docs/leader-ai-overhaul/BOARD.md:1238-1239`: Food/Cookhouse shows hand difficulty, Hut bonus, Rod quality/wear; item detail shows Rod material/quality/durability/augmentation/effect/provenance/reservation; assets include four Hut orientations, docks, boat, activity states, Rod icon, quality badges. |

## Accepted Current/Source Baseline

Current and protected source agree on the pre-LAI.40 fishing baseline:

| Definition | Current origin | Protected source origin | Classification | LAI.40 disposition |
|---|---|---|---|---|
| Work duration baseline | `crates/cat-sim/src/idle_engine.rs:20-22`, `:228-233` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/idle_engine.rs:22`, `:232` | Handler/compat baseline | One Fish job has a 45-game-minute productive work timer before travel/trips/tool productivity. LAI.40 can use this as the accepted deterministic hand-fishing cycle baseline unless the coordinator supersedes it. |
| Catch baseline | `crates/cat-sim/src/world_tick.rs:2442-2445` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:2531-2533` | Handler/compat baseline | One shoreline shift yields 12 scalar fish before skill/haul upgrades and is split over three physical trips. LAI.40 must convert output to LAI.37/38 `food_raw_fish` quality lots, not keep scalar `Fish`. |
| Current tool duration boost | `crates/cat-sim/src/actions.rs:5688-5733`, `crates/cat-sim/src/world_tick.rs:24372-24431`, `crates/cat-sim/src/productivity.rs:6-20`, `:30-38` | Source has the same generic tool path in runtime; P16 spec says tools boost rod->fishing at `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/docs/migration/specs/p16-village-blueprint.md:65-75` | Legacy handler | Any credited unbroken equipped generic `Tool` can shorten Fish by generic 1.20 capped productivity. This is not exact Rod identity/effect and must be cut over. |
| Skill yield boost | `crates/cat-sim/src/world_tick.rs:29126-29135`, `:29175-29184` | Source runtime equivalent around `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:24738` | Handler | Fishing skill scales yield via `trade_yield_multiplier`, floors result. LAI.40 must decide whether Rod/Hut modifiers compose before or after LAI.37 gathering quality/skill formula. |
| Habitat stock/capacity | `crates/cat-sim/src/stockpiles.rs:79-92` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/stockpiles.rs:79-92` | Manifest-adjacent data/handler constant | Persisted scalar `FishPopulation { stock, capacity, last_replenished_at_ms }`, capacity 24, replenish 0.5/game-hour. Keep as ecological source state, but expose through report projection and debit quality lots. |
| Replenishment | `crates/cat-sim/src/world_tick.rs:10349-10380` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:5538-5568` | Handler | Adds elapsed game hours * 0.5 and caps at habitat capacity; removing/repainting a spot cannot refill. Must remain persisted and report-limited. |
| Shore validation | `crates/cat-sim/src/world_tick.rs:26449-26529` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:21976-22032` | Handler | Valid work tile is revealed dry land orthogonally adjacent to revealed water and route-reachable. Use this for shoreline task and Hut dock validation, without making water walkable. |
| Habitat key | `crates/cat-sim/src/world_tick.rs:26479-26500` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:21997-22018` | Handler | Canonical habitat is first revealed adjacent water in N/E/S/W order, but established habitat keys win. Preserve this or replace once, with a migration/cutover receipt. |
| Catch debit | `crates/cat-sim/src/world_tick.rs:26502-26512`, `:28587-28618`, `:17733-17743` | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:22020-22029`, `:24096-24120` | Handler | Fish is debited only on successful on-site catch/trip. Preserve no debit during queue/travel/death/cancel/route failure. |

The inventory found no source-owned Rod/Hut profile, so the coordinator closed it explicitly in
BOARD P1-C02. The authoritative deterministic profiles are hand `12 / 45m / 75%`, Common Rod-only
`15 / 36m / 90%`, staffed Hut-only `18 / 30m / 95%`, and Common Rod+Hut
`24 / 24m / 100%`, capped by actual habitat stock. A keyed failure debits no fish; an accepted Rod
attempt wears that exact instance once whether it catches or not. Rod quality scales only the Rod
reliability contribution with LAI.37's item-effect percentages, so the item detail can name the
actual quality effect without duplicating food-lot quality. Neither Rod nor Hut changes the
source-proven capacity 24 or absolute 0.5-unit/game-hour replenishment.

## Manifest And Stable IDs

| Definition | Origin | Classification | Notes |
|---|---|---|---|
| `content_manifest.rs` candidate | `crates/cat-sim/src/content_manifest.rs:1`, `:364-384`, `:386-419`, `:459-468` | Current-tree unexported LAI.36 candidate; closed behavior enums | The stopped duplicate worker left this untracked file. It contains data-only/closed-enum scaffolding: `ItemFunction::FishingBonus`, `TaskCategory::Fishing`, `StationBehavior::FishingHut`, `FixtureSlot::FishingHut`; do not edit it. It satisfies ID-class classification but not LAI.40 throughput/geometry constants. |
| Fishing Rod item | `crates/cat-sim/src/content_manifest.json:1107-1125` | Manifest data | `id=fishing_rod`, `content_id=item_fishing_rod`, art `art_item_fishing_rod`, class `tool`, equipment slot `tool`, augmentation slot `tool`, function `fishing_bonus`, required capability `fishing_rods`, handler `craft_item`. No material, durability, wear rate, or effect multiplier is encoded. |
| Fishing Hut station | `crates/cat-sim/src/content_manifest.json:2975-2997` | Manifest data | `content_id=station_fishing_hut`, behavior `fishing_hut`, 3x3 geometry, task category `fishing`, fixture slot `fishing_hut`, required capability `fishing_hut`, handler `station_work`. Missing dock-facing land cell and oriented water attachment. |
| Fishing Hut fixture | `crates/cat-sim/src/content_manifest.json:7176-7193` | Manifest data | `fixture_fishing_hut`, art `art_fixture_fishing_hut`, slot `fishing_hut`, consumes `serpent_scale`, compatible with `fishing_hut`, handler `install_fixture`. Eligibility must reuse LAI.37 fixture instances. |
| Capabilities | `crates/cat-sim/src/content_manifest.json:7315-7323`, `:7672-7706` | Manifest data | `hand_fishing` owns `resource_fish_habitat` and `food_raw_fish`; `fishing_hut` requires `hand_fishing`; `fishing_rods` requires `hand_fishing` + `plank_processing`. |
| Food and recipes | `docs/leader-ai-overhaul/evidence/lai38-food-ecology-inventory.md:175-186`; `docs/leader-ai-overhaul/evidence/lai39-cookhouse-inventory.md:70-86`, `:91` | Upstream LAI.38/39 contract | Raw fish is `food_raw_fish` lots, not generic scalar `Fish`; Cookhouse recipes consume `food_raw_fish`. |
| Generic scalar Fish alias | `crates/cat-sim/src/stockpiles.rs:127-130`, `crates/cat-protocol/src/lib.rs:619-625` plus LAI.38 receipt `docs/leader-ai-overhaul/evidence/lai38-food-ecology-inventory.md:109` | Legacy/obsolete compatibility alias | Keep only until LAI.38/40/47/48 cutover. Delete alias after typed food lots, report projection, persistence, and UI consumers stop relying on it. |

## Current Runtime Inventory

| Path | Origin | Classification | LAI.40 notes |
|---|---|---|---|
| Manual action validation | `crates/cat-sim/src/actions.rs:490-535` | Handler | `RequestJob(Fish)` requires a designated fishing site and fishable stock; depleted habitat returns report-unsafe exact message today. |
| Designate fishing spot | `crates/cat-sim/src/actions.rs:2100-2171` | Handler | Water clicks resolve to adjacent bank candidates, then require `is_reachable_fishing_shore` and stockpile placement validity; creates one 1x1 fish stockpile/gather spot with `expires_at_ms=i64::MAX` and initializes habitat. Not a Hut placement. |
| Snapshot fish population | `crates/cat-sim/src/actions.rs:4968-4995` | Wire projection handler | Embeds exact `stock/capacity/last_replenished_at_ms` in `GatherSpotSnapshot`, conflicting with report secrecy. |
| Fishing job target | `crates/cat-sim/src/world_tick.rs:11483-11589`, `:11610-11621` | Handler | Queued Fish jobs pick first fishable site and become `JobMetadata::Hauling` with site, trips, accepted flag. |
| Farmer emergency fishing | `crates/cat-sim/src/world_tick.rs:12574-12618` | Handler | Farmer picks Fish instead of Hunt when there is fishable stock; uses `Labor::Fishing` selection. LAI.40 should preserve Farmer ownership but route through typed reports/beliefs. |
| Productive timer suspension | `crates/cat-sim/src/world_tick.rs:17317-17392`, `:25600-25627` | Handler | Fish timers count only accepted, living, on-site worker time; non-current fishing site cancels. Accept resets fish start/end to actual on-site work window. |
| Cargo trips | `crates/cat-sim/src/world_tick.rs:17690-17788` | Handler | On-site due trips harvest fish, update trips, then carry `CarryingKind::Fish` to destination. No quality lot identity exists. |
| Deposit routing and conservation | `crates/cat-sim/src/world_tick.rs:29050-29070`; `crates/cat-sim/src/stockpiles.rs:53-77`, `:611-680` | Handler | Carried fish deposits to nearest accepting pile/gather spot/storehouse fallback. Current `source_gather_spot=None` for fishing expedition cargo, so overflow can be abandoned once scalar capacity is full. |
| Focused fish tests | `crates/cat-sim/src/world_tick.rs:70693-71545` | Existing red/green behavior evidence | Tests cover fixture shore, finite persisted habitat, no repaint refill, stable adjacent-water habitat key, no generic Food conversion, deterministic replenish/cap, on-site timer, physical cargo observation, seed determinism. |
| Cancel/death/overflow tests | `crates/cat-sim/src/world_tick.rs:71084-71238`, `:71545-71655`; `crates/cat-sim/src/actions.rs:13498-13625` | Existing conservation tests | Retargets full piles without early credit, cancels cargo to storage not origin, death salvage preserves fish up to headroom, removing a fishing spot cancels jobs and retargets earned cargo. Extend to quality lot identities and Hut/Rod reservations. |

## Protocol, Persistence, UI, And Reporting Consumers

| Consumer | Origin | Classification | Cutover need |
|---|---|---|---|
| Protocol fish population | `crates/cat-protocol/src/lib.rs:587-617`, `:4169-4215` | Wire | Exact fish ecology is public today via `GatherSpotSnapshot.fish_population`; LAI.40/47 must replace with report-safe level projection. |
| Protocol Fish job/action | `crates/cat-protocol/src/lib.rs:1128-1140`, `:2078-2090` | Wire | Existing `JobKind::Fish` and `DesignateFishingSpot` are shoreline-designation controls; no Fishing Hut placement/action or Rod assignment DTO. |
| Transport dock DTO | `crates/cat-protocol/src/lib.rs:1688-1707`, `:1944-1951` | Downstream/adjacent | Transport docks have land/water tiles but are not Fishing Hut docks. Do not reuse as LAI.40 authority without a typed Hut attachment discriminator. |
| Persistence schema | `crates/cat-server/src/persistence.rs:115-120`, `:185-199`, `:387-402`, `:733-748`, `:830-857`, `:945-963`, `:1019-1020`, `:1125-1140` | Persistence | Fish habitats are saved both as shared world JSON and colony JSON. This is the highest competing-authority risk; LAI.40/48 needs one persisted ecology authority plus deterministic sync/receipt or delete one lane. |
| Server routing/auth | `crates/cat-server/src/main.rs:2988-3005`, `:3417-3422`, `:5660-5753` | Server action consumer | Placement envelope can produce `DesignateFishingSpot`; auth test covers shore+Fish job. No Hut build/placement path. |
| Client tools/orders | `crates/cat-client/src/lib.rs:1051-1064`, `:3864-3887`, `:11237-11250` | UI consumer | Client has `ToolMode::FishingSpot`, "Fish shore" order, and paint sends `DesignateFishingSpot`. It lacks Hut placement UI and Rod/Hut state display. |
| Client resource UI | `crates/cat-client/src/lib.rs:4388-4508` | UI/legacy alias | HUD maps generic `ResourceKind::Fish` to icon/label/capacity. Cut after typed food UI consumes LAI.38 lots/report projection. |
| Client tooltip | `crates/cat-client/src/lib.rs:18454-18476` | UI test/consumer | Tooltip asserts exact "habitat 7.5 / 24"; this violates report ladder and must be replaced by bounded report text. |
| Transport dock rendering | `crates/cat-client/src/lib.rs:1458-1492`, `:7935-7970`, `:8027-8029` | Adjacent rendering | Existing boat/dock assets/rendering are transport-only. Fishing Hut dock art may reuse/adapt asset files, but logic cannot become a transport dock authority. |
| Server redaction | `crates/cat-server/src/leader_ai_action_routing.rs:821-880`; `crates/cat-server/src/leader_ai_snapshot_projection.rs:243-270` | Report/redaction handler | LAI.24 redaction has a placeholder for stock and hides regeneration below level 4 in Leader AI reports, but the main game snapshot still exposes fish exact values. LAI.40 must route fish ecology through the report ladder. |

## Source Worktree And Asset Receipts

| Source artifact | Origin | Classification | Receipt/risk |
|---|---|---|---|
| Source transfer manifest | `docs/branch-plan-merge/source-transfer-manifest.md:54`, `:73`, `:130-138`, `:217` | Source-transfer receipt | Protected source head/hash and `public/images` digest recorded; transport assets are listed for asset-owner copying only. |
| Protected fish runtime | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/world_tick.rs:2531-2533`, `:5538-5568`, `:21976-22032`, `:24096-24120`; `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/crates/cat-sim/src/stockpiles.rs:79-92` | Source behavior | Confirms current runtime baseline; no extra Hut/Rod modifier implementation found. |
| Protected P16/P19 specs | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/docs/migration/specs/p16-village-blueprint.md:65-75`, `:94-105`; `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/docs/migration/specs/p19-items-materials-trade.md:100-108` | Source design | Says rod boosts fishing and gather spots split work/hauling; identifies fishing rod as a finished tool kind. It does not define exact Rod/Hut constants. |
| Protected assets present | `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/icons/fish.png`; `.../icons/water.png`; `.../terrain/water.png`; `.../terrain/water_edge.png`; `.../transport/{boat,dock_land,dock_water,rail_cart}.png` from inventory command and manifest refs `docs/branch-plan-merge/source-transfer-manifest.md:130-138` | Source assets | Fish icon and generic water/transport dock/boat assets exist. They are receipts for reuse/adaptation, not final Fishing Hut sprites. |
| Missing source/current assets | Search of `public/images/game` in current and protected source found no dedicated `fishing_hut`, `fishing_rod`, four Hut orientations, Hut idle/working states, Hut water-attachment variants, quality badges, or raw fish food icon files. Manifest planned art keys exist at `crates/cat-sim/src/content_manifest.json:9197-9203`, `:9332-9338`, `:9845-9851`, `:10043-10049`, `:10223-10229`. | Asset gap | LAI.49 must create/copy/adapt and validate these; LAI.40 should only name required art keys/states and avoid generating art. |

## Duplicate, Dangling, And Deletion Risks

1. Exact fish ecology leaks through protocol today (`crates/cat-protocol/src/lib.rs:602-609`; `crates/cat-sim/src/actions.rs:4979-4993`) despite Plan 1 report secrecy (`docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:48-60`).
2. Fish habitat persistence has two lanes, `world.sharedFishHabitats` and `colonies.fishHabitats` (`crates/cat-server/src/persistence.rs:115-120`, `:185-199`, `:733-748`, `:945-963`), so LAI.40/48 must declare one persisted authority and cut the duplicate with a receipt.
3. Current Rod support is only generic `ItemKind::Tool`; `fishing_rod` manifest data is not connected to exact tool identity/effects, durability wear, equipment slot reservation, or active job link (`crates/cat-sim/src/items.rs:108-129`, `:440-467`, `:851-859`, `:964-975`).
4. Current Fishing Hut exists only in LAI.36 manifest candidate/data, not in `BuildingType`, protocol `BuildingType`, spatial `footprint_for`, placement, runtime, persistence, or renderer (`crates/cat-sim/src/types.rs:110-139`; `crates/cat-protocol/src/lib.rs:1628-1645`; `crates/cat-sim/src/spatial_tasks.rs:686-718`).
5. Generic scalar `ResourceKind::Fish` remains in stockpiles, HUD, protocol, ledger, storage, and capacity paths; LAI.38 already marks it as a compatibility alias for later deletion (`docs/leader-ai-overhaul/evidence/lai38-food-ecology-inventory.md:109`, `:248-253`).
6. Transport dock code/assets may look reusable but model a different infrastructure type (`crates/cat-sim/src/actions.rs:3048-3078`; `crates/cat-protocol/src/lib.rs:1944-1951`); Fishing Hut needs its own oriented land/water attachment, not a hidden transport route dependency.
7. The current `content_manifest.rs` candidate is untracked and data-only; it should be inventoried by LAI.36 but not treated as an exported runtime API until validated/owned.

## Smallest LAI.40 Authority

Create one `cat-sim` fishing authority that consumes upstream contracts and owns only fishing-specific policy:

- Input IDs: `resource_fish_habitat`, `food_raw_fish`, `item_fishing_rod`, `station_fishing_hut`, `fixture_fishing_hut`, `hand_fishing`, `fishing_rods`, and `fishing_hut` from LAI.36 manifest.
- Quality/lots: use LAI.37 `QualityBand`, `BulkLotKey`, `ItemInstance`, `FixtureInstance`, `LotLocation`, and reservations; do not define another `QualityBand` or item ledger.
- Ecology: use LAI.38 fish source/report projection for habitat stock/capacity/replenishment, then add LAI.40 catch/cycle policies; do not expose exact stock to protocol/God UI.
- Spatial: consume existing `TilePos`, `is_valid_fishing_shore`, pathing/occupancy helpers, and `footprint_for` concepts; add Hut-specific 3x3 plus `dock_land_cell` and `water_attachment` in one place.
- Runtime: one pure resolver should return accepted shore, habitat ID, Hut station bonus, Rod instance/effect/wear, cycle duration, catch amount, cargo lots, and report-safe observations.

Suggested module/test boundary: `crates/cat-sim/src/fishing.rs` with focused tests under the same crate. Runtime integration should call it from action validation, building placement, job queue/accept/complete, cargo recovery, persistence conversion, and snapshot projection; those consumers should not duplicate constants or geometry.

## Red Cases For LAI.40

1. Founding guarantee: deterministic seed matrix has reachable water bank, fish habitat, and shoreline; deleting starter `resources.food/fish` does not break survival if the physical sources remain; adding starter fish reserve alone fails the guarantee.
2. Baseline hand-fishing: no Rod/Hut produces exactly the accepted baseline cycle/catch from the LAI.40 authority, mints `food_raw_fish` quality lots, debits real habitat, and never credits generic scalar `Fish`.
3. Closed profiles locked: the four exact P1-C02 catch/cycle/reliability profiles are explicit named constants/data in the LAI.40 authority; keyed success, Rod-quality reliability scaling, actual-stock caps, or accidental generic-tool fallback fail compilation/tests.
4. Rod identity: only an equipped unbroken `item_fishing_rod` instance applies the Rod modifier; wrong generic Tool, broken Rod, reserved Rod, carried Rod, or unequipped Rod gives no Rod bonus and wears no Rod.
5. Rod wear/restart: accepting work pins the Rod instance/job; completion/cancel/death/restart wears or releases the same instance exactly once per defined use boundary.
6. Hut placement: nonshore Hut rejected atomically; valid Hut reserves/owns all 9 land cells plus dock-facing land cell and oriented water attachment; roads, buildings, stockpiles, Apple footprints, and transport docks cannot overlap reserved cells.
7. Hut operation: staffed Hut improves coordination/storage/cycle but work still occurs at valid shoreline/habitat; no Hut creates fish, catches from depleted habitat, or changes habitat key.
8. Combined modifier: Rod-only, Hut-only, and Hut+Rod outcomes are independently asserted and deterministic; combined is not accidentally double-applied or reduced to max(single).
9. Conservation: cancel before pickup debits nothing; cancel after pickup/death/route loss/restart preserves `lot_id + content_id + quality + quantity + Rod reservation + Hut reservation`; no overflow disappears silently.
10. Report secrecy: protocol/God/tooltip never shows exact habitat stock/capacity/cursor below authorized level; client hiding is not sufficient.
11. UI and assets: Food/Cookhouse panel shows report-safe hand difficulty/Hut/Rod wear; item detail shows Rod quality/durability/augmentation/effect/provenance/reservation; missing sprites fail asset inventory, not sim logic.

## Staged Consumer Order

1. Land LAI.36 manifest export/validation and freeze fishing IDs/art keys.
2. Land LAI.37 quality lots and item/fixture instances, including Rod/fixture eligibility and reservation invariants.
3. Land LAI.38 fish source/report projection and typed `food_raw_fish` lot production boundary.
4. Add LAI.40 pure fishing authority with constants, resolver, Hut geometry, Rod effect/wear, and red tests.
5. Cut action validation and job queue/accept/complete to the authority; retain compatibility aliases only at boundary.
6. Cut persistence to one fish ecology/lot authority and write receipts for deleting duplicate `fishHabitats`/`sharedFishHabitats` lanes or their sync source.
7. Cut protocol/server/client to report-safe Hut/Rod/fishing projections and remove exact `FishPopulationSnapshot` truth.
8. Update UI/asset consumers, then delete generic `Fish` alias paths with LAI.47/48/52 receipts.

## What LAI.40 Must Not Own

- No duplicate content catalog; consume LAI.36 manifest.
- No duplicate `QualityBand`, lot ledger, item instance, fixture instance, or augmentation rules; consume LAI.37.
- No duplicate food manifest, nutrition/spoilage/value, or Apple/source ecology; consume LAI.38.
- No Cookhouse recipe authority; consume LAI.39 food outputs/recipe requirements.
- No transport dock authority; Hut dock attachment is spatial geometry for fishing only.
- No client-only redaction; secrecy must be server/protocol/report enforced.
