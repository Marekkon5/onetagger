<template>
<div>
    <q-card class='inset-shadow-down' :class='{"bg-darker": drag}' >
        <q-card-section>
            <div 
                style='width: 100%; height: 50px;'
                @dragover.prevent='drag = true'
                @dragleave.prevent='drag = false'
                @drop.prevent='drop'
                class='justify-center text-center row items-center'
            >
            <span class='text-subtitle1 text-grey-4' v-if='!filename'><q-icon name='mdi-playlist-music' size='sm' class='q-pr-sm q-pb-xs text-grey-6'></q-icon>Drag and drop M3U Playlist file</span>
                <div v-if='filename'>
                    <q-icon name='mdi-playlist-music' size='sm' class='q-pr-sm q-pb-xs' color='primary'></q-icon>
                    <span class='text-subtitle1 text-grey-4'>{{filename}}</span>
                    <q-btn @click='remove' icon='mdi-close' color='red' flat round class='q-ml-sm q-mb-xs'></q-btn>
                </div>
            </div>
        </q-card-section>
    </q-card>
</div>
</template>

<script>
export default { 
    name: 'PlaylistDropZone',
    props: {
        value: Object
    },
    data() {
        return { 
            drag: false,
            filename: this.value.filename
        }
    },
    methods: {
        drop(e) {
            this.drag = false;
            //Get file
            let files = e.dataTransfer.files;
            if (files.length !== 1) return;
            let file = files[0];
            //Filter supported
            let type = this.getType(file.type);
            if (!type) return;
            this.filename = file.name;

            //Read
            let reader = new FileReader();
            reader.onload = f => {
                //Emit
                this.$emit('input', {
                    data: f.target.result,
                    format: type,
                    filename: file.name
                });
            }
            reader.readAsDataURL(file);
        },
        //Get type from mime
        getType(mime) {
            switch (mime.toLowerCase()) {
                case 'audio/mpegurl':
                case 'audio/x-mpegurl':
                    return 'm3u';
                default: 
                    return null;
            }
        },
        remove() {
            this.filename = null;
            this.$emit('input', {data: null, filename: null, format: null});
        }
    },
}

</script>