<template>
<div>
    <!-- Tracks -->
    <div class='tracklist qt-full-height' v-if='$1t.quickTag.tracks.length > 0' ref='tracklist' :class='{"qt-height": $1t.quickTag.track}'>
        <q-input
            filled
            v-model='filter'
            label='Search'
            class='q-px-md q-py-md'
        ></q-input>
        
        <div v-for='(item, i) in tracks' :key='i'>
            <q-intersection style='height: 136px;' @click.native='trackClick(item)'>
                <QuickTagTile :track='$1t.quickTag.track' v-if='$1t.quickTag.track && item.path == $1t.quickTag.track.path'></QuickTagTile>
                <QuickTagTile :track='item' v-if='!$1t.quickTag.track || item.path != $1t.quickTag.track.path'></QuickTagTile>
            </q-intersection>
        </div>

        <!-- No results -->
        <div v-if='tracks.length == 0'>
            <div class='text-center text-h4 text-grey-6 q-my-sm'>No results!</div>
        </div>
    </div>

    <!-- No path selected -->
    <div v-if='$1t.quickTag.tracks.length == 0' class='qtbg-container qt-full-height' @click='selectFolder'>
        <div>
            <div class='text-center text-h4 text-grey-6 q-my-sm'>No folder selected!</div>
            <div class='text-center text-grey-6 text-h6'>Click here to select folder</div>
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
                <div class='text-h6'>Unsaved changes</div>
            </q-card-section>
            <q-card-section>
                <div>Warning, there are unsaved changes on this track, do you want to save them?</div>
            </q-card-section>
            <q-card-actions align='right'>
                <q-btn color='red' flat text @click='saveDialogCallback(false)'>Discard</q-btn>
                <q-btn color='primary' flat text @click='saveDialogCallback(true)' ref='saveButton'>Save</q-btn>
            </q-card-actions>
        </q-card>
    </q-dialog>

    <!-- Note tag dialog -->
    <q-dialog v-model='noteDialog' @show='onNoteDialogShow'>
        <q-card v-if='$1t.quickTag.track'>
            <q-card-section>
                <div class='text-h6'>Custom note</div>
            </q-card-section>
            <q-card-section>
                <q-input
                    filled
                    dense
                    label="Note tag"
                    style='width: 256px;'
                    :value='$1t.quickTag.track.getNote()'
                    @input='$1t.quickTag.track.setNote($event)'
                    @keyup.enter="noteDialog = false"
                    ref='noteDialogInput'
                ></q-input>
            </q-card-section>
        </q-card>
    </q-dialog>

</div>
</template>

<script>
import QuickTagTile from '../components/QuickTagTile';

export default {
    name: 'QuickTag',
    components: {QuickTagTile},
    data() {
        return {
            saveDialog: false,
            noteDialog: false,
            note: null,
            filter: null
        }
    },
    methods: {
        //Click on track card
        trackClick(track) {
            //Prevent clicking on same track
            if (this.$1t.quickTag.track && track.path == this.$1t.quickTag.track.path) return;
            this.$1t.loadQTTrack(track);
        },
        //Save dialog callback
        async saveDialogCallback(save) {
            if (save) {
                await this.$1t.saveQTTrack();
                this.$q.notify({
                    message: "Tags saved!",
                    color: 'primary',
                    textColor: 'black',
                    timeout: 500,
                })
            }
            this.$1t.loadQTTrack(null, true);
            this.saveDialog = false;
        },
        //Select folder and load tracks
        selectFolder() {
            this.$1t.send('browse', {context: 'qt'});
        },
        //Focus
        onNoteDialogShow() {
            this.$refs.noteDialogInput.focus();
        }
    },
    computed: {
        tracks() {
            if (!this.filter)
                return this.$1t.quickTag.tracks;
            // title, path or artist
            let filter = this.filter.toLowerCase();
            return this.$1t.quickTag.tracks.filter((t) => 
                t.title.toLowerCase().includes(filter) || t.path.toLowerCase().includes(filter) ||
                t.artists.filter((a) => a.toLowerCase().includes(filter)).length > 0
            );
        }
    },
    mounted() {
        this.$1t.onQuickTagEvent = (action, data) => {
            switch (action) {
                // Save dialog
                case 'onUnsavedChanges':
                    //Autosave enabled
                    if (this.$1t.settings.quickTag.autosave) {
                        this.saveDialogCallback(true);
                        return;
                    }

                    this.saveDialog = true;
                    setTimeout(() => {
                        this.$refs.saveButton.$el.focus();
                    }, 100)
                    break;
                // Note tag updated
                case 'onNoteTag':
                    this.noteDialog = true;
                    break;
                // Change track position relatively
                case 'changeTrack':
                    var i = this.tracks.findIndex((t) => t.path == this.$1t.quickTag.track.path);
                    if (i != -1 && (i + data) != this.tracks.length && (i + data) >= 0) {
                        this.$1t.loadQTTrack(this.tracks[i + data]);
                    }
                    break;
                case 'focusSearch':

                    break
                default:
                    console.log(`Unknown QT Event: ${action} ${data}`);
                    break;
            }
        }

        //Load tracks if path available
        this.$1t.loadQuickTag();
    },
    watch: {
        '$1t.quickTag.track'() {
            let index = this.$1t.quickTag.tracks.findIndex((t) => this.$1t.quickTag.track.path == t.path);
            this.$refs.tracklist.scrollTop = index * 136 - 140;
            // window.scrollTo(0, index * 60);
        }
    }
}
</script>

<style>
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

</style>