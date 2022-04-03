<template>
<span class='renamer-token'>

    <span>{{token.name}}</span>
    <span v-if='params && token.kind == "function"'>(<span v-for='(param, i) in token.parameters' :key='i' class='renamer-token'>
        <span
            :class='{"__renamer_syntax_string": param.type == "string", "__renamer_syntax_number": param.type == "number"}'
            >{{param.name}}</span>
        <span v-if='type' class='renamer-token'><span>: {{param.type}}</span></span>
        <span v-if='i != token.parameters.length - 1'>, </span>
    </span>)</span>

</span>
</template>

<script>
export default {
    name: 'RenamerTokenName',
    props: {
        token: {
            type: Object
        },
        params: {
            type: Boolean,
            default: true
        },
        type: {
            type: Boolean,
            default: true
        }
    }
}
</script>

<style>
.renamer-token > * {
    font-family: monospace !important;
}
</style>