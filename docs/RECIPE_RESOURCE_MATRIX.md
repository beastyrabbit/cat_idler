# Recipe and resource implementation matrix

Last updated: 2026-07-15

This is the evidence boundary between the 500-study research ledger and authoritative gameplay.
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

All 13 operations use pile → carried input → station-local input → work → station-local output →
carried delivery. The two Mill operations are deliberately separate: Flour must be delivered to
storage before it can return as the selected baking input. Rules-v7 SQLite migration replaces the
old `grain_to_flour_and_food` queue entry with both explicit entries while preserving order,
repeat intent, pause, progress, and intentionally empty queues.

## Catalog promises that remain future content

There are exactly **91 generated recipe payload IDs** without a runtime descriptor and **64
generated resource payload IDs** without an authoritative `ResourceKind` source/consumer. They
remain visible as `FUTURE` in the research ledger, cannot spend points, and cannot be selected by
the Leader.

| Generated breadth | Missing authoritative layer |
| --- | --- |
| Gem, clay, and sand families | No finite physical source route exists yet |
| Bone item variants | Bone has a physical hunt source, but no canonical consuming station/recipe |
| Baking, herbalism, medicine, pottery, glasswork, masonry goods, jewelry, furniture, toys, decorations, and other generated family recipes | No P19-selected station recipe descriptor and complete physical route |
| Generic `*_sources` resource IDs | They are catalog registry labels, not stable save/wire resource kinds or physical source entitlements |
| Nonfunctional material-variant goods | Item definitions exist, but their selected recipe, station-local capacity, exact cargo, and delivery loops do not |

This boundary prevents research points from buying no-ops. A future entry moves into the live table
only with a design-backed source/station choice and end-to-end simulation, persistence, protocol,
client, unattended, and signed-player evidence.
