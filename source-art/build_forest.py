"""Build and verify the modular Idle Cat Forest art kit."""

import argparse
import hashlib
import importlib
import json
import re
import sys
import uuid
from pathlib import Path

import bpy

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "source-art"
EXPORT = ROOT / "unity/Assets/Resources/ForestArt"


def discover():
    """Return builders and their module ownership in deterministic discovery order."""
    if str(ART) not in sys.path:
        sys.path.insert(0, str(ART))
    builders = {}
    owners = {}
    for path in sorted((ART / "kit").glob("*.py")):
        if path.name.startswith("_") or path.name in {"helpers.py", "__init__.py"}:
            continue
        module = importlib.import_module(f"kit.{path.stem}")
        for name, function in getattr(module, "BUILDERS", {}).items():
            if name in builders:
                raise RuntimeError(f"duplicate asset {name}: {owners[name]} and {path.name}")
            if name.startswith("world_"):
                raise RuntimeError(f"old-kit asset must not start with world_: {name}")
            builders[name] = function
            owners[name] = path.name
    return builders, owners


def ordered_names(builders, owners, canonical):
    """Apply the committed canonical order, then append discovered names."""
    missing = [name for name in canonical if name not in builders]
    if missing:
        raise RuntimeError("missing builders: " + ", ".join(missing))
    canonical_set = set(canonical)
    extras = [name for name in builders if name not in canonical_set]
    return [name for name in canonical if name in builders] + extras


def reset():
    """Reset Blender to an empty metric scene."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for collection in list(bpy.data.collections):
        bpy.data.collections.remove(collection)
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.scene.unit_settings.scale_length = 1.0


def _meta(name):
    path = EXPORT / f"{name}.fbx.meta"
    if path.exists():
        return
    template = (EXPORT / "den.fbx.meta").read_text()
    guid = uuid.uuid5(uuid.NAMESPACE_URL, f"idle-cat-forest/art/{name}.fbx").hex
    path.write_text(re.sub(r"^guid: [0-9a-f]{32}$", f"guid: {guid}", template, flags=re.MULTILINE))


def _write_record(name, entry, path):
    records = ROOT / "artifacts/art-build/records"
    records.mkdir(parents=True, exist_ok=True)
    record = {
        "asset": name,
        "description": entry["description"],
        "vertices": entry["vertices"],
        "triangles": entry["triangles"],
        "bounds_blender": entry["bounds_blender"],
        "mesh_count": len(entry["meshes"]),
        "parts": [re.sub(r"\.\d{3}$", "", obj.name) for obj in entry["meshes"]],
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }
    (records / f"{name}.json").write_text(json.dumps(record, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-render", action="store_true")
    parser.add_argument("--only")
    parser.add_argument("--manifests-only", action="store_true")
    options = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [])
    builders, owners = discover()
    from kit import helpers
    from tools import fbx_checks

    names = ordered_names(builders, owners, helpers.CANONICAL_ORDER)
    selected = [name for name in options.only.split(",") if name] if options.only else names
    unknown = [name for name in selected if name not in builders]
    if unknown:
        raise SystemExit("Unknown assets: " + ", ".join(unknown) + "; valid names: " + ", ".join(names))
    helpers.ASSETS.clear()
    reset()
    EXPORT.mkdir(parents=True, exist_ok=True)
    for name in selected:
        builders[name]()
    if options.only:
        for name in selected:
            helpers.export_one(name, helpers.ASSETS[name])
            _meta(name)
            _write_record(name, helpers.ASSETS[name], EXPORT / f"{name}.fbx")
        print(f"FOREST_ART_ONLY {len(selected)} assets")
        return
    if options.manifests_only:
        expected = {}
        for name in selected:
            entry = helpers.ASSETS[name]
            expected[name] = dict(entry)
            expected[name]["mesh_count"] = len(entry["meshes"])
            expected[name]["parts"] = [re.sub(r"\.\d{3}$", "", obj.name) for obj in entry["meshes"]]
        results = fbx_checks.verify_existing(selected, expected, EXPORT)
        helpers.export_and_save(True, export=False)
        fbx_checks.write_verification(results, ART / "verification.json")
        print(f"FOREST_ART_VERIFIED {len(selected)} FBX assets")
        return
    helpers.export_and_save(options.skip_render)
    for name in selected:
        _meta(name)


if __name__ == "__main__":
    main()
