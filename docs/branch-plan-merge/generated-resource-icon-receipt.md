# Resource icon source-adaptation and generation receipt

Date: 2026-07-25

Scope: all twenty-five canonical resource icons at the registry-native
`16×16 icon` size. Nineteen existing semantically exact project sources were
adapted; six missing identities were generated. No neighboring picture was
substituted for a different resource.

## Source-adapted family

Each source was preserved before conversion under
`tmp/imagegen/resources/source-adapted/`. The production operation was
nearest-neighbour containment on a transparent `16×16` sRGBA canvas at the
manifest-planned path.

| Canonical key | Preserved exact source | Final SHA-256 |
| --- | --- | --- |
| `art_resource_blocks` | `art_resource_blocks-source.png` | `1cbb61b41a66b3d2a4a62c7032e75d5f01867b4fc4d69864e5ceb17222acca26` |
| `art_resource_bone` | `art_resource_bone-source.png` | `6956215fce64d0a1a53152e777d2dd8106021ad30bc471894b7396526cd1d454` |
| `art_resource_cloth` | `art_resource_cloth-source.png` | `3f93a922d86fbd1b5c837c58b6fe1a7d3a032fbe88f0ed2c7e4ef60b5c38bd20` |
| `art_resource_fibre` | `art_resource_fibre-source.png` | `23953018c4ecb61ebe8cf3a11c58170b657dcc354683b952e64bbadb6b9952e8` |
| `art_resource_fish_habitat` | `art_resource_fish_habitat-source.png` | `470983305c2026feb42d91d671fd3e9c7aadb8ea1fc3679ac766036437e41187` |
| `art_resource_flour` | `art_resource_flour-source.png` | `23953018c4ecb61ebe8cf3a11c58170b657dcc354683b952e64bbadb6b9952e8` |
| `art_resource_grain` | `art_resource_grain-source.png` | `663747b092799c1ca1d105a7b9dd7a545497eb5f70c065139d4073d7dd5f6fce` |
| `art_resource_herbs` | `art_resource_herbs-source.png` | `ed95c2b6a17e0a341bd74e2f62f50929f1e9c845ccca05a4bae6af82a692b4f8` |
| `art_resource_hide` | `art_resource_hide-source.png` | `30f48a7a328c4f08c89031642c5cea32040f9866db710e4154d307515ceeb3e2` |
| `art_resource_leather` | `art_resource_leather-source.png` | `504276665e383e9e31e15a69dd64cf7270040ca113e5e19852ed95e787a0ec6b` |
| `art_resource_logs` | `art_resource_logs-source.png` | `ee160235c115cad965b2b3356f4a0fc9cea21cbe3e565b6f84862d0a831dd636` |
| `art_resource_lumber` | `art_resource_lumber-source.png` | `ed0f0f98f3f9d10b7456cc779f387a35a65176b0d3b07cb0e9570ec15399fde1` |
| `art_resource_metal` | `art_resource_metal-source.png` | `b981b54915e4f2d1295d06e0e314d54e79d8f998dee70ccc52ca72a6fa9f62f0` |
| `art_resource_ore` | `art_resource_ore-source.png` | `58d8423f0a7bb5ad8556082414afc8727efdf9513b2a12f67402b461c362bc05` |
| `art_resource_planks` | `art_resource_planks-source.png` | `655d5f7be0d76308b7a3aa497a3b5ab5391d215ad96098961e07f38412a6c5a5` |
| `art_resource_refined` | `art_resource_refined-source.png` | `1980e6a5e6691e5ee590108340b0ef9457bb6190a5faed497381cb4e1352bcf9` |
| `art_resource_stone` | `art_resource_stone-source.png` | `4013e1e329b0ca4ec7f065d22a5ff056d99d68c2c26d1ba08b19a2de724e4f09` |
| `art_resource_thread` | `art_resource_thread-source.png` | `b57edd481c02df6a1f0e18ded9e1419ca4bef30c78e17e5d0533e746d8c9a10f` |
| `art_resource_water_source` | `art_resource_water_source-source.png` | `233eb48f06d80cafa514b2b6db4192b2e065888734b7a1dd3b0774654d69f580` |

`tmp/imagegen/resources/source-adapted/final-hashes.sha256` is the focused
checksum authority for this nineteen-key subset.

## Generated six-key family

Generation used built-in reference-image mode with:

- `tmp/imagegen/foods/generated-reference-contact.png`
  (`4a9fcfcc8a0b2927a41ad41d5119e2f24673e3acb3bc039e2b124475c171ac17`);
- `tmp/imagegen/foods/basic/final-contact.png`
  (`13178834738afcefad7e6f127751a6e1bccc92d559b69b136d516884cbe2ce66`).

The exact prompt was:

> Create one production sprite atlas for the non-commercial pixel-art game Idle Cat Forest, matching the supplied inventory/resource icons exactly in warm hand-painted pixel style, dark crisp outlines, restrained forest palette, slight three-quarter volume, and strong distinct silhouettes. EXACT layout: 3 columns by 2 rows, six equal cells, one centered isolated resource icon per cell, no overlaps. Cell order left-to-right, top-to-bottom MUST be: 1 Apple Tree resource as a small oak branch with three red apples and green leaves (an inventory icon, not a full world tree), 2 Clay as a moist reddish-brown clay lump with one thumb impression, 3 Fuel as a tied bundle combining dark charcoal pieces and one split firewood stick, 4 Gem as one faceted emerald-green crystal, 5 Sand as a small golden sand pile with a tiny shell, 6 Medicine as a corked green-glass vial beside two healing herb leaves. Each must remain unmistakable at 16x16. Use a perfectly flat solid chroma-key background color #D2FF4D across every unused pixel. No transparency, background texture, scenery, cast shadows outside icons, text, labels, letters, numbers, borders, grid lines, UI frames, cats, hands, duplicate objects, clipped pixels, or extra props. Keep every icon fully inside its cell with generous consistent padding.

The `1774×887` source was normalized to `1776×888`, producing exact
`592×444` cells. Chroma was removed, each cell was contained on a transparent
`64×64` master, and the master was nearest-neighbour reduced to the
registry-native `16×16` final.

| Canonical key | `64×64` master SHA-256 | Exact-final SHA-256 |
| --- | --- | --- |
| `art_resource_apple_tree` | `a6c620f8ab86cb85327febabcab47ff9b41640939318decc46f4ac1db5e53cd6` | `478dcf53d9e35e0f03bad8f90aa733fccf238b3c3529948d206db5e5c9d8ab44` |
| `art_resource_clay` | `23a09fc7dadc3b72b2e3ef7c7e1242928f3e63c19b0ce747a83a854b4e30f481` | `2c3c96a944dad0ebec2adc7b92f5ba0521ed47de9f58f38a6a8852825b07048a` |
| `art_resource_fuel` | `f6cd39f8706a6a680caa9ec663b2ff5d9d8caa72558350a4e5f9e71de51f7d62` | `96679c5d2c506ea730469eb4368dc0d806a6cdde20be02c8fa760e1359b242d4` |
| `art_resource_gem` | `63375ac53fa69e9835a407eecbfcd6a9192ff6de2bfb2d31b13f2af6039d29b0` | `371e6b1a775f1ca370d0c562fe0fa006f7ddf0af1488832ee5c95c2ea5096e7e` |
| `art_resource_sand` | `7f8cf663a648e8a0c693bbbe5c8bc736f136226dcf004bf3df41c576f9f245cb` | `20f37b38c1bfb73c8b31df441ba8b6f32db324a5af4c55ff4f0ff5088acfc59c` |
| `art_resource_medicine` | `33f4c381b397ca24dade92a0e13b9a57f1053f1fc534e17cc7be4a0576bc0cf8` | `20eaab0ef24488b431af1d23e8a6465877f581ae0e96c0d6a6f5aeb420a84b3d` |

## Combined production evidence

| Artifact | SHA-256 |
| --- | --- |
| `tmp/imagegen/resources/generated/source-atlas.png` | `43625dd60cd9653903e6ba2fd51d759be609d02ed7fbb5a136e50b5e7b668b29` |
| `tmp/imagegen/resources/generated/normalized-atlas.png` | `253bc543e079323573a142acfc13e7bdcb47067f4844027475bad7c27f1c8eb6` |
| `tmp/imagegen/resources/final-contact.png` | `3691979bce4b1d86c544b1dda3adfac6d5f77367aa69ce637a3468a9ae0be6d2` |
| `tmp/imagegen/resources/final-all-hashes.sha256` | combined source/artifact/final checksum authority |

All twenty-five production files live at
`assets/planned/content/art_resource_*.png`, are exact `16×16` transparent
sRGBA images, and are positively resolved by exact key. Unknown resources still
fail closed.
