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
        <div class='text-subtitle1 text-bold q-pb-sm text-grey-4'>{{tag.name}}</div>
        <!-- Values -->
        <div v-for='(value, j) in tag.values' :key='i+"value"+j' @mouseleave="mouseOver.index = -1">
            <div 
                @mouseenter="mouseOver.index = j; mouseOver.tag = i"
                class='text-subtitle2 clickable' 
                :class='{"text-bold text-primary": selected(i, value.val) || (mouseOver.index == j && mouseOver.tag == i), "text-grey-6": !selected(i, value.val)}'
                @click='valueClick(i, value.val)'
            >{{value.val}}</div>
        </div>
    </div>

</div>
</template>

<script>
export default {
    name: "QuickTagRight",
    data() {
        return {
            mouseOver: {index: -1, tag: -1}
        }
    },
    methods: {
        //If the value is present in tag
        selected(tag, value) {
            if (!this.$1t.quickTag.track) return false;
            return this.$1t.quickTag.track.custom[tag].includes(value);
        },
        //Tag value click
        valueClick(tag, value) {
            if (!this.$1t.quickTag.track) return false;
            let i = this.$1t.quickTag.track.custom[tag].indexOf(value);
            // Add or remove
            if (i == -1) {
                this.$1t.quickTag.track.custom[tag].push(value);
            } else {
                this.$1t.quickTag.track.custom[tag].splice(i, 1);
            }
            
        }
    }
}
</script>