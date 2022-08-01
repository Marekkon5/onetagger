<template>
<div>

    <div class='row'>
        <q-input
            v-model='id3'
            filled
            label='ID3 Separator (MP3 + AIFF)'
            class='col-4 q-pr-sm'
            @input='save'
        ></q-input>
        <q-input
            v-model='vorbis'
            filled
            label='FLAC Separator (Leave empty for default)'
            class='col-4 q-px-sm'
            @input='save'
        ></q-input>
        <q-input
            v-model='mp4'
            filled
            label='M4A Separator'
            class='col-4 q-pl-sm'
            @input='save'
        ></q-input>
    </div>

</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { Separators } from '../scripts/utils.js';

const { initial } = defineProps({
    initial: Object
});
const id3 = ref(initial?.id3??', ');
const vorbis = ref(initial?.vorbis);
const mp4 = ref(initial?.mp4);
const emit = defineEmits(['input']);

function save() {
    emit('input', new Separators(id3.value, vorbis.value, mp4.value));
}

</script>