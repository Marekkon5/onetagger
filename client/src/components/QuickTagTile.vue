<template>
<div>
    <q-card flat class='q-mx-md qt-tile' :class='{"bg-onetagger-icon": selected}'>
        <div class='row qt-tile'>
            <div class='selected-bar bg-primary' v-if='selected'></div>
            <div class='row q-pt-md q-pl-md full-width full-height'>
                <!-- Art -->
                <div class='col-1 qt-tile-main'>
                    <q-img 
                        :src='art' 
                        width='50px' 
                        height='50px' 
                        class='rounded-borders' 
                        :placeholder-src='PLACEHOLDER_IMG'
                    >
                        <template v-slot:error>
                            <q-img :src='PLACEHOLDER_IMG' width='50px' height='50px' class='rounded-borders'></q-img>
                        </template>
                    </q-img>
                </div>
                <!-- Title -->
                <div class='col-4 q-pl-sm'>
                    <span class='text-subtitle2 text-grey-4 text-weight-medium text-no-wrap title-span qt-tile-main'>{{track.title}}</span>
                    <span class='text-subtitle2 text-grey-6 text-weight-medium text-no-wrap title-span'>{{track.artists.join(", ")}}</span>
                </div>
                <!-- Details -->
                <div class='col-7 row text-center text-subtitle2 text-weight-medium items-center'>
                    <div class='col-3 qt-tile-col' @click='removeMood(track.mood)'>
                        <!-- Mood -->
                        <q-chip 
                            v-if='getMood(track.mood)'
                            :color='getMood(track.mood)!.color + ""'
                            :outline='getMood(track.mood)!.outline'
                            :label='getMood(track.mood)!.mood'
                            class='cursor-pointer'
                        ></q-chip>
                    </div>
                    <div class='col-3 qt-tile-col'>
                        <!-- Track rating -->
                        <q-rating 
                            size='1.2em' 
                            v-model='track.energy'
                            no-reset
                            :readonly='!selected'
                        ></q-rating>
                    </div>

                    <div class='col-4 qt-tile-col text-grey-4 text-caption text-weight-bold'>
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

                        <div class='text-grey-6 text-weight-medium monospace'>{{track.year}}</div>
                    </div>
                    <div class='mt-3 col-1 qt-tile-col text-caption text-grey-4 text-weight-medium'>
                        <span class='monospace' v-if='track.bpm'>{{track.bpm}}</span>
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
        <div class='row q-mx-sm no-wrap overflow-hidden custom-tag-chips text-subtitle2'>
            <div v-for='(tag, i) in track.getAllCustom()' :key='"qtc"+i'  @click='removeCustom(tag)'>
                <q-chip 
                    icon='mdi-close'
                    dense 
                    square 
                    :label='tag.value' 
                    outline 
                    color='primary' 
                    class='qt-tile-chip' 
                ></q-chip>
            </div>
        </div>

    </q-card>
</div>
</template>

<script lang='ts' setup>
import { computed, toRef } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { CAMELOT_KEYS, CustomTagInfo, KEY_COLORS, OPENKEY_KEYS, PLACEHOLDER_IMG, QTTrack } from '../scripts/quicktag.js';
import { httpUrl } from '../scripts/utils.js';


const $1t = get1t();
const props = defineProps({
    track: { required: true, type: QTTrack },
    noArtCache: { default: false, type: Boolean }
});
const inputTrack = toRef(props, 'track');
const noArtCache = toRef(props, 'noArtCache');

// Get mood by name
function getMood(name?: string) {
    if (!name) return;
    let mood = $1t.settings.value.quickTag.moods.find(m => m.mood == name);
    // Inject outline if unknown mood
    if (mood) {
        mood.outline = false;
        return mood;
    }
    return { mood: name, color: 'white', outline: true };
}

function removeMood(mood?: string) {
    if (!mood || !selected.value) return;
    track.value.mood = undefined;
}

// Remove genre from track
function removeGenre(genre: string) {
    track.value.toggleGenre(genre);
}

// Get color for musical key
function keyColor(key?: string) {
    if (!key) return;
    key = key.trim().toUpperCase();
    // Camelot or OpenKey
    let color = KEY_COLORS[CAMELOT_KEYS[key.toUpperCase()]] || KEY_COLORS[OPENKEY_KEYS[key.toLowerCase()]];
    // Normal
    if (!color) {
        if (key.length < 3) key = `0${key}`;
        color = KEY_COLORS[key.toUpperCase()];
    }
    if (color) {
        return `color: ${color};`;
    }
}

/// Remove custom tag chip
function removeCustom(tag: CustomTagInfo) {
    if (!selected.value) return;

    if (tag.type === 'custom') {
        track.value.removeCustom(tag.index, tag.value);
        return;
    }    
    // Note
    let values = track.value.getNote().split(",")
                .map(n => n.trim())
                .filter(n => n && n != tag.value);
    track.value.setNote(values.join(", "));  // Note the space after comma
}


/// If selected, use selected track, else input track
const track = computed(() => {
    let track = $1t.quickTag.value.track.getTrack(inputTrack.value.path);
    if (!track) track = inputTrack.value;
    return track;
});

const selected = computed(() => $1t.quickTag.value.track.isSelected(track.value));
const art = computed(() => `${httpUrl()}/thumb?path=${encodeURIComponent(track.value.path)}${noArtCache.value ? "&_=" + Math.random().toString() : ""}`);

</script>

<style>
.selected-bar {
    position: absolute;
    width: 5px;
    height: 104px;
    border-radius: 4px;
}
.qt-tile-chip {
    cursor: pointer;
    font-size: 13px;
}

.qt-tile-chip div {
    color: #E0E0E0;
    font-size: 13px;
}

.qt-tile-chip .q-icon {
    display: none;
}

.qt-tile-chip:hover .q-icon {
    display: inline;
    padding-top: 2px;
    cursor: pointer;
}

.qt-tile {
    height: 104px;
    min-height: 104px;
    max-height: 104px;
}

.qt-tile:hover {
    background: #1A1A1A;
}

.title-span {
    text-overflow: ellipsis;
    overflow: hidden;
    display: block;
}
.custom-tag-chips {
    margin-top: -38px;
    margin-left: 12px;
}

.hover-strike:hover {
    text-decoration: line-through;    
    cursor: pointer;
}

.qt-tile-main {
    margin-top: -4px;
}

.qt-tile-col {
    margin-top: -41px;
}

.mt-3 {
    margin-top: -40px;
}

</style>