import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { openCatalog } from './repository.js';
import type { EntityRecord, MixRecord, ProviderRef } from '@syco23/catalog-domain';

const timestamp = '2026-10-03T12:00:00.000Z';

function mix(id: string): MixRecord {
  return {
    kind: 'mix', id, createdAt: timestamp, updatedAt: timestamp, revision: 1,
    verification: 'source-confirmed', reviewState: 'ready', title: 'Live Mackitek Koalisson III',
    people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
  };
}

function entity(id: string, verification: EntityRecord['verification'], roles: EntityRecord['roles']): EntityRecord {
  return {
    kind: 'entity', id, createdAt: timestamp, updatedAt: timestamp, revision: 1,
    verification, reviewState: 'ready', displayName: 'Mackitek', roles, aliases: [], assets: [], providerRefs: []
  };
}

function withCatalog(run: (path: string) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-'));
  const path = join(directory, 'catalog.sqlite');
  try {
    run(path);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test('records, provider identities and relationships survive a database reopen', () => {
  withCatalog((path) => {
    const provider: ProviderRef = { provider: 'youtube', resourceType: 'video', externalId: 'same-id', url: 'https://www.youtube.com/watch?v=same-id' };
    let repo = openCatalog(path);
    repo.transaction((tx) => {
      tx.saveRecord(mix('mix_01J9CATALOGUE00000000000001'));
      tx.saveRecord(entity('entity_01J9CATALOGUE0000000001', 'source-confirmed', ['artist']));
      tx.addProviderSource('mix_01J9CATALOGUE00000000000001', provider);
      tx.addRelationship('mix_01J9CATALOGUE00000000000001', 'entity_01J9CATALOGUE0000000001', 'artist');
    });
    repo.close();

    repo = openCatalog(path);
    assert.equal(repo.findByProvider(provider), 'mix_01J9CATALOGUE00000000000001');
    assert.equal(repo.getRecord('mix_01J9CATALOGUE00000000000001')?.kind, 'mix');
    assert.equal(repo.findNameCandidates('artist', '  MACKITEK  ').length, 1);
    repo.close();
  });
});

test('equal external IDs in different provider resource namespaces stay distinct', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    const artistRef: ProviderRef = { provider: 'discogs', resourceType: 'artist', externalId: '42' };
    const labelRef: ProviderRef = { provider: 'discogs', resourceType: 'label', externalId: '42' };
    repo.transaction((tx) => {
      tx.saveRecord(entity('entity_01J9CATALOGUE0000000002', 'source-confirmed', ['artist']));
      tx.saveRecord(entity('entity_01J9CATALOGUE0000000003', 'source-confirmed', ['label']));
      tx.addProviderSource('entity_01J9CATALOGUE0000000002', artistRef);
      tx.addProviderSource('entity_01J9CATALOGUE0000000003', labelRef);
    });
    assert.equal(repo.findByProvider(artistRef), 'entity_01J9CATALOGUE0000000002');
    assert.equal(repo.findByProvider(labelRef), 'entity_01J9CATALOGUE0000000003');
    repo.close();
  });
});

test('source identity uniqueness, role constraints and dangling relationships are enforced', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    const ref: ProviderRef = { provider: 'youtube', resourceType: 'video', externalId: 'duplicate' };
    repo.transaction((tx) => {
      tx.saveRecord(mix('mix_01J9CATALOGUE00000000000002'));
      tx.saveRecord(mix('mix_01J9CATALOGUE00000000000003'));
      tx.addProviderSource('mix_01J9CATALOGUE00000000000002', ref);
    });
    assert.throws(() => repo.transaction((tx) => tx.addProviderSource('mix_01J9CATALOGUE00000000000003', ref)));
    assert.throws(() => repo.transaction((tx) => tx.addRelationship('mix_01J9CATALOGUE00000000000002', 'entity_01J9CATALOGUE0000000099', 'artist')));
    repo.close();
  });
});

test('public role indexes omit proposed identities', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    repo.transaction((tx) => {
      tx.saveRecord(entity('entity_01J9CATALOGUE0000000004', 'source-confirmed', ['artist']));
      tx.saveRecord(entity('entity_01J9CATALOGUE0000000005', 'proposed', ['artist']));
    });
    const page = repo.listIndex('artist', { page: 1, pageSize: 25 });
    assert.deepEqual(page.items.map((record) => record.id), ['entity_01J9CATALOGUE0000000004']);
    repo.close();
  });
});

test('legacy IDs redirect and failed transactions roll back every write', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    const original = mix('mix_01J9CATALOGUE00000000000004');
    repo.transaction((tx) => {
      tx.saveRecord(original);
      tx.addLegacyAlias('old-mix-link', original.id);
    });
    const changed = { ...original, title: 'Changed title', revision: 2, updatedAt: '2026-10-03T13:00:00.000Z' };
    assert.throws(() => repo.transaction((tx) => {
      tx.saveRecord(changed, 1);
      tx.addRelationship(original.id, 'entity_missing', 'artist');
    }));
    assert.equal(repo.getRecord('old-mix-link')?.id, original.id);
    const stored = repo.getRecord(original.id);
    assert.equal(stored?.kind === 'mix' ? stored.title : undefined, original.title);
    assert.equal(repo.getRecord(original.id)?.revision, 1);
    repo.close();
  });
});

test('index search is paginated and matches normalized names', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    repo.transaction((tx) => {
      tx.saveRecord(mix('mix_01J9CATALOGUE00000000000005'));
      tx.saveRecord(mix('mix_01J9CATALOGUE00000000000006'));
    });
    const page = repo.listIndex('mix', { page: 2, pageSize: 1, query: '  LIVE ' });
    assert.equal(page.total, 2);
    assert.equal(page.items.length, 1);
    assert.equal(page.page, 2);
    repo.close();
  });
});
test('typed details, aliases, field selections and arrays have durable normalized relation rows',async()=>{
  const {DatabaseSync}=await import('node:sqlite');
  withCatalog(path=>{
    const repo=openCatalog(path);
    repo.transaction(tx=>{
      tx.saveRecord({...entity('entity_normalized','source-confirmed',['artist']),aliases:['DJ Live'],artist:{realName:'Real Person',profile:'Sourced profile'},assets:[{role:'artist-portrait',url:'https://example.org/portrait.png',source:'discogs'}]});
      tx.saveRecord({...mix('mix_normalized'),genres:['Tekno','tekno'],people:[{entityId:'entity_normalized',role:'artist'}]});
    });repo.close();
    const db=new DatabaseSync(path,{readOnly:true});try{
      assert.equal((db.prepare('SELECT real_name FROM entity_details WHERE record_id=? AND role=?').get('entity_normalized','artist') as {real_name:string}).real_name,'Real Person');
      assert.equal((db.prepare('SELECT count(*) AS total FROM record_terms WHERE record_id=?').get('mix_normalized') as {total:number}).total,1);
      assert.equal((db.prepare('SELECT count(*) AS total FROM entity_aliases').get() as {total:number}).total,1);
      assert.equal((db.prepare('SELECT count(*) AS total FROM record_relationships WHERE source_id=?').get('mix_normalized') as {total:number}).total,1);
    }finally{db.close();}
  });
});
