// Returns the WebSocket server URL
function wsUrl(): string {
    return `ws://${window.location.hostname}:36912`;
}

// Returns the HTTP server URL
function httpUrl(): string {
    return `http://${window.location.hostname}:36913`;
}

// Returns the Spotify redirect URL
function spotifyUrl(): string {
    return `http://127.0.0.1:36914/spotify`;
}

// Tag value separators
class Separators {
    id3: string = ', ';
    vorbis?: string;
    mp4: string = ', ';

    constructor(id3?: string, vorbis?: string, mp4?: string) {
        this.id3 = id3??', ';
        this.vorbis = vorbis;
        this.mp4 = mp4??', ';
    }
}

// Frame name in different formats
class FrameName {
    constructor(public id3: string, public vorbis: string, public mp4: string) {}

    // Create new FrameName where the name is same for all formats
    public static same(name: string): FrameName {
        return new FrameName(name, name, name);
    }

    // Create class from JSON
    public static fromJson(json: any): FrameName {
        return Object.assign(FrameName.same(''), json);
    }

    // Get value by audio or tag format
    byFormat(format: string) {
        switch (format) {
            case 'mp3':
            case 'aiff':
            case 'aif':
            case 'id3':
            case 'wav':
                return this.id3;
            case 'flac':
            case 'ogg':
            case 'vorbis':
                return this.vorbis;
            case 'mp4':
            case 'm4a':
                return this.mp4;
            default:
                throw new Error(`Invalid format: ${format}`);
        }
    }
}

// Keybind data
class Keybind {
    ctrl: boolean = false;
    key?: string;
    alt: boolean = false;
    shift: boolean = false;

    // Create class from JSON
    public static fromJson(json: any): Keybind {
        return Object.assign(new Keybind(), json);
    }

    // Check if keybind pressed
    public static check(e: KeyboardEvent, keybind?: Keybind) {
        if (!keybind) return false;
        if (e.code.match(/F\d{1,2}/) || e.code.startsWith('Key') || e.code.startsWith("Digit") || e.code.startsWith("Numpad")) {
            let key = e.code.toLowerCase().replace("key", "").replace("digit", "").replace("numpad", "");
            return (key == keybind.key && 
                e.altKey == keybind.alt && 
                e.shiftKey == keybind.shift && 
                (e.ctrlKey || e.metaKey) == keybind.ctrl);
        }
    }

    // Clear the keybind
    clear() {
        this.ctrl = false;
        this.alt = false;
        this.shift = false;
        this.key = undefined;
    }
}

// Spotify auth data
class Spotify {
    clientId?: string;
    clientSecret?: string;
    authorized: boolean = false;
}

// Playlist data
interface Playlist {
    data?: string;
    format?: string;
    filename?: string;
}

export type { Playlist };
export { wsUrl, httpUrl, spotifyUrl, Separators, FrameName, Keybind, Spotify };