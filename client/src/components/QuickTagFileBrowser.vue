<template>
<div class='q-px-md'>

    <!-- Path -->
    <div class='text-weight-bold clickable path-display q-my-md' v-if='!editPath'>
        <div class='row inline'>
            <span style="direction:ltr;">
                <span @click='browse' class='text-primary monospace q-pr-xs'>{{path}}</span>
                <q-icon name='mdi-pencil' class='q-pb-xs' @click='editPath = true'></q-icon>
            </span>
        </div>
    </div>
    <div class='q-my-sm' v-if='editPath'>
        <form @submit.prevent='loadFiles()'>
            <q-input outlined dense v-model='path'></q-input>
        </form>
    </div>


    <div class='q-mt-sm'>

        <!-- Filter -->        
        <q-input dense filled label='Filter' class='q-mb-sm' @update:model-value='applyFilter' v-model='filter'></q-input>

        <!-- Parent -->
        <div class='q-mb-sm clickable te-file' @click='loadFiles("..")'>
            <q-icon size='xs' class='q-mb-xs text-grey-5' name='mdi-folder-upload'></q-icon>
            <span class='q-ml-sm text-caption text-grey-5'>Parent folder</span>
        </div>

        <!-- Files -->
        <div v-for='file in files' :key='file.filename'>
            <div 
                class='clickable te-file' 
                @click='(file.dir || file.playlist) ? loadFiles(file.filename) : loadFiles(file.path)'
                :class='{"text-primary": isSelected(file.path), "text-grey-5": !isSelected(file.path)}'
            >
                <q-icon size='xs' class='q-mb-xs text-grey-5' v-if='!file.dir && !file.playlist' name='mdi-music'></q-icon>
                <q-icon size='xs' class='q-mb-xs text-grey-5' v-if='file.dir' name='mdi-folder'></q-icon>
                <q-icon size='xs' class='q-mb-xs text-grey-5' v-if='file.playlist' name='mdi-playlist-music'></q-icon>
                <span class='q-ml-sm text-caption'>{{file.filename}}</span>
            </div>
        </div>

    </div>

</div>
</template>

<script lang='ts' setup>
import { onMounted, ref } from 'vue';
import { get1t } from '../scripts/onetagger.js';

const $1t = get1t();
const path = ref($1t.settings.value.path);
const files = ref<any[]>([]);
const originalFiles = ref<any[]>([]);
const filter = ref<string | undefined>(undefined);
const initial = ref(true);
const editPath = ref(false);

function loadFiles(f?: string) {
    $1t.send('quickTagFolder', {path: path.value, subdir: f});
}

function browse() {
    $1t.browse('qt', path.value);
}

function applyFilter() {
    if (!filter.value || filter.value.trim().length == 0) {
        files.value = originalFiles.value;
        return;
    }
    files.value = originalFiles.value.filter(f => f.filename.toLowerCase().includes(filter.value?.toLowerCase()));
}

function isSelected(path: string) {
    return path == $1t.settings.value.path;
}

onMounted(() => {
    // fix path loading
    path.value = $1t.settings.value.path;
    // Register events
    $1t.onQuickTagBrowserEvent = (json) => {
        switch (json.action) {
            case 'quickTagFolder':
                // Load dir
                if (!initial.value) {
                    $1t.settings.value.path = json.path;
                    $1t.loadQuickTag();
                } 
                initial.value = false;
            
                if (json.files.length == 0) return;
                files.value = json.files;
                originalFiles.value = json.files;
                path.value = json.path;
                break;
            case 'pathUpdate':
                initial.value = true;
                $1t.send('quickTagFolder', { path: $1t.settings.value.path, subdir: '..' });
        }
    }

    initial.value = true;
    loadFiles('..');
});

</script>