import { describe, expect, it, vi } from 'vitest';
import type { FieldEvidence, MixRecord, ReviewItem } from '@syco23/catalog-domain';
import { createCatalogStore, type CatalogGateway } from './store';

const mix: MixRecord = {
  kind: 'mix', id: 'mix_01J9CATALOGUE00000000000090',
  createdAt: '2026-10-03T12:00:00.000Z', updatedAt: '2026-10-03T12:00:00.000Z',
  revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Kan10 Live',
  people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
};

function evidence(overrides: Partial<FieldEvidence> = {}): FieldEvidence {
  return {
    id: 'claim_0123456789abcdef',
    fingerprint: 'claim_0123456789abcdef',
    claim: {
      targetRecordId: mix.id,
      field: 'description',
      value: 'From the linked source',
      provider: { provider: 'youtube', resourceType: 'video', externalId: 'vi5miMVpmuI' },
      sourceUrl: 'https://www.youtube.com/watch?v=vi5miMVpmuI',
      observedAt: mix.createdAt,
      evidence: 'direct',
      matchExplanation: 'Linked source identity with compatible duration'
    },
    disposition: 'selected',
    ...overrides
  };
}

function reviewItem(overrides: Partial<ReviewItem> = {}): ReviewItem {
  return {
    id: 'claim_0123456789abcdef',
    targetRecordId: mix.id,
    field: 'genres',
    claim: { ...evidence().claim, field: 'genres', value: ['techno'] },
    state: 'pending',
    createdAt: mix.createdAt,
    recordRevision: 1,
    ...overrides
  };
}

describe('field evidence and review', () => {
  it('keeps the selected value visible with the source that established it', async () => {
    const selectedId = 'claim_0123456789abcdef';
    const gateway = {
      getRecord: vi.fn().mockResolvedValue({ ...mix, selectedEvidence: { description: selectedId } }),
      getEvidence: vi.fn().mockResolvedValue({ recordId: mix.id, evidence: [evidence({ id: selectedId })] })
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadDetail(mix.id);
    await store.loadEvidence(mix.id);
    expect(store.state.evidence).toHaveLength(1);
    expect(store.state.evidence[0].disposition).toBe('selected');
    // The bare evidence id stored on the record must resolve to a real claim, so a
    // reader can see which source established the current value.
    const selected = store.state.evidence.find((item) => item.id === store.state.detail?.selectedEvidence?.description);
    expect(selected?.claim.provider.provider).toBe('youtube');
  });

  it('shows a rejected claim as a decision instead of dropping it silently', async () => {
    const gateway = {
      getRecord: vi.fn().mockResolvedValue(mix),
      getEvidence: vi.fn().mockResolvedValue({ recordId: mix.id, evidence: [evidence({ disposition: 'rejected' })] })
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadDetail(mix.id);
    await store.loadEvidence(mix.id);
    expect(store.state.evidence[0].disposition).toBe('rejected');
    expect(store.state.evidence[0].claim.matchExplanation).toContain('Linked source identity');
  });

  it('still renders the record when evidence cannot be loaded', async () => {
    const gateway = {
      getRecord: vi.fn().mockResolvedValue(mix),
      getEvidence: vi.fn().mockRejectedValue(new Error('Catalog record not found'))
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadDetail(mix.id);
    // A missing explanation must not take the record down with it.
    await expect(store.loadEvidence(mix.id)).resolves.toEqual([]);
    expect(store.state.detail?.id).toBe(mix.id);
    expect(store.state.error).toBeUndefined();
  });

  it('keeps a review decision truthful when the record moved on', async () => {
    const stale = reviewItem({ recordRevision: 1 });
    const gateway = {
      getReview: vi.fn().mockResolvedValue([stale]),
      getRecord: vi.fn().mockResolvedValue({ ...mix, revision: 3 }),
      decideReview: vi.fn().mockRejectedValue(new Error('Revision conflict for mix_01J9CATALOGUE00000000000090'))
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadReview();
    await store.loadDetail(mix.id);
    await expect(store.decideReview(stale.id, 'accept', stale.recordRevision)).rejects.toThrow('Revision conflict');
    // The item stays in the queue: the decision did not happen.
    expect(store.state.review.map((item) => item.id)).toEqual([stale.id]);
    expect(store.state.error).toContain('Revision conflict');
  });

  it('removes a review item only after the server accepted the decision', async () => {
    const item = reviewItem();
    const gateway = {
      getReview: vi.fn().mockResolvedValue([item]),
      getRecord: vi.fn().mockResolvedValue(mix),
      decideReview: vi.fn().mockResolvedValue({ ...item, state: 'accepted' })
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadReview();
    await store.decideReview(item.id, 'accept', item.recordRevision);
    expect(store.state.review).toEqual([]);
  });

  it('lists the mixes that reference an entity from the server response', async () => {
    const entityId = 'entity_01J9CATALOGUE00000000000091';
    const other = { ...mix, id: 'mix_01J9CATALOGUE00000000000092', title: 'Second set' };
    const gateway = {
      getRecord: vi.fn().mockResolvedValue(mix),
      getRelatedMixes: vi.fn().mockResolvedValue({ recordId: entityId, mixes: [mix, other] })
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    const related = await store.loadRelatedMixes(entityId);
    expect(related.map((item) => item.title)).toEqual([mix.title, other.title]);
  });

  it('clears the related mixes so a previous entity cannot leak into the next page', async () => {
    const gateway = {
      getRelatedMixes: vi.fn()
        .mockResolvedValueOnce({ recordId: 'entity_a', mixes: [mix] })
        .mockResolvedValueOnce({ recordId: 'entity_b', mixes: [] })
    } as unknown as CatalogGateway;
    const store = createCatalogStore(gateway);
    await store.loadRelatedMixes('entity_a');
    expect(store.state.relatedMixes).toHaveLength(1);
    await store.loadRelatedMixes('entity_b');
    expect(store.state.relatedMixes).toEqual([]);
  });
});