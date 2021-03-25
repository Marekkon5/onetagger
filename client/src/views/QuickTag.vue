<template>
<div>
    <!-- Tracks -->
    <div class='tracklist' v-if='$1t.quickTag.tracks.length > 0' ref='tracklist'>
        <!-- Tracklist -->
        <div v-for='(track, i) in $1t.quickTag.tracks' :key='i'>
            <q-card flat class='q-mx-md q-my-sm' @click='trackClick(i)'
                :class='{"bg-darker": $1t.quickTag.track && track.path == $1t.quickTag.track.path}'>
                <div class='row'>
                    <div class='selected-bar bg-primary' v-if='$1t.quickTag.track && track.path == $1t.quickTag.track.path'>
                    </div>
                    <div class='row q-pa-md q-pl-md full-width'>
                        <!-- Title -->
                        <div class='col-5'>
                            <span class='text-h6 text-weight-bold text-no-wrap'>{{track.title}}</span><br>
                            <span class='text-subtitle1 text-grey-6 text-weight-medium text-no-wrap'>{{track.artists.join(", ")}}</span>
                        </div>
                        <!-- Details -->
                        <div class='col-7 row text-grey-6 text-weight-medium text-center items-center'>
                            <div class='col-3'>
                                <!-- Mood -->
                                <div v-if='!$1t.quickTag.track || track.path != $1t.quickTag.track.path'>
                                    <q-chip 
                                        v-if='getMood(track.mood)'
                                        :color='getMood(track.mood).color'
                                        :outline='getMood(track.mood).outline'
                                        :label='getMood(track.mood).mood'
                                    ></q-chip>
                                </div>
                                <!-- Current track mood -->
                                <div v-if='$1t.quickTag.track && track.path == $1t.quickTag.track.path'>
                                    <q-chip
                                        v-if='getMood($1t.quickTag.track.mood)'
                                        :color='getMood($1t.quickTag.track.mood).color'
                                        :outline='getMood($1t.quickTag.track.mood).outline'
                                        :label='getMood($1t.quickTag.track.mood).mood'
                                    ></q-chip>
                                </div>
                            </div>
                            <div class='col-3'>
                                <span>{{track.genres.join(", ")}}</span>
                            </div>
                            <div class='col-2'>
                                <span>{{track.releaseDate || 'N/A'}}</span>
                            </div>
                            <div class='col-1'>
                                <span>{{track.bpm || 'N/A'}}</span>
                            </div>
                            <div class='col-1'>
                                <span>G#</span>
                            </div>
                            <div class='col-2 q-mt-xs'>
                                <q-btn round flat icon='mdi-dots-horizontal' color='primary'></q-btn>
                            </div>
                        </div>
                    </div>
                </div>
            </q-card>
            <!-- <q-separator inset></q-separator> -->
        </div>
    </div>

    <!-- No path selected -->
    <div v-if='$1t.quickTag.tracks.length == 0' class='qtbg-container' @click='selectFolder'>
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
                <q-btn color='primary' flat text @click='saveDialogCallback(true)'>Save</q-btn>
            </q-card-actions>
        </q-card>
    </q-dialog>

</div>
</template>

<script>
export default {
    name: 'QuickTag',
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
        //Get mood by name
        getMood(name) {
            if (!name) return null;
            let mood = this.$1t.settings.quickTag.moods.find(m => m.mood == name);
            //Inject outline if unknown mood
            if (mood) {
                mood.outline = false;
                return mood;
            }
            return {mood: name, color: 'white', outline: true};
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
        }
    },
    mounted() {
        //On unsaved changes
        this.$1t.onQTUnsavedChanges = () => {
            this.saveDialog = true;
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
.selected-bar {
    position: absolute;
    width: 5px;
    height: 92px;
    border-radius: 4px;
}

.tracklist {
    overflow-y: auto;
    max-height: calc(100vh - 176px);
}

.qtbg-container {
    height: calc(100vh - 172px);
    display: flex;
    flex-direction: column;
    justify-content: center;
    cursor: pointer;
}
</style>