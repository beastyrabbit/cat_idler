# Art Direction — Idle Cat Forest (top-down, DF-Steam style)

This is the maintained family-level art map. The per-slice catalogs (`ground.md`,
`buildings.md`, `items_ui.md`, `cats.md`) hold the detailed mappings, while the exhaustive
runtime building grammar lives in `crates/cat-client/src/station_layout.rs`.

## The load-bearing constraint

Colonists use `public/images/cats/cat-sheet.png` — `32x64` cells, eight direction groups,
four walk frames — and the world uses readable pixel art around them. The existing cat and
raider sheets are accepted project assets; replacing them is not a maintained release task.

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
| Residential/fallback facades | **Tiny Town** (composited multi-tile) | 16px tiles → pre-baked ~32×48 PNGs | `Tiny Town/Tiles/tile_NNNN.png` |
| Open stations | **Roguelike Base + tracked interior props** | repeated 16px floor cells + typed props | `public/images/game/{interior,props,farm}/` |
| Farm plots + crop growth stages | **Pixel Platformer Farm Expansion** | 18px → downscale to 16 | `Pixel Platformer Farm Expansion/Tiles/` |
| Enemies / critters / raiders | **Roguelike Dungeon + Characters Packs** | 16px, same style as Base | `Roguelike Dungeon Pack/`, `Roguelike Characters Pack/` |
| Cats / colonists | **cat-sheet.png** + hats | 32×64 walk sheet, 32×32 hats | already tracked in `public/images/cats/` |
| HUD / UI / resource icons | **Board Game Icons** + **UI Pack – Adventure** + **Fish Pack** | vector, recolorable | `Icons/Board Game Icons`, `UI assets/UI Pack - Adventure`, `2D assets/Fish Pack` |

**Why not one single pack:** the world is a blend of two Kenney **16px pixel** packs, which
is coherent enough (same resolution/idiom; the cats sit on both). Evidence for the split:
- **Roguelike City Pack — REJECTED** (verified via its `Sample.png`): modern-urban (asphalt,
  cars, traffic lights, glass towers). Wrong theme for a medieval cat forest.
- **Roguelike Base Pack buildings — REJECTED for exteriors** (verified via the sheet): the
  structure tiles are interior furniture + modular door/window/wall components, not standalone
  exterior buildings. Its terrain, trees, and props (barrels/sacks/chests/gold-piles/graves)
  ARE excellent and adopted.
- **Tiny Town — ADOPTED for residential/fallback facades** (verified via `Sample.png`): its
  modular tiles composite into readable exterior village buildings; retained recipes live in
  `buildings.md`. Workplaces now use the open-station grammar instead.
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

**Buildings and stations — `public/images/game/{buildings,interior,props,farm}/`:** residential
rooms use the Tiny Town `den.png` roof silhouette. Non-residential buildings are composed at
runtime as exposed wood, stone, or soil floors with typed top-down props; persistent map-name
plaques are not part of the art direction. The 24 current protocol building variants have an
explicit treatment, including distinct open Mill and Sawmill layouts. The facade PNGs remain
review/fallback choices in `docs/sprite-review.html` rather than the workshop renderer.

**Stockpile props — `public/images/game/props/`:** from the **Roguelike Base** sheet
(barrels, sacks, chests, gold/ore piles, gravestone, campfire, well) — these grow/shrink as
visible stockpiles.

**Farm plots — `public/images/game/farm/`:** **Farm Expansion** tilled-soil + crop growth
stages (sprout → tall → mature → flowering) + scarecrow.

**Enemies — `public/images/game/enemies/`:** Roguelike Dungeon/Characters monster sprites
for fox/badger/bear/rival-beast; rival-cat raiders keep the wired `raider-sheet.png`.

**Icons / UI — `public/images/game/icons/`:** per `items_ui.md`. The resource icons are
tracked PNGs used by the Bevy HUD. Semantic Adventure panel, button, progress, minimap, and
32px cursor PNGs are tracked under `public/images/game/ui/` and integrated through Bevy sliced
images and custom cursors. Native and optimized-WASM framebuffers are verified; the remaining
integrated native building/wall campaign is tracked in `docs/IMPLEMENTATION_AUDIT.md`.

## Source mapping

- Terrain, buildings, enemies, icons, and UI come from the selected Kenney packs.
- `cat-sheet.png` and `raider-sheet.png` share the `32x64`, eight-direction, four-frame runtime
  atlas contract documented in `cats.md`.

## Copy convention

Chosen sprites are copied out of the **gitignored** Kenney pack into tracked
`public/images/game/<slice>/` with semantic names, so nothing at runtime depends on the
ignored source tree. Base-sheet tiles are sliced with
`magick "$SHEET" -crop 16x16+$((col*17))+$((row*17)) +repage dest.png`; City/Dungeon
tiles are `cp`-ed from their individual PNGs.
