<template>
    <div class='q-mx-auto q-my-md text-left' style='max-width: 550px'>
        <q-checkbox v-model='$1t.saveTaggingReport.value' :disable='!!$1t.taggingSelection.value'
            :label="t('tools.saveResults')" />
        <div class='text-caption text-grey-6'>{{ t('tools.resultsDescription') }}</div>
        <q-file data-testid='helper-selection' v-model='file' filled clearable accept='.json,application/json' :label="t('tools.helperManifest')"
            class='q-mt-sm' @update:model-value='loadSelection' />
        <div data-testid='helper-selection-count' v-if='$1t.taggingSelection.value' class='text-caption text-primary q-mt-sm'>
            {{ t('tools.selectedTracks', { count: $1t.taggingSelection.value.tracks.length }, $1t.taggingSelection.value.tracks.length) }}
            <q-btn flat dense :label="t('tools.clearSelection')" @click='clearSelection' />
        </div>
        <div v-if='$1t.selectionError.value' role='alert' class='text-negative q-mt-sm'>{{$1t.selectionError.value}}</div>
    </div>
</template>

<script setup lang='ts'>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
import { ref } from 'vue';
import { get1t } from '../scripts/onetagger';
const $1t = get1t();
const file = ref<File | null>(null);
let generation = 0;
function clearSelection() { file.value = null; void loadSelection(null); }
async function loadSelection(value: File | null) {
    const current = ++generation;
    $1t.selectionError.value = ''; $1t.taggingSelection.value = null;
    $1t.selectionLoading.value = !!value;
    if (!value) return;
    try {
        if (value.size > 2 * 1024 * 1024) throw new Error(t('tools.manifestTooLarge'));
        const selection = JSON.parse(await value.text());
        if (selection.schema !== 1 || !Array.isArray(selection.tracks) || !selection.tracks.length || selection.tracks.length > 500)
            throw new Error(t('tools.invalidManifest'));
        if (current !== generation) return;
        $1t.taggingSelection.value = selection;
        $1t.saveTaggingReport.value = true;
    } catch (e) {
        if (current === generation) $1t.selectionError.value = e instanceof Error ? e.message : String(e);
    } finally {
        if (current === generation) $1t.selectionLoading.value = false;
    }
}
</script>
