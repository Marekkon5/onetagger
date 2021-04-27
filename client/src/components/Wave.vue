<template>
    <div class='wave' :style='style'></div>
</template>

<script>
export default {
    name: 'Wave',
    props: {
        height: String,
        width: String,
        value: Number,
        filled: Number
    },
    computed: {
        //Generate style
        style() {
            return `
                height: calc(${this.height} * ${this.value}); 
                width: ${this.width};
                background: linear-gradient(to right, var(--q-color-primary) ${Math.round(this.filled*100)}%, #3a3c3c ${Math.round(this.filled*100)}%, #3a3c3c);
            `;
        }
    }
}
</script>

<style>
.wave {
    border-radius: 2px;
    transition: height 0.25s;
}
</style>