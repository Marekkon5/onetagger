<template>
<div ref='container' class='index-container' @scroll='onScroll'>
    <!-- Right cards -->
    <div class='getting-started-cards'>
    <div class='text-h10 text-bold text-grey-6 q-py-sm'>GETTING STARTED</div>
        <q-card class='rounded-borders' v-if='section != 1'>
            <q-card-section>
                <div class='text-primary text-h6'>Auto Tagger</div>
                <div class='text-subtitle2 q-pt-sm'>
                    Auto Tagger lets you automatically tag from Beatport,
                    Discogs, Junodownload & Traxsource, to your local
                    audio files, based on Artist & Title tag.<br>
                    <span class='text-primary clickable' @click='scroll(1)'>Click here for more info.</span>
                </div>
            </q-card-section>
        </q-card>

        <q-card class='q-mt-md rounded-borders' v-if='section != 2'>
            <q-card-section>
                <div class='text-primary text-h6'>Audio Features</div>
                <div class='text-subtitle2 q-pt-sm'>
                    Audio Features lets you automatically tag Spotify’s
                    so called audio features to your local audio files,
                    based on ISRC & exact match.<br>
                    <span class='text-primary clickable' @click='scroll(5)'>More info? Click here.</span>
                </div>
            </q-card-section>
        </q-card>

        <q-card class='q-mt-md rounded-borders' v-if='section != 3'>
            <q-card-section>
                <div class='text-primary text-h6'>Quick Tag</div>
                <div class='text-subtitle2 q-pt-sm'>
                    Quick Tag lets you manually tag your local audio files, 
                    based on energy, mood, genre & other characteristics
                    you defined. Key bind compatible for quick access.<br>
                    <span class='text-primary clickable' @click='scroll(6)'>Need help? Check it out here.</span>
                </div>
            </q-card-section>
        </q-card>
    </div>

    <div class='section'>
        <!-- Welcome -->
        <div class='q-pl-xl q-pt-xl'>
            <div class='text-h4 text-weight-bold row'>
                <img src="../assets/icon.png" height='32' class='q-pr-sm q-mt-xs'>
                <span>One Tagger</span>
            </div>
            <div class='q-pt-md text-h6'>The ultimate cross-platform audio tagger for DJ's</div>
            <div class='q-pt-md text-subtitle1 text-primary clickable' @click='$1t.url("https://github.com/Marekkon5/onetagger")'>https://github.com/Marekkon5/onetagger</div>
            <div class='q-pt-md text-subtitle1'>
                Developed by Marekkon5<br>
                UI & logo design by Bas Curtiz
            </div>
            <div class='q-pt-md text-subtitle1 text-weight-bold'>Version 1.0</div>
            <div class='q-pt-md text-subtitle1'>
                Need help? Check out our <span class='text-weight-bold'>Getting started sections</span>.<br>
                Found a bug or have a request? Post it on <span class='text-primary clickable'>Github</span>.<br>
                Like the app? Contribute by donating through <span class='text-primary clickable'>Patreon.</span>
            </div>
        </div>

    </div>

    <div class='section'>
        <h1>Section #AT1</h1>
    </div>

    <div class='section'>
        <h1>Section #AT2</h1>
    </div>

    <div class='section'>
        <h1>Section #AT3</h1>
    </div>

    <div class='section'>
        <h1>Section #AT4</h1>
    </div>

    <div class='section'>
        <h1>Section #AF1</h1>
    </div>

    <div class='section'>
        <h1>Section #QT1</h1>
    </div>

    <div class='section'>
        <h1>Section #QT2</h1>
    </div>

    <div class='section'>
        <h1>Section #QT3</h1>
    </div>


</div>
</template>

<script>
export default {
    name: 'Index',
    data() {
        return {
            section: 0
        }
    },
    methods: {
        //Scroll to section
        scroll(i) {
            let offset = (window.innerHeight - 54.5) * i;
            this.$refs.container.scrollTo({
                top: offset,
                behavior: 'smooth'
            });
        },
        //Capture scroll event
        onScroll() {
            let section = Math.round(this.$refs.container.scrollTop / (window.innerHeight - 54.5));
            const ranges = [[0, 1], [1, 5], [5, 6], [6, 99]];
            for(let i=0; i<ranges.length; i++) {
                if (section >= ranges[i][0] && section < ranges[i][1]) {
                    this.section = i;
                    break;
                }
            }
        }
    }
}
</script>

<style>
.index-container {
    overflow-x: hidden;
    height: calc(100vh - 109px);
    background-image: url("../assets/bg.png");
    background-repeat: no-repeat;
    background-size: contain;    
}
.getting-started-cards {
    position: absolute;
    right: 48px;
    top: 157px;
    max-width: 400px;
}
.section {
    height: 100%;
}
</style>