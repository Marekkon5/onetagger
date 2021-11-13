<template>
<div class='text-center'>

    <!-- No settings available -->
    <div v-if='!beatport && !discogs && !beatsource && !spotify'>
        <div class='text-subtitle1 q-my-md text-grey-4' v-if='!$1t.settings.autoTaggerSinglePage'>
            No platform specific settings available for the selected platform(s)
        </div>
    </div>

    <!-- Beatport settings -->
    <div v-if='beatport'>
        <div class='text-h6 text-grey-4' style='margin-top: 1px;'>Beatport</div>
        <div class='text-subtitle2 text-grey-6 q-mb-md'>Select album art resolution. Drag slider to set amount of search page results to scan for</div>
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
        <div class='q-my-lg'>
            <q-chip text-color='black' color='primary'>Max pages: {{$1t.config.beatport.maxPages}}
                <q-tooltip content-style="font-size: 13px">
                    How many pages of search results to scan for tracks
                </q-tooltip>
            </q-chip>
            <div class='row justify-center'>
                <q-slider 
                    label-text-color='black' 
                    v-model='$1t.config.beatport.maxPages' 
                    :min='1' 
                    :max='10' 
                    label 
                    class='slider q-my-sm q-pb-lg col-10'
                    style='margin-bottom: 13px;'
                ></q-slider>
            </div>  
        </div>        
        <q-separator class='q-mx-auto custom-separator' inset color="dark"/>
    </div>

    
    <!-- Discogs -->
    <div v-if='discogs'>
        <div class='text-h6 q-mt-lg text-grey-4 custom-margin-1'>Discogs</div>
        <div class='text-subtitle2 text-grey-6 q-mb-md'>Copy/paste Discogs token. Drag slider to set amount of album search results to check</div>
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
                        To obtain token, create a free account on Discogs. More info? Click <q-icon style='padding-bottom: 4px;' name='mdi-help-circle-outline'></q-icon> HELP on the right
                    </q-tooltip>
                </q-icon>
            </template>
        </q-input>
        <!-- Max results -->
        <div class='q-my-lg'>
            <q-chip text-color='black' color='primary'>Max albums to check: {{$1t.config.discogs.maxResults}}
                <q-tooltip content-style="font-size: 13px">
                    How many albums in search results to check. Due to rate limiting this increases tagging time by a lot
                </q-tooltip>
            </q-chip>
            <div class='row justify-center'>
                <q-slider label-text-color='black' v-model='$1t.config.discogs.maxResults' :min='1' :max='16' label class='slider q-my-sm q-pb-lg col-10'></q-slider>
            </div>
        </div>
        <!-- Track number as int -->
        <div>
            <q-toggle style='margin-bottom: 10px;' v-model='$1t.config.discogs.trackNumberInt' label="Write track number as number, rather than Discogs's format"><br></q-toggle>
        </div>
        <q-separator class='q-mx-auto q-mt-lg custom-separator' inset color="dark"/>
    </div>

    <!-- Shared -->
    <div v-if='discogs || beatport'>
        <div class='text-h6 q-mt-lg text-grey-4 custom-margin-1'>Discogs / Beatport</div>
        <div class='text-subtitle2 text-grey-6 q-mb-md'>Select Genres/Styles tag to fetch both, if it should merge them, or write elsewhere</div>
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
        <div v-if='$1t.config.stylesOptions == "customTag"'>
            <TagFields v-model='$1t.config.stylesCustomTag' class='input' style='margin-bottom: 28px;'></TagFields>
        </div>
        <q-separator class='q-mx-auto q-mt-lg custom-separator' inset color="dark"/>
    </div>

    <!-- Beatsource settings -->
    <div v-if='beatsource'>
        <div class='text-h6 q-mt-lg text-grey-4 custom-margin-1'>Beatsource</div>
        <div class='text-subtitle2 text-grey-6 q-mb-md'>Select album art resolution</div>
        <!-- Album art resolution -->
        <q-select 
            dark 
            standout='text-grey-4 bg-dark' 
            v-model='$1t.config.beatsource.artResolution' 
            :options='resolutions' 
            class='select' 
            label='Album art resolution'
            style='margin-bottom: 3px;'
        ></q-select>
        <p><br></p>
        <q-separator class='q-mx-auto q-mt-lg custom-separator' inset color="dark"/>
    </div>

    <!-- Spotify -->
    <div v-if='spotify'>
        <div class='text-h6 q-mt-lg text-grey-4 custom-margin-1'>Spotify</div>
        <div class='justify-center' style='max-width: 836px; margin: auto;'>
            <SpotifyLogin v-if='!$1t.spotify.authorized'></SpotifyLogin>
        </div>
        <div v-if='$1t.spotify.authorized'>
            <div class='q-mt-xs text-h7 text-primary'>You are successfully logged in to Spotify</div>
        </div>
        <br>
    </div>


</div>
</template>

<script>
import TagFields from './TagFields.vue';
import SpotifyLogin from './SpotifyLogin.vue';

export default {
    name: 'AutotaggerPlatformSpecific',
    components: {TagFields, SpotifyLogin},
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
        // If enabled
        beatport() {
            return this.$1t.config.platforms.includes('beatport');
        },
        discogs() {
            return this.$1t.config.platforms.includes('discogs');
        },
        beatsource() {
            return this.$1t.config.platforms.includes('beatsource');
        },
        spotify() {
            return this.$1t.config.platforms.includes('spotify');
        }
    },
}
</script>

<style>
.custom-separator {
    max-width: 550px;
    margin: auto;
}
.custom-margin-1 {
    margin-top: 34px !important;
}
</style>