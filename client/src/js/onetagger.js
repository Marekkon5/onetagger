import Vue from 'vue';
import {Notify} from 'quasar';
import {QTTrack} from './quicktag';

class OneTagger {

    constructor() {
        this.WAVES = 250;
        this.ws = new WebSocket('ws://localhost:36912');

        //WS error
        this.ws.onerror = (_, e) => {
            this.onError(e ?? "WebSocket error!");
        };
        this.ws.onclose = () => {
            this.onError('WebSocket closed!');
        }
        //Load settings on load
        this.ws.onopen = () => {
            this.send('loadSettings');
            this.send('spotifyAuthorized');
        }

        //WS Message handler
        this.ws.onmessage = (event) => {
            let json = JSON.parse(event.data);
            if (!json.action) return;

            //Action
            switch (json.action) {
                //Settings loaded
                case 'loadSettings':
                    this.loadSettings(json.settings);
                    break;
                //Path selected
                case 'browse':
                    //Autotagger path
                    if (json.context == 'at')
                        this.config.path = json.path;
                    //Quicktag path
                    if (json.context == 'qt') {
                        Vue.set(this.settings.quickTag, 'path', json.path);
                        this.loadQuickTag();
                    }
                    //Audio features path
                    if (json.context == 'af')
                        this.audioFeatures.path = json.path;
                        
                    break;
                //Error
                case 'error':
                    //Unlock, callback
                    this.lock.locked = false;
                    this.onError(json.message);
                    break;
                //Status
                case 'taggingProgress':
                    this.taggerStatus.progress = json.status.progress;
                    if (json.status.status.status != 'ok') {
                        this.taggerStatus.statuses.push(json.status);
                    } else {
                        //OK - remove from failed
                        this.taggerStatus.statuses = this.taggerStatus.statuses.filter((s) => {
                            return s.status.path != json.status.status.path;
                        });
                        this.taggerStatus.ok += 1;
                    }
                    break;
                //Tagging done
                case 'taggingDone':
                    this.lock.locked = false;
                    this.taggerStatus.done = true;
                    this.taggerStatus.progress = 1.0;
                    this.onTaggingDone();
                    break;
                //Player load track
                case 'playerLoad':
                    this.player.duration = json.duration;
                    this.player.position = 0;
                    this.player.playing = false;
                    break;
                case 'playerSync':
                    this.player.playing = json.playing;
                    break;
                //Quicktag
                case 'quicktagLoad':
                    this.quickTag.tracks = json.data.map(t => new QTTrack(t, this.settings.quickTag));
                    break;
                /*eslint-disable no-case-declarations*/
                case 'quicktagSaved':
                    let i = this.quickTag.tracks.findIndex((t) => t.path == json.path);
                    if (i != -1) {
                        Vue.set(this.quickTag.tracks, i, new QTTrack(json.file, this.settings.quickTag));
                    } else {
                        this.onError('quicktagSaved: Invalid track');
                    }
                    break;
                //Audio features Spotify
                case 'spotifyAuthorized':
                    this.audioFeatures.spotifyAuthorized = json.value;
                    break;
                //New audio features status
                case 'audioFeaturesStatus':
                    this.audioFeatures.statuses.push(json.status);
                    break;
                case 'audioFeaturesDone':
                    this.lock.locked = false;
                    this.audioFeatures.done = true;
                    this.audioFeatures.ended = Date.now();
                    this.onTaggingDone();
                    break;
                //Debug
                default:
                    console.log(json);
                    break;
            }
        } 

        //Default autotagger config
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
            "id3Separator": ", ",
            "id3v24": true,
            "overwrite": true,
            "threads": 16,
            "strictness": 0.7,
            "mergeGenres": false,
            "albumArtFile": false,
            "beatport": {
                "artResolution": 500,
                "maxPages": 1
            },
            "discogs": {
                "token": null,
                "maxResults": 4,
                "styles": "default"
            }
        });
        //Statuses
        this.taggerStatus = Vue.observable({
            statuses: [],
            started: 0,
            ok: 0,
            progress: 0.0,
            done: false
        });
        //Lock, enable when tagging
        this.lock = Vue.observable({locked: false});

        //Player
        this.player = Vue.observable({
            waveform: [],
            playing: false,
            position: 0,
            duration: 1,
            volume: 0.5
        });
        this.generateDefaultWaveform();
        //Player position updater
        setInterval(() => {
            if (this.player.playing)
                this.player.position += 150;
        }, 150);

        //Quick tag
        this.quickTag = Vue.observable({
            tracks: [],
            track: null,
        });

        //Settings for UI
        this.settings = Vue.observable({
            discogsToken: null,
            volume: 0.05,
            quickTag: {
                path: null,
                energyKeys: [null,null,null,null,null],
                moods: [
                    {mood: 'Happy', color: 'amber'},
                    {mood: 'Sad', color: 'indigo'},
                    {mood: 'Bright', color: 'green'},
                    {mood: 'Dark', color: 'deep-purple'},
                    {mood: 'Angry', color: 'red'},
                    {mood: 'Chill', color: 'teal'}
                ],
                moodTag: {vorbis: 'MOOD', id3: 'TMOO'},
                energyTag: {
                    //rating = save to rating tag, symbol = save to custom tag with symbols
                    type: 'rating',
                    symbol: '*',
                    id3: 'TCOM',
                    vorbis: 'COMPOSER'
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
                    id3: 'STYLE',
                    vorbis: 'STYLE',
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
                    id3: '',
                    vorbis: '',
                    values: [
                        {val: 'Start', keybind: null},
                        {val: 'Build', keybind: null},
                        {val: 'Peak', keybind: null},
                        {val: 'Sustain', keybind: null},
                        {val: 'Release', keybind: null},
                    ]
                }, {
                    name: 'Instruments',
                    id3: '',
                    vorbis: '',
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
            }
        });

        //Audio features
        this.audioFeatures = Vue.observable({
            spotifyAuthorized: false,
            path: null,
            statuses: [],
            done: false,
            started: Date.now(),
            ended: null
        });

        //If unsaved changes to track
        this._nextQTTrack = null;
        //Waveform loading lock
        this._waveformLock = [];
        this._waveformPath = null;

        //Keybinds
        document.addEventListener('keydown', (e) => {
            if (e.target.nodeName == "INPUT") return true;
            if (this.handleKeyDown(e)) {
                e.preventDefault();
                return false;
            }
        });

        //So can be triggered globally
        this.helpDialog = Vue.observable({open: false, route: null});
    }

    //SHOULD BE OVERWRITTEN
    onError(msg) {console.error(msg);}
    onTaggingDone() {}
    onQTUnsavedChanges() {}

    //Send to socket
    send(action, params = {}) {
        let data = { action };
        Object.assign(data, params);
        this.ws.send(JSON.stringify(data));
    }

    //Open URL in external browser
    url(url) {
        this.send("browser", {url});
    }

    //Start autotagger
    startTagging() {
        this.lock.locked = true;
        this.taggerStatus.statuses = [];
        this.taggerStatus.started = Date.now();
        this.taggerStatus.done = false;
        this.taggerStatus.ok = 0;
        this.taggerStatus.progress = 0.0;
        this.send('startTagging', {config: this.config});
    }

    //Save settings to file
    saveSettings() {
        //Save discogs token and volume
        if (this.config.discogs.token)
            this.settings.discogsToken = this.config.discogs.token;
        //Only show notification if not changing volume
        let notif = this.settings.volume == this.player.volume;
        this.settings.volume = this.player.volume;
        //Save
        this.send("saveSettings", {settings: JSON.parse(JSON.stringify(this.settings))});
        //Notification
        if (notif)
            Notify.create({
                message: "Settings saved!",
                color: 'primary',
                textColor: 'black',
                timeout: 500,
            });
    }
    //Load settings from JSON
    loadSettings(data) {
        //Load depper dicts separately
        Object.assign(this.settings.quickTag, data.quickTag);
        delete data.quickTag;
        Object.assign(this.settings, data);
        
        //Restore discogs token and volume
        this.config.discogs.token = this.settings.discogsToken;
        this.player.volume = this.settings.volume??0.5;
        this.setVolume(this.player.volume);
    }

    //Wrapper to prevent multiple waveforms
    async generateWaveform(path) {
        this._waveformPath = path;
        //Aquire lock
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

    //Generate waveform
    async _generateWaveform(path) {
        this.generateDefaultWaveform();
        let waveformIndex = 0;
        //Separate socket = separate thread
        let ws = new WebSocket('ws://localhost:36912');
        ws.onmessage = (event) => {
            //Lock
            if (this._waveformLock.length > 1) {
                ws.close();
                this._waveformLock.pop();
                return;
            }

            let json = JSON.parse(event.data);
            //New wave
            if (json.action == 'waveformWave') {
                Vue.set(this.player.waveform, waveformIndex, json.wave);
                waveformIndex++;
            }
            //Finish
            if (json.action == 'waveformDone') {
                ws.close();
                this._waveformLock.pop();
            }
            //Will be ignored, just for updating
            ws.send('waveformRead');
        };
        ws.onopen = () => {
            ws.send(JSON.stringify({
                action: 'waveform',
                path
            }));
        };
    }

    //Default waveform
    generateDefaultWaveform() {
        for (let i=0; i<this.WAVES; i++) {
            this.player.waveform[i] = 0;
        }
    }

    //Load quicktag track
    loadQTTrack(track, force = false) {
        //Check for unsaved changes
        if (!this.quickTag.track || force || !this.quickTag.track.isChanged()) {
            if (!track)
                track = this._nextQTTrack;
            this.quickTag.track = new QTTrack(JSON.parse(JSON.stringify(track)), this.settings.quickTag);
            this.loadTrack(track.path);
            this._nextQTTrack = null;
            return;
        }
        //Prompt for unsaved changes
        this._nextQTTrack = track;
        this.onQTUnsavedChanges();
    }
    //Save quickTagTrack
    async saveQTTrack() {
        if (this.quickTag.track) {
            let changes = this.quickTag.track.getOutput();
            this.send('quicktagSave', {changes});
        }
    }

    //Player controls
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

    //Quicktag
    loadQuickTag() {
        if (!this.settings.quickTag.path) return;
        this.send('quicktagLoad', {path: this.settings.quickTag.path});
    }

    //Handle keydown event for keyboard bindings
    handleKeyDown(event) {
        //QT Keybinds
        if (this.quickTag.track) {
            //Arrow keys
            if (event.key.startsWith('Arrow')) {
                //Seek audio
                if (event.key == 'ArrowLeft') {
                    let pos = this.player.position - 10000;
                    if (pos < 0)
                        this.seek(0);
                    else
                        this.seek(pos)
                }
                //Seek forward
                if (event.key == 'ArrowRight') {
                    let pos = this.player.position + 30000;
                    if (pos > this.player.duration)
                        this.seek(this.player.duration);
                    else
                        this.seek(pos);
                }
                //Get track index
                let i = this.quickTag.tracks.findIndex((t) => t.path == this.quickTag.track.path);
                //Skip tracks using arrow keys
                if (event.key == 'ArrowUp' && i > 0) {
                    this.loadQTTrack(this.quickTag.tracks[i-1]);
                }
                if (event.key == 'ArrowDown' && i >= 0 && i < this.quickTag.tracks.length - 1) {
                    this.loadQTTrack(this.quickTag.tracks[i+1]);
                }
                return true;
            }
            //Play pause
            if (event.code == "Space") {
                if (this.player.playing)
                    this.pause();
                else 
                    this.play();
                return true;
            }

            //Moods
            this.settings.quickTag.moods.forEach((mood) => {
                if (this.checkKeybind(event, mood.keybind)) {
                    this.quickTag.track.mood = mood.mood;
                }
            });
            //Genres
            this.settings.quickTag.genres.forEach((genre) => {
                if (this.checkKeybind(event, genre.keybind)) {
                    this.quickTag.track.setGenre(genre.genre);
                }
            });

            //Energy
            for (let i=0; i<5; i++) {
                if (this.checkKeybind(event, this.settings.quickTag.energyKeys[i])) {
                    this.quickTag.track.energy = i+1;
                    return true;
                }
            }

            //Custom values
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
    //Check if keybind matches event
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