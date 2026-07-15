# P19 — DF-scale item economy (cat-themed): materials, material-variant crafting, traders

> **Living target spec.** The item/material model, a small recipe subset, visiting traders, and
> basic buy/sell actions exist. Stable item-unit identity, weight, finite durability, work wear,
> material-backed repair, a 20 kg trader load limit, persistence, and Goods-panel condition
> controls are verified. Full source/crafting breadth, local physical inventories, and built
> transport remain open. Finite fresh-Fish habitats and the
> physical shore→store route are verified. Configurable, consensual inter-village
> resource barter is verified with a 32-open-offer cap and atomic inventory/storage rechecks;
> deeper item-stack/route/relationship trade remains open in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

User (2026-07-10): give the game a DF-like breadth of resources/items, but **cat-themed**
("humanoid cats", "warrior cats") — similar amount. Like DF, **craft an item from multiple
materials** (a wooden mug OR a stone mug). And add **traders that come by and let you trade**.

## Canonical production contract

This section is the authority for resource names and production chains. P12 remains authoritative
for manual-to-officer ownership and physical station logistics; P16 remains authoritative for the
founding blueprint. The table describes the target, not the current completion state. Stable
serialized resource and building IDs stay intact while the remaining routes are implemented, and
every workshop remains an open-top, function-readable map station.

| Source or station | Input → output | Research | Labor / automation | Required physical route |
| --- | --- | --- | --- | --- |
| Logging site | tree → Logs | `sawmill` unlocks Gather Logs | Woodcut / Forester; manual while vacant | Reach an observed tree, fell it, and carry Logs to a gather spot or finite pile |
| Wood Cutter | Logs → Planks | available from founding; future studies gate recipes, not placement | Process / Forester | Pile → local input → work → local output → finite pile |
| Sawmill | Logs → Lumber | building `sawmill`; study `carpentry_preparation` | Process / Forester | Same station-local carry/work/delivery contract |
| Quarry | rock → Stone; mountain veins can also yield Ore or Gem | `mountaineering` for mountain outputs | Quarry / Forester | Each resulting material is carried; job completion never credits an invisible byproduct |
| Stone Prep | Stone → Blocks | available from founding; future studies gate recipes, not placement | Process / Forester | Same station-local carry/work/delivery contract |
| Woodworking | Planks + Blocks → wooden Tools; Planks → wood goods | available from founding; future Toolmaking/Carpentry studies gate recipes | Craft / Forester | One selected queue recipe per worker; no parallel hidden craft timer |
| Construction | Lumber or Planks + Blocks → scaffold/building | building-specific study | Build / Leader direction or manual order | Finite stock is carried to the scaffold before on-site progress |
| Farms and forage | plots/sites → Grain, Herbs, Catnip, Fibre | crop/source-specific studies | Farm/Forage / Farmer | Harvest basket → gather spot → finite pile |
| Mill | Grain → Flour; Flour → Food | building `milling`; current combined study `grain_milling_preparation` | Mill / Farmer | Split the current combined cycle into two explicit queue recipes through the physical Mill route |
| Hunt | prey → Food plus Hide/Bone | hunting/source studies | Hunt / Farmer, with the Leader safety floor | Every byproduct returns as physical cargo |
| Clothier | Fibre → Cloth; Cloth → clothing | building `textiles`; Textile Work recipes | Textile / Cloth Leader | Same station-local carry/work/delivery contract |
| Tannery | Hide → Leather; Leather → clothing/light armor | building `textiles`; Leatherworking recipes | Textile / Cloth Leader | Same station-local carry/work/delivery contract |
| Smelter | Ore → Metal bars | building `smelting`; study `metallurgy_preparation` | Metalwork / Captain | Same station-local carry/work/delivery contract |
| Smithy | Metal → metal Tools, Weapons, or Armor | building `smithy`; separate Toolmaking, Weaponsmithing, and Armorsmithing recipes | Metalwork / Captain | One selected queue recipe per worker through local stores |
| Workshop | Supplies → Crafted Supplies | `basic_tools`; study `trade_goods_preparation` | Process / Steward | Preserve the current physical `materials_to_refined` route |
| Variant goods | Planks/Blocks/Cloth/Leather/Metal → material-specific items | matching craft-family recipe | Owning station's labor/officer | Stable finite item units remain local until hauled |

### Stable taxonomy and save compatibility

- **Logs** are raw timber. **Planks** are fine boards for tools, furniture, and early building;
  **Lumber** is structural timber. Construction accepts both and prefers Lumber when available.
- Add **Stone** as a defaulted raw resource when its physical quarry route lands. **Blocks** remain
  dressed stone. Existing `materials` must not be silently reinterpreted as Stone.
- Keep the stable `materials` and `refined` wire/save IDs. Player-facing copy may call them
  **Supplies** and **Crafted Supplies** so their generic bulk Workshop chain is unambiguous.
- Keep the stable `tools`, `weapons`, and `armor` resource fields during migration, but finite item
  instances are the eventual condition/identity authority. New crafting must not create two
  independent inventories for one physical object.
- One assigned cat advances one selected recipe. Woodworking, Smithy, Clothier, Tannery, and every
  future bench must use the same ordered/repeatable/pausable queue contract as the four completed
  physical processors.

## Implemented finite-item condition contract

Each unit has a stable ID, material, kind, quality, value, physical weight, and current/maximum
durability. Relevant work wears finite units; reaching zero leaves a broken unit rather than
deleting it. A signed player repair targets that stable ID and succeeds only at the appropriate
completed, staffed workshop with a living worker and one visible unit of matching material. The
durability research effect scales restored condition. These values and actions survive SQLite
restart, and the Goods panel makes condition and repair visible.

One signed caravan sale may transfer at most 20,000 grams of items. That bounded-load seam and the
finite-item loop are implemented foundations; they do not make the catalog complete. Bone, gem,
clay, broader metal variants, finished functional equipment chains, and physical station-local
logistics for the remaining workshops are still product work.

## Resource / item taxonomy (DF breadth, cat-flavoured)
Three tiers (reconciles P16's founding benches with P12's Sawmill chain):
- **Raw materials**: logs/wood, stone, ore, gems (mountain), plant **fibre**,
  herbs/**catnip**, food (prey/**fish**/grain), **hide + bone** (from hunts), clay/sand.
- **Intermediate** (from workshops): planks, structural lumber, stone blocks, **metal bars**, cloth,
  thread, leather, flour, and the stable bulk Supplies/Crafted Supplies pair.
- **Finished goods** (crafted, cat-themed): **mugs, bowls, furniture** (beds/chairs/tables for the
  cats' dens), **tools** (axe/shovel/pick/fishing-rod), **weapons + armor** (warrior-cat claws/
  blades/mail), **clothing**, **decorations/trinkets**, toys. DF-ish breadth, not overwhelming.

## Material-variant crafting (the "wooden mug OR stone mug")
A recipe is **item-type × material** → a variant: `mug` craftable from wood / stone / metal / bone,
each variant differing in **value, quality, weight, durability** (metal weapon > wood; stone mug >
wood mug in value, etc.). The live model adds a stable unit ID and current/maximum durability to
the kind/material/quality identity; a workshop
recipe = (ItemKind, allowed materials) consuming N of that material. This keeps the item list
compact (kinds × materials) while giving DF-like variety. Value = f(kind, material, quality).

## Traders / caravans
Periodic **visiting traders** (like DF caravans): a trader arrives at the village (walks to the
shrine/market), **stays a while**, and opens a **trade menu** — sell your surplus/crafts for
value/coin, buy goods/materials you lack (esp. things from biomes you can't yet reach). A simple
value/coin economy; relations/price could deepen later. Ties to: the market building, the goods
above, and reaching distant-biome resources (P17) before you have transport (P16 trains/ships).

## Scope / sequencing (big — builds on P12.4b)
This is the large economy layer. Realistically staged:
1. **Item/material model** (cat-sim): `Material`, `ItemKind`, `Item`, value fn; extend Resources
   from a fixed struct toward an item/material store (careful — Resources is used everywhere;
   keep the core survival resources fast, add an item store alongside).
2. **Material-variant recipes + workshops** (P12.4b): each workshop crafts item-kinds from allowed
   materials into the store.
3. **Traders** (server + sim): spawn/visit schedule, trade menu action, value/coin.
4. **Client**: trade UI, item/goods in inspectors + stockpiles + the DF-Steam UI (P18).
Do after the current spatial/feel/biome foundations — this is mid/late-game depth, and it's the
biggest single economy expansion. Keep determinism + survival intact throughout.
