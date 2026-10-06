import type { EntityRef, ProviderId, CatalogRecord } from '../domain/types';

export interface ApiMixCandidate {
  provider: ProviderId;
  title: string;
  artists: string[];
  crews: string[];
  durationMs?: number;
  recordedAt?: string;
  description?: string;
  genres?: string[];
  artwork?: string[];
  source: { provider: ProviderId; url: string; externalId?: string };
  externalIds?: Record<string, string>;
  confidence: number;
  reasons: string[];
  raw: Record<string, unknown>;
}

export interface ApiDiscoveryJob {
  id: string;
  provider: ProviderId;
  query?: Record<string, unknown>;
  state: 'queued' | 'running' | 'review' | 'done' | 'error' | 'cancelled';
  progress: number;
  scanned: number;
  found: number;
  candidates: ApiMixCandidate[];
  createdAt?: string;
  updatedAt?: string;
  error?: string;
}

export interface ApiProviderHealth {
  id: ProviderId;
  state: 'ready' | 'limited' | 'offline';
  detail: string;
  checkedAt: string;
}

export interface ApiEnrichmentResult {
  patch: {
    durationMs?: number;
    recordedAt?: string;
    description?: string;
    genres?: string[];
    artwork?: Array<{ url: string; provider: ProviderId; kind: 'cover' | 'artist' | 'crew' }>;
    sources?: Array<{ provider: ProviderId; url: string; externalId?: string }>;
    externalIds?: Record<string, string>;
  };
  candidates: ApiMixCandidate[];
  entities: EntityRef[];
  provenance: Array<{
    provider: ProviderId;
    field: string;
    confidence: number;
    sourceUrl?: string;
  }>;
  attempted: ProviderId[];
  failures: Array<{ provider: ProviderId; error: string }>;
}

export interface LocalTrackEnrichmentInput {
  title: string;
  artists: string[];
  crews?: string[];
  durationMs?: number;
  recordedAt?: string;
  description?: string;
  genres?: string[];
  artwork?: Array<{ url: string; provider?: ProviderId; kind?: 'cover' | 'artist' | 'crew' }>;
  providers: ProviderId[];
}

const API_BASE = (import.meta.env.VITE_API_BASE || '/api').replace(/\/$/, '');

export class ApiRequestError extends Error {
  constructor(message: string, readonly status: number) {
    super(message);
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: 'include',
    headers: {
      'content-type': 'application/json',
      ...(init?.headers || {}),
    },
  });
  if (!response.ok) {
    const text = await response.text();
    let message = text || `API ${response.status}`;
    try {
      const payload = JSON.parse(text) as { error?: unknown };
      if (typeof payload.error === 'string') message = payload.error;
    } catch {
      message = text || `API ${response.status}`;
    }
    throw new ApiRequestError(message, response.status);
  }
  return response.json() as Promise<T>;
}

export function createDiscoveryJob(provider: ProviderId, query: Record<string, unknown>) {
  return request<ApiDiscoveryJob>('/jobs', {
    method: 'POST',
    body: JSON.stringify({ provider, query }),
  });
}

export function getDiscoveryJobs() {
  return request<ApiDiscoveryJob[]>('/jobs');
}

export function getDiscoveryJob(id: string) {
  return request<ApiDiscoveryJob>(`/jobs/${encodeURIComponent(id)}`);
}

export function cancelDiscoveryJob(id: string) {
  return request<ApiDiscoveryJob>(`/jobs/${encodeURIComponent(id)}/cancel`, {
    method: 'POST',
    body: '{}',
  });
}

export function getProviderHealth() {
  return request<ApiProviderHealth[]>('/providers');
}

export function enrichLocalTrackMetadata(input: LocalTrackEnrichmentInput) {
  return request<ApiEnrichmentResult>('/enrich', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export async function fetchProviderArtwork(url: string): Promise<{ bytes: ArrayBuffer; contentType: string }> {
  const response = await fetch(`${API_BASE}/artwork/fetch`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ url }),
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => undefined) as { error?: unknown } | undefined;
    throw new ApiRequestError(typeof payload?.error === 'string' ? payload.error : `Cover request failed (${response.status})`, response.status);
  }
  return {
    bytes: await response.arrayBuffer(),
    contentType: response.headers.get('content-type') || 'image/jpeg',
  };
}

export interface ApiWaveformJob {
  id: string;
  sourceUrl: string;
  state: 'queued' | 'running' | 'done' | 'error';
  progress: number;
  imageDataUrl?: string;
  error?: string;
  analyzedAt?: string;
}

export function createWaveformJob(sourceUrl: string) {
  return request<ApiWaveformJob>('/waveforms', { method: 'POST', body: JSON.stringify({ sourceUrl }) });
}

export function getWaveformJob(id: string) {
  return request<ApiWaveformJob>(`/waveforms/${encodeURIComponent(id)}`);
}

export interface FinalizeImportJobInput {
  curatorPassword: string;
}

export interface FinalizeImportJobResult {
  record: CatalogRecord;
  mixId: string;
  claimsApplied: number;
}

export async function finalizeImportJob(id: string, input: FinalizeImportJobInput): Promise<FinalizeImportJobResult> {
  const raw = await request<FinalizeImportJobResult>(`/imports/${encodeURIComponent(id)}/finalize`, {
    method: 'POST',
    body: JSON.stringify(input),
  });
  return raw;
}

