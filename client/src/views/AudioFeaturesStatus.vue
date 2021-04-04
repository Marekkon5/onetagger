<template>
<div>

    <div class='text-h4 text-center q-my-md'>Status:</div>

    <!-- Status -->
    <div class='row justify-center q-my-md'>
        <q-chip color='primary' text-color='black' icon='mdi-timelapse' class='q-mx-sm'>{{time}}</q-chip>
        <q-chip color='green' icon='mdi-check' class='q-mx-sm'>{{stats.ok}}</q-chip>
        <q-chip color='yellow' text-color='black' icon='mdi-debug-step-over' class='q-mx-sm'>{{stats.skipped}}</q-chip>
        <q-chip color='red' icon='mdi-alert-circle' class='q-mx-sm'>{{stats.failed}}</q-chip>
    </div>

    <!-- List of statuses -->
    <div class='status-list'>
    <q-list bordered>
        <div v-for='(status, i) in $1t.audioFeatures.statuses' :key='"S"+i'>
            <q-item>
                <q-item-section avatar>
                    <q-icon :name="icon(status.state)" :color='iconColor(status.state)'></q-icon>
                </q-item-section>
                <q-item-section>
                    <q-item-label overline class='selectable'>{{status.state.toUpperCase()}}</q-item-label>
                    <q-item-label><span class='selectable'>{{status.filename}}</span></q-item-label>
                </q-item-section>
                
            </q-item>
        </div>
    </q-list>
    </div>

</div>
</template>

<script>
export default {
    name: 'AudioFeaturesStatus',
    data() {
        return {
            stats: {
                ok: 0,
                failed: 0,
                skipped: 0
            },
            time: "0:00"
        }
    },
    methods: {
        //Get state icon
        icon(status) {
            switch (status) {
                case 'skipped':
                    return 'mdi-debug-step-over'
                case 'ok':
                    return 'mdi-check'
                case 'error':
                    return 'mdi-alert-circle'
            }
        },
        //Get state icon color
        iconColor(status) {
            switch (status) {
                case 'skipped':
                    return 'yellow'
                case 'ok':
                    return 'green'
                case 'error':
                    return 'red'
            }
        },
        //Elapsed time
        calculateTime() {
            let s = ((this.$1t.audioFeatures.ended??Date.now()) - this.$1t.audioFeatures.started) / 1000;
            this.time = `${Math.floor((s/60))}:${Math.round(s%60).toString().padStart(2, '0')}`;
        }
    },
    watch: {
        //Recalculate stats
        '$1t.audioFeatures.statuses'() {
            this.calculateTime();
            let s = {ok: 0, failed: 0, skipped: 0};
            this.$1t.audioFeatures.statuses.forEach((status) => {
                switch (status.state) {
                    case 'skipped':
                        s.skipped++; break;
                    case 'ok':
                        s.ok++; break;
                    case 'error':
                        s.failed++; break;
                }
            });
            this.stats = s;
        }
    },
    mounted() {
        //Register done callback
        this.$1t.onTaggingDone = () => {
            this.$q.dialog({
                title: 'Done',
                message: 'Tagging done!',
                ok: {
                    color: 'primary',
                }
            });
        };
    }
}
</script>

<style>
.status-list {
    width: 70%;
    margin-left: 15%;
}
</style>