<template>
<div class='text-center'>

    <div class='q-py-lg' v-if='!$1t.lock.locked'>
        <div style='max-width: 800px; margin: auto;'>
            <!-- Input and output folders -->
            <div class='text-h5 text-grey-4'>Select input / output</div>
            <div class='text-subtitle2 q-mb-md text-grey-6'>Drag & drop folder, copy/paste path directly or click the <q-icon name='mdi-open-in-app'></q-icon> icon to browse</div>
        
            <div class='row justify-center input' style='max-width: 725px; margin: auto;'>
                <q-input filled class='col-10' label='Input folder' v-model='config.path'>
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse(false)'></q-btn>
                    </template>
                </q-input>
            </div>
    
            <div class='q-pt-lg row justify-center input' style='max-width: 725px; margin: auto;'>
                <q-input filled class='col-10' label='Output folder (leave empty for same as input)' v-model='config.outDir'>
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse(true)'></q-btn>
                    </template>
                </q-input>
            </div>
    
            <!-- Template -->
            <q-separator class='q-mx-auto q-mt-xl q-mb-lg custom-separator' inset color="dark" />
            <div class='text-h5 text-grey-4 custom-margin'>Template</div>
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
                    ref='templateInput'
                    @blur='onBlur'
                    @focus='onSelectionChange'
                    @selectionchange='onSelectionChange'
                    @keyup='onSelectionChange'
                    @keydown='onKeyDown'
                    @input='templateInput'
                    @click='onSelectionChange'
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
            <div class='q-mt-md q-mb-sm text-h6 text-grey-4 custom-margin'>Preview</div>
            <div v-for='(file, i) in preview' :key='"prev"+i'>
                <div class='text-caption monospace text-grey-5'>{{file[1]}}</div>
                <br>
            </div>
        </div>
        

        
        <!-- Options -->
        <div class='full-width'>
            <q-separator class='q-mx-auto q-mt-lg q-mb-lg custom-separator' inset color="dark" />
            <div class='q-mb-sm text-h5 text-grey-4 custom-margin'>Options</div>

            <q-toggle v-model='config.copy' label='Copy files instead of moving'></q-toggle>
            <br>
            <q-toggle v-model='config.subfolders' label='Include subfolders'></q-toggle>
            <br>
            <q-toggle v-model='config.overwrite' label='Overwrite existing target files'></q-toggle>
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
    <q-page-sticky position='bottom-right' :offset='[36, 24]'>
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
    </q-page-sticky>

    <!-- Loading -->
    <div v-if='$1t.lock.locked'>
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

<script>
import RenamerTokenName from '../components/RenamerTokenName.vue';

export default { 
    name: 'Renamer',
    components: { RenamerTokenName },
    data() {
        return {
            config: {
                path: null,
                outDir: null,
                template: null,
                copy: false,
                subfolders: true,
                overwrite: false,
                separator: ", ",
            },
            highlighted: null,
            cursor: -99999,
            charWidth: 1.0,
            suggestions: [],
            suggestionIndex: 0,
            suggestionOffset: 0,
            suggestionsTop: 0,
            preview: []
        }
    },
    methods: {
        // Browse folder
        browse(output = false) {
            this.$1t.browse(output ? 'rnOutput' : 'rn', this.config.path);
        },
        // Handle typing into the template box
        templateInput(e) {
            if (!this.config.template) {
                if (!e.data) return;
                this.config.template = e.data;
            }
            
            // Autoclose
            let pos = this.cursor;
            if (e.data == '(') {
                this.injectTemplate(this.cursor + 1, ')');
                this.moveCursor(pos + 1);
            }
            if (e.data == '"') {
                this.injectTemplate(this.cursor, '"');
                this.moveCursor(pos + 1);
            }
            if (e.data == '%') {
                this.injectTemplate(this.cursor, '%');
                this.moveCursor(pos + 1);
            }

            this.config.template = e.target.value;
            this.updateTemplate();
        },
        // Fetch syntax highlighting and ac
        updateTemplate() {
            this.$1t.send('renamerSyntaxHighlight', { template: this.config.template });
            this.$1t.send('renamerAutocomplete', { 
                template: this.config.template.substring(0, this.cursor + 1) 
            });
            // Update cursor
            this.onSelectionChange();
        },
        // Handle global selection change to update fake cursor (yes, pain)
        onSelectionChange() {
            this.cursor = this.$refs.templateInput.selectionStart;
        },
        /// Template blur
        onBlur() {
            this.cursor = -6969;
            this.suggestions = [];
        },
        /// Template key down
        onKeyDown(e) {
            // Control suggestions
            if (e.key == "ArrowDown") {
                if (this.suggestionIndex < this.suggestions.length - 1) this.suggestionIndex += 1;
                e.preventDefault();
                return;
            }
            if (e.key == "ArrowUp") {
                if (this.suggestionIndex > 0) this.suggestionIndex -= 1;
                e.preventDefault();
                return;
            }
            // Enter override
            if (e.key == "Enter") {
                if (this.suggestions[this.suggestionIndex]) {
                    // Fill suggestion
                    let text = this.suggestions[this.suggestionIndex].name.substring(this.suggestionOffset);
                    let pos = this.cursor;
                    this.injectTemplate(this.cursor, text);
                    this.updateTemplate();
                    this.moveCursor(pos + text.length);
                }
                e.preventDefault();
                return;
            }
            // Don't close again
            if (this.$refs.templateInput.selectionStart == this.$refs.templateInput.selectionEnd) {
                if (e.key == ')') {
                    if (this.config.template[this.cursor] == ')') {
                        e.preventDefault();
                        this.moveCursor(this.cursor + 1);
                    }
                }
                if (e.key == '"') {
                    if (this.config.template[this.cursor] == '"') {
                        e.preventDefault();
                        this.moveCursor(this.cursor + 1);
                    }
                }
                if (e.key == '%') {
                    if (this.config.template[this.cursor] == '%') {
                        e.preventDefault();
                        this.moveCursor(this.cursor + 1);
                    }
                }
            }
            

            return true;
        },
        /// Move cursor in template field
        moveCursor(pos) {
            this.$refs.templateInput.setSelectionRange(pos, pos);
        },
        /// Add text to template
        injectTemplate(index, text) {
            this.$refs.templateInput.value = this.$refs.templateInput.value.substring(0, index) + text + this.$refs.templateInput.value.substring(index);
            this.config.template = this.$refs.templateInput.value;
        },
        /// Start renaming
        start(force = false) {
            // Dialog
            if (!force) {
                this.$q.dialog({
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
                    this.start(true);
                });
                return;
            }

            // Prevent reference
            this.$1t.settings.renamer = JSON.parse(JSON.stringify(this.config));
            this.$1t.saveSettings(true);
            this.$1t.lock.locked = true;
            this.$1t.send('renamerStart', { config: this.config });
        },
        /// Move suggestions box
        onScroll(e) {
            this.suggestionsTop = e.target.scrollTop;
        }
    },
    mounted() {
        this.$1t.onRenamerEvent = (json) => {
            switch (json.action) {
                // Browse folder
                case 'browse':
                    if (json.context == 'rnOutput')
                        this.config.outDir = json.path
                    else 
                        this.config.path = json.path;
                    break;
                // Syntax highlight
                case 'renamerSyntaxHighlight':
                    this.highlighted = json.html;
                    break;
                // Finished
                case 'renamerDone':
                    this.$1t.lock.locked = false;
                    this.$q.dialog({
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
                        this.$1t.send('openFolder', { path: this.config.outDir??this.config.path });
                    });
                    break;
                // Suggestions
                case 'renamerAutocomplete':
                    this.suggestions = json.suggestions;
                    this.suggestionOffset = json.offset;
                    if (this.suggestionIndex > this.suggestions.length)
                        this.suggestionIndex = 0;
                    break;
                // Preview renamed files
                case 'renamerPreview':
                    this.preview = json.files;
                    break;
                default:
                    console.error(`Unknown action: ${json}`);
            }
        }
        // Restore settings
        if (this.$1t.settings.renamer) {
            this.config = Object.assign({}, this.config, this.$1t.settings.renamer);
            if (this.config.template) {
                this.$1t.send('renamerSyntaxHighlight', { template: this.config.template });
                document.getElementsByClassName('template-input')[0].value = this.config.template;
            }
        }

        // Pain (character width)
        this.charWidth = this.$refs.textWidthRef.offsetWidth / 36.0;

        // Fix scroll suggestions box
        document.addEventListener('scroll', this.onScroll, true);
    },
    unmounted() {
        // Remove event
        document.removeEventListener('scroll', this.onScroll, true);
    },
    computed: {
        startable() {
            return this.config.path && this.config.template
        },
        cursorStyle() {
            return `margin-left: ${12 + this.cursor * this.charWidth}px`
        },
        // Autocomplete suggestions style
        suggestionsStyle() {
            let top = `margin-top: -${this.suggestionsTop}px;`
            if ((this.cursor * this.charWidth) > 500) {
                return `${top} margin-left: ${12 + this.cursor * this.charWidth - 500}px`;
            }
            return `${top} margin-left: ${12 + this.cursor * this.charWidth}px`
        }
    },
    watch: {
        'config.template'() {
            // Debounce and render preview
            let cur = this.config.template;
            setTimeout(() => {
                if (cur != this.config.template || !this.config.template || !this.startable) return;
                this.$1t.send('renamerPreview', { config: this.config });
            }, 400);
        },
    }
}
</script>

<style lang='scss'>
.template-input {
    text-align: left;
    background-color: #ffffff12;
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
    background-color: #101211;
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
</style>