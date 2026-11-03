<script setup lang="ts">
import { computed } from 'vue';
import type { DeepReadonly } from 'vue';
import type { CatalogRecord } from '@syco23/catalog-domain';
import ArtworkFrame from '../components/ArtworkFrame.vue';
import { KIND_LABELS, VERIFICATION_LABELS } from '../catalog/presentation';

const props = defineProps<{ record: DeepReadonly<CatalogRecord> }>();

const title = computed(() => props.record.kind === 'mix' ? props.record.title
  : props.record.kind === 'entity' ? props.record.displayName : props.record.name);

const kind = computed(() => props.record.kind === 'entity' ? props.record.roles.join(' / ') : KIND_LABELS[props.record.kind] ?? props.record.kind);

const verification = computed(() => VERIFICATION_LABELS[props.record.verification] ?? props.record.verification);

const subtitle = computed(() => {
  if (props.record.kind === 'mix') return `${props.record.people.length} linked people · ${props.record.sources.length} sources`;
  if (props.record.kind === 'entity') return `${props.record.roles.join(' / ')}${props.record.country ? ` · ${props.record.country}` : ''}`;
  return [props.record.venue, props.record.locality, props.record.country].filter(Boolean).join(' · ') || 'Location not recorded';
});

const image = computed(() => {
  if (props.record.kind === 'mix') return props.record.assets.find((asset) => asset.role === 'mix-cover')?.url;
  if (props.record.kind === 'entity') return props.record.assets.find((asset) => ['artist-portrait', 'crew-logo', 'label-logo'].includes(asset.role))?.url;
  return props.record.assets.find((asset) => asset.role === 'event-flyer')?.url;
});

const href = computed(() => props.record.kind === 'mix' ? `/mix/${encodeURIComponent(props.record.id)}`
  : props.record.kind === 'entity' ? `/entity/${encodeURIComponent(props.record.id)}`
    : `/event/${encodeURIComponent(props.record.id)}`);
</script>

<template>
  <router-link class="catalog-row" :to="href">
    <div class="catalog-row__image"><ArtworkFrame :src="image" :alt="title" /></div>
    <div class="catalog-row__main">
      <span class="catalog-row__kind">{{ kind }}</span>
      <strong>{{ title }}</strong>
      <small>{{ subtitle }}</small>
    </div>
    <div class="catalog-row__evidence">
      <span>{{ verification }}</span>
      <span v-if="record.reviewState === 'review'" data-state="review">REVIEW</span>
      <span v-else>REVISION {{ record.revision }}</span>
    </div>
    <q-icon name="mdi-arrow-top-right" size="18px" aria-hidden="true" />
  </router-link>
</template>