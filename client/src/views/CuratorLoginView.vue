<script setup lang="ts">
import { onMounted, shallowRef } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useCatalogStore } from '../catalog/store';

const route = useRoute();
const router = useRouter();
const catalog = useCatalogStore;
const password = shallowRef('');
const error = shallowRef('');
const busy = shallowRef(false);
const migrationAvailable = shallowRef(false);
const migrationResult = shallowRef('');

onMounted(() => {
  migrationAvailable.value = typeof window !== 'undefined' && Boolean(window.localStorage.getItem('syco23.mixsets.library'));
  void catalog.checkSession();
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
    migrationResult.value = result
      ? `${result.imported} imported · ${result.existing} already present · ${result.rejected} excluded or invalid. Your browser copy remains unchanged.`
      : 'No local library was found to migrate.';
    migrationAvailable.value = false;
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : 'Migration did not complete. Your browser copy is still available; retry with the same batch.';
  } finally {
    busy.value = false;
  }
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
        <p class="kicker">CURATOR ACCESS / PRIVATE SESSION</p>
        <h1>Archive control.</h1>
        <p class="hero-copy">Public records remain readable. A curator session is needed to import, enrich, edit and resolve evidence.</p>
      </div>
    </header>

    <section v-if="catalog.state.authenticated" class="panel curator-panel">
      <div class="panel-head"><span>SESSION ACTIVE</span><b>CURATOR</b></div>
      <p class="curator-panel__copy">You are signed in. Provider credentials remain on the archive worker.</p>
      <p v-if="migrationResult" class="migration-result" role="status">{{ migrationResult }}</p>
      <div class="curator-actions">
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