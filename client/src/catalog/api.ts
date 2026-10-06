import {
  CatalogPageSchema,
  CatalogRecordSchema,
  EnrichmentReportSchema,
  FieldEvidenceListSchema,
  ImportCandidateSchema,
  ImportJobSchema,
  ImportResultSchema,
  MigrationResultSchema,
  PROVIDER_CAPABILITIES,
  RelatedMixesSchema,
  ReviewItemSchema,
  type CatalogPage,
  type CatalogRecord,
  type EnrichmentReport,
  type FieldEvidenceList,
  type ImportCandidate,
  type ImportJob,
  type ImportResult,
  type IndexKind,
  type MigrationResult,
  type PageQuery,
  type RecordId,
  type RelatedMixes,
  type ReviewItem
} from '@syco23/catalog-domain';
import { z } from 'zod';

const API_BASE = (import.meta.env.VITE_API_BASE || '/api').replace(/\/$/, '');

export class CatalogApiError extends Error {
  constructor(message: string, readonly status: number, readonly details?: unknown) {
    super(message);
  }
}

async function request(path: string, init: RequestInit = {}): Promise<unknown> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: 'include',
    headers: {
      ...(init.body ? { 'content-type': 'application/json' } : {}),
      ...init.headers
    }
  });
  const payload = response.status === 204 ? undefined : await response.json().catch(() => undefined);
  if (!response.ok) {
    const details = payload && typeof payload === 'object' ? payload : undefined;
    const message = details && 'error' in details && typeof details.error === 'string'
      ? details.error
      : `Catalog API request failed (${response.status})`;
    throw new CatalogApiError(message, response.status, details);
  }
  return payload;
}

export interface IndexOptions {
  page?: number;
  pageSize?: number;
  query?: string;
}

export const catalogApi = {
  async getIndex(kind: IndexKind, options: IndexOptions = {}): Promise<CatalogPage> {
    const query: PageQuery = { page: options.page ?? 1, pageSize: options.pageSize ?? 25, query: options.query || undefined };
    const params = new URLSearchParams({ page: String(query.page), pageSize: String(query.pageSize) });
    if (query.query) params.set('q', query.query);
    return CatalogPageSchema.parse(await request(`/catalog/${kind}?${params}`));
  },

  async getRecord(id: RecordId): Promise<CatalogRecord> {
    return CatalogRecordSchema.parse(await request(`/catalog/records/${encodeURIComponent(id)}`));
  },

  /**
   * Field-level provenance for one record. Public wherever the record itself is
   * public, so a reader can see why a value is selected without a curator session.
   */
  async getEvidence(id: RecordId): Promise<FieldEvidenceList> {
    return FieldEvidenceListSchema.parse(await request(`/catalog/records/${encodeURIComponent(id)}/evidence`));
  },

  /** Mixes that reference this entity, derived server-side from the mix records. */
  async getRelatedMixes(id: RecordId): Promise<RelatedMixes> {
    return RelatedMixesSchema.parse(await request(`/catalog/records/${encodeURIComponent(id)}/related-mixes`));
  },

  async getReview(): Promise<ReviewItem[]> {
    const response = await request('/catalog/review');
    return Array.isArray(response) ? response.map((item) => ReviewItemSchema.parse(item)) : ReviewItemSchema.array().parse(response);
  },

  async checkSession(): Promise<boolean> {
    const response = await request('/auth/session') as { authenticated?: unknown };
    return Boolean(response && response.authenticated === true);
  },

  async login(password: string): Promise<boolean> {
    const response = await request('/auth/login', { method: 'POST', body: JSON.stringify({ password }) }) as { authenticated?: unknown };
    return response?.authenticated === true;
  },

  async logout(): Promise<void> {
    await request('/auth/logout', { method: 'POST', body: '{}' });
  },

  async importCandidate(input: ImportCandidate): Promise<ImportResult> {
    const candidate = ImportCandidateSchema.parse(input);
    return ImportResultSchema.parse(await request('/catalog/import', { method: 'POST', body: JSON.stringify(candidate) }));
  },

  async migrate(batchId: string, records: unknown[]): Promise<MigrationResult> {
    return MigrationResultSchema.parse(await request('/catalog/migrate', { method: 'POST', body: JSON.stringify({ batchId, records }) }));
  },

  async enrichRecord(id: RecordId): Promise<EnrichmentReport> {
    return EnrichmentReportSchema.parse(await request(`/catalog/records/${encodeURIComponent(id)}/enrich`, { method: 'POST', body: '{}' }));
  },

  async persistWaveform(id: RecordId, imageDataUrl: string, sourceUrl: string): Promise<CatalogRecord> {
    const match = /^data:image\/png;base64,([A-Za-z0-9+/=]+)$/.exec(imageDataUrl);
    if (!match) throw new Error('Waveform response is not a PNG data URL');
    const bytes = Uint8Array.from(atob(match[1]), (character) => character.charCodeAt(0));
    const query = new URLSearchParams({ sourceUrl });
    const record = await request(`/catalog/records/${encodeURIComponent(id)}/waveform?${query}`, {
      method: 'PUT',
      headers: { 'content-type': 'image/png' },
      body: bytes
    });
    return CatalogRecordSchema.parse(record);
  },

  async updateRecord(id: RecordId, patch: unknown, expectedRevision: number): Promise<CatalogRecord> {
    return CatalogRecordSchema.parse(await request(`/catalog/records/${encodeURIComponent(id)}`, {
      method: 'PATCH', body: JSON.stringify({ expectedRevision, patch })
    }));
  },

  async decideReview(id: RecordId, decision: 'accept' | 'reject', expectedRevision: number): Promise<ReviewItem> {
    return ReviewItemSchema.parse(await request(`/catalog/review/${encodeURIComponent(id)}/decision`, {
      method: 'POST', body: JSON.stringify({ decision, expectedRevision })
    }));
  },

  async refreshReview(id: RecordId): Promise<ReviewItem> {
    return ReviewItemSchema.parse(await request(`/catalog/review/${encodeURIComponent(id)}/refresh`, { method: 'POST', body: '{}' }));
  },

  /** Finalize a completed/review import job: create the catalog record, convert evidence to claims, apply merge. */
  async finalize(id: string, curatorPassword: string): Promise<CatalogRecord> {
    const response = await request(`/imports/${encodeURIComponent(id)}/finalize`, {
      method: 'POST',
      body: JSON.stringify({ curatorPassword }),
    });
    return CatalogRecordSchema.parse(response);
  },

  /**
   * Curator-confirmed duplicate merge. Both revisions are sent so a concurrent
   * curation session cannot be merged over, and the retired id resolves to the
   * survivor afterwards.
   */
  async mergeRecords(survivor: RecordId, duplicate: RecordId, expectedRevisions: [number, number]): Promise<CatalogRecord> {
    return CatalogRecordSchema.parse(await request('/catalog/merge', {
      method: 'POST',
      body: JSON.stringify({ survivor, duplicate, expectedRevisions })
    }));
  }
};

export const ApiVersionSchema = z.object({
  apiVersion: z.number().int(),
  importsEnabled: z.boolean(),
  providers: z.record(z.string(), z.object({ metadata: z.boolean(), audio: z.boolean() })),
  limits: z.object({
    maxInputBytes: z.number(),
    maxOutputBytes: z.number(),
    maxDurationMs: z.number(),
    maxAttempts: z.number(),
    autoApplyThreshold: z.number(),
  }),
});

export type ApiVersion = z.infer<typeof ApiVersionSchema>;

export const ImportJobDetailSchema = z.object({
  job: ImportJobSchema,
  events: z.array(z.object({
    id: z.string(), jobId: z.string(), sequence: z.number(),
    type: z.string(), payload: z.record(z.string(), z.unknown()), createdAt: z.string(),
  })),
  artifacts: z.array(z.object({
    id: z.string(), jobId: z.string(), role: z.string(), objectKey: z.string(),
    sha256: z.string(), mimeType: z.string().nullable(), sizeBytes: z.number().nullable(),
    codec: z.string().nullable(), durationMs: z.number().nullable(), state: z.string(), createdAt: z.string(),
  })),
  evidence: z.array(z.object({
    id: z.string(), jobId: z.string(), claimId: z.string().nullable(), field: z.string(),
    score: z.number(), algorithmVersion: z.string(),
    components: z.record(z.string(), z.unknown()), gates: z.record(z.string(), z.unknown()),
    decision: z.string(), evaluatedAt: z.string(),
  })),
});

export type ImportJobDetail = z.infer<typeof ImportJobDetailSchema>;

function newIdempotencyKey(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `idem_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;
}

export const importJobsApi = {
  async version(): Promise<ApiVersion> {
    return ApiVersionSchema.parse(await request('/version'));
  },

  /** Create a metadata or audio import job. The server policy gate is authoritative. */
  async create(input: {
    provider: string; url?: string; uploadId?: string; externalId?: string;
    mode: 'metadata' | 'audio'; rightsBasis: string; attestationVersion?: string;
  }): Promise<ImportJobDetail & { policy?: { allowed: boolean; reason?: string } }> {
    const body = {
      source: {
        provider: input.provider,
        ...(input.url ? { url: input.url } : {}),
        ...(input.uploadId ? { uploadId: input.uploadId } : {}),
        ...(input.externalId ? { externalId: input.externalId } : {}),
      },
      mode: input.mode,
      ...(input.mode === 'audio' ? { conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' } } : {}),
      rights: { basis: input.rightsBasis, attestationVersion: input.attestationVersion ?? '2026-10' },
    };
    const raw = await request('/imports', {
      method: 'POST',
      headers: { 'idempotency-key': newIdempotencyKey() },
      body: JSON.stringify(body),
    }) as ImportJobDetail & { policy?: { allowed: boolean; reason?: string } };
    return { ...ImportJobDetailSchema.parse(raw), policy: raw.policy };
  },

  async get(id: RecordId): Promise<ImportJobDetail> {
    return ImportJobDetailSchema.parse(await request(`/imports/${encodeURIComponent(id)}`));
  },

  async cancel(id: RecordId): Promise<ImportJobDetail> {
    return ImportJobDetailSchema.parse(await request(`/imports/${encodeURIComponent(id)}/cancel`, { method: 'POST', body: '{}' }));
  },

  async retry(id: RecordId): Promise<ImportJobDetail> {
    return ImportJobDetailSchema.parse(await request(`/imports/${encodeURIComponent(id)}/retry`, { method: 'POST', body: '{}' }));
  },

  /** Client-side preview of the provider capability matrix (server enforces). */
  providerAudioAllowed(provider: string): boolean {
    return (PROVIDER_CAPABILITIES as Record<string, { metadata: boolean; audio: boolean }>)[provider]?.audio === true;
  },
};