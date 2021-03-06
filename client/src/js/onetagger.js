import Vue from 'vue';
import mergeOptions from 'merge-options';
import { Notify, colors } from 'quasar';
import { QTTrack } from './quicktag';

class OneTagger {

    constructor() {
        this.WAVES = 180;
        this.ws = new WebSocket(`ws://${window.location.hostname}:36912`);
        this._wsPromiseResolve;
        this.wsPromise = new Promise((res) => this._wsPromiseResolve = res);

        // WS error
        this.ws.onerror = (_, e) => {
            this.onError(e ?? "WebSocket error!");
        };
        this.ws.onclose = () => {
            this.onError('WebSocket closed!');
        }
        // Load settings on load
        this.ws.onopen = () => {
            // Mark WS as connected
            if (this._wsPromiseResolve) {
                this._wsPromiseResolve();
                this._wsPromiseResolve = null;
            }
            // init data
            this.send('loadSettings');
            setTimeout(() => {
                this.send('init');
                this.send('spotifyAuthorized');
            }, 100);
        }

        this.info = Vue.observable({
            version: '0.0.0', 
            os: null, 
            ready: false,
            platforms: [],
            renamerDocs: null
        });

        // WS Message handler
        this.ws.onmessage = (event) => {
            let json = JSON.parse(event.data);
            if (!json.action) return;

            // Action
            switch (json.action) {
                // Initial info
                case 'init':
                    this.info.version = json.version;
                    this.info.os = json.os;
                    // Path from args
                    if (json.startContext.startPath) {
                        this.settings.path = json.startContext.startPath;
                        this.config.path = json.startContext.startPath;
                    }
                    this.info.platforms = json.platforms;
                    this.info.renamerDocs = json.renamerDocs;

                    // restore custom platform fields
                    for (const [key, value] of Object.entries(this.config.custom)) {
                        for (let platform of this.info.platforms) {
                            if (platform.platform.id == key) {
                                // restore keys
                                for (const [id, newValue] of Object.entries(value)) {
                                    for (let i in platform.platform.customOptions.options) {
                                        if (platform.platform.customOptions.options[i].id == id) {
                                            platform.platform.customOptions.options[i].value.value = newValue.value;
                                        }
                                    }
                                }

                            }
                        }
                    }

                    this.info.ready = true;
                    break;
                // Settings loaded
                case 'loadSettings':
                    this.loadSettings(json.settings);
                    break;
                // Path selected
                case 'browse':
                    this.onBrowse(json);
                    break;
                // Error
                case 'error':
                    // Unlock, callback
                    this.lock.locked = false;
                    this.onError(json.message);
                    break;
                case 'startTagging':
                    this.lock.locked = true;
                    this.taggerStatus.statuses = [];
                    this.taggerStatus.started = Date.now();
                    this.taggerStatus.done = false;
                    this.taggerStatus.ok = 0;
                    this.taggerStatus.progress = 0.0;
                    this.taggerStatus.total = json.files;
                    this.taggerStatus.type = json.type;
                    this.taggerStatus.data = null;
                    break;
                // Status
                case 'taggingProgress':
                    this.taggerStatus.progress = json.status.progress;
                    // De duplicate failed
                    this.taggerStatus.statuses = this.taggerStatus.statuses.filter((s) => {
                        return s.status.path != json.status.status.path;
                    });
                    this.taggerStatus.statuses.unshift(json.status);
                    
                    break;
                // Tagging done
                case 'taggingDone':
                    this.lock.locked = false;
                    this.taggerStatus.done = true;
                    this.taggerStatus.progress = 1.0;
                    this.taggerStatus.data = json.data;
                    this.onTaggingDone(json.path);
                    break;
                // Player load track
                case 'playerLoad':
                    this.player.duration = json.duration;
                    this.player.position = 0;
                    this.player.playing = false;
                    this.player.title = json.title;
                    this.player.artists = json.artists;
                    break;
                case 'playerSync':
                    this.player.playing = json.playing;
                    break;
                // Quicktag
                case 'quickTagLoad':
                    this.lock.locked = false;
                    this.quickTag.tracks = json.data.files.map(t => new QTTrack(t, this.settings.quickTag));
                    this.quickTag.failed = json.data.failed;
                    this.onQuickTagEvent('quickTagLoad');
                    break;
                /*eslint-disable no-case-declarations*/
                case 'quickTagSaved':
                    let i = this.quickTag.tracks.findIndex((t) => t.path == json.path);
                    if (i != -1) {
                        Vue.set(this.quickTag.tracks, i, new QTTrack(json.file, this.settings.quickTag));
                    } else {
                        // this.onError('quickTagSaved: Invalid track');
                    }
                    // Force reload current track
                    if (this.quickTag.track && json.path == this.quickTag.track.path) {
                        this.onQuickTagEvent('changeTrack', { offset: 0, force: true });
                    }

                    break;
                // Browser folder
                case 'quickTagFolder':
                    this.onQuickTagBrowserEvent(json);
                    break;
                // Spotify
                case 'spotifyAuthorized':
                    this.onSpotifyAuthEvent(json);
                    break;
                // Folder browser
                case 'folderBrowser':
                    this.onFolderBrowserEvent(json);
                    break;
                // Debug
                default:
                    // Tag editor
                    if (json.action.startsWith('tagEditor')) {
                        this.onTagEditorEvent(json);
                        break;
                    }
                    // Renamer
                    if (json.action.startsWith('renamer')) {
                        this.onRenamerEvent(json);
                        break;
                    }

                    console.log(json);
                    break;
            }
        }

        // Webview/OS message. vue = instance
        this.onOSMessage = (json, vue) => {
            switch (json.action) {
                // Drag and drop path
                case 'browse':
                    // Callback by route
                    let route = vue.$router.currentRoute.path.substring(1).split('/')[0];
                    switch (route) {
                        case 'autotagger':
                            this.config.path = json.path;
                            break;
                        case 'audiofeatures':
                            this.onAudioFeaturesEvent(json);
                            break;
                        case 'tageditor':
                            this.onTagEditorEvent(json);
                            break;
                        case 'quicktag':
                            this.settings.path = json.path;
                            this.loadQuickTag(null);
                            break;
                        case 'renamer':
                            this.onRenamerEvent(json);
                            break;
                        default:
                            this.settings.path = json.path;
                            break;
                    }
                    break;
                default:
                    console.log(`Unknown OS action: ${json}`);
                    break;
            }
        }

        // Default autotagger config
        this.config = Vue.observable({
            "platforms": ["beatport"],
            "path": null,
            "title": false,
            "artist": false,
            "albumArtist": false,
            "album": false,
            "key": false,
            "bpm": true,
            "genre": true,
            "style": true,
            "label": true,
            "duration": false,
            "releaseDate": true,
            "publishDate": false,
            "albumArt": false,
            "otherTags": false,
            "url": false,
            "trackId": false,
            "releaseId": false,
            "version": false,
            "remixer": false,
            "trackNumber": false,
            "trackTotal": false,
            "discNumber": false,
            "catalogNumber": false,
            "isrc": false,
            "mood": false,
            "metaTags": false,
            "separators": {id3: ', ', vorbis: null, mp4: ', '},
            "id3v24": true,
            "overwrite": true,
            "threads": 16,
            "strictness": 0.7,
            "mergeGenres": false,
            "albumArtFile": false,
            "camelot": false,
            "parseFilename": false,
            "filenameTemplate": "%artists% - %title%",
            "shortTitle": false,
            "matchDuration": false,
            "maxDurationDifference": 30,
            "matchById": false,
            "multipleMatches": "Default",
            "postCommand": null,
            "stylesOptions": "default",
            "trackNumberLeadingZeroes": 0,
            "enableShazam": false,
            "forceShazam": false,
            "skipTagged": false,
            "onlyYear": false,
            "includeSubfolders": true,
            "stylesCustomTag": {vorbis: 'STYLE', id3: 'STYLE', mp4: 'STYLE'},
            "custom": {},
            "spotify": null
        });
        // Statuses
        this.taggerStatus = Vue.observable({
            statuses: [],
            started: 0,
            progress: 0.0,
            done: false,
            total: 0,
            type: null,
            data: null
        });
        // Lock, enable when tagging/loading
        this.lock = Vue.observable({locked: false});

        // Player
        this.player = Vue.observable({
            waveform: [],
            playing: false,
            position: 0,
            duration: 1,
            volume: 0.5,
            wasPlaying: false,
            title: null,
            artists: [],
            audio: new Audio(),
        });
        this.generateDefaultWaveform();
        // Player position updater
        setInterval(() => {
            if (this.player.playing)
                this.player.position += 150;
        }, 150);

        // Quick tag
        this.quickTag = Vue.observable({
            tracks: [],
            track: null,
            failed: 0
        });

        // Settings for UI
        this.settings = Vue.observable({
            path: null,
            autoTaggerConfig: {},
            autoTaggerSinglePage: false,
            primaryColor: '#00D2BF',
            discogsToken: null,
            volume: 0.05,
            helpButton: true,
            continuePlayback: false,
            clientSidePlayer: false,
            nonNativeBrowser: false,
            renamer: {
                path: null,
                outDir: null,
                template: null,
                copy: false,
                subfolders: true,
                overwrite: false
            },
            quickTag: {
                id3v24: false,
                recursive: false,
                autosave: false,
                sortDescending: false,
                sortOption: 'title',
                trackIndex: -1,
                separators: {id3: ', ', vorbis: null, mp4: ', '},
                noteTag: {
                    tag: {
                        id3: 'COMM', 
                        vorbis: 'COMMENT',
                        mp4: '??cmt'
                    }, 
                    keybind: null
                },
                energyKeys: [null,null,null,null,null],
                moods: [
                    {mood: 'Happy', color: 'amber', keybind: null},
                    {mood: 'Sad', color: 'indigo', keybind: null},
                    {mood: 'Bright', color: 'green', keybind: null},
                    {mood: 'Dark', color: 'deep-purple', keybind: null},
                    {mood: 'Angry', color: 'red', keybind: null},
                    {mood: 'Chill', color: 'teal', keybind: null},
                    {mood: 'Lovely', color: 'pink', keybind: null},
                    {mood: 'Powerful', color: 'light-blue', keybind: null},
                    {mood: 'Sexy', color: 'purple', keybind: null}
                ],
                moodTag: {vorbis: 'MOOD', id3: 'TMOO', mp4: 'iTunes:MOOD'},
                energyTag: {
                    //rating = save to rating tag, symbol = save to custom tag with symbols
                    type: 'rating',
                    symbol: '*',
                    tag: {
                        id3: 'TCOM',
                        vorbis: 'COMPOSER',
                        mp4: '??wrt'
                    }
                },
                genres: [
                    {genre: '2-step', keybind: null, subgenres: []},
                    {genre: 'Acid', keybind: null, subgenres: []},
                    {genre: 'Breakbeat', keybind: null, subgenres: []},
                    {genre: 'Disco', keybind: null, subgenres: []},
                    {genre: 'Drum & Bass', keybind: null, subgenres: []},
                    {genre: 'Electro', keybind: null, subgenres: ['House', 'Dubstep', 'EDM']},
                    {genre: 'Funk', keybind: null, subgenres: []},
                    {genre: 'Hardcore', keybind: null, subgenres: []},
                    {genre: 'Hiphop', keybind: null, subgenres: []},
                    {genre: 'House', keybind: null, subgenres: []},
                    {genre: 'Industrial', keybind: null, subgenres: []},
                    {genre: 'Jungle', keybind: null, subgenres: []},
                    {genre: 'Latin', keybind: null, subgenres: []},
                    {genre: 'Minimal', keybind: null, subgenres: []},
                    {genre: 'Nu-Disco', keybind: null, subgenres: []},
                    {genre: 'Oldies', keybind: null, subgenres: []},
                    {genre: 'Pop', keybind: null, subgenres: []},
                    {genre: 'Reggae', keybind: null, subgenres: []},
                    {genre: 'Rock', keybind: null, subgenres: []},
                    {genre: 'Techno', keybind: null, subgenres: []},
                    {genre: 'Trance', keybind: null, subgenres: []},
                ],
                custom: [{
                    name: 'Vibe',
                    tag: {
                        id3: 'COMM',
                        vorbis: 'COMMENT',
                        mp4: '??cmt'
                    },
                    values: [
                        {val: 'Afro', keybind: null},
                        {val: 'Asian', keybind: null},
                        {val: 'Arabic', keybind: null},
                        {val: 'Classic', keybind: null},
                        {val: 'Dirty', keybind: null},
                        {val: 'Etnic', keybind: null},
                        {val: 'Funky', keybind: null},
                        {val: 'Gangsta', keybind: null},
                        {val: 'Glitchy', keybind: null},
                        {val: 'Melodic', keybind: null},
                        {val: 'Sensual', keybind: null},
                        {val: 'Soulful', keybind: null},
                    ],
                }, {
                    name: 'Situation',
                    tag: {
                        id3: 'COMM',
                        vorbis: 'COMMENT',
                        mp4: '??cmt'
                    },
                    values: [
                        {val: 'Start', keybind: null},
                        {val: 'Build', keybind: null},
                        {val: 'Peak', keybind: null},
                        {val: 'Sustain', keybind: null},
                        {val: 'Release', keybind: null},
                    ]
                }, {
                    name: 'Instruments',
                    tag: {
                        id3: 'COMM',
                        vorbis: 'COMMENT',
                        mp4: '??cmt'
                    },
                    values: [
                        {val: 'Vocals', keybind: null},
                        {val: 'Bass Heavy', keybind: null},
                        {val: 'Congas', keybind: null},
                        {val: 'Guitar', keybind: null},
                        {val: 'Horns', keybind: null},
                        {val: 'Organ', keybind: null},
                        {val: 'Piano', keybind: null},
                        {val: 'Strings', keybind: null},
                        {val: 'Sax', keybind: null},
                    ]
                }]
            },
            audioFeatures: {
                spotifyClientId: null,
                spotifyClientSecret: null,
                config: null
            },
            tagEditorDouble: false,
            tagEditorCustom: [],
            tagEditorAutosave: false,
            tagEditorPlayer: false,
        });

        // Managing spotify login
        this.spotify = Vue.observable({
            clientId: null,
            clientSecret: null,
            authorized: false
        })

        // If unsaved changes to track
        this._nextQTTrack = null;
        // Waveform loading lock
        this._waveformLock = [];
        this._waveformPath = null;

        // Keybinds
        document.addEventListener('keydown', (e) => {
            if (e.target.nodeName == "INPUT") return true;
            if (this.handleKeyDown(e)) {
                e.preventDefault();
                return false;
            }
        });

        // So can be triggered globally
        this.helpDialog = Vue.observable({open: false, route: null});

        // Because the config is global and playlist is passed in wrapper element
        this.autoTaggerPlaylist = Vue.observable({filename: null, data: null, format: null});

        // Open and close folder browser
        this.folderBrowser = Vue.observable({open: false, basePath: '/', context: null});
    }

    // SHOULD BE OVERWRITTEN
    quickTagUnfocus() {}
    onError(msg) {console.error(msg);}
    onTaggingDone() {}
    onQuickTagEvent() {}
    onQuickTagBrowserEvent() {}
    onTagEditorEvent() {}
    onAudioFeaturesEvent() {}
    onSpotifyAuthEvent() {}
    onRenamerEvent() {}
    onFolderBrowserEvent() {}

    // Send to socket
    async send(action, params = {}) {
        // Wait for connection
        if (this.wsPromise) {
            await this.wsPromise;
            this.wsPromise = null;
        }

        let data = { action };
        Object.assign(data, params);
        this.ws.send(JSON.stringify(data));
    }

    // Open URL in external browser
    url(url) {
        this.send("browser", {url});
    }

    // Open native folder browser
    browse(context, path) {
        if (this.settings.nonNativeBrowser) {
            this.folderBrowser.context = context;
            this.folderBrowser.basePath = path;
            this.folderBrowser.open = true;
            return;
        }
        this.send('browse', { context, path });        
    }

    // onBrowse event
    onBrowse(json) {
        // Autotagger path
        if (json.context == 'at')
            this.config.path = json.path;
        // Quicktag path
        if (json.context == 'qt') {
            Vue.set(this.settings, 'path', json.path);
            this.onQuickTagBrowserEvent({action: 'pathUpdate'});
            this.loadQuickTag();
        }
        // Audio features path
        if (json.context == 'af')
            this.onAudioFeaturesEvent(json);
        // Tag editor
        if (json.context == 'te')
            this.onTagEditorEvent(json);
        // Renamer
        if (json.context == 'rn' || json.context == 'rnOutput')
            this.onRenamerEvent(json)
    }

    // Save settings to file
    saveSettings(notif = true) {
        // Very dirty way to clone a dict, but eh
        this.settings.autoTaggerConfig = JSON.parse(JSON.stringify(this.config));
        this.settings.volume = this.player.volume;
        // Save
        this.send("saveSettings", {settings: JSON.parse(JSON.stringify(this.settings))});
        // Notification
        if (notif)
            Notify.create({
                message: "Settings saved!",
                color: 'primary',
                textColor: 'black',
                timeout: 500,
            });
    }
    // Load settings from JSON
    loadSettings(data) {
        // Load depper dicts separately
        Object.assign(this.settings.quickTag, data.quickTag);
        delete data.quickTag;
        Object.assign(this.settings, data);
        
        // AT config (nested)
        let config = mergeOptions({}, this.config, this.settings.autoTaggerConfig??{});
        Object.assign(this.config, config);
 
        // Restore specific
        this.player.volume = this.settings.volume??0.5;
        this.player.audio.volume = this.player.volume;
        this.setVolume(this.player.volume);
        colors.setBrand('primary', this.settings.primaryColor??'#00D2BF');
        if (!this.settings.tagEditorCustom) this.settings.tagEditorCustom = [];
        this.spotify.clientId = this.settings.audioFeatures.spotifyClientId;
        this.spotify.clientSecret = this.settings.audioFeatures.spotifyClientSecret;

        // Migrate to enable subgenres
        for (let i=0; i<this.settings.quickTag.genres.length; i++) {
            if (!this.settings.quickTag.genres[i].subgenres) {
                Vue.set(this.settings.quickTag.genres[i], 'subgenres', []);
            }
        }
    }

    // Wrapper to prevent multiple waveforms
    async generateWaveform(path) {
        this._waveformPath = path;
        // Aquire lock
        this._waveformLock.push(true);
        while (this._waveformLock.length > 1) {
            await new Promise((res) => setTimeout(() => res(), 50));
            if (path != this._waveformPath) {
                this._waveformLock.pop();
                return;
            }
        }
        await this._generateWaveform(path);
    }

    // Generate waveform
    async _generateWaveform(path) {
        this.generateDefaultWaveform();
        let waveformIndex = 0;
        // Separate socket = separate thread
        let ws = new WebSocket(`ws://${window.location.hostname}:36912`);
        ws.onmessage = (event) => {
            // Lock
            if (this._waveformLock.length > 1) {
                ws.close();
                this._waveformLock.pop();
                return;
            }

            let json = JSON.parse(event.data);
            // New wave
            if (json.action == 'waveformWave') {
                Vue.set(this.player.waveform, waveformIndex, json.wave);
                waveformIndex++;
            }
            // Finish
            if (json.action == 'waveformDone' || json.action == 'error') {
                ws.close();
                this._waveformLock.pop();

                // Autoplay, delay just in case for windows
                setTimeout(() => {
                    if (this.settings.continuePlayback && this.player.wasPlaying) {
                        this.play();
                        this.player.wasPlaying = false;
                    }
                }, 100);
            }
            // Will be ignored, just for updating
            ws.send(JSON.stringify({action: '_waveformRead'}));
        };
        ws.onopen = () => {
            ws.send(JSON.stringify({
                action: 'waveform',
                path
            }));
        };
    }

    // Default waveform
    generateDefaultWaveform() {
        for (let i=0; i<this.WAVES; i++) {
            this.player.waveform[i] = 0;
        }
    }

    // Load quicktag track
    loadQTTrack(track, force = false) {
        // Check for unsaved changes
        if (!this.quickTag.track || force || !this.quickTag.track.isChanged()) {
            if (!track)
                track = this._nextQTTrack;
            // For autoplay
            if (this.player.playing)
                this.player.wasPlaying = true;
            this.quickTag.track = new QTTrack(JSON.parse(JSON.stringify(track)), this.settings.quickTag);
            this.loadTrack(track.path);
            this._nextQTTrack = null;
            return;
        }
        // Prompt for unsaved changes
        this._nextQTTrack = track;
        this.onQuickTagEvent('onUnsavedChanges');
    }
    // Save quickTagTrack
    async saveQTTrack() {
        if (this.quickTag.track) {
            let changes = this.quickTag.track.getOutput();
            this.send('quickTagSave', {changes});
        }
    }

    // Player controls
    loadTrack(path) {
        // Setup client-side audio player
        if (this.settings.clientSidePlayer) {
            this.player.audio.pause();
            this.player.playing = false;
            this.player.audio = new Audio(`http://${window.location.hostname}:36913/audio?path=${encodeURIComponent(path)}`);
            this.player.audio.volume = this.player.volume;
            const cb = () => {
                this.player.playing = !this.player.audio.paused;
                this.player.position = Math.round(this.player.audio.currentTime * 1000);
            }
            // this.player.audio.addEventListener('play', cb);
            this.player.audio.addEventListener('playing', cb);
        }

        // Server side
        this.send("playerLoad", { path });
        this.generateWaveform(path);
    }
    play() {
        if (this.settings.clientSidePlayer) {
            this.player.playing = false;
            this.player.audio.play();
        } else {
            this.send("playerPlay");
            this.player.playing = true;
        }        
    }
    pause() {
        this.settings.clientSidePlayer ? this.player.audio.pause() : this.send("playerPause");
        this.player.playing = false;
        this.player.wasPlaying = false;
    }
    seek(pos) {
        this.player.playing = false;

        if (this.settings.clientSidePlayer) {
            this.player.audio.currentTime = pos / 1000.0;
        } else {
            this.send("playerSeek", {pos})
        }
        this.player.position = pos;
    }
    setVolume(volume) {
        if (!volume) return;
        this.player.audio.volume = volume;
        this.send("playerVolume", {volume});
    }

    // Quicktag
    loadQuickTag(playlist = null) {
        // Loading
        if (playlist || this.settings.path) {
            this.lock.locked = true;
            this.quickTag.tracks = [];
        }

        if (playlist) {
            this.send('quickTagLoad', {playlist, separators: this.settings.quickTag.separators});
            return;
        }

        if (this.settings.path) {
            this.lock.locked = true;
            this.send('quickTagLoad', {
                path: this.settings.path,
                recursive: this.settings.quickTag.recursive,
                separators: this.settings.quickTag.separators
            });
            this.saveSettings(false);
        }
    }

    // Handle keydown event for keyboard bindings
    handleKeyDown(event) {
        // QT Keybinds
        if (this.quickTag.track) {
            // Arrow keys
            if (event.key.startsWith('Arrow')) {
                // Seek audio
                if (event.key == 'ArrowLeft') {
                    let pos = this.player.position - 10000;
                    if (pos < 0)
                        this.seek(0);
                    else
                        this.seek(pos)
                }
                // Seek forward
                if (event.key == 'ArrowRight') {
                    let pos = this.player.position + 30000;
                    if (pos > this.player.duration)
                        this.seek(this.player.duration);
                    else
                        this.seek(pos);
                }
                // Get track index
                let i = this.quickTag.tracks.findIndex((t) => t.path == this.quickTag.track.path);
                // Skip tracks using arrow keys
                if (event.key == 'ArrowUp' && i > 0) {
                    this.onQuickTagEvent('changeTrack', {offset: -1});
                }
                if (event.key == 'ArrowDown' && i >= 0 && i < this.quickTag.tracks.length - 1) {
                    this.onQuickTagEvent('changeTrack', {offset: 1});
                }
                return true;
            }
            // Play pause
            if (event.code == "Space") {
                if (this.player.playing)
                    this.pause();
                else 
                    this.play();
                return true;
            }

            // Save
            if (event.code == "KeyS" && (event.ctrlKey || event.metaKey)) {
                this.saveQTTrack().then(() => {
                    Notify.create({
                        message: "Track saved!",
                        timeout: 3000,
                    });
                });
                return true;
            }

            // Note tag
            if (this.checkKeybind(event, this.settings.quickTag.noteTag.keybind)) {
                this.onQuickTagEvent('onNoteTag');
            }

            // Moods
            this.settings.quickTag.moods.forEach((mood) => {
                if (this.checkKeybind(event, mood.keybind)) {
                    this.quickTag.track.mood = mood.mood;
                }
            });
            // Genres
            this.settings.quickTag.genres.forEach((genre) => {
                if (this.checkKeybind(event, genre.keybind)) {
                    this.quickTag.track.toggleGenre(genre.genre);
                }
            });

            // Energy
            for (let i=0; i<5; i++) {
                if (this.checkKeybind(event, this.settings.quickTag.energyKeys[i])) {
                    this.quickTag.track.energy = i+1;
                    return true;
                }
            }

            // Custom values
            this.settings.quickTag.custom.forEach((tag, tagIndex) => {
                for (let i=0; i<tag.values.length; i++) {
                    if (this.checkKeybind(event, tag.values[i].keybind)) {
                        this.quickTag.track.toggleCustom(tagIndex, tag.values[i].val);
                    }
                }
            });

            return true;
        }

        // Tag editor save
        if (event.code == "KeyS" && (event.ctrlKey || event.metaKey) && this.onTagEditorEvent) {
            this.onTagEditorEvent({action: '_tagEditorSave'});
            return true;
        }

        return false;
    }
    // Check if keybind matches event
    checkKeybind(e, keybind) {
        if (!keybind) return;
        if (e.code.match(/F\d{1,2}/) || e.code.startsWith('Key') || e.code.startsWith("Digit") || e.code.startsWith("Numpad")) {
            let key = e.code.toLowerCase().replace("key", "").replace("digit", "").replace("numpad", "");
            return (key == keybind.key && 
                e.altKey == keybind.alt && 
                e.shiftKey == keybind.shift && 
                (e.ctrlKey || e.metaKey) == keybind.ctrl);
        }
    }
}

export default OneTagger;