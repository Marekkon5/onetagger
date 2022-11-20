<template>
<div>

    <!-- Button -->
    <div class='help-button-container' @click='show = true' v-if='showButton'>
        <q-btn color='primary' class='q-px-md help-button text-bold' style='margin-right: -10px'>
            <q-icon name='mdi-help-circle-outline' class='q-pr-xs' size='xs'></q-icon>
            Help
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
                @click='page += 1'
                v-if='page < (pages-1)'
                ><q-icon name='mdi-chevron-right' size='lg'></q-icon>
            </q-btn>
            <q-btn 
                class='help-page-icon clickable help-page-icon-back'
                flat
                round
                @click='page -= 1'
                v-if='page > 0'
                ><q-icon name='mdi-chevron-left' size='lg'></q-icon>
            </q-btn>

            <!-- Auto tagger -->
            <div v-if='route == "autotagger"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-center text-primary text-bold text-uppercase'>Getting started with Auto Tag</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-dark'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>                  

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px'>
                            Automatically tag your local audio files, from several online platforms,
                                based on <q-badge outline color='primary'><span class='text-white'>Artist</span></q-badge> & <q-badge outline color='primary'><span class='text-white'>Title</span></q-badge> tag, filename or playlist. <br> Or identify tracks with <b>Shazam</b>.<br>
                        </div>
                        <div class='row' style='width: 94%; margin-left:5%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>How does it work?</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            It reads the <q-badge outline color='primary'><span class='text-white'>Artist</span></q-badge> & <q-badge outline color='primary'><span class='text-white'>Title</span></q-badge> tag from your local MP3, AIFF, FLAC & M4A files (or filename or playlist), <br>feeds it into the search engine of the selected platforms, fetches the tags & writes it to your files.<p></p>
                            When no tags or proper filename is present, you can enable <span @click='$1t.url("https://shazam.com")' class='clickable doc-link'>Shazam </span> to identify the track & still fetch tags.<br>
                                            
                            <span @click='$1t.url("https://beatport.com")' class='clickable doc-link'>Beatport </span> , <span @click='$1t.url("https://junodownload.com")' class='clickable doc-link'>Juno Download </span> & <span @click='$1t.url("https://traxsource.com")' class='clickable doc-link'>Traxsource </span> are based on a method called scraping. <br>
                            <span @click='$1t.url("https://beatsource.com")' class='clickable doc-link'>Beatsource </span> , <span @click='$1t.url("https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/iTuneSearchAPI/index.html")' class='clickable doc-link'>iTunes </span> & <span @click='$1t.url("https://musicbrainz.org")' class='clickable doc-link'>MusicBrainz </span> are based on their API.
                            <span @click='$1t.url("https://discogs.com")' class='clickable doc-link'>Discogs </span> & <span @click='$1t.url("https://spotify.com")' class='clickable doc-link'>Spotify </span> too, but need a free account.
                        
                        </div>
                        </div>
                        <div class='row' style='width: 94%; margin-left:5%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>What does it do?</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            Fetches metadata automatically & writes tags to your audio files.  Fully configurable. Set to overwrite or add/fill in when empty.
                        </div>
                        <div class='row text-subtitle2 text-grey-4 q-pt-sm' style='width: 100%; margin-left: 16.8%; line-height: 24px'>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Album Art / Cover</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Resolution is platform dependent</q-tooltip>
                                </q-icon>                            
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Title</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Version</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Available from Beatport, Beatsource & Traxsource</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>ISRC</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Available from Beatport, Beatsource, MusicBrainz & Spotify</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Catalog No.</span></q-badge></div>
                            
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Album</span></q-badge></div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>BPM</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Available from Beatport, Beatsource, Juno Download & Traxsource</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Style / Subgenre</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Style is available from Discogs & Bandcamp, Subgenre from Beatport only</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Track No.</span></q-badge></div>

                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Album Artist</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Artist</span></q-badge></div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Key</span></q-badge>
                            <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs'>
                                    <q-tooltip>Available from Beatport, Beatsource & Traxsource</q-tooltip>
                                </q-icon>
                            </div>
                                
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Label</span></q-badge></div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Duration</span></q-badge></div>
                            
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Remixers</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs'>
                                    <q-tooltip>Available from Beatport & Beatsource</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Publish Date</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Publish Date is available from Beatport only</q-tooltip>
                                </q-icon><q-badge outline color='primary'><span class='text-white'>Release Date</span></q-badge></div>
                            
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>Release ID</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Track ID</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='other-tooltip q-mx-xs'>
                                    <q-tooltip>Available from Beatport, Beatsource & Traxsource</q-tooltip>
                                </q-icon>
                            </div>
                            <div class='col-3 q-mt-xs'><q-badge outline color='primary'><span class='text-white'>URLs</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Other</span></q-badge>
                                <q-icon name='mdi-help-circle-outline text-grey-6' class='onetagger-tooltip q-mx-xs'>
                                    <q-tooltip>Adds UNIQUEFILEID tag when Beatport is selected</q-tooltip>
                                </q-icon>
                            </div>
                            </div>
                        </div>
                        
                        <div class='bg-darker text-subtitle2 text-grey-5 q-my-md q-py-md text-center' style='line-height: 24px'>
                            <span class='q-pl-md'>Not all platforms have the same tags available. Hover over the help <q-icon name='mdi-help-circle-outline' style='margin-bottom: -2px;' class='q-pb-xs text-grey-6'></q-icon> icons for more info</span> <br>
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='row' style='width: 90%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>Step-by-step guide</div>
                        <div class='col-10 q-mt-md text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>1.<span class='text-number text-bold text-grey-10'>_</span> </span><span class='q-pl-sm'>Check the box to fetch from particular platform</span> <br>
                            <span class='text-number text-bold text-grey-6'>2. </span><span class='q-pl-sm'>Drag & drop the cards to reorder fallback.  Click <q-badge color='primary' class='text-dark'>Next</q-badge></span> <br>
                            <span class='text-number text-bold text-grey-6'>3. </span><span class='q-pl-sm'>Drag & drop folder, copy/paste path directly or click the browse <q-icon name='mdi-open-in-app' class='q-mb-xs'></q-icon> icon</span> <br>
                            <span class='text-number text-bold text-grey-10'>_</span><span class='q-pl-md'>Or drag & drop playlist onto the card</span> <br>
                            <span class='text-number text-bold text-grey-6'>4. </span><span class='q-pl-sm'>Check the box to fetch stated tag. Click <q-badge color='primary' class='text-dark'>Next</q-badge></span> <br>
                        </div>
                        </div>
                        <div class='row' style='width: 90%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Beatport</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>5. </span><span class='q-pl-sm'>Select <q-badge outline color='primary'><span class='text-white'>Album Art</span></q-badge> resolution. 500x500 is native, above will upscale</span> <br>
                            <span class='text-number text-bold text-grey-6'>6. </span><span class='q-pl-sm'>Drag slider to set amount of search page results to scan for the most corresponding track</span> <br>
                        </div>
                        </div>
                        <div class='row' style='width: 90%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Discogs</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>7. </span><span class='q-pl-sm'>Enter token - To obtain token, create a free account on <span @click='$1t.url("https://discogs.com")' class='clickable doc-link'>Discogs </span></span> <br>
                            <span class='text-number text-bold text-grey-10'>_</span><span class='q-pl-md'>Go to <span @click='$1t.url("https://www.discogs.com/settings/developers")' class='clickable doc-link'>discogs.com/settings/developers </span> & click 'Generate token'</span> <br>
                            <span class='text-number text-bold text-grey-10'>_</span><span class='q-pl-md'>Select it & copy/paste the current token <span @click='$1t.url("https://youtu.be/IvAiMkfdLCw")' class='clickable doc-link'>see video demo <q-icon name='mdi-youtube' class='q-mb-xs q-pr-xs'></q-icon></span></span><p></p>
                            <span class='text-number text-bold text-grey-6'>8. </span><span class='q-pl-sm'>Select <q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Style/Subgenre</span></q-badge> tag to fetch both, if it should merge them, or write elsewhere</span> <br>
                            <span class='text-number text-bold text-grey-6'>9. </span><span class='q-pl-sm'>Drag slider to set amount of album search results to check. Click <q-badge color='primary' class='text-dark'>Next</q-badge></span><br>
                        </div>
                        </div>
                        <div class='row' style='width: 90%; margin-left:15%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Advanced</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            <span class='text-number text-bold text-grey-6'>10.</span><span class='q-pl-sm'>Enable toggle <q-icon name='mdi-toggle-switch' class='text-grey-4'></q-icon> to activate stated options</span><br>
                            <span class='text-number text-bold text-grey-10'>_&nbsp;</span><span class='q-pl-md'>Hover over the help <q-icon name='mdi-help-circle-outline' class='q-pb-xs text-grey-6'></q-icon> icons for more info</span> <br>
                            <span class='text-number text-bold text-grey-10'>_&nbsp;</span><span class='q-pl-md'>Click the start <q-icon name='mdi-play-circle' class='q-mb-xs text-primary'></q-icon> icon to process</span> <p></p>
                        </div>
                        </div>
                        

                    </div>

                </q-card-section>
            </div>

            <!-- Audio features -->
            <div v-if='route == "audiofeatures"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase text-center'>Getting started with Audio Features</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-dark'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px'>
                            Automatically tag your local audio files, with so called audio features by <b>Spotify</b>, based on <q-badge outline color='primary'><span class='text-white'>ISRC</span></q-badge> tag or exact match.
                            
                        </div>
                        <div class='row' style='width: 88%; margin-left:12%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>How does it work?</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            Follow the setup described on the <span class='text-subtitle3 text-bold'>AUDIO FEATURES</span> entrance &nbsp;<span @click='$1t.url("https://youtu.be/i0q5qWQSH9Y")' class='clickable doc-link'>see video demo <q-icon name='mdi-youtube' class='q-pr-xs'></q-icon></span>
                        <div class='col-10 text-subtitle2 text-grey-4' style='line-height: 24px'>
                            Select a folder with tracks you want to get their audio features fetched for - or drag & drop playlist.<br>
                            Based on the <span class='clickable doc-link' @click='$1t.url(isrcWiki)'>ISRC </span> inside the metadata it will search up the track in Spotify's API & return these values. <br>
                            If no <q-badge outline color='primary'><span class='text-white'>ISRC</span></q-badge> tag exists, it will search on <q-badge outline color='primary'><span class='text-white'>Artist</span></q-badge> & <q-badge outline color='primary'><span class='text-white'>Title</span></q-badge> tag by exact match.<br>
                            Prominent tag translates the raw values from <q-badge color='grey-8'><span class='textnumber text-grey-2'>0-100</span></q-badge> to a meaningful  &nbsp;#description.
                        </div>
                        </div>
                        </div>
                        <div class='row' style='width: 88%; margin-left:12%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Audio features?</div>
                        <div class='col-10 q-mt-lg text-subtitle text-grey-4'>
                                
                        <div class='row text-subtitle2 text-grey-4' style='width: 70%; margin-left: 0%; line-height: 24px'>
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Acousticness</span></q-badge></div>     <div class='text-right text-grey-3 col-8'>#acoustic <span class='text-grey-6'>&nbsp;·&nbsp;</span> #electronic</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Danceability</span></q-badge></div>     <div class='text-right text-grey-3 col-8'>#dance-low <span class='text-grey-6'>&nbsp;·&nbsp;</span> #dance-med <span class='text-grey-6'>&nbsp;·&nbsp;</span> #dance-high</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Energy</span></q-badge></div>           <div class='text-right text-grey-3 col-8'>#energy-low <span class='text-grey-6'>&nbsp;·&nbsp;</span> #energy-med <span class='text-grey-6'>&nbsp;·&nbsp;</span> #energy-high</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Instrumentalness</span></q-badge></div> <div class='text-right text-grey-3 col-8'>#vocal-low <span class='text-grey-6'>&nbsp;·&nbsp;</span> #vocal-med <span class='text-grey-6'>&nbsp;·&nbsp;</span> #vocal-high</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Liveness</span></q-badge></div>         <div class='text-right text-grey-3 col-8'>#live <span class='text-grey-6'>&nbsp;·&nbsp;</span> #recording</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Speechiness</span></q-badge></div>      <div class='text-right text-grey-3 col-8'>#speech <span class='text-grey-6'>&nbsp;·&nbsp;</span> #music</div> 
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Valence</span></q-badge></div>          <div class='text-right text-grey-3 col-8'>#positive <span class='text-grey-5'>(happy)</span> <span class='text-grey-6'>&nbsp;·&nbsp;</span> #balanced <span class='text-grey-6'>&nbsp;·&nbsp;</span> #negative <span class='text-grey-5'>(sad, angry)</span></div>
                            <div class='col-4 text-grey-4'><q-badge outline color='primary'><span class='text-white'>Popularity</span></q-badge></div>       <div class='text-right text-grey-3 col-8'>#unpopular <span class='text-grey-6'>&nbsp;·&nbsp;</span> #popular</div> 
                        </div>
                        </div>
                        </div>
                        <div class='bg-darker text-subtitle2 text-grey-5 q-mt-md q-px-md q-py-md q-my-md text-center' style='line-height: 24px'>
                            For a more in-depth description, check out <span class='clickable doc-link' @click='$1t.url("https://developer.spotify.com/documentation/web-api/reference/#/operations/get-several-audio-features")'>Spotify’s official Audio Features </span> documentation<br>
                            <div class='bg-darker text-subtitle2 text-grey-5 text-center q-pt-xs' style='line-height: 24px'>
                            To look up audio features online for other tracks, check out <span class='clickable doc-link' @click='$1t.url("https://songdata.io")'>SongData.io </span> - Ignore the <span class='clickable doc-link' @click='$1t.url("https://www.reddit.com/r/DJs/comments/m3q97z/key_detection_comparison_spotify_vs_tunebat_vs/")'>keys determined by Spotify </span> though
                            </div>
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='row' style='width: 100%; margin-left:1%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md text-right'>Prominent tag</div>
                        <div class='col-10 q-mt-md text-subtitle2 text-grey-4 q-pl-sm' style='line-height: 24px'>
                            You can customize the threshold, so it's not only writing the simple value from <q-badge color='grey-8'><span class='textnumber text-grey-2'>0-100</span></q-badge> per audio feature, <br>
                            but also determine when it should translate the value into a meaningful description & write to a <q-badge outline color='primary'><span class='text-white'>custom</span></q-badge> predefined tag frame.
                        </div>
                        </div>

                        <div class='row' style='width: 100%; margin-left:3%'>
                        <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Acousticness</span></q-badge>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0*</span>
                                    </q-badge>
                                    <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>90-100</span>
                                    </q-badge>                        
                        </div>
                        <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#electronic <br>
                                    <span class='text-grey-10'>-</span>#acoustic
                                    </span>
                            </div>
                            
                            
                                <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Danceability</span></q-badge>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0-20</span>
                                    </q-badge>
                                    <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>21-79</span>
                                    </q-badge>  
                                    <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>80-100</span>
                                    </q-badge>
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#dance-low <br>
                                    <span class='text-grey-10'>-</span>#dance-med <br>
                                    <span class='text-grey-10'>-</span>#dance-high
                                    </span>
                            </div>
                            
                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Energy</span></q-badge>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0-20</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>21-79</span>
                                </q-badge>  
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>80-100</span>
                                </q-badge>
                                </div>
                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                        <span class='text-subtitle2 text-grey-4'>
                                        <span class='text-grey-10'>-</span>#energy-low <br>
                                        <span class='text-grey-10'>-</span>#energy-med <br>
                                        <span class='text-grey-10'>-</span>#energy-high
                                        </span>
                                </div>

                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Instrumental</span></q-badge>
                            <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0-50</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>51-89</span>
                                </q-badge>  
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>90-100</span>
                                </q-badge>
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#vocal-high <br>
                                    <span class='text-grey-10'>-</span>#vocal-med <br>
                                    <span class='text-grey-10'>-</span>#vocal-low
                                    </span>
                            </div>

                            <div class='row' style='width: 100%; margin-left:0%'>
                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Liveness</span></q-badge>
                            <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0*</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>80-100</span>
                                </q-badge>                                  
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#recording <br>
                                    <span class='text-grey-10'>-</span>#live
                                    </span>                                
                            </div>
                            

                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Speechiness</span></q-badge>
                            <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0*</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>70-100</span>
                                </q-badge>                                  
                                </div>    
                                
                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#music<br>
                                    <span class='text-grey-10'>-</span>#speech
                                    </span>                                
                            </div>
                            
                            
                            <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Valence</span></q-badge>
                            <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0-15</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>16-84</span>
                                </q-badge>  
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>85-100</span>
                                </q-badge>
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                    <span class='text-grey-10'>-</span>#negative <br>
                                    <span class='text-grey-10'>-</span>#balanced <br>
                                    <span class='text-grey-10'>-</span>#positive
                                    </span>                                
                            </div>

                        <div class='col-1 text-primary text-bold q-mt-lg text-right' style='line-height: 24px'><q-badge outline color='primary'><span class='text-white'>Popularity</span></q-badge>
                            <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>0*</span>
                                </q-badge>
                                    <br>
                                <q-badge color='grey-8'>
                                        <span class='textnumber text-grey-2'>80-100</span>
                                </q-badge>  
                                    <br>                                
                                </div>

                                <div class='col-2 q-mt-lg text-subtitle3 text-grey-4 q-pl-sm' style='line-height: 24px'>
                                <div class='q-pl-xs text-grey-5'>Threshold default</div>
                                    <span class='text-subtitle2 text-grey-4'>
                                        <span class='text-grey-10'>-</span>#unpopular <br>
                                        <span class='text-grey-10'>-</span>#popular
                                    </span>                                
                            </div>

                        </div>
                        </div>
                            <div class='q-pb-xs text-subtitle3 q-my-md text-grey-5 text-center' style='line-height: 25px'>
                                When threshold is <q-badge color='grey-8'><span class='textnumber text-grey-2'>0*</span></q-badge>&nbsp; it won't write the Prominent tag
                            </div>
                        
                        <div class='bg-darker text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md text-center text-caption'>
                            <DJAppIcons></DJAppIcons>                                                                           
                        </div>
                        <div class='text-subtitle3 text-grey-5 text-center text-caption' style='line-height: 24px'>
                            Which tag frames can be read by your DJ software? <q-icon name='mdi-cursor-default-outline' class='q-pb-xs'></q-icon> Click on the app icon above<br>
                            Also shows you recommended tags to use
                        </div>
                    </div>

                </q-card-section>
            </div>


            <!-- Quicktag -->
            <div v-if='route == "quicktag"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase text-center'>Getting started with Quick Tag</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-dark'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px'>
                            Manually tag your local audio files quickly, based on <q-badge outline color='primary'><span class='text-white'>Energy</span></q-badge> , 
                            <q-badge outline color='primary'><span class='text-white'>Mood</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge>
                                & other characteristics you defined. <br> Key bind compatible for quick access. <br>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>How does it work?</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            Set the path to a folder with tracks you like to quick tag - or drag & drop playlist.<br>
                            Predefine <q-badge outline color='primary'><span class='text-white'>Energy</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Mood</span></q-badge> , <q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge> & other characteristics. <br>
                            Decide in which tag you want them to write, so you are able to read them out in your favorite DJ software. <br>
                            Key bind them for quick access. Skip through a track to determine the mentioned elements. <br>
                            Hit the <q-badge outline color='grey-5'><span class='text-white'>keys</span></q-badge> on your keyboard - or point & click.<br>   
                        </div>                        
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%; margin-top: -1px;'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'>Step-by-step guide</div>
                        <div class='col-10 q-mt-lg q-mb-xs text-subtitle2 text-grey-4' style='line-height: 24px'>  
                            <span class='text-number text-bold text-grey-6'>1. <span class='text-number text-bold text-grey-10'>.</span></span><span class='q-pl-sm'>Click the settings <q-icon name='mdi-cog'></q-icon> icon</span> <br>
                            <span class='text-number text-bold text-grey-6'>2. </span><span class='q-pl-sm'>Set path to a folder which you like to quick tag - or drag & drop playlist</span> <br>
                            <span class='text-number text-bold text-grey-6'>3. </span><span class='q-pl-sm'>Predefine energy level to write to either <q-badge outline color='primary'><span class='text-white'>Rating</span></q-badge> or a different tag frame as symbol, like *'s</span> <br>
                            <span class='text-number text-bold text-grey-6'>4. </span><span class='q-pl-sm'>Predefine <q-badge outline color='primary'><span class='text-white'>Mood</span></q-badge> & assign a color, <q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge> & other characteristics</span> <br>
                            <span class='text-number text-bold text-grey-6'>5. </span><span class='q-pl-sm'>Key bind them all to a character on your keyboard for quick access</span> <br>
                            <span class='text-number text-bold text-grey-6'>6. </span><span class='q-pl-sm'>Decide where you want all the above data written to</span> <br>
                            <span class='text-number text-bold text-grey-6'>7. </span><span class='q-pl-sm'>Hit <q-badge outline color='grey-5'><span class='text-white'>spacebar</span></q-badge> to play/pause a track - skip through it using the arrow  keys <q-icon name='mdi-arrow-left'></q-icon> <q-icon name='mdi-arrow-right'></q-icon> on your keyboard</span> <br>
                            <span class='text-number text-bold text-grey-6'>8. </span><span class='q-pl-sm'>Tag ‘em quickly using the keyboard <q-badge outline color='grey-5'><span class='text-white'>key</span></q-badge> binds - or point & click</span><br>
                        </div>
                        </div>
                        <div class='bg-darker text-subtitle2 text-grey-5 q-py-lg q-my-lg text-center' style='line-height: 12px'>
                            For inspiration, check out the <span class='clickable doc-link' @click='$1t.url("https://www.reddit.com/r/DJs/comments/c3o2jk/my_ultimate_track_tagging_system_the_little_data/")'>Little Data, Lotta Love </span> tagging system by u/nonomomomo
                        </div> 
                        
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md' style='line-height: 24px'>Energy</div>
                        <div class='col-10 q-mt-md text-subtitle2 text-grey-4' style='line-height: 24px'>  
                            Tag energy level using stars <q-icon name='mdi-star'></q-icon>.<br>
                            Some DJ software can’t read out the <q-badge outline color='primary'><span class='text-white'>Rating</span></q-badge> tag, or simply don’t have the ability to show it. <br>
                            Then you can alternatively set the output to ‘symbol’ instead. Predefine the symbol (a * for example)<br>
                            Also predefine where to write the energy rating by selecting a proper tag frame. <br>                            
                        </div>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>Mood</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            Predefine your moods & additionally color code them. Use key bindings for quick access. <br>
                            For inspiration what moods you could use, see this <span class='clickable doc-link' @click='$1t.url(moodOverview)'>Moods & other characteristics overview </span> <br>
                            Since the native <q-badge outline color='primary'><span class='text-white'>Mood</span></q-badge> tag isn’t read by any DJ software, we need to write it elsewhere. <br>                            
                        </div>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>Genre</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            If the genre stated in the original <q-badge outline color='primary'><span class='text-white'>Genre</span></q-badge> tag of your tracks, or the ones fetched with <span class='text-subtitle3 text-bold'>AUTO TAG</span> are still not to your liking, <br>
                            predefine them in settings <q-icon name='mdi-cog' class='q-mb-xs'></q-icon> > <span class='text-subtitle3 text-bold'>QUICK TAG</span>.<br>
                                                        
                            <span class='text-subtitle2 text-grey-5'>Tip: When keybinding, use <q-badge outline color='grey-5'>SHIFT</q-badge> + <q-badge outline color='grey-5'>character</q-badge> for a genre. This way you can keep a <q-badge outline color='grey-5'>character</q-badge> for moods</span>
                        </div>
                        </div>
                        <div class='row' style='width: 93%; margin-left:7%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px; margin-top: 21px;'>Custom</div>
                        <div class='col-10 q-mb-md q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px; margin-top: 21px;'>
                            Whatever you want! Focus on different elements like vibe, instruments, vocals, time to play or situation, etc.
                        </div>
                        </div>
                        <div class='bg-darker text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md text-center'>
                            <DJAppIcons></DJAppIcons>                                                                           
                        </div>
                        <div class='text-subtitle3 text-grey-5 text-center text-caption' style='line-height: 24px'>
                            Which tag frames can be read by your DJ software? <q-icon name='mdi-cursor-default-outline'></q-icon> Click on the app icon above<br>
                            Also shows you recommended tags to use
                            </div>
                                                
                    </div>

                </q-card-section>
            </div>

            <!-- Tag editor -->
            <div v-if='route == "tageditor"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase text-center'>GETTING STARTED WITH EDIT TAGS</div>
                    <div class='q-mt-xs text-center'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-dark'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                    
                        
                        <div class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px'>
                            It reads the metadata of MP3, AIFF, FLAC & M4A files within a folder.<br>
                            You can review what tags got written where inside the metadata, edit them, add a new tag or delete.<br>
                        </div>
                        
                        <div class='row' style='width: 82%; margin-left:18%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg' style='line-height: 24px'>How does it work?</div>
                        <div class='col-10 q-mt-lg text-subtitle2 text-grey-4' style='line-height: 24px'>
                            Click the path on top left to select folder using your OS's picker.<br>
                            A list of all audio files within the folder will be populated.<br>
                            Click on a file to show the metadata & its tag frames sorted by alphabet *<br>
                        </div>
                        </div>
                        <div class='row' style='width: 82%; margin-left:18%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-lg'></div>
                        <div class='text-subtitle2 q-my-md text-grey-4' style='line-height: 24px'>
                            * <q-badge outline color='primary'><span class='text-white'>Comments</span></q-badge> , (Unsynchronized) <q-badge outline color='primary'><span class='text-white'>Lyrics</span></q-badge> & Popularimeter <q-badge outline color='primary'><span class='text-white'>Rating</span></q-badge> are special tag frames.<br>
                            They show at the bottom below the <q-badge outline color='primary'><span class='text-white'>Album art</span></q-badge><br>
                            <br>
                            Review & alter them to your liking. Click <span><q-badge color='primary' class='text-dark'>Save</q-badge></span> at the bottom to write the changes made.<br>
                        </div>
                        </div>                        
                        <div class='row' style='width: 82%; margin-left:18%'>
                        <div class='col-2 text-subtitle2 text-primary text-bold q-mt-xs' style='line-height: 24px'>Your list</div>
                        <div class='col-10 q-mt-xs text-subtitle2 text-grey-4' style='line-height: 24px'>
                            If you only want to show a few audio files, or from multiple folders,<br>
                            click the settings <span><q-icon name='mdi-cog'></q-icon> icon</span> > <span class='text-subtitle3 text-bold'>GENERAL</span> > <span class='text-subtitle3 text-bold'>EDIT TAGS</span> > check the box <span><q-icon name='mdi-checkbox-blank-outline' class='q-mb-xs'></q-icon></span> Show 'Your list'<br>
                            This allows you to drag & drop audio files from the treelist into 'Your list'.
                        </div>
                        </div>
                        <div class='bg-darker text-subtitle2 text-grey-5 q-px-md q-py-md q-my-md text-center' style='line-height: 24px'>
                            The path selected in <span class='text-subtitle3 text-bold'>QUICK TAG</span> will overwrite the path of <span class='text-subtitle3 text-bold'>EDIT TAGS</span> & vice versa.<br>
                            This way it becomes easy to review where your custom tags got written, by switching between the two tabs.
                        </div>

                    </div>

                </q-card-section>
            </div>


            <!-- Renamer -->
            <div v-if='route == "renamer"'>
                <q-card-section class='q-pa-xl'>
                    
                    <div class='row'>
                        
                        <!-- List of all things -->
                        <div class='col-3' style='overflow-y: scroll;'>
                            <div style='height: 500px;'>
                                <div class='text-h6 q-py-sm clickable' @click='renamerDoc = null' :class='{"text-primary": !renamerDoc, "text-grey-4": renamerDoc}'>Auto Rename</div>

                                <div class='text-h6 text-grey-4 q-py-sm'>Variables</div>
                                <div v-for='(v, i) in $1t.info.value.renamerDocs.variables' :key='"RDV"+i' class='renamer-doc-token'  @click='renamerDoc = v'>
                                    <RenamerTokenName :token='v' :class='{"text-primary": renamerDoc == v}'></RenamerTokenName>
                                </div>
        
                                <div class='q-my-md'></div>
        
                                <div class='text-h6 text-grey-4 q-py-sm'>Properties</div>
                                <div v-for='(v, i) in $1t.info.value.renamerDocs.properties' :key='"RDP"+i' class='renamer-doc-token'  @click='renamerDoc = v'>
                                    <RenamerTokenName :token='v' :class='{"text-primary": renamerDoc == v}'></RenamerTokenName>
                                </div>
        
                                <div class='q-my-md'></div>
        
                                <div class='text-h6 text-grey-4 q-py-sm'>Functions</div>
                                <div v-for='(v, i) in $1t.info.value.renamerDocs.functions' :key='"RDF"+i' class='renamer-doc-token'  @click='renamerDoc = v'>
                                    <RenamerTokenName :token='v' :type='false' :class='{"text-primary": renamerDoc == v}'></RenamerTokenName>
                                </div>
                            </div>
                        </div>

                        <!-- Docs -->
                        <div class='col-9 q-pl-xl'>
                            
                            <!-- Not selected -->
                            <div v-if='!renamerDoc' class='text-center'>
                            
                                <div class='text-subtitle1 text-center text-primary text-bold text-uppercase'>Getting started with Auto Rename</div>
                                <div class='q-mt-xs text-center'>
                                    <q-badge color='primary'>
                                        <span class='text-number text-bold text-dark'>{{page+1}} / {{pages}}</span>
                                    </q-badge>
                                </div>      

                                
                                <div class='text-subtitle2 text-grey-4 q-mt-md text-center' style='line-height: 24px'></div>
                                <div class='q-my-sm text-grey-4'>
                                    Automatically rename your files based on tags.<br>
                                    The template string defines the format scheme of the new filename.                                    
                                </div>

                                <div>
                                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>Dynamic content</div>
                                    <div class='text-grey-4'>
                                        The template string describes the filename scheme for the rename operation based on the tag information.<br>
                                        For example, you can use the following variables:
                                    </div>
    
                                    <div class='q-mt-sm' style="padding-left: 64px;">
                                        <div style='width: 240px; margin: auto;' class='row'>
                                            <div class='col-6 text-left text-grey-4'>
                                                <span class='monospace'>%album%</span><br>
                                                <span class='monospace'>%artist%</span><br>
                                                <span class='monospace'>%title%</span><br>
                                                <span class='monospace'>%track%</span><br>
                                            </div>
                                            <div class='col-6 text-left text-grey-4'>
                                                <span>Album</span><br>
                                                <span>Artist</span><br>
                                                <span>Title</span><br>
                                                <span>Track</span><br>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                <div class='q-my-sm text-grey-4'>
                                    Any other variable that is not listed here, you can look up at the left section for a full overview.
                                    Also shows you some typical functions that can be used based on so called regex expressions.
                                </div>    

                                <div class='q-my-sm'>
                                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>Static content</div>
                                    <div class='text-grey-4'>
                                        Any you would like to add to your filenames basically.
                                        Just add it outside the <span class='monospace'>%variable%</span>
                                    </div>
                                </div>

                                <div>
                                    <div class='col-2 text-subtitle2 text-primary text-bold q-mt-md'>Examples</div>
                                    <HelpRenamerExamples></HelpRenamerExamples>
                                </div>

                                <div class='bg-darker text-subtitle2 text-grey-5 q-py-sm q-my-md text-center' style='line-height: 24px'>
                                    Use &nbsp;<q-badge outline color='grey-5'><span class='text-grey-4'>/</span></q-badge>&nbsp; to define a folder (on Windows too).
                                </div>
                                
                            </div>

                            <!-- Selected -->
                            <div v-if='renamerDoc'>
                                <RenamerTokenName :token='renamerDoc' class='text-h5'></RenamerTokenName>
                                <br>
                                <div class='text-caption q-mt-sm q-mb-md'>{{renamerDoc.kind.toUpperCase()}}</div>
                                <div style='font-size: 120%;'>
                                    <div v-html='renamerDoc.doc'></div>
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
import DJAppIcons from './DJAppIcons.vue';
import RenamerTokenName from './RenamerTokenName.vue';
import HelpRenamerExamples from './HelpRenamerExamples.vue';
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

// Hide help
function hide() {
    page.value = 0;
    $1t.helpDialog.value.open = false;
    $1t.helpDialog.value.route = undefined;
}

// Global override for homescreen
const route = computed(() => {
    if ($1t.helpDialog.value.route) return $1t.helpDialog.value.route;
    return useRoute().path.substring(1).split("/")[0];
});

// Get page count
const pages = computed(() => {
    if (route.value == 'tageditor' || route.value == 'renamer') return 1;
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
</style>