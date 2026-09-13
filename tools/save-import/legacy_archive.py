"""Read-only archival export, not a playable Unity save converter.

This preserves every known table, column, row and SQL schema declaration so a future
typed converter can be tested without opening or migrating the source database.
"""

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import tempfile


TABLES = {
    "world", "colonies", "cats", "jobs", "buildings", "world_tiles",
    "shared_world_tiles", "events", "zones", "elections", "votes", "raiders",
    "player_names",
}
REQUIRED = {"world", "colonies", "cats", "jobs", "buildings"}


def _cell(value):
    if isinstance(value, bytes):
        return {"$sqliteBlobBase64": base64.b64encode(value).decode("ascii")}
    return value


def export(source, destination):
    source, destination = Path(source).resolve(strict=True), Path(destination).absolute()
    if destination.exists():
        raise FileExistsError("The archive destination already exists.")
    if source == destination:
        raise ValueError("Source and destination must differ.")
    with sqlite3.connect(source.as_uri() + "?mode=ro", uri=True) as connection:
        connection.execute("PRAGMA query_only=ON")
        connection.execute("BEGIN")
        checked = connection.execute("PRAGMA quick_check").fetchall()
        if checked != [("ok",)]:
            raise ValueError("The source SQLite integrity check failed.")
        schema = connection.execute(
            "SELECT type,name,tbl_name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name"
        ).fetchall()
        tables = {row[1] for row in schema if row[0] == "table"}
        if not REQUIRED <= tables or tables - TABLES:
            raise ValueError("The selected database is not a recognized Idle Cat Forest world schema.")
        contents = {}
        rowids = {}
        for table in sorted(tables):
            # Every interpolated identifier comes from the fixed table allowlist.
            cursor = connection.execute(f'SELECT rowid, * FROM "{table}" ORDER BY rowid')
            names = [column[0] for column in cursor.description][1:]
            rows = cursor.fetchall()
            rowids[table] = [row[0] for row in rows]
            contents[table] = [dict(zip(names, map(_cell, row[1:]))) for row in rows]
        payload = {"Schema": schema, "RowIds": rowids, "Tables": contents}
        encoded_payload = json.dumps(payload, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
        document = {
            "Format": "idle-cat-forest-legacy-sqlite-archive",
            "Version": 1,
            "PlayableUnitySave": False,
            "ContentSha256": hashlib.sha256(encoded_payload.encode("utf-8")).hexdigest(),
            **payload,
        }
        connection.rollback()
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        descriptor, temporary = tempfile.mkstemp(prefix=".forest-archive-", dir=destination.parent)
        with os.fdopen(descriptor, "w", encoding="utf-8") as output:
            os.chmod(temporary, 0o600)
            json.dump(document, output, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
            output.flush()
            os.fsync(output.fileno())
        # A hard-link install is atomic and refuses an existing destination.
        os.link(temporary, destination)
        directory = os.open(destination.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if temporary is not None:
            Path(temporary).unlink(missing_ok=True)
    return {"TableCount": len(contents), "RowCount": sum(map(len, contents.values())), "PlayableUnitySave": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Explicit source SQLite path, opened read-only")
    parser.add_argument("destination", type=Path, help="New private archive path, never overwritten")
    arguments = parser.parse_args()
    try:
        print(json.dumps(export(arguments.source, arguments.destination)))
    except (OSError, ValueError, sqlite3.Error):
        parser.exit(1, "Archive failed. No playable save was created; source data was not modified.\n")


if __name__ == "__main__":
    main()
