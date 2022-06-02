<template>
<div class='text-left q-pt-xs' style='max-width: 550px; margin:auto;'>
    <q-list>
        <!-- Overwrite tags -->
        <AdvancedSettingsToggle 
            label='Overwrite tags'
            tooltip='Overwrite the existing tags in the song'
            v-model='$1t.config.overwrite'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='ID3v2.4'
            tooltip='Use ID3v2.4 instead of IDv2.3 for MP3/AIFF files'
            v-model='$1t.config.id3v24'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Short title'
            tooltip='Write title without version, currently supported for Beatport & Traxsource only'
            v-model='$1t.config.shortTitle'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Save album art to file'
            tooltip='Writes a cover.jpg into the folder'
            v-model='$1t.config.albumArtFile'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Merge/append genre & styles'
            tooltip='Merge the fetched genre & styles with existing ones, instead of replacing'
            v-model='$1t.config.mergeGenres'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Camelot key notation'
            tooltip='Write Camelot key 8A, 8B ... instead of musical key Am, C ...'
            v-model='$1t.config.camelot'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Track or Release ID tag as input to get exact match'
            tooltip='Valid tags are: DISCOGS_RELEASE_ID, BEATPORT_TRACK_ID'
            v-model='$1t.config.matchById'
        ></AdvancedSettingsToggle>

        <div class='row'>
            <img width='16' class='q-ml-md' src='../assets/shazam_icon.svg' svg-inline />
            <AdvancedSettingsToggle 
                label='Identify tracks with Shazam'
                tooltip='Match tracks without any tags using Shazam'
                v-model='$1t.config.enableShazam'
                style='width: calc(100% - 32px)'
            ></AdvancedSettingsToggle>
        </div>
        <AdvancedSettingsToggle 
            label='Force Shazam'
            tooltip='Indentify tracks with Shazam, even if tags are present'
            v-model='$1t.config.forceShazam'
            v-if='$1t.config.enableShazam'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Skip already tagged tracks'
            tooltip='Skip tracks with the 1T_TAGGEDDATE tag (One Tagger Tag)'
            v-model='$1t.config.skipTagged'
            v-if='!$1t.config.forceShazam'
        ></AdvancedSettingsToggle>
        
        <AdvancedSettingsToggle 
            label='Include subfolders'
            tooltip='Tag all subfolders as well'
            v-model='$1t.config.includeSubfolders'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Only write year'
            tooltip='Write just the year instead of full date'
            v-model='$1t.config.onlyYear'
        ></AdvancedSettingsToggle>

        <!-- Parse meta from filename -->
        <AdvancedSettingsToggle 
            label='Parse metadata from filename'
            style='margin-bottom: 29px;' 
            tooltip='Valid variables are: %track%, %artist%, %title%, etc. anything in between % % gets treated as dynamic content'
            v-model='$1t.config.parseFilename'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.parseFilename'>
            <q-input
                v-model='$1t.config.filenameTemplate'
                filled
                label='Filename scheme'
                class='input q-mb-xl' 
            ></q-input>
        </div>
    </q-list>

    <!-- Multiple matches ordering -->
    <q-separator class='q-mx-auto q-mb-xl custom-separator' inset color="dark"/>
    <q-select 
        dark
        standout='text-grey-4 bg-dark'
        class='row select q-ma-auto'
        v-model='$1t.config.multipleMatches' 
        :options='multipleMatches'
        label='Multiple matches ordering'
    ></q-select>    

    
    <!-- Strictness -->
    <div class='row justify-center q-mt-lg' style='max-width: 550px;'>
        <q-slider 
            v-model='$1t.config.strictness' 
            :min='0.0' 
            :max='1.0' 
            :step='0.05' 
            label 
            label-always
            class='slider q-mt-xl q-my-sm q-pb-lg col-10'
            label-text-color='black'
            :label-value='"Strictness: " + Math.round($1t.config.strictness*100) + "%"'
        >
        </q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip content-style="font-size: 13px">
                The higher, the more accurate results, but less potential matches
            </q-tooltip>
        </q-icon>
    </div>

    <!-- Threads -->
    <div class='row justify-center' style='max-width: 550px;'>
        <q-slider 
            v-model='$1t.config.threads' 
            :min='1' 
            :max='24' 
            :step='1' 
            label 
            label-always
            class='slider q-mt-xl q-my-sm q-pb-lg col-10'
            label-text-color='black'
            :label-value='"Search Threads: " + $1t.config.threads'
        ></q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip content-style="font-size: 13px">The higher, the faster, but uses more bandwidth, and not supported by all platforms</q-tooltip>
        </q-icon>
    </div>

    <!-- Duration -->
    <br>
    <div class='row justify-center items-center' style='margin-bottom: -3px;'>
        <q-toggle v-model='$1t.config.matchDuration' label='Match duration'></q-toggle>
        <div class='text-grey-6 q-ml-sm'>
            <q-icon name='mdi-alert-circle-outline' class='q-mb-xs'></q-icon>
            Warning: Strict
        </div>
    </div>
    
    <div class='row justify-center q-mt-sm' v-if='$1t.config.matchDuration'>
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

    <!-- Track number padding -->
    <q-separator class='q-mx-auto q-mt-md custom-separator' inset color="dark"/>
    <div class='text-center text-body1 text-grey-4 q-mt-md' style='margin-top: 14px;'><br>Tag options</div>
    <div class='row q-pb-xs justify-center half-width'>
    <q-input 
        v-model.number='$1t.config.trackNumberLeadingZeroes' 
        filled 
        type='number'
        label='Track number leading zeroes'
        class='input'
    ></q-input>
    </div>

    <!-- Separators -->
    <div class='text-center text-body1 text-grey-4 q-mt-lg q-mb-sm'>Separators</div>
    <div class='row q-pb-xs q-mt-sm justify-center half-width'>
        <Separators
            :initial='$1t.config.separators'
            @input='$1t.config.separators = $event'
        ></Separators>
    </div>
    

    <!-- Command -->
    <div class='text-center text-body1 text-grey-4 q-mt-lg'>
        Execute command when tagging is finished
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm q-mb-xs'>
            <q-tooltip content-style="font-size: 13px">
                $success and $failed will be substituted with paths to files containing paths of affected tracks
            </q-tooltip>
        </q-icon>
    </div>
    <q-input label='$success, $failed will be substituted' filled class='row input q-py-sm justify-center' style='max-width: 526px; margin: auto;' v-model='$1t.config.postCommand'></q-input>
    <br>
    
    <!-- Styles / genres action -->
    <q-separator class='q-mx-auto q-mt-md custom-separator' inset color="dark"/>
    <div class='text-center text-body1 text-grey-4 q-mt-lg'>
        Select Genres/Styles tag to fetch both, if it should merge them, or write elsewhere (used only in supported platforms)
        <!-- Styles -->
        <q-select
            dark
            standout='text-grey-4 bg-dark'
            v-model='stylesOption'
            :options='stylesOptions'
            class='select'
            label='Genres/Styles tag'
            style='margin-bottom: 48px;'
            @input='updateStyleOption'
        ></q-select>
        <!-- Styles custom tag -->
        <div v-if='$1t.config.stylesOptions == "customTag"'>
            <TagFields v-model='$1t.config.stylesCustomTag' class='input' style='margin-bottom: 28px;'></TagFields>
        </div>
    </div>

    <br>
</div>
</template>

<script>
import Separators from './Separators';
import AdvancedSettingsToggle from './AdvancedSettingsToggle.vue';
import TagFields from './TagFields.vue';

export default {
    name: 'AutotaggerAdvanced',
    components: {Separators, AdvancedSettingsToggle, TagFields},
    data() {
        return {
            multipleMatches: ['Default', 'Oldest', 'Newest'],
            stylesOptions: ["Default", "Only genres", "Only styles", "Merge to genres tag", 
                "Merge to styles tag", "Write styles to genre tag", "Write genres to style tag",
                "Write styles to custom tag"],
            values: ["default", "onlyGenres", "onlyStyles", "mergeToGenres", "mergeToStyles",
                "stylesToGenre", "genresToStyle", "customTag"],
            stylesOption: "Default"
        }
    },
    methods: {
        updateStyleOption() {
            this.$1t.config.stylesOptions = this.values[this.stylesOptions.indexOf(this.stylesOption)];
        },
    },
    mounted() {
        this.stylesOption = this.stylesOptions[this.values.indexOf(this.$1t.config.stylesOptions)];
        // In case of null because of update
        if (!this.$1t.config.stylesCustomTag)
            this.$1t.config.stylesCustomTag = {vorbis: 'STYLE', id3: 'STYLE', mp4: 'STYLE'};
    },
}
</script>

<style>
.slider-tooltip {
    margin-top: 36px;
}
.custom-separator {
    max-width: 550px;
    margin: auto;
}
.half-width {
    max-width: 50vw;
}
</style>

<style scoped>
.list {
    width: 36%; 
    margin-left: 32%; 
    text-align: left;
}
</style>