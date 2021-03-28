<template>
<div>

    <!-- Icon -->
    <q-btn round flat dense icon='mdi-keyboard-outline' @click='overlay = true' v-if='!isSet'></q-btn>
    <div v-if='isSet' class='row q-pt-sm justify-around' @click='overlay = true'>
        <q-icon v-if='value.shift' name='mdi-apple-keyboard-shift' size='xs' class='col-2'></q-icon>
        <q-icon v-if='value.ctrl' name='mdi-apple-keyboard-control' size='xs' class='col-2'></q-icon>
        <q-icon v-if='value.alt' name='mdi-apple-keyboard-option' size='xs' class='col-2'></q-icon>
        <q-icon :name='keyIcon(value)' size='xs' class='col-2'></q-icon>
    </div>

    <!-- Dialog picker -->
    <q-dialog @keydown='keydown' v-model='overlay'>
        <q-card>
            <q-card-section>
                <div class='text-h6'>Set keybind</div>
            </q-card-section>
            <q-card-section class='preview'>
                <q-icon v-if='key.shift' name='mdi-apple-keyboard-shift' size='md' class='q-px-sm'></q-icon>
                <q-icon v-if='key.ctrl' name='mdi-apple-keyboard-control' size='md' class='q-px-sm'></q-icon>
                <q-icon v-if='key.alt' name='mdi-apple-keyboard-option' size='md' class='q-px-sm'></q-icon>
                <q-icon :name='keyIcon(key)' size='md' class='q-px-sm'></q-icon>
            </q-card-section>
            <q-card-section class='text-right'>
                <q-btn flat color='red' @click='reset' v-if='isSet'>Reset</q-btn>
                <q-btn flat @click='close'>Cancel</q-btn>
                <q-btn flat color='primary' v-if='key.key' @click='set'>Save</q-btn>
            </q-card-section>
        </q-card>
    </q-dialog>

</div>
</template>

<script>
export default {
    name: 'Keybind',
    props: {
        initial: {
            type: Object
        }
    },
    data() {
        return {
            overlay: false,
            key: {
                ctrl: false,
                key: null,
                alt: false,
                shift: false
            },
            value: null
        }
    },
    methods: {
        keydown(e) {
            //Save key
            if (e.code.match(/F\d{1,2}/) || e.code.startsWith('Key') || e.code.startsWith("Digit") || e.code.startsWith("Numpad")) {
                this.key.key = e.code.toLowerCase().replace("key", "").replace("digit", "").replace("numpad", "");
                this.key.ctrl = e.ctrlKey;
                this.key.alt = e.altKey;
                this.key.shift = e.shiftKey;
                e.preventDefault();
            }
        },
        //Close popup and reset
        close() {
            this.overlay = false;
            this.key = {
                ctrl: false, alt: false, key: null, shift: false
            };
        },
        //Save
        set() {
            this.$emit('set', this.key);
            this.value = this.key;
            this.close();
        },
        //Callback with null
        reset() {
            this.$emit('set', null);
            this.value = null;
            this.close();
        },
        //Get icon for key
        keyIcon(k) {
            if (!k || !k.key) return '';
            //Numeric
            if (!isNaN(parseInt(k.key, 10))) {
                return `mdi-numeric-${k.key}-box-outline`;
            }
            //F
            if (k.key.toString().match(/f\d{1,2}/)) {
                return `mdi-keyboard-${k.key}`;
            }
            return `mdi-alpha-${k.key}-box-outline`;
        }
    },
    computed: {
        isSet() {
            if (!this.value || !this.value.key) return false;
            return true;
        },
    },
    watch: {
        initial() {
            this.value = this.initial;
        }
    },
    mounted() {
        this.value = this.initial;
    }
}
</script>

<style>
.preview {
    text-align: center;
    min-width: 200px;
}
</style>