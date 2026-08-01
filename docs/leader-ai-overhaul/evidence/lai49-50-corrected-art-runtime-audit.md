# LAI.49–LAI.50 Corrected Art and Runtime Audit

Recorded: 2026-07-25

This corrected Opus 5 audit decoded the actual PNGs rather than relying on
receipts or resolver rows. It read both locked plans, the board from line 1,
the style and remaining-asset inventories, the canonical 263-row registry,
resolvers, renderer, layered-sprite compositor, all three LAI.50 inspectors,
accessibility helpers, protocol visual DTOs, and the server projection.

The reviewer made no repository edits and ran no Cargo, compiler, test, build,
lint, formatter, browser, Playwright, image-generation, or validation command.
The PNG inspection was read-only. These are static findings, not acceptance
evidence.

## Corrected verdict

Earlier receipts correctly prove that many files exist at the declared native
dimensions. They do not prove that the files match the shipped art style or can
render in live play.

Three blockers control LAI.49/50:

1. Most generated LAI.49 art is not in the established game style.
2. Server projection sends empty content collections, so major asset families
   and two inspectors are unreachable.
3. The live world renderer falls back to colored quads for most marker roles.

LAI.49 and LAI.50 remain partial `dev`. The off-style images are restyle
inputs, not accepted production art.

## Measured shipped style

The current Bevy client loads the `public/images/game/**`,
`public/images/cats/**`, and compact task/status families. Original-resolution
pixel decoding establishes:

- terrain 16×16: median 2 colors;
- nature 16×16: median 4 colors;
- props/infra 16×16: median 4 colors;
- interior 16×16: median 6 colors;
- buildings 48×48: median 15 colors;
- cats 32×32 frames: about 7 colors;
- task icons 64×64: median 18 colors;
- status icons 32×32: median 6 colors.

Concrete references:

- `workshop.png`: 48×48 indexed, 52 colors, binary alpha, flat timber/stone
  fields, hard dark outline, shallow oblique projection.
- `tree_oak.png`: 16×16 indexed, 7 colors, hard clusters and a tight
  silhouette.
- `barrel.png`: 16×16, 4 colors.
- `cat-sheet.png`: fixed 32×32 frame grid, 7 colors, binary alpha.

The production contract is therefore:

- crisp nearest-neighbor pixels;
- binary alpha unless an explicitly inspected source class proves otherwise;
- limited earthy palette and flat clustered shading;
- strong dark silhouette;
- top-down or shallow oblique perspective matching the neighboring class;
- identical anchors/bounds across state/orientation frames;
- one visual concept per base sprite, with quality/blockage/report state as
  separate overlays;
- readable silhouette at gameplay zoom.

## Measured generated pack mismatch

Median decoded values:

| Family | Native size | Median colors | Colors/pixel |
|---|---:|---:|---:|
| content | 16×16 / some 32×32 | 96 | 0.36 |
| foods | 16×16 | 99 | 0.39 |
| items | 16×16 | 110 | 0.43 |
| materials | 16×16 | 104 | 0.41 |
| recipes | 16×16 | 92 | 0.36 |
| fixtures | 32×32 | 417 | 0.41 |
| augmentations | 32×32 | 408 | 0.40 |
| stations | 48×48 | 1,234 | 0.54 |
| Cookhouse | 48×48 | 1,512 | 0.66 |
| Fishing Hut | 48×48 | 1,188 | 0.52 |
| creatures/portraits | 80×80 | 3,382 | 0.53 |
| Lairs | 80×80 | 3,244 | 0.51 |

The shipped family is typically 0.01–0.04 colors per pixel. The generated
family is 0.36–0.66, with painterly tonal noise, inconsistent outlines, and
different projection. Lairs are the only inspected family with 38–151 alpha
levels and partially transparent edge pixels.

Examples:

- the generated Sawmill has 1,282 colors at 48×48 and reads as a noisy dark
  mass beside the 52-color Workshop;
- Cookhouse uses a steep near-elevation roof while the shipped class is shallow
  oblique;
- Fishing Hut orientation bounds move between states, breaking a stable
  anchor/footprint;
- creature portraits are semi-realistic painterly busts beside seven-color cat
  frames;
- high-tier Lair art contains soft antialiased halos.

These meet the style guide's explicit reject conditions: softness, unstable
anchors, inconsistent perspective, stray alpha, unreadable zoom, and generic
AI-illustration appearance.

## Duplication and registry facts

`assets/planned` contains roughly 360 files but only about 152 distinct images:
208 copies belong to duplicate-content groups. The creatures directory is
byte-identical to portraits for all twenty entries; several food/item/material/
fixture/augmentation families duplicate `content` copies. About 108 files have
no registry row.

The canonical art registry has 263 rows and the delivered files generally
match declared dimensions. Eleven registered station paths do not exist at
their planned path and are redirected to legacy building art. The Black Hole
row declares 48×48 but resolves an 80×80 source.

The redirected legacy stations are stylistically correct for the shipped
family. The generated Cookhouse, Fishing Hut, Sawmill, and Smelter are not,
creating a direct same-map mismatch in the 48×48 building class.

## Dead runtime delivery

The canonical server projection currently emits empty:

- food stocks;
- Hunting sites;
- rare materials;
- fixtures;
- Cookhouse batches;
- Fishing Huts.

Consequences:

- ten Lair bands and six encounter bands cannot render from live data;
- twenty portraits and twenty named material icons cannot be selected through
  their intended Hunting/material paths;
- Cookhouse and Fishing Hut activity/state sheets have no live producer;
- fixture and food icon families do not reach their screens;
- the Food/Cookhouse/Fishing inspector and much of Hole/Hunting are
  structurally empty.

Runtime retention/projection must be completed before art is counted as
delivered.

## Colored-quads fallback

The LAI.68 world renderer uses `Sprite::from_color` when a marker lacks a
resolvable art key. Only five of twenty-two audited marker roles carry a key.
Footprints, work sites, delivery sites, routes, containers, lots, items,
residences, and family markers therefore remain colored rectangles.

Construction sends a non-registry string for scaffold state. Visual state can
also apply a 16×16 resource icon over a building-sized footprint. Fishing dock
and water markers reuse the 48×48 Hut sprite rather than having role-specific
art.

The final renderer must fail missing art into an explicit bounded unavailable
state during development, not a production placeholder. Every visible role
needs a typed `ArtKey`, native class, anchor, accessibility label, and exact
state trigger.

## Hole geometry correction

The base plus thirty axis files are genuinely cumulative and the compositor
correctly selects one current image per axis on an 80×80/5×5 canvas. That
behavior is confirmed good.

However, every file's nontransparent pixels are bounded to the central 48×48.
The sixteen-tile paved ring required by the 5×5 landmark has no pixels. The
Hole family needs a style-matched 80×80 base/ring treatment whose occupied
visual footprint actually communicates the full permanent 5×5 landmark and
central 3×3 work area without leaking hidden levels.

## Confirmed missing or unreachable families

Missing file/key/resolver/producer combinations include:

- quality badges and compositor;
- Basket/Barrel/Crate/Chest/Rack fullness states;
- family/enterprise signs;
- residence/household overlays;
- Apple-empty plus exact empty/low/medium/full states;
- raw/processed pairs for applicable materials;
- Notes and Void icons;
- Cookhouse six-state producer and registry rows;
- Fishing Hut eight orientation/activity-state producer and registry rows;
- crop/Apple/transport/quarry resolver-only keys with no projection producer;
- Food inspector resolver calls;
- explicit Lair site art distinct from Quarry;
- role-specific dock, reserved water, delivery endpoint, work-slot, blocker,
  route, reservation, and confidence overlays.

No current source test opens or decodes a PNG, which explains why missing paths
and style/alpha mismatches were not detected.

## Restyle and generation contract

Generation remains paused while image generation/browser/validation are
forbidden. When that restriction is lifted, generation must use the image
skill and the inspected references; no placeholder, generic AI-pixel-art
prompt, or style guess is acceptable.

Global prompt prefix:

> Match the attached Idle Cat Forest production references exactly: hard
> nearest-neighbor pixel clusters, binary transparency, limited earthy
> Kenney-like palette, strong dark outline, flat clustered shading, readable
> silhouette at native gameplay zoom, no gradients, no antialiasing, no soft
> glow, no painterly texture, no 3D lighting. Preserve the specified native
> canvas, anchor, footprint, and transparent bounds.

Per-family requirements:

- 16×16 foods/materials/items/recipes: one centered object, normally no more
  than the reference icon palette range; raw/processed silhouettes visibly
  distinct; quality not baked in.
- 32×32 fixtures/augmentations: one readable device/component, same dark
  outline and flat fields; no miniature room scene.
- 48×48 buildings: use `workshop.png`, Mill, Tannery, and Research Hut as
  original-resolution references; shallow oblique projection; stable footprint
  and anchor across scaffold/structure/fit-out/idle/working/blocked and every
  Hut orientation.
- 80×80 Hole: full 5×5 visual occupancy, explicit paved ring, central 3×3 work
  region, cumulative public axis states, no exact hidden level text.
- Lairs/creatures: match the small hard-edged enemy/cat language rather than
  realism; fixed band silhouette, no soft alpha, no exact hidden ecology.
- containers: one base per container plus empty/low/medium/high/full states
  with identical anchor and capacity-readable contents.
- family/enterprise/residence: simple world signs/overlays with one semantic
  motif and stable anchor; names remain text/accessibility, not baked pixels.

Expected work is approximately 180 images: roughly 112 restyles of off-family
delivered art plus about 68 genuinely missing state/overlay assets. Exact
counts remain manifest-driven and additive.

## Dependency-ordered completion

1. Finish LAI.46/63 runtime retention and exact visual-state identities.
2. Fill the six empty canonical server collections and all task/construction/
   Hole/trade visual fields.
3. Freeze typed `ArtKey` producers, native classes, anchors, roles, state
   triggers, accessibility text, and shipped paths.
4. Restyle the rejected existing pack against original-resolution shipped
   references, then generate the genuinely missing families.
5. Replace colored quads/non-key strings with exact assets or explicit
   unavailable developer diagnostics; never production placeholders.
6. Wire the Food inspector and all world/panel consumers.
7. After the external serialized build gate, perform original-resolution PNG
   checks, binary-alpha/palette/bounds/anchor checks, native/WASM loading,
   zoom/despawn/restart, color-independent accessibility, screenshot matrix,
   Playwright, and independent visible-browser evidence.

No asset is accepted merely because a file or resolver row exists.
