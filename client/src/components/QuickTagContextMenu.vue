<template>
    <q-menu touch-position context-menu class='no-menu-shadow'>
        <q-list>

            <q-item dense clickable @click='emit("manual-tag")' v-close-popup>
                <q-item-section>
                    Manual Tagger
                </q-item-section>
            </q-item>

        </q-list>
    </q-menu>
</template>

<script lang='ts' setup>

const emit = defineEmits(['manual-tag']);

</script>

<style lang='scss'>
.no-menu-shadow {
    box-shadow: none !important;
    outline: solid 1px #ffffff69;
}
</style>