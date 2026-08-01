# Visual specification QA

This record covers the documentation diagrams only. It proves that the stored
specification is complete, renderable, legible, and linked; it is not evidence
that the production Bevy UI, renderer, sprites, icons, or browser acceptance are
implemented.

## Accepted inventory — 2026-07-25

- Ten source-owned SVG diagrams are present.
- Every SVG has a native `viewBox`, `<title>`, `<desc>`, and prose fallback.
- Ten matching 1600×1000 PNG renderings are checked in.
- One 848×1370 [contact sheet](rendered/contact-sheet.png) shows the complete
  package together.
- Dark forest, wood, moss, and rust panels use explicit cream text; pale panels
  use ink. Full-size review found no clipped headings, pipeline labels, footer
  fallbacks, or out-of-bounds boxes.

## Reproducible checks

The serialized render used `/usr/bin/rsvg-convert --width 1600
--keep-aspect-ratio` once per source. The contact sheet was produced with one
ImageMagick `montage` invocation over the ten PNGs.

Validation results:

- `xmllint --noout docs/leader-ai-overhaul/visual-spec/*.svg`: 10/10 clean.
- ImageMagick `identify`: every named diagram is 1600×1000; the contact sheet is
  848×1370.
- Full-size visual inspection: authority/visibility, progression/Hole,
  construction/storage, diplomacy/barter, family/governance,
  hunting/food/items, shell/responsive, and task-footprint diagrams inspected
  directly.
- Contact-sheet inspection: asset matrix and implementation DAG inspected in
  the complete-package context.

## Acceptance boundary

The package is accepted as LAI.35/LAI.53 design and explanation evidence. LAI.49,
LAI.50, LAI.54, and LAI.66–LAI.68 still own production art, world rendering,
screens, inspectors, responsive behavior, native/WASM behavior, and accessible
interaction. LAI.69–LAI.70 still own the final real-server Playwright and visible
browser evidence.
