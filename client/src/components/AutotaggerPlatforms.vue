<template>
    <div class='row justify-center q-mb-xl'>
        <draggable v-model='$1t.info.platforms' @update='syncPlatforms'>
            <q-card class='card q-ma-md' v-for='platform in $1t.info.platforms' :key='platform.id'>
                <q-card-section horizontal class='row justify-between'>
                    <q-card-section>
                        <div class='row'>
                            <q-checkbox :value='isEnabled(platform.id)' class='cb' @input='update(platform.id)'></q-checkbox>
                            <div class='text-h6 q-mt-xs'>{{platform.platform.name}}</div>
                        </div>
                        <div v-if='!dense' class='text-subtitle2 q-ml-sm text-left text-grey-6'>
                            <!-- Speed icon -->
                            <span class='q-pr-xs'>
                                <q-icon v-if='platform.platform.maxThreads == 1' name='mdi-speedometer-slow' color='red' size='xs' class='q-pb-xs'></q-icon>
                                <q-icon v-if='platform.platform.maxThreads > 1' name='mdi-speedometer-medium' color='yellow' size='xs' class='q-pb-xs'></q-icon>
                                <q-icon v-if='platform.platform.maxThreads == 0' name='mdi-speedometer' color='green' size='xs' class='q-pb-xs'></q-icon>
                            </span>

                            <span v-html='platform.platform.description'></span>
                        </div>
                        <div v-if='!platform.builtIn' class='text-grey-8 q-pl-sm text-bold monospace text-left' style='font-size: 10px;'>
                            [{{platform.id}}@{{platform.platform.version}}]
                        </div>
                    </q-card-section>
                    <q-card-section class='row'>
                        <img :src='platform.icon' :height='dense ? 40 : 50'>
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
        return {}
    },
    methods: {
        // Update config
        update(platform) {
            let i = this.$1t.config.platforms.indexOf(platform);
            if (i == -1)
                this.$1t.config.platforms.push(platform);
            else
                this.$1t.config.platforms.splice(i, 1);
        },
        // Is platform enabled
        isEnabled(platform) {
            return this.$1t.config.platforms.includes(platform);
        },
        // Sync platforms order to config
        syncPlatforms() {
            this.$1t.config.platforms = this.$1t.info.platforms.map((p) => p.id).filter((p) => this.$1t.config.platforms.includes(p));
        }
    },
    mounted() {
        this.$1t.info.platforms.sort((a, b) => {
            let x = this.$1t.config.platforms.indexOf(a.id);
            let y = this.$1t.config.platforms.indexOf(b.id);
            if (x == -1) x = 1000;
            if (y == -1) y = 1000;
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