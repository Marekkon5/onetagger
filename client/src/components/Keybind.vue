<template>
<div>

    <!-- Icon -->
    <q-btn round flat dense icon='mdi-keyboard-outline' @click='overlay = true' v-if='!isSet'></q-btn>
    <div v-if='isSet' class='row q-pt-sm justify-around' @click='overlay = true'>
        <q-icon v-if='value?.shift' name='mdi-apple-keyboard-shift' size='xs' class='col-2'></q-icon>
        <q-icon v-if='value?.ctrl' name='mdi-apple-keyboard-control' size='xs' class='col-2'></q-icon>
        <q-icon v-if='value?.alt' name='mdi-apple-keyboard-option' size='xs' class='col-2'></q-icon>
        <q-icon :name='keyIcon(value)' size='xs' class='col-2'></q-icon>
    </div>

    <!-- Dialog picker -->
    <q-dialog @keydown='keydown' v-model='overlay'>
        <q-card>
            <q-card-section>
                <div class='text-h6'>Set keybind</div>
            </q-card-section>
            <q-card-section class='preview'>
                <q-icon v-if='key.shift' name='mdi-apple-keyboard-shift' size='md' class='q-px-sm'></q-icon>
                <q-icon v-if='key.ctrl' name='mdi-apple-keyboard-control' size='md' class='q-px-sm'></q-icon>
                <q-icon v-if='key.alt' name='mdi-apple-keyboard-option' size='md' class='q-px-sm'></q-icon>
                <q-icon :name='keyIcon(key)' size='md' class='q-px-sm'></q-icon>
            </q-card-section>
            <q-card-section class='text-right'>
                <q-btn flat color='red' @click='reset' v-if='isSet'>Reset</q-btn>
                <q-btn flat @click='close'>Cancel</q-btn>
                <q-btn flat color='primary' v-if='key.key' @click='set'>Save</q-btn>
            </q-card-section>
        </q-card>
    </q-dialog>

</div>
</template>

<script lang='ts' setup>
import { computed, onMounted, ref, watch } from 'vue';
import { Keybind } from '../scripts/utils.js';

const props = defineProps({
    modelValue: Object
});
const overlay = ref(false);
const key = ref(new Keybind());
const value = ref<Keybind | undefined>();
const emit = defineEmits(['update:modelValue']);
   
function keydown(e: KeyboardEvent) {
    //Save key
    if (e.code.match(/F\d{1,2}/) || e.code.startsWith('Key') || e.code.startsWith("Digit") || e.code.startsWith("Numpad")) {
        key.value.key = e.code.toLowerCase().replace("key", "").replace("digit", "").replace("numpad", "");
        key.value.ctrl = (e.ctrlKey || e.metaKey);
        key.value.alt = e.altKey;
        key.value.shift = e.shiftKey;
        e.preventDefault();
    }
}

// Close popup and reset
function close() {
    overlay.value = false;
    key.value.clear();
}

// Save
function set() {
    emit('update:modelValue', Object.assign(new Keybind(), key.value));
    value.value = key.value;
    close();
}

// Callback with null
function reset() {
    emit('update:modelValue', undefined);
    value.value = undefined;
    close();
}

// Get icon for key
function keyIcon(k?: Keybind) {
    if (!k || !k.key) return '';
    //Numeric
    if (!isNaN(parseInt(k.key, 10))) {
        return `mdi-numeric-${k.key}-box-outline`;
    }
    //F
    if (k.key.toString().match(/f\d{1,2}/)) {
        return `mdi-keyboard-${k.key}`;
    }
    return `mdi-alpha-${k.key}-box-outline`;
}

// Is key set
const isSet = computed(() => {
    if (!value.value || !value.value.key) return false;
    return true;
});

// Update initial
watch(props, () => {
    if (props.modelValue)
        value.value = Object.assign(new Keybind(), props.modelValue);
    else
        value.value = undefined;
});

onMounted(() => {
    if (props.modelValue)
        value.value = Object.assign(new Keybind(), props.modelValue);
    else
        value.value = undefined;
});

</script>

<style>
.preview {
    text-align: center;
    min-width: 200px;
}
</style>