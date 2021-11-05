<template>
<div class='text-center'>
    <q-list class='list'>

        <!-- Overwrite tags -->
        <AdvancedSettingsToggle 
            label='Overwrite tags'
            tooltip='Overwrite the existing tags in the song'
            v-model='$1t.config.overwrite'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='ID3v2.4'
            tooltip='Use ID3 version 2.4 rather than version 2.3 for MP3/AIFF files'
            v-model='$1t.config.id3v24'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Short title'
            tooltip='Write title without version, currently supported only for Beatport and Traxsource'
            v-model='$1t.config.shortTitle'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Save album art to file'
            v-model='$1t.config.albumArtFile'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Merge/append genres and styles'
            tooltip='Merge the new genres and styles with existing ones, rather than replacing'
            v-model='$1t.config.mergeGenres'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Use Camelot key notation'
            v-model='$1t.config.camelot'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Use Track or Release ID tag to get exact match'
            tooltip='Valid tags are: DISCOGS_RELEASE_ID, BEATPORT_TRACK_ID'
            v-model='$1t.config.matchById'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Enable Shazam'
            tooltip='Match tracks without any tags on Shazam'
            v-model='$1t.config.enableShazam'
        ></AdvancedSettingsToggle>

        <!-- Parse meta from filename -->
        <AdvancedSettingsToggle 
            label='Parse metadata from filename'
            v-model='$1t.config.parseFilename'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.parseFilename'>
            <q-input
                v-model='$1t.config.filenameTemplate'
                filled
                label='Filename template (variables: %title%, %artists%, anything between %% gets treated as dynamic content)'
                class='input'
            ></q-input>
        </div>
    </q-list>

    <!-- Multiple matches ordering -->
    <q-select 
        dense 
        filled 
        class='input q-my-sm q-mt-md' 
        v-model='$1t.config.multipleMatches' 
        :options='multipleMatches'
        label='Multiple matches ordering'
    ></q-select>    

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
    <div class='row justify-center items-center'>
        <q-toggle v-model='$1t.config.matchDuration' label='Match duration'></q-toggle>
        <div class='text-grey-6 q-ml-sm'>
            <q-icon name='mdi-alert-circle-outline' class='q-mb-xs'></q-icon>
            Warning: Strict
        </div>
    </div>
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
    <div class='text-body1 text-grey-4'>Separators</div>
    <div class='row q-py-md justify-center' style='max-width: 50vw; margin: auto;'>
        <Separators
            :initial='$1t.config.separators'
            @input='$1t.config.separators = $event'
        ></Separators>
    </div>

    <!-- Track number padding -->
    <div class='text-body1 text-grey-4'>Tag options</div>
    <q-input 
        v-model.number='$1t.config.trackNumberLeadingZeroes' 
        filled 
        type='number'
        label='Track number leading zeroes'
        class='input'
    ></q-input>

    <!-- Command -->
    <div class='text-body1 text-grey-4 q-mt-md'>Execute command on finished tagging</div>
    <q-input label='$success, $failed will be substituted' filled class='input q-mt-sm' v-model='$1t.config.postCommand'></q-input>
</div>
</template>

<script>
import Separators from './Separators';
import AdvancedSettingsToggle from './AdvancedSettingsToggle.vue';

export default {
    name: 'AutotaggerAdvanced',
    components: {Separators, AdvancedSettingsToggle},
    data() {
        return {
            multipleMatches: ['Default', 'Oldest', 'Newest']
        }
    }
}
</script>

<style>
.slider-tooltip {
    margin-top: 36px;
}
</style>

<style scoped>
.list {
    width: 36%; 
    margin-left: 32%; 
    text-align: left;
}
</style>