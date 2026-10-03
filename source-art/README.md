# Idle Cat Forest art

This directory holds the original Blender geometry for the Unity migration. The revised kit uses warm timber, cream stone, deep green and blue roofs, and layered forest foliage. Workplaces have low walls and a partial pitched rear roof with overlapping shingles and visible trusses; their open fronts expose tools, inputs and work positions from the management camera. Dens have deeper cottage roofs, cat-shaped gables and five quilted beds. The geometry also supports a camera at cat height.

`idle_cat_forest.blend` is the editable source. Each named collection contains one asset, arranged on a gallery floor for inspection. `build_forest.py` creates the source and all 82 FBX exports with Blender 5.2.1 LTS. The script uses seed `271828`; it needs no addons, downloaded models, texture files or external Python packages.

The separate [world kit](WORLD_KIT.md) adds 16 modular road, dungeon, plant and
creature assets. Its editable source is `world_kit.blend`; `build_world.py`
reproduces and reimports those exports. The combined library contains 98 assets.
Unity generates terrain from saved heights and uses its own included water
shader for depth tint, transparency and small surface ripples.

## Rebuild and inspect

Run these commands from the repository root:

```sh
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_forest.py -- --skip-render
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/verify_fbx.py
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/render_previews.py

# Useful flags
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_forest.py -- --only den,cat
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_forest.py -- --manifests-only
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/verify_fbx.py -- --only den,cat --no-write
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_world.py -- --preview
python3 source-art/tools/geometry_hash.py --write <hashes.json>
python3 source-art/tools/geometry_hash.py --compare <hashes.json>
```

Omit `--skip-render` from the first command to generate a catalog and cat portrait as part of the build. Blender itself is only needed to edit or regenerate art. Unity loads the committed FBX files under `unity/Assets/Resources/ForestArt` without starting Blender.

The source scene includes lighting, a camera and a gallery floor in a preview collection. Those objects do not appear in the FBX exports. The PNG previews show authored geometry; actual Unity screenshots belong to the game validation evidence.

## Scale, orientation and pivots

One Blender unit is one meter. The Blender source uses Z up and the cat faces negative Y. FBX exports use negative Z forward and Y up; Unity imports the cat facing positive Z with Y up. Import at scale 1. Ground pivots stay at the local origin. Roots and rough rocks extend slightly below the ground to avoid floating edges on terrain. Exact source bounds are recorded in `asset-manifest.json`.

The adult cat is 0.8006 m wide, 1.5322 m long including its tail, and 1.367 m tall at the ears; its exported mesh is 2,622 triangles across 19 articulated parts. The kitten is authored at the adult's length (0.8212 m wide, 1.5139 m long, 1.4548 m tall) with a larger head, larger eyes, shorter legs and a stubby tail, in 1,864 triangles; game presentation normalises adults to width 0.85 and kittens to width 0.47. Most workplace foundations span 3.32 by 2.82 m; their roof and front steps extend the usual total bounds to 3.365 by 3.22 m. Equipment may extend farther: the mill's wheel brings its width to 3.7325 m. The oak is 3.8 m tall above ground. The shrine is centered at the origin on a square 2.96 by 2.96 m dais with nine paving bays; its halo reaches 2.8 m. Roads are separate gameplay tiles outside that dais. Building entrances face Blender negative Y, imported as Unity positive Z; the renderer can turn the entire building toward its connected road.

Roads tile at exact 1 m intervals. `fence_rail` contains two rails running from X=-0.5 to X=+0.5, with no posts. `fence_post` is centered at the origin, reaches 0.88 m high, and has 0.228 m wide caps. Place posts on wall tile centers and a rail at each adjacent pair's midpoint; rotate rails around Unity Y for north/south runs. This makes corners join at one post. `fence` remains a complete legacy panel with posts at X=±0.5 m (1.228 m including caps). `gate` is an open entrance with posts at X=±1 m, spanning the gap between the wall nodes beside one gate tile. The clear opening between its posts is 1.82 m; the overhead roof's total bounds are 2.345 by 0.72 m and 2.26 m high. Preserve actual meter scale for the modular fence and gate; normalizing each mesh by its bounds changes the connection spacing. Their pivots are at ground height. Cargo models have their own ground origin; attach them above the cat's back without changing the authoritative inventory.

The cat is a rigid articulated hierarchy rooted at `cat` (an empty at ground origin). Parts are `Body` (root; adult/kitten pivot `(0,0,0)`; torso and bob), `Head` (Body; `(0,-0.31,0.80)` / `(0,-0.3348,0.8208)`; face), `EarL/EarR` (Head; `(+/-0.22,-0.40,1.02)` / `(+/-0.2538,-0.432,1.0368)`; ears), `Jaw` (Head; `(0,-0.59,0.80)` / `(0,-0.6372,0.81)`; jaw), `EyelidL/EyelidR` (Head; `(+/-0.065,-0.505,0.905)` / `(+/-0.06696,-0.40284,0.8964)`; deep eyeball-centre pivots for blink lids), `Tail`, `Tail_1`, `Tail_2`, `Tail_3` (Body, Tail, Tail_1, Tail_2; adult pivots `(0,0.36,0.62)`, `(0.05,0.53,0.72)`, `(0.10,0.65,0.92)`, `(0.14,0.68,1.10)` and kitten pivots `(0,0.324,0.5616)`, `(0.0378,0.4968,0.6372)`, `(0.0702,0.6156,0.7560)`, `(0.0810,0.5940,0.8640)`; tail links), `PawFrontLeft/Right` and `PawBackLeft/Right` (Body; adult shoulders/hips `(+/-0.18,-0.28,0.69)` and `(+/-0.18,0.30,0.69)`, kitten `(+/-0.1836,-0.2484,0.6156)` and `(+/-0.1836,0.2484,0.6156)`; upper legs), and their `LegFrontLeft/RightLower` / `LegBackLeft/RightLower` children (adult lower pivots `(+/-0.18,-0.288,0.455)` / `(+/-0.18,0.312,0.455)`, kitten `(+/-0.1836,-0.2570,0.3672)` / `(+/-0.1836,0.2614,0.3672)`; lower legs and paws). `CarryBack` is an identity socket under Body at adult `(0,0.04,0.86)` / kitten `(0,0.04,0.70)` and `CarryMouth` is an identity socket under Head at adult `(0,-0.78,0.83)` / kitten `(0,-0.74,0.79)`. Left/L labels use negative source X. Parts are baked to identity rotation and unit scale: local X lateral, local Y along the spine with the nose toward -Y, local Z up; yaw Z, pitch X, roll Y, and a Body bob uses the parent local Z. Positive local-X eyelid rotation closes at +70 degrees; the rest cap is rotated up inside the head and the closed shell covers the visible eye with a short dark lower lash. Adult eyeballs use radius 0.225 m with a 0.150 m opening and kitten eyeballs use authored radius 0.270 m (0.292 m after scale) with a 0.205 m opening after scale. The cap uses 20-sided concentric cream, gold and eye circles, a 55% pupil and one outer cream catchlight. Each eyelid is a 0.009 m concentric shell with 60 triangles per lid and a +70 degree local-X close pose; the rest protrusion limit is 0.030 m. Coat slots are `Coat_Base` (main fur), `Coat_Marking` (dark patches), `Coat_White` (bib/muzzle/socks), and `Coat_Point` (ears/legs/points), with runtime materials `Forest_Coat_Base_fur`, `Forest_Coat_Marking_fur_dark`, `Forest_Coat_White_fur_light` and `Forest_Coat_Point_fur`; recipes are tabby (base + marking + white), tuxedo (black base + white), calico (orange/black patches + white), colour point (light base + dark points) and solid (one colour in every slot). The adult asset is 2,622 triangles and the kitten 1,864 triangles, both across 19 articulated mesh renderers; builder assertions enforce adult <=2,900, kitten <=2,000 and each eyelid <=60. The kitten is authored at adult length and rendered at width 0.47 versus adult 0.85. The export wrapper includes socket empties. `world_elder_scarf` attaches under `Head` with identity local pose and natural scale 1; its measured eight-point berry wrap has a narrow flush cream band, offset knot and two cream-striped tails with fringe. Unity root `cat` carries the FBX axis conversion while parts retain source axes; no armature or gameplay state is stored.

All other assets have one joined mesh. Overlapping timber, foliage and stone pieces are intentional. Flat and smooth shading are authored per face, normals are recalculated outwards, and export triangulates the meshes. Runtime colliders should follow gameplay footprints rather than every plank, leaf or stair. Use one simple collider per solid obstacle and keep workplace interaction positions outside those colliders.

## Materials and rendering cost

Materials use plain Principled BSDF base colors, high roughness and no procedural nodes or image textures. Each exported material starts with `Forest_`. `asset-manifest.json` records the named palette and hex values. Unity should share one material per palette entry across imported models. Preserve those colors when mapping to the project's render pipeline; Blender preview lighting and AgX tone mapping are not material data.

The complete library has 145,611 triangles across 82 assets and 88 meshes. The cat has 2,622 triangles across 19 articulated mesh renderers and the kitten has 1,864 triangles across 19; an oak 1,280, a pine 1,788, a den 6,288, a shrine 3,540, a mill 4,772, and a sawmill 5,056. The modular fence post has 134 triangles and its rail has 176; the gate has 2,700. Static meshes retain material submeshes, so one mesh does not mean one draw call. Repeated assets should share meshes and materials, with GPU instancing or Unity batching where appropriate. No asset needs real-time transparency, skeletal skinning, tessellation or shader displacement. The Unity scene owns shadow distance, occlusion, colliders and measured population budgets. The added shingle, furniture and branch geometry increases the complete library by 53,018 triangles; native measurements determine its effect on the active view.

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

## Modular build layout

The old kit is split into `kit/cat.py`, `kit/nature.py`, `kit/buildings_homes.py`,
`kit/buildings_craft.py`, `kit/props.py` and `kit/cargo.py`. Shared geometry,
materials, registration and export helpers live in `kit/helpers.py`. Building
shape helpers live in `kit/buildings_shared.py`, which has no `BUILDERS` table.
Each asset
module exports a `BUILDERS` dictionary. `build_forest.py` discovers every
non-underscore `kit/*.py` in filename order. `helpers.py` and `__init__.py` are
helper-only files. Duplicate names fail with both module names. The committed
canonical order keeps existing gallery and manifest order; new names append after
that order in module and registration order.

World assets use the same pattern in `world/roads.py`, `world/dungeon.py`,
`world/plants.py` and `world/creatures.py`, with world-only code in
`world/helpers.py`. World names must start with `world_`; old-kit names may not.
Adding a module with a `BUILDERS` dictionary is enough for discovery. The build scripts keep export, record measurement, gallery and preview orchestration.
`build_forest.py` and `verify_fbx.py` share the old-kit FBX reimport checker;
`build_world.py` and mixed verification keep the world-specific `build_world.verify` checks.

Each asset gets its own deterministic random stream. The stream starts at the
measured Mersenne Twister word offset in `kit/rng_offsets.json` and
`world/rng_offsets.json`. New assets use a stable name-derived stream.
Builders must draw through `helpers.rng()` (or the world helper's `rng()`) so
adding draws to one asset cannot move another asset. Do not import `RNG`; the
helpers rebind that stream for every asset.

Use `--only` to rebuild selected FBX files without writing a blend, manifest or
verification file. Existing `.meta` files are preserved; a missing one is copied
from `den.fbx.meta` and assigned UUID5 metadata. Use `--manifests-only` to rebuild
source records and verify the FBX files already on disk. `--skip-render` omits old
kit previews. World builds also support `--verify-only`, `--preview-dir DIR` and
`--preview`; preview directories must be outside the repository.

Selected builds write a fresh record and FBX SHA-256 to
`artifacts/art-build/records/<name>.json`. `verify_fbx.py --only a,b` uses a
matching record when present and otherwise uses the committed manifest. Add
`--no-write` to leave `verification.json` untouched. The plain-Python
`tools/geometry_hash.py` fingerprints vertices, faces, materials, transforms,
bounds, triangle counts and vertex counts without Blender.

Every FBX meta file follows Unity's canonical layout from `den.fbx.meta`,
including its eight `key: ` lines with trailing spaces. Non-canonical metadata
causes Unity to dirty the file on import, so new metadata always comes from that
template and existing metadata is never rewritten.
