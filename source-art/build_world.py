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
    contract = entry["contract"]
    parts = contract.get("parts")
    if parts:
        by_name = {}
        for obj in source:
            base = re.sub(r"\.\d{3}$", "", obj.name)
            if base in parts:
                if base in by_name:
                    raise RuntimeError(f"{name}: duplicate source part {base}")
                by_name[base] = obj
        missing = [part for part in parts if part not in by_name]
        if missing or len(by_name) != len(parts) or len(source) != len(parts):
            raise RuntimeError(f"{name}: source parts mismatch; missing={missing}, objects={[obj.name for obj in source]}")
        source = [by_name[part] for part in parts]
        for part, obj in zip(parts, source, strict=True):
            obj.name = f"{name} {part} source"
    root = bpy.data.objects.new(name, None)
    entry["collection"].objects.link(root)
    root["contract"] = json.dumps(contract, sort_keys=True)
    for obj in source:
        obj.parent = root
    entry["root"] = root
    copies = []
    for part, obj in zip(parts or [None] * len(source), source, strict=True):
        duplicate = obj.copy()
        duplicate.data = obj.data.copy()
        duplicate.name = part if parts else obj.name
        duplicate.parent = None
        duplicate.matrix_world = obj.matrix_world.copy()
        bpy.context.scene.collection.objects.link(duplicate)
        if parts:
            bpy.context.view_layer.objects.active = duplicate
            duplicate.select_set(True)
            bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
            duplicate.select_set(False)
            bpy.ops.object.mode_set(mode="EDIT")
            bpy.ops.mesh.select_all(action="SELECT")
            bpy.ops.mesh.quads_convert_to_tris(quad_method="BEAUTY", ngon_method="BEAUTY")
            bpy.ops.object.mode_set(mode="OBJECT")
        duplicate.parent = root
        copies.append(duplicate)
    if parts:
        if len(copies) != len(parts):
            raise RuntimeError(f"{name}: builder produced {len(copies)} meshes for {len(parts)} parts")
        export_objects = copies
    else:
        joined = forest.merge_group(name + "_mesh", copies)
        bpy.context.view_layer.objects.active = joined
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.select_all(action="SELECT")
        bpy.ops.mesh.normals_make_consistent(inside=False)
        bpy.ops.mesh.quads_convert_to_tris(quad_method="BEAUTY", ngon_method="BEAUTY")
        bpy.ops.object.mode_set(mode="OBJECT")
        joined.parent = root
        export_objects = [joined]
    verts = _points(export_objects)
    all_materials = sorted({material.name for obj in export_objects for material in obj.data.materials})
    record = {
        "description": entry["description"],
        "contract": contract,
        "vertices": sum(len(obj.data.vertices) for obj in export_objects),
        "triangles": sum(len(obj.data.polygons) for obj in export_objects),
        "mesh_count": len(export_objects),
        "editable_part_count": len(source),
        "bounds_blender": _bounds(verts),
        "bounds_unity": _bounds([H.unity(point) for point in verts]),
        "pivot_unity": [0, 0, 0],
        "materials": all_materials,
    }
    if parts:
        record["parts"] = list(parts)
        record["pivots_blender"] = [list(obj.matrix_world.translation) for obj in export_objects]
        record["pivots_unity"] = [list(H.unity(obj.matrix_world.translation)) for obj in export_objects]
    record["dimensions_unity_m"] = [round(b - a, 6) for a, b in zip(*record["bounds_unity"], strict=True)]
    if write:
        bpy.ops.object.select_all(action="DESELECT")
        root.select_set(True)
        for obj in export_objects:
            obj.select_set(True)
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
    for obj in export_objects:
        bpy.data.objects.remove(obj, do_unlink=True)
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


CONTRACT_CHECKERS = {
    "world_road_": _check_roads,
    "world_dungeon_wall": _check_wall_pillar,
    "world_dungeon_pillar": _check_wall_pillar,
    "world_dungeon_floor": _check_floor,
}


def check_contract(name, verts, record, objects):
    """Run the owning world module's check, falling back to legacy shared checks."""
    checks = {}
    prefixes = {}
    for path in sorted((ART / "world").glob("*.py")):
        if path.name.startswith("_") or path.name in {"helpers.py", "__init__.py"}:
            continue
        module = importlib.import_module(f"world.{path.stem}")
        for asset_name, check in getattr(module, "CHECKS", {}).items():
            if asset_name in checks:
                raise RuntimeError(f"duplicate check {asset_name}")
            checks[asset_name] = check
        for prefix, check in getattr(module, "CONTRACT_CHECKERS", {}).items():
            if prefix in prefixes or prefix in CONTRACT_CHECKERS:
                raise RuntimeError(f"duplicate contract checker {prefix}: {path.name}")
            prefixes[prefix] = check
    uv = [H.unity(vertex) for vertex in verts]
    lo, hi = H.bounds(uv)
    size = [b - a for a, b in zip(lo, hi, strict=True)]
    if name in checks:
        return checks[name](name, uv, lo, hi, size, record, objects)
    for prefix, check in {**prefixes, **CONTRACT_CHECKERS}.items():
        if name.startswith(prefix):
            return check(name, uv, lo, hi, size, record, objects)
    return []


def _upward_face_check(name, objects):
    """Reject unsupported downward-facing horizontal faces when requested."""
    for obj in objects:
        world = obj.matrix_world
        for poly in obj.data.polygons:
            normal = (world.to_3x3() @ poly.normal).normalized()
            if abs(normal.z) <= 0.95:
                continue
            centre = world @ poly.center
            origin = Vector((centre.x, centre.y, centre.z + 1e-4))
            hit_any = False
            for other in objects:
                inv = other.matrix_world.inverted()
                hit, _, _, _ = other.ray_cast(inv @ origin, inv.to_3x3() @ Vector((0, 0, 1)), distance=1e6)
                if hit:
                    hit_any = True
                    break
            if not hit_any and normal.z < 0:
                raise AssertionError((name, "unsupported downward horizontal face"))


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
        model_names = set()
        def walk(elements):
            for element in elements:
                if element.id == b"Model" and len(element.props) >= 2:
                    raw = element.props[1]
                    model_names.add(raw.decode("utf-8", "replace").split("\x00", 1)[0])
                walk(element.elems)
        walk(document.elems)
        expected_names = {name, *record.get("parts", [])} if record.get("parts") else {name, f"{name}_mesh"}
        if model_names != expected_names:
            raise AssertionError((name, "FBX model names", sorted(model_names), sorted(expected_names)))
        bpy.ops.import_scene.fbx(filepath=str(path), use_anim=False)
        bpy.context.view_layer.update()
        objects = [obj for obj in scene.objects if obj.type == "MESH"]
        expected_mesh_count = len(record.get("parts", record.get("contract", {}).get("parts", []))) or 1
        if len(objects) != expected_mesh_count:
            raise AssertionError((name, "mesh count", len(objects), expected_mesh_count))
        obj = objects[0]
        verts = _points(objects)
        if not all(math.isfinite(coordinate) for point in verts for coordinate in point):
            raise AssertionError((name, "nonfinite vertex"))
        triangles = sum(sum(len(poly.vertices) - 2 for poly in mesh.data.polygons) for mesh in objects)
        if triangles != record["triangles"]:
            raise AssertionError((name, "triangles", triangles, record["triangles"]))
        actual_bounds = _bounds(verts)
        for actual, expected in zip(actual_bounds, record["bounds_blender"], strict=True):
            _assert_close(actual, expected, name + " bounds")
        if record.get("parts"):
            actual_parts = {re.sub(r"\.\d{3}$", "", mesh.name) for mesh in objects}
            if actual_parts != set(record["parts"]):
                raise AssertionError((name, "part names", actual_parts, record["parts"]))
            expected_pivots = record.get("pivots_unity", [])
            actual_pivots = [list(H.unity(mesh.matrix_world.translation)) for mesh in objects]
            for actual_pivot in actual_pivots:
                if not any(max(abs(a - b) for a, b in zip(actual_pivot, expected, strict=True)) < 0.001 for expected in expected_pivots):
                    raise AssertionError((name, "part pivot", actual_pivot, expected_pivots))
        else:
            _assert_close(list(obj.matrix_world.translation), [0, 0, 0], name + " mesh pivot")
        for mesh in objects:
            _assert_close(list(mesh.matrix_world.to_scale()), [1, 1, 1], name + " scale")
        roots = [obj for obj in scene.objects if obj.parent is None]
        if len(roots) != 1 or roots[0].type != "EMPTY":
            raise AssertionError((name, "root hierarchy"))
        _assert_close(list(roots[0].matrix_world.translation), [0, 0, 0], name + " root pivot")
        actual_materials = {re.sub(r"\.\d{3}$", "", material.name) for mesh in objects for material in mesh.data.materials if material}
        if actual_materials != set(record["materials"]):
            raise AssertionError((name, "materials", actual_materials))
        for mesh in objects:
            for material in mesh.data.materials:
                key = re.sub(r"\.\d{3}$", "", material.name).removeprefix("Forest_")
                color = forest.PALETTE[key]
                expected = [int(color[index : index + 2], 16) / 255 for index in (0, 2, 4)]
                _assert_close(list(material.diffuse_color[:3]), expected, name + " material color")
        _upward_face_check(name, objects)
        contract_checks = check_contract(name, verts, record, objects)
        unity_bounds = _bounds([H.unity(point) for point in verts])
        record["verification"] = {
            "passed": True,
            "triangles": triangles,
            "mesh_count": len(objects),
            "bounds_blender": actual_bounds,
            "bounds_unity": unity_bounds,
            "dimensions_unity_m": [round(hi - lo, 6) for lo, hi in zip(*unity_bounds, strict=True)],
            "pivot_unity": list(H.unity(obj.matrix_world.translation)) if not record.get("parts") else [0, 0, 0],
            "scale": [round(value, 6) for value in obj.matrix_world.to_scale()],
            "fbx_axes": {
                key: axes[key] for key in ("UpAxis", "UpAxisSign", "FrontAxis", "FrontAxisSign", "UnitScaleFactor")
            },
            "contract_checks": contract_checks,
            **({"parts": [re.sub(r"\.\d{3}$", "", mesh.name) for mesh in objects],
                "pivots_unity": [list(H.unity(mesh.matrix_world.translation)) for mesh in objects]} if record.get("parts") else {}),
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
        "meshes": sum(entry["mesh_count"] for entry in assets.values()),
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
