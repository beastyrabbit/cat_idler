# P16 — Default village blueprint, tile recalibration, roads & movement (user, 2026-07-10)

Detailed founding/economy/spatial design from playtest. Numbers are the spec.

## Tile recalibration (foundation — everything sizes off this)
- The current 1×1 tile renders **too big** → make the render tile ~**1/3** its current size
  (TILE px ≈ 28 → ~9–10). Interpretation: this is a **render-scale + footprint recalibration on
  the existing integer grid**, NOT a grid subdivision — cats already pathfind per-tile; the world
  just has more, smaller tiles (fine, terrain is infinite). Confirm if that's wrong.
- **Footprints** (in the new small tile units): early **house 2×3**; **workshop 3×3**; **shrine
  3×3** but reserves a **5×5** (a 1-tile road ring around it); **road tile 1×1**; **tree 2×3**;
  small props **1×1**. (Recalibrates P14.1: house 2×2→2×3, workshop 2×3→3×3, add tree 2×3.)
- Cats pathfind on the 1×1 grid (already per-tile; grid is finer now).

## Default founding village (replaces organic start)
A fixed starting blueprint:
- **Shrine** 3×3 dead center, reserving 5×5 (road ring).
- **Roads from the shrine out to the wall in N/S/E/W** (stone roads by default). The wall has a
  **single gate in the SOUTH** (the one opening).
- **3 early houses** (2×3 each), **5 cats** total.
- **Wood-cutting workshop** (3×3): logs → **planks**.
- **Stone-prep workshop** (3×3): raw stone → **prepped stone / blocks**.
- **Woodworking workshop** (3×3): planks + stone → **tools** (axe, shovel, fishing rod, …) + weapons.
- **General stockpile** pre-filled at founding: **50 wood, 50 food, 10 stone**.
- **New house cost**: X planks + X stone (was materials — now the plank/stone chain gates growth).

## Resources & chains (this is the P12.4b expansion, now specified)
New resource types + chains:
- **logs/wood** → wood-cutting workshop → **planks**
- **raw stone** → stone-prep workshop → **prepped stone / blocks**
- **planks + prepped stone** → woodworking workshop → **tools** (axe/shovel/fishing-rod) + weapons
- Buildings consume planks + stone (house = X planks + X stone). Tools presumably boost the
  relevant labor later (axe→woodcutting, rod→fishing, shovel→digging/farming).

## Roads & movement speed
- **Stone roads**: player/leader-**built**, dark-grey, **175%** move speed. Stone ground itself
  **cannot** auto-form dirt roads.
- **Dirt roads**: **auto-form** on any (non-stone) tile that gets heavy foot traffic; **105%** speed.
- **Base surface speed**: **stone ground 100%**, **grass 75%**, **sand 50%**, dirt road 105%,
  built stone road 175%. (Bare grass is the slow default; roads/stone speed cats up. Extends the
  existing pathfinding cost tiers — invert cost↔speed and add the built-stone-road + dirt-auto tiers.)

## Terrain passability rules
- **Water = impassable** (cats cannot move through — already enforced in pathfinding).
- **Mountain = impassable at first**, becomes passable/mineable only after an **upgrade** (new
  terrain type gated by the upgrade tree; blocks the walk grid until unlocked).
- **Fields can only be placed on grass** tiles (farm-plot placement validates grass, like the
  building footprint occupancy checks).

## Build order (foundational → dependent)
1. **Tile recalibration + footprints** (render TILE shrink + footprint sizes; recal P14.1). Client
   render + sim footprint constants. Do with the footprint-render/y-sort card (P14.5).
2. **Default village blueprint** (sim founding): fixed shrine+roads+gate-south+3 houses+3 workshops
   +pre-filled general stockpile+5 cats. Replaces organic founding. (After keep-cats-busy lands —
   both touch world_tick founding/director.)
3. **Resource chains** (P12.4b): logs→planks, stone→blocks, →tools/weapons; house cost = planks+stone.
   Big Resources-struct expansion — the focused card I deferred.
4. **Roads + movement speed**: built stone roads (175%) + auto dirt roads (105%) + surface speeds
   (stone 100% / grass 75%); render dark-grey stone vs worn dirt. (Extends roads.rs + pathfinding
   cost model + snapshot road tiles + client render.)

## Open decision
- **Tile granularity**: render-shrink + footprint recalibration on the existing grid (assumed), vs
  a real 3× grid subdivision (bigger). Assuming the former unless told otherwise.
