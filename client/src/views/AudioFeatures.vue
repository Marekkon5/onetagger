<template>
<div class='text-center'>

    <div class='text-h3 q-mt-md'>Spotify Audio Features</div>

    <!-- Login -->
    <div v-if='!$1t.audioFeatures.spotifyAuthorized' class='full-width'>
        <div class='text-subtitle1 q-mt-md'>
            1. make spotify developer account <br>
            2. in settings add callback url: <div class='selectable text-bold'>http://localhost:36914</div>
            3. enter client id and secret here <br>
        </div>
        <!-- Client ID and secret field -->
        <div class='row q-mt-md auth-container'>
            <q-input v-model='clientId' outlined label='Client ID' class='col-5 q-pr-md'></q-input>
            <q-input v-model='clientSecret' type='password' outlined label='Client Secret' class='col-5 q-pr-md'></q-input>
            <q-btn color='primary' class='text-black' @click='authorize'>Login</q-btn>
        </div>
    </div>

    <!-- Logged in -->
    <div v-if='$1t.audioFeatures.spotifyAuthorized' class='full-width q-mt-xl'>
        <q-input outlined label='Path' v-model='$1t.audioFeatures.path' class='path-field'>
            <template v-slot:append>
                <q-btn round dense flat icon='mdi-open-in-app' class='text-white' @click='browse'></q-btn>
            </template>
        </q-input>

        <div class='text-h5 q-my-md'>//TODO: IDK advanced or something</div>
        <q-btn color='primary' class='text-black q-mt-md' size='md' @click='start'>START</q-btn>
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

            this.$1t.send('audioFeaturesStart', {
                config: {
                    path: this.$1t.audioFeatures.path
                }
            });
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
</style>