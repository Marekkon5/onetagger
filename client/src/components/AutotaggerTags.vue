<template>
<div class='text-center'>
    <!-- Path -->
    <div class='text-h5 text-grey-4 q-mb-md'>Select folder</div>
    <div class='row input'>
        <div class='col-1'></div>
        <q-input filled class='col-10' label='Path' v-model='$1t.config.path'>
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
    <div class='row justify-center input'>
        <div class='col-1'></div>
        <PlaylistDropZone 
            :value='$1t.autoTaggerPlaylist'
            @input='Object.assign($1t.autoTaggerPlaylist, $event)'
            class='q-my-sm q-py-md col-10' 
        ></PlaylistDropZone>
        
        <div class='col-1'>
            <q-icon name='mdi-help-circle-outline text-grey-6' class='playlist-tooltip q-mx-sm q-mt-xl q-pt-sm'>
                <q-tooltip content-style="font-size: 13px">.m3u and .m3u8 extensions are supported</q-tooltip>
            </q-icon>
        </div>
    </div>

    <!-- Tags -->
    <div class='text-h5 q-mt-lg text-grey-4'>Select tags</div>
    <div class='row justify-center q-mb-md' style='width: 100%;'>
        <div class='row justify-between q-ml-xl tags wrap'>
            <q-checkbox class='tag checkbox text-grey-4' label='Album Art' v-model='$1t.config.albumArt'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Genre' v-model='$1t.config.genre'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Album' v-model='$1t.config.album'></q-checkbox>
            <q-checkbox :disabled='!isSupported("style")' class='tag checkbox text-grey-4' label='Style / Subgenre' v-model='$1t.config.style'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Album Artist' v-model='$1t.config.albumArtist'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Label' v-model='$1t.config.label'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Artist' v-model='$1t.config.artist'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Release ID' v-model='$1t.config.releaseId'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!isSupported("remixers")' class='tag checkbox text-grey-4' label='Remixers' v-model='$1t.config.remixer'></q-checkbox>
            <q-checkbox :disabled='!isSupported("trackId")' class='tag checkbox text-grey-4' label='Track ID' v-model='$1t.config.trackId'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Title' v-model='$1t.config.title'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Catalog Number' v-model='$1t.config.catalogNumber'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!isSupported("version")' class='tag checkbox text-grey-4' label='Version' v-model='$1t.config.version'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Track Number' v-model='$1t.config.trackNumber'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!isSupported("bpm")' class='tag checkbox text-grey-4' label='BPM' v-model='$1t.config.bpm'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Duration' v-model='$1t.config.duration'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!isSupported("key")' class='tag checkbox text-grey-4' label='Key' v-model='$1t.config.key'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='URLs' v-model='$1t.config.url'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!isSupported("publishDate")' class='tag checkbox text-grey-4' label='Publish Date' v-model='$1t.config.publishDate'></q-checkbox>
            <q-checkbox :disabled='!isSupported("other")' class='tag checkbox text-grey-4' label='Other' v-model='$1t.config.otherTags'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Other tags are depending on the selected platforms</q-tooltip>
                </q-icon>
            </q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Release Date' v-model='$1t.config.releaseDate'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='One Tagger Tag' v-model='$1t.config.metaTags'>
                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs' style='margin-top: -2px;'>
                    <q-tooltip content-style="font-size: 13px">Adds a 1T_TAGGEDDATE tag with timestamp</q-tooltip>
                </q-icon>
            </q-checkbox>
            <div class='flex-break'></div>  
            <q-checkbox class='tag checkbox text-grey-4' label='ISRC' v-model='$1t.config.isrc'></q-checkbox>
     
        </div>
    </div>

    <!-- Convenience toggles -->
    <div class='row justify-center q-mb-xl'>
        <q-btn color='primary' class='q-mx-sm' @click='toggleTags("enableAll")'>Enable All</q-btn>
        <q-btn color='primary' class='q-mx-sm' @click='toggleTags("disableAll")'>Disable All</q-btn>
        <q-btn color='primary' class='q-mx-sm' @click='toggleTags("toggle")'>Toggle</q-btn>
    </div>

</div>
</template>

<script>
import PlaylistDropZone from './PlaylistDropZone.vue';

const SUPPORTED_TAGS = {
    beatport: ['style', 'remixers', 'trackId', 'version', 'bpm', 'key', 'publishDate', 'other', 'isrc'],
    discogs: ['style'],
    traxsource: ['trackId', 'version', 'bpm', 'key'],
    junodownload: ['bpm'],
    beatsource: ['remixers', 'trackId', 'bpm', 'key', 'version', 'isrc'],
    musicbrainz: ['isrc'],
    spotify: ['isrc']
}
const ALL_TAGS = ['title', 'artist', 'albumArtist', 'album', 'key', 'bpm', 'genre', 'style', 
    'label', 'duration', 'releaseDate', 'publishDate', 'albumArt', 'otherTags', 'url', 'trackId', 
    'releaseId', 'version', 'remixer', 'trackNumber', 'metaTags', 'catalogNumber', 'isrc'];

export default {
    name: 'AutotaggerTags',
    components: {PlaylistDropZone},
    methods: {
        browse() {
            this.$1t.send('browse', {context: 'at', path: this.$1t.config.path});
        },
        // Check if tag is supported on selected platforms
        isSupported(tag) {
            for (let platform of this.$1t.config.platforms) {
                if (SUPPORTED_TAGS[platform] && SUPPORTED_TAGS[platform].includes(tag))
                    return true;
            }
            return false;
        },
        // Enable/Disable/Toggle all tags
        toggleTags(mode) {
            for (let tag of ALL_TAGS) {
                switch (mode) {
                    case 'enableAll':
                        this.$1t.config[tag] = true;
                        break;
                    case 'disableAll':
                        this.$1t.config[tag] = false;
                        break;
                    case 'toggle':
                        this.$1t.config[tag] = !this.$1t.config[tag];
                        break;
                }
            }
        }
    },
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
    width: 150px;
}
.tags {
    max-width: 40vw !important;
}

</style>