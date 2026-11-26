<template>
    <div class='row justify-center'>
        <draggable v-model='providerCards' @update='syncPlatforms()' :item-key='platformKey'
            filter='.q-checkbox' :prevent-on-filter='false'>
            <template #item='{ element: platform }'>
                <q-card flat class='card q-ma-md'>
                    <q-card-section horizontal class='row justify-between'>
                        <q-card-section>
                            <div class='row'>
                                <q-checkbox :model-value='isEnabled(platform.platform.id)' :aria-label='platform.platform.name' class='cb' @update:model-value='update(platform.platform.id)'></q-checkbox>
                                <div class='text-h6 q-mt-xs'>
                                    {{platform.platform.name}}

                                    <!-- Speed icon -->
                                    <span>
                                        <span class='q-pl-xs text-grey-7'>
                                            <q-icon v-if='platform.platform.maxThreads == 1' name='mdi-speedometer-slow' size='xs' class='q-pb-xs'></q-icon>
                                            <q-icon v-if='platform.platform.maxThreads > 1' name='mdi-speedometer-medium' size='xs' class='q-pb-xs'></q-icon>
                                            <q-icon v-if='platform.platform.maxThreads == 0' name='mdi-speedometer' size='xs' class='q-pb-xs'></q-icon>
                                        </span>
                                        <q-tooltip>
                                            <span v-if='platform.platform.maxThreads'>{{ t('autotag.concurrentSearches', { count: platform.platform.maxThreads }) }}</span>
                                            <span v-else>{{ t('autotag.unlimitedSearches') }}</span>
                                        </q-tooltip>
                                    </span>

                                    <!-- Auth info -->
                                    <span v-if='platform.platform.requiresAuth'>
                                        <span class='q-pl-xs text-grey-8'>
                                            <q-icon name='mdi-lock' size='xs' class='q-pb-xs'></q-icon>
                                        </span>
                                        <q-tooltip>
                                            <span>{{ t('autotag.requiresAccount') }}</span>
                                        </q-tooltip>
                                    </span>

                                    <!-- Lyrics icon -->
                                    <span v-if='hasLyrics(platform)'>
                                        <span class='q-pl-xs text-grey-8'>
                                            <q-icon name='mdi-microphone' size='xs' class='q-pb-xs'></q-icon>
                                        </span>
                                        <q-tooltip>
                                            <span>{{ t('autotag.fetchesLyrics') }}</span>
                                        </q-tooltip>
                                    </span>

                                </div>
                            </div>
                            <div v-if='!dense' class='text-subtitle2 q-ml-sm text-left text-grey-6'>
                                <span v-if='providerDescription(platform.builtIn, platform.platform.id) !== undefined'>{{ providerDescription(platform.builtIn, platform.platform.id) }}</span>
                                <span v-else v-html='platform.platform.description'></span>
                            </div>
                            <div v-if='!platform.builtIn' class='text-grey-8 q-pl-sm text-bold monospace text-left' style='font-size: 10px;'>
                                [{{platform.platform.id}}@{{platform.platform.version}}]
                            </div>
                        </q-card-section>
                        <q-card-section class='row'>
                            <img class='q-pa-xs' :class='{"q-mt-sm": !dense}' :src='platform.icon' :height='dense ? 40 : 50'>
                        </q-card-section>
                    </q-card-section>
                </q-card>
            </template>
        </draggable>
    </div>

    <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 24px; margin-bottom: 35px"' inset color="dark"/>

    <div v-if='!dense' class='q-mt-md q-mb-xl'>
        <div class='text-subtitle2 text-bold text-primary'>{{ t('autotag.morePlatforms') }}</div>
        <div class='text-subtitle2 text-grey-6'>{{ t('autotag.customPlatformsHint') }}</div>

        <div class="row items-center justify-center q-mt-md">
            <q-btn dense push color="primary" :loading='platformsRepoButtonLoading' :disable="platformsRepoButtonLoading" class="rounded-borders q-px-sm q-mb-xs text-black text-weight-medium text-caption" @click='openPlatformsRepo()'>{{ t('autotag.platformsRepository') }}</q-btn>
        </div>

        <div class='text-caption text-grey-6 text-center q-py-sm'>
            <span @click='$1t.url("https://github.com/Marekkon5/onetagger-platforms")' class='clickable doc-link'>{{ t('autotag.createPlatform') }}<span class="q-ml-xs"><q-icon name='mdi-open-in-new'></q-icon></span></span>
        </div>

    </div>

</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
import { ref, watch } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import draggable from 'vuedraggable';
import { AutotaggerPlatform, SupportedTag } from '../scripts/autotagger';
import { providerDescription } from '../i18n/providers';


const { dense } = defineProps({
    dense: { type: Boolean, default: false }
});
const $1t = get1t();
const platformsRepoButtonLoading = ref(false);
const providerCards = ref<AutotaggerPlatform[]>([]);

// Platform IDs live inside the descriptor, not on the wrapper object.
function platformKey(platform: AutotaggerPlatform) { return platform.platform.id; }

function update(platform: string) {
    const enabled = new Set($1t.config.value.platforms);
    if (!enabled.delete(platform)) enabled.add(platform);
    syncPlatforms(enabled);
}

// Is platform enabled
function isEnabled(platform: string) {
    return $1t.config.value.platforms.includes(platform);
}

function syncPlatforms(enabled = new Set($1t.config.value.platforms)) {
    const displayed = providerCards.value.map(platformKey);
    $1t.config.value.platforms = [
        ...displayed.filter(id => enabled.has(id)),
        ...[...enabled].filter(id => !displayed.includes(id)),
    ];
}

/// Does the platform have lyrics
function hasLyrics(platform: AutotaggerPlatform) {
    return platform.supportedTags.includes(SupportedTag.UnsyncedLyrics) || platform.supportedTags.includes(SupportedTag.SyncedLyrics);
}

/// Open the platforms repo and make the button load
function openPlatformsRepo() {
    platformsRepoButtonLoading.value = true;
    $1t.send("repoManifest");
    setTimeout(() => {
        platformsRepoButtonLoading.value = false
    }, 2000);
}

watch(
    [() => $1t.info.value.platforms, () => $1t.config.value.platforms],
    ([available, selected], [previousAvailable]) => {
        const rank = new Map(selected.map((id, index) => [id, index]));
        if (available === previousAvailable) {
            const displayed = providerCards.value.map(platformKey);
            const enabled = displayed.filter(id => rank.has(id));
            const configured = selected.filter(id => displayed.includes(id));
            // Checkbox and drag changes already use the displayed order. A
            // loaded profile can change that order and needs to reorder cards.
            if (configured.length === enabled.length && configured.every((id, i) => id === enabled[i])) return;
        }
        providerCards.value = [...available].sort((a, b) =>
            (rank.get(platformKey(a)) ?? selected.length) - (rank.get(platformKey(b)) ?? selected.length));
    },
    { immediate: true },
);

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
