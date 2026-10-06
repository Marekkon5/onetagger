import { afterEach, describe, expect, it, vi } from 'vitest';
import { catalogApi } from './api';

const record = {
  kind: 'mix',
  id: 'mix_01J9CATALOGUE00000000000070',
  createdAt: '2026-10-03T12:00:00.000Z',
  updatedAt: '2026-10-03T12:00:00.000Z',
  revision: 1,
  verification: 'source-confirmed',
  reviewState: 'ready',
  title: 'Live Mackitek Koalisson III',
  people: [],
  eventIds: [],
  genres: [],
  styles: [],
  assets: [],
  sources: []
};

afterEach(() => vi.unstubAllGlobals());

describe('catalogApi', () => {
  it('sends credentialed catalog reads and validates shared response schemas', async () => {
    const fetchMock = vi.fn().mockResolvedValue(Response.json({ items: [record], page: 1, pageSize: 25, total: 1 }));
    vi.stubGlobal('fetch', fetchMock);
    const page = await catalogApi.getIndex('mix', { query: 'Mackitek' });
    expect(fetchMock).toHaveBeenCalledWith('/api/catalog/mix?page=1&pageSize=25&q=Mackitek', expect.objectContaining({ credentials: 'include' }));
    expect(page.items[0]?.id).toBe(record.id);
  });

  it('does not accept malformed server records as canonical client state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ items: [{ ...record, durationMs: -1 }], page: 1, pageSize: 25, total: 1 })));
    await expect(catalogApi.getIndex('mix')).rejects.toThrow();
  });

  it('retains HTTP status and API error detail for unsaved writes', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: 'Revision conflict' }), { status: 409 })));
    await expect(catalogApi.updateRecord(record.id, { title: 'New title' }, 1)).rejects.toMatchObject({ status: 409, message: 'Revision conflict' });
  });
});
import { importJobsApi } from './api';

describe('importJobsApi', () => {
  it('fetches the version handshake and gates on importsEnabled', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({
      apiVersion: 1,
      importsEnabled: false,
      providers: { soundcloud: { metadata: true, audio: false } },
      limits: { maxInputBytes: 1, maxOutputBytes: 1, maxDurationMs: 1, maxAttempts: 5, autoApplyThreshold: 80 },
    })));
    const version = await importJobsApi.version();
    expect(version.importsEnabled).toBe(false);
    expect(version.providers.soundcloud?.audio).toBe(false);
  });

  it('sends an idempotency key when creating import jobs', async () => {
    const fetchMock = vi.fn().mockResolvedValue(Response.json({
      job: {
        id: 'imp_abcdef0123456789abcdef01', requestedBy: 'curator', provider: 'soundcloud',
        sourceUrl: 'https://soundcloud.com/example/example', mode: 'metadata', state: 'queued',
        idempotencyKey: 'idem-key-01', attempt: 0, createdAt: '2026-10-06T12:00:00.000Z', updatedAt: '2026-10-06T12:00:00.000Z',
      },
      events: [], artifacts: [], evidence: [],
      policy: { allowed: true },
    }));
    vi.stubGlobal('fetch', fetchMock);
    const detail = await importJobsApi.create({
      provider: 'soundcloud', url: 'https://soundcloud.com/example/example',
      mode: 'metadata', rightsBasis: 'provider_metadata_only',
    });
    expect(detail.job.state).toBe('queued');
    expect(fetchMock).toHaveBeenCalledWith('/api/imports', expect.objectContaining({
      method: 'POST',
      headers: expect.objectContaining({ 'idempotency-key': expect.any(String) }),
    }));
  });

  it('previews provider audio capabilities from the shared matrix', () => {
    expect(importJobsApi.providerAudioAllowed('soundcloud')).toBe(false);
    expect(importJobsApi.providerAudioAllowed('archiveorg')).toBe(true);
  });
});
