import { ID3Writer } from 'browser-id3-writer';
import { parseBlob } from 'music-metadata';
import type { EntityRef } from './domain/types';
import type { ApiEnrichmentResult } from './services/api';

export const MAX_LOCAL_MP3_BYTES = 300 * 1024 * 1024;

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