import { ID3Writer } from 'browser-id3-writer';
import { parseBlob } from 'music-metadata';
import type { EntityRef, ProviderId } from './domain/types';
import type { ApiEnrichmentResult } from './services/api';

export const MAX_LOCAL_MP3_BYTES = 300 * 1024 * 1024;

/**
 * A filename only auto-accepts — and therefore auto-enriches and auto-writes — when
 * its derived evidence reaches this rate. Below it the track waits for a curator.
 */
export const AUTO_ACCEPT_EVIDENCE = 0.8;

export interface FilenameIdentity {
  title: string;
  artists: string[];
  year?: number;
  evidence: number;
  reasons: string[];
}

export interface LocalMp3Cover {
  data: ArrayBufferLike;
  mimeType: string;
  description: string;
}

export interface LocalMp3Track {
  id: string;
  file: File;
  relativePath: string;
  title: string;
  artists: string[];
  artistInput: string;
  album: string;
  albumArtist: string;
  year?: number;
  trackNumber?: number;
  discNumber?: number;
  durationMs?: number;
  genres: string[];
  comment: string;
  composer: string[];
  label: string[];
  bpm?: number;
  key: string;
  lyrics: string;
  mood: string;
  compilation: boolean;
  entities: Array<Pick<EntityRef, 'kind' | 'name' | 'externalId' | 'url'>>;
  cover?: LocalMp3Cover;
  coverUrl?: string;
  sourceUrl?: string;
  enrichedBy: string[];
  state: 'scanned' | 'enriched' | 'skipped' | 'error' | 'written';
  detail: string;
  /** Filename-derived evidence rate (0–1); set by `applyFilenameIdentity`. */
  evidence?: number;
  /** True once evidence reached `AUTO_ACCEPT_EVIDENCE`, so auto-enrich/write may proceed. */
  accepted?: boolean;
  /** Library-sync outcome for this track. */
  syncState?: 'idle' | 'synced' | 'error' | 'skipped';
}

export function normalizeTagText(value: unknown): string {
  return typeof value === 'string'
    ? value.normalize('NFC').replace(/\u0000/g, '').replace(/\s+/g, ' ').trim()
    : '';
}

function normalizeTagList(values: unknown[] | undefined): string[] {
  const unique = new Map<string, string>();
  for (const value of values || []) {
    const normalized = normalizeTagText(value);
    if (normalized) unique.set(normalized.toLocaleLowerCase(), normalized);
  }
  return [...unique.values()];
}

function copyArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

function filenameTitle(file: File): string {
  return normalizeTagText(file.name.replace(/\.mp3$/i, '').replace(/[._]+/g, ' '));
}

export async function readLocalMp3(file: File, relativePath: string): Promise<LocalMp3Track> {
  const fallbackTitle = filenameTitle(file);
  const track: LocalMp3Track = {
    id: `${relativePath}:${file.size}:${file.lastModified}`,
    file,
    relativePath,
    title: fallbackTitle,
    artists: [],
    artistInput: '',
    album: '',
    albumArtist: '',
    genres: [],
    comment: '',
    composer: [],
    label: [],
    key: '',
    lyrics: '',
    mood: '',
    compilation: false,
    entities: [],
    enrichedBy: [],
    state: 'scanned',
    detail: 'Tag scan complete',
  };

  if (file.size > MAX_LOCAL_MP3_BYTES) {
    track.state = 'error';
    track.detail = 'File exceeds the 300 MiB browser-processing limit';
    return track;
  }

  try {
    const metadata = await parseBlob(file, { skipCovers: false });
    const common = metadata.common;
    const artists = normalizeTagList(common.artists?.length ? common.artists : [common.artist]);
    const cover = common.picture?.find((picture) => /^image\/(jpeg|png|webp)$/i.test(picture.format));
    const year = common.year || Number(/^\d{4}/.exec(common.date || '')?.[0]) || undefined;
    const comments = (common.comment || []).map((comment) => normalizeTagText(comment.text)).filter(Boolean);
    const lyrics = normalizeTagText(common.lyrics?.map((lyric) => lyric.text).filter(Boolean).join('\n'));
    Object.assign(track, {
      title: normalizeTagText(common.title) || fallbackTitle,
      artists,
      artistInput: artists.join('; '),
      album: normalizeTagText(common.album),
      albumArtist: normalizeTagText(common.albumartist || common.albumartists?.join('; ')),
      year,
      trackNumber: common.track.no || undefined,
      discNumber: common.disk.no || undefined,
      durationMs: Number.isFinite(metadata.format.duration) ? Math.round((metadata.format.duration || 0) * 1000) : undefined,
      genres: normalizeTagList(common.genre),
      comment: comments.join('\n') || normalizeTagText(common.longDescription),
      composer: normalizeTagList(common.composer),
      label: normalizeTagList(common.label?.length ? common.label : common.publisher),
      bpm: Number.isFinite(common.bpm) ? common.bpm : undefined,
      key: normalizeTagText(common.key),
      lyrics,
      mood: normalizeTagText(common.mood),
      compilation: common.compilation === true,
      cover: cover ? {
        data: copyArrayBuffer(cover.data),
        mimeType: cover.format.toLowerCase(),
        description: normalizeTagText(cover.description) || 'Front cover',
      } : undefined,
    });
  } catch (error) {
    track.state = 'error';
    track.detail = error instanceof Error ? `Tag scan failed: ${error.message}` : 'Tag scan failed';
  }
  return track;
}

export function hasSolidMetadataBase(track: Pick<LocalMp3Track, 'title' | 'artists'>): boolean {
  const title = normalizeTagText(track.title);
  const artist = track.artists.some((value) => normalizeTagText(value).length > 1);
  const placeholder = /^(unknown|untitled|audio(?: file)?|dj set|mix|track\s*#?\s*\d*)$/i;
  return title.length > 2 && !placeholder.test(title) && artist;
}

export function applyEnrichment(track: LocalMp3Track, result: ApiEnrichmentResult): number {
  const patch = result.patch;
  let added = 0;
  if (!track.durationMs && patch.durationMs) { track.durationMs = patch.durationMs; added += 1; }
  if (!track.year && patch.recordedAt) {
    const year = Number(/^\d{4}/.exec(patch.recordedAt)?.[0]);
    if (year) { track.year = year; added += 1; }
  }
  if (!track.comment && patch.description) { track.comment = normalizeTagText(patch.description); added += 1; }
  if (!track.genres.length && patch.genres?.length) { track.genres = normalizeTagList(patch.genres); added += 1; }
  if (!track.cover && !track.coverUrl) {
    const cover = patch.artwork?.find((artwork) => artwork.kind === 'cover');
    if (cover) { track.coverUrl = cover.url; added += 1; }
  }
  if (!track.sourceUrl) {
    track.sourceUrl = patch.sources?.find((source) => source.provider !== 'discogs')?.url;
    if (track.sourceUrl) added += 1;
  }
  const knownEntities = new Set(track.entities.map((entity) => `${entity.kind}:${entity.externalId || entity.url || entity.name}`));
  const addedEntities = result.entities.filter((entity) => !knownEntities.has(`${entity.kind}:${entity.externalId || entity.url || entity.name}`));
  if (addedEntities.length) { track.entities = [...track.entities, ...addedEntities]; added += 1; }
  track.enrichedBy = [...new Set([
    ...result.provenance.filter((item) => item.field && item.confidence >= 0.7).map((item) => item.provider),
    ...result.entities.map((entity) => entity.provider).filter((provider): provider is NonNullable<typeof provider> => Boolean(provider)),
  ])];
  if (added) {
    track.state = 'enriched';
    track.detail = `${added} metadata field(s) added${track.enrichedBy.length ? ` by ${track.enrichedBy.join(', ')}` : ''}`;
  } else if (!result.failures.length) {
    track.detail = 'No confident missing metadata found';
  }
  return added;
}

export function buildTaggedMp3(fileBuffer: ArrayBuffer, track: LocalMp3Track, cover?: ArrayBufferLike): Blob {
  const writer = new ID3Writer(fileBuffer);
  const title = normalizeTagText(track.title);
  const artists = normalizeTagList(track.artists);
  if (title) writer.setFrame('TIT2', title);
  if (artists.length) writer.setFrame('TPE1', artists);
  if (track.albumArtist) writer.setFrame('TPE2', normalizeTagText(track.albumArtist));
  if (track.album) writer.setFrame('TALB', normalizeTagText(track.album));
  if (track.year) writer.setFrame('TYER', track.year);
  if (track.trackNumber) writer.setFrame('TRCK', String(track.trackNumber));
  if (track.discNumber) writer.setFrame('TPOS', String(track.discNumber));
  if (track.genres.length) writer.setFrame('TCON', normalizeTagList(track.genres));
  if (track.durationMs) writer.setFrame('TLEN', Math.round(track.durationMs));
  if (track.bpm) writer.setFrame('TBPM', Math.round(track.bpm));
  if (track.key) writer.setFrame('TKEY', normalizeTagText(track.key));
  if (track.composer.length) writer.setFrame('TCOM', normalizeTagList(track.composer));
  if (track.label.length) writer.setFrame('TPUB', normalizeTagList(track.label).join('; '));
  writer.setFrame('TCMP', track.compilation ? '1' : '0');
  if (track.comment) writer.setFrame('COMM', { description: '', text: normalizeTagText(track.comment), language: 'eng' });
  if (track.lyrics) writer.setFrame('USLT', { description: '', lyrics: track.lyrics, language: 'eng' });
  if (track.sourceUrl && /^https:\/\//i.test(track.sourceUrl)) {
    writer.setFrame('WXXX', { description: 'Provider source', value: track.sourceUrl });
  }
  const coverData = cover || track.cover?.data;
  if (coverData) writer.setFrame('APIC', { type: 3, data: coverData, description: 'Front cover' });
  const customTags = [
    track.mood ? `Mood: ${normalizeTagText(track.mood)}` : '',
    track.enrichedBy.length ? `Enriched by: ${track.enrichedBy.join(', ')}` : '',
    ...track.entities.map((entity) => `${entity.kind}: ${entity.name}${entity.externalId ? ` (Discogs ID ${entity.externalId})` : ''}${entity.url ? ` <${entity.url}>` : ''}`),
  ].filter(Boolean);
  if (customTags.length) writer.setFrame('TXXX', { description: 'SYCO23', value: customTags.join(' | ') });
  writer.addTag();
  return writer.getBlob();
}

const FILENAME_YEAR = /\b(19[5-9]\d|20[0-4]\d)\b/;
const FILENAME_PLACEHOLDER = /^(unknown|untitled|audio(?: file)?|track\s*\d*|mix|mixset|dj\s*set|set\s*\d*|new file|recording|file\s*\d*|id\s*\d*)$/i;

/**
 * Derives artist/title/year identity from a file path and scores how much the filename
 * actually establishes. A well-formed `Artist - Title (Year)` filename reaches 1.0,
 * `Artist - Title` reaches 0.8 (the auto-accept rate), and noisy names stay well below.
 * A filename is a suggestion of identity, so it is only ever auto-accepted above the
 * threshold; nothing here invents a recording date or performer beyond the name.
 */
export function parseFilenameIdentity(relativePath: string): FilenameIdentity {
  const segments = relativePath.split('/').filter(Boolean);
  const raw = (segments.pop() || '').replace(/\.mp3$/i, '');
  const reasons: string[] = [];
  let evidence = 0;
  let working = normalizeTagText(raw.replace(/[_]+/g, ' ').replace(/[[\]{}()]+/g, ' '));
  let year: number | undefined;
  const yearMatch = working.match(FILENAME_YEAR);
  if (yearMatch) {
    year = Number(yearMatch[1]);
    working = normalizeTagText(working.replace(yearMatch[0], ' '));
  }
  let artists: string[] = [];
  let title = working;
  const parts = working.split(/\s+[–—-]\s+/).map((part) => part.trim()).filter(Boolean);
  if (parts.length >= 3 && /^\d{4}$/.test(parts[0])) {
    year = year ?? Number(parts[0]);
    artists = [parts[1]];
    title = parts.slice(2).join(' - ');
  } else if (parts.length >= 2) {
    artists = [parts[0]];
    title = parts.slice(1).join(' - ');
  }
  const cleanArtists = artists.filter((artist) => artist.length >= 2 && !/^\d+$/.test(artist));
  if (cleanArtists.length && title) {
    evidence += 0.5;
    reasons.push('artist and title are separated in the filename');
  }
  if (year) {
    evidence += 0.2;
    reasons.push('a four-digit year is present');
  }
  if (title.length >= 3 && !FILENAME_PLACEHOLDER.test(title)) {
    evidence += 0.2;
    reasons.push('the title is descriptive');
  }
  if (cleanArtists.length) {
    evidence += 0.1;
    reasons.push('an artist name is present');
  }
  return {
    title: normalizeTagText(title) || fallbackFilename(raw) || 'Untitled',
    artists: cleanArtists,
    year,
    evidence: Math.min(1, Number(evidence.toFixed(2))),
    reasons,
  };
}

function fallbackFilename(value: string): string {
  return normalizeTagText(value.replace(/[._]+/g, ' '));
}

/**
 * Fills a track's missing identity from its filename and records the evidence rate.
 * It never overwrites a curator's explicit title/artist choice: the filename title is
 * only applied when the current title is empty or still the raw filename fallback.
 */
export function applyFilenameIdentity(track: LocalMp3Track): FilenameIdentity {
  const identity = parseFilenameIdentity(track.relativePath);
  const filename = track.relativePath.split('/').pop() || '';
  const fallback = fallbackFilename(filename.replace(/\.mp3$/i, ''));
  const titleIsFallback = normalizeTagText(track.title) === fallback;
  if ((!normalizeTagText(track.title) || titleIsFallback) && identity.title) track.title = identity.title;
  if (!track.artists.length && identity.artists.length) {
    track.artists = [...identity.artists];
    track.artistInput = identity.artists.join('; ');
  }
  if (!track.year && identity.year) track.year = identity.year;
  track.evidence = identity.evidence;
  track.accepted = identity.evidence >= AUTO_ACCEPT_EVIDENCE;
  track.detail = track.accepted
    ? `Auto-tag accepted from filename (${Math.round(identity.evidence * 100)}% evidence)`
    : `Filename evidence ${Math.round(identity.evidence * 100)}% is below the ${Math.round(AUTO_ACCEPT_EVIDENCE * 100)}% auto-tag threshold`;
  return identity;
}

/**
 * Maps a public source URL onto a provider identity so a synced local record keeps an
 * external `(provider, resourceType, externalId)`. A local file with no provider source
 * cannot be synced, because a private file path is not a shared provider identity.
 */
export function providerFromSourceUrl(sourceUrl: string | undefined): ProviderId | undefined {
  if (!sourceUrl) return undefined;
  try {
    const host = new URL(sourceUrl).hostname.toLowerCase();
    if (/(^|\.)archive\.org$/.test(host)) return 'archiveorg';
    if (/(^|\.)youtube\.com$/.test(host) || host === 'youtu.be') return 'youtube';
    if (/(^|\.)mixcloud\.com$/.test(host)) return 'mixcloud';
    if (/(^|\.)soundcloud\.com$/.test(host)) return 'soundcloud';
    if (/(^|\.)hearthis\.at$/.test(host)) return 'hearthis';
    if (/(^|\.)freeteknomusic\.org$/.test(host)) return 'freeteknomusic';
  } catch {
    return undefined;
  }
  return undefined;
}

export function resourceTypeForProvider(provider: ProviderId): string {
  if (provider === 'youtube') return 'video';
  if (provider === 'mixcloud') return 'cloudcast';
  if (provider === 'soundcloud' || provider === 'hearthis') return 'track';
  if (provider === 'archiveorg') return 'item';
  return 'recording';
}