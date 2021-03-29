<template>
<div class='text-center'>

    <div class='text-h4 q-mt-md'>Spotify Audio Features</div>

    <!-- Login -->
    <div v-if='!$1t.audioFeatures.spotifyAuthorized' class='full-width'>
        <div class='text-h6 q-mt-md'>
            1. Open <span class='link' @click='$1t.url("https://developer.spotify.com/dashboard")'>Spotify Developer</span> account and create an app.<br>
            2. In settings set the Callback URL to: <span class='selectable text-bold'>http://localhost:36914/spotify</span> <br>
            3. Enter your Client ID and Client Secret below and press login. <br>
        </div>
        <!-- Client ID and secret field -->
        <div class='row q-mt-xl auth-container'>
            <q-input v-model='clientId' outlined label='Client ID' class='col-5 q-pr-md'></q-input>
            <q-input v-model='clientSecret' type='password' outlined label='Client Secret' class='col-5 q-pr-md'></q-input>
            <q-btn color='primary' class='text-black' @click='authorize'>Login</q-btn>
        </div>
    </div>

    <!-- Logged in -->
    <div v-if='$1t.audioFeatures.spotifyAuthorized' class='full-width q-mt-xl'>
        <!-- Path -->
        <q-input outlined label='Path' v-model='$1t.audioFeatures.path' class='path-field'>
            <template v-slot:append>
                <q-btn round dense flat icon='mdi-open-in-app' class='text-white' @click='browse'></q-btn>
            </template>
        </q-input>
        <!-- Options -->
        <div class='text-h5 q-mt-xl q-mb-md'>Options:</div>
        <q-checkbox class='checkbox' label='Write raw values to custom 1T tags' v-model='config.saveRaw'></q-checkbox>
        

        <!-- Start -->
        <br>
        <q-btn color='primary' class='text-black q-mt-xl' size='md' @click='start' v-if='$1t.audioFeatures.path'>START</q-btn>
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
                saveRaw: true
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
            //Lock UI
            this.$1t.lock.locked = true;
            this.$1t.audioFeatures.done = false;
            this.$1t.audioFeatures.statuses = [];
            this.$1t.audioFeatures.started = Date.now();
            //Start
            let config = this.config;
            this.config.path = this.$1t.audioFeatures.path;
            this.$1t.send('audioFeaturesStart', {config});
            //UI
            this.$router.push('/audiofeatures/status');
        }
    },
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
    color: lightblue;
    cursor: pointer;
}
</style>