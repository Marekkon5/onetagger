<template>
<q-dialog v-model='show' persistent>
<q-card style='min-width: 650px; min-height: 50vh;' class='q-pa-lg'>

    <!-- Title -->
    <q-card-section>
        <div class='text-subtitle1 text-bold text-center text-primary'>MANUAL TAG</div>
        <div class='monospace text-subtitle2 text-grey-6 text-center'>{{ path }}</div>
    </q-card-section>

    <!-- Body -->
    <q-card-section>
        <div class='manualtag-results bg-dark'>

            <!-- Results list -->
            <q-list v-if='$1t.manualTag.value.busy || $1t.manualTag.value.done'>

                <!-- Empty results -->
                <div v-if='$1t.manualTag.value.done && $1t.manualTag.value.matches.length == 0' class='text-center'>
                    <div class='text-h5 q-mt-md'>No results!</div>
                    <div class='text-body1 text-grey-5 q-mt-md'>Try enabling more platforms or correcting title + artist tag.</div>
                </div>

                <!-- Matches -->
                <q-item v-for='(match, i) in $1t.manualTag.value.matches' :key='i'>
                    <q-item-section avatar>
                        <div class='row items-center'>
                            <div>
                                <div class='q-pr-sm'>
                                    <q-checkbox
                                        :model-value="selected.includes(match)"
                                        @update:model-value="(v) => toggleMatch(match)"
                                    ></q-checkbox>
                                </div>
                                <!-- Open URL -->
                                <q-btn 
                                    icon='mdi-open-in-new' 
                                    flat 
                                    round 
                                    size='sm' 
                                    color='grey-5'
                                    style='margin-left: 5px;'
                                    @click='$1t.url(match.track.url)'
                                    v-if='match.track.url'
                                ></q-btn>
                            </div>
                            <q-img 
                                width='48px' 
                                height='48px'
                                :src='match.track.thumbnail??match.track.art'
                                :placeholder-src="PLACEHOLDER_IMG"
                            ></q-img>
                        </div>
                    </q-item-section>
                    <q-item-section>
                        <q-item-label overline class='text-white'>
                            <span>{{ match.track.platform.toUpperCase() }}</span>
                            <span class='q-px-sm' :class='accuracyColor(match.accuracy)'>{{ (match.accuracy * 100.0).toFixed(2) }}%</span>
                            <span v-if='match.reason != "fuzzy"'>{{ match.reason.toUpperCase() }}</span>
                        </q-item-label>
                        <q-item-label class='text-grey-5'>{{ match.track.artists.join(", ") }} - {{ match.track.title }}</q-item-label>
                        <q-item-label class='text-grey-5'>
                            <span v-if='match.track.album'>Album: <span class='text-white'>{{ match.track.album }}</span></span>
                            <span v-if='match.track.album && match.track.genres.length > 0'>, </span>
                            <span v-if='match.track.genres.length > 0'>Genres: <span class='text-white'>{{ match.track.genres.join(", ") }}</span></span>
                            <span v-if='match.track.bpm'>, BPM: <span class='monospace text-white'>{{ match.track.bpm }}</span></span>
                            <span v-if='match.track.key'>, Key: <span class='monospace text-white'>{{ match.track.key }}</span></span>
                        </q-item-label>
                    </q-item-section>
                </q-item>
            </q-list>

            <!-- Config -->
            <div v-else>
                <div class='q-mt-md text-subtitle1 text-bold text-center text-primary'>PLATFORMS</div>
                <autotagger-platforms dense></autotagger-platforms>
                <autotagger-tags manual-tag></autotagger-tags>
                <autotagger-platform-specific class='q-mt-lg q-px-lg'></autotagger-platform-specific>
            </div>

        </div>

        <!-- Errors -->
        <div v-if='$1t.manualTag.value.errors.length > 0' class='text-center text-red text-body1 q-pt-sm clickable' @click='errorList = true'>
            Some platforms failed to search. Click here to see details.
        </div>

    </q-card-section>

    <!-- Actions -->
    <q-separator></q-separator>
    <q-card-section class='row'>
        <q-space></q-space>
        <!-- Cancel / close -->
        <div class='q-px-sm'>
            <q-btn flat color='red' @click='exit' v-if='!saving'>Close</q-btn>
        </div>
        <!-- Start tagging -->
        <div class='q-px-sm' v-if='!$1t.manualTag.value.done'>
            <q-btn 
                flat 
                color='primary' 
                @click='start' 
                :disable='$1t.manualTag.value.busy && !$1t.manualTag.value.done' 
                :loading='$1t.manualTag.value.busy'
            >Start</q-btn>
        </div>
        <!-- Apply -->
        <div class='q-px-sm' v-if='selected.length > 0'>
            <q-btn 
                flat 
                color='primary' 
                @click='apply'
                :disable='saving'
                :loading='saving'
            >Apply</q-btn>
        </div>

    </q-card-section>

</q-card>
</q-dialog>

<!-- Error list -->
<q-dialog v-model='errorList'>
<q-card style='min-width: 420px;'>
    <!-- Title -->
    <q-card-section>
        <div class='text-subtitle1 text-bold text-center text-red'>ERRORS</div>
    </q-card-section>

    <!-- Errors -->
    <q-card-section>
        <div v-for='error in $1t.manualTag.value.errors' class='text-body1'>
            <span><span class='text-bold'>{{ error.platform.toUpperCase() }}</span>: {{ error.error }}</span>
        </div>
    </q-card-section>

    <!-- Hide -->
    <q-card-section class='row'>
        <q-space></q-space>
        <q-btn flat color='red' @click='errorList = false'>Close</q-btn>
    </q-card-section>

</q-card>
</q-dialog>


</template>

<script lang='ts' setup>
import { ref, toRefs, watch } from 'vue';
import { TrackMatch } from '../scripts/manualtag';
import { get1t } from '../scripts/onetagger';
import { AutotaggerConfig } from '../scripts/autotagger';
import { useQuasar } from 'quasar';
import AutotaggerPlatforms from './AutotaggerPlatforms.vue';
import AutotaggerTags from './AutotaggerTags.vue';
import AutotaggerPlatformSpecific from './AutotaggerPlatformSpecific.vue';
import { PLACEHOLDER_IMG } from '../scripts/quicktag';

const $q = useQuasar();
const $1t = get1t();
const show = ref(false);
const emit = defineEmits(['exit']);
const props = defineProps({
    path: { type: String, required: false }
});
const { path } = toRefs(props);
const saving = ref(false);
const selected = ref<TrackMatch[]>([]);
const errorList = ref(false);
let cachedConfig = {};

/// Start manual tagger
function start() {
    $1t.manualTag.value.reset();

    // Generate config
    let config = JSON.parse(JSON.stringify($1t.config.value));
    config.path = '';
    if ($1t.spotify.value.clientId && $1t.spotify.value.clientSecret) {
        config.spotify = {
            clientId: $1t.spotify.value.clientId,
            clientSecret: $1t.spotify.value.clientSecret,
        }
    }
    cachedConfig = config;

    // Start
    $1t.manualTag.value.tagTrack(path!.value!, config);
}

/// Add or remove match
function toggleMatch(match: TrackMatch) {
    let i = selected.value.indexOf(match);
    if (i != -1) {
        selected.value.splice(i, 1);
        return;
    }
    selected.value.push(match);
}

/// Exit manual tagger
function exit() {
    $1t.manualTag.value.reset();
    selected.value = [];
    saving.value = false;
    show.value = false;
    emit('exit');
}

/// Get accuracy color
function accuracyColor(acc: number) {
    if (acc == 1.0) return 'text-green';
    if (acc > 0.85) return 'text-yellow';
    return 'text-red';
}

/// Apply the matches
async function apply() {
    saving.value = true;
    let response: any = await $1t.manualTag.value.apply(selected.value, path!.value!, cachedConfig as AutotaggerConfig);
    // All ok
    if (response.status == 'ok') {
        $q.notify({
            message: "Track saved!",
            timeout: 3000,
            position: 'top-right'
        });
    // Show error
    } else {
        await new Promise((r, _) => {
            $q.dialog({
                title: 'Failed to save track',
                message: response.error,
                ok: true,
                cancel: false
            })
            .onOk(() => r(true));
        });
    }

    exit();
}

// Show / Hide
watch(path!, () => {
    // to bool
    show.value = !!(path!.value);
});

</script>

<style lang='scss' scoped>
.manualtag-results {
    min-height: 50vh;
    height: 50vh;
    overflow-y: scroll;
    overflow-x: hidden;
    border-radius: 8px;
    background-color: #99999910 !important
}

</style>