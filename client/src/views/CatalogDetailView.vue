<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue';
import { useRoute } from 'vue-router';
import { missingFields, normalizeName, type CatalogRecord, type EntityRecord, type EventRecord, type MixRecord, type RecordId } from '@syco23/catalog-domain';
import { useCatalogStore } from '../catalog/store';
import ArtworkFrame from '../components/ArtworkFrame.vue';
import WaveformStrip from '../components/WaveformStrip.vue';
import DuplicateMerge from '../components/DuplicateMerge.vue';
import CatalogEditor from '../components/CatalogEditor.vue';
import FieldEvidence from '../components/FieldEvidence.vue';
import { createWaveformJob, getWaveformJob } from '../services/api';

const route = useRoute();
const catalog = useCatalogStore;
const record = shallowRef<CatalogRecord>();
const linkedEntities = shallowRef<EntityRecord[]>([]);
const linkedEvents = shallowRef<EventRecord[]>([]);
const relatedMixes = shallowRef<MixRecord[]>([]);
const enriching = shallowRef(false);
const analyzing = shallowRef(false);
const analysisProgress = shallowRef(0);
const error = shallowRef('');
const possibleDuplicate = shallowRef<CatalogRecord>();

/**
 * Candidate duplicates for this record.
 *
 * A shared normalized name is only a *suggestion*: same-name artists and events at
 * different dates or venues are distinct records, and name similarity is never proof
 * of a duplicate. The curator still confirms which side survives.
 */
const duplicateCandidates = computed((): CatalogRecord[] => {
  const current = record.value;
  if (!current) return [];
  const ownName = normalizeName(current.kind === 'mix' ? current.title : current.kind === 'entity' ? current.displayName : current.name);
  const indexKind = current.kind === 'event' ? 'event' : 'mix';
  const candidates = catalog.state.records as readonly CatalogRecord[];
  return candidates.filter((candidate) => {
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
  }).filter((candidate) => candidate.kind === indexKind || indexKind === 'mix');
});

const recordId = computed(() => String(route.params.id || ''));
const title = computed(() => !record.value ? '' : record.value.kind === 'mix'
  ? record.value.title : record.value.kind === 'entity' ? record.value.displayName : record.value.name);
const fieldsMissing = computed(() => record.value ? missingFields(record.value) : []);
const heroAsset = computed(() => {
  if (!record.value) return undefined;
  if (record.value.kind === 'mix') return record.value.assets.find((asset) => asset.role === 'mix-cover');
  if (record.value.kind === 'entity') return record.value.assets.find((asset) => ['artist-portrait', 'crew-logo', 'label-logo'].includes(asset.role));
  return record.value.assets.find((asset) => asset.role === 'event-flyer');
});
const dateLabel = computed(() => {
  if (!record.value) return 'DATE NOT RECORDED';
  if (record.value.kind === 'mix') return record.value.recordingDate?.value ?? 'DATE NOT RECORDED';
  if (record.value.kind === 'event') return record.value.startDate?.value ?? 'DATE NOT RECORDED';
  return record.value.country ?? 'ENTITY PROFILE';
});
const waveform = computed(() => record.value?.kind === 'mix' ? record.value.assets.find((asset) => asset.role === 'waveform') : undefined);
const audioSource = computed(() => record.value?.kind === 'mix'
  ? record.value.sources.map((source) => source.url).find((source) => source
    && /^https:\/\/(?:[^/]+\.)?(?:freeteknomusic\.org|archive\.org)\//i.test(source)
    && /\.(?:mp3|flac|ogg|oga|wav|m4a|aac|aif|aiff)(?:[?#]|$)/i.test(source))
  : undefined);

async function loadRecord(id: string) {
  error.value = '';
  linkedEntities.value = [];
  linkedEvents.value = [];
  relatedMixes.value = [];
  try {
    const current = await catalog.loadDetail(id as RecordId);
    record.value = current;
    if (current.kind === 'mix') {
      const [entities, events] = await Promise.all([
        Promise.all(current.people.map((person) => catalog.loadDetail(person.entityId).catch(() => undefined))),
        Promise.all(current.eventIds.map((eventId) => catalog.loadDetail(eventId).catch(() => undefined)))
      ]);
      linkedEntities.value = entities.filter((item): item is EntityRecord => item?.kind === 'entity');
      linkedEvents.value = events.filter((item): item is EventRecord => item?.kind === 'event');
    }
    // Evidence is best-effort: the record still renders without it.
    await catalog.loadEvidence(current.id as RecordId);
    // Only an entity has inbound mix references; other kinds return an empty list.
    relatedMixes.value = await catalog.loadRelatedMixes(current.id as RecordId);
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Record could not be loaded';
  }
}

watch(recordId, (id) => { if (id) void loadRecord(id); }, { immediate: true });

/**
 * After a merge the survivor holds a new revision, so the record is reloaded rather
 * than patched locally: the server is the only authority on what was committed.
 */
async function onMerged(merged: CatalogRecord) {
  possibleDuplicate.value = undefined;
  try {
    await loadRecord(merged.id as string);
  } catch {
    record.value = merged;
  }
}

/** A committed edit advances the revision, so the record and its evidence reload. */
async function onFieldSaved(saved: CatalogRecord) {
  try {
    await loadRecord(saved.id as string);
  } catch {
    record.value = saved;
  }
}

async function runEnrichment() {
  if (!record.value || enriching.value) return;
  enriching.value = true;
  error.value = '';
  try {
    const report = await catalog.enrichRecord(record.value.id);
    record.value = await catalog.loadDetail(record.value.id);
    if (!report.applied && !report.reviewed) error.value = 'No fields added. Remaining gaps and provider outcomes are shown below.';
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Enrichment failed';
  } finally {
    enriching.value = false;
  }
}

async function runWaveformAnalysis() {
  if (!record.value || record.value.kind !== 'mix' || !audioSource.value || analyzing.value) return;
  analyzing.value = true;
  analysisProgress.value = 0;
  error.value = '';
  try {
    const started = await createWaveformJob(audioSource.value);
    for (let attempt = 0; attempt < 160; attempt += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 1500));
      const job = await getWaveformJob(started.id);
      analysisProgress.value = job.progress;
      if (job.state === 'error') throw new Error(job.error || 'Audio analysis failed');
      if (job.state === 'done') {
        if (!job.imageDataUrl) throw new Error('The worker returned no waveform image');
        record.value = await catalog.persistWaveform(record.value.id, job.imageDataUrl, audioSource.value);
        return;
      }
    }
    throw new Error('Audio analysis is still running; retry shortly');
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Audio analysis failed';
  } finally {
    analyzing.value = false;
  }
}
</script>

<template>
  <section v-if="record" class="page catalog-detail">
    <router-link :to="record.kind === 'mix' ? '/' : `/catalog/${record.kind === 'entity' ? record.roles[0] : 'event'}`" class="back-link">
      <q-icon name="mdi-arrow-left" /> Back to {{ record.kind === 'mix' ? 'mixes' : record.kind === 'entity' ? record.roles[0] + 's' : 'events' }}
    </router-link>

    <div class="catalog-detail__hero">
      <div class="catalog-detail__art"><ArtworkFrame :src="heroAsset?.url" :alt="title" /></div>
      <div class="catalog-detail__heading">
        <p class="kicker">{{ record.kind }} / {{ dateLabel }}</p>
        <h1>{{ title }}</h1>
        <p class="catalog-detail__subline" v-if="record.kind === 'mix'">{{ linkedEntities.map((entity) => entity.displayName).join(' · ') || 'Performers not confirmed' }}</p>
        <p class="catalog-detail__subline" v-else-if="record.kind === 'entity'">{{ record.roles.join(' / ') }}<template v-if="record.country"> · {{ record.country }}</template></p>
        <p class="catalog-detail__subline" v-else>{{ [record.venue, record.locality, record.country].filter(Boolean).join(' · ') || 'Location not recorded' }}</p>
        <div class="catalog-detail__actions">
          <button v-if="catalog.state.authenticated" class="btn btn--primary" :disabled="enriching" @click="runEnrichment">
            <q-icon :name="enriching ? 'mdi-loading mdi-spin' : 'mdi-database-sync-outline'" />
            {{ enriching ? 'Checking providers…' : 'Enrich missing fields' }}
          </button>
          <router-link v-else class="btn" to="/login"><q-icon name="mdi-lock-outline" /> Curator login</router-link>
          <router-link v-if="catalog.state.review.length" class="btn" to="/review"><q-icon name="mdi-source-merge" /> Review {{ catalog.state.review.length }}</router-link>
        </div>
        <div class="catalog-detail__sources">
          <a v-for="source in record.kind === 'mix' ? record.sources : record.kind === 'entity' ? record.providerRefs : []" :key="`${source.provider}:${source.resourceType}:${source.externalId}`" :href="source.url" target="_blank" rel="noopener noreferrer">
            {{ source.provider }} <q-icon name="mdi-open-in-new" size="12px" />
          </a>
        </div>
      </div>
    </div>

    <div class="missing-strip" aria-label="Remaining missing fields">
      <span>{{ fieldsMissing.length ? 'MISSING' : 'INDEXED' }}</span>
      <b v-if="!fieldsMissing.length">No fields currently missing</b>
      <b v-for="field in fieldsMissing" :key="field">{{ field }}</b>
    </div>

    <section v-if="record.kind === 'mix'" class="signal-panel">
      <div class="panel-head"><span>AUDIO ANALYSIS</span><b>{{ waveform ? 'WAVEFORM STORED' : analyzing ? `ANALYZING ${analysisProgress}%` : 'NOT RUN' }}</b></div>
      <WaveformStrip :image-data-url="waveform?.url" />
      <div class="waveform-action">
        <button v-if="catalog.state.authenticated && !waveform && audioSource" class="btn" type="button" :disabled="analyzing" @click="runWaveformAnalysis">
          <q-icon :name="analyzing ? 'mdi-loading mdi-spin' : 'mdi-waveform'" />
          {{ analyzing ? `Analyzing ${analysisProgress}%` : 'Analyze waveform' }}
        </button>
        <router-link v-else-if="!catalog.state.authenticated && audioSource && !waveform" class="btn" to="/login"><q-icon name="mdi-lock-outline" /> Curator login to analyze</router-link>
        <small v-else-if="!audioSource && !waveform">A direct public audio file is required for waveform analysis.</small>
        <small v-else-if="waveform">Waveform is stored on the shared archive.</small>
      </div>
    </section>

    <p v-if="error" class="catalog-error" role="alert">{{ error }}</p>

    <div v-if="catalog.state.authenticated" class="panel catalog-editor">
      <div class="panel-head"><span>CURATOR / REV {{ record.revision }}</span><b>EDIT FIELDS</b></div>
      <p class="catalog-editor__note">
        Only fields of this record type are editable. Each save is checked against revision
        {{ record.revision }}, so a change made elsewhere in another session is refused instead of overwriting it.
      </p>
      <CatalogEditor :record="record" @saved="onFieldSaved" />
    </div>

    <section class="panel">
      <div class="panel-head"><span>PROVENANCE</span><b>{{ catalog.state.evidence.length }} CLAIM{{ catalog.state.evidence.length === 1 ? '' : 'S' }}</b></div>
      <FieldEvidence :evidence="catalog.state.evidence" :selected-evidence="record.selectedEvidence" />
    </section>

    <div v-if="catalog.state.authenticated && duplicateCandidates.length" class="panel catalog-editor">
      <div class="panel-head"><span>DUPLICATE CANDIDATES</span><b>{{ duplicateCandidates.length }} SHARED NAME</b></div>
      <p class="catalog-editor__note">
        A matching name alone does not make these the same recording. Compare the fields below before merging.
      </p>
      <ul class="duplicate-candidates">
        <li v-for="candidate in duplicateCandidates" :key="candidate.id">
          <router-link :to="`/${candidate.kind === 'mix' ? 'mix' : candidate.kind === 'entity' ? 'entity' : 'event'}/${encodeURIComponent(candidate.id)}`">
            {{ candidate.kind === 'mix' ? candidate.title : candidate.kind === 'entity' ? candidate.displayName : candidate.name }}
            · rev {{ candidate.revision }}
          </router-link>
          <button class="btn" type="button" @click="possibleDuplicate = candidate">Compare &amp; merge…</button>
        </li>
      </ul>
      <DuplicateMerge
        v-if="possibleDuplicate"
        :survivor="record"
        :duplicate="possibleDuplicate"
        @merged="onMerged"
      />
    </div>

    <div class="catalog-detail__grid">
      <section class="panel">
        <div class="panel-head"><span>CANONICAL / {{ record.verification }}</span><b>METADATA</b></div>
        <dl class="meta-table">
          <template v-if="record.kind === 'mix'">
            <div><dt>DURATION</dt><dd>{{ record.durationMs ? `${Math.floor(record.durationMs / 60000)} min` : 'Not recorded' }}</dd></div>
            <div><dt>RECORDED</dt><dd>{{ record.recordingDate?.value ?? 'Not recorded' }}</dd></div>
            <div><dt>GENRES</dt><dd>{{ record.genres.join(' · ') || 'Not recorded' }}</dd></div>
            <div><dt>STYLES</dt><dd>{{ record.styles.join(' · ') || 'Not recorded' }}</dd></div>
            <div><dt>DESCRIPTION</dt><dd>{{ record.description || 'Not recorded' }}</dd></div>
          </template>
          <template v-else-if="record.kind === 'entity'">
            <div><dt>ALIASES</dt><dd>{{ record.aliases.join(' · ') || 'None recorded' }}</dd></div>
            <div><dt>COUNTRY</dt><dd>{{ record.country || 'Not recorded' }}</dd></div>
            <div><dt>PROFILE</dt><dd class="preline">{{ record.profile || 'Not recorded' }}</dd></div>
          </template>
          <template v-else>
            <div><dt>DATE</dt><dd>{{ record.startDate?.value || 'Not recorded' }}</dd></div>
            <div><dt>VENUE</dt><dd>{{ record.venue || 'Not recorded' }}</dd></div>
            <div><dt>LOCALITY</dt><dd>{{ record.locality || 'Not recorded' }}</dd></div>
            <div><dt>COUNTRY</dt><dd>{{ record.country || 'Not recorded' }}</dd></div>
          </template>
        </dl>
      </section>

      <section class="panel">
        <div class="panel-head">
          <span>{{ record.kind === 'mix' ? linkedEntities.length + linkedEvents.length
            : record.kind === 'event' ? record.mixIds.length
            : relatedMixes.length || record.providerRefs.length }} LINKS</span>
          <b>{{ record.kind === 'entity' ? 'MIXES / SOURCES' : 'CONNECTED RECORDS' }}</b>
        </div>
        <div v-if="record.kind === 'mix' && linkedEntities.length" class="linked-records">
          <router-link v-for="entity in linkedEntities" :key="entity.id" :to="`/entity/${encodeURIComponent(entity.id)}`">
            <span>{{ entity.roles.join(' / ') }}</span><strong>{{ entity.displayName }}</strong><q-icon name="mdi-arrow-top-right" />
          </router-link>
          <router-link v-for="event in linkedEvents" :key="event.id" :to="`/event/${encodeURIComponent(event.id)}`">
            <span>EVENT</span><strong>{{ event.name }}</strong><q-icon name="mdi-arrow-top-right" />
          </router-link>
        </div>
        <div v-else-if="record.kind === 'entity' && relatedMixes.length" class="linked-records">
          <router-link v-for="mix in relatedMixes" :key="mix.id" :to="`/mix/${encodeURIComponent(mix.id)}`">
            <span>MIX</span>
            <strong>{{ mix.title }}</strong>
            <small>{{ mix.recordingDate?.value ?? 'Date not recorded' }}</small>
            <q-icon name="mdi-arrow-top-right" />
          </router-link>
        </div>
        <div v-else-if="record.kind === 'entity'" class="linked-records">
          <a v-for="source in record.providerRefs" :key="`${source.provider}:${source.resourceType}:${source.externalId}`" :href="source.url" target="_blank" rel="noopener noreferrer">
            <span>{{ source.provider }} / {{ source.resourceType }}</span><strong>{{ source.externalId }}</strong><q-icon name="mdi-open-in-new" />
          </a>
        </div>
        <div v-else-if="record.kind === 'event' && record.mixIds.length" class="linked-records">
          <router-link v-for="mixId in record.mixIds" :key="mixId" :to="`/mix/${encodeURIComponent(mixId)}`"><span>MIX</span><strong>{{ mixId }}</strong><q-icon name="mdi-arrow-top-right" /></router-link>
        </div>
        <div v-else class="panel-empty"><strong>No linked records yet.</strong></div>
      </section>
    </div>
  </section>
  <section v-else class="page empty-state" role="status">
    <strong>{{ catalog.state.loading ? 'Loading shared record…' : error || 'Record not found.' }}</strong>
    <router-link to="/" class="btn">Return to archive</router-link>
  </section>
</template>