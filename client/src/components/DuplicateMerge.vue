<script setup lang="ts">
import { computed, ref } from 'vue';
import { missingFields, type CatalogRecord, type RecordId } from '@syco23/catalog-domain';
import { useCatalogStore } from '../catalog/store';

const props = defineProps<{ survivor: CatalogRecord; duplicate: CatalogRecord }>();
const emit = defineEmits<{ merged: [CatalogRecord] }>();

const catalog = useCatalogStore;
const confirm = ref(false);
const busy = ref(false);
const error = ref('');

/** Which side holds which value, so the curator compares facts rather than fields. */
const comparison = computed(() => {
  const single: Array<[string, unknown, unknown]> = [];
  const fields = props.survivor.kind === 'mix'
    ? ['title', 'description', 'durationMs', 'recordingDate'] as const
    : props.survivor.kind === 'entity'
      ? ['displayName', 'profile', 'country'] as const
      : ['name', 'venue', 'locality', 'country'] as const;
  for (const field of fields) {
    const left = (props.survivor as unknown as Record<string, unknown>)[field];
    const right = (props.duplicate as unknown as Record<string, unknown>)[field];
    if (left === right) continue;
    single.push([field, left ?? undefined, right ?? undefined]);
  }
  return single;
});

const survivorOnlyFields = computed(() => missingFields(props.survivor));
const duplicateOnlyFields = computed(() => missingFields(props.duplicate));
const gains = computed(() => survivorOnlyFields.value.filter((field) => duplicateOnlyFields.value.includes(field)));

function display(value: unknown) {
  if (value === undefined || value === null || value === '') return '—';
  return typeof value === 'string' ? value : JSON.stringify(value);
}

async function runMerge() {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    const merged = await catalog.mergeRecords(
      props.survivor.id as RecordId,
      props.duplicate.id as RecordId,
      [props.survivor.revision, props.duplicate.revision]
    );
    emit('merged', merged);
  } catch (caught) {
    // A refused merge leaves both records in place; the message says why.
    error.value = caught instanceof Error ? caught.message : 'Records were not merged';
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="duplicate-merge">
    <div class="duplicate-merge__heads">
      <div>
        <span class="kicker">SURVIVOR / REV {{ survivor.revision }}</span>
        <strong>{{ survivor.kind === 'mix' ? survivor.title : survivor.kind === 'entity' ? survivor.displayName : survivor.name }}</strong>
      </div>
      <div>
        <span class="kicker">DUPLICATE / REV {{ duplicate.revision }}</span>
        <strong>{{ duplicate.kind === 'mix' ? duplicate.title : duplicate.kind === 'entity' ? duplicate.displayName : duplicate.name }}</strong>
      </div>
    </div>

    <p class="duplicate-merge__note">
      The survivor keeps its selected values and receives the duplicate's sources, relationships and missing fields.
      Fields both records state differently stay in Review instead of being resolved by merge order.
      The duplicate's old link keeps resolving to the survivor.
    </p>

    <table v-if="comparison.length" class="diff-table">
      <caption class="sr-only">Fields where the two records disagree</caption>
      <thead><tr><th scope="col">FIELD</th><th scope="col">SURVIVOR KEEPS</th><th scope="col">DUPLICATE HOLDS</th></tr></thead>
      <tbody>
        <tr v-for="[field, own, theirs] in comparison" :key="field">
          <th scope="row">{{ field }}</th>
          <td>{{ display(own) }}</td>
          <td>{{ display(theirs) }}</td>
        </tr>
      </tbody>
    </table>
    <p v-else class="panel-empty"><strong>No single-valued field differs between these records.</strong></p>

    <p class="duplicate-merge__gains">
      <template v-if="gains.length">The survivor gains: {{ gains.join(', ') }}.</template>
      <template v-else>Both records are missing the same fields; the merge adds no new field coverage.</template>
    </p>

    <p v-if="error" class="catalog-error" role="alert">{{ error }}</p>

    <div class="review-actions">
      <button v-if="!confirm" class="btn btn--primary" type="button" :disabled="busy" @click="confirm = true">
        <q-icon name="mdi-call-merge" /> Merge duplicates…
      </button>
      <template v-else>
        <button class="btn" type="button" :disabled="busy" @click="confirm = false">Cancel</button>
        <button class="btn btn--primary" type="button" :disabled="busy" @click="runMerge">
          <q-icon :name="busy ? 'mdi-loading mdi-spin' : 'mdi-call-merge'" />
          {{ busy ? 'Merging…' : `Confirm merge into ${survivor.kind === 'mix' ? survivor.title : survivor.kind === 'entity' ? survivor.displayName : survivor.name}` }}
        </button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.duplicate-merge__heads {
  display: grid;
  gap: 0.5rem 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
}
.duplicate-merge__heads > div { display: grid; gap: 0.2rem; }
.duplicate-merge__note { color: var(--muted); font-size: 0.9rem; }
.duplicate-merge__gains { color: var(--muted); font-size: 0.9rem; }
.diff-table { border-collapse: collapse; width: 100%; font-size: 0.88rem; }
.diff-table th, .diff-table td { border-bottom: 1px solid var(--line); padding: 0.45rem 0.6rem; text-align: left; vertical-align: top; }
.diff-table thead th { color: var(--muted); font-size: 0.72rem; letter-spacing: 0.08em; text-transform: uppercase; }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
</style>