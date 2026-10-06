import type { DiscoveryProvider, MixCandidate, ProviderHealth, SearchQuery } from '../domain.js';
import { confidenceScore, uniqueCandidates, withTimeout } from '../core/utils.js';

const API = 'https://api-v2.hearthis.at/';

type HearthisTrack = {
  id?: string;
  title?: string;
  duration?: string | number;
  description?: string;
  genre?: string;
  tags?: string;
  permalink_url?: string;
  artwork_url?: string;
  user?: { username?: string };
};

export function hearthisPath(sourceUrl: string): string | undefined {
  try {
    const url = new URL(sourceUrl);
    if (!['https:', 'http:'].includes(url.protocol) || !['hearthis.at', 'www.hearthis.at'].includes(url.hostname)
      || url.username || url.password || url.port) return;
    const parts = url.pathname.split('/').filter(Boolean);
    return parts.length >= 2 && parts.every((part) => !['.', '..'].includes(part))
      ? `${parts.join('/')}/`
      : undefined;
  } catch {
    return;
  }
}

function trackArtwork(track: HearthisTrack): string | undefined {
  try {
    const url = new URL(track.artwork_url || '');
    return url.protocol === 'https:' && url.hostname === 'img.hearthis.at' && url.pathname.includes('/image_track/')
      ? url.toString()
      : undefined;
  } catch {
    return;
  }
}

export class HearthisProvider implements DiscoveryProvider {
  id = 'hearthis' as const;
  private healthCache?: ProviderHealth;

  private async get<T>(path: string, params?: URLSearchParams, signal?: AbortSignal): Promise<T> {
    const url = new URL(path, API);
    if (params) url.search = params.toString();
    return withTimeout(async (inner) => {
      const response = await fetch(url, { signal: inner, headers: { accept: 'application/json' } });
      if (!response.ok) throw new Error(`hearthis.at API ${response.status} ${response.statusText || 'request rejected'}`);
      const payload = await response.json() as T | { success?: boolean; message?: string };
      if (payload && typeof payload === 'object' && 'success' in payload && payload.success === false) {
        throw new Error(`hearthis.at API ${payload.message || 'request rejected'}`);
      }
      return payload as T;
    }, 12_000, signal);
  }

  async lookupTrack(sourceUrl: string, signal?: AbortSignal): Promise<HearthisTrack> {
    const path = hearthisPath(sourceUrl);
    if (!path) throw new Error('hearthis.at track URL is invalid');
    const track = await this.get<HearthisTrack>(path, undefined, signal);
    if (!track.id || !track.permalink_url) throw new Error('hearthis.at did not return a public track');
    return track;
  }

  async lookupArtwork(sourceUrl: string, signal?: AbortSignal): Promise<string | undefined> {
    return trackArtwork(await this.lookupTrack(sourceUrl, signal));
  }

  async health(signal?: AbortSignal): Promise<ProviderHealth> {
    if (this.healthCache && Date.now() - Date.parse(this.healthCache.checkedAt) < 5 * 60_000) return this.healthCache;
    const now = new Date().toISOString();
    try {
      const response = await withTimeout((inner) => fetch(API, { signal: inner }), 7_000, signal);
      this.healthCache = { id: this.id, state: response.ok ? 'ready' : 'offline', detail: response.ok ? 'Public API reachable; searches may rate-limit' : `Public API ${response.status}`, checkedAt: now };
    } catch (error) {
      this.healthCache = { id: this.id, state: 'offline', detail: String(error), checkedAt: now };
    }
    return this.healthCache;
  }

  async search(query: SearchQuery, signal?: AbortSignal): Promise<MixCandidate[]> {
    const q = query.q || query.artist || '';
    const limit = Math.min(Math.max(query.limit || 25, 1), 50);
    let tracks: HearthisTrack[];
    try {
      tracks = query.url
        ? [await this.lookupTrack(query.url, signal)]
        : q ? await this.get<HearthisTrack[]>('search/', new URLSearchParams({ t: q, count: String(limit) }), signal) : [];
    } catch (error) {
      if (String(error).toLowerCase().includes('limit reached')) {
        this.healthCache = { id: this.id, state: 'limited', detail: 'Public search rate limit reached; try again later', checkedAt: new Date().toISOString() };
      }
      throw error;
    }
    if (!Array.isArray(tracks)) throw new Error('hearthis.at returned an unexpected search response');
    const minDurationMs = query.minDurationMs ?? 30 * 60_000;
    const candidates = tracks.slice(0, limit).flatMap((track): MixCandidate[] => {
      if (!track.id || !track.permalink_url) return [];
      const durationMs = Number(track.duration) * 1000;
      if (!Number.isFinite(durationMs) || durationMs < minDurationMs) return [];
      const title = track.title || 'Untitled hearthis.at mix';
      const artist = track.user?.username || '';
      const scored = confidenceScore({ query: q, title, artist, durationExpectedMs: query.durationExpectedMs, durationActualMs: durationMs });
      const artwork = trackArtwork(track);
      return [{
        provider: this.id, title, artists: artist ? [artist] : [], crews: [], durationMs,
        description: track.description, genres: [track.genre, ...(track.tags || '').split(',')].map((value) => value?.trim()).filter((value): value is string => Boolean(value)),
        artwork: artwork ? [artwork] : [],
        source: { provider: this.id, url: track.permalink_url, externalId: track.id },
        externalIds: { hearthis: track.id },
        confidence: query.url ? 0.86 : scored.score,
        reasons: query.url ? ['Public track URL supplied directly'] : scored.reasons.length ? scored.reasons : ['hearthis.at public search result'],
        raw: track as Record<string, unknown>,
      }];
    });
    return uniqueCandidates(candidates);
  }
}
