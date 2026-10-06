import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useMixStore } from './useMixStore';

vi.mock('../services/api', () => ({
  getDiscoveryJobs: vi.fn()
}));

const job = (overrides: Record<string, unknown> = {}) => ({
  id: 'job-1', provider: 'youtube', query: { q: 'Kan10' }, state: 'running',
  progress: 40, scanned: 10, found: 2, candidates: [],
  createdAt: '2026-10-03T12:00:00.000Z', updatedAt: '2026-10-03T12:00:05.000Z',
  ...overrides
}) as never;

describe('runtime worker store', () => {
  beforeEach(() => {
    // The store is a module-level singleton; reset observed state between tests.
    const store = useMixStore();
    store.setApiState('checking');
    store.syncApiJobs([]);
  });

  it('never writes catalog state to browser storage', () => {
    const setItem = vi.fn();
    vi.stubGlobal('window', { localStorage: { setItem, getItem: () => null, removeItem: vi.fn() } });
    const store = useMixStore();
    store.syncApiJobs([job()]);
    store.updateProviderHealth([
      { id: 'youtube', state: 'ready', detail: 'ok', checkedAt: '2026-10-03T12:00:00.000Z' }
    ] as never);
    expect(store.state.jobs).toHaveLength(1);
    expect(store.state.providers.find((provider) => provider.id === 'youtube')?.state).toBe('ready');
    // Offline or unsaved work must never look committed.
    expect(setItem).not.toHaveBeenCalled();
  });

  it('reports every known provider even when the worker reports nothing', () => {
    const store = useMixStore();
    store.updateProviderHealth([] as never);
    // A provider without a health report is shown as unavailable, not omitted.
    expect(store.state.providers.length).toBeGreaterThan(0);
    expect(store.state.providers.every((provider) => provider.state === 'offline')).toBe(true);
    expect(store.state.providers.every((provider) => provider.name.length > 0)).toBe(true);
  });

  it('marks only unfinished jobs as running', () => {
    const store = useMixStore();
    store.syncApiJobs([
      job({ id: 'a', state: 'running' }),
      job({ id: 'b', state: 'queued' }),
      job({ id: 'c', state: 'done' })
    ]);
    expect(store.runningJobs.value).toBe(2);
  });

  it('replaces a job in place on upsert instead of duplicating it', () => {
    const store = useMixStore();
    store.syncApiJobs([job({ id: 'a', state: 'running' })]);
    store.upsertApiJob(job({ id: 'a', state: 'done', progress: 100 }));
    expect(store.state.jobs).toHaveLength(1);
    expect(store.state.jobs[0].state).toBe('done');
    expect(store.runningJobs.value).toBe(0);
  });

  it('derives a human-readable label from the discovery query', () => {
    const store = useMixStore();
    store.syncApiJobs([job({ query: { url: 'https://youtube.com/watch?v=x' } })]);
    expect(store.state.jobs[0].label).toBe('https://youtube.com/watch?v=x');
    store.syncApiJobs([job({ query: {} })]);
    expect(store.state.jobs[0].label).toBe('manual discovery');
  });
});
