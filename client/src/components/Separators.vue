<template>
<div class='q-mb-md'>
    <div class='row'>
        <q-input
            v-model='id3'
            filled
            label='ID3 (MP3 + AIFF)'
            class='col-4 q-pr-sm'
            @update:model-value='save'
        ></q-input>
        <q-input
            v-model='vorbis'
            filled
            label='Vorbis (FLAC + OGG + OPUS, default is empty)'
            class='col-4 q-px-sm'
            @update:model-value='save'
        ></q-input>
        <q-input
            v-model='mp4'
            filled
            label='MP4/M4A'
            class='col-4 q-pl-sm'
            @update:model-value='save'
        ></q-input>
    </div>
</div>
</template>

<script lang='ts' setup>
import { PropType, toRefs } from 'vue';
import { Separators } from '../scripts/utils.js';

const { modelValue } = defineProps({
    modelValue: { type: Object as PropType<Separators>, required: true }
});
const { id3, vorbis, mp4 } = toRefs(modelValue);
const emit = defineEmits(['update:modelValue']);

function save() {
    emit('update:modelValue', new Separators(id3.value, vorbis?.value??undefined, mp4.value));
}

</script>