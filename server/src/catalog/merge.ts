import { createHash } from 'node:crypto';
import {
  EnrichmentReportSchema,
  FieldClaimSchema,
  missingFields,
  validateField,
  type CatalogDetail,
  type CatalogRecord,
  type EnrichmentReport,
  type EntityRecord,
  type FieldClaim,
  type ProviderRef,
  type RecordId,
  type ReviewItem
} from '@syco23/catalog-domain';
import { normalizeName, normalizeProviderRef } from '@syco23/catalog-domain';
import type { CatalogRepository, CatalogTransaction } from './repository.js';

function stableJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.entries(value).sort(([left], [right]) => left.localeCompare(right)).map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export function claimFingerprint(claimInput: FieldClaim): string {
  const claim = FieldClaimSchema.parse(claimInput);
  const identity = {
    targetRecordId: claim.targetRecordId,
    field: claim.field,
    value: claim.value,
    provider: normalizeProviderRef(claim.provider),
    sourceUrl: claim.sourceUrl,
    evidence: claim.evidence
  };
  return `claim_${createHash('sha256').update(stableJson(identity)).digest('hex').slice(0, 56)}`;
}

function valueFor(record: CatalogRecord, field: string): unknown {
  if (record.kind === 'mix') {
    if (field === 'cover') {
      const asset = record.assets.find((item) => item.role === 'mix-cover');
      return asset ? { role: asset.role, url: asset.url } : undefined;
    }
    const value = record[field as keyof typeof record];
    if ((field === 'genres' || field === 'styles') && Array.isArray(value) && value.length === 0) return undefined;
    if (typeof value === 'string' && value.trim() === '') return undefined;
    return value;
  }
  if (record.kind === 'entity') {
    const assetRole = field === 'artistPortrait' ? 'artist-portrait'
      : field === 'crewLogo' ? 'crew-logo'
        : field === 'labelLogo' ? 'label-logo' : undefined;
    if (assetRole) {
      const asset = record.assets.find((item) => item.role === assetRole);
      return asset ? { role: asset.role, url: asset.url } : undefined;
    }
    const value = record[field as keyof typeof record];
    if (typeof value === 'string' && value.trim() === '') return undefined;
    return value;
  }
  if (field === 'eventDate') return record.startDate;
  if (field === 'flyer') {
    const asset = record.assets.find((item) => item.role === 'event-flyer');
    return asset ? { role: asset.role, url: asset.url } : undefined;
  }
  return record[field as keyof typeof record];
}

function comparisonValue(value: unknown): string {
  if (typeof value === 'string') return normalizeName(value);
  if (Array.isArray(value)) return stableJson([...value].map((item) => typeof item === 'string' ? normalizeName(item) : item).sort());
  return stableJson(value);
}

function withSelectedField(record: CatalogRecord, field: string, value: unknown, evidenceId: RecordId, claim: FieldClaim): CatalogRecord {
  const selectedEvidence = { ...record.selectedEvidence, [field]: evidenceId };
  const base = { ...record, revision: record.revision + 1, updatedAt: new Date().toISOString(), selectedEvidence };

  if (field === 'cover' && record.kind === 'mix') {
    const cover = value as { role: 'mix-cover'; url: string };
    return { ...base, assets: [...record.assets.filter((asset) => asset.role !== 'mix-cover'), { ...cover, source: claim.provider.provider }] };
  }
  if (field === 'flyer' && record.kind === 'event') {
    const flyer = value as { role: 'event-flyer'; url: string };
    return { ...base, assets: [...record.assets.filter((asset) => asset.role !== 'event-flyer'), { ...flyer, source: claim.provider.provider }] };
  }
  if (record.kind === 'entity' && ['artistPortrait', 'crewLogo', 'labelLogo'].includes(field)) {
    const asset = value as { role: 'artist-portrait' | 'crew-logo' | 'label-logo'; url: string };
    return { ...base, assets: [...record.assets.filter((item) => item.role !== asset.role), { ...asset, source: claim.provider.provider }] };
  }
  if (field === 'eventDate' && record.kind === 'event') {
    return {
      ...record,
      revision: record.revision + 1,
      updatedAt: new Date().toISOString(),
      selectedEvidence,
      startDate: value as typeof record.startDate
    };
  }
  return { ...base, [field]: value } as CatalogRecord;
}

/**
 * A citable provider identity for a record, used when a merge has to attribute a
 * conflict. A record without a provider identity (for example a curator-created
 * one) is attributed to the archive itself rather than to no one.
 */
function duplicateFallbackRef(record: CatalogRecord): ProviderRef {
  const ref = record.kind === 'mix' ? record.sources[0] : record.kind === 'entity' ? record.providerRefs[0] : undefined;
  if (ref) {
    const normalized = normalizeProviderRef(ref);
    return normalized.url ? normalized : { ...normalized, url: 'https://mixsets.syco23.org/' };
  }
  return { provider: 'archiveorg', resourceType: record.kind, externalId: record.id, url: 'https://mixsets.syco23.org/' };
}

type ClaimOutcome = 'selected' | 'corroborated' | 'pending';

function applyOne(tx: CatalogTransaction, claim: FieldClaim): ClaimOutcome | 'duplicate' {
  const fingerprint = claimFingerprint(claim);
  if (tx.getClaim(fingerprint)) return 'duplicate';
  const record = tx.getRecord(claim.targetRecordId);
  if (!record) throw new Error(`Target record ${claim.targetRecordId} does not exist`);
  const value = validateField(record, claim.field, claim.value);
  const currentValue = valueFor(record, claim.field);
  const id = fingerprint as RecordId;

  if (currentValue !== undefined && comparisonValue(currentValue) === comparisonValue(value)) {
    tx.addClaim(id, fingerprint, claim, 'corroborated', record.revision);
    return 'corroborated';
  }

  const sourceIsLinked = tx.findByProvider(claim.provider) === record.id;
  const canSelect = currentValue === undefined && record.verification !== 'proposed'
    && (claim.evidence === 'curated' || (claim.evidence === 'direct' && sourceIsLinked));
  if (canSelect) {
    tx.saveRecord(withSelectedField(record, claim.field, value, id, claim), record.revision);
    tx.addClaim(id, fingerprint, claim, 'selected', record.revision + 1);
    return 'selected';
  }

  tx.addClaim(id, fingerprint, claim, 'pending', record.revision, currentValue);
  return 'pending';
}

export function applyClaims(repository: CatalogRepository, claimInputs: FieldClaim[]): EnrichmentReport {
  const claims = claimInputs.map((claim) => FieldClaimSchema.parse(claim));
  const report: EnrichmentReport = {
    state: 'completed',
    attemptedProviders: [...new Set(claims.map((claim) => claim.provider.provider))],
    applied: 0,
    corroborated: 0,
    reviewed: 0,
    errors: [],
    missingFields: []
  };
  const touchedRecords = new Set<RecordId>();

  for (const claim of claims) {
    try {
      const outcome = repository.transaction((tx) => applyOne(tx, claim));
      touchedRecords.add(claim.targetRecordId);
      if (outcome === 'selected') report.applied += 1;
      if (outcome === 'corroborated') report.corroborated += 1;
      if (outcome === 'pending') report.reviewed += 1;
    } catch (error) {
      report.errors.push({ provider: claim.provider.provider, message: error instanceof Error ? error.message : String(error) });
    }
  }

  report.missingFields = [...new Set([...touchedRecords].flatMap((id) => {
    const record = repository.getRecord(id);
    return record ? missingFields(record) : [];
  }))];
  return EnrichmentReportSchema.parse(report);
}

export function decideReview(
  repository: CatalogRepository,
  id: RecordId,
  decision: 'accept' | 'reject',
  expectedRevision: number,
  actor: string
): ReviewItem {
  repository.transaction((tx) => {
    const item = tx.getReview(id);
    if (!item) throw new Error(`Review item ${id} does not exist`);
    const record = tx.getRecord(item.targetRecordId);
    if (!record) throw new Error(`Target record ${item.targetRecordId} does not exist`);
    if (record.revision !== expectedRevision || item.recordRevision !== expectedRevision) {
      throw new Error(`Revision conflict for ${item.targetRecordId}`);
    }

    if (decision === 'accept') {
      const value = validateField(record, item.field, item.claim.value);
      let updated = withSelectedField(record, item.field, value, item.id, item.claim);
      if (record.kind === 'event' && record.verification === 'proposed' && item.field === 'name') {
        updated = { ...updated, verification: 'curator-confirmed', reviewState: 'ready' };
      }
      tx.saveRecord(updated, expectedRevision);
      tx.decideReview(id, 'accepted', actor);
    } else {
      tx.decideReview(id, 'rejected', actor);
    }
  });
  const decided = repository.getReview(id);
  if (!decided) throw new Error(`Review item ${id} could not be reloaded`);
  return decided;
}

export function refreshReview(repository: CatalogRepository, id: RecordId): ReviewItem {
  repository.transaction((tx) => {
    const item = tx.getReview(id);
    if (!item) throw new Error(`Review item ${id} does not exist`);
    const record = tx.getRecord(item.targetRecordId);
    if (!record) throw new Error(`Target record ${item.targetRecordId} does not exist`);
    tx.refreshReview(id, record.revision, valueFor(record, item.field));
  });
  const refreshed = repository.getReview(id);
  if (!refreshed) throw new Error(`Review item ${id} could not be reloaded`);
  return refreshed;
}

/**
 * Mergeable fields per record kind. Only these are unioned or conflict-checked on a
 * merge; identities, revisions and evidence bookkeeping are handled separately.
 */
const MERGE_FIELDS = {
  mix: ['genres', 'styles', 'playbackUrls', 'assets', 'sources', 'eventIds'],
  entity: ['aliases', 'roles', 'assets', 'providerRefs'],
  event: ['sourceUrls', 'mixIds', 'assets']
} as const satisfies Record<CatalogRecord['kind'], readonly string[]>;

/**
 * Fields that carry a single chosen value. When the survivor and the duplicate both
 * have one and they disagree, the merge must not silently pick a winner: the
 * duplicate's value becomes a Review item against the survivor.
 */
const SINGLE_VALUE_FIELDS = {
  mix: ['title', 'description', 'durationMs', 'recordingDate'],
  entity: ['displayName', 'profile', 'country'],
  event: ['name', 'startDate', 'endDate', 'venue', 'locality', 'country']
} as const satisfies Record<CatalogRecord['kind'], readonly string[]>;

function unionByJson(left: unknown[], right: unknown[]): unknown[] {
  const seen = new Set<string>();
  const merged: unknown[] = [];
  for (const value of [...left, ...right]) {
    const key = comparisonValue(value);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(value);
  }
  return merged;
}

function conflictClaim(
  survivor: CatalogRecord,
  field: string,
  value: unknown,
  ref: ProviderRef,
  sourceUrl: string,
  explanation: string,
  observedAt: string
): FieldClaim {
  return {
    targetRecordId: survivor.id,
    field,
    value,
    provider: ref,
    sourceUrl,
    observedAt,
    evidence: 'curated',
    matchExplanation: explanation
  };
}

/**
 * Curator-confirmed duplicate merge.
 *
 * The survivor keeps its own selected evidence and receives the duplicate's source
 * identities, relationships, aliases and missing fields. Where both records hold a
 * different value for the same single-valued field the merge stops selecting and
 * instead raises a Review item holding the duplicate's value, so a disagreement
 * survives the merge instead of being resolved by write order.
 *
 * Old IDs redirect to the survivor through the legacy alias table, so detail links
 * that were already shared keep resolving. Both revisions are checked so a second
 * curator session cannot merge over newer curation.
 */
export function mergeRecords(
  repository: CatalogRepository,
  survivorId: RecordId,
  duplicateId: RecordId,
  expectedRevisions: readonly [number, number],
  actor: string
): CatalogDetail {
  if (survivorId === duplicateId) throw new Error('Cannot merge a record with itself');
  repository.transaction((tx) => {
    const survivor = tx.getRecord(survivorId);
    const duplicate = tx.getRecord(duplicateId);
    if (!survivor) throw new Error(`Survivor record ${survivorId} does not exist`);
    if (!duplicate) throw new Error(`Duplicate record ${duplicateId} does not exist`);
    if (survivor.kind !== duplicate.kind) throw new Error('Cannot merge records of different kinds');
    if (survivor.revision !== expectedRevisions[0] || duplicate.revision !== expectedRevisions[1]) {
      throw new Error(`Revision conflict for ${survivorId}`);
    }

    const observedAt = new Date().toISOString();
    let merged = survivor;
    const reviewFields: Array<{ field: string; value: unknown; ref: ProviderRef }> = [];

    for (const field of MERGE_FIELDS[survivor.kind] as readonly string[]) {
      const own = (survivor as unknown as Record<string, unknown>)[field];
      const theirs = (duplicate as unknown as Record<string, unknown>)[field];
      if (!Array.isArray(own) || !Array.isArray(theirs) || !theirs.length) continue;
      merged = { ...merged, [field]: unionByJson(own as unknown[], theirs as unknown[]) } as CatalogRecord;
    }

    for (const field of SINGLE_VALUE_FIELDS[survivor.kind] as readonly string[]) {
      const theirs = (duplicate as unknown as Record<string, unknown>)[field];
      if (theirs === undefined || theirs === null) continue;
      const own = (survivor as unknown as Record<string, unknown>)[field];
      if (own === undefined || own === null) {
        merged = { ...merged, [field]: theirs } as CatalogRecord;
        continue;
      }
      if (comparisonValue(own) === comparisonValue(theirs)) continue;
      // Keep the survivor's value selected and carry the disagreement into Review.
      reviewFields.push({ field, value: theirs, ref: duplicateFallbackRef(duplicate) });
    }

    const roleUnion = survivor.kind === 'entity' && duplicate.kind === 'entity'
      ? unionByJson(survivor.roles, duplicate.roles)
      : undefined;

    const updated: CatalogRecord = {
      ...merged,
      ...(roleUnion ? { roles: roleUnion as EntityRecord['roles'] } : {}),
      revision: survivor.revision + 1,
      updatedAt: observedAt,
      // A curator merge is an explicit confirmation of the surviving identity.
      verification: survivor.verification === 'proposed' ? 'curator-confirmed' : survivor.verification
    } as CatalogRecord;
    tx.saveRecord(updated, survivor.revision);

    const duplicateRef = duplicateFallbackRef(duplicate);
    for (const ref of duplicate.kind === 'mix' ? duplicate.sources : duplicate.kind === 'entity' ? duplicate.providerRefs : []) {
      if (tx.findByProvider(ref) === survivorId) continue;
      tx.addProviderSource(survivorId, ref);
    }

    for (const item of reviewFields) {
      const claim = conflictClaim(
        updated,
        item.field,
        item.value,
        item.ref,
        item.ref.url ?? 'https://mixsets.syco23.org/',
        `Merged duplicate ${duplicateId} held a different ${item.field}; the survivor's selected value is unchanged`,
        observedAt
      );
      const fingerprint = claimFingerprint(claim);
      if (tx.getClaim(fingerprint)) continue;
      validateField(updated, item.field, item.value);
      tx.addClaim(fingerprint as RecordId, fingerprint, claim, 'pending', updated.revision, valueFor(survivor, item.field));
    }

    void duplicateRef;
    // The duplicate is retired inside the same transaction: its claims, review
    // decisions and media move to the survivor, and its old id becomes an alias so a
    // detail link that was already shared keeps resolving.
    tx.retireRecord(duplicateId, survivorId);
  });

  const survivor = repository.getRecord(survivorId);
  if (!survivor) throw new Error(`Survivor record ${survivorId} could not be reloaded`);
  void actor;
  return survivor;
}