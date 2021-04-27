<template>
<div class='index-container' @mouseenter='mouseEnter' @mouseleave="mouseLeave">

    <!-- Background -->
    <img src='../assets/bg.png' class='bg-image' ref='bgImage'>

    <!-- Right cards -->
    <div class='getting-started-cards'>
    <div class='text-h10 text-bold q-py-sm'>GETTING STARTED</div>
        <q-card class='rounded-borders'>
            <q-card-section>
                <div class='text-primary text-h6'>Auto Tag</div>
                <div class='text-subtitle2 q-pt-sm'>
                    Auto Tag lets you automatically tag from Beatport,
                    Discogs, Junodownload & Traxsource, to your local
                    audio files, based on Artist & Title tag.<br>
                    <span class='text-primary clickable' @click='help("autotagger")'>Click here for more info.</span>
                </div>
            </q-card-section>
        </q-card>

        <q-card class='q-mt-md rounded-borders'>
            <q-card-section>
                <div class='text-primary text-h6'>Audio Features</div>
                <div class='text-subtitle2 q-pt-sm'>
                    Audio Features lets you automatically tag Spotify’s
                    so called audio features to your local audio files,
                    based on ISRC & exact match.<br>
                    <span class='text-primary clickable' @click='help("audiofeatures")'>More info? Click here.</span>
                </div>
            </q-card-section>
        </q-card>

        <q-card class='q-mt-md rounded-borders'>
            <q-card-section>
                <div class='text-primary text-h6'>Quick Tag</div>
                <div class='text-subtitle2 q-pt-sm'>
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
            <img src="../assets/logo-full.png" height='64' class='q-pr-sm q-mt-xs'>            
        </div>
        <div class='q-pt-md text-h6'>The ultimate cross-platform audio tagger for DJs</div>
        <span class='q-pt-md text-subtitle1 text-primary clickable' @click='$1t.url("https://github.com/Marekkon5/onetagger")'>https://github.com/Marekkon5/onetagger</span>
        <div class='q-pt-md text-subtitle1'>
            Developed by Marekkon5<br>
            UI & logo design by Bas Curtiz
        </div>
        <div class='q-pt-md text-subtitle1 text-weight-bold'>Version 1.0</div>
        <div class='q-pt-md text-subtitle1'>
            Need help? Check out our <span class='text-weight-bold'>Getting started sections</span>.<br>
            Found a bug or have a request? Post it on <span class='text-primary clickable'>Github</span>.<br>
            Like the app? Contribute by donating through <span class='text-primary clickable'>Patreon</span>.
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
        },
        mouseEnter() {
            this.$refs.bgImage.classList.remove('scale-down-anim');
            this.$refs.bgImage.classList.add('scale-up-anim');
        },
        mouseLeave() {
            this.$refs.bgImage.classList.add('scale-down-anim');
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
    transform: scale(1.1);
}

.scale-up-anim {
    animation-name: scale-up;
    animation-duration: 0.5s;
    animation-fill-mode: forwards;
}

.scale-down-anim {
    animation-name: scale-down;
    animation-duration: 0.5s;
    animation-fill-mode: forwards;
}

@keyframes scale-up {
    from { transform: scale(1.1); }
    to   { transform: scale(1.2); }
}

@keyframes scale-down {
    from { transform: scale(1.2); }
    to   { transform: scale(1.1); }
}

</style>