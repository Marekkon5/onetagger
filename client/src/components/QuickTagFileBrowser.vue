<template>
<div class='q-px-sm'>

    <!-- Path -->
    <div class='text-weight-bold text-subtitle2 clickable path-display q-my-sm' @click='browse'>
        <div class='row inline'>
            <span style="direction:ltr;" class='text-primary monospace'>{{path}}</span>
        </div>
    </div>

    <div class='q-mt-sm'>

        <!-- Filter -->
        <q-input dense filled label='Filter' class='q-mb-sm' @input='applyFilter' v-model='filter'></q-input>

        <!-- Parent -->
        <div class='q-mb-sm clickable te-file' @click='loadFiles("..")'>
            <q-icon size='xs' class='q-mb-xs text-grey-4' name='mdi-folder-upload'></q-icon>
            <span class='q-ml-sm text-subtitle2 text-grey-4'>Parent folder</span>
        </div>

        <!-- Files -->
        <div v-for='file in files' :key='file.filename'>
            <div 
                class='clickable te-file' 
                @click='(file.dir || file.playlist) ? loadFiles(file.filename) : loadFile(file.path)'
                :class='{"text-primary": isSelected(file.path), "text-grey-4": !isSelected(file.path)}'
            >
                <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='!file.dir && !file.playlist' name='mdi-music'></q-icon>
                <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='file.dir' name='mdi-folder'></q-icon>
                <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='file.playlist' name='mdi-playlist-music'></q-icon>
                <span class='q-ml-sm text-subtitle2'>{{file.filename}}</span>
            </div>
        </div>

    </div>

</div>
</template>

<script>
export default {
    name: 'QuickTagFileBrowser',
    data() {
        return {
            path: this.$1t.settings.path,
            files: [],
            originalFiles: [],
            filter: null,
            initial: true,
        }
    },
    methods: {
        loadFiles(f = null) {
            this.$1t.send('quickTagFolder', {path: this.path, subdir: f});
        },
        browse() {
            this.$1t.send('browse', {context: 'qt', path: this.path});
        },
        applyFilter() {
            if (!this.filter || this.filter.trim().length == 0) {
                this.files = this.originalFiles;
                return;
            }
            this.files = this.originalFiles.filter(f => f.filename.toLowerCase().includes(this.filter));
        },
        isSelected(path) {
            return path == this.$1t.settings.path;
        }
    },
    mounted() {
        // fix path loading
        this.path = this.$1t.settings.path;
        // Register events
        this.$1t.onQuickTagBrowserEvent = (json) => {
            switch (json.action) {
                case 'quickTagFolder':
                    // Load dir
                    if (!this.initial) {
                        this.$1t.settings.path = json.path;
                        this.$1t.loadQuickTag();
                    } 
                    this.initial = false;
                
                    if (json.files.length == 0) return;
                    this.files = json.files;
                    this.originalFiles = json.files;
                    this.path = json.path;
                    break;
            }
        }

        this.initial = true;
        this.loadFiles('..');
    }
}
</script>