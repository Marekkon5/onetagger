import { createHash, randomUUID } from 'node:crypto';
import {
  LegacyMixSchema,
  MigrationResultSchema,
  ProviderIdSchema,
  RecordingDateSchema,
  normalizeName,
  type CatalogRecord,
  type EntityRecord,
  type LegacyMix,
  type MigrationResult,
  type MixRecord,
  type ProviderRef,
  type RecordId
} from '@syco23/catalog-domain';
import { persistWaveform } from './media.js';
import type { CatalogRepository } from './repository.js';
import type { CuratorActor } from '../auth/curator.js';

const demoIds = new Set(['spiral-warehouse-2001', 'metek-aniane-98', 'novabase-live-tech', 'gotek-roma-2007']);

function stableId(prefix: string, value: string): RecordId {
  return `${prefix}_${createHash('sha256').update(value).digest('hex').slice(0, 32)}`;
}

function parsedRecordingDate(value: string | undefined) {
  if (!value) return undefined;
  const normalized = value.trim();
  const precision = normalized.length === 4 ? 'year' : normalized.length === 7 ? 'month' : normalized.length === 10 ? 'day' : undefined;
  if (!precision) return undefined;
  return RecordingDateSchema.safeParse({ value: normalized, precision }).data;
}

function sourceRefs(record: LegacyMix): ProviderRef[] {
  const refs: ProviderRef[] = [];
  for (const source of record.sources ?? []) {
    if (!source || typeof source !== 'object') continue;
    const value = source as Record<string, unknown>;
    const provider = value.provider;
    const urlValue = value.url;
    if (typeof provider !== 'string' || typeof urlValue !== 'string') continue;
    const parsedProvider = ProviderIdSchema.safeParse(provider);
    if (!parsedProvider.success) continue;
    try {
      const url = new URL(urlValue);
      if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) continue;
      refs.push({
        provider: parsedProvider.data,
        resourceType: typeof value.resourceType === 'string' ? value.resourceType : 'recording',
        externalId: typeof value.externalId === 'string' && value.externalId ? value.externalId : createHash('sha256').update(url.toString()).digest('hex'),
        url: url.toString()
      });
    } catch {
      continue;
    }
  }
  return refs;
}

function linkedEntity(record: LegacyMix, role: 'artist' | 'crew' | 'label', name: string, entityId: RecordId, timestamp: string): EntityRecord {
  const legacyEntities = Array.isArray(record.entities) ? record.entities.filter((entity): entity is Record<string, unknown> => Boolean(entity && typeof entity === 'object')) : [];
  const explicit = legacyEntities.find((entity) => entity.kind === role && typeof entity.name === 'string' && normalizeName(entity.name) === normalizeName(name));
  const provider = typeof explicit?.provider === 'string' ? explicit.provider as ProviderRef['provider'] : undefined;
  const externalId = typeof explicit?.externalId === 'string' ? explicit.externalId : undefined;
  const url = typeof explicit?.url === 'string' ? explicit.url : undefined;
  const parsedProvider = provider ? ProviderIdSchema.safeParse(provider) : undefined;
  const source = parsedProvider?.success && externalId
    ? [{ provider: parsedProvider.data, resourceType: role === 'label' ? 'label' : 'artist', externalId, ...(url ? { url } : {}) }]
    : [];
  const imageUrl = typeof explicit?.imageUrl === 'string' ? explicit.imageUrl : undefined;
  const assetRole = role === 'artist' ? 'artist-portrait' : role === 'crew' ? 'crew-logo' : 'label-logo';
  const asset = imageUrl ? (() => {
    try {
      const parsed = new URL(imageUrl);
      const imageSource: ProviderRef['provider'] | 'local' = parsedProvider?.success ? parsedProvider.data : 'local';
      return ['http:', 'https:'].includes(parsed.protocol) ? [{ role: assetRole as 'artist-portrait' | 'crew-logo' | 'label-logo', url: parsed.toString(), source: imageSource }] : [];
    } catch { return []; }
  })() : [];
  return {
    kind: 'entity', id: entityId, createdAt: timestamp, updatedAt: timestamp, revision: 1,
    verification: source.length ? 'source-confirmed' : 'proposed', reviewState: 'ready', displayName: name,
    roles: [role], aliases: [], assets: asset,
    providerRefs: source,
    ...(typeof explicit?.profile === 'string' ? { profile: explicit.profile.slice(0, 20000) } : {}),
    ...(role === 'artist' ? { artist: {} } : {}),
    ...(role === 'crew' ? { crew: {} } : {}),
    ...(role === 'label' ? { label: {} } : {})
  };
}

function legacyCover(record: LegacyMix): MixRecord['assets'][number] | undefined {
  const artwork = Array.isArray(record.artwork) ? record.artwork : [];
  for (const entry of artwork) {
    if (!entry || typeof entry !== 'object') continue;
    const value = entry as Record<string, unknown>;
    if (value.kind !== 'cover' || typeof value.url !== 'string') continue;
    try {
      const url = new URL(value.url);
      if (!['http:', 'https:'].includes(url.protocol)) continue;
      const provider = value.source;
      const parsedProvider = typeof provider === 'string' ? ProviderIdSchema.safeParse(provider) : undefined;
      const source = parsedProvider?.success ? parsedProvider.data : 'local';
      return { role: 'mix-cover', url: url.toString(), source };
    } catch {
      continue;
    }
  }
  return undefined;
}

function createMix(record: LegacyMix, id: RecordId, timestamp: string, refs: ProviderRef[], entityLinks: Array<{ entityId: RecordId; role: 'artist' | 'crew' | 'label' }>, eventIds: RecordId[]): MixRecord {
  const cover = legacyCover(record);
  const candidates = Array.isArray(record.candidates) ? record.candidates : [];
  const assets = cover ? [cover] : [];
  return {
    kind: 'mix', id,
    createdAt: typeof record.createdAt === 'string' ? record.createdAt : timestamp,
    updatedAt: timestamp,
    revision: 1,
    verification: 'curator-confirmed',
    reviewState: candidates.some((candidate) => candidate && typeof candidate === 'object' && (candidate as Record<string, unknown>).state === 'pending') ? 'review' : 'ready',
    title: record.title,
    ...(parsedRecordingDate(typeof record.recordedAt === 'string' ? record.recordedAt : undefined) ? { recordingDate: parsedRecordingDate(record.recordedAt as string) } : {}),
    ...(typeof record.durationMs === 'number' ? { durationMs: record.durationMs } : {}),
    ...(typeof record.description === 'string' ? { description: record.description.slice(0, 10000) } : {}),
    people: entityLinks,
    eventIds,
    genres: Array.isArray(record.genres) ? record.genres.filter((value): value is string => typeof value === 'string').slice(0, 100) : [],
    styles: Array.isArray(record.styles) ? record.styles.filter((value): value is string => typeof value === 'string').slice(0, 100) : [],
    assets,
    sources: refs.map((ref) => ({ ...ref, addedAt: timestamp }))
  };
}

export function migrateLegacyLibrary(repository: CatalogRepository, input: unknown[], batchId: string, actor: CuratorActor): MigrationResult {
  void actor;
  const prior = repository.getMigrationBatch(batchId);
  if (prior) return prior;
  if (!batchId || batchId.length > 200 || input.length > 1000) throw new Error('Migration batch is outside supported bounds');

  const result: MigrationResult = { batchId, imported: 0, existing: 0, rejected: 0, legacyIds: {} };
  for (const candidate of input) {
    const parsed = LegacyMixSchema.safeParse(candidate);
    if (!parsed.success || demoIds.has(parsed.success ? parsed.data.id : String((candidate as { id?: unknown })?.id ?? ''))) {
      result.rejected += 1;
      continue;
    }
    const legacy = parsed.data;
    const refs = sourceRefs(legacy);
    try {
      const existingId = refs.map((ref) => repository.findBySource(ref) ?? repository.findByProvider(ref)).find(Boolean);
      const existing = existingId ? repository.getRecord(existingId) : undefined;
      if (existing) {
        repository.transaction((tx) => tx.addLegacyAlias(legacy.id, existing.id));
        result.legacyIds[legacy.id] = existing.id;
        result.existing += 1;
        continue;
      }

      const mixId = stableId('mix_mig', legacy.id);
      const timestamp = new Date().toISOString();
      const legacyEventName = typeof legacy.event === 'string' ? legacy.event.trim() : '';
      const eventId = legacyEventName ? stableId('event_mig', `${legacy.id}:event:${normalizeName(legacyEventName)}`) : undefined;
      const event = eventId ? {
        kind: 'event' as const,
        id: eventId,
        createdAt: timestamp,
        updatedAt: timestamp,
        revision: 1,
        verification: 'proposed' as const,
        reviewState: 'review' as const,
        name: legacyEventName,
        assets: [],
        sourceUrls: refs.flatMap((ref) => ref.url ? [ref.url] : []),
        mixIds: [mixId]
      } : undefined;
      const entityLinks: Array<{ entityId: RecordId; role: 'artist' | 'crew' | 'label' }> = [];
      const entities: EntityRecord[] = [];
      for (const [role, names] of [
        ['artist', legacy.artists ?? []],
        ['crew', legacy.crews ?? []],
        ['label', Array.isArray(legacy.entities) ? legacy.entities.flatMap((entity) => {
          if (!entity || typeof entity !== 'object') return [];
          const value = entity as Record<string, unknown>;
          return value.kind === 'label' && typeof value.name === 'string' ? [value.name] : [];
        }) : []]
      ] as const) {
        for (const [index, name] of names.entries()) {
          if (typeof name !== 'string' || !name.trim()) continue;
          const entityId = stableId('entity_mig', `${legacy.id}:${role}:${index}:${normalizeName(name)}`);
          entities.push(linkedEntity(legacy, role, name.trim(), entityId, timestamp));
          entityLinks.push({ entityId, role });
        }
      }

      const mix = createMix(legacy, mixId, timestamp, refs, entityLinks, eventId ? [eventId] : []);
      repository.transaction((tx) => {
        tx.saveRecord(mix);
        if (event) tx.saveRecord(event);
        for (const entity of entities) {
          tx.saveRecord(entity);
          for (const ref of entity.providerRefs) tx.addProviderSource(entity.id, ref);
        }
        for (const ref of refs) tx.addProviderSource(mixId, ref);
        for (const relation of entityLinks) tx.addRelationship(mixId, relation.entityId, relation.role);
        if (event) {
          tx.addRelationship(mixId, event.id, 'event');
          const source = refs[0];
          if (source?.url) {
            const claimId = stableId('claim_mig', `${legacy.id}:event:${normalizeName(legacyEventName)}`);
            tx.addClaim(claimId, `migration:${batchId}:${legacy.id}:event`, {
              targetRecordId: event.id,
              field: 'name',
              value: legacyEventName,
              provider: source,
              sourceUrl: source.url,
              observedAt: timestamp,
              evidence: 'parsed',
              matchExplanation: 'Legacy event text has no event-specific date or venue evidence; confirm the event identity'
            }, 'pending', event.revision);
          }
        }
        tx.addLegacyAlias(legacy.id, mixId);
      });

      if (legacy.waveform && typeof legacy.waveform === 'object') {
        const waveform = legacy.waveform as Record<string, unknown>;
        const dataUrl = typeof waveform.imageDataUrl === 'string' ? waveform.imageDataUrl : '';
        const match = /^data:image\/png;base64,([A-Za-z0-9+/=]+)$/.exec(dataUrl);
        if (match && typeof waveform.sourceUrl === 'string') {
          try { persistWaveform(repository, mixId, Buffer.from(match[1], 'base64'), waveform.sourceUrl); }
          catch { }
        }
      }
      result.legacyIds[legacy.id] = mixId;
      result.imported += 1;
    } catch {
      result.rejected += 1;
    }
  }

  return repository.saveMigrationBatch(MigrationResultSchema.parse(result));
}