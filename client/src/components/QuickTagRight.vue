<template>

<div class='q-pa-md'>
    <!-- Note -->
    <div class='full-width row justify-center'>
        <q-btn class='text-bold' flat color='primary' v-if='$1t.quickTag.track' @click='$1t.onQuickTagEvent("onNoteTag")'>
            Custom note
        </q-btn>
    </div>
    <div v-for='(tag, i) in $1t.settings.quickTag.custom' :key='"tag"+i' class='q-pb-md'>
        <!-- Tag title -->
        <q-expansion-item 
            :label='tag.name' 
            dense 
            :value='true'
            class='text-subtitle1 text-bold q-pb-sm text-grey-4'
            style='margin-bottom: -24px;'
        >
            <!-- Values -->
            <div v-for='(value, j) in tag.values' :key='i+"value"+j'>
                <q-checkbox
                    :label='value.val'
                    :value='selected(i, value.val)'
                    @input='valueClick(i, value.val)'
                    dense
                    class='text-subtitle2 text-grey-6 full-width'
                ></q-checkbox>
            </div>

            <!-- Add new -->
            <q-input dense @keypress.enter="addNewTag" v-if='newTag == i' v-model='newTagValue'></q-input>

            <q-btn round flat color='primary' class='add-custom-btn' v-if='newTag == -1' @click='newTag = i'>
                <q-icon name='mdi-plus'></q-icon>
            </q-btn>

        </q-expansion-item>
    </div>

    <!-- Reorder the values inside of tag -->
    <div class='full-width row justify-center'>
        <q-btn flat class='text-bold' color='primary' @click='sortValues' v-if='this.$1t.quickTag.track'>Sort values</q-btn>
    </div>

</div>
</template>

<script>
export default {
    name: "QuickTagRight",
    data() {
        return {
            // if adding new tag
            newTag: -1,
            newTagValue: null
        }
    },
    methods: {
        // If the value is present in tag
        selected(tag, value) {
            if (!this.$1t.quickTag.track) return false;
            return this.$1t.quickTag.track.custom[tag].includes(value);
        },
        // Tag value click
        valueClick(tag, value) {
            if (!this.$1t.quickTag.track) return false;
            this.$1t.quickTag.track.toggleCustom(tag, value);           
        },
        // Sort values inside of tag
        sortValues() {
            for (let i=0; i<this.$1t.quickTag.track.custom.length; i++) {
                this.$1t.quickTag.track.sortCustom(i);
            }
        },
        // Adding new tag
        addNewTag() {
            if (this.newTagValue) {
                this.$1t.settings.quickTag.custom[this.newTag].values.push({'val': this.newTagValue, 'keybind': null});
                this.$1t.saveSettings();
            }
            this.newTag = -1;
            this.newTagValue = null;
        }
    }
}
</script>

<style>
.q-expansion-item__container:first-child div {
    padding: 0px !important;
}
.q-checkbox__label {
    margin-left: 8px !important;
}
.add-custom-btn {
    position: absolute;
    top: -32px;
    left: 128px;
}
</style>