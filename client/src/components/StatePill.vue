<script setup lang="ts">
import { computed } from 'vue';
import { statePillLabel, stateTone } from '../catalog/presentation';

const props = defineProps<{ kind: string; value: string }>();

const label = computed(() => statePillLabel(props.kind, props.value));
const tone = computed(() => stateTone(props.kind, props.value));
</script>

<template>
  <span class="state-pill" :data-state="tone" :data-kind="kind">{{ label }}</span>
</template>
