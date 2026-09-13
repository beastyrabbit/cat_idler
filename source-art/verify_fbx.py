"""Reimport each FBX and verify exported geometry against the Blender manifest."""

import json
import math
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
manifest = json.loads((ROOT / "source-art/asset-manifest.json").read_text())
results = []
for name, expected in manifest["assets"].items():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.ops.import_scene.fbx(filepath=str(ROOT / "unity/Assets/Resources/ForestArt" / (name + ".fbx")))
    bpy.context.view_layer.update()
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    assert len(meshes) == expected["mesh_count"], (name, "mesh count")
    triangles = sum(sum(len(face.vertices) - 2 for face in obj.data.polygons) for obj in meshes)
    assert triangles == expected["triangles"], (name, triangles, expected["triangles"])
    verts = [obj.matrix_world @ vert.co for obj in meshes for vert in obj.data.vertices]
    assert all(math.isfinite(value) for vert in verts for value in vert), (name, "invalid coordinate")
    actual = [[min(v[i] for v in verts) for i in range(3)], [max(v[i] for v in verts) for i in range(3)]]
    assert all(abs(actual[a][i] - expected["bounds_blender"][a][i]) < 0.001
               for a in range(2) for i in range(3)), (name, "changed orientation, pivot or size", actual)
    assert all(len(obj.data.materials) > 0 and all(mat is not None for mat in obj.data.materials)
               for obj in meshes), (name, "missing material")
    if name == "cat":
        assert {obj.name for obj in meshes} == set(expected["parts"]), "Cat articulation names changed"
        head = bpy.data.objects["Head"]
        assert head.location.y < 0 and head.location.z > 0.5, "Cat front/pivot convention changed"
        assert all(abs(obj.matrix_world.determinant()) > 0.99 for obj in meshes), "Cat scaling changed"
    results.append({"asset": name, "triangles": triangles, "mesh_count": len(meshes), "passed": True})
report = {"blender": bpy.app.version_string, "asset_count": len(results), "passed": True,
          "checks": ["FBX reimport", "triangle preservation", "mesh count", "finite vertices", "meter scale",
                     "orientation and bounds", "material assignment", "cat articulation names and pivots"],
          "results": results}
(ROOT / "source-art/verification.json").write_text(json.dumps(report, indent=2) + "\n")
print("FOREST_ART_VERIFIED " + str(len(results)) + " FBX assets")
