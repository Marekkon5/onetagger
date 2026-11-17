<template>
    <div class='q-mx-auto q-my-md text-left' style='max-width: 550px'>
        <q-checkbox v-model='$1t.saveTaggingReport.value' :disable='!!$1t.taggingSelection.value'
            label='Save tagging results JSON' />
        <div class='text-caption text-grey-6'>Records actual tag changes and matching evidence for library review.</div>
        <q-file v-model='file' filled clearable accept='.json,application/json' label='Helper selection manifest (optional)'
            class='q-mt-sm' @update:model-value='loadSelection' />
        <div v-if='$1t.taggingSelection.value' class='text-caption text-primary q-mt-sm'>
            {{$1t.taggingSelection.value.tracks.length}} selected library tracks. This selection supplies the input files. Disable file moves before starting.
            <q-btn flat dense label='Clear Helper selection' @click='clearSelection' />
        </div>
        <div v-if='$1t.selectionError.value' role='alert' class='text-negative q-mt-sm'>{{$1t.selectionError.value}}</div>
    </div>
</template>

<script setup lang='ts'>
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
        if (value.size > 2 * 1024 * 1024) throw new Error('Selection manifest exceeds 2 MiB');
        const selection = JSON.parse(await value.text());
        if (selection.schema !== 1 || !Array.isArray(selection.tracks) || !selection.tracks.length || selection.tracks.length > 500)
            throw new Error('Choose a Helper selection manifest with 1–500 tracks');
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
