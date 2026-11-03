<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import type { IndexKind, CatalogRecord } from '@syco23/catalog-domain';
import CatalogEditor from '../components/CatalogEditor.vue';
import { recordLink } from '../catalog/views';
import CatalogRecordRow from '../catalog/CatalogRecordRow.vue';
import { useCatalogStore } from '../catalog/store';

const props = defineProps<{ indexKind?: IndexKind }>();
const route = useRoute();
const router = useRouter();
const catalog = useCatalogStore;
const query = ref(typeof route.query.q === 'string' ? route.query.q : '');
const page = ref(Math.max(1, Number(route.query.page) || 1));
const creating = ref(false);
const creatingBusy = ref(false);
const creationError = ref('');
const newRecord = shallowRef<CatalogRecord>();
const kinds: Array<{ id: IndexKind; label: string }> = [
  { id: 'mix', label: 'Mixes' },
  { id: 'artist', label: 'Artists' },
  { id: 'crew', label: 'Crews' },
  { id: 'label', label: 'Labels' },
  { id: 'event', label: 'Events' }
];

const kind = computed<IndexKind>(() => {
  const requested = String(props.indexKind || route.params.kind || 'mix');
  return kinds.some((item) => item.id === requested) ? requested as IndexKind : 'mix';
});

/** Public paths for the non-mix indexes, used for the tab links. */
const shortPath: Record<IndexKind, string> = {
  mix: '/',
  artist: '/artists',
  crew: '/crews',
  label: '/labels',
  event: '/events'
};
const title = computed(() => kinds.find((item) => item.id === kind.value)?.label ?? 'Mixes');
const pageCount = computed(() => Math.max(1, Math.ceil(catalog.state.total / catalog.state.pageSize)));
const rangeStart = computed(() => catalog.state.total ? (page.value - 1) * catalog.state.pageSize + 1 : 0);
const rangeEnd = computed(() => Math.min(page.value * catalog.state.pageSize, catalog.state.total));


function startCreation() {
  if (kind.value === 'mix') return;
  const base = { id: 'draft_record0123456789', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), revision: 1, verification: 'curator-confirmed' as const, reviewState: 'ready' as const, assets: [] };
  newRecord.value = kind.value === 'event' ? { ...base, kind: 'event', name: '', sourceUrls: [], mixIds: [] } : { ...base, kind: 'entity', displayName: '', roles: [kind.value], aliases: [], providerRefs: [] };
  creating.value = true; creationError.value = '';
}
async function createRecord(patch: Record<string, unknown>) {
  if (!newRecord.value || creatingBusy.value) return;
  creatingBusy.value = true; creationError.value = '';
  try {
    const values = Object.fromEntries(Object.entries(patch).filter(([, value]) => value !== null));
    const committed = await catalog.createRecord({ kind: newRecord.value.kind, ...values });
    creating.value = false; await router.push(recordLink(committed));
  } catch (caught) { creationError.value = caught instanceof Error ? caught.message : 'New record was not saved'; }
  finally { creatingBusy.value = false; }
}

async function load() {
  try {
    await catalog.loadIndex(kind.value, { page: page.value, pageSize: 25, query: query.value.trim() || undefined });
  } catch {
    return;
  }
}

function submitSearch() {
  page.value = 1;
  void router.replace({ query: query.value.trim() ? { q: query.value.trim() } : {} }).then(load);
}

function changePage(direction: number) {
  page.value = Math.min(pageCount.value, Math.max(1, page.value + direction));
  void router.replace({ query: { ...(query.value ? { q: query.value } : {}), ...(page.value > 1 ? { page: String(page.value) } : {}) } }).then(load);
}

watch(() => [kind.value, route.query.q, route.query.page], () => {
  creating.value = false;
  query.value = typeof route.query.q === 'string' ? route.query.q : '';
  page.value = Math.max(1, Number(route.query.page) || 1);
  void load();
});
onMounted(() => { void load(); });
</script>

<template>
  <section class="page catalog-index">
    <header class="page-hero page-hero--compact">
      <div>
        <p class="kicker">SHARED ARCHIVE / {{ kind.toUpperCase() }} INDEX</p>
        <h1>{{ title }}</h1>
        <p class="hero-copy">Canonical records with linked sources and explicit review status.</p>
      </div>
    </header>

    <nav class="catalog-tabs" aria-label="Archive indexes">
      <router-link v-for="item in kinds" :key="item.id" :to="item.id === 'mix' ? '/' : shortPath[item.id]">
        {{ item.label }}
      </router-link>
    </nav>

    <form class="catalog-toolbar" role="search" @submit.prevent="submitSearch">
      <label class="search-field">
        <q-icon name="mdi-magnify" size="19px" />
        <input v-model="query" type="search" :aria-label="`Search ${title.toLowerCase()}`" :placeholder="`Search ${title.toLowerCase()}…`" />
      </label>
      <button class="btn btn--primary" type="submit"><q-icon name="mdi-magnify" /> Search</button>
      <button v-if="catalog.state.authenticated && kind !== 'mix'" class="btn btn--primary" type="button" data-testid="create-record" @click="startCreation">Create {{ kind }}</button>
      <router-link v-if="kind === 'mix' && catalog.state.authenticated" class="btn" to="/import"><q-icon name="mdi-plus" /> Discover mixes</router-link>
    </form>

    <div v-if="creating && newRecord"><button class="btn" type="button" @click="creating = false">Cancel creation</button><p v-if="creationError" class="catalog-error" role="alert">{{ creationError }}</p><CatalogEditor :record="newRecord" :busy="creatingBusy" @save="createRecord" /></div>

    <div class="catalog-list-head">
      <strong>{{ catalog.state.total.toLocaleString() }} records</strong>
      <span v-if="catalog.state.total">{{ rangeStart }}–{{ rangeEnd }} · sorted by recent activity</span>
    </div>

    <div v-if="catalog.state.loading" class="panel-empty" role="status">Loading shared archive…</div>
    <div v-else-if="catalog.state.error" class="catalog-error" role="alert">
      <q-icon name="mdi-alert-circle-outline" />
      <span>{{ catalog.state.error }}</span>
      <button class="btn" type="button" @click="load">Retry</button>
    </div>
    <div v-else-if="catalog.state.records.length" class="catalog-list">
      <CatalogRecordRow v-for="record in catalog.state.records" :key="record.id" :record="record" />
    </div>
    <div v-else class="empty-state">
      <q-icon name="mdi-database-search-outline" size="38px" />
      <strong>{{ query ? `No ${title.toLowerCase()} match this search.` : `No ${title.toLowerCase()} are indexed yet.` }}</strong>
      <button v-if="query" class="btn" type="button" @click="query = ''; submitSearch()">Clear search</button>
      <router-link v-else-if="kind === 'mix'" class="btn" to="/import"><q-icon name="mdi-plus" /> Discover mixes</router-link>
    </div>

    <nav v-if="pageCount > 1" class="catalog-pagination" aria-label="Index pages">
      <button class="icon-btn" type="button" :disabled="page <= 1" aria-label="Previous page" @click="changePage(-1)"><q-icon name="mdi-chevron-left" /></button>
      <span>PAGE {{ page }} / {{ pageCount }}</span>
      <button class="icon-btn" type="button" :disabled="page >= pageCount" aria-label="Next page" @click="changePage(1)"><q-icon name="mdi-chevron-right" /></button>
    </nav>
  </section>
</template>