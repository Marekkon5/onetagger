<template>
<div class='full-height'>

    <div class='row full-height'>
        <!-- File browser -->
        <div class='bg-darker q-px-md q-pt-sm' :class='{"col-4": !$1t.settings.tagEditorDouble, "col-3": $1t.settings.tagEditorDouble}'>
            <div class='text-weight-bold text-subtitle2 monospace clickable path-display' @click='browse'>
                <div class='row inline'>
                    <span style="direction:ltr;" class='text-primary'>{{path}}</span>
                </div>
            </div>
            <div class='q-mt-sm'>
                <!-- Parent -->
                <div class='q-mb-sm clickable te-file' @click='loadFiles("..")'>
                    <q-icon size='xs' class='q-mb-xs' name='mdi-folder-upload'></q-icon>
                    <span class='q-ml-sm text-subtitle2'>Parent folder</span>
                </div>

                <draggable 
                    id='fileList' 
                    :move='onFileMove' 
                    group='files' 
                    :list='files' 
                    @change='onFileDrag'>
                    <div v-for='file in files' :key='file.filename'>
                        <div 
                            class='clickable te-file' 
                            @click='file.dir ? loadFiles(file.filename) : loadFile(file.path)'
                            :class='{"text-primary": isSelected(file.path)}'
                        >
                            <q-icon size='xs' class='q-mb-xs' v-if='!file.dir' name='mdi-music'></q-icon>
                            <q-icon size='xs' class='q-mb-xs' v-if='file.dir' name='mdi-folder'></q-icon>
                            <span class='q-ml-sm text-subtitle2'>{{file.filename}}</span>
                        </div>
                    </div>
                </draggable>
            </div>
        </div>

        <!-- Custom list -->
        <div class='col-3 bg-darker q-px-md q-pt-sm' v-if='$1t.settings.tagEditorDouble'>
            <div class='bg-grey-8 separator'></div>
            <div class='text-weight-bold text-subtitle2 text-primary q-pb-sm'>Your list</div>

            <draggable group='files' :move='onFileMove' :list='customList' @change='onFileDrag'  style='height: calc(100% - 32px)'>
                <div v-for='(f, i) in customList' :key='"c"+i'>
                    <div class='row'>
                        <div @click='loadFile(f)' class='te-file clickable q-my-xs q-mr-sm' style='width: calc(100% - 32px)' :class='{"text-primary": isSelected(f)}'>
                            <span>{{filename(f)}}</span>
                        </div>
                        <div>
                            <q-btn size='xs' class='q-mt-xs' flat round style='float: right;' @click='removeCustom(i)'>
                                <q-icon name='mdi-close' color='red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </div>
            </draggable>
        </div>

        <!-- Tags -->
        <div :class='{"col-8": !$1t.settings.tagEditorDouble, "col-6": $1t.settings.tagEditorDouble}'>
            <div v-if='!file' class='justify-center items-center content-center row full-height'>
                <div class='col-12 text-h4 text-grey-7 text-center q-my-sm'>No file selected!</div><br>
                <div class='text-h6 text-grey-7'>Tip: Click the path to select folder using your OS's picker.</div>
            </div>

            <div v-if='file' class='q-px-md'>
                <div class='text-center q-py-md text-subtitle1 text-primary'>{{file.filename}}</div>
                <div class='q-mt-md'>
                    <div v-for='(tag, i) in Object.keys(file.tags)' :key='i' class='row q-my-sm'>
                        <div class='col-3 text-subtitle1 q-mt-xs'>
                            <span v-if='abstractions[tag]'><span class='text-weight-bold'>{{abstractions[tag]}}</span> ({{tag}})</span>
                            <span v-if='!abstractions[tag]'>{{tag}}</span>
                        </div>
                        
                        <q-input
                            v-model='file.tags[tag]'
                            filled
                            dense
                            class='col-8'
                            @change='onChange(tag)'
                        ></q-input>

                        <div class='col-1 q-pl-md q-pt-xs'>
                            <q-btn round dense flat @click='removeTag(tag)'>
                                <q-icon name='mdi-delete' class='text-red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </div>

                <!-- Add new tag -->
                <div class='row q-mt-xl'>
                    <div class='text-subtitle1 text-weight-bold col-3 q-pt-xs'>Add new tag:</div>
                    <TagField tageditor class='col-8' dense :format='tagFormat' @change='newTag = $event'></TagField>
                    <div class='col-1 q-pl-md q-pt-xs'>
                        <q-btn round dense flat @click='addNewTag'>
                            <q-icon name='mdi-plus' class='text-primary'></q-icon>
                        </q-btn>
                    </div>
                </div>

                <div class='text-subtitle1'>
                    ALBUM ART
                    <q-btn round flat class='q-mb-xs q-ml-sm' @click='addAlbumArtDialog = true'>
                        <q-icon name='mdi-plus' color='primary'></q-icon>
                    </q-btn>
                </div>
                <div class='q-mt-sm albumart-container text-center'>
                    <div v-for='(image, i) in file.images' :key='"art"+i' class='q-mr-md'>
                        <q-img :src='image.data' class='albumart clickable' @click='albumArt = image.data; showAlbumArt = true'></q-img>
                        <div class='q-pt-sm'>
                            <div class='text-subtitle1'>{{image.kind}}</div>
                            <div class='text-subtitle1'>{{image.description}}</div>
                            <div class='text-subtitle2 monospace'>{{image.mime}} {{image.width}}x{{image.height}}</div>
                            <q-btn color='red' class='q-mt-sm' @click='removeArt(i)'>Remove</q-btn>
                        </div>
                    </div>
                </div>
                

                <!-- Save -->
                <div class='q-my-md justify-center row'>
                    <q-btn color='primary' class='text-black' @click='save'>Save</q-btn>
                </div>

            </div>
        </div>
    </div>

    <!-- Album art dialog -->
    <q-dialog v-model='showAlbumArt' @hide='albumArt = null'>
        <q-img :src='albumArt' style='max-width: 50%;'></q-img>
    </q-dialog>

    <!-- Add album art dialog -->
    <q-dialog v-model='addAlbumArtDialog'>
        <AddAlbumArt :types='albumArtTypes' @close='addAlbumArtDialog = false' @save='addAlbumArt'></AddAlbumArt>
    </q-dialog>

</div>
</template>

<script>
import Vue from 'vue';
import TagField from '../components/TagField';
import AddAlbumArt from '../components/AddAlbumArt';
import draggable from 'vuedraggable';

export default {
    name: 'TagEditor',
    components: { draggable, TagField, AddAlbumArt },
    data() {
        return {
            path: this.$1t.settings.tagEdiorPath,
            files: [],
            file: null,
            changes: [],
            newTag: null,
            albumArt: null,
            showAlbumArt: false,
            addAlbumArtDialog: false,
            customList: this.$1t.settings.tagEditorCustom,
            abstractions: {
                "TIT2": "Title",
                "TCON": "Genre",
                "TALB": "Album",
                "TPE2": "Album Artist",
                "TCOM": "Composer",
                "TEXT": "Lyricist",
                "TIT3": "Mix Name",
                "TOPE": "Original Artist",
                "TIT1": "Grouping Serato/VDJ",
                "GRP1": "Grouping djay Pro",
                "TPUB": "Label",
                "TPE4": "Remixer",
                "IPLS": "Producer ID3v2.3",
                "TIPL": "Producer ID3v2.4",
                "TPE3": "Conductor"
            }
        } 
    },
    methods: {
        loadFiles(f = null) {
            this.$1t.send('tagEditorFolder', {path: this.path, subdir: f});
        },
        browse() {
            this.$1t.send('browse', {context: 'te'});
        },
        loadFile(path) {
            this.changes = [];
            this.newTag = null;
            //Will be joined in backend
            this.$1t.send('tagEditorLoad', {path});
        },
        //Get filename from path
        filename(path) {
            path = path.toString();
            if (path.trim().startsWith('/')) {
                let s = path.split('/');
                return s[s.length - 1];
            }
            let s = path.split('\\');
            return s[s.length - 1];
        },
        //Vue draggable file drag process
        onFileDrag(e) {
            if (e.added) {
                if (e.added.element.dir) {
                    this.$1t.send('tagEditorFolder', {path: this.path, subdir: e.added.element.filename, recursive: true});
                } else {
                    this.customList.splice(e.added.newIndex, 1, e.added.element.path);
                }
            }
            //Readd again
            if (e.removed) {
                this.files.splice(e.removed.oldIndex, 0, e.removed.element);
            }
            this.$1t.settings.tagEditorCustom = this.customList;
            this.$1t.saveSettings(false);
        },
        //Allow only one way drag
        onFileMove(e) {
            if (e.relatedContext.component.$el.id == 'fileList') return false;
        },
        removeCustom(i) {
            this.customList.splice(i, 1);
            this.$1t.settings.tagEditorCustom = this.customList;
            this.$1t.saveSettings(false);
        },
        //If file is currently open
        isSelected(path) {
            if (!this.file) return false;
            return this.file.path == path;
        },
        onChange(tag) {
            let value = this.file.tags[tag]
            //Split only for tags, MP3 write to single tag as id3 separator
            if (this.file.format == 'flac') {
                value = value.split(',');
            } else {
                value = [value];
            }
            //Generate change
            let index = this.changes.findIndex((c) => c.tag == tag);
            if (index != -1) {
                this.changes[index].value = value; 
            } else {
                this.changes.push({
                    type: 'raw',
                    tag: tag,
                    value: value
                });
            }
        },
        //Delete tag
        removeTag(tag) {
            Vue.delete(this.file.tags, tag);
            this.changes.push({
                type: 'remove',
                tag: tag
            })
        },
        //Delete album art
        removeArt(i) {
            let kind = this.file.images[i].kind;
            this.file.images.splice(i, 1);
            //Remove newly added image
            let index = this.changes.findIndex((c) => c.type == "addPictureBase64" && c.kind == kind);
            if (index != -1) {
                this.changes.splice(index, 1);
                return;
            }
            this.changes.push({
                type: 'removePicture',
                kind
            });
        },
        //Create new tag
        addNewTag() {
            if (!this.newTag) return;
            if (this.file.tags[this.newTag]) {
                this.$q.notify({
                    message: "Tag already exists!",
                    timeout: 2000
                });
                return;
            }

            this.file.tags[this.newTag] = "";
            this.changes.push({
                type: 'raw',
                tag: this.newTag,
                value: ''
            });
        },
        //Add new album art
        addAlbumArt(data) {
            this.changes.push({
                type: 'addPictureBase64',
                mime: data.mime,
                data: data.data,
                kind: data.kind,
                description: data.description
            });
            data.data = `data:${data.mime};base64,${data.data}`;
            this.file.images.push(data);
        },
        //Save to file
        save() {
            this.$1t.send('tagEditorSave', {changes: {path: this.file.path, changes: this.changes}});
            this.changes = [];
        },
      
        //Websocket callback
        wsCallback(e) {
            switch (e.action) {
                case 'browse':
                    this.path = e.path;
                    this.loadFiles();
                    break;
                case 'tagEditorFolder':
                    if (e.recursive) {
                        //Add dir to custom list
                        let files = this.customList.concat(e.files.sort((a, b) => {
                            return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                        }).map((f) => f.path));
                        //For some wtf reason the customList always has an folder object, so remove it
                        this.customList = [... new Set(files.filter((f) => typeof f == 'string'))];
                    } else {
                        this.path = e.path;
                        //Dirs first and sort
                        this.files = e.files.sort((a, b) => {
                            if (a.dir && !b.dir) return -1;
                            if (b.dir && !a.dir) return 1;
                            return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                        });
                    }
                    //Save
                    this.$1t.settings.tagEdiorPath = this.path;
                    this.$1t.settings.tagEditorCustom = this.customList;
                    this.$1t.saveSettings(false);
                    break;
                case 'tagEditorLoad':
                    this.file = e.data;
                    break;
                case 'tagEditorSave':
                    this.$q.notify({
                        message: 'Tags written!',
                        timeout: 4000
                    });
                    break;
                default: 
                    console.log(e);
                    break;
            }
        }
    },
    computed: {
        tagFormat() {
            if (!this.file) return null;
            if (this.file.format == 'flac') return 'flac';
            return 'id3';
        },
        //Filter used types
        albumArtTypes() {
            let types = ["CoverFront", "CoverBack", "Other", "Artist", "Icon", "OtherIcon", 
                "Leaflet", "Media", "LeadArtist", "Conductor", "Band", "Composer", "Lyricist", 
                "RecordingLocation", "DuringRecording", "DuringPerformance", "ScreenCapture", 
                "BrightFish", "Illustration", "BandLogo", "PublisherLogo"];
            if (!this.file) return types;
            return types.filter((t) => this.file.images.find((i) => i.kind == t) ? false : true);
        }
    },
    mounted() {
        //Register callback
        this.$1t.onTagEditorEvent = this.wsCallback;
        this.loadFiles();
    },
}
</script>

<style>
.te-file {
    padding: 2px;
    padding-left: 4px;
    border-radius: 8px;
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
}


.te-file:hover {
    background-color: #111312;
}
.path-display {
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
    direction: rtl;
    text-align: left;
}
.albumart {
    min-width: 128px;
    width: 128px;
    max-width: 128px;
    border-radius: 8px;
}
.albumart-container {
    display: flex;
    width: 180px;
}
.separator {
    width: 2px; 
    margin-left: -17px; 
    position: absolute;
    height: 100%;
}
</style>