<template>
<div
    @dragover.prevent='drag = true'
    @dragleave.prevent='drag = false'
    @drop.prevent='drop'
>
    <q-card flat class='elevation-4' :class='{"bg-highlight": drag, "playlist-drop-zone": !drag}' v-if='!tiny'>
        <q-card-section>
            <div 
                style='width: 100%; height: 50px;'
                class='justify-center text-center row items-center'
            >
                <span class='text-subtitle1 text-grey-4' v-if='!filename'>
                    <q-icon name='mdi-playlist-music' size='sm' class='q-pr-sm q-pb-xs text-grey-6'></q-icon>
                    Drag & drop M3U Playlist file
                </span>
                <div v-if='filename'>
                    <q-icon name='mdi-playlist-music' size='sm' class='q-pr-sm q-pb-xs' color='primary'></q-icon>
                    <span class='text-subtitle1 text-grey-4'>{{filename}}</span>
                    <q-btn @click='remove' icon='mdi-delete' color='red' flat round class='q-ml-sm q-mb-xs'></q-btn>
                </div>
            </div>
        </q-card-section>
    </q-card>

    <!-- Tiny mode -->
    <div v-if='tiny'>
        <q-btn
            round
            flat
            icon='mdi-playlist-music'
            :color='(drag || filename) ? "primary" : undefined'
            @click='filename ? remove() : true'
        >
            <q-tooltip>
                Drag & drop playlist here / click to remove it
            </q-tooltip>
        </q-btn>
    </div>
</div>
</template>

<script lang='ts' setup>
import { PropType, ref } from 'vue';
import { Playlist } from '../scripts/utils';

const { modelValue, tiny } = defineProps({
    modelValue: { type: Object as PropType<Playlist> },
    tiny: { default: false, type: Boolean },
});
const drag = ref(false);
const filename = ref(modelValue?.filename);
const emit = defineEmits(['update:model-value']);

function drop(e: DragEvent) {
    drag.value = false;
    // Get file
    let files = e.dataTransfer!.files;
    if (files.length !== 1) return;
    let file = files[0];
    // Filter supported
    let type = getType(file.type);
    if (!type) return;
    filename.value = file.name;

    // Read
    let reader = new FileReader();
    reader.onload = f => {
        //Emit
        emit('update:model-value', {
            data: f.target?.result,
            format: type,
            filename: file.name
        } as Playlist)
    }
    reader.readAsDataURL(file);
}

// Get type from mime
function getType(mime: string) {
    switch (mime.toLowerCase()) {
        case 'audio/mpegurl':
        case 'audio/x-mpegurl':
        case 'application/x-mpegurl':
            return 'm3u';
        default: 
            return;
    }
}

function remove() {
    filename.value = undefined;
    emit('update:model-value', {} as Playlist);
}

</script>

<style lang='scss'>
.playlist-drop-zone {
    background-color: #99999910 !important;
}
</style>