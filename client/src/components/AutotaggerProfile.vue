<template>
    <div class='row'>
        <q-select
            :options='profiles'
            dense
            outlined
            v-model='$1t.settings.value.autoTaggerProfile'
            label='Profile'
            @update:model-value="loadProfile"
        ></q-select>

        <!-- Save current profile -->
        <div class='q-ml-sm'>
            <q-btn round flat icon='mdi-content-save' @click='saveProfile'></q-btn>
        </div>
        <!-- Save as -->
        <div>
            <q-btn round flat icon='mdi-content-save-edit' @click='saveAs'></q-btn>
        </div>
        <!-- New one -->
        <div>
            <q-btn round flat icon='mdi-plus' @click='newProfile'></q-btn>
        </div>
        <!-- Delete -->
        <div>
            <q-btn round flat icon='mdi-delete' color='red' @click='removeProfile'></q-btn>
        </div>
    </div>
</template>


<script lang='ts' setup>
import { Dialog } from 'quasar';
import { computed } from 'vue';
import { AutotaggerConfig } from '../scripts/autotagger';
import { get1t } from '../scripts/onetagger';

const $1t = get1t();

/// Save current profile as new profile
function saveAs() {
    Dialog.create({
        title: 'Save profile as',
        message: 'Name of new profile:',
        prompt: {
            model: '',
            type: 'text'
        },
        cancel: true,
    }).onOk(name => {
        $1t.settings.value.saveATProfile(name, $1t.config.value);
        $1t.loadATProfile(name);
        $1t.saveSettings();
    });
}

/// Create new profile
function newProfile() {
    Dialog.create({
        title: 'Create new profile',
        message: 'Name of new profile:',
        prompt: {
            model: '',
            type: 'text'
        },
        cancel: true,
    }).onOk(name => {
        $1t.settings.value.saveATProfile(name, new AutotaggerConfig());
        $1t.loadATProfile(name);
        $1t.saveSettings();
    });
}

/// Delete this profile
function removeProfile() {
    let i = $1t.settings.value.autoTaggerProfiles.findIndex(p => p.name == $1t.settings.value.autoTaggerProfile);
    if (i == -1) return;
    $1t.settings.value.autoTaggerProfiles.splice(i, 1);
    $1t.settings.value.autoTaggerProfile = 'Default';
    $1t.saveSettings();
}

/// Load selected profile
function loadProfile() {
    $1t.loadATProfile($1t.settings.value.autoTaggerProfile);
    $1t.saveSettings(false);
}

/// Save current profile
function saveProfile() {
    $1t.settings.value.saveATProfile($1t.settings.value.autoTaggerProfile, $1t.config.value);
    $1t.saveSettings();
}

/// Get profiles list
const profiles = computed(() => {
    let profiles = $1t.settings.value.autoTaggerProfiles.map(p => p.name);
    return profiles;
});

</script>