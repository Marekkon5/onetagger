<template>
<div class='text-center'>

    <!-- Platforms -->
    <div v-for='(platform, i) in platforms' :key='"p"+i' class='q-mb-md'>
        <div class='text-subtitle2 text-bold text-uppercase text-grey-4' style='margin-top: 1px; margin-bottom: -16px'>{{platform.name}}</div>

        <!-- Custom options -->
        <div v-for='(option, j) in platform.customOptions.options' :key='i+"o"+j' class='q-mt-lg'>

            <!-- Slider -->
            <div v-if='option.value.type == "number"'>
                <q-chip text-color='black' color='primary'>
                    {{ optionText(platform, option, 'label') }}: {{$1t.config.value.custom[platform.id][option.id]}}
                    <q-tooltip v-if='option.tooltip'>
                        {{ optionText(platform, option, 'tooltip') }}
                    </q-tooltip>
                </q-chip>
                <div class='row justify-center' :style='"margin-bottom: -12px"'>
                    <q-slider
                        label-text-color='black'
                        v-model='$1t.config.value.custom[platform.id][option.id]'
                        :min='option.value.min'
                        :max='option.value.max'
                        :step='option.value.step'
                        label
                        class='slider col-10'
                    ></q-slider>
                </div>
            </div>

            <!-- Custom tag type -->
            <div v-if='option.value.type == "tag"'>
                <div class='text-body1'>{{ optionText(platform, option, 'label') }}</div>
                <TagFields v-model='$1t.config.value.custom[platform.id][option.id]' class='input'></TagFields>
            </div>

            <!-- Select -->
            <div v-if='option.value.type == "option"'>
                <div class='text-subtitle2 text-grey-6 q-mb-md' v-if='option.tooltip'>
                    {{ optionText(platform, option, 'tooltip') }}
                </div>
                <q-select
                    dark
                    standout='text-grey-4 bg-dark'
                    v-model='$1t.config.value.custom[platform.id][option.id]'
                    :options='option.value.values'
                    class='select'
                    :label="optionText(platform, option, 'label')"
                    popup-content-class='no-shadow'
                ></q-select>
            </div>

            <!-- Input -->
            <div v-if='option.value.type == "string"'>
                <div class='text-subtitle2 text-grey-6 q-mb-sm' v-if='option.tooltip'>
                    <span v-if="translatedTooltip(platform, option) !== undefined">{{ translatedTooltip(platform, option) }}</span>
                    <span v-else v-html='option.tooltip'></span>
                </div>
                <q-input
                    dark
                    standout='text-grey-4 bg-dark'
                    v-model='$1t.config.value.custom[platform.id][option.id]'
                    class='input' 
                    :label="optionText(platform, option, 'label')"
                    :type='option.value.hidden ? "password" : "text"'
                ></q-input>
            </div>

            <!-- Switch -->
            <div v-if='option.value.type == "boolean"'>
                <div>
                    <q-toggle 
                        style='margin-bottom: -14px;' 
                        v-model='$1t.config.value.custom[platform.id][option.id]' 
                        :label="optionText(platform, option, 'label')"
                    ><br></q-toggle>
                </div>
            </div>

            <!-- Button -->
            <div v-if='option.value.type == "button"'>
                <div>
                    <div class='text-subtitle2 text-grey-6 q-mb-md' v-if='option.tooltip'>
                        <span v-if="translatedTooltip(platform, option) !== undefined">{{ translatedTooltip(platform, option) }}</span>
                        <span v-else v-html='option.tooltip'></span>
                    </div>
                    <q-btn color='primary' outline :label="optionText(platform, option, 'label')" @click='$1t.send("configCallback", { platform: platform.id, config: $1t.config.value.custom[platform.id], id: option.id })'></q-btn>
                </div>
            </div>

        </div>

        <!-- Separator -->
        <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 40px; margin-bottom: 35px"' inset color="dark"/>
    </div>

    <!-- No settings available -->
    <div v-if='platforms.length == 0 && !spotify'>
        <div class='text-subtitle2 q-my-md text-grey-4' v-if='!$1t.settings.value.autoTaggerSinglePage'>
            {{ t('autotag.noPlatformSettings') }}
        </div>
    </div>

    <!-- Spotify -->
    <div v-if='spotify'>  
        <div class='text-subtitle2 text-bold text-uppercase text-grey-4' style='margin-top: 35px; margin-bottom: -16px'>Spotify</div>
        <div class='justify-center' style='max-width: 836px; margin: auto;'>
            <SpotifyLogin v-if='!$1t.spotify.value.authorized'></SpotifyLogin>
        </div>
        <div v-if='$1t.spotify.value.authorized'>
            <div class='q-mt-lg text-h7 text-primary'>{{ t('autotag.spotifyLoggedIn') }}</div>
        </div>
        <br>
    </div>


</div>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });
import TagFields from './TagFields.vue';
import SpotifyLogin from './SpotifyLogin.vue';
import { get1t } from '../scripts/onetagger';
import { computed } from 'vue';
import { providerOptionText } from '../i18n/providers';

const $1t = get1t();
const spotify = computed(() => $1t.config.value.platforms.includes('spotify'));
const platforms = computed(() => $1t.info.value.platforms
    .filter((p) => $1t.config.value.platforms.includes(p.platform.id) && p.platform.customOptions.options.length > 0)
    .map((p) => ({ ...p.platform, builtIn: p.builtIn })));

type ProviderIdentity = { builtIn: boolean; id: string };
type ProviderOption = { id: string; label: string; tooltip?: string };

function optionText(platform: ProviderIdentity, option: ProviderOption, field: 'label' | 'tooltip') {
    return providerOptionText(platform.builtIn, platform.id, option.id, field) ?? option[field];
}

function translatedTooltip(platform: ProviderIdentity, option: ProviderOption) {
    return providerOptionText(platform.builtIn, platform.id, option.id, 'tooltip');
}

</script>
