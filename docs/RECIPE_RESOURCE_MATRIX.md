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
| farm basket → Grain | `fine_grain_flour`, `stoneground_flour`, `masterwork_flour` | Mill / Mill | Flour | Grain Milling quality/specialty/masterwork |
| stored Flour | `bake_flatbread`, `bake_loaf`, `bake_biscuits`, `bake_festival_cake`, `bake_masterwork_pastry` | Mill / Mill | Food | Baking preparation/staples/quality/specialty/masterwork |
| farm basket → Herbs | `herbal_poultice`, `herbal_tonic`, `herbal_salve`, `herbal_remedy`, `herbal_masterwork_remedy` | Workshop / Process | Medicine | Herbalism preparation/staples/quality/specialty/masterwork |
| stored Food | `dry_food`, `smoke_food`, `pickle_food`, `preserve_rations`, `preserve_masterwork_feast` | Mill / Mill | Preserves | Food Preservation preparation/staples/quality/specialty/masterwork |
| Grain/Catnip/Herbs | `brew_grain_small`, `brew_catnip_ale`, `brew_herbal_tonic`, `brew_spiced_ale`, `brew_masterwork` | Mill / Mill | Brew | Brewing preparation/staples/quality/specialty/masterwork |
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
| hunt → Bone | `hunting_quality`, `hunting_specialty`, `hunting_masterwork` | Woodworking / Craft | exact bone Weapon, Armor, and quality-2 Tool | matching Hunting study |
| forage → Fibre | five `foraging_*` recipe IDs | Clothier / Textile | exact fibre Bowl, Clothing, Tool, Toy, and Furniture | matching Foraging study |
| stored Planks | five `waterworks_*` recipe IDs | Woodworking / Craft | exact wooden Bowl, Mug, Toy, Furniture, and quality-2 Bowl | matching Waterworks study |
| hunt → Hide | five `animal_husbandry_*` recipe IDs | Tannery / Textile | exact leather Clothing, Toy, Armor, Tool, and Furniture | matching Animal Husbandry study |
| stored Supplies | five `field_craft_*` recipe IDs | Workshop / Process | exact wooden Tool, Clothing, Armor, Furniture, and Toy field wares | matching Field Craft study |
| stored Cloth | five `expedition_supplies_*` recipe IDs | Workshop / Process | exact quality-2/3 fibre Bowl, Clothing, Tool, Armor, and Furniture travel wares | matching Expedition Supplies study |

All 74 operations use pile → carried input → station-local input → work → station-local output →
carried delivery. The 38 exact-item routes create a stable finite item identity in local output and
move that same identity, material, quality, maximum/current durability, and condition to storage;
there is no shadow scalar good. The two Mill operations are deliberately separate: Flour must be
delivered to storage before it can return as the selected baking input. Rules-v7 SQLite migration
replaces the old `grain_to_flour_and_food` queue entry with both explicit entries while preserving
order, repeat intent, pause, progress, and intentionally empty queues. The 28 new frontier routes
use the same exact identity and conservation contract. Their present consumers are ordinary
equipment where an item kind is functional and finite trade otherwise; Field Craft and Expedition
Supplies do not secretly accelerate farms or scouts, Waterworks does not mint Water, and the
one-step Tannery routes deliberately tan finite Hide into the listed finished leather identity.

## Generated resource-stage consumers

The twenty resource nodes in the six frontier families are runtime effects, not new shadow goods.
For each family, `sources` reduces the finite input consumed by its selected kit recipes by 10%,
`preservation` raises the exact output identities' maximum durability by 25%, and `bulk` shortens
only that family's staffed physical cycles by 10%. `hunting_reserves` adds finite Food/Hide/Bone
headroom; `foraging_reserves` adds finite Fibre/Herbs/Catnip headroom. Full and compact hauling
capacity resolution share those exact additions.

## Catalog promises that remain future content

There are exactly **30 generated recipe payload IDs** without a runtime descriptor and **27
generated resource payload IDs** without an authoritative source/consumer. They
remain visible as `FUTURE` in the research ledger, cannot spend points, and cannot be selected by
the Leader.

| Generated breadth | Missing authoritative layer |
| --- | --- |
| Remaining gem, clay, sand, and bone families | The selected starter variants above are live; the other generated combinations have no selected runtime descriptor |
| Furniture and remaining clothing, weapon/armor, decorative, and material-family variants | No P19-selected station recipe descriptor and complete physical route |
| Remaining generic `*_sources` resource IDs | They are catalog registry labels without an exact physical source or consumer |
| Remaining material/category combinations | Item definitions alone do not provide a selected recipe, station-local input, exact output identity, or delivery loop |

This boundary prevents research points from buying no-ops. A future entry moves into the live table
only with a design-backed source/station choice and end-to-end simulation, persistence, protocol,
client, unattended, and signed-player evidence.
