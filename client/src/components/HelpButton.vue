<template>
<div>

    <!-- Button -->
    <div class='help-button-container' @click='show = true' v-if='showButton'>
        <q-btn color='primary' class='q-px-md help-button text-bold' style='margin-right: -10px'>
            <q-icon name='mdi-help-circle-outline' class='q-pr-xs' size='xs'></q-icon>
            {{ t('common.help') }}
        </q-btn>
    </div>

    <!-- Dialog -->
    <q-dialog v-model='show' @hide='hide'>
        <q-card class='help-card text-left'>

            <!-- Page controls -->
            <q-btn 
                class='help-page-icon clickable help-page-icon-forward'
                flat
                round                
                @click='page += 1' :aria-label="t('common.next')"
                v-if='page < (pages-1)'
                ><q-icon name='mdi-chevron-right' size='lg'></q-icon>
            </q-btn>
            <q-btn 
                class='help-page-icon clickable help-page-icon-back'
                flat
                round
                @click='page -= 1' :aria-label="t('common.previous')"
                v-if='page > 0'
                ><q-icon name='mdi-chevron-left' size='lg'></q-icon>
            </q-btn>

            <!-- Auto tagger -->
            <div v-if='route == "autotagger"'>
                <q-card-section class='q-pa-lg'>
                    <div class='q-mt-md text-subtitle2 text-center text-primary text-bold text-uppercase'>{{ t('common.helpAutoTag') }}</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-black'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>                  

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <i18n-t keypath="help.autoAutomaticallyTagYourLocalAudioFilesFrom" tag="div" scope="global" class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px; white-space: pre-line'>
                            <template #artist><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.artist') }}</span></q-badge></template>
                            <template #title><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.title') }}</span></q-badge></template>
                            <template #shazam><b>Shazam</b></template>
                            <template #videoTutorial><span @click='$1t.url("https://youtu.be/BfLtQY8u794")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('common.videoTutorial') }}</span></template>
                        </i18n-t>
                        <div class='row' style='width: 94%; margin-left:5%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('common.howItWorks') }}</div>
                        <i18n-t keypath="help.autoItReadsTheTagFromYourLocal" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #artist><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.artist') }}</span></q-badge></template>
                            <template #title><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.title') }}</span></q-badge></template>
                            <template #shazam><span @click='$1t.url("https://shazam.com")' class='clickable doc-link'>Shazam</span></template>
                            <template #beatport><span @click='$1t.url("https://beatport.com")' class='clickable doc-link'>Beatport</span></template>
                            <template #junoDownload><span @click='$1t.url("https://junodownload.com")' class='clickable doc-link'>Juno Download</span></template>
                            <template #traxsource><span @click='$1t.url("https://traxsource.com")' class='clickable doc-link'>Traxsource</span></template>
                            <template #beatsource><span @click='$1t.url("https://beatsource.com")' class='clickable doc-link'>Beatsource</span></template>
                            <template #itunes><span @click='$1t.url("https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/iTuneSearchAPI/index.html")' class='clickable doc-link'>iTunes</span></template>
                            <template #musicbrainz><span @click='$1t.url("https://musicbrainz.org")' class='clickable doc-link'>MusicBrainz</span></template>
                            <template #discogs><span @click='$1t.url("https://discogs.com")' class='clickable doc-link'>Discogs</span></template>
                            <template #spotify><span @click='$1t.url("https://spotify.com")' class='clickable doc-link'>Spotify</span></template>
                        </i18n-t>
                        </div>
                        <div class='row' style='width: 94%; margin-left:5%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('common.whatItDoes') }}</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>{{ t('help.autoFetchesMetadataAutomaticallyWritesTagsToYour') }}</div>
                        <div class='row text-subtitle2 text-grey-4 q-pt-sm' style='width: 100%; margin-left: 16.8%; line-height: 24px'>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.albumArtCover') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoResolutionIsPlatformDependent') }}</q-tooltip>
                                </q-icon>                            
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.title') }}</span></q-badge> , <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.trackVersion') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatportBeatsourceTraxsource') }}</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>ISRC</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatportBeatsourceMusicBrainzSpotify') }}</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.catalogNumber') }}</span></q-badge></div>
                            
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.album') }}</span></q-badge></div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>BPM</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatportBeatsourceJunoDownloadTraxsource') }}</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.genre') }}</span></q-badge> , <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.style') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoStyleIsAvailableFromDiscogsBandcampSubgenre') }}</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.trackNumber') }}</span></q-badge></div>

                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.albumArtist') }}</span></q-badge> , <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.artist') }}</span></q-badge></div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.key') }}</span></q-badge>
                            <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatportBeatsourceTraxsource2') }}</q-tooltip>
                                </q-icon>
                            </div>
                                
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.label') }}</span></q-badge></div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.duration') }}</span></q-badge></div>
                            
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.remixers') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatportBeatsource') }}</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.publishDate') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatport') }}</q-tooltip>
                                </q-icon><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.releaseDate') }}</span></q-badge></div>
                            
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.releaseId') }}</span></q-badge> , <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.trackId') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoAvailableFromBeatportBeatsourceTraxsource3') }}</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>URL</span></q-badge> , <q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.other') }}</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>{{ t('help.autoSpecificTagsOnlyForSomePlatformsBeatport') }}</q-tooltip>
                                </q-icon>
                            </div>
                            </div>
                        </div>
                        
                        <div class='bg-darker text-subtitle2 text-grey-5 q-my-md q-py-md text-center' style='line-height: 24px'>
                            <i18n-t keypath="help.autoNotAllPlatformsHaveTheSameTags" tag="span" scope="global" class='q-pl-md' style="white-space: pre-line">
                                <template #helpIcon><q-icon name='mdi-help-circle-outline' style='margin-bottom: -2px;' class='q-pb-xs text-grey-6'></q-icon></template>
                            </i18n-t> <br>
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='row' style='width: 86%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>{{ t('common.stepGuide') }}</div>
                        <div class='col-10 q-mt-md text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>1.<span class='text-number text-bold text-grey-10'>_</span> </span><span class='q-pl-sm'>{{ t('help.autoCheckTheBoxToFetchFromParticular') }}</span> <br>
                            <span class='text-number text-bold text-grey-6'>2. </span><i18n-t keypath="help.autoDragDropTheCardsToReorderFallback" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #next><q-badge color='primary' class='text-uppercase text-black'>{{ t('common.next') }}</q-badge></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>3. </span><i18n-t keypath="help.autoDragDropFolderCopyPastePathDirectly" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #browseIcon><q-icon name='mdi-open-in-app' class='q-mb-xs'></q-icon></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-10'>_</span><span class='q-pl-md'>{{ t('help.autoOrDragDropPlaylistOntoTheCard') }}</span> <br>
                            <span class='text-number text-bold text-grey-6'>4. </span><i18n-t keypath="help.autoCheckTheBoxToFetchStatedTag" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #next><q-badge color='primary' class='text-uppercase text-black'>{{ t('common.next') }}</q-badge></template>
                            </i18n-t> <br>
                        </div>
                        </div>
                        <div class='row' style='width: 86%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Beatport</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>5. </span><i18n-t keypath="help.autoSelectResolutionXIsNativeAboveWill" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #albumArt><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.albumArt') }}</span></q-badge></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>6. </span><span class='q-pl-sm'>{{ t('help.autoDragSliderToSetAmountOfSearch') }}</span> <br>
                        </div>
                        </div>
                        <div class='row' style='width: 86%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Discogs</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>7. </span><i18n-t keypath="help.autoEnterTokenToObtainTokenCreateA" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #discogs><span @click='$1t.url("https://discogs.com")' class='clickable doc-link'>Discogs </span></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-10'>_</span><i18n-t keypath="help.autoGoToGenerateToken" tag="span" scope="global" class='q-pl-md' style="white-space: pre-line">
                                <template #discogsComSettingsDevelopers><span @click='$1t.url("https://www.discogs.com/settings/developers")' class='clickable doc-link'>Discogs.com/settings/developers</span></template>
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-10'>_</span><i18n-t keypath="help.autoSelectItCopyPasteTheCurrentToken" tag="span" scope="global" class='q-pl-md' style="white-space: pre-line">
                                <template #videoDemo><span @click='$1t.url("https://youtu.be/IvAiMkfdLCw")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('common.videoDemo') }}</span></template>
                            </i18n-t><p></p>
                            <span class='text-number text-bold text-grey-6'>8. </span><i18n-t keypath="help.autoSelectTagToFetchBothIfIt" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #genre><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.genre') }}</span></q-badge></template>
                                <template #style><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.style') }}</span></q-badge></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>9. </span><i18n-t keypath="help.autoDragSliderToSetAmountOfAlbum" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #next><q-badge color='primary' class='text-uppercase text-black'>{{ t('common.next') }}</q-badge></template>
                            </i18n-t><br>
                        </div>
                        </div>
                        <div class='row' style='width: 86%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('common.advanced') }}</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>10.</span><i18n-t keypath="help.autoEnableToggleToActivateStatedOptions" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #toggleIcon><q-icon name='mdi-toggle-switch' class='text-grey-4'></q-icon></template>
                            </i18n-t><br>
                            <span class='text-number text-bold text-grey-10'>_&nbsp;</span><i18n-t keypath="help.autoHoverOverTheHelpIconsForMore" tag="span" scope="global" class='q-pl-md' style="white-space: pre-line">
                                <template #helpIcon><q-icon name='mdi-help-circle-outline' class='q-pb-xs text-grey-6'></q-icon></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-10'>_&nbsp;</span><i18n-t keypath="help.autoTheStartIconToProcess" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #startIcon><q-icon name='mdi-play-circle' class='q-mb-xs text-primary' ></q-icon></template>
                            </i18n-t> <p><br></p>
                        </div>
                        </div>
                        

                    </div>

                </q-card-section>
            </div>

            <!-- Audio features -->
            <div v-if='route == "audiofeatures"'>
                <q-card-section class='q-pa-lg'>
                    <div class='q-mt-md text-subtitle2 text-primary text-bold text-uppercase text-center'>{{ t('common.helpAudio') }}</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-black'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <i18n-t keypath="help.audioAutomaticallyTagYourLocalAudioFilesWith" tag="div" scope="global" class='text-subtitle2 text-grey-4 q-mt-md text-center' style="white-space: pre-line">
                            <template #spotify><b>Spotify</b></template>
                            <template #value><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>ISRC</span></q-badge></template>
                            <template #videoTutorial><span @click='$1t.url("https://youtu.be/bIBxDH1CltI")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('common.videoTutorial') }}</span></template>
                        </i18n-t>
                        <div class='row' style='width: 88%; margin-left:12%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('common.howItWorks') }}</div>
                        <i18n-t keypath="help.audioFollowTheSetupDescribedOnTheEntrance" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #audioFeatures><span class='text-subtitle3 text-bold'>{{ t('common.audioFeatures') }}</span></template>
                            <template #videoDemo><span @click='$1t.url("https://youtu.be/i0q5qWQSH9Y")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('common.videoDemo') }}</span></template>
                            <template #artist><i18n-t keypath="help.audioSelectAFolderWithTracksYouWant" tag="div" scope="global" class='col-10 text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                                <template #isrc><span class='clickable doc-link' @click='$1t.url(isrcWiki)'>ISRC</span></template>
                                <template #value><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>ISRC</span></q-badge></template>
                                <template #artist><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.artist') }}</span></q-badge></template>
                                <template #title><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.title') }}</span></q-badge></template>
                                <template #value2><q-badge color='grey-9'><span class='textnumber text-grey-2'>0 - 100</span></q-badge></template>
                            </i18n-t></template>
                        </i18n-t>
                        </div>
                        <div class='row' style='width: 88%; margin-left:12%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('help.audioAudioFeatures') }}</div>
                        <div class='col-10 q-mt-lg text-subtitle text-grey-4'>
                                
                        <div class='row text-subtitle2 text-grey-4' style='width: 70%; margin-left: 0%; line-height: 24px'>
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.acousticness') }}</span></q-badge></div>     <div class='text-right text-grey-3 col-8'>#acoustic <span class='text-grey-6'>&nbsp;·&nbsp;</span> #electronic</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.danceability') }}</span></q-badge></div>     <div class='text-right text-grey-3 col-8'>#dance-low <span class='text-grey-6'>&nbsp;·&nbsp;</span> #dance-med <span class='text-grey-6'>&nbsp;·&nbsp;</span> #dance-high</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.energy') }}</span></q-badge></div>           <div class='text-right text-grey-3 col-8'>#energy-low <span class='text-grey-6'>&nbsp;·&nbsp;</span> #energy-med <span class='text-grey-6'>&nbsp;·&nbsp;</span> #energy-high</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.instrumentalness') }}</span></q-badge></div> <div class='text-right text-grey-3 col-8'>#vocal-low <span class='text-grey-6'>&nbsp;·&nbsp;</span> #vocal-med <span class='text-grey-6'>&nbsp;·&nbsp;</span> #vocal-high</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.liveness') }}</span></q-badge></div>         <div class='text-right text-grey-3 col-8'>#live <span class='text-grey-6'>&nbsp;·&nbsp;</span> #recording</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.speechiness') }}</span></q-badge></div>      <div class='text-right text-grey-3 col-8'>#speech <span class='text-grey-6'>&nbsp;·&nbsp;</span> #music</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.valence') }}</span></q-badge></div>          <div class='text-right text-grey-3 col-8'>#positive <span class='text-grey-6'>{{ t('help.audioHappy') }}</span> <span class='text-grey-6'>&nbsp;·&nbsp;</span> #balanced <span class='text-grey-6'>&nbsp;·&nbsp;</span> #negative <span class='text-grey-6'>{{ t('help.audioSadAngry') }}</span></div>
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.popularity') }}</span></q-badge></div>       <div class='text-right text-grey-3 col-8'>#unpopular <span class='text-grey-6'>&nbsp;·&nbsp;</span> #popular</div> 
                        </div>
                        </div>
                        </div>
                        <i18n-t keypath="help.audioForAMoreInDepthDescriptionCheck" tag="div" scope="global" class='bg-darker text-subtitle2 text-grey-5 q-mt-md q-mb-lg q-px-md q-py-md q-my-md text-center' style='line-height: 24px; white-space: pre-line'>
                            <template #spotifySOfficialAudio><span class='clickable doc-link' @click='$1t.url("https://developer.spotify.com/documentation/web-api/reference/#/operations/get-several-audio-features")'>{{ t('help.audioSpotifySOfficialAudioFeatures') }}</span></template>
                            <template #toLookUpAudio><i18n-t keypath="help.audioToLookUpAudioFeaturesOnlineFor" tag="div" scope="global" class='bg-darker text-subtitle2 text-grey-5 text-center q-pt-xs' style='line-height: 24px; white-space: pre-line'>
                                <template #songdataIo><span class='clickable doc-link' @click='$1t.url("https://songdata.io")'>{{ t('help.audioSongDataIo') }}</span></template>
                                <template #keysDeterminedBySpotify><span class='clickable doc-link' @click='$1t.url("https://www.reddit.com/r/DJs/comments/m3q97z/key_detection_comparison_spotify_vs_tunebat_vs/")'>{{ t('help.audioKeysDeterminedBySpotify') }}</span></template>
                            </i18n-t></template>
                        </i18n-t>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='row' style='width: 100%; margin-left:1%; margin-top:2px'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md text-right'>{{ t('common.prominentTag') }}</div>
                        <i18n-t keypath="help.audioYouCanCustomizeTheThresholdSoIt" tag="div" scope="global" class='col-10 q-mt-md text-subtitle2 text-grey-4 q-pl-sm' style='line-height: 24px; white-space: pre-line'>
                            <template #value><q-badge color='grey-9'><span class='textnumber'>0-100</span></q-badge></template>
                            <template #value2><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('help.audioCustom') }}</span></q-badge></template>
                        </i18n-t>
                        </div>

                        <div class='row' style='width: 96%; margin-left:4%; margin-top:2px'>
                        <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.acousticness') }}</span></q-badge>
                                    <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 *</span>
                                    </q-badge>
                                    <br>
                                    <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>90 - 100</span>
                                    </q-badge>                        
                        </div>
                        <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#electronic <br>
                                    <span class='text-grey-10'>-</span>#acoustic
                                    </span>
                            </div>
                            
                            
                                <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.danceability') }}</span></q-badge>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 - 20</span>
                                    </q-badge>
                                    <br>
                                    <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>21 - 79</span>
                                    </q-badge>  
                                    <br>
                                    <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>80 - 100</span>
                                    </q-badge>
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#dance-low <br>
                                    <span class='text-grey-10'>-</span>#dance-med <br>
                                    <span class='text-grey-10'>-</span>#dance-high
                                    </span>
                            </div>
                            
                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.energy') }}</span></q-badge>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 - 20</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>21 - 79</span>
                                </q-badge>  
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>80 - 100</span>
                                </q-badge>
                                </div>
                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                        <span class='text-subtitle2 text-grey-4'>
                                        <span class='text-grey-10'>-</span>#energy-low <br>
                                        <span class='text-grey-10'>-</span>#energy-med <br>
                                        <span class='text-grey-10'>-</span>#energy-high
                                        </span>
                                </div>

                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('help.audioInstrumental') }}</span></q-badge>
                            <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 - 50</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>51 - 89</span>
                                </q-badge>  
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>90 - 100</span>
                                </q-badge>
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#vocal-high <br>
                                    <span class='text-grey-10'>-</span>#vocal-med <br>
                                    <span class='text-grey-10'>-</span>#vocal-low
                                    </span>
                            </div>

                            <div class='row' style='width: 100%; margin-left:0%'>
                                <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.liveness') }}</span></q-badge><br>
                            <q-badge color='grey-9'>
                            
                                        <span class='textnumber text-grey-2'>0 *</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>80 - 100</span>
                                </q-badge>                                  
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#recording <br>
                                    <span class='text-grey-10'>-</span>#live
                                    </span>                                
                            </div>
                            

                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.speechiness') }}</span></q-badge>
                            <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 *</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>70 - 100</span>
                                </q-badge>                                  
                                </div>    
                                
                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#music<br>
                                    <span class='text-grey-10'>-</span>#speech
                                    </span>                                
                            </div>
                            
                            
                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.valence') }}</span></q-badge>
                            <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 - 15</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>16 - 84</span>
                                </q-badge>  
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>85 - 100</span>
                                </q-badge>
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#negative <br>
                                    <span class='text-grey-10'>-</span>#balanced <br>
                                    <span class='text-grey-10'>-</span>#positive
                                    </span>                                
                            </div>

                        <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.popularity') }}</span></q-badge>
                            <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>0 *</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-9'>
                                        <span class='textnumber text-grey-2'>80 - 100</span>
                                </q-badge>  
                                    <br>                                
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-6'>{{ t('common.thresholdDefault') }}</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                        <span class='text-grey-10'>-</span>#unpopular <br>
                                        <span class='text-grey-10'>-</span>#popular
                                    </span>                                
                            </div>

                        </div>
                        </div>
                            <i18n-t keypath="help.audioWhenThresholdIsItWonTWrite" tag="div" scope="global" class='q-pb-xs text-subtitle3 q-my-md text-grey-5 text-center' style='line-height: 36px; white-space: pre-line'>
                                <template #value><q-badge color='grey-9'><span class='textnumber text-grey-2'>0 *</span></q-badge></template>
                            </i18n-t>
                        
                        <div class='bg-darker text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md text-center text-caption'>
                            <DJAppIcons></DJAppIcons>                                                                           
                        </div>
                        <i18n-t keypath="help.audioWhichTagFramesCanBeReadBy" tag="div" scope="global" class='text-subtitle3 text-grey-5 text-center text-caption' style='line-height: 24px; white-space: pre-line'>
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                        </i18n-t>
                    </div>

                </q-card-section>
            </div>


            <!-- Quicktag -->
            <div v-if='route == "quicktag"'>
                <q-card-section class='q-pa-lg'>
                    <div class='q-mt-md text-subtitle2 text-primary text-bold text-uppercase text-center'>{{ t('common.helpQuickTag') }}</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-black'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <i18n-t keypath="help.quickManuallyTagYourLocalAudioFilesQuickly" tag="div" scope="global" class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px; white-space: pre-line'>
                            <template #energy><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.energy') }}</span></q-badge></template>
                            <template #mood><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.mood') }}</span></q-badge></template>
                            <template #genre><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.genre') }}</span></q-badge></template>
                            <template #videoTutorial><span @click='$1t.url("https://youtu.be/kiPCRNBaVEM")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('common.videoTutorial') }}</span></template>
                        </i18n-t>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('common.howItWorks') }}</div>
                        <i18n-t keypath="help.quickSetThePathToAFolderWith" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #energy><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.energy') }}</span></q-badge></template>
                            <template #mood><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.mood') }}</span></q-badge></template>
                            <template #genre><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.genre') }}</span></q-badge></template>
                            <template #keys><q-badge outline color='grey-8'><span class='text-uppercase text-grey-3'>{{ t('common.keys') }}</span></q-badge></template>
                            <template #cursorIcon><q-icon name='mdi-cursor-default-outline'></q-icon></template>
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                        </i18n-t>                        
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%; margin-top: -1px;'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>{{ t('common.stepGuide') }}</div>
                        <div class='col-10 q-mt-lg q-mb-xs text-subtitle2 text-grey-4' style='line-height: 24px'>  
                            <span class='text-number text-bold text-grey-6'>1. <span class='text-number text-bold text-grey-10'>.</span></span><i18n-t keypath="help.quickTheSettingsIcon" tag="span" scope="global" style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #settingsIcon><q-icon name='mdi-cog'></q-icon></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>2. </span><span class='q-pl-sm'>{{ t('help.quickSetPathToAFolderWhichYou') }}</span> <br>
                            <span class='text-number text-bold text-grey-6'>3. </span><i18n-t keypath="help.quickPredefineEnergyLevelToWriteToEither" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #rating><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.rating') }}</span></q-badge></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>4. </span><i18n-t keypath="help.quickPredefineAssignAColorOtherCharacteristics" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #mood><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.mood') }}</span></q-badge></template>
                                <template #genre><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.genre') }}</span></q-badge></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>5. </span><span class='q-pl-sm'>{{ t('help.quickKeyBindThemAllToACharacter') }}</span> <br>
                            <span class='text-number text-bold text-grey-6'>6. </span><span class='q-pl-sm'>{{ t('help.quickDecideWhereYouWantAllTheAbove') }}</span> <br>
                            <span class='text-number text-bold text-grey-6'>7. </span><i18n-t keypath="help.quickHitToPlayPauseATrackSkip" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #spacebar><q-badge outline color='grey-8'><span class='text-uppercase text-grey-3'>{{ t('common.spacebar') }}</span></q-badge></template>
                                <template #leftArrow><q-icon name='mdi-arrow-left'></q-icon></template>
                                <template #rightArrow><q-icon name='mdi-arrow-right'></q-icon></template>
                            </i18n-t> <br>
                            <span class='text-number text-bold text-grey-6'>8. </span><i18n-t keypath="help.quickTagEmQuicklyUsingTheKeyboardBinds" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                                <template #value><q-badge outline color='grey-8'><span class='text-uppercase text-grey-3'>{{ t('help.quickKey') }}</span></q-badge></template>
                                <template #cursorIcon><q-icon name='mdi-cursor-default-outline'></q-icon></template>
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                            </i18n-t><br>
                        </div>
                        </div>
                        <i18n-t keypath="help.quickForInspirationCheckOutTheTaggingSystem" tag="div" scope="global" class='bg-darker text-subtitle2 text-grey-5 q-py-lg q-my-lg text-center' style='line-height: 24px; white-space: pre-line'>
                            <template #littleDataLottaLove><span class='clickable doc-link' @click='$1t.url("https://www.reddit.com/r/DJs/comments/c3o2jk/my_ultimate_track_tagging_system_the_little_data/")'>Little Data, Lotta Love</span></template>
                        </i18n-t> 
                        
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md' style='line-height: 24px'>{{ t('common.energy') }}</div>
                        <i18n-t keypath="help.quickTagEnergyLevelUsingStarsSomeDJ" tag="div" scope="global" class='col-10 q-mt-md text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #starIcon><q-icon name='mdi-star'></q-icon></template>
                            <template #rating><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.rating') }}</span></q-badge></template>
                        </i18n-t>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>{{ t('common.mood') }}</div>
                        <i18n-t keypath="help.quickPredefineYourMoodsAdditionallyColorCodeThem" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #moodsOtherCharacteristicsOverview><span class='clickable doc-link' @click='$1t.url(moodOverview)'>{{ t('help.quickMoodsOverview') }}</span></template>
                            <template #mood><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.mood') }}</span></q-badge></template>
                        </i18n-t>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>{{ t('common.genre') }}</div>
                        <i18n-t keypath="help.quickIfTheGenreStatedInTheOriginal" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #genre><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.genre') }}</span></q-badge></template>
                            <template #autoTag><span class='text-subtitle3 text-bold'>{{ t('common.autoTag') }}</span></template>
                            <template #settingsIcon><q-icon name='mdi-cog' class='q-mb-xs'></q-icon></template>
                            <template #quickTag><span class='text-subtitle3 text-bold'>{{ t('common.quickTag') }}</span></template>
                            <template #character><i18n-t keypath="help.quickTipWhenKeybindingUseForAGenre" tag="span" scope="global" class='text-subtitle2 text-grey-4' style="white-space: pre-line">
                                <template #value><q-badge outline color='grey-8'><span class='text-uppercase text-grey-3'>SHIFT</span></q-badge></template>
                                <template #character><q-badge outline color='grey-8'><span class='text-uppercase text-grey-3'>{{ t('common.character') }}</span></q-badge></template>
                                <template #character2><q-badge outline color='grey-8'><span class='text-uppercase text-grey-3'>{{ t('common.character') }}</span></q-badge></template>
                            </i18n-t></template>
                        </i18n-t>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px; margin-top: 21px;'>{{ t('common.custom') }}</div>
                        <div class='col-10 q-mb-md q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; margin-top: 21px;'>{{ t('help.quickWhateverYouWantFocusOnDifferentElements') }}</div>
                        </div>
                        <div class='bg-darker text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md text-center'>
                            <DJAppIcons></DJAppIcons>                                                                           
                        </div>
                        <i18n-t keypath="help.quickWhichTagFramesCanBeReadBy" tag="div" scope="global" class='text-subtitle3 text-grey-5 text-center text-caption' style='line-height: 24px; white-space: pre-line'>
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                        </i18n-t>
                                                
                    </div>

                </q-card-section>
            </div>

            <!-- Tag editor -->
            <div v-if='route == "tageditor"'>
                <q-card-section class='q-pa-lg'>
                    <div class='q-mt-md text-subtitle2 text-primary text-bold text-uppercase text-center'>{{ t('common.helpEditor') }}</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-black'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>

                    <!-- Page 1 -->
                    <div v-if='page == 0'>                    
                        
                        <div class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px; white-space: pre-line'>{{ t('help.editorItReadsTheMetadataOfMPAIFF') }}</div>
                        
                        <div class='row' style='width: 82%; margin-left:18%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>{{ t('common.howItWorks') }}</div>
                        <i18n-t keypath="help.editorThePathOnTopLeftToSelect" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                            <template #aListOfAll><span class="q-ml-sm">{{ t('help.editorAListOfAllAudioFilesWithin') }}<br></span></template>
                            <template #click2><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                        </i18n-t>
                        </div>
                        <div class='row' style='width: 82%; margin-left:18%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'></div>
                        <div class='text-subtitle2 q-my-md text-grey-4' style='line-height: 23px'>
                            <i18n-t keypath="help.editorUnsynchronizedPopularimeterAreSpecialTagFrames" tag="span" scope="global" class="q-ml-sm" style="white-space: pre-line">
                                <template #comments><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.comments') }}</span></q-badge></template>
                                <template #lyrics><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.lyrics') }}</span></q-badge></template>
                                <template #rating><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.rating') }}</span></q-badge></template>
                            </i18n-t><br>
                            <i18n-t keypath="help.editorTheyShowAtTheBottomBelowThe" tag="span" scope="global" class="q-ml-sm" style="white-space: pre-line">
                                <template #albumArt><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.albumArt') }}</span></q-badge></template>
                            </i18n-t><br>
                            <br>
                            <i18n-t keypath="help.editorReviewAlterThemToYourLikingAt" tag="span" scope="global" class="q-ml-sm" style="white-space: pre-line">
                                <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                                <template #save><span><q-badge color='primary' class='text-uppercase text-black'>{{ t('common.save') }}</q-badge></span></template>
                            </i18n-t><br>
                        </div>
                        </div>                        
                        <div class='row' style='width: 82%; margin-left:18%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-xs' style='line-height: 24px'>{{ t('common.yourList') }}</div>
                        <i18n-t keypath="help.editorTheSettingsCheckTheBox" tag="div" scope="global" class='col-10 q-mt-xs text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                            <template #ifYouOnlyWant><span class="q-ml-sm">{{ t('help.editorIfYouOnlyWantToShowA') }}<br></span></template>
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                            <template #settingsIcon><i18n-t keypath="help.editorIcon" tag="span" scope="global" style="white-space: pre-line">
                                <template #settingsIcon><q-icon name='mdi-cog'></q-icon></template>
                            </i18n-t></template>
                            <template #preferences><span class='text-subtitle3 text-bold'>{{ t('common.preferences') }}</span></template>
                            <template #editTags><span class='text-subtitle3 text-bold'>{{ t('common.editTags') }}</span></template>
                            <template #checkboxIcon><span><q-icon name='mdi-checkbox-blank-outline' class='q-mb-xs'></q-icon></span></template>
                            <template #showYourList>{{ t('common.showYourList') }}</template>
                            <template #thisAllowsYouTo><span class="q-ml-sm">{{ t('help.editorThisAllowsYouToDragDropAudio') }}</span></template>
                        </i18n-t>
                        </div>
                        <i18n-t keypath="help.editorThePathSelectedInWillOverwriteThe" tag="div" scope="global" class='bg-darker text-subtitle2 text-grey-5 q-px-md q-py-md q-my-md text-center' style='line-height: 24px; white-space: pre-line'>
                            <template #quickTag><span class='text-subtitle3 text-bold'>{{ t('common.quickTag') }}</span></template>
                            <template #editTags><span class='text-subtitle3 text-bold'>{{ t('common.editTags') }}</span></template>
                        </i18n-t>

                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>                   
                        
                    <div class='row' style='width: 82%; margin-left:18%'>
                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>{{ t('common.manualTag') }}</div>
                    <i18n-t keypath="help.editorInsteadOfWhichMatchesAutomaticallyBasedOn" tag="div" scope="global" class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; white-space: pre-line'>
                        <template #autoTag><span class='text-subtitle3 text-bold'>{{ t('common.autoTag') }}</span></template>
                        <template #manualTag><span class='text-subtitle3 text-bold'>{{ t('common.manualTag') }}</span></template>
                    </i18n-t>
                    </div>                    
                    
                    <div class='row' style='width: 82%; margin-left:18%'>
                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'></div>
                    <div class='text-subtitle2 q-my-md text-grey-4' style='line-height: 24px'>
                        <span class='text-number text-bold text-grey-6'>1.<span class='text-number text-bold text-grey-10'>_</span> </span><i18n-t keypath="help.editorTheTrackYouWantToSearchUp" tag="span" scope="global" class='q-pl-xs' style="white-space: pre-line">
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                            <template #manualTag><span><q-badge color='primary' class='text-uppercase text-black'>{{ t('common.manualTag') }}</q-badge></span></template>
                        </i18n-t><br>
                        <span class='text-number text-bold text-grey-6'>2.<span class='text-number text-bold text-grey-10'>_</span> </span><i18n-t keypath="help.editorSelectThePlatformsAndTagsYouWant" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                            <template #start><span class='text-subtitle3 text-primary'>{{ t('common.start') }}</span></template>
                        </i18n-t><br>
                        <span class='text-number text-bold text-grey-6'>3.<span class='text-number text-bold text-grey-10'>_</span> </span><i18n-t keypath="help.editorAListOfResultsWillShowUp" tag="span" scope="global" class='q-pl-sm' style="white-space: pre-line">
                            <template #artist><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.artist') }}</span></q-badge></template>
                            <template #title><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>{{ t('common.title') }}</span></q-badge></template>
                            <template #value><q-badge outline color='primary'><span class='text-uppercase text-grey-3'>ISRC</span></q-badge></template>
                        </i18n-t><br>
                        <span class='text-number text-bold text-grey-6'>4.<span class='text-number text-bold text-grey-10'>_</span> </span><i18n-t keypath="help.editorTheCheckboxOfTheResultThatResembles" tag="span" scope="global" class='q-pl-xs' style="white-space: pre-line">
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                        </i18n-t><br>
                        <span class='text-number text-bold text-grey-6'>5.<span class='text-number text-bold text-grey-10'>_</span> </span><i18n-t keypath="help.editorToOverwriteTheExistingTags" tag="span" scope="global" class='q-pl-xs' style="white-space: pre-line">
                            <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                            <template #apply><span class='text-subtitle3 text-primary'>{{ t('common.apply') }}</span></template>
                        </i18n-t>
                    </div>
                    </div>                        
                    
                    <div class='row' style='width: 82%; margin-left:18%'>
                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'></div>
                    <i18n-t keypath="help.editorTipToCheckIfThisIsThe" tag="div" scope="global" class='text-subtitle2 q-my-xs text-grey-4' style='line-height: 23px; white-space: pre-line'>
                        <template #click><span class='text-grey-6 keybind-icon text-caption text-bold'>{{ t('common.click') }}</span></template>
                        <template #externalLinkIcon><span class="text-grey-6"><q-icon name='mdi-open-in-new'></q-icon></span></template>
                    </i18n-t>
                    </div>

                    <div class='row' style='width: 82%; margin-left:18%'>
                    <div class='col-2 text-subtitle2 text-primary text-bold'></div>
                    <div class='text-subtitle2 q-my-md text-grey-4' style='line-height: 23px'>{{ t('help.editorIfThereAreAnyErrorsThatOccured') }}</div>
                    </div>

                    <i18n-t keypath="help.editorIsAlsoAccessableWithAOnA" tag="div" scope="global" class='bg-darker text-subtitle2 text-grey-5 q-py-md q-my-md text-center' style='line-height: 24px; white-space: pre-line'>
                        <template #manualTag><span class='text-subtitle3 text-bold'>{{ t('common.manualTag') }}</span></template>
                        <template #right><span class='text-caption text-bold keybind-icon q-pl-sm'>{{ t('common.right') }}</span></template>
                        <template #click><span class='text-bold text-caption keybind-icon q-px-sm'>{{ t('common.click') }}</span></template>
                        <template #quickTag><span class='text-subtitle3 text-bold'>{{ t('common.quickTag') }}</span></template>
                    </i18n-t>
                    

                </div>

                </q-card-section>
            </div>


            <!-- Renamer -->
            <div v-if='route == "renamer"'>
                <q-card-section class='q-pa-lg'>
                    
                    <div class='row'>
                        
                        <!-- List of all things -->
                        <div class='col-3' style='overflow-y: scroll;'>
                            <div style='height: 500px;'>
                                <div class='text-h6 q-py-sm clickable' @click='renamerDoc = null' :class='{"text-primary": !renamerDoc, "text-grey-4": renamerDoc}'>{{ t('common.autoRename') }}</div>

                                <div class='text-h6 text-grey-4 q-py-sm'>{{ t('common.variables') }}</div>
                                <div v-for='(v, i) in $1t.info.value.renamerDocs.variables' :key='"RDV"+i' class='renamer-doc-token'  @click='renamerDoc = v'>
                                    <RenamerTokenName :token='v' :class='{"text-primary": renamerDoc == v}'></RenamerTokenName>
                                </div>
        
                                <div class='q-my-md'></div>
        
                                <div class='text-h6 text-grey-4 q-py-sm'>{{ t('common.properties') }}</div>
                                <div v-for='(v, i) in $1t.info.value.renamerDocs.properties' :key='"RDP"+i' class='renamer-doc-token'  @click='renamerDoc = v'>
                                    <RenamerTokenName :token='v' :class='{"text-primary": renamerDoc == v}'></RenamerTokenName>
                                </div>
        
                                <div class='q-my-md'></div>
        
                                <div class='text-h6 text-grey-4 q-py-sm'>{{ t('common.functions') }}</div>
                                <div v-for='(v, i) in $1t.info.value.renamerDocs.functions' :key='"RDF"+i' class='renamer-doc-token'  @click='renamerDoc = v'>
                                    <RenamerTokenName :token='v' :type='false' :class='{"text-primary": renamerDoc == v}'></RenamerTokenName>
                                </div>
                            </div>
                        </div>

                        <!-- Docs -->
                        <div class='col-9 q-pl-xl'>
                            
                            <!-- Not selected -->
                            <div v-if='!renamerDoc' class='text-center'>
                            
                                <div class='q-mt-md text-subtitle2 text-center text-primary text-bold text-uppercase'>{{ t('common.helpRenamer') }}</div>
                                <div class='q-mt-xs text-center'>
                                    <q-badge color='primary'>
                                        <span class='text-number text-bold text-black'>{{page+1}} / {{pages}}</span>
                                    </q-badge>
                                </div>      

                                
                                <div class='text-grey-4 q-mt-md text-center' style='line-height: 24px'></div>
                                <i18n-t keypath="help.renamerAutomaticallyRenameYourFilesBasedOnTags" tag="div" scope="global" class='q-my-sm text-grey-4' style="white-space: pre-line">
                                    <template #videoTutorial><span @click='$1t.url("https://youtu.be/_D51bNKK1QY")' class='clickable doc-link text-caption text-uppercase'><span class="q-mr-xs text-subtitle1 margin-yt"><q-icon name='mdi-youtube'></q-icon></span>{{ t('common.videoTutorial') }}</span></template>
                                </i18n-t>

                                <div>
                                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>{{ t('common.dynamicContent') }}</div>
                                    <div class='text-grey-4' style="white-space: pre-line">{{ t('help.renamerTheTemplateStringDescribesTheFilenameScheme') }}</div>
    
                                    <div class='q-mt-sm' style="padding-left: 100px;">
                                        <div style='width: 240px; margin: auto;' class='row'>
                                            <div class='col-4 text-left text-grey-4'>
                                                <span class="__renamer_syntax_operator">%</span><span class="__renamer_syntax_variable">artist</span><span class="__renamer_syntax_operator">%</span><br>
                                                <span class="__renamer_syntax_operator">%</span><span class="__renamer_syntax_variable">album</span><span class="__renamer_syntax_operator">%</span><br>                                                
                                            </div>
                                            <div class='col-4 text-left text-grey-4'>                                              
                                                <span class="__renamer_syntax_operator">%</span><span class="__renamer_syntax_variable">title</span><span class="__renamer_syntax_operator">%</span><br>
                                                <span class="__renamer_syntax_operator">%</span><span class="__renamer_syntax_variable">track</span><span class="__renamer_syntax_operator">%</span><br>
                                            </div>                                            
                                        </div>
                                    </div>
                                </div>

                                <div class='q-my-sm text-grey-4'>{{ t('help.renamerAnyOtherVariableThatIsNotListed') }}</div>    

                                <div class='q-my-sm'>
                                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>{{ t('common.staticContent') }}</div>
                                    <i18n-t keypath="help.renamerBasicallyAnyYouWouldLikeToAdd" tag="div" scope="global" class='text-grey-4' style="white-space: pre-line">
                                        <template #label><span class="__renamer_syntax_operator">%</span></template>
                                        <template #variable><span class="__renamer_syntax_variable">variable</span></template>
                                        <template #label2><span class="__renamer_syntax_operator">%</span></template>
                                    </i18n-t>
                                </div>

                                <div>
                                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>{{ t('common.examples') }}</div>
                                    <HelpRenamerExamples></HelpRenamerExamples>
                                </div>

                                <i18n-t keypath="help.renamerUseToDefineAFolder" tag="div" scope="global" class='bg-darker text-subtitle2 text-grey-5 q-py-sm q-my-md text-center' style='line-height: 24px; white-space: pre-line'>
                                    <template #value><q-badge outline color='grey-5'><span class='text-grey-4'>/</span></q-badge></template>
                                    <template #thisAppliesToWindows><span class='text-subtitle3 text-grey-5'>{{ t('help.renamerThisAppliesToWindowsMacOSLinux') }}</span></template>
                                </i18n-t>
                                
                            </div>

                            <!-- Selected -->
                            <div v-if='renamerDoc'>
                                <RenamerTokenName :token='renamerDoc' class='text-h5'></RenamerTokenName>
                                <br>
                                <div class='text-caption q-mt-sm q-mb-md'>{{ renamerKindLabel(renamerDoc.kind) }}</div>
                                <div style='font-size: 120%;'>
                                    <div style="white-space: pre-wrap">{{ renamerDocumentation(renamerDoc) }}</div>
                                </div>
                            </div>
                        </div>
                    </div>


                </q-card-section>
            </div>

        </q-card>
    </q-dialog>

</div>
</template>

<script lang='ts' setup>
import { useI18n } from 'vue-i18n';
const { t } = useI18n({ useScope: 'global' });

import DJAppIcons from './DJAppIcons.vue';
import RenamerTokenName from './RenamerTokenName.vue';
import HelpRenamerExamples from './HelpRenamerExamples.vue';
import { renamerDocumentation } from '../scripts/renamer-localization';
import { computed, ref, watch } from 'vue';
import { get1t } from '../scripts/onetagger';
import { useRoute } from 'vue-router';

const $1t = get1t();
const $route = useRoute();
const show = ref(false);
const page = ref(0);
const renamerDoc = ref<any | undefined>(undefined);
const moodOverview = 'https://docs.google.com/spreadsheets/d/1wYokScjoS5Xb1IvqFMXbSbknrXJ7bySLLihTucOS4qY/edit?usp=sharing';
const isrcWiki = 'https://en.wikipedia.org/wiki/International_Standard_Recording_Code';

function renamerKindLabel(kind: string): string {
    const kinds: Record<string, string> = { variable: 'variables', property: 'properties', function: 'functions' };
    return kinds[kind] ? t(`common.${kinds[kind]}`) : kind;
}

// Hide help
function hide() {
    page.value = 0;
    $1t.helpDialog.value.open = false;
    $1t.helpDialog.value.route = undefined;
}

// Global override for homescreen
const route = computed(() => {
    if ($1t.helpDialog.value.route) return $1t.helpDialog.value.route;
    return $route.path?.substring(1)?.split("/")[0];
});

// Get page count
const pages = computed(() => {
    if (route.value == 'renamer') return 1;
    return 2;
});

// Show/Hide help button

const showButton = computed(() => {
    if ($1t.helpDialog.value.open || !route.value || 
            $route.path.includes('/status') || !$1t.settings.value.helpButton) return false;
    return true;
});

// Open/close globally
watch(() => $1t.helpDialog.value.open, () => {
    if ($1t.helpDialog.value.open) {
        show.value = true;
    }
});
</script>

<style>
.help-button-container {
    position: fixed;
    right: -18px;
    top: 50%;
    /* Drawers */
    z-index: 1002;
    transform: rotate(270deg);
}

.help-button {
    opacity: 0.5;
}
.help-button:hover {
    opacity: 0.9;
}

@media screen and (max-width: 1200px) {
    .help-card {
        min-height: 550px;
        width: 80%;
        min-width: 80%;
    }
}
@media screen and (min-width: 1200px) {
    .help-card {
        min-height: 550px;
        min-width: 1000px;
    }
}

.help-page-icon {
    position: absolute !important;
    top: calc(50% - 23px);
    z-index: 1000;
}

.help-page-icon-forward {
    left: calc(100% - 50px);
}

.help-page-icon-back {
    right: calc(100% - 50px);
}

.doc-link {
    color: var(--q-color-primary);    
}

.doc-link:hover {
    color: #f0f0f0;    
}

.renamer-doc-token {
    cursor: pointer
}

.renamer-doc-token:hover {
    font-weight: bold;
}
.keybind-icon {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}
</style>
