import mergeOptions from 'merge-options';
import { Notify } from 'quasar';
import { ref, Ref } from 'vue';
import { AutotaggerConfig, AutotaggerPlatform } from './autotagger';
import { Player } from './player';
import { QuickTag } from './quicktag';
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

    // Websocket
    private ws!: WebSocket;
    private wsPromiseResolve?: (_: any) => void;
    private wsPromise?;

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

    }

    // SHOULD BE OVERWRITTEN
    quickTagUnfocus() {}
    onTaggingDone() {}
    onQuickTagEvent(_: any) {}
    onQuickTagBrowserEvent(_: any) {}
    onTagEditorEvent(_: any) {}
    onAudioFeaturesEvent(_: any) {}
    onSpotifyAuthEvent(_: any) {}
    onRenamerEvent(_: any) {}
    onFolderBrowserEvent(_: any) {}
    // =======================

    onError(msg: any) {
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
            case 'init':
                // Fill AppInfo
                this.info.value.version = json.version;
                this.info.value.os = json.os;
                this.info.value.platforms = json.platforms;
                this.info.value.renamerDocs = json.renamerDocs;

                this.info.value.ready = true;
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
        let config = mergeOptions({}, this.config, this.settings.value.autoTaggerConfig??{});
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

    // Quicktag
    loadQuickTag(playlist = null) { 
        alert('//TODO: Unfinished loadQuickTag');
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