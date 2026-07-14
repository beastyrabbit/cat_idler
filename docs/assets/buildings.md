# Building / Prop / Farm / Enemy Art Manifest

Top-down pixel art for **Idle Cat Forest**, per the locked art direction in `docs/assets/SELECTION.md`
(family is final — do not re-evaluate). Source packs are gitignored under
`public/Kenney Game Assets All-in-1 3.5.0/`; only the sliced/composited PNGs under
`public/images/game/` are used at runtime.

**Families:** roofed residential silhouettes and retained facade choices use **Tiny Town**;
open stations use **Roguelike Base Pack** floors/props plus individual interior pieces; farms
use **Pixel Platformer Farm Expansion**; enemies use **Roguelike Characters Pack**. The Bevy
runtime no longer uses facade art for workshops.

Slicing: Base sheet tile `(col,row)` = `crop 16x16 at x=col*17, y=row*17`. Tiny Town / Farm tiles are
individual PNGs (`tile_NNNN.png`, 16px / 18px).

---

## Retained facades — `public/images/game/buildings/` (18 files)

Composited from **Tiny Town** `Tiles/tile_NNNN.png` unless marked RTS. House template = 3-wide × 3-tall
(48×48): `[roof]/[wall,window|badge,wall]/[wall,door,wall]`. Roof shingles: red `52,53,54` / grey
`48,49,50` (vent `51`); walls wood `72,73` / grey stone `76,77`; wood door `86`; stone arch `78`;
window `84`. Verified by montage.

| File | Source | Verdict |
|---|---|---|
| `den.png` | TT red roof + grey wall + window `84` + door `86` | Clean red-roof cottage — the default home. 48×48. |
| `storehouse.png` | TT grey roof + chest `130` + barrel `106` | Grey house showing stored goods. |
| `research_hut.png` | TT grey roof + key `117` | Grey house with a key emblem (archive). |
| `school.png` | TT red roof + signboard `83` | Red-roof civic block with a notice board. |
| `barracks.png` | TT grey roof + axe `127` | Grey house with a weapon emblem. |
| `town_hall.png` | TT castle `96,97,98 / 108,109,110 / 108,103,110` | Stone battlements + portcullis gate — the grand civic seat. |
| `market.png` | TT covered cart `57` + barrel `106` | Trade cart with wares. 48×32. |
| `tent.png` | TT awning booth `104` + sign `83` | Stall/tent (also the accounting tent). 48×32. |
| `monument.png` | TT grey peak `63` + wall `109`×2 | Grey stone obelisk. 16×48. |
| `shrine.png` | RTS `Structure_04` → 48px | Archived facade; the runtime shrine is an open altar court. |

### Open stations — current runtime treatment

The Bevy client now composes non-residential buildings at runtime instead of drawing these
exterior facades. A repeated wood, stone, or soil floor fills the authoritative footprint;
individual top-down props sit on it with their own y-sorted depth. This keeps cats visible while
working, makes Field a real crop plot, and leaves the Shrine open on every side for scout returns.
Only Den, Beds, Nursery, and Elder Corner retain the roofed `den.png` silhouette.

The runtime compositions are exhaustive for all 25 current protocol variants in
`crates/cat-client/src/station_layout.rs`. In addition
to the existing `interior/`, `props/`, and `farm/` sprites, the review bench promoted these
individual pieces into `public/images/game/interior/`: colored beds, bookcase, brazier,
candelabra, display table, forge fire, map table, metal basin, gold reliquary, scroll, stool,
stove, sword block, and weapon stand. They remain separate images; no station is flattened into a
new building texture. Mill and Sawmill are live, distinct open layouts: grain/flour containers on
a stone milling floor versus a saw bed, raw logs, and finished-goods crate on a wood floor.
The Accounting Tent is also snapshot-reachable and renders as an open desk-and-ledger station.
Its assigned Accountant physically walks deterministic rounds to reachable stockpiles, and the
integrated station and active route have been accepted in the client's own framebuffer.

### Retained workshop facades — review/fallback alternatives

Each retained facade is a **TT hut base + a distinct craft indicator** so its craft reads at a glance
(DF-Steam style). Indicators are TT/Roguelike props where the library has one, else a small
hand-drawn pixel primitive built to match the family (anvil, windmill sails, cut-stone blocks,
cloth bolts, workbench, plank stack, hand-saw, log-end bundle). Roguelike Base cell `(col,row)` =
`crop 16x16 at x=col*17, y=row*17` from `Spritesheet/roguelikeSheet_transparent.png`.

| File | Source | Craft indicator / verdict |
|---|---|---|
| `wood_cutter.png` | TT wood hut + drawn log-end bundle + TT axe `127` | Log pile + axe → **lumber/felling**. Reads clearly. |
| `stone_prep.png` | TT stone hut (arch `78`) + drawn cut-stone blocks + TT pickaxe `115` | Stone blocks + pick → **stone-cutting**. Reads clearly. |
| `woodworking.png` | TT wood hut + drawn workbench + drawn plank-stack + drawn saw | Bench + planks + saw → **carpentry**. Distinct from the raw wood-cutter. |
| `smithy.png` | TT stone hut (roof vent `51`) + Roguelike forge-fire `(54,8)` **in the doorway** + drawn anvil + TT hammer `128` | Glowing forge in the door + anvil + hammer → **unmistakable smithy**. Strongest reader. |
| `mill.png` | TT stone hut + drawn windmill sails (X-lattice vanes on the face) | Bold windmill sails → **mill**. Replaces the spare RTS sail-cross. |
| `clothier.png` | TT wood hut + TT striped awning `104` storefront + drawn cloth bolts (red/teal/gold) | Striped awning + folded cloth → **weaver/textiles**. |
| `tannery.png` | Wood floor + workbench/vat props | Hide-processing fallback; runtime uses its typed open layout instead. |
| `workshop.png` | TT wood hut + TT hammer `128` wall emblem + Roguelike barrel/crate props | Generic craft/tool shed — the fallback. |

**Notes:** these eight 48×48 alternatives remain available to the sprite review page and as fallbacks, but
the map renderer no longer uses them for craft stations. The current clothier communicates textile
work with an open display table and colored fabric/bedroll pieces; its typed layout can accept a more
specific loom prop without changing rendering architecture.

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
- `public/images/game/infra/` (10 files) — soil/palisade/gate/bridge/road sprites from the Base
  sheet. The client renders both authored stone roads and live traffic-formed dirt paths.

## Files written / current in each dir

- **buildings: 18** · **interior: 21** · **props: 11** · **farm: 6** · **enemies: 4**
  (infra: 10 from an earlier pass)
