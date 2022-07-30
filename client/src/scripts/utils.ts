// Returns the WebSocket server URL
function wsUrl(): string {
    return `ws://${window.location.hostname}:36912`;
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
}

// Keybind data
class Keybind {
    ctrl: boolean = false;
    key?: string;
    alt: boolean = false;
    shift: boolean = false;
}

// Spotify auth data
class Spotify {
    clientId?: string;
    clientSecret?: string;
    authorized: boolean = false;
}

export { wsUrl, Separators, FrameName, Keybind, Spotify };