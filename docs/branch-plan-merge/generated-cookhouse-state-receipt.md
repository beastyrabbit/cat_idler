# Generated Cookhouse State-Sheet Receipt

Status: production-candidate six-state Cookhouse world sprites.

This receipt covers the Cookhouse construction and operation images required by
Plan 1. The images represent only public construction/operation state. They do
not encode recipe, ingredient stock, worker identity, quality, exact progress,
hidden blockers, or report confidence.

## Generation provenance

- Tool mode: built-in OpenAI image generation, reference-image mode.
- Generation date: 2026-07-25.
- Accepted generated atlas:
  `/home/beasty/.config/orca/codex-runtime-home/home/generated_images/019f8a4e-ff13-7991-b8a8-b2d97232c763/exec-d2165656-71a6-48e1-8e28-338574116cd9.png`.
- Accepted atlas SHA-256:
  `325883ff0e49ddd01683981d5007d94dfe53905f3d8d05d9bf88745318cdd6e6`.
- Accepted atlas format: 1536×1024, 8-bit opaque sRGB PNG.
- Reference contact sheet:
  `tmp/imagegen/stations/reference-contact.png`.
- Reference contact SHA-256:
  `d7c6662c4689a920899e1385f65df4fd1998404d04a462d2be9e09aaba1b4810`.
- Reference contact format: 1120×160, 8-bit sRGB PNG.
- Cropped source cells: `tmp/imagegen/cookhouse/cells/cell-00.png`
  through `cell-05.png`.
- Inspected final contact sheet: `tmp/imagegen/cookhouse/final-contact.png`.
- Final contact SHA-256:
  `33bfd6c3fa7989be50e01c6bb32a3964be1a6fce611f2879b9688d110c540cc0`.
- Project output directory: `assets/planned/cookhouse/`.

The reference contact sheet was assembled, in order, from these pre-existing
project assets:

| Reference source | Native format | SHA-256 |
|---|---|---|
| `public/images/game/buildings/workshop.png` | 48×48 sRGB PNG with alpha | `ac5892815856e51fdbd91c2b489adf68cb1a0056781c6300f8420a2df82a1209` |
| `public/images/game/buildings/mill.png` | 48×48 sRGB PNG with alpha | `5f40f07d51b8b311bbf74cf9e353497071f3a794420bb3bdca4c20469747d850` |
| `public/images/game/buildings/smithy.png` | 48×48 sRGB PNG with alpha | `d56ecc4c19528f21387fd144ddd8fdd6bcabced632f854a4495712279960d3bb` |
| `public/images/game/interior/stove.png` | 16×16 grayscale PNG with alpha | `dba8684c9e330199a977b69fb3e855d2fbb203e40d47a34663c9b3e444784e9c` |
| `public/images/game/transport/dock_land.png` | 16×16 sRGB PNG with alpha | `639fdaf3926f3fb6abe5f51bdf62281434d67b9374e7b8d2f324107fa645329a` |
| `public/images/game/transport/dock_water.png` | 16×16 sRGB PNG with alpha | `e81a23ce01650547b4d7624a0ec271b56b83e1c695315d188948eba1ed222c4a` |
| `public/images/game/transport/boat.png` | 16×16 sRGB PNG with alpha | `27a9ccbe14660af6bde87d7a5ed6a7f7941426e300f5fd8507f0ef0dfb867d59` |

The contact sheet itself was created with a dark `#271c1a` background and
144×144 reference cells. The transport references were included in the shared
station reference sheet but the Cookhouse prompt explicitly asked for a
building only.

## Recovered generation prompt

The complete prompt below is recovered verbatim from the image-generation call
in the local Codex session transcript; it is not inferred from the finished
files:

> Using the supplied Idle Cat Forest pixel-art buildings only as style and scale
> reference, create one production Cookhouse state atlas.
>
> EXACT LAYOUT: a perfectly regular 3-column by 2-row grid of six equal square
> cells, thick saturated magenta (#ff00ff) gutters and flat magenta background
> in every cell. One isolated complete 3x3-world-tile Cookhouse sprite per cell,
> centered with generous padding, never crossing a cell. No text, labels,
> numbers, UI, frames, cats, loose scenery, gradients, or glow.
>
> EXACT ORDER left-to-right:
> Row 1: wooden scaffold with clearly incomplete cooking hearth; completed
> structure shell with chimney but no fit-out; fit-out state with stove, prep
> table, bowls and storage visibly installed.
> Row 2: operational idle Cookhouse with cold/low hearth and no output;
> operational working Cookhouse with warm fire, gentle smoke and visible
> prepared-food activity; blocked Cookhouse with cold hearth and a restrained
> red-brown shortage marker integrated as a physical empty basket, not a UI
> icon.
>
> STYLE: crisp hand-placed cozy-dark fantasy pixel art, matching the reference
> buildings' 3x3 footprint, front/top three-quarter readability, timber and
> stone, warm parchment/wood/dark-forest palette. All six states must be the
> same building silhouette and camera, changing only construction/operation
> state. Transparent-ready chroma background. Output one clean atlas only.

## Processing

The accepted atlas was split with ImageMagick `-crop 3x2@ +repage`. Cell order
was bound exactly as follows:

| Atlas cell | State suffix | Canonical output |
|---:|---|---|
| 0, row 1 column 1 | `scaffold` | `art_station_cookhouse_scaffold.png` |
| 1, row 1 column 2 | `structure` | `art_station_cookhouse_structure.png` |
| 2, row 1 column 3 | `fit_out` | `art_station_cookhouse_fit_out.png` |
| 3, row 2 column 1 | `idle` | `art_station_cookhouse_idle.png` |
| 4, row 2 column 2 | `working` | `art_station_cookhouse_working.png` |
| 5, row 2 column 3 | `blocked` | `art_station_cookhouse_blocked.png` |

Two initial chroma thresholds were inspected and rejected because they retained
magenta fringe. The final pass:

1. enabled alpha;
2. set alpha to zero where
   `(red > green × 1.5) AND (blue > green × 1.5) AND red > 0.2 AND blue > 0.2`;
3. trimmed the transparent bounds and reset the page;
4. used point filtering to reduce each sprite only as needed to fit within
   46×46;
5. centered it on a transparent 48×48 canvas.

Final files are 48×48, 8-bit sRGBA `TrueColorAlpha` PNGs. Every file has both
fully transparent and fully opaque pixels. The occupied alpha bounds remain
within the canvas, ranging from 38×46 to 44×46, with at least one transparent
pixel of outer padding.

## Final file inventory

| Canonical file | Occupied alpha bounds | SHA-256 |
|---|---|---|
| `assets/planned/cookhouse/art_station_cookhouse_scaffold.png` | 42×46+3+1 | `f7f45017442f327cf9b33c0e3d14889ed57ce2ffb10054aa1fbc18a8970bcebf` |
| `assets/planned/cookhouse/art_station_cookhouse_structure.png` | 42×46+3+1 | `f90883500c610606a307726bad963a2c060edaa5544c6684ebf68f11ba9886e9` |
| `assets/planned/cookhouse/art_station_cookhouse_fit_out.png` | 42×46+3+1 | `5d0d733927dddb01004723fbd4aee54808800934e69af0c0d1fff20077741e1b` |
| `assets/planned/cookhouse/art_station_cookhouse_idle.png` | 42×46+3+1 | `44a8532399777859fe0b3b4b7dc70bec6fc3b5920c5fb936a5174cc08eb5be7f` |
| `assets/planned/cookhouse/art_station_cookhouse_working.png` | 38×46+5+1 | `7e8adb8cfa6d3c7f8880248e93b92e3c67c13d2cdce5039eca7bd5c747d4520e` |
| `assets/planned/cookhouse/art_station_cookhouse_blocked.png` | 44×46+2+1 | `58410c6c04d02650926801b7f7f3b6160ebf01307af224b000c67b2d3e3fd84c` |

## Inspection and scope

The final contact sheet was visually inspected at nearest-neighbour enlargement.
It preserves one recognizable timber-and-stone Cookhouse across all six cells,
with readable scaffold, shell, fit-out, idle, working, and physically blocked
states. The blocked indicator is part of the pictured empty basket rather than
a floating UI badge.

This receipt proves source, prompt, processing, output format, ordering, and file
identity. It does not by itself prove runtime art-key selection, Bevy
despawn/restart behavior, gameplay-zoom screenshots, native/WASM rendering, or
browser accessibility; those remain integration acceptance work.
