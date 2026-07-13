# P19 — DF-scale item economy (cat-themed): materials, material-variant crafting, traders

> **Living target spec.** The item/material model, a small recipe subset, visiting traders, and
> basic buy/sell actions exist. Full source/crafting breadth, local physical inventories,
> quality/UI coverage, fishing, and transport remain open. Configurable, consensual inter-village
> resource barter is verified with a 32-open-offer cap and atomic inventory/storage rechecks;
> deeper item-stack/route/relationship trade remains open in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

User (2026-07-10): give the game a DF-like breadth of resources/items, but **cat-themed**
("humanoid cats", "warrior cats") — similar amount. Like DF, **craft an item from multiple
materials** (a wooden mug OR a stone mug). And add **traders that come by and let you trade**.

## Resource / item taxonomy (DF breadth, cat-flavoured)
Three tiers (extends P16's logs/planks/stone/blocks and P12.4b chains):
- **Raw materials**: logs/wood, stone, **ore→metal**, gems (mountain), plant **fibre**,
  herbs/**catnip**, food (prey/**fish**/grain), **hide + bone** (from hunts), clay/sand.
- **Intermediate** (from workshops): planks, stone blocks, **metal bars**, cloth, thread, leather,
  flour. (Wood-cut / stone-prep / smelter / clothier / mill / tannery.)
- **Finished goods** (crafted, cat-themed): **mugs, bowls, furniture** (beds/chairs/tables for the
  cats' dens), **tools** (axe/shovel/pick/fishing-rod), **weapons + armor** (warrior-cat claws/
  blades/mail), **clothing**, **decorations/trinkets**, toys. DF-ish breadth, not overwhelming.

## Material-variant crafting (the "wooden mug OR stone mug")
A recipe is **item-type × material** → a variant: `mug` craftable from wood / stone / metal / bone,
each variant differing in **value, quality, weight, durability** (metal weapon > wood; stone mug >
wood mug in value, etc.). Model: `Item { kind: ItemKind, material: Material, quality }`; a workshop
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
