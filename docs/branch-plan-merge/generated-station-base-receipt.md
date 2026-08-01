# Station base source-reuse and generation receipt

Date: 2026-07-25

Scope: the four formerly unresolved `48×48 world_base` station keys. Cookhouse
and Fishing Hut reuse already accepted exact state art; Sawmill and Smelter
were generated as distinct buildings.

## Exact source reuse

| Canonical key | Reused source | Production path | SHA-256 |
| --- | --- | --- | --- |
| `art_station_cookhouse` | `assets/planned/cookhouse/art_station_cookhouse_idle.png` | `assets/planned/stations/art_station_cookhouse.png` | `44a8532399777859fe0b3b4b7dc70bec6fc3b5920c5fb936a5174cc08eb5be7f` |
| `art_station_fishing_hut` | `assets/planned/fishing_hut/art_station_fishing_hut_idle_north.png` | `assets/planned/stations/art_station_fishing_hut.png` | `2db060cc4a094bed16f8b81441b0a5b3ffaf3929a81fcce0c3e95dbf503f7ffb` |

Each production file is byte-identical to its named source. No duplicate
“generic” building was generated.

## Sawmill and Smelter generation

The call used built-in reference-image mode with:

- `tmp/imagegen/stations/reference-contact.png`
  (`d7c6662c4689a920899e1385f65df4fd1998404d04a462d2be9e09aaba1b4810`);
- `tmp/imagegen/cookhouse/final-contact.png`
  (`33bfd6c3fa7989be50e01c6bb32a3964be1a6fce611f2879b9688d110c540cc0`).

The exact prompt was:

> Create one production world-building sprite atlas for the non-commercial pixel-art game Idle Cat Forest, matching the supplied game's exact warm woodland pixel-art language, three-quarter/isometric view, dark readable outlines, timber-and-stone construction, compact 48x48 building footprint, and no modern machinery. EXACT layout: 2 columns by 1 row, two equal square cells, one complete centered building per cell, equal scale and ground contact, no overlaps. Left cell MUST be a Sawmill clearly distinct from the Woodworking shop: open timber shed, one large visible circular or frame saw, log feed rails, stacked raw logs, small plank output pile. Right cell MUST be a Smelter clearly distinct from the Smithy: squat stone furnace building, tall smoke chimney, iron-banded furnace mouth with restrained orange molten glow, ore cart and ingot tray; no anvil or blacksmith figure. Use a perfectly flat solid chroma-key background color #D2FF4D across every unused pixel. No transparency, scenery, ground plane, cast shadows outside buildings, text, labels, letters, numbers, borders, grid line, UI frame, cats, workers, religious symbols, clipped roofs, or extra buildings. Keep each full building safely within its cell with consistent padding and a silhouette that remains legible at 48x48.

The `1774×887` source was normalized to `1776×888`, giving two exact
`888×888` cells. Chroma was removed and each cell was nearest-neighbour
contained on an exact transparent `48×48` sRGBA canvas.

| Artifact/output | SHA-256 |
| --- | --- |
| `tmp/imagegen/stations/generated-production/source-atlas.png` | `66a18e3a00d10809e51ffe538fb85b7393d9406e164681f2a33b70541f80dce1` |
| `tmp/imagegen/stations/generated-production/normalized-atlas.png` | `e132d13eccd4771f933d5046030f4d77795629ff10a78856594c3dab87b27a5f` |
| `tmp/imagegen/stations/generated-production/final-contact.png` | `2a9453731a2cbf5412991cd7391f73fbbcc8f6e6b3fe9a9d433e09581802ed12` |
| `assets/planned/stations/art_station_sawmill.png` | `93e40774d193ac459a4308b878b2e6ad88f93b01ed0b2cdea2b8fcff5696e44f` |
| `assets/planned/stations/art_station_smelter.png` | `e428aa685abe9253e1c97c1c41c581abadc973e645307596d63a82704dc67780` |

All four outputs are exact registry-native `48×48 world_base` sprites and are
positively resolved by exact canonical key. Woodworking is never substituted
for Sawmill, and Smithy is never substituted for Smelter.
