<template>
<div class='index-container'>

    <!-- Background -->
    <img src='bg.png' class='bg-image' ref='bgImage'>

    <!-- Right cards -->
    <div class='getting-started-cards'>
    <div class='text-h10 text-bold q-py-sm text-grey-4'>GETTING STARTED</div>
        <q-card class='rounded-borders'>
            <q-card-section>
                <div class='text-primary text-bold'>AUTO TAG</div>
                <div class ='text-grey-4 text-subtitle2 q-pt-sm'>
                    Auto Tag lets you automatically tag from Beatport,
                    Discogs, Junodownload & Traxsource, to your local
                    audio files, based on Artist & Title tag.<br>
                    <span class='text-primary clickable' @click='help("autotagger")'>Click here for more info.</span>
                </div>
            </q-card-section>
        </q-card>

        <q-card class='q-mt-md rounded-borders'>
            <q-card-section>
                <div class='text-primary text-bold'>AUDIO FEATURES</div>
                <div class ='text-grey-4 text-subtitle2 q-pt-sm'>
                    Audio Features lets you automatically tag Spotify’s
                    so called audio features to your local audio files,
                    based on ISRC & exact match.<br>
                    <span class='text-primary clickable' @click='help("audiofeatures")'>More info? Click here.</span>
                </div>
            </q-card-section>
        </q-card>

        <q-card class='q-mt-md rounded-borders'>
            <q-card-section>
                <div class='text-primary text-bold'>QUICK TAG</div>
                <div class ='text-grey-4 text-subtitle2 q-pt-sm'>
                    Quick Tag lets you manually tag your local audio files, 
                    based on energy, mood, genre & other characteristics
                    you defined. Key bind compatible for quick access.<br>
                    <span class='text-primary clickable' @click='help("quicktag")'>Need help? Check it out here.</span>
                </div>
            </q-card-section>
        </q-card>
    </div>

    <!-- Welcome -->
    <div class='q-pl-xl q-pt-xl'>
        <div class='text-h4 text-weight-bold row'>
            <img svg-inline src="../assets/logo.svg" height='128' width='250' class='q-mt-xs' style='margin-left: 64px;'>
        </div>
        <div class='q-pt-md text-h6'>The ultimate cross-platform music tagger for DJs</div>
        <div class='q-pt-md text-subtitle1 text-bold text-grey-2'>Version {{$1t.info.version}}</div>
        <span class='q-pt-md text-subtitle1 text-primary clickable' @click='$1t.url("https://onetagger.github.io")'>OneTagger.github.io</span>
        <div class='q-pt-md text-subtitle1 text-grey-2'>
            Developed by Marekkon5<br>
            UI & logo design by Bas Curtiz
        </div>
        
        <div class='q-pt-md text-subtitle1 text-grey-4'>
            Need help? Check out our <span class='text-primary'>Getting started sections</span>.<br>
            Found a bug or have a request? Post it on <span class='text-primary clickable' @click='$1t.url("https://github.com/Marekkon5/onetagger/issues")'>GitHub</span>.<br>
            Like the app? Contribute by donating through <span class='text-primary clickable' @click='$1t.url("https://paypal.me/marekkon5")'>PayPal</span> or <span class='text-primary clickable' @click='$1t.url("https://patreon.com/onetagger")'>Patreon</span>.
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
.index-container {
    overflow: hidden;
    height: calc(100vh - 109px);
}
.getting-started-cards {
    position: absolute;
    right: 48px;
    top: 157px;
    max-width: 400px;    
}
.bg-image {
    width: 100%;
    max-height: 100%;
    max-width: 100%;
    position: fixed;
    z-index: -100;
    transform: scale(1.0);
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