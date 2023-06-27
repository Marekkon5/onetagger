<template>
    <div class='row justify-center q-mb-xl'>
        <draggable v-model='$1t.info.value.platforms' @update='syncPlatforms' item-key='id'>
            <template #item='{ element: platform }'>
                <q-card flat class='card q-ma-md'>
                    <q-card-section horizontal class='row justify-between'>
                        <q-card-section>
                            <div class='row'>
                                <q-checkbox :model-value='isEnabled(platform.id)' class='cb' @update:model-value='update(platform.id)'></q-checkbox>
                                <div class='text-h6 q-mt-xs'>
                                    {{platform.platform.name}}
                                    
                                    <!-- Speed icon -->
                                    <span>
                                        <span class='q-pl-xs text-grey-8'>
                                            <q-icon v-if='platform.platform.maxThreads == 1' name='mdi-speedometer-slow' size='xs' class='q-pb-xs'></q-icon>
                                            <q-icon v-if='platform.platform.maxThreads > 1' name='mdi-speedometer-medium' size='xs' class='q-pb-xs'></q-icon>
                                            <q-icon v-if='platform.platform.maxThreads == 0' name='mdi-speedometer' size='xs' class='q-pb-xs'></q-icon>
                                        </span>
                                        <q-tooltip>
                                            <span v-if='platform.platform.maxThreads'>This platform allows up to {{platform.platform.maxThreads}} concurrent searches</span>
                                            <span v-else>This platform allows unlimited concurrent searches</span>
                                        </q-tooltip>
                                    </span>

                                    <!-- Auth info -->
                                    <span v-if='platform.platform.requiresAuth'>
                                        <span class='q-pl-xs text-grey-8'>
                                            <q-icon name='mdi-lock' size='xs' class='q-pb-xs'></q-icon>
                                        </span>
                                        <q-tooltip>
                                            <span>Platform requires an account</span>
                                        </q-tooltip>
                                    </span>

                                    <!-- Lyrics icon -->
                                    <span v-if='hasLyrics(platform)'>
                                        <span class='q-pl-xs text-grey-8'>
                                            <q-icon name='mdi-microphone' size='xs' class='q-pb-xs'></q-icon>
                                        </span>
                                        <q-tooltip>
                                            <span>Platform can fetch lyrics</span>
                                        </q-tooltip>
                                    </span>
    
                                </div>
                            </div>
                            <div v-if='!dense' class='text-subtitle2 q-ml-sm text-left text-grey-6'>
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
            </template>
        </draggable>        
    </div>    
</template>

<script lang='ts' setup>
import { onMounted } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import draggable from 'vuedraggable';
import { AutotaggerPlatform, SupportedTag } from '../scripts/autotagger';


const { dense } = defineProps({
    dense: { type: Boolean, default: false }
});
const $1t = get1t();

// Update config
function update(platform: string) {
    let i = $1t.config.value.platforms.indexOf(platform);
    if (i == -1)
        $1t.config.value.platforms.push(platform);
    else
        $1t.config.value.platforms.splice(i, 1);
}

// Is platform enabled
function isEnabled(platform: string) {
    return $1t.config.value.platforms.includes(platform);
}

// Sync platforms order to config
function syncPlatforms() {
    $1t.config.value.platforms = $1t.info.value.platforms.map((p) => p.id).filter((p) => $1t.config.value.platforms.includes(p));
}

/// Does the platform have lyrics
function hasLyrics(platform: AutotaggerPlatform) {
    return platform.supportedTags.includes(SupportedTag.UnsyncedLyrics) || platform.supportedTags.includes(SupportedTag.SyncedLyrics);
}

onMounted(() => {
    $1t.info.value.platforms.sort((a, b) => {
        let x = $1t.config.value.platforms.indexOf(a.id);
        let y = $1t.config.value.platforms.indexOf(b.id);
        if (x == -1) x = 1000;
        if (y == -1) y = 1000;
        return x - y;
    });
});

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