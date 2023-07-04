<template>
<div>

    <q-select
        :model-value='value'
        use-input
        fill-input
        hide-selected
        filled
        input-debounce="0"
        :color='color'
        :options='options'
        :dense='dense'
        :label='label'
        :error-message='error'
        :error='error?true:false'
        @update:model-value='onInput'
        @input-value='onInputValue'
        @filter='onFilter'
        popup-content-class='no-shadow'
    ></q-select>


</div>
</template>

<script lang='ts' setup>
import { computed, ref } from 'vue';
import {MP4, VORBIS, ID3} from '../scripts/tags';

type BannedTags = { id3: string[], vorbis: string[], mp4: string[] };

// Tags you shouldn't use
const banned: BannedTags = {
    id3: ["APIC", "TXXX", "POPM", "TYER", "TDAT", "TORY", "TRDA", "TRCK", "TDTG", "TSOT", "TIT2",
        "TDOR", "TKEY", "TSOC", "TCMP", "TBPM", "TSOP", "TSO2", "TSOA", "SYLT", "TSRC"],
    vorbis: ["METADATA_BLOCK_PICTURE", "DATE", "ORIGINALDATE", "TRACKNUMBER", "TAGGINGTIME", 
        "TRACK", "TITLESORT", "ORIGYEAR", "INITIALKEY", "KEY", "COMPOSERSORT", "COMPILATION", 
        "BPM", "ARTISTSORT", "ALBUMARTISTSORT", "ALBUMSORT", "POPULARIMETER", "RATING", "ISRC"],
    mp4: []
}

const bannedTagEditor: BannedTags = {
    id3: ["APIC", "TXXX", "POPM", "COMM", "USLT"],
    vorbis: [],
    mp4: []
}

const { format, initial, dense, tageditor } = defineProps({
    format: { type: String, required: true },
    initial: { type: String, required: false },
    dense: { default: false, type: Boolean },
    tageditor: { default: false, type: Boolean }
});

// Autocompletion
let options: string[] = [];
switch (format) {
    case 'id3':
        options = ID3;
        break;
    case 'vorbis':
        options = VORBIS;
        break;
    case 'mp4':
        options = MP4;
        break;
}
let originalOptions = JSON.parse(JSON.stringify(options));
const value = ref(initial??'');
const error = ref<string | undefined>();
const emit = defineEmits(['change']);


// Remove ID3 helper text
function removeHelper() {
    if ((format == 'id3' || format == 'mp4') && value.value.match(/[©a-zA-Z0-9]{4} \(.*\)/)) {
        value.value = value.value.replace(/ \(.*\)/, '');
    }
}

// Clear helpers
function onInput(v: any) {
    // console.log(v)
    value.value = v;
    removeHelper();
}

// Update text typed without autocomplete taking over
function onInputValue(v: string) {
    v = v.trim();
    value.value = v;
    removeHelper();

    // Validate
    // warning.value = null;
    error.value = undefined;

    if (format != 'mp4' && v.toUpperCase() != v && !(format == 'id3' && v.length != 4))
        error.value = 'Tag names should be uppercase!';

    if (v.includes(' ') && !(format == 'id3' && v.length != 4))
        error.value = "Tag names shouldn't contain spaces!"

    // @ts-ignore
    if ((tageditor ? bannedTagEditor : banned)[format].includes(v.toUpperCase()))
        error.value = "tag.value is reserved, might cause corruption!"

    if (!v)
        error.value = "Shouldn't be empty!"

    emit('change', v);
}

// On filter for quasar
function onFilter(val: any, update: any) {
    update(() => {
        let n = val.toUpperCase();
        options = originalOptions.filter((v: string) => v.toUpperCase().includes(n));
    });
}

const label = computed(() => {
    switch (format) {
        case 'id3': return 'ID3 (MP3/AIFF/WAV)';
        case 'vorbis': return 'Vorbis (FLAC/OGG/OPUS)';
        case 'mp4': return 'MP4/M4A';
    }
});

// Different color if writing to comment/custom tag
const color = computed(() => {
    if ((format == 'id3' || format == 'mp4') && (value.value??'').length != 4) return 'yellow';
    return 'primary';
});

</script>