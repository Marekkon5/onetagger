import { HttpInputError } from '../http.js';
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
import type { EntityRecord, FieldClaim } from '@syco23/catalog-domain';
import { enrichMix } from '../core/enrichment.js';
import { overlapScore, providerFailureMessage } from '../core/utils.js';
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
  const claims: FieldClaim[] = [];
  for (const ref of record.providerRefs.filter((item) => item.provider === 'discogs')) {
    report.attemptedProviders.push('discogs');
    try {
      const kind = ref.resourceType === 'label' ? 'label' : record.roles.includes('artist') ? 'artist' : 'crew';
      const profile = await registry.discogs.hydrateEntity(ref.externalId, kind);
      if (!profile) continue;
      const facts: Record<string, unknown> = {};
      if (profile.profile) facts.profile = profile.profile;
      if (profile.aliases?.length) facts.aliases = profile.aliases;
      if (profile.realName && record.roles.includes('artist')) facts.realName = profile.realName;
      if (profile.websiteUrls?.length && (record.roles.includes('crew') || record.roles.includes('label'))) facts.websiteUrls = profile.websiteUrls;
      const resolveRefs = (refs: import('../domain.js').EntityRef[] | undefined): string[] => (refs ?? []).flatMap(item => {
        if (!item.externalId || !item.url || item.externalId === ref.externalId) return [];
        const source: ProviderRef = { provider: 'discogs', resourceType: item.kind === 'label' ? 'label' : 'artist', externalId: item.externalId, url: item.url };
        const known = repository.findByProvider(source);
        if (known) return [known];
        const now = new Date().toISOString();
        const related: EntityRecord = { kind: 'entity', id: randomUUID(), displayName: item.name,
          roles: [item.kind === 'label' ? 'label' : 'artist'], verification: 'source-confirmed', reviewState: 'ready',
          revision: 1, createdAt: now, updatedAt: now, aliases: [], assets: [], providerRefs: [source] };
        repository.transaction(tx => { tx.saveRecord(related); tx.addProviderSource(related.id, source); });
        return [related.id];
      });
      const groups = resolveRefs(profile.groups), members = resolveRefs(profile.members), subLabels = resolveRefs(profile.subLabels);
      const parent = resolveRefs(profile.parent ? [profile.parent] : []);
      if (groups.length && record.roles.includes('artist')) facts.groupIds = groups;
      if (members.length && (record.roles.includes('artist') || record.roles.includes('crew'))) facts.memberIds = members;
      if (subLabels.length && record.roles.includes('label')) facts.subLabelIds = subLabels;
      if (parent[0] && record.roles.includes('label')) facts.parentId = parent[0];
      // Discogs alias identities remain separate sourced identities; their names are
      // attributable aliases, never a name-only merge of independent catalog IDs.
      if (profile.aliasRefs?.length) facts.aliases = [...new Set([...(profile.aliases ?? []), ...profile.aliasRefs.map(item => item.name)])];
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
      report.errors.push({ provider: 'discogs', message: providerFailureMessage(error) });
    }
  }

  if (!record.providerRefs.some(ref => ref.provider === 'discogs')) {
    report.attemptedProviders.push('discogs');
    try {
      const kind = record.roles.includes('label') ? 'label' : record.roles.includes('artist') ? 'artist' : 'crew';
      const matches = await registry.discogs.enrichEntity(record.displayName, kind);
      for (const match of matches.slice(0, 5)) {
        if (!match.externalId || !match.url) continue;
        const ref: ProviderRef = { provider: 'discogs', resourceType: kind === 'label' ? 'label' : 'artist', externalId: match.externalId, url: match.url };
        const facts: Record<string, unknown> = { providerRefs: [...record.providerRefs, ref] };
        if (match.profile) facts.profile = match.profile;
        claims.push(...claimsFromProvider(record, { provider: ref, sourceUrl: match.url,
          observedAt: new Date().toISOString(), match: { status: 'possible', explanation: 'Discogs name search proposes identity; curator confirmation required' }, facts }));
      }
      if (!matches.length) report.errors.push({ provider: 'discogs', message: 'No supported entity identity found; fields remain missing' });
    } catch { report.errors.push({ provider: 'discogs', message: 'Discogs entity search failed; retry later' }); }
  }
  const applied = applyClaims(repository, claims);
  return EnrichmentReportSchema.parse({
    ...applied,
    attemptedProviders: [...new Set([...report.attemptedProviders, ...applied.attemptedProviders])],
    errors: [...report.errors, ...applied.errors],
    missingFields: missingFields(repository.getRecord(record.id) ?? record)
  });
}

const activeEnrichments = new WeakMap<CatalogRepository, Set<string>>();

export async function enrichCatalogRecord(
  repository: CatalogRepository,
  registry: ProviderRegistry,
  id: string,
  actor: CuratorActor
): Promise<EnrichmentReport> {
  const active = activeEnrichments.get(repository) ?? new Set<string>();
  if (active.has(id) || active.size >= 3) throw new HttpInputError('Enrichment is already running; retry when it finishes', 429);
  activeEnrichments.set(repository, active); active.add(id);
  try { return await runTrackedEnrichment(repository, registry, id, actor, `run_${randomUUID()}`); }
  finally { active.delete(id); }
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
    console.error(`Enrichment run ${runId} for ${id} did not complete`);
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
  if (original.kind !== 'mix') return EnrichmentReportSchema.parse({ ...initialReport(original),
    errors: [{ provider: 'freeteknomusic', message: 'No supported event provider capability; curator-sourced event facts are required' }] });
  const entityReports: EnrichmentReport[] = [];
  for (const entityId of new Set(original.people.map(person => person.entityId))) {
    const entity = repository.getRecord(entityId);
    if (entity?.kind === 'entity') {
      entityReports.push(await enrichEntityRecord(repository, registry, entity, actor));
    }
  }

  const linkedArtists = original.people.flatMap((person) => {
    const entity = repository.getRecord(person.entityId);
    return person.role === 'artist' && entity?.kind === 'entity' ? [entity.displayName] : [];
  });
  const linkedCrews = original.people.flatMap((person) => {
    const entity = repository.getRecord(person.entityId);
    return person.role === 'crew' && entity?.kind === 'entity' ? [entity.displayName] : [];
  });
  const sourceReports: EnrichmentReport[] = [];
  const sourceErrors: EnrichmentReport['errors'] = [];
  const refreshedCandidates: import('../domain.js').MixCandidate[] = [];
  for (const ref of original.sources) {
    if (!ref.url || ref.provider === 'discogs' || ref.provider === 'freeteknomusic') continue;
    const adapter = registry.discovery.get(ref.provider);
    if (!adapter) continue;
    try {
      const candidates = await adapter.search({ url: ref.url, minDurationMs: 1, limit: 1 });
      for (const candidate of candidates) {
        if (candidate.source.externalId && candidate.source.externalId !== ref.externalId) continue;
        refreshedCandidates.push(candidate);
        const facts: Record<string, unknown> = {};
        if (candidate.durationMs) facts.durationMs = candidate.durationMs;
        if (candidate.description) facts.description = candidate.description;
        if (candidate.genres?.length) facts.genres = candidate.genres;
        if (candidate.recordedAt) { const date = recordingDate(candidate.recordedAt); if (date) facts.recordingDate = date; }
        if (candidate.artwork?.[0]) facts.cover = { role: 'mix-cover', url: candidate.artwork[0] };
        const current = repository.getRecord(original.id)!;
        sourceReports.push(applyClaims(repository, claimsFromProvider(current, { provider: { provider: ref.provider, resourceType: ref.resourceType, externalId: ref.externalId, url: ref.url }, sourceUrl: ref.url,
          observedAt: new Date().toISOString(), match: { status: 'confirmed', explanation: 'Refreshed established provider resource identity directly' },
          fieldEvidence: candidate.fieldEvidence, facts })));
      }
    } catch (error) { sourceErrors.push({ provider: ref.provider, message: providerFailureMessage(error) }); }
  }
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
      fieldEvidence: provenance.field === 'recordedAt'
        ? { recordingDate: result.candidates.find(candidate => candidate.source.url === provenance.sourceUrl)?.fieldEvidence?.recordingDate ?? 'direct' }
        : undefined,
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
  claims.push(...parsedDescriptionClaims(repository, refreshed, [...refreshedCandidates, ...result.candidates]));
  const applied = applyClaims(repository, claims);
  const errors = [
    ...result.failures.map((failure) => ({ provider: failure.provider, message: failure.error })),
    ...entityReports.flatMap(report => report.errors),
    ...sourceErrors, ...sourceReports.flatMap(report => report.errors),
    ...applied.errors
  ];
  const report: EnrichmentReport = {
    ...applied,
    applied: applied.applied + [...entityReports, ...sourceReports].reduce((count, report) => count + report.applied, 0),
    corroborated: applied.corroborated + [...entityReports, ...sourceReports].reduce((count, report) => count + report.corroborated, 0),
    reviewed: applied.reviewed + [...entityReports, ...sourceReports].reduce((count, report) => count + report.reviewed, 0),
    attemptedProviders: [...new Set([...result.attempted, ...applied.attemptedProviders, ...[...entityReports, ...sourceReports].flatMap(report => report.attemptedProviders)])],
    errors,
    missingFields: missingFields(repository.getRecord(original.id) ?? refreshed)
  };
  return EnrichmentReportSchema.parse(report);
}
function parsedDescriptionClaims(repository: CatalogRepository, mix: MixRecord, candidates: import('../domain.js').MixCandidate[]): FieldClaim[] {
  const claims: FieldClaim[] = [];
  for (const candidate of candidates) {
    const description = candidate.description?.slice(0,10000);
    if (!description) continue;
    const lines = description.split(/[\r\n]+/);
    const facts = new Map<string, {value:string;excerpt:string}>();
    for (const excerpt of lines) { const match = /^\s*(Event|Venue|Country|Crew)\s*:\s*(.{1,300})\s*$/i.exec(excerpt);
      if (match) facts.set(match[1].toLowerCase(), {value:match[2].trim(),excerpt:excerpt.trim()}); }
    const ref = providerRef(candidate.source), observedAt = new Date().toISOString();
    const eventName = facts.get('event');
    if (eventName) {
      const eventId = `event_${createHash('sha256').update(mix.id + ':' + eventName.value.normalize('NFKC').toLowerCase()).digest('hex').slice(0,40)}`;
      if (!repository.getRecord(eventId)) repository.transaction(tx => tx.saveRecord({ kind:'event',id:eventId,name:eventName.value,
        revision:1,createdAt:observedAt,updatedAt:observedAt,verification:'proposed',reviewState:'review',assets:[],sourceUrls:[],mixIds:[] }));
      const event = repository.getRecord(eventId)!;
      for (const [input,field] of [['event','name'],['venue','venue'],['country','country']] as const) {
        const fact=facts.get(input); if (!fact || (field==='country' && !/^[A-Z]{2}$/.test(fact.value))) continue;
        claims.push(...claimsFromProvider(event,{provider:ref,sourceUrl:candidate.source.url,observedAt,
          match:{status:'possible',explanation:`Parsed description excerpt: "${fact.excerpt}"`},facts:{[field]:fact.value}}));
      }
      claims.push(...claimsFromProvider(mix,{provider:ref,sourceUrl:candidate.source.url,observedAt,
        match:{status:'possible',explanation:`Proposed event relationship from excerpt: "${eventName.excerpt}"`},facts:{eventIds:[...new Set([...mix.eventIds,eventId])]}}));
    }
    const crew=facts.get('crew');
    if (crew) {
      const entityId=`entity_${createHash('sha256').update(mix.id+':crew:'+crew.value.normalize('NFKC').toLowerCase()).digest('hex').slice(0,40)}`;
      if (!repository.getRecord(entityId)) repository.transaction(tx=>tx.saveRecord({kind:'entity',id:entityId,displayName:crew.value,roles:['crew'],aliases:[],assets:[],providerRefs:[],revision:1,createdAt:observedAt,updatedAt:observedAt,verification:'proposed',reviewState:'review'}));
      claims.push(...claimsFromProvider(repository.getRecord(entityId)!,{provider:ref,sourceUrl:candidate.source.url,observedAt,
        match:{status:'possible',explanation:`Parsed description excerpt: "${crew.excerpt}"`},facts:{displayName:crew.value}}));
      claims.push(...claimsFromProvider(mix,{provider:ref,sourceUrl:candidate.source.url,observedAt,
        match:{status:'possible',explanation:`Proposed crew relationship from excerpt: "${crew.excerpt}"`},facts:{people:[...mix.people,{entityId,role:'crew'}]}}));
    }
  }
  return claims;
}
