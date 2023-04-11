<template>
    <q-card>
        <q-card-section>
            <div class='text-h6'>Add new album art</div>
        </q-card-section>
        <q-card-section>
            <q-select
                filled
                dense
                label='Type'
                :options='types'
                v-model='type'
                popup-content-class='no-shadow'
            ></q-select>
            <q-input
                filled
                dense
                label='Description'
                v-model='description'
                class='q-mt-sm'
            ></q-input>
            <div 
                style='height: 200px; width: 200px;' 
                class='justify-center text-center row items-center q-mt-md'
                :class='{"bg-darker": drag}'
                @dragover.prevent='drag = true'
                @dragleave.prevent='drag = false'
                @drop.prevent='drop'
            >
                <q-img v-if='image' :src='image' style='height: 200px;'></q-img>
                <span class='text-grey-7 text-h6' v-if='!image'>Drag & drop image here</span>
            </div>
        </q-card-section>
        <q-card-section class='justify-around row'>
            <q-btn color='red' @click='$emit("close")'>
                Cancel
            </q-btn>
            <q-btn v-if='type && image' color='primary' @click='add'>
                Add
            </q-btn>
        </q-card-section>
    </q-card>
</template>

<script lang='ts' setup>
import { ref } from 'vue';

const drag = ref(false);
const type = ref<string | undefined>();
const image = ref<string | undefined>();
const description = ref<string | undefined>();
const { types } = defineProps({
    types: { type: Array, required: true }
});
const emit = defineEmits(['save', 'close']);

function drop(e: DragEvent) {
    // Get file
    let files = e.dataTransfer!.files;
    if (files.length !== 1) return;
    let file = files[0];
    if (!file.type.includes('image/')) return;

    // Read
    let reader = new FileReader();
    reader.onload = f => {
        image.value = f.target!.result as string;
    }
    reader.readAsDataURL(file);
    drag.value = false;
}

async function add() {
    // Load width/height
    let wh: [number, number] = await new Promise((res) => {
        let i = new Image();
        i.onload = function() {
            res([i.width, i.height]);
        }
        i.src = image.value!;
    });

    let outImage: any = {
        data: image.value!.substring(image.value!.indexOf('base64,')+7).trim(),
        mime: image.value!.substring(5, image.value!.indexOf(';')),
        description: description.value??'',
        kind: type.value,
        width: wh[0],
        height: wh[1]
    }
    emit("save", outImage);
    emit("close");
}

</script>