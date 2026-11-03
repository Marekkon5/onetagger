import type { EntityRef, ProviderHealth } from '../domain.js';
import { retry, withTimeout } from '../core/utils.js';

const API = 'https://api.discogs.com/';
function profileText(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  return value.replace(/\u0000/g, '').replace(/\[(?:a|l|r|m)=([^\]]+)\]/gi, '$1')
    .replace(/\[url=([^\]]+)\]([^]*?)\[\/url\]/gi, '$2 ($1)')
    .replace(/\[\/?(?:b|i|u|s|url)\]/gi, '').trim().slice(0, 20000) || undefined;
}
function textList(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string' && item.length > 0 && item.length <= 500).slice(0,100) : [];
}
function entityRefs(value: unknown, kind: 'artist' | 'label' = 'artist'): EntityRef[] {
  return (Array.isArray(value) ? value : value && typeof value === 'object' ? [value] : []).slice(0,100).flatMap(item => {
    if (!item || typeof item !== 'object') return [];
    const ref=item as Record<string,unknown>;
    if (!/^\d+$/.test(String(ref.id)) || typeof ref.name !== 'string' || !ref.name.trim()) return [];
    return [{kind,name:ref.name.trim().slice(0,500),externalId:String(ref.id),provider:'discogs' as const,url:`https://www.discogs.com/${kind}/${ref.id}`}];
  });
}

export class DiscogsEnricher {
  id = 'discogs' as const;

  private headers(): Record<string, string> {
    const token = process.env.DISCOGS_TOKEN;
    return {
      accept: 'application/vnd.discogs.v2.discogs+json',
      'user-agent': 'SYCO23-Mixsets/0.1 (+metadata-enrichment)',
      ...(token ? { authorization: `Discogs token=${token}` } : {}),
    };
  }

  async health(signal?: AbortSignal): Promise<ProviderHealth> {
    try {
      const response = await withTimeout((inner) => fetch('https://api.discogs.com/database/search?q=spiral+tribe&type=artist&per_page=1', { signal: inner, headers: this.headers() }), 7_000, signal);
      const authenticated = Boolean(process.env.DISCOGS_TOKEN);
      return {
        id: this.id,
        state: response.ok ? (authenticated ? 'ready' : 'limited') : 'offline',
        detail: response.ok
          ? authenticated ? 'Artist/label enrichment authenticated' : 'Unauthenticated rate limit; set DISCOGS_TOKEN'
          : `Discogs API ${response.status} ${response.statusText || 'request rejected'}`,
        checkedAt: new Date().toISOString(),
      };
    } catch (error) {
      return { id: this.id, state: 'offline', detail: String(error), checkedAt: new Date().toISOString() };
    }
  }

  async hydrateEntity(id: string, kind: 'artist' | 'crew' | 'label', signal?: AbortSignal): Promise<EntityRef | undefined> {
    if (!/^\d+$/.test(id)) throw new Error('Discogs entity ID must be numeric');
    const type = kind === 'label' ? 'labels' : 'artists';
    const payload = await retry(() => withTimeout(async (inner) => {
      const response = await fetch(new URL(`${type}/${id}`, API), { signal: inner, headers: this.headers() });
      if (!response.ok) throw new Error(`Discogs entity ${response.status} ${response.statusText || 'request rejected'}`);
      return response.json() as Promise<Record<string, unknown>>;
    }, 12_000, signal));

    if (String(payload.id) !== id) throw new Error('Discogs returned a different entity ID');
    const name = String(payload.name || payload.title || '').trim();
    if (!name) return undefined;
    const images = Array.isArray(payload.images) ? payload.images as Array<{ uri?: unknown }> : [];
    const profile = profileText(payload.profile);
    return {
      kind,
      name,
      provider: this.id,
      externalId: id,
      url: typeof payload.uri === 'string' ? payload.uri : `https://www.discogs.com/${kind === 'label' ? 'label' : 'artist'}/${id}`,
      imageUrl: typeof images[0]?.uri === 'string' ? images[0].uri : undefined,
      profile: profile || undefined,
      aliases: textList(payload.namevariations),
      realName: typeof payload.realname === 'string' ? payload.realname.trim().slice(0,500) || undefined : undefined,
      websiteUrls: textList(payload.urls).filter(value=>{try{return new URL(value).protocol==='https:';}catch{return false;}}),
      aliasRefs: entityRefs(payload.aliases),
      groups: entityRefs(payload.groups),
      members: entityRefs(payload.members),
      parent: entityRefs(payload.parent_label,'label')[0],
      subLabels: entityRefs(payload.sublabels,'label')
    };
  }

  async enrichEntity(name: string, kind: 'artist' | 'crew' | 'label', signal?: AbortSignal): Promise<EntityRef[]> {
    const type = kind === 'label' ? 'label' : 'artist';
    const params = new URLSearchParams({ q: name, type, per_page: '5' });
    const payload = await retry(() => withTimeout(async (inner) => {
      const response = await fetch(new URL(`database/search?${params}`, API), { signal: inner, headers: this.headers() });
      if (!response.ok) throw new Error(`Discogs search ${response.status} ${response.statusText || 'request rejected'}`);
      return response.json() as Promise<{ results?: Array<Record<string, any>> }>;
    }, 12_000, signal));

    const results = payload.results ?? [];
    const exact = results.filter((result) => String(result.title || '').trim().toLowerCase() === name.trim().toLowerCase());
    // A search hit only has a placeholder profile. Hydrate a unique exact match
    // from the entity endpoint; ambiguous names remain review candidates.
    const details = exact.length === 1 && Number.isSafeInteger(Number(exact[0].id))
      ? await retry(() => withTimeout(async (inner) => {
        const response = await fetch(new URL(`${type === 'label' ? 'labels' : 'artists'}/${exact[0].id}`, API), {
          signal: inner,
          headers: this.headers(),
        });
        if (!response.ok) throw new Error(`Discogs entity ${response.status} ${response.statusText || 'request rejected'}`);
        return response.json() as Promise<Record<string, any>>;
      }, 12_000, signal))
      : null;

    return results.map((result) => ({
      kind,
      name: result.title || name,
      provider: this.id,
      externalId: String(result.id || ''),
      url: details && details.id === result.id ? details.uri : result.uri,
      imageUrl: details && details.id === result.id ? details.images?.[0]?.uri || result.cover_image : result.cover_image || result.thumb,
      profile: details && details.id === result.id ? profileText(details.profile) : undefined,
    }));
  }
}
