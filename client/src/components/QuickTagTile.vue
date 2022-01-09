<template>
<div @mouseleave='mouseOver = false' @mouseenter="mouseOver = true">
    <q-card flat class='q-mx-md q-my-sm qt-tile' :class='{"bg-darker": selected || mouseOver}'>
        <div class='row qt-tile'>
            <div class='selected-bar bg-primary' v-if='selected'></div>
            <div class='row q-pt-md q-pl-md full-width full-height'>
                <!-- Art -->
                <div class='col-1 q-pt-xs'>
                    <q-img 
                        :src='art' 
                        width='50px' 
                        height='50px' 
                        class='rounded-borders' 
                        :placeholder-src='require("../assets/placeholder.png")'
                    />
                </div>
                <!-- Title -->
                <div class='col-4 q-pl-sm'>
                    <span class='text-subtitle1 text-grey-4 text-weight-bold text-no-wrap title-span'>{{track.title}}</span>
                    <span class='text-subtitle2 title-span text-grey-6 text-weight-medium text-no-wrap'>{{track.artists.join(", ")}}</span>
                </div>
                <!-- Details -->
                <div class='col-7 row text-grey-6 text-weight-medium text-center items-center'>
                    <div class='col-3'>
                        <!-- Mood -->
                        <q-chip 
                            v-if='getMood(track.mood)'
                            :color='getMood(track.mood).color + "-6"'
                            :outline='getMood(track.mood).outline'
                            :label='getMood(track.mood).mood'
                        ></q-chip>
                    </div>
                    <div class='col-3'>
                        <q-rating 
                            size='1.4em' 
                            v-model='track.energy'
                            no-reset
                        ></q-rating>
                    </div>

                    <div class='col-4 text-grey-4'>
                        <div>{{track.genres.join(", ")}}</div>
                        <div class='text-grey-6'>{{track.year}}</div>
                    </div>
                    <div class='col-1'>
                        <span>{{track.bpm}}</span>
                        <br v-if='track.bpm && track.key'>
                        <span>{{track.key}}</span>
                    </div>
                    <div class='col-1 q-mt-xs'>
                        <!-- <q-btn round flat icon='mdi-dots-horizontal' color='primary'></q-btn> -->
                    </div>
                </div>
            </div>

        </div>

        <!-- Custom tags -->
        <div class='row q-mx-sm no-wrap overflow-hidden custom-tag-chips'>
            <div v-for='(tag, i) in track.getAllCustom()' :key='"qtc"+i'>
                <q-chip dense square :label='tag' outline color='primary' class='chip-text-white'></q-chip>
            </div>
        </div>

    </q-card>
</div>
</template>

<script>
export default {
    name: 'QuickTagTile',
    data() {
        return {
            mouseOver: false
        }
    },
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
        },
        art() {
            return `http://${window.location.hostname}:36913/thumb?path=${encodeURIComponent(this.track.path)}`;
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
.title-span {
    text-overflow: ellipsis;
    overflow: hidden;
    display: block;
}
.custom-tag-chips {
    margin-top: -38px;
}
</style>