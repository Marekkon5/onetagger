<template>
<div class='text-center'>

    <div class='text-subtitle1 text-bold text-primary q-mt-md'>{{ t('autotag.taggingStatus') }}</div>
    <div v-if='stopping && !$1t.taggerStatus.value.done' role='status' class='q-mt-sm'>{{ t('autotag.stopping') }}</div>
    <div v-if='$1t.taggerStatus.value.done' role='status' class='q-mt-sm'>{{ $1t.taggerStatus.value.cancelled ? t('autotag.stopped') : t('autotag.finished') }}</div>

    <!-- Post tagging actions -->
    <div v-if='$1t.taggerStatus.value.done && $1t.taggerStatus.value.data' class='row justify-center q-my-md'>
        <q-btn v-if='$1t.taggerStatus.value.data.failedFile' color='primary' class='q-mx-md text-black' @click='goQT(false)'>{{ t('autotag.openFailed') }}</q-btn>
        <q-btn v-if='$1t.taggerStatus.value.data.successFile' color='primary' class='q-mx-md text-black' @click='goQT(true)'>{{ t('autotag.openSuccessful') }}</q-btn>
        <q-btn v-if='$1t.taggerStatus.value.data.reportFile' color='primary' outline class='q-mx-md'
            @click='$1t.send("openFile", {path: $1t.taggerStatus.value.data.reportFile})'>{{ t('autotag.openResults') }}</q-btn>
    </div>
    <div v-if='$1t.taggerStatus.value.data?.reportFile' class='text-caption q-px-md' style='overflow-wrap: anywhere'>
        {{ t('autotag.resultsPath', { path: $1t.taggerStatus.value.data.reportFile }) }}
    </div>

    <div v-if="$1t.taggerStatus.value.recovered" class="text-caption">{{ t('autotag.recoveredRun') }}</div>

    <!-- Info -->
    <div class='row q-my-sm justify-center'>
        <div class='row justify-between full-width text-subtitle2 q-my-sm list'>
            <div class='col q-mr-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-check' round :color='filter == "ok" ? "primary" : "green"' class='text-black' @click='toggleFilter("ok")'>
                                <q-tooltip>
                                    {{ t('autotag.foundHint') }}
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>{{ t('autotag.matched') }}</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{$1t.taggerStatus.value.results.counts.ok}}</div>
                        </div>
                    </div>
                </q-card>
            </div>

            <div class='col q-mx-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-alert-circle-outline' round :color='filter == "error" ? "primary" : "red"' class='text-black' @click='toggleFilter("error")'>
                                <q-tooltip>
                                    {{ t('autotag.failedHint') }}
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>{{ t('autotag.failed') }}</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{$1t.taggerStatus.value.results.counts.error}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
            
            <div class='col q-mx-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-debug-step-over' round :color='filter == "skipped" ? "primary" : "yellow"' class='text-black' @click='toggleFilter("skipped")'>
                                <q-tooltip>
                                    {{ t('autotag.skippedHint') }}
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>{{ t('autotag.skipped') }}</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{$1t.taggerStatus.value.results.counts.skipped}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
            
            <div class='col q-mx-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-music-box-multiple-outline' round color='grey-6' class='text-black'>
                                <q-tooltip>
                                    {{ t('autotag.totalHint') }}
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>{{ t('autotag.total') }}</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{$1t.taggerStatus.value.total}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
            
            <div class='col q-ml-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn v-if='providerPaces.length' icon='mdi-speedometer' round color='teal' class='text-black'
                                :aria-label="t('autotag.providerPace')" data-testid='provider-pace'>
                                <q-tooltip>{{ t('autotag.providerPace') }}</q-tooltip>
                                <q-menu max-height='50vh'>
                                    <div class='q-pa-sm text-caption' style='max-width: 300px;'>
                                        {{ t('autotag.providerPaceHint') }}
                                    </div>
                                    <q-list data-testid='provider-pace-list' dense style='min-width: 220px'>
                                        <q-item v-for='pace in providerPaces' :key='pace.id'>
                                            <q-item-section>
                                                <q-item-label>{{ pace.name }}</q-item-label>
                                                <q-item-label caption>
                                                    {{ pace.rate === undefined ? t('autotag.providerPaceMeasuring')
                                                        : t('autotag.providerPaceValue', { rate: formatRate(pace.rate) }) }}
                                                </q-item-label>
                                                <q-item-label v-if='pace.limited' caption>
                                                    {{ t('autotag.providerPaceLimited', { count: formatRate(pace.limited) }) }}
                                                </q-item-label>
                                            </q-item-section>
                                        </q-item>
                                    </q-list>
                                </q-menu>
                            </q-btn>
                            <q-btn v-else icon='mdi-timelapse' round color='teal' class='text-black'>
                                <q-tooltip>{{ t('autotag.timeHint') }}</q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>{{ t('autotag.time') }}</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{time}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
        </div>
    </div>
    <!-- Statuses -->
    <q-list class='list text-left bg-dark q-py-sm'>
        <q-btn v-if="retryCount || filter === 'retry'" dense :flat="filter !== 'retry'" rounded color="amber" icon="mdi-timer-sand"
            :aria-pressed="filter === 'retry'" :outline="filter === 'retry'" @click="toggleFilter('retry')">
            {{ t('autotag.retryCount', { count: retryCount }) }}
        </q-btn>
        <q-btn v-if="$1t.taggerStatus.value.done && $1t.taggerStatus.value.retryFiles" dense rounded outline color="primary"
            class="q-ml-sm" icon="mdi-refresh" data-testid="retry-lookups"
            :disable="retrySeconds > 0 || $1t.taggerStatus.value.retrySubmitting" @click="retryLookups">
            {{ retrySeconds > 0 ? t('autotag.retryWait', { seconds: retrySeconds }) : t('autotag.retryLookups', { count: $1t.taggerStatus.value.retryFiles }) }}
        </q-btn>
        <q-virtual-scroll :items-size='statuses.length' :items-fn='visibleStatuses' :class='{"status-list": !$1t.taggerStatus.value.done, "status-list-done": $1t.taggerStatus.value.done}'>
            <template v-slot="{item}: {item: StatusGroup<TaggingStatusWrap>}">
                <q-item :key='item.path'>
                    <q-item-section>
                        <q-item-label overline>
                            <span v-for='(i, index) in item.attempts' :key='index'>
                                <span v-if='$1t.taggerStatus.value.type != "audioFeatures"' class='selectable text-white'>{{platformText(i.platform)}}</span>
                                <span v-if='$1t.taggerStatus.value.type == "audioFeatures"' class='selectable text-white'>{{ t('autotag.audioFeatures') }}</span>
                                <img width='16' height='16' class='q-ml-sm' style='margin-bottom: -3px;' v-if='i.status.usedShazam' svg-inline src='../assets/shazam_icon.svg' />
                                <q-icon size='xs' class='q-ml-sm q-mb-xs' :name='statusIcon(i.status.status)' :color='statusColor(i.status.status)'>
                                    <q-tooltip v-if='i.status.message'>
                                        <span v-if="i.status.status === 'partial'">{{ t('autotag.partialResult') }} · </span>
                                        <span v-if="i.status.status === 'deferred'">{{ t('autotag.deferredResult') }} · </span>
                                        {{ translateBackendMessage(i.status) }}
                                    </q-tooltip>
                                    <q-tooltip v-else-if='i.status.status == "ok" && i.status.accuracy != null'>
                                        {{ t('autotag.accuracy', { percent: accuracyPercent(i.status.accuracy) }) }}
                                        <span v-if='i.status.reason'> — {{ t('autotag.reason', { reason: matchReasonLabel(i.status.reason) }) }}</span>
                                    </q-tooltip>
                                </q-icon>
                                <span class='q-px-sm' v-if='index < item.attempts.length - 1'>|</span>
                            </span>
                        </q-item-label>
                        <span class='selectable text-grey-5'>{{item.path}}</span>
                    </q-item-section>
                </q-item>
            </template>
        </q-virtual-scroll>
    </q-list>

    <!-- Progressbar -->
    <div class='progress'>
        <q-linear-progress 
            :value='$1t.taggerStatus.value.progress'
            color='primary' 
            size='20px'
        >
            <div class='absolute-full flex flex-center'>
                <span class='text-black text-subtitle2'>
                    {{Math.round($1t.taggerStatus.value.progress * 100) + "%"}}
                </span>
            </div>
        </q-linear-progress>
    </div>

    <!-- Stop FAB -->
    <q-page-sticky position="bottom-right" :offset='[36, 32]' v-if='$1t.lock.value.locked'>
        <q-btn @click='stop' :aria-label="t('autotag.stopTagging')" fab icon='mdi-stop' color='red' :loading='stopping' :disabled='stopping'></q-btn>
    </q-page-sticky>

</div>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t, locale } = useI18n({ useScope: 'global' });
import { translateBackendMessage } from '../i18n';
import { useQuasar } from 'quasar';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { get1t } from '../scripts/onetagger.js';
import type { TaggingStatusWrap } from '../scripts/autotagger';
import { newestPage, type StatusGroup } from '../scripts/statusindex';

const $q = useQuasar();
const $1t = get1t();
const $router = useRouter();
const time = ref('0:00');
const paceClock = ref(performance.now());
const wallClock = ref(Date.now());
const filter = ref<string | undefined>(undefined);
const stopping = ref(false);
let timeInterval: any = undefined;

const providerPaces = computed(() => {
    const status = $1t.taggerStatus.value;
    const now = status.done ? status.paceUpdatedAt : Math.max(status.paceUpdatedAt, paceClock.value);
    return Array.from(status.providerPaces, ([id, pace]) => ({
        id, name: pace.name, rate: pace.perMinute(now), limited: pace.limited,
    }));
});

function formatRate(value: number) {
    return new Intl.NumberFormat(locale.value, { maximumFractionDigits: 1 }).format(value);
}

function accuracyPercent(value: number) {
    return new Intl.NumberFormat(locale.value, { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value * 100);
}

function matchReasonLabel(reason: string) {
    // Display labels only: the Rust enum and report values remain unchanged.
    const keys: Record<string, string> = {
        fuzzy: 'autotag.matchReasonFuzzy', isrc: 'autotag.matchReasonIsrc',
        id: 'autotag.matchReasonId', album: 'autotag.matchReasonAlbum',
    };
    return Object.prototype.hasOwnProperty.call(keys, reason) ? t(keys[reason]) : reason;
}

// Conver platform name
function platformText(p: string) {
    if (p == 'junodownload') return 'JUNO DOWNLOAD';
    return p.toUpperCase();
}

function statusIcon(s: string) {
    switch (s) {
        case 'error': return 'mdi-alert-circle';
        case 'ok': return 'mdi-check';
        case 'partial': return 'mdi-circle-half-full';
        case 'deferred': return 'mdi-timer-sand';
        case 'skipped': return 'mdi-debug-step-over';
    }
}

function statusColor(s: string) {
    switch (s) {
        case 'error': return 'red';
        case 'ok': return 'green';
        case 'partial': case 'deferred': return 'amber';
        case 'skipped': return 'yellow';
    }
}

const retryCount = computed(() => {
    const { partial, deferred } = $1t.taggerStatus.value.results.counts;
    return partial + deferred;
});

const retrySeconds = computed(() => Math.max(0, Math.ceil(($1t.taggerStatus.value.retryAt - Math.max(wallClock.value, Date.now())) / 1000)));

async function retryLookups() {
    const status = $1t.taggerStatus.value;
    if (!status.done || !status.runId || !status.retryFiles || retrySeconds.value || status.retrySubmitting) return;
    status.retrySubmitting = true;
    filter.value = undefined;
    stopping.value = false;
    try { await $1t.send('retryTagging', {runId: status.runId}); }
    catch (error) { status.retrySubmitting = false; $1t.onError(String(error)); }
}

// Toggle status filter
function toggleFilter(name: string) {
    if (filter.value == name) {
        filter.value = undefined;
        return;
    }
    filter.value = name;
}

// Stop tagging process
async function stop() {
    stopping.value = true;
    try { await $1t.stopTagging(); }
    catch (error) {
        if ($1t.taggerStatus.value.done) return;
        stopping.value = false;
        $q.dialog({title: t('autotag.stopFailed'), html: false, message: String(error)});
    }
}

// Open QT with result files
function goQT(successful: boolean) {
    if (successful) $1t.settings.value.path = $1t.taggerStatus.value.data.successFile;
    if (!successful) $1t.settings.value.path = $1t.taggerStatus.value.data.failedFile;
    $router.push('/quicktag');
}

const statuses = computed(() => {
    const rows = $1t.taggerStatus.value.results.rows;
    if (!filter.value) return rows;
    if (filter.value === 'retry') return rows.filter(row => row.state === 'partial' || row.state === 'deferred');
    return rows.filter(row => row.state === filter.value);
});
const visibleStatuses = (from: number, size: number) => newestPage(statuses.value, from, size);


onMounted(() => {
    // Undisable stopping
    stopping.value = false;

    // Update timestamp
    timeInterval = setInterval(() => {
        const status = $1t.taggerStatus.value;
        if (status.retryAt > wallClock.value) wallClock.value = Date.now();
        if (status.done || !$1t.lock.value.locked) return;
        paceClock.value = performance.now();
        // Timestamp
        let s = (Date.now() - $1t.taggerStatus.value.started) / 1000;
        time.value = `${Math.floor((s/60))}:${Math.round(s%60).toString().padStart(2, '0')}`;
    }, 1000);
    // Done callback
    $1t.onTaggingDone = (path) => {
        $q.dialog({
            title: $1t.taggerStatus.value.cancelled ? t('autotag.stopped') : t('autotag.done'),
            message: path ? t('autotag.openFolderQuestion') : t('autotag.reviewResults'),
            html: false,
            ok: {
                color: 'primary',
                label: path ? t('autotag.openFolder') : t('autotag.ok')
            },
            cancel: {
                color: 'primary',
                flat: true
            }
        })
        .onOk(() => {
            if (path) {
                $1t.send('openFolder', {path});
            }
        });
        stopping.value = false;
    }
});

onUnmounted(() => {
    clearInterval(timeInterval);
    $1t.onTaggingDone = () => {};
});

</script>

<style>
.status-list {
    height: calc(100vh - 248px);
}

.status-list-done {
    height: calc(100vh - 308px);
}

.list {
    max-width: 80%;
    margin-left: 10%;    
}
.progress {
    width: 100%;
    position: absolute;
    bottom: 0px;
}
</style>
