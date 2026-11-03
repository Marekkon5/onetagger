import { randomUUID } from 'node:crypto';
import type { IncomingMessage, ServerResponse } from 'node:http';
import {
  CatalogRecordSchema,
  FieldClaimSchema,
  getFieldValue,
  setFieldValue,
  validateField,
  IndexKindSchema,
  PageQuerySchema,
  type CatalogRecord,
  type IndexKind,
  type RecordId
} from '@syco23/catalog-domain';
import type { CuratorAuth } from '../auth/curator.js';
import { claimFingerprint, decideReview, mergeRecords, refreshReview } from './merge.js';
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
  if (clientSafeMessage(message) && /does not exist|not found|unknown record|unknown catalog record/i.test(message)) return { status: 404, body: { error: message } };
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
  try { return decodeURIComponent(value) as RecordId; }
  catch { throw new HttpInputError('Record ID encoding is invalid', 400); }
}
function inputObject(input: unknown): Record<string, unknown> {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new HttpInputError('A JSON object is required', 400);
  return input as Record<string, unknown>;
}
function curateRecord(context: CatalogRouteContext, record: CatalogRecord, patch: Record<string, unknown>, actor: string, expectedRevision?: number): CatalogRecord {
  const observedAt = new Date().toISOString();
  const revision = expectedRevision === undefined ? 1 : expectedRevision + 1;
  let updated = { ...record, revision, updatedAt: observedAt } as CatalogRecord;
  let validationRecord = record;
  if (record.kind === 'entity' && Array.isArray(patch.roles)) {
    validationRecord = CatalogRecordSchema.parse({ ...record, roles: [...new Set([...record.roles, ...patch.roles])] });
    updated = { ...validationRecord, revision, updatedAt: observedAt };
  }
  const claims = Object.entries(patch).sort(([a], [b]) => a === 'roles' ? 1 : b === 'roles' ? -1 : 0).map(([field, raw]) => {
    const value = validateField(validationRecord, field, raw);
    updated = setFieldValue(updated, field, value);
    return FieldClaimSchema.parse({ targetRecordId: record.id, field, value,
      provider: { provider: 'freeteknomusic', resourceType: 'curator', externalId: actor },
      sourceUrl: 'https://mixsets.syco23.org/catalog/' + record.id, observedAt,
      evidence: 'curated', matchExplanation: 'Explicit curator correction' });
  });
  updated = CatalogRecordSchema.parse({ ...updated, selectedEvidence: { ...updated.selectedEvidence,
    ...Object.fromEntries(claims.map(claim => [claim.field, claimFingerprint(claim)])) } });
  context.repository.transaction(tx => {
    tx.saveRecord(updated, expectedRevision);
    for (const claim of claims) { const fingerprint = claimFingerprint(claim);
      if (!tx.getClaim(fingerprint)) tx.addClaim(fingerprint as RecordId, fingerprint, claim, 'selected', revision);
    }
  });
  return updated;
}

async function handlePatch(context: CatalogRouteContext, request: IncomingMessage, response: ServerResponse, id: RecordId) {
  if (!requireCurator(context, request, response)) return;
  try {
    const input = inputObject(await readJsonBody(request));
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
    const updated = curateRecord(context, current, patch, context.auth.authenticate(request)!.sessionId, current.revision);
    sendJson(response, 200, updated);
  } catch (error) {
    const parsed = parsedError(error);
    sendJson(response, parsed.status, parsed.body);
  }
}

async function dispatchCatalogRoute(context: CatalogRouteContext, request: IncomingMessage, response: ServerResponse): Promise<boolean> {
  applyCorsHeaders(request, response);
  const url = new URL(request.url ?? '/', `http://${request.headers.host ?? 'localhost'}`);
  const path = url.pathname;


  if (request.method === 'POST' && path === '/api/catalog/merge') {
    if (!requireCurator(context, request, response)) return true;
    const input = inputObject(await readJsonBody(request));
    if (typeof input.survivor !== 'string' || typeof input.duplicate !== 'string' || !Array.isArray(input.expectedRevisions)
      || input.expectedRevisions.length !== 2 || !input.expectedRevisions.every(value => Number.isInteger(value) && value > 0))
      throw new HttpInputError('survivor, duplicate and two expected revisions are required', 400);
    sendJson(response, 200, mergeRecords(context.repository, input.survivor as RecordId, input.duplicate as RecordId,
      input.expectedRevisions as [number, number], context.auth.authenticate(request)!));
    return true;
  }
  if (request.method === 'POST' && path === '/api/catalog/records') {
    if (!requireCurator(context, request, response)) return true;
    const input = inputObject(await readJsonBody(request));
    if (input.kind !== 'entity' && input.kind !== 'event') throw new HttpInputError('Choose an entity or event record', 400);
    const { kind, ...patch } = input;
    if (Object.keys(patch).some(field => !editableFields[kind].includes(field))) throw new HttpInputError('Patch contains a field that cannot be edited directly', 400);
    const now = new Date().toISOString();
    const base = { id: randomUUID(), kind, createdAt: now, updatedAt: now, revision: 1,
      verification: 'curator-confirmed', reviewState: 'ready', assets: [] };
    const record = CatalogRecordSchema.parse(kind === 'entity'
      ? { ...base, displayName: input.displayName, roles: input.roles, aliases: [], providerRefs: [], ...patch }
      : { ...base, name: input.name, sourceUrls: [], mixIds: [], ...patch });
    sendJson(response, 201, curateRecord(context, record, patch, context.auth.authenticate(request)!.sessionId));
    return true;
  }
  const analysisHistory = path.match(/^\/api\/catalog\/records\/([^/]+)\/analysis-runs$/);
  if (request.method === 'GET' && analysisHistory) {
    if (!context.auth.authenticate(request)) { sendJson(response, 401, { error: 'Curator login is required' }); return true; }
    const target=context.repository.getRecord(recordId(analysisHistory[1]));
    if (!target) { sendJson(response,404,{error:'Catalog record not found'}); return true; }
    sendJson(response,200,context.repository.listAnalysisJobs(target.id)); return true;
  }
  const extraMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/(related|connections)$/);
  if (request.method === 'GET' && extraMatch) {
    const target = context.repository.getRecord(recordId(extraMatch[1]));
    const curator = Boolean(context.auth.authenticate(request));
    if (!target || (target.verification === 'proposed' && !curator)) { sendJson(response, 404, { error: 'Catalog record not found' }); return true; }

    const direct = referencedIds(target);
    const related = new Map<string, CatalogRecord>();
    for (const id of direct) { const item = context.repository.getRecord(id as RecordId); if (item && (curator || item.verification !== 'proposed')) related.set(item.id, item); }
    for (const kind of ['mix','artist','crew','label','event'] as const) {
      let page = 1;
      for (;;) { const result = context.repository.listIndex(kind, { page, pageSize: 50 });
        for (const item of result.items) if (item.id !== target.id && referencedIds(item).includes(target.id)) related.set(item.id, item);
        if (page * result.pageSize >= result.total) break; page += 1;
      }
    }
    sendJson(response, 200, [...related.values()].map(item => CatalogRecordSchema.parse(item))); return true;
  }

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
      const input = inputObject(await readJsonBody(request));
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

  const runMatch = path.match(/^\/api\/catalog\/records\/([^/]+)\/(?:enrichment-runs|runs)$/);
  if (request.method === 'GET' && runMatch) {
    const record = context.repository.getRecord(recordId(runMatch[1]));
    if (!record || (record.verification === 'proposed' && !context.auth.authenticate(request))) {
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
      const input = inputObject(await readJsonBody(request));
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
function referencedIds(record: CatalogRecord): string[] {
  if (record.kind === 'mix') return [...record.people.map(person => person.entityId), ...record.eventIds];
  if (record.kind === 'event') return record.mixIds;
  return [...record.artist?.memberIds ?? [], ...record.artist?.groupIds ?? [], ...record.crew?.memberIds ?? [],
    ...record.label?.subLabelIds ?? [], ...record.label?.parentId ? [record.label.parentId] : []];
}
export async function handleCatalogRoute(context: CatalogRouteContext, request: IncomingMessage, response: ServerResponse): Promise<boolean> {
  try { return await dispatchCatalogRoute(context, request, response); }
  catch (error) { const parsed = parsedError(error); sendJson(response, parsed.status, parsed.body); return true; }
}
