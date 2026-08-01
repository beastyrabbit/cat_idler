# Generated Named-Material Icon Receipt

Status: production candidate inventory icons for all twenty canonical named
Hunting drops.

This set implements the unique-icon requirement in P1.09, P1.29, and P1.36.
The icons identify physical named materials only; quality, augmentation,
condition, provenance, exact Lair level, and hidden drop odds remain report
text/detail state rather than being encoded into the picture.

## Generation

- Tool mode: built-in OpenAI image generation, new-image mode.
- Original atlas:
  `/home/beasty/.config/orca/codex-runtime-home/home/generated_images/019f8a4e-ff13-7991-b8a8-b2d97232c763/exec-51138512-4e0c-4dc8-b367-3d22d19cca62.png`.
- Original dimensions: 1402×1122.
- Project output: `assets/planned/materials/`.
- Preserved generated masters: `tmp/imagegen/materials/64px-masters/`.
- Exact-final contact sheet:
  `tmp/imagegen/materials/final-native-contact.png`
  (`208564ac8d75ec29070176df804ee667edf086bf505bb4fd841a733e89676e58`).

The final prompt was:

> Create a production inventory-icon atlas for a cozy-dark pixel-art fantasy
> strategy game called Idle Cat Forest. EXACT LAYOUT: a perfectly regular
> 5-column by 4-row grid of 20 equal square cells, with thick saturated magenta
> (#ff00ff) gutters separating every cell and the same flat magenta background
> inside each cell. Every item must stay entirely inside its own cell, centered
> with generous padding, no overlaps. STYLE: crisp hand-placed pixel art
> inventory icons, readable at 64x64 per cell, dark-forest palette with warm
> parchment highlights, consistent three-quarter object lighting, strong
> distinct silhouettes, one isolated harvested material per cell,
> transparent-ready chroma background. No creatures or hands holding the item,
> no text, letters, numbers, labels, UI, frames, badges, scenery, piles spanning
> cells, or duplicates. EXACT ORDER, left-to-right: Row 1: single cave-bat wing;
> rolled red-fox pelt; folded badger pelt; wild-boar tusk; folded gray-wolf
> pelt. Row 2: spotted lynx pelt; great-stag antler; giant-serpent scale; thick
> brown-bear pelt; great-eagle feather. Row 3: luminous moon-stag antler; warg
> fang; preserved cockatrice eye; forest-troll hide; griffin plume. Row 4:
> basilisk scale; manticore tail barb; glowing beast core crystal-organ;
> translucent wyvern wing membrane; ancient dragon heart. Progress from mundane
> natural materials in the upper rows to rare mystical materials in the lower
> rows. Keep violet magic restrained to the explicitly mystical materials.
> Every icon must be recognizable without text. Output one clean atlas only.

## Processing and QA

The atlas was split in exact 5×4 order. Saturated chroma pixels satisfying red
> 58%, blue > 58%, and green < 40% were made transparent. Each icon was
trimmed, nearest-neighbour reduced to fit within 56×56, and centered on a
transparent 64×64 sRGBA master. Those masters are preserved under
`tmp/imagegen/materials/64px-masters/`.

The closed art registry is authoritative and requires `16×16 icon`.
Therefore each preserved master was nearest-neighbour reduced to an exact
`16×16` sRGBA production file under `assets/planned/materials/`. The
`final-native-contact.png` sheet, not the older `final-contact.png`, is the
inspection authority. It uses lexicographic canonical-key order and was checked
for complete key coverage, distinct identity, complete transparent bounds, and
absence of text/quality/level leakage.

The consolidated exact-final checksum authority is
`tmp/imagegen/native-size-final-hashes.sha256`.

| Canonical art key / file | SHA-256 |
|---|---|
| `art_material_bat_wing.png` | `3928b7d875fbb0fcc4aca947e5604f8cea170d443e180191d5730b2e54c0fc74` |
| `art_material_fox_pelt.png` | `ae61a8c4ee8ba2535aea42d033bd871a260aeb7ddeb611d155ba87311f616ae2` |
| `art_material_badger_pelt.png` | `71592aa6146464c8c605ebe74a2537e2b40a89928603e9392cc42f5b3efd2f95` |
| `art_material_boar_tusk.png` | `79828f0710a2ba80c9e4852d6bc5fe4714f9b9924f3f8a18aadbcca5e5e80117` |
| `art_material_wolf_pelt.png` | `884f482edcb1a1b575f30a35517ce1c7f329b1a07c93937f30e3d356f70bf4fd` |
| `art_material_lynx_pelt.png` | `3d4267ac0e7e7cd40229551b996a89f68c6b3cb39cd00016124b17a5e0f2af53` |
| `art_material_stag_antler.png` | `518c0aaa180e864277dfcef4cb2fa11e66ca63347d25d193ea1fd63edf02bad4` |
| `art_material_serpent_scale.png` | `90aed3baf63aa84aa879227cbd7c151df8861c89cdd378305fe4b81170c25bb7` |
| `art_material_bear_pelt.png` | `bbd8dc697f93807e312848a68bedfdc31649dd698a21a535d84a63a3d5ba5b17` |
| `art_material_eagle_feather.png` | `a50c5894ff1c13f4adfb30b86205ea3b74d871d3adb859e526d58d51923d0d38` |
| `art_material_moon_antler.png` | `10ff3274f8d5463b5a80b3ae16a33078967bc90ca25029036f6a47abcf1003bb` |
| `art_material_warg_fang.png` | `5536af39c79237c93b8183803cd4d1bef5634b264dea7d024a32249736137520` |
| `art_material_cockatrice_eye.png` | `2b1f0e1a7521bfa2e738d86850773fc774cf0298a97e5d38fe9f1e096d35dcde` |
| `art_material_troll_hide.png` | `871f22eafd73285c5e75ddf82adc6899372bed3d2f24903a178df8f1afd4c122` |
| `art_material_griffin_plume.png` | `fe7b2a76def42e61fe6c86e1a125379c820eaf1587889039e6392f980ccd79c1` |
| `art_material_basilisk_scale.png` | `d32d7a6daeef84003b705e6742d9d6ed4a22281248b7febcd047b4490264a707` |
| `art_material_manticore_barb.png` | `2c1ed59813956bdac4ed14729ccf5513338c3e9895976187f11d36d2046bdd5b` |
| `art_material_beast_core.png` | `b4092a831c8b232047ae567f675b312d724844a4dcb2f9a7aa7c5f8d924eb725` |
| `art_material_wyvern_membrane.png` | `a367bfa487c13979f46bf2271be3e836627845b47bf13a4ccd13a05955ef374f` |
| `art_material_dragon_heart.png` | `d68add3c5092b7bb4337f4adb6c0eb5384f0406d2f78e247d4fafbea465e62ac` |

The bounded client catalog resolves only these exact canonical manifest keys.
An unknown material key returns no image rather than borrowing a generic
materials icon.
