# P17 — 2D biome generator (Minecraft-style, ~25 biomes)

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
- Each **biome** carries properties (a table, like `biomes.rs`):
  - **ground tile / tint**, **decoration set + DENSITY** (this is the fix: plains = few trees,
    forest = dense, desert = cacti/none, tundra = none), **tree species**,
  - **movement-speed factor** (feeds P14.2/movement — e.g. desert/sand slow, plains medium),
  - **resources** available (forest→wood, rocky/mountain→stone, coast/river→fish, plains→farmable
    grass for fields) — drives the scout/gather loop,
  - passability hints (mountain blocks until unlocked; water blocks).

## ~25-biome palette (starting set — tune)
ocean, river, beach/coast, plains/grassland, meadow, flower-field, oak forest, birch forest,
dark/dense forest, taiga, pine forest, jungle, savanna, desert, badlands/mesa, swamp, marsh,
tundra, snowy plains, snowy taiga, ice, highland/hills, rocky peaks/mountains, stony shore,
mushroom/odd. (≈25; trim/merge as needed.)

## Ripple effects (why this is foundational)
- **Tree density per biome** → fixes the uniform-tree look directly.
- **Per-biome movement speed** → feeds the movement-stagger + P14.2 cost model.
- **Per-biome resources** → the scout-driven fog loop ("find 5 wood spots") + gather spots need
  to know which biomes have wood/stone/fish/farmland.
- **Render + assets**: ~25 biomes need distinct ground tiles/tints. Curate from the Roguelike
  sheet (multi-shade grass/sand/snow/stone) or tint a base per biome; a real asset task.

## Sequencing
Foundational terrain change → do it as a focused `cat-sim` card (grow terrain_gen + biomes
table, keep it deterministic + per-chunk + infinite; golden-test a few chunks), THEN the client
biome→tile/tint render + tree-density render. Coordinates with movement-speed (per-biome factor)
and P14.2 (cost). Big but self-contained; slot after the current feel/movement cards land so the
terrain subsystem isn't being edited by two agents at once.
