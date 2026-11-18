<template>
<div class='index-container' oncontextmenu="return false;">

    <!-- Background -->
    <img src='../assets/bg.jpg' class='bg-image' ref='bgImage'>

    <!-- Welcome -->
    <div class='flex flex-center column'>
        <img src="../assets/logo.svg" height='125' style='margin-top: 5%;'>
        
        
        <div class='q-pt-md text-center text-h6 text-grey-4 text-weight-medium'>{{ t('common.welcomeTagline') }}</div>
        
        <div class='q-pt-lg text-subtitle1 text-bold text-grey-4'>{{ t('common.version', { version: $1t.info.value.version }) }}</div>
        <span a class='doc-link text-subtitle2 clickable' @click='$1t.url("https://onetagger.github.io")'>OneTagger.GitHub.io</span>
        <div class='q-pt-sm q-pb-xs text-subtitle2 text-grey-4 text-center'>
            {{ t('common.developerCredits') }}<br>
            {{ t('common.auditCredits') }}
        </div>

        <div class="getting-started-cards flex flex-center column" style="min-height: 50px; width: 100%;">
            <div class="row justify-center items-stretch full-width">
                <div class="text-subtitle1 home-feature">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold text-subtitle2'>{{ t('common.autoTag') }}</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                {{ t('common.autoSummary') }}<br>
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("autotagger")'>{{ t('common.readMore') }}</q-btn>
                                
                            </div>
                        </q-card-section>
                    </q-card>                    
                </div>

                <div class="text-subtitle1 home-feature">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold text-subtitle2'>{{ t('common.audioFeatures') }}</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                {{ t('common.audioSummary') }}<br>
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("audiofeatures")'>{{ t('common.readMore') }}</q-btn>
                            </div>
                        </q-card-section>
                    </q-card>
                </div>
                
                <div class="text-subtitle1 home-feature">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold text-subtitle2'>{{ t('common.quickTag') }}</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                {{ t('common.quickSummary') }}<br>
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("quicktag")'>{{ t('common.readMore') }}</q-btn>
                            </div>                
                        </q-card-section>            
                    </q-card>                    
                </div>

                <div class="text-subtitle1 home-feature">
                    <q-card class='col-3 rounded-borders' style="max-width: 400px; margin-left: 25px; margin-right: 25px;">
                        <q-card-section>
                            <div class='col-3 text-primary text-bold text-subtitle2'>{{ t('common.autoRename') }}</div>
                            <div class ='col-3 text-grey-4 text-subtitle2 q-pt-sm' style='line-height: 24px'>
                                {{ t('common.renameSummary') }}<br>
                                <q-btn dense push color='primary' class='rounded-borders q-px-sm q-mt-md text-black text-weight-medium text-caption' @click='help("renamer")'>{{ t('common.readMore') }}</q-btn>
                            </div>
                        </q-card-section>
                    </q-card>
                </div>
                
            </div>

        </div>

        <div class="row bottom-line flex-center" style="width: 100%;">
            <div class='col text-center text-subtitle2 text-grey-4' style="max-width: 400px; margin-top: 20px; margin-left: 25px; margin-right: 25px;">
                {{ t('common.helpReadMore') }}
            </div>
            <div class='col text-center text-subtitle2 text-grey-4' style="max-width: 400px; margin-top: 20px; margin-left: 25px; margin-right: 25px;">
                <i18n-t keypath="common.bugReport" tag="span" scope="global">
                    <template #github><span class='tiny-logo text-bold clickable' @click='$1t.url("https://github.com/fantuz/onetagger/discussions")'>GitHub</span></template>
                </i18n-t>
            </div>
            <div class='col text-center text-subtitle2 text-grey-4' style="max-width: 400px; margin-top: 20px; margin-left: 25px; margin-right: 25px;">
                <i18n-t keypath="common.donation" tag="span" scope="global">
                    <template #paypal><span class='tiny-logo text-bold clickable' @click='$1t.url("https://paypal.me/marekkon5")'>PayPal</span></template>
                    <template #patreon><span class='tiny-logo text-bold clickable' @click='$1t.url("https://patreon.com/onetagger")'>Patreon</span></template>
                </i18n-t>
            </div>
        </div>
    </div>

</div>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });

import { get1t } from '../scripts/onetagger.js';

const $1t = get1t();
function help(route: string) {
    $1t.helpDialog.value.route = route;
    $1t.helpDialog.value.open = true;
}
</script>

<style>
.tiny-logo {
    opacity: 0.9;
    text-decoration-line: underline;
}
.tiny-logo:hover {
    opacity: 1.0;
    text-decoration-line: underline;    
}
.index-container {
    overflow: auto;
    min-height: calc(100vh - 59px);
}
.getting-started-cards {
    position: relative;
    margin-top: 30px;
    max-width: 100%;  
}
.bottom-line {
    position: static;
    padding-bottom: 35px;
}
.home-feature {
    flex: 1 1 260px;
    min-width: 0;
    max-width: 450px;
}
.home-feature .q-card {
    height: calc(100% - 24px);
    margin: 12px !important;
}
.bg-image {
    max-width: 100%;    
    position: fixed;    
    z-index: -100;    
    animation: anim 30s infinite;
}
.doc-link {
    color: var(--q-primary);
    text-decoration-line: underline;
}
.doc-link:hover {
    color: #f0f0f0;
    text-decoration-line: underline;
}

@keyframes anim {
    0% {
    transform: scale(1,1);
    }
    50% {
    transform: scale(1.05,1.05);
    }
    100% {
    transform: scale(1,1);
    }
}
</style>
