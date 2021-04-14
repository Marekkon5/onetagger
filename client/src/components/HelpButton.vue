<template>
<div>

    <!-- Button -->
    <div class='help-button-container' @click='show = true' v-if='route'>
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
                    <div class='text-h5 text-bold text-primary'>Getting started with Auto Tagger</div>
                    <div class='text-subtitle1 text-bold text-primary'>{{page+1}} / {{pages}}</div>

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle1 q-my-md'>
                            Auto Tagger lets you automatically tag from Beatport, Discogs, Junodownload & Traxsource,
                            to your local audio files, based on Artist & Title tag.
                        </div>
                        <div class='text-h6 text-bold q-mb-sm'>How does it work?</div>
                        <div class='text-subtitle1'>
                        It reads the Artist & Title tag from your local MP3, AIFF and FLAC files,
                        feeds it into the search engines of the several platforms* and writes the data.
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-sm q-my-sm'>
                        *Beatport, Junodownload & Traxsource are based on a method called scraping. 
                        Discogs is based on its API. Therefor it needs you to sign up for a free account.
                        </div>
                        <div class='text-h6 text-bold q-mb-sm q-mt-md'>What?</div>
                        <div class='text-subtitle1'>
                        It can fetch the following metadata (and based on the user input,
                        it can append, overwrite or fill in when empty):
                        </div>
                        <div class='row text-subtitle1 q-pt-sm' style='width: 50%; margin-left: 25%;'>
                            <div class='col-6'>Title</div>  <div class='col-6'>Artist</div>
                            <div class='col-6'>Album</div>  <div class='col-6'>Label</div>
                            <div class='col-6'>BPM</div>  <div class='col-6'>Key*</div>
                            <div class='col-6'>Genre</div>  <div class='col-6'>Style*</div>
                            <div class='col-6'>Release Date</div>  <div class='col-6'>Publish Date*</div>
                            <div class='col-6'>Other URL(s)</div>  <div class='col-6'>Album Art</div>
                        </div>
                        <div class='bg-grey-9 text-subtitle2 q-px-md q-py-sm q-my-md'>
                        *Style is only available from Discogs <br>
                        *Publish Date is only available from Beatport <br>
                        *Key is only available from Beatport & Traxsource <br>
                        </div>
                    </div>

                </q-card-section>
            </div>

            <!-- Autotagger page 2 -->
            <div v-if='route == "autotagger"'>

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
            page: 0
        }
    },
    computed: {
        route() {
            return this.$route.path.substring(1).split("/")[0];
        },
        pages() {
            if (this.route == 'autotagger') return 2;
            return 0;
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
    opacity: 50%;
}
.help-button:hover {
    opacity: 100%;
}

.help-card {
    min-width: 80%;
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