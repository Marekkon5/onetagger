import Vue from 'vue';
import { Notify, colors } from 'quasar';
import { QTTrack } from './quicktag';

class OneTagger {

    constructor() {
        this.WAVES = 180;
        this.ws = new WebSocket(`ws://${window.location.hostname}:36912`);

        // WS error
        this.ws.onerror = (_, e) => {
            this.onError(e ?? "WebSocket error!");
        };
        this.ws.onclose = () => {
            this.onError('WebSocket closed!');
        }
        // Load settings on load
        this.ws.onopen = () => {
            this.send('loadSettings');
            setTimeout(() => {
                this.send('init');
                this.send('spotifyAuthorized');
            }, 100);
        }

        this.info = Vue.observable({version: '0.0.0'});

        // WS Message handler
        this.ws.onmessage = (event) => {
            let json = JSON.parse(event.data);
            if (!json.action) return;

            // Action
            switch (json.action) {
                // Initial info
                case 'init':
                    this.info.version = json.version;
                    //Path from args
                    if (json.startContext.startPath) {
                        this.settings.path = json.startContext.startPath;
                        this.config.path = json.startContext.startPath;
                    }
                    break;
                // Settings loaded
                case 'loadSettings':
                    this.loadSettings(json.settings);
                    break;
                // Path selected
                case 'browse':
                    // Autotagger path
                    if (json.context == 'at')
                        this.config.path = json.path;
                    // Quicktag path
                    if (json.context == 'qt') {
                        Vue.set(this.settings, 'path', json.path);
                        this.loadQuickTag();
                        this.saveSettings();
                    }
                    // Audio features path
                    if (json.context == 'af')
                        this.onAudioFeaturesEvent(json);
                    // Tag editor
                    if (json.context == 'te')
                        this.onTagEditorEvent(json);
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
                    break;
                // Status
                case 'taggingProgress':
                    this.taggerStatus.progress = json.status.progress;
                    // De duplicate failed
                    this.taggerStatus.statuses = this.taggerStatus.statuses.filter((s) => {
                        return s.status.path != json.status.status.path;
                    });
                    this.taggerStatus.statuses.push(json.status);
                    
                    break;
                // Tagging done
                case 'taggingDone':
                    this.lock.locked = false;
                    this.taggerStatus.done = true;
                    this.taggerStatus.progress = 1.0;
                    this.onTaggingDone();
                    break;
                // Player load track
                case 'playerLoad':
                    this.player.duration = json.duration;
                    this.player.position = 0;
                    this.player.playing = false;
                    break;
                case 'playerSync':
                    this.player.playing = json.playing;
                    break;
                // Quicktag
                case 'quickTagLoad':
                    this.quickTag.tracks = json.data.map(t => new QTTrack(t, this.settings.quickTag));
                    break;
                /*eslint-disable no-case-declarations*/
                case 'quickTagSaved':
                    let i = this.quickTag.tracks.findIndex((t) => t.path == json.path);
                    if (i != -1) {
                        Vue.set(this.quickTag.tracks, i, new QTTrack(json.file, this.settings.quickTag));
                    } else {
                        this.onError('quickTagSaved: Invalid track');
                    }
                    break;
                // Audio features Spotify
                case 'spotifyAuthorized':
                    this.onAudioFeaturesEvent(json);
                    break;
                // Debug
                default:
                    // Tag editor
                    if (json.action.startsWith('tagEditor')) {
                        this.onTagEditorEvent(json);
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
            "album": false,
            "key": false,
            "bpm": true,
            "genre": true,
            "style": true,
            "label": true,
            "releaseDate": true,
            "publishDate": false,
            "albumArt": false,
            "otherTags": false,
            "url": false,
            "trackId": false,
            "releaseId": false,
            "version": false,
            "separators": {id3: ', ', vorbis: null, mp4: ', '},
            "id3v24": true,
            "overwrite": true,
            "threads": 16,
            "strictness": 0.7,
            "mergeGenres": false,
            "albumArtFile": false,
            "camelot": false,
            "catalogNumber": false,
            "parseFilename": false,
            "filenameTemplate": "%trackNumber% - %artists% - %title%",
            "shortTitle": false,
            "matchDuration": false,
            "maxDurationDifference": 30,
            "beatport": {
                "artResolution": 500,
                "maxPages": 1
            },
            "discogs": {
                "token": null,
                "maxResults": 4,
                "styles": "default",
                "stylesCustomTag": {vorbis: 'STYLE', id3: 'STYLE', mp4: 'STYLE'}
            }
        });
        // Statuses
        this.taggerStatus = Vue.observable({
            statuses: [],
            started: 0,
            progress: 0.0,
            done: false,
            total: 0,
            type: null
        });
        // Lock, enable when tagging
        this.lock = Vue.observable({locked: false});

        // Player
        this.player = Vue.observable({
            waveform: [],
            playing: false,
            position: 0,
            duration: 1,
            volume: 0.5,
            wasPlaying: false
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
            quickTag: {
                recursive: false,
                autosave: false,
                noteTag: {
                    tag: {
                        id3: 'COMM', 
                        vorbis: 'COMMENT',
                        mp4: '©cmt'
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
                        mp4: '©wrt'
                    }
                },
                genres: [
                    {genre: '2-step', keybind: null},
                    {genre: 'Acid', keybind: null},
                    {genre: 'Breakbeat', keybind: null},
                    {genre: 'Disco', keybind: null},
                    {genre: 'Drum & Bass', keybind: null},
                    {genre: 'Electro', keybind: null},
                    {genre: 'Funk', keybind: null},
                    {genre: 'Hardcore', keybind: null},
                    {genre: 'Hiphop', keybind: null},
                    {genre: 'House', keybind: null},
                    {genre: 'Industrial', keybind: null},
                    {genre: 'Jungle', keybind: null},
                    {genre: 'Latin', keybind: null},
                    {genre: 'Minimal', keybind: null},
                    {genre: 'Nu-Disco', keybind: null},
                    {genre: 'Oldies', keybind: null},
                    {genre: 'Pop', keybind: null},
                    {genre: 'Reggae', keybind: null},
                    {genre: 'Rock', keybind: null},
                    {genre: 'Techno', keybind: null},
                    {genre: 'Trance', keybind: null},
                ],
                custom: [{
                    name: 'Vibe',
                    tag: {
                        id3: 'COMM',
                        vorbis: 'COMMENT',
                        mp4: '©cmt'
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
                        mp4: '©cmt'
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
                        mp4: '©cmt'
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
            tagEditorCustom: []
        });

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
    }

    // SHOULD BE OVERWRITTEN
    onError(msg) {console.error(msg);}
    onTaggingDone() {}
    onQTUnsavedChanges() {}
    onQTNoteTag() {}
    onTagEditorEvent() {}
    onAudioFeaturesEvent() {}

    // Send to socket
    send(action, params = {}) {
        let data = { action };
        Object.assign(data, params);
        this.ws.send(JSON.stringify(data));
    }

    // Open URL in external browser
    url(url) {
        this.send("browser", {url});
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
        
        // Restore specific

        // AT config (nested)
        Object.assign(this.config.discogs, this.settings.autoTaggerConfig??{}.discogs??{});
        Object.assign(this.config, this.settings.autoTaggerConfig??{});
        
        this.player.volume = this.settings.volume??0.5;
        this.setVolume(this.player.volume);
        colors.setBrand('primary', this.settings.primaryColor??'#00D2BF');
        if (!this.settings.tagEditorCustom) this.settings.tagEditorCustom = [];
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
        this.onQTUnsavedChanges();
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
        this.send("playerLoad", {path});
        this.generateWaveform(path);
        // console.log(path);
    }
    play() {
        this.send("playerPlay");
        this.player.playing = true;
    }
    pause() {
        this.send("playerPause");
        this.player.playing = false;
        this.player.wasPlaying = false;
    }
    seek(pos) {
        this.send("playerSeek", {pos})
        this.player.playing = false;
        this.player.position = pos;
    }
    setVolume(volume) {
        if (!volume) return;
        this.send("playerVolume", {volume});
    }

    // Quicktag
    loadQuickTag(playlist = null) {
        if (playlist) {
            this.send('quickTagLoad', {playlist});
            return;
        }
           
        if (this.settings.path)
            this.send('quickTagLoad', {
                path: this.settings.path,
                recursive: this.settings.quickTag.recursive
            });
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
                    this.loadQTTrack(this.quickTag.tracks[i-1]);
                }
                if (event.key == 'ArrowDown' && i >= 0 && i < this.quickTag.tracks.length - 1) {
                    this.loadQTTrack(this.quickTag.tracks[i+1]);
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
            if (event.code == "KeyS" && event.ctrlKey) {
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
                this.onQTNoteTag();
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
                    this.quickTag.track.setGenre(genre.genre);
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
            this.settings.quickTag.custom.forEach((tag) => {
                for (let i=0; i<tag.values.length; i++) {
                    if (this.checkKeybind(event, tag.values[i].keybind)) {
                        this.quickTag.track.toggleCustom(tag, i);
                    }
                }
            });

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
                e.ctrlKey == keybind.ctrl);
        }
    }
}

export default OneTagger;