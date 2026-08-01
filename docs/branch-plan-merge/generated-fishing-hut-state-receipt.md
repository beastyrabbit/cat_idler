# Generated Fishing-Hut Orientation and Activity Receipt

Status: production-candidate eight-state Fishing Hut world sprites.

This receipt covers the four dock-facing orientations in both idle and working
states. The images describe public station orientation/activity only. The
separate water dock, boat, worker, Rod identity/durability, habitat stock,
regeneration, route, cargo, exact progress, and report confidence are not
encoded in these sprites.

## Generation provenance

- Tool mode: built-in OpenAI image generation, reference-image mode.
- Generation date: 2026-07-25.
- Accepted generated atlas:
  `/home/beasty/.config/orca/codex-runtime-home/home/generated_images/019f8a4e-ff13-7991-b8a8-b2d97232c763/exec-afb12e4e-f081-4307-a631-249fee3cbb0f.png`.
- Accepted atlas SHA-256:
  `b7cc5e902b5e0be8c96578227c1d67e705d06dfa1c5e2a8a4d90a6c20e205ba7`.
- Accepted atlas format: 1536×1024, 8-bit opaque sRGB PNG.
- Reference contact sheet:
  `tmp/imagegen/stations/reference-contact.png`.
- Reference contact SHA-256:
  `d7c6662c4689a920899e1385f65df4fd1998404d04a462d2be9e09aaba1b4810`.
- Reference contact format: 1120×160, 8-bit sRGB PNG.
- Cropped source cells: `tmp/imagegen/fishing_hut/cells/cell-00.png`
  through `cell-07.png`.
- Inspected final contact sheet:
  `tmp/imagegen/fishing_hut/final-contact.png`.
- Final contact SHA-256:
  `3c276f97d5b9a44a3e09b611a50a1e6250fedfcc78f00d07ab1bdf3e4691413c`.
- Project output directory: `assets/planned/fishing_hut/`.

The same seven-source project contact sheet documented in
[the Cookhouse receipt](generated-cookhouse-state-receipt.md) was supplied as
the sole reference. In particular, it includes the exact existing land-dock,
water-dock, and boat assets used only for style and scale:

| Relevant reference source | Native format | SHA-256 |
|---|---|---|
| `public/images/game/buildings/workshop.png` | 48×48 sRGB PNG with alpha | `ac5892815856e51fdbd91c2b489adf68cb1a0056781c6300f8420a2df82a1209` |
| `public/images/game/buildings/mill.png` | 48×48 sRGB PNG with alpha | `5f40f07d51b8b311bbf74cf9e353497071f3a794420bb3bdca4c20469747d850` |
| `public/images/game/buildings/smithy.png` | 48×48 sRGB PNG with alpha | `d56ecc4c19528f21387fd144ddd8fdd6bcabced632f854a4495712279960d3bb` |
| `public/images/game/interior/stove.png` | 16×16 grayscale PNG with alpha | `dba8684c9e330199a977b69fb3e855d2fbb203e40d47a34663c9b3e444784e9c` |
| `public/images/game/transport/dock_land.png` | 16×16 sRGB PNG with alpha | `639fdaf3926f3fb6abe5f51bdf62281434d67b9374e7b8d2f324107fa645329a` |
| `public/images/game/transport/dock_water.png` | 16×16 sRGB PNG with alpha | `e81a23ce01650547b4d7624a0ec271b56b83e1c695315d188948eba1ed222c4a` |
| `public/images/game/transport/boat.png` | 16×16 sRGB PNG with alpha | `27a9ccbe14660af6bde87d7a5ed6a7f7941426e300f5fd8507f0ef0dfb867d59` |

## Recovered generation prompt

The complete prompt below is recovered verbatim from the image-generation call
in the local Codex session transcript; it is not inferred from the finished
files:

> Using the supplied Idle Cat Forest pixel-art buildings, docks, and boat only
> as style/scale reference, create one production Fishing Hut
> orientation-and-activity atlas.
>
> EXACT LAYOUT: a perfectly regular 4-column by 2-row grid of eight equal square
> cells, thick saturated magenta (#ff00ff) gutters and flat magenta background
> inside every cell. One isolated complete 3x3-land-footprint Fishing Hut sprite
> per cell, centered with generous padding, never crossing cells. No text,
> labels, numbers, compass letters, UI, frames, cats, loose scenery, gradients,
> or glow.
>
> EXACT ORDER left-to-right:
> Row 1, IDLE states: dock-facing north; dock-facing east; dock-facing south;
> dock-facing west.
> Row 2, WORKING states in the same orientation order: north; east; south; west.
>
> The orientation must be unambiguous from the hut doorway, attached short
> land-side dock connector, roofline, and equipment placement. Do not include
> the separate water dock or boat; those are layered by the game. Idle has a
> tied rod/net and no activity. Working has a deployed rod/net, small fish
> basket, and restrained water-side activity cues only on the dock-facing edge.
> All eight use the same timber-and-thatch Fishing Hut silhouette/camera, rotated
> or redrawn consistently.
>
> STYLE: crisp hand-placed cozy-dark fantasy pixel art, front/top three-quarter
> readability, matching a 3x3 world footprint and the reference palette.
> Transparent-ready chroma background. Output one clean atlas only.

## Processing

The accepted atlas was split with ImageMagick `-crop 4x2@ +repage`. Each crop
was 384×512 before transparency and trim. Cell order was bound exactly as
follows:

| Atlas cell | State and orientation | Canonical output |
|---:|---|---|
| 0, row 1 column 1 | idle, north | `art_station_fishing_hut_idle_north.png` |
| 1, row 1 column 2 | idle, east | `art_station_fishing_hut_idle_east.png` |
| 2, row 1 column 3 | idle, south | `art_station_fishing_hut_idle_south.png` |
| 3, row 1 column 4 | idle, west | `art_station_fishing_hut_idle_west.png` |
| 4, row 2 column 1 | working, north | `art_station_fishing_hut_working_north.png` |
| 5, row 2 column 2 | working, east | `art_station_fishing_hut_working_east.png` |
| 6, row 2 column 3 | working, south | `art_station_fishing_hut_working_south.png` |
| 7, row 2 column 4 | working, west | `art_station_fishing_hut_working_west.png` |

The final pass:

1. enabled alpha;
2. set alpha to zero where
   `(red > green × 1.5) AND (blue > green × 1.5) AND red > 0.2 AND blue > 0.2`;
3. trimmed the transparent bounds and reset the page;
4. used point filtering to reduce each sprite only as needed to fit within
   46×46;
5. centered it on a transparent 48×48 canvas.

Final files are 48×48, 8-bit sRGBA `TrueColorAlpha` PNGs. Every file has both
fully transparent and fully opaque pixels. The occupied alpha bounds remain
within the canvas, ranging from 29×46 to 42×46, with at least one transparent
pixel of outer padding.

## Final file inventory

| Canonical file | Occupied alpha bounds | SHA-256 |
|---|---|---|
| `assets/planned/fishing_hut/art_station_fishing_hut_idle_north.png` | 33×46+7+1 | `2db060cc4a094bed16f8b81441b0a5b3ffaf3929a81fcce0c3e95dbf503f7ffb` |
| `assets/planned/fishing_hut/art_station_fishing_hut_idle_east.png` | 40×46+4+1 | `31ddf9863d84f09b6ec731c6da8318fd06cf9880413814eb066c4270d988d2be` |
| `assets/planned/fishing_hut/art_station_fishing_hut_idle_south.png` | 33×46+7+1 | `de84f325848c4e6242cd65ee6af16207dd2597dc1d9fd44bc103a69998ebddb2` |
| `assets/planned/fishing_hut/art_station_fishing_hut_idle_west.png` | 39×46+4+1 | `b15e4397378e29ee71d0c65c781353c4d698ceded8315067bd68036163639d39` |
| `assets/planned/fishing_hut/art_station_fishing_hut_working_north.png` | 38×46+5+1 | `bb390d504001f341df93d1b83c5f8021858f065b5f53192461e46e1f7f6f728a` |
| `assets/planned/fishing_hut/art_station_fishing_hut_working_east.png` | 42×46+2+1 | `fe6a8186a1dc97d9cc114029678901ef18e126c132e8bec99ec7b94bb5ed348b` |
| `assets/planned/fishing_hut/art_station_fishing_hut_working_south.png` | 29×46+9+1 | `f04ae731ce0911b1d4e8df3f9b8aad28fa7ccd4f043f7e927f33d817b986706a` |
| `assets/planned/fishing_hut/art_station_fishing_hut_working_west.png` | 42×46+3+1 | `5ea66fcb63c8cb802fe2d11eff6b990155f366de25c1e022cde2cd3568ec2212` |

## Inspection and scope

The final contact sheet was visually inspected at nearest-neighbour enlargement.
The idle row retains tied equipment and no water-side activity; the working row
adds deployed gear, fish baskets, and restrained edge activity. The four
orientations preserve the requested north/east/south/west cell order. The
separate water dock and boat do not appear in the generated station canvases,
so their existing exact assets remain independently layerable by the renderer.

This receipt proves source, prompt, processing, output format, ordering, and file
identity. It does not by itself prove orientation-to-authoritative-field
selection, separate dock/boat compositing, shoreline placement, Bevy
despawn/restart behavior, gameplay-zoom screenshots, native/WASM rendering, or
browser accessibility; those remain integration acceptance work.
