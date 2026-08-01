# Art Style Inspection and Generation Contract

Date: 2026-07-25

This is the required visual reference for LAI.36, LAI.38–LAI.43, LAI.49–LAI.51, and
LAI.68–LAI.70. Missing art is an implementation deliverable. Existing or protected source art is
reused when its receipt permits that; genuinely missing art is generated only after comparison
with these shipped references.

## Inspected sources

- Current game: `public/images/`, including resource icons, task/status icons, isometric terrain,
  buildings, enemies, and cat sheets.
- Protected Shrine-upgrade source:
  `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade/public/images/game/`.
- Source-transfer ownership and copy rules:
  `docs/branch-plan-merge/source-transfer-manifest.md`.

Representative files inspected visually at original resolution include:

- the currently loaded Bevy assets
  `public/images/game/buildings/{workshop,town_hall}.png` (`48×48`),
  `public/images/game/nature/tree_oak.png` (`16×16`),
  `public/images/game/props/campfire.png` (`16×16`), and
  `public/images/cats/cat-sheet.png` (`1024×64`);
- protected-source `nature/tree_oak.png` and
  `tree_oak_apples_{low,mid,full}.png`;
- `terrain/{grass,water,water_edge,farmland}.png`;
- `sites/{lair,quarry}.png`;
- `props/{barrel,crate,sack,log_pile,stone_pile,well}.png`;
- `icons/{fish,water,food,grain,flour,logs,planks,stone,blocks,hide,bone}.png`;
- the layered Hole/base/axis art recorded by the source-transfer manifest;
- legacy isometric `public/images/iso/buildings/workshop.png`, task icons, and enemies. These are
  comparison inputs only: the current Bevy client actually loads the `public/images/game/**`
  family above, so new world art must match that family unless an explicit renderer/card chooses
  and documents a different asset class.

## Established visual language

- Crisp transparent pixel art with hard nearest-neighbor edges. No blur, antialias haze, vector
  smoothness, glow, glass, gradient wash, photographic texture, or 3D render lighting.
- Limited earthy palettes: dark forest greens, muted moss and ochre, warm timber, parchment tan,
  stone gray, iron blue-gray, and restrained high-saturation accents for food, danger, quality,
  and divine state.
- Strong dark silhouettes and high-contrast internal clusters so objects survive gameplay zoom.
- Top-down or shallow oblique world projection consistent with the neighboring tile/building
  class. Icons use a centered readable object silhouette, not a miniature scene.
- Transparent unused pixels and tight nontransparent bounds. Do not hide opaque matte pixels in
  corners or ship oversized empty canvases.
- One visual idea per icon. Quality, reservation, blockage, construction phase, and report
  uncertainty are separate overlays/badges rather than baked into every base sprite.
- Animation sheets keep identical anchors, footprint, frame dimensions, and nearest-neighbor
  alignment across states.

## Native dimension classes

The protected source contains 81 `16×16`, 15 `48×48`, 7 `32×32`, 32 `80×80`, and a smaller
number of explicit multi-frame/layer sheets. The registry must record each asset deliberately;
blanket `64×64` defaults are invalid.

| Asset class | Native contract | Notes |
|---|---:|---|
| Terrain, small props, crop stages, tree base/overlays | `16×16` | One world tile; Apple overlays align pixel-for-pixel with `tree_oak.png`. |
| World sites and medium actors | `32×32` unless the inspected neighbor uses a sheet | Lair/quarry references are `32×32`; retain exact ground anchor. |
| Standalone buildings | `48×48` when using the protected-source building class | Footprint is still simulation-owned; transparent overhang may exceed occupied cells. |
| Hole base and axis/state layers | `80×80` | Fixed 5×5 landmark; every layer shares identical bounds and anchor. |
| Resource/food/item detail icons | match the selected icon family | Protected legacy icons are often `128×128`; small HUD badges remain compact. Do not mix scales inside one UI list. |
| Cat/creature animation | explicit frame and sheet dimensions | Registry records both frame size and sheet layout; no inferred slicing. |
| UI nine-slices/panels | source-native dimensions | Preserve corner pixels and nearest-neighbor stretch regions. |

## Missing-art workflow

1. Resolve the manifest `ArtKey`, role, target path, native size, layer/anchor, states, and
   accessibility label before generating.
2. Search current and protected source receipts. Reuse/copy a permitted exact asset before
   generating a substitute.
3. View at least one base reference and one same-class neighboring reference at original
   resolution.
4. Generate or edit with the image-generation skill using the exact native canvas, transparent
   background, palette/style constraints above, and all required states in one coherent request
   when consistency matters.
5. Inspect the original-resolution result. Reject softness, inconsistent perspective, stray
   alpha, wrong dimensions, clipped silhouettes, unstable anchors, unreadable gameplay zoom, or
   a style that resembles generic AI illustration.
6. Register source/generation provenance, content hash, dimensions, logical key, state/layer,
   accessibility binding, and replacement/deletion disposition.
7. Verify transparent bounds, art-key completeness, nearest-neighbor rendering, native and WASM
   load, all zoom levels, color-independent meaning, and screenshot/browser checkpoints.

## Required generated families

The final inventory is manifest-driven and additive. At minimum it includes any unresolved:

- concrete Water, Apple, raw Fish, raw Meat, and all eighteen Cookhouse meal icons;
- Apple `empty/low/medium/full` states and exact tree-task feedback;
- five quality badges and lot/reservation/spoilage overlays;
- Fishing Hut, shoreline/dock states, Rod condition, and fish habitat states;
- twenty creature sprites, portraits, lair encounter/clear/respawn states, and twenty named drop
  icons/detail layers;
- Hole base, ten public axis bands, feed/upgrade/cargo/blocked states, and Void/Notes icons;
- material raw/processed states, tools, fixtures, augmentations, microscopes, and item detail
  layers;
- Cookhouse and construction phases, physical storage fullness, family/enterprise/housing,
  institution, governance, diplomacy, barter, task, blocker, and report-confidence visuals.

An asset family is not accepted merely because a path exists. Its simulation state, protocol
projection, renderer binding, UI use, accessibility label, and visible screenshot evidence must
all be present.

## Corrected measured audit: generated pack is not yet style-matched

The 2026-07-25 Opus 5 audit decoded every relevant PNG and found that the
delivered generated pack does not satisfy the inspected style above. Shipped
world references usually use 4–15 colors per sprite (the 48×48 Workshop uses
52; the 16×16 Oak uses 7; the Barrel uses 4; cat frames use 7) with binary
alpha. Generated stations/Cookhouse/Huts use roughly 1,188–1,512 colors at
48×48; portraits/Lairs use roughly 3,244–3,382 at 80×80; Lairs introduce
38–151 alpha levels. Perspective and state anchors also drift.

These files are restyle inputs, not accepted production art. The canonical
measurements, missing families, runtime-delivery gaps, and generation prompts
are recorded in
[`lai49-50-corrected-art-runtime-audit.md`](lai49-50-corrected-art-runtime-audit.md).
Any earlier receipt claiming completeness must be read only as path/dimension
provenance, never as style or live-runtime acceptance.
