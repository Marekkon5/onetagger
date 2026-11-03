import {
  ImportMetadataFactsSchema,
  type ImportMetadataFacts,
} from '@syco23/catalog-domain';
import type { ProviderCredentials } from './config.js';
import { boundedFetch } from './security.js';

/**
 * Resolver layer: each provider goes through its own adapter. Metadata-capable
 * providers return a complete, schema-validated `ImportMetadataFacts` snapshot
 * so finalize never has to invent a title or read from a URL.
 *
 * Audio is only returned through provider-authorised mechanisms:
 * - SoundCloud: the official API `download_url` of a track the uploader marked
 *   downloadable. A non-downloadable track fails honestly; the worker never
 *   extracts protected streams.
 * - hearthis.at: the track's `download_url`, or its official `stream_url`
 *   listen redirect that resolves to the provider's own MP3.
 * - Mixcloud: the show's download route, only when Mixcloud reports the show as
 *   downloadable. The obfuscated stream URL is intentionally not decoded.
 *
 * Discogs and MusicBrainz supply release/recording metadata only.
 */
export interface ResolvedSource {
  facts?: ImportMetadataFacts;
  /** Direct, downloadable media URL — only when permitted for the provider. */
  audioUrl?: string;
  /** Credentials that must accompany the audio fetch (for example SoundCloud OAuth). */
  audioHeaders?: Record<string, string>;
  /** Private object key for a user-upload original. Never a filesystem path or fetch URL. */
  objectKey?: string;
}

export class NonRetryableError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NonRetryableError';
  }
}

export function isUploadUrn(sourceUrl: string): boolean {
  return /^urn:syco23:upload:.+$/.test(sourceUrl);
}

const AUDIO_EXT = /\.(mp3|flac|wav|ogg|m4a|aiff?)$/i;
const MAX_JSON_BYTES = 512 * 1024;
const DEFAULT_USER_AGENT = 'syco23-mixsets/1.0 (https://mixsets.syco23.org)';
const MBID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function observedAt(): string {
  return new Date().toISOString();
}

function asHttpUrl(value: unknown): string | undefined {
  if (typeof value !== 'string' || !value) return undefined;
  try {
    const url = new URL(value);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.toString() : undefined;
  } catch {
    return undefined;
  }
}

function text(value: unknown, max = 500): string | undefined {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed.slice(0, max) : undefined;
}

function list(value: unknown): string[] {
  if (Array.isArray(value)) return value.map((entry) => String(entry).trim()).filter(Boolean);
  if (value === undefined || value === null || value === '') return [];
  return [String(value).trim()].filter(Boolean);
}

function userAgent(credentials: ProviderCredentials): string {
  return text(credentials.musicbrainzUserAgent, 300) ?? DEFAULT_USER_AGENT;
}

async function getJson<T>(
  url: string,
  maxBytes = MAX_JSON_BYTES,
  headers: Record<string, string> = {},
): Promise<T> {
  const { bytes } = await boundedFetch(url, { maxBytes, headers: { accept: 'application/json', ...headers } });
  try {
    return JSON.parse(bytes.toString('utf8')) as T;
  } catch {
    throw new NonRetryableError('Provider returned unreadable JSON metadata');
  }
}

export function youtubeVideoId(value: string): string | undefined {
  try {
    const url = new URL(value);
    if (!['https:', 'http:'].includes(url.protocol) || url.username || url.password || url.port) return;
    const host = url.hostname.toLowerCase();
    const parts = url.pathname.split('/').filter(Boolean);
    const id = host === 'youtu.be' ? parts[0]
      : ['youtube.com', 'www.youtube.com', 'm.youtube.com', 'music.youtube.com'].includes(host)
        ? url.pathname === '/watch' ? url.searchParams.get('v') || undefined
          : ['shorts', 'live', 'embed'].includes(parts[0]) ? parts[1] : undefined
        : undefined;
    return id && /^[A-Za-z0-9_-]{11}$/.test(id) ? id : undefined;
  } catch {
    return;
  }
}

export function hearthisPath(sourceUrl: string): string | undefined {
  try {
    const url = new URL(sourceUrl);
    if (!['https:', 'http:'].includes(url.protocol) || !['hearthis.at', 'www.hearthis.at'].includes(url.hostname)
      || url.username || url.password || url.port) return;
    const parts = url.pathname.split('/').filter(Boolean);
    return parts.length >= 2 && parts.every((part) => !['.', '..'].includes(part)) ? `${parts.join('/')}/` : undefined;
  } catch {
    return;
  }
}

export function mixcloudKey(sourceUrl: string): string | undefined {
  try {
    const url = new URL(sourceUrl);
    if (!['https:', 'http:'].includes(url.protocol) || !['mixcloud.com', 'www.mixcloud.com'].includes(url.hostname)
      || url.username || url.password || url.port) return;
    const parts = url.pathname.split('/').filter(Boolean);
    return parts.length >= 2 && parts.every((part) => !['.', '..'].includes(part)) ? `/${parts.join('/')}/` : undefined;
  } catch {
    return;
  }
}

export function archiveOrgIdentifier(sourceUrl: string): string | undefined {
  try {
    const url = new URL(sourceUrl);
    if (!['https:', 'http:'].includes(url.protocol)
      || !['archive.org', 'www.archive.org'].includes(url.hostname)
      || url.username || url.password || url.port) return;
    return /^\/(?:details|download)\/([A-Za-z0-9._-]+)/.exec(url.pathname)?.[1];
  } catch {
    return;
  }
}

/** Discogs release/master links are metadata import targets. */
export function discogsResource(sourceUrl: string): { kind: 'release' | 'master'; id: string } | undefined {
  try {
    const url = new URL(sourceUrl);
    if (!['https:', 'http:'].includes(url.protocol) || !['discogs.com', 'www.discogs.com'].includes(url.hostname)
      || url.username || url.password || url.port) return;
    const match = /^\/(release|master)\/(\d+)(?:-|$)/.exec(url.pathname);
    return match ? { kind: match[1] as 'release' | 'master', id: match[2] } : undefined;
  } catch {
    return;
  }
}

/** MusicBrainz recording, release and release-group pages are metadata targets. */
export function musicbrainzResource(sourceUrl: string): { kind: 'recording' | 'release' | 'release-group'; mbid: string } | undefined {
  try {
    const url = new URL(sourceUrl);
    if (!['https:', 'http:'].includes(url.protocol) || !['musicbrainz.org', 'www.musicbrainz.org'].includes(url.hostname)
      || url.username || url.password || url.port) return;
    const match = /^\/(recording|release|release-group)\/([0-9a-f-]+)\/?$/i.exec(url.pathname);
    if (!match || !MBID.test(match[2])) return;
    return { kind: match[1].toLowerCase() as 'recording' | 'release' | 'release-group', mbid: match[2].toLowerCase() };
  } catch {
    return;
  }
}

interface SoundcloudOembed { title?: string; author_name?: string; thumbnail_url?: string }
interface SoundcloudTrack {
  kind?: string;
  id?: number | string;
  title?: string;
  description?: string;
  duration?: number;
  genre?: string;
  tag_list?: string;
  created_at?: string;
  permalink_url?: string;
  artwork_url?: string;
  downloadable?: boolean;
  download_url?: string;
  user?: { username?: string };
}
interface YoutubeOembed { title?: string; author_name?: string; thumbnail_url?: string }
interface HearthisTrack {
  id?: string;
  title?: string;
  duration?: string | number;
  description?: string;
  genre?: string;
  tags?: string;
  permalink_url?: string;
  artwork_url?: string;
  download_url?: string;
  stream_url?: string;
  user?: { username?: string };
}
interface MixcloudCloudcast {
  key?: string;
  name?: string;
  url?: string;
  slug?: string;
  description?: string;
  audio_length?: number;
  created_time?: string;
  tags?: Array<{ name?: string }>;
  pictures?: { large?: string; medium?: string; small?: string };
  user?: { name?: string; username?: string };
}
interface DiscogsArtist { name?: string }
interface DiscogsImage { uri?: string; type?: string }
interface DiscogsRelease {
  id?: number;
  title?: string;
  uri?: string;
  notes?: string;
  genres?: string[];
  styles?: string[];
  images?: DiscogsImage[];
  artists?: DiscogsArtist[];
}
interface MusicbrainzArtistCredit { name?: string }
interface MusicbrainzEntity {
  id?: string;
  title?: string;
  name?: string;
  disambiguation?: string;
  length?: number;
  tags?: Array<{ name?: string }>;
  'artist-credit'?: MusicbrainzArtistCredit[];
}
interface ArchiveOrgFile { name?: string; length?: string | number; source?: string; format?: string }
interface ArchiveOrgDetail {
  metadata?: Record<string, unknown>;
  files?: ArchiveOrgFile[];
}

function requireTitle(title: string | undefined, provider: string): string {
  if (!title) throw new NonRetryableError(`${provider} returned no title metadata`);
  return title;
}

function soundcloudFacts(track: SoundcloudTrack, sourceUrl: string): ImportMetadataFacts {
  const title = requireTitle(text(track.title), 'SoundCloud');
  const durationMs = Number(track.duration) > 0 ? Math.round(Number(track.duration)) : undefined;
  const tags = (track.tag_list ?? '').split(/\s+/).map((tag) => tag.trim()).filter(Boolean);
  const genres = [track.genre, ...tags].filter((value): value is string => Boolean(value));
  const artwork = asHttpUrl(track.artwork_url);
  return ImportMetadataFactsSchema.parse({
    provider: 'soundcloud',
    sourceUrl: asHttpUrl(track.permalink_url) ?? sourceUrl,
    title,
    ...(text(track.user?.username) ? { uploader: text(track.user?.username) } : {}),
    ...(durationMs ? { durationMs } : {}),
    ...(text(track.description, 10000) ? { description: text(track.description, 10000) } : {}),
    ...(track.created_at ? { uploadedAt: track.created_at } : {}),
    ...(genres.length ? { genres } : {}),
    ...(artwork && new URL(artwork).hostname.endsWith('.sndcdn.com') ? { artworkUrl: artwork } : {}),
    ...(track.id !== undefined ? { externalId: String(track.id) } : {}),
    observedAt: observedAt(),
  });
}

let soundcloudTokenCache: { key: string; value: string; expiresAt: number } | undefined;

async function soundcloudToken(credentials: ProviderCredentials): Promise<string | undefined> {
  if (credentials.soundcloudAccessToken) return credentials.soundcloudAccessToken;
  const clientId = credentials.soundcloudClientId;
  const clientSecret = credentials.soundcloudClientSecret;
  if (!clientId || !clientSecret) return undefined;
  const key = `${clientId}:${clientSecret}`;
  if (soundcloudTokenCache && soundcloudTokenCache.key === key && soundcloudTokenCache.expiresAt > Date.now()) {
    return soundcloudTokenCache.value;
  }
  const { bytes } = await boundedFetch('https://secure.soundcloud.com/oauth/token', {
    maxBytes: 64 * 1024,
    timeoutMs: 12_000,
    method: 'POST',
    headers: {
      accept: 'application/json',
      'content-type': 'application/x-www-form-urlencoded',
      authorization: `Basic ${Buffer.from(`${clientId}:${clientSecret}`).toString('base64')}`,
    },
    body: 'grant_type=client_credentials',
  });
  const payload = JSON.parse(bytes.toString('utf8')) as { access_token?: string; expires_in?: number };
  if (!payload.access_token) throw new NonRetryableError('SoundCloud token endpoint returned no access token');
  const ttl = Math.max(60, Number(payload.expires_in) || 3600) * 1000;
  soundcloudTokenCache = { key, value: payload.access_token, expiresAt: Date.now() + Math.max(60_000, ttl - 60_000) };
  return payload.access_token;
}

async function resolveSoundcloud(
  job: { sourceUrl: string; mode: string },
  credentials: ProviderCredentials,
): Promise<ResolvedSource> {
  const token = await soundcloudToken(credentials);
  if (!token) {
    if (job.mode === 'audio') {
      throw new NonRetryableError('SoundCloud audio import requires SOUNDCLOUD_CLIENT_ID/SECRET or SOUNDCLOUD_ACCESS_TOKEN in the worker environment');
    }
    const oembedUrl = new URL('https://soundcloud.com/oembed');
    oembedUrl.search = new URLSearchParams({ format: 'json', url: job.sourceUrl }).toString();
    const payload = await getJson<SoundcloudOembed>(oembedUrl.toString());
    const title = requireTitle(text(payload.title), 'SoundCloud');
    const artwork = asHttpUrl(payload.thumbnail_url);
    return {
      facts: ImportMetadataFactsSchema.parse({
        provider: 'soundcloud',
        sourceUrl: job.sourceUrl,
        title,
        ...(text(payload.author_name) ? { uploader: text(payload.author_name) } : {}),
        ...(artwork && new URL(artwork).hostname.endsWith('.sndcdn.com') ? { artworkUrl: artwork } : {}),
        observedAt: observedAt(),
      }),
    };
  }

  const resolveUrl = new URL('https://api.soundcloud.com/resolve');
  resolveUrl.search = new URLSearchParams({ url: job.sourceUrl }).toString();
  const track = await getJson<SoundcloudTrack>(resolveUrl.toString(), MAX_JSON_BYTES, {
    authorization: `OAuth ${token}`,
  });
  if (track.kind && track.kind !== 'track') {
    throw new NonRetryableError('SoundCloud import requires a track URL, not a playlist or user profile');
  }
  const facts = soundcloudFacts(track, job.sourceUrl);
  if (job.mode !== 'audio') return { facts };
  const downloadUrl = asHttpUrl(track.download_url);
  if (!track.downloadable || !downloadUrl) {
    throw new NonRetryableError('SoundCloud only permits audio import for tracks the uploader marked downloadable');
  }
  return { facts, audioUrl: downloadUrl, audioHeaders: { authorization: `OAuth ${token}` } };
}

async function resolveYoutube(job: { sourceUrl: string; mode: string }): Promise<ResolvedSource> {
  if (job.mode === 'audio') {
    throw new NonRetryableError('Server-side YouTube audio acquisition is blocked by provider policy');
  }
  const id = youtubeVideoId(job.sourceUrl);
  if (!id) throw new NonRetryableError('YouTube video URL is invalid');
  const oembedUrl = new URL('https://www.youtube.com/oembed');
  oembedUrl.search = new URLSearchParams({ url: `https://www.youtube.com/watch?v=${id}`, format: 'json' }).toString();
  const payload = await getJson<YoutubeOembed>(oembedUrl.toString());
  const title = requireTitle(text(payload.title), 'YouTube');
  const artwork = asHttpUrl(payload.thumbnail_url);
  return {
    facts: ImportMetadataFactsSchema.parse({
      provider: 'youtube',
      sourceUrl: `https://www.youtube.com/watch?v=${id}`,
      title,
      // A channel is the uploader, not a verified performer.
      ...(text(payload.author_name) ? { uploader: text(payload.author_name) } : {}),
      ...(artwork ? { artworkUrl: artwork } : {}),
      externalId: id,
      observedAt: observedAt(),
    }),
  };
}

async function resolveHearthis(job: { sourceUrl: string; mode: string }): Promise<ResolvedSource> {
  const path = hearthisPath(job.sourceUrl);
  if (!path) throw new NonRetryableError('hearthis.at track URL is invalid');
  const track = await getJson<HearthisTrack>(`https://api-v2.hearthis.at/${path}`);
  if (!track.id || !track.permalink_url) throw new NonRetryableError('hearthis.at did not return a public track');
  const title = requireTitle(text(track.title), 'hearthis.at');
  const duration = Number(track.duration);
  const durationMs = Number.isFinite(duration) && duration > 0 ? Math.round(duration * 1000) : undefined;
  const genres = [track.genre, ...(track.tags ?? '').split(',')]
    .map((value) => value?.trim()).filter((value): value is string => Boolean(value));
  const artwork = asHttpUrl(track.artwork_url);
  const facts = ImportMetadataFactsSchema.parse({
    provider: 'hearthis',
    sourceUrl: track.permalink_url,
    title,
    ...(text(track.user?.username) ? { uploader: text(track.user?.username) } : {}),
    ...(durationMs ? { durationMs } : {}),
    ...(text(track.description, 10000) ? { description: text(track.description, 10000) } : {}),
    ...(genres.length ? { genres } : {}),
    ...(artwork && new URL(artwork).hostname === 'img.hearthis.at' ? { artworkUrl: artwork } : {}),
    externalId: track.id,
    observedAt: observedAt(),
  });
  if (job.mode !== 'audio') return { facts };
  // Prefer a real download URL; otherwise use the provider's own listen
  // redirect, which resolves to the same public MP3 stream the site plays.
  const audioUrl = asHttpUrl(track.download_url) ?? asHttpUrl(track.stream_url);
  if (!audioUrl) throw new NonRetryableError('hearthis.at did not expose a public audio stream for this track');
  return { facts, audioUrl };
}

async function mixcloudDownloadUrl(key: string): Promise<string> {
  const parts = key.split('/').filter(Boolean);
  if (parts.length !== 2) throw new NonRetryableError('Mixcloud show URL must contain a user and a show slug');
  const body = JSON.stringify({
    query: 'query($u:String!,$s:String!){cloudcastLookup(lookup:{username:$u,slug:$s}){isDownloadable download}}',
    variables: { u: parts[0], s: parts[1] },
  });
  const { bytes } = await boundedFetch('https://app.mixcloud.com/graphql', {
    maxBytes: 64 * 1024,
    timeoutMs: 15_000,
    method: 'POST',
    headers: { accept: 'application/json', 'content-type': 'application/json' },
    body,
  });
  let payload: { data?: { cloudcastLookup?: { isDownloadable?: boolean; download?: boolean } } };
  try {
    payload = JSON.parse(bytes.toString('utf8')) as typeof payload;
  } catch {
    throw new NonRetryableError('Mixcloud returned unreadable download metadata');
  }
  const lookup = payload.data?.cloudcastLookup;
  if (lookup?.isDownloadable !== true && lookup?.download !== true) {
    throw new NonRetryableError('Mixcloud only permits audio import for shows the uploader made downloadable');
  }
  return `https://www.mixcloud.com/${parts[0]}/${parts[1]}/download/`;
}

async function resolveMixcloud(job: { sourceUrl: string; mode: string }): Promise<ResolvedSource> {
  const key = mixcloudKey(job.sourceUrl);
  if (!key) throw new NonRetryableError('Mixcloud show URL is invalid');
  const cloudcast = await getJson<MixcloudCloudcast>(`https://api.mixcloud.com/${key.replace(/^\//, '')}`);
  if (!cloudcast.url || !cloudcast.name) throw new NonRetryableError('Mixcloud did not return a public cloudcast');
  const durationMs = Number(cloudcast.audio_length) > 0 ? Math.round(Number(cloudcast.audio_length) * 1000) : undefined;
  const genres = (cloudcast.tags ?? []).map((tag) => tag?.name?.trim()).filter((name): name is string => Boolean(name));
  const artwork = asHttpUrl(cloudcast.pictures?.large || cloudcast.pictures?.medium);
  const externalId = cloudcast.key || cloudcast.slug || cloudcast.url;
  const facts = ImportMetadataFactsSchema.parse({
    provider: 'mixcloud',
    sourceUrl: cloudcast.url,
    title: cloudcast.name.trim(),
    ...(text(cloudcast.user?.name || cloudcast.user?.username) ? { uploader: text(cloudcast.user?.name || cloudcast.user?.username) } : {}),
    ...(durationMs ? { durationMs } : {}),
    ...(text(cloudcast.description, 10000) ? { description: text(cloudcast.description, 10000) } : {}),
    ...(cloudcast.created_time ? { uploadedAt: cloudcast.created_time } : {}),
    ...(genres.length ? { genres } : {}),
    ...(artwork ? { artworkUrl: artwork } : {}),
    externalId,
    observedAt: observedAt(),
  });
  if (job.mode !== 'audio') return { facts };
  return { facts, audioUrl: await mixcloudDownloadUrl(key) };
}

async function resolveDiscogs(job: { sourceUrl: string; mode: string }, credentials: ProviderCredentials): Promise<ResolvedSource> {
  if (job.mode === 'audio') {
    throw new NonRetryableError('Discogs supplies release metadata only and never mix audio');
  }
  const resource = discogsResource(job.sourceUrl);
  if (!resource) throw new NonRetryableError('Discogs URL must point to a release or master page');
  const headers: Record<string, string> = { 'user-agent': userAgent(credentials) };
  if (credentials.discogsToken) headers.authorization = `Discogs token=${credentials.discogsToken}`;
  const payload = await getJson<DiscogsRelease>(
    `https://api.discogs.com/${resource.kind === 'release' ? 'releases' : 'masters'}/${resource.id}`,
    MAX_JSON_BYTES,
    headers,
  );
  const title = requireTitle(text(payload.title), 'Discogs');
  const artists = (payload.artists ?? []).map((artist) => text(artist.name)).filter((name): name is string => Boolean(name));
  const genres = [...(payload.genres ?? []), ...(payload.styles ?? [])]
    .map((value) => value?.trim()).filter((value): value is string => Boolean(value));
  const images = Array.isArray(payload.images) ? payload.images : [];
  const primary = images.find((image) => image?.type === 'primary') ?? images[0];
  const artworkUrl = asHttpUrl(primary?.uri);
  const sourceUrl = asHttpUrl(payload.uri) ?? `https://www.discogs.com/${resource.kind}/${resource.id}`;
  return {
    facts: ImportMetadataFactsSchema.parse({
      provider: 'discogs',
      sourceUrl,
      title,
      ...(artists.length ? { artists } : {}),
      ...(text(payload.notes, 10000) ? { description: text(payload.notes, 10000) } : {}),
      ...(genres.length ? { genres } : {}),
      ...(artworkUrl && new URL(artworkUrl).hostname === 'i.discogs.com' ? { artworkUrl } : {}),
      externalId: String(payload.id ?? resource.id),
      observedAt: observedAt(),
    }),
  };
}

async function resolveMusicbrainz(job: { sourceUrl: string; mode: string }, credentials: ProviderCredentials): Promise<ResolvedSource> {
  if (job.mode === 'audio') {
    throw new NonRetryableError('MusicBrainz supplies recording metadata only and never mix audio');
  }
  const resource = musicbrainzResource(job.sourceUrl);
  if (!resource) throw new NonRetryableError('MusicBrainz URL must point to a recording, release or release group');
  const query = new URLSearchParams({ inc: 'artist-credits+tags+releases+release-groups', fmt: 'json' });
  const payload = await getJson<MusicbrainzEntity>(
    `https://musicbrainz.org/ws/2/${resource.kind}/${resource.mbid}?${query}`,
    MAX_JSON_BYTES,
    { 'user-agent': userAgent(credentials) },
  );
  const title = requireTitle(text(payload.title) ?? text(payload.name), 'MusicBrainz');
  const artists = (payload['artist-credit'] ?? [])
    .map((credit) => text(credit.name)).filter((name): name is string => Boolean(name));
  const genres = (payload.tags ?? [])
    .map((tag) => tag?.name?.trim()).filter((name): name is string => Boolean(name));
  const durationMs = Number(payload.length) > 0 ? Math.round(Number(payload.length)) : undefined;
  return {
    facts: ImportMetadataFactsSchema.parse({
      provider: 'musicbrainz',
      sourceUrl: `https://musicbrainz.org/${resource.kind}/${resource.mbid}`,
      title,
      ...(artists.length ? { artists } : {}),
      ...(durationMs ? { durationMs } : {}),
      ...(text(payload.disambiguation, 10000) ? { description: text(payload.disambiguation, 10000) } : {}),
      ...(genres.length ? { genres } : {}),
      externalId: payload.id ?? resource.mbid,
      observedAt: observedAt(),
    }),
  };
}

function isAudioFile(file: ArchiveOrgFile): boolean {
  return Boolean(file.name) && AUDIO_EXT.test(file.name!) && !/thumb|spectrogram/i.test(file.name!);
}

async function resolveArchiveorg(job: { sourceUrl: string; mode: string }): Promise<ResolvedSource> {
  const identifier = archiveOrgIdentifier(job.sourceUrl);
  if (!identifier) throw new NonRetryableError('Archive.org source URL is invalid');
  const detail = await getJson<ArchiveOrgDetail>(
    `https://archive.org/metadata/${encodeURIComponent(identifier)}`,
    2 * 1024 * 1024,
  );
  const metadata = detail.metadata ?? {};
  const files = Array.isArray(detail.files) ? detail.files : [];
  const title = requireTitle(text(metadata.title) ?? identifier, 'Archive.org');
  const artists = list(metadata.creator);
  const description = list(metadata.description).join('\n');
  const genres = [...list(metadata.subject), ...list(metadata.collection)];
  const recordedAt = text(metadata.date, 40)?.slice(0, 10);
  const audio = files.find(isAudioFile);
  const image = files.find((file) => /\.(jpg|jpeg|png)$/i.test(file.name ?? '') && /cover|thumb|itemimage/i.test(`${file.name} ${file.source ?? ''}`));
  const duration = Number(audio?.length);
  const durationMs = Number.isFinite(duration) && duration > 0 ? Math.round(duration * 1000) : undefined;
  const artworkUrl = image?.name
    ? asHttpUrl(`https://archive.org/download/${identifier}/${encodeURIComponent(image.name)}`)
    : undefined;
  if (job.mode === 'audio' && !audio) {
    throw new NonRetryableError('Archive.org item has no downloadable audio file');
  }
  return {
    facts: ImportMetadataFactsSchema.parse({
      provider: 'archiveorg',
      sourceUrl: `https://archive.org/details/${identifier}`,
      title,
      ...(artists.length ? { artists } : {}),
      ...(durationMs ? { durationMs } : {}),
      ...(recordedAt ? { recordedAt } : {}),
      ...(description ? { description } : {}),
      ...(genres.length ? { genres } : {}),
      ...(artworkUrl ? { artworkUrl } : {}),
      externalId: identifier,
      observedAt: observedAt(),
    }),
    ...(job.mode === 'audio' && audio?.name
      ? { audioUrl: `https://archive.org/download/${identifier}/${encodeURIComponent(audio.name)}` }
      : {}),
  };
}

/**
 * Freeteknomusic exposes an HTTP directory index instead of an API. The source
 * URL is either a direct audio file or a directory; identity comes from the
 * filename/folder (parsed evidence), never from a synthetic placeholder.
 */
export function freeteknomusicIdentity(sourceUrl: string): { title: string; artists?: string[] } {
  const url = new URL(sourceUrl);
  const segments = url.pathname.split('/').filter(Boolean);
  const name = decodeURIComponent(segments.at(-1) ?? '');
  const stem = name.replace(AUDIO_EXT, '').replace(/_+/g, ' ').replace(/\s+/g, ' ').trim();
  if (!stem) throw new NonRetryableError('Freeteknomusic source has no usable file name');
  const parts = stem.split(/\s+-\s+/).map((part) => part.trim()).filter(Boolean);
  const parent = decodeURIComponent(segments.at(-2) ?? '').replace(/_+/g, ' ').trim();
  const title = parts.length > 1 ? parts.slice(1).join(' — ') : stem;
  const artists = parts.length > 1 ? [parts[0]] : parent ? [parent] : [];
  return { title, ...(artists.length ? { artists } : {}) };
}

export function firstAudioHref(html: string, baseUrl: string): string | undefined {
  for (const match of html.matchAll(/<a\s+[^>]*href=["']([^"']+)["']/gi)) {
    const href = match[1];
    let url: URL;
    try {
      url = new URL(href, baseUrl);
    } catch {
      continue;
    }
    if (!['https:', 'http:'].includes(url.protocol) || url.origin !== new URL(baseUrl).origin) continue;
    if (AUDIO_EXT.test(url.pathname)) return url.toString();
  }
  return undefined;
}

async function resolveFreeteknomusic(job: { sourceUrl: string; mode: string }): Promise<ResolvedSource> {
  if (isUploadUrn(job.sourceUrl) || job.sourceUrl.startsWith('/') || job.sourceUrl.startsWith('file:')) {
    throw new NonRetryableError('Provider audio must be an http(s) URL');
  }
  const url = new URL(job.sourceUrl);
  if (url.origin !== 'https://archive.freeteknomusic.org') {
    throw new NonRetryableError('Freeteknomusic source must remain on archive.freeteknomusic.org');
  }
  let audioUrl = url.toString();
  if (!AUDIO_EXT.test(url.pathname)) {
    if (!url.pathname.endsWith('/')) url.pathname += '/';
    const { bytes } = await boundedFetch(url.toString(), { maxBytes: 1024 * 1024 });
    const found = firstAudioHref(bytes.toString('utf8'), url.toString());
    if (!found) throw new NonRetryableError('Freeteknomusic directory has no audio file');
    audioUrl = found;
  }
  const identity = freeteknomusicIdentity(audioUrl);
  return {
    facts: ImportMetadataFactsSchema.parse({
      provider: 'freeteknomusic',
      sourceUrl: audioUrl,
      title: identity.title,
      ...(identity.artists ? { artists: identity.artists } : {}),
      externalId: audioUrl,
      observedAt: observedAt(),
    }),
    ...(job.mode === 'audio' ? { audioUrl } : {}),
  };
}

export async function resolveSource(job: {
  provider: string;
  sourceUrl: string;
  mode: string;
  originalObjectKey?: string;
  credentials?: ProviderCredentials;
}): Promise<ResolvedSource> {
  const credentials = job.credentials ?? {};
  if (job.provider === 'user_upload') {
    if (job.mode !== 'audio') {
      throw new NonRetryableError('User uploads are audio-only');
    }
    if (!job.originalObjectKey) {
      throw new NonRetryableError('User upload has no original object');
    }
    if (job.sourceUrl.startsWith('/') || job.sourceUrl.startsWith('file:') || job.sourceUrl.includes('://')) {
      throw new NonRetryableError('User upload source must be a private URN');
    }
    if (!isUploadUrn(job.sourceUrl)) {
      throw new NonRetryableError('User upload source must be a private URN');
    }
    return { objectKey: job.originalObjectKey };
  }
  if (job.provider === 'soundcloud') return resolveSoundcloud(job, credentials);
  if (job.provider === 'youtube') return resolveYoutube(job);
  if (job.provider === 'hearthis') return resolveHearthis(job);
  if (job.provider === 'mixcloud') return resolveMixcloud(job);
  if (job.provider === 'archiveorg') return resolveArchiveorg(job);
  if (job.provider === 'freeteknomusic') return resolveFreeteknomusic(job);
  if (job.provider === 'discogs') return resolveDiscogs(job, credentials);
  if (job.provider === 'musicbrainz') return resolveMusicbrainz(job, credentials);
  throw new NonRetryableError(`No resolver registered for provider ${job.provider}`);
}

/** Providers the worker can resolve; tests assert this covers every metadata capability. */
export const RESOLVER_PROVIDERS = [
  'user_upload',
  'soundcloud',
  'youtube',
  'hearthis',
  'mixcloud',
  'archiveorg',
  'freeteknomusic',
  'discogs',
  'musicbrainz',
] as const;
