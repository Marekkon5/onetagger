<template>
<div class='index-container'>

    <!-- Background -->
    <img src='bg.png' class='bg-image' ref='bgImage'>

    <!-- Right cards -->
    <div class='getting-started-cards'>
    <div class='text-h10 text-bold q-py-sm text-grey-4'>GETTING STARTED</div>
        <q-card class='col-3 rounded-borders'>
            <q-card-section>
                <div class='col-3 text-primary text-bold'>AUTO TAG</div>
                <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm'>
                    Auto Tag lets you automatically tag from Beatport,
                    Discogs, Junodownload & Traxsource, to your local
                    audio files, based on Artist & Title tag.<br>
                    
                    <q-btn dense push color='primary' class='rounded-borders q-mt-sm text-black text-weight-medium text-caption' @click='help("autotagger")'>More info</q-btn>
                    
                </div>
            </q-card-section>
        </q-card>
        
        <q-card class='col-3 q-mt-md rounded-borders'>
            <q-card-section>
                <div class='col-3 text-primary text-bold'>AUDIO FEATURES</div>
                <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm'>
                    Audio Features lets you automatically tag Spotify’s
                    so called audio features to your local audio files,
                    based on ISRC & exact match.<br>
                    
                    <q-btn dense push color='primary' class='rounded-borders q-mt-sm text-black text-weight-medium text-caption' @click='help("audiofeatures")'>More info</q-btn>
                    
                </div>
            </q-card-section>
        </q-card>
        
        <q-card class='col-3 q-mt-md rounded-borders'>
            <q-card-section>
                <div class='col-3 text-primary text-bold'>QUICK TAG</div>
                <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm'>
                    Quick Tag lets you manually tag your local audio files, 
                    based on energy, mood, genre & other characteristics
                    you defined. Key bind compatible for quick access.<br>
                    
                    <q-btn dense push color='primary' class='rounded-borders q-mt-sm text-black text-weight-medium text-caption' @click='help("quicktag")'>More info</q-btn>
                    
                </div>
            </q-card-section>
        </q-card>
        
    </div>

    <!-- Welcome -->
    <div class='q-pl-xl q-pt-lg'>
       
        <img svg-inline src="../assets/logo.svg" height='180' width='250' class='q-mt-xs' style='margin-left: -15px; margin-bottom: -30px;'>
        
        <div class='q-pt-md text-h4 text-grey-4 text-weight-medium'>The ultimate cross-platform</div>
        <div class='q-pt-xs text-h4 text-grey-4 text-weight-medium'>music tagger for DJs</div>
        
        <div class='q-pt-lg text-subtitle1 text-bold text-grey-4'>Version {{$1t.info.version}}</div>
        <span class='dotted-underline q-pt-md text-subtitle1 text-primary clickable' @click='$1t.url("https://onetagger.github.io")'>OneTagger.github.io</span>
        <div class='q-pt-md text-subtitle1 text-grey-4'>
            Developed by Marekkon5<br>
            UI & logo design by Bas Curtiz
        </div>
        
        <div class='q-pt-lg text-subtitle1 text-grey-4'>
            Need help? Check out our <span class='text-bold'>Getting started</span> sections.<br>
            Found a bug or have a request? Post it on <span class='text-primary clickable' @click='$1t.url("https://github.com/Marekkon5/onetagger/issues")'><img src="../assets/github.png" width='64px'></span><br>
            Like the app? Contribute by donating through <span class='text-primary clickable' @click='$1t.url("https://paypal.me/marekkon5")'><img src="../assets/paypal.png" width='64px'></span> or <span class='text-primary clickable' @click='$1t.url("https://patreon.com/onetagger")'><img src="../assets/patreon.png" width='70px'></span>
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
.dotted-underline { 
    border-bottom: 1px dotted;
}
.dotted-underline:hover { 
    border-bottom: 1px dotted white;    
}
.index-container {
    overflow: hidden;
    height: calc(100vh - 109px);
}
.getting-started-cards {
    position: absolute;
    right: 48px;
    top: 128px;
    max-width: 400px;    
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