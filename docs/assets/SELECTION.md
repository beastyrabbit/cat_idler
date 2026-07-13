# Art Direction — Idle Cat Forest (top-down, DF-Steam style)

**Decision owner:** orchestrator. This doc is the single source of truth; the per-slice
catalogs (`ground.md`, `buildings.md`, `items_ui.md`, `cats.md`) hold the exhaustive
tile-by-tile tables. Where they disagree, this doc wins.

## The load-bearing constraint

Our only cohesive **cat** art is `public/images/cats/cat-sheet.png` — 32×64 cells,
8-dir × 4-frame walk, near-top-down **pixel** art (already wired into the renderer).
No cohesive cat art exists in any other style. **Therefore the whole world must be
pixel-art**, so the 32px pixel cats sit on it without a style break.

That single fact resolves the cross-slice tie: the buildings agent's beautiful
**RTS Medieval** family is 64px flat-vector and would clash with the pixel cats, so it
is **rejected as the world family** (kept only as a reference). The ground agent's
**Kenney Roguelike family (16px pixel)** wins — and it is also the closest match in the
whole Kenney library to the actual **Dwarf Fortress Steam** tileset (chunky, readable,
roguelike pixel). Both agents explicitly deferred this call to the orchestrator, and the
buildings agent pre-conceded "use the 16px set if the ground agent picks 16px roguelike."

## The unified family

| Slice | Family | Format | Path (under `public/Kenney Game Assets All-in-1 3.5.0/2D assets/`) |
|---|---|---|---|
| Terrain + nature + stockpile props | **Roguelike Base Pack** | one 16px sheet, 57×31 grid, 17px pitch | `Roguelike Base Pack/Spritesheet/roguelikeSheet_transparent.png` |
| Exterior buildings | **Tiny Town** (composited multi-tile) | 16px tiles → pre-baked ~32×48 PNGs | `Tiny Town/Tiles/tile_NNNN.png` |
| Farm plots + crop growth stages | **Pixel Platformer Farm Expansion** | 18px → downscale to 16 | `Pixel Platformer Farm Expansion/Tiles/` |
| Enemies / critters / raiders | **Roguelike Dungeon + Characters Packs** | 16px, same style as Base | `Roguelike Dungeon Pack/`, `Roguelike Characters Pack/` |
| Cats / colonists | **cat-sheet.png** (Paws & Whiskers) + hats | 32×64 walk sheet, 32×32 hats | already tracked in `public/images/cats/` |
| HUD / UI / resource icons | **Board Game Icons** + **UI Pack – Adventure** + **Fish Pack** | vector, recolorable | `Icons/Board Game Icons`, `UI assets/UI Pack - Adventure`, `2D assets/Fish Pack` |

**Why not one single pack:** the world is a blend of two Kenney **16px pixel** packs, which
is coherent enough (same resolution/idiom; the cats sit on both). Evidence for the split:
- **Roguelike City Pack — REJECTED** (verified via its `Sample.png`): modern-urban (asphalt,
  cars, traffic lights, glass towers). Wrong theme for a medieval cat forest.
- **Roguelike Base Pack buildings — REJECTED for exteriors** (verified via the sheet): the
  structure tiles are interior furniture + modular door/window/wall components, not standalone
  exterior buildings. Its terrain, trees, and props (barrels/sacks/chests/gold-piles/graves)
  ARE excellent and adopted.
- **Tiny Town — ADOPTED for buildings** (verified via `Sample.png`): its modular tiles
  composite into beautiful, readable exterior village buildings; per-type composite recipes
  live in `buildings.md`.
- **RTS Medieval — REJECTED as world family**: 64px flat-vector, clashes with the pixel cats.
  (Optionally borrow ~4 single-sprite gap buildings — mill/chapel/forge/loom — downscaled, if
  a Tiny Town composite reads as a generic house; noted as a caveat, not the default.)

The UI layer is *deliberately* allowed a cleaner/different style from the world tiles —
that's normal (map = pixel, HUD = crisp icons + wood-frame panels).

## Slice → sprite mapping to the sim

The sim (`cat-sim`) emits `BiomeRole {Lowland, Grassland, Forest, Rocky, Highland}` per
tile, plus `river`, and `DecorationRole::{Tree, Rock}`. Terrain sprites are named by role.

**Terrain — `public/images/game/terrain/` (DONE, sliced + eyeball-verified):**

| File | Base sheet (col,row) | Role |
|---|---|---|
| `grass.png` | (5,0) | Grassland / Lowland / Forest ground base |
| `grass_var.png` | (9,1) | grass variation (pebbled) to break up expanses |
| `rocky.png` | (7,0) | Rocky biome (grey stone) |
| `highland.png` | (8,0) | Highland (tan/sand) |
| `water.png` | (0,0) | water body / river fill |
| `water_edge.png` | (0,2) | shore / ripple accent |
| `dirt.png` | (6,0) | bare earth / worn path |
| `farmland.png` | (10,13) | tilled farm-plot base |
| `flowers_red/white/blue.png` | (0,6)/(0,9)/(0,12) | decorative flower scatter |

**Nature decoration — `public/images/game/nature/` (DONE):**
`tree_oak.png` (13,9) 16×32 · `tree_pine.png` (16,9) 16×32 · `stump.png` (13,8).
Trees are 2 tiles tall, bottom-anchored on their lower tile.

**Buildings — `public/images/game/buildings/`:** pre-baked composites from **Tiny Town**
tiles (recipes in `buildings.md`), ~32×48 per building type (den, storehouse, workshop,
smithy, mill, clothier, research_hut, school, barracks, town_hall, market, tent/accounting,
shrine, monument).

**Stockpile props — `public/images/game/props/`:** from the **Roguelike Base** sheet
(barrels, sacks, chests, gold/ore piles, gravestone, campfire, well) — these grow/shrink as
visible stockpiles.

**Farm plots — `public/images/game/farm/`:** **Farm Expansion** tilled-soil + crop growth
stages (sprout → tall → mature → flowering) + scarecrow.

**Enemies — `public/images/game/enemies/`:** Roguelike Dungeon/Characters monster sprites
for fox/badger/bear/rival-beast; rival-cat raiders keep the wired `raider-sheet.png`.

**Icons / UI — `public/images/game/{icons,ui}/`:** per `items_ui.md`. Board Game Icons are
white glyphs → recolor per resource at runtime (CSS/shader mask). Cat-food glyph = Fish
Pack `fish_blue.png`.

## Provenance

- Terrain, buildings, enemies, icons, and UI come from the selected Kenney packs.
- `cat-sheet.png` and `raider-sheet.png` come from the Paws & Whiskers pack. Their
  32×64, 8-direction, 4-frame layout is the runtime atlas contract.

## Copy convention

Chosen sprites are copied out of the **gitignored** Kenney pack into tracked
`public/images/game/<slice>/` with semantic names, so nothing at runtime depends on the
ignored source tree. Base-sheet tiles are sliced with
`magick "$SHEET" -crop 16x16+$((col*17))+$((row*17)) +repage dest.png`; City/Dungeon
tiles are `cp`-ed from their individual PNGs.
