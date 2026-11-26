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
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
import { Ref, computed, ref, toRef, watch } from 'vue';
import { tagOptions } from '../scripts/tags';

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

const props = defineProps({
    format: { type: String, required: true },
    modelValue: { type: String, required: false, default: '' },
    dense: { default: false, type: Boolean },
    tageditor: { default: false, type: Boolean }
});
const { format, dense, tageditor } = props;

// Labels are recomputed on locale changes; option values still reduce to frame IDs.
const filterText = ref('');
const options = computed(() => tagOptions(format).filter(option =>
    option.toLocaleUpperCase().includes(filterText.value.toLocaleUpperCase())));

const value: Ref<string> = ref(props.modelValue);
const errorKey = ref<string | undefined>();
const error = computed(() => errorKey.value ? t('tools.' + errorKey.value) : undefined);
const emit = defineEmits(['update:modelValue']);


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

    // The displayed helper is localized; only the unchanged frame identifier
    // may reach a tag configuration or a save request.
    v = value.value;

    // Validate
    // warning.value = null;
    errorKey.value = undefined;

    if (format != 'mp4' && v.toUpperCase() != v && !(format == 'id3' && v.length != 4))
        errorKey.value = 'uppercaseTag';

    if (v.includes(' ') && !(format == 'id3' && v.length != 4))
        errorKey.value = 'noSpacesTag'

    // @ts-ignore
    if ((tageditor ? bannedTagEditor : banned)[format].includes(v.toUpperCase()))
        errorKey.value = 'reservedTag'

    if (!v)
        errorKey.value = 'emptyTag'

    emit('update:modelValue', v);
}

// On filter for quasar
function onFilter(val: any, update: any) {
    update(() => {
        filterText.value = String(val);
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

watch(() => props.modelValue, () => {
    onInput(props.modelValue);
});

</script>
