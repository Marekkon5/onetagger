import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import type { EntityRecord, MixRecord } from '@syco23/catalog-domain';
import type { ProviderRegistry } from '../core/registry.js';
import { openCatalog } from './repository.js';
import { enrichCatalogRecord } from './enrich.js';

const timestamp = '2026-10-03T12:00:00.000Z';

async function withCatalog(run: (repo: ReturnType<typeof openCatalog>) => Promise<void>) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-enrich-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const mix: MixRecord = {
    kind: 'mix', id: 'mix_01J9CATALOGUE00000000000040', createdAt: timestamp, updatedAt: timestamp,
    revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Kan10 Live Mackitek Koalisson III',
    durationMs: 2_641_424, people: [{ entityId: 'entity_01J9CATALOGUE0000000040', role: 'artist' }],
    eventIds: [], genres: [], styles: [], assets: [], sources: []
  };
  const artist: EntityRecord = {
    kind: 'entity', id: 'entity_01J9CATALOGUE0000000040', createdAt: timestamp, updatedAt: timestamp,
    revision: 1, verification: 'source-confirmed', reviewState: 'ready', displayName: 'Mackitek',
    roles: ['artist'], aliases: [], assets: [], providerRefs: []
  };
  repo.transaction((tx) => { tx.saveRecord(mix); tx.saveRecord(artist); });
  try {
    await run(repo);
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
}

function registry(candidate: Record<string, unknown>): ProviderRegistry {
  const youtube = {
    id: 'youtube' as const,
    async health() { return { id: 'youtube' as const, state: 'ready' as const, detail: 'ok', checkedAt: timestamp }; },
    async search() { return [{
      provider: 'youtube' as const,
      title: 'Live Mackitek Koalisson III',
      artists: ['Archive Channel'],
      crews: [],
      confidence: 0.82,
      reasons: ['matching title and duration'],
      source: { provider: 'youtube' as const, url: 'https://www.youtube.com/watch?v=vi5miMVpmuI', externalId: 'vi5miMVpmuI' },
      artwork: ['https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg'],
      ...candidate
    }]; }
  };
  return {
    discovery: new Map([['youtube', youtube]]),
    discogs: { async health() { return { id: 'discogs' as const, state: 'offline' as const, detail: 'unavailable', checkedAt: timestamp }; }, async enrichEntity() { return []; } }
  } as unknown as ProviderRegistry;
}

test('confirmed YouTube title and compatible duration fill the cover without promoting the uploader', async () => {
  await withCatalog(async (repo) => {
    const report = await enrichCatalogRecord(repo, registry({ durationMs: 2_650_000 }), 'mix_01J9CATALOGUE00000000000040', { sessionId: 'curator-session' });
    const record = repo.getRecord('mix_01J9CATALOGUE00000000000040');
    assert.equal(report.applied, 1);
    assert.equal(record?.kind === 'mix' ? record.assets[0]?.url : undefined, 'https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg');
    assert.equal(record?.kind === 'mix' ? record.people[0]?.entityId : undefined, 'entity_01J9CATALOGUE0000000040');
    assert.equal(repo.findByProvider({ provider: 'youtube', resourceType: 'video', externalId: 'vi5miMVpmuI' }), record?.id);
  });
});

test('a candidate with no duration remains review material rather than selected artwork', async () => {
  await withCatalog(async (repo) => {
    const report = await enrichCatalogRecord(repo, registry({ durationMs: undefined }), 'mix_01J9CATALOGUE00000000000040', { sessionId: 'curator-session' });
    const record = repo.getRecord('mix_01J9CATALOGUE00000000000040');
    assert.equal(report.applied, 0);
    assert.equal(report.reviewed, 1);
    assert.equal(record?.kind === 'mix' ? record.assets.length : -1, 0);
    assert.equal(repo.listReview().length, 1);
  });
});

test('a completed enrichment run is persisted with its provider results and missing fields', async () => {
  await withCatalog(async (repo) => {
    const report = await enrichCatalogRecord(repo, registry({}), 'mix_01J9CATALOGUE00000000000040', { sessionId: 'curator-session' });
    assert.equal(report.state, 'completed');
    const runs = repo.listEnrichmentRuns('mix_01J9CATALOGUE00000000000040');
    assert.equal(runs.length, 1);
    assert.equal(runs[0].state, 'completed');
    assert.ok(runs[0].finishedAt, 'a finished run records when it completed');
    assert.deepEqual(runs[0].missingFields, report.missingFields);
  });
});

test('a provider failure completes the run with recorded errors and does not roll back other results', async () => {
  await withCatalog(async (repo) => {
    // A single provider being unreachable is not a run failure: the plan requires
    // valid results from the other providers to survive, so the run still completes.
    const degraded = {
      discovery: new Map(),
      discogs: {},
      async search() { throw new Error('provider network unreachable'); }
    } as unknown as ProviderRegistry;
    const report = await enrichCatalogRecord(repo, degraded, 'mix_01J9CATALOGUE00000000000040', { sessionId: 'curator-session' });
    assert.equal(report.state, 'completed');
    assert.equal(repo.listEnrichmentRuns('mix_01J9CATALOGUE00000000000040')[0].state, 'completed');
    assert.ok(report.missingFields.length > 0, 'remaining missing fields are reported honestly');
  });
});

test('a run that cannot finish is recorded as interrupted, hides internal detail and stays retryable', async () => {
  await withCatalog(async (repo) => {
    // A structural failure inside the pass, after the run has been opened. A provider
    // being unreachable is not such a failure: that completes with recorded errors.
    const realGetRecord = repo.getRecord.bind(repo);
    let runOpened = false;
    const broken = new Proxy(repo, {
      get(target, property, receiver) {
        if (property === 'startEnrichmentRun') {
          runOpened = true;
          return Reflect.get(target, property, receiver);
        }
        if (property === 'getRecord') {
          return (id: never) => {
            if (runOpened) throw new Error('database I/O error at /app/data/catalog.sqlite');
            return realGetRecord(id);
          };
        }
        const value = Reflect.get(target, property, receiver);
        return typeof value === 'function' ? value.bind(target) : value;
      }
    });

    const report = await enrichCatalogRecord(broken as unknown as typeof repo, registry({}), 'mix_01J9CATALOGUE00000000000040', { sessionId: 'curator-session' });
    assert.equal(report.state, 'interrupted');
    // The internal path must not reach the client; only a generic message.
    const reported = JSON.stringify(report.errors);
    assert.doesNotMatch(reported, /\/app\/data|catalog\.sqlite/);
    const run = repo.listEnrichmentRuns('mix_01J9CATALOGUE00000000000040')[0];
    assert.equal(run.state, 'interrupted');
    assert.ok(run.finishedAt, 'an interrupted run records when it stopped');

    // The interrupted run must not block a retry: a new run opens afterwards.
    const retry = await enrichCatalogRecord(repo, registry({}), 'mix_01J9CATALOGUE00000000000040', { sessionId: 'curator-session' });
    assert.equal(retry.state, 'completed');
    assert.equal(repo.listEnrichmentRuns('mix_01J9CATALOGUE00000000000040').length, 2);
  });
});

test('worker start closes runs left running by a previous process', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-enrich-stale-'));
  const dbPath = join(directory, 'catalog.sqlite');
  try {
    const first = openCatalog(dbPath);
    const mix: MixRecord = {
      kind: 'mix', id: 'mix_01J9CATALOGUE00000000000060', createdAt: timestamp, updatedAt: timestamp,
      revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Interrupted run',
      durationMs: 1000, people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
    };
    first.transaction((tx) => tx.saveRecord(mix));
    first.startEnrichmentRun({
      id: 'run_stale', recordId: mix.id, attemptedProviders: ['youtube'], actor: 'curator-session', startedAt: timestamp
    });
    first.close();

    // Simulate a restart: the run is still 'running' on disk.
    const reopened = openCatalog(dbPath);
    assert.equal(reopened.listEnrichmentRuns(mix.id)[0].state, 'running');
    assert.equal(reopened.interruptStaleEnrichmentRuns(), 1);
    assert.equal(reopened.listEnrichmentRuns(mix.id)[0].state, 'interrupted');
    // Idempotent: a second sweep finds nothing left to close.
    assert.equal(reopened.interruptStaleEnrichmentRuns(), 0);
    reopened.close();
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('linked Discogs entity IDs hydrate one shared profile and role-specific portrait', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-entity-enrich-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const entity: EntityRecord = {
    kind: 'entity', id: 'entity_01J9CATALOGUE0000000050', createdAt: timestamp, updatedAt: timestamp,
    revision: 1, verification: 'source-confirmed', reviewState: 'ready', displayName: 'Mackitek',
    roles: ['artist'], aliases: [], assets: [], providerRefs: [
      { provider: 'discogs', resourceType: 'artist', externalId: '42', url: 'https://www.discogs.com/artist/42' }
    ]
  };
  try {
    repo.transaction((tx) => {
      tx.saveRecord(entity);
      tx.addProviderSource(entity.id, entity.providerRefs[0]);
    });
    const providerRegistry = {
      discovery: new Map(),
      discogs: {
        async hydrateEntity() {
          return { kind: 'artist', name: 'Mackitek', provider: 'discogs', externalId: '42', url: 'https://www.discogs.com/artist/42', profile: 'Shared artist profile', imageUrl: 'https://i.discogs.com/artist.jpg' };
        }
      }
    } as unknown as ProviderRegistry;
    const report = await enrichCatalogRecord(repo, providerRegistry, entity.id, { sessionId: 'curator-session' });
    const stored = repo.getRecord(entity.id);
    assert.equal(report.applied, 2);
    assert.equal(stored?.kind === 'entity' ? stored.profile : undefined, 'Shared artist profile');
    assert.equal(stored?.kind === 'entity' ? stored.assets[0]?.role : undefined, 'artist-portrait');
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});