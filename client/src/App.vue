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
          <small>{{ state.apiState === 'online' ? 'WORKER' : 'INDEX STATE' }}</small>
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
          <button class="curator-status" type="button" @click="toggleCurator">
            <q-icon :name="catalog.state.authenticated ? 'mdi-account-check-outline' : 'mdi-account-lock-outline'" />
            {{ catalog.state.authenticated ? 'CURATOR' : 'PUBLIC' }}
          </button>
        </div>
      </div>
      <div class="mobile-runtime" :data-state="state.apiState">
        <span></span>{{ runtimeLabel }}
        <router-link to="/login">{{ catalog.state.authenticated ? 'CURATOR' : 'PUBLIC' }}</router-link>
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
const router = useRouter();
const reviewCount = computed(() => catalog.state.review.length);

const nav = computed(() => [
  { to: '/', label: 'Library', mobile: 'Library', icon: 'mdi-view-grid-outline', badge: 0 },
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
  if (runningJobs.value) return `${runningJobs.value} ${runningJobs.value === 1 ? 'JOB' : 'JOBS'}`;
  return state.apiState === 'online' ? 'ONLINE / IDLE' : state.apiState === 'checking' ? 'CHECKING' : 'LOCAL ONLY';
});

onMounted(async () => {
  setApiState('checking');
  try {
    const [health, authenticated] = await Promise.all([getProviderHealth(), catalog.checkSession()]);
    updateProviderHealth(health);
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
});

async function toggleCurator() {
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
