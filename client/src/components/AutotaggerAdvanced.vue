<template>
<div class='text-center'>

    <q-toggle v-model='$1t.config.overwrite' label='Overwrite tags'>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 13px">Overwrite the existing tags in the song</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <q-toggle v-model='$1t.config.id3v24' label='ID3v2.4'>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 13px">Use ID3 version 2.4 rather than version 2.3 for MP3/AIFF files</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <q-toggle v-model='$1t.config.shortTitle' label='Short title'>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 13px">Write title without version, currently supported only for Beatport and Traxsource</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <q-toggle v-model='$1t.config.albumArtFile' label='Save album art to file'></q-toggle>
    <br>
    <q-toggle v-model='$1t.config.mergeGenres' label='Merge/append genres and styles'>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm'>
            <q-tooltip content-style="font-size: 13px">Merge the new genres and styles with existing ones, rather than replacing</q-tooltip>
        </q-icon>
    </q-toggle>
    <br>
    <q-toggle v-model='$1t.config.camelot' label='Use Camelot key notation'></q-toggle>
    <br>

    <!-- Parse meta from filename -->
    <q-toggle v-model='$1t.config.parseFilename' label='Parse metadata from filename'></q-toggle>
    <div v-if='$1t.config.parseFilename'>
        <q-input
            v-model='$1t.config.filenameTemplate'
            filled
            label='Filename template (variables: %title%, %artists%, anything between %% gets treated as dynamic content)'
            class='input'
        ></q-input>
    </div>

    <br>
    <!-- Strictness -->
    <div class='row justify-center'>
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
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm slider-tooltip'>
            <q-tooltip content-style="font-size: 13px">
                The higher, the more accurate results, but less potential matches
            </q-tooltip>
        </q-icon>
    </div>
    <!-- Threads -->
    <div class='row justify-center'>
        <q-slider 
            v-model='$1t.config.threads' 
            :min='1' 
            :max='24' 
            :step='1' 
            label 
            label-always
            class='slider q-mt-lg'
            label-text-color='black'
            :label-value='"Search Threads: " + $1t.config.threads'
        ></q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='slider-tooltip q-mx-sm'>
            <q-tooltip content-style="font-size: 13px">The higher, the faster, but uses more bandwidth, and not supported by all platforms</q-tooltip>
        </q-icon>
    </div>

    <!-- Duration -->
    <br>
    <q-toggle v-model='$1t.config.matchDuration' label='Match duration (WARNING: strict)'></q-toggle>
    <br>
    <div class='row justify-center' v-if='$1t.config.matchDuration'>
        <q-slider
            v-model='$1t.config.maxDurationDifference'
            :min='1'
            :max='60'
            :step='1'
            label
            label-always
            class='slider q-mt-lg'
            label-text-color='black'
            :label-value='"Max difference: " + $1t.config.maxDurationDifference + "s"'
        ></q-slider>
    </div>
    <br>

    <!-- Separators -->
    <div class='q-mt-md text-body1 text-grey-4'>Separators</div>
    <div class='row q-py-md justify-center' style='max-width: 50vw; margin: auto;'>
        <Separators
            :initial='$1t.config.separators'
            @input='$1t.config.separators = $event'
        ></Separators>
    </div>
</div>
</template>

<script>
import Separators from './Separators';

export default {
    name: 'AutotaggerAdvanced',
    components: {Separators},
}
</script>

<style>
.slider-tooltip {
    margin-top: 36px;
}
</style>