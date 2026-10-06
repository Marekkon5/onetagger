import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable } from 'node:stream';
import type { IncomingMessage, ServerResponse } from 'node:http';
import test from 'node:test';
import type { MixRecord } from '@syco23/catalog-domain';
import { openCatalog } from './repository.js';
import { createCuratorAuth, hashCuratorPassword } from '../auth/curator.js';
import { handleCatalogRoute } from './routes.js';
import { applyClaims } from './merge.js';

const timestamp = '2026-10-03T12:00:00.000Z';

function request(method: string, url: string, value?: unknown, headers: Record<string, string> = {}) {
  const input = value === undefined ? [] : [JSON.stringify(value)];
  const req = Readable.from(input) as IncomingMessage;
  req.method = method;
  req.url = url;
  req.headers = headers;
  Object.defineProperty(req, 'socket', { value: { remoteAddress: '127.0.0.1' } });
  return req;
}

function response() {
  const result: { status?: number; headers?: Record<string, string>; body?: string } = {};
  const res = {
    setHeader(name: string, value: string) { result.headers = { ...result.headers, [name]: value }; },
    writeHead(status: number, headers: Record<string, string>) { result.status = status; result.headers = headers; return this; },
    end(body = '') { result.body = body; }
  } as unknown as ServerResponse;
  return { res, result };
}

function mix(): MixRecord {
  return {
    kind: 'mix', id: 'mix_01J9CATALOGUE00000000000020', createdAt: timestamp, updatedAt: timestamp,
    revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Original mix',
    people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
  };
}

test('catalog indexes are public while review is curator-only', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  try {
    repo.transaction((tx) => tx.saveRecord(mix()));
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };
    const publicResponse = response();
    await handleCatalogRoute(context, request('GET', '/api/catalog/mix?page=1&pageSize=10'), publicResponse.res);
    assert.equal(publicResponse.result.status, 200);
    assert.equal(JSON.parse(publicResponse.result.body ?? '{}').items[0].id, mix().id);

    const privateResponse = response();
    await handleCatalogRoute(context, request('GET', '/api/catalog/review'), privateResponse.res);
    assert.equal(privateResponse.result.status, 401);
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('curator edits use revision checks and never accept revision fields from the patch', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  try {
    repo.transaction((tx) => tx.saveRecord(mix()));
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const context = { repository: repo, auth };
    const accepted = response();
    await handleCatalogRoute(context, request('PATCH', `/api/catalog/records/${mix().id}`, {
      expectedRevision: 1,
      patch: { title: 'Curated mix', revision: 99 }
    }, { cookie, origin: 'http://localhost:5173' }), accepted.res);
    assert.equal(accepted.result.status, 400);

    const changed = response();
    await handleCatalogRoute(context, request('PATCH', `/api/catalog/records/${mix().id}`, {
      expectedRevision: 1,
      patch: { title: 'Curated mix' }
    }, { cookie, origin: 'http://localhost:5173' }), changed.res);
    assert.equal(changed.result.status, 200);
    assert.equal(JSON.parse(changed.result.body ?? '{}').revision, 2);

    const stale = response();
    await handleCatalogRoute(context, request('PATCH', `/api/catalog/records/${mix().id}`, {
      expectedRevision: 1,
      patch: { title: 'Stale write' }
    }, { cookie, origin: 'http://localhost:5173' }), stale.res);
    assert.equal(stale.result.status, 409);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('catalog import requires curator access and is idempotent across retries', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  try {
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };
    const input = {
      provider: 'youtube', title: 'Live Mackitek Koalisson III', artists: ['Channel Name'], crews: [],
      artwork: ['https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg'], confidence: 0.85,
      reasons: ['selected source'], source: {
        provider: 'youtube', resourceType: 'video', externalId: 'vi5miMVpmuI',
        url: 'https://www.youtube.com/watch?v=vi5miMVpmuI'
      }
    };
    const denied = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/import', input, { origin: 'http://localhost:5173' }), denied.res);
    assert.equal(denied.result.status, 401);

    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const created = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/import', input, { cookie, origin: 'http://localhost:5173' }), created.res);
    assert.equal(created.result.status, 201);
    assert.equal(JSON.parse(created.result.body ?? '{}').record.people.length, 0);

    const retried = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/import', input, { cookie, origin: 'http://localhost:5173' }), retried.res);
    assert.equal(retried.result.status, 200);
    assert.equal(JSON.parse(retried.result.body ?? '{}').record.id, JSON.parse(created.result.body ?? '{}').record.id);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('proposed identity details are private until their identity is confirmed', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  try {
    const event = {
      kind: 'event' as const, id: 'event_01J9CATALOGUE00000000000020',
      createdAt: timestamp, updatedAt: timestamp, revision: 1, verification: 'proposed' as const,
      reviewState: 'review' as const, name: 'Aniane Free Party', assets: [], sourceUrls: [], mixIds: []
    };
    repo.transaction((tx) => tx.saveRecord(event));
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };
    const publicResponse = response();
    await handleCatalogRoute(context, request('GET', `/api/catalog/records/${event.id}`), publicResponse.res);
    assert.equal(publicResponse.result.status, 404);

    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const curatorResponse = response();
    await handleCatalogRoute(context, request('GET', `/api/catalog/records/${event.id}`, undefined, { cookie }), curatorResponse.res);
    assert.equal(curatorResponse.result.status, 200);
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('legacy migration requires curator access and returns a stable batch result', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  try {
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };
    const input = { batchId: 'browser-migration-1', records: [{ id: 'legacy-route-1', title: 'Archive mix', sources: [] }] };
    const denied = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/migrate', input, { origin: 'http://localhost:5173' }), denied.res);
    assert.equal(denied.result.status, 401);

    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const created = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/migrate', input, { cookie, origin: 'http://localhost:5173' }), created.res);
    assert.equal(created.result.status, 200);
    const first = JSON.parse(created.result.body ?? '{}');
    assert.equal(first.imported, 1);

    const retried = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/migrate', input, { cookie, origin: 'http://localhost:5173' }), retried.res);
    assert.deepEqual(JSON.parse(retried.result.body ?? '{}'), first);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('curator waveform upload persists PNG bytes and public media GET serves the linked asset', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-waveform-route-'));
  const priorOrigin = process.env.CORS_ORIGIN;
  const priorMediaPath = process.env.CATALOG_MEDIA_PATH;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  process.env.CATALOG_MEDIA_PATH = join(directory, 'media');
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a8ioAAAAASUVORK5CYII=', 'base64');
  try {
    repo.transaction((tx) => tx.saveRecord(mix()));
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const context = { repository: repo, auth };
    const uploadRequest = Readable.from([png]) as IncomingMessage;
    uploadRequest.method = 'PUT';
    uploadRequest.url = `/api/catalog/records/${mix().id}/waveform?sourceUrl=${encodeURIComponent('https://archive.org/audio.mp3')}`;
    uploadRequest.headers = { cookie, origin: 'http://localhost:5173', 'content-type': 'image/png' };
    Object.defineProperty(uploadRequest, 'socket', { value: { remoteAddress: '127.0.0.1' } });
    const uploaded = response();
    await handleCatalogRoute(context, uploadRequest, uploaded.res);
    assert.equal(uploaded.result.status, 200);
    const result = JSON.parse(uploaded.result.body ?? '{}');
    const mediaId = result.assets.find((asset: { role: string }) => asset.role === 'waveform').mediaId;

    const mediaResponse = { status: 0, headers: {} as Record<string, string>, bytes: Buffer.alloc(0) };
    const getRequest = request('GET', `/api/catalog/media/${mediaId}`);
    const getResponse = {
      writeHead(status: number, headers: Record<string, string>) { mediaResponse.status = status; mediaResponse.headers = headers; return this; },
      end(bytes: Buffer) { mediaResponse.bytes = Buffer.from(bytes); }
    } as unknown as ServerResponse;
    await handleCatalogRoute(context, getRequest, getResponse);
    assert.equal(mediaResponse.status, 200);
    assert.equal(mediaResponse.headers['content-type'], 'image/png');
    assert.deepEqual(mediaResponse.bytes, png);
  } finally {
    if (priorOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = priorOrigin;
    if (priorMediaPath === undefined) delete process.env.CATALOG_MEDIA_PATH;
    else process.env.CATALOG_MEDIA_PATH = priorMediaPath;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('an entity page lists the mixes that reference it, including imported ones', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-related-mixes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  const entityId = 'entity_01J9CATALOGUE00000000000070';
  try {
    repo.transaction((tx) => {
      tx.saveRecord({
        kind: 'entity', id: entityId, createdAt: timestamp, updatedAt: timestamp, revision: 1,
        verification: 'source-confirmed', reviewState: 'ready', displayName: 'Mackitek',
        roles: ['artist'], aliases: [], assets: [], providerRefs: []
      });
      tx.saveRecord({ ...mix(), people: [{ entityId, role: 'artist' }] });
      // A second mix for another artist must not leak into this entity's page.
      tx.saveRecord({ ...mix(), id: 'mix_01J9CATALOGUE00000000000031', title: 'Unrelated set' });
    });
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };

    const response_ = response();
    await handleCatalogRoute(context, request('GET', `/api/catalog/records/${entityId}/related-mixes`), response_.res);
    assert.equal(response_.result.status, 200);
    const body = JSON.parse(response_.result.body ?? '{}');
    assert.equal(body.recordId, entityId);
    assert.deepEqual(body.mixes.map((item: { title: string }) => item.title), ['Original mix']);

    // A mix has no inbound entity references; the shape stays stable and empty.
    const mixResponse = response();
    await handleCatalogRoute(context, request('GET', `/api/catalog/records/${mix().id}/related-mixes`), mixResponse.res);
    assert.deepEqual(JSON.parse(mixResponse.result.body ?? '{}').mixes, []);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('field evidence is public and explains why a selected value holds its field', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-evidence-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  try {
    repo.transaction((tx) => {
      tx.saveRecord(mix());
      tx.addProviderSource(mix().id, { provider: 'youtube', resourceType: 'video', externalId: 'video-1', url: 'https://www.youtube.com/watch?v=video-1' });
    });
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };

    // A confirmed identity with a linked source fills a missing description directly.
    applyClaims(repo, [{
      targetRecordId: mix().id,
      field: 'description',
      value: 'Confirmed from the linked source',
      provider: { provider: 'youtube', resourceType: 'video', externalId: 'video-1' },
      sourceUrl: 'https://www.youtube.com/watch?v=video-1',
      observedAt: timestamp,
      evidence: 'direct',
      matchExplanation: 'Linked source identity with compatible duration'
    }]);

    // A public reader may see why a field holds its value without a curator session.
    const publicResponse = response();
    await handleCatalogRoute(context, request('GET', `/api/catalog/records/${mix().id}/evidence`), publicResponse.res);
    assert.equal(publicResponse.result.status, 200);
    const body = JSON.parse(publicResponse.result.body ?? '{}');
    assert.equal(body.recordId, mix().id);
    const selected = body.evidence.find((item: { disposition: string }) => item.disposition === 'selected');
    assert.ok(selected, 'the selected claim is returned');
    assert.equal(selected.claim.field, 'description');
    assert.equal(selected.claim.value, 'Confirmed from the linked source');
    assert.equal(selected.claim.provider.provider, 'youtube');

    // A proposed record is not public, and neither is its evidence.
    repo.transaction((tx) => tx.saveRecord({ ...mix(), id: 'mix_01J9CATALOGUE00000000000022', title: 'Proposal', verification: 'proposed' }));
    const hidden = response();
    await handleCatalogRoute(context, request('GET', '/api/catalog/records/mix_01J9CATALOGUE00000000000022/evidence'), hidden.res);
    assert.equal(hidden.result.status, 404);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('curator duplicate merge is authenticated, revision-checked and redirects the old id', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-merge-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  const duplicate = { ...mix(), id: 'mix_01J9CATALOGUE00000000000021', title: 'Duplicate import' };
  try {
    repo.transaction((tx) => { tx.saveRecord(mix()); tx.saveRecord(duplicate); });
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const context = { repository: repo, auth };

    // Public readers must not be able to collapse two records.
    const denied = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/merge', {
      survivor: mix().id, duplicate: duplicate.id, expectedRevisions: [1, 1]
    }, { origin: 'http://localhost:5173' }), denied.res);
    assert.equal(denied.result.status, 401);

    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const merged = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/merge', {
      survivor: mix().id, duplicate: duplicate.id, expectedRevisions: [1, 1]
    }, { cookie, origin: 'http://localhost:5173' }), merged.res);
    assert.equal(merged.result.status, 200);
    assert.equal(JSON.parse(merged.result.body ?? '{}').id, mix().id);

    // The retired id must still resolve, so an already shared link keeps working.
    const legacy = response();
    await handleCatalogRoute(context, request('GET', `/api/catalog/records/${duplicate.id}`), legacy.res);
    assert.equal(legacy.result.status, 200);
    assert.equal(JSON.parse(legacy.result.body ?? '{}').id, mix().id);

    // Replaying the same merge with the revisions it was given is refused.
    const replay = response();
    await handleCatalogRoute(context, request('POST', '/api/catalog/merge', {
      survivor: mix().id, duplicate: duplicate.id, expectedRevisions: [1, 1]
    }, { cookie, origin: 'http://localhost:5173' }), replay.res);
    assert.ok(replay.result.status === 404 || replay.result.status === 409, `expected 404 or 409, got ${replay.result.status}`);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('catalog domain failures keep their HTTP status and never leak internal detail', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-catalog-errors-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const previousOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  try {
    repo.transaction((tx) => tx.saveRecord(mix()));
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const cookie = auth.login('private-local-password', '127.0.0.1').split(';')[0];
    const headers = { cookie, origin: 'http://localhost:5173' };

    // A body that is declared as PNG but is not one is a bad request the curator can
    // act on, so the specific reason must survive instead of a generic 500.
    const notPng = Readable.from([Buffer.from('this is definitely not a PNG')]) as IncomingMessage;
    notPng.method = 'PUT';
    notPng.url = `/api/catalog/records/${mix().id}/waveform?sourceUrl=${encodeURIComponent('https://archive.org/audio.mp3')}`;
    notPng.headers = { ...headers, 'content-type': 'image/png' };
    Object.defineProperty(notPng, 'socket', { value: { remoteAddress: '127.0.0.1' } });
    const dangling = response();
    await handleCatalogRoute({ repository: repo, auth }, notPng, dangling.res);
    assert.equal(dangling.result.status, 400);
    assert.match(JSON.parse(dangling.result.body ?? '{}').error, /PNG/i);

    // An internal fault must never be echoed back, only logged. Migration is not the
    // right probe here: it deliberately absorbs per-record failures as `rejected`.
    const broken = new Proxy(repo, {
      get(target, property, receiver) {
        if (property === 'transaction') {
          return () => { throw new Error('Internal server error'); };
        }
        const value = Reflect.get(target, property, receiver);
        return typeof value === 'function' ? value.bind(target) : value;
      }
    });
    const internal = response();
    await handleCatalogRoute({ repository: broken as unknown as typeof repo, auth }, request('PATCH', `/api/catalog/records/${mix().id}`, {
      expectedRevision: 1,
      patch: { title: 'Internal fault' }
    }, headers), internal.res);
    assert.equal(internal.result.status, 500);
    assert.equal(JSON.parse(internal.result.body ?? '{}').error, 'Request failed');
    assert.doesNotMatch(internal.result.body ?? '', /SQLITE|private key|\/app\/data/);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});