<template>
<q-dialog ref='dialogRef'>
    <q-card class='q-dialog-plugin'>

        <q-card-section>
            <div class='text-h6' v-if='isQt'>Unsaved changes</div>
            <div class='text-h6' v-else>Tagging in progress</div>
        </q-card-section>

        <q-card-section>
            <div v-if='isQt'>Do you want to save pending changes before exitting?</div>
            <div v-else>Tagging is in progress, do you really want to exit?</div>
        </q-card-section>

        <q-card-section class='row'>
            <q-space></q-space>
            <q-btn color='white' flat @click='onDialogCancel()'>Cancel</q-btn>
            <q-btn color='red' flat @click='exit()'>
                <span v-if='isQt'>Exit without saving</span>
                <span v-else>Exit anyway</span>
            </q-btn>
            <q-btn v-if='isQt' color='primary' flat @click='saveExit()'>Save and exit</q-btn>
        </q-card-section>

    </q-card>
</q-dialog>
</template>

<script lang='ts' setup>
import { useDialogPluginComponent } from 'quasar';
import { get1t } from '../scripts/onetagger';
import { toRef } from 'vue';
import { wsUrl } from '../scripts/utils';

const { dialogRef, onDialogCancel } = useDialogPluginComponent();
const $1t = get1t();
const props = defineProps({ isQt: Boolean });
const isQt = toRef(props, 'isQt');

defineEmits([...useDialogPluginComponent.emits]);


// Exit
function exit() {
    // Use another connection because main thread is locked in AT
    if (!isQt.value) {
        let ws = new WebSocket(wsUrl());
        ws.onopen = () => {
            ws.send(JSON.stringify({action: 'exit'}));
        }
        return;
    }

    $1t.send('exit');
}

// Save QT and exit
async function saveExit() {
    $1t.saveQTTrack();
    await $1t.quickTag.value.waitForSave();
    $1t.send('exit');
}

</script>