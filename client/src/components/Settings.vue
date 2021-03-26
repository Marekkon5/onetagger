<template>
<q-dialog v-model='open' @hide='close()'>

    <q-card class='parent-card'>
        <!-- Header -->
        <q-card-section>
            <div class='text-h6 q-mb-sm'>Settings</div>
            <q-tabs v-model='tab'>
                <q-tab label='QuickTag' name='quicktag'></q-tab>
                <q-tab label='QuickTag Custom' name='quicktag-custom'></q-tab>
            </q-tabs>
        </q-card-section>
        <!-- Content -->
        <q-card-section>
            <!-- Quicktag options -->
            <div v-if='tab == "quicktag"'>
                <!-- Path options -->
                <div class='text-h6'>Folder:</div>
                <div class='row q-mb-sm items-center'>
                    <div class='col-11 text-subtitle2'>Current: {{$1t.settings.quickTag.path}}</div>
                    <div class='col-1'>
                        <q-btn round flat icon='mdi-open-in-app' @click='browseQuickTag'></q-btn>
                    </div>
                </div>

                <!-- Energy keybinds -->
                <div class='text-h6 q-mb-sm'>Energy keybinds:</div>
                <div class='row q-mb-md'>
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
                <div class='text-h6 q-mb-sm'>Energy tag options:</div>
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
                <div class='q-mb-md'></div>
                <!-- Mood tag -->
                <div class='text-h6 q-my-sm'>Mood tag:</div>
                <div class='row q-mb-md'>
                    <div class='col-6 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.moodTag.id3' outlined dense label='ID3 (MP3, AIFF)'></q-input>
                    </div>
                    <div class='col-6 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.moodTag.vorbis' outlined dense label='FLAC'></q-input>
                    </div>
                </div>
                <!-- Moods -->
                <div class='text-h6 q-my-sm'>Moods:</div>
                <div class='q-mb-md'>
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
                <!-- Genres -->
                <div class='text-h6 q-my-sm'>Genres:</div>
                <div>
                    <div v-for='(genre, i) in $1t.settings.quickTag.genres' :key='"genre"+i'>
                        <div class='row q-my-sm'>
                            <q-input outlined dense class='col-10' v-model='$1t.settings.quickTag.genres[i].genre'></q-input>
                            <Keybind
                                class='col-2 text-center'
                                @set='$1t.settings.quickTag.genres[i].keybind = $event'
                                :initial='$1t.settings.quickTag.genres[i].keybind'
                            ></Keybind>
                        </div>
                    </div>
                </div>

            </div>

            <!-- Quicktag custom -->
            <div v-if='tab == "quicktag-custom"'>
                <div v-for='(tag, i) in $1t.settings.quickTag.custom' :key='"tag"+i' class='q-mb-md'>
                    <div class='text-h6 q-mb-sm'>{{tag.name}}</div>
                    <div class='row q-pt-sm'>
                        <q-input class='col-6 q-pr-sm' outlined label='ID3 (MP3+AIFF)' v-model='$1t.settings.quickTag.custom[i].id3'></q-input>
                        <q-input class='col-6 q-pr-sm' outlined label='FLAC' v-model='$1t.settings.quickTag.custom[i].vorbis'></q-input>
                    </div>
                    <!-- Values -->
                    <div v-for='(value, j) in tag.values' :key='value.value+j.toString()'>
                        <div class='row'>
                            <q-btn class='col-1 q-mt-sm' round flat icon='mdi-close' @click='$1t.settings.quickTag.custom[i].values.splice(j, 1)'></q-btn>
                            <q-input class='col-9 q-px-sm q-pt-sm' dense outlined v-model='$1t.settings.quickTag.custom[i].values[j].val'></q-input>
                            <Keybind
                                class='col-2 text-center q-pt-sm'
                                @set='$1t.settings.quickTag.custom[i].values[j].keybind = $event'
                                :initial='$1t.settings.quickTag.custom[i].values[j].keybind'
                            ></Keybind>
                        </div>
                    </div>
                    <!-- Add new value -->
                    <q-btn 
                        flat 
                        color='primary' 
                        class='q-mt-sm' 
                        icon='mdi-plus'
                        @click='$1t.settings.quickTag.custom[i].values.push({val: "New"})'
                    >Add new value</q-btn>
                </div>
                <!-- Add new tag -->
                <div class='row q-mt-md'>
                    <div class='text-h6 q-my-md col-3'>Add new tag</div>
                    <q-input v-model='newCustomQT' outlined label='Name' class='q-mt-sm col-8 q-pr-md'></q-input>
                    <div class='q-mt-md col-1'>
                        <q-btn round flat icon='mdi-plus' size='md' color='primary' @click='addCustomQT'></q-btn>
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
            newMood: {mood: null, color: 'red'},
            newCustomQT: null
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
        //Add new custom quicktag
        addCustomQT() {
            this.$1t.settings.quickTag.custom.push({
                name: this.newCustomQT,
                id3: "CUSTOM",
                vorbis: "CUSTOM",
                values: []
            });
            this.newCustomQT = null;
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