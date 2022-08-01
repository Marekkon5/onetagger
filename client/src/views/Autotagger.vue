<template>
<div>

    <q-stepper 
        v-model='step' 
        header-nav 
        color='primary' 
        animated 
        alternative-labels
        flat 
        class='bg-dark-page'
        v-if='!$1t.settings.value.autoTaggerSinglePage'>

        <!-- Platforms -->
        <q-step 
            :name='0' 
            title='Select Platforms' 
            :done='step > 0 && $1t.config.value.platforms.length > 0'
            icon='mdi-web'
            :error='step > 0 && $1t.config.value.platforms.length == 0'
            class='text-center step'>

            <div class='text-h5 text-grey-4'>Select platforms</div>
            <div class='text-subtitle2 text-grey-6'>Check the box to fetch tags from stated platform, drag & drop to reorder fallback</div>
            <AutotaggerPlatforms class='q-mb-xl'></AutotaggerPlatforms>
        </q-step>

        <!-- Tags -->
        <q-step
            :name='1'
            title='Input & Tags'
            :done='canStart && step > 1'
            icon='mdi-label-multiple'
            :error='!canStart && step > 1'
            class='text-center step'>

            <AutotaggerTags class='q-px-xl q-mx-xl q-mb-xl'></AutotaggerTags>
        </q-step>

        <!-- Platform Specific -->
        <q-step
            :name='2'
            title='Platform Specific Settings'
            :done='step > 2'
            icon='mdi-tune'
            class='text-center step'>

            <AutotaggerPlatformSpecific class='q-mb-xl'></AutotaggerPlatformSpecific>
        </q-step>

        <!-- Advanced -->
        <q-step
            :name='3'
            title='Advanced'
            :done='step > 3'
            icon='mdi-cog'
            class='text-center step'>

            <div class='text-h5 text-grey-4'>Advanced</div>
            <span class='text-subtitle2 text-grey-6'>Miscellaneous options</span>
            <br>
            <AutotaggerAdvanced class='q-mt-xs q-mb-xl'></AutotaggerAdvanced>
        </q-step>

    </q-stepper>

    <!-- Stepper bar -->
    <div class='at-stepper-bar row justify-center content-center' v-if='!$1t.settings.value.autoTaggerSinglePage'>
        <div>
            <q-btn push color='primary' class='text-black' @click='step += 1' v-if='step < 3'>
                Next
            </q-btn>
        </div>
    </div>

    <!-- Single page -->
    <div v-if='$1t.settings.value.autoTaggerSinglePage' class='text-center'>
        <div class='row q-mx-xl'>
            <div class='col q-px-xl'>
                <AutotaggerTags class='q-mt-md'></AutotaggerTags>
                <AutotaggerAdvanced class='q-mt-md'></AutotaggerAdvanced>
            </div>
            <div class='col q-px-xl'>
                <div class='text-h5 q-mt-md text-grey-4'>Select platforms</div>
                <div class='text-subtitle2 text-grey-6'>Check the box to fetch tags from stated platform, drag & drop to reorder fallback</div>
                <AutotaggerPlatforms dense></AutotaggerPlatforms>
                <AutotaggerPlatformSpecific></AutotaggerPlatformSpecific>
            </div>
        </div>
        
    </div>

    <!-- Start FAB -->
    <q-page-sticky position='bottom-right' :offset='[36, 24]'>
        <q-btn 
            fab 
            push
            icon='mdi-play' 
            color='primary'
            :disable='!canStart'
            @click='startTagging'
        >
            <q-tooltip anchor="top middle" self="bottom middle" :offset="[10, 10]">            
                <span class='text-weight-bold'>START</span>
            </q-tooltip>
        </q-btn>
    </q-page-sticky>

</div>
</template>

<script lang='ts' setup>
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { get1t } from '../scripts/onetagger';

import AutotaggerPlatforms from '../components/AutotaggerPlatforms.vue';
import AutotaggerTags from '../components/AutotaggerTags.vue';
import AutotaggerPlatformSpecific from '../components/AutotaggerPlatformSpecific.vue';
import AutotaggerAdvanced from '../components/AutotaggerAdvanced.vue';

const $1t = get1t();
const $router = useRouter();
const step = ref(0);

function startTagging() {
    // Merge custom fields
    let custom: any = {};
    for (let platform of $1t.info.value.platforms) {
        if (platform.platform.customOptions.options.length > 0) {
            custom[platform.platform.id] = {}
            for (let option of platform.platform.customOptions.options) {
                custom[platform.platform.id][option.id] = option.value;
            }
        }
    }
    $1t.config.value.custom = custom;

    // Save settings
    $1t.saveSettings();
    $1t.config.value.type = 'autoTagger';

    // Tag playlist rather than folder
    let playlist = null;
    if ($1t.autoTaggerPlaylist.value && $1t.autoTaggerPlaylist.value.data)
        playlist = $1t.autoTaggerPlaylist.value;

    // Spotify auth
    if ($1t.settings.value.audioFeatures.spotifyClientId && $1t.settings.value.audioFeatures.spotifyClientSecret) {
        $1t.config.value.spotify = {
            clientId: $1t.settings.value.audioFeatures.spotifyClientId,
            clientSecret: $1t.settings.value.audioFeatures.spotifyClientSecret
        }
    } else {
        $1t.config.value.spotify = undefined;
    }

    // Start
    $1t.send('startTagging', {
        config: $1t.config.value,
        playlist
    });
    $router.push('/autotagger/status');
}

const canStart = computed(() => (($1t.config.value.path || ($1t.autoTaggerPlaylist.value && $1t.autoTaggerPlaylist.value.data)) 
    && $1t.config.value.platforms.length > 0) ? true : false);

</script>


<style lang='scss'>
.step {
    min-height: calc(100vh - 164px);
    max-height: calc(100vh - 164px);
    background: #1a1c1b;
}
.q-stepper__step-inner {
    background: #1a1c1b;
}

.input {
    max-width: 526px;
    margin: auto;
    margin-top: 8px;
    padding-left: 16px;
    padding-right: 16px;
}

.select {
    max-width: 526px;
    margin: auto;
    margin-top: 8px;
    padding-left: 16px;
    padding-right: 16px;
}

.slider {
    max-width: 550px !important;
}

.at-stepper-bar {
    width: 100%;
    position: absolute;
    height: 64px;
    bottom: 0%;
    background-color: var(--q-color-accent);
}
</style>