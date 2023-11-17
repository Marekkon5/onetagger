<template>
<div>
    <div class='text-subtitle1 text-grey-6 q-mt-md'>
        <span class='text-grey-4'>1.</span> Open 
            <span class='clickable doc-link' @click='$1t.url("https://developer.spotify.com/dashboard")'>Spotify Developer<span class="q-ml-xs text-caption margin-open"><q-icon name='mdi-open-in-new'></q-icon></span></span> account & create an app<br>
        <span class='text-grey-4'>2.</span> In settings set the Callback URL to: <span class='selectable text-grey-4'>{{redirectUrl}}</span> <br>
        <span class='text-grey-4'>3.</span> Enter your Client ID & Client Secret below &<span class='keybind-icon q-px-sm text-caption text-bold'>CLICK</span> login <br>
        <span @click='$1t.url("https://youtu.be/i0q5qWQSH9Y")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>video demo</span>
    </div>
    <!-- Client ID and secret field -->
    <form class='row q-mt-xl auth-container justify-evenly'>
        <q-input v-model='$1t.spotify.value.clientId' outlined label='Client ID' class='col-5 q-pr-xs'></q-input>
        <q-input v-model='$1t.spotify.value.clientSecret' :type='$1t.info.value.os == "macos" ? "text" : "password"' outlined label='Client Secret' class='col-5 q-pr-xs q-pl-sm'></q-input>
        <q-btn push color='primary' class='text-black q-ml-sm' @click='authorize'>Login</q-btn>
    </form>
</div>
</template>

<script lang='ts' setup>
import { get1t } from '../scripts/onetagger.js';
import { spotifyUrl } from '../scripts/utils';

const $1t = get1t();
const redirectUrl = spotifyUrl();

function authorize() {
    $1t.send("spotifyAuthorize", {
        clientId: $1t.spotify.value.clientId,
        clientSecret: $1t.spotify.value.clientSecret
    });
    // Save (using AF for compatibility with older settings)
    $1t.settings.value.audioFeatures.spotifyClientId = $1t.spotify.value.clientId;
    $1t.settings.value.audioFeatures.spotifyClientSecret = $1t.spotify.value.clientSecret;
    $1t.saveSettings();
}
</script>

<style>

.doc-link {
    color: var(--q-primary) !important;  
    text-decoration-line: underline;  
}

.doc-link:hover {
    color: #f0f0f0 !important; 
    text-decoration-line: underline;   
}

.margin-open {
    margin-right: 2px;    
}

</style>