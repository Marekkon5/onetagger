<template>
<div class='q-mx-sm'>
    <div class='q-mt-sm moods row text-center text-subtitle2 text-weight-medium'>
        <div v-for='(mood, i) in $1t.settings.value.quickTag.moods' :key='"mood"+i' class='q-mx-xs' @click='moodSelect(mood)'>
            <q-chip 
                :outline='!(moodSelected(mood) || moodHover == i)' 
                :color='mood.color'
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
    return $1t.quickTag.value.track.mood == mood.mood;
}

// Set mood
function moodSelect(mood: QuickTagMood) {
    if ($1t.quickTag.value.track.mood == mood.mood) $1t.quickTag.value.track.mood = undefined;
    else $1t.quickTag.value.track.mood = mood.mood;
}

</script>

<style>
.pointer {
    cursor: pointer;    
}
</style>