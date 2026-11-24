<template>
<q-dialog ref='dialogRef' :persistent='$1t.quickTag.value.saving > 0'>
    <q-card class='q-dialog-plugin' :inert='$1t.quickTag.value.saving > 0'>

        <q-card-section>
            <div class='text-h6' v-if='isQt'>{{ t('common.unsavedChanges') }}</div>
            <div class='text-h6' v-else>{{ t('common.taggingProgress') }}</div>
        </q-card-section>

        <q-card-section>
            <div v-if='isQt'>{{ t('common.saveBeforeExit') }}</div>
            <div v-else>{{ t('common.confirmExit') }}</div>
        </q-card-section>

        <q-card-section class='row'>
            <q-space></q-space>
            <q-btn color='white' flat @click='onDialogCancel()'>{{ t('common.cancel') }}</q-btn>
            <q-btn color='red' flat @click='exit()'>
                <span v-if='isQt'>{{ t('common.exitWithoutSave') }}</span>
                <span v-else>{{ t('common.exitAnyway') }}</span>
            </q-btn>
            <q-btn v-if='isQt' color='primary' flat @click='saveExit()'>{{ t('common.saveExit') }}</q-btn>
        </q-card-section>

    </q-card>
</q-dialog>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });

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
    if (!await $1t.saveQTTrack()) return;
    $1t.send('exit');
}

</script>
