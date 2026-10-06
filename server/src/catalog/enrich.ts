import { createHash, randomUUID } from 'node:crypto';
import {
  EnrichmentReportSchema,
  missingFields,
  type CatalogRecord,
  type EnrichmentReport,
  type MixRecord,
  type ProviderRef,
  type RecordingDateSchema
} from '@syco23/catalog-domain';
import { applyClaims } from './merge.js';
import { claimsFromProvider } from './provider-claims.js';
import type { CatalogRepository, CatalogTransaction } from './repository.js';
import type { CuratorActor } from '../auth/curator.js';
import type { EntityRecord } from '@syco23/catalog-domain';
import { enrichMix } from '../core/enrichment.js';
import { overlapScore } from '../core/utils.js';
import type { ProviderRegistry } from '../core/registry.js';

type SourceLike = { provider: ProviderRef['provider']; url: string; externalId?: string };

function resourceType(provider: ProviderRef['provider']): string {
  if (provider === 'youtube') return 'video';
  if (provider === 'soundcloud' || provider === 'hearthis') return 'track';
  if (provider === 'archiveorg') return 'item';
  if (provider === 'discogs') return 'artist';
  return 'recording';
}

function providerRef(source: SourceLike): ProviderRef {
  return {
    provider: source.provider,
    resourceType: resourceType(source.provider),
    externalId: source.externalId || createHash('sha256').update(source.url).digest('hex'),
    url: source.url
  };
}

function recordingDate(value: string | undefined): ReturnType<typeof RecordingDateSchema.parse> | undefined {
  if (!value) return undefined;
  const candidate = value.trim();
  if (/^\d{4}$/.test(candidate)) return { value: candidate, precision: 'year' };
  if (/^\d{4}-\d{2}$/.test(candidate)) return { value: candidate, precision: 'month' };
  if (/^\d{4}-\d{2}-\d{2}$/.test(candidate)) return { value: candidate, precision: 'day' };
  return undefined;
}

function candidateConfirmsRecording(record: MixRecord, candidate: Awaited<ReturnType<ProviderRegistry['search']>>[number], artists: string[]): boolean {
  if (!record.durationMs || !candidate.durationMs || candidate.confidence < 0.7) return false;
  const durationDelta = Math.abs(record.durationMs - candidate.durationMs) / record.durationMs;
  if (durationDelta > 0.08 || overlapScore(candidate.title, record.title) < 0.65) return false;
  if (candidate.provider === 'youtube') return true;
  return Boolean(artists.length && candidate.artists.length && overlapScore(candidate.artists.join(' '), artists.join(' ')) >= 0.3);
}

function linkProviderSource(repository: CatalogRepository, recordId: string, source: SourceLike): boolean {
  const ref = providerRef(source);
  const existing = repository.findByProvider(ref);
  if (existing) return existing === recordId;
  try {
    repository.transaction((tx: CatalogTransaction) => {
      const current = tx.getRecord(recordId as MixRecord['id']);
      if (!current || current.kind !== 'mix') throw new Error('Mix record disappeared during enrichment');
      const sources = current.sources.some((item) => item.provider === source.provider && item.url === source.url)
        ? current.sources
        : [...current.sources, { ...ref, addedAt: new Date().toISOString() }];
      tx.saveRecord({ ...current, sources, revision: current.revision + 1, updatedAt: new Date().toISOString() }, current.revision);
      tx.addProviderSource(recordId as MixRecord['id'], ref);
    });
    return true;
  } catch {
    return repository.findByProvider(ref) === recordId;
  }
}

function initialReport(record: import('@syco23/catalog-domain').CatalogRecord | undefined): EnrichmentReport {
  return {
    state: 'completed', attemptedProviders: [], applied: 0, corroborated: 0, reviewed: 0, errors: [],
    missingFields: record ? missingFields(record) : []
  };
}

async function enrichEntityRecord(repository: CatalogRepository, registry: ProviderRegistry, record: EntityRecord, actor: CuratorActor): Promise<EnrichmentReport> {
  void actor;
  const report = initialReport(record);
  const claims = [];
  for (const ref of record.providerRefs.filter((item) => item.provider === 'discogs')) {
    report.attemptedProviders.push('discogs');
    try {
      const kind = ref.resourceType === 'label' ? 'label' : record.roles.includes('artist') ? 'artist' : 'crew';
      const profile = await registry.discogs.hydrateEntity(ref.externalId, kind);
      if (!profile) continue;
      const facts: Record<string, unknown> = {};
      if (profile.profile) facts.profile = profile.profile;
      if (profile.imageUrl) {
        const imageField = record.roles.includes('artist') ? 'artistPortrait'
          : record.roles.includes('crew') ? 'crewLogo'
            : record.roles.includes('label') ? 'labelLogo' : undefined;
        const role = record.roles.includes('artist') ? 'artist-portrait'
          : record.roles.includes('crew') ? 'crew-logo' : 'label-logo';
        if (imageField) facts[imageField] = { role, url: profile.imageUrl };
      }
      if (!Object.keys(facts).length) continue;
      const linked = repository.findByProvider(ref) === record.id;
      claims.push(...claimsFromProvider(record, {
        provider: ref,
        sourceUrl: profile.url || ref.url!,
        observedAt: new Date().toISOString(),
        match: {
          status: linked ? 'confirmed' : 'possible',
          explanation: linked ? 'Hydrated by the linked Discogs resource ID' : 'Possible Discogs entity match; linked identity needs confirmation'
        },
        facts
      }));
    } catch (error) {
      report.errors.push({ provider: 'discogs', message: error instanceof Error ? error.message : String(error) });
    }
  }

  const applied = applyClaims(repository, claims);
  return EnrichmentReportSchema.parse({
    ...applied,
    attemptedProviders: [...new Set([...report.attemptedProviders, ...applied.attemptedProviders])],
    errors: [...report.errors, ...applied.errors],
    missingFields: missingFields(repository.getRecord(record.id) ?? record)
  });
}

export async function enrichCatalogRecord(
  repository: CatalogRepository,
  registry: ProviderRegistry,
  id: string,
  actor: CuratorActor
): Promise<EnrichmentReport> {
  const runId = `run_${randomUUID()}`;
  const report = await runTrackedEnrichment(repository, registry, id, actor, runId);
  return report;
}

/**
 * Wraps an enrichment pass with a durable run record. The run is opened before any
 * provider work so a crash leaves an `interrupted` run behind instead of silently
 * dropping the attempt, and provider failures are still committed as a completed run
 * with their errors rather than discarding valid results from the other providers.
 */
async function runTrackedEnrichment(
  repository: CatalogRepository,
  registry: ProviderRegistry,
  id: string,
  actor: CuratorActor,
  runId: string
): Promise<EnrichmentReport> {
  const original = repository.getRecord(id as MixRecord['id']);
  if (!original) throw new Error(`Catalog record ${id} does not exist`);
  repository.startEnrichmentRun({
    id: runId,
    recordId: original.id,
    attemptedProviders: [],
    actor: actor.sessionId,
    startedAt: new Date().toISOString()
  });
  try {
    const report = await performEnrichment(repository, registry, original, actor);
    repository.finishEnrichmentRun(runId, 'completed', report);
    return report;
  } catch (error) {
    repository.finishEnrichmentRun(runId, 'interrupted');
    console.error(`Enrichment run ${runId} for ${id} did not complete:`, error);
    const partial = EnrichmentReportSchema.parse({
      state: 'interrupted',
      attemptedProviders: [],
      applied: 0,
      corroborated: 0,
      reviewed: 0,
      errors: [{ provider: 'freeteknomusic', message: 'Enrichment did not finish; retry the run' }],
      missingFields: missingFields(original)
    });
    return partial;
  }
}

async function performEnrichment(
  repository: CatalogRepository,
  registry: ProviderRegistry,
  original: CatalogRecord,
  actor: CuratorActor
): Promise<EnrichmentReport> {
  const id = original.id;
  if (original.kind === 'entity') return enrichEntityRecord(repository, registry, original, actor);
  if (original.kind !== 'mix') return EnrichmentReportSchema.parse(initialReport(original));

  const linkedArtists = original.people.flatMap((person) => {
    const entity = repository.getRecord(person.entityId);
    return person.role === 'artist' && entity?.kind === 'entity' ? [entity.displayName] : [];
  });
  const linkedCrews = original.people.flatMap((person) => {
    const entity = repository.getRecord(person.entityId);
    return person.role === 'crew' && entity?.kind === 'entity' ? [entity.displayName] : [];
  });
  const result = await enrichMix(registry, {
    title: original.title,
    artists: linkedArtists,
    crews: linkedCrews,
    durationMs: original.durationMs,
    recordedAt: original.recordingDate?.value,
    description: original.description,
    genres: original.genres,
    artwork: original.assets.filter((asset) => asset.role === 'mix-cover').map((asset) => ({ url: asset.url, kind: 'cover' as const })),
    sources: original.sources.flatMap((source) => source.url
      ? [{ provider: source.provider, url: source.url, externalId: source.externalId }]
      : [])
  });

  const confirmedCandidates = result.candidates.filter((candidate) => candidateConfirmsRecording(original, candidate, linkedArtists));
  const confirmedUrls = new Set<string>();
  for (const candidate of confirmedCandidates) {
    if (candidate.provider === 'discogs') continue;
    if (linkProviderSource(repository, original.id, candidate.source)) confirmedUrls.add(`${candidate.source.provider}:${candidate.source.url}`);
  }

  const refreshed = repository.getRecord(original.id);
  if (!refreshed || refreshed.kind !== 'mix') throw new Error('Mix record disappeared during enrichment');
  const factsByProvenance = result.provenance.flatMap((provenance) => {
    let field: string | undefined;
    let value: unknown;
    if (provenance.field === 'durationMs') { field = 'durationMs'; value = result.patch.durationMs; }
    if (provenance.field === 'recordedAt') { field = 'recordingDate'; value = recordingDate(result.patch.recordedAt); }
    if (provenance.field === 'description') { field = 'description'; value = result.patch.description; }
    if (provenance.field === 'genres') { field = 'genres'; value = result.patch.genres; }
    if (provenance.field === 'artwork') {
      const cover = result.patch.artwork?.find((asset) => asset.kind === 'cover');
      if (cover) { field = 'cover'; value = { role: 'mix-cover', url: cover.url }; }
    }
    if (!field || value === undefined || !provenance.sourceUrl) return [];
    const source = [...refreshed.sources, ...result.patch.sources ?? []]
      .find((item) => item.provider === provenance.provider && item.url === provenance.sourceUrl);
    const matchSource = source?.url
      ? { provider: source.provider, url: source.url, externalId: source.externalId }
      : { provider: provenance.provider, url: provenance.sourceUrl };
    const ref = providerRef(matchSource);
    const isConfirmed = repository.findByProvider(ref) === refreshed.id
      || confirmedUrls.has(`${matchSource.provider}:${matchSource.url}`);
    return [{
      provider: ref,
      sourceUrl: provenance.sourceUrl,
      observedAt: new Date().toISOString(),
      match: {
        status: isConfirmed ? 'confirmed' as const : 'possible' as const,
        explanation: isConfirmed
          ? 'Linked provider identity and compatible recording evidence'
          : `Possible match: ${provenance.confidence.toFixed(2)} provider confidence; curator confirmation required`
      },
      facts: { [field]: value }
    }];
  });

  const possibleMatches = result.candidates.flatMap((candidate) => {
    if (confirmedUrls.has(`${candidate.provider}:${candidate.source.url}`)
      || candidate.confidence < 0.55 || overlapScore(candidate.title, original.title) < 0.5) return [];
    const facts: Record<string, unknown> = {};
    if (candidate.durationMs) facts.durationMs = candidate.durationMs;
    if (candidate.description) facts.description = candidate.description;
    if (candidate.genres?.length) facts.genres = candidate.genres;
    if (candidate.artwork?.[0]) facts.cover = { role: 'mix-cover', url: candidate.artwork[0] };
    if (!Object.keys(facts).length) return [];
    return [{
      provider: providerRef(candidate.source),
      sourceUrl: candidate.source.url,
      observedAt: new Date().toISOString(),
      match: { status: 'possible' as const, explanation: `Possible match: ${candidate.reasons.join('; ') || 'title similarity without confirming duration'}` },
      facts
    }];
  });

  const claims = [...factsByProvenance, ...possibleMatches].flatMap((metadata) => claimsFromProvider(refreshed, metadata));
  const applied = applyClaims(repository, claims);
  const errors = [
    ...result.failures.map((failure) => ({ provider: failure.provider, message: failure.error })),
    ...applied.errors
  ];
  const report: EnrichmentReport = {
    ...applied,
    attemptedProviders: [...new Set([...result.attempted, ...applied.attemptedProviders])],
    errors,
    missingFields: missingFields(repository.getRecord(original.id) ?? refreshed)
  };
  return EnrichmentReportSchema.parse(report);
}