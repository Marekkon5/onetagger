<template>
<div class='text-center'>

    <div class='text-h5 q-mt-xl'>Select platforms:</div>
    <div class='text-subtitle-1 q-mt-xs'>Use the checkbox to enable/disable, drag and drop to reorder fallback.</div>

    <!-- Platforms -->
    <div class='cards'>
        <draggable v-model='platforms' @change='update'>
            <q-card class='card q-ma-md' v-for='platform in platforms' :key='platform.value'>
                <q-card-section horizontal class='row justify-between'>
                    <q-card-section>
                        <div class='row'>
                            <q-checkbox v-model='platform.enabled' class='cb' @input='update'></q-checkbox>
                            <div class='text-h5 q-mt-xs'>{{platform.name}}</div>
                        </div>
                        <div class='text-subtitle1 q-ml-sm text-left' v-html='platform.description'></div>
                    </q-card-section>
                    <q-card-section class='right'>
                        <img :src='platform.image' height='64'>
                    </q-card-section>
                </q-card-section>
            </q-card>
        </draggable>
    </div>

    <!-- Next -->
    <q-btn color='primary text-black q-mt-sm q-mb-md' @click='$router.push("/autotagger/2")' v-if='allowNext'>Next</q-btn>

</div>
</template>

<script>
import draggable from 'vuedraggable';

export default {
    name: 'Autotagger',
    components: {draggable},
    data() {
        return {
            platforms: [
                {
                    name: 'Beatport',
                    value: 'beatport',
                    enabled: false,
                    description: 'Overall more specialized in Techno. <br>Genre specialty: Bass House, Big Room, Electro (Classic/Detroit/Modern), Future House, Melodic House & Techno, Techno (Peak Time/Driving), Techno (Raw/Deep/Hypnotic)',
                    image: require('../assets/beatport.png')
                },
                {
                    name: 'Traxsource',
                    value: 'traxsource',
                    enabled: false,
                    description: 'Overall more specialized in House. <br>Genre specialty: Acapella, Afro/Latin/Brazillian, Classic House, Lounge/Chill Out, Soul/Funk/Disco, Soulful House',
                    image: require('../assets/traxsource.png')
                },
                {
                    name: 'Discogs',
                    value: 'discogs',
                    enabled: false,
                    description: 'Most variety in genres. <br>Also ability to fetch styles narrated by Discogs Contributors. <br>Due API rate limits, the process is slow (~20 tracks / minute).',
                    image: require('../assets/discogs.png')
                },
                {
                    name: 'Juno Download',
                    value: 'junodownload',
                    enabled: false,
                    description: 'Overall a mixed bag with additionally a lot of niche genres. <br>Genre specialty: Ambient/Drone, Coldwave/Synth, Dancehall/Ragga, Dub, Footwork/Juke, Gabba, Industrial/Noise, Jazz, Pop Trance, Reggae, Rock, Scouse House, Soundtracks, UK Hardcore',
                    image: require('../assets/junodownload.png')
                }
            ]
        }
    },
    methods: {
        //Update config
        update() {
            this.$1t.config.platforms = this.platforms.filter(p => p.enabled).map(p => p.value);
        }
    },
    computed: {
        //At least one platform selected
        allowNext() {
            if (this.platforms.find(p => p.enabled)) {
                return true;
            }
            return false;
        }
    }
}
</script>

<style lang='scss'>
.cards {
    display: flex;
    justify-content: center;
    margin-top: 16px;
}
.card {
    width: 500px;
    user-select: none;
}
.right {
    display: flex;
}
.cb svg {
    color: #000;
}
</style>