import type { EntityRef, MixCandidate, ProviderId, SourceRef } from '../domain.js';
import { normalizeQuery, overlapScore } from './utils.js';
import type { ProviderRegistry } from './registry.js';
import { probeAudioDuration } from './waveform.js';

export interface MixEnrichmentInput {
  title: string;
  artists: string[];
  providers?: ProviderId[];
  crews?: string[];
  durationMs?: number;
  recordedAt?: string;
  description?: string;
  genres?: string[];
  artwork?: Array<{ url: string; provider?: ProviderId; kind?: 'cover' | 'artist' | 'crew' }>;
  sources?: SourceRef[];
  externalIds?: Record<string, string>;
}

export interface EnrichmentArtwork {
  url: string;
  provider: ProviderId;
  kind: 'cover' | 'artist' | 'crew';
}

export interface EnrichmentProvenance {
  provider: ProviderId;
  field: string;
  confidence: number;
  sourceUrl?: string;
}

export interface MixEnrichmentResult {
  patch: {
    durationMs?: number;
    recordedAt?: string;
    description?: string;
    genres?: string[];
    artwork?: EnrichmentArtwork[];
    sources?: SourceRef[];
    externalIds?: Record<string, string>;
  };
  candidates: MixCandidate[];
  entities: EntityRef[];
  provenance: EnrichmentProvenance[];
  attempted: ProviderId[];
  failures: Array<{ provider: ProviderId; error: string }>;
}

function exactEntity(name: string, entities: EntityRef[]): EntityRef | undefined {
  const wanted = normalizeQuery(name);
  const matches = entities.filter((entity) => normalizeQuery(entity.name) === wanted);
  return matches.length === 1 ? matches[0] : undefined;
}

function providerQuery(provider: ProviderId, input: MixEnrichmentInput) {
  const identity = [input.artists?.[0], input.title].filter(Boolean).join(' ').trim();
  if (provider === 'freeteknomusic') {
    const anchor = input.crews?.[0] || input.artists?.[0];
    if (!anchor) return null;
    const slug = normalizeQuery(anchor).replace(/\s+/g, '');
    if (!slug) return null;
    return {
      url: `https://archive.freeteknomusic.org/${encodeURIComponent(slug)}/`,
      q: identity,
      maxDepth: 0,
      maxItems: 120,
      limit: 8,
      durationExpectedMs: input.durationMs,
    };
  }
  return {
    q: identity || input.artists?.[0] || input.title,
    minDurationMs: Math.max(30 * 60_000, Math.floor((input.durationMs || 0) * 0.65)) || 30 * 60_000,
    durationExpectedMs: input.durationMs,
    limit: 8,
  };
}

export async function enrichMix(registry: ProviderRegistry, input: MixEnrichmentInput): Promise<MixEnrichmentResult> {
  const attempted: ProviderId[] = [];
  const failures: Array<{ provider: ProviderId; error: string }> = [];
  const candidates: MixCandidate[] = [];
  const selectedProviders = input.providers ? new Set(input.providers) : undefined;

  const discoveryTasks = [...registry.discovery.entries()]
    .filter(([providerId]) => !selectedProviders || selectedProviders.has(providerId as ProviderId))
    .map(async ([providerId, provider]) => {
      const id = providerId as ProviderId;
      const query = providerQuery(id, input);
      if (!query) return;
      attempted.push(id);
      try {
        const health = await provider.health();
        if (health.state === 'offline' || (['soundcloud', 'youtube', 'hearthis'].includes(id) && health.state !== 'ready')) {
          failures.push({ provider: id, error: health.detail });
          return;
        }
        const results = await provider.search(query);
        candidates.push(...results.filter((candidate) => candidate.confidence >= 0.5));
      } catch (error) {
        failures.push({ provider: id, error: error instanceof Error ? error.message : String(error) });
      }
    });

  await Promise.all(discoveryTasks);
  candidates.sort((a, b) => b.confidence - a.confidence);

  const patch: MixEnrichmentResult['patch'] = {};
  const provenance: EnrichmentProvenance[] = [];
  const addedSources = new Map<string, SourceRef>();
  const externalIds: Record<string, string> = {};
  const artwork: EnrichmentArtwork[] = [];

  if (!input.durationMs) {
    const audio = (input.sources || []).find((source) => source.provider === 'freeteknomusic' && /\.(mp3|flac|ogg|oga|wav|m4a|aac|aif|aiff)(?:[?#]|$)/i.test(source.url));
    if (audio) {
      attempted.push('freeteknomusic');
      try {
        const durationMs = await probeAudioDuration(audio.url);
        if (durationMs) {
          patch.durationMs = durationMs;
          provenance.push({ provider: 'freeteknomusic', field: 'durationMs', confidence: 0.99, sourceUrl: audio.url });
        }
      } catch (error) {
        failures.push({ provider: 'freeteknomusic', error: error instanceof Error ? error.message : String(error) });
      }
    }
  }

  if (!(input.artwork || []).length) {
    for (const source of (input.sources || []).filter((item) => ['soundcloud', 'youtube', 'hearthis'].includes(item.provider))) {
      attempted.push(source.provider);
      try {
        const resolver = source.provider === 'youtube' ? registry.youtube : source.provider === 'hearthis' ? registry.hearthis : registry.soundcloud;
        const url = await resolver.lookupArtwork(source.url);
        if (url) {
          artwork.push({ url, provider: source.provider, kind: 'cover' });
          provenance.push({ provider: source.provider, field: 'artwork', confidence: 0.99, sourceUrl: source.url });
          break;
        }
      } catch (error) {
        failures.push({ provider: source.provider, error: error instanceof Error ? error.message : String(error) });
      }
    }
  }

  const fill = <K extends 'durationMs' | 'recordedAt' | 'description' | 'genres'>(
    field: K,
    value: MixEnrichmentResult['patch'][K],
    candidate: MixCandidate,
  ) => {
    if (value === undefined || value === '' || (Array.isArray(value) && !value.length)) return;
    if (patch[field] !== undefined) return;
    (patch as Record<string, unknown>)[field] = value;
    provenance.push({
      provider: candidate.provider,
      field,
      confidence: candidate.confidence,
      sourceUrl: candidate.source.url,
    });
  };

  for (const candidate of candidates) {
    if (candidate.confidence < 0.58) continue;
    const titleOverlap = overlapScore(candidate.title, input.title);
    const artistOverlap = overlapScore(candidate.artists.join(' '), input.artists.join(' '));
    const effectiveDurationMs = input.durationMs || patch.durationMs;
    const durationDelta = effectiveDurationMs && candidate.durationMs
      ? Math.abs(effectiveDurationMs - candidate.durationMs) / effectiveDurationMs
      : 0;
    const curatorUploadWithMatchingDuration = candidate.provider === 'youtube' && titleOverlap >= 0.75
      && Boolean(effectiveDurationMs && candidate.durationMs && durationDelta <= 0.08);
    const sameRecording = titleOverlap >= 0.5 && (artistOverlap >= 0.3 || !candidate.artists.length || curatorUploadWithMatchingDuration)
      && (!effectiveDurationMs || !candidate.durationMs || durationDelta <= 0.15);
    if (!sameRecording || candidate.confidence < 0.7) continue;
    if (!input.durationMs && candidate.durationMs) fill('durationMs', candidate.durationMs, candidate);
    if (!input.recordedAt && candidate.recordedAt) fill('recordedAt', candidate.recordedAt, candidate);
    if (!input.description && candidate.description) fill('description', candidate.description, candidate);
    if (!(input.genres || []).length && candidate.genres?.length) fill('genres', candidate.genres, candidate);

    if (!(input.artwork || []).length && !artwork.length && candidate.artwork?.length) {
      artwork.push(...candidate.artwork.slice(0, 2).map((url) => ({ url, provider: candidate.provider, kind: 'cover' as const })));
      provenance.push({
        provider: candidate.provider,
        field: 'artwork',
        confidence: candidate.confidence,
        sourceUrl: candidate.source.url,
      });
    }

    if (candidate.confidence >= 0.64) {
      addedSources.set(`${candidate.source.provider}:${candidate.source.url}`, candidate.source);
      Object.assign(externalIds, candidate.externalIds || {});
    }
  }

  const entities: EntityRef[] = [];
  if ((!selectedProviders || selectedProviders.has('discogs')) && (input.artists?.length || input.crews?.length)) {
    attempted.push('discogs');
    try {
      const health = await registry.discogs.health();
      if (health.state === 'offline') {
        failures.push({ provider: 'discogs', error: health.detail });
      } else {
        for (const [kind, names] of [
          ['artist', input.artists || []],
          ['crew', input.crews || []],
        ] as const) {
          for (const name of names.slice(0, 3)) {
            try {
              const matches = await registry.discogs.enrichEntity(name, kind);
              const exact = exactEntity(name, matches);
              if (!exact) continue;
              entities.push(exact);
              if (exact.profile) provenance.push({ provider: 'discogs', field: `${kind} profile`, confidence: 0.88, sourceUrl: exact.url });
              const idKey = kind === 'artist' ? 'discogsArtist' : 'discogsCrew';
              if (exact.externalId && !externalIds[idKey]) externalIds[idKey] = exact.externalId;
              if (exact.url) addedSources.set(`discogs:${exact.url}`, { provider: 'discogs', url: exact.url, externalId: exact.externalId });
            } catch (error) {
              failures.push({ provider: 'discogs', error: error instanceof Error ? error.message : String(error) });
              break;
            }
          }
        }
      }
    } catch (error) {
      failures.push({ provider: 'discogs', error: error instanceof Error ? error.message : String(error) });
    }
  }

  if (artwork.length) patch.artwork = artwork;
  if (addedSources.size) patch.sources = [...addedSources.values()];
  if (Object.keys(externalIds).length) patch.externalIds = externalIds;

  return {
    patch,
    candidates: candidates.slice(0, 12),
    entities,
    provenance,
    attempted: [...new Set(attempted)],
    failures,
  };
}
