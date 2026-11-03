import type { DiscoveryProvider, MixCandidate, ProviderHealth, SearchQuery } from '../domain.js';
import { confidenceScore, uniqueCandidates, withTimeout } from '../core/utils.js';

// Mixcloud's read API is token-free ("None to read; OAuth 2.0 to write"), so it
// gives this archive a credential-free discovery source for long DJ mixes.
const API = 'https://api.mixcloud.com/';

type MixcloudPicture = { large?: string; medium?: string; small?: string; thumbnail?: string };
type MixcloudCloudcast = {
  key?: string;
  name?: string;
  url?: string;
  slug?: string;
  type?: string;
  description?: string;
  audio_length?: number;
  created_time?: string;
  tags?: Array<{ name?: string }>;
  pictures?: MixcloudPicture;
  user?: { name?: string; username?: string; url?: string };
  hosts?: Array<{ name?: string; username?: string }>;
};

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

function cloudcastArtwork(cloudcast: MixcloudCloudcast): string | undefined {
  try {
    const url = new URL(cloudcast.pictures?.large || cloudcast.pictures?.medium || '');
    return url.protocol === 'https:' && (url.hostname === 'mixcloud.com' || url.hostname.endsWith('.mixcloud.com'))
      ? url.toString()
      : undefined;
  } catch {
    return;
  }
}

export class MixcloudProvider implements DiscoveryProvider {
  id = 'mixcloud' as const;
  private healthCache?: ProviderHealth;

  private async get<T>(path: string, params?: URLSearchParams, signal?: AbortSignal): Promise<T> {
    const url = new URL(path, API);
    if (params) url.search = params.toString();
    return withTimeout(async (inner) => {
      const response = await fetch(url, { signal: inner, headers: { accept: 'application/json' } });
      if (!response.ok) throw new Error(`Mixcloud API ${response.status} ${response.statusText || 'request rejected'}`);
      const payload = await response.json() as T & { error?: { message?: string } };
      if (payload && typeof payload === 'object' && payload.error) throw new Error(`Mixcloud API ${payload.error.message || 'request rejected'}`);
      return payload as T;
    }, 12_000, signal);
  }

  async lookupCloudcast(sourceUrl: string, signal?: AbortSignal): Promise<MixcloudCloudcast> {
    const key = mixcloudKey(sourceUrl);
    if (!key) throw new Error('Mixcloud track URL is invalid');
    const cloudcast = await this.get<MixcloudCloudcast>(key.replace(/^\//, ''), undefined, signal);
    if (!cloudcast.url || !cloudcast.name) throw new Error('Mixcloud did not return a public cloudcast');
    return cloudcast;
  }

  async lookupArtwork(sourceUrl: string, signal?: AbortSignal): Promise<string | undefined> {
    return cloudcastArtwork(await this.lookupCloudcast(sourceUrl, signal));
  }

  async health(signal?: AbortSignal): Promise<ProviderHealth> {
    if (this.healthCache && Date.now() - Date.parse(this.healthCache.checkedAt) < 5 * 60_000) return this.healthCache;
    const now = new Date().toISOString();
    try {
      const response = await withTimeout((inner) => fetch(API, { signal: inner }), 7_000, signal);
      this.healthCache = {
        id: this.id,
        state: response.ok ? 'ready' : 'offline',
        detail: response.ok ? 'Public API reachable; no credentials required' : `Public API ${response.status}`,
        checkedAt: now,
      };
    } catch (error) {
      this.healthCache = { id: this.id, state: 'offline', detail: String(error), checkedAt: now };
    }
    return this.healthCache;
  }

  async search(query: SearchQuery, signal?: AbortSignal): Promise<MixCandidate[]> {
    const q = query.q || query.artist || '';
    const limit = Math.min(Math.max(query.limit || 25, 1), 50);
    let cloudcasts: MixcloudCloudcast[];
    try {
      if (query.url) {
        cloudcasts = [await this.lookupCloudcast(query.url, signal)];
      } else if (q) {
        const payload = await this.get<{ data?: MixcloudCloudcast[] }>('search/', new URLSearchParams({ q, type: 'cloudcast', limit: String(limit) }), signal);
        cloudcasts = Array.isArray(payload.data) ? payload.data : [];
      } else {
        cloudcasts = [];
      }
    } catch (error) {
      if (/\b429\b|rate limit/i.test(String(error))) {
        this.healthCache = { id: this.id, state: 'limited', detail: 'Public search rate limit reached; try again later', checkedAt: new Date().toISOString() };
      }
      throw error;
    }
    const minDurationMs = query.minDurationMs ?? 30 * 60_000;
    const candidates = cloudcasts.slice(0, limit).flatMap((cloudcast): MixCandidate[] => {
      if (!cloudcast.url || !cloudcast.name) return [];
      const durationMs = Number(cloudcast.audio_length) * 1000;
      if (!Number.isFinite(durationMs) || durationMs <= 0 || durationMs < minDurationMs) return [];
      const title = cloudcast.name;
      const uploader = cloudcast.user?.name || cloudcast.user?.username || '';
      const scored = confidenceScore({ query: q, title, artist: uploader, durationExpectedMs: query.durationExpectedMs, durationActualMs: durationMs });
      const artwork = cloudcastArtwork(cloudcast);
      const externalId = cloudcast.key || cloudcast.slug || cloudcast.url;
      return [{
        provider: this.id,
        title,
        // A Mixcloud uploader/host is not a verified performing artist.
        artists: [],
        crews: [],
        uploader,
        durationMs,
        // audio_length is the upload duration and created_time is the upload date,
        // not the recording date — neither is promoted to a recording fact here.
        uploadedAt: cloudcast.created_time,
        description: cloudcast.description,
        genres: (cloudcast.tags || []).map((tag) => tag?.name).filter((name): name is string => Boolean(name)),
        artwork: artwork ? [artwork] : [],
        source: { provider: this.id, url: cloudcast.url, externalId },
        externalIds: { mixcloud: externalId },
        confidence: query.url ? 0.86 : scored.score,
        reasons: query.url ? ['Public Mixcloud URL supplied directly'] : scored.reasons.length ? scored.reasons : ['Mixcloud public search result'],
        raw: cloudcast as Record<string, unknown>,
      }];
    });
    return uniqueCandidates(candidates);
  }
}
