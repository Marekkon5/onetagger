<template>
<div class='text-center'>

    <div class='text-subtitle1 text-bold text-primary q-mt-md'>TAGGING STATUS</div>
    <div v-if='stopping && !$1t.taggerStatus.value.done' role='status' class='q-mt-sm'>Stopping… waiting for in-flight work to finish.</div>
    <div v-if='$1t.taggerStatus.value.done' role='status' class='q-mt-sm'>{{$1t.taggerStatus.value.cancelled ? 'Stopped' : 'Finished'}}</div>

    <!-- Post tagging actions -->
    <div v-if='$1t.taggerStatus.value.done && $1t.taggerStatus.value.data' class='row justify-center q-my-md'>
        <q-btn v-if='$1t.taggerStatus.value.data.failedFile' color='primary' class='q-mx-md text-black' @click='goQT(false)'>Open failed in QuickTag</q-btn>
        <q-btn v-if='$1t.taggerStatus.value.data.successFile' color='primary' class='q-mx-md text-black' @click='goQT(true)'>Open successful in QuickTag</q-btn>
        <q-btn v-if='$1t.taggerStatus.value.data.reportFile' color='primary' outline class='q-mx-md'
            @click='$1t.send("openFile", {path: $1t.taggerStatus.value.data.reportFile})'>Open results JSON</q-btn>
    </div>
    <div v-if='$1t.taggerStatus.value.data?.reportFile' class='text-caption q-px-md' style='overflow-wrap: anywhere'>
        Results JSON: {{$1t.taggerStatus.value.data.reportFile}}
    </div>

    <!-- Info -->
    <div class='row q-my-sm justify-center'>
        <div class='row justify-between full-width text-subtitle2 q-my-sm list'>
            <div class='col q-mr-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-check' round :color='filter == "ok" ? "primary" : "green"' class='text-black' @click='toggleFilter("ok")'>
                                <q-tooltip>
                                    Total amount found
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>Matched</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{countStatus("ok")}}</div>
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
                                    Total amount not found
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>Failed</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{countStatus("error")}}</div>
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
                                    Total amount skipped due missing tags, corruption, or Shazam not being able to identify
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>Skipped</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{countStatus("skipped")}}</div>
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
                                    Total amount of files to process
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>Total</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{$1t.taggerStatus.value.total}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
            
            <div class='col q-ml-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-timelapse' round color='teal' class='text-black'>
                                <q-tooltip>
                                    Total amount of elapsed time
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>Time</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{time}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
        </div>
    </div>
    <!-- Statuses -->
    <q-list class='list text-left bg-dark q-py-sm'>
        <q-virtual-scroll :items='statuses' :class='{"status-list": !$1t.taggerStatus.value.done, "status-list-done": $1t.taggerStatus.value.done}'>
            <template v-slot="{item, index: i}: {item: TaggingStatusWrap[]; index: number}">
                <q-item :key='i'>
                    <q-item-section>
                        <q-item-label overline>
                            <span v-for='(i, index) in item'>
                                <span v-if='$1t.taggerStatus.value.type != "audioFeatures"' class='selectable text-white'>{{platformText(i.platform)}}</span>
                                <span v-if='$1t.taggerStatus.value.type == "audioFeatures"' class='selectable text-white'>AUDIO FEATURES</span>
                                <img width='16' height='16' class='q-ml-sm' style='margin-bottom: -3px;' v-if='i.status.usedShazam' svg-inline src='../assets/shazam_icon.svg' />
                                <q-icon size='xs' class='q-ml-sm q-mb-xs' :name='statusIcon(i.status.status)' :color='statusColor(i.status.status)'>
                                    <q-tooltip v-if='i.status.message'>
                                        {{i.status.message}}
                                    </q-tooltip>
                                    <q-tooltip v-if='i.status.status == "ok" && i.status.accuracy != null'>
                                        Accuracy: {{ (i.status.accuracy * 100).toFixed(2) }}%
                                        <span v-if='i.status.reason'>, Reason: {{ i.status.reason }}</span>
                                    </q-tooltip>
                                </q-icon>
                                <span class='q-px-sm' v-if='index < item.length - 1'>|</span>
                            </span>
                        </q-item-label>
                        <span class='selectable text-grey-5'>{{item[0].status.path}}</span>
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
        <q-btn @click='stop' aria-label='Stop tagging' fab icon='mdi-stop' color='red' :loading='stopping' :disabled='stopping'></q-btn>
    </q-page-sticky>

</div>
</template>

<script lang='ts' setup>
import { useQuasar } from 'quasar';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { get1t } from '../scripts/onetagger.js';
import { TaggingStatusWrap } from '../scripts/autotagger';

const $q = useQuasar();
const $1t = get1t();
const $router = useRouter();
const time = ref('0:00');
const filter = ref<string | undefined>(undefined);
const stopping = ref(false);
let timeInterval: any = undefined;

// Conver platform name
function platformText(p: string) {
    if (p == 'junodownload') return 'JUNO DOWNLOAD';
    return p.toUpperCase();
}

function statusIcon(s: string) {
    switch (s) {
        case 'error': return 'mdi-alert-circle';
        case 'ok': return 'mdi-check';
        case 'skipped': return 'mdi-debug-step-over';
    }
}

function statusColor(s: string) {
    switch (s) {
        case 'error': return 'red';
        case 'ok': return 'green';
        case 'skipped': return 'yellow';
    }
}

/// Get actual status from status list
function getStatus(s: TaggingStatusWrap[]): string {
    if (s.find((s) => s.status.status == 'ok')) {
        return 'ok';
    }
    if (s.find((s) => s.status.status == 'skipped')) {
        return 'skipped';
    }
    return 'error';
}

function countStatus(status: any) {
    return $1t.taggerStatus.value.statuses.reduce((a, c) => (getStatus(c) == status) ? a + 1 : a, 0);
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
        $q.dialog({title: 'Could not stop tagging', html: false, message: String(error)});
    }
}

// Open QT with result files
function goQT(successful: boolean) {
    if (successful) $1t.settings.value.path = $1t.taggerStatus.value.data.successFile;
    if (!successful) $1t.settings.value.path = $1t.taggerStatus.value.data.failedFile;
    $router.push('/quicktag');
}

const statuses = computed(() => {
    if (!filter.value)
        return $1t.taggerStatus.value.statuses;
    return $1t.taggerStatus.value.statuses.filter((s) => getStatus(s) == filter.value);
});


onMounted(() => {
    // Undisable stopping
    stopping.value = false;

    // Update timestamp
    timeInterval = setInterval(() => {
        // Already done
        if ($1t.taggerStatus.value.done || !$1t.lock.value.locked) {
            if (timeInterval)
                clearInterval(timeInterval);
            return;
        }
        // Timestamp
        let s = (Date.now() - $1t.taggerStatus.value.started) / 1000;
        time.value = `${Math.floor((s/60))}:${Math.round(s%60).toString().padStart(2, '0')}`;
    }, 400);
    // Done callback
    $1t.onTaggingDone = (path) => {
        $q.dialog({
            title: $1t.taggerStatus.value.cancelled ? 'Stopped' : 'Done',
            message: path ? 'Tagging ended. Would you like to open the folder?' : 'Tagging ended. Review the results below.',
            html: false,
            ok: {
                color: 'primary',
                label: path ? 'Open Folder' : 'OK'
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
