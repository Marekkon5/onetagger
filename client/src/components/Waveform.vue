<template>
<div>
    <div class='row items-center'>
        <span class='q-pr-sm monospace q-pb-xs'>{{time}}</span>

        <div ref='waveform' class='row container' @mouseover='onHover' @mouseleave="hover = false" @click='seek'>
            <div v-for='(wave, i) in $1t.player.value.waveform' :key='i'>
                <Wave 
                    height='50px' 
                    width='0.2vw' 
                    :value='wave + 0.05' 
                    class='wave'
                    :filled='filled(i)'
                ></Wave>
            </div>
        </div>

        <span class='q-pl-sm monospace q-pb-xs'>{{duration($1t.player.value.duration)}}</span>

    </div>
</div>
</template>

<script lang='ts' setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { get1t } from '../scripts/onetagger';
import { WAVES } from '../scripts/player';
import Wave from './Wave.vue';

const $1t = get1t();
const hover = ref(false);
const pos = ref(0);
const waveform = ref<any>();


// If wave filled
function filled(i: number) {
    if (i < Math.floor(pos.value)) 
        return 1;
    
    if (Math.floor(pos.value) == i) 
        return pos.value % 1;
    
    return 0;
}

function waveOffset(cx: number) {
    return (cx - waveform.value.offsetLeft) / waveform.value.clientWidth;
}

// Mouse hover fill
function onHover(e: MouseEvent) {
    hover.value = true;
    pos.value = waveOffset(e.clientX) * WAVES;
}

// On click seek
function seek(e: MouseEvent) {
    let pos = waveOffset(e.clientX) * $1t.player.value.duration;
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
        pos.value = ($1t.player.value.position / $1t.player.value.duration) * WAVES;
    }, 300);
});
onUnmounted(() => {
    clearInterval(interval);
});

const time = computed(() => duration((pos.value / WAVES) * $1t.player.value.duration));
</script>

<style>
.container {
    height: 60px;
    max-height: 60px;
    min-height: 60px;
    align-items: center;
    /* WAVES * 0.3 */
    width: 54vw;
}
.wave {
    margin-left: 0.1vw;
}
</style>