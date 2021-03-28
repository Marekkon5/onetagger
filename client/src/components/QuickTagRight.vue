<template>
<div class='q-pa-md'>

    <div v-for='(tag, i) in $1t.settings.quickTag.custom' :key='"tag"+i' class='q-pb-md'>
        <!-- Tag title -->
        <div class='text-subtitle1 text-bold q-pb-sm text-primary'>{{tag.name}}</div>
        <!-- Values -->
        <div v-for='(value, j) in tag.values' :key='i+"value"+j'>
            <div 
                class='text-subtitle2' 
                :class='{"text-bold": selected(tag, j), "text-grey-6": !selected(tag, j)}'
                @click='valueClick(tag, j)'
            >{{value.val}}</div>
        </div>
    </div>

</div>
</template>

<script>
export default {
    name: "QuickTagRight",
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