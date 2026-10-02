"""Shared Blender FBX reimport checks for the old art kit."""

import json
import math
import re
from pathlib import Path

import bpy


def _reimport(name, expected, export_dir):
    """Reimport one FBX and validate its measured geometry and materials."""
    path = Path(export_dir) / f"{name}.fbx"
    if not path.exists():
        raise RuntimeError(f"Missing FBX for {name}")
    bpy.ops.import_scene.fbx(filepath=str(path))
    bpy.context.view_layer.update()
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    expected_meshes = expected.get("mesh_count", 1)
    if len(meshes) != expected_meshes:
        raise RuntimeError(f"{name}: mesh count changed ({len(meshes)} != {expected_meshes})")
    vertices = [obj.matrix_world @ vertex.co for obj in meshes for vertex in obj.data.vertices]
    if not vertices or not all(math.isfinite(value) for vertex in vertices for value in vertex):
        raise RuntimeError(f"{name}: non-finite or empty vertices")
    triangles = sum(sum(len(face.vertices) - 2 for face in obj.data.polygons) for obj in meshes)
    if triangles != expected["triangles"]:
        raise RuntimeError(f"{name}: triangles changed ({triangles} != {expected['triangles']})")
    actual = [
        [min(vertex[i] for vertex in vertices) for i in range(3)],
        [max(vertex[i] for vertex in vertices) for i in range(3)],
    ]
    if "bounds_blender" in expected and any(
        abs(actual[axis][index] - expected["bounds_blender"][axis][index]) > 0.001
        for axis in range(2)
        for index in range(3)
    ):
        raise RuntimeError(f"{name}: bounds, pivot or orientation changed")
    for obj in meshes:
        if not obj.data.materials or any(material is None for material in obj.data.materials):
            raise RuntimeError(f"{name}: missing material")
        if name.startswith("world_") and any(
            not material.name.startswith("Forest_") for material in obj.data.materials
        ):
            raise RuntimeError(f"{name}: non Forest material")
    if name == "cat":
        actual_parts = {re.sub(r"\.\d{3}$", "", obj.name) for obj in meshes}
        if actual_parts != set(expected.get("parts", [])):
            raise RuntimeError("cat: articulation names changed")
        head = next((obj for obj in meshes if re.sub(r"\.\d{3}$", "", obj.name) == "Head"), None)
        if head is None or head.location.y >= 0 or head.location.z <= 0.5:
            raise RuntimeError("cat: head pivot changed")
        if any(abs(obj.matrix_world.determinant()) <= 0.99 for obj in meshes):
            raise RuntimeError("cat: scaling changed")
    if name.startswith("world_"):
        roots = [obj for obj in bpy.context.scene.objects if obj.type == "EMPTY"]
        if not any(obj.matrix_world.translation.length < 0.001 for obj in roots):
            raise RuntimeError(f"{name}: origin root missing")
    return {"asset": name, "triangles": triangles, "mesh_count": len(meshes), "passed": True}


def verify_existing(names, expected, export_dir, report_path=None):
    """Verify FBXs in a temporary scene and optionally write the shared report."""
    original_scene = bpy.context.scene
    scene = bpy.data.scenes.new("Forest FBX verification only")
    bpy.context.window.scene = scene
    results = []
    try:
        for name in names:
            for obj in list(scene.objects):
                bpy.data.objects.remove(obj, do_unlink=True)
            results.append(_reimport(name, expected[name], export_dir))
    finally:
        for obj in list(scene.objects):
            bpy.data.objects.remove(obj, do_unlink=True)
        bpy.context.window.scene = original_scene
        bpy.data.scenes.remove(scene)
    if report_path is not None:
        write_verification(results, report_path)
    return results


def write_verification(results, report_path):
    """Write the verified inventory only after every FBX passed."""
    report = {
        "blender": bpy.app.version_string,
        "asset_count": len(results),
        "passed": True,
        "checks": [
            "FBX reimport",
            "triangle preservation",
            "mesh count",
            "finite vertices",
            "meter scale",
            "orientation and bounds",
            "material assignment",
            "cat articulation names and pivots",
        ],
        "results": results,
    }
    Path(report_path).write_text(json.dumps(report, indent=2) + "\n")
