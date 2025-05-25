<template>
  <q-card class="cli-card q-pa-md">
    <div class="text-h3 text-center q-my-md">Command line version</div>

    <!-- Config -->
    <div class="text-body1 text-center q-mb-lg">
      1. Copy and save this config into
      <span class="monospace">config.json</span>
    </div>
    <div class="config code monospace">
      {{ JSON.stringify(config, null, 2) }}
    </div>

    <!-- Run -->
    <div class="text-body1 text-center q-my-lg">
      2. Start <span class="monospace">onetagger-cli</span>
    </div>
    <div class="monospace code text-center">
      {{ bin }} <span class="monospace">{{ command }}</span> --config
      config.json --path {{ config.path }}
      <span class="monospace">{{ extra }}</span>
    </div>
  </q-card>
</template>

<script lang="ts" setup>
import { get1t } from "../scripts/onetagger";

const $1t = get1t();

const { config, command } = defineProps({
  config: { type: Object, required: true },
  command: { type: String, required: true },
  extra: { type: String, required: false },
});

// Binary name
let bin = "onetagger-cli";
if ($1t.info.value.os == "windows") {
  bin = "onetagger-cli.exe";
}
</script>

<style lang="scss" scoped>
.cli-card {
  min-width: 700px;
  height: 420px;
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
