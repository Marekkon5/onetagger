<template>
  <div>
    <div class="row justify-around">
      <div v-for="(icon, i) in icons" :key="i" class="col">
        <img
          :src="icon.toString()"
          class="dj-icon clickable"
          @click="$1t.url(urls[i])"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { get1t } from "../scripts/onetagger.js";

const $1t = get1t();
const icons = [
  new URL("../assets/apps/rekordbox.png", import.meta.url),
  new URL("../assets/apps/serato.png", import.meta.url),
  new URL("../assets/apps/traktor.png", import.meta.url),
  new URL("../assets/apps/virtualdj.png", import.meta.url),
  new URL("../assets/apps/djaypro.png", import.meta.url),
  new URL("../assets/apps/djuced.png", import.meta.url),
  new URL("../assets/apps/engineprime.png", import.meta.url),
  new URL("../assets/apps/crossdj.png", import.meta.url),
  new URL("../assets/apps/mixxx.png", import.meta.url),
  new URL("../assets/apps/dex.png", import.meta.url),
];
const urls = [
  "https://docs.google.com/spreadsheets/d/1ALyk1WRTR_-FLq7kntdX2e4jM9ZARvieJAHz9-pjRQ8/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/1QAnFmRMhcNrM9yARNrjyIP5wif5h-36eSGMcXarXNsk/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/1Atb26winoLgb_w_oX12npmZzqx3IVkCbkzn9Ug6R6B8/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/12oVBNt5Zs8fC9qId3M9jkRT9DYf3hdaaet0W1IQIuB4/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/1mhmfq7D2hh-LAy7Had85gDInZZlzJV42eSgLta6fTzg/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/1iHqxDiaB05r1apvRpv9wtD4wkIRfW0-hWJFLqMg-WVA/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/14WNpiEBtgkSqCAVcNlxU2gHV4Lok5n-DUx1shx_BKDI/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/13cZMox_eMvqvmtxICuI19hbCNBCrCd-fso9_ckXpKw0/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/15AgfpSLxLKxvJwd4DRM1JqIg09bpOKO2LUdWwDYSYqs/edit?usp=sharing",
  "https://docs.google.com/spreadsheets/d/1EezlJ3R9VJYyxYvTc4G1xE4XwnV3o3wFjYwZJWSRq5E/edit?usp=sharing",
];
</script>

<style>
.dj-icon {
  opacity: 0.7;
}

.dj-icon:hover {
  opacity: 1;
}
</style>
