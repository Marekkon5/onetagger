<template>
<div class='genre-bar'>

    <div class='row q-mx-md q-pt-sm no-wrap' @mouseleave="menuOpen ? mouseOver = -1 : null">
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
                    <q-list @mouseenter='menuOpen = true' @mouseleave="menuMouseLeave">
                        <div v-for='(subgenre, j) in genre.subgenres' :key='"sg"+j'>
                            <q-item clickable @click='setGenre(subgenre)'>
                                <q-item-section>
                                    <div class='row'>
                                        <q-icon name='mdi-check' class='q-pr-sm q-pt-xs' v-if='isSelected(subgenre)'></q-icon>
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
            menuOpen: false
        }
    },
    methods: {
        isSelected(genre) {
            return this.$1t.quickTag.track.genres.includes(genre);
        },
        setGenre(genre) {
            this.$1t.quickTag.track.toggleGenre(genre);
        },
        // Hide menu
        menuMouseLeave() {
            this.menuOpen = false;
            this.mouseOver = -1;
        }
    }
}
</script>

<style>
.genre-bar {
    height: 36px;
}
</style>