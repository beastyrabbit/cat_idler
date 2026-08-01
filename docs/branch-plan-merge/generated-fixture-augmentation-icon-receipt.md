# Generated fixture and augmentation icon receipt

Date: 2026-07-25

Scope: six exact construction fixtures and four exact item/research
augmentations for the integrated Leader-AI content manifest. These are physical
fit-out or attachable-detail icons, never whole buildings, generic equipment,
Shrine/Favor imagery, or hidden-state indicators.

## Generation contract

The ten-icon atlas was produced in built-in reference-image mode using:

- `tmp/imagegen/foods/generated-reference-contact.png`
  (`4a9fcfcc8a0b2927a41ad41d5119e2f24673e3acb3bc039e2b124475c171ac17`);
- `tmp/imagegen/items/final-contact.png`
  (`63d35918f876fd5ac07f4356f6b6fa2c2f64264755eb6a028045c944447e6141`),
  which was the pre-native-correction `64×64` item-family contact at generation
  time.

The exact prompt was:

> Create one production sprite atlas for the non-commercial pixel-art game Idle Cat Forest, matching the supplied inventory icons exactly in warm hand-painted pixel style, dark crisp outlines, restrained forest/copper/iron palette, slight three-quarter volume, and strong distinct silhouettes. EXACT layout: 5 columns by 2 rows, ten equal cells, one centered isolated fixture or augmentation icon per cell, no overlaps. Cell order left-to-right, top-to-bottom MUST be: 1 Cookhouse Fixture as an iron cooking grate with cauldron hook, 2 Fishing Hut Fixture as a compact rope winch with folded net, 3 Workshop Fixture as a sturdy iron bench vise, 4 Research Fixture as a calibrated brass lens stand, 5 Storage Fixture as a compact labeled shelf insert; 6 Hole Fixture as a non-religious obsidian void-stabilizing ring with subtle purple darkness, 7 Weapon Augmentation as a fang-and-metal blade charm, 8 Armor Augmentation as a fitted iridescent scale plate inlay, 9 Tool Augmentation as a precision gear and reinforced grip assembly, 10 Research Instrument Augmentation as a crystal lens and brass focusing assembly. These are small attachable parts, not whole buildings or full weapons. Each must remain unmistakable at 16x16. Use a perfectly flat solid chroma-key background color #D2FF4D across every unused pixel. No transparency, background texture, scenery, cast shadows outside icons, text, labels, letters, numbers, borders, grid lines, UI frames, cats, hands, shrines, religious symbols, duplicate items, clipped pixels, or extra objects. Keep every icon fully inside its cell with generous consistent padding.

## Processing and retained artifacts

The generator returned a `1774×887` source atlas. It was normalized to
`1775×888`, providing ten exact `355×444` cells in the required 5-by-2 order.
Each cell was isolated, the flat chroma background was removed, content was
contained within a transparent square canvas, and a `64×64` sRGBA master was
created. The ten masters are retained in
`tmp/imagegen/fixtures-augmentations/` with the suffix `-master.png`.

The closed registry's authoritative `ui_detail` production target is `32×32`.
Each master was nearest-neighbour reduced to an exact transparent sRGBA
production file under
`assets/planned/fixtures/` or `assets/planned/augmentations/`. Every production
corner is transparent and every nontransparent bound remains inside its
canvas. The brown contact background is inspection-only.

The prompt's “unmistakable at 16x16” phrase is a legibility request, not a
native-dimension authority; the registry-owned shipped size is `32×32`.

| Artifact | Purpose | SHA-256 |
| --- | --- | --- |
| `tmp/imagegen/fixtures-augmentations/source-atlas.png` | retained generator output | `ef79eaf7a5733453741b74598870d2de25b80f4e32baf9bda593d949f516ea55` |
| `tmp/imagegen/fixtures-augmentations/normalized-atlas.png` | exact-grid extraction source | `9c414ac161bf68cf931180eae799c880f664f02d3a0a2369cbd74415fe254dd0` |
| `tmp/imagegen/fixtures-augmentations/final-contact.png` | all ten exact-final `32×32` icons in prompt order | `1e38ab4056436ec01cb621a32cf6c3b7052071d8122e249d1f88cdee61845a26` |
| `tmp/imagegen/fixtures-augmentations/final-32px-hashes.sha256` | exact-final checksum authority | checksums listed below |

## Preserved `64×64` masters

| Canonical art key | Master SHA-256 |
| --- | --- |
| `art_fixture_cookhouse` | `2babade1852644d5fc2394e4674af6b57b12252756d59392106ba8afb26763e8` |
| `art_fixture_fishing_hut` | `dc6e1e999646f32208789ca435784e75422b8d825dea0f70a59bf482484ef734` |
| `art_fixture_workshop` | `cde5985aa47a4f4cc4678b86b876f42e024369ccbfda653969a7f18c44d77620` |
| `art_fixture_research` | `a036747c405eb5e0c34999a701c7c0085cbb9f33c16cfa03345ccfc58a587d85` |
| `art_fixture_storage` | `ffd940e659c56cd3c95e8b354948666ccc19eed811dc630c754f1e2f3d441ebf` |
| `art_fixture_black_hole` | `6cf454ab1797fbaa48130259b404743e8716644e77fe18e5ee1e48e438fa1224` |
| `art_augmentation_weapon` | `ea98d696eb00f433efa820ee25705077dadc4b519b6cecfc03d4358222b6ae7a` |
| `art_augmentation_armor` | `b701ffae26e123eed4f4f325765c61119d140a478c844ba67bbbbb520c60298d` |
| `art_augmentation_tool` | `32118a187d8a5ffe9176ea6fbb3bf6ba04eb8d70b8331a77cad3473549a5f7e8` |
| `art_augmentation_research` | `bbb4de775f37efe6b2f552c7f35ae8f0113b4810135a73c9f64b2907f191559e` |

## Exact-final production outputs

All files below are exact-registry `32×32` sRGBA PNGs.

| Canonical art key | Production path | SHA-256 |
| --- | --- | --- |
| `art_fixture_cookhouse` | `assets/planned/fixtures/art_fixture_cookhouse.png` | `a4492e42afd4393ba724eef86181f974ad091677aa9f2389b727e6e1ffcb6368` |
| `art_fixture_fishing_hut` | `assets/planned/fixtures/art_fixture_fishing_hut.png` | `dae847d4500213e1a5857b688b4243abc24ad314817e2b040aa66e150bbd2485` |
| `art_fixture_workshop` | `assets/planned/fixtures/art_fixture_workshop.png` | `d11dec13b1e0a591c1117aa7ebbba3cc5c79f6131a63ca6612c59b301dbb6c4a` |
| `art_fixture_research` | `assets/planned/fixtures/art_fixture_research.png` | `45e93600f5f19ac012e00887c2766fba62f36cc2e72b737d92be8ed114f20edc` |
| `art_fixture_storage` | `assets/planned/fixtures/art_fixture_storage.png` | `dbdf93faa3fe3dfbea0ce8bedf5d89d14ef41ab89187b56e2e43f06ff39dfd24` |
| `art_fixture_black_hole` | `assets/planned/fixtures/art_fixture_black_hole.png` | `ba49e96db25068668a388a5ee5813c5602211810bde518cb8ed868b2b123253f` |
| `art_augmentation_weapon` | `assets/planned/augmentations/art_augmentation_weapon.png` | `34c173c44169584742e0b56ce227d3b4df8e053c3503f1985892435d4f7bd058` |
| `art_augmentation_armor` | `assets/planned/augmentations/art_augmentation_armor.png` | `26db8955c026407101a155ace0b55fd25072b208795913990685023dbcf02752` |
| `art_augmentation_tool` | `assets/planned/augmentations/art_augmentation_tool.png` | `7939a6055fc02dfeea43d81646d1737c2c927137c2d07b88f3b5d75ab79476b5` |
| `art_augmentation_research` | `assets/planned/augmentations/art_augmentation_research.png` | `bc1b0879da25c8792a0c51cc03c89bf016cd97018447baf601c338b4d341d1ad` |

## Integration note

The positive resolver exposes only these ten exact keys and actual production
paths. Unknown or category-ambiguous keys remain unresolved. The `64×64`
images are masters only; runtime-native `ui_detail` dimensions are `32×32`.
