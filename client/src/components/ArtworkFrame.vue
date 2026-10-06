<template>
  <div class="artwork-frame" :class="{ 'artwork-frame--empty': !usableSource }">
    <img
      v-if="usableSource"
      :src="usableSource"
      :alt="alt"
      loading="lazy"
      decoding="async"
      @error="broken = true"
    />
    <div v-else class="artwork-fallback" aria-hidden="true">
      <span>S//23</span>
      <strong>{{ initials }}</strong>
      <small>NO IMAGE / SIGNAL INDEX</small>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';

const props = withDefaults(defineProps<{ src?: string; alt?: string }>(), {
  src: '',
  alt: 'Mix artwork',
});

const broken = ref(false);

watch(() => props.src, () => {
  broken.value = false;
});

const usableSource = computed(() => {
  if (broken.value || !props.src) return '';
  return props.src.startsWith('http://') ? props.src.replace(/^http:/, 'https:') : props.src;
});

const initials = computed(() => {
  const words = props.alt
    .replace(/[^\p{L}\p{N}\s]/gu, ' ')
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 3);
  return words.map((word) => word[0]).join('').toUpperCase() || '23';
});
</script>
