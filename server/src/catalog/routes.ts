import type { IncomingMessage, ServerResponse } from 'node:http';
import {
  CatalogRecordSchema,
  IndexKindSchema,
  PageQuerySchema,
  type CatalogRecord,
  type IndexKind,
  type RecordId
} from '@syco23/catalog-domain';
import type { CuratorAuth } from '../auth/curator.js';
import { decideReview, mergeRecords, refreshReview } from './merge.js';
import type { CatalogRepository } from './repository.js';
import { applyCorsHeaders, HttpInputError, isAllowedOrigin, readJsonBody, sendJson } from '../http.js';
import type { ProviderRegistry } from '../core/registry.js';
import { enrichCatalogRecord } from './enrich.js';
import { importCandidate } from './identity.js';
import { ImportCandidateSchema } from '@syco23/catalog-domain';
import { migrateLegacyLibrary } from './migration.js';
import { persistWaveform, readWaveformMedia } from './media.js';
import { readRequestBytes } from '../http.js';

export interface CatalogRouteContext {
  repository: CatalogRepository;
  auth: CuratorAuth;
  registry?: ProviderRegistry;
}

const editableFields: Record<CatalogRecord['kind'], readonly string[]> = {
  mix: ['title', 'description', 'durationMs', 'recordingDate', 'people', 'eventIds', 'genres', 'styles', 'assets', 'playbackUrls'],
  entity: ['displayName', 'roles', 'aliases', 'profile', 'country', 'assets', 'artist', 'crew', 'label'],
  event: ['name', 'startDate', 'endDate', 'venue', 'locality', 'country', 'assets', 'sourceUrls', 'mixIds']
};

// Transport limit for an uploaded waveform. Well below the 8 MiB stored-asset
// bound so oversized bodies are cut off mid-stream rather than buffered whole.
const maxWaveformUploadBytes = 2 * 1024 * 1024;

function requireCurator(context: CatalogRouteContext, request: IncomingMessage, response: ServerResponse): boolean {
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

function parsedError(error: unknown): { status: number; body: unknown } {
  if (error instanceof HttpInputError) return { status: error.statusCode, body: { error: error.message } };
  if (error && typeof error === 'object' && 'issues' in error) {
    return { status: 400, body: { error: 'Input validation failed', issues: error.issues } };
  }
  const message = error instanceof Error ? error.message : String(error);
  // Only curated domain messages are safe to echo. Anything else is logged
  // server-side so paths, SQL and dependency errors never reach a client.
  if (/revision conflict/i.test(message)) return { status: 409, body: { error: message } };
  if (/does not exist|not found|unknown record|unknown catalog record/i.test(message)) return { status: 404, body: { error: message } };
  if (clientSafeMessage(message)) return { status: 400, body: { error: message } };
  console.error('Unhandled catalog failure:', error);
  return { status: 500, body: { error: 'Request failed' } };
}

/**
 * Curated domain rejections that describe a bad request rather than an internal
 * fault. Each entry maps a message prefix used by the catalog layer to the HTTP
 * status a client should see.
 *
 * This is an explicit list rather than a broad regex so a newly thrown message
 * fails the `routes.test.ts` contract test (500, generic body) instead of silently
 * downgrading an internal error into a 400 that echoes internal detail.
 */
function clientSafeMessage(message: string): boolean {
  return SAFE_MESSAGES.some((prefix) => message.startsWith(prefix));
}

const SAFE_MESSAGES = [
  'Event relationship target must be an event',
  'Imported record could not be reloaded',
  'Legacy ID is invalid',
  'Media path is invalid',
  'Migration batch is outside supported bounds',
  'Migration batch',
  'Mix record disappeared during enrichment',
  'New records must start at revision 1',
  'Pending review item',
  'Provider identity points to missing record',
  'Record revision must advance to',
  'Relationship references an unknown record',
  'Relationship target does not have the',
  'Review item',
  'Survivor record',
  'Duplicate record',
  'Cannot merge',
  'Target record',
  'Unknown record',
  'Waveform image must be a valid PNG with an IHDR header',
  'Waveform PNG dimensions exceed the supported bounds',
  'Waveform PNG exceeds the 8 MiB size limit',
  'Waveform source URL is invalid',
  'Waveform target must be an existing mix',
  'Catalog record',
  'Patch contains a field that cannot be edited directly',
  'Unsupported catalog field',
  'Field'
] as const;

function recordId(value: string): RecordId {
  return decodeURIComponent(value) as RecordId;
}

async function handlePatch(context: CatalogRouteContext, request: IncomingMessage, response: ServerResponse, id: RecordId) {
  if (!requireCurator(context, request, response)) return;
  try {
    const input = await readJsonBody(request) as { expectedRevision?: unknown; patch?: unknown };
    if (!Number.isInteger(input.expectedRevision) || !input.patch || typeof input.patch !== 'object' || Array.isArray(input.patch)) {
      sendJson(response, 400, { error: 'expectedRevision and an object patch are required' });
      return;
    }
    const current = context.repository.getRecord(id);
    if (!current) {
      sendJson(response, 404, { error: 'Catalog record not found' });
      return;
    }
    const patch = input.patch as Record<string, unknown>;
    if (Object.keys(patch).some((field) => !editableFields[current.kind].includes(field))) {
      sendJson(response, 400, { error: 'Patch contains a field that cannot be edited directly' });
      return;
    }
    if (current.revision !== input.expectedRevision) {
      sendJson(response, 409, { error: `Revision conflict for ${id}` });
      return;
    }
    const updated = CatalogRecordSchema.parse({
      ...current,
      ...patch,
      revision: current.revision + 1,
      updatedAt: new Date().toISOString()
    });
    context.repository.transaction((tx) => tx.saveRecord(updated, current.revision));
    sendJson(response, 200, updated);
  } catch (error) {
    const parsed = parsedError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

export async function handleCatalogRoute(context: CatalogRouteContext, request: IncomingMessage, response: ServerResponse): Promise<boolean> {
  applyCorsHeaders(request, response);
  const url = new URL(request.url ?? '/', `http://${request.headers.host ?? 'localhost'}`);
  const path = url.pathname;

  const mediaMatch = path.match(/^\/api\/catalog\/media\/([A-Za-z0-9_-]{8,128})$/);
  if (request.method === 'GET' && mediaMatch) {
    const bytes = readWaveformMedia(context.repository, recordId(mediaMatch[1]));
    if (!bytes) {
      sendJson(response, 404, { error: 'Media asset not found' });
      return true;
    }
    response.writeHead(200, {
      'content-type': 'image/png',
      'content-length': bytes.byteLength,
      'cache-control': 'public, max-age=31536000, immutable',
      'x-content-type-options': 'nosniff'
    });
    response.end(bytes);
    return true;
  }

  if (request.method === 'POST' && path === '/api/catalog/migrate') {
    if (!requireCurator(context, request, response)) return true;
    try {
      const input = await readJsonBody(request) as { batchId?: unknown; records?: unknown };
      if (typeof input.batchId !== 'string' || !Array.isArray(input.records)) {
        sendJson(response, 400, { error: 'batchId and records are required' });
        return true;
      }
      const actor = context.auth.authenticate(request)!;
      const result = migrateLegacyLibrary(context.repository, input.records, input.batchId, actor);
      sendJson(response, 200, result);
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  const waveformMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/waveform$/);
  if (request.method === 'PUT' && waveformMatch) {
    if (!requireCurator(context, request, response)) return true;
    if (!/^image\/png(?:;|$)/i.test(request.headers['content-type'] ?? '')) {
      sendJson(response, 415, { error: 'Waveform uploads must use image/png' });
      return true;
    }
    try {
      // The transport limit is tighter than the stored-asset limit: an oversized or
      // non-PNG body is rejected while streaming instead of after buffering 8 MiB.
      const png = await readRequestBytes(request, maxWaveformUploadBytes);
      const sourceUrl = url.searchParams.get('sourceUrl') ?? '';
      const record = persistWaveform(context.repository, recordId(waveformMatch[1]), png, sourceUrl);
      sendJson(response, 200, record);
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  if (request.method === 'POST' && path === '/api/catalog/import') {
    if (!requireCurator(context, request, response)) return true;
    try {
      const input = ImportCandidateSchema.parse(await readJsonBody(request));
      const actor = context.auth.authenticate(request)!;
      const result = importCandidate(context.repository, input, actor);
      sendJson(response, result.outcome === 'created' ? 201 : 200, result);
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  const enrichMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/enrich$/);
  if (request.method === 'POST' && enrichMatch) {
    if (!requireCurator(context, request, response)) return true;
    if (!context.registry) {
      sendJson(response, 503, { error: 'Provider registry is unavailable' });
      return true;
    }
    try {
      const actor = context.auth.authenticate(request)!;
      const report = await enrichCatalogRecord(context.repository, context.registry, recordId(enrichMatch[1]), actor);
      sendJson(response, 200, report);
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  const runMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/enrichment-runs$/);
  if (request.method === 'GET' && runMatch) {
    const record = context.repository.getRecord(recordId(runMatch[1]));
    if (!record) {
      sendJson(response, 404, { error: 'Catalog record not found' });
      return true;
    }
    // Unfinished runs are curator-only; public readers see the settled history.
    const runs = context.repository.listEnrichmentRuns(record.id);
    sendJson(response, 200, context.auth.authenticate(request) ? runs : runs.filter((run) => run.state !== 'running'));
    return true;
  }

  const relatedMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/related-mixes$/);
  if (request.method === 'GET' && relatedMatch) {
    const record = context.repository.getRecord(recordId(relatedMatch[1]));
    if (!record || (record.verification === 'proposed' && !context.auth.authenticate(request))) {
      sendJson(response, 404, { error: 'Catalog record not found' });
      return true;
    }
    // Only an entity has inbound mix references; an event lists its own mixIds and a
    // mix has none. Returning [] keeps the response shape stable for the detail view.
    sendJson(response, 200, {
      recordId: record.id,
      mixes: record.kind === 'entity' ? context.repository.listRelatedMixes(record.id) : []
    });
    return true;
  }

  const evidenceMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/evidence$/);
  if (request.method === 'GET' && evidenceMatch) {
    const record = context.repository.getRecord(recordId(evidenceMatch[1]));
    if (!record || (record.verification === 'proposed' && !context.auth.authenticate(request))) {
      sendJson(response, 404, { error: 'Catalog record not found' });
      return true;
    }
    // Evidence explains a record's own provenance, so it is public wherever the
    // record itself is: a public reader must be able to see why a field holds its
    // value without holding a curator session.
    sendJson(response, 200, {
      recordId: record.id,
      evidence: context.repository.listClaims(record.id)
    });
    return true;
  }

  const recordMatch = path.match(/^\/api\/catalog\/records\/([^/]+)$/);
  if (request.method === 'GET' && recordMatch) {
    const record = context.repository.getRecord(recordId(recordMatch[1]));
    if (record?.verification === 'proposed' && !context.auth.authenticate(request)) {
      sendJson(response, 404, { error: 'Catalog record not found' });
      return true;
    }
    sendJson(response, record ? 200 : 404, record ?? { error: 'Catalog record not found' });
    return true;
  }

  if (request.method === 'PATCH' && recordMatch) {
    await handlePatch(context, request, response, recordId(recordMatch[1]));
    return true;
  }

  if (request.method === 'GET' && path === '/api/catalog/review') {
    if (!context.auth.authenticate(request)) {
      sendJson(response, 401, { error: 'Curator login is required' });
      return true;
    }
    sendJson(response, 200, context.repository.listReview());
    return true;
  }

  const decisionMatch = path.match(/^\/api\/catalog\/review\/([^/]+)\/decision$/);
  if (request.method === 'POST' && decisionMatch) {
    if (!requireCurator(context, request, response)) return true;
    try {
      const input = await readJsonBody(request) as { decision?: unknown; expectedRevision?: unknown };
      if (!['accept', 'reject'].includes(String(input.decision)) || !Number.isInteger(input.expectedRevision)) {
        sendJson(response, 400, { error: 'decision and expectedRevision are required' });
        return true;
      }
      const actor = context.auth.authenticate(request)!;
      const item = decideReview(context.repository, recordId(decisionMatch[1]), input.decision as 'accept' | 'reject', input.expectedRevision as number, actor.sessionId);
      sendJson(response, 200, item);
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  if (request.method === 'POST' && path === '/api/catalog/merge') {
    if (!requireCurator(context, request, response)) return true;
    try {
      const input = await readJsonBody(request) as {
        survivor?: unknown; duplicate?: unknown; expectedRevisions?: unknown;
      };
      if (typeof input.survivor !== 'string' || typeof input.duplicate !== 'string'
        || !Array.isArray(input.expectedRevisions) || input.expectedRevisions.length !== 2
        || !input.expectedRevisions.every((revision) => Number.isInteger(revision))) {
        sendJson(response, 400, { error: 'survivor, duplicate and two expectedRevisions are required' });
        return true;
      }
      const actor = context.auth.authenticate(request)!;
      const merged = mergeRecords(
        context.repository,
        recordId(input.survivor),
        recordId(input.duplicate),
        input.expectedRevisions as [number, number],
        actor.sessionId
      );
      sendJson(response, 200, merged);
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  const refreshMatch = path.match(/^\/api\/catalog\/review\/([^/]+)\/refresh$/);
  if (request.method === 'POST' && refreshMatch) {
    if (!requireCurator(context, request, response)) return true;
    try {
      sendJson(response, 200, refreshReview(context.repository, recordId(refreshMatch[1])));
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  const indexMatch = path.match(/^\/api\/catalog\/([^/]+)$/);
  if (request.method === 'GET' && indexMatch) {
    try {
      const kind = IndexKindSchema.parse(indexMatch[1]) as IndexKind;
      const page = PageQuerySchema.parse({
        page: url.searchParams.has('page') ? Number(url.searchParams.get('page')) : undefined,
        pageSize: url.searchParams.has('pageSize') ? Number(url.searchParams.get('pageSize')) : undefined,
        query: url.searchParams.get('q') ?? undefined
      });
      sendJson(response, 200, context.repository.listIndex(kind, page));
    } catch (error) {
      const parsed = parsedError(error);
      sendJson(response, parsed.status, parsed.body);
    }
    return true;
  }

  return false;
}