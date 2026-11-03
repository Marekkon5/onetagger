<script setup lang="ts">
import { onMounted, shallowRef } from 'vue';
import type { MigrationResult } from '@syco23/catalog-domain';
import { useRoute, useRouter } from 'vue-router';
import { useCatalogStore } from '../catalog/store';
import { MIGRATION_STATUS_LABELS } from '../catalog/presentation';

const route = useRoute();
const router = useRouter();
const catalog = useCatalogStore;
const authOff = import.meta.env.VITE_AUTH_MODE === 'off';
const password = shallowRef('');
const error = shallowRef('');
const busy = shallowRef(false);
const migrationAvailable = shallowRef(false);
const migrationResult = shallowRef('');
const migrationDetails = shallowRef<MigrationResult>();

onMounted(() => {
  migrationAvailable.value = typeof window !== 'undefined' && Boolean(window.localStorage.getItem('syco23.mixsets.library'));
  void catalog.checkSession().catch((caught) => { error.value = caught instanceof Error ? caught.message : 'Session could not be checked'; });
});

async function login() {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    await catalog.login(password.value);
    await catalog.loadReview().catch(() => undefined);
    password.value = '';
    if (!migrationAvailable.value) await continueToArchive();
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Login failed';
  } finally {
    busy.value = false;
  }
}

async function migrate() {
  busy.value = true;
  error.value = '';
  try {
    const result = await catalog.migrateLocalLibrary();
    migrationDetails.value = result;
    migrationResult.value = result
      ? `${result.imported} imported · ${result.existing} already present · ${result.rejected} excluded or invalid. Your browser copy remains unchanged.`
      : 'No local library was found to migrate.';
    migrationAvailable.value = Boolean(result?.rejected || result?.outcomes?.some((outcome) => ['partial', 'rejected'].includes(outcome.status)));
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Migration did not complete. Your browser copy is still available; retry with the same batch.';
  } finally {
    busy.value = false;
  }
}

async function logout() {
  error.value = '';
  try { await catalog.logout(); } catch (caught) { error.value = caught instanceof Error ? caught.message : 'Logout did not complete'; }
}
async function continueToArchive() {
  const destination = typeof route.query.redirect === 'string' ? route.query.redirect : '/';
  await router.replace(destination.startsWith('/') ? destination : '/');
}
</script>

<template>
  <section class="page curator-page">
    <header class="page-hero page-hero--compact">
      <div>
        <p class="kicker">{{ authOff ? 'ARCHIVE ACCESS / OPEN MODE' : 'CURATOR ACCESS / PRIVATE SESSION' }}</p>
        <h1>Archive control.</h1>
        <p class="hero-copy">{{ authOff ? 'The archive is open for importing, enriching and editing without signing in.' : 'Public records remain readable. A curator session is needed to import, enrich, edit and resolve evidence.' }}</p>
      </div>
    </header>

    <section v-if="catalog.state.authenticated" class="panel curator-panel">
      <div class="panel-head"><span>{{ authOff ? 'OPEN ACCESS' : 'SESSION ACTIVE' }}</span><b>{{ authOff ? 'NO LOGIN REQUIRED' : 'CURATOR' }}</b></div>
      <p class="curator-panel__copy">{{ authOff ? 'No sign-in is required. Provider credentials remain on the archive worker.' : 'You are signed in. Provider credentials remain on the archive worker.' }}</p>
      <p v-if="error" class="catalog-error" role="alert">{{ error }}</p>
      <p v-if="migrationResult" data-testid="migration-results" class="migration-result" role="status">{{ migrationResult }}</p>
      <ul v-if="migrationDetails?.outcomes?.length" class="migration-outcomes">
        <li v-for="outcome in migrationDetails.outcomes" :key="outcome.legacyId">
          <strong>{{ outcome.legacyId }} · {{ MIGRATION_STATUS_LABELS[outcome.status] ?? outcome.status }}</strong>
          <router-link v-if="outcome.recordId" :to="`/catalog/records/${encodeURIComponent(outcome.recordId)}`">Open shared record</router-link>
          <p v-for="message in outcome.errors" :key="message">{{ message }}</p>
        </li>
      </ul>
      <div class="curator-actions">
        <button v-if="!authOff" class="btn" :disabled="busy" @click="logout">Sign out</button>
        <button v-if="migrationAvailable" class="btn btn--primary" :disabled="busy" @click="migrate">
          <q-icon :name="busy ? 'mdi-loading mdi-spin' : 'mdi-database-import-outline'" />
          Migrate this browser’s library
        </button>
        <button v-if="migrationAvailable" class="btn" :disabled="busy" @click="migrationAvailable = false; continueToArchive()">Keep local copy for later</button>
        <button v-else class="btn btn--primary" @click="continueToArchive">Open archive</button>
      </div>
    </section>

    <section v-else class="panel curator-panel">
      <div class="panel-head"><span>SECURE COOKIE SESSION</span><b>LOGIN</b></div>
      <form class="curator-form" @submit.prevent="login">
        <label for="curator-password">Curator password</label>
        <input id="curator-password" v-model="password" type="password" autocomplete="current-password" required minlength="12" maxlength="1024" />
        <p v-if="error" class="catalog-error" role="alert">{{ error }}</p>
        <button class="btn btn--primary" type="submit" :disabled="busy || password.length < 12">
          <q-icon :name="busy ? 'mdi-loading mdi-spin' : 'mdi-lock-open-outline'" />
          {{ busy ? 'Signing in…' : 'Sign in' }}
        </button>
      </form>
    </section>
  </section>
</template>
