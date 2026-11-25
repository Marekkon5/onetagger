<template>
<div class='text-left q-pt-xs' style='max-width: 450px; margin:auto;'>
    <q-list>
        <!-- Overwrite tags -->
        <AdvancedSettingsToggle 
            :label="t('autotag.overwriteTags')"
            :tooltip="t('autotag.overwriteHint')"
            v-model='$1t.config.value.overwrite'
        ></AdvancedSettingsToggle>

        <!-- Overwrite select -->
        <div v-if='!$1t.config.value.overwrite' class='q-pb-md'>
            <q-select
                dark
                standout='text-grey-4 bg-dark'
                class='row select q-ma-auto q-mt-xl'
                :model-value='SUPPORTED_TAGS.filter(t => $1t.config.value.overwriteTags.includes(t.tag))'
                @update:model-value='(v) => $1t.config.value.overwriteTags = v.map((v: any)=> v.tag)'
                :options='SUPPORTED_TAGS'
                use-chips
                multiple
                :label="t('autotag.selectOverwriteTags')"
            >
                <template v-slot:selected-item='scope'>
                    <q-chip 
                        color='primary' 
                        :label='scope.opt.label' 
                        class='text-black'
                        removable
                        @remove='scope.removeAtIndex(scope.index)'
                    ></q-chip>
                </template>
            </q-select>
        </div>

        <AdvancedSettingsToggle 
            label='ID3v2.4'
            :tooltip="t('autotag.id3VersionHint')"
            v-model='$1t.config.value.id3v24'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.shortTitle')"
            :tooltip="t('autotag.shortTitleHint')"
            v-model='$1t.config.value.shortTitle'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.saveCover')"
            :tooltip="t('autotag.saveCoverHint')"
            v-model='$1t.config.value.albumArtFile'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.mergeGenres')"
            :tooltip="t('autotag.mergeGenresHint')"
            v-model='$1t.config.value.mergeGenres'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.camelot')"
            :tooltip="t('autotag.camelotHint')"
            v-model='$1t.config.value.camelot'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.matchId')"
            :tooltip="t('autotag.matchIdHint')"
            v-model='$1t.config.value.matchById'
        ></AdvancedSettingsToggle>

        <div class='row'>
            <img width='16' class='q-ml-md' src='../assets/shazam_icon.svg' svg-inline />
            <AdvancedSettingsToggle 
                :label="t('autotag.shazam')"
                :tooltip="t('autotag.shazamHint')"
                v-model='$1t.config.value.enableShazam'
                style='width: calc(100% - 32px)'
            ></AdvancedSettingsToggle>
        </div>
        <AdvancedSettingsToggle 
            :label="t('autotag.forceShazam')"
            :tooltip="t('autotag.forceShazamHint')"
            v-model='$1t.config.value.forceShazam'
            v-if='$1t.config.value.enableShazam'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.skipTagged')"
            :tooltip="t('autotag.skipTaggedHint')"
            v-model='$1t.config.value.skipTagged'
        ></AdvancedSettingsToggle>
        
        <AdvancedSettingsToggle 
            :label="t('autotag.subfolders')"
            :tooltip="t('autotag.subfoldersHint')"
            v-model='$1t.config.value.includeSubfolders'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.yearOnly')"
            :tooltip="t('autotag.yearOnlyHint')"
            v-model='$1t.config.value.onlyYear'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.multiplatform')"
            :tooltip="t('autotag.multiplatformHint')"
            v-model='$1t.config.value.multiplatform'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle v-if="!$1t.config.value.albumTagging"
            :label="t('autotag.rotateProviders')" :tooltip="t('autotag.rotateHint')"
            v-model="$1t.settings.value.rotateProviders"
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.removeCovers')"
            :tooltip="t('autotag.removeCoversHint')"
            v-model='$1t.config.value.removeAllCovers'
        ></AdvancedSettingsToggle>

        <!-- Parse meta from filename -->
        <AdvancedSettingsToggle 
            :label="t('autotag.parseFilename')"
            :tooltip="t('autotag.parseFilenameHint')"
            v-model='$1t.config.value.parseFilename'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.value.parseFilename'>
            <q-input
                v-model='$1t.config.value.filenameTemplate'
                filled
                :label="t('autotag.filenameScheme')"
                class='input q-mb-sm' 
            ></q-input>
        </div>

        <!-- Move files -->
        <AdvancedSettingsToggle
            :label="t('autotag.moveSuccessful')"
            v-model='$1t.config.value.moveSuccess'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.value.moveSuccess'>
            <q-input
                v-model='$1t.config.value.moveSuccessPath'
                filled
                :label="t('autotag.targetFolder')"
                class='input q-mb-sm' 
            ></q-input>
        </div>
        <AdvancedSettingsToggle
            :label="t('autotag.moveFailed')"
            v-model='$1t.config.value.moveFailed'
        ></AdvancedSettingsToggle>
        <div v-if='$1t.config.value.moveFailed'>
            <q-input
                v-model='$1t.config.value.moveFailedPath'
                filled
                :label="t('autotag.targetFolder')"
                class='input q-mb-sm' 
            ></q-input>
        </div>

        <AdvancedSettingsToggle 
            :label="t('autotag.writeLrc')"
            :tooltip="t('autotag.writeLrcHint')"
            v-model='$1t.config.value.writeLrc'
        ></AdvancedSettingsToggle>
        <AdvancedSettingsToggle 
            :label="t('autotag.enhancedLrc')"
            :tooltip="t('autotag.enhancedLrcHint')"
            v-if='$1t.config.value.writeLrc'
            v-model='$1t.config.value.enhancedLrc'
        ></AdvancedSettingsToggle>

        <AdvancedSettingsToggle 
            :label="t('autotag.capitalizeGenres')"
            :tooltip="t('autotag.capitalizeGenresHint')"
            v-model='$1t.config.value.capitalizeGenres'
        ></AdvancedSettingsToggle>

        <!-- Album Tagging -->
        <AdvancedSettingsToggle
            :label="t('autotag.albumTagging')"
            :tooltip="t('autotag.albumTaggingHint')"
            v-model='$1t.config.value.albumTagging'
        ></AdvancedSettingsToggle>
        <div class='row justify-center items-center q-px-md' v-if='$1t.config.value.albumTagging'>
            <div class='q-pb-lg'>{{ t('autotag.albumRatio') }} </div>
            <q-slider
                class='q-mt-sm'
                :min='0.0'
                :max='1.0'
                :step='0.02'
                v-model='$1t.config.value.albumTaggingRatio'
                label
                label-always
                :label-value="`${Math.round($1t.config.value.albumTaggingRatio  * 100)}%`"
            ></q-slider>
        </div>

        <!-- Duration -->   
        <div class='row justify-center items-center q-px-md'>
            <div>{{ t('autotag.matchDuration') }}</div>
            <div class='text-grey-6 q-ml-sm'>
                <q-icon name='mdi-alert-circle-outline' class='q-mb-xs'></q-icon>
                {{ t('autotag.strictWarning') }}
            </div>
            <q-space></q-space>
            <q-toggle v-model='$1t.config.value.matchDuration'></q-toggle>
        </div>

    </q-list>

    <!-- Styles / genres action -->
    <q-separator class='q-mx-auto q-mt-md custom-sep-advanced' :style='"margin-top: 21px;"' inset color="dark"/>
    <div class='text-subtitle2 text-center text-bold text-primary q-mt-lg' :style='"margin-top: 35px;"'>
        {{ t('autotag.genreStyleHeading') }}
        <div class='text-subtitle2 q-mb-md text-grey-6'>
            {{ t('autotag.genreStyleHint') }}
        </div>        
        
        <!-- Styles -->
        <q-select
            dark
            standout='text-grey-4 bg-dark'
            v-model='$1t.config.value.stylesOptions'
            :options='stylesOptions'
            emit-value
            map-options
            class='select'
            :label="t('autotag.genreStyleTag')"
            style='margin-bottom: 40px;'
            popup-content-class='no-shadow'
        ></q-select>
        <!-- Styles custom tag -->
        <div v-if='$1t.config.value.stylesOptions == "customTag"'>
            <TagFields v-model='$1t.config.value.stylesCustomTag' class='q-mb-md'></TagFields>
        </div>
    </div>


    <!-- Multiple matches ordering -->
    <q-separator class='q-mx-auto q-mb-xl custom-sep-advanced' :style='"margin-top: 21px;"' inset color="dark"/>
    
    <q-select 
        dark
        standout='text-grey-4 bg-dark'
        class='row select q-ma-auto' :style='"margin-top: -8px;"'
        v-model='$1t.config.value.multipleMatches' 
        :options='multipleMatches'
        emit-value
        map-options
        :label="t('autotag.multipleMatches')"
        popup-content-class='no-shadow'
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
            :label-value="t('autotag.strictnessValue', { percent: Math.round($1t.config.value.strictness * 100) })"
        >
        </q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip>
                {{ t('autotag.strictnessHint') }}
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
            :label-value="t('autotag.threadsValue', { count: $1t.config.value.threads })"
        ></q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip>{{ t('autotag.threadsHint') }}</q-tooltip>
        </q-icon>
    </div>

    <div class='row justify-center' style='max-width: 550px;'>
        <q-slider
            v-model='$1t.config.value.providerSlowdown'
            :min='1' :max='10' :step='1'
            label label-always
            class='slider q-mt-xl q-my-sm q-pb-lg col-10'
            label-text-color='black'
            :label-value="t('autotag.providerSlowdownValue', { factor: $1t.config.value.providerSlowdown })"
            data-testid='provider-slowdown'
        ></q-slider>
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-pt-md q-mx-sm slider-tooltip'>
            <q-tooltip>{{ t('autotag.providerSlowdownHint') }}</q-tooltip>
        </q-icon>
    </div>

    <!-- Title cleanup regex -->
    <q-input
        v-model='$1t.config.value.titleRegex'
        filled 
        :label="t('autotag.titleRegex')"
        class='input'
    ></q-input>

    <!-- Cover filename -->
    <q-input
        v-model='$1t.config.value.coverFilename' 
        filled 
        :label="t('autotag.coverFilename')"
        class='input monospace'
    ></q-input>

    
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
            :label-value="t('autotag.maxDifference', { seconds: $1t.config.value.maxDurationDifference })"
        ></q-slider>
    </div>
    <br>

    

    <!-- Separators -->
    <q-separator class='q-mx-auto q-mt-md custom-sep-advanced' :style='"margin-top: 8px;"' inset color="dark"/>
    <div class='text-subtitle2 text-center text-bold text-primary q-mt-lg q-mb-sm' :style='"margin-top: 35px;"'>{{ t('autotag.separators') }}</div>
    <div class='row q-pb-md q-mt-sm justify-center half-width'>
        <Separators v-model='$1t.config.value.separators'></Separators>
    </div>

    <!-- Track number padding -->
    
    <div class='text-subtitle2 text-center text-bold text-primary' style='margin-top: 3px;'>{{ t('autotag.tagOptions') }}</div>
    <div class='row q-pb-xs justify-center half-width'>
        <q-input 
            v-model.number='$1t.config.value.trackNumberLeadingZeroes' 
            filled 
            type='number'
            :label="t('autotag.leadingZeroes')"
            class='input'
        ></q-input>
    </div>

    <!-- ID3 Lang -->
    <div class='row q-pb-xs justify-center half-width'>
        <q-input :labelStyle="{ fontSize: '10px' }"
            v-model='$1t.config.value.id3CommLang' 
            filled 
            :label="t('autotag.id3Language')"            
            class='input'
            :rules="[val => !val || val.length == 3]"
        ></q-input>
    </div>
    
    <q-separator class='q-mx-auto q-mb-lg custom-sep-advanced' style='margin-top: 16px;' inset color="dark"/>

    <!-- Command -->
    <div class='text-subtitle2 text-center text-bold text-primary q-mt-lg' style='margin-top: 35px;'>
        {{ t('autotag.postCommand') }}
        <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm q-mb-xs'>
            <q-tooltip>
                {{ t('autotag.postCommandHint') }}
            </q-tooltip>
        </q-icon>
    </div>
    <q-input :label="t('autotag.postCommandInput')" filled class='row input q-py-sm justify-center' style='max-width: 526px; margin: auto;' v-model='$1t.config.value.postCommand'></q-input>
    <br>

    <br>
</div>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
import { computed, onMounted } from 'vue';
import { get1t } from '../scripts/onetagger';
import { FrameName } from '../scripts/utils';

import Separators from './Separators.vue';
import AdvancedSettingsToggle from './AdvancedSettingsToggle.vue';
import TagFields from './TagFields.vue';
import { SUPPORTED_TAGS, SupportedTag } from '../scripts/autotagger';

const $1t = get1t();
// Labels can change language without changing the Rust enum/config values.
const multipleMatches = computed(() => ['Default', 'Oldest', 'Newest'].map(value => ({
    value, label: t(`autotag.${value.toLowerCase()}`)
})));
const values = ["default", "onlyGenres", "onlyStyles", "mergeToGenres", "mergeToStyles",
    "stylesToGenre", "genresToStyle", "customTag"];
const stylesOptions = computed(() => values.map(value => ({ value, label: t(`autotag.${value}`) })));

onMounted(() => {
    // In case of null because of update
    if (!$1t.config.value.stylesCustomTag)
        $1t.config.value.stylesCustomTag = FrameName.same('STYLE');
});
</script>

<style>
.slider-tooltip {
    margin-top: 36px;
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

.custom-sep-advanced {
    min-width: 600px;
    margin-left: -75px;    
}
</style>
