import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { openCatalog } from './repository.js';
import { migrateLegacyLibrary } from './migration.js';
import { readWaveformMedia } from './media.js';
import { decideReview } from './merge.js';

const pngData = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII=';

test('legacy migration preserves curated covers, waveform and IDs while excluding demo fixtures', () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-migration-'));
  const path = join(directory, 'catalog.sqlite');
  const priorMediaPath = process.env.CATALOG_MEDIA_PATH;
  process.env.CATALOG_MEDIA_PATH = join(directory, 'media');
  const repo = openCatalog(path);
  try {
    const source = { provider: 'freeteknomusic', url: 'https://archive.freeteknomusic.org/mix-1', externalId: 'mix-1' };
    const records = [
      {
        id: 'legacy-real-1', title: 'Aniane Live 98', artists: ['Metek'], crews: ['Metek Sound System'],
        event: 'Aniane Free Party', recordedAt: '1998', durationMs: 5_392_000, description: 'Curated description', genres: ['Hardtek'], styles: ['Tribe'],
        artwork: [{ url: 'https://example.org/manual-cover.jpg', source: 'local', kind: 'cover' }],
        sources: [source], waveform: { imageDataUrl: `data:image/png;base64,${pngData}`, sourceUrl: source.url }
      },
      { id: 'legacy-real-2', title: 'Aniane Live 98', artists: ['Metek'], durationMs: 5_392_000, artwork: [], sources: [source] },
      { id: 'spiral-warehouse-2001', title: 'Demo fixture', artwork: [], sources: [] },
      { id: 'legacy-invalid', title: '', durationMs: -1 }
    ];

    const result = migrateLegacyLibrary(repo, records, 'browser-batch-1', { sessionId: 'curator-session' });
    assert.equal(result.imported, 1);
    assert.equal(result.existing, 1);
    assert.equal(result.rejected, 2);
    assert.equal(result.legacyIds['legacy-real-1'], result.legacyIds['legacy-real-2']);
    const migrated = repo.getRecord('legacy-real-1');
    assert.equal(migrated?.kind === 'mix' ? migrated.people.length : -1, 0);
    assert.ok(repo.listReview().some(item=>item.field==='people'));
    const eventId = (repo.listReview().find(item=>item.field==='eventIds')?.claim.value as string[]|undefined)?.[0];
    const event = eventId ? repo.getRecord(eventId) : undefined;
    assert.equal(event?.kind, 'event');
    assert.equal(event?.verification, 'proposed');
    assert.equal(event?.kind === 'event' ? event.startDate : undefined, undefined);
    assert.equal(repo.listIndex('event', { page: 1, pageSize: 25 }).total, 0);
    const eventClaim = repo.listReview().find((item) => item.targetRecordId === eventId && item.field === 'name');
    assert.ok(eventClaim);
    decideReview(repo, eventClaim.id, 'accept', eventClaim.recordRevision, 'curator-session');
    assert.equal(repo.listIndex('event', { page: 1, pageSize: 25 }).total, 1);
    const migratedMix = repo.getRecord('legacy-real-1');
    assert.equal(migratedMix?.kind === 'mix' ? migratedMix.recordingDate?.value : undefined, '1998');
    assert.equal(migrated?.kind === 'mix' ? migrated.assets.find((asset) => asset.role === 'mix-cover')?.url : undefined, 'https://example.org/manual-cover.jpg');
    assert.equal(migrated?.kind === 'mix' ? migrated.recordingDate?.value : undefined, '1998');
    const waveform = migrated?.kind === 'mix' ? migrated.assets.find((asset) => asset.role === 'waveform') : undefined;
    assert.ok(waveform?.mediaId);
    assert.ok(readWaveformMedia(repo, waveform.mediaId));
    assert.equal(repo.getRecord('spiral-warehouse-2001'), undefined);

    const retry = migrateLegacyLibrary(repo, records, 'browser-batch-1', { sessionId: 'curator-session' });
    assert.deepEqual(retry, result);
  } finally {
    repo.close();
    if (priorMediaPath === undefined) delete process.env.CATALOG_MEDIA_PATH;
    else process.env.CATALOG_MEDIA_PATH = priorMediaPath;
    rmSync(directory, { recursive: true, force: true });
  }
});
test('migration reports per-record media failures and retries without duplicating real records', () => {
  const directory=mkdtempSync(join(tmpdir(),'syco23-migration-retry-'));const prior=process.env.CATALOG_MEDIA_PATH;
  process.env.CATALOG_MEDIA_PATH=join(directory,'media');const repo=openCatalog(join(directory,'catalog.sqlite'));
  try {
    const records=[{id:'legacy-retry-1',title:'Real archive',sources:[{provider:'archiveorg',externalId:'retry-source',url:'https://archive.org/details/retry-source'}],waveform:{imageDataUrl:'data:image/png;base64,bm90LXBuZw==',sourceUrl:'https://archive.org/audio.mp3'}}];
    const first=migrateLegacyLibrary(repo,records,'retry-batch',{sessionId:'curator'});
    assert.equal(first.outcomes?.[0].status,'partial');assert.ok(first.outcomes?.[0].errors.some(message=>/PNG/.test(message)));
    records[0].waveform.imageDataUrl=`data:image/png;base64,${pngData}`;
    const retry=migrateLegacyLibrary(repo,records,'retry-batch',{sessionId:'curator'});
    assert.equal(retry.outcomes?.[0].status,'existing');assert.deepEqual(retry.outcomes?.[0].errors,[]);
    assert.equal(repo.listIndex('mix',{page:1,pageSize:25}).total,1);
  } finally {repo.close();if(prior===undefined)delete process.env.CATALOG_MEDIA_PATH;else process.env.CATALOG_MEDIA_PATH=prior;rmSync(directory,{recursive:true,force:true});}
});

test('migration retains rejected legacy decisions and checks every conflicting source identity',()=>{
  const repo=openCatalog(':memory:');
  try{
    const source=(id:string)=>({provider:'archiveorg',externalId:id,url:`https://archive.org/details/${id}`});
    const first=migrateLegacyLibrary(repo,[{id:'legacy-first',title:'First',sources:[source('first')],candidates:[{id:'candidate-old',provider:'archiveorg',state:'rejected',fields:{title:'Rejected title'},reasons:['Old suggestion']}]}],'first-batch',{sessionId:'curator'});
    const id=first.legacyIds['legacy-first'];assert.ok(repo.listClaims(id).some(claim=>claim.disposition==='rejected'));
    migrateLegacyLibrary(repo,[{id:'legacy-second',title:'Second',sources:[source('second')]}],'second-batch',{sessionId:'curator'});
    const collision=migrateLegacyLibrary(repo,[{id:'legacy-collision',title:'Collision',sources:[source('first'),source('second')]}],'collision-batch',{sessionId:'curator'});
    assert.equal(collision.rejected,1);assert.ok(collision.outcomes?.[0].errors.some(message=>/source identities/i.test(message)));
    assert.equal(repo.getRecord('legacy-collision'),undefined);
  }finally{repo.close();}
});
