<template>
<div class='text-center af-wrapper'>

    <!-- Login -->
    <div v-if='!$1t.audioFeatures.spotifyAuthorized' class='af-content'>
        <div class='text-h5 q-mt-md'>Setup</div>
        <div class='text-subtitle1 q-mt-md'>
            1. Open <span class='link' @click='$1t.url("https://developer.spotify.com/dashboard")'>Spotify Developer</span> account and create an app.<br>
            2. In settings set the Callback URL to: <span class='selectable text-bold'>http://localhost:36914/spotify</span> <br>
            3. Enter your Client ID and Client Secret below and press login. <br>
        </div>
        <!-- Client ID and secret field -->
        <div class='row q-mt-xl auth-container justify-evenly'>
            <q-input v-model='clientId' outlined label='Client ID' class='col-5 q-pr-xs'></q-input>
            <q-input v-model='clientSecret' type='password' outlined label='Client Secret' class='col-5 q-pr-xs'></q-input>
            <q-btn color='primary' class='text-black' @click='authorize'>Login</q-btn>
        </div>
        <!-- Description -->
        <div class='q-mt-xl text-subtitle2 text-grey-6'>
            Automatically tag Spotify’s so called audio features to your local audio files, based on ISRC & exact match.<br>
        </div>
    </div>

    <!-- Logged in -->
    <div v-if='$1t.audioFeatures.spotifyAuthorized' class='af-content'>
        <!-- Path -->
        <div class='text-h5 q-mt-md q-mb-md'>Select folder</div>
        <q-input filled class='path-field' label='Path' v-model='$1t.audioFeatures.path'>
            <template v-slot:append>
                <q-btn round dense flat icon='mdi-open-in-app' class='text-white' @click='browse'></q-btn>
            </template>
        </q-input>

        <!-- Main tag -->
        <div class='text-h5 q-mt-xl'>Prominent tag</div>
        <div class='text-subtitle1 q-mt-xs'>Converts most prominent audio features value (0-100) to a description - based on treshold - and writes to entered tagcode field.</div>
        <div class='text-subtitle2 q-mt-xs q-mb-sm text-grey-6'>e.g. Acoustic, Danceable, Energetic, Instrumental, Live, Speech, Positive</div>

        <div class='row q-mx-xl'>
            <div class='col-6 q-px-sm'>
                <q-input filled v-model='config.mainTag.id3' label='ID3 (MP3/AIFF)'></q-input>
            </div>
            <div class='col-6 q-px-sm'>
                <q-input filled v-model='config.mainTag.flac' label='FLAC'></q-input>
            </div>
        </div>

        <!-- Values -->
        <div class='text-h5 q-mt-xl q-mb-md'>Properties</div>
        <div class='q-px-xl'>
            <!-- Header -->
            <div class='row text-subtitle1 text-bold q-mb-sm'>
                <div class='col-1'>Include
                    <q-icon name='mdi-help-circle-outline' class='q-ml-xs q-mb-xs'>
                        <q-tooltip content-style="font-size: 12px">
                            Include the audio feature in prominent tag.
                        </q-tooltip>
                    </q-icon>
                </div>
                <div class='col-2'>Audio feature</div>
                <div class='col-3'>ID3 Tag Name (MP3/AIFF)</div>
                <div class='col-3'>FLAC Tag Name</div>
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
                    <div class='col-3'>
                        <q-input 
                            filled
                            dense 
                            class='q-px-sm q-pb-xs'
                            v-model='config.properties[key].tag.id3'
                            label='ID3 (MP3 + AIFF)'
                        ></q-input>
                    </div>
                    <div class='col-3'>
                        <q-input 
                            filled
                            dense 
                            class='q-px-sm q-pb-xs'
                            v-model='config.properties[key].tag.flac'
                            label='FLAC'
                        ></q-input>
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

        <!-- Start -->
        <br>
        <q-btn color='primary' class='text-black q-my-md' size='md' @click='start' v-if='$1t.audioFeatures.path'>START</q-btn>
    </div>

</div>
</template>

<script>
export default {
    name: 'AudioFeatures',
    data() {
        return {
            clientId: this.$1t.settings.audioFeatures.spotifyClientId,
            clientSecret: this.$1t.settings.audioFeatures.spotifyClientSecret,
            config: {
                mainTag: {id3: 'STYLE', flac: 'STYLE'},
                properties: {
                    acousticness: {enabled: true, range: {min: 0, max: 90}, tag: {id3: '1T_ACOUSTICNESS', flac: '1T_ACOUSTICNESS'}},
                    danceability: {enabled: true, range: {min: 20, max: 80}, tag: {id3: '1T_DANCEABILITY', flac: '1T_DANCEABILITY'}},
                    energy: {enabled: true, range: {min: 20, max: 90}, tag: {id3: '1T_ENERGY', flac: '1T_ENERGY'}},
                    instrumentalness: {enabled: true, range: {min: 50, max: 90}, tag: {id3: '1T_INSTRUMENTALNESS', flac: '1T_INSTRUMENTALNESS'}},
                    liveness: {enabled: true, range: {min: 0, max: 80}, tag: {id3: '1T_LIVENESS', flac: '1T_LIVENESS'}},
                    speechiness: {enabled: true, range: {min: 0, max: 70}, tag: {id3: '1T_SPEECHINESS', flac: '1T_SPEECHINESS'}},
                    valence: {enabled: true, range: {min: 15, max: 85}, tag: {id3: '1T_VALENCE', flac: '1T_VALENCE'}}
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
            this.$1t.send('browse', {context: 'af'});
        },
        //Start tagging
        start() {
            //Save config
            this.$1t.settings.audioFeatures.config = this.config;
            this.$1t.saveSettings();

            //Lock UI
            this.$1t.lock.locked = true;
            this.$1t.audioFeatures.done = false;
            this.$1t.audioFeatures.statuses = [];
            this.$1t.audioFeatures.started = Date.now();
            this.$1t.audioFeatures.ended = null;
            //Start
            let config = this.config;
            this.config.path = this.$1t.audioFeatures.path;
            this.$1t.send('audioFeaturesStart', {config});
            //UI
            this.$router.push('/audiofeatures/status');
        }
    },
    mounted() {
        //Load config from settings
        if (this.$1t.settings.audioFeatures.config) {
            this.config = Object.assign({}, this.config, this.$1t.settings.audioFeatures.config);
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
.link {
    font-weight: bold;
    color: #00d2bf;
    cursor: pointer;
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
    background: #00D2BF !important;
}
</style>