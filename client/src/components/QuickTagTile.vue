<template>
<div>
    <q-card flat class='q-mx-md q-my-sm qt-tile' :class='{"bg-darker": selected}'>
        <div class='row'>
            <div class='selected-bar bg-primary' v-if='selected'></div>
            <div class='row q-pt-md q-pl-md full-width'>
                <!-- Title -->
                <div class='col-6'>
                    <span class='text-h6 text-weight-bold text-no-wrap'>{{track.title}}</span><br>
                    <span class='text-subtitle1 text-grey-6 text-weight-medium text-no-wrap'>{{track.artists.join(", ")}}</span>
                </div>
                <!-- Details -->
                <div class='col-6 row text-grey-6 text-weight-medium text-center items-center'>
                    <div class='col-4'>
                        <!-- Mood -->
                        <q-chip 
                            v-if='getMood(track.mood)'
                            :color='getMood(track.mood).color'
                            :outline='getMood(track.mood).outline'
                            :label='getMood(track.mood).mood'
                        ></q-chip>
                    </div>
                    <div class='col-6'>
                        <span>{{track.genres.join(", ")}}</span>
                    </div>
                    <div class='col-1'>
                        <span>{{track.bpm}}</span>
                    </div>
                    <div class='col-1 q-mt-xs'>
                        <!-- <q-btn round flat icon='mdi-dots-horizontal' color='primary'></q-btn> -->
                    </div>
                </div>
            </div>
            <!-- Custom tags -->
            <div class='row q-mx-sm q-my-sm'>
                <div v-for='(tag, i) in track.getAllCustom($1t.settings.quickTag.custom)' :key='"qtc"+i'>
                    <q-chip :label='tag' outline color='primary' text-color='white' class='chip-text-white'></q-chip>
                </div>
            </div>
            

        </div>
    </q-card>
</div>
</template>

<script>
export default {
    name: 'QuickTagTile',
    props: {
        track: Object
    },
    methods: {
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
    },
    computed: {
        selected() {
            return this.$1t.quickTag.track && this.track.path == this.$1t.quickTag.track.path;
        }
    }
}
</script>

<style>
.selected-bar {
    position: absolute;
    width: 5px;
    height: 128px;
    border-radius: 4px;
}
.chip-text-white div {
    color: white;
}
.qt-tile {
    height: 128px;
    min-height: 128px;
    max-height: 128px;
}
</style>