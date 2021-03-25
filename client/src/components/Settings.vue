<template>
<q-dialog v-model='open' @hide='close()'>

    <q-card class='parent-card'>
        <!-- Header -->
        <q-card-section>
            <div class='text-h6 q-mb-sm'>Settings</div>
            <q-tabs v-model='tab'>
                <q-tab label='QuickTag' name='quicktag'></q-tab>
            </q-tabs>
        </q-card-section>
        <!-- Content -->
        <q-card-section>
            <!-- Quicktag options -->
            <div v-if='tab == "quicktag"'>
                <!-- Path options -->
                <div class='text-subtitle1'>Folder:</div>
                <div class='row q-mb-sm items-center'>
                    <div class='col-11 text-subtitle2'>Current: {{$1t.settings.quickTag.path}}</div>
                    <div class='col-1'>
                        <q-btn round flat icon='mdi-open-in-app' @click='browseQuickTag'></q-btn>
                    </div>
                </div>

                <!-- Energy keybinds -->
                <div class='text-subtitle1 q-mb-sm'>Energy keybinds:</div>
                <div class='row'>
                    <div v-for='i in 5' :key='"energy" + i' class='col row'>
                        <div class='col-4 q-pt-xs'>
                            <span>{{i}}</span>
                            <q-icon name='mdi-star' size='xs' class='q-pl-xs q-pb-xs' color='yellow'></q-icon>
                        </div>
                        <div class='col-8'>
                            <Keybind 
                                class='energy-keybind' 
                                @set='setEnergyKeybind(i-1, $event)'
                                :initial='$1t.settings.quickTag.energyKeys[i-1]'
                            ></Keybind>
                        </div>
                    </div>
                </div>
                <!-- Energy settings -->
                <div class='text-subtitle1 q-mb-sm'>Energy tag options:</div>
                <q-select
                    v-model='$1t.settings.quickTag.energyTag.type'
                    dense
                    outlined
                    label='Tag type'
                    :options='["rating", "symbol"]'
                    class='q-mb-sm q-pr-md'
                ></q-select>
                <div v-if='$1t.settings.quickTag.energyTag.type != "rating"' class='row'>
                    <div class='col-2 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.energyTag.symbol' outlined dense label='Symbol'></q-input>
                    </div>
                    <div class='col-5 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.energyTag.id3' outlined dense label='ID3 (MP3, AIFF)'></q-input>
                    </div>
                    <div class='col-5 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.energyTag.vorbis' outlined dense label='FLAC'></q-input>
                    </div>
                </div>
                <!-- Mood tag -->
                <div class='text-subtitle1 q-my-sm'>Mood tag:</div>
                <div class='row'>
                    <div class='col-6 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.moodTag.id3' outlined dense label='ID3 (MP3, AIFF)'></q-input>
                    </div>
                    <div class='col-6 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.moodTag.vorbis' outlined dense label='FLAC'></q-input>
                    </div>
                </div>
                <!-- Moods -->
                <div class='text-subtitle1 q-my-sm'>Moods:</div>
                <div>
                    <div v-for='(mood, i) in $1t.settings.quickTag.moods' :key='"mood"+i'>
                        <div class='row justify-around'>
                            <q-input v-model='$1t.settings.quickTag.moods[i].mood' outlined dense class='col-5 q-pr-md q-mb-sm'></q-input>
                            <q-select 
                                v-model='$1t.settings.quickTag.moods[i].color' 
                                dense 
                                outlined 
                                label='Color'
                                :options='colors' 
                                :label-color='$1t.settings.quickTag.moods[i].color'
                                :color='$1t.settings.quickTag.moods[i].color'
                                class='col-5 q-pr-md'
                            ></q-select>
                            <Keybind 
                                class='col-2 text-center' 
                                @set='setMoodKeybind(i, $event)'
                                :initial='$1t.settings.quickTag.moods[i].keybind'
                            ></Keybind>
                        </div>
                    </div>
                    <!-- Add new mood -->
                    <div class='text-subtitle2 q-mb-sm'>Create new mood:</div>
                    <div class='row'>
                        <q-input v-model='newMood.mood' outlined dense class='col-6 q-pr-md'></q-input>
                        <q-select v-model='newMood.color' :options='colors' outlined dense class='col-5 q-pr-md'></q-select>
                        <q-btn @click='addMood' round flat dense icon='mdi-plus' class='col-1 text-primary'></q-btn>
                    </div>
                </div>

            </div>
        </q-card-section>
    </q-card>

</q-dialog>
</template>

<script>
import Keybind from './Keybind';

export default {
    name: 'Settings',
    components: {Keybind},
    data() {
        return {
            open: this.value,
            tab: 'quicktag',
            colors: ['red', 'pink', 'purple', 'deep-purple', 'indigo', 'blue', 'light-blue',
                'cyan', 'teal', 'green', 'light-green', 'lime', 'yellow', 'amber', 'orange',
                'deep-orange', 'brown', 'grey', 'blue-grey'],
            newMood: {mood: null, color: 'red'}
        }
    },
    props: {
        value: {
            type: Boolean
        }
    },
    methods: {
        //Adds new quicktag mood
        addMood() {
            if (this.newMood.mood) {
                //Exists
                if (this.$1t.settings.quickTag.moods.find(m => this.newMood.mood.toLowerCase() == m.mood.toLowerCase())) return;
                this.$1t.settings.quickTag.moods.push(this.newMood);
            }
        },
        //Mood keybind
        setMoodKeybind(i, key) {
            this.$1t.settings.quickTag.moods[i].keybind = key;
        },
        //Energy
        setEnergyKeybind(i, key) {
            this.$1t.settings.quickTag.energyKeys[i] = key;
        },
        browseQuickTag() {
            this.$1t.send('browse', {context: 'qt'});
        },

        //Save on close
        close() {
            this.$1t.saveSettings();
            this.$emit("close");
        }
    },
    watch: {
        //Sync prop value
        'value'() {
            this.open = this.value;
        }
    }
}
</script>

<style>
.parent-card {
    max-height: 80vh;
    height: 80vh;
    width: 60vw;
    min-width: 500px;
}
.energy-keybind {
    margin-top: -2px;
}
</style>