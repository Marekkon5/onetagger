<template>
<div class='row'>

    <TagField
        format='id3'
        :dense='dense'
        :tageditor='tageditor'
        :initial='tag.id3'
        @change='tag.id3 = $event; onChange()'
        class='col-6 q-pr-md'
    ></TagField>

    <TagField
        format='vorbis'
        :dense='dense'
        :tageditor='tageditor'
        :initial='tag.vorbis'
        @change='tag.vorbis = $event; onChange()'
        class='col-6 q-pr-md'
    ></TagField>

</div>
</template>

<script>
import TagField from './TagField';

export default {
    name: 'TagFields',
    components: {TagField},
    props: {
        value: Object,
        //Passthru
        dense: {
            type: Boolean,
            default: false
        },
        tageditor: {
            default: false,
            type: Boolean
        }
    },
    data() {
        return {
            tag: this.value
        }
    },
    methods: {
        //Update
        onChange() {
            this.$emit('input', this.tag);
        }
    }
}

</script>