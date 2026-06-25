<template>
<div class='text-center'>

    <div class='text-subtitle1 text-bold text-primary q-mt-md'>TAGGING STATUS</div>

    <!-- Post tagging actions -->
    <div v-if='$1t.taggerStatus.value.done && $1t.taggerStatus.value.data' class='row justify-center q-my-md'>
        <q-btn color='primary' class='q-mx-md text-black' @click='goQT(false)'>Open failed in QuickTag</q-btn>
        <q-btn color='primary' class='q-mx-md text-black' @click='goQT(true)'>Open successful in QuickTag</q-btn>
    </div>

    <!-- Info -->
    <div class='row q-my-sm justify-center'>
        <div class='row justify-between full-width text-subtitle2 q-my-sm stats'>
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

    <!-- Statuses table -->
    <div class='table-wrap' :class='{"status-list": !$1t.taggerStatus.value.done, "status-list-done": $1t.taggerStatus.value.done}'>
        <q-table
            class='status-table bg-dark'
            :rows='rows'
            :columns='columns'
            row-key='fullPath'
            virtual-scroll
            :rows-per-page-options='[0]'
            hide-pagination
            flat
            dark
            dense
            binary-state-sort
            :pagination='pagination'
            :virtual-scroll-sticky-size-start='48'
        >
            <template v-slot:body-cell='props'>
                <q-td :props='props'>
                    <span v-if='props.col.name === "filename"' class='selectable text-white'>
                        {{ props.row.filename }}
                    </span>
                    <span v-else-if='props.col.name === "path"' class='selectable text-grey-5'>
                        {{ props.row.path }}
                    </span>
                    <span v-else>
                        <template v-if='props.row.platforms[props.col.name]'>
                            <img
                                v-if='props.row.platforms[props.col.name].status.usedShazam'
                                width='16'
                                height='16'
                                class='q-mr-xs'
                                style='margin-bottom: -3px;'
                                svg-inline
                                src='../assets/shazam_icon.svg'
                            />
                            <q-icon
                                size='xs'
                                :name='statusIcon(props.row.platforms[props.col.name].status.status)'
                                :color='statusColor(props.row.platforms[props.col.name].status.status)'
                            >
                                <q-tooltip v-if='props.row.platforms[props.col.name].status.message'>
                                    {{ props.row.platforms[props.col.name].status.message }}
                                </q-tooltip>
                                <q-tooltip v-else-if='props.row.platforms[props.col.name].status.status === "ok"'>
                                    Accuracy: {{ ((props.row.platforms[props.col.name].status.accuracy ?? 0) * 100).toFixed(2) }}%
                                    <span v-if='props.row.platforms[props.col.name].status.reason'>, Reason: {{ props.row.platforms[props.col.name].status.reason }}</span>
                                </q-tooltip>
                            </q-icon>
                        </template>
                        <span v-else class='text-grey-8'>—</span>
                    </span>
                </q-td>
            </template>
        </q-table>
    </div>

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
        <q-btn @click='stop' fab icon='mdi-stop' color='red' :loading='stopping' :disabled='stopping'></q-btn>
    </q-page-sticky>

</div>
</template>

<script lang='ts' setup>
import { useQuasar } from 'quasar';
import { computed, onMounted, ref } from 'vue';
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

// Convert platform name to display label
function platformText(p: string) {
    if (p == 'audioFeatures') return 'AUDIO FEATURES';
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
function stop() {
    stopping.value = true;
    $1t.stopTagging();
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

// Path helpers — handle both forward and backslashes for cross-platform paths
function basename(p: string): string {
    const parts = p.split(/[\\/]/);
    return parts[parts.length - 1] || p;
}

function dirname(p: string): string {
    const parts = p.split(/[\\/]/);
    parts.pop();
    return parts.join('/');
}

// Discover platforms present in current statuses, ordered by config.platforms
const platformList = computed<{ id: string; label: string }[]>(() => {
    if ($1t.taggerStatus.value.type === 'audioFeatures') {
        return [{ id: 'audioFeatures', label: 'AUDIO FEATURES' }];
    }
    const seen = new Set<string>();
    for (const row of $1t.taggerStatus.value.statuses) {
        for (const entry of row) seen.add(entry.platform);
    }
    const ordered: string[] = [];
    for (const p of $1t.config.value.platforms) {
        if (seen.has(p)) ordered.push(p);
    }
    for (const p of seen) {
        if (!ordered.includes(p)) ordered.push(p);
    }
    return ordered.map((id) => ({ id, label: platformText(id) }));
});

interface Row {
    filename: string;
    path: string;
    fullPath: string;
    platforms: Record<string, TaggingStatusWrap>;
}

const rows = computed<Row[]>(() => {
    const isAudio = $1t.taggerStatus.value.type === 'audioFeatures';
    return statuses.value.map((entries) => {
        const fullPath = entries[0]?.status.path ?? '';
        const platforms: Record<string, TaggingStatusWrap> = {};
        if (isAudio) {
            if (entries[0]) platforms['audioFeatures'] = entries[0];
        } else {
            for (const e of entries) platforms[e.platform] = e;
        }
        return {
            filename: basename(fullPath),
            path: dirname(fullPath),
            fullPath,
            platforms,
        };
    });
});

// Higher score = better match. Used as the sort key for platform columns.
function platformScore(w: TaggingStatusWrap | undefined): number {
    if (!w) return 0;
    switch (w.status.status) {
        case 'ok': return 3000 + (w.status.accuracy ?? 0) * 100;
        case 'skipped': return 200;
        case 'error': return 100;
        default: return 0;
    }
}

const columns = computed(() => {
    const cols: any[] = [
        { name: 'filename', label: 'Filename', field: 'filename', sortable: true, align: 'left' },
        { name: 'path', label: 'Path', field: 'path', sortable: true, align: 'left' },
    ];
    for (const p of platformList.value) {
        cols.push({
            name: p.id,
            label: p.label,
            field: (row: Row) => row.platforms[p.id],
            sortable: true,
            align: 'center',
            sort: (a: TaggingStatusWrap | undefined, b: TaggingStatusWrap | undefined) =>
                platformScore(a) - platformScore(b),
        });
    }
    return cols;
});

const pagination = ref({ sortBy: 'filename', descending: false, rowsPerPage: 0 });

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
        });
        stopping.value = false;
    }
});

</script>

<style>
.status-list {
    height: calc(100vh - 248px);
}

.status-list-done {
    height: calc(100vh - 308px);
}

.stats {
    max-width: 80%;
    margin-left: 10%;
}

.table-wrap {
    margin: 0 16px;
    padding-bottom: 40px; /* Clear the floating Stop FAB so the last row stays visible */
    display: flex;
    flex-direction: column;
}

.status-table {
    flex: 1 1 auto;
    min-height: 0;
}

.status-table .q-table__top {
    padding: 4px 12px;
}

.status-table thead tr th {
    position: sticky;
    top: 0;
    z-index: 1;
    background-color: #1d1d1d;
}

.progress {
    width: 100%;
    position: absolute;
    bottom: 0px;
}
</style>
