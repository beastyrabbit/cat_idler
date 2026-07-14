# P16 — Default village blueprint, tile recalibration, roads & movement (user, 2026-07-10)

> **Living target spec.** The original five-cat start was superseded by the 2026-07-13
> playtest direction: every ordinary personal village starts with **15 adult cats in three
> five-bed Dens**. Founding/housing, authoritative interior clearing, exterior water, and the
> exact dirt/stone road model are verified. Selectable/removable gather controls, finite shoreline
> fishing, persisted exterior agricultural territory, physical farm labor, and physical
> Mill/Sawmill/Workshop/Smelter routes are live; the remaining workshop routes remain open. See
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

Detailed founding/economy/spatial design from playtest. Numbers are the spec.

## Tile recalibration (foundation — everything sizes off this)
- **Implemented authority:** one integer simulation/pathfinding cell corresponds to one 16×16
  source-art tile. The Bevy client scales that cell to `TILE = 10` world units and camera zoom
  determines screen size; there is no 3× logical-grid subdivision. This is the settled
  render-scale + footprint recalibration, not an open interpretation.
- **Footprints** (in the new small tile units): early **house 2×3**; **workshop 3×3**; **shrine
  3×3** but reserves a **5×5** (a 1-tile road ring around it); **road tile 1×1**; **tree 2×3**;
  small props **1×1**. (Recalibrates P14.1: house 2×2→2×3, workshop 2×3→3×3, add tree 2×3.)
- Cats pathfind on the 1×1 grid (already per-tile; grid is finer now).

## Default founding village (replaces organic start)
**Anchor placement rule:** a village always spawns on a **grass** biome and must have **water
nearby** (a **river** or a lake/sea within a short radius) as a water source. World/founding gen
picks (or guarantees) a grass site adjacent to water; "river" is one of the P17 biomes. (Replaces
the current fixed flat-plateau anchor with a grass+water search.)

A fixed starting blueprint:
- **Shrine** 3×3 dead center, reserving 5×5 (road ring).
- **Roads from the shrine out to the wall in N/S/E/W** (stone roads by default). The wall has a
  **single gate in the SOUTH** (the one opening).
- **3 early Dens** (house form, 2×3 each), **15 adult cats** total; each Den provides exactly
  **5 permanent beds**.
- **Wood-cutting workshop** (3×3): logs → **planks**.
- **Stone-prep workshop** (3×3): raw stone → **prepped stone / blocks**.
- **Woodworking workshop** (3×3): planks + stone → **tools** (axe, shovel, fishing rod, …) + weapons.
- **Finite general storehouse** pre-filled at a personal founding with **50 food, 100 water,
  16 herbs, 60 general materials, 10 planks, and 10 blocks**. The larger communal blueprint
  receives twice that runway. Logs, lumber, grain, flour, fibre, hide, cloth, leather, ore,
  metal, fish, tools, weapons, armor, catnip, refined goods, and blessings begin at zero and
  must enter through their real chains.
- **New house cost**: X planks + X stone (was materials — now the plank/stone chain gates growth).

Population rules attached to this blueprint:
- A fresh run has no spare beds. Pregnancy begins only after 36 game-hours, only when food/water
  are healthy, and only after reserving a permanent future bed; gestation is 18 game-hours.
- Prosperity migration begins after 30 game-hours and samples at 12-game-hour intervals. It
  requires at least 4 food and 5 water per currently present cat (including probationers) plus
  construction wealth worth 0.5 raw materials per cat (floor 8); directly buildable planks,
  blocks, and lumber count at their raw-input value. A deterministic one-cat cohort arrives when
  those bars are met. The real arrival may work and consume resources while unhoused, but leaves
  after a 36-game-hour probation unless a permanent bed opens.
- Extinction reset atomically rebuilds the 15-adult/three-Den state and clears stale work,
  reservations, officers, and migration probation. New-run identities are deterministic and
  cannot collide with the prior run.
- An emergency water shortage schedules a real source→carry→deposit fetch; it never grants a
  free resource bucket.
- The maintained idle-game old-age thresholds are 240 game-hours for ordinary cats and 288 for
  leaders/healers, deliberately replacing the archived prototype's 48/57.6-hour thresholds.

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

## Gather spots (temp drop points — decouple gathering from hauling)
- Player/leader can **build a "gather spot"** — a **temporary**, resource-specific drop point that
  may be placed **outside the village** (unlike the general stockpile). Types: wood, farm, fishing,
  mining, etc.
- **Split the work**: a **gatherer** (wood cutter / farmer / fisher / miner) works the nearby
  resource and drops yield into the adjacent gather spot (short trips → stays productive); a
  **mover/hauler** then carries the goods from the gather spot back to the village stockpiles/shrine
  (the long trips). Net: faster throughput, specialists don't waste time walking.
- Implementation: a gather spot is essentially a **temp, single-resource stockpile placeable
  outside claimed ground**, plus a **hauling job** (mover role, ties to Steward/logistics) that
  routes gather-spot → village stockpile. Reuses P12.3 stockpiles + haul-fill (deposit at nearest
  accepting pile) + the officer/role system. "Temporary" = expires or is cleared when the nearby
  resource is exhausted.

## Build order (foundational → dependent)
1. **Tile recalibration + footprints** (render TILE shrink + footprint sizes; recal P14.1). Client
   render + sim footprint constants. Do with the footprint-render/y-sort card (P14.5).
2. **Default village blueprint** (sim founding): fixed shrine+roads+gate-south+3 Dens+3 workshops
   +pre-filled general stockpile+15 cats. Replaces organic founding. (After keep-cats-busy lands —
   both touch world_tick founding/director.)
3. **Resource chains** (P12.4b): logs→planks, stone→blocks, →tools/weapons; house cost = planks+stone.
   Big Resources-struct expansion — the focused card I deferred.
4. **Roads + movement speed**: built stone roads (175%) + auto dirt roads (105%) + surface speeds
   (stone 100% / grass 75%); render dark-grey stone vs worn dirt. (Extends roads.rs + pathfinding
   cost model + snapshot road tiles + client render.)

## Implemented decision
- **Tile granularity is closed:** retain the existing one-cell logical grid, use one 16×16
  source-art tile per cell, and achieve the smaller on-screen scale through the client's 10-unit
  world spacing and camera. Footprints and pathfinding use the same integer cells.
