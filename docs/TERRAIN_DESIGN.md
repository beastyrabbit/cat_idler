# Terrain Overhaul — Isometric Nature Design

> **SUPERSEDED.** Isometric sprite work for the TypeScript web game (frozen on branch
> `archive/web-game`). The Rust/Bevy client renders a flat **top-down** world with Kenney
> Roguelike 16px sprites instead — see `docs/assets/SELECTION.md` and `docs/ARCHITECTURE.md`.
> Kept as design history.

Rebuilding the world map on the Kenney **Isometric Nature** pack
(`public/Kenney Game Assets All-in-1 3.5.0/2D assets/Isometric Nature/PNG/`).
This is a geometry change: the current map uses the "Miniature" packs
(256×512 canvas, 256×128 diamond); Nature is 220×379 canvas, **180×115**
ground diamond (per the pack's `Tile information.txt`). We adopt Nature's
native ratio rather than stretching sprites.

Companion tool: **`/dev/tiles`** (`app/dev/tiles/page.tsx`) renders every
group's four rotations with per-group annotation + JSON export — use it to
resolve the opaque `naturePack_NNN_R` numbering into named roles.

## Pack inventory (measured)

- **752 PNGs.** Naming: `naturePack_NNN_R.png` (NNN = object group, R = 0..3
  rotation of the same object) and `naturePack_flat_NNN_R.png`.
- **188 groups × 4 rotations:** numeric groups `001`–`175` (contiguous) plus
  `flat_001`–`flat_013`.
- Every sprite is **220×379**; the ground diamond is **180×115** anchored near
  the bottom, with tall content (cliffs, trees) rising above it — same
  bottom-anchored convention the current renderer already assumes.
- From the pack Preview: flat ground tiles, single/multi-floor **height/cliff
  blocks**, **stairs**, rounded hills/mountains, **river** start/end segments
  embedded in flat tiles (no lakes), a log/bridge piece, many **tree** types
  (deciduous, conifer, autumn), **rocks/stones**, small **plants**, a tent, and
  a fox/animal. No roads (see §4).

## 1. Isometric terrain auto-generation

Target: a heightmapped world where cliff/edge tiles are correctly **oriented**
from neighbor height deltas, biomes layer on top, and rivers carve with
oriented start/end segments.

**Pipeline (all in `lib/game/`, pure, seeded — extends the existing
`worldGen.ts` + `seededRng.ts`):**

1. **Height field.** Sample layered value/Perlin/simplex noise per tile
   (fractal sum of octaves) → continuous elevation, then quantize to a small
   number of discrete floors (e.g. 0–4) matching how many cliff-floor variants
   the pack actually provides (confirm counts via `/dev/tiles`). Quantized
   levels are what drive tile selection; keep the continuous value for biome
   moisture/temperature too. Reference: Red Blob Games, *Making maps with noise
   functions* (redblobgames.com/maps/terrain-from-noise) and *Polygonal Map
   Generation* for the elevation→moisture→biome model.

2. **Cliff orientation via autotiling / bitmasking.** For each tile, compare
   its floor to its 4 (edges) or 8 (edges+corners) neighbors. Encode the set of
   *lower* neighbors as a bitmask and map that mask → a specific cliff sprite +
   rotation. This is the standard **bitmask/blob tileset** technique:
   - 4-bit edge mask → 16 cases (fast, coarse); 8-bit → the **47-tile "blob"**
     set for clean corners. Choose per available art.
   - References: "How to Use Tile Bitmasking" (gamedev/quin`t`), boristhebrave's
     *Tileset roundup* / *Wang tiles* articles, and the Godot TileSet terrain
     ("autotile") docs — all describe mask→tile lookup tables.
   - Nature ships each object in 4 rotations, so we mostly need to pick the
     *base* cliff/edge/corner shape and the *rotation index* (0..3 = N/E/S/W)
     from the mask, instead of authoring 47 unique sprites. The `/dev/tiles`
     annotations should record, per group: role (`cliff-straight`,
     `cliff-inner-corner`, `cliff-outer-corner`, `stairs`) and which rotation
     faces which compass direction.

3. **Biome layering.** After heights, assign biome per tile from
   (elevation, moisture, latitude/temperature) — the existing `biomes.ts` /
   `worldGen.ts` classification can feed sprite/decor choice: ground tint +
   scattered tree/rock/plant groups drawn as *object* sprites on top of the
   ground diamond. Keep biome purely cosmetic over the height/cliff structure so
   generation stays layered and testable.

4. **River carving with oriented segments.** Rivers are **start/end segment**
   art (no lakes), so model a river as a path: pick a high-elevation source,
   flow downhill (steepest descent on the height field, as in Red Blob's river
   generation), and lay a sequence of river-tile variants. Each segment tile
   picks base + rotation from its in/out neighbor directions (a 4-neighbor
   connection mask, same lookup style as cliffs: straight, bend, source-cap,
   mouth-cap). Rivers force the underlying tiles to floor 0 (carved) and block
   building. Bridges (see §4) are the only crossings.

5. **Transition tiles.** Where the pack provides beach/edge blends (Nature is
   thin here; Roads Base is rich — §4), select them by the same neighbor-mask
   approach at biome/water boundaries.

**Determinism:** every stage seeds from `setTestRngSeed`; add boundary unit
tests in `tests/unit/game/` for mask→tile lookups (each cliff/river mask maps
to exactly one base+rotation) per the testing contract.

## 2. Renderer: keep DOM, or move to canvas?

**Current renderer** (`components/map/TileLayer.tsx`): one `<img>` per tile
(plus an optional grass `base` underlay), chunked at 12 tiles/chunk, 60s fetch
cache, viewport-culled via `visibleChunksIso`, painter's z-order from
`isoProjection.zIndexFor`, pan/zoom by CSS transform on a content plane. React
19, ~1 Hz dashboard updates.

**Why Nature stresses DOM.** Today each *cell* is ~1–2 DOM nodes. With heights,
a single map column becomes a **stack**: ground + N cliff floors + optional
river overlay + 0–3 decor objects (trees/rocks). That is 3–6× the nodes, taller
overdraw (bigger cull padding for tall cliffs), and per-cell z-ordering that now
depends on height, not just `x+y`. Thousands of transformed `<img>` with
per-node z-index is where DOM layout/compositing starts to hurt on pan/zoom, and
DOM gives us no cheap pixel-accurate tile picking under stacked sprites.

**Recommendation: hybrid — a single `<canvas>` terrain layer, DOM for actors.**

- **Terrain → one `<canvas>`.** Blit cached sprite `Image` objects in painter's
  order using the *same* `isoProjection` math (reuse `tileToIso`, extend
  `zIndexFor` for height). Collapses thousands of nodes into one element, gives
  exact z-ordering with elevation, and enables pixel/ray tile-picking
  (screen→tile already exists as `isoToTile`; add height-aware hit testing).
  Pan/zoom becomes a canvas transform (`setTransform`) — no per-node CSS.
- **Cats + buildings + tooltips → stay DOM overlays**, absolutely positioned via
  the same projection. There are dozens, they're interactive, and React keeps
  owning them (hover cards, click actions in `useGameDashboard`). Current cat/
  building sprites are Miniature-style (256×512); re-scale them to Nature's
  diamond (factor ≈ 180/256 ≈ 0.70 on width) or, longer term, swap to
  Nature-scale character art. Keep them a hair above their tile's object z-slot.
- **Escalation path — PixiJS** (`@pixi/react` or bare Pixi in a ref): adopt only
  if the custom canvas layer bottlenecks on sprite count/effects. Pixi adds
  WebGL batching (big win at this sprite volume), a scene graph, and culling for
  ~1 extra MB of dependency and some React-integration friction. Phaser is
  heavier/game-loop-oriented and a poor fit for a mostly-static, React-driven
  map — not recommended.

**Migration sketch (incremental, keeps the game shippable):**
1. Land the geometry constants for Nature (§3) behind a flag; keep DOM tiles.
2. Add a `CanvasTileLayer` that reads the same chunk API + `isoProjection` and
   draws ground+cliff+river to canvas; render it *under* the existing DOM
   object/cat/building layers. Swap TileLayer→CanvasTileLayer behind the flag.
3. Add height-aware z-order and canvas tile-picking; port fog-of-war shading to
   a canvas pass (or a translucent DOM overlay).
4. Delete the DOM tile path once parity is verified. Cats/buildings never move
   off DOM.

## 3. Geometry plan

- **Adopt Nature's native diamond.** New `IsoGeometry`: `tileWidth: 180`,
  `tileHeight: 115` (note: not exactly 2:1 — 180/115 ≈ 1.565; keep the real
  ratio, do **not** force 2:1), `imageHeight: 379`. Recompute `surfaceOffset` /
  `surfacePadding` from the pack (diamond top vertex sits at
  `379 − (diamond_bottom_margin) − 115` within the canvas — measure one
  `flat` tile in `/dev/tiles` and set exactly). `isoProjection.ts` is already
  fully parameterized by `IsoGeometry`, so most math is unchanged; only the
  constant in `components/map/constants.ts` (`DEFAULT_ISO_GEOMETRY`) changes.
- **Elevation offset.** Draw a tile on floor `f` at `top − f * FLOOR_PX`, where
  `FLOOR_PX` = the pack's per-floor pixel rise (measure a 1- vs 2-floor cliff in
  `/dev/tiles`; typically ~½ diamond height). Cliff sprites fill the vertical
  gap between floors so columns read as solid.
- **Z-ordering with height.** Extend `zIndexFor` to
  `(depth * (MAX_FLOORS+1) + floor) * 2 + objectBit`, so a taller tile in front
  still occludes a shorter tile behind. Decor/actors take the `objectBit` slot
  above their own tile+floor.
- **Stairs** connect adjacent floors: place a stair object where two neighbor
  columns differ by exactly one floor and a path crosses (or player-built),
  rotation chosen from the ascent direction. Pathing (`paths.ts`/roadmap #6)
  treats stairs as the only walkable vertical link between floors.
- **Actors on slopes.** A cat/building sits at its tile's *top* floor surface;
  reuse `tileDiamondCenter` + the floor offset for placement.

## 4. Roads (and bridges) — the pack has none

Two in-collection, iso-compatible options were inspected:

- **`Isometric Vector Roads Base`** — 100×65 tiles, **semantically named**
  (`road`, `dirt`, `crossroad*`, `end{N,E,S,W}`, `beach*` water-edge blends,
  `bridgeEW`/`bridgeNS`, plus `conifer*`). Grass-topped, so roads and beaches
  drop straight onto grass. Ratio 100/65 ≈ 1.54 ≈ Nature's 1.565. **Half
  scale** — upscale ×1.8 → 180×117 (≈115). Also `Isometric Vector Roads Water`
  adds bridges/banked crossings (100×77).
- **`Isometric Modular Roads`** — 180×**125** tile, **native 180 width** but
  opaque `roadTile_NNN_R` numbering (1208 files) and a diamond 10px taller than
  Nature's 115 (~9% — roads sit on ground so minor overlap is tolerable).

**Recommendation: use `Isometric Vector Roads Base` (+ Roads Water for
bridges), upscaled ×1.8 to Nature's 180-wide diamond.** Its named pieces make
autotiling trivial (the same neighbor-mask lookup as §1: straight/bend/tee/
cross/end by connection mask), it *ships beach transitions and bridges* — which
directly serve ROADMAP §1 ("visible roads on trafficked routes", "bridges over
rivers") and §6 ("deliberate road building") — and grass-topped tiles blend with
Nature ground. Fall back to Modular Roads only if the ×1.8 upscale looks soft at
target zoom. Roads remain an *object/overlay* layer keyed by `pathWear` +
player-built flags (as today's `road_built`), not part of ground generation.

## Open items to resolve with `/dev/tiles`

- Which numeric groups are ground vs cliff-floor-1/2/3 vs stairs vs rivers vs
  trees vs rocks vs plants, and **which rotation index = which compass edge**.
- Exact `surfaceOffset` and per-floor `FLOOR_PX` (measure flat vs multi-floor).
- River segment vocabulary (source-cap, straight, bend, mouth) and their masks.
- Cliff variant coverage → decide 4-bit (16-case) vs 47-blob autotiling.

## Sources

- Red Blob Games — *Terrain from noise functions*, *Polygonal Map Generation
  (Voronoi)*, isometric/grid references (redblobgames.com).
- Tile bitmasking / autotiling — "How to Use Tile Bitmasking to Auto-Tile Your
  Level" (gamedevacademy), boristhebrave.com *Wang tiles* & tileset roundup,
  Godot TileSet **terrains/autotile** docs (47-tile blob set).
- Marching-squares tiling and the 2×2 corner (dual-grid) autotiling variant for
  clean transitions.
- Kenney Isometric Nature / Vector Roads Base / Modular Roads pack
  `Tile information.txt` and `Preview.png` (measured above).
