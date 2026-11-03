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
test('catalog bundle restores source identities, rejected decisions and waveform files, and rejects tampering',async()=>{
  const {backupCatalog,restoreCatalog}=await import('./backup.js');
  const {openCatalog}=await import('./repository.js');const {migrateLegacyLibrary}=await import('./migration.js');const {readWaveformMedia}=await import('./media.js');
  const {readFileSync,writeFileSync}=await import('node:fs');
  const directory=mkdtempSync(join(tmpdir(),'syco23-full-backup-')),oldMedia=process.env.CATALOG_MEDIA_PATH;
  process.env.CATALOG_MEDIA_PATH=join(directory,'live','media');const repo=openCatalog(join(directory,'live','catalog.sqlite'));
  try{
    const source={provider:'archiveorg',externalId:'backup-real',url:'https://archive.org/details/backup-real'};
    const result=migrateLegacyLibrary(repo,[{id:'backup-local',title:'Real backup',sources:[source],waveform:{imageDataUrl:'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII=',sourceUrl:'https://archive.org/download/backup-real/audio.mp3'},candidates:[{provider:'archiveorg',state:'rejected',fields:{title:'Bad title'}}]}],'backup-batch',{sessionId:'curator'});
    const id=result.legacyIds['backup-local'];const bundle=join(directory,'backup'),restored=join(directory,'restored');
    await backupCatalog(repo,bundle);await restoreCatalog(bundle,restored);
    process.env.CATALOG_MEDIA_PATH=join(restored,'media');const copy=openCatalog(join(restored,'catalog.sqlite'));
    try{assert.equal(copy.findByProvider({provider:'archiveorg',resourceType:'recording',externalId:'backup-real'}),id);assert.ok(copy.listClaims(id).some(claim=>claim.disposition==='rejected'));const record=copy.getRecord(id);const asset=record?.assets.find(asset=>asset.role==='waveform');assert.ok(asset?.mediaId);assert.ok(readWaveformMedia(copy,asset.mediaId));}finally{copy.close();}
    await assert.rejects(()=>restoreCatalog(bundle,restored),/exist|new|overwrite/i);
    const media=repo.listMediaAssets()[0];const png=join(bundle,'media',media.relativePath);const bytes=readFileSync(png);bytes[bytes.length-1]^=1;writeFileSync(png,bytes);
    await assert.rejects(()=>restoreCatalog(bundle,join(directory,'tampered')),/hash|checksum|integrity/i);
  }finally{repo.close();if(oldMedia===undefined)delete process.env.CATALOG_MEDIA_PATH;else process.env.CATALOG_MEDIA_PATH=oldMedia;rmSync(directory,{recursive:true,force:true});}
});
