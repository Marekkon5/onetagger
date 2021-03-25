<template>
<div class='text-center'>

    <div class='text-h5 q-mt-md'>Tagging status:</div>
    <!-- Chips -->
    <div class='row q-my-sm chips'>
        <q-chip color='primary' text-color='black' icon='mdi-timelapse' class='q-mx-sm'>
            {{time}}
        </q-chip>
        <q-chip color='red' icon='mdi-alert-circle' class='q-mx-sm'>
            {{$1t.taggerStatus.statuses.length}}
        </q-chip>
        <q-chip color='green' icon='mdi-check' class='q-mx-sm'>
            {{$1t.taggerStatus.ok}}
        </q-chip>
    </div>
    <!-- Failed -->
    <div class='text-h5 q-mt-md'>Failed tracks:</div>
    <q-list class='list q-mt-md text-left'>
        <div v-for='(status, i) in $1t.taggerStatus.statuses' :key='i'>
            <q-item class='item'>
                <q-item-section>
                    <q-item-label overline>
                        <span :class='color(status.platform)'>{{status.platform.toUpperCase()}}</span> | {{status.status.status.toUpperCase()}}
                    </q-item-label>
                    {{status.status.path}}
                </q-item-section>
            </q-item>
        </div>
    </q-list>

    <!-- Progressbar -->
    <div class='progress'>
        <q-linear-progress 
            :value='$1t.taggerStatus.progress'
            color='primary' 
            size='4px'
        ></q-linear-progress>
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
                    return 'text-yellow-6';
                case 'junodownload':
                    return 'text-light-green-7'
                default:
                    return '';
            }
        }
    },
    mounted() {
        //Update timestamp
        this.timeInterval = setInterval(() => {
            //Already done
            if (this.$1t.taggerStatus.done) {
                if (this.timeInterval != null)
                    this.clearInterval(this.timeInterval);
                return;
            }
            //Timestamp
            let s = (Date.now() - this.$1t.taggerStatus.started) / 1000;
            this.time = `${Math.floor((s/60))}:${Math.round(s%60).toString().padStart(2, '0')}`;
        }, 500);
        //Done callback
        this.$1t.onTaggingDone = () => {
            clearInterval(this.timeInterval);
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
.chips {
    justify-content: center;
}
.progress {
    width: 100%;
    position: absolute;
    bottom: 0px;
}
.item {
    background-color: #242626;
}
</style>