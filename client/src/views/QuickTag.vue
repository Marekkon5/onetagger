<template>
<div>
    <!-- Tracks -->
    <div class='tracklist qt-full-height' v-if='$1t.quickTag.tracks.length > 0' ref='tracklist' :class='{"qt-height": $1t.quickTag.track}'>
        <!-- Tracklist -->
        <div v-for='(track, i) in $1t.quickTag.tracks' :key='i'>
            <QuickTagTile @click.native='trackClick(i)' :track='$1t.quickTag.track' v-if='$1t.quickTag.track && track.path == $1t.quickTag.track.path'></QuickTagTile>
            <QuickTagTile @click.native='trackClick(i)' :track='track' v-if='!$1t.quickTag.track || track.path != $1t.quickTag.track.path'></QuickTagTile>
        </div>
    </div>

    <!-- No path selected -->
    <div v-if='$1t.quickTag.tracks.length == 0' class='qtbg-container qt-full-height' @click='selectFolder'>
        <div>
            <div class='text-center text-grey-6 text-h3'>No folder selected!</div>
            <div class='text-center text-grey-6 text-h5 q-pt-md'>Click here to select folder</div>
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

</div>
</template>

<script>
import QuickTagTile from '../components/QuickTagTile';

export default {
    name: 'QuickTag',
    components: {QuickTagTile},
    data() {
        return {
            saveDialog: false
        }
    },
    methods: {
        //Click on track card
        trackClick(i) {
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
        //On unsaved changes
        this.$1t.onQTUnsavedChanges = () => {
            this.saveDialog = true;
            setTimeout(() => {
                this.$refs.saveButton.$el.focus();
            }, 100)
        }

        //Load tracks if path available
        this.$1t.loadQuickTag();
    },
    watch: {
        '$1t.quickTag.track'() {
            let index = this.$1t.quickTag.tracks.findIndex((t) => this.$1t.quickTag.track.path == t.path);
            this.$refs.tracklist.scrollTop = index * 100 - 200;
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

</style>