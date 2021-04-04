<template>
<div class='text-center'>
    <!-- Path -->
    <div class='text-h5 q-mt-md'>Select folder:</div>
    <div class='text-subtitle1 q-mt-xs q-pb-md'>Subfolders are included.</div>
    <div class='path q-mt-md'>
        <q-input standout='text-white bg-dark' class='text-white input' label='Path' v-model='$1t.config.path'>
            <template v-slot:append>
                <q-btn round dense flat icon='mdi-open-in-app' class='text-white' @click='browse'></q-btn>
            </template>
        </q-input>
    </div>

    <!-- Tags -->
    <div class='text-h5 q-mt-lg'>Select tags:</div>
    <div class='row tags'>
        <q-checkbox class='tag checkbox' label='Title' v-model='$1t.config.title'></q-checkbox>
        <q-checkbox class='tag checkbox' label='Artist' v-model='$1t.config.artist'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox' label='Album' v-model='$1t.config.album'></q-checkbox>
        <q-checkbox class='tag checkbox' label='Label' v-model='$1t.config.label'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox' label='BPM' v-model='$1t.config.bpm'></q-checkbox>
        <q-checkbox class='tag checkbox' label='Initial Key' v-model='$1t.config.key'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox' label='Genre' v-model='$1t.config.genre'></q-checkbox>
        <q-checkbox class='tag checkbox' label='Style' v-model='$1t.config.style'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox' label='Release Date' v-model='$1t.config.releaseDate'></q-checkbox>
        <q-checkbox class='tag checkbox' label='Publish Date' v-model='$1t.config.publishDate'></q-checkbox>
        <div class='flex-break'></div>
        <q-checkbox class='tag checkbox' label='Other (URLs)' v-model='$1t.config.otherTags'></q-checkbox>
        <q-checkbox class='tag checkbox' label='Album Art' v-model='$1t.config.albumArt'></q-checkbox>
    </div>

    <!-- Next -->
    <q-btn 
        class='q-mt-xl text-black' 
        color='primary' 
        @click='$router.push("/autotagger/3")'
        v-if='$1t.config.path'
    >Next</q-btn>

</div>
</template>

<script>
export default {
    name: 'AutotaggerTags',
    methods: {
        browse() {
            this.$1t.send('browse', {context: 'at'});
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