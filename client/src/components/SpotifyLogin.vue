<template>
<div>
    <div class='text-subtitle1 text-grey-6 q-mt-md'>
        <span class='text-grey-4'>1.</span>
        <i18n-t keypath='autotag.spotifyCreateApp' scope='global'>
            <template #developer><span class='clickable doc-link' @click='$1t.url("https://developer.spotify.com/dashboard")'>Spotify Developer<span class='q-ml-xs text-caption margin-open'><q-icon name='mdi-open-in-new'></q-icon></span></span></template>
        </i18n-t><br>
        <span class='text-grey-4'>2.</span>
        <i18n-t keypath='autotag.spotifyCallback' scope='global'>
            <template #url><span class='selectable text-grey-4'>{{ redirectUrl }}</span></template>
        </i18n-t><br>
        <span class='text-grey-4'>3.</span> {{ t('autotag.spotifyCredentials') }}<br>
        <span @click='$1t.url("https://youtu.be/i0q5qWQSH9Y")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('autotag.videoDemo') }}</span>
    </div>
    <!-- Client ID and secret field -->
    <form class='row q-mt-xl auth-container justify-evenly'>
        <q-input v-model='$1t.spotify.value.clientId' outlined :label="t('autotag.clientId')" class='col-5 q-pr-xs'></q-input>
        <q-input v-model='$1t.spotify.value.clientSecret' :type='$1t.info.value.os == "macos" ? "text" : "password"' outlined :label="t('autotag.clientSecret')" class='col-5 q-pr-xs q-pl-sm'></q-input>
        <q-btn push color='primary' class='text-black q-ml-sm' @click='authorize'>{{ t('autotag.login') }}</q-btn>
    </form>
</div>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
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
