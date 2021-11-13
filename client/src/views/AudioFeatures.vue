<template>
<div class='text-center af-wrapper'>

    <!-- Login -->
    <div v-if='!$1t.spotify.authorized' class='af-content'>
        <div class='text-h5 q-mt-lg text-grey-4'>Setup</div>
        <SpotifyLogin></SpotifyLogin>
        <!-- Description -->
        <div class='q-mt-xl text-subtitle2 text-grey-6' style='line-height: 24px'>
            Automatically tag your local audio files, with so called audio features by Spotify, based on <q-badge outline color='primary'><span class='text-white'>ISRC</span></q-badge> tag or exact match.<br>
            More info? Click <q-icon style='padding-bottom: 3px;' name='mdi-help-circle-outline'></q-icon> HELP on the right <br>            
        </div>
    </div>

    <!-- Logged in -->
    <div v-if='$1t.spotify.authorized' class='af-content'>
        <!-- Path -->
        <div class='text-h5 q-mt-lg text-grey-4'>Select input</div>
            <div class='text-subtitle2 q-mb-md text-grey-6'>Drag & drop folder, copy/paste path directly or click the <q-icon name='mdi-open-in-app'></q-icon> icon to browse</div>
        <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
            <div class='col-1'></div>
            <q-input filled class='col-10' label='Path' v-model='config.path'>
                <template v-slot:append>
                    <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse'></q-btn>
                </template>
            </q-input>

            <div class='col-1'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='path-tooltip q-mx-sm q-pt-md q-mt-xs'>
                    <q-tooltip content-style="font-size: 13px">Subfolders are included</q-tooltip>
                </q-icon>
            </div>
        </div>
            
        <!-- Drag and drop -->
        <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
            <div class='col-1'></div>
            <PlaylistDropZone 
                :value='playlist' 
                @input='Object.assign(playlist, $event)'
                class='q-my-sm q-pt-md q-pb-md col-10'                 
                ></PlaylistDropZone>            
                    
            <div class='col-1'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='playlist-tooltip q-mx-sm q-mt-xl q-pt-sm'>
                    <q-tooltip content-style="font-size: 13px">.m3u and .m3u8 extensions are supported</q-tooltip>
                </q-icon>
            </div>
        </div>

        <!-- Main tag -->
        <q-separator class='q-mx-auto q-mt-lg q-mb-lg custom-separator' inset color="dark" />
        <div class='text-h5 text-grey-4 custom-margin'>Prominent tag</div>
        <div class='text-subtitle2 text-grey-6'>Converts most prominent audio features value 0-100 to a description - based on threshold - and writes to selected tagcode field</div>
        <div class='text-subtitle2 q-mt-xs q-mb-md text-grey-5'>e.g. #dance-high, #energy-med, #vocal-low, #positive, #popular</div>

        <TagFields style='max-width: 550px; margin: auto;' v-model='config.mainTag'></TagFields>

        <!-- Values -->
        <q-separator class='q-mx-auto q-mt-lg q-mb-lg custom-separator' style='margin-top: 28px;' inset color="dark"/>
        <div class='text-h5 q-mt-lg text-grey-4 custom-margin'>Properties</div>
        <div class='q-px-xl'>
            <!-- Header -->
            <div class='row text-subtitle2 q-mb-md text-grey-6'>
                <div class='col-1'>Include
                    <q-icon name='mdi-help-circle-outline' class='q-ml-xs q-mb-xs'>
                        <q-tooltip content-style="font-size: 13px">
                            Include the audio feature in Prominent tag
                        </q-tooltip>
                    </q-icon>
                </div>
                <div class='col-2'>Audio feature</div>
                <div class='col-6'>Tag name</div>
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
                        <span class='text-subtitle1' style='text-transform: capitalize;'>{{key}}</span>
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
        <div class='text-center text-body1 text-grey-4 q-mt-md q-mb-sm'>Separators</div>
        <div class='row q-pb-lg q-mt-sm justify-center'>            
            <Separators :initial='config.separators' @input='config.separators = $event'></Separators>
        </div>


       <!-- Start -->
        <q-page-sticky position='bottom-right' :offset='[36, 18]'>
            <q-btn 
                fab push
                icon='mdi-play' 
                color='primary'
                :disable='!config.path && !playlist'
                @click='start'                
            >
            <q-tooltip anchor="top middle" self="bottom middle" :offset="[10, 10]">            
                <span class='text-weight-bold'>START</span>
            </q-tooltip>
            </q-btn>
        </q-page-sticky>
       
    </div>



</div>
</template>

<script>
import TagFields from '../components/TagFields';
import PlaylistDropZone from '../components/PlaylistDropZone.vue';
import Separators from '../components/Separators.vue';
import SpotifyLogin from '../components/SpotifyLogin.vue';

export default {
    name: 'AudioFeatures',
    components: {TagFields, PlaylistDropZone, Separators, SpotifyLogin},
    data() {
        return {
            playlist: {filename: null, data: null, format: null},
            config: {
                path: null,
                mainTag: {id3: 'AUDIO_FEATURES', vorbis: 'AUDIO_FEATURES', mp4: 'AUDIO_FEATURES'},
                separators: {id3: ', ', vorbis: null, mp4: ', '},
                properties: {
                    acousticness: {enabled: true, range: {min: 0, max: 90}, 
                        tag: {id3: '1T_ACOUSTICNESS', vorbis: '1T_ACOUSTICNESS', mp4: '1T_ACOUSTICNESS'}},
                    danceability: {enabled: true, range: {min: 20, max: 80}, 
                        tag: {id3: '1T_DANCEABILITY', vorbis: '1T_DANCEABILITY', mp4: '1T_DANCEABILITY'}},
                    energy: {enabled: true, range: {min: 20, max: 90}, 
                        tag: {id3: '1T_ENERGY', vorbis: '1T_ENERGY', mp4: '1T_ENERGY'}},
                    instrumentalness: {enabled: true, range: {min: 50, max: 90}, 
                        tag: {id3: '1T_INSTRUMENTALNESS', vorbis: '1T_INSTRUMENTALNESS', mp4: '1T_INSTRUMENTALNESS'}},
                    liveness: {enabled: true, range: {min: 0, max: 80}, 
                        tag: {id3: '1T_LIVENESS', vorbis: '1T_LIVENESS', mp4: '1T_LIVENESS'}},
                    speechiness: {enabled: true, range: {min: 0, max: 70}, 
                        tag: {id3: '1T_SPEECHINESS', vorbis: '1T_SPEECHINESS', mp4: '1T_SPEECHINESS'}},
                    valence: {enabled: true, range: {min: 15, max: 85}, 
                        tag: {id3: '1T_VALENCE', vorbis: '1T_VALENCE', mp4: '1T_VALENCE'}},
                    popularity: {enabled: true, range: {min: 0, max: 80}, 
                        tag: {id3: '1T_POPULARITY', vorbis: '1T_POPULARITY', mp4: '1T_POPULARITY'}}
                    
                }
            }
        }
    },
    methods: {
        // Browse folder
        browse() {
            this.$1t.send('browse', {context: 'af', path: this.config.path});
        },
        // Start tagging
        start() {
            // Save config
            this.$1t.settings.audioFeatures.config = this.config;
            this.$1t.saveSettings();

            let playlist = null;
            if (this.playlist && this.playlist.data)
                playlist = this.playlist;

            // Start
            this.config.type = 'audioFeatures';
            this.$1t.send('startTagging', {config: this.config, playlist});
            this.$router.push('/audiofeatures/status');
        }
    },
    mounted() {
        // Load config from settings
        if (this.$1t.settings.audioFeatures.config) {
            let properties = Object.assign({}, this.config.properties, this.$1t.settings.audioFeatures.config.properties);
            this.config = Object.assign({}, this.config, this.$1t.settings.audioFeatures.config);
            this.config.properties = properties;
        }
        // Register events
        this.$1t.onAudioFeaturesEvent = (json) => {
            switch (json.action) {
                case 'browse':
                    this.config.path = json.path;
                    break;
            }
        }
    }
}
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
.t-range .q-slider__track-container {
    background: var(--q-color-primary) !important;    
}
.custom-separator {
    max-width: 550px;
    margin: auto;
}
.custom-margin {
    margin-top: 35px !important;
}
</style>