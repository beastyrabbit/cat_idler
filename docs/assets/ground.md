# Ground & Nature Tile Catalog — Idle Cat Forest

Curated top-down terrain + nature art for the Dwarf-Fortress-Steam-style forest colony sim.
Scope: **ground terrain + nature props** (grass / forest floor / water / dirt / stone / sand / farmland /
flowers, plus trees / bushes / rocks / stumps). Buildings, characters, and UI are out of scope.

All candidates below are 16×16 top-down and CC0. Isometric packs, 8×8 (Micro Roguelike), monochrome,
side-view platformer, and smooth-vector families were excluded up front.

---

## TL;DR recommendation

**Primary terrain family → `RTS Medieval (Pixel)`.** It is the one pack that satisfies the selection rule
("one cohesive family covering grass / forest / water / dirt / stone at a consistent tile size") — 207
individual 16×16 PNGs with grass, sand, dirt, stone, two water types, dirt-path autotiles, grass-bordered
ponds, plus trees, rocks, bushes, stumps, and mushrooms. Clean chunky pixel style, subtle outlines,
Stardew/DF-Steam-adjacent. Ships pre-sliced (no spritesheet slicing needed).

**Strong stylistic alternative → `Tiny Town` + `Tiny Battle`** (same "Tiny" family, both 16×16 individual
PNGs). Boldest, cutest, highest-contrast chunky look. Caveat: **Tiny Town alone has no water and no stone
ground** (grass + dirt only), so it is *not* a complete single-family primary — you must pair it with Tiny
Battle for water/roads, and even then the family lacks a stone-ground tile and sand. Pick this pair only if
the cute chunky look outweighs terrain completeness.

**Runner-up (single-sheet) → `Roguelike Base Pack`** — comparably complete, but it's one spritesheet that
must be sliced, and its water reads teal. Kept below as a fallback.

### Families evaluated

| Family | Tile | Format | Grass | Forest/trees | Water | Dirt/path | Stone | Verdict |
|---|---|---|---|---|---|---|---|---|
| **RTS Medieval (Pixel)** | 16×16 | 207 individual PNGs | yes | yes (pine+round, clusters) | yes shallow + pond | yes 2 path autotiles | yes ground + cobble | **PRIMARY — most complete single family** |
| Tiny Town | 16×16 | 132 individual PNGs | yes | yes (great trees/bushes/mushrooms) | no | dirt clearing only | no (walls only) | Cutest, but terrain-incomplete alone |
| Tiny Battle | 16×16 | 198 individual PNGs | yes | few | yes lakes + waterfall | roads | no | Completes Tiny Town's water/roads |
| Roguelike Base Pack | 16×16 | 1 spritesheet | yes | yes | yes (teal) | yes | yes | Complete but needs slicing |
| RTS Medieval (vector) | vector | PNG/SVG | yes | yes | yes | yes | yes | Smooth vector — wrong pixel aesthetic |
| Map Pack | 64×64 | vector PNGs | yes | ~ | yes | ~ | yes | Smooth blobby vector, too large/different |
| Pixel Platformer Farm Exp. | **18×18** | 112 PNGs | — | — | — | tilled | — | Side-view, size mismatch — crops only (see below) |
| Foliage Pack | vector | PNG/SVG | — | side-view | — | — | — | Side-view foliage, not top-down |

> **Tile-index convention:** these packs ship `Tiles/tile_NNNN.png` in row-major order.
> `index = row * (columns) + col`, zero-indexed. Reference a tile by that filename.
> Crop-from-sheet fallback = 16×16 at `x = col*17`, `y = row*17` (1 px spacing).

---

## PRIMARY — RTS Medieval (Pixel)

- **Tile size:** 16 × 16 px, 1 px margin. **Grid 23 cols × 9 rows** → `index = row*23 + col`.
- **Individual tiles:** `public/Kenney Game Assets All-in-1 3.5.0/2D assets/RTS Medieval (Pixel)/Tiles/tile_NNNN.png`
- **Combined sheet (fallback):** `.../RTS Medieval (Pixel)/Tilemap/tilemap.png` (390×152, spaced) or `tilemap_packed.png` (368×144).
- **Verdict:** the most game-ready single family in the library for a forest village. Muted teal-green grass,
  proper clustered forest tiles, boulders, grass-bordered ponds, and two dirt-path autotile sets (one paved
  on grass, one on bare ground) — everything the terrain + hauling-path layers need, in one palette.

### Terrain (rows 0–2)

| Depicts | (col,row) → index | Tile file | Notes |
|---|---|---|---|
| Grass — plain (primary ground) | (0,0)=0, (1,0)=1 | `tile_0000/0001` | Base forest/meadow fill. |
| Sand / tan ground | (2,0)=2, (3,0)=3 | `tile_0002/0003` | `0003` has a rock deco. |
| Dirt / bare earth ground | (0,1)=23, (1,1)=24 | `tile_0023/0024` | Plowed/worn soil fill. |
| Stone / cobble ground | (2,1)=25, (3,1)=26 | `tile_0025/0026` | Rocky/quarry floor. |
| Water — shallow (light blue) | (0,2)=46, (1,2)=47, (2,2)=48 | `tile_0046..0048` | Open water; `(3,2)=49` shore/foam edge. |
| **Dirt path on grass** (autotile) | cols 4–9 × rows 0–2 | idx `4–9, 27–32, 50–55` | Full 6×3 winding-path set (straights, corners, tee, loop) over grass — ideal for cat hauling trails. |
| **Dirt path on bare ground** (autotile) | cols 10–15 × rows 0–2 | idx `10–15, 33–38, 56–61` | Same path set over dark/bare ground. |
| **Pond** — grass-bordered water (autotile) | cols 16–18 × rows 0–2 | idx `16–18, 39–41, 62–64` | Rounded lake/pond edge (deep teal). |
| Sand pit / stone-bordered plaza | cols 19–20 × rows 0–2 | idx `19–20, 42–43, 65–66` | Built pit / dug clearing. |

### Nature props (rows 3–5)

Single-tile 16×16 props (no 2-tile-tall trees here — everything sits in one cell).

| Prop | (col,row) → index | Tile file | Notes |
|---|---|---|---|
| Forest — trees baked on grass (full tiles) | cols 0–3 × row 3 | idx `69–72` | Green pines + round trees on grass; tile as woodland. |
| Dense forest on grass | cols 0–3 × row 4 | idx `92–95` | Thicker canopy variant. |
| Bushes / shrubs on grass | cols 0–3 × row 5 | idx `115–118` | Undergrowth. |
| Tree — round/leafy (standalone) | (4,3)=73, (5,3)=74 | `tile_0073/0074` | Place on any ground. |
| Tree — pine (standalone) | (6,3)=75, (7,3)=76 | `tile_0075/0076` | Conifer. |
| Stump / cut log | (8,3)=77, (9,3)=78 | `tile_0077/0078` | Logging remnants. |
| Rocks / boulders (single→cluster) | cols 4–8 × row 4 | idx `96–100` | Gray stones for quarry/scatter. |
| Dirt mounds / dug earth piles | cols 4–8 × row 5 | idx `119–123` | Excavation / molehills. |
| Sprout / small plant | (9,4)=101, (9,5)=124 | `tile_0101/0124` | Newly-planted marker. |
| Gravestone | (10,3)=79 | `tile_0079` | Death/burial marker. |

> Rows 6–8 are buildings/wells/tents/market-stalls/fences/signs (village hardscape) and UI arrows — same
> family, available when the buildings-art slice needs matching props.

---

## ALTERNATIVE — Tiny Town + Tiny Battle (the "Tiny" family)

Same bold chunky family, both 16×16 individual PNGs. Cutest, most readable look; thinner terrain palette.
Use **Tiny Town** for grass/dirt/trees/props/buildings and **Tiny Battle** for water/roads.

### Tiny Town — grid 12×11, `index = row*12 + col`
`public/Kenney Game Assets All-in-1 3.5.0/2D assets/Tiny Town/Tiles/tile_NNNN.png`

| Depicts | (col,row) → index | Notes |
|---|---|---|
| Grass — plain | (0,0)=0, (1,0)=1 | `(2,0)=2` grass with flowers. |
| Dirt / sand clearing on grass (autotile) | cols 0–3 × rows 1–3 → idx `12–15, 24–27, 36–39` | Dug clearing blob. |
| Sand / dirt plain + edges | cols 4–9 × row 3 → idx `40–45` | Bare ground fills. |
| Grass with pebbles | (10,3)=46 | Detail variant. |
| Tree — orange (tall look) | (3,0)=3 | Autumn tree. |
| Tree — green pine | (4,0)=4 | Conifer. |
| Bush / round shrub | (5,0)=5 | Undergrowth. |
| Mushrooms (red) | (5,2)=29 | Forage prop. |
| More trees/bush clusters | cols 6–11 × rows 0–2 | Assorted forest fill. |
| (buildings/roofs/wells/fences) | rows 4–10 | Village hardscape — **the blue/red tiles are ROOFS, not water.** |

### Tiny Battle — grid 18×11, `index = row*18 + col`
`public/Kenney Game Assets All-in-1 3.5.0/2D assets/Tiny Battle/Tiles/tile_NNNN.png`
(Ignore the tanks/flags/units — this pack is raided only for terrain.)

| Depicts | (col,row) → index | Notes |
|---|---|---|
| Grass — plain (matches Tiny Town) | (0,0)=0 … (3,0)=3 | Same vivid green. |
| **Water — lake, grass-bordered (autotile)** | cols 0–4 × rows 0–6 | Full lake edge set; `(4,0)=4` spring pond. |
| Waterfall | cols 1–2 × row 4 → idx `73–74` | Elevation water accent. |
| Road (gray, autotile) | cols 0–4 × rows 6+ | Paved path/road. |
| Tree / bush props | col 5 × rows 5–6 → idx `95, 113–114` | A few standalone plants. |

> Gap vs. RTS Medieval (Pixel): the Tiny family has **no stone-ground tile, no sand, and no
> path-on-grass autotile** — supplement or accept a simpler terrain palette.

---

## RUNNER-UP — Roguelike Base Pack (single spritesheet)

Complete top-down terrain but ships as one sheet that must be sliced; water reads teal.
- **Tile size:** 16 × 16, 1 px spacing. **57 cols × 31 rows**, pitch 17. `crop 16×16 at x=col*17, y=row*17`.
- **Sheet:** `public/Kenney Game Assets All-in-1 3.5.0/2D assets/Roguelike Base Pack/Spritesheet/roguelikeSheet_transparent.png`

Key tiles (`(col,row)`): grass `(5,0)`; open water `(0,0),(1,0),(0,1),(1,1)`; grass-bordered pond 3×3
`(2,0)–(4,2)`; stone-lined pool 3×3 `(2,3)–(4,5)`; dirt path `(6,0),(6,1)`; bare-earth autotile blobs
`cols 5–9, rows 6–12`; gray rock autotile `cols 5–9, rows 13–15`; stone floor `(7,0),(7,1)`; sand `(8,0)`;
flowers orange `(0,6)`, white `(0,9)`, blue `(0,12)`; tilled farmland fill `(10,13),(10,14)`.
Trees (2-tile-tall, crop 16×32) at `cols 13–18, rows 9–10`; round bushes/cactus/berry bush `row 8`.

---

## Supplements

- **Farmland / crops** — `Pixel Platformer Farm Expansion` has tilled-soil tiles and growing crops
  (carrots, corn, tomatoes, pumpkins, sprouts). **But it is 18×18 and side-view**, so it does not tile
  with any 16×16 primary. Use it only as a source of individual **crop icons** (rescaled), not terrain.
  Prefer the primary's own farmland: RTS Medieval Pixel dirt ground `tile_0023`, or Roguelike Base `(10,13)`.
  Path: `.../Pixel Platformer Farm Expansion/Tiles/`.
- **Foliage Pack / Foliage Sprites** — side-view / white-silhouette vector foliage; not top-down colored
  tiles. Skip for terrain; only useful if you need decorative overlays and will recolor them.

---

## License / attribution

**CC0 1.0 (public domain)** for every pack here — confirmed in each `License.txt` ("free to use in
personal, educational and commercial projects; written permission not required"). No attribution required;
crediting Kenney (kenney.nl) is appreciated, not mandatory.

The full pack lives in the **gitignored** `public/Kenney Game Assets All-in-1 3.5.0/`. Per repo convention,
**copy the chosen tiles into tracked `public/images/game/terrain/`** (and `public/images/game/nature/` for
props) rather than depend on the ignored source tree.

### Copy / slice recipes

```bash
BASE="public/Kenney Game Assets All-in-1 3.5.0/2D assets"

# PRIMARY (individual PNGs — just copy, no slicing):
cp "$BASE/RTS Medieval (Pixel)/Tiles/tile_0000.png" public/images/game/terrain/grass.png
cp "$BASE/RTS Medieval (Pixel)/Tiles/tile_0046.png" public/images/game/terrain/water-shallow.png
cp "$BASE/RTS Medieval (Pixel)/Tiles/tile_0073.png" public/images/game/nature/tree-leafy.png
# index -> filename: printf 'tile_%04d.png' $((row*23 + col))

# RUNNER-UP (Roguelike Base Pack is a sheet — slice by col,row):
SHEET="$BASE/Roguelike Base Pack/Spritesheet/roguelikeSheet_transparent.png"
magick "$SHEET" -crop 16x16+$((5*17))+$((0*17)) +repage public/images/game/terrain/grass.png
```

> Before a bulk copy, spot-check 2–3 `tile_NNNN.png` files against the index math above — Kenney's
> `Tiles/` export is row-major, but verify the corner cases (first/last row) once.
