<template>
  <q-dialog ref="dialogRef" @hide="onDialogHide">
    <q-card class="q-pa-md" style="min-width: 570px">
      <!-- Title -->
      <q-card-section>
        <div class="text-subtitle2 text-bold text-center text-primary">
          CUSTOM PLATFORMS
        </div>
      </q-card-section>

      <!-- Platforms -->
      <div style="overflow-y: scroll; height: 480px">
        <div v-for="platform in platforms">
          <q-card flat class="bg-darker q-ma-md">
            <q-card-section horizontal class="row justify-between">
              <q-card-section>
                <div class="row">
                  <div class="text-h6 q-mt-xs">
                    {{ platform.name }}

                    <!-- Speed icon -->
                    <span>
                      <span class="q-pl-xs text-grey-7">
                        <q-icon
                          v-if="platform.maxThreads == 1"
                          name="mdi-speedometer-slow"
                          size="xs"
                          class="q-pb-xs"
                        ></q-icon>
                        <q-icon
                          v-if="platform.maxThreads > 1"
                          name="mdi-speedometer-medium"
                          size="xs"
                          class="q-pb-xs"
                        ></q-icon>
                        <q-icon
                          v-if="platform.maxThreads == 0"
                          name="mdi-speedometer"
                          size="xs"
                          class="q-pb-xs"
                        ></q-icon>
                      </span>
                      <q-tooltip>
                        <span v-if="platform.maxThreads"
                          >This platform allows up to
                          {{ platform.maxThreads }} concurrent searches</span
                        >
                        <span v-else
                          >This platform allows unlimited concurrent
                          searches</span
                        >
                      </q-tooltip>
                    </span>

                    <!-- Auth info -->
                    <span v-if="platform.requiresAuth">
                      <span class="q-pl-xs text-grey-8">
                        <q-icon
                          name="mdi-lock"
                          size="xs"
                          class="q-pb-xs"
                        ></q-icon>
                      </span>
                      <q-tooltip>
                        <span>Platform requires an account</span>
                      </q-tooltip>
                    </span>
                  </div>
                </div>
                <span v-html="platform.description"></span>
                <br />
                <!-- Info -->
                <span class="text-grey-5">Author: {{ platform.author }}</span>
                <div
                  class="text-grey-8 text-bold monospace text-left"
                  style="font-size: 10px"
                >
                  {{ platform.id }}@{{ platform.version }}
                </div>
                <div
                  class="text-red text-bold"
                  v-if="
                    !Object.values(platform.versions).includes(
                      $1t.info.value.customPlatformCompat,
                    )
                  "
                >
                  <span v-if="platform.language != 'rust'"
                    >Platform potentially incompatible!</span
                  >
                  <span v-else>Platform is incompatible!</span>
                </div>
              </q-card-section>
              <q-card-section class="column">
                <img class="q-pa-xs" :src="iconUrl(platform.id)" :height="50" />
                <div class="q-my-xs q-pl-xs">
                  <q-btn
                    icon="mdi-download"
                    flat
                    round
                    @click="
                      () => {
                        selectedPlatform = platform;
                        downloadDialog = true;
                      }
                    "
                  ></q-btn>
                </div>
              </q-card-section>
            </q-card-section>
          </q-card>
        </div>
      </div>

      <!-- Show incompatible -->
      <div>
        <q-checkbox
          label="Show incompatible"
          v-model="showIncompatible"
        ></q-checkbox>
      </div>
    </q-card>

    <!-- Download dialog -->
    <q-dialog v-model="downloadDialog" v-if="selectedPlatform">
      <q-card style="min-width: 420px; min-height: 200px">
        <!-- Title -->
        <q-card-section>
          <div class="text-subtitle1 text-bold text-center text-primary">
            INSTALL {{ selectedPlatform.name.toUpperCase() }}
          </div>
        </q-card-section>

        <!-- Versions -->
        <div>
          <div
            v-for="(compat, version) in selectedPlatform.versions"
            class="q-pa-sm"
          >
            <div class="row items-center">
              <div class="text-body1 q-pb-xs q-px-sm">
                Version: <b>{{ version }}</b
                >, Compatibility:
                <span
                  class="text-bold"
                  :class="{
                    'text-red': compat != $1t.info.value.customPlatformCompat,
                    'text-green': compat == $1t.info.value.customPlatformCompat,
                  }"
                  >{{ compat }}</span
                >
              </div>
              <q-space></q-space>
              <q-btn
                icon="mdi-download"
                flat
                round
                @click="installPlatform(selectedPlatform!, version.toString())"
              ></q-btn>
            </div>
          </div>
        </div>
      </q-card>
    </q-dialog>
  </q-dialog>
</template>

<script lang="ts" setup>
import { useDialogPluginComponent, useQuasar } from "quasar";
import { RepoPlatform } from "../scripts/autotagger";
import { PropType, computed, ref } from "vue";
import { get1t } from "../scripts/onetagger";

const ICON_URL =
  "https://raw.githubusercontent.com/Marekkon5/onetagger-platforms/master/platforms";

const { manifest } = defineProps({
  manifest: { required: true, type: Object as PropType<RepoPlatform[]> },
});
const { dialogRef, onDialogHide, onDialogOK, onDialogCancel } =
  useDialogPluginComponent();
const emit = defineEmits([...useDialogPluginComponent.emits]);
const $q = useQuasar();
const $1t = get1t();
const selectedPlatform = ref<RepoPlatform | undefined>(undefined);
const downloadDialog = ref(false);
const showIncompatible = ref(false);

function iconUrl(id: string) {
  return `${ICON_URL}/${id}/icon.png`;
}

// Install the selected platform
function installPlatform(platform: RepoPlatform, version: string) {
  $q.dialog({
    title: "Installing",
    progress: {
      color: "primary",
    },
    persistent: true,
    ok: false,
  });
  $1t.send("installPlatform", {
    id: platform.id,
    version,
    isNative: platform.language == "rust",
  });
}

/// Filter incompatible platforms
const platforms = computed(() =>
  manifest.filter(
    (p) =>
      showIncompatible.value ||
      Object.values(p.versions).includes($1t.info.value.customPlatformCompat),
  ),
);
</script>
