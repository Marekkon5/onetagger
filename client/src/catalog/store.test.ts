import { describe, expect, it, vi } from 'vitest';
import type { CatalogPage, MixRecord } from '@syco23/catalog-domain';
import { createCatalogStore, type CatalogGateway } from './store';

const mix: MixRecord = {
  kind: 'mix', id: 'mix_01J9CATALOGUE00000000000080',
  createdAt: '2026-10-03T12:00:00.000Z', updatedAt: '2026-10-03T12:00:00.000Z',
  revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Original title',
  people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
};

describe('catalog store', () => {
  it('uses validated server pages as the only source of canonical records', async () => {
    const page: CatalogPage = { items: [mix], page: 1, pageSize: 25, total: 1 };
    const gateway = { getIndex: vi.fn().mockResolvedValue(page) } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadIndex('mix');
    expect(store.state.records).toEqual([mix]);
    expect(gateway.getIndex).toHaveBeenCalledWith('mix', { page: 1, pageSize: 25, query: undefined });
  });

  it('keeps failed writes unsaved and leaves the last confirmed record untouched', async () => {
    const gateway = {
      getIndex: vi.fn().mockResolvedValue({ items: [mix], page: 1, pageSize: 25, total: 1 }),
      updateRecord: vi.fn().mockRejectedValue(new Error('Revision conflict'))
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadIndex('mix');
    await expect(store.updateRecord(mix.id, { title: 'Unconfirmed title' }, 1)).rejects.toThrow('Revision conflict');
    expect(store.state.records[0]).toEqual(mix);
    expect(store.state.error).toBe('Revision conflict');
  });

  it('keeps server records even when the same name comes from distinct providers', async () => {
    const other = { ...mix, id: 'mix_01J9CATALOGUE00000000000081', sources: [{ provider: 'soundcloud', resourceType: 'track', externalId: 'same-id' }] } as MixRecord;
    const gateway = { getIndex: vi.fn().mockResolvedValue({ items: [mix, other], page: 1, pageSize: 25, total: 2 }) } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadIndex('mix');
    expect(store.state.records.map((record) => record.id)).toEqual([mix.id, other.id]);
  });

  it('refreshes pending Review after an enrichment report', async () => {
    const item = {
      id: 'claim_0123456789abcdef', targetRecordId: mix.id, field: 'description',
      claim: { targetRecordId: mix.id, field: 'description', value: 'Proposed', provider: { provider: 'youtube', resourceType: 'video', externalId: 'video-1' }, sourceUrl: 'https://youtube.com/watch?v=video-1', observedAt: mix.createdAt, evidence: 'parsed', matchExplanation: 'Possible match' },
      state: 'pending', createdAt: mix.createdAt, recordRevision: 1
    };
    const gateway = {
      enrichRecord: vi.fn().mockResolvedValue({ state: 'completed', attemptedProviders: ['youtube'], applied: 0, corroborated: 0, reviewed: 1, errors: [], missingFields: ['description'] }),
      getRecord: vi.fn().mockResolvedValue(mix),
      getReview: vi.fn().mockResolvedValue([item])
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.enrichRecord(mix.id);
    expect(store.state.review).toEqual([item]);
  });

  it('migrates waveform media separately from the bounded JSON batch and keeps retry IDs stable', async () => {
    const stored = JSON.stringify([{ id: 'legacy-with-wave', title: 'Long mix', waveform: { imageDataUrl: 'data:image/png;base64,iVBORw0KGgo=', sourceUrl: 'https://archive.org/audio.mp3' } }]);
    const values = new Map([['syco23.mixsets.library', stored]]);
    vi.stubGlobal('window', { localStorage: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => { values.set(key, value); }
    } });
    const migrateMock = vi.fn().mockResolvedValue({ batchId: 'stable-batch', imported: 1, existing: 0, rejected: 0, legacyIds: { 'legacy-with-wave': 'mix_01J9CATALOGUE00000000000082' } });
    const gateway = {
      migrate: migrateMock,
      persistWaveform: vi.fn().mockResolvedValue(mix)
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.migrateLocalLibrary();
    await store.migrateLocalLibrary();
    const sentRecords = migrateMock.mock.calls[0]?.[1] as Array<Record<string, unknown>>;
    expect(migrateMock.mock.calls[1]?.[0]).toBe(migrateMock.mock.calls[0]?.[0]);
    expect(sentRecords[0]?.waveform).toBeUndefined();
    expect(gateway.persistWaveform).toHaveBeenCalledTimes(2);
    expect(gateway.persistWaveform).toHaveBeenLastCalledWith('mix_01J9CATALOGUE00000000000082', 'data:image/png;base64,iVBORw0KGgo=', 'https://archive.org/audio.mp3');
    expect(values.get('syco23.mixsets.library')).toBe(stored);
  });

  it('drops the duplicate from the local index only after the server committed the merge', async () => {
    const duplicate = { ...mix, id: 'mix_01J9CATALOGUE00000000000083', title: 'Duplicate import' };
    const merged = { ...mix, revision: 2 };
    const failing = {
      getIndex: vi.fn().mockResolvedValue({ items: [mix, duplicate], page: 1, pageSize: 25, total: 2 }),
      mergeRecords: vi.fn().mockRejectedValue(new Error('Revision conflict for mix_01J9CATALOGUE00000000000080'))
    } as unknown as CatalogGateway;
    const refused = createCatalogStore(failing);
    await refused.loadIndex('mix');
    await expect(refused.mergeRecords(mix.id, duplicate.id, [1, 1])).rejects.toThrow('Revision conflict');
    // A refused merge must leave the archive looking untouched.
    expect(refused.state.records.map((record) => record.id)).toEqual([mix.id, duplicate.id]);
    expect(refused.state.total).toBe(2);

    const gateway = {
      getIndex: vi.fn().mockResolvedValue({ items: [mix, duplicate], page: 1, pageSize: 25, total: 2 }),
      mergeRecords: vi.fn().mockResolvedValue(merged),
      getReview: vi.fn().mockResolvedValue([])
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadIndex('mix');
    const result = await store.mergeRecords(mix.id, duplicate.id, [1, 1]);
    expect(result.revision).toBe(2);
    expect(store.state.records.map((record) => record.id)).toEqual([mix.id]);
    expect(store.state.total).toBe(1);
  });
});