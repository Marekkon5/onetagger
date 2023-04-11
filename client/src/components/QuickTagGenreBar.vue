<template>
<div class='genre-bar' @mouseleave="onMouseLeave">

    <div class='row q-mx-md q-pt-sm no-wrap genre-bar-container'>
        <div v-for='(genre, i) in $1t.settings.value.quickTag.genres' :key='"genre"+i'>
            <div 
                @mouseenter="mouseOver = i"
                class='q-mx-sm text-subtitle1 clickable text-no-wrap text-grey-4' 
                :class='{"text-weight-bolder": isSelected(genre.genre) || mouseOver == i, "text-grey-7": !isSelected(genre.genre) && mouseOver != i}'
                @click='setGenre(genre.genre)'
            >
                {{genre.genre}}

                <!-- Subgenres -->
                <q-menu v-if='genre.subgenres' :model-value='mouseOver == i' class='no-shadow'>
                    <q-list @mouseleave="mouseOver = -1" class='bg-darker'>
                        <div v-for='(subgenre, j) in genre.subgenres' :key='"sg"+j'>
                            <q-item clickable @click='setGenre(subgenre)'>
                                <q-item-section>
                                    <div class='row'>
                                        <q-icon name='mdi-check' class='q-pr-sm q-pt-xs' color='primary' v-if='isSelected(subgenre)'></q-icon>
                                        <div>{{subgenre}}</div>
                                    </div>

                                </q-item-section>
                            </q-item>
                        </div>
                    </q-list>
                </q-menu>

            </div>
        </div>
    </div>

</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { get1t } from '../scripts/onetagger.js';

const $1t = get1t();
const mouseOver = ref(-1);

function isSelected(genre: string) {
    return $1t.quickTag.value.track!.genres.includes(genre);
}

function setGenre(genre: string) {
    $1t.quickTag.value.track!.toggleGenre(genre);
}

// Stay open if subgenres are present
function onMouseLeave() {
    if (mouseOver.value == -1) return;
    if ($1t.settings.value.quickTag.genres[mouseOver.value].subgenres.length > 0) return;
    mouseOver.value = -1;
}

</script>

<style>
.genre-bar {
    height: 40px;

}
.genre-bar-container {
    max-width: calc(100vw - 32px); 
    width: calc(100vw - 32px); 
    overflow-x: scroll;
}
.genre-bar-container::-webkit-scrollbar:horizontal {
    height: 5px !important;
}

</style>