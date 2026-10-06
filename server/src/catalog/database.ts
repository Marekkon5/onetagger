import { mkdirSync, readFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

export function openDatabase(path: string): DatabaseSync {
  if (path !== ':memory:') mkdirSync(dirname(path), { recursive: true });
  const database = new DatabaseSync(path);
  database.exec('PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;');

  const currentVersion = database.prepare('PRAGMA user_version').get() as { user_version: number };
  const migrations = [
    { version: 1, file: '001-catalog.sql' },
    { version: 2, file: '002-claims.sql' },
    { version: 3, file: '003-curator-sessions.sql' },
    { version: 4, file: '004-media-assets.sql' },
    { version: 5, file: '005-migration-batches.sql' },
    { version: 6, file: '006-enrichment-runs.sql' },
    { version: 7, file: '007-import-jobs.sql' }
  ];
  for (const migration of migrations) {
    if (currentVersion.user_version >= migration.version) continue;
    const sql = readFileSync(new URL(`./migrations/${migration.file}`, import.meta.url), 'utf8');
    database.exec('BEGIN IMMEDIATE');
    try {
      database.exec(sql);
      database.exec(`PRAGMA user_version = ${migration.version}`);
      database.exec('COMMIT');
    } catch (error) {
      database.exec('ROLLBACK');
      database.close();
      throw error;
    }
  }

  return database;
}