<template>
<div>

    <!-- Button -->
    <div class='help-button-container' @click='show = true' v-if='showButton'>
        <q-btn color='primary' class='q-px-sm help-button text-bold'>
            <q-icon name='mdi-help-circle-outline' class='q-pr-sm' size='xs'></q-icon>
            Help
        </q-btn>
    </div>

    <!-- Dialog -->
    <q-dialog v-model='show' @hide='hide'>
        <q-card class='help-card text-center'>

            <!-- Page controls -->
            <q-btn 
                class='help-page-icon clickable help-page-icon-forward'
                flat
                round
                @click='page += 1'
                v-if='page < (pages-1)'
                ><q-icon name='mdi-chevron-right' size='xl'></q-icon>
            </q-btn>
            <q-btn 
                class='help-page-icon clickable help-page-icon-back'
                flat
                round
                @click='page -= 1'
                v-if='page > 0'
                ><q-icon name='mdi-chevron-left' size='xl'></q-icon>
            </q-btn>

            <!-- Auto tagger -->
            <div v-if='route == "autotagger"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase'>Getting started with Auto Tag</div>
                    <div class='q-mt-xs'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-grey-9'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>                  

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 text-grey-4 q-mt-md'>
                            Auto Tag lets you automatically tag from Beatport, Discogs, Junodownload & Traxsource,
                            to your local audio files, based on Artist & Title tag.
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>How does it work?</div>
                        <div class='text-subtitle2 text-grey-4'>
                            It reads the Artist & Title tag from your local MP3, AIFF and FLAC files, <br>
                            feeds it into the search engines of the several platforms* <br> and writes the data to your tags.<br><br>
                        <p>                    
                            *<span @click='$1t.url("https://beatport.com")' class='clickable text-primary'>Beatport</span>, <span @click='$1t.url("https://junodownload.com")' class='clickable text-primary'>Juno Download</span> & <span @click='$1t.url("https://traxsource.com")' class='clickable text-primary'>Traxsource</span> are based on a method called scraping. <br>
                            <span @click='$1t.url("https://discogs.com")' class='clickable text-primary'>Discogs</span> is based on its API. Therefor it needs you to sign up for a free account.
                        </p>  
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>What?</div>
                        <div class='text-subtitle2 text-grey-4'>
                            It can fetch the following metadata (and based on the user input,
                            it can append, overwrite or fill in when empty):
                        </div>
                        <div class='row text-subtitle2 text-grey-4 q-pt-sm' style='width: 50%; margin-left: 25%;'>
                            <div class='col-6 q-mt-xs'>TITLE</div>  <div class='col-6 q-mt-xs'>ARTIST</div>
                            <div class='col-6 q-mt-xs'>ALBUM</div>  <div class='col-6 q-mt-xs'>LABEL</div>
                            <div class='col-6 q-mt-xs'>BPM</div>  <div class='col-6 q-mt-xs'>KEY *</div>
                            <div class='col-6 q-mt-xs'>GENRE</div>  <div class='col-6 q-mt-xs'>STYLE *</div>
                            <div class='col-6 q-mt-xs'>RELEASE DATE</div>  <div class='col-6 q-mt-xs'>PUBLISH DATE *</div>
                            <div class='col-6 q-mt-xs'>OTHER URL(s)</div>  <div class='col-6 q-mt-xs'>ALBUM ART</div>
                        </div>
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-sm q-my-md'>
                            * Key is only available from Beatport & Traxsource <br>
                            * Style is only available from Discogs <br>
                            * Publish Date is only available from Beatport <br>                            
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='text-subtitle2 text-primary text-bold q-mt-sm'>Step-by-step guide</div>
                        <div class='text-subtitle2 text-grey-4'>
                            <span class='text-number text-bold text-grey-6'>1. </span>Use the checkbox to enable/disable to fetch from particular platform. <br>
                            <span class='text-number text-bold text-grey-6'>2. </span>Drag & drop the cards to reorder fallback. <br>
                            <span class='text-number text-bold text-grey-6'>3. </span>Select folder to process (subfolders included) by clicking on the icon or copy/paste the path directly. <br>
                            <span class='text-number text-bold text-grey-6'>4. </span>Use the checkboxes to enable/disable to fetch the stated tag & hit ‘Next’. <br>
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Beatport</div>
                        <div class='text-subtitle2 text-grey-4'>
                            <span class='text-number text-bold text-grey-6'>5. </span>Select Album art resolution (500x500 is native resolution; above will upscale). <br>
                            <span class='text-number text-bold text-grey-6'>6. </span>Drag slider to determine amount of search page results to scan for the most corresponding track. <br>
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Discogs</div>
                        <div class='text-subtitle2 text-grey-4'>
                            <span class='text-number text-bold text-grey-6'>7. </span>Enter token. To obtain token, create a free account on <span @click='$1t.url("https://discogs.com")' class='clickable text-primary'>discogs.com</span> <br>
                            Go to <span @click='$1t.url("https://www.discogs.com/settings/developers")' class='clickable text-primary'>discogs.com/settings/developers</span> and click ‘Generate token’. <br>
                            Select it & copy/paste the current token - <span @click='$1t.url("https://youtu.be/IvAiMkfdLCw")' class='clickable text-primary'>see video demo</span>.<br>
                            <span class='text-number text-bold text-grey-6'>8. </span>Select Genres/Styles tag to either fetch genre, style or both and if it should merge them, or write elsewhere. <br>
                            <span class='text-number text-bold text-grey-6'>9. </span>Drag slider to determine amount of album search results to check. Hit ‘Next’. <br>
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Advanced</div>
                        <div class='text-subtitle2 text-grey-4'>
                            <span class='text-number text-bold text-grey-6'>10. </span>Set the settings to your likings. Hover over the <q-icon name='mdi-help-circle-outline' class='q-pb-sm'></q-icon> icons for more info. <br>
                            Hit ‘Start’ to process.
                        </div>
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-sm q-my-md'>
                            On top it shows amount of time taken + amount not matched + amount matched. <br>
                            In the middle section it will populate which tracks were not matched. <br>
                            At the bottom a bar will grow indicating the progression. <br>
                        </div>

                    </div>

                </q-card-section>
            </div>

            <!-- Audio features -->
            <div v-if='route == "audiofeatures"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase'>Getting started with Audio Features</div>
                    <div class='q-mt-xs'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-grey-9'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 text-grey-4 q-mt-md'>
                            Audio Features lets you automatically tag Spotify’s so called audio features to your local audio files, based on ISRC & exact match.
                            
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>How does it work?</div>
                        <div class='text-subtitle2 text-grey-4'>
                            Follow the setup described on the Audio Features entrance - <span @click='$1t.url("https://youtu.be/i0q5qWQSH9Y")' class='clickable text-primary'>see video demo</span>. <br>
                            Select a folder with tracks you want to get their audio features fetched for. <br>
                            Based on the <span class='clickable text-primary' @click='$1t.url(isrcWiki)'>ISRC</span> inside the metadata it will search up the track in Spotify's API and return these values. <br>
                            If no ISRC exists, it will search by artist and title tag using an exact match. <br>
                            Eventually it will write the audio features to the metadata and its value derived from Spotify.
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>What?</div>
                        <div class='text-subtitle2 text-grey-4'>In a nutshell audio features are:</div>
                        <div class='row text-subtitle2 q-py-sm text-left' style='width: 60%; margin-left: 20%;'>
                            <div class='col-4 text-grey-4'>ACOUSTICNESS</div>     <div class='text-right text-grey-4 col-8'>acoustic <span class='text-grey-6'>vs.</span> electronic</div> 
                            <div class='col-4 text-grey-4'>DANCEABILITY</div>     <div class='text-right text-grey-4 col-8'>dynamics-low <span class='text-grey-6'>vs.</span> dynamics-med <span class='text-grey-6'>vs.</span> dynamics-high</div> 
                            <div class='col-4 text-grey-4'>ENERGY</div>           <div class='text-right text-grey-4 col-8'>energy-low <span class='text-grey-6'>vs.</span> energy-med <span class='text-grey-6'>vs.</span> energy-high</div> 
                            <div class='col-4 text-grey-4'>INSTRUMENTALNESS</div> <div class='text-right text-grey-4 col-8'>vocal-low <span class='text-grey-6'>vs.</span> vocal-med <span class='text-grey-6'>vs.</span> vocal-high</div> 
                            <div class='col-4 text-grey-4'>LIVENESS</div>         <div class='text-right text-grey-4 col-8'>live <span class='text-grey-6'>vs.</span> recording</div> 
                            <div class='col-4 text-grey-4'>SPEECHINESS</div>      <div class='text-right text-grey-4 col-8'>speech <span class='text-grey-6'>vs.</span> music</div> 
                            <div class='col-4 text-grey-4 q-mb-xs'>VALENCE</div>          <div class='text-right text-grey-4 col-8'>positive <span class='text-grey-6'>vs.</span> neutral <span class='text-grey-6'>vs.</span> negative <span class='text-grey-4'>(happy <span class='text-grey-6'>vs.</span> sad/angry)</span>  </div>
                        </div>
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md'>
                            For a more in-depth description, check out <span class='clickable text-primary' @click='$1t.url("https://developer.spotify.com/documentation/web-api/reference/#object-audiofeaturesobject")'>Spotify’s official AudioFeaturesObject</span> section.<br>
                            To look up audio features online for other tracks, check out <span class='text-primary clickable' @click='$1t.url("https://songdata.io")'>SongData.io</span> - Ignore the <span class='text-primary clickable' @click='$1t.url("https://www.reddit.com/r/DJs/comments/m3q97z/key_detection_comparison_spotify_vs_tunebat_vs/")'>keys determined by Spotify</span> though.
                        </div>
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='text-subtitle2 text-primary text-bold q-mt-sm'>Prominent tag</div>
                        <div class='text-subtitle2 text-grey-4'>
                            You can customize the threshold, so it doesn't only write the simple value from 0-100 per audio feature, <br>
                            but also determine when it should translate the value into a meaningful description and write to a custom predefined tag field. <br>
                            Look up which tag code represents what tag name by clicking on the icon of your DJ app below. <br>
                            Also shows you recommended tags to use.
                        </div>

                        <div class='row q-mx-xl q-mt-md justify-around'>
                            <div class='col-3 q-mb-xs'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Acousticness</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>0*</span>
                                    </q-badge>  <span class='text-grey-10'>-</span>#electronic <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>90-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#acoustic
                                </span>
                            </div>
                            <div class='col-3'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Danceability</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>0-20</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#dynamics-high <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>21-79</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#dynamics-med <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>80-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#dynamics-low <br>
                                </span>
                            </div>
                            <div class='col-3'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Energy</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>0-20</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#energy-low <br>
                                      <q-badge color='grey-8'>
                                        <span class='textnumber'>21-89</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#energy-mid <br>
                                      <q-badge color='grey-8'>
                                        <span class='textnumber'>90-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#energy-high
                                </span>
                            </div>

                            <div class='col-3'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Instrumentalness</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>0-50</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#vocal-high <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>51-89</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#vocal-med <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>90-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#vocal-low
                                </span>
                            </div>

                            <div class='col-3 q-mt-md'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Liveness</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                 <q-badge color='grey-8'>
                                        <span class='textnumber'>0*</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#recording <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>80-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#live <br>
                                </span>                                
                            </div>

                            <div class='col-3 q-mt-md'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Speechiness</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                 <q-badge color='grey-8'>
                                        <span class='textnumber'>0*</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#music <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>70-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#speech <br>
                                </span>                                
                            </div>
                            <div class='col-3 q-mt-md'>
                                <span class='text-subtitle2 text-primary text-bold q-mt-md'>Valence</span><br>
                                <span class='text-subtitle2 text-grey-6'>Threshold default: </span><br>
                                <span class='text-subtitle2 text-grey-4'>
                                 <q-badge color='grey-8'>
                                        <span class='textnumber'>0-15</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#negative <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>16-84</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#neutral <br>
                                    <q-badge color='grey-8'>
                                        <span class='textnumber'>85-100</span>
                                    </q-badge> <span class='text-grey-10'>-</span>#positive <br>
                                </span>                                
                            </div>
                        </div>
                            <div class='text-subtitle2 q-my-md text-grey-6'>
                                0* = When threshold is 0, it won't write the prominent tag.
                            </div>

                        
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md'>
                            <DJAppIcons></DJAppIcons>
                        </div>
                    </div>

                </q-card-section>
            </div>


            <!-- Quicktag -->
            <div v-if='route == "quicktag"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase'>Getting started with Quick Tag</div>
                    <div class='q-mt-xs'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-grey-9'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                        <div class='text-subtitle2 text-grey-4 q-mt-md'>
                            Quick Tag lets you manually tag your local audio files, based on energy, mood, genre & other characteristics you defined.<br>
                            Key bind compatible for quick access.
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>How does it work?</div>
                        <div class='text-subtitle2 qt-mb-xs text-grey-4'>
                            Ever felt like the genre of downloaded tracks are simply not what you think it is?<br>
                            Ever felt like you wanted the mood of a song in your tag too?<br>
                            Ever felt like you wanted other characteristics of a track showing in your DJ app?<br>
                            <br>
                            Set the path to a folder with tracks you like to quick tag. Predefine energy, moods, genres and other characteristics. <br>
                            Decide in which tag you want them to write, so you are able to read them out in your favorite DJ software. <br>
                            Key bind them for quick access. Skip through a song to determine the mentioned elements. <br>
                            Hit the key binds (or point ’nd click).<br>                            
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Step-by-step guide</div>
                        <div class='text-subtitle2 text-grey-4'>
                            <span class='text-number text-bold text-grey-6'>1. </span>On top right click the <q-icon name='mdi-cog' class='q-mb-xs q-pr-xs'></q-icon>icon. <br>
                            <span class='text-number text-bold text-grey-6'>2. </span>Set path to a folder which you like to quick tag. <br>
                            <span class='text-number text-bold text-grey-6'>3. </span>Predefine energy level to write to either Rating or a tag field (as symbol). <br>
                            <span class='text-number text-bold text-grey-6'>4. </span>Predefine moods (and color code them), genres and other characteristics. <br>
                            <span class='text-number text-bold text-grey-6'>5. </span>Key bind them all to a character on your keyboard for quick access. <br>
                            <span class='text-number text-bold text-grey-6'>6. </span>Decide where you want all the above data written to. <br>
                            <span class='text-number text-bold text-grey-6'>7. </span>Play a song and listen - or quickly skip through a track using the arrow keys on your keyboard. <br>
                            <span class='text-number text-bold text-grey-6'>8. </span>Tag ‘em quickly using the keyboard key binds (or point ’nd click). <br>
                        </div>
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md'>
                            For inspiration, check out the <span class='text-primary clickable' @click='$1t.url("https://www.reddit.com/r/DJs/comments/c3o2jk/my_ultimate_track_tagging_system_the_little_data/")'>Little Data, Lotta Love </span>tagging system by u/nonomomomo
                        </div> 
                        
                    </div>

                    <!-- Page 2 -->
                    <div v-if='page == 1'>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Energy</div>
                        <div class='text-subtitle2 text-grey-4'>
                            Tag energy level using stars. <br>
                            Set it to 1-3-5 stars to show low-mid-high energy level, or go with the 1-2-3-4-5 scale.  <br>
                            Some DJ software can’t read out the Rating tag, or simply don’t have the ability to show it.  <br>
                            Then you can alternatively set the output to ‘symbol’ instead. Predefine the symbol, (an asterisk * for example).  <br>
                            Also predefine where to write the energy rating by entering the proper tag code. <br>                            
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Mood</div>
                        <div class='text-subtitle2 text-grey-4'>
                            Predefine your moods and additionally color code them. Use key bindings for quick access.  <br>
                            For inspiration what moods you could use, see this <span class='text-primary clickable' @click='$1t.url(moodOverview)'>Moods and other characteristics overview.</span> <br>
                            Since the native mood tag isn’t read by any DJ software, we need to write it elsewhere. <br>
                            Look up which tag code represents what tag name by clicking on the icon of your DJ app below. <br>
                            Also shows you recommended tags to use.
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-md'>Genre</div>
                        <div class='text-subtitle2 text-grey-4'>
                            If the genre stated in the original genre tag of your tracks (or the ones fetched with Auto Tag) are still not to your liking, <br>
                            predefine them in Settings and use key bindings for quick access.<br>
                            <q-icon name='mdi-information-outline text-grey-6' class='q-pb-sm q-pr-xs'></q-icon>
                            <span class='text-grey-6'>Pro-tip: Use <span class='monospace'>SHIFT + character</span> for genres. This way you can keep <span class='monospace'>characters</span> for moods.</span>
                            
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-sm'>Custom</div>
                        <div class='text-subtitle2 text-grey-4'>
                            Whatever you want! For example focus on different elements like vibe, instruments, vocals, time to play or situation, etc. 
                        </div>
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md'>
                            <DJAppIcons></DJAppIcons>
                        </div>
                                               
                    </div>

                </q-card-section>
            </div>

            <!-- Tag editor -->
            <div v-if='route == "tageditor"'>
                <q-card-section class='q-pa-xl'>
                    <div class='text-subtitle1 text-primary text-bold text-uppercase'>GETTING STARTED WITH EDIT TAGS</div>
                    <div class='q-mt-xs'>
                    <q-badge color='primary'>
                        <span class='text-number text-bold text-grey-9'>{{page+1}} / {{pages}}</span>
                    </q-badge>
                    </div>    

                    <!-- Page 1 -->
                    <div v-if='page == 0'>
                    
                        <div class='text-subtitle2 text-grey-4 q-mt-md'>
                            Edit Tags lets you do what it says... Edit tags.<br>
                            It reads the metadata of MP3, AIFF and FLAC files within a folder.<br>
                            You can review what tags got written where inside the metadata, edit them, add a new tag or delete.<br>
                        </div>
                        <div class='text-subtitle2 text-primary text-bold q-mt-sm'>How does it work?</div>
                        <div class='text-subtitle2 text-grey-4'>
                            Click the path on top left to select folder using your OS's picker.<br>
                            A list of all audio files within the folder will be populated.<br>
                            Click on a file to show the metadata and its tag codes sorted by alphabet*.<br>
                            Review or alter them to your liking. <br>
                            Hit Save on the bottom to write the changes made.<br>
                        </div>
                        <div class='text-subtitle2 q-my-md text-grey-6'>
                            *Comments, (Unsynchronized) lyrics & Popularimeter (Rating) are special tag fields.<br>
                            They show at the bottom below the album art.
                        </div>                           
                        <div class='text-subtitle2 text-primary text-bold q-mt-sm'>Your list</div>
                        <div class='text-subtitle2 text-grey-4'>
                            If you only want to show a few audio files, or from multiple folders,<br>
                            toggle <span class='text-italic'>Show 'Your list' in Edit Tags</span> by clicking the <q-icon name='mdi-cog' class='q-mb-xs q-pr-xs'></q-icon>icon > Display > General.<br>
                            This allows you to drag 'nd drop audio files from the treelist into 'Your list'.
                        </div>
                        <div class='bg-grey-10 text-subtitle2 text-grey-6 q-px-md q-py-md q-my-md'>
                            The path selected in Quick Tag will overwrite the path of Edit Tags.<br>
                            This way it becomes easy to review where your custom tags got written, by switching between the two tabs.
                        </div>

                    </div>

                </q-card-section>
            </div>

        </q-card>
    </q-dialog>

</div>
</template>

<script>
import DJAppIcons from './DJAppIcons';

export default {
    name: 'HelpButton',
    components: { DJAppIcons },
    data() {
        return {
            show: false,
            page: 0,
            metadataMatrix: 'https://docs.google.com/spreadsheets/d/1zhIJPOtYIueV72Gd81aVnbSa6dIA-azq9fnGC2rHUzo/edit?usp=sharing',
            moodOverview: 'https://docs.google.com/spreadsheets/d/1wYokScjoS5Xb1IvqFMXbSbknrXJ7bySLLihTucOS4qY/edit?usp=sharing',
            isrcWiki: 'https://en.wikipedia.org/wiki/International_Standard_Recording_Code'
        }
    },
    methods: {
        hide() {
            this.page = 0;
            this.$1t.helpDialog.open = false;
            this.$1t.helpDialog.route = null;
        }
    },
    computed: {
        route() {
            //Global override for homescreen
            if (this.$1t.helpDialog.route) return this.$1t.helpDialog.route;
            return this.$route.path.substring(1).split("/")[0];
        },
        pages() {
            if (this.route == 'tageditor') return 1;
            return 2;
        },
        //Show/Hide button
        showButton() {
            if (this.$1t.helpDialog.open || !this.route || 
                this.$route.path.includes('/status') || !this.$1t.settings.helpButton) return false;
            return true;
        },
    },
    watch: {
        '$1t.helpDialog.open'() {
            if (this.$1t.helpDialog.open) {
                this.show = true;
            }
        }
    }
}
</script>

<style>
.help-button-container {
    position: fixed;
    right: -36px;
    top: 50%;
    /* Drawers */
    z-index: 1002;
    transform: rotate(270deg);
}

.help-button {
    opacity: 0.5;
}
.help-button:hover {
    opacity: 1.0;
}

@media screen and (max-width: 1200px) {
    .help-card {
       width: 80%;
       min-width: 80%;
    }
}
@media screen and (min-width: 1200px) {
    .help-card {
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
</style>