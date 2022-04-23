<template>
<div @mouseleave='mouseOver = false' @mouseenter="mouseOver = true">
    <q-card flat class='q-mx-md qt-tile' :class='{"bg-darker": selected || mouseOver}'>
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
                        <!-- Not current track rating -->
                        <q-rating 
                            size='1.4em' 
                            v-model='track.energy'
                            no-reset
                            readonly
                            v-if='!selected'
                        ></q-rating>
                        <!-- Current track rating -->
                        <q-rating 
                            size='1.4em' 
                            v-model='$1t.quickTag.track.energy'
                            v-if='selected'
                        ></q-rating>
                    </div>

                    <div class='col-4 text-grey-4'>
                        <!-- Genres -->
                        <div v-if='selected'>
                            <span 
                                v-for='(genre, i) in track.genres' 
                                :key='"gen"+i'
                                :class='{"hover-strike": selected}'
                                @click='removeGenre(genre)'
                            >
                                {{genre}}<span v-if='i != track.genres.length - 1'>, </span>
                            </span>
                        </div>
                        <div v-if='!selected'>{{track.genres.join(', ')}}</div>

                        <div class='text-grey-6'>{{track.year}}</div>
                    </div>
                    <div class='col-1'>
                        <span v-if='track.bpm'>{{track.bpm}}</span>
                        <br v-if='track.bpm && track.key'>
                        <span :style='keyColor(track.key)'>{{track.key}}</span>
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

const KEY_COLORS = {
    "12A": "#00e5e5",
    "12B": "#00e5e5",
    "01A": "#00d58f",
    "01B": "#00d58f",
    "02A": "#3ffb3f",
    "02B": "#3ffb3f",
    "03A": "#97fb00",
    "03B": "#97fb00",
    "04A": "#fed600",
    "04B": "#fed600",
    "05A": "#f98c28",
    "05B": "#f98c28",
    "06A": "#fe642d",
    "06B": "#fe642d",
    "07A": "#f94949",
    "07B": "#f94949",
    "08A": "#fe3fea",
    "08B": "#fe3fea",
    "09A": "#ac64fe",
    "09B": "#ac64fe",
    "10A": "#3e89fa",
    "10B": "#3e89fa",
    "11A": "#00c9fe",
    "11B": "#00c9fe"
};

const CAMELOT_KEYS = {
    "ABM" :"01A",
    "G#M" :"01A",
    "B"   :"01B",
    "D#M" :"02A",
    "EBM" :"02A",
    "GB"  :"02B",
    "F#"  :"02B",
    "A#M" :"03A",
    "BBM" :"03A",
    "C#"  :"03B",
    "DB"  :"03B",
    "DD"  :"03B",
    "FM"  :"04A",
    "G#"  :"04B",
    "AB"  :"04B",
    "CM"  :"05A",
    "D#"  :"05B",
    "EB"  :"05B",
    "GM"  :"06A",
    "A#"  :"06B",
    "BB"  :"06B",
    "DM"  :"07A",
    "F"   :"07B",
    "AM"  :"08A",
    "C"   :"08B",
    "EM"  :"09A",
    "G"   :"09B",
    "BM"  :"10A",
    "D"   :"10B",
    "GBM" :"11A",
    "F#M" :"11A",
    "A"   :"11B",
    "C#M" :"12A",
    "DBM" :"12A",
    "E"   :"12B",
}

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
        // Get mood by name
        getMood(name) {
            if (!name) return null;
            let mood = this.$1t.settings.quickTag.moods.find(m => m.mood == name);
            // Inject outline if unknown mood
            if (mood) {
                mood.outline = false;
                return mood;
            }
            return {mood: name, color: 'white', outline: true};
        },
        // Remove genre from track
        removeGenre(genre) {
            this.$1t.quickTag.track.toggleGenre(genre);
        },
        // Get color for musical key
        keyColor(key) {
            if (!key) return;
            key = key.trim().toUpperCase();
            // Camelot
            let color = KEY_COLORS[CAMELOT_KEYS[key]];
            // Normal
            if (!color) {
                if (key.length < 3) key = `0${key}`;
                color = KEY_COLORS[key];
            }
            if (color) {
                return `color: ${color};`;
            }
        }
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
.hover-strike:hover {
    text-decoration: line-through;
    cursor: pointer;
}
</style>