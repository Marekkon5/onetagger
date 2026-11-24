<template>
    <q-select
        v-model="locale"
        :loading="loading"
        data-testid="language-selector"
        :options="localeOptions"
        :label="t('core.language')"
        :aria-label="t('core.language')"
        emit-value map-options dense outlined
        class="language-selector"
        :class="{ 'language-selector-compact': compact }"
    >
        <q-tooltip>{{ t('core.languageHelp') }}</q-tooltip>
    </q-select>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { Notify } from 'quasar';
import { useI18n } from 'vue-i18n';
import { i18n, localeOptions, setLocale } from '../i18n';

defineProps<{ compact?: boolean }>();
const { t } = useI18n({ useScope: 'global' });
const loading = ref(false);
let request = 0;
const locale = computed({ get: () => i18n.global.locale.value, set: async value => {
    const id = ++request;
    loading.value = true;
    try { await setLocale(value); }
    catch { if (id === request) Notify.create({ type: 'negative', message: t('core.unexpectedError') }); }
    finally { if (id === request) loading.value = false; }
} });
</script>

<style scoped>
.language-selector { min-width: 150px; max-width: 320px; }
.language-selector-compact { width: 150px; min-width: 125px; margin-inline: 8px; }
</style>
