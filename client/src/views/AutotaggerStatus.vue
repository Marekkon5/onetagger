<template>
<div class='text-center'>

    <div class='text-h5 q-mt-md text-grey-4'>Tagging status</div>
    <!-- Info -->
    <div class='row q-my-sm justify-center'>
        <div class='row justify-between full-width text-subtitle2 q-my-sm list'>
            <div class='col q-mr-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-check' round :color='filter == "ok" ? "primary" : "green"' class='text-black' @click='toggleFilter("ok")'>
                                <q-tooltip content-style="font-size: 13px">
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
                                <q-tooltip content-style="font-size: 13px">
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
                                <q-tooltip content-style="font-size: 13px">
                                    Total amount skipped due missing tags or corruption
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
                                <q-tooltip content-style="font-size: 13px">
                                    Total amount of files to process
                                </q-tooltip>
                            </q-btn>
                        </div>
                        <div class='col q-my-sm text-right q-pr-md'>
                            <div class='text-subtitle2 text-grey-6'>Total</div>
                            <div class='text-subtitle1 monospace text-weight-bold'>{{$1t.taggerStatus.total}}</div>
                        </div>
                    </div>
                </q-card>
            </div>
            
            <div class='col q-ml-sm'>
                <q-card flat>
                    <div class='row'>
                        <div class='col q-mt-sm q-pt-xs text-left q-pl-md'>
                            <q-btn icon='mdi-timelapse' round color='teal' class='text-black'>
                                <q-tooltip content-style="font-size: 13px">
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
    <!-- Statuses -->
    <q-list class='list text-left bg-dark q-py-sm'>
        <q-virtual-scroll :items='statuses' class='status-list'>
            <template v-slot="{item, i}">
                <q-item :key='i'>
                    <q-item-section>
                        <q-item-label overline>
                            <span>
                                <span v-if='$1t.taggerStatus.type != "audioFeatures"' class='selectable text-white'>{{platformText(item.platform)}}</span>
                                <img width='14' class='q-ml-sm' style='margin-bottom: -2px;' v-if='item.status.usedShazam' :src='require("../assets/shazam_icon.svg")' />
                                <q-icon size='xs' class='q-ml-sm q-mb-xs' :name='statusIcon(item.status.status)' :color='statusColor(item.status.status)'></q-icon>
                            </span>
                        </q-item-label>
                        <span class='selectable text-grey-5'>{{item.status.path}}</span>
                    </q-item-section>
                </q-item>
            </template>
        </q-virtual-scroll>
    </q-list>

    <!-- Progressbar -->
    <div class='progress'>
        <q-linear-progress 
            :value='$1t.taggerStatus.progress'
            color='primary' 
            size='20px'
        >
            <div class='absolute-full flex flex-center'>
                <span class='text-black text-subtitle2'>
                    {{Math.round($1t.taggerStatus.progress * 100) + "%"}}
                </span>
            </div>
        </q-linear-progress>
    </div>

</div>
</template>

<script>
export default {
    name: 'AutotaggerStatus',
    data() {
        return {
            time: '0:00',
            timeInterval: null,
            filter: null
        }
    },
    methods: {
        // Conver platform name
        platformText(p) {
            if (p == 'junodownload') return 'JUNO DOWNLOAD';
            return p.toUpperCase();
        },
        statusIcon(s) {
            switch (s) {
                case 'error': return 'mdi-alert-circle';
                case 'ok': return 'mdi-check';
                case 'skipped': return 'mdi-debug-step-over';
            }
        },
        statusColor(s) {
            switch (s) {
                case 'error': return 'red';
                case 'ok': return 'green';
                case 'skipped': return 'yellow';
            }
        },
        countStatus(status) {
            return this.$1t.taggerStatus.statuses.reduce((a, c) => (c.status.status == status) ? a + 1 : a, 0);
        },
        // Toggle status filter
        toggleFilter(name) {
            if (this.filter == name) {
                this.filter = null;
                return;
            }
            this.filter = name;
        }
    },
    computed: {
        statuses() {
            if (!this.filter)
                return this.$1t.taggerStatus.statuses;
            return this.$1t.taggerStatus.statuses.filter((s) => s.status.status == this.filter);
        }
    },
    mounted() {
        // Update timestamp
        this.timeInterval = setInterval(() => {
            // Already done
            if (this.$1t.taggerStatus.done || !this.$1t.lock.locked) {
                if (this.timeInterval != null)
                    clearInterval(this.timeInterval);
                return;
            }
            // Timestamp
            let s = (Date.now() - this.$1t.taggerStatus.started) / 1000;
            this.time = `${Math.floor((s/60))}:${Math.round(s%60).toString().padStart(2, '0')}`;
        }, 400);
        // Done callback
        this.$1t.onTaggingDone = (path) => {
            this.$q.dialog({
                title: 'Done',
                message: 'Tagging finished! Would you like to open the folder?',
                html: true,
                ok: {
                    color: 'primary',
                },
                cancel: true
            })
            .onOk(() => {
                if (path) {
                    this.$1t.send('openFolder', {path});
                }
            })
        }
    },
}
</script>

<style>
.list {
    max-width: 80%;
    margin-left: 10%;    
}
.progress {
    width: 100%;
    position: absolute;
    bottom: 0px;
}
.status-list {
    height: calc(100vh - 248px);
}
</style>