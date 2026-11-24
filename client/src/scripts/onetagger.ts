import { Requests } from './requests';
import { Dialog, DialogChainObject, Notify, setCssVar } from 'quasar';
import { ref, Ref } from 'vue';
import { AutotaggerConfig, AutotaggerPlatform, ConfigCallbackResponse, TaggerStatus } from './autotagger';
import { Player } from './player';
import { QTTrack, QuickTag, QuickTagFile } from './quicktag';
import { Settings } from './settings';
import { Keybind, Playlist, Spotify, wsUrl } from './utils';
import { ManualTag } from './manualtag';
import ExitDialog from '../components/ExitDialog.vue';
import router from './router';
import DevToolsVue from '../components/DevTools.vue';
import PlatformsRepoVue from '../components/PlatformsRepo.vue';
import { translate as t, translateBackendMessage } from '../i18n';

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
    saveTaggingReport = ref(false);
    selectionError = ref('');
    selectionLoading = ref(false);
    taggingSelection: Ref<{schema: number; id: string; tracks: {track_id: string; local_path: string}[]} | null> = ref(null);
    manualTag: Ref<ManualTag> = ref(new ManualTag());

    // Websocket
    private ws!: WebSocket;
    private wsPromiseResolve?: (_: any) => void;
    private wsPromise?;
    private reconnectAttempts = 0;
    private settingsLoaded = false;
    private platformsLoaded = false;
    private leaving = false;
    private reconnectTimer?: number;
    private stateTimer?: number;

    // Quicktag track loading
    private nextQTTrack?: QTTrack;
    private quickTagSaves = new Requests<void>();
    private quickTagSaveSequence = 0;
    private quickTagSave?: Promise<boolean>;
    private librarySocket?: WebSocket;
    private librarySequence = 0;

    // Dialog when loading platforms
    loadingPlatformsDialog?: DialogChainObject;

    static getInstance(): OneTagger { return OneTagger.instance ?? new OneTagger(); }

    private constructor() {
        // Singleton
        if (OneTagger.instance) {
            return OneTagger.instance;
        }
        OneTagger.instance = this;


        // WS connection promise
        this.wsPromise = new Promise((res) => this.wsPromiseResolve = res);
        this.connect();
        window.addEventListener('pagehide', () => {
            this.leaving = true;
            clearTimeout(this.reconnectTimer);
            clearTimeout(this.stateTimer);
            this.ws.close(1000);
            this.cancelQuickTagLoad();
        });
        window.addEventListener('pageshow', event => {
            if (event.persisted) { this.leaving = false; this.connect(); }
        });

        // Keybinds
        document.addEventListener('keydown', (e) => {
            // Can be safely error ignored
            // @ts-ignore
            if (e.target && e.target.nodeName == "INPUT") return true;

            // Dev tools
            if (e.key == 'F3') {
                Dialog.create({ component: DevToolsVue });
                e.preventDefault();
                e.stopPropagation();
                return true;
            }

            if (this.handleKeyDown(e)) {
                e.preventDefault();
                return false;
            }
        });
    }

    private connect() {
        const socket = new WebSocket(wsUrl());
        this.ws = socket;
        socket.addEventListener('close', () => {
            if (socket !== this.ws || this.leaving) return;
            clearTimeout(this.stateTimer);
            this.taggerStatus.value.retrySubmitting = false;
            this.loadingPlatformsDialog?.hide();
            this.loadingPlatformsDialog = undefined;
            this.onTagEditorEvent({action: 'tagEditorDisconnected'});
            this.quickTagSaves.disconnect(new Error(t('tools.connectionClosed')));
            this.player.value.disconnected();
            if (this.reconnectAttempts === 0) this.onError(t('core.websocketClosed'));
            if (this.reconnectAttempts < 5) {
                this.reconnectTimer = window.setTimeout(() => this.connect(), 250 * 2 ** this.reconnectAttempts++);
            } else {
                this.wsPromiseResolve?.(null);
                this.wsPromiseResolve = undefined;
            }
        });
        socket.addEventListener('open', () => {
            this.wsPromiseResolve?.(null);
            this.wsPromiseResolve = undefined;
            this.reconnectAttempts = 0;
            if (!this.settingsLoaded) this.send('loadSettings');
            if (!this.info.value.ready) this.send('init');
            // A new connection has its own backend context. Refresh read-only
            // state without replaying edits or starting another tagging run.
            this.send('spotifyAuthorized');
            this.send('taggingState');
        });
        socket.addEventListener('message', event => {
            if (socket !== this.ws) return;
            const data = JSON.parse(event.data);
            if (data.action) this.incomingEvent(data);
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
            title: t('core.error'),
            html: false,
            message: `${msg}`,
            ok: {
                color: 'primary'
            }
        });

        console.error(msg);
    }

    get isConnected(): boolean {
        return this.ws.readyState === WebSocket.OPEN;
    }

    // Send action to WebSocket
    async send(action: string, params = {}) {
        // Wait for WS connection
        if (this.wsPromise) {
            await this.wsPromise;
            this.wsPromise = undefined;
        }
        if (!this.isConnected) throw new Error(t('core.websocketClosed'));
        let data = { action };
        Object.assign(data, params);
        this.ws.send(JSON.stringify(data));
    }

    // Process incoming event
    private async incomingEvent(json: any) {
        /*eslint-disable no-case-declarations*/
        switch (json.action) {
            // Initial info
            case 'init':
                // Fill AppInfo
                Object.assign(this.info.value, json);
                this.info.value.platforms = [];
                // Path from args
                if (json.startContext.startPath) {
                    this.settings.value.path = json.startContext.startPath;
                    this.config.value.path = json.startContext.startPath;
                }

                this.info.value.ready = true;
                break;
        
            // Load log
            case 'log':
                this.info.value.log = json.log;
                break;

            // Settings loaded
            case 'loadSettings':
                this.settingsLoaded = true;
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
                this.taggerStatus.value.retrySubmitting = false;
                // Unlock, callback
                this.lock.value.locked = false;
                this.onError(translateBackendMessage(json));
                break;
            case 'taggingState': {
                const status = this.taggerStatus.value;
                clearTimeout(this.stateTimer);
                // Loading or replacing provider plugins takes the same registry
                // lock as the tagging workers. Wait until an active run finishes.
                if ((!json.runId || json.completion) && !this.platformsLoaded && !this.loadingPlatformsDialog) this.loadPlatforms();
                if (!json.runId) break;
                if (status.runId !== json.runId) {
                    status.reset();
                    status.runId = json.runId;
                    status.recovered = true;
                    if (!json.completion) router.replace(json.type === 'autoTagger' ? '/autotagger/status' : '/audiofeatures/status');
                }
                status.type = json.type;
                status.total = json.files;
                status.progress = json.progress;
                if (json.completion) {
                    status.done = true;
                    status.cancelled = json.completion.cancelled;
                    status.data = json.completion.data;
                    status.retryFiles = json.retryCount ?? 0;
                    status.retryAt = Date.now() + (json.retryAfterMs ?? 0);
                    if (!status.cancelled) status.progress = 1;
                } else {
                    status.recovered = true;
                    this.stateTimer = window.setTimeout(() => {
                        if (this.isConnected) this.send('taggingState');
                    }, 1200);
                }
                this.lock.value.locked = !status.done;
                break;
            }
            case 'startTagging':
                clearTimeout(this.stateTimer);
                this.lock.value.locked = true;
                this.taggerStatus.value.reset();
                this.taggerStatus.value.runId = json.runId;
                this.taggerStatus.value.total = json.files;
                this.taggerStatus.value.type = json.type;
                break;
            // Status
            case 'taggingProgress':
                if (this.taggerStatus.value.done) break;
                this.taggerStatus.value.recordProgress(json.status);
                break;
            // Tagging done
            case 'taggingDone':
                this.taggerStatus.value.retryFiles = json.retryCount ?? 0;
                this.taggerStatus.value.retryAt = Date.now() + (json.retryAfterMs ?? 0);
                this.lock.value.locked = false;
                this.taggerStatus.value.done = true;
                this.taggerStatus.value.cancelled = json.cancelled === true;
                if (!this.taggerStatus.value.cancelled) this.taggerStatus.value.progress = 1.0;
                this.taggerStatus.value.data = json.data;
                this.onTaggingDone(json.path);
                if (json.data?.reportError) {
                    Dialog.create({ title: t('core.reportSaveFailed'), html: false, message: json.data.reportError });
                }
                if (json.data?.moveErrors?.length) {
                    Dialog.create({ title: t('core.moveFailed'), html: false,
                        message: json.data.moveErrors.join('\n') });
                }
                break;
            // Player load track
            case 'playerLoad':
                this.player.value.loaded(json);
                break;
            case 'playerSync':
                this.player.value.synced(json);
                break;
            // Quicktag
            case 'quickTagLoad':
                this.lock.value.locked = false;
                this.quickTag.value.tracks = json.data.files.map((t: QuickTagFile) => new QTTrack(t, this.settings.value.quickTag));
                this.quickTag.value.failed = json.data.failed;
                this.onQuickTagEvent('quickTagLoad');
                break;
            /*eslint-disable no-case-declarations*/
            case 'quickTagSaved': {
                if (!this.quickTagSaves.has(json.requestId, json.path)) break;
                if (json.status !== 'ok') {
                    this.quickTagSaves.finish(json.requestId, json.path, undefined, new Error(translateBackendMessage(json)));
                    break;
                }
                const track = new QTTrack(json.file, this.settings.value.quickTag);
                const i = this.quickTag.value.tracks.findIndex(t => t.path === json.path);
                if (i !== -1) this.quickTag.value.tracks[i] = track;
                const selected = this.quickTag.value.track.tracks.findIndex(t => t.path === json.path);
                if (selected !== -1) this.quickTag.value.track.tracks[selected] = new QTTrack(json.file, this.settings.value.quickTag);
                this.quickTagSaves.finish(json.requestId, json.path);
                this.onQuickTagEvent('quickTagSaved');
                break;
            }
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
            // Manual tag applied tags
            case 'manualTagApplied':
                this.manualTag.value._resolveSaving!(json);
                break;

            // Callback from config
            case 'configCallback':
                let response = json.response as ConfigCallbackResponse;
                switch (response.type) {
                    case 'empty':
                        break;
                    case 'error':
                        this.onError(response.error);
                        break;
                    case 'updateConfig':
                        if (['__proto__', 'constructor', 'prototype'].includes(json.platform)) {
                            console.error(`Invalid platform key: ${json.platform}`);
                            break;
                        }
                        Object.assign(this.config.value.custom[json.platform], response.config);
                        break;
                }
                break;
            // Platforms loaded
            case 'loadPlatforms':
                this.platformsLoaded = true;
                this.info.value.platforms = json.platforms;
                this.send('defaultCustomPlatformSettings');
                this.loadingPlatformsDialog?.hide();
                this.loadingPlatformsDialog = undefined;
                break;

            // Show repo dialog
            case 'repoManifest':
                Dialog.create({
                    component: PlatformsRepoVue,
                    componentProps: {
                        manifest: json.manifest
                    }
                });
                break;

            // Platform installed
            case 'installPlatform':
                let isOk = json.status == 'ok';
                Dialog.create({
                    title: isOk ? t('core.platformInstalled') : t('core.platformInstallFailed'),
                    message: isOk ? t('core.platformInstallSuccess') : translateBackendMessage(json),
                    html: false,
                    persistent: true,
                    cancel: false,
                    ok: {
                        color: 'primary'
                    }
                })
                .onOk(() => {
                    window.location.reload();
                });
                break;

            // Show notification
            case 'notify':
                Notify.create({
                    position: 'top-right',
                    timeout: 5000,
                    progress: true,
                    message: translateBackendMessage(json),
                    html: false,
                });
                break

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
        let route = router.currentRoute.value.path.substring(1).split('/')[0].toLowerCase();

        switch (json.action) {
            // Drag and drop path
            case 'browse':
                // Callback by route
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

            // Exit app
            case 'exit':
                // QT pending changes
                if (route == 'quicktag' && this.quickTag.value.track.isChanged()) {
                    Dialog.create({
                        component: ExitDialog,
                        componentProps: {
                            isQt: true
                        },
                        persistent: true,
                        ok: false,
                        cancel: false,
                    });
                }
                // AT
                else if (this.lock.value.locked && !this.taggerStatus.value.done) {
                    Dialog.create({
                        component: ExitDialog,
                        componentProps: {
                            isQt: false
                        },
                        persistent: true,
                        ok: false,
                        cancel: false,
                    });
                }
                // Exit anyway
                else {
                    try {
                        // @ts-ignore
                        window.ipc.postMessage('exit');
                    } catch (e) {
                        this.send('exit');
                    }
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
                message: t('core.settingsSaved'),
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
        if (profile == null) throw Error(t('core.invalidProfile'));

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

    /// Load AT platforms
    loadPlatforms() {
        this.platformsLoaded = false;
        this.send('loadPlatforms');
        this.loadingPlatformsDialog = Dialog.create({
            title: t('core.loadingPlatforms'),
            progress: {
                color: 'primary'
            },
            persistent: true,
            ok: false,
            cancel: false,
        });
    }

    addQTTrack(track: QTTrack) {
        this.quickTag.value.track.addTrack(new QTTrack(JSON.parse(JSON.stringify(track)), this.settings.value.quickTag));
    }

    /// Add or remove a track to the multitrack
    toggleQTTrack(track: QTTrack) {
        if (this.quickTag.value.track.getTrack(track.path)) {
            this.quickTag.value.track.removeTrack(track);
            return;
        }
        this.addQTTrack(track);
    }

    // Save quickTagTrack
    saveQTTrack(): Promise<boolean> {
        if (this.quickTagSave) return this.quickTagSave;
        this.quickTagSave = this.saveQuickTagChanges().finally(() => { this.quickTagSave = undefined; });
        return this.quickTagSave;
    }

    private async saveQuickTagChanges(): Promise<boolean> {
        const changes = this.quickTag.value.track.getOutputs() as {path: string}[];
        this.quickTag.value.saving = changes.length;
        try {
            // Limit in-flight writes and retain every edit without a confirmed save.
            for (let offset = 0; offset < changes.length; offset += 4) {
                const results = await Promise.all(changes.slice(offset, offset + 4).map(async change => {
                    const requestId = `quick-tag-${++this.quickTagSaveSequence}`;
                    try {
                        await this.quickTagSaves.run(requestId, change.path,
                            () => this.send('quickTagSave', {requestId, changes: change}));
                        return null;
                    } catch (error) { return String(error); }
                }));
                const error = results.find(result => result !== null);
                if (error) { this.onError(t('tools.saveUnconfirmed', {detail: error})); return false; }
            }
            return true;
        } finally { this.quickTag.value.saving = 0; }
    }


    // Quicktag
    cancelQuickTagLoad() {
        const socket = this.librarySocket;
        this.librarySocket = undefined;
        socket?.close();
        this.quickTag.value.loading = false;
        if (!this.taggerStatus.value.runId || this.taggerStatus.value.done) this.lock.value.locked = false;
    }

    loadQuickTag(playlist?: Playlist, limit = true) {
        this.cancelQuickTagLoad();
        if (!playlist && !this.settings.value.path) return;
        const requestId = `library-${++this.librarySequence}`;
        const socket = new WebSocket(wsUrl());
        this.librarySocket = socket;
        this.quickTag.value.loading = true;
        this.quickTag.value.stats = undefined;
        this.quickTag.value.wasLimited = false;
        this.quickTag.value.tracks = [];
        this.quickTag.value.failed = [];
        this.quickTag.value.track.removeAll();
        this.lock.value.locked = true;
        let finished = false;
        const finish = (error?: string) => {
            if (socket !== this.librarySocket) return;
            finished = true;
            this.librarySocket = undefined;
            this.quickTag.value.loading = false;
            this.lock.value.locked = false;
            socket.close();
            this.onQuickTagEvent('quickTagLoad');
            if (error) this.onError(error);
        };
        socket.onopen = () => socket.send(JSON.stringify({action: 'quickTagLoad', requestId, progressive: true,
            ...(playlist ? {playlist} : {path: this.settings.value.path}),
            recursive: this.settings.value.quickTag.recursive,
            separators: this.settings.value.quickTag.separators, limit}));
        socket.onmessage = event => {
            if (socket !== this.librarySocket) return;
            try {
                const response = JSON.parse(event.data);
                // Accept the original single-response protocol from older servers.
                if (response.action === 'quickTagLoad') {
                    this.incomingEvent(response);
                    this.quickTag.value.wasLimited = limit && this.quickTag.value.tracks.length + this.quickTag.value.failed.length === 500;
                    finish(); return;
                }
                if (response.action === 'error') { finish(translateBackendMessage(response)); return; }
                if (response.requestId !== requestId) return;
                if (response.action === 'quickTagLoadBatch') {
                    this.quickTag.value.tracks.push(...response.data.files.map((file: QuickTagFile) => new QTTrack(file, this.settings.value.quickTag)));
                    this.quickTag.value.failed.push(...response.data.failed);
                    if (this.quickTag.value.tracks.length) this.lock.value.locked = false;
                } else if (response.action === 'quickTagLoadDone') {
                    this.quickTag.value.stats = response.stats;
                    this.quickTag.value.wasLimited = response.stats?.limited === true;
                    finish(response.status === 'error' ? translateBackendMessage(response) : undefined);
                }
            } catch (error) { finish(String(error)); }
        };
        socket.onerror = () => finish(t('core.websocketClosed'));
        socket.onclose = () => { if (!finished) finish(t('core.websocketClosed')); };
        this.saveSettings(false);
    }

    /// Stop the tagging process
    stopTagging(): Promise<void> {
        return new Promise((resolve, reject) => {
            const ws = new WebSocket(wsUrl());
            const finish = (error?: Error) => {
                clearTimeout(timer);
                ws.onclose = null; ws.onerror = null; ws.onmessage = null;
                ws.close();
                if (error) reject(error); else resolve();
            };
            const timer = setTimeout(() => finish(new Error(t('core.stopNotAcknowledged'))), 10000);
            ws.onopen = () => ws.send(JSON.stringify({action: 'stopTagging'}));
            ws.onmessage = (event) => {
                try {
                    const response = JSON.parse(event.data);
                    if (response.action === 'stopTagging' && response.accepted) finish();
                    else if (response.action === 'error') finish(new Error(translateBackendMessage(response)));
                } catch { finish(new Error(t('core.invalidStopAcknowledgement'))); }
            };
            ws.onerror = () => finish(new Error(t('core.stopSendFailed')));
            ws.onclose = () => finish(new Error(t('core.stopConnectionClosed')));
        });
    }

    // Handle keydown event for keyboard bindings
    handleKeyDown(event: KeyboardEvent) {
        if (this.quickTag.value.saving > 0) return false;
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
                this.saveQTTrack().then(ok => {
                    if (!ok) return;
                    Notify.create({
                        message: t('core.trackSaved'),
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
            if ((event.code == "Delete" && (event.ctrlKey || event.metaKey)) || (event.code == 'Backspace' && (event.ctrlKey || event.metaKey))) {
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
    arch: string;
    customPlatformCompat: number;
    ready: boolean;
    platforms: AutotaggerPlatform[];
    renamerDocs: any,
    commit: string,
    workDir: string,
    dataDir: string,
    startContext: { serverMode: boolean, expose: boolean, startPath?: string, browser: boolean },
    log?: string,
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
    return OneTagger.getInstance();
}

export { OneTagger, get1t };
