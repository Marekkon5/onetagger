#!/usr/bin/env python3
import os
import sqlite3
import sys
import tempfile
from pathlib import Path


def validate(database: sqlite3.Connection) -> None:
    integrity = database.execute("PRAGMA integrity_check").fetchone()
    if not integrity or integrity[0] != "ok":
        raise RuntimeError("SQLite integrity check failed")
    if database.execute("PRAGMA foreign_key_check").fetchone():
        raise RuntimeError("SQLite foreign-key check failed")


def snapshot(source: Path, destination: Path) -> None:
    """Write a consistent, self-contained copy of a live (possibly WAL) database.

    The source is opened read-write so SQLite reads its write-ahead log. A
    read-only connection can silently read only the main database file and miss
    schema and rows that are still in the WAL, which yields a structurally stale
    backup. ``VACUUM INTO`` copies the current committed content in its own
    transaction and produces a database that does not depend on a sidecar WAL,
    so any read-only consumer can open it reliably. It refuses an existing target.
    """
    if source == destination:
        raise RuntimeError("Backup destination must differ from the live database")
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{destination.name}.", suffix=".tmp", dir=destination.parent)
    os.close(descriptor)
    temporary = Path(temporary_name)
    temporary.unlink()  # VACUUM INTO refuses a pre-existing target path.
    try:
        with sqlite3.connect(source, timeout=30) as source_db:
            source_db.execute("VACUUM INTO ?", (str(temporary),))
        with sqlite3.connect(temporary, timeout=30) as backup_db:
            validate(backup_db)
        os.chmod(temporary, 0o600)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def main() -> int:
    if len(sys.argv) == 5 and sys.argv[3] == '--media-dir':
        from catalog_bundle import backup_bundle
        backup_bundle(Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]).resolve(), Path(sys.argv[4]).resolve(strict=True))
        print('Verified database and media bundle written.')
        return 0
    if len(sys.argv) != 3:
        print("Usage: backup_catalog.py DATABASE BACKUP [--media-dir MEDIA]", file=sys.stderr)
        return 2
    source = Path(sys.argv[1]).resolve(strict=True)
    destination = Path(sys.argv[2]).resolve()
    if source == destination:
        print("Backup destination must differ from the live database.", file=sys.stderr)
        return 2
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    snapshot(source, destination)
    print(f"Verified SQLite backup written to {destination}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())