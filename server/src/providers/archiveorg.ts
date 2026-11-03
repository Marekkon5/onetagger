import type { DiscoveryProvider, MixCandidate, ProviderHealth, SearchQuery } from '../domain.js';
import { confidenceScore, retry, uniqueCandidates, withTimeout } from '../core/utils.js';

const SEARCH = 'https://archive.org/advancedsearch.php';
const METADATA = 'https://archive.org/metadata/';

const list = (value: unknown): string[] => Array.isArray(value) ? value.map(String) : value ? [String(value)] : [];

export class ArchiveOrgProvider implements DiscoveryProvider {
  id = 'archiveorg' as const;

  async health(signal?: AbortSignal): Promise<ProviderHealth> {
    try {
      const response = await withTimeout((inner) => fetch('https://archive.org/metadata/etree', { signal: inner }), 7_000, signal);
      return { id: this.id, state: response.ok ? 'ready' : 'limited', detail: `Metadata API ${response.status}`, checkedAt: new Date().toISOString() };
    } catch (error) {
      return { id: this.id, state: 'offline', detail: String(error), checkedAt: new Date().toISOString() };
    }
  }

  async search(query: SearchQuery, signal?: AbortSignal): Promise<MixCandidate[]> {
    const q = query.q || query.artist || 'freetekno';
    const params = new URLSearchParams({
      q: `mediatype:audio AND (${q})`,
      output: 'json',
      rows: String(Math.min(query.limit ?? 25, 75)),
      page: '1',
    });
    for (const field of ['identifier', 'title', 'creator', 'date', 'description', 'subject', 'collection']) params.append('fl[]', field);

    let knownId: string | undefined;
    if (query.url) {
      const source=new URL(query.url);
      if (source.protocol!=='https:' || source.hostname!=='archive.org' || source.username || source.password || source.port) throw new Error('Archive.org source URL is invalid');
      knownId=/^\/(?:details|download)\/([A-Za-z0-9._-]+)/.exec(source.pathname)?.[1];
      if (!knownId) throw new Error('Archive.org source URL is invalid');
    }
    const searchPayload = knownId ? { response: { docs: [{ identifier: knownId }] } } : await retry(() => withTimeout(async (inner) => {
      const response = await fetch(`${SEARCH}?${params}`, { signal: inner });
      if (!response.ok) throw new Error(`Archive.org search ${response.status}`);
      return response.json() as Promise<{ response?: { docs?: Array<Record<string, unknown>> } }>;
    }, 12_000, signal));

    let docs = searchPayload.response?.docs ?? [];
    const candidates: MixCandidate[] = [];

    for (const doc of docs.slice(0, query.limit ?? 25)) {
      const identifier = String(doc.identifier || '');
      if (!identifier) continue;
      const detail = await retry(() => withTimeout(async (inner) => {
        const response = await fetch(`${METADATA}${encodeURIComponent(identifier)}`, { signal: inner });
        if (!response.ok) throw new Error(`Archive.org metadata ${response.status}`);
        return response.json() as Promise<Record<string, any>>;
      }, 12_000, signal));

      const metadata = detail.metadata || {};
      const files = Array.isArray(detail.files) ? detail.files : [];
      const audio = files.find((file: any) => /\.(mp3|ogg|flac|m4a)$/i.test(file.name || '') && !/thumb|spectrogram/i.test(file.name || ''));
      const image = files.find((file: any) => /\.(jpg|jpeg|png)$/i.test(file.name || '') && /cover|thumb|itemimage/i.test(`${file.name} ${file.source || ''}`));
      const durationMs = audio?.length ? Math.round(Number(audio.length) * 1000) : undefined;
      if (query.minDurationMs && durationMs && durationMs < query.minDurationMs) continue;

      const title = String(metadata.title || doc.title || identifier);
      const creators = list(metadata.creator || doc.creator);
      const scored = confidenceScore({ query: q, title, artist: creators.join(' '), durationExpectedMs: query.durationExpectedMs, durationActualMs: durationMs });
      candidates.push({
        provider: this.id,
        title,
        artists: creators,
        crews: [],
        durationMs,
        recordedAt: String(metadata.date || doc.date || '').slice(0, 10) || undefined,
        description: list(metadata.description || doc.description).join('\n'),
        genres: [...list(metadata.subject || doc.subject), ...list(metadata.collection || doc.collection)],
        artwork: image ? [`https://archive.org/download/${identifier}/${encodeURIComponent(image.name)}`] : [],
        source: { provider: this.id, url: `https://archive.org/details/${identifier}`, externalId: identifier },
        externalIds: { archiveorg: identifier },
        confidence: scored.score,
        reasons: scored.reasons.length ? scored.reasons : ['Archive.org metadata match'],
        raw: { metadata, audioFile: audio, imageFile: image },
      });
    }
    return uniqueCandidates(candidates);
  }
}
