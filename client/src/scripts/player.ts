import { get1t, OneTagger } from "./onetagger";
import { httpUrl, wsUrl } from "./utils";

const WAVES = 180;

class Player {
    waveform: number[] = [];
    playing: boolean = false;
    position: number = 0;
    duration: number = 1;
    volume: number = 0.5;
    wasPlaying: boolean = false;
    title?: string;
    artists: string[] = [];
    audio: HTMLAudioElement = new Audio();
    path?: string;

    // Using _ instead of private because of reactivity proxy
    _waveformLock: boolean[] = [];
    _waveformPath?: string;

    constructor($1t: OneTagger) {
        // Setup
        this.generateDefaultWaveform();
        setInterval(() => {
            if (this.position > this.duration) {
                // Auto play next track
                if ($1t.settings.value.autoPlayNext) {
                    $1t.onQuickTagEvent('changeTrack', { offset: 1 });
                } else {
                    this.pause();
                }
            }

            if (this.playing)
                this.position += 150;
        }, 150);
    }

    // Stop & reset player
    stop() {
        this.pause();
        const $1t = get1t();
        $1t.send('playerStop');
        this.position = 0;
        this.duration = 1;
        this.title = undefined;
        this.artists = [];
        this.generateDefaultWaveform();
    }

    // Fill waveform array with empty data
    generateDefaultWaveform() {
        for (let i=0; i < WAVES; i++) {
            this.waveform[i] = 0;
        }
    }

    // Start generating waveform in background
    async generateWaveform(path: string) {
        this._waveformPath = path;
        // Resolve locks
        this._waveformLock.push(true);
        while (this._waveformLock.length > 1) {
            await new Promise((res) => setTimeout(() => res(null), 50));
            if (path != this._waveformPath) {
                this._waveformLock.pop();
                return;
            }
        }
        await this._generateWaveformInner(path);
    }

    // Actually generate the waveform
    async _generateWaveformInner(path: string) {
        const $1t = get1t();
        this.generateDefaultWaveform();
        let waveformIndex = 0;
        // Separate socket = separate thread
        let ws = new WebSocket(wsUrl());
        ws.addEventListener('message', (event) => {
            // Lock
            if (this._waveformLock.length > 1) {
                ws.close();
                this._waveformLock.pop();
                return;
            }

            let json = JSON.parse(event.data);
            // New wave
            if (json.action == 'waveformWave') {
                this.waveform[waveformIndex] = json.wave;
                waveformIndex++;
            }
            // Finish
            if (json.action == 'waveformDone' || json.action == 'error') {
                ws.close();
                this._waveformLock.pop();

                // Autoplay, delay just in case for windows
                setTimeout(() => {
                    if ($1t.settings.value.continuePlayback && this.wasPlaying) {
                        this.play();
                        this.wasPlaying = false;
                    }
                }, 100);
            }
            // Will be ignored, just for updating
            ws.send(JSON.stringify({action: '_waveformRead'}));
        });
        // Start loading on connection
        ws.addEventListener('open', (_) => {
            ws.send(JSON.stringify({
                action: 'waveform',
                path
            }));
        });

        // Auto play on unix systems (on Windows it causes errors, because file is read twice)
        if ($1t.info.value.os != 'windows' && $1t.settings.value.continuePlayback && this.wasPlaying) {
            this.play();
            this.wasPlaying = false;
        }
    }

    // Start playback
    play() {
        const $1t = get1t();

        // External
        if ($1t.settings.value.externalAudioPlayer) {
            $1t.send('openFile', { path: this.path });
            return;
        }
            
        // Client side player
        if ($1t.settings.value.clientSidePlayer) {
            this.playing = false;
            this.audio.play();
            return;
        }

        // Normal
        $1t.send("playerPlay");
        this.playing = true;
    }

    // Pause playback
    pause() {
        const $1t = get1t();
        $1t.settings.value.clientSidePlayer ? this.audio.pause() : $1t.send("playerPause");
        this.playing = false;
        this.wasPlaying = false;
    }

    // Seek to position (ms)
    seek(pos: number) {
        const $1t = get1t();
        this.playing = false;

        if ($1t.settings.value.clientSidePlayer) {
            this.audio.currentTime = pos / 1000.0;
        } else {
            $1t.send("playerSeek", { pos })
        }
        this.position = pos;
    }

    // Set volume
    setVolume(volume: number | undefined | null) {
        if (!volume) return;
        this.audio.volume = volume;
        get1t().send("playerVolume", {volume});
    }

    // Load track to play
    loadTrack(path: string) {
        const $1t = get1t();

        // Setup client-side audio player
        if ($1t.settings.value.clientSidePlayer) {
            this.audio.pause();
            this.playing = false;
            this.audio = new Audio(`${httpUrl()}/audio?path=${encodeURIComponent(path)}`);
            this.audio.volume = this.volume;
            const cb = () => {
                this.playing = !this.audio.paused;
                this.position = Math.round(this.audio.currentTime * 1000);
            }
            this.audio.addEventListener('playing', cb);
        }

        // Server side
        $1t.send("playerLoad", { path });
        this.generateWaveform(path);

        this.path = path;
    }
}

export { Player, WAVES };