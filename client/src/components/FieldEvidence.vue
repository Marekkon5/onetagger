<script setup lang="ts">
import type { CatalogRecord, FieldClaim } from '@syco23/catalog-domain';
import { formatValue } from '../catalog/views';
import { fieldLabel, providerLabel, EVIDENCE_LABELS, DISPOSITION_LABELS, REVIEW_STATE_LABELS, timeAgo } from '../catalog/presentation';
export interface EvidenceEntry { id: string; fingerprint: string; claim: FieldClaim; disposition: string }
defineProps<{ record: CatalogRecord; evidence: EvidenceEntry[] }>();
</script>
<template>
  <section class="panel field-evidence" data-testid="field-evidence">
    <div class="panel-head"><span>SOURCES / {{ REVIEW_STATE_LABELS[record.reviewState] ?? record.reviewState }}</span><b>FIELD EVIDENCE</b></div>
    <p v-if="!evidence.length" class="panel-empty">No attributable field evidence is recorded yet.</p>
    <article v-for="entry in evidence" :key="entry.id" class="field-evidence__item">
      <div><strong>{{ fieldLabel(entry.claim.field) }}</strong><b>{{ record.selectedEvidence?.[entry.claim.field] === entry.id ? 'Selected' : DISPOSITION_LABELS[entry.disposition] ?? entry.disposition }}</b></div>
      <p class="preline">{{ formatValue(entry.claim.value) }}</p>
      <p>{{ entry.claim.matchExplanation }}</p>
      <a :href="entry.claim.sourceUrl" target="_blank" rel="noopener noreferrer">{{ providerLabel(entry.claim.provider.provider) }} / {{ EVIDENCE_LABELS[entry.claim.evidence] ?? entry.claim.evidence }} · Open source</a>
      <small>Observed {{ timeAgo(entry.claim.observedAt) }}</small>
    </article>
  </section>
</template>
