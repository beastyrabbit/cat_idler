# Shrine Asset and Layered-Sprite Transfer Receipt

**Recorded:** 2026-07-25  
**Source worktree:** `/home/beasty/orca/workspaces/cat_idler/the-shrine-upgrade`  
**Source branch/head:** `the-shrine-upgrade` / `25b5b27943d31f420984bde80faae57be1041706`  
**Source state:** untracked leaves and untracked image assets in the frozen source working tree  
**Manifest source freeze (retained):** `b1bcc2433d29d23f10167de07465f4c39a7164bc782d9ec292fa8cafe3a4bdaf`  
**Observed source snapshot at transfer:** `f4ccc839f46d40090cbc385b8c2909e5d86c4c39d12fac55d8f4dfe028772907` (82 files)  
**Observed source asset digest at transfer:** `9158e3dea02c5f0bff7fca355d23984f1ea0dc10c66c63a8269abb2a4b6d723c` (53 files)  
**Manifest authority:** [source-transfer-manifest.md](source-transfer-manifest.md), Shrine asset and layered-sprite rows  
**Plan authority:** LAI.49 and LAI.68; Plan 1 §11 asset deliverables; Plan 2 world-rendering/state-sheet requirements.

## Scope and disposition

### Source-drift audit

The manifest's retained source freeze and `public/images` domain digest
(`f6752d02e2883aa4dc60d7a7483bea8c20e7e9368a52e3d001daccf6b9d5780b`) do not match the
current source worktree when recomputed with its stated sorted
`<sha256><two spaces><relative path>` algorithm. The current source still exposes exactly the
same 82 working paths and exactly the same 53 manifest-listed image paths, but their aggregate
digests are recorded above. No source was edited, reset, cleaned, or otherwise changed. This
receipt preserves the manifest snapshot, records the exact bytes actually transferred, and leaves
the unexplained historical-byte delta for the branch-plan coordinator to audit; it does not silently
replace the manifest record.

This is a semantic leaf transfer, not a merge. The 53 manifest-listed PNGs were absent from the
target before transfer, were copied without transformation, and retain their source-relative paths
and SHA-256 bytes. `crates/cat-client/src/layered_sprite.rs` is an **adapted** reusable foundation:
its deterministic composition, visibility predicates, exact signatures, per-owner reconciliation,
pixel-to-world geometry, and Bevy 0.19 sprite helpers come from the source leaf, while its target
documentation records the target's repository-root asset configuration and its deliberate lack of
renderer-system registration. The focused static characterization test is similarly adapted only
with target/provenance documentation.

The target export makes the foundation available but does **not** bind it to LAI.68, register a
system, create art keys, or change `lai68.rs`. The LAI.68 owner remains the sole owner of concrete
world rendering and state projection.

| Source path | Source state | Source SHA-256 | Target path | Disposition | Provenance / extracted knowledge |
|---|---|---|---|---|---|
| `crates/cat-client/src/layered_sprite.rs` | untracked | `ade92bb4f5622f20e7a0d39f112dbb85d2a9c5c38379ff97cd727212b8ec64e4` | `crates/cat-client/src/layered_sprite.rs` (`827def87449e8d0058a733ff3b5b699e43001e8db50e4f3ac4c01f3872b03bb7`) | adapt | `VisualOwner`, `LayerSlot`, predicates/state, validated canvas/parts, exact signature/reconcile plan, Bevy 0.19 image/sprite/transform helpers. |
| `crates/cat-client/tests/layered_sprite.rs` | untracked | `233e94e8a907c84a72e63fdff047f829802862d18f9d36c5cd11b7853c633cef` | `crates/cat-client/tests/layered_sprite.rs` (`32199ba9b1c11aa189b6ce33dd9d96d887b8ca041569fd343e0d036fc8e78dcd`) | adapt | Stable order, signature no-op/rebuild scope, flag visibility, geometry conversion, tie-break, and invalid-canvas characterization. |
| `crates/cat-client/src/lib.rs` | modified | `032f21620182fa84f9fe6139b44f6edcdbe6613237da525947791eda2b8be502` | `crates/cat-client/src/lib.rs` | adapt | Source's modified export/use site establishes the module boundary. Target carries only `pub mod layered_sprite;`; no source imports, renderer/plugin, or system wiring were copied into the hot root. |

## Binary asset receipts

Every source asset below was untracked in the frozen Shrine working tree. The target path is
identical to the source-relative path; `copy` means a byte-for-byte `cp` after confirming the
target did not exist. SHA-256 was verified after copying.

| Source path | Target path | SHA-256 | Disposition |
|---|---|---|---|
| `public/images/game/buildings/black-hole.png` | `public/images/game/buildings/black-hole.png` | `8e8ae7aaa116ae853e9ba2fbc63f802d9959398af4a02e96dbb6d6334e5c45c3` | copy |
| `public/images/game/buildings/black-hole/base.png` | `public/images/game/buildings/black-hole/base.png` | `8e8ae7aaa116ae853e9ba2fbc63f802d9959398af4a02e96dbb6d6334e5c45c3` | copy |
| `public/images/game/buildings/black-hole/width-01.png` | `public/images/game/buildings/black-hole/width-01.png` | `068100ee573d66b9e1087f8d366b0c61a877bac1002e300a9b4333216527be4e` | copy |
| `public/images/game/buildings/black-hole/width-02.png` | `public/images/game/buildings/black-hole/width-02.png` | `0502e92a8f5390ea2209c4a936f89fbf60cb68c3aad9fee038ea27c7fac68f35` | copy |
| `public/images/game/buildings/black-hole/width-03.png` | `public/images/game/buildings/black-hole/width-03.png` | `5dfe294d466f8105ca040603cda92b11c2d00109fff5a36cef78aad189a4dad0` | copy |
| `public/images/game/buildings/black-hole/width-04.png` | `public/images/game/buildings/black-hole/width-04.png` | `59a9ef0c854532d32caac93598063f31e8e3f53d3d1f2d339e051878a2e9d2e8` | copy |
| `public/images/game/buildings/black-hole/width-05.png` | `public/images/game/buildings/black-hole/width-05.png` | `8889b77263a7445f41822699e03e0ff861526dd6c02ff57247ed72ee2edaef9c` | copy |
| `public/images/game/buildings/black-hole/width-06.png` | `public/images/game/buildings/black-hole/width-06.png` | `e586fe3efa1db0fc9c7a9e6890f1fbbe7bfd8a9e1b1af1e4ffcee54468223031` | copy |
| `public/images/game/buildings/black-hole/width-07.png` | `public/images/game/buildings/black-hole/width-07.png` | `66c44c6c7313c6baf87044b9b326e462bcbc8543b522176d5ca429e80db60313` | copy |
| `public/images/game/buildings/black-hole/width-08.png` | `public/images/game/buildings/black-hole/width-08.png` | `85a39a439ccaabb29606d0ccea9b9b9a1f622a8bb6bb88b1bf3887175125022c` | copy |
| `public/images/game/buildings/black-hole/width-09.png` | `public/images/game/buildings/black-hole/width-09.png` | `69e3388099bdc83d93a560c666ff5920b196a3835437264eeee29e3c5c1d45b3` | copy |
| `public/images/game/buildings/black-hole/width-10.png` | `public/images/game/buildings/black-hole/width-10.png` | `7d3417502d5628b1f220dbc4913a6f0cf0943c58be4c948c7e5ed0c9e443819b` | copy |
| `public/images/game/buildings/black-hole/depth-01.png` | `public/images/game/buildings/black-hole/depth-01.png` | `c67be19523ca6a155365f8208ec621751d0ff4078960438aafacdb03a6da23ce` | copy |
| `public/images/game/buildings/black-hole/depth-02.png` | `public/images/game/buildings/black-hole/depth-02.png` | `268c8ca0cf0ca1f2867fcad0d8a53f704821724b8971e2bce350901975c29d62` | copy |
| `public/images/game/buildings/black-hole/depth-03.png` | `public/images/game/buildings/black-hole/depth-03.png` | `8418da539302b6fc009ff7b2614c4760fd3d3d1c5199da6f8f13e7907285b74a` | copy |
| `public/images/game/buildings/black-hole/depth-04.png` | `public/images/game/buildings/black-hole/depth-04.png` | `e6deded383c4434e2684d6682c5433685eb4b1f7929574441fdbff2a98685cbf` | copy |
| `public/images/game/buildings/black-hole/depth-05.png` | `public/images/game/buildings/black-hole/depth-05.png` | `a7c164e60549e445d3e02228320c6cd92858322cb1ef9f82e9e29d4e6293cbbc` | copy |
| `public/images/game/buildings/black-hole/depth-06.png` | `public/images/game/buildings/black-hole/depth-06.png` | `e29b6e7486c2c7e64200bf0aeeea8a1211880ae8f4d0a767abe667c0f8f85722` | copy |
| `public/images/game/buildings/black-hole/depth-07.png` | `public/images/game/buildings/black-hole/depth-07.png` | `af47c9e8aad7b15db78f7856f8a2695e821be4d1a0c0c1a95c4dae84bb3b396d` | copy |
| `public/images/game/buildings/black-hole/depth-08.png` | `public/images/game/buildings/black-hole/depth-08.png` | `009d60f7bf0a9e1304064557fbf2d2598130caf8ce19185aab31cd900a835839` | copy |
| `public/images/game/buildings/black-hole/depth-09.png` | `public/images/game/buildings/black-hole/depth-09.png` | `09b7748ba25df9332c55984f6fbaab690d242c71abe7696dd0df5c0084412d51` | copy |
| `public/images/game/buildings/black-hole/depth-10.png` | `public/images/game/buildings/black-hole/depth-10.png` | `31ad7c4f39db20ccbad1ede925fb382858d441750795ee07567ca1ab8d041538` | copy |
| `public/images/game/buildings/black-hole/darkness-01.png` | `public/images/game/buildings/black-hole/darkness-01.png` | `514e43a8d9559a2899878055ac4fb7013a2ff905ab0bc60b9fc5ed14863256cb` | copy |
| `public/images/game/buildings/black-hole/darkness-02.png` | `public/images/game/buildings/black-hole/darkness-02.png` | `52fe892faca1b54378ee25c1ee0274f7a87faa7c7f67aa6c768de2380d6c8165` | copy |
| `public/images/game/buildings/black-hole/darkness-03.png` | `public/images/game/buildings/black-hole/darkness-03.png` | `39a24c767fffacf55765564c715fb7eca08f7c474127e3a628d93e3a9cfe236c` | copy |
| `public/images/game/buildings/black-hole/darkness-04.png` | `public/images/game/buildings/black-hole/darkness-04.png` | `4d887c242a29e8d41ad8968c89538479cb7e176e3a146da3780f32b239cfbcb9` | copy |
| `public/images/game/buildings/black-hole/darkness-05.png` | `public/images/game/buildings/black-hole/darkness-05.png` | `0eff2bf9cd92fe36a8389c6b2afc759cebfa64306cc56fb2613502a4dfe42477` | copy |
| `public/images/game/buildings/black-hole/darkness-06.png` | `public/images/game/buildings/black-hole/darkness-06.png` | `389e59465a413ccab8809f2b63a789dbcde647da74c0766274e94ec538ce025a` | copy |
| `public/images/game/buildings/black-hole/darkness-07.png` | `public/images/game/buildings/black-hole/darkness-07.png` | `443009e52df3ebab0bc19af7a573bcb68bb5a4701270802ed86fd1378e7b7e75` | copy |
| `public/images/game/buildings/black-hole/darkness-08.png` | `public/images/game/buildings/black-hole/darkness-08.png` | `22d1837f83174e67cd926eaa1245be86c4ea38ed755f02b460f7e04324682250` | copy |
| `public/images/game/buildings/black-hole/darkness-09.png` | `public/images/game/buildings/black-hole/darkness-09.png` | `4c1dfa5694bcb9ab78572179b0f227668843963870061cd65dd3fa323db7ccf6` | copy |
| `public/images/game/buildings/black-hole/darkness-10.png` | `public/images/game/buildings/black-hole/darkness-10.png` | `1acd08774144405ecc7287287f8cde3779306622c5475bcfed416d06bdbd54c4` | copy |
| `public/images/game/farm/dynamic/catnip-sprout.png` | `public/images/game/farm/dynamic/catnip-sprout.png` | `3be246d7f3b5e1d623b590f9804c8b68f8ee4ac2be08371fa85cbc8a8d87eea0` | copy |
| `public/images/game/farm/dynamic/catnip-growing.png` | `public/images/game/farm/dynamic/catnip-growing.png` | `b3f4ea17314b86bf143ea2f9fe03ff2feca907c6fb267d930cd809eb41d7d085` | copy |
| `public/images/game/farm/dynamic/catnip-flowering.png` | `public/images/game/farm/dynamic/catnip-flowering.png` | `329c95bcbc8b8fc19b9c991eb2189a5f9dcf08cd3d8fd192248d4e5355b281d2` | copy |
| `public/images/game/farm/dynamic/catnip-mature.png` | `public/images/game/farm/dynamic/catnip-mature.png` | `650e32d46ff609b8cb6cd6deda8b692da47dcaf322997498ffe517d404f9e3d2` | copy |
| `public/images/game/farm/dynamic/grain-sprout.png` | `public/images/game/farm/dynamic/grain-sprout.png` | `9800c9f84bca3d3f9ef5344684095fb844bfeea6dbc6e4c3f16ada32399b75a7` | copy |
| `public/images/game/farm/dynamic/grain-growing.png` | `public/images/game/farm/dynamic/grain-growing.png` | `4f67c0a47d43100b2b6e658b8fde69f7999afe3a3f23fd0128efd5717f782ae7` | copy |
| `public/images/game/farm/dynamic/grain-flowering.png` | `public/images/game/farm/dynamic/grain-flowering.png` | `7fd81d0f341b6949334e59bf58a417e14923bc7dea18d42e48a65670404e9493` | copy |
| `public/images/game/farm/dynamic/grain-mature.png` | `public/images/game/farm/dynamic/grain-mature.png` | `8242869d34e6ed69092262cff83f331ffe615a1d7d57a696083371b000a86d94` | copy |
| `public/images/game/farm/dynamic/herb-sprout.png` | `public/images/game/farm/dynamic/herb-sprout.png` | `97e7859a345f7af0e02ea36b61f668c1117f5c2d346180b765f6e8fa483dd411` | copy |
| `public/images/game/farm/dynamic/herb-growing.png` | `public/images/game/farm/dynamic/herb-growing.png` | `4867e218b10d75cdad669bacda0401b48bf7d0d2674856a3931a9f4e3cf9cbd6` | copy |
| `public/images/game/farm/dynamic/herb-flowering.png` | `public/images/game/farm/dynamic/herb-flowering.png` | `f4050254c173248d9937a19a0f9a31477efbed395d00ef88867228dd803284df` | copy |
| `public/images/game/farm/dynamic/herb-mature.png` | `public/images/game/farm/dynamic/herb-mature.png` | `32004bf7d57a080da775b923f79577327ef28a183c4a2dc5c10b8356665e576a` | copy |
| `public/images/game/nature/tree_oak_apples_low.png` | `public/images/game/nature/tree_oak_apples_low.png` | `c6d276e16d9b6b96ffc8b7c6d1e9f1a163408346206fd9e27404b73ddc59ce78` | copy |
| `public/images/game/nature/tree_oak_apples_mid.png` | `public/images/game/nature/tree_oak_apples_mid.png` | `46ea9bf145ce362f450c9a617c53e58637ebda43d342947c579a83bb20667872` | copy |
| `public/images/game/nature/tree_oak_apples_full.png` | `public/images/game/nature/tree_oak_apples_full.png` | `4cb7c4edc4d8bcb5d22deb1d9424187d01727cbf04267af80c150ef3e1ac780c` | copy |
| `public/images/game/sites/lair.png` | `public/images/game/sites/lair.png` | `7a329b100a2b72e60b15afc97bfc0b11ae242058449ebcd56d35176f725736d3` | copy |
| `public/images/game/sites/quarry.png` | `public/images/game/sites/quarry.png` | `a5044ef5f0bec606555081476883a580e1b276d1479e4437da0999aa0207aaf6` | copy |
| `public/images/game/transport/boat.png` | `public/images/game/transport/boat.png` | `27a9ccbe14660af6bde87d7a5ed6a7f7941426e300f5fd8507f0ef0dfb867d59` | copy |
| `public/images/game/transport/dock_land.png` | `public/images/game/transport/dock_land.png` | `639fdaf3926f3fb6abe5f51bdf62281434d67b9374e7b8d2f324107fa645329a` | copy |
| `public/images/game/transport/dock_water.png` | `public/images/game/transport/dock_water.png` | `e81a23ce01650547b4d7624a0ec271b56b83e1c695315d188948eba1ed222c4a` | copy |
| `public/images/game/transport/rail_cart.png` | `public/images/game/transport/rail_cart.png` | `b3fcb064f28052246f68674be2aaedd1e92bb1eb4d3f9cc9d969a149925bbdd2` | copy |

## Deliberate gaps retained for the owning cards

- The source set contains one general `sites/lair.png`; it does **not** contain the required ten
  lair level-band world sprites. LAI.49/LAI.68 must author, validate, map, and render those ten
  band-specific assets rather than treating this transfer as a substitute.
- The source set also lacks the required twenty creature portraits, twenty named-material icons,
  Cookhouse state sheet, four Fishing-Hut orientations/idle-working states, empty Apple-tree state,
  food/tool/fixture/augmentation/quality assets, task markers, and the required art-key,
  transparency/bounds, accessibility, zoom screenshot, native/WASM, restart/despawn evidence.
- No target renderer has been changed. The copied files and generic composition primitives are
  inputs to later authoritative art-key/state mapping, not evidence that a snapshot field currently
  selects an asset.
- Per the bounded transfer instruction, no Cargo/test/browser/full-format run was performed. Only
  owned Rust files were `rustfmt`-formatted; `git diff --check` passed immediately afterward.
