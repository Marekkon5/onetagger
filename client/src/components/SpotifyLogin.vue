<template>
<div>
    <div class='text-subtitle1 text-grey-6 q-mt-md'>
        <span class='text-grey-4'>1.</span> Open 
            <span class='clickable doc-link' @click='$1t.url("https://developer.spotify.com/dashboard")'>Spotify Developer <q-icon name='mdi-open-in-new'></q-icon></span> account & create an app<br>
        <span class='text-grey-4'>2.</span> In settings set the Callback URL to: <span class='selectable text-grey-4'>{{redirectUrl}}</span> <br>
        <span class='text-grey-4'>3.</span> Enter your Client ID & Client Secret below & click login <br>
        <span @click='$1t.url("https://youtu.be/i0q5qWQSH9Y")' class='clickable doc-link'><q-icon name='mdi-youtube' class='q-pr-xs'></q-icon>video demo</span>
    </div>
    <!-- Client ID and secret field -->
    <form class='row q-mt-xl auth-container justify-evenly'>
        <q-input v-model='$1t.spotify.clientId' outlined label='Client ID' class='col-5 q-pr-xs'></q-input>
        <q-input v-model='$1t.spotify.clientSecret' :type='$1t.info.os == "macos" ? "text" : "password"' outlined label='Client Secret' class='col-5 q-pr-xs'></q-input>
        <q-btn push color='primary' class='text-black' @click='authorize'>Login</q-btn>
    </form>
</div>
</template>

<script>
export default {
    name: 'SpotifyLogin',
    methods: {
        authorize() {
            this.$1t.send("spotifyAuthorize", {
                clientId: this.$1t.spotify.clientId,
                clientSecret: this.$1t.spotify.clientSecret
            });
            // Save (using AF for compatibility with older settings)
            this.$1t.settings.audioFeatures.spotifyClientId = this.$1t.spotify.clientId;
            this.$1t.settings.audioFeatures.spotifyClientSecret = this.$1t.spotify.clientSecret;
            this.$1t.saveSettings();
        }
    },
    mounted() {
        // Register events
        this.$1t.onSpotifyAuthEvent = (json) => {
            switch (json.action) {
                case 'spotifyAuthorized':
                    this.$1t.spotify.authorized = json.value;
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
.doc-link {
    color: var(--q-color-primary);
}
.doc-link:hover {
    color: #f0f0f0;    
}
</style>