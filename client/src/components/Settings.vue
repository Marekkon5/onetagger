<template>
<q-dialog v-model='open' @hide='close()'>

    <q-card class='parent-card'>
        <!-- Header -->
        <q-card-section>
            <div class='text-h6 q-mb-sm'>Settings</div>
            <q-tabs v-model='tab'>
                <q-tab label='Quick Tag' name='quicktag'></q-tab>
                <q-tab label='Quick Tag Custom' name='quicktag-custom'></q-tab>
                <q-tab label='Advanced' name='advanced'></q-tab>
            </q-tabs>
        </q-card-section>
        <!-- Content -->
        <q-card-section>
            <!-- Quicktag options -->
            <div v-if='tab == "quicktag"'>
                <!-- Path options -->
                <div class='text-h10 text-bold text-grey-6'>FOLDER</div>
                <div class='row q-mb-sm items-center'>
                    <div class='col-11 text-subtitle2'>{{$1t.settings.quickTag.path}}</div>
                    <div class='col-1'>
                        <q-btn round flat icon='mdi-open-in-app' @click='browseQuickTag'></q-btn>
                    </div>
                </div>

                <!-- Energy keybinds -->
                <div class='text-h10 q-mb-sm text-bold text-grey-6'>ENERGY KEYBINDS</div>
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
                <div class='text-h10 q-mb-sm text-bold text-grey-6'>ENERGY TAG OPTIONS</div>
                <q-select
                    v-model='$1t.settings.quickTag.energyTag.type'
                    dense
                    filled
                    label='Tag type'
                    :options='["rating", "symbol"]'
                    class='q-mb-sm q-pr-md'
                ></q-select>
                <div v-if='$1t.settings.quickTag.energyTag.type != "rating"' class='row'>
                    <div class='col-2 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.energyTag.symbol' filled dense label='Symbol'></q-input>
                    </div>
                    <div class='col-5 q-pr-md'>
                        <TagField format='id3' dense :initial='$1t.settings.quickTag.energyTag.id3' @change='$1t.settings.quickTag.energyTag.id3 = $event'></TagField>
                    </div>
                    <div class='col-5 q-pr-md'>
                        <TagField format='flac' dense :initial='$1t.settings.quickTag.energyTag.vorbis' @change='$1t.settings.quickTag.energyTag.vorbis = $event'></TagField>
                    </div>
                </div>
                <div class='q-mb-md'></div>
                <!-- Mood tag -->
                <div class='text-h10 q-my-sm text-bold text-grey-6'>MOOD TAG</div>
                <div class='row q-mb-md'>
                    <div class='col-6 q-pr-md'>
                        <TagField format='id3' :initial='$1t.settings.quickTag.moodTag.id3' @change='$1t.settings.quickTag.moodTag.id3 = $event'></TagField>
                    </div>
                    <div class='col-6 q-pr-md'>
                        <TagField format='flac' :initial='$1t.settings.quickTag.moodTag.vorbis' @change='$1t.settings.quickTag.moodTag.vorbis = $event'></TagField>
                    </div>
                </div>
                <!-- Moods -->
                <div class='text-h10 q-my-sm text-bold text-grey-6'>MOODS</div>
                <div class='q-mb-md'>
                    <div v-for='(mood, i) in $1t.settings.quickTag.moods' :key='"mood"+i'>
                        <div class='row justify-around'>
                            <q-input 
                                clearable 
                                @clear='$1t.settings.quickTag.moods.splice(i, 1)' 
                                v-model='$1t.settings.quickTag.moods[i].mood' 
                                filled 
                                dense 
                                class='col-5 q-pr-md q-mb-sm'
                            ></q-input>
                            <q-select 
                                v-model='$1t.settings.quickTag.moods[i].color' 
                                dense 
                                filled 
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
                        <q-input v-model='newMood.mood' filled dense class='col-6 q-pr-md'></q-input>
                        <q-select v-model='newMood.color' :options='colors' filled dense class='col-5 q-pr-md'></q-select>
                        <q-btn @click='addMood' round flat dense icon='mdi-plus' class='col-1 text-primary'></q-btn>
                    </div>
                </div>
                <!-- Genres -->
                <div class='text-h10 q-my-sm text-bold text-grey-6'>GENRES</div>
                <div>
                    <div v-for='(genre, i) in $1t.settings.quickTag.genres' :key='"genre"+i'>
                        <div class='row q-my-sm'>
                            <q-input 
                                clearable
                                filled 
                                dense 
                                class='col-10' 
                                v-model='$1t.settings.quickTag.genres[i].genre'
                                @clear='$1t.settings.quickTag.genres.splice(i, 1)'
                            ></q-input>
                            <Keybind
                                class='col-2 text-center'
                                @set='$1t.settings.quickTag.genres[i].keybind = $event'
                                :initial='$1t.settings.quickTag.genres[i].keybind'
                            ></Keybind>
                        </div>
                    </div>
                    <!-- Add new genre -->
                    <div class='text-subtitle2 q-mb-sm'>Create new genre:</div>
                    <div class='row'>
                        <q-input filled dense class='col-11 q-pr-md' v-model='newGenre'></q-input>
                        <div class='col-1'>
                            <q-btn flat round icon='mdi-plus' @click='addGenre' color='primary'></q-btn>
                        </div>
                    </div>
                    
                </div>

            </div>

            <!-- Quicktag custom -->
            <div v-if='tab == "quicktag-custom"'>
                <div v-for='(tag, i) in $1t.settings.quickTag.custom' :key='"tag"+i' class='q-mb-md'>
                    <div class='row'>
                        <div class='text-h6 q-mb-sm' v-if='!customQTEdit[i]'>{{tag.name}}</div>
                        <q-input dense filled v-if='customQTEdit[i]' v-model='$1t.settings.quickTag.custom[i].name'></q-input>
                        <div class='q-mx-md mt-2'>
                            <q-btn
                                size='sm' 
                                flat 
                                round 
                                :icon='customQTEdit[i] ? "mdi-check" : "mdi-pencil"' 
                                class='q-mr-sm' 
                                color='primary' 
                                @click='editCustomQT(i)'
                            ></q-btn>
                            
                            <q-btn size='sm' flat round icon='mdi-delete' color='red' @click='deleteCustomQT(i)'></q-btn>
                        </div>
                    </div>
                    <div class='row q-pt-sm'>
                        <TagField class='col-6 q-pr-sm' format='id3' :initial='$1t.settings.quickTag.custom[i].id3' @change='$1t.settings.quickTag.custom[i].id3 = $event'></TagField>
                        <TagField class='col-6 q-pr-sm' format='flac' :initial='$1t.settings.quickTag.custom[i].vorbis' @change='$1t.settings.quickTag.custom[i].vorbis = $event'></TagField>
                    </div>
                    <!-- Values -->
                    <div v-for='(value, j) in tag.values' :key='value.value+j.toString()'>
                        <div class='row'>
                            <q-btn class='col-1 q-mt-sm' round flat icon='mdi-close' @click='$1t.settings.quickTag.custom[i].values.splice(j, 1)'></q-btn>
                            <q-input class='col-9 q-px-sm q-pt-sm' dense filled v-model='$1t.settings.quickTag.custom[i].values[j].val'></q-input>
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
                    <div class='text-h6 q-my-md col-4'>Add new section</div>
                    <q-input v-model='newCustomQT' filled label='Name' class='q-mt-sm col-7 q-pr-md'></q-input>
                    <div class='q-mt-md col-1'>
                        <q-btn round flat icon='mdi-plus' size='md' color='primary' @click='addCustomQT'></q-btn>
                    </div>
                </div>
            </div>

            <!-- Advanced -->
            <div v-if='tab == "advanced"'>
                <q-checkbox
                    v-model='$1t.settings.helpButton'
                    label='Show help button'
                ></q-checkbox>
            </div>

        </q-card-section>
    </q-card>

</q-dialog>
</template>

<script>
import Keybind from './Keybind';
import TagField from './TagField';
import Vue from 'vue';

export default {
    name: 'Settings',
    components: {Keybind, TagField},
    data() {
        return {
            open: this.value,
            tab: 'quicktag',
            colors: ['red', 'pink', 'purple', 'deep-purple', 'indigo', 'blue', 'light-blue',
                'cyan', 'teal', 'green', 'light-green', 'lime', 'yellow', 'amber', 'orange',
                'deep-orange', 'brown', 'grey', 'blue-grey'],
            newMood: {mood: null, color: 'red'},
            newGenre: null,
            newCustomQT: null,
            customQTEdit: [],
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
                this.$1t.settings.quickTag.moods.push(JSON.parse(JSON.stringify(this.newMood)));
                this.newMood.mood = null;
            }
        },
        //Add new genre
        addGenre() {
            if (!this.newGenre || this.newGenre.trim() == "") return;
            if (this.$1t.settings.quickTag.genres.find((g) => g.genre.toLowerCase() == this.newGenre.toLowerCase())) return;
            this.$1t.settings.quickTag.genres.push({genre: this.newGenre, keybind: null});
            this.newGenre = null;
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
        //Delete and edit cusotm qt tag
        deleteCustomQT(i) {
            this.$1t.settings.quickTag.custom.splice(i, 1);
        },
        editCustomQT(i) {
            Vue.set(this.customQTEdit, i, !this.customQTEdit[i]);
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
.mt-2 {
    margin-top: 2px;
}
</style>