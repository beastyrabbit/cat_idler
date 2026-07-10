# Building / Prop / Enemy Art Manifest — Kenney Roguelike family

> **Correction (team lead, post-verification):** the Roguelike **Base-sheet building composites
> documented below read as generic "roofed shelves"** on view (its structure tiles are interior
> furniture, not exteriors). So the 9 generic buildings (den, storehouse, barracks, school,
> shrine, mill, town_hall, research_hut, workshop) were **regenerated as Tiny Town composites**
> (roof+wall+door stacks; town_hall = stone battlement + portcullis). The genuinely-good Base
> **standalones are kept**: smithy (forge), clothier + market (awnings), tent, monument
> (gravestone). Props, infra, and enemies below are as-shipped from the Base/Characters sheets.
> Buildings are normalized to a 32×48 bottom-anchored canvas. See `SELECTION.md`.


Top-down pixel art for **Idle Cat Forest**. Cross-slice decision (made by the team lead): because the only
cohesive cat art (`cat-sheet.png`) is 32px near-top-down **pixel** art, the whole world is pixel art and the
ground agent's **Kenney Roguelike** family wins. RTS Medieval's 64px flat-vector (a prior revision of this
doc) would clash with the pixel cats and is dropped.

Files in this manifest are **generated and committed** into tracked `public/images/game/{buildings,infra,props,enemies}/`.

## Licensing — all CC0 (verified from each pack's `License.txt`)

- Roguelike Base Pack (1.0) — CC0
- Roguelike Modern City (2.0) — CC0
- Roguelike Dungeon Pack (1.0) — CC0
- Roguelike Characters pack (2.0) — CC0

Source packs live in the **gitignored** `public/Kenney Game Assets All-in-1 3.5.0/` — never referenced at
runtime; the committed crops under `public/images/game/` are the only runtime assets.

## Sources & method

- **Primary: Roguelike Base Pack sheet** — `.../Roguelike Base Pack/Spritesheet/roguelikeSheet_transparent.png`
  (968x526, 57col x 31row, **17px pitch, 16px tile**). Tile at `(col,row)` = `crop 16x16 at x=col*17, y=row*17`.
  This is Kenney's **medieval fantasy** RPG set (market awnings, forge, tents, graves, crosses, banners,
  bookshelves, fences, mine carts, trees, farm crops) — the correct theme for a cat forest.
- **Enemies: Roguelike Characters Pack sheet** — `.../roguelikeChar_transparent.png` (918x203, 54x12, 17px).
- **Rejected: Roguelike City Pack** — evaluated its Preview/tilemap; it is **modern city** (cars, asphalt,
  offices, traffic lights). Theme clash with a medieval cat forest, so no buildings were sourced from it.
- **Dungeon Pack** — mostly dungeon walls/floors/ore-veins/bones; no distinct village structures or animals
  needed here, so nothing was pulled from it (ore-vein tiles are a future option for mining nodes).

Buildings are **composited** from the Base sheet's top-down roof/wall/door kit (the pack has no single-tile
whole-buildings), except the ones that exist as standalone objects (tent, market/clothier awnings, forge,
gravestone). Composite template = 2-wide **32x48** `[roof, wall, door]` with a 16x16 "sign" badge on the wall.

---

## Buildings — `public/images/game/buildings/` (14 files)

Base tiles used: roof-tan `(13,21)+(14,21)`, roof-brown `(20,21)+(21,21)`, roof/top-grey `(20,12)+(21,12)`,
wall-tan `(13,15)+(14,15)`, wall-grey `(20,15)+(21,15)`, door-tan `(17,17)+(19,17)`, door-grey `(24,17)+(26,17)`.

| File | Source (Base sheet) | Verdict |
|---|---|---|
| `den.png` | tan roof + tan wall + tan door | Clean tan peaked cottage — the default home. 32x48. |
| `storehouse.png` | brown roof + tan wall + door, badge crate `(15,6)` | Brown-roof barn with a produce-crate emblem. |
| `workshop.png` | tan house, badge hammer `(41,16)` | Craft shed with a tool sign. |
| `research_hut.png` | tan house, badge book `(49,15)` | Small hut with a book emblem. |
| `school.png` | grey house, badge book `(48,15)` | Grey civic block, different book color from research. |
| `barracks.png` | grey house, badge axe `(43,16)` | Grey building with a weapon emblem (martial). |
| `town_hall.png` | grey house, badge plain banner `(52,1)` | Grey official building with a hung banner — the civic seat. |
| `shrine.png` | tan house, badge cross `(50,10)` | Cottage with a cross relic — reads sacred. |
| `mill.png` | brown roof + tan wall, badge barrel/grain `(53,15)` | **Approximation** — the Roguelike set has no windmill; brown-roof granary with a grain emblem stands in. |
| `smithy.png` | **standalone** forge furnace `(54,9)+(54,10)` | Lit stone forge — unmistakable smithy. 16x32. |
| `tent.png` | **standalone** green tent `(46,10)+(46,11)` | Camp tent (also good for the accounting tent). 16x32. |
| `market.png` | **standalone** orange awning `(10,0)+(10,1)` | Striped market stall. 16x32. |
| `clothier.png` | **standalone** green awning `(11,0)+(11,1)` | Green striped cloth stall (distinct from market by color). 16x32. |
| `monument.png` | **standalone** gravestone `(51,9)` | Grey cross headstone — memorial/monument. 16x16. |

---

## Infrastructure — `public/images/game/infra/` (10 files)

| File | Source (Base `col,row`) | Verdict |
|---|---|---|
| `soil.png` | `(6,0)` | Plain brown dirt — field/plot base. |
| `palisade.png` | `(47,23)` | Horizontal wooden fence rail. |
| `gate_closed.png` | `(46,23)` | Fence run with a closed gate. |
| `gate_open.png` | `(45,23)` | Fence end / open gate post. |
| `bridge.png` | `(34,17)` | Horizontal wood-plank deck — lay over water tiles. |
| `road_straight_h.png` | `(6,8)` | Dirt-path autotile cell (horizontal). |
| `road_straight_v.png` | `(6,10)` | Dirt-path autotile cell (vertical). |
| `road_corner.png` | `(5,7)` | Dirt-path autotile corner. |
| `road_cross.png` | `(7,10)` | Dirt-path autotile centre (4-way). |
| `road_t.png` | `(7,7)` | Dirt-path autotile T-junction. |

### Road autotile index map (important)

The Roguelike Base Pack has **no thin line-road**; "roads" are **area-fill autotiles** (a dirt blob you
paint, the tile auto-selects its edge/corner). The 5 `road_*` files above are representative cells of the
**dirt-path autotile block** at Base **cols 5–9, rows 7–12** (16 tiles: centre-fill, 4 edges, 4 outer
corners, 4 inner corners, ends). Two sibling autotile blocks exist for other surfaces:

- Dirt path: **cols 5–9 × rows 7–12** (used here)
- Cobblestone/brick path: **cols 5–9 × rows 2–5**
- Stone path: **cols 5–9 × rows 13–15**

Full autotile neighbour-mask wiring belongs to the **ground/terrain agent** (who owns `public/images/game/terrain`),
so these 5 files are a starting set, not a hand-authored straight/corner sprite family. Flag overlap with
that agent before wiring roads.

---

## Props / stockpiles — `public/images/game/props/` (14 files)

| File | Source (Base `col,row`) | Verdict |
|---|---|---|
| `log_pile.png` | `(13,8)` | Crossed logs — woodpile. |
| `stone_pile.png` | `(44,11)` | Grey stone chunk. |
| `ore_pile.png` | `(43,11)` | Gold/ore pile. |
| `barrel.png` | `(53,15)` | Wooden barrel/keg. |
| `crate.png` | `(49,17)` | Wooden crate. |
| `haystack.png` | `(43,10)` | Grain sack (**approx** for haystack; no hay sprite in pack). |
| `campfire.png` | `(14,8)` | Lit campfire. |
| `well.png` | `(50,10)+(50,11)` | Grey stone well/statue centrepiece. 16x32. |
| `gravestone.png` | `(51,9)` | Cross headstone. |
| `bush.png` | `(44,23)` | Green bush. |
| `stump.png` | `(27,11)` | Bare dead tree/snag (**approx** for stump). |
| `seedling.png` | `(22,11)` | Small green sprout — crop stage 0. |
| `crop_growing.png` | `(0,9)` | White-flower crop plot — mid growth. |
| `crop_mature.png` | `(0,6)` | Red-flower crop plot — harvest-ready. |

---

## Enemies — `public/images/game/enemies/` (5 files)

**Caveat (honest):** the Roguelike Characters/Dungeon packs contain **fantasy humanoids and blob-monsters,
no literal forest animals** (no fox/badger/bear/hawk). The 5 files below map each dest name to the closest
cohesive creature; they are **stand-ins**, not real animals. If literal forest predators are required, source
a dedicated CC0 animal pixel pack later — nothing in the Roguelike family fits.

| File | Source (Characters `col,row`) | What it actually is |
|---|---|---|
| `bear.png` | `(1,2)` | Brown blob creature — biggest/brownest beast. |
| `rival_beast.png` | `(1,3)` | Green fanged goblin — aggressive. |
| `badger.png` | `(0,0)` | Pale/grey blob creature. |
| `fox.png` | `(1,10)` | Orange horned brute — the orange-hued raider. |
| `hawk.png` | `(0,6)` | Red-bearded brigand — a humanoid raider stand-in (no bird exists in-pack). |

---

## Files written this pass: **43**

- buildings: 14 · infra: 10 · props: 14 · enemies: 5

Also present from the ground/terrain agent: `public/images/game/{terrain,nature}/`. Building composites are
32x48 (or 16x32/16x16 for standalone icons); infra/props/enemies are 16x16 unless noted 16x32. All are
NEAREST-scaled 16px pixel art cohesive with the cat sheet and the Roguelike ground family.
