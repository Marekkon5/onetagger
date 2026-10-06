<template>
  <section class="page">
    <header class="page-hero page-hero--compact">
      <div>
        <p class="kicker">PROVIDER REGISTRY / HEALTH</p>
        <h1>Sources stay explicit.</h1>
        <p class="hero-copy">Discovery and enrichment are separate capabilities. Provider state, auth requirements and limits are visible before a job starts.</p>
      </div>
    </header>

    <div class="section-heading provider-toolbar">
      <div>
        <span>{{ readyCount }} READY · {{ limitedCount }} LIMITED/OFFLINE</span>
        <strong>Provider health</strong>
      </div>
      <button class="btn" :disabled="checking" @click="refresh">
        <q-icon :name="checking ? 'mdi-loading mdi-spin' : 'mdi-refresh'" />
        {{ checking ? 'Checking registry…' : 'Refresh registry' }}
      </button>
    </div>

    <div class="provider-grid provider-grid--flush">
      <article v-for="provider in state.providers" :key="provider.id" class="provider-card">
        <div class="provider-card__top">
          <SourceBadge :provider="provider.id" />
          <span class="state-pill" :data-state="provider.state">{{ provider.state }}</span>
        </div>
        <h2>{{ provider.name }}</h2>
        <p>{{ provider.detail }}</p>
        <dl>
          <div><dt>MODE</dt><dd>{{ provider.mode }}</dd></div>
          <div><dt>AUTH</dt><dd>{{ provider.auth }}</dd></div>
          <div><dt>LAST CHECK</dt><dd>{{ formatCheck(provider.lastCheck) }}</dd></div>
        </dl>
        <button
          v-if="provider.id === 'soundcloud' || provider.id === 'discogs' || provider.id === 'youtube'"
          class="btn provider-card__setup"
          type="button"
          aria-controls="provider-setup"
          :aria-expanded="setupProvider === provider.id"
          @click="setupProvider = setupProvider === provider.id ? null : provider.id"
        >
          <q-icon name="mdi-cog-outline" />
          {{ provider.state === 'ready' ? 'Setup details' : 'Set up provider' }}
        </button>
      </article>
    </div>

    <section v-if="setupProvider" id="provider-setup" class="panel provider-setup" aria-label="Provider setup guide">
      <div class="panel-head"><span>PRIVATE SETUP</span><b>{{ setupProvider.toUpperCase() }}</b></div>
      <div class="provider-setup__body">
        <p class="provider-setup__intro">Enter credentials only in the private VPS terminal. This public page never asks for or stores them.</p>

        <template v-if="setupProvider === 'soundcloud'">
          <h2>SoundCloud search requires Artist Pro</h2>
          <p class="provider-setup__note">No Artist Pro? Open an indexed mix with missing artwork and choose <strong>Add cover from public link</strong>. Paste its SoundCloud track URL, preview the cover, and confirm it belongs to that mix. This works without API credentials. Automatic SoundCloud search remains unavailable.</p>
          <ol>
            <li>Sign in to SoundCloud and <a href="https://developers.soundcloud.com/docs/api/register-app" target="_blank" rel="noopener noreferrer">follow its app registration guide</a>. SoundCloud currently requires Artist Pro to create API credentials.</li>
            <li>Create or open your app and copy its Client ID and Client Secret. Public search uses an app token; there is no separate Mixsets account login.</li>
            <li>On VPS-L, run the private setup command below. It checks the credentials with SoundCloud, saves them outside Git, restarts the worker, and checks provider health.</li>
          </ol>
          <pre><code>cd /opt/syco23-mixsets
python3 deploy/vps/setup_providers.py soundcloud</code></pre>
          <p>Already-linked public SoundCloud tracks can also supply artwork during normal enrichment without these credentials.</p>
        </template>

        <template v-else-if="setupProvider === 'discogs'">
          <h2>Raise the Discogs artist lookup limit</h2>
          <ol>
            <li>Sign in to Discogs and open <a href="https://www.discogs.com/settings/developers" target="_blank" rel="noopener noreferrer">Developer settings</a>.</li>
            <li>Create a personal API token there. Discogs artist and crew enrichment works without it, but under a lower rate limit.</li>
            <li>On VPS-L, run the private setup command below. It tests the token, saves it outside Git, restarts the worker, and checks provider health.</li>
          </ol>
          <pre><code>cd /opt/syco23-mixsets
python3 deploy/vps/setup_providers.py discogs</code></pre>
        </template>

        <template v-else-if="setupProvider === 'youtube'">
          <h2>Enable YouTube text search</h2>
          <p>Known public video links and thumbnail previews work without a key. To search YouTube during discovery and enrichment:</p>
          <ol>
            <li>In <a href="https://console.cloud.google.com/apis/library/youtube.googleapis.com" target="_blank" rel="noopener noreferrer">Google Cloud</a>, enable the YouTube Data API v3 for your project.</li>
            <li>Create an API key in Credentials and restrict it to YouTube Data API v3. For an IP restriction, allow the VPS outbound IP.</li>
            <li>On VPS-L, run the private setup command below. It validates the key, saves it outside Git, restarts the worker, and checks provider health.</li>
          </ol>
          <pre><code>cd /opt/syco23-mixsets
python3 deploy/vps/setup_providers.py youtube</code></pre>
        </template>

        <button class="btn btn--primary" type="button" :disabled="checking" @click="refresh">
          <q-icon :name="checking ? 'mdi-loading mdi-spin' : 'mdi-refresh'" />
          {{ checking ? 'Checking…' : 'Check connection again' }}
        </button>
      </div>
    </section>

    <section class="panel limits-panel">
      <div class="panel-head"><span>INDEXING RULE</span><b>NO INVISIBLE SCRAPING</b></div>
      <p>SoundCloud search needs official API credentials; a known public track link can supply its cover. YouTube text search needs a Google API key, while known video links work without one. hearthis.at uses its public API and may rate-limit. Discogs enriches artist profiles. Archive.org uses its public APIs. Freeteknomusic uses bounded HTTP directory listings.</p>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import SourceBadge from '../components/SourceBadge.vue';
import { useMixStore } from '../composables/useMixStore';
import { getProviderHealth } from '../services/api';
import type { ProviderId } from '../domain/types';

const { state, updateProviderHealth, setApiState } = useMixStore();
const checking = ref(false);
const setupProvider = ref<ProviderId | null>(null);
const readyCount = computed(() => state.providers.filter((provider) => provider.state === 'ready').length);
const limitedCount = computed(() => state.providers.length - readyCount.value);

function formatCheck(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? '—'
    : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

async function refresh() {
  checking.value = true;
  try {
    const health = await getProviderHealth();
    updateProviderHealth(health);
    setApiState('online');
  } catch {
    setApiState('offline');
  } finally {
    checking.value = false;
  }
}

onMounted(() => void refresh());
</script>
