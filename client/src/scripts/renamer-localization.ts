import { translate } from '../i18n';

type SymbolDocumentation = { kind: string; name: string; doc?: string };
type TagDocumentation = [label: string, frames: string, first?: boolean];

// Stable Rust symbol names and tag-frame IDs are syntax/data, never translated.
const variables: Record<string, TagDocumentation> = {
    title: ['title', 'MP3: TIT2; FLAC: TITLE; MP4: ©nam'],
    artist: ['artist', 'MP3: TPE1; FLAC: ARTIST; MP4: ©ART', true],
    artists: ['artist', 'MP3: TPE1; FLAC: ARTIST; MP4: ©ART'],
    album: ['album', 'MP3: TALB; FLAC: ALBUM; MP4: ©alb'],
    albumartist: ['albumArtist', 'MP3: TPE2; FLAC: ALBUMARTIST; MP4: aART', true],
    albumartists: ['albumArtist', 'MP3: TPE2; FLAC: ALBUMARTIST; MP4: aART'],
    key: ['key', 'MP3: TKEY; FLAC: INITIALKEY; MP4: com.apple.iTunes:initialkey'],
    bpm: ['bpm', 'MP3: TBPM; FLAC: BPM; MP4: tmpo'],
    genre: ['genre', 'MP3: TCON; FLAC: GENRE; MP4: ©gen'],
    label: ['label', 'MP3: TPUB; FLAC: LABEL; MP4: com.apple.iTunes:LABEL'],
    style: ['style', 'MP3: STYLE; FLAC: STYLE; MP4: com.apple.iTunes:STYLE'],
    isrc: ['isrc', 'MP3: TSRC; FLAC: ISRC; MP4: com.apple.iTunes:ISRC'],
    catalognumber: ['catalogNumber', 'MP3: CATALOGNUMBER; FLAC: CATALOGNUMBER; MP4: com.apple.iTunes:CATALOGNUMBER'],
    version: ['version', 'MP3: TIT3; FLAC: SUBTITLE; MP4: com.apple.iTunes:SUBTITLE'],
    tracknumber: ['trackNumber', 'MP3: TRCK; FLAC: TRACKNUMBER; MP4: trkn'],
    track: ['trackNumber', 'MP3: TRCK; FLAC: TRACKNUMBER; MP4: trkn'],
    duration: ['length', 'MP3: TLEN; FLAC: LENGTH; MP4: com.apple.iTunes:LENGTH'],
    remixer: ['remixer', 'MP3: TPE4; FLAC: REMIXER; MP4: com.apple.iTunes:REMIXER'],
    year: ['year', 'MP3: TYER / TDRC; FLAC: DATE; MP4: ©day'],
    month: ['month', 'MP3: TYER / TDRC; FLAC: DATE; MP4: ©day'],
    day: ['day', 'MP3: TYER / TDRC; FLAC: DATE; MP4: ©day'],
    tracktotal: ['trackTotal', 'MP3: TRCK; FLAC: TRACKNUMBER; MP4: trkn'],
    total: ['trackTotal', 'MP3: TRCK; FLAC: TRACKNUMBER; MP4: trkn'],
    disk: ['discNumber', 'MP3: TPOS; FLAC: DISCNUMBER; MP4: disk'],
    disknumber: ['discNumber', 'MP3: TPOS; FLAC: DISCNUMBER; MP4: disk'],
    disc: ['discNumber', 'MP3: TPOS; FLAC: DISCNUMBER; MP4: disk'],
    discnumber: ['discNumber', 'MP3: TPOS; FLAC: DISCNUMBER; MP4: disk'],
};
const properties: Record<string, string> = { first: 'renamerFirstDoc', last: 'renamerLastDoc' };
const functions: Record<string, string> = {
    lower: 'renamerLowerDoc', lowercase: 'renamerLowerDoc', upper: 'renamerUpperDoc', uppercase: 'renamerUpperDoc',
    slice: 'renamerSliceDoc', range: 'renamerSliceDoc', capitalize: 'renamerCapitalizeDoc', titlecase: 'renamerTitlecaseDoc',
    replace: 'renamerReplaceDoc', pad: 'renamerPadDoc', sort: 'renamerSortDoc', reverse: 'renamerReverseDoc',
    join: 'renamerJoinDoc', parent: 'renamerParentDoc', filename: 'renamerPathFilenameDoc',
    camelot: 'renamerCamelotDoc', uncamelot: 'renamerUncamelotDoc',
};

/** Render documentation at the presentation boundary; Rust syntax stays stable. */
export function renamerDocumentation(symbol: SymbolDocumentation): string {
    if (symbol.kind === 'variable') {
        const special: Record<string, string> = {
            filename: 'renamerFilenameDoc', path: 'renamerPathDoc', abspath: 'renamerAbsolutePathDoc',
        };
        if (Object.hasOwn(special, symbol.name)) return translate('tools.' + special[symbol.name]);
        if (Object.hasOwn(variables, symbol.name)) {
            const [label, frames, first] = variables[symbol.name];
            return translate(first ? 'tools.renamerFirstTagDoc' : 'tools.renamerTagDoc', {
                field: translate('tools.' + label), frames,
            });
        }
    }
    const table: Record<string, string> = symbol.kind === 'property' ? properties : symbol.kind === 'function' ? functions : {};
    if (Object.hasOwn(table, symbol.name)) return translate('tools.' + table[symbol.name]);
    // Unknown/new backend symbols remain readable in their source language.
    // This is plain text for Vue interpolation, never HTML sanitization/injection.
    return String(symbol.doc ?? '').replace(/<br\s*\/?>|<\/(?:p|div)>/gi, '\n')
        .replace(/<[^>]*>/g, '').replace(/&nbsp;/g, ' ').replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;|&apos;/g, "'")
        .replace(/&amp;/g, '&').trim();
}
