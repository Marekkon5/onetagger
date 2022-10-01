<template>
<div class='text-left q-pt-xs' style='max-width: 550px; margin:auto;'>
    <q-list>
        <!-- Overwrite tags -->
        <AdvancedSettingsToggle 
            label='Overwrite tags'
            tooltip='Overwrite the existing tags in the song'
            v-model='$1t.config.value.overwrite'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='ID3v2.4'
            tooltip='Use ID3v2.4 instead of IDv2.3 for MP3/AIFF files'
            v-model='$1t.config.value.id3v24'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Short title'
            tooltip='Write title without version, currently supported for Beatport & Traxsource only'
            v-model='$1t.config.value.shortTitle'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Save album art to file'
            tooltip='Writes a cover.jpg into the folder'
            v-model='$1t.config.value.albumArtFile'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Merge/append genre & styles'
            tooltip='Merge the fetched genre & styles with existing ones, instead of replacing'
            v-model='$1t.config.value.mergeGenres'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Camelot key notation'
            tooltip='Write Camelot key 8A, 8B ... instead of musical key Am, C ...'
            v-model='$1t.config.value.camelot'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Track or Release ID tag as input to get exact match'
            tooltip='Valid tags are: DISCOGS_RELEASE_ID, BEATPORT_TRACK_ID'
            v-model='$1t.config.value.matchById'
        ></AdvancedSettingsToggle>

        <div class='row'>
            <img width='16' class='q-ml-md' src='../assets/shazam_icon.svg' svg-inline />
            <AdvancedSettingsToggle 
                label='Identify tracks with Shazam'
                tooltip='Match tracks without any tags using Shazam'
                v-model='$1t.config.value.enableShazam'
                style='width: calc(100% - 32px)'
            ></AdvancedSettingsToggle>
        </div>
        <AdvancedSettingsToggle 
            label='Force Shazam'
            tooltip='Indentify tracks with Shazam, even if tags are present'
            v-model='$1t.config.value.forceShazam'
            v-if='$1t.config.value.enableShazam'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Skip already tagged tracks'
            tooltip='Skip tracks with the 1T_TAGGEDDATE tag (One Tagger Tag)'
            v-model='$1t.config.value.skipTagged'
            v-if='!$1t.config.value.forceShazam'
        ></AdvancedSettingsToggle>
        
        <AdvancedSettingsToggle 
            label='Include subfolders'
            tooltip='Tag all subfolders as well'
            v-model='$1t.config.value.includeSubfolders'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            label='Only write year'
            tooltip='Write just the year instead of full date'
            v-model='$1t.config.value.onlyYear'
        ></AdvancedSettingsToggle>

        <!-- Parse meta from filename -->
        <AdvancedSettingsToggle 
            label='Parse metadata from filename'
            tooltip='Valid variables are: %track%, %artist%, %title%, etc. anything in between % % gets treated as dynamic content'
            v-model='$1t.config.value.parseFilename'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.value.parseFilename'>
            <q-input
                v-model='$1t.config.value.filenameTemplate'
                filled
                label='Filename scheme'
                class='input q-mb-sm' 
            ></q-input>
        </div>

        <!-- Move file -->
        <AdvancedSettingsToggle
            label='Move files after tagging'
            v-model='$1t.config.value.moveFiles'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.value.moveFiles'>
            <q-input
                v-model='$1t.config.value.moveTarget'
                filled
                label='Target folder'
                class='input q-mb-sm' 
            ></q-input>
        </div>

    </q-list>

    <!-- Multiple matches ordering -->
    <q-separator class='q-mx-auto q-mb-xl custom-separator' inset color="dark"/>
    
    <q-select 
        dark
        standout='text-grey-4 bg-dark'
        class='row select q-ma-auto q-mt-xl'
        v-model='$1t.config.value.multipleMatches' 
        :options='multipleMatches'
        label='Multiple matches ordering'
    ></q-select>    

    
    <!-- Strictness -->
    <div class='row justify-center q-mt-lg' style='max-width: 550px;'>
        <q-slider 
            v-model='$1t.config.value.strictness' 
            :min='0.0' 
            :max='1.0' 
            :step='0.05' 
            label 
            label-always
            class='slider q-mt-xl q-my-sm q-pb-lg col-10'
            label-text-color='black'
            :label-value='"Strictness: " + Math.round($1t.config.value.strictness*100) + "%"'
        >
        </q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip>
                The higher, the more accurate results, but less potential matches
            </q-tooltip>
        </q-icon>
    </div>

    <!-- Threads -->
    <div class='row justify-center' style='max-width: 550px;'>
        <q-slider 
            v-model='$1t.config.value.threads' 
            :min='1' 
            :max='24' 
            :step='1' 
            label 
            label-always
            class='slider q-mt-xl q-my-sm q-pb-lg col-10'
            label-text-color='black'
            :label-value='"Search Threads: " + $1t.config.value.threads'
        ></q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip>The higher, the faster, but uses more bandwidth, and not supported by all platforms</q-tooltip>
        </q-icon>
    </div>

    <!-- Duration -->
    <br>
    <div class='row justify-center items-center' style='margin-bottom: -3px;'>
        <q-toggle v-model='$1t.config.value.matchDuration' label='Match duration'></q-toggle>
        <div class='text-grey-6 q-ml-sm'>
            <q-icon name='mdi-alert-circle-outline' class='q-mb-xs'></q-icon>
            Warning: Strict
        </div>
    </div>
    
    <div class='row justify-center q-mt-sm' v-if='$1t.config.value.matchDuration'>
        <q-slider
            v-model='$1t.config.value.maxDurationDifference'
            :min='1'
            :max='60'
            :step='1'
            label
            label-always
            class='slider q-mt-lg'
            label-text-color='black'
            :label-value='"Max difference: " + $1t.config.value.maxDurationDifference + "s"'
        ></q-slider>
    </div>
    <br>

    <!-- Track number padding -->
    <q-separator class='q-mx-auto q-mt-md custom-separator' inset color="dark"/>
    <div class='text-subtitle1 text-center text-bold text-primary q-mt-md' style='margin-top: 31px;'>TAG OPTIONS</div>
    <div class='row q-pb-xs justify-center half-width'>
    <q-input 
        v-model.number='$1t.config.value.trackNumberLeadingZeroes' 
        filled 
        type='number'
        label='Track number leading zeroes'
        class='input'
    ></q-input>
    </div>

    <!-- Separators -->
    <div class='text-subtitle1 text-center text-bold text-primary q-mt-lg q-mb-sm'>SEPARATORS</div>
    <div class='row q-pb-xs q-mt-sm justify-center half-width'>
        <Separators v-model='$1t.config.value.separators'></Separators>
    </div>
    

    <!-- Command -->
    <div class='text-subtitle1 text-center text-bold text-primary q-mt-lg'>
        EXECUTE COMMAND WHEN TAGGING IS FINISHED
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm q-mb-xs'>
            <q-tooltip>
                $success and $failed will be substituted with paths to files containing paths of affected tracks
            </q-tooltip>
        </q-icon>
    </div>
    <q-input label='$success, $failed will be substituted' filled class='row input q-py-sm justify-center' style='max-width: 526px; margin: auto;' v-model='$1t.config.value.postCommand'></q-input>
    <br>
    
    <!-- Styles / genres action -->
    <q-separator class='q-mx-auto q-mt-md custom-separator' inset color="dark"/>
    <div class='text-subtitle1 text-center text-bold text-primary q-mt-lg'>
        GENRES/STYLES OPTIONS
        <div class='text-subtitle2 q-mb-md text-grey-6'>
            Fetch both <i>(default)</i>, if it should merge them, or write elsewhere <i>(used only in supported platforms)</i>
        </div>
        
        <!-- Styles -->
        <q-select
            dark
            standout='text-grey-4 bg-dark'
            v-model='stylesOption'
            :options='stylesOptions'
            class='select'
            label='Genres/Styles tag'
            style='margin-bottom: 48px;'
            @update:model-value='updateStyleOption'
        ></q-select>
        <!-- Styles custom tag -->
        <div v-if='$1t.config.value.stylesOptions == "customTag"'>
            <TagFields v-model='$1t.config.value.stylesCustomTag' class='input' style='margin-bottom: 28px;'></TagFields>
        </div>
    </div>

    <br>
</div>
</template>

<script lang='ts' setup>
import { onMounted, ref } from 'vue';
import { get1t } from '../scripts/onetagger';
import { FrameName } from '../scripts/utils';

import Separators from './Separators.vue';
import AdvancedSettingsToggle from './AdvancedSettingsToggle.vue';
import TagFields from './TagFields.vue';

const $1t = get1t();
const multipleMatches = ['Default', 'Oldest', 'Newest'];
const stylesOptions = ["Default", "Only genres", "Only styles", "Merge to genres tag", 
    "Merge to styles tag", "Write styles to genre tag", "Write genres to style tag",
    "Write styles to custom tag"];
const values = ["default", "onlyGenres", "onlyStyles", "mergeToGenres", "mergeToStyles",
    "stylesToGenre", "genresToStyle", "customTag"];
const stylesOption = ref('Default');

function updateStyleOption() {
    $1t.config.value.stylesOptions = values[stylesOptions.indexOf(stylesOption.value)];
}

onMounted(() => {
    stylesOption.value = stylesOptions[values.indexOf($1t.config.value.stylesOptions)];
    // In case of null because of update
    if (!$1t.config.value.stylesCustomTag)
        $1t.config.value.stylesCustomTag = FrameName.same('STYLE');
});
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