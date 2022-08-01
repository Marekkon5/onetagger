<template>
<div class='text-center'>
    <!-- Path -->
    <div class='text-h5 text-grey-4'>Select input</div>
    <div class='text-subtitle2 q-mb-md text-grey-6'>Drag & drop folder, copy/paste path directly or click the browse <span><q-icon name='mdi-open-in-app' class='q-mb-xs'></q-icon> icon</span></div>
    <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
        <div class='col-1'></div>
        <q-input filled class='col-10' label='Path' v-model='$1t.config.value.path'>
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
            :value='$1t.autoTaggerPlaylist'
            @input='Object.assign($1t.autoTaggerPlaylist, $event)'
            class='q-my-sm q-pt-md q-pb-lg col-10'            
        ></PlaylistDropZone>
                
        <div class='col-1'>
            <q-icon name='mdi-help-circle-outline text-grey-6' class='playlist-tooltip q-mx-sm q-mt-xl q-pt-sm'>
                <q-tooltip content-style="font-size: 13px">.m3u and .m3u8 extensions are supported</q-tooltip>
            </q-icon>
        </div>
    </div>
    
    <!-- Tags -->
    <q-separator class='q-mx-auto q-mt-md custom-separator' inset color="dark"/>
    <div class='text-h5 q-mt-lg text-grey-4' style='margin-top: 35px;'>Select tags</div>
    <div class='text-subtitle2 q-mb-sm text-grey-6'>Check the box to fetch stated tag</div>
    
    <div class='q-pt-xs q-mb-md' style='max-width: 550px; margin:auto;'>
        <div class='row justify-between q-ml-xl tags wrap'>
            <!-- Album art -->
            <q-checkbox class='tag checkbox text-grey-4' label='Album Art' v-model='$1t.config.value.albumArt'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Resolution is platform dependent</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Album -->
            <q-checkbox class='tag checkbox text-grey-4' label='Album' v-model='$1t.config.value.album'></q-checkbox> 
            <!-- Album artist -->
            <q-checkbox class='tag checkbox text-grey-4' label='Album Artist' v-model='$1t.config.value.albumArtist'></q-checkbox>
            <!-- Artist -->
            <q-checkbox class='tag checkbox text-grey-4' label='Artist' v-model='$1t.config.value.artist'></q-checkbox>
            <!-- Title -->
            <q-checkbox class='tag checkbox text-grey-4' label='Title' v-model='$1t.config.value.title'></q-checkbox>
            <!-- Version -->
            <q-checkbox :disabled='!isSupported("version")' class='tag checkbox text-grey-4' label='Version' v-model='$1t.config.value.version'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport, Beatsource & Traxsource</q-tooltip>
                </q-icon>                
            </q-checkbox>
            <!-- Remixers -->
            <q-checkbox :disabled='!isSupported("remixers")' class='tag checkbox text-grey-4' label='Remixers' v-model='$1t.config.value.remixer'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport & Beatsource</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Genre -->
            <q-checkbox class='tag checkbox' label='Genre' v-model='$1t.config.value.genre'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Spotify will populate multiple genres based on artist</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Style/Subgenre -->
            <q-checkbox :disabled='!isSupported("style")' class='tag checkbox text-grey-4' label='Style/Subgenre' v-model='$1t.config.value.style'>            
                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Style is available from Discogs only, Subgenre from Beatport only</q-tooltip>
                </q-icon>            
            </q-checkbox>   
            <!-- Label -->                
            <q-checkbox class='tag checkbox text-grey-4' label='Label' v-model='$1t.config.value.label'></q-checkbox>
            <!-- Release ID -->
            <q-checkbox class='tag checkbox text-grey-4' label='Release ID' v-model='$1t.config.value.releaseId'></q-checkbox>
            <!-- Track ID -->
            <q-checkbox :disabled='!isSupported("trackId")' class='tag checkbox text-grey-4' label='Track ID' v-model='$1t.config.value.trackId'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport, Beatsource & Traxsource</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- BPM -->
            <q-checkbox :disabled='!isSupported("bpm")' class='tag checkbox text-grey-4' label='BPM' v-model='$1t.config.value.bpm'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport, Beatsource, Juno Download & Traxsource</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Key -->
            <q-checkbox :disabled='!isSupported("key")' class='tag checkbox text-grey-4' label='Key' v-model='$1t.config.value.key'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport, Beatsource & Traxsource</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Mood -->
            <q-checkbox :disabled='!isSupported("mood")' class='tag checkbox text-grey-4' label='Mood' v-model='$1t.config.value.mood'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from BPM Supreme</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Catalog number -->
            <q-checkbox class='tag checkbox text-grey-4' label='Catalog Number' v-model='$1t.config.value.catalogNumber'></q-checkbox>
            <!-- Track number -->
            <q-checkbox class='tag checkbox text-grey-4' label='Track Number' v-model='$1t.config.value.trackNumber'></q-checkbox>
            <!-- Disc number -->
            <q-checkbox :disabled='!isSupported("discNumber")' class='tag checkbox text-grey-4' label='Disc Number' v-model='$1t.config.value.discNumber'></q-checkbox>
            <!-- Duration -->
            <q-checkbox class='tag checkbox text-grey-4' label='Duration' v-model='$1t.config.value.duration'></q-checkbox>
            <!-- Track Total -->
            <q-checkbox :disabled='!isSupported("trackTotal") || !$1t.config.value.trackNumber' class='tag checkbox text-grey-4' label='Track Total' v-model='$1t.config.value.trackTotal'></q-checkbox>
            <!-- ISRC -->
            <q-checkbox :disabled='!isSupported("isrc")' class='tag checkbox text-grey-4' label='ISRC' v-model='$1t.config.value.isrc'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport, Beatsource, MusicBrainz & Spotify</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Publish Date -->
            <q-checkbox :disabled='!isSupported("publishDate")' class='tag checkbox text-grey-4' label='Publish Date' v-model='$1t.config.value.publishDate'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Available from Beatport only</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- Release Date -->
            <q-checkbox class='tag checkbox text-grey-4' label='Release Date' v-model='$1t.config.value.releaseDate'></q-checkbox>
            <!-- URLs -->
            <q-checkbox class='tag checkbox text-grey-4' label='URLs' v-model='$1t.config.value.url'></q-checkbox>
            <!-- Other -->
            <q-checkbox :disabled='!isSupported("other")' class='tag checkbox text-grey-4' label='Other' v-model='$1t.config.value.otherTags'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Adds UNIQUEFILEID tag when Beatport is selected</q-tooltip>
                </q-icon>
            </q-checkbox>
            <!-- One Tagger Tag -->
            <q-checkbox class='tag checkbox text-grey-4' label='One Tagger Tag' v-model='$1t.config.value.metaTags'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Adds 1T_TAGGEDDATE tag with timestamp</q-tooltip>
                </q-icon>
            </q-checkbox>
        </div>
    </div>

    <!-- Convenience toggles -->
    <div class='row justify-center q-mb-xs'>
        <q-btn push dense color='primary' class='rounded-borders q-mx-sm q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='toggleTags("enableAll")'>Enable All</q-btn>
        <q-btn push dense color='primary' class='rounded-borders q-mx-sm q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='toggleTags("disableAll")'>Disable All</q-btn>
        <q-btn push dense color='primary' class='rounded-borders q-mx-sm q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='toggleTags("toggle")'>Toggle</q-btn>
    </div><br>
</div>
</template>

<script lang='ts' setup>
import { get1t } from '../scripts/onetagger';
import PlaylistDropZone from './PlaylistDropZone.vue';

const SUPPORTED_TAGS: Record<string, string[]> = {
    beatport: ['style', 'remixers', 'trackId', 'version', 'bpm', 'key', 'publishDate', 'other', 'isrc', 'trackTotal'],
    discogs: ['style', 'trackTotal', 'discNumber'],
    traxsource: ['trackId', 'version', 'bpm', 'key', 'trackTotal'],
    junodownload: ['bpm', 'trackTotal'],
    beatsource: ['remixers', 'trackId', 'bpm', 'key', 'version', 'isrc'],
    musicbrainz: ['isrc'],
    spotify: ['isrc', 'key'],
    bpmsupreme: ['key', 'other', 'trackId', 'mood'],
    itunes: ['trackTotal']
}
const ALL_TAGS = ['title', 'artist', 'albumArtist', 'album', 'key', 'bpm', 'genre', 'style', 
    'label', 'duration', 'releaseDate', 'publishDate', 'albumArt', 'otherTags', 'url', 'trackId', 
    'releaseId', 'version', 'remixer', 'trackNumber', 'metaTags', 'catalogNumber', 'isrc', 'mood'];

const $1t = get1t();

function browse() {
    $1t.browse('at', $1t.config.value.path);
}

// Check if tag is supported on selected platforms
function isSupported(tag: string) {
    for (let platform of $1t.config.value.platforms) {
        if (SUPPORTED_TAGS[platform] && SUPPORTED_TAGS[platform].includes(tag))
            return true;
    }
    return false;
}

// Enable/Disable/Toggle all tags
function toggleTags(mode: string) {
    for (let tag of ALL_TAGS) {
        switch (mode) {
            case 'enableAll':
                ($1t.config.value as any)[tag] = true;
                break;
            case 'disableAll':
                ($1t.config.value as any)[tag] = false;
                break;
            case 'toggle':
                ($1t.config.value as any)[tag] = !($1t.config.value as any)[tag];
                break;
        }
    }
}

</script>

<style lang='scss'>
.path {
    min-width: 80%;
}
.flex-break {
    height: 0 !important;
    flex: 1 0 100% !important;
    flex-basis: 100% !important;
}
.tag {
    width: 164px;
}
.tags {
    max-width: 40vw !important;
}
.doc-link {
    color: var(--q-color-primary);
}

.doc-link:hover {
    color: #f0f0f0;    
}
</style>