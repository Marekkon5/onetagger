<template>
<div>
    <!-- Tracks -->
    <div class='tracklist qt-full-height' v-if='$1t.quickTag.tracks.length > 0' ref='tracklist' :class='{"qt-height": $1t.quickTag.track}'>
        <!-- Tracklist -->
        <q-intersection v-for='(track, i) in $1t.quickTag.tracks' :key='i' style='height: 136px;'>
            <QuickTagTile @click.native='trackClick(i)' :track='$1t.quickTag.track' v-if='$1t.quickTag.track && track.path == $1t.quickTag.track.path'></QuickTagTile>
            <QuickTagTile @click.native='trackClick(i)' :track='track' v-if='!$1t.quickTag.track || track.path != $1t.quickTag.track.path'></QuickTagTile>
        </q-intersection>
    </div>

    <!-- No path selected -->
    <div v-if='$1t.quickTag.tracks.length == 0' class='qtbg-container qt-full-height' @click='selectFolder'>
        <div>
            <div class='text-center text-grey-6 text-h3'>No folder selected!</div>
            <div class='text-center text-grey-6 text-h5 q-pt-md'>Click here to select folder</div>
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
    <q-dialog v-model='noteDialog'>
        <q-card v-if='$1t.quickTag.track'>
            <q-card-section>
                <div class='text-h6'>Note tag</div>
            </q-card-section>
            <q-card-section>
                <q-input
                    filled
                    dense
                    label="Note tag"
                    style='width: 256px;'
                    @input='$1t.quickTag.track.setNote($event)'
                    :value='$1t.quickTag.track.getNote()'
                ></q-input>
            </q-card-section>
            <q-card-section class='justify-end row'>
                <q-btn color='primary' @click='noteDialog = false' class='text-black'>Save</q-btn>
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
            note: null
        }
    },
    methods: {
        //Click on track card
        trackClick(i) {
            //Prevent clicking on same track
            if (this.$1t.quickTag.track && this.$1t.quickTag.tracks[i].path == this.$1t.quickTag.track.path) return;
            this.$1t.loadQTTrack(this.$1t.quickTag.tracks[i]);
        },
        //Save dialog callback
        async saveDialogCallback(save) {
            if (save) {
                await this.$1t.saveQTTrack();
            }
            this.$1t.loadQTTrack(null, true);
            this.saveDialog = false;
        },
        //Select folder and load tracks
        selectFolder() {
            this.$1t.send('browse', {context: 'qt'});
        },
    },
    mounted() {
        //Keybind callbacks
        this.$1t.onQTUnsavedChanges = () => {
            this.saveDialog = true;
            setTimeout(() => {
                this.$refs.saveButton.$el.focus();
            }, 100)
        }
        this.$1t.onQTNoteTag = () => {
            this.noteDialog = true;
        }

        //Load tracks if path available
        this.$1t.loadQuickTag();
    },
    watch: {
        '$1t.quickTag.track'() {
            let index = this.$1t.quickTag.tracks.findIndex((t) => this.$1t.quickTag.track.path == t.path);
            this.$refs.tracklist.scrollTop = index * 136 - 200;
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
    height: calc(100vh - 172px);
}

.qt-height {
    height: calc(100vh - 220px);
}

.keybind-icon {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}

</style>