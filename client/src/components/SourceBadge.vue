<template>
  <span class="source-badge" :data-provider="provider">
    <q-icon :name="icon" size="14px" />
    {{ label }}
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { ProviderId } from '../domain/types';

const props = defineProps<{ provider: ProviderId }>();

const label = computed(() => ({
  freeteknomusic: 'FTM',
  soundcloud: 'SoundCloud',
  archiveorg: 'Archive.org',
  discogs: 'Discogs',
  youtube: 'YouTube',
  hearthis: 'hearthis.at',
  mixcloud: 'Mixcloud',
})[props.provider]);

const icon = computed(() => ({
  freeteknomusic: 'mdi-folder-music-outline',
  soundcloud: 'mdi-soundcloud',
  archiveorg: 'mdi-archive-outline',
  discogs: 'mdi-album',
  youtube: 'mdi-youtube',
  hearthis: 'mdi-headphones',
  mixcloud: 'mdi-cloud-music-outline',
})[props.provider]);
</script>
