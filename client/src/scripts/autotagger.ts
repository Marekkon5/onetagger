import { FrameName, Separators } from "./utils";

interface AutotaggerPlatform {
    id: string;
    builtIn: boolean;
    platform: PlatformInfo;
    icon: string;
    requiresAuth: boolean;
    supportedTags: SupportedTag[];
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
    tags: SupportedTag[] = [SupportedTag.Genre, SupportedTag.Style, SupportedTag.BPM, SupportedTag.ReleaseDate, SupportedTag.Label];
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
    capitalizeGenres: boolean = false;
    id3CommLang?: string;

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

enum SupportedTag {
    Title = "title",
    Artist = "artist",
    Album = "album",
    Key = "key",
    Genre = "genre",
    Style = "style",
    ReleaseDate = "releaseDate",
    PublishDate = "publishDate",
    AlbumArt = "albumArt",
    OtherTags = "otherTags",
    CatalogNumber = "catalogNumber",
    TrackId = "trackId",
    ReleaseId = "releaseId",
    Version = "version",
    Duration = "duration",
    AlbumArtist = "albumArtist",
    Remixer = "remixer",
    TrackNumber = "trackNumber",
    TrackTotal = "trackTotal",
    DiscNumber = "discNumber",
    Mood = "mood",
    SyncedLyrics = "syncedLyrics",
    UnsyncedLyrics = "unsyncedLyrics",
    Label = "label",
    Explicit = "explicit",
    MetaTags = "metaTags",
    BPM = "bpm",
    URL = "url",
    ISRC = "isrc"
}


export type { AutotaggerPlatform, PlatformInfo, AutotaggerProfile };
export { AutotaggerConfig, TaggerStatus, SupportedTag };