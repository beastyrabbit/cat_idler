"""Real maintained SQLite writer -> read-only normalizer -> playable C# save.

Build the normalizer and converter before running. Every input is generated in a
new temporary directory; these tests never locate or open a player's database.
"""
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "tools/save-import/normalize-legacy/target/debug"
CONVERTER = ROOT / "tools/save-import/Forest.SaveImport/bin/Debug/net10.0/Forest.SaveImport.dll"


def run(*arguments):
    return subprocess.run([str(value) for value in arguments], capture_output=True, text=True)


class PlayableImport(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="forest-synthetic-import-")
        self.root = Path(self.temporary.name)
        self.source = self.root / "synthetic.sqlite"
        self.assertEqual(run(BIN / "synthetic-fixture", self.source).returncode, 0)

    def tearDown(self):
        self.temporary.cleanup()

    def convert(self):
        before = hashlib.sha256(self.source.read_bytes()).digest()
        normalized = self.root / "normalized.json"
        playable = self.root / "playable.json"
        self.assertEqual(run(BIN / "forest-normalize-legacy", self.source, normalized).returncode, 0)
        self.assertEqual(run("dotnet", CONVERTER, normalized, playable).returncode, 0)
        self.assertEqual(hashlib.sha256(self.source.read_bytes()).digest(), before)
        for output in (normalized, playable):
            self.assertEqual(output.stat().st_mode & 0o777, 0o600)
        self.assertNotEqual(run(BIN / "forest-normalize-legacy", self.source, normalized).returncode, 0)
        self.assertNotEqual(run("dotnet", CONVERTER, normalized, playable).returncode, 0)
        envelope = json.loads(playable.read_text())
        self.assertEqual(envelope["Format"], "idle-cat-forest-unity")
        self.assertEqual(hashlib.sha256(envelope["Payload"].encode()).hexdigest(), envelope["Sha256"])
        environment = os.environ.copy()
        environment["FOREST_IMPORTED_WORLD"] = str(playable)
        environment["FOREST_TEST"] = "external synthetic SQLite"
        checked = subprocess.run(["dotnet", str(ROOT / "server/Forest.Tests/bin/Debug/net10.0/Forest.Tests.dll")],
                                 env=environment, capture_output=True, text=True)
        self.assertEqual(checked.returncode, 0, "Synthetic imported continuation failed.")
        self.assertIn("PASS external synthetic SQLite world resumes deterministically", checked.stdout)
        return json.loads(envelope["Payload"]), json.loads(normalized.read_text())

    def test_current_typed_save_preserves_identity_geometry_and_source(self):
        world, source = self.convert()
        self.assertEqual(len(world["Villages"]), 1)
        village = world["Villages"][0]
        self.assertEqual(len(village["Cats"]), 30)
        cat = next(cat for cat in village["Cats"] if cat["Name"] == "Fixture Moss")
        original = next(cat for cat in source["Tables"]["cats"] if cat["name"] == "Fixture Moss")
        position = json.loads(original["position"])
        self.assertEqual(cat["X"], position["x"] + village["Center"]["X"])
        self.assertEqual(cat["Z"], position["y"] + village["Center"]["Z"])
        self.assertEqual(cat["Hunger"], 71)
        self.assertEqual(cat["AgeHours"], 47.25)
        self.assertEqual(cat["Cargo"], [{"Resource": "logs", "Amount": 2.5}])
        self.assertIsNone(cat["ParentIds"][1])
        self.assertTrue(cat["ParentIds"][0].endswith("synthetic-parent"))
        self.assertGreater(len(village["BoundaryEdges"]), 20)
        self.assertTrue(any(pile["ResourceLimits"] for pile in village["Stockpiles"]))
        self.assertTrue(all(not tile["Wall"] for tile in world["Tiles"]))
        self.assertEqual(len(village["Buildings"]), len(source["Tables"]["buildings"]))
        station = next(job for job in village["Jobs"] if job["Kind"] == "production")
        self.assertEqual(station["Phase"], "input_delivery")
        self.assertEqual(station["Progress"], 599)
        self.assertEqual(station["Local"], [{"Resource": "logs", "Amount": 2}])
        worker = next(cat for cat in village["Cats"] if cat["Id"] == station["CatId"])
        self.assertEqual(worker["Cargo"], [{"Resource": "logs", "Amount": 3}])

    def test_older_additive_columns_are_migrated_only_in_memory(self):
        with sqlite3.connect(self.source) as connection:
            for table, column in (("cats", "preferredLabors"), ("cats", "boosted"),
                                  ("colonies", "transportState"), ("colonies", "migrationState"),
                                  ("buildings", "additionalWorkSlots"), ("buildings", "constructionCargo")):
                connection.execute(f'ALTER TABLE "{table}" DROP COLUMN "{column}"')
        world, _ = self.convert()
        self.assertEqual(len(world["Villages"][0]["Cats"]), 30)
        with sqlite3.connect(f"file:{self.source}?mode=ro", uri=True) as connection:
            self.assertNotIn("boosted", [row[1] for row in connection.execute("PRAGMA table_info(cats)")])

    def test_unknown_schema_is_refused_without_output(self):
        with sqlite3.connect(self.source) as connection:
            connection.execute("CREATE TABLE future_inventory (amount REAL)")
        before = self.source.read_bytes()
        result = run(BIN / "forest-normalize-legacy", self.source, self.root / "refused.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "refused.json").exists())
        self.assertEqual(self.source.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
