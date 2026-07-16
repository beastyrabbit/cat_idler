# P17 — 2D biome generator (Minecraft-style, ~25 biomes)

> **Living target spec.** The deterministic 26-biome climate generator and rendering exist.
> Fine-biome movement, finite physical fishing, finite Gem/Clay/Sand source ecology, and
> constructed staffed rail/ship transport are verified. Downstream material recipes remain open in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

User (2026-07-10): trees should be sparse on grass biomes and dense in forest biomes, etc. —
we need a proper **biome generator, like Minecraft but 2D**, targeting **~25 biomes**.

## What exists today (extend, don't rebuild)
`crates/cat-sim/src/terrain_gen.rs` already does deterministic per-chunk value-noise terrain:
continuous **elevation + moisture** → quantized heights → 5 coarse `BiomeRole`
{Lowland, Grassland, Forest, Rocky, Highland} + rivers + tree/rock decoration. `biomes.rs`
has a richer `BiomeType` table (oak/pine/dead forest, …) with per-type properties. So we have
noise fields + a biome-table pattern — P17 grows the palette to ~25 and makes a clean
climate→biome map with per-biome decoration density.

## Design: climate-driven biome map (Minecraft's model, 2D)
- Sample low-frequency **climate noise** per tile (deterministic from `world_seed`, per-chunk,
  infinite — same machinery as now): **temperature**, **humidity/moisture**, and reuse
  **elevation/continentalness** (already there). Optionally a 4th (weirdness/erosion) later.
- A **biome lookup**: map (temperature band × humidity band), modified by elevation (mountains
  high, beach at coast, ocean/river low) → a biome from the ~25-palette. Hard borders are fine
  to start; add edge blending later.
- **Biome SCALE ~10× the village** (user): climate noise is very low-frequency so each biome
  region is large — you start in one biome (grass near water) and **other biomes are a journey
  away → reached mid/late game**, not early. Tune the noise wavelength for ~10×-village regions.
- Each **biome** carries properties (a table, like `biomes.rs`):
  - **ground tile / tint**, **decoration set + DENSITY** (this is the fix: plains = few trees,
    forest = dense, desert = cacti/none, tundra = none), **tree species**,
  - **movement-speed factor** (feeds P14.2/movement — e.g. desert/sand slow, plains medium),
  - **resources** available (forest→wood, coast/river→fish, plains→farmable grass) — drives the
    scout/gather loop. **Mining rules**: **mines only on MOUNTAIN biomes** (stone + ore); on
    **stone/gravel** biomes you can collect stone too but **only tiny amounts** (a trickle);
    other biomes = no mining.
  - **crop-growth multiplier** (per-biome fertility): e.g. **grass ~80%**, **marsh/fertile ~150%**,
    desert/tundra ~0 (not farmable). Relaxes P16's "fields only on grass" → fields go on any
    **farmable** biome, and the biome's fertility scales how fast the crop grows (P12.5 farm plots),
  - passability hints (mountain blocks until unlocked; water blocks).

## ~25-biome palette (starting set — tune)
**Water tiers** (all impassable): **river** (thin, winding — one biome), **lake/sea** (medium),
**ocean** (big seas — large open water). Villages found near river/lake (P16).

ocean, river, beach/coast, plains/grassland, meadow, flower-field, oak forest, birch forest,
dark/dense forest, taiga, pine forest, jungle, savanna, desert, badlands/mesa, swamp, marsh,
tundra, snowy plains, snowy taiga, ice, highland/hills, rocky peaks/mountains, stony shore,
mushroom/odd. (≈25; trim/merge as needed.)

## Ripple effects (why this is foundational)
- **Tree density per biome** → fixes the uniform-tree look directly.
- **Per-biome movement speed** → feeds the movement-stagger + P14.2 cost model.
  This is now live: A* pays inverse fine-biome speed and physical movement consumes elapsed time
  at each crossed biome/road/obstacle boundary. One prewarmed derived chunk cache serves every
  mover and A* candidate in the tick; claimed non-agricultural settlement ground remains cleared
  meadow, while exterior agricultural claims retain their generated biome.
- **Per-biome resources** → the scout-driven fog loop ("find 5 wood spots") + gather spots need
  to know which biomes have wood/stone/fish/farmland.
- **Render + assets**: ~25 biomes need distinct ground tiles/tints. Curate from the Roguelike
  sheet (multi-shade grass/sand/snow/stone) or tint a base per biome; a real asset task.
- **Distant-biome logistics (late-game upgrades)**: because biomes are ~10× the village and far
  apart, reaching a distant biome (e.g. a far **marsh** to farm, or a mountain to mine) needs
  **transport upgrades** — **trains** (land) and **ships** (water/ocean) unlocked via the upgrade
  tree — to haul over long distances. Late-game; gated behind the tech tree + escalating cost.

## Sequencing
Foundational terrain change → do it as a focused `cat-sim` card (grow terrain_gen + biomes
table, keep it deterministic + per-chunk + infinite; golden-test a few chunks), THEN the client
biome→tile/tint render + tree-density render. Coordinates with movement-speed (per-biome factor)
and P14.2 (cost). Big but self-contained; slot after the current feel/movement cards land so the
terrain subsystem isn't being edited by two agents at once.

## Fine-movement verification (2026-07-14)

- Generated-biome cache, inverse A* cost, actual route choice, road and soft-obstacle composition,
  claimed-meadow authority, and tick-partition-independent boundary crossing have focused tests.
- All 11 signed guided/manual campaign cases pass after the integration.
- Passive seeds 7, 42, and 20240712 each pass four game-hours at the live 1-second cadence as
  byte-identical twins, with no death/reset and fog growth from 289 to 459, 414, and 474 tiles.
- The complete `cat-sim` gate passes 981/981 tests (one separately skipped).
