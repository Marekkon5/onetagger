<template>
<div class='text-center af-wrapper'>

    <!-- Login -->
    <div v-if='!spotifyAuthorized' class='af-content'>
        <div class='text-h5 q-mt-md text-grey-4'>Setup</div>
        <div class='text-subtitle1 text-grey-6 q-mt-md'>
            <span class='text-grey-4'>1.</span> Open <span class='dotted-underline clickable text-primary' @click='$1t.url("https://developer.spotify.com/dashboard")'>Spotify Developer</span> account and create an app<br>
            <span class='text-grey-4'>2.</span> In settings set the Callback URL to: <span class='selectable text-grey-4'>{{redirectUrl}}</span> <br>
            <span class='text-grey-4'>3.</span> Enter your Client ID and Client Secret below and press login <br>
        </div>
        <!-- Client ID and secret field -->
        <form class='row q-mt-xl auth-container justify-evenly'>
            <q-input v-model='clientId' outlined label='Client ID' class='col-5 q-pr-xs'></q-input>
            <q-input v-model='clientSecret' :type='$1t.info.os == "macos" ? "text" : "password"' outlined label='Client Secret' class='col-5 q-pr-xs'></q-input>
            <q-btn push color='primary' class='text-black' @click='authorize'>Login</q-btn>
        </form>
        <!-- Description -->
        <div class='q-mt-xl text-subtitle2 text-grey-6'>
            Automatically tag Spotify’s so called audio features to your local audio files, based on ISRC & exact match<br>
            More info? Hit <q-icon style='padding-bottom: 3px;' name='mdi-help-circle-outline'></q-icon> HELP on the right
        </div>
    </div>

    <!-- Logged in -->
    <div v-if='spotifyAuthorized' class='af-content'>
        <!-- Path -->
        <div class='text-h5 q-mt-md q-mb-md text-grey-4'>Select folder</div>
        <q-input filled class='path-field inset-shadow-down' label='Path' v-model='config.path'>
            <template v-slot:append>
                <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse'></q-btn>
            </template>
        </q-input>

        <!-- Drag and drop -->
        <div class='row justify-center' style='width: 100%'>
            <PlaylistDropZone 
                :value='playlist' 
                @input='Object.assign(playlist, $event)'
                class='q-my-sm q-py-md' 
                style='width: 50%'
            ></PlaylistDropZone>
        </div>

        <!-- Main tag -->
        <div class='text-h5 q-mt-xl text-grey-4'>Prominent tag</div>
        <div class='text-subtitle1 q-mt-xs text-grey-6'>Converts most prominent audio features value (0-100) to a description - based on threshold - and writes to entered tagcode field</div>
        <div class='text-subtitle2 q-mt-xs q-mb-sm text-grey-6'>e.g. #acoustic, #dynamics-low, #energy-high, #vocal-med, #live, #speech, #positive</div>

        <TagFields class='q-mx-xl q-pl-md' v-model='config.mainTag'></TagFields>

        <!-- Values -->
        <div class='text-h5 q-mt-xl q-mb-md text-grey-4'>Properties</div>
        <div class='q-px-xl'>
            <!-- Header -->
            <div class='row text-subtitle1 text-bold q-mb-sm text-grey-6'>
                <div class='col-1'>Include
                    <q-icon name='mdi-help-circle-outline' class='q-ml-xs q-mb-xs'>
                        <q-tooltip content-style="font-size: 13px">
                            Include the audio feature in prominent tag
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
        <div class='row q-mx-xl q-my-md justify-center'>
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

export default {
    name: 'AudioFeatures',
    components: {TagFields, PlaylistDropZone, Separators},
    data() {
        return {
            clientId: this.$1t.settings.audioFeatures.spotifyClientId,
            clientSecret: this.$1t.settings.audioFeatures.spotifyClientSecret,
            spotifyAuthorized: false,
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
                    popularity: {enabled: false, range: {min: 0, max: 80}, 
                        tag: {id3: '1T_POPULARITY', vorbis: '1T_POPULARITY', mp4: '1T_POPULARITY'}}
                    
                }
            }
        }
    },
    methods: {
        //Authorize spotify
        authorize() {
            this.$1t.send("spotifyAuthorize", {
                clientId: this.clientId,
                clientSecret: this.clientSecret
            });
            //Save
            this.$1t.settings.audioFeatures.spotifyClientId = this.clientId;
            this.$1t.settings.audioFeatures.spotifyClientSecret = this.clientSecret;
            this.$1t.saveSettings();
        },
        //Browse folder
        browse() {
            this.$1t.send('browse', {context: 'af', path: this.config.path});
        },
        //Start tagging
        start() {
            //Save config
            this.$1t.settings.audioFeatures.config = this.config;
            this.$1t.saveSettings();

            let playlist = null;
            if (this.playlist && this.playlist.data)
                playlist = this.playlist;

            //Start
            this.config.type = 'audioFeatures';
            this.$1t.send('startTagging', {config: this.config, playlist});
            this.$router.push('/audiofeatures/status');
        }
    },
    mounted() {
        //Load config from settings
        if (this.$1t.settings.audioFeatures.config) {
            let properties = Object.assign({}, this.config.properties, this.$1t.settings.audioFeatures.config.properties);
            this.config = Object.assign({}, this.config, this.$1t.settings.audioFeatures.config);
            this.config.properties = properties;
        }
        //Register events
        this.$1t.onAudioFeaturesEvent = (json) => {
            switch (json.action) {
                case 'browse':
                    this.config.path = json.path;
                    break;
                case 'spotifyAuthorized':
                    this.spotifyAuthorized = json.value;
                    break;
            }
        }
    },
    computed: {
        redirectUrl() {
            return `http://${window.location.hostname}:36914/spotify`
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

</style>