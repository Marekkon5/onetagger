<template>
    <div>
        <q-layout view="hHh lpR fFf" class="bg-background">
            <!-- Header -->
            <q-header class="bg-darker text-white" height-hint="98">
                <q-toolbar class="row justify-between">
                    <div class="col-2 row" style="height: 58px">
                        <img src="./assets/icon.png" height="42" @click="home" class="logo q-mt-sm" :class="{ spin: $1t.lock.value.locked }" />
                        <LogoText class='q-mt-sm q-ml-xs'></LogoText>
                    </div>
                    
                    <div class="col-8">
                        <q-tabs style="padding-top: 8px">
                            <q-route-tab :disable="$1t.lock.value.locked" to="/" class="text-weight-bolder" @click="hideSide">
                                <q-icon name="mdi-home" size="sm"></q-icon>
                            </q-route-tab>
                            <q-route-tab :disable="$1t.lock.value.locked" to="/autotagger" class="text-weight-bolder" @click="hideSide" >Auto tag</q-route-tab >
                            <q-route-tab :disable="$1t.lock.value.locked" to="/audiofeatures" class="text-weight-bolder" @click="audioFeatures" >Audio features</q-route-tab >
                            <q-route-tab :disable="$1t.lock.value.locked" to="/quicktag" class="text-weight-bolder" @click="showSide" >Quick Tag</q-route-tab >
                            <q-route-tab :disable="$1t.lock.value.locked" to="/tageditor" class="text-weight-bolder" @click="hideSide" >Edit Tags</q-route-tab >
                            <q-route-tab :disable="$1t.lock.value.locked" to="/renamer" class="text-weight-bolder" @click="hideSide" >Auto Rename</q-route-tab>
                        </q-tabs>
                    </div>

                    <!-- Settings -->
                    <div class="col-2 row justify-end items-center">
                        <q-btn flat round dense icon="mdi-cog" @click="settings = true" ></q-btn>
                    </div>
                </q-toolbar>
            </q-header>

            <!-- Help button -->
            <HelpButton v-if='$1t.info.value.ready'></HelpButton>

            <!-- Drawers -->
            <div v-if='$1t.info.value.ready'>
                <q-drawer :breakpoint="1000" v-model="left" side="left" :width="200" >
                    <QuickTagFileBrowser v-if='left'></QuickTagFileBrowser>
                </q-drawer>
                <q-drawer :breakpoint="1000" v-model="right" side="right" :width="200" >
                    <QuickTagRight></QuickTagRight>
                </q-drawer>
            </div>

            <!-- Content -->
            <q-page-container class="content" ref="contentContainer">
                <router-view v-slot='{ Component }'>
                    <transition name="fade" v-if='$1t.info.value.ready'>
                        <keep-alive :include='["AudioFeatures"]'>
                            <component :is='Component'></component>
                        </keep-alive>
                    </transition>
                </router-view>
                <!-- Loading -->
                <div v-if='!$1t.info.value.ready' class='row justify-center items-center' style='height: calc(100vh - 64px)'>
                    <q-circular-progress indeterminate color='primary' size='64px'></q-circular-progress>
                </div>
            </q-page-container>



            <!-- Footer -->
            <q-footer reveal class="bg-darker text-white" v-if="footer">
                
                <div v-if='isRoute("quicktag")'>
                    <QuickTagMoods v-if="$1t.quickTag.value.track"></QuickTagMoods>
                    <QuickTagGenreBar v-if="$1t.quickTag.value.track"></QuickTagGenreBar>
                </div>

                <PlayerBar v-if='($1t.settings.value.tagEditorPlayer && isRoute("tageditor")) || isRoute("quicktag")'></PlayerBar>
                
            </q-footer>
        </q-layout>

        <!-- Settings -->
        <Settings v-model="settings" @close="settingsClosed"></Settings>

        <!-- Min size dialog -->
        <q-dialog v-model="sizeDialog" persistent>
            <q-card>
                <q-card-section>
                    <div class="text-h6">Warning</div>
                </q-card-section>
                <q-card-section>
                    One Tagger requires atleast 1200x550 window size. Please
                    resize to continue.
                </q-card-section>
            </q-card>
        </q-dialog>

        <!-- Update dialog -->
        <q-dialog v-model="updateDialog">
            <q-card v-if="update">
                <q-card-section class="text-center">
                    <div class="text-h5">New update available!</div>
                </q-card-section>
                <q-card-section>
                    <div class="text-center">
                        <div class="text-h6 text-weight-bold">
                            {{ update.version }}
                        </div>
                        <br />
                        <div v-html="update.changelog" class="text-subtitle1"></div>
                    </div>
                </q-card-section>
                <q-card-section class="justify-center row">
                    <q-btn color="primary" class="text-black" @click="$1t.url(update!.url)" > Download </q-btn>
                </q-card-section>
            </q-card>
        </q-dialog>

        <!-- Folder browser dialog -->
        <q-dialog v-model='$1t.folderBrowser.value.open'>
            <FolderBrowser v-if='$1t.folderBrowser.value.open' :base='$1t.folderBrowser.value.basePath'></FolderBrowser>
        </q-dialog>
    </div>
</template>

<script lang='ts' setup>
import axios from 'axios';

import { compareVersions } from 'compare-versions';
import { useQuasar } from 'quasar';
import {computed, onMounted, onUpdated, ref, watch} from "vue";
import { useRoute, useRouter } from "vue-router";
import { get1t } from "./scripts/onetagger.js";

import HelpButton from './components/HelpButton.vue';
import Settings from './components/Settings.vue';
import QuickTagRight from './components/QuickTagRight.vue';
import QuickTagGenreBar from './components/QuickTagGenreBar.vue';
import QuickTagMoods from './components/QuickTagMoods.vue';
import QuickTagFileBrowser from './components/QuickTagFileBrowser.vue';
import PlayerBar from './components/PlayerBar.vue';
import FolderBrowser from './components/FolderBrowser.vue';
import LogoText from './components/LogoText.vue';

const $1t = get1t();
const $q = useQuasar();
const router = useRouter();

const left = ref(false);
const right = ref(false);
const footer = ref(false);
const settings = ref(false);
const sizeDialog = ref(false);
const update = ref<undefined | { url: string, version: string, changelog: string }>(undefined);
const updateDialog = ref(false);

// Hide/Show footer and drawer
function hideSide() {
    left.value = false;
    right.value = false;
    footer.value = false;
};

function showSide() {
    left.value = true;
    right.value = true;
    footer.value = true;
}

// Navigate to homepage
function home() {
    if (!$1t.lock.value.locked) {
        hideSide();
        router.push("/");
    }
};

/// Navigate to audio features
function audioFeatures() {
    if (!$1t.lock.value.locked) {
        hideSide();
        router.push("/audiofeatures");
    }
}

/// Check if is on route
function isRoute(route: string) {
    return router.currentRoute.value.path.includes(route);
}

/// Check for updates
async function checkUpdates() {
    // Fetch latest version info
    let url = "https://1t.marekkon5.workers.dev/latest";
    let data = null;
    try {
        let res = await axios.get(url);
        data = res.data;
    } catch (e) {
        return;
    }
    if (!data) return;
    // New version
    if (compareVersions(data.version, $1t.info.value.version) == 1) {
        update.value = data;
        $q.notify({
            message: `New update available (${data.version})!`,
            timeout: 10000,
            progress: true,
            actions: [
                {
                    label: "Show",
                    handler: () => {
                        updateDialog.value = true;
                    },
                },
            ],
            position: 'top-right'
        });
    }
}

// When settings closed
function settingsClosed() {
    settings.value = false;
    $1t.quickTagUnfocus();
}

function setWaveformWidth() {
    document.documentElement.style.setProperty('--waveform-wave', Math.min(Math.round(20 + ((window.innerWidth - 1200) / 10)), 70).toString());
}

// Setup
onMounted(() => {
    $q.dark.set(true);

    // Handle resize to apply min height/width
    setWaveformWidth();
    window.addEventListener("resize", () => {
        // Fix waveform
        setWaveformWidth();
        
        if (window.innerHeight < 550 || window.innerWidth < 1200) {
            sizeDialog.value = true;
        } else {
            sizeDialog.value = false;
        }
    });

    // Show QT sidebar
    if (isRoute('quicktag')) {
        showSide();
    }
    // Player
    if (isRoute('tageditor')) {
        footer.value = true;
    }

    // Wait for app to load to check for updates
    setTimeout(() => checkUpdates(), 5000);
});

// Dont show scrollbar while transition
const contentContainer = ref(null);
watch(useRoute(), (r) => {
    // @ts-ignore
    contentContainer.value!.$el.style.overflowY = "hidden";
    if (r.path == '/quicktag') showSide();
    if (r.path == '/tageditor') {
        hideSide();
        footer.value = true;
    }
});

// Show again scrollbar after transition
onUpdated(() => {
    setTimeout(() => {
        // @ts-ignore
        contentContainer.value!.$el.style.overflowY = "auto";
    }, 250);
});

</script>

<style lang='scss'>
.content {
    overflow-y: auto !important;
    height: calc(100vh);
    min-height: 100vh;
}

.logo {
    cursor: pointer;
}

.fade-enter-active,
.fade-leave-active {
    transition-property: opacity;
    transition-duration: 0.25s;
}
.fade-enter-active {
    transition-delay: 0.25s;
}
.fade-enter,
.fade-leave-active {
    opacity: 0;
}

@keyframes rotation {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.spin {
    animation: rotation 2s infinite linear;
}

/* Hide tabs arrow */
.q-tabs__arrow { 
    opacity: 0 !important;
}
.q-tab__indicator {
    background-color: var(--q-primary) !important;
    color: var(--q-primary) !important;
}
</style>