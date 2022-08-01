<template>
    <div class='wave' :style='style'></div>
</template>

<script lang='ts' setup>
import { computed, toRefs } from 'vue';

const props = defineProps({
    height: String,
    width: String,
    value: Number,
    filled: { type: Number, required: true }
});
const { height, width, value, filled } = toRefs(props);

const style = computed(() => {
    return `
        height: calc(${height?.value} * ${value?.value}); 
        width: ${width?.value};
        background: linear-gradient(to right, var(--q-primary) ${Math.round(filled.value*100)}%, #3a3c3c ${Math.round(filled.value*100)}%, #3a3c3c);
    `;
});


</script>

<style>
.wave {
    border-radius: 2px;
    transition: height 0.25s;
}
</style>