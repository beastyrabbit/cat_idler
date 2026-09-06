import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("legacy_archive", Path(__file__).with_name("legacy_archive.py"))
archive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(archive)


class LegacyArchiveTests(unittest.TestCase):
    def test_authoritative_rows_cargo_and_unknown_columns_survive_without_source_write(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.db"
            destination = Path(directory) / "archive.json"
            with sqlite3.connect(source) as connection:
                for table in ("world", "colonies", "cats", "jobs", "buildings"):
                    connection.execute(f'CREATE TABLE "{table}" (id TEXT, payload TEXT, nullable TEXT)')
                connection.execute("INSERT INTO world VALUES ('1', '{\"worldSeed\":42}',NULL)")
                connection.execute("INSERT INTO cats VALUES (?,?,NULL)", ("village\x1fcat-1", '{"carrying":{"amount":2.5,"itemIds":["exact-1"]},"futureField":7}'))
                connection.execute("INSERT INTO jobs VALUES (?,?,NULL)", ("village\x1fjob-1", '{"reservations":[{"source":"pile-1","amount":1.25}]}'))
            before = source.read_bytes()
            archive.export(source, destination)
            self.assertEqual(source.read_bytes(), before)
            document = json.loads(destination.read_text())
            self.assertEqual(document["Tables"]["cats"][0]["id"], "village\x1fcat-1")
            self.assertIsNone(document["Tables"]["cats"][0]["nullable"])
            self.assertEqual(json.loads(document["Tables"]["cats"][0]["payload"])["futureField"], 7)
            self.assertEqual(os.stat(destination).st_mode & 0o777, 0o600)
            with self.assertRaises(FileExistsError):
                archive.export(source, destination)
            self.assertEqual(source.read_bytes(), before)

    def test_unknown_tables_fail_without_partial_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.db"
            destination = Path(directory) / "archive.json"
            with sqlite3.connect(source) as connection:
                connection.execute("CREATE TABLE unexpected_secret_store (id TEXT)")
            with self.assertRaises(ValueError):
                archive.export(source, destination)
            self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
