
<template>
<div class='text-center'>

    <!-- Path -->
    <div class='text-subtitle1 text-bold text-primary'>SELECT INPUT</div>
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
                <q-tooltip>Subfolders are included</q-tooltip>
            </q-icon>
        </div>
    </div>

    <!-- Drag and drop -->
    <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
        <div class='col-1'></div>
        <PlaylistDropZone 
            :value='$1t.autoTaggerPlaylist'
            @update:model-value='(p) => $1t.autoTaggerPlaylist.value = p'
            class='q-my-sm q-pt-md q-pb-lg col-10'            
        ></PlaylistDropZone>
                
        <div class='col-1'>
            <q-icon name='mdi-help-circle-outline text-grey-6' class='playlist-tooltip q-mx-sm q-mt-xl q-pt-sm'>
                <q-tooltip>.m3u and .m3u8 extensions are supported</q-tooltip>
            </q-icon>
        </div>
    </div>
    
    <!-- Tags -->
    <q-separator class='q-mx-auto custom-separator' :style='"margin-top: 16px;"' inset color="dark"/>
    <div class='text-subtitle1 q-mt-lg text-primary text-bold' style='margin-top: 35px;'>SELECT TAGS</div>
    <div class='text-subtitle2 q-mb-sm text-grey-6'>Check the box to fetch stated tag</div>
    
    <div class='q-pt-xs q-mb-md' :style='"max-width: 550px; margin: auto;"'>
        <div class='row justify-between q-ml-xl tags wrap'>

            <!-- All tags -->
            <div v-for='tag in TAGS'>
                <q-checkbox 
                    :disable='!isSupported(tag.tag)'
                    class='tag checkbox text-grey-4' 
                    :label='tag.label'
                    :model-value='$1t.config.value.tags.includes(tag.tag)'
                    @update:model-value="toggleTag(tag.tag)"
                >
                    <q-icon v-if='tag.tooltip' name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs mt-n2' >
                        <q-tooltip>{{tag.tooltip}}</q-tooltip>
                    </q-icon>
                </q-checkbox>
            </div>

        </div>
    </div>

    <!-- Convenience toggles -->
    <div class='row justify-center q-mb-xs'>
        <q-btn push dense color='primary' class='rounded-borders q-mx-sm q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='toggleTags("enableAll")'>Enable All</q-btn>
        <q-btn push dense color='primary' class='rounded-borders q-mx-sm q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='toggleTags("disableAll")'>Disable All</q-btn>
        <q-btn push dense color='primary' class='rounded-borders q-mx-sm q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='toggleTags("toggle")'>Toggle</q-btn>
    </div>
    <br>
</div>
</template>

<script lang='ts' setup>
import { get1t } from '../scripts/onetagger';
import { SupportedTag } from '../scripts/autotagger';
import PlaylistDropZone from './PlaylistDropZone.vue';

const TAGS = [
    { tag: SupportedTag.AlbumArt, label: 'Album Art', tooltip: 'Resolution is platform dependent' },
    { tag: SupportedTag.Album, label: 'Album' },
    { tag: SupportedTag.AlbumArtist, label: 'Album Artist' },
    { tag: SupportedTag.Artist, label: 'Artist' },
    { tag: SupportedTag.Title, label: 'Title' },
    { tag: SupportedTag.Version, label: 'Version' },
    { tag: SupportedTag.Remixer, label: 'Remixers', tooltip: 'Available from Beatport & Beatsource' },
    { tag: SupportedTag.Genre, label: 'Genre', tooltip: 'Spotify will populate multiple genres based on artist' },
    { tag: SupportedTag.Style, label: 'Style / Subgenre', tooltip: 'Style is available from Discogs & Bandcamp, Subgenre from Beatport only'},
    { tag: SupportedTag.Label, label: 'Label' },
    { tag: SupportedTag.ReleaseId, label: 'Release ID' },
    { tag: SupportedTag.TrackId, label: 'Track ID' },
    { tag: SupportedTag.BPM, label: 'BPM' },
    { tag: SupportedTag.Key, label: 'Key' },
    { tag: SupportedTag.Mood, label: 'Mood' },
    { tag: SupportedTag.CatalogNumber, label: 'Catalog Number' },
    { tag: SupportedTag.TrackNumber, label: 'Track Number' },
    { tag: SupportedTag.DiscNumber, label: 'Disc Number' },
    { tag: SupportedTag.Duration, label: 'Duration' },
    { tag: SupportedTag.TrackTotal, label: 'Track Total' },
    { tag: SupportedTag.ISRC, label: 'ISRC' },
    { tag: SupportedTag.PublishDate, label: 'Publish Date', tooltip: 'Available from Beatport only' },
    { tag: SupportedTag.ReleaseDate, label: 'Release Date' },
    { tag: SupportedTag.URL, label: 'URL' },
    { tag: SupportedTag.OtherTags, label: 'Other Tags', tooltip: 'Specific tags only for some platforms (Beatport, Discogs)' },
    { tag: SupportedTag.MetaTags, label: 'OneTagger Tags', tooltip: 'Adds 1T_TAGGEDDATE tag with timestamp' },
    { tag: SupportedTag.UnsyncedLyrics, label: 'Unsynced Lyrics' },
    { tag: SupportedTag.SyncedLyrics, label: 'Synced Lyrics' },
    { tag: SupportedTag.Explicit, label: 'Explicit' },
];

const $1t = get1t();

function browse() {
    $1t.browse('at', $1t.config.value.path);
}

/// Check if tag is supported on selected platforms
function isSupported(tag: SupportedTag) {
    if (tag == SupportedTag.MetaTags) return true;
    return ($1t.info.value.platforms.find(p => $1t.config.value.platforms.includes(p.id) && p.supportedTags.includes(tag))) != null;
}

/// Toggle tag on and off
function toggleTag(tag: SupportedTag) {
    let i = $1t.config.value.tags.indexOf(tag)
    if (i == -1) {
        $1t.config.value.tags.push(tag);
        return
    }
    $1t.config.value.tags.splice(i, 1);
}

/// Enable/Disable/Toggle all tags
function toggleTags(mode: string) {
    switch (mode) {
        case 'enableAll':
            $1t.config.value.tags = TAGS.map(t => t.tag);
            break;
        case 'disableAll':
            $1t.config.value.tags.length = 0;
            break;
        case 'toggle':
            $1t.config.value.tags = TAGS.map(t => t.tag).filter(t => !$1t.config.value.tags.includes(t));
            break;
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

.q-checkbox[aria-disabled=true] {
    color: #666666 !important;
    .q-checkbox__bg {
        opacity: 0.4 !important;
    }
}

.mt-n2 {
    margin-top: -2px;
}
</style>