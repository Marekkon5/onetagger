<script setup lang="ts">
import { shallowRef, onMounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import type { CatalogRecord, RecordId } from '@syco23/catalog-domain';
import { catalogApi } from '../catalog/api';
import { useCatalogStore } from '../catalog/store';
import { duplicatePreview, recordName, metadataRows, formatValue } from '../catalog/views';
const props = defineProps<{ record: CatalogRecord }>();
const emit = defineEmits<{ merged: [record: CatalogRecord] }>();
const route = useRoute();
const duplicateId = shallowRef(typeof route.query.duplicate === 'string' ? route.query.duplicate : '');
const preview = shallowRef<ReturnType<typeof duplicatePreview>>();
const busy = shallowRef(false);
const error = shallowRef('');
async function lookup() {
  preview.value = undefined; error.value = ''; busy.value = true;
  try {
    const [survivor, duplicate] = await Promise.all([catalogApi.getRecord(props.record.id), catalogApi.getRecord(duplicateId.value.trim() as RecordId)]);
    preview.value = duplicatePreview(survivor, duplicate);
  } catch (caught) { error.value = caught instanceof Error ? caught.message : 'Duplicate preview could not be loaded'; }
  finally { busy.value = false; }
}
onMounted(() => { if (duplicateId.value) void lookup(); });
watch(() => route.query.duplicate, (value) => { if (typeof value === 'string') { duplicateId.value = value; void lookup(); } });
async function merge() {
  if (!preview.value || busy.value) return;
  busy.value = true; error.value = '';
  try {
    const { survivor, duplicate, revisions } = preview.value;
    const committed = await useCatalogStore.mergeRecords(survivor.id, duplicate.id, revisions);
    preview.value = undefined; duplicateId.value = ''; emit('merged', committed);
  } catch (caught) { error.value = caught instanceof Error ? caught.message : 'Merge was not saved. Refresh the preview before retrying.'; preview.value = undefined; }
  finally { busy.value = false; }
}
</script>
<template>
  <section class="panel duplicate-merge" data-testid="merge-preview">
    <div class="panel-head"><span>CURATOR / IDENTITIES</span><b>MERGE A CONFIRMED DUPLICATE</b></div>
    <div class="duplicate-merge__body">
      <p>Load both records and inspect their values before confirming that they represent the same identity. Sources and evidence are retained; disagreements go to Review. The old ID redirects to the survivor.</p>
      <form data-testid="duplicate-lookup" class="duplicate-lookup" @submit.prevent="lookup">
        <label for="duplicate-id">Duplicate Archive ID</label>
        <input id="duplicate-id" v-model="duplicateId" aria-label="Duplicate record ID" required :disabled="busy" />
        <button class="btn" type="submit" :disabled="busy || !duplicateId.trim()">{{ busy ? 'Loading…' : 'Preview duplicate' }}</button>
      </form>
      <small class="duplicate-hint">Find the Archive ID at the bottom of the other record's detail header, then paste it here to compare both sides before merging.</small>
      <p v-if="error" class="catalog-error" role="alert">{{ error }}</p>
      <div v-if="preview" class="duplicate-comparison">
        <article v-for="(item, index) in [preview.survivor, preview.duplicate]" :key="item.id">
          <strong>{{ index ? 'DUPLICATE' : 'SURVIVOR' }} / REV {{ item.revision }}</strong>
          <h3>{{ recordName(item) }}</h3><code>Archive ID: {{ item.id }}</code>
          <dl class="meta-table"><div v-for="row in metadataRows(item)" :key="row.field"><dt>{{ row.label }}</dt><dd>{{ formatValue(row.value) }}</dd></div></dl>
        </article>
      </div>
      <button v-if="preview" class="btn btn--primary" type="button" data-testid="confirm-merge" :disabled="busy" @click="merge">Confirm same identity and merge</button>
    </div>
  </section>
</template>
