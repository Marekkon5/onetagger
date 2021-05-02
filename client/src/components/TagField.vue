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
//Tags you shouldn't use
const banned = {
    id3: ["APIC", "TXXX", "POPM", "TYER", "TDAT", "TORY", "TRDA", "TRCK", "TDTG", "TSOT", "TIT2",
        "TDOR", "TKEY", "TSOC", "TCMP", "TBPM", "TSOP", "TSO2", "TSOA", "SYLT", "TSRC"],
    flac: ["METADATA_BLOCK_PICTURE", "DATE", "ORIGINALDATE", "TRACKNUMBER", "TAGGINGTIME", 
        "TRACK", "TITLESORT", "ORIGYEAR", "INITIALKEY", "KEY", "COMPOSERSORT", "COMPILATION", 
        "BPM", "ARTISTSORT", "ALBUMARTISTSORT", "ALBUMSORT", "POPULARIMETER", "RATING", "ISRC"]
}
//Autocompletion
const options = {
    id3: [
        "TCON (Genre)",
        "TALB (Album)",
        "TPE2 (Album Artist)",
        "TCOM (Composer)",
        "TEXT (Lyricist)",
        "TIT3 (Mix Name)",
        "TOPE (Original Artist)",
        "TIT1 (Grouping Serato/VDJ)",
        "GRP1 (Grouping djay Pro)",
        "TPUB (Label)",
        "TPE4 (Remixer)",
        "IPLS (Producer ID3v2.3)",
        "TIPL (Producer ID3v2.4)",
        "TPE3 (Conductor)",
        "COMM (Comment)",
        "USLT (Unsynchronized Lyrics)"
    ],
    flac: [
        "ALBUM",
        "ALBUMARTIST",
        "COMPOSER",
        "GENRE",
        "LYRICS",
        "VERSION",
        "GROUPING",
        "ORGANIZATION",
        "PUBLISHER",
        "MIXARTIST",
        "REMIXER",
        "CONDUCTOR",
        "COMMENT",
        "LABEL"
    ]
};

export default {
    name: 'TagField',
    props: {
        //id3 or flac
        format: String,
        initial: String,
        dense: {
            default: false,
            type: Boolean
        },
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
            if (this.format == 'id3' && this.value.match(/[A-Z0-9]{4} \(.*\)/)) {
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

            if (v.toUpperCase() != v && !(this.format == 'id3' && v.length != 4))
                this.error = 'Tag names should be uppercase!';

            if (v.includes(' ') && !(this.format == 'id3' && v.length != 4))
                this.error = "Tag names shouldn't contain spaces!"

            if (banned[this.format].includes(v.toUpperCase()))
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
                case 'flac': return 'FLAC';
            }
            return null;
        },
        //Different color if writing to comment/custom tag
        color() {
            if (this.format == 'id3' && (this.value??'').length != 4) return 'yellow';
            return 'primary';
        }
    }
}
</script>