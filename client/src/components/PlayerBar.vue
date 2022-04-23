<template>
<div>

    <div class="row q-mx-md">
        <!-- Meta -->
        <div class="row q-mr-md" style="width: 264px">
            <div class="column q-mt-sm q-pt-xs" style="width: 200px">
                <div class="text-caption text-weight-bold full-width">
                    <div v-if="$1t.player.title" class="text-no-wrap overflow-hidden" style="text-overflow: ellipsis">
                        {{ $1t.player.title }}
                    </div>
                </div>

                <div class="text-caption full-width text-grey-5">
                    <div v-if="$1t.player.artists" class="text-no-wrap overflow-hidden" style="text-overflow: ellipsis">
                        {{ $1t.player.artists.join(', ') }}
                    </div>
                </div>
            </div>

            <!-- Controls -->
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
                    v-else
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
            <q-btn round icon="mdi-open-in-app" @click="browseQuickTag">
                <q-tooltip content-style="font-size: 13px;">
                    Click here to browse for new path
                </q-tooltip>
            </q-btn>
        </div>

        <!-- Playlist -->
        <div v-if='enablePlaylist'>
            <PlaylistDropZone
                tiny
                v-model="qtPlaylist"
                @input="loadQTPlaylist(); $1t.quickTagUnfocus()"
                @click.native='$1t.quickTagUnfocus'
                class="q-mt-sm q-mr-sm"
            ></PlaylistDropZone>
        </div>

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

</div>
</template>

<script>
import Waveform from './Waveform.vue';
import PlaylistDropZone from "./PlaylistDropZone.vue";

export default {
    name: 'PlayerBar',
    components: { Waveform, PlaylistDropZone },
    data() {
        return { 
            qtPlaylist: {},
            enablePlaylist: true,
        };
    },
    methods: {
        // Open QuickTag file browser
        browseQuickTag() {
            this.$1t.send('browse', {
                context: 'qt',
                path: this.$1t.settings.path,
            });
            this.$1t.quickTagUnfocus();
        },
        // Load quicktag playlist
        loadQTPlaylist() {
            if (!this.qtPlaylist || !this.qtPlaylist.data) {
                this.$1t.loadQuickTag();
                return;
            }
            this.$1t.loadQuickTag(this.qtPlaylist);
        },
    },
    mounted() {
        // Unfocus callback
        this.$1t.quickTagUnfocus = () => {
            this.$refs.playButton.$el.focus();
            this.$refs.playButton.$el.blur();
        }
        // Enable playlist dropzone
        this.enablePlaylist = this.$router.currentRoute.path.includes('quicktag');
    },
    destroyed() {
        // Remove callback
        this.$1t.quickTagUnfocus = () => {};
    },
    watch: {
        // Disable/enable playlist coz only for QT
        $route(r) {
            if (r.path == '/quicktag') this.enablePlaylist = true;
            else this.enablePlaylist = false;
        }
    }
}
</script>