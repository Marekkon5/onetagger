<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue';
import { useRoute } from 'vue-router';
import { missingFields, normalizeName, type CatalogRecord, type RecordId, type EnrichmentRun, type AnalysisRun } from '@syco23/catalog-domain';
import { useCatalogStore } from '../catalog/store';
import { catalogApi, type FieldEvidenceEntry } from '../catalog/api';
import { recordName, recordLink, metadataRows, formatValue, enrichmentSummary } from '../catalog/views';
import { fieldLabel, providerLabel, KIND_LABELS, VERIFICATION_LABELS, statePillLabel, timeAgo } from '../catalog/presentation';
import ArtworkFrame from '../components/ArtworkFrame.vue';
import WaveformStrip from '../components/WaveformStrip.vue';
import CatalogEditor from '../components/CatalogEditor.vue';
import FieldEvidence from '../components/FieldEvidence.vue';
import DuplicateMerge from '../components/DuplicateMerge.vue';
import { createWaveformJob, getWaveformJob } from '../services/api';
const route = useRoute(); const catalog = useCatalogStore;
const record = shallowRef<CatalogRecord>(); const related = shallowRef<CatalogRecord[]>([]);
const evidence = shallowRef<FieldEvidenceEntry[]>([]); const runs = shallowRef<EnrichmentRun[]>([]);
const analysisRuns = shallowRef<AnalysisRun[]>([]);
const analysisInProgress = computed(() => analysisRuns.value.some((run) => ['queued', 'running'].includes(run.state)));
const saving = shallowRef(false); const enriching = shallowRef(false); const analyzing = shallowRef(false);
const analysisProgress = shallowRef(0); const error = shallowRef(''); const auxiliaryErrors = shallowRef<string[]>([]); const resultSummary = shallowRef('');
let loadGeneration = 0;

/**
 * Candidate duplicates for this record, suggested from the currently loaded index.
 *
 * A shared normalized name is only a *suggestion*: same-name artists and events at
 * different dates or venues are distinct records, and name similarity is never proof
 * of a duplicate. The curator still confirms which side survives by opening a
 * candidate with the duplicate comparison flow.
 */
const duplicateCandidates = computed((): CatalogRecord[] => {
  const current = record.value;
  if (!current) return [];
  const ownName = normalizeName(current.kind === 'mix' ? current.title : current.kind === 'entity' ? current.displayName : current.name);
  return (catalog.state.records as readonly CatalogRecord[]).filter((candidate) => {
    if (candidate.id === current.id || candidate.kind !== current.kind) return false;
    const name = normalizeName(candidate.kind === 'mix' ? candidate.title : candidate.kind === 'entity' ? candidate.displayName : candidate.name);
    if (name !== ownName) return false;
    if (current.kind === 'event' && candidate.kind === 'event') {
      // Same name is not the same event: a differing date or venue keeps them apart.
      const currentDate = current.startDate?.value ?? '';
      const candidateDate = candidate.startDate?.value ?? '';
      return currentDate !== '' && candidateDate !== '' && currentDate === candidateDate
        && (current.venue ?? '') === (candidate.venue ?? '');
    }
    return true;
  });
});
const recordId = computed(() => String(route.params.id || ''));
const title = computed(() => record.value ? recordName(record.value) : '');
const kindLabel = computed(() => record.value ? KIND_LABELS[record.value.kind] ?? record.value.kind : '');
const verificationLabel = computed(() => record.value ? VERIFICATION_LABELS[record.value.verification] ?? record.value.verification : '');
const fieldsMissing = computed(() => record.value ? missingFields(record.value) : []);
const rows = computed(() => record.value ? metadataRows(record.value) : []);
const heroAsset = computed(() => record.value?.assets.find((asset) => record.value?.kind === 'mix' ? asset.role === 'mix-cover' : record.value?.kind === 'entity' ? ['artist-portrait', 'crew-logo', 'label-logo'].includes(asset.role) : asset.role === 'event-flyer'));
const waveform = computed(() => record.value?.kind === 'mix' ? record.value.assets.find((asset) => asset.role === 'waveform') : undefined);
const audioSource = computed(() => record.value?.kind === 'mix' ? [...(record.value.playbackUrls ?? []), ...record.value.sources.map((source) => source.url)].find((source) => typeof source === 'string' && /^https:\/\/(?:[^/]+\.)?(?:freeteknomusic\.org|archive\.org)\//i.test(source) && /\.(?:mp3|flac|ogg|oga|wav|m4a|aac|aif|aiff)(?:[?#]|$)/i.test(source)) : undefined);
const indexLink = computed(() => record.value?.kind === 'mix' ? '/' : record.value?.kind === 'entity' ? `/${record.value.roles[0] ? `${record.value.roles[0]}s` : 'artists'}` : '/events');
const sources = computed(() => !record.value ? [] : record.value.kind === 'mix' ? record.value.sources.map((source) => ({ url: source.url, label: providerLabel(source.provider) })) : record.value.kind === 'entity' ? record.value.providerRefs.map((source) => ({ url: source.url, label: providerLabel(source.provider) })) : record.value.sourceUrls.map((url) => ({ url, label: 'Event source' })));
async function loadSupporting(id: RecordId, generation: number) {
  const responses = await Promise.allSettled([catalogApi.getRelated(id), catalogApi.getEvidence(id), catalogApi.getRuns(id), catalog.state.authenticated ? catalogApi.getAnalysisRuns(id) : Promise.resolve([])]);
  if (generation !== loadGeneration) return;
  const failures: string[] = [];
  const [connections, claims, history, analyses] = responses;
  if (connections.status === 'fulfilled') related.value = connections.value; else failures.push(`Connected records unavailable: ${String(connections.reason)}`);
  if (claims.status === 'fulfilled') evidence.value = claims.value; else failures.push(`Field evidence unavailable: ${String(claims.reason)}`);
  if (history.status === 'fulfilled') runs.value = history.value; else failures.push(`Enrichment history unavailable: ${String(history.reason)}`);
  if (analyses.status === 'fulfilled') analysisRuns.value = analyses.value; else failures.push(`Audio analysis history unavailable: ${String(analyses.reason)}`);
  auxiliaryErrors.value = failures;
}
async function loadRecord(id: string) {
  const generation = ++loadGeneration;
  record.value = undefined; analysisRuns.value = []; related.value = []; evidence.value = []; runs.value = []; error.value = ''; auxiliaryErrors.value = []; resultSummary.value = '';
  try {
    const current = await catalog.loadDetail(id as RecordId);
    if (generation !== loadGeneration) return;
    record.value = current; await loadSupporting(current.id, generation);
  } catch (caught) { if (generation === loadGeneration) error.value = caught instanceof Error ? caught.message : 'Record could not be loaded'; }
}
watch(recordId, (id) => { if (id) void loadRecord(id); }, { immediate: true });
async function save(patch: Record<string, unknown>) {
  if (!record.value || saving.value) return;
  saving.value = true; error.value = '';
  try { record.value = await catalog.updateRecord(record.value.id, patch, record.value.revision); await loadSupporting(record.value.id, loadGeneration); }
  catch (caught) { error.value = `${caught instanceof Error ? caught.message : 'Changes were not saved'}. The draft remains available. Reload the record before retrying a revision conflict.`; }
  finally { saving.value = false; }
}
async function runEnrichment() {
  if (!record.value || enriching.value) return;
  enriching.value = true; error.value = ''; resultSummary.value = '';
  try {
    const report = await catalog.enrichRecord(record.value.id); resultSummary.value = enrichmentSummary(report);
    record.value = await catalog.loadDetail(record.value.id); await loadSupporting(record.value.id, loadGeneration);
  } catch (caught) { error.value = caught instanceof Error ? caught.message : 'Enrichment failed'; }
  finally { enriching.value = false; }
}
async function runWaveformAnalysis(sourceOverride?: string) {
  const source = sourceOverride || audioSource.value;
  if (!record.value || !source || analyzing.value || analysisInProgress.value) return;
  const id = record.value.id;
  analyzing.value = true; error.value = ''; analysisProgress.value = 0;
  try {
    const started = await createWaveformJob(source, id);
    await loadSupporting(id, loadGeneration);
    for (let attempt = 0; attempt < 160; attempt += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 1500));
      const job = await getWaveformJob(started.id); analysisProgress.value = job.progress;
      if (job.state === 'error' || job.state === 'interrupted') throw new Error(job.error || 'Audio analysis was interrupted; retry is available');
      if (job.state === 'done') {
        record.value = await catalog.loadDetail(id); await loadSupporting(id, loadGeneration); return;
      }
    }
    throw new Error('Audio analysis is still running. Its status persists on the worker; check again shortly.');
  } catch (caught) { error.value = caught instanceof Error ? caught.message : 'Audio analysis failed'; }
  finally { analyzing.value = false; await loadSupporting(id, loadGeneration); }
}
async function merged(committed: CatalogRecord) { record.value = committed; await loadSupporting(committed.id, loadGeneration); }
</script>
<template>
  <section v-if="record" class="page catalog-detail">
    <router-link :to="indexLink" class="back-link"><q-icon name="mdi-arrow-left" /> Back to archive index</router-link>
    <div class="catalog-detail__hero">
      <div class="catalog-detail__art"><ArtworkFrame :src="heroAsset?.url" :alt="title" /></div>
      <div class="catalog-detail__heading">
        <p class="kicker">{{ kindLabel }} / {{ verificationLabel }}</p><h1>{{ title }}</h1>
        <p class="catalog-detail__subline" v-if="record.kind === 'mix'">{{ related.filter((item) => item.kind === 'entity').map(recordName).join(' · ') || 'Performers not confirmed' }}</p>
        <p class="catalog-detail__subline" v-else-if="record.kind === 'entity'">{{ record.roles.join(' / ') }}<template v-if="record.country"> · {{ record.country }}</template></p>
        <p class="catalog-detail__subline" v-else>{{ [record.venue, record.locality, record.country].filter(Boolean).join(' · ') || 'Location not recorded' }}</p>
        <div class="catalog-detail__actions">
          <button v-if="catalog.state.authenticated" class="btn btn--primary" :disabled="enriching" @click="runEnrichment">{{ enriching ? 'Checking providers…' : 'Enrich missing fields' }}</button>
          <router-link v-else class="btn" :to="`/login?redirect=${encodeURIComponent(route.fullPath)}`">Curator login</router-link>
          <router-link v-if="catalog.state.authenticated" class="btn" to="/review">Review evidence ({{ catalog.state.review.length }})</router-link>
          <button class="btn" type="button" @click="loadRecord(record.id)">Reload record</button>
        </div>
        <div class="catalog-detail__sources"><a v-for="(source, index) in sources" :key="index" :href="source.url" target="_blank" rel="noopener noreferrer">{{ source.label }} <q-icon name="mdi-open-in-new" size="12px" /></a></div>
        <small class="record-id">Archive ID: {{ record.id }}</small>
      </div>
    </div>
    <div class="missing-strip" aria-label="Remaining missing fields"><span>{{ fieldsMissing.length ? 'MISSING' : 'NO GAPS' }}</span><b v-if="!fieldsMissing.length">No fields currently missing</b><b v-for="field in fieldsMissing" :key="field">{{ fieldLabel(field) }}</b></div>
    <p v-if="resultSummary" class="enrichment-result" role="status">{{ resultSummary }}</p>
    <p v-if="error" class="catalog-error" role="alert">{{ error }}</p>
    <p v-for="failure in auxiliaryErrors" :key="failure" class="catalog-error" role="alert">{{ failure }}</p>
    <section v-if="record.kind === 'mix'" class="signal-panel">
      <div class="panel-head"><span>AUDIO ANALYSIS</span><b>{{ waveform ? 'WAVEFORM STORED' : analyzing ? `ANALYZING ${analysisProgress}%` : 'NOT RUN' }}</b></div>
      <WaveformStrip :image-data-url="waveform?.url" />
      <div class="waveform-action">
        <button v-if="catalog.state.authenticated && !waveform && audioSource" class="btn" type="button" :disabled="analyzing || analysisInProgress" @click="runWaveformAnalysis()">{{ analyzing ? `Analyzing ${analysisProgress}%` : 'Analyze waveform' }}</button>
        <small v-if="!audioSource && !waveform">A permitted direct public audio file is required for waveform analysis.</small><small v-if="waveform">Waveform is stored on the shared archive.</small>
        <button v-if="analysisInProgress" class="btn" type="button" @click="loadRecord(record.id)">Check analysis status</button>
      </div>
      <div v-if="catalog.state.authenticated && analysisRuns.length" class="analysis-history" data-testid="analysis-history">
        <article v-for="run in analysisRuns" :key="run.id" class="run-history__item">
          <strong>{{ statePillLabel('analysis', run.state) }} · {{ run.progress }}%</strong><p>Started {{ timeAgo(run.createdAt) }} · {{ run.sourceUrl }}</p>
          <p v-if="run.error" class="catalog-error" role="alert">{{ run.error }}</p>
          <button v-if="['error', 'interrupted'].includes(run.state)" class="btn" type="button" :disabled="analyzing || analysisInProgress" @click="runWaveformAnalysis(run.sourceUrl)">Retry analysis</button>
        </article>
      </div>
    </section>

    <div v-if="catalog.state.authenticated && duplicateCandidates.length" class="panel catalog-editor">
      <div class="panel-head"><span>DUPLICATE CANDIDATES</span><b>{{ duplicateCandidates.length }} SHARED NAME</b></div>
      <p class="catalog-editor__note">
        A matching name in the loaded index alone does not make these the same recording.
        Open a candidate to compare both records before merging.
      </p>
      <ul class="duplicate-candidates">
        <li v-for="candidate in duplicateCandidates" :key="candidate.id">
          <router-link :to="{ path: route.path, query: { ...route.query, duplicate: candidate.id } }">
            {{ candidate.kind === 'mix' ? candidate.title : candidate.kind === 'entity' ? candidate.displayName : candidate.name }}
            · revision {{ candidate.revision }}
          </router-link>
        </li>
      </ul>
    </div>

    <div class="catalog-detail__grid">
      <section class="panel"><div class="panel-head"><span>{{ verificationLabel }} · REVISION {{ record.revision }}</span><b>METADATA</b></div>
        <dl class="meta-table"><div v-for="row in rows" :key="row.field"><dt>{{ row.label }}</dt><dd class="preline">{{ formatValue(row.value) }}</dd></div></dl>
      </section>
      <section class="panel"><div class="panel-head"><span>{{ related.length }} LINKS</span><b>CONNECTED RECORDS</b></div>
        <div v-if="related.length" class="linked-records"><router-link v-for="item in related" :key="item.id" :to="recordLink(item)"><span>{{ item.kind === 'entity' ? item.roles.join(' / ') : KIND_LABELS[item.kind] ?? item.kind }}</span><strong>{{ recordName(item) }}</strong><q-icon name="mdi-arrow-top-right" /></router-link></div>
        <div v-else class="panel-empty">No connected records are confirmed yet.</div>
      </section>
    </div>
    <FieldEvidence :record="record" :evidence="evidence" />
    <section class="panel run-history" data-testid="run-history">
      <div class="panel-head"><span>ENRICHMENT / PERSISTED RESULTS</span><b>PROVIDER OUTCOMES</b></div>
      <p v-if="!runs.length" class="panel-empty">No enrichment run is recorded. Missing fields have not yet been checked.</p>
      <article v-for="run in runs" :key="run.id" class="run-history__item">
        <strong>{{ statePillLabel('run', run.state) }} · {{ timeAgo(run.startedAt) }}</strong>
        <p>Providers tried: {{ run.attemptedProviders.map(providerLabel).join(' · ') || 'None usable' }}</p>
        <p>{{ run.applied }} {{ run.applied === 1 ? 'field' : 'fields' }} added · {{ run.corroborated }} {{ run.corroborated === 1 ? 'source' : 'sources' }} agreed · {{ run.reviewed }} sent to Review</p>
        <p>{{ run.missingFields.length ? 'Still missing: ' + run.missingFields.map(fieldLabel).join(' · ') : 'No fields currently missing' }}</p>
        <p v-for="(failure, index) in run.errors" :key="index" class="provider-outcome">{{ providerLabel(failure.provider) }} unavailable: {{ failure.message }}</p>
        <button v-if="catalog.state.authenticated && ['interrupted', 'failed'].includes(run.state)" class="btn" :disabled="enriching" @click="runEnrichment">Retry enrichment</button>
      </article>
    </section>
    <CatalogEditor v-if="catalog.state.authenticated" :record="record" :busy="saving" @save="save" />
    <DuplicateMerge v-if="catalog.state.authenticated" :key="record.id" :record="record" @merged="merged" />
  </section>
  <section v-else class="page empty-state" role="status"><strong>{{ catalog.state.loading ? 'Loading shared record…' : error || 'Record not found.' }}</strong><router-link to="/" class="btn">Return to archive</router-link></section>
</template>
