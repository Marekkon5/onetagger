import type { DiscoveryProvider, MixCandidate, ProviderHealth, SearchQuery } from '../domain.js';
import { confidenceScore, retry, uniqueCandidates, withTimeout } from '../core/utils.js';

const API = 'https://api.soundcloud.com';
let cachedToken: { value: string; expiresAt: number } | undefined;

async function token(signal?: AbortSignal): Promise<string> {
  const clientId = process.env.SOUNDCLOUD_CLIENT_ID;
  const clientSecret = process.env.SOUNDCLOUD_CLIENT_SECRET;
  if (clientId && clientSecret) {
    if (cachedToken && cachedToken.expiresAt > Date.now()) return cachedToken.value;
    const response = await withTimeout((inner) => fetch('https://secure.soundcloud.com/oauth/token', {
      method: 'POST',
      signal: inner,
      headers: {
        accept: 'application/json',
        'content-type': 'application/x-www-form-urlencoded',
        authorization: `Basic ${Buffer.from(`${clientId}:${clientSecret}`).toString('base64')}`,
      },
      body: 'grant_type=client_credentials',
    }), 12_000, signal);
    if (!response.ok) throw new Error(`SoundCloud token ${response.status} ${response.statusText || 'request rejected'}`);
    const payload = await response.json() as { access_token?: string; expires_in?: number };
    if (!payload.access_token) throw new Error('SoundCloud token response has no access token');
    cachedToken = {
      value: payload.access_token,
      expiresAt: Date.now() + Math.max(60, (payload.expires_in || 3600) - 120) * 1000,
    };
    return cachedToken.value;
  }
  if (process.env.SOUNDCLOUD_ACCESS_TOKEN) return process.env.SOUNDCLOUD_ACCESS_TOKEN;
  throw new Error('SoundCloud app credentials are not configured');
}

async function api<T>(path: string, params: URLSearchParams, signal?: AbortSignal): Promise<T> {
  const accessToken = await token(signal);
  return retry(() => withTimeout(async (inner) => {
    const url = new URL(path, API);
    url.search = params.toString();
    const response = await fetch(url, {
      signal: inner,
      headers: { accept: 'application/json', authorization: `OAuth ${accessToken}` },
    });
    if (!response.ok) throw new Error(`SoundCloud ${response.status} ${response.statusText || 'request rejected'}`);
    return response.json() as Promise<T>;
  }, 12_000, signal));
}

export class SoundCloudProvider implements DiscoveryProvider {
  id = 'soundcloud' as const;

  async health(signal?: AbortSignal): Promise<ProviderHealth> {
    if (!process.env.SOUNDCLOUD_ACCESS_TOKEN && !(process.env.SOUNDCLOUD_CLIENT_ID && process.env.SOUNDCLOUD_CLIENT_SECRET)) {
      return { id: this.id, state: 'limited', detail: 'Source artwork lookup available; search requires SoundCloud app credentials', checkedAt: new Date().toISOString() };
    }
    try {
      await api('/tracks', new URLSearchParams({ q: 'freetekno', limit: '1', linked_partitioning: 'true' }), signal);
      return { id: this.id, state: 'ready', detail: 'Official API reachable', checkedAt: new Date().toISOString() };
    } catch (error) {
      return { id: this.id, state: 'offline', detail: String(error), checkedAt: new Date().toISOString() };
    }
  }

  async resolvePublicArtwork(sourceUrl: string, signal?: AbortSignal): Promise<{ sourceUrl: string; title?: string; artworkUrl?: string }> {
    let source: URL;
    try {
      source = new URL(sourceUrl);
    } catch {
      throw new Error('SoundCloud source URL is invalid');
    }
    if (!['https:', 'http:'].includes(source.protocol)
      || !['soundcloud.com', 'www.soundcloud.com'].includes(source.hostname)
      || source.username || source.password || source.port
      || source.pathname.split('/').filter(Boolean).length < 2) {
      throw new Error('SoundCloud source URL is invalid');
    }
    source.protocol = 'https:';
    source.search = '';
    source.hash = '';
    const url = new URL('https://soundcloud.com/oembed');
    url.search = new URLSearchParams({ format: 'json', url: source.toString() }).toString();
    const response = await withTimeout((inner) => fetch(url, { signal: inner, headers: { accept: 'application/json' } }), 12_000, signal);
    if (!response.ok) throw new Error(`SoundCloud oEmbed ${response.status} ${response.statusText || 'request rejected'}`);
    const payload = await response.json() as { title?: string; thumbnail_url?: string };
    if (!payload.thumbnail_url) return { sourceUrl: source.toString(), title: payload.title };
    const thumbnail = new URL(payload.thumbnail_url);
    // oEmbed may return the uploader avatar when the track has no cover.
    const artworkUrl = thumbnail.hostname.endsWith('.sndcdn.com') && thumbnail.pathname.startsWith('/artworks-')
      ? thumbnail.toString()
      : undefined;
    return { sourceUrl: source.toString(), title: payload.title, artworkUrl };
  }

  async lookupArtwork(sourceUrl: string, signal?: AbortSignal): Promise<string | undefined> {
    return (await this.resolvePublicArtwork(sourceUrl, signal)).artworkUrl;
  }

  async search(query: SearchQuery, signal?: AbortSignal): Promise<MixCandidate[]> {
    const q = query.q || query.artist || '';
    if (!q) return [];
    const params = new URLSearchParams({
      q,
      limit: String(Math.min(query.limit ?? 50, 100)),
      access: 'playable',
      linked_partitioning: 'true',
      'duration[from]': String(query.minDurationMs ?? 30 * 60_000),
    });
    const payload = await api<{ collection?: Array<Record<string, any>> }>('/tracks', params, signal);
    const tracks = payload.collection ?? [];
    const candidates = tracks.map((track): MixCandidate => {
      const user = track.user || {};
      const scored = confidenceScore({
        query: q,
        title: track.title,
        artist: user.username,
        durationExpectedMs: query.durationExpectedMs,
        durationActualMs: track.duration,
      });
      return {
        provider: this.id,
        title: track.title || 'Untitled SoundCloud mix',
        artists: user.username ? [user.username] : [],
        crews: [],
        durationMs: track.duration,
        // created_at is the upload date, not necessarily the mix recording date.
        description: track.description,
        genres: [track.genre, ...(Array.isArray(track.tag_list) ? track.tag_list : String(track.tag_list || '').split(/\s+/))].filter(Boolean),
        artwork: track.artwork_url ? [track.artwork_url] : [],
        source: { provider: this.id, url: track.permalink_url, externalId: track.urn || String(track.id || '') },
        externalIds: { soundcloud: track.urn || String(track.id || '') },
        confidence: scored.score,
        reasons: scored.reasons.length ? scored.reasons : ['official SoundCloud API result'],
        raw: track,
      };
    });
    return uniqueCandidates(candidates);
  }
}
