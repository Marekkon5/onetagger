import { markRaw } from 'vue';
import { get1t, type OneTagger } from './onetagger';
import { httpUrl, wsUrl } from './utils';
import { Requests } from './requests';

const WAVES = 180;
interface LoadedTrack { duration: number; title?: string; artists: string[] }

class Player {
    waveform: number[] = Array(WAVES).fill(0);
    playing = false;
    position = 0;
    duration = 1;
    volume = 0.5;
    wasPlaying = false;
    title?: string;
    artists: string[] = [];
    audio: HTMLAudioElement = markRaw(new Audio());
    path?: string;
    _playerLoadPromise?: Promise<boolean>;
    _requests = markRaw(new Requests<LoadedTrack>());
    _waveformSocket?: WebSocket;
    _generation = 0;
    _wantedPlay = false;
    _ended = false;
    _syncRequested = 0;
    _fallback = false;
    _starting = false;

    constructor($1t: OneTagger) {
        // Read actual playback state. A delayed/background timer must never
        // advance the track, stop it early, or accumulate an invented position.
        setInterval(() => {
            if (!this.path) return;
            if ($1t.settings.value.clientSidePlayer) {
                this.position = Math.round(this.audio.currentTime * 1000);
            } else if (this._wantedPlay && $1t.isConnected && performance.now() - this._syncRequested >= 500) {
                this._syncRequested = performance.now();
                $1t.send('playerState').catch(() => {});
            }
        }, 500);
    }

    generateDefaultWaveform() { this.waveform = Array(WAVES).fill(0); }

    stop() {
        ++this._generation;
        this._requests.disconnect(new Error('Track changed'));
        this._waveformSocket?.close(); this._waveformSocket = undefined;
        this.audio.pause(); this.audio.removeAttribute('src'); this.audio.load();
        this._wantedPlay = false; this.wasPlaying = false; this.playing = false;
        this.path = undefined; this.position = 0; this.duration = 1;
        this.title = undefined; this.artists = []; this.generateDefaultWaveform();
        if (get1t().isConnected) get1t().send('playerStop').catch(() => {});
    }

    disconnected() {
        this._requests.disconnect(new Error('Connection closed'));
        this._waveformSocket?.close(); this._waveformSocket = undefined;
        this.audio.pause(); this._wantedPlay = false; this.playing = false;
    }

    generateWaveform(path: string, generation = this._generation) {
        this._waveformSocket?.close();
        const socket = markRaw(new WebSocket(wsUrl()));
        this._waveformSocket = socket;
        const requestId = `waveform-${generation}`;
        socket.onopen = () => {
            if (generation !== this._generation) { socket.close(); return; }
            socket.send(JSON.stringify({action: 'waveform', path, requestId}));
        };
        socket.onmessage = event => {
            if (generation !== this._generation) return;
            try {
                const message = JSON.parse(event.data);
                if (message.action === 'waveformDone' && message.requestId === requestId && message.path === path) {
                    this.waveform = message.waves;
                    socket.close();
                } else if (message.action === 'error') socket.close();
            } catch { socket.close(); }
        };
        // A failed waveform never blocks audio or later track selections.
        socket.onerror = () => socket.close();
        socket.onclose = () => { if (this._waveformSocket === socket) this._waveformSocket = undefined; };
    }

    loaded(message: any) {
        if (message.requestId !== `player-${this._generation}` || message.path !== this.path) return;
        if (message.status === 'error') {
            this._requests.finish(message.requestId, message.path, undefined, new Error(message.message));
            return;
        }
        this.duration = Math.max(1, message.duration);
        this.title = message.title; this.artists = message.artists;
        this._requests.finish(message.requestId, message.path, message);
    }

    synced(message: any) {
        if (get1t().settings.value.clientSidePlayer || !this.path) return;
        if (Number.isFinite(message.position)) this.position = message.position;
        this.playing = message.playing === true;
        if (message.seek && get1t().settings.value.playOnSeek && !this.playing && !message.ended) this.play();
        if (message.ended && this._wantedPlay && !message.seek) this.ended();
    }

    ended() {
        if (this._ended) return;
        this._ended = true; this._wantedPlay = false; this.playing = false;
        this.position = this.duration;
        if (get1t().settings.value.autoPlayNext) get1t().onQuickTagEvent('changeTrack', {offset: 1});
    }

    async play() {
        const $1t = get1t();
        if ($1t.settings.value.externalAudioPlayer) {
            await $1t.send('openFile', {path: this.path}); return;
        }
        if (this._playerLoadPromise && !await this._playerLoadPromise) return;
        this._wantedPlay = true; this._ended = false;
        const generation = this._generation;
        if ($1t.settings.value.clientSidePlayer) {
            if (this._starting) return;
            this._starting = true;
            try { await this.audio.play(); }
            catch (error) {
                if (generation !== this._generation) return;
                // Browsers differ in codec support. Unsupported originals get a
                // bounded, seekable disk preview rather than a full WAV in RAM.
                if (!this._fallback && this.audio.error) {
                    this._fallback = true;
                    this.audio.src = `${httpUrl()}/audio?path=${encodeURIComponent(this.path!)}&transcode=true`;
                    this.audio.load();
                    try { await this.audio.play(); return; } catch (failure) { error = failure; }
                }
                this._wantedPlay = false; this.playing = false;
                $1t.onError(String(error));
            } finally { this._starting = false; }
        } else {
            try { await $1t.send('playerPlay'); this.playing = true; }
            catch (error) { this._wantedPlay = false; $1t.onError(String(error)); }
        }
    }

    pause() {
        this._wantedPlay = false; this.wasPlaying = false; this.playing = false;
        if (get1t().settings.value.clientSidePlayer) this.audio.pause();
        else if (get1t().isConnected) get1t().send('playerPause').catch(() => {});
    }

    seek(position: number) {
        const pos = Math.max(0, Math.min(this.duration, Math.round(position)));
        this._ended = false;
        if (get1t().settings.value.clientSidePlayer) {
            this.audio.currentTime = pos / 1000;
            if (get1t().settings.value.playOnSeek && this.audio.paused) this.play();
        } else get1t().send('playerSeek', {pos}).catch(error => get1t().onError(String(error)));
        this.position = pos;
    }

    setVolume(volume: number | undefined | null) {
        if (volume === undefined || volume === null || !Number.isFinite(volume)) return;
        this.volume = Math.max(0, Math.min(1, volume));
        this.audio.volume = this.volume;
        if (get1t().isConnected) get1t().send('playerVolume', {volume: this.volume}).catch(() => {});
    }

    loadTrack(path: string) {
        const $1t = get1t();
        const resume = $1t.settings.value.continuePlayback && this.wasPlaying;
        const generation = ++this._generation;
        this._requests.disconnect(new Error('Track changed'));
        this.audio.pause(); this.audio.removeAttribute('src'); this.audio.load();
        this.playing = false; this._wantedPlay = false; this._ended = false; this._fallback = false; this._starting = false;
        this.path = path; this.position = 0; this.duration = 1;
        this.generateDefaultWaveform();
        if ($1t.settings.value.clientSidePlayer) {
            const audio = markRaw(new Audio(`${httpUrl()}/audio?path=${encodeURIComponent(path)}`));
            this.audio = audio; audio.preload = 'none'; audio.volume = this.volume;
            const update = () => {
                if (generation !== this._generation) return;
                this.position = Math.round(audio.currentTime * 1000);
                this.playing = !audio.paused && !audio.ended;
            };
            for (const event of ['playing', 'pause', 'timeupdate', 'seeked']) audio.addEventListener(event, update);
            audio.addEventListener('ended', () => { if (generation === this._generation) this.ended(); });
            audio.addEventListener('error', () => {
                if (generation === this._generation && this._wantedPlay && !this._fallback && !this._starting) this.play();
            });
            // Release a previously selected native player without opening an audio device.
            $1t.send('playerStop').catch(() => {});
        }
        const requestId = `player-${generation}`;
        this._playerLoadPromise = this._requests.run(requestId, path,
            () => $1t.send('playerLoad', {path, requestId, clientSide: $1t.settings.value.clientSidePlayer}), 15000)
            .then(() => generation === this._generation)
            .catch(error => { if (generation === this._generation) $1t.onError(String(error)); return false; });
        this.generateWaveform(path, generation);
        if (resume) this.play();
        this.wasPlaying = false;
    }
}
export { Player, WAVES };
