"""Real maintained SQLite writer -> read-only normalizer -> playable C# save.

Build the normalizer and converter before running. Every input is generated in a
new temporary directory; these tests never locate or open a player's database.
"""
import hashlib
import json
import os
from contextlib import closing
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

    def test_station_output_writer_markers_preserve_destination_and_exact_cargo(self):
        self.source = self.root / "station-output.sqlite"
        generated = run(BIN / "synthetic-fixture", self.source, "station-output")
        self.assertEqual(generated.returncode, 0, generated.stderr)
        with closing(sqlite3.connect(f"file:{self.source}?mode=ro", uri=True)) as connection:
            cats = {name: (cat_id, json.loads(carrying)) for cat_id, name, carrying in
                    connection.execute("SELECT id, name, carrying FROM cats WHERE name LIKE 'Fixture output %'")}
            stockpiles, items = connection.execute("SELECT stockpiles, items FROM colonies").fetchone()
        original_piles = json.loads(stockpiles)
        original_items = json.loads(items)["instances"]
        world, normalized = self.convert()
        village = world["Villages"][0]
        self.assertEqual(len(village["Cats"]), 30)
        self.assertEqual(len(cats), 5)
        def qualify(identity):
            return identity if "\u001f" in identity or identity.startswith("world-item:") else village["Id"] + "\u001f" + identity
        converted_items = {item["Id"]: item for item in village["Items"]}
        self.assertEqual(set(converted_items), {qualify(item["id"]) for item in original_items})
        normalized_cats = {cat["name"]: cat for cat in normalized["Tables"]["cats"]}

        for label, resource, kind in (("planks", "planks", None), ("tools", "tools", "tool"),
                                      ("weapons", "weapons", "weapon"), ("armor", "armor", "armor"),
                                      ("trinket", "refined", "trinket")):
            with self.subTest(output=label):
                name = "Fixture output " + label
                cat_id, carrying = cats[name]
                self.assertEqual(json.loads(normalized_cats[name]["carrying"]), carrying)
                marker = carrying["sourceGatherSpot"].split("|")
                self.assertEqual(marker[:2], ["station-out", "fixture-output-" + label])
                self.assertEqual(carrying["kind"], resource)
                self.assertEqual(len(marker), 4 if label == "trinket" else 3)
                destination = next(pile for pile in original_piles if pile["id"] == marker[2])
                self.assertFalse(marker[2].startswith("station-"))
                pile = next(pile for pile in village["Stockpiles"] if pile["Id"] == qualify(marker[2]))
                cat = next(cat for cat in village["Cats"] if cat["Id"] == qualify(cat_id))
                job = next(job for job in village["Jobs"] if job["CatId"] == cat["Id"])
                self.assertEqual(job["Phase"], "output_delivery")
                self.assertEqual(job["TargetId"], qualify(marker[1]))
                self.assertEqual(cat["JobId"], job["Id"])

                if kind is None:
                    self.assertEqual(next(stack["Amount"] for stack in pile["Goods"] if stack["Resource"] == resource),
                                     destination["contents"][resource])
                    self.assertEqual(cat["Cargo"], [{"Resource": resource, "Amount": carrying["amount"]}])
                    self.assertEqual(job["ItemIds"], [])
                    continue

                carried = [item for item in original_items if item["location"]["kind"] == "carrier"
                           and qualify(item["location"]["cat_id"]) == qualify(cat_id)]
                self.assertEqual(len(carried), carrying["amount"])
                self.assertEqual(set(job["ItemIds"]), {qualify(item["id"]) for item in carried})
                self.assertEqual(cat["Cargo"], [])
                for original in carried:
                    item = converted_items[qualify(original["id"])]
                    self.assertEqual(item["LocationId"], job["Id"])
                    self.assertEqual(item["Kind"], kind)
                    self.assertEqual([item["Kind"], item["Material"], str(item["Quality"])], original["item"].split(":"))
                    self.assertEqual(item["Condition"], original["durability"])
                    self.assertEqual(item["MaxCondition"], original["maxDurability"])
                    self.assertFalse(item["Credited"])
                if label == "trinket":
                    self.assertEqual(marker[3], "item:" + carried[0]["id"])
                    self.assertEqual(next(stack["Amount"] for stack in pile["Goods"] if stack["Resource"] == "refined"),
                                     destination["contents"]["refined"])
                else:
                    mirror = next(source for source in original_piles if source["id"] == "station-output:" + marker[1])
                    self.assertEqual(mirror["contents"][resource], 1)
                for original in original_items:
                    location = original["location"]
                    expected = None
                    if location == {"kind": "stockpile", "stockpile_id": marker[2]}:
                        expected = qualify(marker[2])
                    elif location["kind"] == "equipped" and qualify(location["cat_id"]) == cat["Id"]:
                        expected = cat["Id"]
                    elif location == {"kind": "station", "building_id": marker[1], "compartment": "local_output"}:
                        expected = qualify(marker[1])
                    if expected is not None:
                        self.assertEqual(converted_items[qualify(original["id"])]["LocationId"], expected)

        for goods in ([pile["Goods"] for pile in village["Stockpiles"]]
                      + [cat["Cargo"] for cat in village["Cats"]]
                      + [job["Local"] for job in village["Jobs"]]
                      + [building[field] for building in village["Buildings"] for field in ("Inputs", "Outputs")]):
            self.assertFalse(any(stack["Resource"] in ("tools", "weapons", "armor") and stack["Amount"] != 0 for stack in goods))

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
        with closing(sqlite3.connect(self.source)) as connection, connection:
            for table, column in (("cats", "preferredLabors"), ("cats", "boosted"),
                                  ("colonies", "transportState"), ("colonies", "migrationState"),
                                  ("buildings", "additionalWorkSlots"), ("buildings", "constructionCargo")):
                connection.execute(f'ALTER TABLE "{table}" DROP COLUMN "{column}"')
        world, _ = self.convert()
        self.assertEqual(len(world["Villages"][0]["Cats"]), 30)
        with closing(sqlite3.connect(f"file:{self.source}?mode=ro", uri=True)) as connection:
            self.assertNotIn("boosted", [row[1] for row in connection.execute("PRAGMA table_info(cats)")])

    def test_unknown_schema_is_refused_without_output(self):
        with closing(sqlite3.connect(self.source)) as connection, connection:
            connection.execute("CREATE TABLE future_inventory (amount REAL)")
        before = self.source.read_bytes()
        result = run(BIN / "forest-normalize-legacy", self.source, self.root / "refused.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "refused.json").exists())
        self.assertEqual(self.source.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
