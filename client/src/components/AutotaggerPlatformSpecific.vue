<template>
<div class='text-center'>

    <!-- No settings available -->
    <div v-if='!beatport && !discogs'>
        <div class='text-h5 q-my-md text-grey-4' v-if='!$1t.settings.autoTaggerSinglePage'>
            No platform specific settings available for the selected platform(s)
        </div>
    </div>

    <!-- Beatport settings -->
    <div v-if='beatport' class='q-mb-xl'>
        <div class='text-h5 text-grey-4'>Beatport</div>
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
                    How many pages of search results to scan for tracks
                </q-tooltip>
            </q-chip>
            <div class='row justify-center'>
                <q-slider label-text-color='black' v-model='$1t.config.beatport.maxPages' :min='1' :max='10' label class='slider'></q-slider>
            </div>  
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
                        To obtain token, create a free account on discogs.com<br> More info? Hit <q-icon style='padding-bottom: 3px;' name='mdi-help-circle-outline'></q-icon> HELP on the right
                    </q-tooltip>
                </q-icon>
            </template>
        </q-input>
        <!-- Max results -->
        <div class='q-my-sm'>
            <q-chip text-color='black' color='primary'>Max albums to check: {{$1t.config.discogs.maxResults}}
                <q-tooltip content-style="font-size: 13px">
                    How many albums (search results) to check<br>Due to rate limiting this increases tagging time by a lot
                </q-tooltip>
            </q-chip>
            <div class='row justify-center'>
                <q-slider label-text-color='black' v-model='$1t.config.discogs.maxResults' :min='1' :max='16' label class='slider'></q-slider>
            </div>
        </div>
        <!-- Track number as int -->
        <div>
            <q-toggle v-model='$1t.config.discogs.trackNumberInt' label="Write track number as number, rather than Discogs's format"></q-toggle>
        </div>
    </div>

    <!-- Shared -->
    <div v-if='discogs || beatport' class='q-mb-xl'>
        <div class='text-h5 q-mt-md text-grey-4'>Discogs & Beatport</div>
        <!-- Styles -->
        <q-select
            dark
            standout='text-grey-4 bg-dark'
            v-model='stylesOption'
            :options='stylesOptions'
            class='select'
            label='Genres/Styles tag'
            @input='updateStyleOption'
        ></q-select>
        <!-- Styles custom tag -->
        <div v-if='$1t.config.stylesOptions == "customTag"' class='q-my-sm q-mx-md'>
            <TagFields v-model='$1t.config.stylesCustomTag' class='input'></TagFields>
        </div>
    </div>

</div>
</template>

<script>
import TagFields from './TagFields.vue';

export default {
    name: 'AutotaggerPlatformSpecific',
    components: {TagFields},
    data() {
        return {
            resolutions: [200,300,400,500,600,700,800,900,1000,1100,1200,1300,1400,1500,1600],
            stylesOptions: ["Default", "Only genres", "Only styles", "Merge to genres tag", 
                "Merge to styles tag", "Write styles to genre tag", "Write genres to style tag",
                "Write styles to custom tag"],
            values: ["default", "onlyGenres", "onlyStyles", "mergeToGenres", "mergeToStyles",
                "stylesToGenre", "genresToStyle", "customTag"],
            stylesOption: "Default"
        }
    },
    methods: {
        // Update styles options value
        updateStyleOption() {
            this.$1t.config.stylesOptions = this.values[this.stylesOptions.indexOf(this.stylesOption)];
        },
    },
    mounted() {
        this.stylesOption = this.stylesOptions[this.values.indexOf(this.$1t.config.stylesOptions)];

        // In case of null because of update
        if (!this.$1t.config.stylesCustomTag)
            this.$1t.config.stylesCustomTag = {vorbis: 'STYLE', id3: 'STYLE', mp4: 'STYLE'};

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