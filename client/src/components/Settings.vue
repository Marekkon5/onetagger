<template>
<q-dialog v-model='open' @hide='close()'>

    <q-card class='parent-card q-pa-lg'>
        <!-- Header -->
        <q-card-section>
            <div class='text-subtitle1 text-bold q-mb-sm text-center text-uppercase text-primary'>Settings</div>
            <q-tabs v-model='tab'>
                <q-tab label='Quick Tag' name='quicktag'></q-tab>
                <q-tab label='Quick Tag Custom' name='quicktag-custom'></q-tab>
                <q-tab label='General' name='advanced'></q-tab>
            </q-tabs>
        </q-card-section>
        <!-- Content -->
        <q-card-section>
            <!-- Quicktag options -->
            <div v-if='tab == "quicktag"'>
                <!-- Path options -->
                <div class='select'>
                <div class='col-1'></div>
                    <q-input filled v-model='$1t.settings.path'>
                        <!-- <div class='row items-center col-12 text-subtitle2 monospace text-bold text-primary'>{{$1t.settings.path}}</div> -->
                        <template v-slot:append>
                            <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browseQuickTag'></q-btn>
                        </template>
                    </q-input>
                </div>
                <div class='q-mt-sm q-pl-sm'>
                    <q-checkbox 
                        v-model='$1t.settings.quickTag.recursive'
                        label='Include subfolders'
                        class='checkbox'                        
                        @input="$1t.loadQuickTag()"
                    ></q-checkbox>
                </div>
                <PlaylistDropZone 
                    v-model='qtPlaylist' 
                    @input='loadQTPlaylist' 
                    class='input'   
                    style='margin-bottom: 40px;'                 
                ></PlaylistDropZone>
                
                <!-- Energy keybinds --> 
                <q-separator class='custom-separator' inset color="darker"/>
                <div class='text-subtitle1 text-bold text-grey-4 q-mt-lg q-mb-sm' style='margin-top: 28px;'>Energy<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;Key binds</span></div>
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
                <div class='q-mb-sm text-bold text-grey-4'>Energy tag<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp; &nbsp;  &nbsp;  &nbsp;  &nbsp;  Save to</span></div>
                <q-select
                    v-model='$1t.settings.quickTag.energyTag.type'
                    dense
                    filled
                    label='Tag type'
                    :options='["rating", "symbol"]'
                    class='q-mb-lg'
                    style='margin-bottom: 40px;'                    
                ></q-select>
                
                <div v-if='$1t.settings.quickTag.energyTag.type != "rating"' class='row'>
                    <div class='col-2 q-pr-md'>
                        <q-input v-model='$1t.settings.quickTag.energyTag.symbol' filled dense label='Symbol'></q-input>
                    </div>
                    <div class='col-10 q-mb-md'>
                        <TagFields dense v-model='$1t.settings.quickTag.energyTag.tag'></TagFields>
                    </div>
                </div>
                
                <!-- Mood tag -->
                <q-separator class='custom-separator' inset color="darker"/>
                <div class='text-subtitle1 text-bold text-grey-4 q-mt-lg q-mb-sm' style='margin-top: 28px;'>Mood tag<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp; &nbsp;  &nbsp;  &nbsp;  &nbsp; Save to</span></div>
                <TagFields class='q-mb-sm' v-model='$1t.settings.quickTag.moodTag'></TagFields>

                <!-- Moods -->
                <div class='q-mb-sm text-bold text-grey-4'>Moods<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  Key binds</span></div>
                <div class='q-mb-md'>
                    <draggable v-model='$1t.settings.quickTag.moods'>
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
                    </draggable>
                    <!-- Add new mood -->
                    <div class='q-mt-sm q-mb-sm text-uppercase text-primary text-subtitle2'>Add new mood</div>
                    <div class='row'>
                        <q-input v-model='newMood.mood' filled dense class='col-5 q-pr-md q-mb-lg'></q-input>
                        <q-select v-model='newMood.color' :options='colors' filled dense class='col-5 q-pr-md'></q-select>
                        <div class='col-1'>
                        <q-btn flat round icon='mdi-plus' @click='addMood' color='primary'></q-btn>  
                        </div>                      
                    </div>
                </div>
                
                <!-- Genres -->
                <q-separator class='custom-separator' inset color="darker"/>                
                <div class='q-mb-sm q-mt- text-bold text-grey-4' style='margin-top:31px;'>Genres<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;Key binds</span></div>
                <div>
                    <draggable v-model='$1t.settings.quickTag.genres'>
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
                            <div class='row'>
                                <div class='col-2 text-body q-mt-sm'>Subgenres: </div>
                                <q-input
                                    clearable
                                    filled
                                    dense
                                    class='col-10'
                                    placeholder='Use , as separator'
                                    @input='onSubgenreInput($event, i)'
                                    :value='($1t.settings.quickTag.genres[i].subgenres||[]).join(",")'
                                ></q-input>
                            </div>
                            <div class='q-mt-xl'></div>
                        </div>
                    </draggable>
                    <!-- Add new genre -->
                    <div class='q-mt-md q-mb-sm text-uppercase text-primary text-subtitle2'>Add new genre</div>
                    <div class='row'>
                        <q-input filled dense class='col-10 q-pr-md' v-model='newGenre'></q-input>
                        <div class='col-1'>
                            <q-btn flat round icon='mdi-plus' @click='addGenre' color='primary'></q-btn>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Quicktag custom -->
            <div v-if='tab == "quicktag-custom"'>

                <!-- Separators -->
                <div class='text-center text-body1 text-grey-4 q-mb-sm'>Separators</div>
                <div class='text-center' style='margin-bottom: 40px;'>
                    <Separators
                        :initial="$1t.settings.quickTag.separators"
                        @input='$1t.settings.quickTag.separators = $event'                    
                    ></Separators>
                </div>
                <q-separator class='custom-separator q-my-lg' inset color="darker"/>

                <!-- Note tag -->
                <div class='text-primary text-bold q-mb-md row' style='margin-top: 35px'>
                    CUSTOM NOTE
                    <span class='text-grey-5 text-uppercase text-caption' style='padding-left: 300px;'>
                        <span>Save to</span>
                        <span style='padding-left: 26px;'>Key binds</span>
                    </span>
                </div>
                <div class='row'>
                    <TagFields class='col-10 q-mb-md' style='margin-bottom: 20px;' dense v-model='$1t.settings.quickTag.noteTag.tag'></TagFields>
                    <Keybind 
                        class='col-2 text-center' 
                        @set='$1t.settings.quickTag.noteTag.keybind = $event'
                        :initial='$1t.settings.quickTag.noteTag.keybind'
                    ></Keybind>
                </div>

                <q-separator class='custom-separator' inset color="darker"/>
                <div class='q-mb-lg'></div>
                <div v-for='(tag, i) in $1t.settings.quickTag.custom' :key='"tag"+i' class='q-mb-md'>
                    <div class='row'>
                        <div class='text-subtitle1 text-bold q-mb-sm' style='margin-top: 4px;' v-if='!customQTEdit[i]'>{{tag.name}}</div>
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
                    
                    <TagFields class='q-pt-sm' v-model='$1t.settings.quickTag.custom[i].tag'></TagFields>
                    <!-- Values -->
                    <draggable v-model='tag.values'>
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
                    </draggable>
                    <!-- Add new value -->
                    <q-btn 
                        flat 
                        color='primary' 
                        class='q-mt-sm q-mb-sm' 
                        style='margin-bottom: 11px;'
                        icon='mdi-plus'
                        @click='$1t.settings.quickTag.custom[i].values.push({val: "New", keybind: null})'
                    >Add new value</q-btn>
                </div>
                <!-- Add new tag -->
                <q-separator class='custom-separator' inset color="darker"/>
                <div class='q-mb-lg'></div>
                <div class='row q-mt-md'>
                    <div class='text-subtitle1 text-bold q-my-lg col-4'>Add new section</div>
                    <q-input v-model='newCustomQT' filled label='Name' class='q-mt-sm col-7 q-pr-md'></q-input>
                    <div class='q-mt-md col-1'>
                        <q-btn round flat icon='mdi-plus' size='md' color='primary' @click='addCustomQT'></q-btn>
                    </div>
                </div>
            </div>

            <!-- Advanced -->
            <div v-if='tab == "advanced"'>
                
                <!-- Open settings folder -->
                <q-btn
                    color='primary'
                    class='text-black q-mb-md'
                    @click='$1t.send("openSettingsFolder")'
                >Open data folder</q-btn>
                
                
                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Auto Tag</div>
                <q-checkbox
                    v-model='$1t.settings.autoTaggerSinglePage'
                    label="Show as single page"
                    class='checkbox'
                ></q-checkbox><br>
                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Quick Tag</div>
                <q-checkbox
                    v-model='$1t.settings.quickTag.autosave'
                    label='Autosave changes when switching to a different track'
                    class='checkbox'
                ></q-checkbox>
                <q-checkbox
                    v-model='$1t.settings.continuePlayback'
                    label='Continue playback when switching to a different track'
                    class='checkbox'
                ></q-checkbox><br>
                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Edit Tags</div>
                <q-checkbox
                    v-model='$1t.settings.tagEditorDouble'
                    label="Show 'Your list'"
                    class='checkbox'
                ></q-checkbox><br>
                <q-checkbox
                    v-model='$1t.settings.tagEditorAutosave'
                    label='Autosave changes when switching to a different track'
                    class='checkbox'
                ></q-checkbox><br>
                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Display</div>
                <q-checkbox
                    v-model='$1t.settings.helpButton'
                    label='Show help button'
                    class='checkbox'
                ></q-checkbox>                
                <!-- Color picker -->
                <div class='q-pt-xs q-my-sm text-subtitle2 text-bold text-grey-4'>Primary color</div>
                <q-color 
                    v-model='$1t.settings.primaryColor'
                    @change='colorChange'
                ></q-color>
                <q-btn
                    @click='$1t.settings.primaryColor = "#00d2bf"; colorChange()'
                    color='primary'
                    flat
                    class='q-mt-sm'
                >
                    Reset color to default
                </q-btn>
            </div>

        </q-card-section>
    </q-card>

</q-dialog>
</template>

<script>
import Keybind from './Keybind';
import TagFields from './TagFields';
import Separators from './Separators.vue';
import PlaylistDropZone from './PlaylistDropZone.vue';
import Vue from 'vue';
import draggable from 'vuedraggable';
import { colors } from 'quasar';

export default {
    name: 'Settings',
    components: {Keybind, TagFields, PlaylistDropZone, Separators, draggable},
    data() {
        return {
            open: this.value,
            tab: 'quicktag',
            colors: ['red', 'pink', 'purple', 'deep-purple', 'indigo', 'blue', 'light-blue',
                'cyan', 'teal', 'green', 'light-green', 'lime', 'yellow', 'amber', 'orange',
                'deep-orange', 'brown', 'grey', 'blue-grey'],
            newMood: {mood: null, color: 'red', keybind: null},
            newGenre: null,
            newCustomQT: null,
            customQTEdit: [],
            qtPlaylist: {}
        }
    },
    props: {
        value: {
            type: Boolean
        }
    },
    methods: {
        // Primary color change
        colorChange() {
            colors.setBrand('primary', this.$1t.settings.primaryColor);
        },
        // Adds new quicktag mood
        addMood() {
            if (this.newMood.mood) {
                //Exists
                if (this.$1t.settings.quickTag.moods.find(m => this.newMood.mood.toLowerCase() == m.mood.toLowerCase())) return;
                this.$1t.settings.quickTag.moods.push(JSON.parse(JSON.stringify(this.newMood)));
                this.newMood.mood = null;
            }
        },
        // Add new genre
        addGenre() {
            if (!this.newGenre || this.newGenre.trim() == "") return;
            if (this.$1t.settings.quickTag.genres.find((g) => g.genre.toLowerCase() == this.newGenre.toLowerCase())) return;
            this.$1t.settings.quickTag.genres.push({genre: this.newGenre, keybind: null});
            this.newGenre = null;
        },
        // On subgenres changed
        onSubgenreInput(e, i) {
            if (!e) {
                this.$1t.settings.quickTag.genres[i].subgenres = [];
                return;
            }
            this.$1t.settings.quickTag.genres[i].subgenres = e.split(",");
        },
        // Mood keybind
        setMoodKeybind(i, key) {
            this.$1t.settings.quickTag.moods[i].keybind = key;
        },
        // Energy
        setEnergyKeybind(i, key) {
            this.$1t.settings.quickTag.energyKeys[i] = key;
        },
        browseQuickTag() {
            this.$1t.send('browse', {context: 'qt', path: this.$1t.settings.path});
        },
        // Add new custom quicktag
        addCustomQT() {
            this.$1t.settings.quickTag.custom.push({
                name: this.newCustomQT,
                tag: {
                    id3: "CUSTOM",
                    vorbis: "CUSTOM",
                    mp4: "CUSTOM",
                },
                values: []
            });
            this.newCustomQT = null;
        },
        // Delete and edit cusotm qt tag
        deleteCustomQT(i) {
            this.$1t.settings.quickTag.custom.splice(i, 1);
        },
        editCustomQT(i) {
            Vue.set(this.customQTEdit, i, !this.customQTEdit[i]);
        },

        // Load quicktag playlist
        loadQTPlaylist(playlist) {
            if (!playlist || !playlist.data) {
                this.$1t.loadQuickTag();
                return;
            }
            this.$1t.loadQuickTag(playlist)
        },

        // Save on close
        close() {
            this.$1t.saveSettings();
            this.$emit("close");
        }
    },
    watch: {
        // Sync prop value
        'value'() {
            this.open = this.value;
        }
    }
}
</script>

<style>
.parent-card {
    max-height: 80vh;
    height: 78vh;
    width: 70vw;
    min-width: 600px;
}
.energy-keybind {
    margin-top: -2px;
}
.mt-2 {
    margin-top: 2px;
}
.custom-separator {
    max-width: 550px; 
    margin: auto;
}
</style>