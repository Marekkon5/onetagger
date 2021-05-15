<template>
<div class='text-center'>

    <!-- No settings available -->
    <div v-if='!beatport && !discogs'>
        <div class='text-h5 q-mt-md text-grey-4'>
            No platform specific settings available for the selected platforms!
        </div>
    </div>

    <!-- Beatport settings -->
    <div v-if='beatport' class='q-mb-xl'>
        <div class='text-h5 q-mt-md text-grey-4'>Beatport</div>
        <!-- Album art resolution -->
        <q-select 
            dark 
            standout='text-grey-4 bg-dark' 
            v-model='$1t.config.beatport.artResolution' 
            :options='resolutions' 
            class='select' 
            label='Album art resolution'
        ></q-select>
        <!-- Max pages -->
        <div class='q-my-sm'>
            <q-chip text-color='black' color='primary'>Max pages: {{$1t.config.beatport.maxPages}}
                <q-tooltip content-style="font-size: 13px">
                    How many pages of search results to scan for tracks.
                </q-tooltip>
            </q-chip>
            <q-slider label-text-color='black' v-model='$1t.config.beatport.maxPages' :min='1' :max='10' label class='slider'></q-slider>
        </div>
    </div>

    <!-- Discogs -->
    <div v-if='discogs' class='q-mb-xl'>
        <div class='text-h5 q-mt-md text-grey-4'>Discogs</div>
        <!-- Token -->
        <q-input
            dark
            standout='text-grey-4 bg-dark'
            v-model='$1t.config.discogs.token'
            class='input'
            label='Token'
        >
            <template v-slot:append>
                <q-icon name='mdi-help-circle-outline text-grey-6' size='xs'>
                    <q-tooltip content-style='font-size: 13px'>
                        To obtain token, create a free account on discogs.com. More info? Hit <q-icon style='padding-bottom: 3px;' name='mdi-help-circle-outline'></q-icon> HELP on the right.
                    </q-tooltip>
                </q-icon>
            </template>
        </q-input>
        <!-- Styles -->
        <q-select
            dark
            standout='text-grey-4 bg-dark'
            v-model='discogsStyle'
            :options='discogsStyles'
            class='select'
            label='Genres/Styles tag'
            @input='updateDiscogsStyle'
        ></q-select>
        <!-- Max results -->
        <div class='q-my-sm'>
            <q-chip text-color='black' color='primary'>Max albums to check: {{$1t.config.discogs.maxResults}}
                <q-tooltip content-style="font-size: 13px">
                    How many albums (search results) to check, due to rate limiting this increases tagging time by a lot.
                </q-tooltip>
            </q-chip>
            <q-slider label-text-color='black' v-model='$1t.config.discogs.maxResults' :min='1' :max='16' label class='slider'></q-slider>
        </div>
    </div>  

</div>
</template>

<script>
export default {
    name: 'AutotaggerPlatformSpecific',
    data() {
        return {
            resolutions: [200,300,400,500,600,700,800,900,1000,1100,1200,1300,1400,1500,1600],
            discogsStyles: ["Default", "Only genres", "Only styles", "Merge to genres tag", 
                "Merge to styles tag", "Write styles to genre tag", "Write genres to style tag"],
            discogsStyle: "Default"
        }
    },
    methods: {
        //Update discogs style value
        updateDiscogsStyle() {
            let values = ["default", "onlyGenres", "onlyStyles", "mergeToGenres", "mergeToStyles",
                "stylesToGenre", "genresToStyle"];
            this.$1t.config.discogs.styles = values[this.discogsStyles.indexOf(this.discogsStyle)];
        },
    },
    computed: {
        //If enabled
        beatport() {
            return this.$1t.config.platforms.includes('beatport');
        },
        discogs() {
            return this.$1t.config.platforms.includes('discogs');
        }
    },
}
</script>

<style>
.select {
    width: 50%;
    margin-left: 25%;
    margin-top: 8px;
}
.input {
    width: 50%;
    margin-left: 25%;
    margin-top: 8px;
}
.slider {
    max-width: 50%;
    margin-left: 25%;
}

</style>