import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable } from 'node:stream';
import type { IncomingMessage, ServerResponse } from 'node:http';
import test from 'node:test';
import { CatalogRecordSchema, type ImportMetadataFacts } from '@syco23/catalog-domain';
import { openCatalog } from '../catalog/repository.js';
import { createCuratorAuth, hashCuratorPassword } from '../auth/curator.js';
import { applyCorsHeaders } from '../http.js';
import { handleImportRoute, type ImportRouteContext } from './routes.js';
import { signWorkerRequest } from './service-auth.js';
import { createImportStore, objectKey, sha256 } from './storage.js';

const ORIGIN = 'https://mixsets.example.org';
const WORKER_ID = 'worker-1';

function mockRequest(method: string, url: string, value?: unknown, headers: Record<string, string> = {}) {
  const input = value === undefined ? [] : [JSON.stringify(value)];
  const request = Readable.from(input) as IncomingMessage;
  request.method = method;
  request.url = url;
  request.headers = headers;
  Object.defineProperty(request, 'socket', { value: { remoteAddress: '127.0.0.1' } });
  return request;
}

function mockResponse() {
  const captured: { status?: number; headers?: Record<string, string>; body?: string } = {};
  const response = {
    setHeader(name: string, value: string) { captured.headers = { ...captured.headers, [name]: value }; },
    writeHead(status: number, headers: Record<string, string>) { captured.status = status; captured.headers = headers; return this; },
    end(body = '') { captured.body = body; }
  } as unknown as ServerResponse;
  return { response, captured };
}

function setup(importsEnabled = true) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-import-routes-'));
  const repository = openCatalog(join(directory, 'catalog.sqlite'));
  const priorOrigin = process.env.CORS_ORIGIN;
  const priorSecret = process.env.IMPORT_WORKER_SECRET;
  const priorStorage = process.env.IMPORT_STORAGE_LOCAL_ROOT;
  process.env.CORS_ORIGIN = ORIGIN;
  process.env.IMPORT_WORKER_SECRET = 'test-worker-secret';
  process.env.IMPORT_STORAGE_LOCAL_ROOT = join(directory, 'imports');
  const auth = createCuratorAuth(repository, hashCuratorPassword('private-local-password'));
  const cookie = auth.login('private-local-password', '127.0.0.1');
  const context: ImportRouteContext = { repository, auth, importsEnabled };
  const cleanup = () => {
    if (priorOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = priorOrigin;
    if (priorSecret === undefined) delete process.env.IMPORT_WORKER_SECRET;
    else process.env.IMPORT_WORKER_SECRET = priorSecret;
    if (priorStorage === undefined) delete process.env.IMPORT_STORAGE_LOCAL_ROOT;
    else process.env.IMPORT_STORAGE_LOCAL_ROOT = priorStorage;
    repository.close();
    rmSync(directory, { recursive: true, force: true });
  };
  return { context, cookie, cleanup, repository };
}

const metadataRequest = {
  source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
  mode: 'metadata',
};

const audioRequest = {
  source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
  mode: 'audio',
  conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
  // Deliberately the metadata-only basis: the global policy must override it.
  rights: { basis: 'provider_metadata_only', attestationVersion: '2026-10' },
};

const facts: ImportMetadataFacts = {
  provider: 'soundcloud',
  sourceUrl: 'https://soundcloud.com/example/example',
  title: 'Real SoundCloud Set',
  uploader: 'Example Channel',
  description: 'A sourced description from the provider.',
  durationMs: 3_600_000,
  genres: ['tekno'],
  artworkUrl: 'https://i1.sndcdn.com/artworks-example-large.jpg',
  externalId: 'example',
  observedAt: '2026-10-06T12:00:00.000Z',
};

function workerHeaders(method: string, path: string, raw: string, worker = WORKER_ID) {
  const timestamp = String(Math.floor(Date.now() / 1000));
  return {
    'x-import-worker-timestamp': timestamp,
    'x-import-worker-signature': signWorkerRequest('test-worker-secret', timestamp, method, path, raw),
    'x-import-worker-scope': 'import_worker',
    'x-import-worker-id': worker,
  };
}

async function workerGet(context: ImportRouteContext, path: string, worker = WORKER_ID) {
  const response = mockResponse();
  await handleImportRoute(context, mockRequest('GET', path, undefined, workerHeaders('GET', path, '', worker)), response.response);
  return response;
}

async function workerPost(context: ImportRouteContext, path: string, body: Record<string, unknown>, worker = WORKER_ID) {
  const raw = JSON.stringify(body);
  const response = mockResponse();
  await handleImportRoute(context, mockRequest('POST', path, body, workerHeaders('POST', path, raw, worker)), response.response);
  return response;
}

async function createMetadataJob(context: ImportRouteContext, cookie: string, key: string): Promise<string> {
  const created = mockResponse();
  await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
    origin: ORIGIN, cookie, 'idempotency-key': key
  }), created.response);
  assert.equal(created.captured.status, 202);
  return JSON.parse(created.captured.body ?? '{}').job.id as string;
}

async function claimJob(context: ImportRouteContext, jobId: string): Promise<void> {
  const claimed = await workerGet(context, '/internal/imports/next');
  assert.equal(claimed.captured.status, 200);
  assert.equal(JSON.parse(claimed.captured.body ?? '{}').job.id, jobId);
}

async function writeMetadataArtifact(context: ImportRouteContext, jobId: string, value: ImportMetadataFacts = facts) {
  const bytes = Buffer.from(JSON.stringify(value), 'utf8');
  const digest = sha256(bytes);
  const key = objectKey(jobId, 'metadata', digest, 'json');
  await createImportStore().putPrivate(key, bytes, digest);
  const registered = await workerPost(context, `/internal/imports/${jobId}/artifacts`, {
    role: 'metadata', objectKey: key, sha256: digest,
    mimeType: 'application/json', sizeBytes: bytes.length,
  });
  assert.equal(registered.captured.status, 201);
  return { digest, key, bytes };
}

function titleScore(value: string, decision: 'auto_apply' | 'reject' = 'auto_apply') {
  return {
    field: 'title',
    value,
    score: decision === 'reject' ? 0 : 96,
    algorithmVersion: 'evidence-v1',
    components: { I: decision === 'reject' ? 0 : 1, T: 0.9, D: 1, R: 0.9, P: 1, C: 1, X: decision === 'reject' ? 1 : 0 },
    hardGates: { identity: decision !== 'reject', rights: true, noConflict: decision !== 'reject', policy: true },
    decision,
    evaluatedAt: '2026-10-06T12:00:00.000Z',
  };
}

test('version handshake exposes capabilities and import gate', async () => {
  const { context, cleanup } = setup(false);
  try {
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('GET', '/api/version'), response.response);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.equal(response.captured.status, 200);
    assert.equal(payload.importsEnabled, false);
    assert.equal(payload.providers.soundcloud.audio, true);
    assert.equal(payload.providers.soundcloud.metadata, true);
    assert.equal(payload.providers.mixcloud.metadata, true);
    assert.equal(payload.providers.mixcloud.audio, true);
    assert.equal(payload.providers.hearthis.audio, true);
    assert.equal(payload.providers.discogs.metadata, true);
    assert.equal(payload.providers.musicbrainz.metadata, true);
    assert.equal(payload.rightsPolicy.globalConfirmation, true);
  } finally { cleanup(); }
});

test('CORS preflight allows the idempotency-key header', () => {
  const prior = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = ORIGIN;
  try {
    const request = mockRequest('OPTIONS', '/api/imports', undefined, { origin: ORIGIN });
    const response = mockResponse();
    applyCorsHeaders(request, response.response);
    assert.match(response.captured.headers?.['access-control-allow-headers'] ?? '', /idempotency-key/);
  } finally {
    if (prior === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = prior;
  }
});

test('the global rights policy overrides a per-mixset metadata-only basis', async () => {
  const prior = process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION;
  delete process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION;
  const { context, cookie, cleanup } = setup(true);
  try {
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', audioRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-sc-audio-1'
    }), response.response);
    assert.equal(response.captured.status, 202);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.equal(payload.job.state, 'queued');
    assert.equal(payload.policy.allowed, true);
    assert.ok(context.repository.claimDueImportJob('worker-x'), 'audio job is queued for the worker');
  } finally {
    if (prior === undefined) delete process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION;
    else process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION = prior;
    cleanup();
  }
});

test('disabling the global rights policy requires an explicit request basis', async () => {
  const prior = process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION;
  process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION = 'false';
  const { context, cookie, cleanup } = setup(true);
  try {
    const denied = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', audioRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-no-global-1'
    }), denied.response);
    assert.equal(denied.captured.status, 202);
    const deniedPayload = JSON.parse(denied.captured.body ?? '{}');
    assert.equal(deniedPayload.job.state, 'blocked_policy');
    assert.equal(deniedPayload.policy.allowed, false);

    const accepted = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', {
      ...audioRequest,
      rights: { basis: 'licensed_archive', attestationVersion: '2026-10' },
    }, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-no-global-2'
    }), accepted.response);
    const acceptedPayload = JSON.parse(accepted.captured.body ?? '{}');
    assert.equal(acceptedPayload.job.state, 'queued');
    assert.equal(acceptedPayload.policy.allowed, true);
  } finally {
    if (prior === undefined) delete process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION;
    else process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION = prior;
    cleanup();
  }
});

test('user-upload audio jobs persist a private URN and never a filesystem path', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', {
      source: { provider: 'user_upload', uploadId: 'upl_123' },
      mode: 'audio',
      conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
      rights: { basis: 'user_authorized_copy', attestationVersion: '2026-10' },
    }, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-upload-urn-1'
    }), response.response);
    assert.equal(response.captured.status, 202);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.equal(payload.job.state, 'created');
    assert.equal(payload.job.sourceUrl, 'urn:syco23:upload:upl_123');
    assert.equal(payload.policy.allowed, true);
    assert.match(payload.job.sourceUrl, /^urn:syco23:upload:/);
    assert.doesNotMatch(payload.job.sourceUrl, /^(\/|file:)/);
  } finally { cleanup(); }
});

test('soundcloud metadata import is queued and idempotent', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const first = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-sc-meta-1'
    }), first.response);
    assert.equal(first.captured.status, 202);
    const created = JSON.parse(first.captured.body ?? '{}');
    assert.equal(created.job.state, 'queued');
    assert.equal(created.policy.allowed, true);

    const repeat = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-sc-meta-1'
    }), repeat.response);
    assert.equal(repeat.captured.status, 200);
    const again = JSON.parse(repeat.captured.body ?? '{}');
    assert.equal(again.job.id, created.job.id);
  } finally { cleanup(); }
});

test('import creation without idempotency key or curator session is rejected', async () => {
  const { context, cleanup } = setup(true);
  try {
    const noKey = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie: 'syco23_curator=forged'
    }), noKey.response);
    assert.equal(noKey.captured.status, 401);

    const { context: ctx2, cookie: cookie2, cleanup: cleanup2 } = setup(true);
    try {
      const missing = mockResponse();
      await handleImportRoute(ctx2, mockRequest('POST', '/api/imports', metadataRequest, {
        origin: ORIGIN, cookie: cookie2
      }), missing.response);
      assert.equal(missing.captured.status, 400);
    } finally { cleanup2(); }
  } finally { cleanup(); }
});

test('imports stay dark while disabled', async () => {
  const { context, cookie, cleanup } = setup(false);
  try {
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-disabled-1'
    }), response.response);
    assert.equal(response.captured.status, 503);
  } finally { cleanup(); }
});

test('worker endpoints require a valid HMAC service identity', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-worker-flow-1');

    const anonymous = mockResponse();
    await handleImportRoute(context, mockRequest('GET', `/internal/imports/${jobId}/work`), anonymous.response);
    assert.equal(anonymous.captured.status, 401);

    const forged = mockResponse();
    const forgedTimestamp = String(Math.floor(Date.now() / 1000));
    await handleImportRoute(context, mockRequest('GET', `/internal/imports/${jobId}/work`, undefined, {
      'x-import-worker-timestamp': forgedTimestamp,
      'x-import-worker-signature': signWorkerRequest('wrong-secret', forgedTimestamp, 'GET', `/internal/imports/${jobId}/work`, ''),
      'x-import-worker-scope': 'import_worker',
    }), forged.response);
    assert.equal(forged.captured.status, 401);

    const valid = await workerGet(context, `/internal/imports/${jobId}/work`);
    assert.equal(valid.captured.status, 200);
    const spec = JSON.parse(valid.captured.body ?? '{}');
    assert.equal(spec.job.id, jobId);
    assert.ok(spec.limits.maxInputBytes > 0);
    assert.equal(spec.capabilities.soundcloud.audio, true);
    assert.equal(spec.capabilities.musicbrainz.metadata, true);
  } finally { cleanup(); }
});

test('worker mutations require the current lease owner', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-lease-owner-1');

    // A valid worker credential that does not hold the lease cannot mutate.
    const beforeClaim = await workerPost(context, `/internal/imports/${jobId}/events`, { type: 'progress' }, 'worker-2');
    assert.equal(beforeClaim.captured.status, 409);

    await claimJob(context, jobId);
    const outsider = await workerPost(context, `/internal/imports/${jobId}/events`, { type: 'progress' }, 'worker-2');
    assert.equal(outsider.captured.status, 409);

    const owner = await workerPost(context, `/internal/imports/${jobId}/events`, { type: 'progress' });
    assert.equal(owner.captured.status, 204);
  } finally { cleanup(); }
});

test('worker can report progress, artifacts, evidence and completion', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-worker-report-1');
    await claimJob(context, jobId);

    const event = await workerPost(context, `/internal/imports/${jobId}/events`, { type: 'progress', payload: { stage: 'resolving' }, state: 'resolving' });
    assert.equal(event.captured.status, 204);

    await writeMetadataArtifact(context, jobId);
    const evidence = await workerPost(context, `/internal/imports/${jobId}/evidence`, { scores: [titleScore(facts.title)] });
    assert.equal(evidence.captured.status, 201);

    const complete = await workerPost(context, `/internal/imports/${jobId}/complete`, { state: 'completed', attempt: 1 });
    assert.equal(complete.captured.status, 200);
    const view = JSON.parse(complete.captured.body ?? '{}');
    assert.equal(view.job.state, 'completed');
    assert.equal(view.artifacts.length, 1);
    assert.equal(view.evidence[0].score, 96);
    assert.ok(view.evidence[0].hardGates, 'detail payload exposes hardGates');
    assert.equal(view.evidence[0].value, facts.title);
    assert.equal(context.repository.claimDueImportJob('worker-1'), undefined);
  } finally { cleanup(); }
});

test('non-retryable worker failures stay failed instead of re-queueing', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-non-retry-1');
    await claimJob(context, jobId);

    const response = await workerPost(context, `/internal/imports/${jobId}/complete`, {
      state: 'failed', attempt: 1, retryable: false, error: 'ffmpeg failed', errorCode: 'non_retryable'
    });
    assert.equal(response.captured.status, 200);
    const view = JSON.parse(response.captured.body ?? '{}');
    assert.equal(view.job.state, 'failed');
    assert.equal(view.job.errorCode, 'non_retryable');
    assert.equal(context.repository.claimDueImportJob('worker-1'), undefined);
  } finally { cleanup(); }
});

test('provider finalize refuses jobs without a readable metadata snapshot', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-finalize-missing-1');
    await claimJob(context, jobId);
    const complete = await workerPost(context, `/internal/imports/${jobId}/complete`, { state: 'completed', attempt: 1 });
    assert.equal(complete.captured.status, 200);

    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, {}, {
      origin: ORIGIN, cookie,
    }), response.response);
    assert.equal(response.captured.status, 409);
    assert.match(JSON.parse(response.captured.body ?? '{}').error, /metadata snapshot/i);
  } finally { cleanup(); }
});

test('metadata finalize uses the worker facts for the real title and claims', async () => {
  const { context, cookie, cleanup, repository } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-finalize-1');
    await claimJob(context, jobId);
    await writeMetadataArtifact(context, jobId);
    await workerPost(context, `/internal/imports/${jobId}/evidence`, {
      scores: [titleScore(facts.title)],
    });
    await workerPost(context, `/internal/imports/${jobId}/complete`, { state: 'completed', attempt: 1 });

    // Detail polling exposes hardGates and values.
    const detail = mockResponse();
    await handleImportRoute(context, mockRequest('GET', `/api/imports/${jobId}`, undefined, { origin: ORIGIN, cookie }), detail.response);
    assert.equal(detail.captured.status, 200);
    const detailPayload = JSON.parse(detail.captured.body ?? '{}');
    assert.equal(detailPayload.evidence[0].hardGates.identity, true);
    assert.equal(detailPayload.evidence[0].value, facts.title);

    // No curator session -> 401.
    const noAuth = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, {}, {
      origin: ORIGIN, cookie: 'syco23_curator=forged',
    }), noAuth.response);
    assert.equal(noAuth.captured.status, 401);

    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, {}, {
      origin: ORIGIN, cookie,
    }), response.response);
    assert.equal(response.captured.status, 200);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.equal(payload.record.title, facts.title);
    assert.equal(payload.mixId, payload.record.id);
    assert.ok(payload.claimsApplied >= 1);
    assert.ok(repository.listClaims(payload.mixId).some((claim) =>
      claim.claim.field === 'title' && claim.claim.value === facts.title
    ));
  } finally { cleanup(); }
});

test('user-upload finalize creates a valid record without a private URN source', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const created = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', {
      source: { provider: 'user_upload', uploadId: 'upl_finalize_1' },
      mode: 'audio',
      conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
      rights: { basis: 'user_authorized_copy', attestationVersion: '2026-10' },
    }, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-upload-finalize-1'
    }), created.response);
    const jobId = JSON.parse(created.captured.body ?? '{}').job.id as string;
    context.repository.updateImportJobState(jobId, 'completed');

    // A title is required because there are no provider facts.
    const noTitle = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, {}, {
      origin: ORIGIN, cookie,
    }), noTitle.response);
    assert.equal(noTitle.captured.status, 400);

    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, {
      title: 'Private Upload Set',
      playbackUrl: 'https://media.example.org/private-set.mp3',
    }, {
      origin: ORIGIN, cookie,
    }), response.response);
    assert.equal(response.captured.status, 200);
    const payload = JSON.parse(response.captured.body ?? '{}');
    const record = CatalogRecordSchema.parse(payload.record);
    assert.equal(record.kind, 'mix');
    assert.equal(record.title, 'Private Upload Set');
    assert.deepEqual(record.sources, []);
    assert.equal(record.playbackUrls?.[0], 'https://media.example.org/private-set.mp3');
    assert.doesNotMatch(JSON.stringify(record), /urn:syco23:upload:/);
  } finally { cleanup(); }
});

test('rejected worker evidence never becomes a claim', async () => {
  const { context, cookie, cleanup, repository } = setup(true);
  try {
    const jobId = await createMetadataJob(context, cookie, 'idem-rejected-evidence-1');
    await claimJob(context, jobId);
    await writeMetadataArtifact(context, jobId);
    await workerPost(context, `/internal/imports/${jobId}/evidence`, {
      scores: [titleScore('Rejected Title', 'reject'), titleScore(facts.title)],
    });
    await workerPost(context, `/internal/imports/${jobId}/complete`, { state: 'completed', attempt: 1 });

    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, {}, {
      origin: ORIGIN, cookie,
    }), response.response);
    assert.equal(response.captured.status, 200);
    const mixId = JSON.parse(response.captured.body ?? '{}').mixId as string;
    const claims = repository.listClaims(mixId);
    assert.ok(claims.length >= 1);
    assert.ok(!claims.some((claim) => claim.claim.value === 'Rejected Title'));
  } finally { cleanup(); }
});
