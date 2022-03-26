<template>
<div class='text-center'>

    <div class='q-py-xl' style='max-width: 800px; margin: auto;' v-if='!$1t.lock.locked'>
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
            <div class='fake-cursor' :style='"margin-left: " + cursor + "px;"'>|</div>
            <div class='template-text'>
                <span v-if='config.template' v-html='highlighted'></span>
                <span v-if='!config.template' class='template-input-placeholder'>Filename template</span>
            </div>
            <div 
                class='template-input monospace' 
                @blur='cursor = -10000;'  
                contenteditable="true" 
                @focus='onSelectionChange'
                @input='templateInput'>
            </div>
        </div>

        <!-- Options -->
        <div class='text-h4 q-my-md'>Options</div>
        <q-toggle v-model='config.copy' label='Copy files instead of moving'></q-toggle>
        <br>
        <q-toggle v-model='config.subfolders' label='Include subfolders'></q-toggle>
        <br>
        <q-toggle v-model='config.overwrite' label='Overwrite existing target files'></q-toggle>

        <!-- Start -->
        <br>
        <div class='q-mt-lg'></div>
        <q-btn round size='xl' color='primary' icon='mdi-play' :disabled='!startable' @click='start'></q-btn>
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
                overwrite: false
            },
            highlighted: null,
            cursor: -99999,
            charWidth: 1.0
        }
    },
    methods: {
        // Browse folder
        browse(output = false) {
            this.$1t.send('browse', { context: output ? 'rnOutput' : 'rn', path: this.config.path });
        },
        // Handle typing into the template box
        templateInput(e) {
            // Line breaks
            //TODO: FIX PRESSING ENTER
            if (e.inputType == 'insertParagraph') {
                e.target.innerText = e.target.innerText.replace('\n', '');
            }

            if (!this.config.template) {
                if (!e.data) return;
                this.config.template = e.data;
            } else {
                this.config.template = e.target.innerText;
            }
            this.$1t.send('renamerSyntaxHighlight', { template: this.config.template });
            // Update cursor
            this.onSelectionChange();
        },
        // Handle global selection change to update fake cursor (yes, pain)
        onSelectionChange() {
            let selection = document.getSelection();
            if (!selection.anchorNode) return; 
            if (selection.anchorNode.parentElement.classList.contains('template-input')) {
                this.cursor = 12 + selection.anchorOffset * this.charWidth;
            }
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
        }
    }
}
</script>

<style>
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