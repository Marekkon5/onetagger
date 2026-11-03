import type { DiscoveryProvider, MixCandidate, ProviderHealth, SearchQuery } from '../domain.js';
import { confidenceScore, uniqueCandidates, withTimeout } from '../core/utils.js';

const API = 'https://www.googleapis.com/youtube/v3/';
const VIDEO_ID = /^[A-Za-z0-9_-]{11}$/;

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
    return id && VIDEO_ID.test(id) ? id : undefined;
  } catch {
    return;
  }
}

export function youtubeDurationMs(value: string): number | undefined {
  const match = /^P(?:(\d+)D)?T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?$/.exec(value);
  if (!match) return;
  const seconds = Number(match[1] || 0) * 86400 + Number(match[2] || 0) * 3600 + Number(match[3] || 0) * 60 + Number(match[4] || 0);
  return seconds > 0 ? seconds * 1000 : undefined;
}

type YoutubeSnippet = {
  title?: string;
  channelTitle?: string;
  description?: string;
  tags?: string[];
  thumbnails?: Record<string, { url?: string }>;
};

function videoIdentity(title: string, channel: string): { title: string; artists: string[] } {
  const match = title.match(/^([^–—-]{2,60})\s+[–—-]\s+(.+)$/);
  return match ? { title: match[2].trim(), artists: [match[1].trim()] }
    : { title, artists: [] };
}

export class YouTubeProvider implements DiscoveryProvider {
  id = 'youtube' as const;
  private healthCache?: ProviderHealth;

  private async api<T>(path: string, params: URLSearchParams, signal?: AbortSignal): Promise<T> {
    const key = process.env.YOUTUBE_API_KEY;
    if (!key) throw new Error('YouTube API key required for text search; use a public video URL instead');
    params.set('key', key);
    const url = new URL(path, API);
    url.search = params.toString();
    return withTimeout(async (inner) => {
      const response = await fetch(url, { signal: inner, headers: { accept: 'application/json' } });
      if (!response.ok) throw new Error(`YouTube Data API ${response.status} ${response.statusText || 'request rejected'}`);
      return response.json() as Promise<T>;
    }, 12_000, signal);
  }

  async lookupVideo(sourceUrl: string, signal?: AbortSignal): Promise<{ id: string; title: string; artist: string; artworkUrl?: string }> {
    const id = youtubeVideoId(sourceUrl);
    if (!id) throw new Error('YouTube video URL is invalid');
    const url = new URL('https://www.youtube.com/oembed');
    url.search = new URLSearchParams({ url: `https://www.youtube.com/watch?v=${id}`, format: 'json' }).toString();
    const payload = await withTimeout(async (inner) => {
      const response = await fetch(url, { signal: inner, headers: { accept: 'application/json' } });
      if (!response.ok) throw new Error(`YouTube oEmbed ${response.status} ${response.statusText || 'request rejected'}`);
      return response.json() as Promise<{ title?: string; author_name?: string; thumbnail_url?: string }>;
    }, 12_000, signal);
    return { id, title: payload.title || 'Untitled YouTube video', artist: payload.author_name || '', artworkUrl: payload.thumbnail_url };
  }

  async lookupArtwork(sourceUrl: string, signal?: AbortSignal): Promise<string | undefined> {
    return (await this.lookupVideo(sourceUrl, signal)).artworkUrl;
  }

  async health(signal?: AbortSignal): Promise<ProviderHealth> {
    const now = new Date().toISOString();
    if (!process.env.YOUTUBE_API_KEY) return { id: this.id, state: 'limited', detail: 'Public video links available; text search needs YOUTUBE_API_KEY', checkedAt: now };
    if (this.healthCache && Date.now() - Date.parse(this.healthCache.checkedAt) < 5 * 60_000) return this.healthCache;
    try {
      await this.api('videos', new URLSearchParams({ part: 'id', id: 'dQw4w9WgXcQ' }), signal);
      this.healthCache = { id: this.id, state: 'ready', detail: 'YouTube Data API key accepted', checkedAt: now };
    } catch (error) {
      this.healthCache = { id: this.id, state: 'offline', detail: String(error), checkedAt: now };
    }
    return this.healthCache;
  }

  async search(query: SearchQuery, signal?: AbortSignal): Promise<MixCandidate[]> {
    if (query.url && process.env.YOUTUBE_API_KEY) {
      const id = youtubeVideoId(query.url);
      if (!id) throw new Error('YouTube video URL is invalid');
      const payload = await this.api<{ items?: Array<{id:string;snippet?:YoutubeSnippet;contentDetails?:{duration?:string}}> }>('videos',new URLSearchParams({part:'snippet,contentDetails',id}),signal);
      const video=payload.items?.find(item=>item.id===id);
      if (!video) return [];
      const snippet=video.snippet??{},title=snippet.title??'Untitled YouTube video';
      const identity=videoIdentity(title,snippet.channelTitle??'');
      const artwork=snippet.thumbnails?.maxres?.url??snippet.thumbnails?.high?.url??snippet.thumbnails?.medium?.url;
      return [{provider:this.id,title:identity.title,artists:identity.artists,uploader:snippet.channelTitle,
        fieldEvidence:{artists:'parsed'},crews:[],durationMs:youtubeDurationMs(video.contentDetails?.duration??''),description:snippet.description,
        genres:snippet.tags??[],artwork:artwork?[artwork]:[],source:{provider:this.id,url:`https://www.youtube.com/watch?v=${id}`,externalId:id},
        externalIds:{youtube:id},confidence:.99,reasons:['Known public video resource hydrated directly'],raw:{videoId:id}}];
    }
    if (query.url) {
      const video = await this.lookupVideo(query.url, signal);
      const identity = videoIdentity(video.title, video.artist);
      return [{
        provider: this.id,
        title: identity.title,
        artists: identity.artists,
        uploader: video.artist,
        fieldEvidence: { artists: 'parsed' },
        crews: [],
        artwork: video.artworkUrl ? [video.artworkUrl] : [],
        source: { provider: this.id, url: `https://www.youtube.com/watch?v=${video.id}`, externalId: video.id },
        externalIds: { youtube: video.id },
        confidence: 0.86,
        reasons: ['Public video URL supplied directly; verify this is a long mix'],
        raw: { videoId: video.id, oembed: true },
      }];
    }
    const q = query.q || query.artist || '';
    if (!q) return [];
    const result = await this.api<{ items?: Array<{ id?: { videoId?: string } }> }>('search', new URLSearchParams({
      part: 'snippet', type: 'video', videoDuration: 'long', q,
      maxResults: String(Math.min(Math.max(query.limit || 25, 1), 50)),
    }), signal);
    const ids = (result.items || []).map((item) => item.id?.videoId).filter((id): id is string => Boolean(id && VIDEO_ID.test(id)));
    if (!ids.length) return [];
    const details = await this.api<{ items?: Array<{ id?: string; snippet?: YoutubeSnippet; contentDetails?: { duration?: string } }> }>('videos', new URLSearchParams({
      part: 'snippet,contentDetails', id: ids.join(','),
    }), signal);
    const minDurationMs = query.minDurationMs ?? 30 * 60_000;
    const candidates = (details.items || []).flatMap((video): MixCandidate[] => {
      if (!video.id || !VIDEO_ID.test(video.id)) return [];
      const durationMs = youtubeDurationMs(video.contentDetails?.duration || '');
      if (!durationMs || durationMs < minDurationMs) return [];
      const snippet = video.snippet || {};
      const title = snippet.title || 'Untitled YouTube mix';
      const artist = snippet.channelTitle || '';
      const identity = videoIdentity(title, artist);
      const scored = confidenceScore({ query: q, title: identity.title, artist: identity.artists.join(' '), durationExpectedMs: query.durationExpectedMs, durationActualMs: durationMs });
      const artworkUrl = snippet.thumbnails?.maxres?.url || snippet.thumbnails?.high?.url || snippet.thumbnails?.medium?.url;
      return [{
        provider: this.id, title: identity.title, artists: identity.artists, uploader: artist, fieldEvidence: { artists: 'parsed' }, crews: [], durationMs,
        description: snippet.description, genres: snippet.tags || [],
        artwork: artworkUrl ? [artworkUrl] : [],
        source: { provider: this.id, url: `https://www.youtube.com/watch?v=${video.id}`, externalId: video.id },
        externalIds: { youtube: video.id },
        confidence: scored.score, reasons: scored.reasons.length ? scored.reasons : ['YouTube long-video result; verify uploader identity'],
        raw: { videoId: video.id, snippet, contentDetails: video.contentDetails },
      }];
    });
    return uniqueCandidates(candidates);
  }
}
