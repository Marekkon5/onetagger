// Returns the WebSocket server URL
function wsUrl(): string {
    return `ws://${window.location.hostname}:36912`;
}

// Returns the HTTP server URL
function httpUrl(): string {
    return `http://${window.location.hostname}:36913`
}

// Tag value separators
class Separators {
    id3?: string = ', ';
    vorbis?: string;
    mp4?: string = ', ';
}

// Frame name in different formats
class FrameName {
    constructor(public id3: string, public vorbis: string, public mp4: string) {}

    // Create new FrameName where the name is same for all formats
    public static same(name: string): FrameName {
        return new FrameName(name, name, name);
    }

    // Get value by audio or tag format
    byFormat(format: string) {
        switch (format) {
            case 'mp3':
            case 'aiff':
            case 'aif':
            case 'id3':
                return this.id3;
            case 'flac':
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

    // Check if keybind pressed
    check(e: KeyboardEvent) {
        if (e.code.match(/F\d{1,2}/) || e.code.startsWith('Key') || e.code.startsWith("Digit") || e.code.startsWith("Numpad")) {
            let key = e.code.toLowerCase().replace("key", "").replace("digit", "").replace("numpad", "");
            return (key == this.key && 
                e.altKey == this.alt && 
                e.shiftKey == this.shift && 
                (e.ctrlKey || e.metaKey) == this.ctrl);
        }
    }
}

// Spotify auth data
class Spotify {
    clientId?: string;
    clientSecret?: string;
    authorized: boolean = false;
}

export { wsUrl, httpUrl, Separators, FrameName, Keybind, Spotify };