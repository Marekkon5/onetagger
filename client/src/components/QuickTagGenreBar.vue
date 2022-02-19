<template>
<div class='genre-bar' @mouseleave="onMouseLeave">

    <div class='row q-mx-md q-pt-sm no-wrap genre-bar-container'>
        <div v-for='(genre, i) in $1t.settings.quickTag.genres' :key='"genre"+i'>
            <div 
                @mouseenter="mouseOver = i"
                class='q-mx-sm text-subtitle1 clickable text-no-wrap text-grey-4' 
                :class='{"text-weight-bolder": isSelected(genre.genre) || mouseOver == i, "text-grey-7": !isSelected(genre.genre) && mouseOver != i}'
                @click='setGenre(genre.genre)'
            >
                {{genre.genre}}

                <!-- Subgenres -->
                <q-menu v-if='genre.subgenres' :value='mouseOver == i'>
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

<script>
export default {
    name: 'QuickTagGenreBar',
    data() {
        return {
            mouseOver: -1,
        }
    },
    methods: {
        isSelected(genre) {
            return this.$1t.quickTag.track.genres.includes(genre);
        },
        setGenre(genre) {
            this.$1t.quickTag.track.toggleGenre(genre);
        },
        // Stay open if subgenres are present
        onMouseLeave() {
            if (this.mouseOver == -1) return;
            if (this.$1t.settings.quickTag.genres[this.mouseOver].subgenres.length > 0) return;
            this.mouseOver = -1;
        }
    }
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