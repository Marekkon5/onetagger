import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable } from 'node:stream';
import type { IncomingMessage, ServerResponse } from 'node:http';
import test from 'node:test';
import { openCatalog } from '../catalog/repository.js';
import { createCuratorAuth, hashCuratorPassword } from '../auth/curator.js';
import { handleImportRoute, type ImportRouteContext } from './routes.js';
import { signWorkerRequest } from './service-auth.js';

const ORIGIN = 'https://mixsets.example.org';

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
  process.env.CORS_ORIGIN = ORIGIN;
  process.env.IMPORT_WORKER_SECRET = 'test-worker-secret';
  const auth = createCuratorAuth(repository, hashCuratorPassword('private-local-password'));
  const cookie = auth.login('private-local-password', '127.0.0.1');
  const context: ImportRouteContext = { repository, auth, importsEnabled };
  const cleanup = () => {
    if (priorOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = priorOrigin;
    if (priorSecret === undefined) delete process.env.IMPORT_WORKER_SECRET;
    else process.env.IMPORT_WORKER_SECRET = priorSecret;
    repository.close();
    rmSync(directory, { recursive: true, force: true });
  };
  return { context, cookie, cleanup, repository };
}

const metadataRequest = {
  source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
  mode: 'metadata',
  rights: { basis: 'provider_metadata_only', attestationVersion: '2026-10' },
};

const audioRequest = {
  source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
  mode: 'audio',
  conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
  rights: { basis: 'provider_metadata_only', attestationVersion: '2026-10' },
};

test('version handshake exposes capabilities and import gate', async () => {
  const { context, cleanup } = setup(false);
  try {
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('GET', '/api/version'), response.response);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.equal(response.captured.status, 200);
    assert.equal(payload.importsEnabled, false);
    assert.equal(payload.providers.soundcloud.audio, false);
    assert.equal(payload.providers.soundcloud.metadata, true);
  } finally { cleanup(); }
});

test('soundcloud audio is blocked as policy, not as technical failure', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', audioRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-sc-audio-1'
    }), response.response);
    assert.equal(response.captured.status, 202);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.equal(payload.job.state, 'blocked_policy');
    assert.equal(payload.policy.allowed, false);
    // Blocked jobs are never queued for a worker.
    assert.equal(context.repository.claimDueImportJob('worker-x'), undefined);
  } finally { cleanup(); }
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
    // user_upload jobs start in 'created' state (awaiting original file upload via PUT /api/imports/{id}/original)
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
  const { context, cookie, cleanup, repository } = setup(true);
  try {
    const created = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-worker-flow-1'
    }), created.response);
    const jobId = JSON.parse(created.captured.body ?? '{}').job.id as string;

    // No signature at all.
    const anonymous = mockResponse();
    await handleImportRoute(context, mockRequest('GET', `/internal/imports/${jobId}/work`), anonymous.response);
    assert.equal(anonymous.captured.status, 401);

    // Wrong secret.
    const forged = mockResponse();
    const forgedTimestamp = String(Math.floor(Date.now() / 1000));
    await handleImportRoute(context, mockRequest('GET', `/internal/imports/${jobId}/work`, undefined, {
      'x-import-worker-timestamp': forgedTimestamp,
      'x-import-worker-signature': signWorkerRequest('wrong-secret', forgedTimestamp, 'GET', `/internal/imports/${jobId}/work`, ''),
      'x-import-worker-scope': 'import_worker',
    }), forged.response);
    assert.equal(forged.captured.status, 401);

    // Correct signature: work spec contains job + limits, never the DB path.
    const timestamp = String(Math.floor(Date.now() / 1000));
    const path = `/internal/imports/${jobId}/work`;
    const valid = mockResponse();
    await handleImportRoute(context, mockRequest('GET', path, undefined, {
      'x-import-worker-timestamp': timestamp,
      'x-import-worker-signature': signWorkerRequest('test-worker-secret', timestamp, 'GET', path, ''),
      'x-import-worker-scope': 'import_worker',
    }), valid.response);
    assert.equal(valid.captured.status, 200);
    const spec = JSON.parse(valid.captured.body ?? '{}');
    assert.equal(spec.job.id, jobId);
    assert.ok(spec.limits.maxInputBytes > 0);
    assert.equal(spec.capabilities.soundcloud.audio, false);
  } finally { cleanup(); }
});

test('worker can report progress, artifacts, evidence and completion', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const created = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-worker-report-1'
    }), created.response);
    const jobId = JSON.parse(created.captured.body ?? '{}').job.id as string;

    const post = (path: string, body: Record<string, unknown>, method = 'POST') => {
      const raw = JSON.stringify(body);
      const timestamp = String(Math.floor(Date.now() / 1000));
      const response = mockResponse();
      return handleImportRoute(context, mockRequest(method, path, body, {
        'x-import-worker-timestamp': timestamp,
        'x-import-worker-signature': signWorkerRequest('test-worker-secret', timestamp, method, path, raw),
        'x-import-worker-scope': 'import_worker',
      }), response.response).then(() => response);
    };

    const event = await post(`/internal/imports/${jobId}/events`, { type: 'progress', payload: { stage: 'resolving' }, state: 'resolving' });
    assert.equal(event.captured.status, 204);

    const artifact = await post(`/internal/imports/${jobId}/artifacts`, {
      role: 'metadata', objectKey: `imports/${jobId}/metadata/${'b'.repeat(64)}.json`, sha256: 'b'.repeat(64),
    });
    assert.equal(artifact.captured.status, 201);

    const evidence = await post(`/internal/imports/${jobId}/evidence`, {
      scores: [{
        field: 'title', score: 92, algorithmVersion: 'evidence-v1',
        components: { I: 1, T: 1, D: 1, R: 1, P: 1, C: 0.7, X: 0 },
        hardGates: { identity: true, rights: true, noConflict: true, policy: true },
        decision: 'auto_apply', evaluatedAt: '2026-10-06T12:00:00.000Z',
      }],
    });
    assert.equal(evidence.captured.status, 201);

    const complete = await post(`/internal/imports/${jobId}/complete`, { state: 'completed', attempt: 1 });
    assert.equal(complete.captured.status, 200);
    const view = JSON.parse(complete.captured.body ?? '{}');
    assert.equal(view.job.state, 'completed');
    assert.equal(view.artifacts.length, 1);
    assert.equal(view.evidence[0].score, 92);
    // Outbox cleared: nothing left to claim.
    assert.equal(context.repository.claimDueImportJob('worker-1'), undefined);
  } finally { cleanup(); }
});

test('non-retryable worker failures stay failed instead of re-queueing', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const created = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-non-retry-1'
    }), created.response);
    const jobId = JSON.parse(created.captured.body ?? '{}').job.id as string;

    const raw = JSON.stringify({ state: 'failed', attempt: 1, retryable: false, error: 'ffmpeg failed', errorCode: 'non_retryable' });
    const timestamp = String(Math.floor(Date.now() / 1000));
    const path = `/internal/imports/${jobId}/complete`;
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', path, JSON.parse(raw), {
      'x-import-worker-timestamp': timestamp,
      'x-import-worker-signature': signWorkerRequest('test-worker-secret', timestamp, 'POST', path, raw),
      'x-import-worker-scope': 'import_worker',
    }), response.response);
    assert.equal(response.captured.status, 200);
    const view = JSON.parse(response.captured.body ?? '{}');
    assert.equal(view.job.state, 'failed');
    assert.equal(view.job.errorCode, 'non_retryable');
    assert.equal(context.repository.claimDueImportJob('worker-1'), undefined);
  } finally { cleanup(); }
});

test('/api/imports/{id}/finalize creates the catalog record and applies evidence', async () => {
  const { context, cookie, cleanup } = setup(true);
  try {
    const created = mockResponse();
    await handleImportRoute(context, mockRequest('POST', '/api/imports', metadataRequest, {
      origin: ORIGIN, cookie, 'idempotency-key': 'idem-finalize-1'
    }), created.response);
    const jobId = JSON.parse(created.captured.body ?? '{}').job.id as string;

    // Run through worker lifecycle: progress -> artifact -> evidence -> complete
    const post = (path: string, body: Record<string, unknown>, method = 'POST') => {
      const raw = JSON.stringify(body);
      const timestamp = String(Math.floor(Date.now() / 1000));
      const response = mockResponse();
      return handleImportRoute(context, mockRequest(method, path, body, {
        'x-import-worker-timestamp': timestamp,
        'x-import-worker-signature': signWorkerRequest('test-worker-secret', timestamp, method, path, raw),
        'x-import-worker-scope': 'import_worker',
      }), response.response).then(() => response);
    };

    await post(`/internal/imports/${jobId}/events`, { type: 'progress', payload: { stage: 'resolving' }, state: 'resolving' });
    await post(`/internal/imports/${jobId}/artifacts`, {
      role: 'metadata', objectKey: `imports/${jobId}/metadata/${'b'.repeat(64)}.json`, sha256: 'b'.repeat(64),
    });
    await post(`/internal/imports/${jobId}/evidence`, {
      scores: [{
        field: 'title', score: 87, algorithmVersion: 'evidence-v1',
        components: { I: 1, T: 1, D: 1, R: 1, P: 1, C: 0.7, X: 0 },
        hardGates: { identity: true, rights: true, noConflict: true, policy: true },
        decision: 'auto_apply', evaluatedAt: '2026-10-06T12:00:00.000Z',
      }],
    });
    const complete = await post(`/internal/imports/${jobId}/complete`, { state: 'completed', attempt: 1 });
    assert.equal(complete.captured.status, 200);

    // Finalize without curator session -> 401
    const noAuth = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, JSON.stringify({ curatorPassword: 'wrong' }), {
      origin: ORIGIN, cookie: 'syco23_curator=forged',
    }), noAuth.response);
    assert.equal(noAuth.captured.status, 401);

    // Finalize with curator -> 200, record created
    // Use the same context as the worker lifecycle (same repository).
    const response = mockResponse();
    await handleImportRoute(context, mockRequest('POST', `/api/imports/${jobId}/finalize`, JSON.stringify({ curatorPassword: 'private-local-password' }), {
      origin: ORIGIN, cookie: cookie,
    }), response.response);
    assert.equal(response.captured.status, 200);
    const payload = JSON.parse(response.captured.body ?? '{}');
    assert.ok(payload.record);
    assert.ok(payload.record.id);
    assert.equal(payload.mixId, payload.record.id);
    assert.ok(payload.claimsApplied >= 1);
    // Evidence was converted to a claim and merged
    const recordId = payload.record.id;
    const evidenceScores = context.repository.listImportEvidenceScores(jobId);
    assert.ok(evidenceScores.length >= 1);
    assert.equal(evidenceScores[0].claimId, null);
  } finally { cleanup(); }
});
