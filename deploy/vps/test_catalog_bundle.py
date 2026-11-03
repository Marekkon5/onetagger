import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).parent


class CatalogBundleTests(unittest.TestCase):
    def test_media_and_database_restore_together_and_tampering_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            database = root / 'catalog.sqlite'
            media = root / 'media'
            media.mkdir()
            (media / 'waveform.png').write_bytes(b'disposable-media-content')
            with sqlite3.connect(database) as connection:
                connection.execute('CREATE TABLE records(id TEXT PRIMARY KEY)')
                connection.execute("INSERT INTO records VALUES ('stable-record')")
            backup = root / 'bundle'
            result = subprocess.run([sys.executable, str(SCRIPTS / 'backup_catalog.py'), str(database), str(backup), '--media-dir', str(media)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((backup / 'catalog.sqlite').is_file())
            self.assertEqual((backup / 'media/waveform.png').read_bytes(), b'disposable-media-content')
            destination = root / 'restored/catalog.sqlite'
            restore = subprocess.run([sys.executable, str(SCRIPTS / 'restore_catalog.py'), str(backup), str(destination)], capture_output=True, text=True)
            self.assertEqual(restore.returncode, 0, restore.stderr)
            with sqlite3.connect(destination) as connection:
                self.assertEqual(connection.execute('SELECT id FROM records').fetchone()[0], 'stable-record')
            self.assertEqual((destination.parent / 'media/waveform.png').read_bytes(), b'disposable-media-content')
            self.assertNotEqual(subprocess.run([sys.executable, str(SCRIPTS / 'restore_catalog.py'), str(backup), str(destination)], capture_output=True).returncode, 0)
            (backup / 'media/waveform.png').write_bytes(b'tampered')
            other = root / 'tampered/catalog.sqlite'
            invalid = subprocess.run([sys.executable, str(SCRIPTS / 'restore_catalog.py'), str(backup), str(other)], capture_output=True)
            self.assertNotEqual(invalid.returncode, 0)
            self.assertFalse(other.exists())
            self.assertFalse((other.parent / 'media').exists())

    def test_bundle_rejects_symlinks_and_path_traversal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            database = root / 'catalog.sqlite'
            with sqlite3.connect(database) as connection:
                connection.execute('CREATE TABLE records(id TEXT)')
            media = root / 'media'
            media.mkdir()
            (media / 'escape.png').symlink_to(database)
            result = subprocess.run([sys.executable, str(SCRIPTS / 'backup_catalog.py'), str(database), str(root / 'bundle'), '--media-dir', str(media)], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / 'bundle').exists())


    def test_bundle_snapshot_includes_wal_resident_records(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            database = root / 'catalog.sqlite'
            media = root / 'media'
            media.mkdir()
            live = sqlite3.connect(database)
            live.execute('PRAGMA journal_mode=WAL')
            live.execute('CREATE TABLE baseline(id TEXT)')
            live.commit()
            live.execute('PRAGMA wal_checkpoint(TRUNCATE)')
            live.execute('CREATE TABLE records(id TEXT PRIMARY KEY)')
            live.execute("INSERT INTO records VALUES ('wal-only')")
            live.commit()
            try:
                self.assertGreater((root / 'catalog.sqlite-wal').stat().st_size, 0)
                backup = root / 'bundle'
                result = subprocess.run([sys.executable, str(SCRIPTS / 'backup_catalog.py'), str(database), str(backup), '--media-dir', str(media)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                with sqlite3.connect(backup / 'catalog.sqlite') as connection:
                    self.assertEqual(connection.execute('SELECT id FROM records').fetchone()[0], 'wal-only')
                    self.assertNotEqual(connection.execute('PRAGMA journal_mode').fetchone()[0], 'wal')
            finally:
                live.close()


if __name__ == '__main__':
    unittest.main()
