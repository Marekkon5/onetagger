<template>
<q-dialog ref="dialogRef" @hide="onDialogHide">
<q-card class='dev-tools-card'>
    <!-- Title -->
    <q-card-section>
        <div class='text-subtitle1 text-bold text-center text-primary'>DEV TOOLS</div>
    </q-card-section>

    <!-- Body -->
    <q-card-section class='text-body2'>
        <div>Version: <span class='monospace'>{{ $1t.info.value.version }}</span></div>
        <div>Commit: <span class='monospace'>{{ $1t.info.value.commit }}</span></div>
        <div>OS: <span class='monospace'>{{ $1t.info.value.os }}</span></div>
        <div>Custom Platforms: <span class='monospace'>{{ $1t.info.value.platforms.filter(p => !p.builtIn).map(p => p.platform.id) }}</span></div>
        <div>Data Directory: <span class='monospace'>{{ $1t.info.value.dataDir }}</span></div>
        <div>Working Directory: <span class='monospace'>{{ $1t.info.value.workDir }}</span></div>
        <div>Start Context: <span class='monospace'>{{ $1t.info.value.startContext }}</span></div>
        <div>
            <q-checkbox @update:model-value="(v) => {$1t.settings.value.devtools = v; $1t.saveSettings(true)}" :model-value='$1t.settings.value.devtools' label='Enable webview devtools (requires restart)'></q-checkbox>
        </div>

        <!-- Actions -->
        <div class='q-my-sm'></div>
        <q-btn class='q-mr-sm' outline color='primary' @click='reload()'>Reload</q-btn>
        <q-btn class='q-mx-sm' outline color='primary' @click='devServer()'>Go to dev server</q-btn>
        <q-btn class='q-mx-sm' outline color='primary' @click='devtools()'>Open webview devtools</q-btn>
        <q-btn class='q-mx-sm' outline color='primary' @click='$1t.send("openSettingsFolder")'>Open data dir</q-btn>
        <q-btn class='q-mx-sm' outline color='primary' @click='pythonDocs()'>Generate and open Python Docs</q-btn>
        <div class='q-my-sm'></div>

        <!-- Log -->
        <div class='q-py-sm text-bold monospace'>
            Log:
        </div>
        <q-virtual-scroll ref='logRef' :items='log' v-slot='{ item, index }' style='max-height: calc(100vh - 460px);'>
            <div :key='index'>
                <span class='monospace text-bold'>{{ item.time }} &nbsp;</span>
                <span class='monospace text-bold' :class='`text-${levelColor(item.level)}`'> {{ item.level }} &nbsp;</span>
                <span class='monospace text-bold'> {{ item.module }} &nbsp;</span>
                <span class='monospace text-bold' v-if='item.count > 1'> x{{ item.count }}</span>
                <br>
                <span class='monospace'>{{ item.line }}</span>
                <br><br>
            </div>
        </q-virtual-scroll>
        <div class='q-pt-sm row'>
            <div class='q-mt-xs q-pt-xs text-bold text-primary clickable monospace' @click='fullLog = !fullLog'>SHOW FULL</div>
            <div class='q-mt-xs q-pt-xs text-bold q-ml-md text-primary clickable monospace' @click='logBottom()'>TO BOTTOM</div>
            <!-- Filter -->
            <q-checkbox
                v-for='level in levels'
                :model-value="filterLevels.includes(level)"
                @update:model-value="filterLevel(level)"
                :label='level'
                class='q-mx-sm'
                :key='level'
            ></q-checkbox>
        </div>
    </q-card-section>


</q-card>
</q-dialog>
</template>

<script lang='ts' setup>
import { useDialogPluginComponent, useQuasar } from 'quasar';
import { get1t } from '../scripts/onetagger';
import { computed, onMounted, ref } from 'vue';

const { dialogRef, onDialogHide } = useDialogPluginComponent();
const $q = useQuasar();
const $1t = get1t();
const logRef = ref(null);
const fullLog = ref(false);
const filterLevels = ref<string[]>(['DEBUG', 'WARN', 'ERROR', 'INFO']);

const levels = ['DEBUG', 'WARN', 'ERROR', 'INFO'];

defineEmits([
    ...useDialogPluginComponent.emits
]);

// Add level to filter
function filterLevel(level: string) {
    let i = filterLevels.value.indexOf(level);
    if (i != -1) {
        filterLevels.value.splice(i, 1);
        return;
    }
    filterLevels.value.push(level);
}

function reload() {
    location.reload();
}

function devServer() {
    window.location.href = 'http://localhost:8080';
}

function devtools() {
    //@ts-ignore
    window.ipc.postMessage('devtools');
}

// Scroll to bottom
function logBottom() {
    //@ts-ignore
    logRef.value!.scrollTo(log.value.length);
}

// Generate python docs and show warning
function pythonDocs() {
    $1t.send('pythonDocs');
    $q.dialog({
        title: 'Python Docs',
        message: 'Python docs are generating, which might take a while and will open once ready. You can also access them later from 1T folder',
        ok: {
            color: 'primary'
        }
    });
}

// Get color for level
function levelColor(level: string) {
    return {
        'INFO': 'green',
        'WARN': 'orange',
        'ERROR': 'red',
        'DEBUG': 'blue'
    }[level];
}

// Parse the log
const log = computed(() => {
    let lines: { time: string, level: string, module: string, line: string, count: number }[] = [];
    let prev = '';
    ($1t.info.value.log?.split('\n')??[]).forEach(l => {
        if (l.match(/^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d \[INFO|WARN|ERROR|\] /)) {
            let parts = l.split(' ');
            let content = parts.slice(4).join(" ");
            // Deduplicate
            if (content == prev && prev != '') {
                lines[lines.length - 1].count += 1;
                return;
            }

            // Filter
            if (lines.length > 0 && !filterLevels.value.includes(lines[lines.length - 1].level)) {
                lines.pop();
            }

            prev = content;
            lines.push({
                time: `${parts[0]} ${parts[1]}`,
                level: parts[2].substring(1, parts[2].length - 1),
                module: parts[3].substring(0, parts[3].length - 1),
                line: content,
                count: 1,
            });
        } else {
            // Only this log
            if (!fullLog.value && l.includes('Starting OneTagger') && lines.length > 1) {
                lines = lines.slice(lines.length - 1);
            }
            lines[lines.length - 1].line += '\n' + l;
        }
        
    });
    // Filter
    if (lines.length > 0 && !filterLevels.value.includes(lines[lines.length - 1].level)) {
        lines.pop();
    }
    return lines;
});

onMounted(() => {
    $1t.send('getLog');
});

</script>

<style lang='scss' scoped>
.dev-tools-card {
    min-width: 900px;
}

</style>