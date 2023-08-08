import { Dialog, Notify, setCssVar } from 'quasar';
import { ref, Ref } from 'vue';
import { AutotaggerConfig, AutotaggerPlatform, TaggerStatus } from './autotagger';
import { Player } from './player';
import { QTTrack, QuickTag, QuickTagFile } from './quicktag';
import { Settings } from './settings';
import { Keybind, Playlist, Spotify, wsUrl } from './utils';
import router from './router';

class OneTagger {
    // Singleton
    private static instance: OneTagger;

    info: Ref<AppInfo> = ref({}) as Ref<AppInfo>;
    config: Ref<AutotaggerConfig> = ref(new AutotaggerConfig());
    lock: Ref<{ locked: boolean }> = ref({ locked: false });
    player: Ref<Player> = ref(new Player(this));
    quickTag: Ref<QuickTag> = ref(new QuickTag());
    settings: Ref<Settings> = ref(new Settings());
    spotify: Ref<Spotify> = ref(new Spotify());
    helpDialog: Ref<{ open: boolean, route?: string }> = ref({ open: false });
    folderBrowser: Ref<FolderBrowser> = ref(new FolderBrowser());
    taggerStatus: Ref<TaggerStatus> = ref(new TaggerStatus());
    autoTaggerPlaylist: Ref<Playlist> = ref({});

    // Websocket
    private ws!: WebSocket;
    private wsPromiseResolve?: (_: any) => void;
    private wsPromise?;

    // Quicktag track loading
    private nextQTTrack?: QTTrack;

    constructor() {
        // Singleton
        if (OneTagger.instance) {
            return OneTagger.instance;
        }
        OneTagger.instance = this;


        // WS connection promise
        this.wsPromise = new Promise((res) => this.wsPromiseResolve = res);
        // Setup WS connection
        this.ws = new WebSocket(wsUrl());
        this.ws.addEventListener('error', (e) => this.onError(e ?? 'Websocket error!'));
        this.ws.addEventListener('close', (_) => this.onError('WebSocket closed!'));
        this.ws.addEventListener('open', (_) => {
            // Resolve connection promise
            if (this.wsPromiseResolve) {
                this.wsPromiseResolve(null);
                this.wsPromiseResolve = undefined;
            }

            // Load initial data
            this.send('loadSettings');
            setTimeout(() => {
                this.send('init');
                this.send('spotifyAuthorized');
                // Update custom to v2
                this.send('defaultCustomPlatformSettings');
            }, 100);
        });
        this.ws.addEventListener('message', (event) => {
            // Parse incoming message
            let json = JSON.parse(event.data);
            if (!json.action) return;
            this.incomingEvent(json);
        });

        // Keybinds
        document.addEventListener('keydown', (e) => {
            // Can be safely error ignored
            // @ts-ignore
            if (e.target && e.target.nodeName == "INPUT") return true;

            if (this.handleKeyDown(e)) {
                e.preventDefault();
                return false;
            }
        });
    }

    // SHOULD BE OVERWRITTEN
    quickTagUnfocus() {}
    onTaggingDone(_: any) {}
    onQuickTagEvent(_: any, __?: any) {}
    onQuickTagBrowserEvent(_: any) {}
    onTagEditorEvent(_: any) {}
    onAudioFeaturesEvent(_: any) {}
    onRenamerEvent(_: any) {}
    onFolderBrowserEvent(_: any) {}
    // =======================

    // Display error to the user
    onError(msg: any) {
        // Show error dialog
        Dialog.create({
            title: 'Error',
            message: `${msg}`,
            ok: {
                color: 'primary'
            }
        });

        console.error(msg);
    }

    // Send action to WebSocket
    async send(action: string, params = {}) {
        // Wait for WS connection
        if (this.wsPromise) {
            await this.wsPromise;
            this.wsPromise = undefined;
        }
        let data = { action };
        Object.assign(data, params);
        this.ws.send(JSON.stringify(data));
    }

    // Process incoming event
    private async incomingEvent(json: any) {
        switch (json.action) {
            // Initial info
            case 'init':
                // Fill AppInfo
                this.info.value.version = json.version;
                this.info.value.os = json.os;
                this.info.value.platforms = json.platforms;
                this.info.value.renamerDocs = json.renamerDocs;
                // Path from args
                if (json.startContext.startPath) {
                    this.settings.value.path = json.startContext.startPath;
                    this.config.value.path = json.startContext.startPath;
                }

                this.info.value.ready = true;
                break;
            
            // Settings loaded
            case 'loadSettings':
                this.loadSettings(json.settings);
                break;
            // Load custom platform settings
            case 'defaultCustomPlatformSettings':
                // Update custom platform settings to V2
                if (!this.config.value.custom['beatport'] || typeof this.config.value.custom['beatport']['art_resolution'] === 'object') {
                    this.config.value.custom = json.custom;
                }
                // Merge platform custom
                for (let platform of Object.keys(json.custom)) {
                    // All keys
                    if (!this.config.value.custom[platform]) {
                        this.config.value.custom[platform] = json.custom[platform];
                        continue;
                    }
                    // Per key
                    for (let key of Object.keys(json.custom[platform])) {
                        if (!this.config.value.custom[platform][key]) {
                            this.config.value.custom[platform][key] = json.custom[platform][key];
                        }
                    }
                }
                
                break;
            // Path selected
            case 'browse':
                this.onBrowse(json);
                break;
            // Error
            case 'error':
                // Unlock, callback
                this.lock.value.locked = false;
                this.onError(json.message);
                break;
            case 'startTagging':
                this.lock.value.locked = true;
                this.taggerStatus.value.reset();
                this.taggerStatus.value.total = json.files;
                this.taggerStatus.value.type = json.type;
                break;
            // Status
            case 'taggingProgress':
                this.taggerStatus.value.progress = json.status.progress;
                // De duplicate failed
                this.taggerStatus.value.statuses = this.taggerStatus.value.statuses.filter((s) => {
                    return s.status.path != json.status.status.path;
                });
                this.taggerStatus.value.statuses.unshift(json.status);
                
                break;
            // Tagging done
            case 'taggingDone':
                this.lock.value.locked = false;
                this.taggerStatus.value.done = true;
                this.taggerStatus.value.progress = 1.0;
                this.taggerStatus.value.data = json.data;
                this.onTaggingDone(json.path);
                break;
            // Player load track
            case 'playerLoad':
                this.player.value.duration = json.duration;
                this.player.value.position = 0;
                this.player.value.playing = false;
                this.player.value.title = json.title;
                this.player.value.artists = json.artists;
                break;
            case 'playerSync':
                this.player.value.playing = json.playing;
                // Play after seeking
                if (this.settings.value.playOnSeek && !json.playing)
                    this.player.value.play();
                break;
            // Quicktag
            case 'quickTagLoad':
                this.lock.value.locked = false;
                this.quickTag.value.tracks = json.data.files.map((t: QuickTagFile) => new QTTrack(t, this.settings.value.quickTag));
                this.quickTag.value.failed = json.data.failed;
                this.onQuickTagEvent('quickTagLoad');
                break;
            /*eslint-disable no-case-declarations*/
            case 'quickTagSaved':
                let i = this.quickTag.value.tracks.findIndex((t) => t.path == json.path);
                if (i != -1) {
                    this.quickTag.value.tracks[i] = new QTTrack(json.file, this.settings.value.quickTag);
                } else {
                    // this.onError('quickTagSaved: Invalid track');
                }
                this.quickTag.value.saving -= 1;

                break;
            // Browser folder
            case 'quickTagFolder':
                this.onQuickTagBrowserEvent(json);
                break;
            // Spotify
            case 'spotifyAuthorized':
                this.spotify.value.authorized = json.value;
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

                console.log(`Unknown action: ${json.action}`);
                console.log(json);
                break;
        }
    }

    // Handle message from Webview/OS
    onOSMessage(json: any) {
        switch (json.action) {
            // Drag and drop path
            case 'browse':
                // Callback by route
                let route = router.currentRoute.value.path.substring(1).split('/')[0];
                switch (route) {
                    case 'autotagger':
                        this.config.value.path = json.path;
                        break;
                    case 'audiofeatures':
                        this.onAudioFeaturesEvent(json);
                        break;
                    case 'tageditor':
                        this.onTagEditorEvent(json);
                        break;
                    case 'quicktag':
                        this.settings.value.path = json.path;
                        this.loadQuickTag();
                        break;
                    case 'renamer':
                        this.onRenamerEvent(json);
                        break;
                    default:
                        this.settings.value.path = json.path;
                        break;
                }
                break;
            default:
                console.log(`Unknown OS action: ${json}`);
                break;
        }
    }

    // Open URL in external browser
    url(url: string) {
        this.send('browser', {url});
    }

    // Open native folder browser
    browse(context: string, path?: string) {
        if (this.settings.value.nonNativeBrowser) {
            this.folderBrowser.value.context = context;
            if (path)
                this.folderBrowser.value.basePath = path;
            this.folderBrowser.value.open = true;
            return;
        }
        this.send('browse', { context, path });        
    }

    // onBrowse event
    onBrowse(json: BrowseEvent) {
        // Autotagger path
        if (json.context == 'at')
            this.config.value.path = json.path;
        // Quicktag path
        if (json.context == 'qt') {
            this.settings.value.path = json.path;
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
        this.settings.value.saveATProfile(this.settings.value.autoTaggerProfile, this.config.value);
        // Very dirty way to clone a dict, but eh
        this.settings.value.autoTaggerConfig = JSON.parse(JSON.stringify(this.config.value));
        this.settings.value.volume = this.player.value.volume;
        // Save
        this.send("saveSettings", {settings: JSON.parse(JSON.stringify(this.settings.value))});
        // Notification
        if (notif)
            Notify.create({
                message: "Settings saved!",
                color: 'primary',
                textColor: 'black',
                timeout: 500,
                position: 'top-right'
            });
    }

    // Load settings from JSON
    loadSettings(data: any) {
        this.settings.value = Settings.fromJson(data);
        
        // AT config (nested)
        this.config.value.loadSettings(this.settings.value.autoTaggerConfig);
 
        // Restore specific
        this.player.value.volume = this.settings.value.volume??0.5;
        this.player.value.audio.volume = this.player.value.volume;
        this.player.value.setVolume(this.player.value.volume);
        setCssVar('primary', this.settings.value.primaryColor??'#00D2BF');
        if (!this.settings.value.tagEditorCustom) this.settings.value.tagEditorCustom = [];
        this.spotify.value.clientId = this.settings.value.audioFeatures.spotifyClientId;
        this.spotify.value.clientSecret = this.settings.value.audioFeatures.spotifyClientSecret;

        // Migrate to enable subgenres
        for (let i=0; i<this.settings.value.quickTag.genres.length; i++) {
            if (!this.settings.value.quickTag.genres[i].subgenres) {
                this.settings.value.quickTag.genres[i]['subgenres'] = [];
            }
        }
    }

    // Load autotagger profile
    loadATProfile(name: string) {
        let profile = this.settings.value.autoTaggerProfiles.find(p => p.name == name);
        if (profile == null) throw Error('Invalid profile');

        this.config.value.loadSettings(profile.config);
        this.settings.value.autoTaggerProfile = name;
        // Update custom values in case
        this.send('defaultCustomPlatformSettings');
    }

    // Load quicktag track
    loadQTTrack(track?: QTTrack, force = false) {
        // Check for unsaved changes
        if (force || !this.quickTag.value.track.isChanged()) {
            if (!track && !this.nextQTTrack)
                return;
            if (!track)
                track = this.quickTag.value.tracks.find(t => t.path == this.nextQTTrack!.path);

            // For autoplay
            if (this.player.value.playing)
                this.player.value.wasPlaying = true;

            this.quickTag.value.track.loadSingle(new QTTrack(JSON.parse(JSON.stringify(track)), this.settings.value.quickTag));
            this.player.value.loadTrack(track!.path);
            this.nextQTTrack = undefined;
            return;
        }
        // Prompt for unsaved changes
        this.nextQTTrack = track;
        this.onQuickTagEvent('onUnsavedChanges');
    }

    /// Add or remove a track to the multitrack
    toggleQTTrack(track: QTTrack) {
        if (this.quickTag.value.track.getTrack(track.path)) {
            this.quickTag.value.track.removeTrack(track);
            return;
        }
        this.quickTag.value.track.addTrack(new QTTrack(JSON.parse(JSON.stringify(track)), this.settings.value.quickTag));
    }

    // Save quickTagTrack
    async saveQTTrack() {
        let changes = this.quickTag.value.track.getOutputs();
        for (const change of changes) {
            this.quickTag.value.saving += 1;
            this.send('quickTagSave', { changes: change });
        }
        await this.quickTag.value.waitForSave();
    }


    // Quicktag
    loadQuickTag(playlist?: Playlist) { 
        // Loading
        if (playlist || this.settings.value.path) {
            this.lock.value.locked = true;
            this.quickTag.value.tracks = [];
        }

        // Load playlist
        if (playlist) {
            this.send('quickTagLoad', { playlist, separators: this.settings.value.quickTag.separators });
            return;
        }

        // Load by path
        if (this.settings.value.path) {
            this.lock.value.locked = true;
            this.send('quickTagLoad', {
                path: this.settings.value.path,
                recursive: this.settings.value.quickTag.recursive,
                separators: this.settings.value.quickTag.separators
            });
            this.saveSettings(false);
        }
    }

    /// Stop the tagging process
    stopTagging() {
        let ws = new WebSocket(wsUrl());
        ws.onopen = () => {
            ws.send(JSON.stringify({action: 'stopTagging'}));
        };
    }

    // Handle keydown event for keyboard bindings
    handleKeyDown(event: KeyboardEvent) {
        // QT Keybinds
        if (this.quickTag.value.track.hasTracks()) {
            // Arrow keys
            if (event.key.startsWith('Arrow')) {
                // Seek audio
                if (event.key == 'ArrowLeft') {
                    let pos = this.player.value.position - 10000;
                    if (pos < 0)
                        this.player.value.seek(0);
                    else
                        this.player.value.seek(pos)
                }
                // Seek forward
                if (event.key == 'ArrowRight') {
                    let pos = this.player.value.position + 30000;
                    if (pos > this.player.value.duration)
                        this.player.value.seek(this.player.value.duration);
                    else
                        this.player.value.seek(pos);
                }

                // Skip tracks using arrow keys
                if (event.key == 'ArrowUp') {
                    this.onQuickTagEvent(event.shiftKey ? 'addTrack' : 'changeTrack', { offset: -1 });
                }
                if (event.key == 'ArrowDown') {
                    this.onQuickTagEvent(event.shiftKey ? 'addTrack' : 'changeTrack', { offset: 1 });
                }
                return true;
            }
            
            // Play pause
            if (event.code == "Space") {
                if (this.player.value.playing)
                    this.player.value.pause();
                else 
                    this.player.value.play();
                return true;
            }

            // Save
            if (event.code == "KeyS" && (event.ctrlKey || event.metaKey)) {
                this.saveQTTrack().then(() => {
                    Notify.create({
                        message: "Track saved!",
                        timeout: 3000,
                        position: 'top-right'
                    });
                });
                return true;
            }

            // Note tag
            if (Keybind.check(event, this.settings.value.quickTag.noteTag.keybind)) {
                this.onQuickTagEvent('onNoteTag');
            }

            // Moods
            this.settings.value.quickTag.moods.forEach((mood) => {
                if (Keybind.check(event, mood.keybind)) {
                    this.quickTag.value.track!.mood = mood.mood;
                }
            });
            // Genres
            this.settings.value.quickTag.genres.forEach((genre) => {
                if (Keybind.check(event, genre.keybind)) {
                    this.quickTag.value.track!.toggleGenre(genre.genre);
                }
            });

            // Energy
            for (let i=0; i<5; i++) {
                if (Keybind.check(event, this.settings.value.quickTag.energyKeys[i])) {
                    this.quickTag.value.track.energy = i+1;
                    return true;
                }
            }

            // Custom values
            this.settings.value.quickTag.custom.forEach((tag, tagIndex) => {
                for (let i=0; i<tag.values.length; i++) {
                    if (Keybind.check(event, tag.values[i].keybind)) {
                        this.quickTag.value.track!.toggleCustom(tagIndex, tag.values[i].val);
                    }
                }
            });

            // Delete tracks
            if ((event.code == "Delete" && (event.ctrlKey || event.metaKey)) || (event.code == 'Backspace' && event.metaKey)) {
                this.onQuickTagEvent('onDeleteTrack');
            }

            return true;
        }

        // Tag editor save
        if (event.code == "KeyS" && (event.ctrlKey || event.metaKey) && this.onTagEditorEvent) {
            this.onTagEditorEvent({action: '_tagEditorSave'});
            return true;
        }

        return false;
    }
    
}

interface AppInfo {
    version: string;
    os: string;
    ready: boolean;
    platforms: AutotaggerPlatform[];
    renamerDocs: any
}

class FolderBrowser {
    open: boolean = false;
    basePath: string = '/';
    context?: string;
}

interface BrowseEvent {
    path: string;
    context?: string;
    action?: string;
}


/// Get OneTagger instance
function get1t(): OneTagger {
    return new OneTagger();
}

export { OneTagger, get1t };