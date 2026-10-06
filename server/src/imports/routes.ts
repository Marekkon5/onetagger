import type { IncomingMessage, ServerResponse } from 'node:http';
import {
  ImportJobSchema,
  PROVIDER_CAPABILITIES,
  ScoredEvidenceSchema,
  terminalImportState,
  type CatalogRecord,
  type ImportJob,
  type ImportJobState,
  type ImportCandidate,
  type ScoredEvidence,
} from '@syco23/catalog-domain';
import type { CatalogRepository } from '../catalog/repository.js';
import type { CuratorAuth } from '../auth/curator.js';
import {
  applyCorsHeaders,
  isAllowedOrigin,
  readJsonBody,
  readRequestBytes,
  sendJson,
} from '../http.js';
import {
  ImportValidationError,
  decideImportPolicy,
  newImportId,
  parseImportRequest,
} from './policy.js';
import { WorkerAuthError, authenticateWorker, workerBodyText } from './service-auth.js';
import { createImportStore, extensionFromMime, objectKey, sha256 } from './storage.js';

export interface ImportRouteContext {
  repository: CatalogRepository;
  auth: CuratorAuth;
  importsEnabled: boolean;
}

// Hard administrative limits for the first release. Configurable via env so
// throughput and storage ceilings are never implicit assumptions in code.
export function importLimits() {
  return {
    maxInputBytes: Number(process.env.IMPORT_MAX_INPUT_BYTES || 2 * 1024 * 1024 * 1024),
    maxOutputBytes: Number(process.env.IMPORT_MAX_OUTPUT_BYTES || 1024 * 1024 * 1024),
    maxDurationMs: Number(process.env.IMPORT_MAX_DURATION_MS || 12 * 60 * 60 * 1000),
    maxAttempts: Number(process.env.IMPORT_MAX_ATTEMPTS || 5),
    autoApplyThreshold: Number(process.env.IMPORT_AUTO_APPLY_THRESHOLD || 80),
  };
}

function requireEnabled(context: ImportRouteContext, response: ServerResponse): boolean {
  if (!context.importsEnabled) {
    sendJson(response, 503, { error: 'Import jobs are disabled (IMPORTS_ENABLED)' });
    return false;
  }
  return true;
}

function requireCurator(context: ImportRouteContext, request: IncomingMessage, response: ServerResponse): boolean {
  if (!isAllowedOrigin(request)) {
    sendJson(response, 403, { error: 'Origin is not allowed' });
    return false;
  }
  if (!context.auth.authenticate(request)) {
    sendJson(response, 401, { error: 'Curator login is required' });
    return false;
  }
  return true;
}

function jobView(context: ImportRouteContext, id: string) {
  const job = context.repository.getImportJob(id);
  if (!job) return undefined;
  return {
    job,
    events: context.repository.listImportEvents(id),
    artifacts: context.repository.listImportArtifacts(id),
    evidence: context.repository.listImportEvidenceScores(id),
  };
}

function originalObjectKey(context: ImportRouteContext, jobId: string): string | undefined {
  return context.repository.listImportArtifacts(jobId).find((artifact) => artifact.role === 'original')?.objectKey;
}

function workSpec(context: ImportRouteContext, job: NonNullable<ReturnType<CatalogRepository['getImportJob']>>) {
  const original = originalObjectKey(context, job.id);
  return {
    job: { ...job, attempt: job.attempt + 1 },
    limits: importLimits(),
    capabilities: PROVIDER_CAPABILITIES,
    ...(original ? { originalObjectKey: original } : {}),
  };
}

function parseError(error: unknown): { status: number; body: unknown } {
  if (error instanceof ImportValidationError) return { status: error.statusCode, body: { error: error.message } };
  if (error instanceof WorkerAuthError) return { status: error.statusCode, body: { error: error.message } };
  if (error && typeof error === 'object' && 'issues' in error) {
    return { status: 400, body: { error: 'Input validation failed', issues: (error as { issues: unknown }).issues } };
  }
  const message = error instanceof Error ? error.message : String(error);
  if (/unknown import job/i.test(message)) return { status: 404, body: { error: 'Import job not found' } };
  if (/duplicate|idempotency/i.test(message)) return { status: 409, body: { error: 'Idempotency key already used' } };
  if (message.includes('Input validation failed')) return { status: 400, body: { error: message } };
  console.error('Import route failure:', error);
  return { status: 500, body: { error: 'Request failed' } };
}

async function handleCreate(context: ImportRouteContext, request: IncomingMessage, response: ServerResponse) {
  if (!requireEnabled(context, response)) return;
  if (!requireCurator(context, request, response)) return;
  try {
    const body = await readJsonBody(request) as Record<string, unknown>;
    const headerKey = request.headers['idempotency-key'];
    const idempotencyKey = (typeof headerKey === 'string' ? headerKey : undefined)
      ?? (typeof body.idempotencyKey === 'string' ? body.idempotencyKey : undefined);
    if (!idempotencyKey) {
      sendJson(response, 400, { error: 'Idempotency-Key header is required' });
      return;
    }
    const request0 = parseImportRequest(body);
    const existing = context.repository.getImportJobByIdempotency(idempotencyKey);
    if (existing) {
      sendJson(response, 200, jobView(context, existing.id));
      return;
    }
    const actor = context.auth.authenticate(request)!;
    const policy = decideImportPolicy(request0);
    const waitingForOriginal = policy.allowed && request0.source.provider === 'user_upload';
    const state: ImportJobState = policy.allowed ? (waitingForOriginal ? 'created' : 'queued') : 'blocked_policy';
    const now = new Date().toISOString();
    const sourceUrl = request0.source.url ?? `urn:syco23:upload:${request0.source.uploadId}`;
    const job = context.repository.createImportJob(ImportJobSchema.parse({
      id: newImportId(),
      requestedBy: actor.sessionId,
      provider: request0.source.provider,
      sourceUrl,
      ...(request0.source.externalId ? { sourceExternalId: request0.source.externalId } : {}),
      mode: request0.mode,
      state,
      idempotencyKey,
      createdAt: now,
      updatedAt: now,
    }));
    context.repository.saveImportProvenance(job.id, {
      provider: request0.source.provider,
      sourceUrl,
      ...(request0.source.externalId ? { externalId: request0.source.externalId } : {}),
      retrievalMethod: request0.source.provider === 'user_upload' ? 'user-upload' : 'provider-api',
      observedAt: now,
      snapshot: { mode: request0.mode, policy: { allowed: policy.allowed, reason: policy.reason } },
    });
    context.repository.saveImportRightsConsent({
      id: newImportId('rights'),
      jobId: job.id,
      requestedBy: actor.sessionId,
      basis: request0.rights.basis,
      provider: request0.source.provider,
      sourceUrl,
      attestationVersion: request0.rights.attestationVersion,
      ...(request0.rights.proofObjectKey ? { proofObjectKey: request0.rights.proofObjectKey } : {}),
    });
    if (policy.allowed && !waitingForOriginal) {
      context.repository.enqueueImportJob(job.id);
    } else if (!policy.allowed) {
      context.repository.appendImportEvent(job.id, 'blocked_policy', { reason: policy.reason });
    } else {
      context.repository.appendImportEvent(job.id, 'awaiting_original', { uploadId: request0.source.uploadId });
    }
    sendJson(response, 202, {
      ...jobView(context, job.id),
      policy: { allowed: policy.allowed, reason: policy.reason, capabilities: policy.capabilities },
    });
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

async function handleFinalize(
  context: ImportRouteContext,
  request: IncomingMessage,
  response: ServerResponse,
  id: string,
) {
  if (!requireEnabled(context, response)) return;
  if (!requireCurator(context, request, response)) return;
  try {
    const job = context.repository.getImportJob(id);
    if (!job) {
      sendJson(response, 404, { error: 'Import job not found' });
      return;
    }
    if (job.state !== 'completed' && job.state !== 'review') {
      sendJson(response, 409, { error: `Job must be completed or in review to finalize (state ${job.state})` });
      return;
    }

    // Read artifacts and evidence scores
    const artifacts = context.repository.listImportArtifacts(job.id);
    const evidenceScores = context.repository.listImportEvidenceScores(job.id) as unknown as ScoredEvidence[];
    const artifactMap = new Map<string, { role: string; objectKey: string }>();
    for (const art of artifacts) {
      artifactMap.set(art.id, { role: art.role, objectKey: art.objectKey });
    }

    const { importCandidate } = await import('../catalog/identity.js');
    const { claimsFromImportJob } = await import('../catalog/provider-claims.js');
    const { applyClaims } = await import('../catalog/merge.js');
    const { randomUUID } = await import('node:crypto');

    let mixId = job.mixId;
    if (!mixId) {
      // Create a new catalog record from this job
      const actor = context.auth.authenticate(request)!;
      const now = new Date().toISOString();
      const id = `mix_${randomUUID()}`;

      // user_upload jobs create the record directly (source is a private URN, not httpUrl)
      if (job.provider === 'user_upload') {
        const record: CatalogRecord = {
          kind: 'mix', id, createdAt: now, updatedAt: now, revision: 1,
          verification: 'curator-confirmed', reviewState: 'ready', title: 'Imported mix',
          people: [], eventIds: [], genres: [], styles: [], assets: [], sources: [{
            provider: 'freeteknomusic',
            resourceType: 'recording',
            externalId: job.sourceExternalId ?? job.sourceUrl,
            url: job.sourceUrl,
            addedAt: now,
          }],
        };
        context.repository.transaction((tx) => {
          tx.saveRecord(record);
          tx.addProviderSource(record.id, { provider: 'freeteknomusic', resourceType: 'recording', externalId: job.sourceExternalId ?? job.sourceUrl, url: undefined });
        });
        mixId = record.id;
        context.repository.updateImportJobState(job.id, job.state, { mixId });
      } else {
        // Catalog providers go through importCandidate (source is httpUrl).
        // job.provider is narrowed to the catalog provider union here.
        const provider = job.provider as 'freeteknomusic' | 'soundcloud' | 'archiveorg' | 'discogs' | 'youtube' | 'hearthis';
        const url = job.sourceUrl ?? '';
        const externalId = job.sourceExternalId ?? url;
        const candidate: ImportCandidate = {
          provider,
          title: artifacts.find(a => a.role === 'metadata') ? 'Imported mix' : url,
          artists: [],
          crews: [],
          durationMs: undefined,
          recordedAt: undefined,
          description: undefined,
          genres: [],
          artwork: [],
          source: {
            provider,
            resourceType: 'recording',
            externalId,
            url,
          },
          confidence: 0.8,
          reasons: ['Imported via durable import job'],
        };
        const result = importCandidate(context.repository, candidate, actor);
        mixId = result.record.id;
        context.repository.updateImportJobState(job.id, job.state, { mixId });
      }
    }

    // Convert worker evidence to claims and apply merge policy
    const claims = claimsFromImportJob(job, evidenceScores, artifactMap, mixId) || [];
    if (claims.length > 0) {
      applyClaims(context.repository, claims);
    }

    // Return the updated record
    const record = context.repository.getRecord(mixId);
    sendJson(response, 200, { record, mixId, claimsApplied: claims.length });
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

async function handleJobAction(
  context: ImportRouteContext,
  request: IncomingMessage,
  response: ServerResponse,
  id: string,
  action: string,
) {
  if (!requireEnabled(context, response)) return;
  if (!requireCurator(context, request, response)) return;
  try {
    const job = context.repository.getImportJob(id);
    if (!job) {
      sendJson(response, 404, { error: 'Import job not found' });
      return;
    }
    if (action === 'cancel') {
      if (terminalImportState(job.state)) {
        sendJson(response, 409, { error: `Job is already ${job.state}` });
        return;
      }
      context.repository.updateImportJobState(job.id, 'cancelled');
      sendJson(response, 200, jobView(context, job.id));
      return;
    }
    if (action === 'retry') {
      if (job.state !== 'failed' && job.state !== 'cancelled') {
        sendJson(response, 409, { error: 'Only failed or cancelled jobs can be retried' });
        return;
      }
      if (job.attempt >= importLimits().maxAttempts) {
        sendJson(response, 409, { error: 'Retry limit reached for this job' });
        return;
      }
      context.repository.updateImportJobState(job.id, 'queued', { errorCode: null, error: null });
      context.repository.enqueueImportJob(job.id);
      sendJson(response, 200, jobView(context, job.id));
      return;
    }
    sendJson(response, 404, { error: 'not found' });
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

async function handleOriginal(
  context: ImportRouteContext,
  request: IncomingMessage,
  response: ServerResponse,
  id: string,
) {
  if (!requireEnabled(context, response)) return;
  if (!requireCurator(context, request, response)) return;
  try {
    const job = context.repository.getImportJob(id);
    if (!job) {
      sendJson(response, 404, { error: 'Import job not found' });
      return;
    }
    if (job.provider !== 'user_upload') {
      sendJson(response, 409, { error: 'Only user-upload jobs accept an original object' });
      return;
    }
    if (job.state !== 'created') {
      sendJson(response, 409, { error: `Job is not awaiting an original (state ${job.state})` });
      return;
    }
    const existing = originalObjectKey(context, job.id);
    if (existing) {
      sendJson(response, 200, jobView(context, job.id));
      return;
    }
    const mimeType = typeof request.headers['content-type'] === 'string' ? request.headers['content-type'] : '';
    const extension = extensionFromMime(mimeType);
    if (!extension) {
      sendJson(response, 415, { error: 'Unsupported original audio type' });
      return;
    }
    const bytes = await readRequestBytes(request, importLimits().maxInputBytes);
    if (!bytes.length) {
      sendJson(response, 400, { error: 'Original object is empty' });
      return;
    }
    const digest = sha256(bytes);
    const key = objectKey(job.id, 'original', digest, extension);
    await createImportStore().putPrivate(key, bytes, digest);
    context.repository.registerImportArtifact({
      id: newImportId('art'),
      jobId: job.id,
      role: 'original',
      objectKey: key,
      sha256: digest,
      mimeType: mimeType.split(';')[0].trim().slice(0, 120),
      sizeBytes: bytes.length,
    });
    context.repository.updateImportJobState(job.id, 'queued');
    context.repository.enqueueImportJob(job.id);
    sendJson(response, 201, jobView(context, job.id));
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

async function readWorkerBody(request: IncomingMessage): Promise<Record<string, unknown>> {
  const stashed = workerBodyText(request);
  if (stashed !== undefined) {
    if (!stashed) return {};
    return JSON.parse(stashed) as Record<string, unknown>;
  }
  return await readJsonBody(request) as Record<string, unknown>;
}

/**
 * Lease endpoint: atomically assigns the oldest due job to this worker.
 * The outbox row is the durable queue; the lease expires after 30s so a crashed
 * worker's job becomes claimable again without losing it.
 */
async function handleWorkerNext(context: ImportRouteContext, request: IncomingMessage, response: ServerResponse) {
  try {
    await authenticateWorker(request);
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
    return;
  }
  try {
    const workerId = typeof request.headers['x-import-worker-id'] === 'string'
      ? request.headers['x-import-worker-id'].slice(0, 80)
      : 'worker';
    const job = context.repository.claimDueImportJob(workerId);
    if (!job) {
      sendJson(response, 204, {});
      return;
    }
    context.repository.appendImportEvent(job.id, 'claimed', { workerId });
    sendJson(response, 200, workSpec(context, job));
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

async function handleWorker(
  context: ImportRouteContext,
  request: IncomingMessage,
  response: ServerResponse,
  id: string,
  action: string,
) {
  try {
    await authenticateWorker(request);
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
    return;
  }
  try {
    const job = context.repository.getImportJob(id);
    if (!job) {
      sendJson(response, 404, { error: 'Import job not found' });
      return;
    }
    if (request.method === 'GET' && action === 'work') {
      if (job.state === 'blocked_policy' || terminalImportState(job.state)) {
        sendJson(response, 409, { error: `Job is not workable (state ${job.state})` });
        return;
      }
      sendJson(response, 200, workSpec(context, job));
      return;
    }
    if (request.method === 'POST' && action === 'heartbeat') {
      const owner = typeof request.headers['x-import-worker-id'] === 'string'
        ? request.headers['x-import-worker-id'].slice(0, 80)
        : '';
      const renewed = owner ? context.repository.renewImportLease(job.id, owner) : false;
      if (!renewed) {
        sendJson(response, 409, { error: 'Lease is not held by this worker' });
        return;
      }
      sendJson(response, 204, {});
      return;
    }
    if (request.method === 'POST' && action === 'events') {
      const body = await readWorkerBody(request);
      const type = typeof body.type === 'string' ? body.type : '';
      if (!type || type.length > 80) {
        sendJson(response, 400, { error: 'type is required' });
        return;
      }
      const payload = body.payload && typeof body.payload === 'object' && !Array.isArray(body.payload)
        ? body.payload as Record<string, unknown>
        : {};
      context.repository.appendImportEvent(job.id, type, payload);
      if (typeof body.state === 'string') {
        const state = body.state as ImportJobState;
        if (state === 'failed' || state === 'completed' || state === 'review' || state === 'blocked_policy') {
          context.repository.updateImportJobState(job.id, state, {
            attempt: Number.isInteger(body.attempt) ? body.attempt as number : job.attempt,
            ...(typeof body.error === 'string' ? { error: body.error.slice(0, 2000) } : {}),
            ...(typeof body.errorCode === 'string' ? { errorCode: body.errorCode.slice(0, 120) } : {}),
            ...(typeof body.mixId === 'string' ? { mixId: body.mixId } : {}),
          });
        }
      }
      sendJson(response, 204, {});
      return;
    }
    if (request.method === 'POST' && action === 'artifacts') {
      const body = await readWorkerBody(request);
      // The backend re-validates every worker report: backend validates SHA,
      // size and type, because the worker is an untrusted producer.
      const role = typeof body.role === 'string' ? body.role : '';
      const objectKey = typeof body.objectKey === 'string' ? body.objectKey : '';
      const artifactSha = typeof body.sha256 === 'string' ? body.sha256 : '';
      if (!['original', 'staging', 'normalized', 'artwork', 'metadata'].includes(role)) {
        sendJson(response, 400, { error: 'Artifact role is not valid' });
        return;
      }
      if (!/^imports\/imp_[a-f0-9]+\/(original|staging|normalized|artwork|metadata)\/[0-9a-f]{64}\.[a-z0-9]+$/.test(objectKey)) {
        sendJson(response, 400, { error: 'Artifact object key does not match the expected shape' });
        return;
      }
      if (!/^[0-9a-f]{64}$/.test(artifactSha)) {
        sendJson(response, 400, { error: 'Artifact sha256 must be a lowercase hex digest' });
        return;
      }
      const sizeBytes = typeof body.sizeBytes === 'number' && Number.isInteger(body.sizeBytes) ? body.sizeBytes : null;
      if (sizeBytes !== null && (sizeBytes <= 0 || sizeBytes > 4294967296)) {
        sendJson(response, 400, { error: 'Artifact size is outside the allowed bounds' });
        return;
      }
      const durationMs = typeof body.durationMs === 'number' && Number.isFinite(body.durationMs) ? Math.round(body.durationMs) : null;
      if (durationMs !== null && (durationMs <= 0 || durationMs > 86400000)) {
        sendJson(response, 400, { error: 'Artifact duration is outside the allowed bounds' });
        return;
      }
      const mimeType = typeof body.mimeType === 'string' ? body.mimeType.slice(0, 120) : null;
      const codec = typeof body.codec === 'string' ? body.codec.slice(0, 60) : null;
      const idem = typeof body.id === 'string' && body.id.length >= 8 && body.id.length <= 128 ? body.id : newImportId('art');
      try {
        context.repository.registerImportArtifact({
          id: idem,
          jobId: job.id,
          role,
          objectKey,
          sha256: artifactSha,
          mimeType,
          sizeBytes,
          codec,
          durationMs,
        });
      } catch (error) {
        // Duplicate registration from an at-least-once redelivery is idempotent.
        if (!/UNIQUE|PRIMARY/i.test(error instanceof Error ? error.message : String(error))) throw error;
      }
      sendJson(response, 201, { id: idem });
      return;
    }
    if (request.method === 'POST' && action === 'evidence') {
      const body = await readWorkerBody(request);
      const scores = Array.isArray(body.scores) ? body.scores : [];
      const saved = [] as unknown[];
      for (const raw of scores) {
        const scored = ScoredEvidenceSchema.parse(raw);
        saved.push(context.repository.saveImportEvidenceScore({
          id: newImportId('evd'),
          jobId: job.id,
          field: scored.field ?? 'unknown',
          scored,
        }));
      }
      sendJson(response, 201, { saved: saved.length });
      return;
    }
    if (request.method === 'POST' && action === 'complete') {
      const body = await readWorkerBody(request);
      const attempt = Number.isInteger(body.attempt) ? body.attempt as number : job.attempt;
      const wanted = body.state === 'review' ? 'review' : body.state === 'failed' ? 'failed' : 'completed';
      if (wanted === 'failed' && body.retryable === true && attempt < importLimits().maxAttempts) {
        // At-least-once retry with exponential backoff + jitter: the job goes back
        // to queued and becomes claimable after the delay, never silently lost.
        const backoffMs = Math.min(30_000 * 2 ** Math.max(0, attempt - 1), 15 * 60_000)
          + Math.floor(Math.random() * 2000);
        context.repository.updateImportJobState(job.id, 'queued', {
          attempt,
          ...(typeof body.error === 'string' ? { error: body.error.slice(0, 2000) } : {}),
          ...(typeof body.errorCode === 'string' ? { errorCode: body.errorCode.slice(0, 120) } : {}),
        });
        context.repository.enqueueImportJob(job.id, backoffMs);
        sendJson(response, 200, { ...jobView(context, job.id), scheduledRetryInMs: backoffMs });
        return;
      }
      context.repository.updateImportJobState(job.id, wanted, {
        attempt,
        ...(typeof body.errorCode === 'string' ? { errorCode: body.errorCode.slice(0, 120) } : {}),
        ...(typeof body.mixId === 'string' ? { mixId: body.mixId } : {}),
        ...(typeof body.error === 'string' ? { error: body.error.slice(0, 2000) } : {}),
      });
      sendJson(response, 200, jobView(context, job.id));
      return;
    }
    sendJson(response, 404, { error: 'not found' });
  } catch (error) {
    const parsed = parseError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

/**
 * Public capability handshake. The client gates its import UI on this so an
 * older UI never posts to a disabled or newer control plane.
 */
export function versionPayload(context: ImportRouteContext) {
  return {
    apiVersion: 1,
    importsEnabled: context.importsEnabled,
    providers: PROVIDER_CAPABILITIES,
    limits: importLimits(),
  };
}

export async function handleImportRoute(
  context: ImportRouteContext,
  request: IncomingMessage,
  response: ServerResponse,
): Promise<boolean> {
  applyCorsHeaders(request, response);
  const url = new URL(request.url ?? '/', `http://${request.headers.host ?? 'localhost'}`);
  const path = url.pathname;

  if (request.method === 'GET' && path === '/api/version') {
    sendJson(response, 200, versionPayload(context));
    return true;
  }

  if (request.method === 'POST' && path === '/api/imports') {
    await handleCreate(context, request, response);
    return true;
  }

  const internalMatch = path.match(/^\/internal\/imports\/([^/]+)\/(work|events|artifacts|evidence|complete|heartbeat)$/);
  if (internalMatch) {
    await handleWorker(context, request, response, decodeURIComponent(internalMatch[1]), internalMatch[2]);
    return true;
  }

  if (request.method === 'GET' && path === '/internal/imports/next') {
    await handleWorkerNext(context, request, response);
    return true;
  }

  const originalMatch = path.match(/^\/api\/imports\/([^/]+)\/original$/);
  if (request.method === 'PUT' && originalMatch) {
    await handleOriginal(context, request, response, decodeURIComponent(originalMatch[1]));
    return true;
  }

  const actionMatch = path.match(/^\/api\/imports\/([^/]+)\/(cancel|retry)$/);
  if (request.method === 'POST' && actionMatch) {
    await handleJobAction(context, request, response, decodeURIComponent(actionMatch[1]), actionMatch[2]);
    return true;
  }

  const finalizeMatch = path.match(/^\/api\/imports\/([^/]+)\/finalize$/);
  if (request.method === 'POST' && finalizeMatch) {
    if (!requireEnabled(context, response)) return true;
    if (!requireCurator(context, request, response)) return true;
    await handleFinalize(context, request, response, decodeURIComponent(finalizeMatch[1]));
    return true;
  }

  const jobMatch = path.match(/^\/api\/imports\/([^/]+)$/);
  if (request.method === 'GET' && jobMatch) {
    if (!requireEnabled(context, response)) return true;
    if (!requireCurator(context, request, response)) return true;
    const view = jobView(context, decodeURIComponent(jobMatch[1]));
    sendJson(response, view ? 200 : 404, view ?? { error: 'Import job not found' });
    return true;
  }

  if (path === '/api/imports' || path.startsWith('/api/imports/') || path.startsWith('/internal/imports/')) {
    sendJson(response, 404, { error: 'not found' });
    return true;
  }

  return false;
}
