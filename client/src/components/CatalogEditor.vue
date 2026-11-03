<script setup lang="ts">
import { computed, reactive, watch } from 'vue';
import type { CatalogRecord } from '@syco23/catalog-domain';
import { editorDraft, editorFields, parseEditorPatch } from '../catalog/views';
const props = defineProps<{ record: CatalogRecord; busy?: boolean }>();
const emit = defineEmits<{ save: [patch: Record<string, unknown>] }>();
const isDraft = computed(() => props.record.id.startsWith('draft_'));
const draft = reactive<Record<string, any>>({});
const error = reactive({ message: '' });
watch(() => props.record, (record) => { Object.keys(draft).forEach((key) => delete draft[key]); Object.assign(draft, editorDraft(record)); error.message = ''; }, { immediate: true });
const fields = computed(() => editorFields(props.record.kind === 'entity' ? { ...props.record, roles: draft.roles ?? props.record.roles } : props.record));
const assetRoles = computed(() => props.record.kind === 'mix' ? ['mix-cover'] : props.record.kind === 'event' ? ['event-flyer'] : (draft.roles ?? props.record.roles).map((role: string) => role === 'artist' ? 'artist-portrait' : `${role}-logo`));
function addAsset() { draft.assets.push({ role: assetRoles.value[0], url: '', source: 'curator' }); }
function addPerson() { draft.people.push({ entityId: '', role: 'artist' }); }
function save() {
  error.message = '';
  try {
    // New fields appear when a curator explicitly confirms an additional role.
    const current = props.record.kind === 'entity' ? { ...props.record, roles: draft.roles ?? props.record.roles } : props.record;
    emit('save', parseEditorPatch(current, draft));
  } catch (caught) { error.message = caught instanceof Error ? caught.message : 'Check the field values before saving'; }
}
</script>
<template>
  <section class="panel catalog-editor" data-testid="catalog-editor">
    <div class="panel-head"><span>{{ isDraft ? 'NEW · NOT YET SAVED' : 'CURATOR · REVISION ' + record.revision }}</span><b>EDIT SOURCED FIELDS</b></div>
    <form class="typed-editor" @submit.prevent="save">
      <p class="editor-help">Changes are saved with curator evidence. Dates preserve their precision. Confirm roles and relationship IDs using supporting sources.</p>
      <div v-for="field in fields" :key="field.key" class="editor-field">
        <label :for="`edit-${field.key}`">{{ field.label }}</label>
        <textarea v-if="field.kind === 'list' || field.kind === 'longtext'" :id="`edit-${field.key}`" v-model="draft[field.key]" :rows="field.kind === 'longtext' ? 4 : 2" :disabled="busy" />
        <fieldset v-else-if="field.kind === 'roles'" class="editor-roles">
          <legend>Roles confirmed by the curator</legend>
          <label v-for="role in ['artist', 'crew', 'label']" :key="role"><input v-model="draft.roles" type="checkbox" :value="role" :disabled="busy" />{{ role }}</label>
        </fieldset>
        <div v-else-if="field.kind === 'assets'" class="editor-rows">
          <div v-for="(asset, index) in draft.assets" :key="index" class="editor-asset-row">
            <select v-model="asset.role" :aria-label="`Asset ${index + 1} role`" :disabled="busy || asset.role === 'waveform'"><option v-for="role in [...assetRoles, 'waveform'].filter((r) => r !== 'waveform' || asset.role === 'waveform')" :key="role">{{ role }}</option></select>
            <input v-model="asset.url" type="url" :aria-label="`Asset ${index + 1} URL`" :disabled="busy || asset.role === 'waveform'" required />
            <button class="btn" type="button" :disabled="busy" :aria-label="`Remove asset ${index + 1}`" @click="draft.assets.splice(index, 1)">Remove</button>
          </div>
          <button class="btn" type="button" :disabled="busy" @click="addAsset">Add image</button>
        </div>
        <div v-else-if="field.kind === 'people'" class="editor-rows">
          <div v-for="(person, index) in draft.people" :key="index" class="editor-asset-row">
            <select v-model="person.role" :aria-label="`Relationship ${index + 1} role`" :disabled="busy"><option>artist</option><option>crew</option><option>label</option></select>
            <input v-model="person.entityId" :aria-label="`Relationship ${index + 1} entity ID`" :disabled="busy" required />
            <button class="btn" type="button" :disabled="busy" @click="draft.people.splice(index, 1)">Remove</button>
          </div>
          <button class="btn" type="button" :disabled="busy" @click="addPerson">Add relationship</button>
        </div>
        <input v-else :id="`edit-${field.key}`" v-model="draft[field.key]" :type="field.kind === 'number' ? 'number' : 'text'" :required="field.required" :min="field.kind === 'number' ? 1 : undefined" :max="field.kind === 'number' ? 86400000 : undefined" :maxlength="field.kind === 'date' ? 10 : 500" :disabled="busy" />
      </div>
      <p v-if="error.message" class="catalog-error" role="alert">{{ error.message }}</p>
      <button class="btn btn--primary" data-testid="save-record" type="submit" :disabled="busy">{{ busy ? 'Saving…' : 'Save changes' }}</button>
    </form>
  </section>
</template>
