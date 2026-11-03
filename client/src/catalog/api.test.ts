import { afterEach, describe, expect, it, vi } from 'vitest';
import { catalogApi } from './api';
import { importJobsApi } from './api';

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

  it('requires a valid session response rather than treating malformed successful HTTP as public', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ authenticated: 'true' })));
    await expect(catalogApi.checkSession()).rejects.toThrow('Malformed session response');
  });

  it('validates evidence dispositions before they can be displayed as selected or rejected', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json([{ id: 'claim_0123456789abcdef', fingerprint: 'stable', claim: {}, disposition: 'invented' }])));
    await expect(catalogApi.getEvidence(record.id)).rejects.toThrow();
  });

  it('accepts persisted selected/pending evidence without losing the source or disposition', async () => {
    const evidence = { id: 'claim_0123456789abcdef', fingerprint: 'stable', disposition: 'selected', claim: { targetRecordId: record.id, field: 'title', value: record.title, provider: { provider: 'youtube', resourceType: 'video', externalId: 'video1' }, sourceUrl: 'https://youtube.com/watch?v=video1', observedAt: record.createdAt, evidence: 'direct', matchExplanation: 'Confirmed source' } };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json([evidence])));
    expect(await catalogApi.getEvidence(record.id)).toEqual([evidence]);
  });
});

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
      mode: 'metadata',
    });
    expect(detail.job.state).toBe('queued');
    expect(fetchMock).toHaveBeenCalledWith('/api/imports', expect.objectContaining({
      method: 'POST',
      headers: expect.objectContaining({ 'idempotency-key': expect.any(String) }),
    }));
  });

  it('previews provider audio capabilities from the shared matrix', () => {
    expect(importJobsApi.providerAudioAllowed('soundcloud')).toBe(true);
    expect(importJobsApi.providerAudioAllowed('hearthis')).toBe(true);
    expect(importJobsApi.providerAudioAllowed('archiveorg')).toBe(true);
    expect(importJobsApi.providerAudioAllowed('youtube')).toBe(false);
    expect(importJobsApi.providerAudioAllowed('musicbrainz')).toBe(false);
  });

  it('parses hardGates evidence and keeps the sourced value', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({
      job: {
        id: 'imp_abcdef0123456789abcdef01', requestedBy: 'curator', provider: 'mixcloud',
        sourceUrl: 'https://www.mixcloud.com/example/show/', mode: 'metadata', state: 'completed',
        idempotencyKey: 'idem-key-02', attempt: 1, createdAt: '2026-10-06T12:00:00.000Z', updatedAt: '2026-10-06T12:00:00.000Z',
      },
      events: [], artifacts: [], evidence: [{
        id: 'evd_0123456789', jobId: 'imp_abcdef0123456789abcdef01', claimId: null, field: 'title',
        value: 'Real Mixcloud Show', score: 97, algorithmVersion: 'evidence-v1',
        components: { I: 1, T: 0.9, D: 1, R: 0.9, P: 1, C: 1, X: 0 },
        hardGates: { identity: true, rights: true, noConflict: true, policy: true },
        decision: 'auto_apply', evaluatedAt: '2026-10-06T12:00:00.000Z',
      }],
    })));
    const detail = await importJobsApi.get('imp_abcdef0123456789abcdef01');
    expect(detail.evidence[0]?.value).toBe('Real Mixcloud Show');
    expect(detail.evidence[0]?.hardGates).toMatchObject({ identity: true });
  });

  it('rejects evidence that still uses the legacy gates spelling', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({
      job: {
        id: 'imp_abcdef0123456789abcdef01', requestedBy: 'curator', provider: 'mixcloud',
        sourceUrl: 'https://www.mixcloud.com/example/show/', mode: 'metadata', state: 'completed',
        idempotencyKey: 'idem-key-03', attempt: 1, createdAt: '2026-10-06T12:00:00.000Z', updatedAt: '2026-10-06T12:00:00.000Z',
      },
      events: [], artifacts: [], evidence: [{
        id: 'evd_0123456789', jobId: 'imp_abcdef0123456789abcdef01', claimId: null, field: 'title',
        score: 97, algorithmVersion: 'evidence-v1', components: {}, gates: { identity: true },
        decision: 'auto_apply', evaluatedAt: '2026-10-06T12:00:00.000Z',
      }],
    })));
    await expect(importJobsApi.get('imp_abcdef0123456789abcdef01')).rejects.toThrow();
  });

  it('parses the finalize envelope instead of a bare catalog record', async () => {
    const fetchMock = vi.fn().mockResolvedValue(Response.json({ record, mixId: record.id, claimsApplied: 3 }));
    vi.stubGlobal('fetch', fetchMock);
    const result = await catalogApi.finalize('imp_abcdef0123456789abcdef01');
    expect(result.mixId).toBe(record.id);
    expect(result.claimsApplied).toBe(3);
    expect(fetchMock).toHaveBeenCalledWith('/api/imports/imp_abcdef0123456789abcdef01/finalize', expect.objectContaining({ method: 'POST' }));
  });

  it('rejects a bare catalog record as a finalize response', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(record)));
    await expect(catalogApi.finalize('imp_abcdef0123456789abcdef01')).rejects.toThrow();
  });
});
