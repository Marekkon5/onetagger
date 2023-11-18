<template>
    <q-menu touch-position context-menu class='no-menu-shadow'>
        <q-list>

            <!-- Manual tag -->
            <q-item dense clickable @click='emit("manual-tag")' v-close-popup>
                <q-item-section avatar>
                    <q-icon name='mdi-magnify'></q-icon>
                </q-item-section>
                <q-item-section>
                    Manual Tag
                </q-item-section>
            </q-item>

            <!-- Edit tags -->
            <q-item dense clickable v-close-popup @click='tagEditor'>
                <q-item-section avatar>
                    <q-icon name='mdi-pencil'></q-icon>
                </q-item-section>
                <q-item-section>
                    Edit tags
                </q-item-section>
            </q-item>

            <!-- Delete file -->
            <q-item dense clickable v-close-popup @click='deleteFile'>
                <q-item-section avatar>
                    <q-icon name='mdi-delete' color='red'></q-icon>
                </q-item-section>
                <q-item-section class='text-red'>
                    Delete
                </q-item-section>
            </q-item>


        </q-list>
    </q-menu>
</template>

<script lang='ts' setup>
import { toRefs } from 'vue';
import { get1t } from '../scripts/onetagger';
import { useRouter } from 'vue-router';
import { useQuasar } from 'quasar';

const emit = defineEmits(['manual-tag']);
const props = defineProps({
    path: { type: String, required: true }
});
const { path } = toRefs(props);
const $1t = get1t();
const $q = useQuasar();
const $router = useRouter();

// Open tag editor
function tagEditor() {
    $1t.quickTag.value.toTagEditor = path.value;
    $router.push('/tageditor');
}

// Delete file option
function deleteFile() {
    // Confirm dialog
    $q.dialog({
        title: 'Delete File',
        message: 'Do you really want to delete the selected file?',
        persistent: false,
        ok: {
            color: 'red'                        
        },
        cancel: {
            color: ''
        }
    }).onOk(() => {
        if ($1t.player.value.path == path.value)
            $1t.player.value.stop();
        $1t.send('deleteFiles', { paths: [path.value] });
        setTimeout(() => {
            $1t.quickTag.value.track.removeAll();
            $1t.loadQuickTag();
        }, 50);
    });
}

</script>

<style lang='scss'>
.no-menu-shadow {
    box-shadow: none !important;
    outline: solid 1px #ffffff69;
}
</style>