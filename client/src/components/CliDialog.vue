<template>
<q-card class='cli-card q-pa-md'>

    <div class='text-h3 text-center q-my-md'>{{ t('tools.cliVersion') }}</div>

    <!-- Config -->
    <div class='text-body1 text-center q-mb-lg'>{{ t('tools.cliCopyConfig', { filename: 'config.json' }) }}</div>
    <div class='config code monospace'>
        {{JSON.stringify(config, null, 2)}}
    </div>

    <!-- Run -->
    <div class='text-body1 text-center q-my-lg'>{{ t('tools.cliStart', { program: 'onetagger-cli' }) }}</div>
    <div class='monospace code text-center'>
        {{bin}} <span class='monospace'>{{command}}</span> --config config.json --path {{config.path}} <span class='monospace'>{{extra}}</span>
    </div>

</q-card>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
import { get1t } from '../scripts/onetagger';

const $1t = get1t();

const { config, command } = defineProps({
    config: { type: Object, required: true },
    command: { type: String, required: true },
    extra: { type: String, required: false }
})

// Binary name
let bin = 'onetagger-cli';
if ($1t.info.value.os == 'windows') {
    bin =  'onetagger-cli.exe';
}

</script>

<style lang='scss' scoped>

.cli-card {
    width: 800px;
    max-width: 94vw;
    min-height: 420px;
}

.code {
    width: 100%;
    overflow-y: scroll;
    user-select: all;
    background-color: var(--q-dark-page);
    padding: 4px;
}

.code * {
    user-select: all;
}

.config {
    height: 100px;
    font-size: 10px;
}

</style>
