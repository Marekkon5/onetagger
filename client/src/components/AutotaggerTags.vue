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
    <div class='row justify-center' style='width: 100%;'>
        <div class='row justify-between q-ml-xl tags wrap'>
            <q-checkbox class='tag checkbox text-grey-4' label='Title' v-model='$1t.config.title'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Artist' v-model='$1t.config.artist'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Album' v-model='$1t.config.album'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Label' v-model='$1t.config.label'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!traxsource && !beatport && !junodownload' class='tag checkbox text-grey-4' label='BPM' v-model='$1t.config.bpm'></q-checkbox>
            <q-checkbox :disabled='!traxsource && !beatport' class='tag checkbox text-grey-4' label='Initial Key' v-model='$1t.config.key'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Genre' v-model='$1t.config.genre'></q-checkbox>
            <q-checkbox :disabled='!discogs' class='tag checkbox text-grey-4' label='Style' v-model='$1t.config.style'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Release Date' v-model='$1t.config.releaseDate'></q-checkbox>
            <q-checkbox :disabled='!beatport' class='tag checkbox text-grey-4' label='Publish Date' v-model='$1t.config.publishDate'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='URLs' v-model='$1t.config.url'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Album Art' v-model='$1t.config.albumArt'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Catalog Number' v-model='$1t.config.catalogNumber'></q-checkbox>
            <q-checkbox :disabled='!beatport' class='tag checkbox text-grey-4' label='Other' v-model='$1t.config.otherTags'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!beatport && !traxsource' class='tag checkbox text-grey-4' label='Track ID' v-model='$1t.config.trackId'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Release ID' v-model='$1t.config.releaseId'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox :disabled='!beatport && !traxsource' class='tag checkbox text-grey-4' label='Version' v-model='$1t.config.version'></q-checkbox>
            <q-checkbox class='tag checkbox text-grey-4' label='Duration' v-model='$1t.config.duration'></q-checkbox>
            <div class='flex-break'></div>
            <q-checkbox class='tag checkbox text-grey-4' label='Album Artist' v-model='$1t.config.albumArtist'></q-checkbox>
        </div>
    </div>
</div>
</template>

<script>
import PlaylistDropZone from './PlaylistDropZone.vue';

export default {
    name: 'AutotaggerTags',
    components: {PlaylistDropZone},
    methods: {
        browse() {
            this.$1t.send('browse', {context: 'at', path: this.$1t.config.path});
        }
    },
    computed: {
        //Quick check if platforms enabled
        beatport() {
            return this.$1t.config.platforms.includes('beatport');
        },
        traxsource() {
            return this.$1t.config.platforms.includes('traxsource');
        },
        discogs() {
            return this.$1t.config.platforms.includes('discogs');
        },
        junodownload() {
            return this.$1t.config.platforms.includes('junodownload');
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
    width: 130px;
}
.tags {
    max-width: 40vw !important;
}

</style>