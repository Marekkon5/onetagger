<template>

<div class='q-pa-md'>
    <!-- Note -->
    <div class='full-width row justify-center q-my-xs'>
            <q-btn outline color='primary' class='text-caption' v-if='$1t.quickTag.value.track' @click='$1t.onQuickTagEvent("onNoteTag")'>         
            Add custom note            
        </q-btn>
    </div>
    <div v-for='(tag, i) in $1t.settings.value.quickTag.custom' :key='"tag"+i' class='q-pb-md'>
        <!-- Tag title -->
        <q-expansion-item 
            :label='tag.name' 
            class='text-subtitle2 text-bold q-pb-sm'
            style='margin-bottom: -24px;'
            default-opened
            :model-value="true"
            :switch-toggle-side='false'
        >
            <!-- Values -->
            <div v-for='(value, j) in tag.values' :key='i+"value"+j'>
                <q-checkbox
                    :label='value.val'
                    :model-value='selected(i, value.val)'
                    @update:model-value='valueClick(i, value.val)'
                    dense
                    class='text-subtitle2 text-grey-5 full-width q-mt-xs'
                ></q-checkbox>
            </div>

            <!-- Add new -->
            <q-input ref='addNewTagRef' dense @keypress.enter="addNewTag" v-if='newTag == i' v-model='newTagValue'></q-input>

            <q-btn round flat color='primary' class='add-custom-btn' v-if='newTag == -1' @click='showNewTag(i)'>
                <q-icon name='mdi-plus'></q-icon>
            </q-btn>

        </q-expansion-item>

        <div class='q-mb-md'></div>
    </div>

    <!-- Reorder the values inside of tag -->
    <!-- <div class='full-width row justify-center'>
        <q-btn outline color='primary' class='text-caption'  @click='sortValues' v-if='$1t.quickTag.value.track'>Sort values</q-btn>
    </div> -->

    <!-- Manual tag -->
    <div class='full-width row justify-center' v-if='$1t.quickTag.value.track.tracks.length == 1'>
        <q-btn outline color='primary' class='text-caption' @click='$1t.onQuickTagEvent("onManualTag", {path: $1t.quickTag.value.track.tracks[0].path})'>MANUAL TAG</q-btn>
    </div>

</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { get1t } from '../scripts/onetagger.js';

const $1t = get1t();
const newTag = ref(-1);
const newTagValue = ref<string | undefined>(undefined);

// If the value is present in tag
function selected(tag: number, value: string) {
    return $1t.quickTag.value.track.getCustom(tag, value);
}

// Tag value click
function valueClick(tag: number, value: string) {
    $1t.quickTag.value.track.toggleCustom(tag, value);           
}

// Sort values inside of tag
function sortValues() {
    $1t.quickTag.value.track.sortCustom();
}

// Show add new tag input
const addNewTagRef = ref<any>();
function showNewTag(i: number) {
    newTag.value = i;
    setTimeout(() => {
        addNewTagRef.value[0].focus();
    }, 25);
}

// Adding new tag
function addNewTag() {
    if (newTagValue.value) {
        $1t.settings.value.quickTag.custom[newTag.value].values.push({val: newTagValue.value, keybind: undefined});
        $1t.saveSettings();
    }
    newTag.value = -1;
    newTagValue.value = undefined;
}

</script>

<style>
.q-expansion-item__container:first-child div {
    padding: 0px !important;
}
.q-checkbox__label {
    margin-left: 8px !important;
}
.add-custom-btn {
    position: absolute;
    bottom: -10px;
    left: 128px;
}
.hide-expand-icon {
    display: none !important;
}
</style>