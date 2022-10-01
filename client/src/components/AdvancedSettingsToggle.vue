<template>
    <q-item style='padding-top: 0px; padding-bottom: 0px;'>
        <q-item-section>
            <q-item-label>
                {{label}}
                <q-icon name='mdi-help-circle-outline text-grey-6' class='q-mx-sm' v-if='tooltip'>
                    <q-tooltip>
                        {{tooltip}}
                    </q-tooltip>
                </q-icon>
            </q-item-label>
        </q-item-section>
        <q-item-section avatar>
            <q-toggle :model-value='modelValue' @update:model-value='(v) => emit("update:modelValue", v)'></q-toggle>
        </q-item-section>
    </q-item>
</template>

<script lang='ts' setup>
const { label, tooltip, modelValue } = defineProps({
    label: String,
    tooltip: {
        type: String,
        default: null
    },
    modelValue: Boolean
});
const emit = defineEmits(['update:modelValue']);
</script>