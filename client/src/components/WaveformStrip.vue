<template>
  <div class="waveform" :class="{ 'waveform--empty': !imageDataUrl }" :aria-label="imageDataUrl ? 'Analyzed audio waveform' : 'Waveform analysis pending'">
    <img v-if="imageDataUrl" class="waveform__image" :src="imageDataUrl" alt="Waveform generated from the full audio recording" />
    <div v-else class="waveform__empty">
      <q-icon name="mdi-waveform" size="28px" />
      <strong>Waveform not analyzed yet</strong>
      <small>Audio analysis stays separate from metadata discovery.</small>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ imageDataUrl?: string }>();
</script>
