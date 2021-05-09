<template>
<div class='genre-bar'>

    <div class='row q-mx-md q-pt-sm no-wrap' @mouseleave="mouseOver = -1">
        <div v-for='(genre, i) in $1t.settings.quickTag.genres' :key='"genre"+i'>
            <div 
                @mouseenter="mouseOver = i"
                class='q-mx-sm text-subtitle1 clickable text-no-wrap text-grey-4' 
                :class='{"text-weight-bolder": isSelected(genre) || mouseOver == i, "text-grey-7": !isSelected(genre) && mouseOver != i}'
                @click='setGenre(genre)'
            >{{genre.genre}}</div>
        </div>
    </div>

</div>
</template>

<script>
export default {
    name: 'QuickTagGenreBar',
    data() {
        return {
            mouseOver: -1
        }
    },
    methods: {
        isSelected(genre) {
            return this.$1t.quickTag.track.genres.includes(genre.genre);
        },
        setGenre(genre) {
            this.$1t.quickTag.track.setGenre(genre.genre);
        }
    }
}
</script>

<style>
.genre-bar {
    height: 36px;
}
</style>