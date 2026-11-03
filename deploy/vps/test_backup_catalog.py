import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).parent


def write_live_wal_database(root: Path) -> sqlite3.Connection:
    """A live WAL database whose newest schema and rows are still in the WAL.

    The baseline is checkpointed into the main file; the ``mixes`` table and its
    row are committed afterwards and stay in the write-ahead log, which is the
    state a running worker produces.
    """
    connection = sqlite3.connect(root / "catalog.sqlite")
    connection.execute("PRAGMA journal_mode=WAL")
    connection.execute("CREATE TABLE baseline (id TEXT)")
    connection.commit()
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    connection.execute("CREATE TABLE mixes (id TEXT PRIMARY KEY, title TEXT)")
    connection.execute("INSERT INTO mixes VALUES ('mix-1', 'WAL-resident recording')")
    connection.commit()
    return connection


class BackupCatalogWalTests(unittest.TestCase):
    def test_backup_captures_wal_resident_schema_and_is_self_contained(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            live = write_live_wal_database(root)
            try:
                # Precondition: the new schema and rows really are only in the WAL.
                self.assertGreater((root / "catalog.sqlite-wal").stat().st_size, 0)
                destination = root / "backup.sqlite"
                result = subprocess.run(
                    [sys.executable, str(SCRIPTS / "backup_catalog.py"), str(root / "catalog.sqlite"), str(destination)],
                    capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                # The backup must carry the WAL-resident table and rows for a read-only consumer.
                with sqlite3.connect(f"file:{destination.as_posix()}?mode=ro", uri=True) as backup:
                    self.assertEqual(backup.execute("SELECT title FROM mixes").fetchone()[0], "WAL-resident recording")
                # It must not remain WAL-flagged: a read-only consumer cannot reliably read a
                # WAL database that has no -wal/-shm beside it (the original production defect,
                # where a read-only backup silently captured only part of the schema).
                with sqlite3.connect(destination) as backup:
                    self.assertNotEqual(backup.execute("PRAGMA journal_mode").fetchone()[0], "wal")
                self.assertFalse((root / "backup.sqlite-wal").exists())
            finally:
                live.close()

    def test_backup_refuses_to_overwrite_the_live_database(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            live = write_live_wal_database(root)
            try:
                result = subprocess.run(
                    [sys.executable, str(SCRIPTS / "backup_catalog.py"), str(root / "catalog.sqlite"), str(root / "catalog.sqlite")],
                    capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
            finally:
                live.close()


if __name__ == "__main__":
    unittest.main()
