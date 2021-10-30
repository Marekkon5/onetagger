<template>
<div class='index-container'>

    <!-- Background -->
    <img src='bg.png' class='bg-image' ref='bgImage'>

    <!-- Welcome -->
    <div class='flex flex-center column'>
       
        <img svg-inline src="../assets/logo.svg" width='310px' height='100%' style='margin-top: 6%;'>
        
        <div class='q-pt-lg text-h7 text-grey-4 text-weight-medium'>The ultimate cross-platform music tagger for DJs</div>
        
        
        <div class='q-pt-lg text-subtitle1 text-bold text-grey-4'>Version {{$1t.info.version}}</div>
        <span class='dotted-underline q-pt-xs text-subtitle1 text-primary clickable' @click='$1t.url("https://onetagger.github.io")'>OneTagger.github.io</span>
        <div class='q-pt-md text-h7 text-grey-4'>
            Developed by Marekkon5 & UI design by Bas Curtiz
        </div>

        <div class="getting-started-cards flex flex-center column" style="min-height: 50px; width: 100%;">
            <div class="fit row no-wrap row justify-center content-center" style="overflow: hidden;">
   
                <div class="text-subtitle1" style="overflow: auto;">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>AUTO TAG</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm'>
                                Auto Tag lets you automatically tag from Beatport,
                                Discogs, Junodownload & Traxsource, to your local
                                audio files, based on Artist & Title tag, filename or playlist.<br>
                                
                                <q-btn dense push color='primary' class='rounded-borders q-mt-md text-black text-weight-medium text-caption' @click='help("autotagger")'>More info</q-btn>
                                
                            </div>
                        </q-card-section>
                    </q-card>
                </div>

                <div class="text-subtitle1" style="overflow: auto;">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>AUDIO FEATURES</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm'>
                                Audio Features lets you automatically tag Spotify’s
                                so called audio features to your local audio files,
                                based on ISRC & exact match.
                                (Free) Spotify account required.<br>
                                <q-btn dense push color='primary' class='rounded-borders q-mt-md text-black text-weight-medium text-caption' @click='help("audiofeatures")'>More info</q-btn>
                            </div>
                        </q-card-section>
                    </q-card>
                </div>
                <div class="text-subtitle1" style="overflow: auto;">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold'>QUICK TAG</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm'>
                                Quick Tag lets you manually tag your local audio files, 
                                based on energy, mood, genre & other characteristics
                                you defined. Key bind compatible for quick access.<br>
                                
                                <q-btn dense push color='primary' class='rounded-borders q-mt-md text-black text-weight-medium text-caption' @click='help("quicktag")'>More info</q-btn>
                            </div>                
                        </q-card-section>            
                    </q-card>
                </div>

            </div>
        </div>

        <div class="flex flex-center column">
            <div class="bottom-line" style="min-height: 10px; width: 100%;">
                <div class="fit row no-wrap row justify-center content-center" style="overflow: hidden;">
                    <div class="text-subtitle1 text-grey-4" style="overflow: auto;">
                        <div style="max-width: 400px; margin-left: 0px; padding-top: 4px;">
                            Need help? Click on <span class='text-bold'>More info</span> buttons<br>
                        </div>
                    </div>
                    <div class="text-subtitle1 text-grey-4" style="overflow: auto;">
                        <div style="max-width: 400px; margin-left: 50px; padding-top: 4px;">
                            Found a bug or have a request? Post it on <span class='tiny-logo text-primary clickable' @click='$1t.url("https://github.com/Marekkon5/onetagger/issues")'><img svg-inline src="../assets/github.svg" width='64px' style='margin-bottom: 0px;'></span><br>
                        </div>
                    </div>
                    <div class="text-subtitle1 text-grey-4" style="overflow: auto;">
                        <div style="max-width: 400px; margin-left: 50px;">
                            Like the app? Donate through <span class='tiny-logo text-primary clickable' @click='$1t.url("https://paypal.me/marekkon5")'><img svg-inline src="../assets/paypal.svg" width='64px' style='margin-bottom: -2px;'></span> or <span class='tiny-logo text-primary clickable' @click='$1t.url("https://patreon.com/onetagger")'><img svg-inline src="../assets/patreon.svg" width='70px' style='margin-bottom: -2px;'></span>            
                        </div>
                    </div>
                </div>
            </div>
        </div>

    </div>

</div>
</template>

<script>
export default {
    name: 'Index',
    methods: {
        help(route) {
            this.$1t.helpDialog.route = route;
            this.$1t.helpDialog.open = true;
        }
    }
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
    height: calc(100vh - 109px);
}

.getting-started-cards {
    position: relative;
    margin-top: 30px;
    max-width: 100%;  
}
.bottom-line {
    position: absolute;
    bottom: 25px;
    max-width: 100%;    
}

.bg-image {
    max-width: 100%;    
    position: fixed;    
    z-index: -100;    
    animation: anim 30s infinite;
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