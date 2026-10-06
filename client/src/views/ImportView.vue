<template>
  <section class="page">
    <header class="page-hero page-hero--compact">
      <div>
        <p class="kicker">DISCOVERY / JOB CONTROL</p>
        <h1>Pull a signal. Keep the source.</h1>
        <p class="hero-copy">Imports run as bounded jobs. Discovery creates reviewable candidates; indexing can optionally fill missing metadata from the remaining available providers.</p>
      </div>
    </header>

    <div v-if="versionChecked && !versionOk" class="panel import-warning">
      <div class="panel-head"><span>DURABLE IMPORTS UNAVAILABLE</span><b>CAPABILITY CHECK</b></div>
      <p>{{ versionMessage }}</p>
    </div>

    <div class="import-layout">
      <section class="panel import-console">
        <div class="panel-head"><span>NEW JOB</span><b>DISCOVERY TARGET</b></div>

        <div class="provider-switch">
          <button
            v-for="provider in discoveryProviders"
            :key="provider.id"
            :class="{ active: selectedProvider === provider.id }"
            :disabled="providerBlocked(provider.id)"
            :title="providerBlocked(provider.id) ? provider.detail : provider.detail"
            @click="selectedProvider = provider.id"
          >
            <SourceBadge :provider="provider.id" />
            <small>{{ provider.mode }} · {{ provider.state }}</small>
          </button>
        </div>

        <label class="field-label" for="discovery-query">QUERY OR DIRECTORY URL</label>
        <textarea
          id="discovery-query"
          v-model="query"
          rows="4"
          :placeholder="selectedProvider === 'freeteknomusic'
            ? 'https://archive.freeteknomusic.org/metek/\nor: Metek live 1998'
            : selectedProvider === 'youtube' || selectedProvider === 'hearthis'
              ? 'Paste a public video/track URL, or search artist + mix title'
              : 'Artist + title, crew, event or other identifying terms'"
        ></textarea>

        <div class="split-fields" :class="{ 'split-fields--compact': selectedProvider !== 'freeteknomusic' }">
          <label v-if="selectedProvider === 'freeteknomusic'">
            <span>MAX DEPTH</span>
            <input v-model.number="maxDepth" type="number" min="0" max="6" />
          </label>
          <label v-if="selectedProvider === 'freeteknomusic'">
            <span>MAX ITEMS</span>
            <input v-model.number="maxItems" type="number" min="10" max="2500" step="10" />
          </label>
          <label v-else>
            <span>RESULT LIMIT</span>
            <input v-model.number="resultLimit" type="number" min="5" max="75" step="5" />
          </label>
          <label>
            <span>MIN DURATION</span>
            <select v-model="minDuration">
              <option :value="30">30 min</option>
              <option :value="45">45 min</option>
              <option :value="60">60 min</option>
            </select>
          </label>
        </div>

        <label class="enrichment-toggle">
          <input v-model="autoEnrich" type="checkbox" />
          <span>
            <strong>Auto-enrich missing metadata after indexing</strong>
            <small>Queries remaining available providers for missing artwork, description, date, genres and source IDs. Conflicts stay in Review.</small>
          </span>
        </label>

        <button class="btn btn--primary btn--wide" :disabled="submitting || !query.trim() || providerBlocked(selectedProvider) || youtubeTextSearchBlocked" @click="queueDiscovery">
          <q-icon :name="submitting ? 'mdi-loading mdi-spin' : 'mdi-radar'" />
          {{ submitting ? 'Contacting worker…' : 'Queue discovery job' }}
        </button>
        <p class="legal-note">Public metadata only. No protected-content bypass, no full-audio download during discovery.</p>

        <div v-if="versionOk" class="import-jobs">
          <div class="panel-head"><span>DURABLE IMPORT</span><b>METADATA / AUDIO JOB</b></div>

          <label class="field-label" for="import-url">SOURCE URL</label>
          <input
            id="import-url"
            v-model="importUrl"
            type="url"
            placeholder="https://soundcloud.com/example/example"
          />
          <div class="split-fields">
            <label>
              <span>MODE</span>
              <select v-model="importMode">
                <option value="metadata">metadata</option>
                <option value="audio">audio</option>
              </select>
            </label>
            <label>
              <span>RIGHTS BASIS</span>
              <select v-model="importRights">
                <option value="provider_metadata_only">provider metadata only</option>
                <option value="user_authorized_copy">user authorized copy</option>
                <option value="licensed_archive">licensed archive</option>
                <option value="rights_holder">rights holder</option>
                <option value="separate_agreement">separate agreement</option>
              </select>
            </label>
          </div>
          <p v-if="audioBlockedHint" class="legal-note">{{ audioBlockedHint }}</p>
          <button class="btn btn--primary btn--wide" :disabled="importSubmitting || !importUrl.trim()" @click="queueImport">
            <q-icon :name="importSubmitting ? 'mdi-loading mdi-spin' : 'mdi-tray-arrow-down'" />
            {{ importSubmitting ? 'Creating import…' : 'Create import job' }}
          </button>

          <div v-if="importJobs.length" class="job-list">
            <article v-for="job in importJobs" :key="job.job.id" class="job-row">
              <div class="job-row__body">
                <div><strong>{{ shortId(job.job.id) }}</strong><span>{{ job.job.state.toUpperCase() }}</span></div>
                <small>{{ job.job.provider }} · {{ job.job.mode }} · attempt {{ job.job.attempt }}</small>
                <small v-if="job.job.state === 'blocked_policy'" class="job-error">Blocked by provider policy — metadata import remains available.</small>
                <small v-if="job.job.error" class="job-error">{{ job.job.error }}</small>
                <div v-if="job.evidence.length" class="job-evidence">
                  <span v-for="score in job.evidence" :key="score.id">
                    {{ score.field }}: {{ score.score }} ({{ score.decision }})
                  </span>
                </div>
              </div>
              <button v-if="job.job.state === 'failed' || job.job.state === 'cancelled'" class="icon-btn" title="Retry" aria-label="Retry" @click="retryImport(job)">
                <q-icon name="mdi-refresh" />
              </button>
              <button v-if="job.job.state === 'completed' || job.job.state === 'review'" class="icon-btn" title="Finalize as catalog record" aria-label="Finalize as catalog record" @click="finalizeImport(job)">
                <q-icon name="mdi-check-circle" />
              </button>
            </article>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <span>{{ state.jobs.length ? 'ACTIVE / RECENT' : 'NO RECENT JOBS' }}</span>
          <b>JOB QUEUE</b>
        </div>

        <div v-if="state.jobs.length" class="job-list">
          <article v-for="job in state.jobs" :key="job.id" class="job-row">
            <div class="job-row__icon">
              <q-icon :name="job.state === 'running' || job.state === 'queued' ? 'mdi-loading mdi-spin' : job.state === 'error' ? 'mdi-alert-circle-outline' : 'mdi-radar'" />
            </div>
            <div class="job-row__body">
              <div><strong>{{ job.label }}</strong><span>{{ job.state.toUpperCase() }}</span></div>
              <small>{{ job.provider }} · {{ job.scanned }} scanned · {{ job.found }} candidates</small>
              <small v-if="job.error" class="job-error">{{ job.error }}</small>
              <div class="job-progress"><i :style="{ width: job.progress + '%' }"></i></div>
            </div>
            <button
              class="icon-btn"
              :title="jobActionLabel(job)"
              :aria-label="jobActionLabel(job)"
              @click="runJobAction(job)"
            >
              <q-icon :name="jobActionIcon(job)" />
            </button>
          </article>
        </div>

        <div v-else class="panel-empty">
          <q-icon name="mdi-radar" size="28px" />
          <strong>No worker jobs yet.</strong>
          <small>Queue a discovery request to start the index flow.</small>
        </div>
      </section>
    </div>

    <section v-if="discovered.length" class="panel discovery-panel">
      <div class="panel-head"><span>{{ discovered.length }} CANDIDATES</span><b>DISCOVERY RESULTS</b></div>
      <div class="discovery-list">
        <article v-for="candidate in discovered" :key="candidate.source.url" class="discovery-row">
          <div class="discovery-row__source">
            <SourceBadge :provider="candidate.provider" />
            <strong>{{ Math.round(candidate.confidence * 100) }}%</strong>
          </div>
          <div class="discovery-row__identity">
            <h3>{{ candidate.title }}</h3>
            <p>{{ candidate.artists.join(' · ') || 'Unknown artist' }}</p>
            <small>{{ candidate.reasons.join(' · ') }}</small>
          </div>
          <div class="discovery-row__meta">
            <span v-if="candidate.durationMs">{{ duration(candidate.durationMs) }}</span>
            <span v-if="candidate.recordedAt">{{ candidate.recordedAt }}</span>
          </div>

          <router-link
            v-if="indexedMix(candidate)"
            class="btn"
            :to="'/mix/' + encodeURIComponent(indexedMix(candidate)!.id)"
          >
            <q-icon name="mdi-open-in-new" /> Open indexed
          </router-link>
          <button
            v-else
            class="btn btn--primary"
            :disabled="busySources.has(candidate.source.url)"
            @click="importCandidate(candidate)"
          >
            <q-icon :name="busySources.has(candidate.source.url) ? 'mdi-loading mdi-spin' : 'mdi-plus'" />
            {{ busySources.has(candidate.source.url)
              ? (autoEnrich ? 'Index + enrich…' : 'Indexing…')
              : matchingMix(candidate)
                ? 'Merge source'
                : 'Add to index' }}
          </button>
        </article>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useQuasar } from 'quasar';
import { useRouter } from 'vue-router';
import SourceBadge from '../components/SourceBadge.vue';
import { useMixStore } from '../composables/useMixStore';
import { CatalogApiError, importJobsApi, type ApiVersion, type ImportJobDetail } from '../catalog/api';
import { useCatalogStore } from '../catalog/store';
import type { ImportJob, ProviderId } from '../domain/types';
import {
  cancelDiscoveryJob,
  createDiscoveryJob,
  getDiscoveryJob,
  getDiscoveryJobs,
  getProviderHealth,
  ApiRequestError,
  type ApiDiscoveryJob,
  type ApiMixCandidate,
  finalizeImportJob,
  type FinalizeImportJobInput,
  type FinalizeImportJobResult,
} from '../services/api';

const {
  state,
  syncApiJobs,
  upsertApiJob,
  updateProviderHealth,
  setApiState,
} = useMixStore();

const $q = useQuasar();
const router = useRouter();
const catalog = useCatalogStore;
const selectedProvider = ref<ProviderId>('freeteknomusic');
const query = ref('https://archive.freeteknomusic.org/metek/');
const maxDepth = ref(1);
const maxItems = ref(250);
const resultLimit = ref(25);
const minDuration = ref(45);
const submitting = ref(false);
const discovered = ref<ApiMixCandidate[]>([]);
const busySources = ref(new Set<string>());
const autoEnrich = ref(typeof window === 'undefined' ? true : window.localStorage.getItem('syco23.mixsets.autoEnrich') !== 'false');
let alive = true;

// Capability handshake: the client gates the whole import surface on the
// server's version/capability payload so an old UI never posts to a disabled
// or newer control plane.
const apiVersion = ref<ApiVersion | null>(null);
const versionChecked = ref(false);
const versionOk = computed(() => apiVersion.value?.importsEnabled === true && (apiVersion.value?.apiVersion ?? 0) >= 1);
const versionMessage = ref('');

async function checkVersion() {
  try {
    apiVersion.value = await importJobsApi.version();
    if (!apiVersion.value.importsEnabled) {
      versionMessage.value = 'Import jobs are disabled on this API (IMPORTS_ENABLED). Provider discovery remains available.';
    }
  } catch {
    versionMessage.value = 'The API version check failed; imports are unavailable.';
  } finally {
    versionChecked.value = true;
  }
}

// Durable import jobs (control plane): mode + rights are explicit, the server
// policy gate is authoritative and SoundCloud audio stays blocked there.
const importUrl = ref('');
const importMode = ref<'metadata' | 'audio'>('metadata');
const importRights = ref('provider_metadata_only');
const importSubmitting = ref(false);
const importJobs = ref<ImportJobDetail[]>([]);

const audioBlockedHint = computed(() => {
  if (importMode.value !== 'audio') return '';
  return `Server-side audio depends on provider capabilities and rights: SoundCloud audio stays blocked by policy, metadata imports remain available.`;
});

function shortId(id: string) {
  return id.length > 18 ? `${id.slice(0, 12)}…` : id;
}

async function queueImport() {
  if (!importUrl.value.trim() || !versionOk.value) return;
  importSubmitting.value = true;
  try {
    const detail = await importJobsApi.create({
      provider: selectedProvider.value,
      url: importUrl.value.trim(),
      mode: importMode.value,
      rightsBasis: importRights.value,
    });
    const existing = importJobs.value.findIndex((entry) => entry.job.id === detail.job.id);
    if (existing >= 0) importJobs.value[existing] = detail;
    else importJobs.value.unshift(detail);
    if (detail.job.state === 'blocked_policy') {
      $q.notify({ type: 'warning', message: detail.policy?.reason ?? 'Blocked by provider policy', timeout: 7000 });
    } else {
      $q.notify({ message: `Import ${detail.job.state}: ${shortId(detail.job.id)}`, position: 'top-right' });
      void pollImport(detail.job.id);
    }
  } catch (error) {
    if (error instanceof CatalogApiError && error.status === 401) {
      await router.push('/login?redirect=/import');
      return;
    }
    $q.notify({ type: 'negative', message: error instanceof Error ? error.message : 'Import could not be created.' });
  } finally {
    importSubmitting.value = false;
  }
}

async function pollImport(id: string) {
  for (let attempt = 0; attempt < 120 && alive; attempt += 1) {
    await new Promise((resolve) => window.setTimeout(resolve, 2000));
    try {
      const detail = await importJobsApi.get(id);
      const index = importJobs.value.findIndex((entry) => entry.job.id === id);
      if (index >= 0) importJobs.value[index] = detail;
      if (['completed', 'blocked_policy', 'failed', 'cancelled', 'review'].includes(detail.job.state)) {
        return;
      }
    } catch {
      return;
    }
  }
}

async function retryImport(job: ImportJobDetail) {
  try {
    const detail = await importJobsApi.retry(job.job.id);
    const index = importJobs.value.findIndex((entry) => entry.job.id === job.job.id);
    if (index >= 0) importJobs.value[index] = detail;
    void pollImport(detail.job.id);
  } catch (error) {
    $q.notify({ type: 'negative', message: error instanceof Error ? error.message : 'Retry failed.' });
  }
}

watch(autoEnrich, (value) => {
  window.localStorage.setItem('syco23.mixsets.autoEnrich', String(value));
});

const discoveryProviders = computed(() => state.providers.filter((provider) => provider.mode !== 'enrich'));
const youtubeTextSearchBlocked = computed(() => selectedProvider.value === 'youtube'
  && !/^https?:\/\//i.test(query.value.trim())
  && state.providers.find((provider) => provider.id === 'youtube')?.state !== 'ready');

function providerBlocked(id: ProviderId) {
  const provider = state.providers.find((item) => item.id === id);
  if (!provider) return false;
  if (id === 'youtube') return false;
  if (provider.state === 'offline') return true;
  return id === 'soundcloud' && provider.state !== 'ready';
}

function duration(ms: number) {
  const minutes = Math.round(ms / 60_000);
  return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
}

function indexedMix(candidate: ApiMixCandidate) {
  return catalog.state.records.find((record) => record.kind === 'mix'
    && record.sources.some((source) => source.provider === candidate.provider
      && (source.externalId === candidate.source.externalId || source.url === candidate.source.url)));
}

function matchingMix(candidate: ApiMixCandidate) {
  return indexedMix(candidate);
}

function providerResourceType(provider: ProviderId) {
  if (provider === 'youtube') return 'video';
  if (provider === 'soundcloud' || provider === 'hearthis') return 'track';
  if (provider === 'archiveorg') return 'item';
  return 'recording';
}

function providerQuery() {
  const input = query.value.trim();
  const next: Record<string, unknown> = {
    minDurationMs: minDuration.value * 60_000,
    limit: selectedProvider.value === 'freeteknomusic' ? 100 : resultLimit.value,
  };
  if (selectedProvider.value === 'freeteknomusic') {
    next.maxDepth = maxDepth.value;
    next.maxItems = maxItems.value;
    if (/^https?:/i.test(input)) next.url = input;
    else next.q = input;
  } else {
    if ((selectedProvider.value === 'youtube' || selectedProvider.value === 'hearthis') && /^https?:\/\//i.test(input)) next.url = input;
    else next.q = input;
  }
  return next;
}

async function pollJob(id: string) {
  for (let attempt = 0; attempt < 160 && alive; attempt += 1) {
    await new Promise((resolve) => window.setTimeout(resolve, 750));
    const job = await getDiscoveryJob(id);
    upsertApiJob(job);
    if (!['queued', 'running'].includes(job.state)) {
      discovered.value = job.candidates || [];
      if (job.state === 'error') {
        $q.notify({ type: 'negative', message: job.error || 'Discovery job failed' });
      } else {
        $q.notify({ message: `${job.found} candidates ready for review`, position: 'top-right' });
      }
      return;
    }
  }
}

async function queueDiscovery() {
  if (!query.value.trim()) return;
  submitting.value = true;
  discovered.value = [];
  try {
    const job = await createDiscoveryJob(selectedProvider.value, providerQuery());
    setApiState('online');
    upsertApiJob(job);
    void pollJob(job.id);
  } catch (error) {
    if (error instanceof ApiRequestError && error.status === 401) {
      await router.push('/login?redirect=/import');
    } else {
      setApiState('offline');
    }
    $q.notify({
      type: 'negative',
      message: error instanceof Error ? error.message : 'Worker unavailable. No job was queued.',
    });
  } finally {
    submitting.value = false;
  }
}

function jobActionLabel(job: ImportJob) {
  if (job.state === 'running' || job.state === 'queued') return 'Cancel job';
  if (job.state === 'review') return 'Show candidates';
  return 'Run job again';
}

function jobActionIcon(job: ImportJob) {
  if (job.state === 'running' || job.state === 'queued') return 'mdi-close';
  if (job.state === 'review') return 'mdi-eye-outline';
  return 'mdi-refresh';
}

async function runJobAction(job: ImportJob) {
  if (job.state === 'running' || job.state === 'queued') {
    try {
      upsertApiJob(await cancelDiscoveryJob(job.id));
    } catch {
      $q.notify({ type: 'negative', message: 'Could not cancel remote job.' });
    }
    return;
  }

  if (job.state === 'review') {
    try {
      const remote = await getDiscoveryJob(job.id);
      upsertApiJob(remote);
      discovered.value = remote.candidates || [];
      document.querySelector('.discovery-panel')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    } catch {
      $q.notify({ type: 'negative', message: 'Could not restore discovery results.' });
    }
    return;
  }

  selectedProvider.value = job.provider;
  const original = job.query || {};
  query.value = String(original.url || original.q || job.label);
  submitting.value = true;
  try {
    const remote = await createDiscoveryJob(job.provider, original);
    upsertApiJob(remote);
    void pollJob(remote.id);
  } catch (error) {
    $q.notify({ type: 'negative', message: error instanceof Error ? error.message : 'Could not rerun job.' });
  } finally {
    submitting.value = false;
  }
}

async function finalizeImport(job: ImportJobDetail) {
  const curatorPassword = prompt('Enter curator password to finalize this import job as a catalog record:');
  if (!curatorPassword) return;
  const alreadyFinalized = importJobs.value.find((entry) => entry.job.id === job.job.id && (entry as any).mixId);
  if (alreadyFinalized) {
    $q.notify({ type: 'positive', message: 'This import is already linked to a catalog record.' });
    return;
  }
  try {
    const result = await finalizeImportJob(job.job.id, { curatorPassword }) as FinalizeImportJobResult;
    const index = importJobs.value.findIndex((entry) => entry.job.id === job.job.id);
    if (index >= 0) {
      const updated = { ...importJobs.value[index], ...result };
      updated.job.mixId = result.mixId;
      importJobs.value[index] = updated;
    }
    const storeJob = state.jobs.find((entry) => entry.id === job.job.id);
    if (storeJob) {
      (storeJob as any).mixId = result.mixId;
      (storeJob as any).state = 'finalized';
    }
    const message = (result.claimsApplied > 0 ? `Finalized: ${(result.record as any).title} → ` : '') + `Finalized: ${shortId(result.mixId)} (${result.claimsApplied} claim(s) applied)`;
    $q.notify({ type: 'positive', message, timeout: 7000 });
    await catalog.loadReview();
  } catch (error) {
    if (error instanceof CatalogApiError && error.status === 401) {
      await router.push('/login?redirect=/import');
      return;
    }
    $q.notify({ type: 'negative', message: error instanceof Error ? error.message : 'Finalize failed.' });
  }
}

async function importCandidate(candidate: ApiMixCandidate) {
  busySources.value.add(candidate.source.url);
  try {
    const imported = await catalog.importCandidate({
      provider: candidate.provider,
      title: candidate.title,
      artists: candidate.artists || [],
      crews: candidate.crews || [],
      durationMs: candidate.durationMs,
      recordedAt: candidate.recordedAt,
      description: candidate.description,
      genres: candidate.genres || [],
      artwork: candidate.artwork || [],
      source: {
        provider: candidate.provider,
        resourceType: providerResourceType(candidate.provider),
        externalId: candidate.source.externalId || candidate.source.url,
        url: candidate.source.url
      },
      confidence: candidate.confidence,
      reasons: candidate.reasons || []
    });
    const mix = imported.record;
    if (mix.kind !== 'mix') throw new Error('The server returned a non-mix record for this import');
    if (!autoEnrich.value) {
      $q.notify({ message: `${imported.outcome === 'existing' ? 'Already indexed' : 'Indexed'}: “${mix.title}”`, position: 'top-right' });
      return;
    }

    const summary = await catalog.enrichRecord(mix.id);
    const message = summary.applied
      ? `${summary.applied} field(s) added`
      : 'No fields added';
    const review = summary.reviewed ? ` · ${summary.reviewed} claim(s) sent to Review` : '';
    const failures = summary.errors.length ? ` · ${summary.errors.length} provider result(s) unavailable` : '';
    $q.notify({
      message: `Indexed “${mix.title}”: ${message}${review}${failures} · ${summary.missingFields.length} fields still missing`,
      position: 'top-right',
      timeout: 7000,
    });
  } catch (error) {
    if (error instanceof CatalogApiError && error.status === 401) {
      await router.push('/login?redirect=/import');
      return;
    }
    $q.notify({
      type: 'negative',
      message: `No catalog change was confirmed: ${error instanceof Error ? error.message : String(error)}`,
      timeout: 5000,
    });
  } finally {
    busySources.value.delete(candidate.source.url);
  }
}

onMounted(async () => {
  try {
    const [health, authenticated, version] = await Promise.all([
      getProviderHealth(),
      catalog.checkSession(),
      checkVersion().then(() => apiVersion.value).catch(() => null),
    ]);
    updateProviderHealth(health);
    if (authenticated) {
      const jobs = await getDiscoveryJobs();
      syncApiJobs(jobs);
      await catalog.loadReview();
    } else {
      syncApiJobs([]);
    }
    setApiState('online');
    void version;
  } catch {
    setApiState('offline');
  }
});

onBeforeUnmount(() => {
  alive = false;
});
</script>
