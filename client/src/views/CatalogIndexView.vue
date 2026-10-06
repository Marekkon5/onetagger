<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import type { IndexKind } from '@syco23/catalog-domain';
import CatalogRecordRow from '../catalog/CatalogRecordRow.vue';
import { useCatalogStore } from '../catalog/store';

// Short paths such as /artists pass their kind as a prop; /catalog/:kind uses the
// parameter. Both must resolve to the same index, otherwise a shared short link
// would silently show the mix index.
const props = defineProps<{ kind?: IndexKind }>();

const route = useRoute();
const router = useRouter();
const catalog = useCatalogStore;
const query = ref(typeof route.query.q === 'string' ? route.query.q : '');
const page = ref(1);
const kinds: Array<{ id: IndexKind; label: string }> = [
  { id: 'mix', label: 'Mixes' },
  { id: 'artist', label: 'Artists' },
  { id: 'crew', label: 'Crews' },
  { id: 'label', label: 'Labels' },
  { id: 'event', label: 'Events' }
];

const kind = computed<IndexKind>(() => {
  const requested = String(props.kind || route.params.kind || 'mix');
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

async function load() {
  try {
    await catalog.loadIndex(kind.value, { page: page.value, pageSize: 25, query: query.value.trim() || undefined });
  } catch {
    return;
  }
}

function submitSearch() {
  page.value = 1;
  void router.replace({ query: query.value.trim() ? { q: query.value.trim() } : {} });
  void load();
}

function changePage(direction: number) {
  page.value = Math.min(pageCount.value, Math.max(1, page.value + direction));
  void load();
}

watch(kind, () => { page.value = 1; void load(); });
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
      <router-link v-if="kind === 'mix'" class="btn" to="/import"><q-icon name="mdi-plus" /> Discover mixes</router-link>
    </form>

    <div class="catalog-list-head">
      <strong>{{ catalog.state.total.toLocaleString() }} records</strong>
      <span>{{ rangeStart }}–{{ rangeEnd }} · sorted by recent activity</span>
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
      <strong>No {{ title.toLowerCase() }} match this search.</strong>
      <button class="btn" type="button" @click="query = ''; submitSearch()">Clear search</button>
    </div>

    <nav v-if="pageCount > 1" class="catalog-pagination" aria-label="Index pages">
      <button class="icon-btn" type="button" :disabled="page <= 1" aria-label="Previous page" @click="changePage(-1)"><q-icon name="mdi-chevron-left" /></button>
      <span>PAGE {{ page }} / {{ pageCount }}</span>
      <button class="icon-btn" type="button" :disabled="page >= pageCount" aria-label="Next page" @click="changePage(1)"><q-icon name="mdi-chevron-right" /></button>
    </nav>
  </section>
</template>