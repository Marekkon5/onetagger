<template>
<div>
    <!-- Tracks -->
    <div class='tracklist qt-full-height' v-if='$1t.quickTag.value.tracks.length > 0' ref='tracklist' :class='{"qt-height": $1t.quickTag.value.track}'>
        <!-- Search -->
        <q-input
            filled 
            dense
            v-model='filter'
            :label-slot="true"
            class='q-px-md q-pt-md'
        >
            <template v-slot:label>
                <q-icon name="mdi-magnify" size="sm"></q-icon>
            </template>
        </q-input>

        <!-- Sort -->
        <div class='row text-grey-6 justify-between q-mx-lg q-mt-sm q-mb-xs'>
            <div v-for='(option, i) in sortOptions' :key='"so"+i' @click='sort(option)' class='row clickable'>
                <div :class='{"text-bold": sortOption == option}'>{{option}}</div>
                <div v-if='sortOption == option' class='q-pl-xs'>
                    <q-icon class='q-pb-xs' name='mdi-arrow-up' v-if='!sortDescending'></q-icon>
                    <q-icon class='q-pb-xs' name='mdi-arrow-down' v-if='sortDescending'></q-icon>
                </div>
            </div>
        </div>

        <!-- Stats -->
        <div class='q-mx-lg text-grey-6 q-my-xs text-caption'>
            Loaded files: <span class='text-bold'>{{$1t.quickTag.value.tracks.length}}</span>
             | Filtered: <span class='text-bold'>{{tracks.length}}</span>
            <span v-if='$1t.quickTag.value.failed.length != 0'> | Failed to load: 
                <span class='text-bold cursor-pointer' @click='failedDialog = true'>{{$1t.quickTag.value.failed.length}} 
                    <span class='text-underline q-pl-xs'>(show)</span>
                </span>
            </span>
        </div>

        <!-- Tracklist -->
        <div v-for='(item, i) in tracks' :key='i'>
            <q-intersection style='height: 140px;' @click.native='trackClick(item)' once>
                <QuickTagTile :track='$1t.quickTag.value.track' v-if='$1t.quickTag.value.track && item.path == $1t.quickTag.value.track.path'></QuickTagTile>
                <QuickTagTile :track='item' v-if='!$1t.quickTag.value.track || item.path != $1t.quickTag.value.track.path'></QuickTagTile>
            </q-intersection>
        </div>

        <!-- No results -->
        <div v-if='tracks.length == 0'>
            <div class='text-center text-h4 text-grey-6 q-my-lg'>No results!</div>
        </div>
    </div>

    <div v-if='$1t.quickTag.value.tracks.length == 0' class='qtbg-container qt-full-height'>
        <!-- Loading -->
        <div v-if='$1t.lock.value.locked' class='row justify-center'>
            <q-circular-progress indeterminate color='primary' size='64px'></q-circular-progress>
        </div>

        <!-- No path selected -->
        <div @click='selectFolder' v-if='!$1t.lock.value.locked'>
            <div class='text-center text-subtitle1 text-bold text-primary q-my-sm'>NO FOLDER SELECTED</div>
            <div class='text-center text-subtitle1 text-grey-6'>Click here to select folder</div>
            <div class='q-mt-xl text-subtitle1 text-grey-6 text-center'>
                Play/Pause: <q-icon name='mdi-keyboard-space' class='keybind-icon'></q-icon><br>
                Seek -10s: <q-icon name='mdi-chevron-left' class='keybind-icon'></q-icon><br>
                Seek +30s: <q-icon name='mdi-chevron-right' class='keybind-icon'></q-icon><br>
                Change tracks: <q-icon name='mdi-chevron-up' class='keybind-icon q-mr-xs'></q-icon> / <q-icon name='mdi-chevron-down' class='keybind-icon'></q-icon> <br>
                Save: <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm text-subtitle2'>S</span><br>
                Confirm save: <q-icon name='mdi-keyboard-return' class='keybind-icon'></q-icon><br>
            </div>
        </div>
    </div>
    
    <!-- Save dialog -->
    <q-dialog v-model='saveDialog'>
        <q-card>
            <q-card-section>
                <div class='text-h6 text-grey-3'>Unsaved changes</div>
            </q-card-section>
            <q-card-section>
                <div class='text-grey-3'>Warning, there are unsaved changes on this track, do you want to save them?</div>
            </q-card-section>
            <q-card-actions align='right'>
                <q-btn color='red' flat text @click='saveDialogCallback(false)'>Discard</q-btn>
                <q-btn color='primary' flat text @click='saveDialogCallback(true)' ref='saveButton'>Save</q-btn>
            </q-card-actions>
        </q-card>
    </q-dialog>

    <!-- Note tag dialog -->
    <q-dialog v-model='noteDialog' @show='onNoteDialogShow'>
        <q-card v-if='$1t.quickTag.value.track'>
            <q-card-section>
                <div class='text-h6'>Custom note</div>
            </q-card-section>
            <q-card-section>
                <q-input
                    filled
                    dense
                    label="Note tag"
                    style='width: 256px;'
                    :model-value='$1t.quickTag.value.track.getNote()'
                    @update:model-value='(d: any) => $1t.quickTag.value.track!.setNote(d)'
                    @keyup.enter="noteDialog = false"
                    ref='noteDialogInput'
                ></q-input>
            </q-card-section>
        </q-card>
    </q-dialog>

    <!-- Failed files dialog -->
    <q-dialog v-model='failedDialog'>
        <q-card class='q-pa-md'>
            <q-card-section>
                <div class='text-h4'>Failed to load</div>
            </q-card-section>
            <q-card-section>
                <div>
                    <div v-for='failed in $1t.quickTag.value.failed' class='q-my-sm'>
                        <div class='text-body2 monospace'>{{failed.path}}</div>
                        <div class='text-body2 text-red'>{{failed.error}}</div>
                    </div>
                </div>
            </q-card-section>
            <q-card-section horizontal>
                <q-space></q-space>
                <q-btn flat color='primary' @click='failedDialog = false'>Close</q-btn>
            </q-card-section>
        </q-card>
    </q-dialog>

</div>
</template>

<script lang='ts' setup>
import { scroll, useQuasar } from 'quasar';
import { computed, onDeactivated, onMounted, ref, watch } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { QTTrack } from '../scripts/quicktag.js';

import QuickTagTile from '../components/QuickTagTile.vue';

const { setVerticalScrollPosition } = scroll;

const $1t = get1t();
const $q = useQuasar();
const sortOptions = ['title', 'artist', 'custom', 'mood', 'energy', 'genre', 'year', 'bpm', 'key'];
const saveDialog = ref(false);
const noteDialog = ref(false);
const filter = ref<string | undefined>(undefined);
const sortDescending = ref(false);
const sortOption = ref('title');
const failedDialog = ref(false);


// Click on track card
function trackClick(track: QTTrack) {
    // Prevent clicking on same track
    if ($1t.quickTag.value.track && track.path == $1t.quickTag.value.track.path) return;
    $1t.loadQTTrack(track);
}

// Save dialog callback
async function saveDialogCallback(save: boolean) {
    if (save) {
        await $1t.saveQTTrack();
        $q.notify({
            message: "Tags saved!",
            color: 'primary',
            textColor: 'black',
            timeout: 500,
        });
    }
    $1t.loadQTTrack(undefined, true);
    saveDialog.value = false;
    // focus on custom tags fix
    setTimeout(() => { $1t.quickTagUnfocus(); }, 50);
}

// Select folder and load tracks
function selectFolder() {
    $1t.browse('qt');
}

// Focus
const noteDialogInput = ref<HTMLElement | undefined>();
function onNoteDialogShow() {
    noteDialogInput.value?.focus();
}

// Sort by option
function sort(option: string) {
    if (sortOption.value != option) {
        // reset sort direction
        sortDescending.value = false;
        sortOption.value = option;
    } else {
        sortDescending.value = !sortDescending.value;
    }
}

// Scroll to track index
const tracklist = ref<HTMLElement | undefined>();
function scrollToIndex(index: number) {
    setVerticalScrollPosition(tracklist.value!, index * 140 - 154, 250);
    // this.$refs.tracklist.scrollTop = index * 140 - 140;
}

const tracks = computed(() => {
    let tracks = $1t.quickTag.value.tracks;
    if (filter.value) {
        let newFilter = filter.value.toLowerCase();
        // title, artist or track or tags
        tracks = $1t.quickTag.value.tracks.filter((t) => 
            t.title.toLowerCase().includes(newFilter) || t.path.toLowerCase().includes(newFilter) ||
            t.artists.filter((a: any) => a.toLowerCase().includes(newFilter)).length > 0 ||
            (t.mood??'').toLowerCase().includes(newFilter) ||
            t.getAllCustom().some((i: any) => i.toLowerCase().includes(newFilter)) ||
            (t.genres??[]).some((i: any) => i.toLowerCase().includes(newFilter)) 
        );
    }
    if (!sortOption.value) return tracks;
    
    // Sort
    tracks.sort((a, b) => {
        let va, vb;
        switch (sortOption.value) {
            // Arrays
            case 'artist':
            case 'genre':
                va = a[`${sortOption.value}s`].join(', ').toLowerCase();
                vb = b[`${sortOption.value}s`].join(', ').toLowerCase();
                break;
            default:
                va = (a as any)[sortOption.value]??''.toLowerCase();
                vb = (b as any)[sortOption.value]??''.toLowerCase();
                break;
        }
        

        // Compare
        if (va < vb) {
            return -1;
        }
        if (va > vb) {
            return 1;
        }
        return 0;
    });
    if (sortDescending.value) tracks.reverse();

    return tracks;
});

const saveButton = ref<any>();
onMounted(() => {
    $1t.onQuickTagEvent = (action, data) => {
        switch (action) {
            // Save dialog
            case 'onUnsavedChanges':
                //Autosave enabled
                if ($1t.settings.value.quickTag.autosave) {
                    saveDialogCallback(true);
                    return;
                }

                saveDialog.value = true;
                setTimeout(() => {
                    saveButton.value?.$el.focus()
                }, 100)
                break;
            // Note tag updated
            case 'onNoteTag':
                noteDialog.value = true;
                break;
            // Change track position relatively
            case 'changeTrack':
                var offset = data.offset;
                var i = tracks.value.findIndex((t) => t.path == $1t.quickTag.value.track!.path);
                if (i != -1 && (i + offset) != tracks.value.length && (i + offset) >= 0) {
                    $1t.loadQTTrack(tracks.value[i + offset], data.force??false);
                }
                break;
            case 'focusSearch':
                break
            case 'quickTagLoad':
                if ($1t.settings.value.quickTag.trackIndex == -1 || $1t.quickTag.value.tracks.length == 0 || $1t.lock.value.locked) return;
                // Reload last opened track track
                setTimeout(() => {
                    $1t.loadQTTrack($1t.quickTag.value.tracks[$1t.settings.value.quickTag.trackIndex]);
                    $1t.settings.value.quickTag.trackIndex = -1;
                }, 50);

                break;
            default:
                console.log(`Unknown QT Event: ${action} ${data}`);
                break;
        }
    }

    // Restore sort state
    sortOption.value = $1t.settings.value.quickTag.sortOption||'title';
    sortDescending.value = $1t.settings.value.quickTag.sortDescending === true;

    // Load tracks if path available
    $1t.loadQuickTag();
});

onDeactivated(() => {
    // Save track index
    if ($1t.quickTag.value.track)
        $1t.settings.value.quickTag.trackIndex = $1t.quickTag.value.tracks.findIndex((t) => $1t.quickTag.value.track!.path == t.path);
    else
        $1t.settings.value.quickTag.trackIndex = -1;
    // Save sorting
    $1t.settings.value.quickTag.sortOption = sortOption.value;
    $1t.settings.value.quickTag.sortDescending = sortDescending.value;

    $1t.saveSettings(false);
});

watch(() => $1t.quickTag.value.track, () => {
    let index = $1t.quickTag.value.tracks.findIndex((t) => $1t.quickTag.value.track!.path == t.path);
    scrollToIndex(index);
});
</script>

<style lang='scss'>
.tracklist {
    overflow-y: auto;
}

.qtbg-container {
    display: flex;
    flex-direction: column;
    justify-content: center;
    cursor: pointer;
}

.qt-full-height {
    height: calc(100vh - 132px);
}

.qt-height {
    height: calc(100vh - 160px);
}

.keybind-icon {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}

.bar-bg {
    background: #00ff00;
}

</style>