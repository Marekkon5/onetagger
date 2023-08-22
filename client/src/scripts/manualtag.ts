import { AutotaggerConfig, Track } from "./autotagger";
import { get1t } from "./onetagger";
import { wsUrl } from "./utils";

class ManualTag {

    ws?: WebSocket;
    busy = false;
    matches: TrackMatch[] = [];
    errors: ManualTagError[] = [];
    
    _resolveSaving?: Function;

    constructor() {}

    /// Reset current state
    reset() {
        if (this.ws) {
            this.ws.close();
            this.ws = undefined;
        }
        this.matches = [];
        this.errors = [];
        this.busy = false;
    }

    /// Start tagging a track
    tagTrack(path: string, config: AutotaggerConfig) {
        this.reset();
        this.busy = true;

        // Open new WS connection because separate thread
        this.ws = new WebSocket(wsUrl());
        this.ws.addEventListener('message', (ev) => {
            this.onWsMessage(JSON.parse(ev.data));
        });
        this.ws.addEventListener('open', () => {
            this.ws!.send(JSON.stringify({
                action: 'manualTag',
                config: config,
                path
            }))
        });
    }

    /// Apply matches
    async apply(matches: TrackMatch[], path: string, config: AutotaggerConfig) {
        // Send to socket and wait for response
        const $1t = get1t();
        let promise = new Promise((res, rej) => this._resolveSaving = res);
        $1t.send('manualTagApply', { matches, path, config });
        let r = await promise;
        this._resolveSaving = undefined;
        return r;
    }

    /// WebSocket message handler
    onWsMessage(json: any) {
        switch (json.action) {
            // New result
            case 'manualTag':
                switch (json.status) {
                    case 'ok':
                        this.addMatches(json.matches, json.platform);
                        break;
                    case 'error':
                        this.errors.push({ platform: json.platform, error: json.error });
                        break;
                }
                break;
            
            // Finished
            case 'manualTagDone':
                this.busy = false;
                this.ws?.close();
                this.ws = undefined;
                break;
        }
    }

    /// Add new matches to array
    addMatches(matches: TrackMatch[], platform: string) {
        this.matches.push(...matches.map((m) => {
            m.track.platform = platform;
            return m;
        }));
        this.matches.sort((a, b) => b.accuracy - a.accuracy);
    }

}

/// Matched track
interface TrackMatch {
    accuracy: number;
    track: Track;
    reason: string;
}

interface ManualTagError {
    platform: string;
    error: string;
}

export type { TrackMatch };
export { ManualTag };
