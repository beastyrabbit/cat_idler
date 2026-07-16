# P19 — DF-scale item economy (cat-themed): materials, material-variant crafting, traders

> **Living target spec.** The item/material model, all 108 physical recipes, and finite
> shrine-visiting traders with buy/sell actions are implemented. Stable item-unit identity, exact functional
> equipment location/loadout, weight, finite durability, work/combat wear, material-backed repair,
> exact caravan cargo transfer, finite trader
> stock/purse/capacity, persistence, and Goods/trade-panel truth are implemented. Constructed Rail and
> Shipping routes plus physical exact-equipment village caravans are live. Finite fresh-Fish habitats and the
> physical shore→store route are verified. Configurable, consensual inter-village
> resource barter is verified with a 32-open-offer cap, exact escrow, visible obstacle-aware
> gate-to-gate travel, and cancellation/restart conservation. Focused implementation gates pass;
> the integrated generalized passive/player-guided gate passes. Evidence lives in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

User (2026-07-10): give the game a DF-like breadth of resources/items, but **cat-themed**
("humanoid cats", "warrior cats") — similar amount. Like DF, **craft an item from multiple
materials** (a wooden mug OR a stone mug). And add **traders that come by and let you trade**.

## Canonical production contract

This section is the authority for resource names and production chains. P12 remains authoritative
for manual-to-officer ownership and physical station logistics; P16 remains authoritative for the
founding blueprint. The table describes the implemented maintained target. Stable
serialized resource and building IDs stay intact across every route, and
every workshop remains an open-top, function-readable map station.

| Source or station | Input → output | Research | Labor / automation | Required physical route |
| --- | --- | --- | --- | --- |
| Logging site | tree → Logs | `sawmill` unlocks Gather Logs | Woodcut / Forester; manual while vacant | Reach an observed tree, fell it, and carry Logs to a gather spot or finite pile |
| Wood Cutter | Logs → Planks | available from founding; later studies gate recipes, not placement | Process / Forester | Pile → local input → work → local output → finite pile |
| Sawmill | Logs → Lumber | building `sawmill`; study `carpentry_preparation` | Process / Forester | Same station-local carry/work/delivery contract |
| Quarry | rock → Stone; mountain veins can also yield Ore or Gem | `mountaineering` for mountain outputs | Quarry / Forester | Each resulting material is carried; job completion never credits an invisible byproduct |
| Stone Prep | Stone → Blocks | available from founding; later studies gate recipes, not placement | Process / Forester | Same station-local carry/work/delivery contract |
| Woodworking | Planks + Blocks → wooden Tools; Planks → wood goods | available from founding; later Toolmaking/Carpentry studies gate recipes | Craft / Forester | One selected queue recipe per worker; no parallel hidden craft timer |
| Construction | Lumber or Planks + Blocks → scaffold/building | building-specific study | Build / Leader direction or manual order | Finite stock is carried to the scaffold before on-site progress |
| Farms and forage | plots/sites → Grain, Herbs, Catnip, Fibre | crop/source-specific studies | Farm/Forage / Farmer | Harvest basket → gather spot → finite pile |
| Mill | Grain → Flour; Flour → Food | building `milling`; `grain_milling_preparation` and `grain_milling_staples` | Mill / Farmer | Two explicit selected queue recipes through the physical Mill route |
| Hunt | prey → Food plus Hide/Bone | hunting/source studies | Hunt / Farmer, with the Leader safety floor | Every byproduct returns as physical cargo |
| Clothier | Fibre → Thread → Cloth; Cloth → clothing | building `textiles`; Textile Work recipes | Textile / Cloth Leader | Two selected station-local carry/work/delivery cycles; Thread is credited and physically returned before weaving |
| Tannery | Hide → Leather; Leather → clothing/light armor | building `textiles`; Leatherworking recipes | Textile / Cloth Leader | Same station-local carry/work/delivery contract |
| Smelter | Ore → Metal bars | building `smelting`; study `metallurgy_preparation` | Metalwork / Captain | Same station-local carry/work/delivery contract |
| Smithy | Metal → metal Tools, Weapons, or Armor | building `smithy`; separate Toolmaking, Weaponsmithing, and Armorsmithing recipes | Metalwork / Captain | One selected queue recipe per worker through local stores |
| Workshop | Supplies → Crafted Supplies | `basic_tools`; study `trade_goods_preparation` | Process / Steward | Preserve the current physical `materials_to_refined` route |
| Variant goods | Bone → Tools/Trinkets/Toys; Gem → jewelry; Clay → pottery/Bricks; Sand → glassy goods | matching craft-family recipe | Owning station's labor/officer | Stable finite item units remain local until hauled |

### Stable taxonomy and save compatibility

- **Logs** are raw timber. **Planks** are fine boards for tools, furniture, and early building;
  **Lumber** is structural timber. Construction accepts both and prefers Lumber when available.
- **Stone** is a defaulted raw resource with a physical quarry route. **Blocks** remain dressed
  stone. Existing `materials` are not reinterpreted as Stone. **Bone** is likewise a distinct
  defaulted hunt byproduct carried after Hide; selected Tool, Trinket, and Toy recipes are live.
- Keep the stable `materials` and `refined` wire/save IDs. Player-facing copy may call them
  **Supplies** and **Crafted Supplies** so their generic bulk Workshop chain is unambiguous.
- Keep the stable `tools`, `weapons`, and `armor` resource fields for old-save and wire
  compatibility. Finite item instances are now the condition/identity/location authority and the
  scalar fields are derived credited projections, not independent inventories.
- One assigned cat advances one selected recipe. All ten maintained processors, including
  Woodworking, Smithy, Clothier, and Tannery, use the same ordered/repeatable/pausable queue
  contract; any new benches must preserve it.

The completed scaffold and sourced-breadth work gives all 108 maintained station recipes stable data-owned descriptors,
canonical input/output resource sets, deterministic default queues, and exact catalog-derived
availability. C2.1 completes Wood Cutter's five-Logs-to-one-Plank physical route. C2.2 completes
Stone Prep's five-Stone-to-one-Block physical route. C2.3 completes Woodworking's sequential
two-Planks plus two-Blocks route into one exact finite Tool after one 600-game-second Craft batch.
P19.C2.4 completes Tannery's five-Hide-to-one-Leather physical route after one 600-game-second
Textile batch, with no aggregate credit before outbound delivery. P19.C2.5 completes Clothier as
two selected physical cycles: five separately foraged and delivered Fibre produce five Thread,
which a living carrier delivers to finite storage; a later batch carries five Thread back to local
input, weaves one Cloth, and credits it only after outbound delivery. P19.C2.6
completes Smithy: two Metal travel through local input, one selected 900-game-second Metalwork
batch produces one whole Tool, Weapon, or Armor, and aggregate credit waits for outbound delivery. All six make their
ordered/repeatable/pausable selected queues authoritative. Their
`logs_to_planks`, `stone_to_blocks`, and `planks_and_blocks_to_tools` recipes are
founding-available in fresh rules-v1 colonies; their studies gate later recipes. Tannery requires
the Textiles entitlement, as does Clothier; Toolmaking, Weaponsmithing, and Armorsmithing independently gate the
three Smithy selections. Rules-v0 saves remain grandfathered. Woodworking's old
`wood_craft_progress`, Clothier's old hidden clothing timer, and Smithy's old aggregate forge
timers are frozen and preserved only for save compatibility. P19.C3 now creates one stable finite
Tool/Weapon/Armor ID in local output, carries that same ID to storage before derived scalar credit,
and preserves it through equipment, work/combat wear, repair, exact sale, death recovery, and
restart without double-counting. Separate additive trade-craft timers are not selected queue recipes.

## Implemented finite-item condition contract

Each unit has a stable ID, material, kind, quality, value, physical weight, and current/maximum
durability. Relevant work wears finite units; reaching zero leaves a broken unit rather than
deleting it. A signed player repair targets that stable ID and succeeds only at the appropriate
completed, staffed workshop with a living worker and one visible unit of matching material. The
durability research effect scales restored condition. These values and actions survive SQLite
restart, and the Goods panel makes condition and repair visible.

One signed caravan sale may transfer at most 20,000 grams of items. The finite-item loop, ten
selected Bone/Gem/Clay/Sand variants, complete generated material/recipe breadth, and functional
Tool/Weapon/Armor chains are verified and must retain their single
finite authority. All ten maintained processors, including Smithy's three selected recipes, already
use physical station-local logistics.

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

The four canonical Mug materials are live selected recipes: Wood at Woodworking, Stone and Bone at
Stone Prep, and Metal at Smithy. Clay and Sand retain their additional pottery/glassy Mug variants;
they do not replace the canonical four-material requirement. Every output keeps one finite identity
through station output, paws, storage, trade, and restart. The generalized integrated campaign for
the new canonical routes passes.

## Traders / caravans
Periodic **visiting traders** (like DF caravans): a trader arrives at the village (walks to the
shrine/market), **stays a while**, and opens a **trade menu** — sell your surplus/crafts for
value/coin, buy goods/materials you lack (esp. things from biomes you can't yet reach). A simple
value/coin economy; relations/price could deepen later. Ties to: the market building, the goods
above, and reaching distant-biome resources (P17) before you have transport (P16 trains/ships).

The implemented physical contract is stricter than the original sketch. Each visit receives a
deterministic bounded-search exterior that is genuinely passable and routeable to the existing
shrine contact tile; no new market building is implied. The wagon uses ordinary obstacle-aware
A* through the retained gate, waits and replans when closed, never gains water walking from
Shipping, and receives no abstract Rail speed bonus. Trade actions remain invalid until physical
shrine arrival. The exact arrival phase, destination, exterior, visit number, trading deadline,
finite resource manifest, purse, and purchased item cargo persist across restart. Buying removes
exact stock and can sell out; selling transfers exact stable item IDs and is bounded by the
merchant's remaining coin and 100 kg wagon capacity (with the existing 20 kg per-action item-load
limit). Restocking happens only when a new deterministic visit begins. When the deadline expires,
the wagon follows the same physical rules back to its persisted exterior and despawns only on
arrival.

Simulation transitions use the exact physical time consumed inside each tick, so spawn, shrine
contact, deadline, exterior arrival, and a following visit are identical under one-second, minute,
hourly, or coarse partitions. If the route is unavailable at a scheduled boundary, a later reopen
starts travel at that later time and never grants backdated movement. Expansion revalidates the
persisted exterior and deterministically chooses another reachable outside tile if the old one has
become claimed.

The trade panel opens only during physical shrine contact. It stays within the 1024×768 support
bound and pages every colony craft offer six rows at a time, so a large finite inventory never hides
actionable item identities. Disabled buy guidance may use only the Accountant's reported pile books;
exact unreported stock or headroom remains server-private, and an exact signed action can still fail
with a generic storage denial when the books are stale.

## Scope / sequencing (completed; builds on P12.4b)
This large economy layer landed in the following order:
1. **Item/material model** (cat-sim): `Material`, `ItemKind`, `Item`, value fn; extend Resources
   from a fixed struct toward an item/material store (careful — Resources is used everywhere;
   keep the core survival resources fast, add an item store alongside).
2. **Material-variant recipes + workshops** (P12.4b): each workshop crafts item-kinds from allowed
   materials into the store.
3. **Traders** (server + sim): spawn/visit schedule, trade menu action, value/coin.
4. **Client**: trade UI, item/goods in inspectors + stockpiles + the DF-Steam UI (P18).
The ordering is retained as implementation history. Determinism and survival remain regression
contracts for any future economy extension.
