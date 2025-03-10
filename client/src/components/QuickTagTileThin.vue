<template>
    <div class='selected-bar-thin bg-primary' v-if='selected'></div>

    <q-card class='qt-tile-thin row items-center' :class='{"qt-tile-thin-selected": selected, "qt-tile-thin-odd": odd}'>
        <div class='text-caption text-grey-4 text-weight-medium q-px-sm text-no-wrap ellipsis' style='width: 12vw;'>{{ track.title }}</div>
        <div class='text-caption title-span text-grey-6 text-weight-medium q-px-sm text-no-wrap ellipsis' style='width: 12vw;'>{{ track.artists.join(", ") }}</div>

        <!-- Mood -->
        <div class='q-px-sm text-center text-weight-medium' style='width: 8vw;'>
            <q-chip 
                dense 
                v-if='getMood(track.mood)' 
                :color='getMood(track.mood)!.color'  
                :outline='getMood(track.mood)!.outline'
                :label='getMood(track.mood)!.mood'
                class='text-no-wrap ellipsis'
            ></q-chip>
        </div>

        <!-- Rating -->
        <div class='q-px-sm' style='width: 120px;'>
            <q-rating 
                size='1.2em' 
                v-model='track.energy'
                no-reset
                :readonly='!selected'
            ></q-rating>
        </div>

        <!-- Genres -->
        <div class='text-grey-4 text-caption text-weight-bold text-no-wrap ellipsis' style='width: 10vw;'>
            <span v-if='selected'>
                <span 
                    v-for='(genre, i) in track.genres' 
                    :key='"gen"+i'
                    :class='{"hover-strike": selected}'
                    @click='track.toggleGenre(genre)'
                >
                    {{genre}}<span v-if='i != track.genres.length - 1'>, </span>
                </span>
            </span>
            <span v-if='!selected'>{{track.genres.join(', ')}}</span>
        </div>

        <!-- Year, BPM, Key -->
        <div class='text-no-wrap ellipsis row q-pl-sm' style='width: 132px;'>
            <div class='col-4'><div class='text-caption monospace text-grey-6 text-weight-medium'>{{track.year}}</div></div>
            <div class='col-4'><div class='text-caption monospace text-grey-4 text-weight-medium' v-if='track.bpm'>{{track.bpm}}</div></div>
            <div class='col-4'><div class='text-caption monospace text-weight-medium' :style='keyColor(track.key)'>{{track.key}}</div></div>
        </div>

        <!-- Custom -->
        <div class='col no-wrap text-no-wrap row q-px-sm'>
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
</template>

<script lang='ts' setup>
import { computed, toRef } from 'vue';
import { get1t } from '../scripts/onetagger';
import { CustomTagInfo, keyColor, QTTrack } from '../scripts/quicktag';

const $1t = get1t();
const props = defineProps({
    track: { required: true, type: QTTrack },
    odd: { required: false, type: Boolean, default: false }
});
const inputTrack = toRef(props, 'track');
// Is this track odd (highlight)
const odd = props.odd;

/// Get mood by name
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
/// Is track selected
const selected = computed(() => $1t.quickTag.value.track.isSelected(track.value));

</script>

<style lang='scss' scoped>

.qt-tile-thin {
    height: 29px;
    margin-top: 2px;
    margin-bottom: 2px;
    margin-left: 4px;
    margin-right: 4px;
}

// .qt-tile-thin-odd {
//     background-color: #303030;
// }

.qt-tile-thin:hover {
    background: #1A1A1A;
}

.qt-tile-thin-selected {
    background: #1A1A1A;
}

.selected-bar-thin {
    position: absolute;
    width: 5px;
    height: 29px;
    border-radius: 4px;
    left: 4px;
    z-index: 3;
}

</style>