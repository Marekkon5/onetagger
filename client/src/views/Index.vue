<template>
<div class='index-container'>

    <!-- Background -->
    <img src='../assets/bg.jpg' class='bg-image' ref='bgImage'>

    <!-- Welcome -->
    <div class='flex flex-center column'>
       
        <img src="../assets/logo.svg" height='150' style='margin-top: 5%;'>
        
        
        <div class='q-pt-md text-center text-h6 text-grey-4 text-weight-medium'>The ultimate cross-platform music tagger for DJs</div>
        
        <div class='q-pt-lg text-subtitle1 text-bold text-grey-4'>Version {{$1t.info.value.version}}</div>
        <span a class='doc-link text-subtitle1 clickable' @click='$1t.url("https://onetagger.github.io")'>OneTagger.github.io</span>
        <div class='q-pt-sm text-subtitle2 text-grey-4'>
            Developed by Marekkon5 & UI design by Bas Curtiz
        </div>

        <div class="getting-started-cards flex flex-center column" style="min-height: 50px; width: 100%;">
            <div class="fit row no-wrap row justify-center content-center" style="overflow: hidden;">
   
                <div class="text-subtitle1">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>AUTO TAG</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                Automatically tag your local audio files, from several online platforms,
                                based on <q-badge outline color='primary'><span class='text-white'>Artist</span></q-badge> & <q-badge outline color='primary'><span class='text-white'>Title</span></q-badge> tag, filename or playlist. <br> Or identify tracks with <b>Shazam</b>.<br>
                                
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("autotagger")'>Read more</q-btn>
                                
                            </div>
                        </q-card-section>
                    </q-card>                    
                </div>

                <div class="text-subtitle1" style="overflow: auto;">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>AUDIO FEATURES</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                Automatically tag your local audio files, with so called audio features by <b>Spotify</b>, based on <q-badge outline color='primary'><span class='text-white'>ISRC</span></q-badge> tag or exact match.<br>
                                (free) account required.<br>
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("audiofeatures")'>Read more</q-btn>
                            </div>
                        </q-card-section>
                    </q-card>
                </div>
                
                <div class="text-subtitle1" style="overflow: auto;">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>QUICK TAG</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                Manually tag your local audio files quickly
                                based on 
                                    <q-badge outline color='primary'><span class='text-white'>Energy</span></q-badge> , 
                                    <q-badge outline color='primary'><span class='text-white'>Mood</span></q-badge> , 
                                    <q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge>
                                     & other characteristics
                                you defined. <br> Key bind compatible for quick access.<br>
                                
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("quicktag")'>Read more</q-btn>
                            </div>                
                        </q-card-section>            
                    </q-card>                    
                </div>

                <div class="text-subtitle1" style="overflow: auto;">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>AUTO RENAME</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                Automatically rename your files based on tags. 
                                The template string defines the format scheme of the new filename.<br>
                                Regex compatible.<br>
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("renamer")'>Read more</q-btn>
                            </div>
                        </q-card-section>
                    </q-card>
                </div>
                
            </div>

        </div>

        <div class="row bottom-line flex-center" style="width: 100%;">
            <div class='col text-center text-subtitle2 text-grey-4' style="max-width: 400px; margin-top: 20px; margin-left: 25px; margin-right: 25px;">
                Need help? Click on <span class='text-bold'>Read more</span> buttons
            </div>
            <div class='col text-center text-subtitle2 text-grey-4' style="max-width: 400px; margin-top: 20px; margin-left: 25px; margin-right: 25px;">
                Found a bug or have a request? Post it on 
                <span class='tiny-logo text-primary clickable' @click='$1t.url("https://github.com/Marekkon5/onetagger/issues")'>
                    <img src="../assets/github.svg" width='64' style='margin-bottom: -2px;'>
                </span>
            </div>
            <div class='col text-center text-subtitle2 text-grey-4' style="max-width: 400px; margin-top: 20px; margin-left: 25px; margin-right: 25px;">
                Like the app? Donate through 
                <span class='tiny-logo text-primary clickable' @click='$1t.url("https://paypal.me/marekkon5")'>
                    <img src="../assets/paypal.svg" width='64' style='margin-bottom: -2px;'>
                </span> or 
                <span class='tiny-logo text-primary clickable' @click='$1t.url("https://patreon.com/onetagger")'>
                    <img src="../assets/patreon.svg" width='75' style='margin-bottom: -2px;'>
                </span>
            </div>
        </div>
    </div>

</div>
</template>

<script lang='ts' setup>
import { get1t } from '../scripts/onetagger.js';

const $1t = get1t();
function help(route: string) {
    $1t.helpDialog.value.route = route;
    $1t.helpDialog.value.open = true;
}
</script>

<style>
.tiny-logo {
    filter: brightness(90%)
}
.tiny-logo:hover {
    filter: brightness(1)
}
.index-container {
    overflow: hidden;
    height: calc(100vh - 59px);
}
.getting-started-cards {
    position: relative;
    margin-top: 30px;
    max-width: 100%;  
}
.bottom-line {
    position: absolute;
    bottom: 35px;    
}
.bg-image {
    max-width: 100%;    
    position: fixed;    
    z-index: -100;    
    animation: anim 30s infinite;
}
.doc-link {
    color: var(--q-primary);
}
.doc-link:hover {
    color: #f0f0f0;
}

@keyframes anim {
  0% {
    transform: scale(1,1);
  }
  50% {
    transform: scale(1.05,1.05);
  }
  100% {
    transform: scale(1,1);
  }
}
</style>