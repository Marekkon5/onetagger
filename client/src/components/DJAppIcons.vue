<template>
<div>

    <div class='row justify-around'>
        <div v-for='(icon, i) in icons' :key='i' class='col'>
            <img 
                :src='require("../assets/apps/" + icon)' 
                class='dj-icon clickable' 
                @click='$1t.url(urls[i])'
            />
        </div>
    </div>

</div>
</template>

<script>
export default {
    name: 'DJAppIcons',
    data() {
        return {
            icons: [
                'rekordbox.png', 'serato.png', 'traktor.png', 'virtualdj.png', 'djaypro.png',
                'djuced.png', 'engineprime.png', 'crossdj.png', 'mixxx.png', 'dex.png'
            ],
            urls: [
                'https://docs.google.com/spreadsheets/d/1ALyk1WRTR_-FLq7kntdX2e4jM9ZARvieJAHz9-pjRQ8/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/1QAnFmRMhcNrM9yARNrjyIP5wif5h-36eSGMcXarXNsk/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/1Atb26winoLgb_w_oX12npmZzqx3IVkCbkzn9Ug6R6B8/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/12oVBNt5Zs8fC9qId3M9jkRT9DYf3hdaaet0W1IQIuB4/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/1mhmfq7D2hh-LAy7Had85gDInZZlzJV42eSgLta6fTzg/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/1iHqxDiaB05r1apvRpv9wtD4wkIRfW0-hWJFLqMg-WVA/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/14WNpiEBtgkSqCAVcNlxU2gHV4Lok5n-DUx1shx_BKDI/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/13cZMox_eMvqvmtxICuI19hbCNBCrCd-fso9_ckXpKw0/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/15AgfpSLxLKxvJwd4DRM1JqIg09bpOKO2LUdWwDYSYqs/edit?usp=sharing',
                'https://docs.google.com/spreadsheets/d/1EezlJ3R9VJYyxYvTc4G1xE4XwnV3o3wFjYwZJWSRq5E/edit?usp=sharing'
            ]
        }
    }
};
</script>

<style>
.dj-icon {
    opacity: 0.7;
}

.dj-icon:hover {
    opacity: 1.0;
}
</style>