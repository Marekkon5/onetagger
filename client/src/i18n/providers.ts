import { translate } from './index';

// Presentation-only keys for built-in descriptors. Provider and option IDs,
// authentication values, callback IDs and select-option values stay untouched.
// Maps deliberately avoid interpreting arbitrary plugin strings as message keys.
const descriptions = new Map<string, string>([
    ['bandcamp', 'providers.bandcampDescription'],
    ['beatport', 'providers.beatportDescription'],
    ['beatsource', 'providers.beatsourceDescription'],
    ['bpmsupreme', 'providers.bpmsupremeDescription'],
    ['deezer', 'providers.deezerDescription'],
    ['discogs', 'providers.discogsDescription'],
    ['itunes', 'providers.itunesDescription'],
    ['junodownload', 'providers.junodownloadDescription'],
    ['musicbrainz', 'providers.musicbrainzDescription'],
    ['musixmatch', 'providers.musixmatchDescription'],
    ['spotify', 'providers.spotifyDescription'],
    ['traxsource', 'providers.traxsourceDescription'],
]);

const options = new Map<string, string>([
    ['bandcamp/match_artist/label', 'providers.bandcampMatchArtistLabel'],
    ['beatport/art_resolution/label', 'providers.beatportArtResolutionLabel'],
    ['beatport/max_pages/label', 'providers.beatportMaxPagesLabel'],
    ['beatport/max_pages/tooltip', 'providers.beatportMaxPagesTooltip'],
    ['beatport/ignore_version/label', 'providers.beatportIgnoreVersionLabel'],
    ['beatport/ignore_version/tooltip', 'providers.beatportIgnoreVersionTooltip'],
    ['beatsource/art_resolution/label', 'providers.beatsourceArtResolutionLabel'],
    ['beatsource/art_resolution/tooltip', 'providers.beatsourceArtResolutionTooltip'],
    ['bpmsupreme/email/label', 'providers.bpmsupremeEmailLabel'],
    ['bpmsupreme/password/label', 'providers.bpmsupremePasswordLabel'],
    ['bpmsupreme/library/label', 'providers.bpmsupremeLibraryLabel'],
    ['deezer/art_resolution/label', 'providers.deezerArtResolutionLabel'],
    ['deezer/content_language/label', 'providers.deezerContentLanguageLabel'],
    ['deezer/content_language/tooltip', 'providers.deezerContentLanguageTooltip'],
    ['discogs/token/label', 'providers.discogsTokenLabel'],
    ['discogs/token/tooltip', 'providers.discogsTokenTooltip'],
    ['discogs/max_albums/label', 'providers.discogsMaxAlbumsLabel'],
    ['discogs/max_albums/tooltip', 'providers.discogsMaxAlbumsTooltip'],
    ['discogs/track_number_int/label', 'providers.discogsTrackNumberIntLabel'],
    ['itunes/art_resolution/label', 'providers.itunesArtResolutionLabel'],
]);

/** Undefined keeps an unknown or custom provider's original presentation. */
export function providerDescription(builtIn: boolean, providerId: string): string | undefined {
    if (builtIn !== true) return undefined;
    const key = descriptions.get(providerId);
    return key ? translate(key) : undefined;
}

/** Catalog messages are plain text, never translated HTML. */
export function providerOptionText(builtIn: boolean, providerId: string, optionId: string,
    field: 'label' | 'tooltip'): string | undefined {
    if (builtIn !== true) return undefined;
    const key = options.get(`${providerId}/${optionId}/${field}`);
    return key ? translate(key) : undefined;
}
