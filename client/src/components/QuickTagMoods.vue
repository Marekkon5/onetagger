<template>
<div class='q-mx-md'>
    <div class='q-mt-sm moods row'>
        <div v-for='(mood, i) in $1t.settings.value.quickTag.moods' :key='"mood"+i' class='q-mx-xs' @click='moodSelect(mood)'>
            <q-chip 
                :outline='!(moodSelected(mood) || moodHover == i)' 
                :color='mood.color + ((moodSelected(mood) || moodHover == i) ? "-6" : "")'
                :label='mood.mood'
                @mousemove='moodHover = i'
                @mouseleave="moodHover = -1"
                class='pointer'
            ></q-chip>
        </div>
    </div>
</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { QuickTagMood } from '../scripts/quicktag.js';

const $1t = get1t();
const moodHover = ref(-1);


// If mood is in track data
function moodSelected(mood: QuickTagMood) {
    return $1t.quickTag.value.track && $1t.quickTag.value.track.mood == mood.mood
}

// Set mood
function moodSelect(mood: QuickTagMood) {
    console.log('a');
    if (!$1t.quickTag.value.track) return;
    // toggle
    if ($1t.quickTag.value.track.mood == mood.mood) $1t.quickTag.value.track.mood = undefined;
    else $1t.quickTag.value.track.mood = mood.mood;
}

</script>

<style>
.pointer {
    cursor: pointer;    
}
.pointer:hover {
    transform: scale(1.1);
}
</style>