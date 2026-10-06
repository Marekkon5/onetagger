import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import test from 'node:test';

const scripts = new URL('../../../deploy/vps/', import.meta.url);

test('online SQLite backup restores a verified database into a new target', () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-backup-'));
  const source = join(directory, 'live.sqlite');
  const backup = join(directory, 'backups', 'catalog.sqlite');
  const restored = join(directory, 'restore', 'catalog.sqlite');
  try {
    const database = new DatabaseSync(source);
    database.exec('CREATE TABLE sample(value TEXT NOT NULL); INSERT INTO sample(value) VALUES (\'persistent\')');
    database.close();

    const backupResult = spawnSync('python3', [new URL('backup_catalog.py', scripts).pathname, source, backup], { encoding: 'utf8' });
    assert.equal(backupResult.status, 0, backupResult.stderr);
    const restoreResult = spawnSync('python3', [new URL('restore_catalog.py', scripts).pathname, backup, restored], { encoding: 'utf8' });
    assert.equal(restoreResult.status, 0, restoreResult.stderr);

    const restoredDb = new DatabaseSync(restored);
    assert.equal((restoredDb.prepare('SELECT value FROM sample').get() as { value: string }).value, 'persistent');
    restoredDb.close();
    const overwrite = spawnSync('python3', [new URL('restore_catalog.py', scripts).pathname, backup, restored], { encoding: 'utf8' });
    assert.notEqual(overwrite.status, 0);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});