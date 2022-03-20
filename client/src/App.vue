<template>
    <div>
        <q-layout view="hHh lpR fFf" class="bg-background">
            <!-- Header -->
            <q-header class="bg-darker text-white" height-hint="98">
                <q-toolbar class="row justify-between">
                    <div class="col-2" style="height: 58px">
                        <img
                            src="./assets/icon.png"
                            height="42"
                            @click="home"
                            class="logo q-mt-sm"
                            :class="{ spin: $1t.lock.locked }"
                        />
                        <img
                            src="./assets/logo-text.png"
                            height="42"
                            @click="home"
                            class="logo"
                        />
                    </div>

                    <div class="col-8">
                        <q-tabs style="padding-top: 8px">
                            <q-route-tab
                                :disable="$1t.lock.locked"
                                to="/"
                                class="text-weight-bolder"
                                @click="hideSide"
                                ><q-icon name="mdi-home" size="sm"></q-icon
                            ></q-route-tab>
                            <q-route-tab
                                :disable="$1t.lock.locked"
                                to="/autotagger"
                                class="text-weight-bolder"
                                @click="hideSide"
                                >Auto tag</q-route-tab
                            >
                            <q-route-tab
                                :disable="$1t.lock.locked"
                                to="/audiofeatures"
                                class="text-weight-bolder"
                                @click="audioFeatures"
                                >Audio features</q-route-tab
                            >
                            <q-route-tab
                                :disable="$1t.lock.locked"
                                to="/quicktag"
                                class="text-weight-bolder"
                                @click="showSide"
                                >Quick Tag</q-route-tab
                            >
                            <q-route-tab
                                :disable="$1t.lock.locked"
                                to="/tageditor"
                                class="text-weight-bolder"
                                @click="hideSide"
                                >Edit Tags</q-route-tab
                            >

                            <q-route-tab
                                :disable="$1t.lock.locked"
                                to="/renamer"
                                class="text-weight-bolder"
                                @click="hideSide"
                                >Renamer</q-route-tab>
                            
                        </q-tabs>
                    </div>

                    <!-- Settings -->
                    <div class="col-2 row justify-end items-center">
                        <q-btn
                            flat
                            round
                            dense
                            icon="mdi-cog"
                            @click="settings = true"
                        ></q-btn>
                    </div>
                </q-toolbar>
            </q-header>

            <HelpButton></HelpButton>

            <!-- Drawers -->
            <div v-if='$1t.info.ready'>
                <q-drawer
                    :breakpoint="1000"
                    v-model="left"
                    side="left"
                    :width="200"
                >
                    <QuickTagFileBrowser v-if='left'></QuickTagFileBrowser>
                </q-drawer>
                <q-drawer
                    :breakpoint="1000"
                    v-model="right"
                    side="right"
                    :width="200"
                >
                    <QuickTagRight></QuickTagRight>
                </q-drawer>
            </div>

            <!-- Content -->
            <q-page-container class="content" ref="contentContainer">
                <transition name="fade" v-if='$1t.info.ready'>
                    <keep-alive :include='["AudioFeatures"]'>
                        <router-view />
                    </keep-alive>
                </transition>
                <!-- Loading -->
                <div v-if='!$1t.info.ready' class='row justify-center items-center' style='height: calc(100vh - 64px)'>
                    <q-circular-progress indeterminate color='primary' size='64px'></q-circular-progress>
                </div>
            </q-page-container>



            <!-- Footer -->
            <q-footer reveal class="bg-darker text-white" v-if="footer">
                
                <QuickTagMoods v-if="$1t.quickTag.track"></QuickTagMoods>
                <QuickTagGenreBar v-if="$1t.quickTag.track"></QuickTagGenreBar>

                <div class="row q-mx-md">
                    <div class="row q-mr-md" style="width: 264px">
                        <div
                            class="column q-mt-sm q-pt-xs"
                            style="width: 200px"
                        >
                            <div
                                class="text-caption text-weight-bold full-width"
                            >
                                <div
                                    v-if="$1t.quickTag.track"
                                    class="text-no-wrap overflow-hidden"
                                    style="text-overflow: ellipsis"
                                >
                                    {{ $1t.quickTag.track.title }}
                                </div>
                            </div>
                            <div class="text-caption full-width text-grey-5">
                                <div
                                    v-if="$1t.quickTag.track"
                                    class="text-no-wrap overflow-hidden"
                                    style="text-overflow: ellipsis"
                                >
                                    {{ $1t.quickTag.track.artists.join(", ") }}
                                </div>
                            </div>
                        </div>

                        <div class="col q-mt-sm" style="margin-left: 16px">
                            <!-- Play button -->
                            <q-btn
                                round
                                flat
                                icon="mdi-play"
                                class="q-mr-sm"
                                :ripple="false"
                                v-if="!$1t.player.playing"
                                @click="$1t.play()"
                                ref='playButton'
                            ></q-btn>
                            <!-- Pause -->
                            <q-btn
                                round
                                flat
                                icon="mdi-pause"
                                class="q-mr-sm"
                                :ripple="false"
                                v-if="$1t.player.playing"
                                @click="$1t.pause()"
                                ref='playButton'
                            ></q-btn>
                        </div>
                    </div>

                    <div class="col">
                        <Waveform></Waveform>
                    </div>

                    <!-- Browse button -->
                    <div class="q-mt-sm q-pr-sm">
                        <q-btn
                            round
                            icon="mdi-open-in-app"
                            @click="browseQuickTag"
                        >
                            <q-tooltip content-style="font-size: 13px;">
                                Click here to browse for new path
                            </q-tooltip>
                        </q-btn>
                    </div>

                    <!-- Playlist -->
                    <PlaylistDropZone
                        tiny
                        v-model="qtPlaylist"
                        @input="loadQTPlaylist(); quickTagUnfocus()"
                        @click.native='quickTagUnfocus'
                        class="q-mt-sm q-mr-sm"
                    ></PlaylistDropZone>

                    <!-- Volume -->
                    <div class="q-pt-sm" style="width: 90px">
                        <q-slider
                            v-model="$1t.player.volume"
                            :min="0.0"
                            :max="1.0"
                            :step="0.01"
                            @input="$1t.setVolume($event)"
                            @change="$1t.saveSettings(false)"
                            style="margin-top: 6px"
                        ></q-slider>
                    </div>
                </div>
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
                    One Tagger requires atleast 1150x550 window size. Please
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
                        <div
                            v-html="update.changelog"
                            class="text-subtitle1"
                        ></div>
                    </div>
                </q-card-section>
                <q-card-section class="justify-center row">
                    <q-btn
                        color="primary"
                        class="text-black"
                        @click="$1t.url(update.url)"
                    >
                        Download
                    </q-btn>
                </q-card-section>
            </q-card>
        </q-dialog>
    </div>
</template>

<script>
    import Waveform from "./components/Waveform.vue";
    import QuickTagFileBrowser from './components/QuickTagFileBrowser.vue';
    import Settings from "./components/Settings";
    import QuickTagGenreBar from "./components/QuickTagGenreBar";
    import QuickTagMoods from './components/QuickTagMoods.vue';
    import QuickTagRight from "./components/QuickTagRight";
    import HelpButton from "./components/HelpButton";
    import PlaylistDropZone from "./components/PlaylistDropZone.vue";

    import axios from "axios";
    import compareVersions from "compare-versions";

    export default {
        name: "App",
        components: {
            Waveform,
            Settings,
            QuickTagGenreBar,
            QuickTagRight,
            HelpButton,
            PlaylistDropZone,
            QuickTagFileBrowser,
            QuickTagMoods
        },
        data() {
            return {
                left: false,
                right: false,
                footer: false,
                settings: false,
                sizeDialog: false,
                update: null,
                updateDialog: false,
                qtPlaylist: {},
            };
        },
        methods: {
            // Hide/Show footer and drawer
            hideSide() {
                this.left = false;
                this.right = false;
                this.footer = false;
            },
            showSide() {
                this.left = true;
                this.right = true;
                this.footer = true;
            },
            // Navigate to homepage
            home() {
                if (!this.$1t.lock.locked) {
                    this.hideSide();
                    this.$router.push("/");
                }
            },
            // Navigate to audio features
            audioFeatures() {
                if (!this.$1t.lock.locked) {
                    this.hideSide();
                    this.$router.push("/audiofeatures");
                }
            },
            // Load quicktag playlist
            loadQTPlaylist() {
                if (!this.qtPlaylist || !this.qtPlaylist.data) {
                    this.$1t.loadQuickTag();
                    return;
                }
                this.$1t.loadQuickTag(this.qtPlaylist);
            },
            async checkUpdates() {
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
                if (compareVersions(data.version, this.$1t.info.version) == 1) {
                    this.update = data;
                    this.$q.notify({
                        message: `New update available (${data.version})!`,
                        timeout: 10000,
                        progress: true,
                        actions: [
                            {
                                label: "Show",
                                handler: () => {
                                    this.updateDialog = true;
                                },
                            },
                        ],
                    });
                }
            },
            // Unfocus from current element to make shortcuts work
            quickTagUnfocus() {
                if (this.$router.currentRoute.path.includes('quicktag')) {
                    this.$refs.playButton.$el.focus();
                    this.$refs.playButton.$el.blur();
                }
            },
            settingsClosed() {
                this.settings = false;
                this.quickTagUnfocus();
            },
            browseQuickTag() {
                this.$1t.send('browse', {
                    context: 'qt',
                    path: this.$1t.settings.path,
                });
                this.quickTagUnfocus();
            }
        },
        mounted() {
            this.$q.dark.set(true);

            //Handle resize to apply min height/width
            window.addEventListener("resize", () => {
                if (window.innerHeight < 550 || window.innerWidth < 1150) {
                    this.sizeDialog = true;
                } else {
                    this.sizeDialog = false;
                }
            });

            //Show QT sidebar
            if (this.$router.currentRoute.path.includes('quicktag')) {
                this.showSide();
            }

            //Wait for app to load
            setTimeout(() => this.checkUpdates(), 2000);
        },
        watch: {
            //Dont show scrollbar while transition
            $route() {
                this.$refs.contentContainer.$el.style.overflowY = "hidden";
            },
        },
        updated() {
            //Show again scrollbar after transition
            setTimeout(() => {
                this.$refs.contentContainer.$el.style.overflowY = "auto";
            }, 250);
        },
    };
</script>

<style>
    .content {
        overflow-y: auto;
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
</style>