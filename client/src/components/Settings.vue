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
                    <q-input filled v-model='$1t.settings.value.path'>
                        <!-- <div class='row items-center col-12 text-subtitle2 monospace text-bold text-primary'>{{$1t.settings.value.path}}</div> -->
                        <template v-slot:append>
                            <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browseQuickTag'></q-btn>
                        </template>
                    </q-input>
                </div>
                <div class='q-mt-sm q-pl-sm'>
                    <q-checkbox 
                        v-model='$1t.settings.value.quickTag.recursive'
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
                            <KeybindVue
                                class='energy-keybind' 
                                v-model='$1t.settings.value.quickTag.energyKeys[i-1]'
                            ></KeybindVue>
                        </div>
                    </div>
                </div>
                
                <!-- Energy settings -->                
                <div class='q-mb-sm text-bold text-grey-4'>Energy tag<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp; &nbsp;  &nbsp;  &nbsp;  &nbsp;  Save to</span></div>
                <q-select
                    v-model='$1t.settings.value.quickTag.energyTag.type'
                    dense
                    filled
                    label='Tag type'
                    :options='["rating", "symbol"]'
                    class='q-mb-lg'
                    style='margin-bottom: 40px;'                    
                ></q-select>
                
                <div v-if='$1t.settings.value.quickTag.energyTag.type != "rating"' class='row'>
                    <div class='col-2 q-pr-md'>
                        <q-input v-model='$1t.settings.value.quickTag.energyTag.symbol' filled dense label='Symbol'></q-input>
                    </div>
                    <div class='col-10 q-mb-md'>
                        <TagFields dense v-model='$1t.settings.value.quickTag.energyTag.tag'></TagFields>
                    </div>
                </div>
                
                <!-- Mood tag -->
                <q-separator class='custom-separator' inset color="darker"/>
                <div class='text-subtitle1 text-bold text-grey-4 q-mt-lg q-mb-sm' style='margin-top: 28px;'>Mood tag<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp; &nbsp;  &nbsp;  &nbsp;  &nbsp; Save to</span></div>
                <TagFields class='q-mb-sm' v-model='$1t.settings.value.quickTag.moodTag'></TagFields>

                <!-- Moods -->
                <div class='q-mb-sm text-bold text-grey-4'>Moods<span class='text-grey-5 text-uppercase text-caption'> &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;   &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  &nbsp;  Key binds</span></div>
                <div class='q-mb-md'>
                    <draggable v-model='$1t.settings.value.quickTag.moods' :item-key='(e: any) => `mood-${e.mood}`'>
                        <template #item='{ element: mood, index: i }'>
                            <div class='row justify-around'>
                                <q-input 
                                    clearable 
                                    @clear='$1t.settings.value.quickTag.moods.splice(i, 1)' 
                                    v-model='$1t.settings.value.quickTag.moods[i].mood' 
                                    filled 
                                    dense 
                                    class='col-5 q-pr-md q-mb-sm'
                                ></q-input>
                                <q-select 
                                    v-model='$1t.settings.value.quickTag.moods[i].color' 
                                    dense 
                                    filled 
                                    label='Color'
                                    :options='colors' 
                                    :label-color='$1t.settings.value.quickTag.moods[i].color'
                                    :color='$1t.settings.value.quickTag.moods[i].color'
                                    class='col-5 q-pr-md'
                                ></q-select>
                                <KeybindVue
                                    class='col-2 text-center' 
                                    v-model='$1t.settings.value.quickTag.moods[i].keybind'
                                ></KeybindVue>
                            </div>
                        </template>
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
                    <draggable v-model='$1t.settings.value.quickTag.genres' :item-key='(e: any) => `genre-${e.genre}`'>
                        <template #item='{ element, index: i }'>
                            <div>
                                <div class='row q-my-sm'>
                                    <q-input 
                                        clearable
                                        filled 
                                        dense 
                                        class='col-10' 
                                        v-model='$1t.settings.value.quickTag.genres[i].genre'
                                        @clear='$1t.settings.value.quickTag.genres.splice(i, 1)'
                                    ></q-input>
                                    <KeybindVue
                                        class='col-2 text-center'
                                        v-model='$1t.settings.value.quickTag.genres[i].keybind'
                                    ></KeybindVue>
                                </div>
                                <div class='row'>
                                    <div class='col-2 text-body q-mt-sm'>Subgenres: </div>
                                    <q-input
                                        clearable
                                        filled
                                        dense
                                        class='col-10'
                                        placeholder='Use , as separator'
                                        @update:model-value='(e) => onSubgenreInput(e as string, i)'
                                        :model-value='($1t.settings.value.quickTag.genres[i].subgenres||[]).join(",")'
                                    ></q-input>
                                </div>
                                <div class='q-mt-xl'></div>
                            </div>
                        </template>
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
                        :initial="$1t.settings.value.quickTag.separators"
                        @input='$1t.settings.value.quickTag.separators = $event'                    
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
                    <TagFields class='col-10 q-mb-md' style='margin-bottom: 20px;' dense v-model='$1t.settings.value.quickTag.noteTag.tag'></TagFields>
                    <KeybindVue
                        class='col-2 text-center' 
                        @set='$1t.settings.value.quickTag.noteTag.keybind = $event'
                        :initial='$1t.settings.value.quickTag.noteTag.keybind'
                    ></KeybindVue>
                </div>

                <q-separator class='custom-separator' inset color="darker"/>
                <div class='q-mb-lg'></div>
                <div v-for='(tag, i) in $1t.settings.value.quickTag.custom' :key='"tag"+i' class='q-mb-md'>
                    <div class='row'>
                        <div class='text-subtitle1 text-bold q-mb-sm' style='margin-top: 4px;' v-if='!customQTEdit[i]'>{{tag.name}}</div>
                        <q-input dense filled v-if='customQTEdit[i]' v-model='$1t.settings.value.quickTag.custom[i].name'></q-input>
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
                            
                            <!-- Reoredering -->
                            <q-btn size='sm' class='q-mr-sm' flat round icon='mdi-chevron-up' color='primary' @click='reorderCustomQT(i, -1)' v-if='i > 0'></q-btn>
                            <q-btn size='sm' class='q-mr-sm' flat round icon='mdi-chevron-down' color='primary' @click='reorderCustomQT(i, 1)' v-if='i != $1t.settings.value.quickTag.custom.length - 1'></q-btn>
                            <!-- Delete -->
                            <q-btn size='sm' flat round icon='mdi-delete' color='red' @click='deleteCustomQT(i)'></q-btn>

                        </div>
                    </div>
                    
                    <TagFields class='q-pt-sm' v-model='$1t.settings.value.quickTag.custom[i].tag'></TagFields>
                    <!-- Values -->
                    <draggable v-model='tag.values' :item-key='(e: any) => `qtc-${i}-${e.val}`'>
                        <template #item='{ element, index: j }'>
                            <div class='row'>
                                <q-btn class='col-1 q-mt-sm' round flat icon='mdi-close' @click='$1t.settings.value.quickTag.custom[i].values.splice(j, 1)'></q-btn>
                                <q-input 
                                    class='col-9 q-px-sm q-pt-sm' 
                                    dense 
                                    filled 
                                    v-model='$1t.settings.value.quickTag.custom[i].values[j].val'
                                ></q-input>
                                <KeybindVue
                                    class='col-2 text-center q-pt-sm'
                                    v-model='$1t.settings.value.quickTag.custom[i].values[j].keybind'
                                ></KeybindVue>
                            </div>
                        </template>
                    </draggable>
                    <!-- Add new value -->
                    <q-btn 
                        flat 
                        color='primary' 
                        class='q-mt-sm q-mb-sm' 
                        style='margin-bottom: 11px;'
                        icon='mdi-plus'
                        @click='addNewQTValue(i)'
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
                    v-model='$1t.settings.value.autoTaggerSinglePage'
                    label="Show as single page"
                    class='checkbox'
                ></q-checkbox><br>
                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Quick Tag</div>
                <q-checkbox
                    v-model='$1t.settings.value.quickTag.autosave'
                    label='Autosave changes when switching to a different track'
                    class='checkbox'
                ></q-checkbox>
                <q-checkbox
                    v-model='$1t.settings.value.continuePlayback'
                    label='Continue playback when switching to a different track'
                    class='checkbox'
                ></q-checkbox><br>
                <q-checkbox
                    v-model='$1t.settings.value.playOnSeek'
                    label='Start/continue playback after seeking'
                    class='checkbox'
                ></q-checkbox><br>
                <q-checkbox
                    v-model='$1t.settings.value.quickTag.id3v24'
                    label='Use ID3v2.4 for MP3 and AIFF'
                    class='checkbox'
                ></q-checkbox><br>

                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Edit Tags</div>
                <q-checkbox
                    v-model='$1t.settings.value.tagEditorDouble'
                    label="Show 'Your list'"
                    class='checkbox'
                ></q-checkbox><br>
                <q-checkbox
                    v-model='$1t.settings.value.tagEditorAutosave'
                    label='Autosave changes when switching to a different track'
                    class='checkbox'
                ></q-checkbox><br>
                <q-checkbox
                    v-model='$1t.settings.value.tagEditorPlayer'
                    label='Show player in tag editor'
                    class='checkbox'
                ></q-checkbox><br>

                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Display</div>
                <q-checkbox
                    v-model='$1t.settings.value.helpButton'
                    label='Show help button'
                    class='checkbox'
                ></q-checkbox>     
                
                <div class='q-mt-md text-subtitle2 text-uppercase text-bold text-grey-4'>Advanced</div>
                <q-checkbox
                    v-model='$1t.settings.value.clientSidePlayer'
                    label='Client side player (for server mode)'
                    class='checkbox'
                ></q-checkbox><br>
                <q-checkbox
                    v-model='$1t.settings.value.nonNativeBrowser'
                    label='Client side folder browser'
                    class='checkbox'
                ></q-checkbox>    
                        
                <!-- Color picker -->
                <div class='q-pt-xs q-my-sm text-subtitle2 text-bold text-grey-4'>Primary color</div>
                <q-color 
                    v-model='$1t.settings.value.primaryColor'
                    @change='colorChange'
                ></q-color>
                <q-btn
                    @click='$1t.settings.value.primaryColor = "#00d2bf"; colorChange()'
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

<script lang='ts' setup>
import draggable from 'vuedraggable';
import { Ref, ref, watch } from 'vue';
import { get1t } from '../scripts/onetagger';
import { FrameName, Keybind, Playlist } from '../scripts/utils';
import KeybindVue from './Keybind.vue';
import PlaylistDropZone from './PlaylistDropZone.vue';
import Separators from './Separators.vue';
import TagFields from './TagFields.vue';
import { setCssVar } from 'quasar';

const props = defineProps({
    modelValue: { type: Boolean, required: true }
});
const $1t = get1t();
const colors = ['red', 'pink', 'purple', 'deep-purple', 'indigo', 'blue', 'light-blue',
    'cyan', 'teal', 'green', 'light-green', 'lime', 'yellow', 'amber', 'orange',
    'deep-orange', 'brown', 'grey', 'blue-grey'];
const open = ref(props.modelValue);
const tab = ref('quicktag');
const newMood: Ref<{ mood?: string, color: string, keybind?: Keybind }> = ref({ mood: undefined, color: 'red', keybind: undefined });
const newGenre = ref<string | undefined>();
const newCustomQT = ref('');
const customQTEdit = ref<boolean[]>([]);
const qtPlaylist = ref({});
const emit = defineEmits(['close']);

// Primary color change
function colorChange() {
    setCssVar('primary', $1t.settings.value.primaryColor);
}

// Adds new quicktag mood
function addMood() {
    if (newMood.value.mood) {
        // Exists
        if ($1t.settings.value.quickTag.moods.find(m => newMood.value.mood!.toLowerCase() == m.mood.toLowerCase())) return;
        $1t.settings.value.quickTag.moods.push(JSON.parse(JSON.stringify(newMood.value)));
        newMood.value.mood = undefined;
    }
}

// Add new genre
function addGenre() {
    if (!newGenre.value || newGenre.value.trim() == "") return;
    if ($1t.settings.value.quickTag.genres.find((g) => g.genre.toLowerCase() == newGenre.value!.toLowerCase())) return;
    $1t.settings.value.quickTag.genres.push({genre: newGenre.value, keybind: undefined, subgenres: []});
    newGenre.value = undefined;
}

// On subgenres changed
function onSubgenreInput(e: string | null, i: number) {
    if (!e) {
        $1t.settings.value.quickTag.genres[i].subgenres = [];
        return;
    }
    $1t.settings.value.quickTag.genres[i].subgenres = e.split(",");
}


function browseQuickTag() {
    $1t.browse('qt', $1t.settings.value.path);
}

// Add new custom quicktag
function addCustomQT() {
    $1t.settings.value.quickTag.custom.push({
        name: newCustomQT.value,
        tag: FrameName.same('CUSTOM'),
        values: []
    });
    newCustomQT.value = '';
}

// Delete and edit cusotm qt tag
function deleteCustomQT(i: number) {
    $1t.settings.value.quickTag.custom.splice(i, 1);
}

function editCustomQT(i: number) {
    customQTEdit.value[i] = !customQTEdit.value[i];
}

function addNewQTValue(i: number) {
    $1t.settings.value.quickTag.custom[i].values.push({val: "New", keybind: undefined});
    //TODO: Focus on new value
}

// Move QT tag
function reorderCustomQT(now: number, offset: number) {
    let item = $1t.settings.value.quickTag.custom.splice(now, 1);
    $1t.settings.value.quickTag.custom.splice(now + offset, 0, item[0]);
    
}

// Load quicktag playlist
function loadQTPlaylist(playlist?: Playlist) {
    if (!playlist || !playlist.data) {
        $1t.loadQuickTag();
        return;
    }
    $1t.loadQuickTag(playlist!)
}


// Save on close
function close() {
    $1t.saveSettings();
    emit("close");
}

// Watch if opened or no
watch(props, () => {
    open.value = props.modelValue;
});


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