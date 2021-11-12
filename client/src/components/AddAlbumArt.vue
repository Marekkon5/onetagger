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
                <q-img v-if='image' :src='image' style='height: 200px;' ref='image'></q-img>
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

<script>
export default {
    name: 'AddAlbumArt',
    props: {
        types: Array
    },
    data() {
        return { 
            drag: false,
            type: null,
            image: null,
            description: null
        }
    },
    methods: {
        drop(e) {
            //Get file
            let files = e.dataTransfer.files;
            if (files.length !== 1) return;
            let file = files[0];
            if (!file.type.includes('image/')) return;

            //Read
            let reader = new FileReader();
            reader.onload = f => {
                this.image = f.target.result;
            }
            reader.readAsDataURL(file);
            this.drag = false;
        },
        async add() {
            //Load width/height
            let wh = await new Promise((res) => {
                let i = new Image();
                i.onload = function() {
                    res([i.width, i.height]);
                }
                i.src = this.image;
            });

            let image = {
                data: this.image.substring(this.image.indexOf('base64,')+7).trim(),
                mime: this.image.substring(5, this.image.indexOf(';')),
                description: this.description??'',
                kind: this.type,
                width: wh[0],
                height: wh[1]
            }
            this.$emit("save", image);
            this.$emit("close");
        }
    }
}
</script>