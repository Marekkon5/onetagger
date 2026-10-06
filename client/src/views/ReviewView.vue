<script setup lang="ts">
import { onMounted, shallowRef } from 'vue';
import type { CatalogRecord, ReviewItem } from '@syco23/catalog-domain';
import { useCatalogStore } from '../catalog/store';

interface ReviewRow {
  item: ReviewItem;
  record?: CatalogRecord;
  error?: string;
  busy?: boolean;
}

const catalog = useCatalogStore;
const rows = shallowRef<ReviewRow[]>([]);
const loading = shallowRef(false);
const error = shallowRef('');

async function load() {
  loading.value = true;
  error.value = '';
  try {
    const items = await catalog.loadReview();
    rows.value = await Promise.all(items.map(async (item) => {
      try { return { item, record: await catalog.loadDetail(item.targetRecordId) }; }
      catch (caught) { return { item, error: caught instanceof Error ? caught.message : 'Record could not be loaded' }; }
    }));
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Review queue could not be loaded';
  } finally {
    loading.value = false;
  }
}

function recordName(record?: CatalogRecord) {
  if (!record) return 'Record unavailable';
  return record.kind === 'mix' ? record.title : record.kind === 'entity' ? record.displayName : record.name;
}

function format(value: unknown) {
  if (value === undefined || value === null || value === '') return 'No selected value';
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

async function decide(row: ReviewRow, decision: 'accept' | 'reject') {
  row.busy = true;
  row.error = undefined;
  try {
    await catalog.decideReview(row.item.id, decision, row.item.recordRevision);
    rows.value = rows.value.filter((item) => item.item.id !== row.item.id);
  } catch (caught) {
    row.error = caught instanceof Error ? caught.message : 'Decision was not saved';
  } finally {
    row.busy = false;
  }
}

async function refresh(row: ReviewRow) {
  row.busy = true;
  row.error = undefined;
  try {
    row.item = await catalog.refreshReview(row.item.id);
    row.record = await catalog.loadDetail(row.item.targetRecordId);
  } catch (caught) {
    row.error = caught instanceof Error ? caught.message : 'Review item could not be refreshed';
  } finally {
    row.busy = false;
  }
}

onMounted(() => { void load(); });
</script>

<template>
  <section class="page">
    <header class="page-hero page-hero--compact">
      <div>
        <p class="kicker">HUMAN REVIEW / SOURCE EVIDENCE</p>
        <h1>Accept fields, not guesses.</h1>
        <p class="hero-copy">Each proposal stays separate until a curator accepts it. Stale decisions must be refreshed before they can replace newer curation.</p>
      </div>
    </header>

    <div v-if="!catalog.state.authenticated" class="catalog-error" role="status">
      Curator login is required to view pending evidence.
      <router-link class="btn" to="/login?redirect=/review">Curator login</router-link>
    </div>
    <p v-else-if="loading" class="panel-empty" role="status">Loading review evidence…</p>
    <div v-else-if="error" class="catalog-error" role="alert">{{ error }} <button class="btn" @click="load">Retry</button></div>

    <div v-else-if="rows.length" class="review-stack">
      <article v-for="row in rows" :key="row.item.id" class="review-card evidence-review">
        <div class="review-card__identity evidence-review__identity">
          <div>
            <span class="kicker">CANONICAL / {{ row.item.field }}</span>
            <router-link :to="`/${row.record?.kind === 'mix' ? 'mix' : row.record?.kind === 'entity' ? 'entity' : 'event'}/${encodeURIComponent(row.item.targetRecordId)}`" class="review-title-link">
              <h2>{{ recordName(row.record) }}</h2>
            </router-link>
            <p>{{ format(row.item.currentValue) }}</p>
          </div>
        </div>
        <div class="candidate-card">
          <div class="candidate-card__top">
            <strong>{{ row.item.claim.provider.provider }} / {{ row.item.claim.provider.resourceType }}</strong>
            <a :href="row.item.claim.sourceUrl" target="_blank" rel="noopener noreferrer">Open source <q-icon name="mdi-open-in-new" size="12px" /></a>
          </div>
          <dl class="diff-list">
            <dt>PROPOSED</dt><dd class="evidence-value">{{ format(row.item.claim.value) }}</dd>
            <dt>WHY</dt><dd>{{ row.item.claim.matchExplanation }}</dd>
            <dt>OBSERVED</dt><dd>{{ row.item.claim.observedAt }}</dd>
            <dt>REVISION</dt><dd>{{ row.item.recordRevision }}</dd>
          </dl>
          <p v-if="row.error" class="catalog-error" role="alert">{{ row.error }}</p>
          <div class="review-actions">
            <button v-if="row.error?.toLowerCase().includes('revision') || (row.record && row.record.revision !== row.item.recordRevision)" class="btn" :disabled="row.busy" @click="refresh(row)"><q-icon name="mdi-refresh" /> Refresh evidence</button>
            <button class="btn" :disabled="row.busy || Boolean(row.record && row.record.revision !== row.item.recordRevision)" @click="decide(row, 'reject')">Reject</button>
            <button class="btn btn--primary" :disabled="row.busy || Boolean(row.record && row.record.revision !== row.item.recordRevision)" @click="decide(row, 'accept')"><q-icon name="mdi-check" /> Accept field</button>
          </div>
        </div>
      </article>
    </div>
    <div v-else-if="catalog.state.authenticated && !loading" class="empty-state">
      <q-icon name="mdi-check-decagram-outline" size="42px" />
      <strong>Review queue is clear.</strong>
      <router-link to="/import" class="btn">Discover more mixes</router-link>
    </div>
  </section>
</template>