<template>
<div>
    <div class='row container' @mouseover='onHover' @mouseleave="hover = false" @click='seek'>
        <div v-for='(wave, i) in $1t.player.waveform' :key='i'>
            <Wave 
                height='50px' 
                width='0.2vw' 
                :value='wave + 0.05' 
                class='wave'
                :filled='filled(i)'
            ></Wave>
        </div>
    </div>
</div>
</template>

<script>
import Wave from './Wave';
export default {
    name: 'Waveform',
    components: {Wave},
    data() {
        return {
            hover: false,
            pos: 0,
        }
    },
    methods: {
        //If wave filled
        filled(i) {
            if (i < Math.floor(this.pos)) 
                return 1;
            
            if (Math.floor(this.pos) == i) 
                return this.pos % 1;
            
            return 0;
        },
        //Mouse hover fill
        onHover(e) {
            this.hover = true;
            this.pos = ((e.clientX - 276) / (window.innerWidth * 0.55)) * this.$1t.WAVES;
        },
        //On click seek
        seek(e) {
            let pos = ((e.clientX - 276) / (window.innerWidth * 0.55)) * this.$1t.player.duration;
            this.$1t.seek(Math.round(pos));
        }
    },
    watch: {
        '$1t.player.position'() {
            if (this.hover) return;
            this.pos = (this.$1t.player.position / this.$1t.player.duration) * this.$1t.WAVES;
        }
    }
}
</script>

<style>
.container {
    height: 60px;
    max-height: 60px;
    min-height: 60px;
    align-items: center;
}
.wave {
    margin-left: 0.1vw;
}
</style>