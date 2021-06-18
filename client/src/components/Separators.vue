<template>
<div>

    <div class='row'>
        <q-input
            v-model='id3'
            filled
            label='ID3 Separator (MP3 + AIFF)'
            class='col-4 q-px-sm'
            @input='save'
        ></q-input>
        <q-input
            v-model='vorbis'
            filled
            label='FLAC Separator (Leave empty for default)'
            class='col-4 q-px-sm'
            @input='save'
        ></q-input>
        <q-input
            v-model='mp4'
            filled
            label='M4A Separator'
            class='col-4 q-px-sm'
            @input='save'
        ></q-input>
    </div>

</div>
</template>

<script>
export default {
    name: 'Separators',
    props: {
        initial: Object
    },
    data() {
        return {
            id3: this.initial.id3??', ',
            vorbis: this.initial.vorbis,
            mp4: this.initial.mp4
        }
    },
    methods: {
        save() {
            this.$emit('input', {
                id3: this.id3,
                vorbis: this.vorbis,
                mp4: this.mp4
            })
        }
    }
}
</script>