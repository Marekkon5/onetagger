import { computed, reactive } from 'vue';
import type { ImportJob, ProviderId } from '../domain/types';
import {
  getDiscoveryJobs,
  type ApiDiscoveryJob,
  type ApiProviderHealth,
} from '../services/api';

export type ApiState = 'checking' | 'online' | 'offline';

/**
 * Presentation-only provider metadata. The worker owns live health (`state`, `detail`,
 * `checkedAt`); these static fields only describe how each provider is used and how it
 * is authenticated, so no runtime state is duplicated or invented here.
 */
const providerPresentation: Record<string, { name: string; mode: 'discover' | 'enrich' | 'both'; auth: 'none' | 'required' | 'recommended' }> = {
  freeteknomusic: { name: 'Freeteknomusic', mode: 'both', auth: 'none' },
  archiveorg: { name: 'Archive.org', mode: 'both', auth: 'none' },
  soundcloud: { name: 'SoundCloud', mode: 'both', auth: 'required' },
  youtube: { name: 'YouTube', mode: 'both', auth: 'required' },
  hearthis: { name: 'hearthis.at', mode: 'both', auth: 'none' },
  discogs: { name: 'Discogs', mode: 'enrich', auth: 'recommended' }
};

export interface ProviderView extends ApiProviderHealth {
  name: string;
  mode: 'discover' | 'enrich' | 'both';
  auth: 'none' | 'required' | 'recommended';
  /** Alias of `checkedAt`, kept for the provider view templates. */
  lastCheck: string;
}

/**
 * Runtime-only worker state: provider health and discovery job progress.
 *
 * This store deliberately holds no catalog records. The server catalog is the single
 * source of truth and is read through `client/src/catalog/store`. Nothing here is
 * persisted to browser storage, so a failed or offline write can never look committed
 * and a second browser never sees a private, unsaved copy of the archive.
 */
const state = reactive({
  jobs: [] as ImportJob[],
  providers: [] as ProviderView[],
  apiState: 'checking' as ApiState
});

/**
 * Merges worker health with the static presentation metadata so every known provider is
 * listed even when the worker reports nothing for it. An unreported provider is shown as
 * unavailable rather than silently omitted, so the UI never implies full coverage.
 */
function toProviderViews(health: ApiProviderHealth[]): ProviderView[] {
  const reported = new Map<string, ApiProviderHealth>(health.map((entry) => [entry.id as string, entry]));
  return Object.entries(providerPresentation).map(([id, presentation]) => {
    const entry = reported.get(id);
    return {
      id: id as ProviderId,
      name: presentation.name,
      mode: presentation.mode,
      auth: presentation.auth,
      state: entry?.state ?? 'offline',
      detail: entry?.detail ?? 'No health report from the worker',
      checkedAt: entry?.checkedAt ?? new Date(0).toISOString(),
      lastCheck: entry?.checkedAt ?? new Date(0).toISOString()
    };
  });
}

function jobLabel(job: ApiDiscoveryJob): string {
  const query = job.query || {};
  return String(query.url || query.q || query.artist || 'manual discovery');
}

/**
 * Normalizes a worker job into the shape the views render. The worker remains
 * authoritative for progress; this only adapts its payload.
 */
function toImportJob(job: ApiDiscoveryJob): ImportJob {
  return {
    id: job.id,
    provider: job.provider as ProviderId,
    label: jobLabel(job),
    state: job.state,
    progress: job.progress,
    scanned: job.scanned,
    found: job.found,
    createdAt: job.createdAt,
    updatedAt: job.updatedAt,
    ...(job.error ? { error: job.error } : {})
  } as ImportJob;
}

export function useMixStore() {
  const runningJobs = computed(() => state.jobs.filter((job) => job.state === 'running' || job.state === 'queued').length);

  function upsertApiJob(job: ApiDiscoveryJob) {
    const next = toImportJob(job);
    const index = state.jobs.findIndex((item) => item.id === next.id);
    if (index >= 0) state.jobs[index] = next;
    else state.jobs = [next, ...state.jobs];
  }

  /** Replaces the local job list with the worker's current view. */
  function syncApiJobs(jobs: ApiDiscoveryJob[]) {
    state.jobs = jobs.map(toImportJob);
  }

  async function refreshJobs() {
    state.jobs = (await getDiscoveryJobs()).map(toImportJob);
  }

  function updateProviderHealth(health: ApiProviderHealth[]) {
    state.providers = toProviderViews(health);
  }

  function setApiState(value: ApiState) {
    state.apiState = value;
  }

  return {
    state,
    runningJobs,
    upsertApiJob,
    syncApiJobs,
    refreshJobs,
    updateProviderHealth,
    setApiState
  };
}
