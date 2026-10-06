<script setup lang="ts">
import { computed } from 'vue';
import type { ClaimDisposition, FieldEvidence } from '@syco23/catalog-domain';

const props = defineProps<{ evidence: readonly FieldEvidence[]; selectedEvidence?: Record<string, string> }>();

/** Which evidence entry the selected value of each field came from. */
const selectedIds = computed(() => new Set(Object.values(props.selectedEvidence ?? {})));

function isSelected(item: FieldEvidence) {
  return item.disposition === 'selected' || selectedIds.value.has(item.id);
}

const grouped = computed(() => {
  const byField = new Map<string, FieldEvidence[]>();
  for (const item of props.evidence) {
    const list = byField.get(item.claim.field) ?? [];
    list.push(item);
    byField.set(item.claim.field, list);
  }
  // Fields with a selected value first, then alphabetical, so a reader sees why the
  // current values hold before the claims that did not win.
  return [...byField.entries()].sort(([left, leftItems], [right, rightItems]) => {
    const leftSelected = leftItems.some(isSelected);
    const rightSelected = rightItems.some(isSelected);
    if (leftSelected !== rightSelected) return leftSelected ? -1 : 1;
    return left.localeCompare(right);
  });
});

const DISPOSITION_LABEL: Record<ClaimDisposition, string> = {
  selected: 'SELECTED',
  corroborated: 'AGREES',
  pending: 'IN REVIEW',
  rejected: 'REJECTED'
};

function display(value: unknown) {
  if (value === undefined || value === null || value === '') return '—';
  return typeof value === 'string' ? value : JSON.stringify(value);
}
</script>

<template>
  <div class="field-evidence">
    <p v-if="!evidence.length" class="panel-empty">
      <strong>No provider claims recorded for this record.</strong>
      Every value here was entered by a curator rather than taken from a source.
    </p>

    <details v-for="[field, items] in grouped" :key="field" class="field-evidence__field" :open="items.some(isSelected)">
      <summary>
        <span class="kicker">{{ field }}</span>
        <strong>{{ display(items.find(isSelected)?.claim.value) }}</strong>
        <span class="field-evidence__count">{{ items.length }} claim{{ items.length === 1 ? '' : 's' }}</span>
      </summary>
      <ul class="field-evidence__list">
        <li v-for="item in items" :key="item.id" :data-disposition="item.disposition">
          <div class="field-evidence__head">
            <span class="field-evidence__badge">{{ DISPOSITION_LABEL[item.disposition] }}</span>
            <a :href="item.claim.sourceUrl" target="_blank" rel="noopener noreferrer">
              {{ item.claim.provider.provider }} / {{ item.claim.provider.resourceType }}
              <q-icon name="mdi-open-in-new" size="12px" />
            </a>
          </div>
          <dl class="field-evidence__detail">
            <dt>VALUE</dt><dd class="evidence-value">{{ display(item.claim.value) }}</dd>
            <dt>EVIDENCE</dt><dd>{{ item.claim.evidence }}</dd>
            <dt>WHY</dt><dd>{{ item.claim.matchExplanation }}</dd>
            <dt>OBSERVED</dt><dd>{{ item.claim.observedAt }}</dd>
          </dl>
        </li>
      </ul>
    </details>
  </div>
</template>

<style scoped>
.field-evidence { display: grid; gap: 10px; }
.field-evidence__field { border: 1px solid var(--line); }
.field-evidence__field summary { cursor: pointer; display: grid; gap: 2px; padding: 10px 12px; }
.field-evidence__field summary strong { font-size: 0.95rem; overflow-wrap: anywhere; }
.field-evidence__count { color: var(--muted); font-size: 0.72rem; letter-spacing: 0.06em; text-transform: uppercase; }
.field-evidence__list { list-style: none; margin: 0; padding: 0 12px 12px; display: grid; gap: 8px; }
.field-evidence__list li { border-top: 1px solid var(--line); padding-top: 8px; }
.field-evidence__head { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; justify-content: space-between; }
.field-evidence__badge { font-size: 0.68rem; letter-spacing: 0.08em; padding: 2px 6px; border: 1px solid var(--line-strong); color: var(--muted); }
.field-evidence__list li[data-disposition='selected'] .field-evidence__badge { color: var(--success); border-color: var(--success); }
.field-evidence__list li[data-disposition='pending'] .field-evidence__badge { color: var(--amber); border-color: var(--amber); }
.field-evidence__list li[data-disposition='rejected'] .field-evidence__badge { color: var(--danger); border-color: var(--danger); }
.field-evidence__detail { display: grid; grid-template-columns: minmax(74px, auto) 1fr; gap: 3px 10px; margin: 8px 0 0; font-size: 0.84rem; }
.field-evidence__detail dt { color: var(--muted); font-size: 0.68rem; letter-spacing: 0.08em; padding-top: 2px; }
.field-evidence__detail dd { margin: 0; overflow-wrap: anywhere; }
.evidence-value { font-weight: 600; }
@media (max-width: 600px) {
  .field-evidence__detail { grid-template-columns: 1fr; }
  .field-evidence__detail dt { padding-top: 6px; }
}
</style>