"""Verified SQLite/media bundles for a stopped-target restore."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import tempfile


def validate(database):
    if database.execute('PRAGMA integrity_check').fetchone() != ('ok',):
        raise RuntimeError('SQLite integrity check failed')
    if database.execute('PRAGMA foreign_key_check').fetchone():
        raise RuntimeError('SQLite foreign-key check failed')


def digest(path):
    if path.is_symlink() or not path.is_file():
        raise RuntimeError('Bundle entries must be regular files')
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def check_media_references(database, media):
    with sqlite3.connect(f'file:{database.as_posix()}?mode=ro', uri=True) as connection:
        validate(connection)
        exists = connection.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='media_assets'").fetchone()
        if exists:
            for (relative,) in connection.execute('SELECT relative_path FROM media_assets'):
                if Path(relative).name != relative or not (media / relative).is_file() or (media / relative).is_symlink():
                    raise RuntimeError('Backup is missing referenced catalog media')


def backup_bundle(source, destination, media):
    if destination.exists():
        raise RuntimeError('Bundle destination must be a new path')
    if media.is_symlink() or not media.is_dir():
        raise RuntimeError('Media directory must exist and must not be a symlink')
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    stage = Path(tempfile.mkdtemp(prefix='.catalog-backup-', dir=destination.parent))
    try:
        database = stage / 'catalog.sqlite'
        # Read the live database through a read-write connection so SQLite reads the
        # write-ahead log; a read-only connection can miss WAL-resident schema and rows.
        # VACUUM INTO yields a self-contained snapshot with no sidecar WAL dependency.
        with sqlite3.connect(source, timeout=30) as live:
            live.execute('VACUUM INTO ?', (str(database),))
        with sqlite3.connect(database, timeout=30) as snapshot:
            validate(snapshot)
        os.chmod(database, 0o600)
        (stage / 'media').mkdir(mode=0o700)
        hashes = {'catalog.sqlite': digest(database)}
        for asset in sorted(media.iterdir()):
            if asset.name.startswith('.'):
                continue  # Atomic-write temporary files are not committed catalog media.
            digest(asset)  # Refuse symlinks or nested directories before copying.
            target = stage / 'media' / asset.name
            shutil.copyfile(asset, target)
            os.chmod(target, 0o600)
            hashes[f'media/{asset.name}'] = digest(target)
        check_media_references(database, stage / 'media')
        manifest = stage / 'manifest.json'
        manifest.write_text(json.dumps({'version': 1, 'sha256': hashes}, indent=2) + '\n')
        os.chmod(manifest, 0o600)
        verify_bundle(stage)
        os.rename(stage, destination)
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def verify_bundle(source):
    manifest_path = source / 'manifest.json'
    if manifest_path.is_symlink() or manifest_path.stat().st_size > 2 * 1024 * 1024:
        raise RuntimeError('Invalid backup manifest')
    manifest = json.loads(manifest_path.read_text())
    hashes = manifest.get('sha256')
    if manifest.get('version') != 1 or not isinstance(hashes, dict) or 'catalog.sqlite' not in hashes:
        raise RuntimeError('Invalid backup manifest')
    for name, expected in hashes.items():
        parts = Path(name).parts
        if name != 'catalog.sqlite' and (len(parts) != 2 or parts[0] != 'media' or parts[1] in ('.', '..')):
            raise RuntimeError('Invalid bundle path')
        if Path(name).is_absolute() or '..' in parts or digest(source / name) != expected:
            raise RuntimeError('Backup content verification failed')
    if (source / 'media').is_symlink():
        raise RuntimeError('Bundle media must not be a symlink')
    actual = {'catalog.sqlite'} | {f'media/{asset.name}' for asset in (source / 'media').iterdir()}
    if actual != set(hashes):
        raise RuntimeError('Bundle manifest does not match its files')
    check_media_references(source / 'catalog.sqlite', source / 'media')


def restore_bundle(source, destination):
    verify_bundle(source)
    media_target = destination.parent / 'media'
    if destination.exists() or media_target.exists():
        raise RuntimeError('Restore database and media destinations must be new paths on a stopped target')
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    stage = Path(tempfile.mkdtemp(prefix='.catalog-restore-', dir=destination.parent))
    try:
        shutil.copytree(source / 'media', stage / 'media')
        with sqlite3.connect(f'file:{(source / "catalog.sqlite").as_posix()}?mode=ro', uri=True) as snapshot:
            with sqlite3.connect(stage / 'catalog.sqlite') as restored:
                snapshot.backup(restored)
                validate(restored)
        os.chmod(stage / 'catalog.sqlite', 0o600)
        os.rename(stage / 'media', media_target)
        try:
            os.rename(stage / 'catalog.sqlite', destination)
        except BaseException:
            shutil.rmtree(media_target)
            raise
    finally:
        shutil.rmtree(stage)
