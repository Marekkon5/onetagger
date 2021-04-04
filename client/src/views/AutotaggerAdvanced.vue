<template>
<div class='text-center'>

    <div class='text-h5 q-my-md'>Advanced</div>

    <q-toggle v-model='$1t.config.overwrite' label='Overwrite tags'>
        <q-icon name='mdi-help-circle-outline' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 12px">Overwrite the existing tags in the song.</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <q-toggle v-model='$1t.config.id3v24' label='ID3v2.4'>
        <q-icon name='mdi-help-circle-outline' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 12px">Use ID3 version 2.4 rather than version 2.3 for MP3/AIFF files.</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <q-toggle v-model='$1t.config.albumArtFile' label='Save album art to file'></q-toggle>
    <br>
    <q-toggle v-model='$1t.config.mergeGenres' label='Merge/append genres and styles'>
        <q-icon name='mdi-help-circle-outline' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 12px">Merge the new genres and styles with existing ones, rather than replacing.</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <!-- Strictness -->
    <div class='row'>
        <q-slider 
            v-model='$1t.config.strictness' 
            :min='0.0' 
            :max='1.0' 
            :step='0.05' 
            label 
            label-always
            class='slider q-mt-lg'
            label-text-color='black'
            :label-value='"Strictness: " + Math.round($1t.config.strictness*100) + "%"'
        >
        </q-slider>
        <q-icon name='mdi-help-circle-outline' class='q-mx-sm slider-tooltip'>
            <q-tooltip content-style="font-size: 12px">
                The higher, the more accurate results, but less potential matches.
            </q-tooltip>
        </q-icon>
    </div>
    <!-- Threads -->
    <div class='row'>
        <q-slider 
            v-model='$1t.config.threads' 
            :min='1' 
            :max='24' 
            :step='1' 
            label 
            label-always
            class='slider q-mt-lg'
            label-text-color='black'
            :label-value='"Threads: " + $1t.config.threads'
        ></q-slider>
        <q-icon name='mdi-help-circle-outline' class='q-mx-sm slider-tooltip'>
            <q-tooltip content-style="font-size: 12px">The higher, the faster, but uses more bandwith, and not supported by all platforms.</q-tooltip>
        </q-icon>
    </div>
    <!-- Start tagging -->
    <q-btn class='q-mt-xl text-black' color='primary' @click='start'>Start</q-btn>


</div>
</template>

<script>
export default {
    name: 'AutotaggerAdvanced',
    methods: {
        //Start tagging
        start() {
            this.$1t.saveSettings();
            this.$1t.startTagging();
            this.$router.push('/autotagger/status');
        }
    }
}
</script>

<style>
.slider-tooltip {
    margin-top: 36px;
}
</style>