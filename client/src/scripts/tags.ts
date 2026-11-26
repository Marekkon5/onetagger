import { translate } from '../i18n';

const TAG_LABEL_KEYS: Record<string, string> = {
    "TIT2": 'title',
    "TCON": 'genre',
    "TALB": 'album',
    "TPE2": 'albumArtist',
    "TCOM": 'composer',
    "TEXT": 'lyricist',
    "TIT3": 'mixName',
    "TOPE": 'originalArtist',
    "TIT1": 'contentGroup',
    "GRP1": 'groupingItunes',
    "TPUB": 'label',
    "TPE4": 'remixer',
    "IPLS": 'producer23',
    "TIPL": 'producer24',
    "TPE3": 'conductor',
    "TBPM": 'bpm',
    "TCOP": 'copyright',
    "TDAT": 'releaseDate',
    "TDOR": 'originalYear',
    "TDRL": 'releaseTime',
    "TKEY": 'key',
    "TLEN": 'length',
    "TMOO": 'mood',
    "TPE1": 'artist',
    "TPOS": 'discNumber',
    "TRCK": 'trackNumber',
    "TSRC": 'isrc',
    "TYER": 'year',
    "TDRC": 'recordedDate',
    "COMM": 'comment',
    "USLT": 'unsynchronizedLyrics',
    "©alb": 'album',
    "aART": 'albumArtist',
    "tmpo": 'bpm',
    "©cmt": 'comment',
    "cpil": 'compilationItunes',
    "©wrt": 'composer',
    "cond": 'conductor',
    "cprt": 'copyright',
    "©day": 'yearDate',
    "desc": 'description',
    "disk": 'diskNumber',
    "©too": 'encodedBy',
    "©gen": 'genre',
    "©ART": 'artist',
    "©lyr": 'lyrics',
    "©grp": 'grouping',
    "©ope": 'originalArtist',
    "©wrk": 'work',
    "soaa": 'albumArtistSort',
    "soal": 'albumSort',
    "soar": 'artistSort',
    "soco": 'composerSort',
    "MVNM": 'movementName',
    "©prd": 'producer',
    "©mvn": 'movementName',
    "©nam": 'title'
};

const MP4 = [
    "©nam (Title)",
    "©alb (Album)", 
    "©ART (Artist)",
    "tmpo (BPM)", 
    "©cmt (Comment)", 
    "cpil (Compilation iTunes)",
    "©wrt (Composer)", 
    "cond (Conductor)", 
    "cprt (Copyright)",
    "desc (Description)", 
    "disk (Disk Number)", 
    "©too (Encoded By)", 
    "©gen (Genre)",
    "©grp (Grouping)",
    "©lyr (Lyrics)",
    "©ope (Original Artist)",
    "©wrk (Work)",
    "©day (Year/Date)",
    "aART (Album Artist)",
    "©prd (Producer)",
    "©mvn (Movement Name)",
    "LABEL",
    "CATALOGNUMBER"
];

const VORBIS = [
    "ALBUM", 
    "ALBUMARTIST", 
    "COMMENT",
    "COMPOSER",
    "CONDUCTOR",  
    "GENRE", 
    "GROUPING",
    "LABEL",
    "LYRICS", 
    "ORGANIZATION", 
    "PUBLISHER", 
    "MIXARTIST", 
    "REMIXER", 
    "VERSION", 
];

const ID3 = [
    "TIT2 (Title)",
    "TPE1 (Artist)",
    "TALB (Album)", 
    "TPE2 (Album Artist)", 
    "COMM (Comment)",
    "TCOM (Composer)",
    "TPE3 (Conductor)", 
    "TIT1 (Content Group)",
    "TCON (Genre)", 
    "GRP1 (Grouping iTunes)", 
    "TPUB (Label)", 
    "TEXT (Lyricist)", 
    "TIT3 (Mix Name)", 
    "TOPE (Original Artist)", 
    "IPLS (Producer ID3v2.3)",
    "TIPL (Producer ID3v2.4)", 
    "TPE4 (Remixer)", 
    "USLT (Unsynchronized Lyrics)",
    "MVNM (Movement Name)"
];

// Tag editor payloads mirror onetagger-ui/src/tageditor.rs.
export interface TagEditorImage {
    mime: string;
    data: string;
    kind: string;
    description: string;
    width: number;
    height: number;
}

export interface ID3Comment {
    lang: string;
    description: string;
    text: string;
}

/** Translate only the label; tag identifiers and stored values never change. */
export function tagDescription(frame: string): string | undefined {
    const key = Object.hasOwn(TAG_LABEL_KEYS, frame) ? TAG_LABEL_KEYS[frame] : undefined;
    return key ? translate('tools.' + key) : undefined;
}

export function tagOptions(format: string): string[] {
    const source = format === 'id3' ? ID3 : format === 'mp4' ? MP4 : format === 'vorbis' ? VORBIS : [];
    return source.map(option => {
        const match = option.match(/^(.{4}) \(.*\)$/u);
        if (!match) return option;
        const label = tagDescription(match[1]);
        return label ? `${match[1]} (${label})` : option;
    });
}

const COVER_LABEL_KEYS: Record<string, string> = {
    CoverFront: 'coverFront', CoverBack: 'coverBack', Other: 'coverOther', Artist: 'artist',
    Icon: 'coverIcon', OtherIcon: 'coverOtherIcon', Leaflet: 'coverLeaflet', Media: 'coverMedia',
    LeadArtist: 'coverLeadArtist', Conductor: 'conductor', Band: 'coverBand', Composer: 'composer',
    Lyricist: 'lyricist', RecordingLocation: 'coverRecordingLocation', DuringRecording: 'coverDuringRecording',
    DuringPerformance: 'coverDuringPerformance', ScreenCapture: 'coverScreenCapture', BrightFish: 'coverBrightFish',
    Illustration: 'coverIllustration', BandLogo: 'coverBandLogo', PublisherLogo: 'coverPublisherLogo',
};
export function coverLabel(kind: string): string {
    return Object.hasOwn(COVER_LABEL_KEYS, kind) ? translate('tools.' + COVER_LABEL_KEYS[kind]) : kind;
}

export { MP4, VORBIS, ID3 };
