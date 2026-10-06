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


def main() -> int:
    if len(sys.argv) != 3:
        print("Usage: restore_catalog.py VERIFIED_BACKUP NEW_DATABASE", file=sys.stderr)
        return 2
    source = Path(sys.argv[1]).resolve(strict=True)
    destination = Path(sys.argv[2]).resolve()
    if source == destination or destination.exists():
        print("Restore destination must be a new path different from the backup.", file=sys.stderr)
        return 2
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{destination.name}.", suffix=".tmp", dir=destination.parent)
    os.close(descriptor)
    temporary = Path(temporary_name)
    try:
        with sqlite3.connect(f"file:{source.as_posix()}?mode=ro", uri=True, timeout=30) as source_db:
            validate(source_db)
            with sqlite3.connect(temporary, timeout=30) as restore_db:
                source_db.backup(restore_db, pages=256, sleep=0.05)
                validate(restore_db)
        os.chmod(temporary, 0o600)
        os.replace(temporary, destination)
        print(f"Verified database restored to new path {destination}.")
        return 0
    finally:
        temporary.unlink(missing_ok=True)


if __name__ == "__main__":
    raise SystemExit(main())