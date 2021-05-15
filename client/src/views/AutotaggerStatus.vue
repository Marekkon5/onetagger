<template>
<div class='text-center'>

    <div class='text-h5 q-mt-md text-grey-4'>Tagging status</div>
    <!-- Info -->
    <div class='row q-my-sm justify-center'>
        <div class='row justify-around full-width text-subtitle1 q-my-sm q-px-xl'>
            <div class='col'>
                <q-icon name='mdi-check' class='q-mb-xs q-mr-sm'></q-icon>
                <span>Successful: </span>
                <span class='text-weight-bold'>{{countStatus('ok')}}</span>
            </div>

            <div class='col'>
                <q-icon name='mdi-alert-circle' class='q-mb-xs q-mr-sm'></q-icon>
                <span>Failed: </span>
                <span class='text-weight-bold'>{{countStatus('error')}}</span>
            </div>
            
            <div class='col'>
                <q-icon name='mdi-debug-step-over' class='q-mb-xs q-mr-sm'></q-icon>
                <span>Skipped: </span>
                <span class='text-weight-bold'>{{countStatus('skipped')}}</span>
            </div>
            
            <div class='col'>
                <q-icon name='mdi-music-box-multiple-outline' class='q-mb-xs q-mr-sm'></q-icon>
                <span>Total: </span>
                <span class='text-weight-bold'>{{$1t.taggerStatus.total}}</span>
            </div>
            
            <div class='col'>
                <q-icon name='mdi-timelapse' class='q-mb-xs q-mr-sm'></q-icon>
                <span>Elapsed time: </span><span class='text-weight-bold'>{{time}}</span>
            </div>
        </div>
    </div>
    <!-- Statuses -->
    <q-list class='list q-mt-xl q-mb-xl text-left bg-dark q-py-sm'>
        <div v-for='(status, i) in $1t.taggerStatus.statuses' :key='i'>
            <q-item>
                <q-item-section>
                    <q-item-label overline>
                        <span>
                            <span v-if='$1t.taggerStatus.type != "af"' class='selectable' :class='color(status.platform)'>{{platformText(status.platform)}} | </span>
                            <span class='selectable' :class='"text-" + statusColor(status.status.status)'>{{statusText(status.status.status)}}</span>
                        </span>
                    </q-item-label>
                    <span class='selectable'>{{status.status.path}}</span>
                </q-item-section>
            </q-item>
        </div>
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
            timeInterval: null
        }
    },
    methods: {
        color(v) {
            switch (v) {
                case 'beatport':
                    return 'text-lime-14';
                case 'traxsource':
                    return 'text-light-blue-7';
                case 'discogs':
                    return 'text-yellow-7';
                case 'junodownload':
                    return 'text-light-green-7'
                default:
                    return '';
            }
        },
        //Convert status
        statusText(s) {
            if (s == 'error') return 'NO MATCH';
            return s.toUpperCase();
        },
        //Conver platform name
        platformText(p) {
            if (p == 'junodownload') return 'JUNO DOWNLOAD';
            return p.toUpperCase();
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
        }
    },
    mounted() {
        //Update timestamp
        this.timeInterval = setInterval(() => {
            //Already done
            if (this.$1t.taggerStatus.done || !this.$1t.lock.locked) {
                if (this.timeInterval != null)
                    clearInterval(this.timeInterval);
                return;
            }
            //Timestamp
            let s = (Date.now() - this.$1t.taggerStatus.started) / 1000;
            this.time = `${Math.floor((s/60))}:${Math.round(s%60).toString().padStart(2, '0')}`;
        }, 400);
        //Done callback
        this.$1t.onTaggingDone = () => {
            this.$q.dialog({
                title: 'Done',
                message: 'Tagging done!',
                ok: {
                    color: 'primary',
                }
            });
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
</style>