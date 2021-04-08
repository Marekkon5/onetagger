<template>
<div class='q-pa-md'>

    <div v-for='(tag, i) in $1t.settings.quickTag.custom' :key='"tag"+i' class='q-pb-md'>
        <!-- Tag title -->
        <div class='text-subtitle1 text-bold q-pb-sm text-primary'>{{tag.name}}</div>
        <!-- Values -->
        <div v-for='(value, j) in tag.values' :key='i+"value"+j' @mouseleave="mouseOver = -1">
            <div 
                @mouseenter="mouseOver = j"
                class='text-subtitle2 clickable' 
                :class='{"text-bold": selected(tag, j) || mouseOver == j, "text-grey-6": !selected(tag, j)}'
                @click='valueClick(tag, j)'
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
            mouseOver: -1
        }
    },
    methods: {
        //If the value is present in tag
        selected(tag, j) {
            if (!this.$1t.quickTag.track) return false;
            return this.$1t.quickTag.track.hasCustom(tag, j);
        },
        //Tag value click
        valueClick(tag, j) {
            if (!this.$1t.quickTag.track) return false;
            this.$1t.quickTag.track.toggleCustom(tag, j);
        }
    }
}
</script>