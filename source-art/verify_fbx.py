"""Reimport and verify old-kit and world-kit FBX exports."""

import argparse
import hashlib
import json
import sys
from pathlib import Path


sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "source-art"
EXPORT = ROOT / "unity/Assets/Resources/ForestArt"


def parse_args():
    """Parse arguments passed after Blender's ``--`` separator."""
    arguments = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--only")
    parser.add_argument("--no-write", action="store_true")
    return parser.parse_args(arguments)


def _load_expected(name):
    old = json.loads((ART / "asset-manifest.json").read_text())
    world = json.loads((ART / "world-asset-manifest.json").read_text())
    manifest = world if name.startswith("world_") else old
    expected = manifest.get("assets", {}).get(name)
    record_path = ROOT / "artifacts/art-build/records" / f"{name}.json"
    if record_path.exists():
        record = json.loads(record_path.read_text())
        path = EXPORT / f"{name}.fbx"
        if hashlib.sha256(path.read_bytes()).hexdigest() == record.get("sha256"):
            expected = record
    if expected is None:
        raise RuntimeError(f"No manifest or current build record for {name}; run the build with --only {name}")
    return expected


def verify(names, write_report):
    """Verify old-kit names through the shared temporary-scene checker."""
    if str(ART) not in sys.path:
        sys.path.insert(0, str(ART))
    from tools import fbx_checks

    expected = {name: _load_expected(name) for name in names}
    report_path = ART / "verification.json" if write_report else None
    return fbx_checks.verify_existing(names, expected, EXPORT, report_path=report_path)


def main():
    options = parse_args()
    manifest = json.loads((ART / "asset-manifest.json").read_text())
    if options.only:
        names = [name for name in options.only.split(",") if name]
    else:
        names = list(manifest["assets"])
    world_names = [name for name in names if name.startswith("world_")]
    old_names = [name for name in names if not name.startswith("world_")]
    if world_names:
        if str(ART) not in sys.path:
            sys.path.insert(0, str(ART))
        import build_world

        world_records = {name: _load_expected(name) for name in world_names}
        build_world.reset()
        build_world.verify({"assets": world_records})
    results = verify(old_names, write_report=not options.no_write and not options.only)
    print(f"FOREST_ART_VERIFIED {len(results) + len(world_names)} FBX assets")


if __name__ == "__main__":
    main()
