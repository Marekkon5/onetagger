// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { reactive, nextTick } from 'vue';
import type { CatalogRecord } from '@syco23/catalog-domain';
import ReviewView from '../views/ReviewView.vue';
import CatalogDetailView from '../views/CatalogDetailView.vue';
import CuratorLoginView from '../views/CuratorLoginView.vue';
import CatalogIndexView from '../views/CatalogIndexView.vue';
import { catalogApi } from './api';
import { useCatalogStore } from './store';

const base = { id: 'record_0123456789abcdef', createdAt: '2026-10-03T12:00:00.000Z', updatedAt: '2026-10-03T12:00:00.000Z', revision: 1, verification: 'curator-confirmed' as const, reviewState: 'ready' as const };
const event: CatalogRecord = { ...base, id: 'event_0123456789abcdef', kind: 'event', name: 'Only event link', sourceUrls: ['https://example.org/event'], assets: [], mixIds: [] };
const mix: CatalogRecord = { ...base, kind: 'mix', title: 'Test recording', people: [], eventIds: [event.id], genres: [], styles: [], assets: [], sources: [] };
const claim = { targetRecordId: mix.id, field: 'description', value: 'Proposal', provider: { provider: 'youtube' as const, resourceType: 'video', externalId: 'video1' }, sourceUrl: 'https://youtube.com/watch?v=video1', observedAt: base.createdAt, evidence: 'parsed' as const, matchExplanation: 'Possible recording match' };
const item = { id: 'claim_0123456789abcdef', targetRecordId: mix.id, field: 'description', claim, state: 'pending' as const, createdAt: base.createdAt, recordRevision: 1 };
const wrappers: Array<{ unmount(): void }> = [];
async function render(component: any, path = '/mix/' + mix.id) {
  const router = createRouter({ history: createMemoryHistory(), routes: ['/mix/:id', '/entity/:id', '/event/:id', '/catalog/:kind', '/login', '/review', '/:pathMatch(.*)*'].map((path) => ({ path, component })) });
  await router.push(path); await router.isReady();
  const wrapper = mount(component, { global: { plugins: [router], stubs: { QIcon: true, ArtworkFrame: true, WaveformStrip: true } } });
  wrappers.push(wrapper); await flushPromises(); return { wrapper, router };
}
beforeEach(async () => {
  vi.spyOn(catalogApi, 'login').mockResolvedValue(true);
  vi.spyOn(catalogApi, 'checkSession').mockResolvedValue(true);
  vi.spyOn(catalogApi, 'getReview').mockResolvedValue([]);
  vi.spyOn(catalogApi, 'getRecord').mockImplementation(async (id) => id === event.id ? event : mix);
  vi.spyOn(catalogApi, 'getIndex').mockResolvedValue({ items: [mix], page: 1, pageSize: 25, total: 1 });
  await useCatalogStore.login('disposable-local-password');
  const gateway = catalogApi as any;
  if (gateway.getRelated) vi.spyOn(gateway, 'getRelated').mockResolvedValue([event]);
  if (gateway.getEvidence) vi.spyOn(gateway, 'getEvidence').mockResolvedValue([{ id: item.id, fingerprint: 'stable-fingerprint', claim, disposition: 'pending' }]);
  if (gateway.getAnalysisRuns) vi.spyOn(gateway, 'getAnalysisRuns').mockResolvedValue([]);
  if (gateway.getRuns) vi.spyOn(gateway, 'getRuns').mockResolvedValue([]);
});
afterEach(() => { wrappers.splice(0).forEach((w) => w.unmount()); vi.restoreAllMocks(); localStorage.clear(); });

describe('curator rendered workflows', () => {
  it('renders event-only connected records on a mix and the selected field source', async () => {
    const { wrapper } = await render(CatalogDetailView);
    expect(wrapper.find('a[href="/event/' + event.id + '"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="field-evidence"]').text()).toContain('Possible recording match');
  });
  it('replaces a previous detail with an explicit failure when a new route cannot load', async () => {
    const { wrapper, router } = await render(CatalogDetailView);
    vi.mocked(catalogApi.getRecord).mockRejectedValueOnce(new Error('Record not found'));
    await router.push('/mix/missing_record1234'); await flushPromises();
    expect(wrapper.text()).not.toContain('Test recording'); expect(wrapper.text()).toContain('Record not found');
  });
  it('shows a failed review decision and refreshed revisions immediately', async () => {
    vi.mocked(catalogApi.getReview).mockResolvedValue([item]);
    vi.spyOn(catalogApi, 'decideReview').mockRejectedValue(new Error('Revision conflict. Refresh evidence.'));
    vi.spyOn(catalogApi, 'refreshReview').mockResolvedValue({ ...item, recordRevision: 2 });
    const { wrapper } = await render(ReviewView, '/review');
    await wrapper.findAll('button').find((b) => b.text().includes('Accept field'))!.trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Revision conflict. Refresh evidence.');
    vi.mocked(catalogApi.getRecord).mockResolvedValue({ ...mix, revision: 2 });
    await wrapper.findAll('button').find((b) => b.text().includes('Refresh evidence'))!.trigger('click'); await flushPromises();
    expect(wrapper.findAll('button').find((b) => b.text().includes('Accept field'))!.attributes('disabled')).toBeUndefined();
  });
  it('displays migration failures in the active curator session and permits retry', async () => {
    localStorage.setItem('syco23.mixsets.library', JSON.stringify([{ id: 'local-mix', title: 'Real browser copy' }]));
    vi.spyOn(catalogApi, 'migrate').mockRejectedValue(new Error('Migration transport failed'));
    const { wrapper } = await render(CuratorLoginView, '/login');
    await wrapper.findAll('button').find((b) => b.text().includes('Migrate this browser'))!.trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Migration transport failed');
    expect(wrapper.findAll('button').some((b) => b.text().includes('Migrate this browser'))).toBe(true);
    expect(localStorage.getItem('syco23.mixsets.library')).toContain('Real browser copy');
  });
  it('resets index search text when changing the index route', async () => {
    const { wrapper, router } = await render(CatalogIndexView, '/catalog/mix?q=Old+search');
    await router.push('/catalog/artist'); await flushPromises();
    expect((wrapper.find('input[type="search"]').element as HTMLInputElement).value).toBe('');
  });
});

describe('typed editor and duplicate preview', () => {
  it('renders all typed correction controls and saves the committed title', async () => {
    vi.spyOn(catalogApi, 'updateRecord').mockResolvedValue({ ...mix, title: 'Curated correction', revision: 2 });
    const { wrapper } = await render(CatalogDetailView);
    expect(wrapper.find('[data-testid="catalog-editor"]').exists()).toBe(true);
    expect(wrapper.find('textarea[id="edit-genres"]').exists()).toBe(true);
    await wrapper.find('input[id="edit-title"]').setValue('Curated correction');
    await wrapper.find('[data-testid="catalog-editor"] form').trigger('submit'); await flushPromises();
    expect(wrapper.find('h1').text()).toBe('Curated correction');
  });
  it('requires loading a duplicate preview before confirming the merge', async () => {
    const duplicate = { ...mix, id: 'duplicate_0123456789abcdef', title: 'Duplicate title', revision: 3 };
    vi.mocked(catalogApi.getRecord).mockImplementation(async (id) => id === duplicate.id ? duplicate : mix);
    vi.spyOn(catalogApi, 'mergeRecords').mockResolvedValue({ ...mix, revision: 2 });
    const { wrapper } = await render(CatalogDetailView);
    expect(wrapper.find('[data-testid="merge-preview"]').exists()).toBe(true);
    expect(wrapper.find('button[data-testid="confirm-merge"]').exists()).toBe(false);
    await wrapper.find('input[aria-label="Duplicate record ID"]').setValue(duplicate.id);
    await wrapper.find('form[data-testid="duplicate-lookup"]').trigger('submit'); await flushPromises();
    expect(wrapper.text()).toContain('Duplicate title'); expect(wrapper.text()).toContain('REV 3');
    await wrapper.find('button[data-testid="confirm-merge"]').trigger('click'); await flushPromises();
    expect(catalogApi.mergeRecords).toHaveBeenCalledWith(mix.id, duplicate.id, [1, 3]);
  });
});

describe('curated entity and event creation', () => {
  it('offers an explicit new entity form and navigates to the committed identity', async () => {
    const entity: CatalogRecord = { ...base, id: 'entity_0123456789abcdef', kind: 'entity', displayName: 'Confirmed crew', roles: ['crew'], aliases: [], assets: [], providerRefs: [] };
    vi.spyOn(catalogApi, 'createRecord').mockResolvedValue(entity);
    const { wrapper, router } = await render(CatalogIndexView, '/catalog/crew');
    expect(wrapper.find('[data-testid="create-record"]').exists()).toBe(true);
    await wrapper.find('[data-testid="create-record"]').trigger('click');
    await wrapper.find('#edit-displayName').setValue('Confirmed crew');
    await wrapper.find('[data-testid="catalog-editor"] form').trigger('submit'); await flushPromises();
    expect(router.currentRoute.value.path).toBe('/entity/' + entity.id);
    expect(catalogApi.createRecord).toHaveBeenCalledWith(expect.objectContaining({ kind: 'entity', displayName: 'Confirmed crew', roles: ['crew'] }));
  });
});

it('keeps partial migration retry available and displays each record failure to the curator', async () => {
  localStorage.setItem('syco23.mixsets.library', JSON.stringify([{ id: 'legacy-partial', title: 'Real browser copy' }]));
  vi.spyOn(catalogApi, 'migrate').mockResolvedValue({ batchId: 'stable-batch', imported: 1, existing: 0, rejected: 0, legacyIds: { 'legacy-partial': mix.id }, outcomes: [{ legacyId: 'legacy-partial', status: 'partial', recordId: mix.id, errors: ['Manual cover requires review'] }] });
  const { wrapper } = await render(CuratorLoginView, '/login');
  await wrapper.findAll('button').find((b) => b.text().includes('Migrate this browser'))!.trigger('click'); await flushPromises();
  expect(wrapper.text()).toContain('Manual cover requires review');
  expect(wrapper.findAll('button').some((b) => b.text().includes('Migrate this browser'))).toBe(true);
});

it('shows interrupted analysis history with an explicit retry action', async () => {
  const gateway = catalogApi as any;
  expect(gateway.getAnalysisRuns).toBeTypeOf('function');
  vi.mocked(gateway.getAnalysisRuns).mockResolvedValue([{ id: 'analysis_0123456789abcdef', recordId: mix.id, sourceUrl: 'https://archive.org/test.mp3', state: 'interrupted', progress: 0, error: 'Worker restarted before analysis finished', createdAt: base.createdAt, updatedAt: base.updatedAt }]);
  const { wrapper } = await render(CatalogDetailView);
  expect(wrapper.text()).toContain('Worker restarted before analysis finished');
  expect(wrapper.findAll('button').some((b) => b.text().includes('Retry analysis'))).toBe(true);
});

it('requires a duplicate preview instead of a generic field acceptance for uncertain recording identity', async () => {
  const duplicateItem = { ...item, field: 'possibleDuplicate', claim: { ...claim, field: 'possibleDuplicate', value: { recordId: 'duplicate_0123456789abcdef', revision: 1 } } };
  vi.mocked(catalogApi.getReview).mockResolvedValue([duplicateItem]);
  const { wrapper } = await render(ReviewView, '/review');
  expect(wrapper.findAll('button').some((b) => b.text().includes('Accept field'))).toBe(false);
  const previewLink = wrapper.findAll('a').find((a) => a.text().includes('Preview duplicate'));
  expect(previewLink?.attributes('href')).toBe(`/mix/${mix.id}?duplicate=duplicate_0123456789abcdef`);
});
