# Generated item icon receipt

Date: 2026-07-25

Scope: the eighteen exact item, tool, equipment, and container icons generated
for the integrated Leader-AI content manifest. Barrel and Crate are not part of
this atlas: their existing exact source sprites remain the canonical reused
assets.

## Generation contract

The atlas was generated in built-in reference-image mode using:

- `tmp/imagegen/foods/generated-reference-contact.png`
  (`4a9fcfcc8a0b2927a41ad41d5119e2f24673e3acb3bc039e2b124475c171ac17`);
- `tmp/imagegen/foods/basic/final-contact.png`
  (`13178834738afcefad7e6f127751a6e1bccc92d559b69b136d516884cbe2ce66`).

The exact generation prompt was:

> Create one production sprite atlas for the non-commercial pixel-art game Idle Cat Forest, matching the supplied inventory icons exactly in warm hand-painted pixel style, dark crisp outlines, restrained forest/copper/iron palette, slight three-quarter volume, strong distinct silhouettes, and consistent scale. EXACT layout: 6 columns by 3 rows, eighteen equal cells, one centered isolated item icon per cell, no overlaps. Cell order left-to-right, top-to-bottom MUST be: 1 woven Basket, 2 iron-banded wooden Chest, 3 compact wooden storage Rack, 4 Fishing Rod with line and small hook, 5 round glass Lens, 6 brass-and-glass Microscope; 7 Advanced Research Instrument with brass stand, lenses and crystal gauge, 8 sturdy one-handed Weapon as a short sword, 9 practical metal-and-leather Armor cuirass, 10 warm Treated Pelt Clothing coat, 11 translucent scaled Membrane Clothing cloak, 12 wooden Mug; 13 ceramic Bowl, 14 compact carved Furniture stool, 15 general-purpose Tool as a hammer and wrench bundle, 16 small polished Trinket pendant, 17 simple wooden mouse Toy, 18 single fired clay Brick. Each silhouette must remain unmistakable at 64x64, with no repeated generic bag/chest shapes. Use a perfectly flat solid chroma-key background color #D2FF4D across every unused pixel. No transparency, background texture, scenery, cast shadows outside icons, text, labels, letters, numbers, borders, grid lines, UI frames, cats, hands, duplicate items, clipped pixels, or extra objects. Keep every icon fully inside its cell with generous consistent padding.

## Processing and retained artifacts

The generator returned a `1774×887` source atlas. It was normalized to
`1776×888`, providing eighteen exact `296×296` cells in the requested 6-by-3
order. Each cell was isolated, the flat chroma background was removed, content
was contained within a transparent square canvas, and the result was
nearest-neighbour reduced to a `64×64` sRGBA master. Those masters are
preserved under `tmp/imagegen/items/64px-masters/`.

The closed art registry is authoritative and requires `16×16 icon`.
Therefore every master was nearest-neighbour reduced to an exact `16×16` sRGBA
production file under `assets/planned/items/`. All production corners are
transparent and every nontransparent bounding box remains inside the canvas.
The brown contact-sheet background is inspection-only.

| Artifact | Purpose | SHA-256 |
| --- | --- | --- |
| `tmp/imagegen/items/source-atlas.png` | retained generator output | `5e5a4af596a3e0bd51acb8f9ed52cc50100d2de4d113e2faaeaf01216b5a758a` |
| `tmp/imagegen/items/normalized-atlas.png` | exact-grid extraction source | `2a8a6759f0c640f4241042182ac9ab2f9ca6fe7ad8ff3d140366eac72a4d3fd1` |
| `tmp/imagegen/items/final-contact.png` | ordered `64×64` master-stage contact | `63d35918f876fd5ac07f4356f6b6fa2c2f64264755eb6a028045c944447e6141` |
| `tmp/imagegen/items/final-native-contact.png` | exact-final `16×16` contact in lexicographic key order | `764f8b1e5b9816d6975aa475255497c305cf77b6b427e9c01644589624342dd2` |

The consolidated exact-final checksum authority is
`tmp/imagegen/native-size-final-hashes.sha256`.

## Production outputs

All files below are exact-final `16×16` sRGBA PNGs under
`assets/planned/items/`. Their `64×64` masters remain under
`tmp/imagegen/items/64px-masters/`.

| Canonical art key | SHA-256 |
| --- | --- |
| `art_item_basket` | `f3ad0dcf79728b5f3d469f90ee313e9c257f9dfe6cc8302b22bc666741623944` |
| `art_item_chest` | `aa8a0a93920105040f7c466f272168b65aa1cd45b70e5408550c78e2e1a47275` |
| `art_item_rack` | `d6612183163f7e92dd59de27b8b8154628629dc3029f002a7d3ec8358da50210` |
| `art_item_fishing_rod` | `93e336d0e517403908a9df9b06a5e9a0aa7eb1de9dbcdc8150c9298aafe29ca1` |
| `art_item_lens` | `262d70fb7bed2d722df63f3866581b2c082161f24834d513091ea415a317c021` |
| `art_item_microscope` | `25f9ca99fc0e4ebb42da5f9a08d98ab9ea6fd226ecfc3a1e035d6f6fa65c5ee0` |
| `art_item_advanced_instrument` | `2047dadb46d89e39d06d85570c6d4372a7f07af2f6e1399e56058e336dfbb90b` |
| `art_item_weapon` | `fa156c576046181645bc70acd026b483241de3ce073cb14daa64332478db582c` |
| `art_item_armor` | `78e14d03443cb6a6259ecbc60a703a4a986c1464f37aabba36fea191e744639e` |
| `art_item_treated_pelt_clothing` | `6f193b6ef43dc2b7563ffb3e331d0389e3d5d079c4196270c9ec0d1579b97ae7` |
| `art_item_membrane_clothing` | `145377eb1d64d2a72e45bd98e55cb0a05419fe150e562d89f1fafbc2c2e8b64a` |
| `art_item_mug` | `65ed07d0150ec23de441db083613cc73f7374cb4694bb15d836e790b547421f1` |
| `art_item_bowl` | `e719917628ed206faa46cdef1104ac6398317d9c4f1500a4be0956b85a412b59` |
| `art_item_furniture` | `82b610944b1f7a688db30f89dbaee31242b5fcf6090ce24c855c2ac88ffd0e41` |
| `art_item_generic_tool` | `5332b8e59df4376b1d601a2737b9aaf870bf06df344f0de6e4ec5ba4a09dcd23` |
| `art_item_trinket` | `ed8bb577decbd96a5c1bb351ef87d5dd79a3a506d5cce20ae55f8504e8a147fa` |
| `art_item_toy` | `369f1325da3575b86546e2eed53c0df82e15fe43ed8f22cef375ffe3d50bc65a` |
| `art_item_brick` | `b114bcf79bdedc82601d204adf6af8fc6ffb22e1489b85ed881bf4a149b7e51e` |

## Existing-source containers

These two canonical item keys remain source reuse, not generated output:

| Canonical art key | Existing source path | Native size |
| --- | --- | --- |
| `art_item_barrel` | `public/images/game/props/barrel.png` | `16×16` |
| `art_item_crate` | `public/images/game/props/crate.png` | `16×16` |

## Integration note

The positive allow-list and production files now agree with the authoritative
registry's exact `16×16` native dimensions. The generated `64×64` images are
masters only and must never be reported as runtime-native assets. The registry
still names `assets/planned/content/<key>.png` while the positive resolver uses
`assets/planned/items/`; that path reconciliation remains separate from native
dimension correctness.
