# Idle Cat Forest art

This directory holds the original Blender geometry for the Unity migration. The revised kit uses warm timber, cream stone, deep green and blue roofs, and layered forest foliage. Workplaces have low walls and a partial pitched rear roof with overlapping shingles and visible trusses; their open fronts expose tools, inputs and work positions from the management camera. Dens have deeper cottage roofs, cat-shaped gables and five quilted beds. The geometry also supports a camera at cat height.

`idle_cat_forest.blend` is the editable source. Each named collection contains one asset, arranged on a gallery floor for inspection. `build_forest.py` creates the source and all 82 FBX exports with Blender 5.2.1 LTS. The script uses seed `271828`; it needs no addons, downloaded models, texture files or external Python packages.

## Rebuild and inspect

Run these commands from the repository root:

```sh
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_forest.py -- --skip-render
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/verify_fbx.py
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/render_previews.py
```

Omit `--skip-render` from the first command to generate a catalog and cat portrait as part of the build. Blender itself is only needed to edit or regenerate art. Unity loads the committed FBX files under `unity/Assets/Resources/ForestArt` without starting Blender.

The source scene includes lighting, a camera and a gallery floor in a preview collection. Those objects do not appear in the FBX exports. The PNG previews show authored geometry; actual Unity screenshots belong to the game validation evidence.

## Scale, orientation and pivots

One Blender unit is one meter. The Blender source uses Z up and the cat faces negative Y. FBX exports use negative Z forward and Y up; Unity imports the cat facing positive Z with Y up. Import at scale 1. Ground pivots stay at the local origin. Roots and rough rocks extend slightly below the ground to avoid floating edges on terrain. Exact source bounds are recorded in `asset-manifest.json`.

The cat is 0.7822 m wide, 1.5578 m long including its tail, and 1.34 m tall at the ears. Most workplace foundations span 3.32 by 2.82 m; their roof and front steps extend the usual total bounds to 3.365 by 3.22 m. Equipment may extend farther: the mill's wheel brings its width to 3.7325 m. The oak is 3.8 m tall above ground. The shrine is centered at the origin on a square 2.96 by 2.96 m dais with nine paving bays; its halo reaches 2.8 m. Roads are separate gameplay tiles outside that dais. Building entrances face Blender negative Y, imported as Unity positive Z; the renderer can turn the entire building toward its connected road.

Roads tile at exact 1 m intervals. `fence_rail` contains two rails running from X=-0.5 to X=+0.5, with no posts. `fence_post` is centered at the origin, reaches 0.88 m high, and has 0.228 m wide caps. Place posts on wall tile centers and a rail at each adjacent pair's midpoint; rotate rails around Unity Y for north/south runs. This makes corners join at one post. `fence` remains a complete legacy panel with posts at X=±0.5 m (1.228 m including caps). `gate` is an open entrance with posts at X=±1 m, spanning the gap between the wall nodes beside one gate tile. The clear opening between its posts is 1.82 m; the overhead roof's total bounds are 2.345 by 0.72 m and 2.26 m high. Preserve actual meter scale for the modular fence and gate; normalizing each mesh by its bounds changes the connection spacing. Their pivots are at ground height. Cargo models have their own ground origin; attach them above the cat's back without changing the authoritative inventory.

The cat has seven mesh transforms: `Body`, `Head`, `Tail`, `PawFrontLeft`, `PawFrontRight`, `PawBackLeft` and `PawBackRight`. Head and tail origins sit at their attachment points; paw origins sit at the upper legs. The left/right labels refer to the cat's left/right. Animate local rotations relative to the imported bind transforms. The kit uses rigid procedural articulation and needs no armature, skin weights or animation controller. The model does not contain gameplay state.

All other assets have one joined mesh. Overlapping timber, foliage and stone pieces are intentional. Flat and smooth shading are authored per face, normals are recalculated outwards, and export triangulates the meshes. Runtime colliders should follow gameplay footprints rather than every plank, leaf or stair. Use one simple collider per solid obstacle and keep workplace interaction positions outside those colliders.

## Materials and rendering cost

Materials use plain Principled BSDF base colors, high roughness and no procedural nodes or image textures. Each exported material starts with `Forest_`. `asset-manifest.json` records the named palette and hex values. Unity should share one material per palette entry across imported models. Preserve those colors when mapping to the project's render pipeline; Blender preview lighting and AgX tone mapping are not material data.

The complete library has 145,611 triangles across 82 assets and 88 meshes. A cat has 2,572 triangles and seven mesh renderers, an oak 1,280, a pine 1,788, a den 6,288, a shrine 3,540, a mill 4,772, and a sawmill 5,056. The modular fence post has 134 triangles and its rail has 176; the gate has 2,700. Static meshes retain material submeshes, so one mesh does not mean one draw call. Repeated assets should share meshes and materials, with GPU instancing or Unity batching where appropriate. No asset needs real-time transparency, skeletal skinning, tessellation or shader displacement. The Unity scene owns shadow distance, occlusion, colliders and measured population budgets. The added shingle, furniture and branch geometry increases the complete library by 53,018 triangles; native measurements determine its effect on the active view.

## Asset mapping

The 25 maintained building identifiers map directly to FBX filenames:

| Gameplay identifier | Visible equipment or contents |
| --- | --- |
| `den` | Five quilted nests, deeper cottage roof and cat gable |
| `food_storage` | Open berry crates and tied sacks |
| `water_bowl` | Terracotta spring bowl, stones and spout |
| `beds` | Three low framed nests |
| `herb_garden` | Raised rows of blue-green herbs |
| `nursery` | Nests, hanging mobile and kitten toys |
| `elder_corner` | Nests, book and herb bowl |
| `walls` | Sharpened timber palisade |
| `mouse_farm` | Straw pen and modeled mice |
| `shrine` | Nine-bay square dais, seated cat statue, sun halo and offerings |
| `workshop` | Mortar, preparation bench and preserve jars |
| `field` | Grain rows and scarecrow |
| `smithy` | Anvil, forge and tool rack |
| `barracks` | Weapon rack and straw training targets |
| `accounting_tent` | Ledgers, counting board and storage crate |
| `wood_cutter` | Chopping stump, axe and cut logs |
| `stone_prep` | Dressed blocks, quarry rocks and chisel |
| `woodworking` | Planing bench, planks and chair assembly |
| `clothier` | Warp-thread loom, cloth, wheel and spools |
| `tannery` | Drying hide, scraping bench and tanning bowl |
| `research_hut` | Books, desk and globe |
| `smelter` | Tall furnace, chimney and metal ingots |
| `mill` | Millstones, grain hopper, drive wheel and flour sacks |
| `sawmill` | Toothed saw, feed rails, drive wheel and logs |
| `school` | Chalkboard, books and small desks |

Vegetation and deposits are `tree_oak`, `tree_pine`, `shrub`, `berry_bush`, `stump`, `rock`, `ore_iron`, `ore_coal`, `grain_plot`, `herb_plot`, `catnip_plot` and `reeds`. Supporting props are `stockpile`, `road`, `fence`, `fence_post`, `fence_rail`, `gate`, `rail`, `dock`, `cart`, `boat`, `watchtower` and `healing_tent`.

The 32 cargo assets use the prefix `cargo_` followed by the maintained carrying identifier: `food`, `fish`, `blessings`, `materials`, `stone`, `refined`, `logs`, `lumber`, `planks`, `blocks`, `tools`, `water`, `catnip`, `grain`, `flour`, `preserves`, `medicine`, `brew`, `herbs`, `hide`, `leather`, `fibre`, `thread`, `cloth`, `bone`, `ore`, `gem`, `clay`, `sand`, `metal`, `weapons` and `armor`. Their silhouettes distinguish sacks, bundles, jars, boards, rocks, ingots, cloth, tools and fish. Similar shapes retain separate material colors.

## Provenance and verification

All geometry, materials and modeling scripts were authored for this repository during the Unity migration. No purchased, downloaded or AI image-generation assets were used. Blender exports are original project artwork and add no third-party asset license obligations. Blender is licensed under GPL; its license does not apply to artwork created with it. Distribution of this artwork follows the repository owner's project terms. This directory grants no separate license to third parties.

`verify_fbx.py` reimports every exported model in Blender and fails if triangle counts, mesh counts, finite coordinates, scale, orientation, bounds, material assignments or cat articulation names change. The resulting `verification.json` records the checked inventory. This check complements Unity's import and scene validation; it does not replace testing the management and cat-control views in the running game.
