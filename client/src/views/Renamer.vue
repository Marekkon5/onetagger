<template>
<div class='text-center'>

    <div class='q-py-lg' v-if='!$1t.lock.value.locked'>
        <div style='max-width: 800px; margin: auto;'>
            <!-- Input and output folders -->
            <div class='text-subtitle1 text-bold text-primary'>SELECT INPUT / OUTPUT</div>
            <div class='text-subtitle2 q-mb-md text-grey-6'>Drag & drop folder, copy/paste path directly or click the <q-icon name='mdi-open-in-app'></q-icon> icon to browse</div>
        
            <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
                <q-input filled class='col-10' label='Input folder' v-model='config.path' @update:model-value="updatePreview()">
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse(false)'></q-btn>
                    </template>
                </q-input>
            </div>
    
            <div class='q-pt-lg row justify-center input' style='max-width: 725px; margin: auto;'>
                <q-input filled class='col-10' label='Output folder (leave empty for same as input)' v-model='config.outDir' @update:model-value="updatePreview()">
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse(true)'></q-btn>
                    </template>
                </q-input>                
            </div>
            <q-separator class='q-mx-auto q-mb-lg custom-separator' style='margin-top: 41px;' inset color="dark" />
    
            <!-- Template -->            
            <div class='text-subtitle1 text-bold text-primary custom-margin'>TEMPLATE</div>
                <div class='text-subtitle2 text-grey-6'>Enter dynamic content and/or static content. More info? Click <q-icon style='padding-bottom: 3px;' name='mdi-help-circle-outline'></q-icon> HELP on the right</div>
            
            <div style='margin-top: -30px;'>
                <div class='fake-cursor' :style='cursorStyle'>|</div>
                <div class='template-text'>
                    <span v-if='config.template' v-html='highlighted'></span>
                    <span v-if='!config.template' class='template-input-placeholder'>Filename template</span>
                </div>
                <input
                    class='template-input monospace' 
                    spellcheck="false"
                    ref='templateInputElem'
                    @blur='onBlur'
                    @focus='onSelectionChange'
                    @selectionchange='onSelectionChange'
                    @keyup='onSelectionChange'
                    @keydown='onKeyDown'
                    @input='(e) => templateInput(e as InputEvent)'
                    @click='onSelectionChange'
                    @paste='onPaste'
                >
            </div>
    
            <!-- Autocomplete / suggestions -->
            <div v-if='suggestions.length > 0'>
                <div class='suggestions-box' :style='suggestionsStyle'>
                    <!-- Suggestions -->
                    <div style='width: 40%'>
                        <div v-for='(suggestion, i) in suggestions' :key="'s'+i" class='q-mr-sm q-pa-xs' :class='{"help-suggestion-selected": i == suggestionIndex}'>
                            <!-- icon -->
                            <q-icon name='mdi-variable' class='q-mb-xs' v-if='suggestion.kind == "variable"'></q-icon>
                            <q-icon name='mdi-information-outline' class='q-mb-xs' v-if='suggestion.kind == "property"'></q-icon>
                            <q-icon name='mdi-function' class='q-mb-xs' v-if='suggestion.kind == "function"'></q-icon>
    
                            <!-- name -->
                            <span class='q-ml-sm' :class='{"text-primary": i == suggestionIndex}'>
                                <RenamerTokenName :token='suggestion' :params='false'></RenamerTokenName>
                            </span>
    
                            <!-- selected icon -->
                            <span v-if='i == suggestionIndex' style='float: right;'>
                                <q-icon name='mdi-chevron-right' class='q-mb-xs' color='primary'></q-icon>
                            </span>
                        </div>
                    </div>
                    <!-- Help -->
                    <div style='width: 60%'>
                        <div v-if='suggestions[suggestionIndex]'>
                            <!-- Function info -->
                            <div v-if='suggestions[suggestionIndex].kind == "function"' class='q-mb-sm suggestion-help-function'>
                                <RenamerTokenName :token='suggestions[suggestionIndex]'></RenamerTokenName>
                                <br>
                            </div>
    
                            <!-- Actual suggestion -->
                            <div v-html='suggestions[suggestionIndex].doc'></div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        

        <!-- Preview -->              
        <div class='full-width'>            
            <div class='q-mt-md q-mb-sm text-subtitle1 text-bold text-primary custom-margin'>PREVIEW</div>
            <div v-for='(file, i) in preview' :key='"prev"+i'>
                <div class='text-caption monospace text-grey-5'>{{file[1]}}</div>
                <br>
            </div>
        </div>
        
        
        <!-- Options -->
        <div class='full-width'>    
            <p></p><q-separator class='q-mb-lg custom-separator' inset color="dark" />
            <div class='q-mb-sm text-subtitle1 text-bold text-primary custom-margin'>OPTIONS</div>

            <q-toggle v-model='config.copy' label='Copy files instead of moving'></q-toggle>
            <br>
            <q-toggle v-model='config.subfolders' label='Include subfolders'></q-toggle>
            <br>
            <q-toggle v-model='config.overwrite' label='Overwrite existing target files'></q-toggle>
            <br>
            <q-toggle v-model='config.keepSubfolders' label='Keep original subfolders'></q-toggle>
            <br>

            <div class='row justify-center q-my-sm'>
                <q-input
                    v-model='config.separator'
                    label='Separator'
                    filled
                    style='max-width: 200px;'
                ></q-input>
            </div>
        </div>



    </div>

    <!-- Start FAB -->
    <q-page-sticky position='bottom-right' :offset='[36, 34]'>
        <div class='row'>
            <div>
                <q-btn 
                    fab 
                    push
                    icon='mdi-play' 
                    color='primary'
                    :disabled='!startable'
                    @click='start(false)'>
        
                    <q-tooltip anchor="top middle" self="bottom middle" :offset="[10, 10]">            
                        <span class='text-weight-bold'>START</span>
                    </q-tooltip>
                </q-btn>
            </div>
        </div>
    </q-page-sticky>

    <!-- Loading -->
    <div v-if='$1t.lock.value.locked'>
        <div style='margin-top: 45vh;'>
            <q-circular-progress indeterminate size='64px' color='primary'></q-circular-progress>
        </div>
    </div>

    <!-- For cursor calculations -->
    <div>
        <span style='visibility: hidden; font-size: 16px;' class='monospace' ref='textWidthRef'>abcdefghijklmnopqrstuvwxyz0123456789</span>
    </div>

</div>
</template>

<script lang='ts' setup>
import RenamerTokenName from '../components/RenamerTokenName.vue';
import { computed, onMounted, onUnmounted, ref, watch, watchEffect } from 'vue';
import { get1t } from '../scripts/onetagger';
import { useQuasar } from 'quasar';

class RenamerConfig {
    path?: string;
    outDir?: string;
    template = '';
    copy = false;
    subfolders = true;
    overwrite = false;
    separator = ', ';
    keepSubfolders = false;
}

const $1t = get1t();
const $q = useQuasar();
const config = ref(new RenamerConfig());
const highlighted = ref(undefined);
const cursor = ref(-99999);
const charWidth = ref(1.0);
const suggestions = ref<any[]>([]);
const suggestionIndex = ref(0);
const suggestionOffset = ref(0);
const suggestionsTop = ref(0);
const preview = ref([]);

const templateInputElem = ref<HTMLInputElement | undefined>();
const textWidthRef = ref<HTMLElement | undefined>();

// Browse folder
function browse(output = false) {
    $1t.browse(output ? 'rnOutput' : 'rn', config.value.path);
};

// Handle typing into the template box
function templateInput(e: InputEvent) {
    if (!config.value.template) {
        if (!e.data) return;
        config.value.template = e.data;
    }
    
    // Autoclose
    let pos = cursor.value;
    if (e.data == '(') {
        injectTemplate(cursor.value + 1, ')');
        moveCursor(pos + 1);
    }
    if (e.data == '"') {
        injectTemplate(cursor.value, '"');
        moveCursor(pos + 1);
    }
    if (e.data == '%') {
        injectTemplate(cursor.value, '%');
        moveCursor(pos + 1);
    }

    // @ts-ignore
    config.value.template = e.target.value;
    updateTemplate();
};

// Fetch syntax highlighting and ac
function updateTemplate() {
    $1t.send('renamerSyntaxHighlight', { template: config.value.template });
    $1t.send('renamerAutocomplete', { 
        template: config.value.template.substring(0, cursor.value + 1) 
    });
    // Update cursor
    onSelectionChange();
};

// Handle paste event
function onPaste(e: ClipboardEvent) {
    setTimeout(() => {
        const value = (e as any).target.value;
        if (value) {
            config.value.template = value;
            updateTemplate();
        }
    }, 25);
}

// Handle global selection change to update fake cursor (yes, pain)
function onSelectionChange() {
    cursor.value = templateInputElem.value!.selectionStart!;
};

/// Template blur
function onBlur() {
    cursor.value = -6969;
    suggestions.value = [];
};

/// Template key down
function onKeyDown(e: KeyboardEvent) {
    // Control suggestions
    if (e.key == "ArrowDown") {
        if (suggestionIndex.value < suggestions.value.length - 1) suggestionIndex.value += 1;
        e.preventDefault();
        return;
    }
    if (e.key == "ArrowUp") {
        if (suggestionIndex.value > 0) suggestionIndex.value -= 1;
        e.preventDefault();
        return;
    }
    // Enter override
    if (e.key == "Enter") {
        if (suggestions.value[suggestionIndex.value]) {
            // Fill suggestion
            let text = suggestions.value[suggestionIndex.value].name.substring(suggestionOffset.value);
            let pos = cursor.value;
            injectTemplate(cursor.value, text);
            updateTemplate();
            moveCursor(pos + text.length);
        }
        e.preventDefault();
        return;
    }
    // Don't close again
    if (templateInputElem.value?.selectionStart == templateInputElem.value?.selectionEnd) {
        if (e.key == ')') {
            if (config.value.template[cursor.value] == ')') {
                e.preventDefault();
                moveCursor(cursor.value + 1);
            }
        }
        if (e.key == '"') {
            if (config.value.template[cursor.value] == '"') {
                e.preventDefault();
                moveCursor(cursor.value + 1);
            }
        }
        if (e.key == '%') {
            if (config.value.template[cursor.value] == '%') {
                e.preventDefault();
                moveCursor(cursor.value + 1);
            }
        }
    }
    
    return true;
}

/// Move cursor in template field
function moveCursor(pos: number) {
    templateInputElem.value?.setSelectionRange(pos, pos);
}

/// Add text to template
function injectTemplate(index: number, text: string) {
    templateInputElem.value!.value = templateInputElem.value!.value.substring(0, index) + text + templateInputElem.value!.value.substring(index);
    config.value.template = templateInputElem.value!.value;
}

/// Update the preview
function updatePreview() {
    $1t.send('renamerPreview', { config: config.value });
}

// Start renaming
function start(force = false) {
    // Dialog
    if (!force) {
        $q.dialog({
            title: 'Warning',
            message: 'Many DJ apps store cue points and other metadata based on the original file name. When renamed, this information will be lost and you will have to reimport these files.',
            html: true,
            ok: {
                color: 'primary',
                label: 'Start'
            },
            cancel: {
                color: 'primary',
                flat: true
            }
        })
        .onOk(() => {
            start(true);
        });
        return;
    }

    // Prevent reference
    $1t.settings.value.renamer = JSON.parse(JSON.stringify(config.value));
    $1t.saveSettings(true);
    $1t.lock.value.locked = true;
    $1t.send('renamerStart', { config: config.value });
}

/// Move suggestions box
function onScroll(e: Event) {
    // @ts-ignore
    suggestionsTop.value = e.target.scrollTop;
}

onMounted(() => {
    $1t.onRenamerEvent = (json: any) => {
        switch (json.action) {
            // Browse folder
            case 'browse':
                if (json.context == 'rnOutput')
                    config.value.outDir = json.path
                else 
                    config.value.path = json.path;

                updatePreview();
                break;
            // Syntax highlight
            case 'renamerSyntaxHighlight':
                highlighted.value = json.html;
                break;
            // Finished
            case 'renamerDone':
                $1t.lock.value.locked = false;
                $q.dialog({
                    title: 'Done',
                    message: 'Renaming finished!',
                    html: true,
                    ok: {
                        color: 'primary',
                        label: 'Open Folder'
                    },
                    cancel: {
                        color: 'primary',
                        flat: true
                    }
                }).onOk(() => {
                    $1t.send('openFolder', { path: config.value.outDir??config.value.path });
                });
                break;
            // Suggestions
            case 'renamerAutocomplete':
                suggestions.value = json.suggestions;
                suggestionOffset.value = json.offset;
                if (suggestionIndex.value > suggestions.value.length)
                    suggestionIndex.value = 0;
                break;
            // Preview renamed files
            case 'renamerPreview':
                preview.value = json.files;
                break;
            default:
                console.error(`Unknown action: ${json}`);
        }
    }

    // Restore settings
    if ($1t.settings.value.renamer) {
        config.value = Object.assign({}, config.value, $1t.settings.value.renamer);
        // console.log(config.value);
        if (config.value.template) {
            $1t.send('renamerSyntaxHighlight', { template: config.value.template });
            // @ts-ignore
            document.getElementsByClassName('template-input')[0].value = config.value.template;
        }
    }

    // Fix scroll suggestions box
    document.addEventListener('scroll', onScroll, true);
});

// Calculate character width after render
watchEffect(() => {
    setTimeout(() => {
        charWidth.value = textWidthRef.value!.offsetWidth / 36.0;
    }, 100);
})

onUnmounted(() => {
    // Remove event
    document.removeEventListener('scroll', onScroll, true);
});

const startable = computed(() => config.value.path && config.value.template);
const cursorStyle = computed(() => `margin-left: ${12 + cursor.value * charWidth.value}px`);
const suggestionsStyle = computed(() => {
    let top = `margin-top: -${suggestionsTop}px;`;
        if ((cursor.value * charWidth.value) > 500) {
            return `${top} margin-left: ${12 + cursor.value * charWidth.value - 500}px`;
        }
    return `${top} margin-left: ${12 + cursor.value * charWidth.value}px`;
});

watch(() => config.value.template, () => {
    // Debounce and render preview
    let cur = config.value.template;
    setTimeout(() => {
        if (cur != config.value.template || !config.value.template || !startable.value) return;
        updatePreview();
    }, 400);
});
</script>

<style lang='scss'>
.template-input {
    text-align: left;
    background-color: #99999910;
    padding-left: 12px;
    padding-right: 12px;
    padding-top: 20px;
    padding-bottom: 20px;
    outline: none !important;
    border-radius: 4px;
    font-size: 16px;
    width: 800px;
    border: none;
    color: #ffffff00;
}

.template-input-placeholder {
    color: #ffffffb2;
    border-style: none;
}

.template-text span {
    font-family: monospace !important;
}

.template-text {
    position: relative; 
    z-index: 10; 
    top: 44px;
    margin-left: 12px;
    font-size: 16px;
    max-width: 776px;
    text-align: left;
    pointer-events: none;
}

.fake-cursor {
    position: relative;
    top: 57px;
    z-index: 20;
    height: 16px;
    width: 4px;
    margin-left: 12px;
    font-weight: bold;
    font-size: 20px;
    transition: margin-left 0.1s;
    animation-name: blink;
    animation-duration: 2s;
    animation-iteration-count: infinite;
}

@keyframes blink {
    0% { opacity: 0.25; }
    50% { opacity: 0.64; }
    100% { opacity: 0.25; }
}

.suggestions-box {
    background-color: #111111;
    max-width: 500px;
    width: 500px;
    font-size: 16px;
    text-align: left;
    padding: 8px;
    display: flex;
    position: absolute;
    z-index: 10;
}

.suggestion-help-function {
    font-size: 14px;
}

.help-suggestion-selected {
    background-color: #070707;
}

.custom-margin {
    margin-top: 35px !important;
}

.custom-separator {
    width: 150px;
}
</style>