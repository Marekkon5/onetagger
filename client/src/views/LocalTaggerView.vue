<template>
  <section class="page local-tagger">
    <header class="page-hero page-hero--compact">
      <div>
        <p class="kicker">LOCAL AUDIO / SEQUENTIAL TAGGING</p>
        <h1>Curate the files you own.</h1>
        <p class="hero-copy">Scan a folder of MP3s, enrich only strong title-and-artist matches, then write normalized ID3 tags and front covers to a separate output folder.</p>
      </div>
    </header>

    <div class="local-tagger__pick">
      <button class="btn btn--primary" :disabled="busy !== ''" @click="chooseFolder">
        <q-icon :name="scanning ? 'mdi-loading mdi-spin' : 'mdi-folder-music-outline'" />
        {{ scanning ? 'Scanning folder…' : folder ? 'Choose another folder' : 'Choose MP3 folder' }}
      </button>
      <div>
            <strong>{{ folder?.name || 'No folder selected' }}</strong>
        <small>Audio stays in this browser. Originals are never changed.</small>
      </div>
    </div>

    <div v-if="errorMessage" class="local-tagger__error" role="alert">{{ errorMessage }}</div>

    <div class="local-tagger__layout">
      <section class="panel local-tagger__controls">
        <div class="panel-head"><span>ENRICHMENT SOURCES</span><b>{{ selectedProviders.length }} SELECTED</b></div>
        <div v-if="!providersLoaded" class="local-tagger__loading" role="status">Checking provider availability…</div>
        <div v-else class="local-tagger__providers">
          <label v-for="provider in state.providers" :key="provider.id" class="local-provider">
            <input
              v-model="selectedProviders"
              type="checkbox"
              :value="provider.id"
              :disabled="busy !== '' || providerUnavailable(provider.id, provider.state)"
            />
            <span class="local-provider__name">{{ provider.name }}</span>
            <small :data-state="provider.state">{{ provider.state }}</small>
          </label>
        </div>
        <p class="local-tagger__note">Only missing fields are added. Automatic matching requires a meaningful title and at least one artist. Existing tags and covers take priority.</p>
        <p v-if="!authenticated" class="local-tagger__login">Provider lookup requires curator access. <router-link to="/login?redirect=/local-tags">Sign in to enrich files</router-link></p>

        <button class="btn btn--wide" :disabled="!canAutoTag" data-testid="auto-tag" @click="autoTagAndEnrich">
          <q-icon :name="busy === 'auto' ? 'mdi-loading mdi-spin' : 'mdi-auto-fix'" />
          {{ busy === 'auto' ? 'Auto-tagging accepted files…' : 'Auto-tag from filenames' }}
        </button>
        <p class="local-tagger__note">Auto-tag reads artist, title and year from each filename. Files with at least {{ autoAcceptPercent }}% evidence are enriched automatically; weaker names are left for you.</p>
        <label v-if="authenticated" class="local-tagger__sync">
          <input v-model="syncLibrary" type="checkbox" :disabled="busy !== '' || syncing" data-testid="sync-library" />
          <span>Also sync accepted tracks to the shared archive during auto-tag</span>
        </label>
        <button v-if="authenticated" class="btn btn--wide" :disabled="!canSync" data-testid="sync-now" @click="syncAcceptedNow">
          <q-icon :name="syncing ? 'mdi-loading mdi-spin' : 'mdi-cloud-upload-outline'" />
          {{ syncing ? 'Syncing to the shared archive…' : `Sync ${acceptedCount} accepted to archive` }}
        </button>
        <button class="btn btn--wide" :disabled="!canEnrich" @click="enrichFolder">
          <q-icon :name="busy === 'enrich' ? 'mdi-loading mdi-spin' : 'mdi-database-search-outline'" />
          {{ busy === 'enrich' ? 'Enriching sequentially…' : `Enrich ${eligibleCount} eligible files` }}
        </button>
        <button class="btn btn--primary btn--wide" :disabled="!canWrite" @click="writeFolder">
          <q-icon :name="busy === 'write' ? 'mdi-loading mdi-spin' : 'mdi-content-save-check-outline'" />
          {{ busy === 'write' ? 'Writing tagged copies…' : `Write ${writableCount} tagged copies` }}
        </button>
        <p class="local-tagger__destination">Output: <strong>{{ folder ? `${folder.name}/Mixsets-tagged` : 'Choose a folder first' }}</strong></p>
      </section>

      <section class="panel local-tagger__files">
        <div class="panel-head">
          <span>{{ tracks.length }} MP3 FILES</span>
          <b>{{ acceptedCount }} AUTO-ACCEPTED · {{ completedCount }} DONE</b>
        </div>
        <div v-if="busy" class="local-tagger__progress" role="progressbar" :aria-valuenow="progress.done" :aria-valuemin="0" :aria-valuemax="progress.total">
          <i :style="{ width: `${progress.total ? (progress.done / progress.total) * 100 : 0}%` }"></i>
          <span>{{ progress.phase }} {{ progress.done }} / {{ progress.total }}</span>
        </div>
        <div v-if="tracks.length" class="local-track-list">
          <article v-for="track in tracks" :key="track.id" class="local-track" :data-state="track.state">
            <div class="local-track__top">
              <span class="local-track__path" :title="track.relativePath">{{ track.relativePath }}</span>
              <span class="local-track__state">{{ LOCAL_TRACK_STATE_LABELS[track.state] ?? track.state }}</span>
            </div>
            <div class="local-track__fields">
              <label>
                <span>TITLE</span>
                <input v-model.trim="track.title" :disabled="busy !== ''" :aria-label="`Title for ${track.relativePath}`" />
              </label>
              <label>
                <span>ARTISTS · separate with semicolons</span>
                <input v-model="track.artistInput" :disabled="busy !== ''" :aria-label="`Artists for ${track.relativePath}`" @change="updateArtists(track)" />
              </label>
            </div>
            <div class="local-track__meta">
              <span v-if="track.album">{{ track.album }}</span>
              <span v-if="track.year">{{ track.year }}</span>
              <span v-if="track.durationMs">{{ formatDuration(track.durationMs) }}</span>
              <span>{{ track.cover || track.coverUrl ? 'cover ready' : 'no cover' }}</span>
              <span v-if="track.genres.length">{{ track.genres.join(' · ') }}</span>
              <span v-if="track.evidence !== undefined" :data-accepted="track.accepted ? 'true' : 'false'">{{ track.accepted ? 'auto-tag' : 'review' }} · {{ percent(track.evidence) }}%</span>
              <span v-if="track.syncState === 'synced'" data-synced="true">synced</span>
              <span v-else-if="track.syncState === 'skipped'" data-synced="false">not synced</span>
              <span v-else-if="track.syncState === 'error'" data-synced="false">sync failed</span>
            </div>
            <small class="local-track__detail">{{ track.detail }}</small>
          </article>
        </div>
        <div v-else class="local-tagger__empty">
          <q-icon name="mdi-folder-open-outline" size="30px" />
          <strong>No local MP3s scanned.</strong>
          <small>Choose a folder to inspect its MP3 tags and embedded covers.</small>
        </div>
      </section>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue';
import { useRouter } from 'vue-router';
import { useMixStore } from '../composables/useMixStore';
import { catalogApi } from '../catalog/api';
import { ApiRequestError, enrichLocalTrackMetadata, fetchProviderArtwork, getProviderHealth } from '../services/api';
import type { ProviderId } from '../domain/types';
import { applyEnrichment, applyFilenameIdentity, AUTO_ACCEPT_EVIDENCE, buildTaggedMp3, hasSolidMetadataBase, normalizeTagText, providerFromSourceUrl, readLocalMp3, type LocalMp3Track } from '../local-mp3';
import { toImportCandidate } from '../catalog/views';
import { LOCAL_TRACK_STATE_LABELS } from '../catalog/presentation';

interface LocalFileHandle {
  readonly kind: 'file';
  readonly name: string;
  getFile(): Promise<File>;
  createWritable(): Promise<{ write(data: Blob): Promise<void>; close(): Promise<void>; abort?(): Promise<void> }>;
}

interface LocalDirectoryHandle {
  readonly kind: 'directory';
  readonly name: string;
  entries(): AsyncIterableIterator<[string, LocalDirectoryHandle | LocalFileHandle]>;
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<LocalDirectoryHandle>;
  getFileHandle(name: string, options?: { create?: boolean }): Promise<LocalFileHandle>;
}

interface DirectoryPickerWindow extends Window {
  showDirectoryPicker?: (options?: { mode: 'readwrite' }) => Promise<LocalDirectoryHandle>;
}

type LocalEntryHandle = LocalDirectoryHandle | LocalFileHandle;
const OUTPUT_DIRECTORY = 'Mixsets-tagged';
const MAX_FILES = 2500;

const router = useRouter();
const { state, updateProviderHealth, setApiState } = useMixStore();
const folder = shallowRef<LocalDirectoryHandle | null>(null);
const tracks = shallowRef<LocalMp3Track[]>([]);
const selectedProviders = ref<ProviderId[]>([]);
const providersLoaded = ref(false);
const authenticated = ref(false);
const scanning = ref(false);
const busy = ref<'' | 'enrich' | 'write' | 'auto'>('');
const errorMessage = ref('');
const syncLibrary = ref(false);
const syncing = ref(false);
const progress = ref({ done: 0, total: 0, phase: '' });
let alive = true;

const eligibleCount = computed(() => tracks.value.filter((track) => track.state !== 'error' && hasSolidMetadataBase(track)).length);
const writableTracks = computed(() => tracks.value.filter((track) => {
  // Only files that could actually be read can be written. A track that failed while
  // writing kept its original on disk, so it remains retryable.
  const readable = track.state !== 'error' || track.detail.startsWith('Write failed');
  // Once auto-tag has judged a filename, only accepted files are written automatically.
  return readable && track.accepted !== false;
}));
const writableCount = computed(() => writableTracks.value.length);
const completedCount = computed(() => tracks.value.filter((track) => track.state === 'written' || track.state === 'enriched' || track.state === 'skipped').length);
const canEnrich = computed(() => Boolean(authenticated.value && folder.value && tracks.value.length && !busy.value && selectedProviders.value.length && eligibleCount.value));
const canWrite = computed(() => Boolean(folder.value && writableCount.value && !busy.value));
const acceptedCount = computed(() => tracks.value.filter((track) => track.accepted).length);
const canAutoTag = computed(() => Boolean(folder.value && tracks.value.length && !busy.value && !syncing.value));
const canSync = computed(() => Boolean(authenticated.value && acceptedCount.value && !busy.value && !syncing.value));
const autoAcceptPercent = Math.round(AUTO_ACCEPT_EVIDENCE * 100);
function percent(value: number) { return Math.round(value * 100); }

async function collectMp3Files(directory: LocalDirectoryHandle, parent = '', result: Array<{ path: string; handle: LocalFileHandle }> = []) {
  for await (const [name, entry] of directory.entries()) {
    if (parent === '' && entry.kind === 'directory' && name === OUTPUT_DIRECTORY) continue;
    const path = parent ? `${parent}/${name}` : name;
    if (entry.kind === 'directory') {
      await collectMp3Files(entry, path, result);
    } else if (/\.mp3$/i.test(name)) {
      result.push({ path, handle: entry });
      if (result.length > MAX_FILES) throw new Error(`Folder contains more than ${MAX_FILES} MP3 files; split it into smaller batches.`);
    }
  }
  return result.sort((left, right) => left.path.localeCompare(right.path, undefined, { numeric: true, sensitivity: 'base' }));
}

async function chooseFolder() {
  errorMessage.value = '';
  const picker = (window as DirectoryPickerWindow).showDirectoryPicker;
  if (!picker) {
    errorMessage.value = 'This browser cannot write to a local folder. Open the app in a current Chromium browser on localhost or HTTPS.';
    return;
  }
  try {
    const selectedFolder = await picker({ mode: 'readwrite' });
    folder.value = selectedFolder;
    tracks.value = [];
    scanning.value = true;
    progress.value = { done: 0, total: 0, phase: 'Scanning' };
    const files = await collectMp3Files(selectedFolder);
    progress.value = { done: 0, total: files.length, phase: 'Reading tags' };
    for (const entry of files) {
      if (!alive) return;
      try {
        const file = await entry.handle.getFile();
        const track = await readLocalMp3(file, entry.path);
        tracks.value = [...tracks.value, track];
      } catch (error) {
        errorMessage.value = error instanceof Error ? error.message : 'Could not read the selected folder';
      }
      progress.value = { ...progress.value, done: progress.value.done + 1 };
    }
    if (!files.length) errorMessage.value = 'No MP3 files were found in this folder.';
  } catch (error) {
    if (!(error instanceof DOMException && error.name === 'AbortError')) {
      errorMessage.value = error instanceof Error ? error.message : 'Could not scan the selected folder';
    }
  } finally {
    scanning.value = false;
    progress.value = { done: 0, total: 0, phase: '' };
  }
}

function updateArtists(track: LocalMp3Track) {
  track.artists = [...new Set(track.artistInput.split(';').map(normalizeTagText).filter(Boolean))];
  if (track.state === 'skipped') {
    track.state = 'scanned';
    track.detail = hasSolidMetadataBase(track) ? 'Metadata base ready' : 'Add a meaningful title and at least one artist to enrich';
  }
}

function providerUnavailable(id: ProviderId, status: 'ready' | 'limited' | 'offline') {
  return status === 'offline' || (['soundcloud', 'youtube'].includes(id) && status !== 'ready');
}

async function enrichTrack(track: LocalMp3Track): Promise<'ok' | 'skipped' | 'unauthorized' | 'failed'> {
  updateArtists(track);
  if (!hasSolidMetadataBase(track)) {
    track.state = 'skipped';
    track.detail = 'Needs a meaningful title and at least one artist';
    return 'skipped';
  }
  try {
    const result = await enrichLocalTrackMetadata({
      title: normalizeTagText(track.title),
      artists: track.artists,
      durationMs: track.durationMs,
      recordedAt: track.year ? String(track.year) : undefined,
      description: track.comment || undefined,
      genres: track.genres,
      artwork: track.cover || track.coverUrl ? [{ url: 'https://local.invalid/existing-cover', kind: 'cover' }] : [],
      providers: [...selectedProviders.value],
    });
    applyEnrichment(track, result);
    if (result.failures.length && !track.enrichedBy.length) {
      track.detail = `No confident update; ${result.failures.length} provider(s) unavailable`;
    }
    return 'ok';
  } catch (error) {
    if (error instanceof ApiRequestError && error.status === 401) return 'unauthorized';
    track.detail = `Provider request failed: ${error instanceof Error ? error.message : 'unknown error'}`;
    return 'failed';
  }
}

async function enrichFolder() {
  if (!canEnrich.value) return;
  busy.value = 'enrich';
  errorMessage.value = '';
  const batch = tracks.value.filter((track) => track.state !== 'error');
  progress.value = { done: 0, total: batch.length, phase: 'Enriching' };
  for (const track of batch) {
    if (!alive) return;
    if (await enrichTrack(track) === 'unauthorized') { await router.push('/login?redirect=/local-tags'); break; }
    progress.value = { ...progress.value, done: progress.value.done + 1 };
  }
  busy.value = '';
  progress.value = { done: 0, total: 0, phase: '' };
}

/**
 * Derives identity from each filename, scores the evidence, and then automatically
 * enriches only the files whose evidence reached the auto-accept rate. Everything below
 * the threshold is left for a curator and is never enriched or synced automatically.
 */
async function autoTagAndEnrich() {
  if (!canAutoTag.value) return;
  busy.value = 'auto';
  errorMessage.value = '';
  for (const track of tracks.value) {
    if (!alive) return;
    applyFilenameIdentity(track);
  }
  const accepted = tracks.value.filter((track) => track.accepted && track.state !== 'error');
  if (authenticated.value && selectedProviders.value.length) {
    progress.value = { done: 0, total: accepted.length, phase: 'Auto-enriching' };
    for (const track of accepted) {
      if (!alive) return;
      if (await enrichTrack(track) === 'unauthorized') { await router.push('/login?redirect=/local-tags'); break; }
      progress.value = { ...progress.value, done: progress.value.done + 1 };
    }
  }
  if (syncLibrary.value && authenticated.value) await syncAcceptedToLibrary();
  // `tracks` is a shallowRef, so reassign it to refresh the acceptance summary computed.
  tracks.value = [...tracks.value];
  busy.value = '';
  progress.value = { done: 0, total: 0, phase: '' };
}

/**
 * Optional library sync: uploads accepted, provider-backed local records to the shared
 * archive. A local file with no public provider source has no shared identity, so it is
 * reported as skipped rather than imported under a private path.
 */
async function syncAcceptedToLibrary() {
  if (!authenticated.value) return;
  syncing.value = true;
  for (const track of tracks.value) {
    if (!alive) return;
    if (!track.accepted) { track.syncState = 'skipped'; continue; }
    const provider = providerFromSourceUrl(track.sourceUrl);
    if (!provider || !track.sourceUrl) {
      track.syncState = 'skipped';
      track.detail = `${track.detail} · not synced (no public provider source)`;
      continue;
    }
    try {
      track.syncState = 'idle';
      const candidate = toImportCandidate({
        provider,
        title: normalizeTagText(track.title),
        artists: track.artists,
        crews: [],
        durationMs: track.durationMs,
        recordedAt: track.year ? String(track.year) : undefined,
        description: track.comment || undefined,
        genres: track.genres,
        artwork: track.coverUrl ? [track.coverUrl] : [],
        source: { provider, url: track.sourceUrl, externalId: track.sourceUrl },
        confidence: track.evidence ?? AUTO_ACCEPT_EVIDENCE,
        reasons: ['Accepted local auto-tag synced to the shared archive'],
        raw: {},
      });
      await catalogApi.importCandidate(candidate);
      track.syncState = 'synced';
      track.detail = `${track.detail} · synced to the shared archive`;
    } catch (error) {
      if (error instanceof ApiRequestError && error.status === 401) { await router.push('/login?redirect=/local-tags'); break; }
      track.syncState = 'error';
      track.detail = `Archive sync failed: ${error instanceof Error ? error.message : 'unknown error'}`;
    }
  }
  syncing.value = false;
}

async function syncAcceptedNow() {
  if (!canSync.value) return;
  await syncAcceptedToLibrary();
  tracks.value = [...tracks.value];
}

async function outputFileHandle(directory: LocalDirectoryHandle, relativePath: string): Promise<LocalFileHandle> {
  const parts = relativePath.split('/').filter(Boolean);
  const filename = parts.pop();
  if (!filename) throw new Error('MP3 output path is empty');
  let destination = directory;
  for (const part of parts) destination = await destination.getDirectoryHandle(part, { create: true });
  return destination.getFileHandle(filename, { create: true });
}

async function writeFolder() {
  if (!folder.value || !canWrite.value) return;
  busy.value = 'write';
  errorMessage.value = '';
  const batch = writableTracks.value;
  progress.value = { done: 0, total: batch.length, phase: 'Writing' };
  try {
    const output = await folder.value.getDirectoryHandle(OUTPUT_DIRECTORY, { create: true });
    for (const track of batch) {
      if (!alive) return;
      try {
        let cover: ArrayBuffer | undefined;
        if (!track.cover && track.coverUrl) cover = (await fetchProviderArtwork(track.coverUrl)).bytes;
        const tagged = buildTaggedMp3(await track.file.arrayBuffer(), track, cover);
        const target = await outputFileHandle(output, track.relativePath);
        const writable = await target.createWritable();
        try {
          await writable.write(tagged);
          await writable.close();
        } catch (error) {
          await writable.abort?.();
          throw error;
        }
        track.state = 'written';
        track.detail = `Tagged copy saved: ${OUTPUT_DIRECTORY}/${track.relativePath}`;
      } catch (error) {
        track.state = 'error';
        track.detail = `Write failed; original unchanged: ${error instanceof Error ? error.message : 'unknown error'}`;
      }
      progress.value = { ...progress.value, done: progress.value.done + 1 };
    }
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : 'Could not create the tagged output folder';
  } finally {
    busy.value = '';
    progress.value = { done: 0, total: 0, phase: '' };
  }
}

function formatDuration(milliseconds: number) {
  const minutes = Math.round(milliseconds / 60_000);
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return hours ? `${hours}h ${String(remainder).padStart(2, '0')}m` : `${remainder}m`;
}

onMounted(async () => {
  try {
    authenticated.value = await catalogApi.checkSession();
    setApiState('online');
  } catch {
    setApiState('offline');
  }
  // Provider availability is loaded independently so an unavailable provider registry
  // never reports the worker itself as offline.
  try {
    const health = await getProviderHealth();
    updateProviderHealth(health);
    selectedProviders.value = health.filter((provider) => !providerUnavailable(provider.id, provider.state)).map((provider) => provider.id);
  } catch {
    updateProviderHealth([]);
  }
  providersLoaded.value = true;
});

onBeforeUnmount(() => { alive = false; });
</script>