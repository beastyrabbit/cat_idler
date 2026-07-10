# Cat & Character Sprite Catalog

Curated recommendation for cat colonists, role variants, the "carrying" look, and
forest enemies/raiders for the **top-down, Dwarf-Fortress-Steam-style** colony sim.
Paths are repo-relative. Verdicts are honest about what is real art, what is a
placeholder, and — the key question — **what actually reads top-down next to the
16px Tiny Town tileset.**

---

## TL;DR

- **The art-direction fork matters.** There are two incompatible cat options:
  1. **Paws & Whiskers** (`cat-sheet.png`) — beautiful animated 8-dir cats, **but
     ~32px soft/outline-less near-iso art that does NOT match 16px Tiny Town.**
  2. **A 16px Tiny-scale cat** to sit natively next to Tiny Town / Tiny Dungeon /
     Tiny Battle — **does not exist in the repo yet; needs new/edited art.**
- **If the map is Tiny Town (16px chunky outlined top-down): P&W clashes.** It's
  double the resolution, soft where Tiny is hard-outlined, and slightly isometric
  where Tiny is flat top-down. It will read as a sprite from a different game.
  **Recommendation: make/commission a 16px top-down cat in the Tiny style** (or edit
  a Tiny Dungeon humanoid). Keep P&W only if you also keep the iso map.
- **P&W license is also a hard commercial blocker** (non-commercial, no redistribution
  of modifications, no AI-basis) — a second reason not to build the shipped look on it.
- **Roles:** 32×32 pixel **hats** are real art (for P&W). For Tiny-scale, tint/hat in
  the 16px style instead. Coat-color `black/`, `calico/`… folders are **placeholders
  (colored circles) — unusable.**
- **Enemies/raiders (top-down, cohesive with Tiny Town):** use **Tiny Battle**
  faction units (recolor per raider warband) and **Tiny Dungeon** monsters
  (orc/ghost/spider/slime). All **CC0**. The existing `public/images/enemies/*.png`
  are **placeholder circles — unusable.** True fox/badger/hawk/bear have **no
  cohesive pixel source** in the repo (all packs lack them).

---

## The cohesion problem, stated plainly

The new direction is a top-down map built on Kenney's **Tiny Town** family:

| | Tiny Town / Dungeon / Battle | Paws & Whiskers cats |
|---|---|---|
| Grid | **16×16 px** | 32×64 cells (~2× taller) |
| Outline | hard dark-brown 1px outline | none / soft |
| Rendering | flat, saturated, chunky | anti-aliased, pastel |
| Projection | flat top-down | slight 3/4 isometric |

Put next to each other, the cats look like a higher-res soft sprite pasted onto a
chunky low-res town. **Honest verdict: P&W is not usable *next to Tiny Town*.** It is
still fine on the current *isometric* map (where it already lives) — the mismatch is
specifically against the 16px Tiny tiles.

---

## Colonist cat — two paths

### Path A (Tiny Town map): 16px Tiny-style cat — RECOMMENDED, needs art

- **No cat exists in Tiny Town / Tiny Dungeon / Tiny Battle** (verified — Tiny Town
  ships one bearded villager; the others ship humanoids + monsters, no felines).
- **Do:** author a small 16px top-down cat in the Tiny palette/outline, or edit a
  Tiny Dungeon humanoid body into a cat. This is the only way to get a colonist that
  sits natively next to the tiles.
- **Reference for style/scale:** `…/Tiny Dungeon/Tilemap/tilemap_packed.png`
  (192×176) and `…/Tiny Town/Tilemap/tilemap_packed.png` (192×176) — the humanoid
  characters there define the exact body scale, outline weight and palette to match.
- **Verdict:** correct long-term choice for the DF-Steam top-down look, but it is a
  **new-art task**, not a copy job.

### Path B (keep iso map): Paws & Whiskers `cat-sheet.png` — real art, wired up

- **Path:** `public/images/cats/cat-sheet.png` — `1024×64`.
- **Layout:** **8 direction groups × 4 walk frames**, cell **32w × 64h**; each facing
  group is 128px wide. Direction order **S, SW, W, NW, N, NE, E, SE.**
- **Look:** tiny pinkish cats, clean silhouette, believable 4-frame walk; reads
  near-top-down on its own — the problem is only *cohesion with Tiny Town*, not the
  sprite itself.
- **Already wired:** `components/map/CatLayer.tsx` (facing via `directionGroup`,
  `-group*128px`, CSS `steps(4)` in `app/globals.css` ~L414–447) and
  `lib/render/pixi/textures.ts` (`CAT_SHEET_URL`; its "32×32 cells" comment is off —
  cells are 32×64).
- **Verdict:** excellent sprite, **only** for an isometric/higher-res map. Do not mix
  with Tiny Town. License is non-commercial (see [Licenses](#licenses)).

### Static single-frame cats (UI only)

`public/images/cats/cat.png`, `cat-hunter.png`, `cat-architect.png`,
`cat-ritualist.png` — all `32×32`, static, front-ish; the role ones have the hat
pre-composited. Fine for roster/tooltip avatars, not for the moving map cat.

### Placeholder — DO NOT USE

`public/images/cats/{black,calico,gray-tabby,orange-tabby,tuxedo,white}/{idle,
walking,alert,eating,sleeping}.png` — **flat colored circles with a text label**
(e.g. `orange-tabby/walking.png` is an orange disc). Referenced by the retired
`components/colony/CatSprite.tsx`. Coat variety must come from **runtime tinting**,
not these files.

---

## Roles / specializations

Specializations: `hunter`, `architect`, `ritualist`, `warrior` (per
`MapCat.specialization`). No `farmer` art exists.

- **P&W path — hats (real 32×32 pixel art):** `hat-hunter.png` (green cap),
  `hat-architect.png` (builder hat), `hat-ritualist.png` (ritual headwear),
  `hat-warrior.png` (grey helmet). Already overlaid by `CatLayer.tsx`. Recommended
  role signal for the iso path.
- **Tiny path — hats/tints in 16px:** the Roguelike Characters Pack and Tiny Dungeon
  are built around hat/helmet overlays; author role hats at 16px to match. A colored
  outline/tint (as Tiny Battle does for factions) also reads well at this scale.
- **Tints (either path):** hue-shift the base cat per coat/role at runtime. The `*/`
  color folders can't supply this — they're placeholders.

---

## Carrying look

- **Current:** emoji glyph only — `CatLayer.tsx` draws `🎒` (food) / `✨` (blessings)
  offset to the cat's left. No carry sprite anywhere in the repo.
- **Verdict:** keep the glyph, or draw a small carried-item icon from
  `public/images/resources/` (or a Tiny Town item like the sack/chest at 16px) above
  the cat. A bespoke carrying frame would be new art.

---

## Enemies & raiders

### Recommended (cohesive with Tiny Town, all CC0)

- **Tiny Battle** — `…/2D assets/Tiny Battle/` (16px top-down). Faction-colored units
  in **grey/green/blue/red/orange**: infantry, tanks, planes, flags, HQ buildings.
  **Best raider source** — recolor a faction per raiding warband; the little soldier
  sprites are top-down and match Tiny Town exactly. Sheet:
  `Tiny Battle/Tilemap/tilemap_packed.png` (288×176); individual tiles in `Tiles/`.
- **Tiny Dungeon** — `…/2D assets/Tiny Dungeon/` (16px top-down). Monsters usable as
  wild threats: **orc, ghost, spider/vermin, slime**, plus skeleton/knight foes.
  Sheet `Tiny Dungeon/Tilemap/tilemap_packed.png` (192×176). Same palette/outline as
  Tiny Town — drops in cleanly.
- **Roguelike Characters Pack** — `…/2D assets/Roguelike Characters Pack/Spritesheet/
  roguelikeChar_transparent.png` (918×203). Modular **body + hat + hair + weapon +
  shield** constructor (front-facing, ~16px). Good for building rival-faction
  raiders/humanoid enemies; note it's front-view, not strict top-down, so it reads
  best as unit portraits or lightly.

### The good existing one — rival cats

- `public/images/cats/raider-sheet.png` (`1024×64`, same 8×4 / 32×64 layout as the cat
  sheet) — a dark recolor of the P&W cats, used by `components/map/RaiderLayer.tsx`.
  **Only real "cat raider" art**, but shares P&W's style-clash-with-Tiny and license
  caveat. Fine on the iso path; replace with a Tiny-scale rival on the Tiny path.

### Unusable / weak

- `public/images/enemies/{fox,badger,hawk,bear,rival_cat}.png` — **flat colored discs
  with text labels. Placeholders, do not ship.**
- **Animal Pack** (`elephant, giraffe, hippo, monkey, panda, parrot, penguin, pig,
  rabbit, snake`) and **Animal Pack Remastered** (`bear, buffalo, moose, gorilla,
  owl, wolf/dog, rhino, …` — **no fox, no badger**), both CC0: smooth flat-vector
  **round "face" style**, single static frame. **Clashes with both the pixel cats and
  Tiny Town.** Only reach for `bear`/`moose`/`owl` here if you accept a hard style
  break for a set-piece enemy.
- **Monster Builder Pack** (CC0): modular flat-vector monster parts — build custom
  beasts, but vector style, assembly-heavy, clashes.
- **Fox specifically:** only a **3D voxel** fox exists (`…/Voxel Pack/PNG/Characters/
  Fox/`) — wrong style entirely.

### Forest-critter gap (fox / badger / hawk / bear)

No pack in the repo has cohesive top-down **pixel** fox/badger/hawk/bear. Cohesive
options are all compromises: (a) recolor/edit Tiny Dungeon vermin + Tiny Battle units
into "beast raiders", (b) author new 16px critters, or (c) accept the vector style
break with Animal Pack Remastered `bear`/`moose`/`owl`. Treat true forest mammals as
a **new-art / sourcing task.**

---

## Licenses

- **All Kenney packs** (Tiny Town, Tiny Dungeon, Tiny Battle, Roguelike Characters,
  Animal Pack (Remastered), Monster Builder, Micro Roguelike, Fish, Voxel): **CC0 1.0**
  — free for commercial use, attribution appreciated, not required. **Safe.**
- **Paws & Whiskers Free** (source of `cat-sheet.png` + `raider-sheet.png`;
  `public/Paws & Whiskers - Isometric Cats Pack (Free)/readme_free.txt`):
  - ✅ Non-commercial only; may modify.
  - ❌ No resale/redistribution **even if modified** (incl. NFTs).
  - ❌ No use as a **basis for AI-generated content**.
  - Page: `netherzapdos.itch.io/paws-whiskers-isometric-cats-pack`.
  - **Hard blocker for any commercial release** and for generating derived art — a
    second reason to move the colonist to CC0 / original 16px art for the Tiny path.

---

## Gitignored-pack copy note

- Both source packs are gitignored:
  `/public/Kenney Game Assets All-in-1 3.5.0/` and
  `/public/Paws & Whiskers - Isometric Cats Pack (Free)/`.
- **Copy what you need into a tracked folder.** Convention for freshly-pulled game
  art: **`public/images/game/cats/`** (doesn't exist yet — create on first copy).
- The wired-up sheets (`cat-sheet.png`, `raider-sheet.png`, hats) already live tracked
  under `public/images/cats/`; keep those paths to avoid touching `CatLayer.tsx` /
  `RaiderLayer.tsx` / `textures.ts`. When pulling Tiny Town / Tiny Battle / Tiny
  Dungeon tiles for the top-down build, copy them out into `public/images/game/…`
  since the whole Kenney pack directory is gitignored.
