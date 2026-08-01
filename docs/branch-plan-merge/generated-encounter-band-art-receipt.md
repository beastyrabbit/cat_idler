# Generated Encounter-Band Art Receipt

Date: 2026-07-25  
Status: delivered production candidates, bound only by canonical public encounter key.

This receipt closes the six coarse encounter-band images registered by the
content manifest. They are intentionally distinct from the ten world-facing
`art_lair_visual_*` bands: encounter art communicates only the six public
roster/risk bands and never exposes an exact Lair level, roster, statistics,
regeneration, or respawn deadline.

## Generation and reference

- Tool: built-in OpenAI image generation in local-reference mode.
- Accepted visual reference:
  `tmp/imagegen/lairs/contact-sheet.png`.
- Immutable generated source:
  `tmp/imagegen/encounter-bands/source-atlas.png`
  (`7603f6e3a7e8e654282656d14d1930b4bc6925e3e7b1628359b2d093f27aeaf9`).
- Production directory: `assets/planned/lairs/`.
- Inspected contact:
  `tmp/imagegen/encounter-bands/final-contact.png`
  (`e58b95d8e45b06d6fdddd6197454ca22abaf01ae79a25314eb1a0a259eb0321d`).

The prompt requested one exact 3×2 atlas, ordered from `01–19` through
`95–100`, using the accepted root/rock/moss/timber/violet Lair language. It
required increasing danger without printed levels, six isolated top-down
sprites, identical cell scale, and no cats, creatures, text, numerals, UI,
frames, or hidden encounter information. Empty pixels used a vivid green
chroma field for deterministic alpha extraction.

## Post-processing and inspection

The 1536×1024 atlas was split into six ordered 512×512 cells. ImageMagick
removed only the high-saturation green field, trimmed the isolated sprite,
used point filtering, and centered the result on an 80×80 transparent canvas.
Two harmless detached generation specks below the second and third cells were
excluded before reduction. The final contact was visually inspected for
ordering, isolation, transparency, consistent silhouette, and increasing
danger.

All production files are 80×80 sRGBA PNGs:

| Canonical key / file | SHA-256 |
|---|---|
| `art_encounter_band_01_19.png` | `557613a8a156a26d843ea427a25872be4ad701ad7f37a2716d9e56be568c4d38` |
| `art_encounter_band_20_39.png` | `885f563653cc5073667b03ade48f1668b81d63d73bfdccaa5fed647c8e3f8d62` |
| `art_encounter_band_40_59.png` | `61a3fed8f040f6c5b7009e99388612ede17d8ca300ded1fec8ee7c7209e4bb79` |
| `art_encounter_band_60_79.png` | `7728a21fbd5357735e77c500b7655590350c47511d75fc53758eff8baf6c4136` |
| `art_encounter_band_80_94.png` | `895ea34e986be944dcbbe7691f946e8e11c34dbae0b09351be04948c7cb1f758` |
| `art_encounter_band_95_100.png` | `b3439dd8376df9d8dffbf20c19485fbb6ee0ba4c39833768c67416d746b542e5` |

## Runtime binding

`leader_ai_ui::art_assets` contains six explicit 80×80 `LairBand` allow-list
entries. Unknown encounter-like keys remain unresolved. The six keys never
alias the ten visual-band keys or paths, so a client cannot convert a coarse
encounter report into a more exact level band.
