<template>
<div class='text-center'>

    <div class='q-py-xl' v-if='!$1t.lock.locked'>
        <div style='max-width: 800px; margin: auto;'>
            <!-- Input and output folders -->
            <div class='text-h4 q-mb-md'>Folders</div>
    
            <div class='q-my-md'>
                <q-input filled class='col-10' label='Input folder' v-model='config.path'>
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse'></q-btn>
                    </template>
                </q-input>
            </div>
    
            <div class='q-my-md'>
                <q-input filled class='col-10' label='Output folder (leave empty for same as input)' v-model='config.outDir'>
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browse(true)'></q-btn>
                    </template>
                </q-input>
            </div>
    
            <!-- Template -->
            <div class='text-h4 q-mt-md'>Template</div>
            <div style='margin-top: -10px;'>
                <div class='fake-cursor' :style='cursorStyle'>|</div>
                <div class='template-text'>
                    <span v-if='config.template' v-html='highlighted'></span>
                    <span v-if='!config.template' class='template-input-placeholder'>Filename template</span>
                </div>
                <div 
                    class='template-input monospace' 
                    spellcheck="false"
                    ref='templateInput'
                    @blur='onBlur'  
                    contenteditable="true" 
                    @focus='onSelectionChange'
                    @keydown='onKeyDown'
                    @input='templateInput'>
                </div>
            </div>
    
            <!-- Autocomplete / suggestions -->
            <div v-if='suggestions.length > 0'>
                <div class='suggestions-box' :style='cursorStyle'>
                    <!-- Suggestions -->
                    <div style='width: 40%'>
                        <div v-for='(suggestion, i) in suggestions' :key="'s'+i" class='q-mr-sm q-pa-xs' :class='{"help-suggestion-selected": i == suggestionIndex}'>
                            <!-- icon -->
                            <q-icon name='mdi-variable' class='q-mb-xs' v-if='suggestion.kind == "variable"'></q-icon>
                            <q-icon name='mdi-information-outline' class='q-mb-xs' v-if='suggestion.kind == "property"'></q-icon>
                            <q-icon name='mdi-function' class='q-mb-xs' v-if='suggestion.kind == "function"'></q-icon>
    
                            <!-- name -->
                            <span class='monospace' :class='{"text-primary": i == suggestionIndex}'>
                                {{suggestion.name}}
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
                                <span>
                                    <span class='monospace'>{{suggestions[suggestionIndex].name}}</span>
                                    <span class='monospace'>(</span>
                                        <span v-for='(param, i) in suggestions[suggestionIndex].parameters' :key='"p"+i'>
                                            <span 
                                                class='monospace' 
                                                :class='{"syntax_string": param.type == "string", "syntax_number": param.type == "number"}'
                                            >{{param.name}}</span>
                                            <span class='monospace'>: {{param.type}}<span class='monospace' v-if='!param.required'>?</span></span>
                                            <span class='monospace' v-if='i != suggestions[suggestionIndex].parameters.length - 1'>, </span>
                                        </span>
                                    <span class='monospace'>)</span>
                                </span>
                                <br>
                            </div>
    
                            <!-- Actual suggestion -->
                            <div v-html='suggestions[suggestionIndex].doc'></div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <div class='row q-mt-md q-mx-xl'>
            <!-- Options -->
            <div style='width: 50%'>
                <div class='text-h4 q-my-md'>Options:</div>
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

            <!-- Preview -->
            <div style='width: 50%'>
                <div class='text-h4 q-my-md'>Preview:</div>
                <div v-for='(file, i) in preview' :key='"prev"+i'>
                    <div class='text-body1 monospace'>{{file[1]}}</div>
                    <br>
                </div>
            </div>
        </div>

        <!-- Start -->
        <br>
        <div class='q-mt-lg'></div>
        <q-btn round size='xl' color='primary' icon='mdi-play' :disabled='!startable' @click='start'></q-btn>
    </div>

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
export default { 
    name: 'Renamer',
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
            preview: []
        }
    },
    methods: {
        // Browse folder
        browse(output = false) {
            this.$1t.send('browse', { context: output ? 'rnOutput' : 'rn', path: this.config.path });
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
            this.config.template = e.target.innerText;
            
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
            let selection = document.getSelection();
            if (!selection.anchorNode) return; 
            if (selection.anchorNode.parentElement.classList.contains('template-input')) {
                this.cursor = selection.anchorOffset;
            }
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
                    this.injectTemplate(this.cursor, text);
                    this.updateTemplate();
                    this.moveCursor(this.cursor + text.length);
                }
                e.preventDefault();
                return;
            }
            // Don't close again
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
        },
        /// Move cursor in template field
        moveCursor(pos) {
            let selection = document.getSelection()
            selection.removeAllRanges()
            let range = document.createRange();
            range.setStart(this.$refs.templateInput.childNodes[0], pos);
            range.collapse();
            selection.addRange(range);
        },
        /// Add text to template
        injectTemplate(index, text) {
            this.$refs.templateInput.innerText = this.$refs.templateInput.innerText.substring(0, index) + text + this.$refs.templateInput.innerText.substring(index);
            this.config.template = this.$refs.templateInput.innerText;
        },
        /// Start renaming
        start() {
            // Prevent reference
            this.$1t.settings.renamer = JSON.parse(JSON.stringify(this.config));
            this.$1t.saveSettings(true);
            this.$1t.lock.locked = true;
            this.$1t.send('renamerStart', { config: this.config });
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
                        ok: {
                            color: 'primary',
                            label: 'OK'
                        },
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
                document.getElementsByClassName('template-input')[0].innerText = this.config.template;
            }
        }

        // Pain
        document.addEventListener('selectionchange', this.onSelectionChange);
        this.charWidth = this.$refs.textWidthRef.offsetWidth / 36.0;
    },
    destroyed() {
        document.removeEventListener('selectionchange', this.onSelectionChange);
    },
    computed: {
        startable() {
            return this.config.path && this.config.template
        },
        cursorStyle() {
            return `margin-left: ${12 + this.cursor * this.charWidth}px`
        },
    },
    watch: {
        'config.template'() {
            // Debounce and render preview
            let cur = this.config.template;
            setTimeout(() => {
                if (cur != this.config.template || !this.config.template || !this.startable) return;
                this.$1t.send('renamerPreview', { config: this.config });
            }, 400);
        }
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
    font-size: 16px;
    text-align: left;
    padding: 8px;
    display: flex;
}

.suggestion-help-function {
    font-size: 14px;
}

.help-suggestion-selected {
    background-color: #070707;
}

/* Syntax colors */
.syntax_text {
    color: #9e9e9e;
}
.syntax_operator {
    color: #78909c;
}
.syntax_string {
    color: #4caf50;
}
.syntax_number {
    color: #ff5722;
}
.syntax_function {
    color: #2196f3;
}
.syntax_property {
    color: #cfd8dc;
}
.syntax_variable {
    color: #cfd8dc;
}
</style>