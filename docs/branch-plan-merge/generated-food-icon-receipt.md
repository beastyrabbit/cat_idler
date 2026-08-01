# Generated food icon receipt

Date: 2026-07-25

Scope: the twenty-six canonical food icons required by the integrated Leader-AI
content manifest. This receipt records generation provenance, retained source
atlases, normalization artifacts, production outputs, and visual-QA contacts.
The final production files match the authoritative registry's `16×16` native
size; the generated `64×64` masters are preserved separately.

## Generation contract

Both calls used built-in reference-image mode with the maintained Idle Cat
Forest inventory-art reference contact. The generation prompts were:

### Basic/raw atlas

> Create one production sprite atlas for the non-commercial pixel-art game Idle Cat Forest, matching the supplied reference's warm, readable, hand-painted pixel inventory-icon style, dark crisp outlines, restrained forest palette, and slight three-quarter volume. EXACT layout: 3 columns by 2 rows, six equal cells, no gutters except equal spacing, one centered isolated icon per cell, no overlaps. Cell order left-to-right, top-to-bottom MUST be: 1 clear water in a small wooden cup with a blue droplet, 2 one bright red apple with leaf, 3 one whole raw silver-blue fish, 4 one raw red meat cut, 5 one bundle of green catnip leaves, 6 one amber brew in a small wooden mug. Use a perfectly flat solid chroma-key background color #D2FF4D across every unused pixel. No transparency, no texture or shadow in the background. No text, labels, letters, numbers, borders, grid lines, plates, cats, hands, UI frames, scenery, duplicate props, clipped pixels, or cast shadows outside each icon. Keep every icon fully inside its cell with generous consistent padding and a strong silhouette that remains legible when reduced to 64x64.

### Prepared/divine atlas

> Create one production sprite atlas for the non-commercial pixel-art game Idle Cat Forest, matching the supplied inventory icons exactly in warm hand-painted pixel style, dark crisp outlines, restrained forest-and-hearth palette, slight three-quarter volume, strong distinct silhouettes, and consistent scale. EXACT layout: 5 columns by 4 rows, twenty equal cells, one centered isolated food icon per cell, no overlaps. Cell order left-to-right, top-to-bottom MUST be: 1 Baked Apples in a tiny roasting dish, 2 Grilled Fish on a small wooden board, 3 Roasted Meat joint, 4 round Flatbread, 5 Apple Porridge bowl with apple slice; 6 Fish Stew bowl visibly containing fish, 7 Meat Stew bowl visibly containing meat, 8 Apple Preserves in a sealed glass jar, 9 Smoked Fish with a small curling smoke cue, 10 Dried Meat strips; 11 lattice Apple Tart, 12 Herb-crusted Fish with green herb crust, 13 closed Meat Pie with a small meat-shaped vent, 14 Surf and Turf plate with both fish and meat, 15 wrapped Travel Rations bundle; 16 decorated Festival Cake with berries and tiny pennants, 17 Hunter's Feast platter with meat and rustic antler motif, 18 Grand Lair Feast ornate platter combining fish, meat, bread and a dark dragon-scale garnish, 19 Divine Ration as a compact white-and-gold glowing provision bundle, 20 Divine Water as a white-and-gold glowing blue crystal vial. Make each cell unmistakable even at 64x64 and visually increase richness from simple foods toward feast foods. Use a perfectly flat solid chroma-key background color #D2FF4D across every unused pixel. No transparency, background texture, scenery, cast shadows outside icons, text, labels, letters, numbers, borders, grid lines, UI frames, cats, hands, duplicate dishes, clipped pixels, or extra objects. Keep every icon fully inside its cell with generous consistent padding.

## Processing and retained artifacts

The basic source is a `1536×1024` 3-by-2 atlas, giving exact `512×512`
source cells. The prepared generator returned a `1568×1003` image; the retained
`1535×1024` normalized atlas provides exact `307×256` cells for its required
5-by-4 split. Each ordered cell was isolated, the flat `#D2FF4D` background was
removed, content was contained within a transparent square canvas, and the
result was nearest-neighbour reduced to a `64×64` sRGBA master. Those masters
are preserved under `tmp/imagegen/foods/64px-masters/`.

The closed art registry is authoritative and requires `16×16 icon`.
Therefore every preserved master was nearest-neighbour reduced to an exact
`16×16` sRGBA production file under `assets/planned/foods/`. Production corners
are transparent and content remains padded inside the canvas. The brown
background in the contact sheets is inspection-only and is not present in the
production files. `final-native-contact.png` is the inspection authority for
the shipped files; the earlier contacts show the preserved master stage.

| Artifact | Purpose | SHA-256 |
| --- | --- | --- |
| `tmp/imagegen/foods/reference-contact.png` | supplied style reference contact | `3681d44b37acb3b16380d7cd51d82dac3d10c4b5d91ee63ab4b7efa33bb0db0b` |
| `tmp/imagegen/foods/generated-reference-contact.png` | delivered-food comparison contact | `4a9fcfcc8a0b2927a41ad41d5119e2f24673e3acb3bc039e2b124475c171ac17` |
| `tmp/imagegen/foods/basic/source-atlas.png` | retained basic/raw generator output | `ab5412cdfa80f50f69d38fe8cb61cdfa78eca1ef4a5e05860c668805420fca34` |
| `tmp/imagegen/foods/basic/final-contact.png` | six-icon `64×64` master-stage contact | `13178834738afcefad7e6f127751a6e1bccc92d559b69b136d516884cbe2ce66` |
| `tmp/imagegen/foods/prepared/source-atlas.png` | retained prepared/divine generator output | `013e5dc2611dc3ee52fc185a3fe764491b86d65a384e00f3d107a6e826d1ba6a` |
| `tmp/imagegen/foods/prepared/normalized-atlas.png` | exact-grid prepared atlas used for extraction | `d0907114806b29840ef95e74368293aa61e23348b1f3bfab830df3a2e49e1246` |
| `tmp/imagegen/foods/prepared/final-contact.png` | twenty-icon `64×64` master-stage contact | `c4573e82b01827fab01757c1742e2a80d68f604c3e751503acd5b890e503827a` |
| `tmp/imagegen/foods/final-native-contact.png` | all twenty-six exact-final `16×16` icons in lexicographic key order | `24a6e095b743f5d8dd186533a93e51fb07800549613136d65f99f523b4c80443` |

`tmp/imagegen/foods/malformed-basic-attempt/` is retained only as failure
evidence from a malformed extraction attempt. Its duplicated files are not
production inputs and must not be copied into the resolver.

The consolidated exact-final checksum authority is
`tmp/imagegen/native-size-final-hashes.sha256`.

## Production outputs

All files below are exact-final `16×16` sRGBA PNGs under
`assets/planned/foods/`. Their `64×64` masters remain under
`tmp/imagegen/foods/64px-masters/`.

| Canonical art key | SHA-256 |
| --- | --- |
| `art_food_water` | `0bb757150376f04b631442c927d903c31c336e3ff6d96d59e4b0972f83744500` |
| `art_food_apple` | `8304690cb7b9434c0aebddf38a727f10a76f1577db6237d66856f01832a2ca36` |
| `art_food_raw_fish` | `0c847668d56f2e2ef1b7af80d7f63c8ae3757bc24d9afd4bf51aabbbe36dc2fd` |
| `art_food_raw_meat` | `50917284d5d898940c2cb764d509d7adc46d56f98dc973514e55cc0f09f29e59` |
| `art_food_catnip` | `e7dcb3ae992d5beac6f10ceae30307ea82739640b99103d5a877f47678fe0451` |
| `art_food_brew` | `b54fc794ca51898a6945f947252bcb10125c4706447171504ab6f424c4bf1452` |
| `art_food_baked_apples` | `99d6d9989e46c08f714df9900d4d4d0d6f0b088b83b55183847cfbcfc681a691` |
| `art_food_grilled_fish` | `df65b777e18d0914391e9cddf44705ccf43a9395a4e6a8b25126f09109693ffb` |
| `art_food_roasted_meat` | `76a56046f2fc4f0d3d6766463a0d47cba92ebed2e099e957c740c2f2ed497e3c` |
| `art_food_flatbread` | `2df35784886260b561efc54c71c8fa2431104faeb0bd06e06cb38eaf347f2bcf` |
| `art_food_apple_porridge` | `73fe2059b90d0bf45bb0ec5f33269abb58632f74198a47993fb3fd0972efc891` |
| `art_food_fish_stew` | `7b596890235d76181462981bc80dd51dea331d1908c09260e93a52f7cfa4e284` |
| `art_food_meat_stew` | `38d053dcfb04fd38d2131dcb715d726b8b1a7cb15505849284a0787c177c6d8e` |
| `art_food_apple_preserves` | `17f0116ea2797a6d5c1f41480ce77db82c06c8c3d0df31b1a500ace60f802d29` |
| `art_food_smoked_fish` | `dcab9423685ee317bdfee0e6fa0ec21ca3b06867f1749fd2700b51f050b89868` |
| `art_food_dried_meat` | `41e240467f40f76a742f443833c48bbf0bd9ecb169faabbf72d29f881837a4e9` |
| `art_food_apple_tart` | `562b12975310e8caa3f453d41cc0920f7986033d5474fe300a672bdeac6f48db` |
| `art_food_herb_crusted_fish` | `4f378711a95c50d03f4aa0abeec6aedc44546cda0ef685b1fb079c2e1031d1f4` |
| `art_food_meat_pie` | `0a3c4604c5ebd5f8cdbb454203caf3a7e3085cbd2eef02da9e54e6db3b2e2825` |
| `art_food_surf_and_turf` | `1708b3d058fd0a4a24af8a1a629ed55ad96c05d8dc3be5ab52eb8e5c332cf0f6` |
| `art_food_travel_rations` | `171436b47d1615a448bcc1088845c920cdd8b934f691ddb630fc3d939c6a1066` |
| `art_food_festival_cake` | `f39c56067f7dc06f269b5f5f40bd54814df1dc9c7021c424c340b0e1d665f8b6` |
| `art_food_hunters_feast` | `32c170da36beb42d1aa92e157923574ecc033b677c135d5807c7d4ae14f3b0bb` |
| `art_food_grand_lair_feast` | `ecb0bb7c85945b02358d76ce279a06ca4a65e8ebc9497fce014a5ec997acaa77` |
| `art_food_divine_ration` | `31a4839d8e158b8577686094c7cc0a95135369b53123d4c1a2e1a2a66b6fca2b` |
| `art_food_divine_water` | `c055472a9563a17ef2807615d809355156e332f797238ede9fad4a909ccc6327` |

## Integration note

The positive allow-list and production files now agree with the authoritative
registry's exact `16×16` native dimensions. The generated `64×64` images are
masters only and must never be reported as runtime-native assets.
