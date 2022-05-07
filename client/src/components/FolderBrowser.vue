<template>
<div class='folder-browser'>
    <q-card class='q-pa-md'>

        <div class='text-h6'>Select folder</div>

        <!-- Tree view -->
        <div class='q-my-md folder-list' ref='folderList'>
            <q-tree dense :nodes='folders' @lazy-load='lazyLoad' node-key='path' v-if='baseLoaded' default-expand-all>
                <template v-slot:default-header="prop">
                    <div @click='path = prop.node.path' class='row clickable'>
                        <q-icon :class='{"text-primary": prop.node.path == path}' name='mdi-folder' size='xs' class='q-pr-sm'></q-icon>
                        <div :class='{"text-bold text-primary": prop.node.path == path}'>{{prop.node.label}}</div>
                    </div>
                </template>
            </q-tree>
        </div>

        <!-- Actions -->
        <div>
            <q-input filled dense label='Path' v-model='path'></q-input>
            <div class='row justify-end q-pt-md'>
                <q-btn flat color='red' @click='cancel'>Cancel</q-btn>
                <q-btn class='q-ml-md' flat color='primary' @click='save'>OK</q-btn>
            </div>
        </div>
    </q-card>
</div>
</template>

<script>
export default {
    name: 'FileBrowser',
    data() {
        return {
            path: '/',
            folders: [],
            onResolve: null,
            baseLoaded: false,
        }
    },
    props: {
        base: {
            type: String,
            default: '/'
        },
    },
    methods: {
        // Tree lazy loading
        lazyLoad({ node, done }) {
            this.path = node.parent;
            this.onResolve = done;
            this.$1t.send('folderBrowser', { path: this.path, child: node.label, base: false });
        },
        // Convert entry into a tree node
        convEntry(entry) {
            if (entry.children)
                entry.children = entry.children.map((e) => {
                    let parts = e.path.replace("\\", "/").split('/');
                    // Recurse
                    if (e.children) {
                        e = this.convEntry(e);
                    }
                    return {
                        label: parts[parts.length - 1],
                        lazy: e.children ? false : true,
                        parent: entry.path,
                        path: e.path,
                        children: e.children
                    }; 
                });
            return entry;
        },
        // Find scroll offset of a path
        findScrollOffset(entry, parts, prev = 0) {
            for(let i=0; i<(entry.children??[]).length; i++) {
                if (entry.children[i].label == parts[0]) {
                    parts.splice(0, 1);
                    return this.findScrollOffset(entry.children[i], parts, prev + i);
                }
            }
            return prev;
        },
        // Split path to parts
        pathParts(path) {
            let parts = path.replace("\\", "/").split("/");
            if (parts.length == 0) return [];
            if (parts[0] == '') {
                parts.splice(0, 1);
            }
            return parts;
        },
        // Close
        cancel() {
            this.$1t.folderBrowser.open = false;
        },
        // Save
        save() {
            this.$1t.onBrowse({ context: this.$1t.folderBrowser.context, path: this.path, action: 'browse' });
            this.$1t.folderBrowser.open = false;
        }
    },
    mounted() {
        // Register events
        this.$1t.onFolderBrowserEvent = (json) => {
            switch (json.action) {
                case 'folderBrowser':
                    // Base folder structure
                    if (json.base) {
                        var entry = this.convEntry(json.entry);
                        this.folders = entry.children;
                        this.path = json.path;
                        this.baseLoaded = true;

                        // Scroll
                        setTimeout(() => {
                            let offset = this.findScrollOffset(entry, this.pathParts(this.path));
                            this.$refs.folderList.scrollTo({
                                top: offset * 23,
                                behavior: 'smooth'
                            });
                        }, 64);
                        
                        break;
                    }

                    // Structure
                    var folders = this.convEntry(json.entry).children;

                    // Resolve lazy load
                    if (this.onResolve) {
                        this.onResolve(folders);
                        this.onResolve = null;
                    } else {
                        this.folders = folders;
                        this.baseLoaded = true;
                    }

                    this.path = json.path;
                    break;
            }
        }

        // Load base
        if (this.base) {
            this.$1t.send('folderBrowser', { path: this.base, child: '', base: true });
        } else {
            this.$1t.send('folderBrowser', { path: this.path, child: '', base: false });
        }
    }
}
</script>

<style>
.folder-browser {
    width: 500px;
    height: 590px;
}
.folder-list {
    overflow-y: scroll;
    height: 400px;
}
</style>