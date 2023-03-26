import { FrameName, Separators } from "./utils";

interface AutotaggerPlatform {
    id: string;
    builtIn: boolean;
    platform: PlatformInfo;
    icon: string;
    requiresAuth: boolean;
    supportedTags: string[];
}

interface PlatformInfo {
    id: string;
    name: string;
    description: string;
    version: string;
    maxThreads: number;
    customOptions: { options: CustomPlatformOption[] }
}

interface CustomPlatformOption {
    id: string;
    label: string;
    tooltip?: string;
    value: CustomPlatformValue
}

// boolean, number, string, tag, option
type CustomPlatformValue = 
    | { type: "boolean", value: boolean }
    | { type: "number", min: number, max: number, step: number, value: number }
    | { type: "string", value: string, hidden?: boolean }
    | { type: "tag", value: FrameName }
    | { type: "option", values: string[], value: string }


interface AutotaggerProfile {
    name: string,
    config: AutotaggerConfig
}

class AutotaggerConfig {
    platforms: string[] = ['beatport'];
    path?: string;
    title: boolean = false;
    artist: boolean = false;
    albumArtist: boolean = false;
    album: boolean = false;
    key: boolean = false;
    bpm: boolean = true;
    genre: boolean = true;
    style: boolean = true;
    label: boolean = true;
    duration: boolean = false;
    releaseDate: boolean = true;
    publishDate: boolean = false;
    albumArt: boolean = false;
    otherTags: boolean = false;
    url: boolean = false;
    trackId: boolean = false;
    releaseId: boolean = false;
    version: boolean = false;
    remixer: boolean = false;
    trackNumber: boolean = false;
    trackTotal: boolean = false;
    discNumber: boolean = false;
    catalogNumber: boolean = false;
    isrc: boolean = false;
    mood: boolean = false;
    syncedLyrics: boolean = false;
    unsyncedLyrics: boolean = false;
    metaTags: boolean = false;
    separators: Separators = new Separators();
    id3v24: boolean = true;
    overwrite: boolean = true;
    threads: number = 16;
    strictness: number = 0.7;
    mergeGenres: boolean = false;
    albumArtFile: boolean = false;
    camelot: boolean = false;
    parseFilename: boolean = false;
    filenameTemplate: string = '%artists% - %title%';
    shortTitle: boolean = false;
    matchDuration: boolean = false;
    maxDurationDifference: number = 30;
    matchById: boolean = false;
    multipleMatches: string = 'Default';
    postCommand?: string;
    stylesOptions: string = 'default';
    trackNumberLeadingZeroes: number = 0;
    enableShazam: boolean = false;
    forceShazam: boolean = false;
    skipTagged: boolean = false;
    onlyYear: boolean = false;
    includeSubfolders: boolean = true;
    stylesCustomTag: FrameName = FrameName.same('STYLE');
    multiplatform: boolean = false;
    titleRegex?: string;
    moveSuccess: boolean = false;
    moveSuccessPath?: string;
    moveFailed: boolean = false;
    moveFailedPath?: string;
    writeLrc: boolean = false;
    enhancedLrc: boolean = false;

    spotify?: SpotifyConfig;

    // { platform: { option1: value, option2: value ... }, ... }
    custom: Record<string, Record<string, any>> = {};

    // autotagger or audiofeatures, not actually serialized, dynamically changed
    type?: string;

    /// Load settings from JSON
    loadSettings(data: any) {
        Object.assign(this, data);
        this.stylesCustomTag = Object.assign(FrameName.same('STYLE'), data.stylesCustomTag);
        this.separators = Object.assign(new Separators(), data.separators);
    }
}

interface SpotifyConfig {
    clientId: string;
    clientSecret: string;
}

class TaggerStatus {
    statuses: TaggingStatusWrap[] = [];
    started: number = 0;
    progress: number = 0.0;
    done: boolean = false;
    total: number = 0;
    ok: number = 0;
    type?: 'autoTagger' | 'audioFeatures';
    data?: any;

    reset() {
        this.statuses = [];
        this.started = Date.now();
        this.done = false;
        this.progress = 0.0;
        this.total = 0;
        this.ok = 0;
        this.type = undefined;
        this.data = undefined;
    }
}

interface TaggingStatusWrap {
    status: TaggingStatus;
    platform: string;
    progress: number;
}

interface TaggingStatus {
    status: 'ok' | 'error' | 'skipped';
    path: 'string',
    message?: string;
    accuracy?: number;
    usedShazam: boolean
}



export type { AutotaggerPlatform, PlatformInfo, AutotaggerProfile };
export { AutotaggerConfig, TaggerStatus };