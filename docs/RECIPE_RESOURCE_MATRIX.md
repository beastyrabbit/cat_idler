# Recipe and resource implementation matrix

Last updated: 2026-07-16

This is the evidence boundary between the exact 487-study ("about 500") research ledger and authoritative gameplay.
A recipe is **live** only when it has a physical source, finite input, selected station work,
finite output/delivery, and a catalog entitlement (or an explicit founding baseline). A generated
catalog label alone is not an implementation.

The source contract comes from `GAME_VISION.md`, P12's physical/officer rules, P17's biome source
rules, and P19's canonical production table. Runtime identity comes only from
`cat-sim::station_recipes`; the regression count is pinned in `research_catalog` tests.

## Live source → station → output routes

| Physical source | Selected runtime recipe | Station / work | Delivered output | Entitlement |
| --- | --- | --- | --- | --- |
| observed tree → Logs | `logs_to_planks` | Wood Cutter / Process | Planks | founding baseline; `carpentry_staples` owns later breadth |
| observed tree → Logs | `logs_to_lumber` | Sawmill / Process | Lumber | `carpentry_preparation` |
| quarry rock → Stone | `stone_to_blocks` | Stone Prep / Process | Blocks | founding baseline; `stonecraft_preparation` owns later breadth |
| finite Planks + Blocks | `planks_and_blocks_to_tools` | Woodworking / Craft | exact wooden Tool | founding baseline; `toolmaking_preparation` owns later breadth |
| farm basket → Grain | `grain_to_flour` | Mill / Mill | Flour | `grain_milling_preparation` |
| stored Flour | `flour_to_food` | Mill / Mill | Food | `grain_milling_staples` |
| finite Supplies | `materials_to_refined` | Workshop / Process | Crafted Supplies | `trade_goods_preparation` |
| mountain quarry → Ore | `ore_to_metal` | Smelter / Metalwork | Metal | `metallurgy_preparation` |
| forage → Fibre | `fibre_to_cloth` | Clothier / Textile | Cloth | `textiles` |
| hunt → Hide | `hide_to_leather` | Tannery / Textile | Leather | `textiles` |
| stored Metal | `smithy_weapon` | Smithy / Metalwork | exact metal Weapon | `weaponsmithing` |
| stored Metal | `smithy_tool` | Smithy / Metalwork | exact metal Tool | `toolmaking_staples` |
| stored Metal | `smithy_armor` | Smithy / Metalwork | exact metal Armor | `armorsmithing` |
| hunt → Bone | `bone_tool` | Woodworking / Craft | exact bone Tool | `toolmaking_quality` |
| hunt → Bone | `bone_trinket` | Stone Prep / Process | exact bone Trinket | `hunting_preparation` |
| hunt → Bone | `bone_toy` | Stone Prep / Process | exact bone Toy | `hunting_staples` |
| mountain deposit → Gem | `gem_jewelry` | Workshop / Process | exact quality-2 gem Trinket | `trade_goods_staples` |
| wetland/badlands deposit → Clay | `clay_mug` | Stone Prep / Process | exact clay Mug | `stonecraft_staples` |
| wetland/badlands deposit → Clay | `clay_bowl` | Stone Prep / Process | exact clay Bowl | `stonecraft_quality` |
| wetland/badlands deposit → Clay | `clay_brick` | Stone Prep / Process | exact clay Brick | `stonecraft_specialty` |
| beach/desert deposit → Sand | `sand_glass_mug` | Workshop / Process | exact glassy sand Mug | `trade_goods_quality` |
| beach/desert deposit → Sand | `sand_glass_bowl` | Workshop / Process | exact glassy sand Bowl | `trade_goods_specialty` |
| beach/desert deposit → Sand | `sand_glass_trinket` | Workshop / Process | exact quality-2 glassy sand Trinket | `trade_goods_masterwork` |

All 23 operations use pile → carried input → station-local input → work → station-local output →
carried delivery. The ten variant routes create a stable finite item identity in local output and
move that same identity, material, quality, maximum/current durability, and condition to storage;
there is no shadow scalar good. The two Mill operations are deliberately separate: Flour must be
delivered to storage before it can return as the selected baking input. Rules-v7 SQLite migration
replaces the old `grain_to_flour_and_food` queue entry with both explicit entries while preserving
order, repeat intent, pause, progress, and intentionally empty queues.

## Catalog promises that remain future content

There are exactly **81 generated recipe payload IDs** without a runtime descriptor and **64
generated resource payload IDs** without an authoritative `ResourceKind` source/consumer. They
remain visible as `FUTURE` in the research ledger, cannot spend points, and cannot be selected by
the Leader.

| Generated breadth | Missing authoritative layer |
| --- | --- |
| Remaining gem, clay, sand, and bone families | The selected starter variants above are live; the other generated combinations have no selected runtime descriptor |
| Baking beyond the live Mill baseline, herbalism, medicine, furniture, clothing, and other generated family recipes | No P19-selected station recipe descriptor and complete physical route |
| Generic `*_sources` resource IDs | They are catalog registry labels, not stable save/wire resource kinds or physical source entitlements |
| Remaining material/category combinations | Item definitions alone do not provide a selected recipe, station-local input, exact output identity, or delivery loop |

This boundary prevents research points from buying no-ops. A future entry moves into the live table
only with a design-backed source/station choice and end-to-end simulation, persistence, protocol,
client, unattended, and signed-player evidence.
