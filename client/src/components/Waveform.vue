<template>
<div>
    <div class='row items-center'>
        <span class='q-pr-sm monospace q-pb-xs'>{{time}}</span>

        <div 
            ref='waveform' 
            @mouseover='onHover' 
            @mouseleave="hover = false" 
            @click='seek'
            :style='`background-image: linear-gradient(to right, var(--q-primary) ${pos*100}%, #3a3c3c ${pos*100}%, #3a3c3c);`'
            class='waveform' 
        >
            <span v-for='(wave, i) in $1t.player.value.waveform' class='wavefont' :key='wave + i'>
                {{waveChar(wave)}}
            </span>
        </div>

        <span class='q-pl-sm monospace q-pb-xs'>{{duration($1t.player.value.duration)}}</span>

    </div>
</div>
</template>

<script lang='ts' setup>
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { get1t } from '../scripts/onetagger';

const $1t = get1t();
const hover = ref(false);
const pos = ref(0);
const waveform = ref<any>();

// Generate wave character
function waveChar(wave: number) {
    return String.fromCharCode(0x100 + Math.min(Math.round((wave + 0.05) * 100), 100));
}

function waveOffset(e: MouseEvent) {
    return (e.pageX - waveform.value.offsetLeft) / waveform.value.clientWidth;
}

// Mouse hover fill
function onHover(e: MouseEvent) {
    hover.value = true;
    pos.value = waveOffset(e);
}

// On click seek
function seek(e: MouseEvent) {
    let pos = waveOffset(e) * $1t.player.value.duration;
    $1t.player.value.seek(Math.round(pos));
}

// Pretty print duration
function duration(a: number) {
    let s = Math.round(a / 1000);
    return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, '0')}`
}

// Update player position
let interval: any = undefined;
onMounted(() => {
    interval = setInterval(() => {
        if (hover.value) return;
        pos.value = ($1t.player.value.position / $1t.player.value.duration);
    }, 300);
});
onUnmounted(() => {
    clearInterval(interval);
});

const time = computed(() => duration(pos.value * $1t.player.value.duration));

</script>

<style>

@keyframes wave-anim {
    0% {
        transform: scale(1, 0.1);        
    }

    100% {
        transform: scale(1, 1.0);
    }
}

.waveform {
    /* background: linear-gradient(to right, var(--q-primary) var(--waveform-filled), #3a3c3c var(--waveform-filled), #3a3c3c); */
    background-clip: text;
    color: transparent;
    -webkit-background-clip: text;
	-webkit-text-fill-color: transparent;
}

.wavefont {
    font-family: wavefont, blank !important;
    font-variation-settings: 'wdth' 50, 'algn' 0.5, 'radi' 30;
    font-size: 40px;
    margin: 1px;
    display: inline-block;
    animation-name: wave-anim;
    animation-duration: 200ms;
}

</style>