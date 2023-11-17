<template>
<div class='text-center af-wrapper'>

    <!-- Login -->
    <div v-if='!$1t.spotify.value.authorized' class='af-content'>
        <div class='text-subtitle2 text-bold text-primary q-mt-lg'>SETUP</div>
        <SpotifyLogin></SpotifyLogin>
        <!-- Description -->
        <div class='q-mt-xl text-subtitle2 text-grey-6' style='line-height: 24px'>
            Automatically tag your local audio files, with so called audio features by Spotify, based on <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>ISRC</span></q-badge> tag or exact match.<br>
            More info?<span class='q-px-sm text-caption text-bold click-highlight'>CLICK</span> 
            <span class="text-weight-bold text-caption q-pl-xs">
                <q-icon style='padding-bottom: 3px;' name='mdi-help-circle-outline'></q-icon> 
                HELP
            </span> 
            on the right <br>            
        </div>
    </div>

    <!-- Logged in -->
    <div v-if='$1t.spotify.value.authorized' class='af-content'>
        <!-- Path -->
        <div class='text-subtitle2 text-bold text-primary q-mt-lg'>SELECT INPUT</div>
        <div class='text-subtitle2 q-mb-md text-grey-6'>
            Drag & drop folder, copy/paste path directly or<span class='q-px-sm text-caption text-bold click-highlight'>CLICK</span>  
            the 
            <q-icon name='mdi-open-in-app'></q-icon> 
            icon to browse
        </div>
        <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
            <div class='col-1'></div>
            <q-input filled class='col-10' label='Path' v-model='config.path'>
                <template v-slot:append>
                    <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse'></q-btn>
                </template>
            </q-input>

            <div class='col-1'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='path-tooltip q-mx-sm q-pt-md q-mt-xs'>
                    <q-tooltip>Subfolders are included</q-tooltip>
                </q-icon>
            </div>
        </div>
            
        <!-- Drag and drop -->
        <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
            <div class='col-1'></div>
            <PlaylistDropZone 
                v-model='playlist'
                class='q-my-sm q-pt-md q-pb-md col-10'                 
                ></PlaylistDropZone>            
                    
            <div class='col-1'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='playlist-tooltip q-mx-sm q-mt-xl q-pt-sm'>
                    <q-tooltip>.m3u and .m3u8 extensions are supported</q-tooltip>
                </q-icon>
            </div>
        </div>

        <!-- Main tag -->
        <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 16px; margin-bottom: 35px"' inset color="dark"/>
        <div class='text-subtitle2 text-bold text-primary custom-margin'>PROMINENT TAG</div>
        <div class='text-subtitle2 text-grey-6'>Converts most prominent audio features value 0-100 to a description - based on threshold - and writes to selected tag frame</div>
        <div class='text-subtitle2 q-mt-xs q-mb-md text-grey-4'>e.g. #dance-high, #energy-med, #vocal-low, #positive, #popular</div>

        <TagFields style='max-width: 550px; margin: auto;' v-model='config.mainTag'></TagFields>

        <!-- Values -->
        <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 20px; margin-bottom: 35px"' inset color="dark"/>
        <div class='text-subtitle2 text-bold text-primary custom-margin'>PROPERTIES</div>
        <div class='q-px-xl'>
            <!-- Header -->
            <div class='row text-subtitle3 text-weight-medium q-mb-md text-grey-6'>
                <div class='col-1'>Include
                    <q-icon name='mdi-help-circle-outline' class='q-ml-xs q-mb-xs'>
                        <q-tooltip>
                            Include the audio feature in Prominent tag
                        </q-tooltip>
                    </q-icon>
                </div>
                <div class='col-2'>Audio feature</div>
                <div class='col-6'>Tag frame</div>
                <div class='col-3'>Threshold</div>
            </div>

            <div v-for='(_, key, i) in config.properties' :key='"P"+i'>
                <div class='row'>
                    <!-- Enabled -->
                    <div class='col-1'>
                        <q-checkbox v-model='config.properties[key].enabled' class='checkbox'></q-checkbox>
                    </div>
                    <!-- Title -->
                    <div class='col-2'>
                        <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{key}}</span></q-badge>
                    </div>
                    <!-- Tags -->
                    <div class='col-6'>
                        <TagFields dense v-model='config.properties[key].tag'></TagFields>
                    </div>
                    <!-- Range -->
                    <div class='col-3 q-px-md'>
                        <q-range 
                            label 
                            :min='0' 
                            :max='100' 
                            v-model='config.properties[key].range'
                            class='t-range'
                            color='grey-8'
                        ></q-range>
                    </div>
                </div>
            </div>
        </div>

        <!-- Separators -->
        <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 20px; margin-bottom: 35px"' inset color="dark"/>
        <div class='text-subtitle2 text-bold text-primary custom-margin'>SEPARATORS</div>
        <div class='row q-pb-md q-mt-sm justify-center'>            
            <Separators v-model='config.separators'></Separators>
        </div>

        <!-- Advanced -->
        <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 8px; margin-bottom: 35px"' inset color="dark"/>
        <div class='text-subtitle2 text-bold text-primary custom-margin' style='margin-bottom: 8px;'>OPTIONS</div>

        <div class='column flex-center'>
            <q-toggle class='justify-between' style='width: 200px;' label='Write OneTagger meta tag' left-label v-model='config.metaTag'></q-toggle>
            <q-toggle class='justify-between' style='width: 200px;' label='Skip already tagged tracks' left-label v-model='config.skipTagged'></q-toggle>
            <q-toggle class='justify-between' style='width: 200px;' label='Include subfolders' left-label v-model='config.includeSubfolders'></q-toggle>
        </div>
        

        <div class='q-my-xl'></div>

        <!-- Start -->
        <q-page-sticky position='bottom-right' :offset='[36, 32]'>
            <div class='row'>
                <!-- CLI FAB -->
                <div class='q-mr-md q-mt-md'>
                    <q-btn class='bg-grey-9' flat round icon='mdi-console' color='grey-4' @click='cliDialog = true'>
                        <q-tooltip anchor="top middle" self="bottom middle" :offset="[10, 10]">            
                            <span class='text-weight-medium'>CLI Version Config</span>
                        </q-tooltip>
                    </q-btn>
                </div>

                <!-- Start FAB -->
                <div>
                    <q-btn 
                        fab 
                        push
                        icon='mdi-play' 
                        color='primary'
                        :disable='!config.path && !playlist.data'
                        @click='start'                
                    >
                        <q-tooltip anchor="top middle" self="bottom middle" :offset="[10, 10]">            
                            <span class='text-weight-medium'>START</span>
                        </q-tooltip>
                    </q-btn>
                </div>
            </div>
        </q-page-sticky>
    </div>


    <!-- CLI Dialog -->
    <q-dialog v-model='cliDialog'>
        <CliDialog :config='config' command='audiofeatures' :extra='`--client-id ${$1t.spotify.value.clientId} --client-secret ${$1t.spotify.value.clientSecret}`'></CliDialog>
    </q-dialog>

</div>
</template>

<script lang='ts' setup>
import TagFields from '../components/TagFields.vue';
import PlaylistDropZone from '../components/PlaylistDropZone.vue';
import Separators from '../components/Separators.vue';
import SpotifyLogin from '../components/SpotifyLogin.vue';
import CliDialog from '../components/CliDialog.vue';
import { Playlist } from '../scripts/utils';
import { onMounted, ref } from 'vue';
import { AudioFeaturesConfig } from '../scripts/settings';
import { get1t } from '../scripts/onetagger';
import { useRouter } from 'vue-router';

const $1t = get1t();
const $router = useRouter();
const playlist = ref<Playlist>({});
const config = ref(new AudioFeaturesConfig());
const cliDialog = ref(false);

// Browse folder
function browse() {
    $1t.browse('af', config.value.path);
}

// Start tagging
function start() {
    // Save config
    $1t.settings.value.audioFeatures.config = config.value;
    $1t.saveSettings();

    let p = null;
    if (playlist.value && playlist.value.data)
        p = playlist.value;

    // Start
    config.value.type = 'audioFeatures';
    $1t.send('startTagging', {config: config.value, playlist: p});
    $router.push('/audiofeatures/status');
}
    

onMounted(() => {
    // Load config from settings
    if ($1t.settings.value.audioFeatures.config) {
        config.value = AudioFeaturesConfig.fromJson($1t.settings.value.audioFeatures.config);
    }
    // Register events
    $1t.onAudioFeaturesEvent = (json) => {
        switch (json.action) {
            case 'browse':
                config.value.path = json.path;
                break;
        }
    }
});
</script>

<style>
.selectable {
    user-select: all;
}
.auth-container {
    width: 60%;
    margin-left: 20%;
}
.path-field {
    width: 50% !important;
    margin-left: 25%;
}
.af-wrapper {
    width: 100%;
    display: flex;
    justify-content: center;
}
.af-content {
    width: 100%;
    max-width: 1400px;
}
.t-range .q-slider__inner.absolute {
    background: var(--q-primary) !important;    
}

.custom-margin {
    margin-top: 35px !important;
}

.click-highlight {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}

</style>
