<template>
<div class='q-mx-md'>
    <div class='q-mt-sm moods row'>
        <div v-for='(mood, i) in $1t.settings.quickTag.moods' :key='"mood"+i' class='q-mx-xs'>
            <q-chip 
                :outline='!(moodSelected(mood) || moodHover == i)' 
                :color='mood.color + ((moodSelected(mood) || moodHover == i) ? "-6" : "")'
                :label='mood.mood'
                @mousemove.native='moodHover = i'
                @mouseleave.native="moodHover = -1"
                @click.native='moodSelect(mood)'
                class='pointer'
            ></q-chip>
        </div>
    </div>
</div>
</template>

<script>
export default {
    name: 'QuickTagLeft',
    data() {
        return {
            moodHover: -1
        }
    },
    methods: {
        //If mood is in track data
        moodSelected(mood) {
            return this.$1t.quickTag.track && this.$1t.quickTag.track.mood == mood.mood
        },
        //Set mood
        moodSelect(mood) {
            if (!this.$1t.quickTag.track) return;
            // toggle
            if (this.$1t.quickTag.track.mood == mood.mood) this.$1t.quickTag.track.mood = null;
            else this.$1t.quickTag.track.mood = mood.mood;
        }
    },
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