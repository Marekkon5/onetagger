<template>
<div class='text-center'>

    <div class='text-h5 q-mt-md text-grey-4'>Tagging status</div>
    <!-- Info -->
    <div class='row q-my-sm justify-center'>
        <div class='row justify-around full-width text-subtitle1 q-my-sm q-px-xl'>
            <div class='col'>
                <q-chip icon='mdi-check' :label='countStatus("ok")' color='green'></q-chip>
                <br>
                <span class='text-grey-6'>Match</span>
            </div>

            <div class='col'>
                <q-chip icon='mdi-alert-circle' :label='countStatus("error")' color='red'></q-chip>
                <br>
                <span class='text-grey-6'>Failed</span>
            </div>
            
            <div class='col'>
                <q-chip class='text-black' icon='mdi-debug-step-over' :label='countStatus("skipped")' color='yellow'></q-chip>
                <br>
                <span class='text-grey-6'>Skipped</span>
            </div>
            
            <div class='col'>
                <q-chip class='text-black' icon='mdi-music-box-multiple-outline' :label='$1t.taggerStatus.total' color='grey-6'></q-chip>
                <br>
                <span class='text-grey-6'>Total</span>
            </div>
            
            <div class='col'>
                <q-chip class='text-black' icon='mdi-timelapse' :label='time' color='primary'></q-chip>
                <br>
                <span class='text-grey-6'>Elapsed time</span>
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
                            <span v-if='$1t.taggerStatus.type != "af"' class='selectable'>{{platformText(status.platform)}}</span>
                            <q-icon size='xs' class='q-ml-sm q-mb-xs' :name='statusIcon(status.status.status)' :color='statusColor(status.status.status)'></q-icon>
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
        //Conver platform name
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