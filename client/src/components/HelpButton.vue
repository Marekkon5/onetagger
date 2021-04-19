<template>
<div>

    <!-- Button -->
    <div class='help-button-container' @click='show = true' v-if='showButton'>
        <q-btn color='primary' class='q-px-sm help-button text-bold'>
            <q-icon name='mdi-help-circle-outline' class='q-pr-sm' size='xs'></q-icon>
            Help
        </q-btn>
    </div>

    <!-- Dialog -->
    <q-dialog v-model='show'>
        <q-card class='help-card text-center'>

            <!-- Page controls -->
            <q-btn 
                class='help-page-icon clickable help-page-icon-forward'
                flat
                round
                @click='page += 1'
                v-if='page < (pages-1)'
                ><q-icon name='mdi-chevron-right' size='xl'></q-icon>
            </q-btn>
            <q-btn 
                class='help-page-icon clickable help-page-icon-back'
                flat
                round
                @click='page -= 1'
                v-if='page > 0'
                ><q-icon name='mdi-chevron-left' size='xl'></q-icon>
            </q-btn>

            <!-- Auto tagger -->
            <div v-if='route == "autotagger"'>
                <q-card-section class='q-px-xl'>
                    <div class='text-h5 text-bold text-primary'>Getting started with Auto Tag</div>
                    <div class='text-subtitle1 text-bold text-primary'>{{page+1}} / {{pages}}</div>

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 q-mt-md'>
                            Auto Tag lets you automatically tag from Beatport, Discogs, Junodownload & Traxsource,
                            to your local audio files, based on Artist & Title tag.
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>How does it work?</div>
                        <div class='text-subtitle2'>
                            It reads the Artist & Title tag from your local MP3, AIFF and FLAC files, <br>
                            feeds it into the search engines of the several platforms* <br> and writes the data to your tags.
                            <p>                       
                            <br>*<span @click='$1t.url("https://beatport.com")' class='clickable text-primary'>Beatport</span>, <span @click='$1t.url("https://junodownload.com")' class='clickable text-primary'>Juno Download</span> & <span @click='$1t.url("https://traxsource.com")' class='clickable text-primary'>Traxsource</span> are based on a method called scraping. <br>
                            <span @click='$1t.url("https://discogs.com")' class='clickable text-primary'>Discogs</span> is based on its API. Therefor it needs you to sign up for a free account.
                            </p>
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>What?</div>
                        <div class='text-subtitle2'>
                            It can fetch the following metadata (and based on the user input,
                            it can append, overwrite or fill in when empty):
                        </div>
                        <div class='row text-subtitle2 q-pt-sm' style='width: 50%; margin-left: 25%;'>
                            <div class='col-6 q-mt-xs'>TITLE</div>  <div class='col-6 q-mt-xs'>ARTIST</div>
                            <div class='col-6 q-mt-xs'>ALBUM</div>  <div class='col-6 q-mt-xs'>LABEL</div>
                            <div class='col-6 q-mt-xs'>BPM</div>  <div class='col-6 q-mt-xs'>KEY *</div>
                            <div class='col-6 q-mt-xs'>GENRE</div>  <div class='col-6 q-mt-xs'>STYLE *</div>
                            <div class='col-6 q-mt-xs'>RELEASE DATE</div>  <div class='col-6 q-mt-xs'>PUBLISH DATE *</div>
                            <div class='col-6 q-mt-xs'>OTHER URL(s)</div>  <div class='col-6 q-mt-xs'>ALBUM ART</div>
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-sm q-my-md'>
                            * Key is only available from Beatport & Traxsource <br>
                            * Style is only available from Discogs <br>
                            * Publish Date is only available from Beatport <br>                            
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>Step-by-step guide</div>
                        <div class='text-subtitle2'>
                            <span class='text-number text-bold text-grey-6'>1. </span>Use the checkbox to enable/disable to fetch from particular platform. <br>
                            <span class='text-number text-bold text-grey-6'>2. </span>Drag & drop the cards to reorder fallback <br>
                            <span class='text-number text-bold text-grey-6'>3. </span>Select folder to process (subfolders included) by clicking on the icon or copy/paste the path directly. <br>
                            <span class='text-number text-bold text-grey-6'>4. </span>Use the checkboxes to enable/disable to fetch the stated tag & hit ‘Next’. <br>
                        </div>
                        <div class='text-subtitle2 text-bold text-primary q-mt-md'>Beatport</div>
                        <div class='text-subtitle2'>
                            <span class='text-number text-bold text-grey-6'>5. </span>Select Album art resolution (500x500 is native resolution; above will upscale). <br>
                            <span class='text-number text-bold text-grey-6'>6. </span>Drag slider to determine amount of search page results to scan for the most corresponding track. <br>
                        </div>
                        <div class='text-subtitle2 text-bold text-primary q-mt-md'>Discogs</div>
                        <div class='text-subtitle2'>
                            <span class='text-number text-bold text-grey-6'>7. </span>Enter token. To obtain token, create a free account on <span @click='$1t.url("https://discogs.com")' class='clickable text-primary'>discogs.com</span> <br>
                            Go to <span @click='$1t.url("https://www.discogs.com/settings/developers")' class='clickable text-primary'>discogs.com/settings/developers</span> and click ‘Generate token’. <br>
                            Select it & copy/paste the current token. <br>
                            <span class='text-number text-bold text-grey-6'>8. </span>Select Genres/Styles tag to either fetch genre, style or both and if it should merge them, or write elsewhere. <br>
                            <span class='text-number text-bold text-grey-6'>9. </span>Drag slider to determine amount of album search results to check. Hit ‘Next’. <br>
                        </div>
                        <div class='text-subtitle2 text-bold text-primary q-mt-md'>Advanced</div>
                        <div class='text-subtitle2'>
                            <span class='text-number text-bold text-grey-6'>10. </span>Set the settings to your likings. Hover over the <q-icon name='mdi-help-circle-outline' class='q-mb-xs'></q-icon> icons for more info. <br>
                            Hit ‘Start’ to process.
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-sm q-my-md'>
                            On top it shows amount of time taken + amount not matched + amount matched. <br>
                            In the middle section it will populate which tracks were not matched. <br>
                            At the bottom a bar will grow indicating the progression. <br>
                        </div>

                    </div>

                </q-card-section>
            </div>

            <!-- Audio features -->
            <div v-if='route == "audiofeatures"'>
                <q-card-section class='q-px-xl'>
                    <div class='text-h5 text-bold text-primary'>Getting started with Audio Features</div>
                    <div class='text-subtitle1 text-bold text-primary'>{{page+1}} / {{pages}}</div>

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 q-mt-md'>
                            Audio Features lets you automatically tag Spotify’s so called audio features to your local audio files, based on ISRC & exact match.
                            
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>How does it work?</div>
                        <div class='text-subtitle2'>
                            Follow the setup described on the Audio Features entrance. <br>
                            Select a folder with tracks you want to get their audio features fetched for. <br>
                            Based on the <span class='clickable text-primary' @click='$1t.url(isrcWiki)'>ISRC</span> inside the metadata it will search up the track in Spotify's API and return these values. <br>
                            If no ISRC exists, it will search by artist and title tag using an exact match. <br>
                            Eventually it will write the audio features to the metadata and its value derived from Spotify. <br>
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>What?</div>
                        <div class='text-subtitle2 q-pb-sm'>In a nutshell audio features are:</div>
                        <div class='row text-subtitle2 text-left' style='width: 60%; margin-left: 20%;'>
                            <div class='col-4'>ACOUSTICNESS</div>     <div class='text-right col-8'>Acoustic vs. Electronic</div> 
                            <div class='col-4'>DANCEABILITY</div>     <div class='text-right col-8'>Danceable vs. Non-rhythmic</div> 
                            <div class='col-4'>ENERGY</div>           <div class='text-right col-8'>Energetic vs. Non-energetic</div> 
                            <div class='col-4'>INSTRUMENTALNESS</div> <div class='text-right col-8'>Instrumental vs. Vocal</div> 
                            <div class='col-4'>LIVENESS</div>         <div class='text-right col-8'>Live vs. Recording</div> 
                            <div class='col-4'>SPEECHINESS</div>      <div class='text-right col-8'>Speech vs. Music</div> 
                            <div class='col-4'>VALENCE</div>          <div class='text-right col-8'>Positive vs. Negative (happy vs. sad/angry)</div> 
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-sm q-my-md'>
                            For a more in-depth description, check out <span class='clickable text-primary' @click='$1t.url("https://developer.spotify.com/documentation/web-api/reference/#endpoint-get-several-audio-features")'>Spotify’s official AudioFeaturesObject</span> section.
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>Prominent tag</div>
                        <div class='text-subtitle2'>
                            You can customize the treshold, so it doesn't only write the simple value from 0-100 per audio feature, <br>
                            but also determine when it should translate the value into a meaningful description and write to a custom predefined tag field. <br>
                            Look up which tag code represents what tag name in the <span class='clickable text-primary' @click='$1t.url(metadataMatrix)'>Metadata matrix</span>. <br>
                        </div>
                        <div class='text-subtitle2 q-mt-md'>For example, a track has the following values:</div> <br>
                        <div class='row text-subtitle2 text-left' style='width: 26%; margin-left: 37%;'>
                            <div class='col-10'>ACOUSTICNESS</div>     <div class='text-right col-2'>10</div>
                            <div class='col-10'>DANCEABILITY</div>     <div class='text-right col-2'>70</div>
                            <div class='col-10'>ENERGY</div>           <div class='text-right col-2'>80</div>
                            <div class='col-10'>INSTRUMENTALNESS</div> <div class='text-right col-2'>90</div>
                            <div class='col-10'>LIVENESS</div>         <div class='text-right col-2'>20</div>
                            <div class='col-10'>SPEECHINESS</div>      <div class='text-right col-2'>10</div>
                            <div class='col-10'>VALENCE</div>          <div class='text-right col-2'>88</div>
                        </div>
                        <div class='text-subtitle2 q-mt-md'>
                            When going with the default treshold values it will write the following prominent tag: <br>
                            Electronic, Energetic, Instrumental, Positive <br><br>
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-md q-my-md'>
                            To look up audio features online for other tracks, check out <span class='text-primary clickable' @click='$1t.url("https://songdata.io")'>SongData.io</span> - Ignore the <span class='text-primary clickable' @click='$1t.url("https://www.reddit.com/r/DJs/comments/m3q97z/key_detection_comparison_spotify_vs_tunebat_vs/")'>keys determined by Spotify</span> though.
                        </div>
                    </div>

                </q-card-section>
            </div>


            <!-- Quicktag -->
            <div v-if='route == "quicktag"'>
                <q-card-section class='q-px-xl'>
                    <div class='text-h5 text-bold text-primary'>Getting started with Quick Tag</div>
                    <div class='text-subtitle1 text-bold text-primary'>{{page+1}} / {{pages}}</div>

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 q-mt-md'>
                            Quick Tag lets you manually tag your local audio files, based on energy, mood, genre & other characteristics you defined.<br>
                            Key bind compatible for quick access.
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>How does it work?</div>
                        <div class='text-subtitle2'>
                            Set the path to a folder with tracks you like to quick tag. Predefine energy, moods, genres and other characteristics. <br>
                            Decide in which tag you want them to write, so you are able to read them out in your favorite DJ software. <br>
                            Key bind them for quick access. Skip through a song to determine the mentioned elements. <br>
                            Hit the key binds (or point ’nd click).
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>Step-by-step guide</div>
                        <div class='text-subtitle2'>
                            <span class='text-number text-bold text-grey-6'>1. </span>On top right click the <q-icon name='mdi-cog' class='q-mb-xs q-pr-xs'></q-icon>icon. <br>
                            <span class='text-number text-bold text-grey-6'>2. </span>Set path to a folder which you like to quick tag. <br>
                            <span class='text-number text-bold text-grey-6'>3. </span>Predefine energy level to write to either Rating or a tag field (as symbol). <br>
                            <span class='text-number text-bold text-grey-6'>4. </span>Predefine moods (and color code them), genres and other characteristics. <br>
                            <span class='text-number text-bold text-grey-6'>5. </span>Key bind them all to a character on your keyboard for quick access. <br>
                            <span class='text-number text-bold text-grey-6'>6. </span>Decide where you want all the above data written to. <br>
                            <span class='text-number text-bold text-grey-6'>7. </span>Play a song and listen - or quickly skip through a track using the arrow keys on your keyboard. <br>
                            <span class='text-number text-bold text-grey-6'>8. </span>Tag ‘em quickly using the keyboard key binds (or point ’nd click). <br>
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-md q-my-md'>
                            For inspiration, check out the <span class='text-primary clickable' @click='$1t.url("https://www.reddit.com/r/DJs/comments/c3o2jk/my_ultimate_track_tagging_system_the_little_data/")'>Little Data, Lotta Love </span>tagging system by u/nonomomomo
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='text-subtitle1 text-bold text-primary q-mt-sm'>Energy</div>
                        <div class='text-subtitle2'>
                            Tag energy level using stars. <br>
                            Set it to 1-3-5 stars to show low-mid-high energy level, or go with the 1-2-3-4-5 scale.  <br>
                            Some DJ software can’t read out the Rating tag, or simply don’t have the ability to show it.  <br>
                            Then you can alternatively set the output to ‘symbol’ instead. Predefine the symbol, (an asterisk * for example).  <br>
                            Also predefine where to write the energy rating (Composer tag field for example) by entering the proper tag code. <br>
                            More info which tag codes are what tag field, see the <span class='clickable text-primary' @click='$1t.url(metadataMatrix)'>Metadata matrix</span>. <br>
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>Mood</div>
                        <div class='text-subtitle2'>
                            Predefine your moods and additionally color code them. Use key bindings for quick access.  <br>
                            For inspiration what moods you could use, see this <span class='text-primary clickable' @click='$1t.url(moodOverview)'>Overview of moods and other characteristics.</span> <br>
                            Since the native mood tag isn’t read by any DJ software, we need to write it elsewhere. <br>
                            More info which tag codes are what tag field, once again, see the <span class='clickable text-primary' @click='$1t.url(metadataMatrix)'>Metadata matrix</span>. <br>
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-md'>Genre</div>
                        <div class='text-subtitle2'>
                            If the genre stated in the original genre tag of your tracks (or the ones fetched with Auto Tag) are still not to your liking, <br>
                            predefine them in Settings and use key bindings for quick access. <br>
                            <div class='bg-grey-9 q-my-sm'>
                                <q-icon name='mdi-information-outline' class='q-mb-xs q-pr-xs'></q-icon>
                                Pro-tip: Use <span class='text-overline'>CTRL</span> + character for genres. This way you can keep <span class='text-overline'></span> characters for moods.
                            </div>
                        </div>
                        <div class='text-subtitle1 text-bold text-primary q-mt-sm'>Custom</div>
                        <div class='text-subtitle2'>
                            Whatever you want! For example focus on different elements like vibe, instruments, vocals, time to play or situation, etc. 
                        </div><br>
                    </div>

                </q-card-section>
            </div>

        </q-card>
    </q-dialog>

</div>
</template>

<script>
export default {
    name: 'HelpButton',
    data() {
        return {
            show: false,
            page: 0,
            metadataMatrix: 'https://docs.google.com/spreadsheets/d/1zhIJPOtYIueV72Gd81aVnbSa6dIA-azq9fnGC2rHUzo/edit?usp=sharing',
            moodOverview: 'https://docs.google.com/spreadsheets/d/1wYokScjoS5Xb1IvqFMXbSbknrXJ7bySLLihTucOS4qY/edit?usp=sharing',
            isrcWiki: 'https://en.wikipedia.org/wiki/International_Standard_Recording_Code'
        }
    },
    computed: {
        route() {
            //Global override for homescreen
            if (this.$1t.helpDialog.route) return this.$1t.helpDialog.route;
            return this.$route.path.substring(1).split("/")[0];
        },
        pages() {
            return 2;
        },
        //Show/Hide button
        showButton() {
            if (this.$1t.helpDialog.open) return false;
            if (!this.route) return false;
            if (this.$route.path.includes('/status')) return false;
            return true;
        }
    },
    watch: {
        show() {
            if (!this.show) {
                this.page = 0;
                this.$1t.helpDialog.open = false;
                this.$1t.helpDialog.route = null;
            }
        },
        '$1t.helpDialog.open'() {
            if (this.$1t.helpDialog.open) {
                this.show = true;
            }
        }
    }
}
</script>

<style>
.help-button-container {
    position: fixed;
    right: -36px;
    top: 50%;
    /* Drawers */
    z-index: 1002;
    transform: rotate(270deg);
}

.help-button {
    opacity: 0.5;
}
.help-button:hover {
    opacity: 1.0;
}

@media screen and (max-width: 1200px) {
    .help-card {
       width: 80%;
       min-width: 80%;
    }
}
@media screen and (min-width: 1200px) {
    .help-card {
        min-width: 1000px;
    }
}


.help-page-icon {
    position: absolute !important;
    top: calc(50% - 23px);
    z-index: 1000;
}
.help-page-icon-forward {
    left: calc(100% - 50px);
}
.help-page-icon-back {
    right: calc(100% - 50px);
}
</style>