<template>
<div>

    <q-select
        v-model='value'
        use-input
        fill-input
        hide-selected
        filled
        input-debounce="0"
        :color='color'
        :options='options'
        :dense='dense'
        :label='label'
        :error-message='error'
        :error='error?true:false'
        @input='onInput'
        @input-value='onInputValue'
        @filter='onFilter'
    ></q-select>


</div>
</template>
<script>
import {MP4, VORBIS, ID3} from '../js/tags';

//Tags you shouldn't use
const banned = {
    id3: ["APIC", "TXXX", "POPM", "TYER", "TDAT", "TORY", "TRDA", "TRCK", "TDTG", "TSOT", "TIT2",
        "TDOR", "TKEY", "TSOC", "TCMP", "TBPM", "TSOP", "TSO2", "TSOA", "SYLT", "TSRC"],
    vorbis: ["METADATA_BLOCK_PICTURE", "DATE", "ORIGINALDATE", "TRACKNUMBER", "TAGGINGTIME", 
        "TRACK", "TITLESORT", "ORIGYEAR", "INITIALKEY", "KEY", "COMPOSERSORT", "COMPILATION", 
        "BPM", "ARTISTSORT", "ALBUMARTISTSORT", "ALBUMSORT", "POPULARIMETER", "RATING", "ISRC"],
    mp4: []
}
const bannedTagEditor = {
    id3: ["APIC", "TXXX", "POPM", "COMM", "USLT"],
    vorbis: [],
    mp4: []
}

//Autocompletion
const options = {id3: ID3, vorbis: VORBIS, mp4: MP4};

export default {
    name: 'TagField',
    props: {
        //id3, vorbis, mp4
        format: String,
        initial: String,
        dense: {
            default: false,
            type: Boolean
        },
        tageditor: {
            default: false,
            type: Boolean
        }
    },
    data() {
        return {
            value: this.initial,
            error: null,
            options: options[this.format],
            
        }
    },
    methods: {
        //Remove ID3 helper text
        removeHelper() {
            if ((this.format == 'id3' || this.format == 'mp4') && this.value.match(/[©a-zA-Z0-9]{4} \(.*\)/)) {
                this.value = this.value.replace(/ \(.*\)/, '');
            }
        },
        //Clear helpers
        onInput(v) {
            this.value = v;
            this.removeHelper();
        },
        //Update text typed without autocomplete taking over
        onInputValue(v) {
            v = v.trim();
            this.value = v;
            this.removeHelper();

            //Validate
            this.warning = null;
            this.error = null;

            if (this.format != 'mp4' && v.toUpperCase() != v && !(this.format == 'id3' && v.length != 4))
                this.error = 'Tag names should be uppercase!';

            if (v.includes(' ') && !(this.format == 'id3' && v.length != 4))
                this.error = "Tag names shouldn't contain spaces!"

            if ((this.tageditor ? bannedTagEditor : banned)[this.format].includes(v.toUpperCase()))
                this.error = "This tag is reserved, might cause corruption!"

            if (!v)
                this.error = "Shouldn't be empty!"

            this.$emit('change', v);
        },
        //On filter for quasar
        onFilter(val, update) {
            update(() => {
                let n = val.toUpperCase();
                this.options = options[this.format].filter(v => v.toUpperCase().includes(n));
            });
        },
    },
    computed: {
        label() {
            switch (this.format) {
                case 'id3': return 'ID3 (MP3 + AIFF)';
                case 'vorbis': return 'FLAC';
                case 'mp4': return 'MP4/M4A';
            }
            return null;
        },
        //Different color if writing to comment/custom tag
        color() {
            if ((this.format == 'id3' || this.format == 'mp4') && (this.value??'').length != 4) return 'yellow';
            return 'primary';
        }
    }
}
</script>