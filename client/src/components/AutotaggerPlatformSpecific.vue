<template>
<div class='text-center'>

    <!-- Platforms -->
    <div v-for='(platform, i) in platforms' :key='"p"+i' class='q-mt-md'>
        <div class='text-h6 text-grey-4 q-mb-md' style='margin-top: 1px;'>{{platform.name}}</div>

        <!-- Custom options -->
        <div v-for='(option, j) in platform.customOptions.options' :key='i+"o"+j' class='q-mb-md'>

            <!-- Slider -->
            <div v-if='option.value.type == "number"'>
                <q-chip text-color='black' color='primary'>
                    {{option.label}}: {{option.value.value}}
                    <q-tooltip v-if='option.tooltip'>
                        {{option.tooltip}}
                    </q-tooltip>
                </q-chip>
                <div class='row justify-center'>
                    <q-slider
                        label-text-color='black'
                        v-model='option.value.value'
                        :min='option.value.min'
                        :max='option.value.max'
                        :step='option.value.step'
                        label
                        class='slider q-my-sm q-pb-lg col-10'
                    ></q-slider>
                </div>
            </div>

            <!-- Custom tag type -->
            <div v-if='option.value.type == "tag"'>
                <div class='text-body1'>{{option.label}}</div>
                <TagFields v-model='(option.value.value as any)' class='input'></TagFields>
            </div>

            <!-- Select -->
            <div v-if='option.value.type == "option"'>
                <div class='text-subtitle2 text-grey-6 q-mb-md' v-if='option.tooltip'>
                    {{option.tooltip}}
                </div>
                <q-select
                    dark
                    standout='text-grey-4 bg-dark'
                    v-model='option.value.value'
                    :options='option.value.values'
                    class='select'
                    :label='option.label'
                ></q-select>
            </div>

            <!-- Input -->
            <div v-if='option.value.type == "string"'>
                <div class='text-subtitle2 text-grey-6 q-mb-md' v-if='option.tooltip' v-html='option.tooltip'></div>
                <q-input
                    dark
                    standout='text-grey-4 bg-dark'
                    v-model='option.value.value'
                    class='input' 
                    :label='option.label'
                    :type='option.value.hidden ? "password" : "text"'
                ></q-input>
            </div>

            <!-- Switch -->
            <div v-if='option.value.type == "boolean"'>
                <div>
                    <q-toggle 
                        style='margin-bottom: 10px;' 
                        v-model='option.value.value' 
                        :label="option.label"
                    ><br></q-toggle>
                </div>
            </div>

        </div>

        <!-- Separator -->
        <q-separator class='q-mx-auto q-mt-lg custom-separator' inset color="dark"/>
    </div>

    <!-- No settings available -->
    <div v-if='platforms.length == 0 && !spotify'>
        <div class='text-subtitle1 q-my-md text-grey-4' v-if='!$1t.settings.value.autoTaggerSinglePage'>
            No platform specific settings available for the selected platform(s)
        </div>
    </div>

    <!-- Spotify -->
    <div v-if='spotify'>
        <div class='text-h6 q-mt-lg text-grey-4 custom-margin-1'>Spotify</div>
        <div class='justify-center' style='max-width: 836px; margin: auto;'>
            <SpotifyLogin v-if='!$1t.spotify.value.authorized'></SpotifyLogin>
        </div>
        <div v-if='$1t.spotify.value.authorized'>
            <div class='q-mt-xs text-h7 text-primary'>You are successfully logged in to Spotify</div>
        </div>
        <br>
    </div>


</div>
</template>

<script lang='ts' setup>
import TagFields from './TagFields.vue';
import SpotifyLogin from './SpotifyLogin.vue';
import { get1t } from '../scripts/onetagger';
import { computed } from 'vue';

const $1t = get1t();
const spotify = computed(() => $1t.config.value.platforms.includes('spotify'));
const platforms = computed(() => $1t.info.value.platforms
    .filter((p) => $1t.config.value.platforms.includes(p.id) && p.platform.customOptions.options.length > 0)
    .map((p) => p.platform));
</script>

<style>
.custom-separator {
    max-width: 550px;
    margin: auto;
}
.custom-margin-1 {
    margin-top: 34px !important;
}
</style>