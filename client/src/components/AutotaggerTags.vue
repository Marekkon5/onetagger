<template>
<div class='text-center'>
    <!-- Path -->
    <div class='text-h5 q-mt-md text-grey-4'>Select folder</div>
    <div class='text-subtitle1 q-mt-xs q-pb-md text-grey-6'>Subfolders are included</div>
    <div class='path q-mt-md'>
        <q-input standout='text-grey-4 bg-dark' class='text-grey-4 input' label='Path' v-model='$1t.config.path'>
            <template v-slot:append>
                <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse'></q-btn>
            </template>
        </q-input>
    </div>

    <!-- Tags -->
    <div class='text-h5 q-mt-lg text-grey-4'>Select tags</div>
    <div class='row tags'>
        <q-checkbox class='tag checkbox text-grey-4' label='Title' v-model='$1t.config.title'></q-checkbox>
        <q-checkbox class='tag checkbox text-grey-4' label='Artist' v-model='$1t.config.artist'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox text-grey-4' label='Album' v-model='$1t.config.album'></q-checkbox>
        <q-checkbox class='tag checkbox text-grey-4' label='Label' v-model='$1t.config.label'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox text-grey-4' label='BPM' v-model='$1t.config.bpm'></q-checkbox>
        <q-checkbox class='tag checkbox text-grey-4' label='Initial Key' v-model='$1t.config.key'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox text-grey-4' label='Genre' v-model='$1t.config.genre'></q-checkbox>
        <q-checkbox class='tag checkbox text-grey-4' label='Style' v-model='$1t.config.style'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox text-grey-4' label='Release Date' v-model='$1t.config.releaseDate'></q-checkbox>
        <q-checkbox class='tag checkbox text-grey-4' label='Publish Date' v-model='$1t.config.publishDate'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox text-grey-4' label='Other (URLs)' v-model='$1t.config.otherTags'></q-checkbox>
        <q-checkbox class='tag checkbox text-grey-4' label='Album Art' v-model='$1t.config.albumArt'></q-checkbox>
    </div>
</div>
</template>

<script>
export default {
    name: 'AutotaggerTags',
    methods: {
        browse() {
            this.$1t.send('browse', {context: 'at', path: this.$1t.config.path});
        }
    }
}
</script>

<style lang='scss'>
.path {
    max-width: 80%;
    margin-left: 10%;
}
.flex-break {
    height: 0 !important;
    flex: 1 0 100% !important;
}
.tags {
    width: 50%;
    margin-left: 33%;
}
.tag {
    width: 50%;
}
</style>