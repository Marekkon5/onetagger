<template>
<div class='q-ma-md'>

    <!-- Energy -->
    <div class='text-subtitle1 text-bold'>Energy</div>
    <q-rating 
        size='1.5em' 
        class='q-mt-sm' 
        v-if='$1t.quickTag.track'
        v-model='$1t.quickTag.track.energy'
    ></q-rating>
    <q-rating 
        size='1.5em' 
        class='q-mt-sm' 
        :value='0' 
        v-if='!$1t.quickTag.track'
    ></q-rating>

    <!-- Mood -->
    <div class='text-subtitle1 text-bold q-mt-md'>Mood</div>
    <div class='q-mt-sm moods'>
        <div v-for='(mood, i) in $1t.settings.quickTag.moods' :key='"mood"+i'>
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
            this.$1t.quickTag.track.mood = mood.mood;
        }
    },
}
</script>

<style>
.moods {
    margin-left: -4px;
}
.pointer {
    cursor: pointer;    
}
.pointer:hover {
    transform: scale(1.1);
}
</style>