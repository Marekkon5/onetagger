<template>
<div class='full-height'>

    <div class='row full-height'>
        <!-- File browser -->
        <div 
            @contextmenu.prevent="" 
            class='q-px-md q-pt-sm bg-darker' 
            :class='{"col-4": !$1t.settings.value.tagEditorDouble, "col-3": $1t.settings.value.tagEditorDouble}'
            style='max-height: 100%; overflow-y: scroll;'
        >
            <div class='text-weight-bold text-subtitle2 clickable path-display' @click='browse'>
                <div class='row inline'>
                    <span style="direction:ltr;" class='text-primary monospace'>{{path}}</span>
                </div>
            </div>
            <div class='q-mt-sm'>

                <!-- Filter -->
                <q-input dense filled label='Filter' class='q-mb-sm' @update:model-value='(v: any) => applyFilter(v as string)' v-model='filter'></q-input>

                <!-- Parent -->
                <div class='q-mb-sm clickable te-file' @click='loadFiles("..")'>
                    <q-icon size='xs' class='q-mb-xs text-grey-4' name='mdi-folder-upload'></q-icon>
                    <span class='q-ml-sm text-subtitle2 text-grey-4'>Parent folder</span>
                </div>

                <draggable 
                    id='fileList' 
                    :move='onFileMove' 
                    group='files' 
                    :list='files' 
                    item-key='filename'
                    @change='onFileDrag'>
                    <template #item='{ element: file }'>
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
                    </template>

                    
                </draggable>
            </div>
        </div>

        <!-- Custom list -->
        <div 
            @contextmenu.prevent="" 
            class='col-3 bg-darker q-px-md q-pt-sm' 
            v-if='$1t.settings.value.tagEditorDouble'
            style='max-height: 100%; overflow-y: scroll;'
        >
            <div class='bg-darker separator'></div>
            <div class='row justify-between'>
                <div class='text-weight-bold text-subtitle2 text-primary q-pb-sm'>Your list</div>
                <div>
                    <q-btn round dense size='xs' flat style='margin-right: 2px;' @click='clearCustom'>
                        <q-icon name='mdi-close' color='red'></q-icon>
                    </q-btn>
                </div>
            </div>
            
            <draggable 
                group='files' 
                :move='onFileMove' 
                :list='customList' 
                @change='onFileDrag' 
                style='height: calc(100% - 32px)'
                :item-key="(e: any) => `//CUSTOM${e}`"
            >
                <template #item='{ element: f }'>
                    <div class='row'>
                        <div 
                            @click='loadFile(f)' 
                            class='te-file clickable q-my-xs q-mr-sm' 
                            style='width: calc(100% - 32px)' 
                            :class='{"text-primary": isSelected(f), "text-grey-4": !isSelected(f)}'
                        >
                            <span>{{filename(f)}}</span>
                        </div>
                        <div>
                            <q-btn size='xs' class='q-mt-xs' flat round style='float: right;' @click='removeCustom(f)'>
                                <q-icon name='mdi-close' color='red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </template>
            </draggable>
        </div>

        <!-- Tags -->
        <div 
            :class='{"col-8": !$1t.settings.value.tagEditorDouble, "col-6": $1t.settings.value.tagEditorDouble}'
            style='max-height: 100%; overflow-y: scroll;'>
            <div v-if='!file' class='justify-center items-center content-center row full-height'>
                
                <div class='col-12 text-subtitle1 text-bold text-primary text-center q-my-sm'>NO FILE SELECTED</div><br>
                <span class='text-center text-subtitle1 text-grey-6'>Tip: Click the path to select folder using your OS's picker</span>
            </div>

            <div v-if='file' class='q-px-md'>
                <div class='text-center q-py-md text-subtitle1 text-primary'>{{file.filename}}</div>
                <div class='q-mt-md'>
                    <div v-for='(tag, i) in Object.keys(file.tags)' :key='i' class='row q-my-sm'>
                        <div class='col-3 text-subtitle2  text-grey-3 q-mt-sm q-pr-xs' style='text-overflow: ellipsis; overflow: hidden;'>
                            <span v-if='ABSTRACTIONS[tag]'><span class='text-weight-bold'>{{ABSTRACTIONS[tag]}}</span> ({{tag}})</span>
                            <span v-if='!ABSTRACTIONS[tag]'>{{tag}}</span>
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
                    <div class='col-3 q-pt-sm text-uppercase text-primary text-subtitle2'>Add new text tag</div>
                    <TagField tageditor class='col-8' dense :format='tagFormat!' @change='newTag = $event'></TagField>
                    <div class='col-1 q-pl-md q-pt-xs'>
                        <q-btn round dense flat @click='addNewTag'>
                            <q-icon name='mdi-plus' class='text-primary'></q-icon>
                        </q-btn>
                    </div>
                </div>

                <!-- Album art -->
                <div class='text-subtitle2 text-grey-3 text-weight-bold'>
                    Album art
                    <q-btn round flat class='q-mb-xs q-ml-sm' @click='addAlbumArtDialog = true'>
                        <q-icon name='mdi-plus' color='primary'></q-icon>
                    </q-btn>
                </div>
                <div class='q-mt-sm  text-grey-4 albumart-container text-center'>
                    <div v-for='(image, i) in file.images' :key='"art"+i' class='q-mr-md'>
                        <q-img :src='image.data' class='albumart clickable' @click='albumArt = image.data; showAlbumArt = true'></q-img>
                        <div class='q-pt-sm q-mb-md'>
                            <div v-if='file.format != "mp4"' class='text-subtitle1'>{{image.kind}}</div>
                            <div v-if='file.format != "mp4"' class='text-subtitle1'>{{image.description}}</div>
                            <div class='text-subtitle2 monospace'>{{image.mime}} {{image.width}}x{{image.height}}</div>
                            <q-btn color='red' class='q-mt-sm' @click='removeArt(i)'>Remove</q-btn>
                        </div>
                    </div>
                </div>

                <!-- ID3 specific tags -->
                <div v-if='file.id3'>
                    <!-- Comments -->
                    <div class='text-subtitle2'>
                        <span class='text-grey-3 text-weight-bold'>Comments</span> (COMM)
                        <q-btn round flat class='q-mb-xs q-ml-sm' @click='addID3Comment'>
                            <q-icon name='mdi-plus' color='primary'></q-icon>
                        </q-btn>
                    </div>
                    <div>
                        <div v-for='(comment, i) in file.id3.comments' :key='"comm"+i' class='row q-py-sm'>
                            <q-input
                                filled
                                dense
                                label='Language'
                                class='col-2'
                                v-model='file.id3.comments[i].lang'
                                maxlength='3'
                                @change='id3CommentsChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                label='Description'
                                class='col-4 q-pl-sm'
                                v-model='file.id3.comments[i].description'
                                @change='id3CommentsChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                label='Text'
                                class='col-5 q-pl-sm'
                                v-model='file.id3.comments[i].text'
                                @change='id3CommentsChange'
                            ></q-input>
                            <div class='col-1 q-pl-md q-pt-xs'>
                                <q-btn round dense flat @click='removeID3Comment(i)'>
                                    <q-icon name='mdi-delete' class='text-red'></q-icon>
                                </q-btn>
                            </div>
                        </div>
                    </div>

                    <!-- Unsynchronized lyrics -->
                    <div class='text-subtitle2 text-grey-3'>
                        <span class='text-weight-bold'>Unsynchronized lyrics</span> (USLT)
                        <q-btn round flat class='q-mb-xs q-ml-sm' @click='addID3USLT'>
                            <q-icon name='mdi-plus' color='primary'></q-icon>
                        </q-btn>
                    </div>
                    <div>
                        <div v-for='(lyric, i) in file.id3.unsync_lyrics' :key='"uslt"+i' class='q-py-sm'>
                            <div class='row'>
                                <q-input
                                    filled
                                    dense
                                    label='Language'
                                    class='col-3'
                                    v-model='file.id3.unsync_lyrics[i].lang'
                                    maxlength='3'
                                    @change='id3USLTChange'
                                ></q-input>
                                <q-input
                                    filled
                                    dense
                                    label='Description'
                                    class='col-8 q-pl-md'
                                    v-model='file.id3.unsync_lyrics[i].description'
                                    @change='id3USLTChange'
                                ></q-input>
                                <div class='col-1 q-pl-md q-pt-xs'>
                                    <q-btn round dense flat @click='removeID3USLT(i)'>
                                        <q-icon name='mdi-delete' class='text-red'></q-icon>
                                    </q-btn>
                                </div>
                            </div>
                            <q-input
                                filled
                                dense
                                label='Text'
                                v-model='file.id3.unsync_lyrics[i].text'
                                type='textarea'
                                class='q-pt-sm'
                                @change='id3USLTChange'
                            ></q-input>
                        </div>
                    </div>

                    <!-- Popularimeter -->
                    <div>
                        <div class='text-subtitle2 text-grey-3'>
                            <span class='text-weight-bold'>Popularimeter</span> (POPM)
                            <q-btn v-if='!file.id3.popularimeter' round flat class='q-mb-xs q-ml-sm' @click='addPOPM'>
                                <q-icon name='mdi-plus' color='primary'></q-icon>
                            </q-btn>
                        </div>
                        <div v-if='file.id3.popularimeter' class='row q-py-sm'>
                            <q-input
                                filled
                                dense
                                label='Email'
                                class='col-4'
                                v-model='file.id3.popularimeter.email'
                                @change='id3POPMChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                type='number'
                                label='Play count'
                                class='col-3 q-pl-sm'
                                v-model='file.id3.popularimeter.counter'
                                maxlength='9'
                                @change='id3POPMChange'
                            ></q-input>
                            <div class='col-4 q-pl-md'>
                                <q-slider
                                    :min='0'
                                    :max='255'
                                    label
                                    label-text-color='black'
                                    :label-value='POPMLabel'
                                    v-model='file.id3.popularimeter.rating'
                                    @change='id3POPMChange'
                                ></q-slider>
                            </div>
                            <div class='col-1 q-pl-md q-pt-xs'>
                                <q-btn round dense flat @click='removePOPM'>
                                    <q-icon name='mdi-delete' class='text-red'></q-icon>
                                </q-btn>
                            </div>
                        </div>
                    </div>

                    <!-- ID3v2.4 -->
                    <div class='q-mb-xl q-mt-sm'>
                            <div class='text-subtitle2 text-grey-3'>
                            <span class='text-weight-bold'>Options</span>
                        </div>
                        <q-toggle label='Use ID3v2.4' class='q-mt-md' v-model='id3v24'></q-toggle>
                    </div>

                </div>

                <!-- Save -->
                <q-page-sticky position='bottom-right' :offset='[36, 18]'>
                    <q-btn push
                        @click='save'
                        color="primary"
                        class='text-black'
                        label="Save"
                    ></q-btn>
                </q-page-sticky>

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

<script lang='ts' setup>
import TagField from '../components/TagField.vue';
import AddAlbumArt from '../components/AddAlbumArt.vue';
import draggable from 'vuedraggable';
import { ABSTRACTIONS } from '../scripts/tags';
import { computed, onDeactivated, onMounted, ref } from 'vue';
import { get1t } from '../scripts/onetagger';
import { useQuasar } from 'quasar';

const $1t = get1t();
const $q = useQuasar();
const path = ref($1t.settings.value.path);
const files = ref<any[]>([]);
const originalFiles = ref<any[]>([]);
const file = ref<any>(undefined);
const filter = ref<any>(undefined);
const changes = ref<any[]>([]);
const newTag = ref<any>(undefined);
const albumArt = ref<any>(undefined);
const showAlbumArt = ref(false);
const addAlbumArtDialog = ref(false);
const customList = ref($1t.settings.value.tagEditorCustom);
const id3v24 = ref(false);


function loadFiles(f?: string) {
    $1t.send('tagEditorFolder', {path: path.value, subdir: f});
}

function browse() {
    $1t.browse('te', path.value);
}

function loadFile(path: string) {
    // Autosave
    if (file.value && $1t.settings.value.tagEditorAutosave) {
        save();
    }
    changes.value = [];

    // Will be joined in backend
    $1t.send('tagEditorLoad', {path});
    if ($1t.settings.value.tagEditorPlayer)
        $1t.player.value.loadTrack(path);
}

// If file is currently open
function isSelected(path: string) {
    if (!file.value) return false;
    return file.value.path == path;
}

function applyFilter(v: string) {
    filter.value = v;
    if (!filter.value || filter.value.trim().length == 0) {
        files.value = originalFiles.value;
        return;
    }
    files.value = originalFiles.value.filter(f => f.filename.toLowerCase().includes(filter.value));
}


/*
    Custom list
*/

// Vue draggable file drag process
function onFileDrag(e: any) {
    if (e.added) {
        if (e.added.element.dir || e.added.element.playlist) {
            $1t.send('tagEditorFolder', {path: path.value, subdir: e.added.element.filename, recursive: true});
            // Don't copy
            customList.value.splice(e.added.newIndex, 1);
        } else {
            // Duplicate
            if (!customList.value.find((i) => i == e.added.element.path)) 
                customList.value.splice(e.added.newIndex, 1, e.added.element.path);
            else 
                customList.value.splice(e.added.newIndex, 1);
        }
    }
    // Read again
    if (e.removed) {
        files.value.splice(e.removed.oldIndex, 0, e.removed.element);
    }
    saveSettings();
}

// Allow only one way drag
function onFileMove(e: any) {
    if (e.relatedContext.component.$el.id == 'fileList') return false;
}
function removeCustom(i: string) {
    customList.value.splice(customList.value.indexOf(i), 1);
    saveSettings();
}

// Get filename from path
function filename(path: string) {
    path = path.toString();
    if (path.trim().startsWith('/')) {
        let s = path.split('/');
        return s[s.length - 1];
    }
    let s = path.split('\\');
    return s[s.length - 1];
}
function clearCustom() {
    customList.value = [];
    saveSettings();
}

/*
    Text Tags
*/

// Delete tag
function removeTag(tag: string) {
    delete file.value.tags[tag];
    changes.value.push({
        type: 'remove',
        tag: tag
    })
}

// Create new tag
function addNewTag() {
    if (!newTag.value) return;
    if (file.value.tags[newTag.value]) {
        $q.notify({
            message: "Tag already exists!",
            timeout: 2000,
            position: 'top-right'
        });
        return;
    }
    // Remove removal of tag
    let i = changes.value.findIndex((c) => c.type == 'remove' && c.tag == newTag.value);
    if (i > -1) changes.value.splice(i, 1);

    file.value.tags[newTag.value] = '';
    changes.value.push({
        type: 'raw',
        tag: newTag.value,
        value: []
    });
}

function onChange(tag: string) {
    let value = file.value.tags[tag]
    // Split only for tags, MP3 write to single tag as id3 separator
    if (file.value.format != 'mp3') {
        value = value.split(',');
    } else {
        value = [value];
    }
    // Generate change
    let index = changes.value.findIndex((c) => c.tag == tag);
    if (index != -1) {
        changes.value[index].value = value; 
    } else {
        changes.value.push({
            type: 'raw',
            tag: tag,
            value: value
        });
    }
}

/*
    Album Art
*/

// Add new album art
function addAlbumArt(data: any) {
    changes.value.push({
        type: 'addPictureBase64',
        mime: data.mime,
        data: data.data,
        kind: data.kind,
        description: data.description
    });
    data.data = `data:${data.mime};base64,${data.data}`;
    file.value.images.push(data);
}

// Delete album art
function removeArt(i: number) {
    let kind = file.value.images[i].kind;
    file.value.images.splice(i, 1);
    //Remove newly added image
    let index = changes.value.findIndex((c) => c.type == "addPictureBase64" && c.kind == kind);
    if (index != -1) {
        changes.value.splice(index, 1);
        return;
    }
    changes.value.push({
        type: 'removePicture',
        kind
    });
}

/*
    ID3 Comments
*/

// Generate new change for ID3 comments
function id3CommentsChange() {
    let i = changes.value.findIndex((c) => c.type == 'id3Comments');
    if (i > -1) {
        changes.value.splice(i, 1);
    }
    changes.value.push({
        type: 'id3Comments',
        comments: file.value.id3.comments
    });
}

function addID3Comment() {
    file.value.id3.comments.push({
        lang: "eng",
        description: "",
        text: ""
    });
    id3CommentsChange();
}

function removeID3Comment(i: number) {
    file.value.id3.comments.splice(i, 1);
    id3CommentsChange();
}


/*
    ID3 Unsynchronized lyrics
*/
function id3USLTChange() {
    let i = changes.value.findIndex((c) => c.type == 'id3UnsynchronizedLyrics');
    if (i > -1) changes.value.splice(i, 1);
    changes.value.push({
        type: 'id3UnsynchronizedLyrics',
        lyrics: file.value.id3.unsync_lyrics
    });
}
function removeID3USLT(i: number) {
    file.value.id3.unsync_lyrics.splice(i, 1);
    id3USLTChange();
}
function addID3USLT() {
    file.value.id3.unsync_lyrics.push({
        lang: 'eng',
        description: '',
        text: ''
    });
    id3USLTChange();
}

/*
    ID3 Popularimeter
*/
function id3POPMChange() {
    // Remove existing popm changes
    let i = changes.value.findIndex((c) => c.type == 'id3Popularimeter');
    if (i > -1) changes.value.splice(i, 1);
    i = changes.value.findIndex((c) => c.type == "remove" && c.tag == "POPM");
    if (i > -1) changes.value.splice(i, 1);
    // Add new changes
    if (file.value.id3.popularimeter) {
        file.value.id3.popularimeter.counter = parseInt(file.value.id3.popularimeter.counter.toString());
        changes.value.push({
            type: 'id3Popularimeter',
            popm: file.value.id3.popularimeter
        });
    } else {
        changes.value.push({
            type: 'remove',
            tag: 'POPM'
        });
    }
}
function addPOPM() {
    file.value.id3.popularimeter = {
        email: "no@email",
        rating: 0,
        counter: 0
    }
    id3POPMChange();
}
function removePOPM() {
    file.value.id3.popularimeter = null;
    id3POPMChange();
}


/*
    Saving and backend
*/

// Save to file
function save() {
    $1t.send('tagEditorSave', {
        changes: {
            path: file.value.path, 
            changes: changes.value,
            separators: {id3: ', ', vorbis: null, mp4: ', '},
            id3v24: id3v24.value
        }
    });
    changes.value = [];
}

function saveSettings() {
    $1t.settings.value.path = path.value;
    $1t.settings.value.tagEditorCustom = customList.value;
    $1t.saveSettings(false);
}

// Websocket callback
function wsCallback(e: any) {
    switch (e.action) {
        case 'browse':
            path.value = e.path;
            loadFiles();
            break;
        case 'tagEditorFolder':
            if (e.recursive) {
                // Add dir to custom list
                let files = customList.value.concat(e.files.sort((a: any, b: any) => {
                    return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                }).map((f: any) => f.path));
                // Deduplicate
                customList.value = [... new Set(files)];
            } else {
                path.value = e.path;
                //Dirs first and sort
                originalFiles.value = e.files.sort((a: any, b: any) => {
                    if (a.dir && !b.dir) return -1;
                    if (b.dir && !a.dir) return 1;
                    return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                });
                applyFilter(filter.value);
            }
            saveSettings();
            break;
        case 'tagEditorLoad':
            file.value = e.data;
            break;
        case 'tagEditorSave':
            $q.notify({
                message: 'Tags written!',
                timeout: 4000,
                position: 'top-right'
            });
            break;
        // Internal callback
        case '_tagEditorSave':
            save();
            break;
        default: 
            console.log(e);
            break;
    }
}

const tagFormat = computed(() => {
    if (!file.value) return null;
    if (file.value.format == 'flac' || file.value.format == 'ogg') return 'vorbis';
    if (file.value.format == 'mp4') return 'mp4';
    return 'id3';
});

// Filter used types
const albumArtTypes = computed(() => {
    let types = ["CoverFront", "CoverBack", "Other", "Artist", "Icon", "OtherIcon", 
        "Leaflet", "Media", "LeadArtist", "Conductor", "Band", "Composer", "Lyricist", 
        "RecordingLocation", "DuringRecording", "DuringPerformance", "ScreenCapture", 
        "BrightFish", "Illustration", "BandLogo", "PublisherLogo"];
    if (!file.value) return types;
    return types.filter((t) => file.value.images.find((i: any) => i.kind == t) ? false : true);
});

const POPMLabel = computed(() => {
    let v = file.value.id3.popularimeter.rating;
    let stars = Math.ceil(v / 51);
    if (stars == 0) stars = 1;
    return `${v} (${stars}⭐)`;
});

// Register callback
onMounted(() => {
    $1t.onTagEditorEvent = wsCallback;
    loadFiles();

    // Load QT track
    if ($1t.quickTag.value.track.tracks.length == 1) {
        loadFile($1t.quickTag.value.track.tracks[0].path);
    }
})

// Unregister
onDeactivated(() => {
    $1t.onTagEditorEvent = () => {};
})

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