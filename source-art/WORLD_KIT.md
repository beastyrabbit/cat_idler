# Forest world kit

`build_world.py` authors 16 independent assets. It safely imports the geometry helpers and warm timber, muted stone and moss palette from `build_forest.py`; it never calls the old kit's builder. All geometry is original project artwork, with no textures, downloaded models, paid services or new asset license obligations. Distribution follows the repository owner's project terms.

## Build status

Blender 5.2.1 LTS generated and reimported all 16 assets successfully. The kit contains 16 meshes, 10,145 triangles, 6,256 vertices and 20 shared materials. The FBX files total 561,632 bytes. `world_kit.blend` retains editable geometry and `world-asset-manifest.json` records measured bounds and individual checks. Unity imported the kit and built the ARM64 player. Native checks covered joined roads, the entrance, physical stair descent, cutaway chambers and lake depths in management and direct control. The 30/150-cat measurements are in `docs/unity/PERFORMANCE.md`.

## Rebuild and inspect

From the repository root:

```sh
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_world.py
/opt/homebrew/bin/blender --background --python-exit-code 1 --python source-art/build_world.py -- --verify-only
```

The default build exports only `unity/Assets/Resources/ForestArt/world_*.fbx` and matching `.meta` files, reimports every FBX, then writes `world_kit.blend` and `world-asset-manifest.json` after verification passes. The script does not write old kit files, Unity scenes or game saves. Importer metadata copies the existing road importer's schema and uses deterministic UUID5 GUIDs, global scale 1 and file units. It refuses to replace a world asset's unrelated GUID.

For optional Blender inspection renders, append `-- --preview-dir /tmp/world-kit-preview` to the build command. Images stay outside the repository. The editable source retains named component objects in one collection per asset, arranged in a gallery with a management camera and lighting. The exporter joins temporary copies into one mesh per FBX. Repeated runtime instances should share those meshes and the `Forest_` materials. Submeshes still contribute draw calls.

## Placement contracts

One Blender unit is one meter. Sources use Z up and negative Y forward. FBX exports use negative Z forward and Y up. Expected Unity coordinates are `(source.x, source.z, -source.y)`. Every export has an origin pivot and unit scale. Consumers must use `naturalScale=true`; normalization by bounds would break road, wall, floor and stair joins.

| FBX stem | Contract in Unity meters |
| --- | --- |
| `world_road_straight` | Exact 1 × 1 footprint; connects +Z and -Z |
| `world_road_corner` | Exact 1 × 1 footprint; connects +Z and +X |
| `world_road_junction` | Exact 1 × 1 footprint; connects -X, +X and +Z |
| `world_road_cross` | Exact 1 × 1 footprint; connects all four directions |
| `world_road_end` | Exact 1 × 1 footprint; connects +Z |
| `world_dungeon_entrance` | 3 wide, 3.5 high; timber-braced rock arch with a real opening; front +Z |
| `world_dungeon_wall` | 1 long along X, 0.32 thick, 2.5 high; ground pivot and no roof |
| `world_dungeon_pillar` | 0.35 × 0.35 footprint, 2.5 high; ground pivot |
| `world_dungeon_stairs` | 1 wide; 16 treads of 0.25 run and rise; +Z descent from `(0,0,0)` to `(0,-4,4)` |
| `world_dungeon_floor` | Exact 1 × 1 footprint; irregular slabs have flat tops at Y=0; underside Y=-0.08 |
| `world_crystals` | Faceted teal cluster on slate; dungeon and highland dressing |
| `world_mushrooms` | Five russet caps with pale stems; damp forest and dungeon dressing |
| `world_tree_pine` | Ragged tiered needles, visible branches and roots; pine forest and highland dressing |
| `world_tree_birch` | Pale forked trunk, bark marks and light clustered leaves; groves and meadow edges |
| `world_reeds` | Bent blades and brown seed heads; marsh and riverbank dressing |
| `world_cave_beetle` | 0.9 wide, 1.25 long, ground pivot, faces +Z; static six-legged enemy mesh |

Road boundaries meet at ±0.5. Each connected edge has a 0.68 m paved width, a top at Y=0.045 and the same three-stone joint pattern. Irregular paving occupies the interior. Soil starts below ground and rises to Y=0.006; moss and sparse grass form low shoulders. There is no tall continuous curb. Rotate a complete road module around its origin to select another connection orientation.

The stair origin is the leading edge of the top tread. Tread `i`, numbered from zero, spans Z=`i/4` through `(i+1)/4` at Y=`-i/4`. The final riser ends at Z=4, Y=-4. Place a lower floor tile centered at Z=4.5 and Y=-4. The underside stays inside the 4 m descent envelope. Geometry is solid, with actual horizontal treads and vertical risers. Gameplay owns route validity, floor visibility and collision; the mesh does not authorize movement between levels.

The entrance check samples a clear opening 1.64 m wide and 2.55 m high. Dungeon walls have exposed stone tops for cutaway views. The parent controls visibility of upper levels; the assets contain no floor, spawn, terrain or combat logic. The beetle has split teal armor, amber eyes, hooked jaws and six separate modeled legs. It has no rig or animation clips.

## Verification contract

The generated manifest records measured dimensions in both coordinate systems, origin pivots, triangles, vertices, editable part counts, shared materials, GUIDs and actual FBX reimport results.

Reimport validation checks FBX axis metadata and meter units, finite vertices, triangle counts, one mesh per export, source bounds, pivots, unit transforms, palette names and colors. Asset checks cover all road ports and absent ports, paving width and height, exact modular bounds, entrance clearance rays, every stair tread height and the beetle footprint. Blender verification complements the parent's Unity import and camera checks.
