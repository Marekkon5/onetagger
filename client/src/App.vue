<template>
  <div class="app-shell">
    <aside class="side-rail">
      <router-link to="/" class="brand-mark" aria-label="SYCO23 Mixsets">
        <span class="brand-mark__sig">S//23</span>
        <span class="brand-mark__name">MIXSETS</span>
      </router-link>

      <nav class="main-nav" aria-label="Primary navigation">
        <router-link v-for="item in nav" :key="item.to" :to="item.to" class="nav-link">
          <q-icon :name="item.icon" size="20px" />
          <span>{{ item.label }}</span>
          <b v-if="item.badge">{{ item.badge }}</b>
        </router-link>
      </nav>

      <div class="rail-status">
        <span class="pulse-dot" :data-state="state.apiState"></span>
        <div>
          <small>{{ state.apiState === 'offline' ? 'LOCAL CACHE' : 'WORKER' }}</small>
          <strong>{{ railStatus }}</strong>
        </div>
      </div>
    </aside>

    <main class="app-main">
      <div class="top-line">
        <div><span>SYCO23 / SYSTEM CORRUPT</span><b>LONGFORM SIGNAL ARCHIVE</b></div>
        <div class="top-line__right">
          <span>{{ runtimeLabel }}</span>
          <i :data-state="state.apiState"></i>
          <button class="curator-status" type="button" @click="toggleCurator" :disabled="authOff">
            <q-icon :name="catalog.state.authenticated ? 'mdi-account-check-outline' : 'mdi-account-lock-outline'" />
            {{ authOff ? 'OPEN ACCESS' : catalog.state.authenticated ? 'CURATOR' : 'PUBLIC' }}
          </button>
        </div>
      </div>
      <div class="mobile-runtime" :data-state="state.apiState">
        <span></span>{{ runtimeLabel }}
        <router-link v-if="!authOff" to="/login">{{ catalog.state.authenticated ? 'CURATOR' : 'PUBLIC' }}</router-link>
        <span v-else>OPEN ACCESS</span>
      </div>
      <router-view />
    </main>

    <nav class="mobile-nav" aria-label="Primary navigation">
      <router-link v-for="item in nav" :key="item.to" :to="item.to">
        <q-icon :name="item.icon" size="21px" />
        <span>{{ item.mobile }}</span>
        <b v-if="item.badge">{{ item.badge }}</b>
      </router-link>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useMixStore } from './composables/useMixStore';
import { useCatalogStore } from './catalog/store';
import { getDiscoveryJobs, getProviderHealth } from './services/api';

const {
  state,
  runningJobs,
  syncApiJobs,
  updateProviderHealth,
  setApiState,
} = useMixStore();
const catalog = useCatalogStore;
const authOff = import.meta.env.VITE_AUTH_MODE === 'off';
const router = useRouter();
const reviewCount = computed(() => catalog.state.review.length);

const nav = computed(() => [
  { to: '/', label: 'Library', mobile: 'Library', icon: 'mdi-view-grid-outline', badge: catalog.state.total },
  { to: '/import', label: 'Import / Crawl', mobile: 'Import', icon: 'mdi-radar', badge: runningJobs.value },
  { to: '/local-tags', label: 'Local Tagger', mobile: 'Tag MP3', icon: 'mdi-folder-music-outline', badge: 0 },
  { to: '/review', label: 'Review Queue', mobile: 'Review', icon: 'mdi-source-merge', badge: reviewCount.value },
  { to: '/providers', label: 'Providers', mobile: 'Sources', icon: 'mdi-server-network-outline', badge: 0 },
]);

const runtimeLabel = computed(() => {
  if (state.apiState === 'online') return 'WORKER ONLINE';
  if (state.apiState === 'offline') return 'WORKER OFFLINE · LOCAL CACHE';
  return 'CHECKING WORKER…';
});

const railStatus = computed(() => {
  const jobs = runningJobs.value ? `${runningJobs.value} ${runningJobs.value === 1 ? 'JOB' : 'JOBS'}` : '';
  const connection = state.apiState === 'online' ? 'ONLINE / IDLE' : state.apiState === 'checking' ? 'CHECKING' : 'LOCAL ONLY';
  return jobs ? `${jobs} · ${connection}` : connection;
});

onMounted(async () => {
  setApiState('checking');
  try {
    const authenticated = await catalog.checkSession();
    if (authenticated) {
      const jobs = await getDiscoveryJobs();
      syncApiJobs(jobs);
      await catalog.loadReview();
    } else {
      syncApiJobs([]);
    }
    setApiState('online');
  } catch {
    setApiState('offline');
  }
  // Provider health is a separate concern: a slow or unavailable provider registry
  // must not report the whole worker as offline.
  getProviderHealth().then(updateProviderHealth).catch(() => {});
});

async function toggleCurator() {
  if (authOff) return;
  if (!catalog.state.authenticated) {
    await router.push('/login');
    return;
  }
  try {
    await catalog.logout();
  } catch {
    await router.push('/login');
  }
}
</script>
