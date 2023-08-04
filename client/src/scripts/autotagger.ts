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
    overwriteTags: SupportedTag[] = [];
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
    removeAllCovers: boolean = false;

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

/// All supported tags with labels and tooltips
const SUPPORTED_TAGS = [
    { tag: SupportedTag.AlbumArt, label: 'Album Art', tooltip: 'Resolution is platform dependent' },
    { tag: SupportedTag.Album, label: 'Album' },
    { tag: SupportedTag.AlbumArtist, label: 'Album Artist' },
    { tag: SupportedTag.Artist, label: 'Artist' },
    { tag: SupportedTag.Title, label: 'Title' },
    { tag: SupportedTag.Version, label: 'Version' },
    { tag: SupportedTag.Remixer, label: 'Remixers', tooltip: 'Available from Beatport & Beatsource' },
    { tag: SupportedTag.Genre, label: 'Genre', tooltip: 'Spotify will populate multiple genres based on artist' },
    { tag: SupportedTag.Style, label: 'Style / Subgenre', tooltip: 'Style is available from Discogs & Bandcamp, Subgenre from Beatport only'},
    { tag: SupportedTag.Label, label: 'Label' },
    { tag: SupportedTag.ReleaseId, label: 'Release ID' },
    { tag: SupportedTag.TrackId, label: 'Track ID' },
    { tag: SupportedTag.BPM, label: 'BPM' },
    { tag: SupportedTag.Key, label: 'Key' },
    { tag: SupportedTag.Mood, label: 'Mood' },
    { tag: SupportedTag.CatalogNumber, label: 'Catalog Number' },
    { tag: SupportedTag.TrackNumber, label: 'Track Number' },
    { tag: SupportedTag.DiscNumber, label: 'Disc Number' },
    { tag: SupportedTag.Duration, label: 'Duration' },
    { tag: SupportedTag.TrackTotal, label: 'Track Total' },
    { tag: SupportedTag.ISRC, label: 'ISRC' },
    { tag: SupportedTag.PublishDate, label: 'Publish Date', tooltip: 'Available from Beatport only' },
    { tag: SupportedTag.ReleaseDate, label: 'Release Date' },
    { tag: SupportedTag.URL, label: 'URL' },
    { tag: SupportedTag.OtherTags, label: 'Other Tags', tooltip: 'Specific tags only for some platforms (Beatport, Discogs)' },
    { tag: SupportedTag.MetaTags, label: 'OneTagger Tags', tooltip: 'Adds 1T_TAGGEDDATE tag with timestamp' },
    { tag: SupportedTag.UnsyncedLyrics, label: 'Unsynced Lyrics' },
    { tag: SupportedTag.SyncedLyrics, label: 'Synced Lyrics' },
    { tag: SupportedTag.Explicit, label: 'Explicit' },
];


export type { AutotaggerPlatform, PlatformInfo, AutotaggerProfile };
export { AutotaggerConfig, TaggerStatus, SupportedTag, SUPPORTED_TAGS };