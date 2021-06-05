<template>
    <div class='row justify-center q-mt-md'>
        <draggable v-model='platforms' @change='update'>
            <q-card class='card q-ma-md' v-for='platform in platforms' :key='platform.value'>
                <q-card-section horizontal class='row justify-between'>
                    <q-card-section>
                        <div class='row'>
                            <q-checkbox v-model='platform.enabled' class='cb' @input='update'></q-checkbox>
                            <div class='text-h6 q-mt-xs'>{{platform.name}}</div>
                        </div>
                        <div v-if='!dense' class='text-subtitle2 q-ml-sm text-left text-grey-6' v-html='platform.description'></div>
                    </q-card-section>
                    <q-card-section class='row'>
                        <img :src='platform.image' :height='dense ? 40 : 50'>
                    </q-card-section>
                </q-card-section>
            </q-card>
        </draggable>
    </div>
</template>

<script>
import draggable from 'vuedraggable';

export default {
    name: 'AutotaggerPlatforms',
    components: {draggable},
    props: {
        dense: {
            type: Boolean,
            default: false
        }
    },
    data() {
        return {
            platforms: [
                {
                    name: 'Beatport',
                    value: 'beatport',
                    enabled: false,
                    description: 'Overall more specialized in Techno',
                    image: require('../assets/beatport.png')
                },
                {
                    name: 'Traxsource',
                    value: 'traxsource',
                    enabled: false,
                    description: 'Overall more specialized in House',
                    image: require('../assets/traxsource.png')
                },
                {
                    name: 'Juno Download',
                    value: 'junodownload',
                    enabled: false,
                    description: 'Overall a mixed bag with a lot of niche genres<br><b class="text-subtitle3 text-grey-4">Process is slow due rate limits (~20 tracks / min)</b>',
                    image: require('../assets/junodownload.png')
                },
                {
                    name: 'Discogs',
                    value: 'discogs',
                    enabled: false,
                    description: 'Most variety in genres / styles<br><b class="text-subtitle3 text-grey-4">Process is slow due rate limits (~25 tracks / min) & requires a free account</b>',
                    image: require('../assets/discogs.png')
                },
            ]
        }
    },
    methods: {
        //Update config
        update() {
            this.$1t.config.platforms = this.platforms.filter(p => p.enabled).map(p => p.value);
        }
    },
    mounted() {
        //Load enabled platforms
        for (let i=0; i<this.platforms.length; i++) {
            this.platforms[i].enabled = this.$1t.config.platforms.includes(this.platforms[i].value);
        }
        //Sort, disabled on bottom
        this.platforms.sort((a, b) => {
            let x = this.$1t.config.platforms.indexOf(a.value);
            let y = this.$1t.config.platforms.indexOf(b.value);
            if (x == -1) x = 100;
            if (y == -1) y = 100;
            return x - y;
        });
    }
}
</script>

<style lang='scss'>
.card {
    max-width: 500px;
    min-width: 400px;
    user-select: none;
}
.cb svg {
    color: #000;
}
.text-subtitle3 {
    font-size: 12px;
}
</style>