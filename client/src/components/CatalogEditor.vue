<script setup lang="ts">
import { computed, reactive, watch } from 'vue';
import type { CatalogRecord, RecordId } from '@syco23/catalog-domain';
import { useCatalogStore } from '../catalog/store';

const props = defineProps<{ record: CatalogRecord }>();
const emit = defineEmits<{ saved: [CatalogRecord] }>();

const catalog = useCatalogStore;
const busy = reactive<Record<string, boolean>>({});
const errors = reactive<Record<string, string>>({});

interface EditorField {
  key: string;
  label: string;
  type: 'text' | 'textarea' | 'number';
  value: string;
}

function currentValue(field: string): unknown {
  const record = props.record as unknown as Record<string, unknown>;
  const value = record[field];
  if (field === 'cover' || field === 'artistPortrait' || field === 'crewLogo' || field === 'labelLogo' || field === 'flyer') {
    const asset = props.record.assets.find((item) => roleForField(field) === item.role);
    return asset?.url ?? '';
  }
  if (field === 'recordingDate' || field === 'startDate') {
    const date = (record[field] as { value?: string } | undefined)?.value;
    return date ?? '';
  }
  return value === undefined || value === null ? '' : String(value);
}

function roleForField(field: string) {
  return { cover: 'mix-cover', artistPortrait: 'artist-portrait', crewLogo: 'crew-logo', labelLogo: 'label-logo', flyer: 'event-flyer' }[field] ?? '';
}

/**
 * Editable fields are type-specific: the server rejects a patch that touches a field
 * belonging to another kind, and a mix recording date must never be offered next to
 * an event start date. Offering only the fields this record actually has keeps the
 * editor honest instead of hiding the rejection behind a generic 400.
 */
const fields = computed<EditorField[]>(() => {
  const record = props.record;
  const base: EditorField[] = [];
  if (record.kind === 'mix') {
    base.push(
      { key: 'title', label: 'Title', type: 'text', value: String(currentValue('title') ?? '') },
      { key: 'description', label: 'Description', type: 'textarea', value: String(currentValue('description') ?? '') },
      { key: 'durationMs', label: 'Duration (ms)', type: 'number', value: String(currentValue('durationMs') ?? '') }
    );
    const date = currentValue('recordingDate');
    if (date) base.push({ key: 'recordingDate', label: 'Recording date', type: 'text', value: String(date) });
  } else if (record.kind === 'entity') {
    base.push(
      { key: 'displayName', label: 'Display name', type: 'text', value: String(currentValue('displayName') ?? '') },
      { key: 'profile', label: 'Profile', type: 'textarea', value: String(currentValue('profile') ?? '') }
    );
    if (record.roles.includes('artist')) {
      base.push({ key: 'artistPortrait', label: 'Artist portrait URL', type: 'text', value: String(currentValue('artistPortrait') ?? '') });
    }
  } else {
    base.push(
      { key: 'name', label: 'Event name', type: 'text', value: String(currentValue('name') ?? '') },
      { key: 'venue', label: 'Venue', type: 'text', value: String(currentValue('venue') ?? '') },
      { key: 'locality', label: 'Locality', type: 'text', value: String(currentValue('locality') ?? '') }
    );
  }
  return base;
});

const dirty = reactive<Record<string, string>>({});

watch(fields, (next) => {
  for (const field of next) {
    if (dirty[field.key] === undefined) dirty[field.key] = field.value;
  }
}, { immediate: true });

watch(() => props.record.id, () => {
  for (const key of Object.keys(dirty)) delete dirty[key];
  for (const field of fields.value) dirty[field.key] = field.value;
});

function isDirty(field: EditorField) {
  return (dirty[field.key] ?? '') !== field.value;
}

/**
 * Serializes one field back into the shape the shared schemas validate.
 *
 * Returns `null` when nothing should be sent: an unchanged draft, or a value the
 * shared schema would reject. Sending an invalid shape would only earn a generic 400,
 * so the field reports the problem instead of provoking one.
 */
function patchFor(field: EditorField): Record<string, unknown> | null {
  const raw = (dirty[field.key] ?? '').trim();
  if (raw === field.value.trim()) return null;
  if (field.type === 'number') {
    if (!raw) return { [field.key]: undefined };
    const parsed = Number(raw);
    return Number.isFinite(parsed) && parsed > 0 ? { [field.key]: parsed } : null;
  }
  if (field.key === 'recordingDate') {
    // Date precision is part of the value: a year-only source must not become a
    // full day, so the precision prefix is preserved.
    return /^\d{4}(-\d{2})?(-\d{2})?$/.test(raw) ? { [field.key]: { value: raw } } : null;
  }
  if (!raw) return { [field.key]: undefined };
  return { [field.key]: raw };
}

async function save(field: EditorField) {
  const patch = patchFor(field);
  if (patch === null || patch === undefined || !Object.keys(patch).length) {
    errors[field.key] = field.type === 'number' && (dirty[field.key] ?? '').trim() !== '' ? 'Enter a number of milliseconds.' : '';
    return;
  }
  busy[field.key] = true;
  errors[field.key] = '';
  try {
    const saved = await catalog.updateRecord(props.record.id as RecordId, patch, props.record.revision);
    dirty[field.key] = field.value;
    emit('saved', saved);
  } catch (caught) {
    // The save did not commit; the draft stays so the curator can correct it.
    errors[field.key] = caught instanceof Error ? caught.message : 'Change was not saved';
  } finally {
    busy[field.key] = false;
  }
}
</script>

<template>
  <div class="catalog-editor__fields">
    <div v-for="field in fields" :key="field.key" class="catalog-editor__field">
      <label :for="`field-${record.id}-${field.key}`">{{ field.label }}</label>
      <textarea
        v-if="field.type === 'textarea'"
        :id="`field-${record.id}-${field.key}`"
        v-model="dirty[field.key]"
        rows="3"
        maxlength="10000"
      />
      <input
        v-else
        :id="`field-${record.id}-${field.key}`"
        v-model="dirty[field.key]"
        :type="field.type === 'number' ? 'number' : 'text'"
        :step="field.type === 'number' ? 1000 : undefined"
        min="0"
        maxlength="10000"
      />
      <p v-if="errors[field.key]" class="catalog-error" role="alert">{{ errors[field.key] }}</p>
      <button
        class="btn"
        type="button"
        :disabled="busy[field.key] || !isDirty(field)"
        @click="save(field)"
      >
        <q-icon :name="busy[field.key] ? 'mdi-loading mdi-spin' : 'mdi-content-save-outline'" />
        {{ busy[field.key] ? 'Saving…' : 'Save' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.catalog-editor__fields { display: grid; gap: 14px; }
.catalog-editor__field { display: grid; gap: 6px; }
.catalog-editor__field label { font-size: 0.72rem; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
.catalog-editor__field input, .catalog-editor__field textarea {
  background: var(--bg-raised); border: 1px solid var(--line-strong); color: var(--text);
  padding: 8px 10px; font: inherit; width: 100%; resize: vertical;
}
.catalog-editor__field .btn { justify-self: start; }
</style>