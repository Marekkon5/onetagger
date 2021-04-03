<template>
<div class='genre-bar'>

    <div class='row q-mx-md q-pt-sm no-wrap'>
        <div v-for='(genre, i) in $1t.settings.quickTag.genres' :key='"genre"+i'>
            <div 
                class='q-mx-sm text-subtitle1 genre-text text-no-wrap' 
                :class='{"text-weight-bolder": isSelected(genre)}'
                @click='setGenre(genre)'
            >{{genre.genre}}</div>
        </div>
    </div>

</div>
</template>

<script>
export default {
    name: 'QuickTagGenreBar',
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
    height: 32px;
}
.genre-text {
    cursor: pointer;
}
</style>