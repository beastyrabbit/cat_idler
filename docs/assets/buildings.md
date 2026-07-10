# Building / Prop / Farm / Enemy Art Manifest

Top-down pixel art for **Idle Cat Forest**, per the locked art direction in `docs/assets/SELECTION.md`
(family is final — do not re-evaluate). All source packs are **CC0** and gitignored under
`public/Kenney Game Assets All-in-1 3.5.0/`; only the sliced/composited PNGs under
`public/images/game/` are used at runtime.

**Families (locked):** exterior buildings = **Tiny Town** (16px, composited); stockpile props =
**Roguelike Base Pack** sheet; farm = **Pixel Platformer Farm Expansion**; enemies = **Roguelike
Characters Pack**. RTS Medieval is borrowed only for the 4 building types Tiny Town cannot render
distinctly (windmill / chapel / forge / loom), downscaled to 48px — sanctioned by SELECTION.md.

Slicing: Base sheet tile `(col,row)` = `crop 16x16 at x=col*17, y=row*17`. Tiny Town / Farm tiles are
individual PNGs (`tile_NNNN.png`, 16px / 18px).

---

## Buildings — `public/images/game/buildings/` (14 files)

Composited from **Tiny Town** `Tiles/tile_NNNN.png` unless marked RTS. House template = 3-wide × 3-tall
(48×48): `[roof]/[wall,window|badge,wall]/[wall,door,wall]`. Roof shingles: red `52,53,54` / grey
`48,49,50`; walls grey `76,77`; wood door `86`; window `84`. Verified by montage.

| File | Source | Verdict |
|---|---|---|
| `den.png` | TT red roof + grey wall + window `84` + door `86` | Clean red-roof cottage — the default home. 48×48. |
| `storehouse.png` | TT grey roof + chest `130` + barrel `106` | Grey house showing stored goods. |
| `workshop.png` | TT grey roof + wood wall `72/75` + hammer `128` + wood door `74` | Wood craft shed with a tool sign. |
| `research_hut.png` | TT grey roof + key `117` | Grey house with a key emblem (archive). |
| `school.png` | TT red roof + signboard `83` | Red-roof civic block with a notice board. |
| `barracks.png` | TT grey roof + axe `127` | Grey house with a weapon emblem. |
| `town_hall.png` | TT castle `96,97,98 / 108,109,110 / 108,103,110` | Stone battlements + portcullis gate — the grand civic seat. |
| `market.png` | TT covered cart `57` + barrel `106` | Trade cart with wares. 48×32. |
| `tent.png` | TT awning booth `104` + sign `83` | Stall/tent (also the accounting tent). 48×32. |
| `monument.png` | TT grey peak `63` + wall `109`×2 | Grey stone obelisk. 16×48. |
| `mill.png` | **RTS** `Structure_14` → 48px | Windmill (Tiny Town has no windmill). |
| `shrine.png` | **RTS** `Structure_04` → 48px | Cross chapel — reads sacred. |
| `smithy.png` | **RTS** `Structure_20` → 48px | Chimney forge — unmistakable smithy. |
| `clothier.png` | **RTS** `Structure_22` → 48px | Striped cloth stall / loom. |

**Notes:** the 4 RTS sprites are flat-vector, a mild style seam against the 10 pixel Tiny Town ones, but
each reads unambiguously as its type (better than a Tiny Town composite that reads as a generic house).
`mill` downscales to mostly the sail-cross — reads as a mill but is spare; can composite a body under it
if desired.

---

## Stockpile props — `public/images/game/props/` (11 files)

All from the **Roguelike Base** sheet `(col,row)`.

| File | (col,row) | Verdict |
|---|---|---|
| `barrel.png` | `(53,15)` | Wooden barrel/keg. |
| `crate.png` | `(49,17)` | Wooden crate. |
| `sack.png` | `(41,10)` | Cloth sack. |
| `log_pile.png` | `(13,8)` | Crossed logs — woodpile. |
| `stone_pile.png` | `(44,11)` | Grey stone chunk. |
| `ore_pile.png` | `(43,11)` | Ore pile. |
| `gold_pile.png` | `(42,11)` | Gold pile. |
| `gravestone.png` | `(51,9)` | Cross headstone. |
| `campfire.png` | `(14,8)` | Lit campfire. |
| `well.png` | `(50,10)+(50,11)` | Grey stone well/statue centrepiece. 16×32. |
| `haystack.png` | `(43,10)` | Grain sack (**approx** — no hay sprite in pack). |

---

## Farm — `public/images/game/farm/` (6 files)

From **Pixel Platformer Farm Expansion** `Tiles/`, downscaled 18→16.

| File | tile | Verdict |
|---|---|---|
| `soil.png` | `0030` | Clean tilled dirt — plot base. |
| `crop_sprout.png` | `0072` | Stage 1 seedling. |
| `crop_growing.png` | `0073` | Stage 2 growing. |
| `crop_mature.png` | `0075` | Stage 3 mature. |
| `crop_flowering.png` | `0074` | Stage 4 flowering/fruiting. |
| `scarecrow.png` | composite `109` (post) + `95` (arms) + `04` (pumpkin head) | 16×32 pumpkin-head harvest totem. |

**scarecrow caveat:** the pack has **no scarecrow sprite** (verified via its Sample.png; tiles 94/95 are
branches/logs, 92/93/108-110 are posts). `scarecrow.png` is therefore a **composite stand-in** built from
cohesive Farm Expansion parts (pumpkin head on a crossbar post) — reads as a harvest totem/scarecrow;
swap for a purpose-made sprite later if a truer scarecrow is wanted. Also note the crop sprites are
side-view platformer plants on a dark soil mound — they read as planted crops top-down but aren't strictly
top-down.

---

## Enemies — `public/images/game/enemies/` (4 files)

From the **Roguelike Characters Pack** sheet `(col,row)`. **Caveat:** the Roguelike family has no literal
forest animals — these are the closest cohesive fantasy-creature stand-ins, not real fox/badger/bear.

| File | (col,row) | What it actually is |
|---|---|---|
| `bear.png` | `(1,2)` | Brown blob creature — biggest/brownest beast. |
| `badger.png` | `(0,0)` | Pale/grey blob creature. |
| `fox.png` | `(1,10)` | Orange horned brute. |
| `rival_beast.png` | `(1,3)` | Green fanged goblin. |

(`raider-sheet.png` for rival-cat raiders stays wired separately per SELECTION.md.)

---

## Also present (other agents / earlier passes)

- `public/images/game/terrain/`, `nature/` — ground + trees, owned by the ground/terrain agent.
- `public/images/game/infra/` (10 files) — soil/palisade/gate/bridge/roads from the Base sheet
  (a prior pass; roads are dirt-path autotile cells — full autotile wiring is the terrain agent's).

## Files written / current in each dir

- **buildings: 14** · **props: 11** · **farm: 6** · **enemies: 4**  (infra: 10 from an earlier pass)
