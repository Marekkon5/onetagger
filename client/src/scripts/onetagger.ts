import { Dialog, Notify } from 'quasar';
import { ref, Ref } from 'vue';
import { useRouter } from 'vue-router';
import { AutotaggerConfig, AutotaggerPlatform, TaggerStatus } from './autotagger';
import { Player } from './player';
import { QTTrack, QuickTag, QuickTagFile } from './quicktag';
import { Settings } from './settings';
import { Spotify, wsUrl } from './utils';

class OneTagger {
    // Singleton
    private static instance: OneTagger;

    info: Ref<AppInfo> = ref({}) as Ref<AppInfo>;
    config: Ref<AutotaggerConfig> = ref(new AutotaggerConfig());
    lock: Ref<{ locked: boolean }> = ref({ locked: false });
    player: Ref<Player> = ref(new Player());
    quickTag: Ref<QuickTag> = ref(new QuickTag());
    settings: Ref<Settings> = ref(new Settings());
    spotify: Ref<Spotify> = ref(new Spotify());
    helpDialog: Ref<{ open: boolean, route?: string }> = ref({ open: false });
    folderBrowser: Ref<FolderBrowser> = ref(new FolderBrowser());
    taggerStatus: Ref<TaggerStatus> = ref(new TaggerStatus());

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
    onSpotifyAuthEvent(_: any) {}
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

                //TODO: REFACTOR THIS MESS AND FIND BETTER WAY TO SAVE JUST VALUES
                // restore custom platform fields
                for (const [key, value] of Object.entries(this.config.value.custom)) {
                    for (let platform of this.info.value.platforms) {
                        if (platform.platform.id == key) {
                            // restore keys
                            // @ts-ignore
                            for (const [id, newValue] of Object.entries(value)) {
                                for (let i in platform.platform.customOptions.options) {
                                    if (platform.platform.customOptions.options[i].id == id) {
                                        // @ts-ignore
                                        platform.platform.customOptions.options[i].value.value = newValue.value;
                                    }
                                }
                            }

                        }
                    }
                }
                this.info.value.ready = true;
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
                    this.quickTag.value.tracks[i] = new QTTrack(json.file, this.settings.value.quickTag)
                } else {
                    // this.onError('quickTagSaved: Invalid track');
                }
                // Force reload current track
                if (this.quickTag.value.track && json.path == this.quickTag.value.track.path) {
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
                let route = useRouter().currentRoute.value.path.substring(1).split('/')[0];
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
                        this.loadQuickTag(null);
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
    browse(context: string, path: string) {
        if (this.settings.value.nonNativeBrowser) {
            this.folderBrowser.value.context = context;
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
        // Very dirty way to clone a dict, but eh
        this.settings.value.autoTaggerConfig = JSON.parse(JSON.stringify(this.config));
        this.settings.value.volume = this.player.value.volume;
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
    loadSettings(data: any) {
        // Load depper dicts separately
        this.settings.value.quickTag = data.quickTag;
        delete data.quickTag;
        Object.assign(this.settings.value, data);
        
        // AT config (nested)
        let config = Object.assign({}, this.config.value, this.settings.value.autoTaggerConfig);
        Object.assign(this.config.value, config);
 
        // Restore specific
        this.player.value.volume = this.settings.value.volume??0.5;
        this.player.value.audio.volume = this.player.value.volume;
        this.player.value.setVolume(this.player.value.volume);
        //TODO: Branding colors
        // colors.setBrand('primary', this.settings.value.primaryColor??'#00D2BF');
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

    // Load quicktag track
    loadQTTrack(track?: QTTrack, force = false) {
        // Check for unsaved changes
        if (!this.quickTag.value.track || force || !this.quickTag.value.track.isChanged()) {
            if (!track)
                track = this.nextQTTrack;
            // For autoplay
            if (this.player.value.playing)
                this.player.value.wasPlaying = true;
            this.quickTag.value.track = new QTTrack(JSON.parse(JSON.stringify(track)), this.settings.value.quickTag);
            this.player.value.loadTrack(track!.path);
            this.nextQTTrack = undefined;
            return;
        }
        // Prompt for unsaved changes
        this.nextQTTrack = track;
        this.onQuickTagEvent('onUnsavedChanges');
    }

    // Save quickTagTrack
    async saveQTTrack() {
        if (this.quickTag.value.track) {
            let changes = this.quickTag.value.track.getOutput();
            this.send('quickTagSave', {changes});
        }
    }


    // Quicktag
    loadQuickTag(playlist = null) { 
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

    // Handle keydown event for keyboard bindings
    handleKeyDown(event: KeyboardEvent) {
        // QT Keybinds
        if (this.quickTag.value.track) {
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
                // Get track index
                let i = this.quickTag.value.tracks.findIndex((t) => t.path == this.quickTag.value.track.path);
                // Skip tracks using arrow keys
                if (event.key == 'ArrowUp' && i > 0) {
                    this.onQuickTagEvent('changeTrack', {offset: -1});
                }
                if (event.key == 'ArrowDown' && i >= 0 && i < this.quickTag.value.tracks.length - 1) {
                    this.onQuickTagEvent('changeTrack', {offset: 1});
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
                    });
                });
                return true;
            }

            // Note tag
            if (this.settings.value.quickTag.noteTag.keybind?.check(event)) {
                this.onQuickTagEvent('onNoteTag');
            }

            // Moods
            this.settings.value.quickTag.moods.forEach((mood) => {
                if (mood.keybind?.check(event)) {
                    this.quickTag.value.track.mood = mood.mood;
                }
            });
            // Genres
            this.settings.value.quickTag.genres.forEach((genre) => {
                if (genre.keybind?.check(event)) {
                    this.quickTag.value.track.toggleGenre(genre.genre);
                }
            });

            // Energy
            for (let i=0; i<5; i++) {
                if (this.settings.value.quickTag.energyKeys[i]?.check(event)) {
                    this.quickTag.value.track.energy = i+1;
                    return true;
                }
            }

            // Custom values
            this.settings.value.quickTag.custom.forEach((tag, tagIndex) => {
                for (let i=0; i<tag.values.length; i++) {
                    if (tag.values[i].keybind?.check(event)) {
                        this.quickTag.value.track.toggleCustom(tagIndex, tag.values[i].val);
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
    context: string;
}


/// Get OneTagger instance
function get1t(): OneTagger {
    return new OneTagger();
}

export { get1t };