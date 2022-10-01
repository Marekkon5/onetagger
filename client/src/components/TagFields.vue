<template>
<div class='row'>

    <TagField
        format='id3'
        :dense='dense'
        :tageditor='tageditor'
        :initial='tag.id3'
        @change='tag.id3 = $event; onChange()'
        class='col-4 q-pr-sm'
    ></TagField>

    <TagField
        format='vorbis'
        :dense='dense'
        :tageditor='tageditor'
        :initial='tag.vorbis'
        @change='tag.vorbis = $event; onChange()'
        class='col-4 q-px-sm'
    ></TagField>

    <TagField
        format='mp4'
        :dense='dense'
        :tageditor='tageditor'
        :initial='tag.mp4'
        @change='tag.mp4 = $event; onChange()'
        class='col-4 q-pl-sm'
    ></TagField>

</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { FrameName } from '../scripts/utils';
import TagField from './TagField.vue';

const { modelValue, dense, tageditor } = defineProps({
    modelValue: { type: FrameName, required: true },
    dense: { default: false, type: Boolean },
    tageditor: { default: false, type: Boolean }
});
const tag = ref(modelValue);
const emit = defineEmits(['update:modelValue']);

function onChange() {
    emit('update:modelValue', tag.value);
}
</script>