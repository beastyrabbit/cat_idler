"""Build, export, and verify the modular world art kit."""

import argparse
import hashlib
import importlib
import json
import math
import re
import sys
import tempfile
import uuid
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "source-art"
EXPORT = ROOT / "unity/Assets/Resources/ForestArt"
MANIFEST = ART / "world-asset-manifest.json"
RECORDS = ROOT / "artifacts/art-build/records"
SEED = 161803
sys.dont_write_bytecode = True
if str(ART) not in sys.path:
    sys.path.insert(0, str(ART))
forest = importlib.import_module("kit.helpers")
H = importlib.import_module("world.helpers")


def discover():
    """Load builder modules and apply the canonical order."""
    builders = {}
    modules = {}
    for path in sorted((ART / "world").glob("*.py")):
        if path.name.startswith("_") or path.name in {"helpers.py", "__init__.py"}:
            continue
        module = importlib.import_module(f"world.{path.stem}")
        for name, builder in getattr(module, "BUILDERS", {}).items():
            if name in builders:
                raise RuntimeError(f"duplicate asset {name}: {modules[name]} and {path.name}")
            if not name.startswith("world_"):
                raise RuntimeError(f"world asset must start with world_: {name}")
            builders[name] = builder
            modules[name] = path.name
    missing = [name for name in H.CANONICAL_ORDER if name not in builders]
    if missing:
        raise RuntimeError("canonical world assets missing: " + ", ".join(missing))
    canonical = [name for name in H.CANONICAL_ORDER if name in builders]
    extras = [name for name in builders if name not in H.CANONICAL_ORDER]
    extras.sort(key=lambda name: (modules[name], list(builders).index(name)))
    return {name: builders[name] for name in canonical + extras}


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.world = bpy.data.worlds.new("World kit sky")
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.scene.unit_settings.scale_length = 1


def _points(objects):
    return H.points(objects)


def _bounds(vertices):
    return H.bounds(vertices)


def _prepare(name, entry, write):
    """Join a temporary export copy, measure it, and optionally write FBX."""
    source = [obj for obj in entry["collection"].objects if obj.type == "MESH"]
    root = bpy.data.objects.new(name, None)
    entry["collection"].objects.link(root)
    root["contract"] = json.dumps(entry["contract"], sort_keys=True)
    for obj in source:
        obj.parent = root
    entry["root"] = root
    copies = []
    for obj in source:
        duplicate = obj.copy()
        duplicate.data = obj.data.copy()
        duplicate.parent = None
        duplicate.matrix_world = obj.matrix_world.copy()
        bpy.context.scene.collection.objects.link(duplicate)
        copies.append(duplicate)
    joined = forest.merge_group(name + "_mesh", copies)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.mesh.quads_convert_to_tris(quad_method="BEAUTY", ngon_method="BEAUTY")
    bpy.ops.object.mode_set(mode="OBJECT")
    joined.parent = root
    verts = _points([joined])
    record = {
        "description": entry["description"],
        "contract": entry["contract"],
        "vertices": len(joined.data.vertices),
        "triangles": len(joined.data.polygons),
        "mesh_count": 1,
        "editable_part_count": len(source),
        "bounds_blender": _bounds(verts),
        "bounds_unity": _bounds([H.unity(point) for point in verts]),
        "pivot_unity": [0, 0, 0],
        "materials": sorted({material.name for material in joined.data.materials}),
    }
    record["dimensions_unity_m"] = [round(b - a, 6) for a, b in zip(*record["bounds_unity"], strict=True)]
    if write:
        bpy.ops.object.select_all(action="DESELECT")
        root.select_set(True)
        joined.select_set(True)
        bpy.context.view_layer.objects.active = root
        bpy.ops.export_scene.fbx(
            filepath=str(EXPORT / f"{name}.fbx"),
            use_selection=True,
            global_scale=1,
            apply_unit_scale=True,
            apply_scale_options="FBX_SCALE_UNITS",
            axis_forward="-Z",
            axis_up="Y",
            object_types={"EMPTY", "MESH"},
            use_mesh_modifiers=True,
            mesh_smooth_type="FACE",
            use_triangles=True,
            add_leaf_bones=False,
            bake_anim=False,
            path_mode="AUTO",
            embed_textures=False,
        )
    bpy.data.objects.remove(joined, do_unlink=True)
    if write:
        _ensure_meta(name)
    record["guid"] = uuid.uuid5(uuid.NAMESPACE_URL, f"idle-cat-forest/art/{name}.fbx").hex
    return record


def _ensure_meta(name):
    meta_path = EXPORT / f"{name}.fbx.meta"
    guid = uuid.uuid5(uuid.NAMESPACE_URL, f"idle-cat-forest/art/{name}.fbx").hex
    if meta_path.exists():
        match = re.search(r"^guid: ([0-9a-f]{32})$", meta_path.read_text(), re.MULTILINE)
        if not match or match.group(1) != guid:
            raise RuntimeError(f"refusing to replace unrelated GUID in {meta_path}")
        return
    template = (EXPORT / "road.fbx.meta").read_text()
    meta_path.write_text(re.sub(r"^guid: [0-9a-f]{32}$", f"guid: {guid}", template, flags=re.MULTILINE))


def _assert_close(actual, expected, label, tolerance=0.001):
    if len(actual) != len(expected) or any(abs(a - b) >= tolerance for a, b in zip(actual, expected, strict=True)):
        raise AssertionError((label, actual, expected))


def _check_roads(name, uv, lo, hi, size, record, objects):
    checks = []
    _assert_close([lo[0], hi[0], lo[2], hi[2]], [-0.5, 0.5, -0.5, 0.5], f"{name} grid")
    if hi[1] >= 0.065:
        raise AssertionError((name, "tall shoulder"))
    ports = record["contract"]["ports_unity"]
    for port, axis, edge in (("+X", 0, 0.5), ("-X", 0, -0.5), ("+Z", 2, 0.5), ("-Z", 2, -0.5)):
        paved = [vertex for vertex in uv if abs(vertex[axis] - edge) < 0.0001 and abs(vertex[1] - 0.045) < 0.0001]
        if bool(paved) != (port in ports):
            raise AssertionError((name, "incorrect paved port", port))
        if paved:
            across = 2 if axis == 0 else 0
            _assert_close(
                [min(vertex[across] for vertex in paved), max(vertex[across] for vertex in paved)],
                [-0.34, 0.34],
                name + port,
            )
    checks += [
        "exact 1 m grid bounds",
        "all requested ports and no extra ports",
        "matching 0.68 m paving width and 0.045 m top",
        "low shoulders",
    ]
    return checks


def _check_stairs(name, uv, lo, hi, size, record, objects):
    checks = []
    _assert_close(lo, [-0.5, -4, 0], "stair lower bounds")
    _assert_close(hi, [0.5, 0, 4], "stair upper bounds")
    obj = objects[0]
    inverse = obj.matrix_world.inverted()
    for index in range(16):
        hit, local, _, _ = obj.ray_cast(inverse @ Vector((0, -(index + 0.5) * 0.25, 1)), Vector((0, 0, -1)))
        if not hit or abs((obj.matrix_world @ local).z + index * 0.25) >= 0.001:
            raise AssertionError((name, "wrong tread", index))
    checks += [
        "16 actual horizontal treads",
        "4 m run along Unity +Z",
        "4 m descent",
        "top stair origin",
        "1 m width",
    ]
    return checks


def _check_entrance(name, uv, lo, hi, size, record, objects):
    checks = []
    _assert_close([size[0], hi[1], lo[1]], [3, 3.5, 0], "entrance envelope")
    obj = objects[0]
    inverse = obj.matrix_world.inverted()
    for x in (-0.82, 0, 0.82):
        for z in (0.05, 1.2, 2.55):
            hit, _, _, _ = obj.ray_cast(inverse @ Vector((x, -2, z)), Vector((0, 1, 0)), distance=4)
            if hit:
                raise AssertionError((name, "blocked entrance opening", x, z))
    checks += ["3 m width", "3.5 m height", "nine rays pass through the real opening"]
    return checks


def _check_wall_pillar(name, uv, lo, hi, size, record, objects):
    checks = []
    expected = [1, 2.5, 0.32] if name.endswith("wall") else [0.35, 2.5, 0.35]
    _assert_close(size, expected, f"{name} dimensions")
    if abs(lo[1]) >= 0.001:
        raise AssertionError((name, "not grounded"))
    checks += ["exact modular dimensions", "ground pivot"]
    return checks


def _check_floor(name, uv, lo, hi, size, record, objects):
    checks = []
    _assert_close(lo, [-0.5, -0.08, -0.5], name + " lower")
    _assert_close(hi, [0.5, 0, 0.5], name + " upper")
    checks += ["exact 1 m grid", "flat y=0 slab tops"]
    return checks


def _check_beetle(name, uv, lo, hi, size, record, objects):
    checks = []
    _assert_close([size[0], size[2], lo[1]], [0.9, 1.25, 0], name + " silhouette")
    checks += ["0.9 m width", "1.25 m length", "ground pivot"]
    return checks


CONTRACT_CHECKERS = {
    "world_road_": _check_roads,
    "world_dungeon_stairs": _check_stairs,
    "world_dungeon_entrance": _check_entrance,
    "world_dungeon_wall": _check_wall_pillar,
    "world_dungeon_pillar": _check_wall_pillar,
    "world_dungeon_floor": _check_floor,
    "world_cave_beetle": _check_beetle,
}


def check_contract(name, verts, record, objects):
    uv = [H.unity(vertex) for vertex in verts]
    lo, hi = H.bounds(uv)
    size = [b - a for a, b in zip(lo, hi, strict=True)]
    for prefix, check in CONTRACT_CHECKERS.items():
        if name.startswith(prefix):
            return check(name, uv, lo, hi, size, record, objects)
    return []


def verify(manifest):
    """Reimport each FBX and verify generic metadata and world contracts."""
    from io_scene_fbx import parse_fbx

    original_scene = bpy.context.scene
    scene = bpy.data.scenes.new("FBX verification only")
    bpy.context.window.scene = scene
    for name, record in manifest["assets"].items():
        for obj in list(scene.objects):
            bpy.data.objects.remove(obj, do_unlink=True)
        path = EXPORT / f"{name}.fbx"
        if not path.exists():
            raise FileNotFoundError(path)
        document, _ = parse_fbx.parse(str(path))
        settings = next(element for element in document.elems if element.id == b"GlobalSettings")
        properties = next(element for element in settings.elems if element.id == b"Properties70")
        axes = {element.props[0].decode(): element.props[-1] for element in properties.elems}
        if axes.get("UpAxis") != 1 or axes.get("UpAxisSign") != 1:
            raise AssertionError((name, "FBX Y up"))
        if axes.get("FrontAxis") != 2 or axes.get("FrontAxisSign") != 1:
            raise AssertionError((name, "FBX -Z forward metadata", axes))
        if abs(axes.get("UnitScaleFactor", 0) - 100) >= 0.001:
            raise AssertionError((name, "FBX meter units"))
        bpy.ops.import_scene.fbx(filepath=str(path), use_anim=False)
        bpy.context.view_layer.update()
        objects = [obj for obj in scene.objects if obj.type == "MESH"]
        if len(objects) != 1:
            raise AssertionError((name, "mesh count"))
        obj = objects[0]
        verts = _points(objects)
        if not all(math.isfinite(coordinate) for point in verts for coordinate in point):
            raise AssertionError((name, "nonfinite vertex"))
        triangles = sum(len(poly.vertices) - 2 for poly in obj.data.polygons)
        if triangles != record["triangles"]:
            raise AssertionError((name, "triangles", triangles, record["triangles"]))
        actual_bounds = _bounds(verts)
        for actual, expected in zip(actual_bounds, record["bounds_blender"], strict=True):
            _assert_close(actual, expected, name + " bounds")
        _assert_close(list(obj.matrix_world.translation), [0, 0, 0], name + " mesh pivot")
        _assert_close(list(obj.matrix_world.to_scale()), [1, 1, 1], name + " scale")
        roots = [obj for obj in scene.objects if obj.parent is None]
        if len(roots) != 1 or roots[0].type != "EMPTY":
            raise AssertionError((name, "root hierarchy"))
        _assert_close(list(roots[0].matrix_world.translation), [0, 0, 0], name + " root pivot")
        actual_materials = {re.sub(r"\.\d{3}$", "", material.name) for material in obj.data.materials if material}
        if actual_materials != set(record["materials"]):
            raise AssertionError((name, "materials", actual_materials))
        for material in obj.data.materials:
            key = re.sub(r"\.\d{3}$", "", material.name).removeprefix("Forest_")
            color = forest.PALETTE[key]
            expected = [int(color[index : index + 2], 16) / 255 for index in (0, 2, 4)]
            _assert_close(list(material.diffuse_color[:3]), expected, name + " material color")
        contract_checks = check_contract(name, verts, record, objects)
        unity_bounds = _bounds([H.unity(point) for point in verts])
        record["verification"] = {
            "passed": True,
            "triangles": triangles,
            "mesh_count": 1,
            "bounds_blender": actual_bounds,
            "bounds_unity": unity_bounds,
            "dimensions_unity_m": [round(hi - lo, 6) for lo, hi in zip(*unity_bounds, strict=True)],
            "pivot_unity": list(H.unity(obj.matrix_world.translation)),
            "scale": [round(value, 6) for value in obj.matrix_world.to_scale()],
            "fbx_axes": {
                key: axes[key] for key in ("UpAxis", "UpAxisSign", "FrontAxis", "FrontAxisSign", "UnitScaleFactor")
            },
            "contract_checks": contract_checks,
        }
    for obj in list(scene.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    bpy.context.window.scene = original_scene
    bpy.data.scenes.remove(scene)
    manifest["verification"] = {
        "passed": True,
        "asset_count": len(manifest["assets"]),
        "method": "Actual FBX reimport in Blender; no Unity run",
        "checks": [
            "FBX axes and units",
            "finite geometry",
            "triangle preservation",
            "one mesh per export",
            "source/reimport bounds",
            "origin pivots",
            "unit transforms",
            "palette materials",
            "asset geometry contracts",
        ],
    }


def gallery():
    for index, (name, entry) in enumerate(H.ASSETS.items()):
        entry["root"].location = ((index % 5) * 4.6, (index // 5) * 6.1, 4 if name.endswith("stairs") else 0)
    collection = bpy.data.collections.new("Inspection only - never exported")
    bpy.context.scene.collection.children.link(collection)
    forest.CURRENT = collection
    forest.box("Gallery floor", (9.1, 9.2, -0.16), (24, 26, 0.2), "soil", edge=0)
    bpy.ops.object.camera_add(location=(25, -30, 33))
    camera = bpy.context.object
    camera.name = "World kit management inspection"
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 31
    forest.aim(camera, (9, 8.7, 1))
    bpy.context.scene.camera = camera
    bpy.ops.object.light_add(type="SUN", location=(0, -5, 12))
    bpy.context.object.rotation_euler = (0.3, -0.5, -0.4)
    bpy.context.object.data.energy = 2.3
    bpy.context.object.data.angle = 0.3
    bpy.ops.object.light_add(type="AREA", location=(6, -6, 15))
    bpy.context.object.data.energy = 1800
    bpy.context.object.data.shape = "DISK"
    bpy.context.object.data.size = 14
    forest.aim(bpy.context.object, (8, 8, 0))
    scene = bpy.context.scene
    scene.world.color = (0.35, 0.4, 0.45)
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = True
    scene.render.resolution_x = 1800
    scene.render.resolution_y = 1500
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = "AgX"
    bpy.ops.object.select_all(action="DESELECT")


def preview_directory(directory):
    """Resolve and reject repository paths before any build or preview work."""
    directory = Path(directory).resolve()
    repository = ROOT.resolve()
    if directory == repository or repository in directory.parents:
        raise argparse.ArgumentTypeError("preview directory must be outside the repository")
    return directory


def previews(directory):
    directory = preview_directory(directory)
    directory.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    scene.render.filepath = str(directory / "world-kit.png")
    bpy.ops.render.render(write_still=True)
    scene.camera.location = (8, -9, 11)
    scene.camera.data.ortho_scale = 7
    forest.aim(scene.camera, (4.6, 0, 0))
    scene.render.filepath = str(directory / "roads.png")
    bpy.ops.render.render(write_still=True)
    scene.camera.location = (3.7, 0.8, 3)
    scene.camera.data.ortho_scale = 4.8
    forest.aim(scene.camera, (0, 6.1, 1.7))
    scene.render.filepath = str(directory / "entrance.png")
    bpy.ops.render.render(write_still=True)


def _write_record(name, record):
    path = EXPORT / f"{name}.fbx"
    RECORDS.mkdir(parents=True, exist_ok=True)
    record = dict(record)
    record["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    (RECORDS / f"{name}.json").write_text(json.dumps(record, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--only")
    parser.add_argument("--manifests-only", action="store_true")
    parser.add_argument("--preview-dir", type=preview_directory)
    parser.add_argument("--preview", action="store_true")
    options = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [])
    if options.preview and options.preview_dir:
        raise SystemExit("--preview and --preview-dir are mutually exclusive")
    if options.preview:
        options.preview_dir = tempfile.mkdtemp(prefix="world-kit-")
    if options.verify_only and (options.only or options.manifests_only):
        raise SystemExit("--verify-only cannot be combined with --only or --manifests-only")
    reset()
    if options.verify_only:
        manifest = json.loads(MANIFEST.read_text())
        verify(manifest)
        print("WORLD_ART_VERIFIED " + json.dumps(manifest.get("totals", {}), sort_keys=True))
        return
    builders = discover()
    selected = [name for name in (options.only.split(",") if options.only else builders) if name]
    unknown = [name for name in selected if name not in builders]
    if unknown:
        raise SystemExit("Unknown assets: " + ", ".join(unknown) + "; valid names: " + ", ".join(builders))
    H.ASSETS.clear()
    forest.ASSETS.clear()
    for name in selected:
        builders[name]()
    if options.only:
        for name in selected:
            record = _prepare(name, H.ASSETS[name], write=True)
            _write_record(name, record)
        verify({"assets": {name: json.loads((RECORDS / f"{name}.json").read_text()) for name in selected}})
        print(f"WORLD_ART_ONLY {len(selected)} assets")
        return
    assets = {name: _prepare(name, H.ASSETS[name], write=not options.manifests_only) for name in selected}
    manifest = {
        "generator": "source-art/build_world.py",
        "source": "source-art/world_kit.blend",
        "blender": bpy.app.version_string,
        "seed": SEED,
        "unit": "meter",
        "source_up": "+Z",
        "source_forward": "-Y",
        "fbx_forward": "-Z",
        "fbx_up": "+Y",
        "unity_up": "+Y",
        "unity_forward": "+Z",
        "unity_import_scale": 1,
        "consumer_naturalScale": True,
        "textures": 0,
        "provenance": (
            "Original project geometry; no downloaded assets, images, paid services or additional asset licenses"
        ),
        "assets": assets,
    }
    verify(manifest)
    used = {material.removeprefix("Forest_") for entry in assets.values() for material in entry["materials"]}
    manifest["palette_srgb_hex"] = {key: forest.PALETTE[key] for key in sorted(used)}
    manifest["totals"] = {
        "assets": len(assets),
        "meshes": len(assets),
        "triangles": sum(entry["triangles"] for entry in assets.values()),
        "vertices": sum(entry["vertices"] for entry in assets.values()),
        "shared_materials": len(used),
        "fbx_bytes": sum((EXPORT / f"{name}.fbx").stat().st_size for name in assets),
    }
    gallery()
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(ART / "world_kit.blend"), compress=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    if options.preview_dir:
        previews(options.preview_dir)
        print("WORLD_PREVIEW_DIR " + str(Path(options.preview_dir).resolve()))
    print("WORLD_ART_VERIFIED " + json.dumps(manifest["totals"], sort_keys=True))


if __name__ == "__main__":
    main()
