# Paws & Whiskers Pack Audit

Forgejo issue: Refs #8

## Source Pack

- Local source path: `public/Paws & Whiskers - Isometric Cats Pack (Free)/`
- Files found:
  - `Cat_1_spritesheet_free.png`
  - `readme_free.txt`
- Source pack status: gitignored source asset library. Do not commit the pack or its symlink.
- Curated asset status: no sprites copied into tracked `public/images/iso/cats/`.

## License Status

`readme_free.txt` confirms the free pack may be used in non-commercial projects and modified, but it also says the pack cannot be redistributed, even if modified.

That blocks committing copied sprites to this repository. A curated copy under `public/images/iso/cats/` would redistribute the asset, so the Paws & Whiskers integration should stay blocked until the owner provides a license grant or an alternate asset source that explicitly allows redistribution of the committed subset.

## Measured Inventory

Measurements were taken directly from `Cat_1_spritesheet_free.png` alpha pixels.

- Image: PNG RGBA, `1024 x 64`
- Frame grid: `32 x 32` cells
- Cell count: `32` columns x `2` rows = `64` non-empty cells
- Facing count: `8` facings per row
- Frames per facing: `4` frames per facing
- Facing column groups, inferred visually and matching the existing map sheet order:
  - `S`: columns `0-3`
  - `SW`: columns `4-7`
  - `W`: columns `8-11`
  - `NW`: columns `12-15`
  - `N`: columns `16-19`
  - `NE`: columns `20-23`
  - `E`: columns `24-27`
  - `SE`: columns `28-31`
- Animation rows:
  - Row `0`: all facings, 4 frames, stable foot baseline at y `25`
  - Row `1`: all facings, 4 frames, alternate stepping; odd frames lift to y `23-24` for some facings
  - The pack files do not name the row semantics.
- Opaque union across all cells: `(x=9, y=5) -> (x=24, y=26)` exclusive, so the maximum drawn body is `15 x 21` px inside a `32 x 32` frame.
- Per-frame opaque bbox ranges:
  - `minX`: `9-11`
  - `maxX`: `22-24` exclusive
  - `minY`: `5-8`
  - `maxY`: `24-26` exclusive
- Proposed fixed ground anchor: `(16, 26)` in cell pixels, normalized `(0.5, 0.8125)`.
  - This sits one pixel below the common row-0 foot baseline and avoids visible vertical bobbing from row-1 lifted feet.

## Foot Contact Notes

Row 0 bottom-contact centers by facing:

| Facing | Bottom y | Foot center x |
| --- | ---: | ---: |
| S | 25 | 16.0 |
| SW | 25 | 13.5 |
| W | 25 | 15.5 |
| NW | 25 | 18.5 |
| N | 25 | 16.0 |
| NE | 25 | 13.5 |
| E | 25 | 16.5 |
| SE | 25 | 18.5 |

Row 1 alternates planted and lifted frames. Frame `0` and frame `2` generally keep the row-0 bottom contact. Frame `1` and frame `3` shift the bottom contact within approximately x `12.5-19.5` and y `23-24`, so the renderer should keep a fixed anchor rather than anchoring each frame to its measured bottom pixel.

## Coat And Life-Stage Coverage

- Coat/color variants in the free pack: `1`
- Variant: `Cat_1`, a peach/cream cat with dark outline
- Opaque palette:
  - `#fdcbb0` body
  - `#fca790` shadow
  - `#fce0d2` and `#f8e4d8` highlights
  - `#2e222f` outline
  - `#7f708a` feature color
  - `#f68181` feature color
- Life-stage variants: none found
- Carry/accessory variants: none found

Because the free pack only contains one coat variant, deterministic genetics-to-variant mapping would collapse every cat to the same visual variant. If an allowed redistributable pack with multiple variants is supplied, map existing inherited `spriteParams` traits to variants in a pure helper and unit-test that the same traits always select the same variant.

## Integration Notes If Unblocked

- Copy only the used sheet into a tracked path such as `public/images/iso/cats/paws-whiskers/cat-1.png`.
- Keep the source pack gitignored and untracked.
- Share sprite constants between DOM and Pixi:
  - cell size `32`
  - columns per row `32`
  - row count `2`
  - frames per facing `4`
  - anchor `(16, 26)`
- DOM renderer needs y-offset support for the selected animation row; the current CSS only animates horizontal sheet offsets.
- Pixi renderer needs `frame.y = row * 32` in addition to the existing `frame.x` update.
- Existing life-stage scaling can remain renderer-side because the source sheet has no kitten/adult/elder art.
- Re-check name label, leader crown, badge, hat, and carry icon offsets against anchor `(16, 26)` after visual integration.
