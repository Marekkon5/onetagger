<template>
<div class='text-center'>

    <div class='text-h5 q-mt-md text-grey-4'>Tagging status</div>

    <!-- Post tagging actions -->
    <div v-if='$1t.taggerStatus.value.done && $1t.taggerStatus.value.data' class='row justify-center q-my-md'>
        <q-btn color='primary' class='q-mx-md text-black' @click='goQT(false)'>Open failed in QuickTag</q-btn>
        <q-btn color='primary' class='q-mx-md text-black' @click='goQT(true)'>Open successful in QuickTag</q-btn>
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
        <q-virtual-scroll :items='statuses' class='status-list'>
            <template v-slot="{item, index: i}">
                <q-item :key='i'>
                    <q-item-section>
                        <q-item-label overline>
                            <span>
                                <span v-if='$1t.taggerStatus.value.type != "audioFeatures"' class='selectable text-white'>{{platformText(item.platform)}}</span>
                                <span v-if='$1t.taggerStatus.value.type == "audioFeatures"' class='selectable text-white'>AUDIO FEATURES</span>
                                <img width='16' height='16' class='q-ml-sm' style='margin-bottom: -3px;' v-if='item.status.usedShazam' svg-inline src='../assets/shazam_icon.svg' />
                                <q-icon size='xs' class='q-ml-sm q-mb-xs' :name='statusIcon(item.status.status)' :color='statusColor(item.status.status)'>
                                    <q-tooltip v-if='item.status.message'>
                                       {{item.status.message}}
                                    </q-tooltip>
                                </q-icon>
                            </span>
                        </q-item-label>
                        <span class='selectable text-grey-5'>{{item.status.path}}</span>
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

</div>
</template>

<script lang='ts' setup>
import { useQuasar } from 'quasar';
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { get1t } from '../scripts/onetagger.js';

const $q = useQuasar();
const $1t = get1t();
const $router = useRouter();
const time = ref('0:00');
const filter = ref<string | undefined>(undefined);
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

function countStatus(status: any) {
    return $1t.taggerStatus.value.statuses.reduce((a, c) => (c.status.status == status) ? a + 1 : a, 0);
}

// Toggle status filter
function toggleFilter(name: string) {
    if (filter.value == name) {
        filter.value = undefined;
        return;
    }
    filter.value = name;
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
    return $1t.taggerStatus.value.statuses.filter((s) => s.status.status == filter.value);
});


onMounted(() => {
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
            title: 'Done',
            message: 'Tagging finished! Would you like to open the folder?',
            html: true,
            ok: {
                color: 'primary',
                label: 'Open Folder'
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
        })
    }
});

</script>

<style>
.status-list {
    height: calc(100vh - 248px);
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