# Generated Creature Portrait Receipt

Status: production candidate asset set for all twenty canonical Hunting
creatures.

This receipt closes the missing-image portion of P1.09/P1.36 for creature
portraits. It does not expose hidden roster membership: the UI may show a
portrait only when the selected revealed Lair's report includes that creature.

## Generation

- Tool mode: built-in OpenAI image generation, new-image mode.
- Original atlas:
  `/home/beasty/.config/orca/codex-runtime-home/home/generated_images/019f8a4e-ff13-7991-b8a8-b2d97232c763/exec-f6ec0a2b-5e9a-4465-9fce-422e15f718dc.png`.
- Original dimensions: 1402×1122.
- Project output: `assets/planned/creatures/`.
- Preserved generated masters: `tmp/imagegen/creatures/96px-masters/`.
- Exact-final contact sheet:
  `tmp/imagegen/creatures/final-native-contact.png`
  (`55cc5e73ea55f8a62ed02c2a9270a0df5754cdb5e871baf047a694ac236afcf9`).

The final prompt was:

> Create a production sprite atlas for a cozy-dark fantasy strategy game
> called Idle Cat Forest. EXACT LAYOUT: a perfectly regular 5-column by 4-row
> grid of 20 equal square cells, with thick saturated magenta (#ff00ff)
> gutters separating every cell and the same magenta flat background inside
> each cell. Every creature must stay entirely inside its own cell, centered
> with generous padding, no overlaps. STYLE: crisp hand-placed pixel art,
> readable at 96x96 per cell, painterly but visibly pixelated, dark forest
> palette with warm parchment highlights, consistent three-quarter
> portrait/bust framing, transparent-ready chroma background, consistent
> lighting and scale, distinctive silhouettes. No text, letters, numbers,
> labels, UI, frames, badges, cats, scenery, ground shadows crossing cells, or
> duplicate creatures. EXACT ORDER, left-to-right: Row 1: cave bat; red fox;
> badger; wild boar; gray wolf. Row 2: lynx; great stag with natural antlers;
> giant serpent; brown bear; great eagle. Row 3: mystical moon stag with
> restrained pale-violet antlers; warg; cockatrice; forest troll; griffin. Row
> 4: basilisk; manticore; chimera; wyvern; ancient elder dragon. Progress
> naturally from mundane woodland creatures in the upper rows to rare mythic
> creatures in the lower rows. Each portrait must be recognizable without
> text. The elder dragon is visually strongest, but do not add level numbers
> or boss labels. Output one clean atlas only.

## Processing and QA

The atlas was split in its exact 5×4 order. Saturated chroma pixels satisfying
red > 58%, blue > 58%, and green < 40% were made transparent; each isolated
portrait was trimmed, nearest-neighbour reduced to fit within 88×88, and
centered on a transparent 96×96 sRGBA master. Those twenty masters are
preserved byte-for-byte under `tmp/imagegen/creatures/96px-masters/`.

The closed art registry is authoritative and requires `80×80 portrait`.
Therefore each preserved master was nearest-neighbour reduced to the exact
`80×80` sRGBA production file under `assets/planned/creatures/`. The
`final-native-contact.png` sheet, not the older `final-contact.png`, is the
inspection authority for the shipped files. It uses lexicographic canonical-key
order and was visually inspected for complete key coverage, distinct
silhouettes, complete transparent bounds, and absence of text/exact-level
leakage.

The consolidated exact-final checksum authority is
`tmp/imagegen/native-size-final-hashes.sha256`.

| Canonical art key / file | SHA-256 |
|---|---|
| `art_creature_cave_bat.png` | `710aec819e921dc6f7d622126017a2ff566a64ae1ebdb2a606c03bfd3ca27a4d` |
| `art_creature_red_fox.png` | `4c1b646c27e19ea8a3444fe1b16db06a9943bbe073a09aa4840fef4d460d3ec8` |
| `art_creature_badger.png` | `df3fc3398e8036baeafb1c25f29558376288059847a87cbf4c11e086081a190b` |
| `art_creature_wild_boar.png` | `1c39c420c0a638bd4bce652df561631a1a438cd685df9cd37cfe9c7c07a75960` |
| `art_creature_gray_wolf.png` | `a6964dd28dbc1ecb437ecc853e776ff5ad8d57ac56c683bc412df71ff460d8fd` |
| `art_creature_lynx.png` | `6619e5f8c8609698c2294e20538cf51860ed265ef957b3dffde7d7acf52feccf` |
| `art_creature_great_stag.png` | `8f99dc8d208ee3370190bbc798319e4e30c8ac98a771fd5c53f696171ce4d49a` |
| `art_creature_giant_serpent.png` | `690935fe338d010bbc0629e2903b76b3753114396e897b39e5c58f849c64cf0f` |
| `art_creature_brown_bear.png` | `58718d05a50dca0289aa3b9d26b06058b3988a8438c567da1bc82f70d266f508` |
| `art_creature_great_eagle.png` | `70834fb2a23286a2ddfa75234f8b8757474531e290582c92e9f99a17b384250b` |
| `art_creature_moon_stag.png` | `3d6122f3e31d6abfcca71fcddc4aad440665efd59453746b58f1b641ec86c64a` |
| `art_creature_warg.png` | `d5e1687d0b85a5ba3209f12a318acd073f68c908dcd27ba97f5ddf08f004ac14` |
| `art_creature_cockatrice.png` | `d7c81765bfb276c2197e6bc47773c28f830a923e8a2758a71d4ce8bb2ad1cc25` |
| `art_creature_forest_troll.png` | `4a605003023ac72cff8fe9b7fc8805b78174f633b6bc3e6afa093e765e179926` |
| `art_creature_griffin.png` | `ec0747c6a12f90ad8f91e8737d4faecbfc77072af92031b9c5cef8c592d24e84` |
| `art_creature_basilisk.png` | `6a94582a412743ea985ec914d8dad43dccff1ac97559a6331cd108e839d8fbc9` |
| `art_creature_manticore.png` | `c242519b598839b57d616e0d2e81bb8eee56c7cf9988dfed2a943a3b148244ac` |
| `art_creature_chimera.png` | `9ea59e6ecce8c2ccefec6b0b938dc6dc14293590cec5d7ed426ca72316d1ba16` |
| `art_creature_wyvern.png` | `da9a214b396ca9c74a1803b98fb598c3ff54448e4891b7aae04e25976f102cdf` |
| `art_creature_elder_dragon.png` | `ac904a3d682943855770591a2fd1a02609db1fc4d3acb1f6ce30bc5017b954b3` |

The canonical content manifest already owns these twenty exact `art_key`
values. Runtime binding must resolve these paths only after the corresponding
report-safe creature row is present; there is no generic portrait fallback.
