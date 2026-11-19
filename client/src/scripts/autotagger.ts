import { translate } from '../i18n';
import { FrameName, RustDuration, Separators } from "./utils";

interface AutotaggerPlatform {
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
    | { type: "button" }


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
    fetchAllResults: boolean = false;
    albumTagging: boolean = false;
    albumTaggingRatio: number = 0.5;
    coverFilename: string | undefined = undefined;

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
    statuses: TaggingStatusWrap[][] = [];
    started: number = 0;
    progress: number = 0.0;
    done: boolean = false;
    cancelled: boolean = false;
    total: number = 0;
    ok: number = 0;
    type?: 'autoTagger' | 'audioFeatures';
    data?: any;

    reset() {
        this.statuses = [];
        this.started = Date.now();
        this.done = false;
        this.cancelled = false;
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
    path: string;
    message?: string;
    messageKey?: string;
    messageArgs?: Record<string, string | number>;
    accuracy?: number;
    reason?: string;
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
    { tag: SupportedTag.AlbumArt, get label() { return translate('autotag.tagAlbumArt'); }, get tooltip() { return translate('autotag.tagAlbumArtHint'); } },
    { tag: SupportedTag.Album, get label() { return translate('autotag.tagAlbum'); } },
    { tag: SupportedTag.AlbumArtist, get label() { return translate('autotag.tagAlbumArtist'); } },
    { tag: SupportedTag.Artist, get label() { return translate('autotag.tagArtist'); } },
    { tag: SupportedTag.Title, get label() { return translate('autotag.tagTitle'); } },
    { tag: SupportedTag.Version, get label() { return translate('autotag.tagVersion'); } },
    { tag: SupportedTag.Remixer, get label() { return translate('autotag.tagRemixers'); }, tooltip: '' },
    { tag: SupportedTag.Genre, get label() { return translate('autotag.tagGenre'); }, get tooltip() { return translate('autotag.tagGenreHint'); } },
    { tag: SupportedTag.Style, get label() { return translate('autotag.tagStyle'); }, tooltip: ''},
    { tag: SupportedTag.Label, get label() { return translate('autotag.tagLabel'); } },
    { tag: SupportedTag.ReleaseId, get label() { return translate('autotag.tagReleaseId'); } },
    { tag: SupportedTag.TrackId, get label() { return translate('autotag.tagTrackId'); } },
    { tag: SupportedTag.BPM, label: 'BPM' },
    { tag: SupportedTag.Key, get label() { return translate('autotag.tagKey'); } },
    { tag: SupportedTag.Mood, get label() { return translate('autotag.tagMood'); } },
    { tag: SupportedTag.CatalogNumber, get label() { return translate('autotag.tagCatalogNumber'); } },
    { tag: SupportedTag.TrackNumber, get label() { return translate('autotag.tagTrackNumber'); } },
    { tag: SupportedTag.DiscNumber, get label() { return translate('autotag.tagDiscNumber'); } },
    { tag: SupportedTag.Duration, get label() { return translate('autotag.tagDuration'); } },
    { tag: SupportedTag.TrackTotal, get label() { return translate('autotag.tagTrackTotal'); } },
    { tag: SupportedTag.ISRC, label: 'ISRC' },
    { tag: SupportedTag.PublishDate, get label() { return translate('autotag.tagPublishDate'); }, tooltip: '' },
    { tag: SupportedTag.ReleaseDate, get label() { return translate('autotag.tagReleaseDate'); } },
    { tag: SupportedTag.URL, label: 'URL' },
    { tag: SupportedTag.OtherTags, get label() { return translate('autotag.tagOther'); }, get tooltip() { return translate('autotag.tagOtherHint'); } },
    { tag: SupportedTag.MetaTags, get label() { return translate('autotag.tagOneTagger'); }, get tooltip() { return translate('autotag.tagOneTaggerHint'); } },
    { tag: SupportedTag.UnsyncedLyrics, get label() { return translate('autotag.tagUnsyncedLyrics'); } },
    { tag: SupportedTag.SyncedLyrics, get label() { return translate('autotag.tagSyncedLyrics'); } },
    { tag: SupportedTag.Explicit, get label() { return translate('autotag.tagExplicit'); } },
];

/// Track from 
interface Track {
    platform: string;
    title: string;
    version?: string;
    artists: string[];
    album_artists: string[];
    album?: string;
    key?: string;
    bpm?: number;
    genres: string[];
    styles: string[];
    art?: string;
    url: string;
    label?: string;
    catalog_number?: string;
    other: [FrameName, String][];
    track_id?: string;
    release_id: string;
    duration: RustDuration;
    remixers: string[];
    track_number?: number | string;
    track_total?: number;
    disc_number?: number;
    isrc?: string;
    mood?: string;
    explicit?: boolean;
    lyrics?: any;
    release_year?: number;
    release_date?: string;
    publish_year?: number;
    publish_date?: string;
    thumbnail?: string;
    custom: { [key: string]: string };
}

type ConfigCallbackResponse = 
    { type: 'empty' } |
    { type: 'error', error: string } |
    { type: 'updateConfig', config: any };


interface RepoPlatform {
    id: string;
    name: string;
    author: string;
    description: string;
    requiresAuth: string;
    language: 'rust';
    version: string;
    maxThreads: number;
    versions: { [key: string]: number };
}

export type { AutotaggerPlatform, PlatformInfo, AutotaggerProfile, TaggingStatusWrap, Track, ConfigCallbackResponse, RepoPlatform };
export { AutotaggerConfig, TaggerStatus, SupportedTag, SUPPORTED_TAGS };
