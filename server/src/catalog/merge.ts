import { createHash } from 'node:crypto';
import {
  EnrichmentReportSchema,
  FieldClaimSchema,
  missingFields,
  validateField,
  getFieldValue,
  setFieldValue,
  FieldDefinitions,
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

const valueFor = getFieldValue;

function comparisonValue(value: unknown): string {
  if (typeof value === 'string') return normalizeName(value);
  if (Array.isArray(value)) return stableJson([...value].map((item) => typeof item === 'string' ? normalizeName(item) : item).sort());
  return stableJson(value);
}

function withSelectedField(record: CatalogRecord, field: string, value: unknown, evidenceId: RecordId, claim: FieldClaim): CatalogRecord {
  return {...setFieldValue(record,field,value,claim.provider.provider),revision:record.revision+1,updatedAt:new Date().toISOString(),selectedEvidence:{...record.selectedEvidence,[field]:evidenceId}};
}


type ClaimOutcome = 'selected' | 'corroborated' | 'pending';

export function applyOne(tx: CatalogTransaction, claim: FieldClaim): ClaimOutcome | 'duplicate' {
  const fingerprint = claimFingerprint(claim);
  if (tx.getClaim(fingerprint)) return 'duplicate';
  const record = tx.getRecord(claim.targetRecordId);
  if (!record) throw new Error(`Target record ${claim.targetRecordId} does not exist`);
  const value = validateField(record, claim.field, claim.value);
  const currentValue = valueFor(record, claim.field);
  const id = fingerprint as RecordId;

  if (record.verification !== 'proposed' && currentValue !== undefined && comparisonValue(currentValue) === comparisonValue(value)) {
    tx.addClaim(id, fingerprint, claim, 'corroborated', record.revision);
    return 'corroborated';
  }

  const sourceIsLinked = tx.findByProvider(claim.provider) === record.id;
  const canSelect = claim.field!=='possibleDuplicate' && currentValue === undefined && record.verification !== 'proposed'
    && (claim.evidence === 'curated' || (claim.evidence === 'direct' && sourceIsLinked));
  if (canSelect) {
    tx.saveRecord(withSelectedField(record, claim.field, value, id, claim), record.revision);
    tx.addClaim(id, fingerprint, claim, 'selected', record.revision + 1);
    return 'selected';
  }

  const pendingRecord=record.reviewState==='review'?record:{...record,reviewState:'review' as const,revision:record.revision+1,updatedAt:new Date().toISOString()};
  if(pendingRecord!==record) tx.saveRecord(pendingRecord,record.revision);
  tx.addClaim(id, fingerprint, claim, 'pending', pendingRecord.revision, currentValue);
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
): CatalogDetail {
  let targetId: RecordId;
  repository.transaction((tx) => {
    const item = tx.getReview(id);
    if (!item) throw new Error(`Review item ${id} does not exist`);
    targetId=item.targetRecordId;
    const record = tx.getRecord(item.targetRecordId);
    if (!record) throw new Error(`Target record ${item.targetRecordId} does not exist`);
    // A possible duplicate can never be accepted through field review, independent of revision
    // freshness. Checked first so callers get the actionable merge guidance rather than a
    // revision conflict that would still block them once the record changes again.
    if (decision === 'accept' && item.field === 'possibleDuplicate') {
      throw new Error('Possible duplicate acceptance requires the duplicate merge action');
    }
    if (record.revision !== expectedRevision || item.recordRevision !== expectedRevision) {
      throw new Error(`Revision conflict for ${item.targetRecordId}`);
    }

    if (decision === 'accept') {
      const value = validateField(record, item.field, item.claim.value);
      let updated = withSelectedField(record, item.field, value, item.id, item.claim);
      if (record.verification === 'proposed' && ((record.kind==='event' && item.field==='name') || (record.kind==='entity' && item.field==='displayName'))) {
        updated = { ...updated, verification: 'curator-confirmed', reviewState: 'ready' };
      }
      if(item.field==='providerRefs' && updated.kind==='entity') { for(const ref of updated.providerRefs) { const owner=tx.findByProvider(ref);if(owner&&owner!==updated.id) throw new Error('Provider identity belongs to another entity');if(!owner)tx.addProviderSource(updated.id,ref); } updated={...updated,verification:'curator-confirmed'}; }
      updated={...updated,reviewState:repository.listReview().some(other=>other.targetRecordId===record.id&&other.id!==id)?'review':'ready'};
      tx.saveRecord(updated, expectedRevision);
      tx.decideReview(id, 'accepted', actor);
    } else {
      tx.decideReview(id, 'rejected', actor);
      if(record.reviewState==='review'&&!repository.listReview().some(other=>other.targetRecordId===record.id))tx.saveRecord({...record,reviewState:'ready',revision:record.revision+1,updatedAt:new Date().toISOString()},record.revision);
    }
  });
  const committed=repository.getRecord(targetId!);
  if(!committed) throw new Error('Target record could not be reloaded');
  return committed;
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
export function mergeRecords(repository: CatalogRepository,survivorId:RecordId,duplicateId:RecordId,expectedRevisions:[number,number],actor:string|{sessionId:string}):CatalogDetail {
  if(survivorId===duplicateId) throw new Error('Cannot merge a record with itself');
  const sessionId=typeof actor==='string'?actor:actor.sessionId;
  const duplicateClaims=repository.listClaims(duplicateId);
  repository.transaction(tx=>{
    const survivor=tx.getRecord(survivorId),duplicate=tx.getRecord(duplicateId);
    if(!survivor) throw new Error(`Survivor record ${survivorId} does not exist`);
    if(!duplicate) throw new Error(`Duplicate record ${duplicateId} does not exist`);
    if(survivor.id!==survivorId||duplicate.id!==duplicateId) throw new Error(`Revision conflict for ${survivorId}`);
    if(survivor.kind!==duplicate.kind) throw new Error('Cannot merge records of different kinds');
    if(survivor.revision!==expectedRevisions[0]||duplicate.revision!==expectedRevisions[1]) throw new Error(`Revision conflict for ${survivorId}`);
    // A curator merge is an explicit confirmation of a surviving proposed identity.
    let merged={...survivor,revision:survivor.revision+1,updatedAt:new Date().toISOString(),verification:survivor.verification==='proposed'?'curator-confirmed':survivor.verification} as CatalogRecord;
    if(merged.kind==='entity'&&duplicate.kind==='entity') merged={...merged,roles:[...new Set([...merged.roles,...duplicate.roles])]};
    const conflicts:Array<{field:string;value:unknown}>=[];
    for(const [field,definition] of Object.entries(FieldDefinitions)) {
      if(!definition.targets.includes(merged.kind)||['roles','assets','sources','eventDate'].includes(field)) continue;
      const current=getFieldValue(merged,field),proposed=getFieldValue(duplicate,field);
      if(proposed===undefined) continue;
      if(current===undefined) {merged=setFieldValue(merged,field,proposed);const evidence=duplicate.selectedEvidence?.[field];if(evidence)merged.selectedEvidence={...merged.selectedEvidence,[field]:evidence};}
      else if(['aliases','sourceUrls','playbackUrls','genres','styles','eventIds','mixIds','providerRefs'].includes(field)&&Array.isArray(current)&&Array.isArray(proposed)) merged=setFieldValue(merged,field,[...current,...proposed]);
      else if(comparisonValue(current)!==comparisonValue(proposed)) conflicts.push({field,value:proposed});
    }
    if(merged.kind==='mix'&&duplicate.kind==='mix') merged={...merged,sources:[...merged.sources,...duplicate.sources.filter(ref=>!merged.kind||!(merged as typeof duplicate).sources.some(source=>source.provider===ref.provider&&source.resourceType===ref.resourceType&&source.externalId===ref.externalId))]};
    if(merged.kind==='entity'&&duplicate.kind==='entity') merged={...merged,providerRefs:[...merged.providerRefs,...duplicate.providerRefs.filter(ref=>!(merged as typeof duplicate).providerRefs.some(source=>source.provider===ref.provider&&source.resourceType===ref.resourceType&&source.externalId===ref.externalId))]};
    merged.assets=[...merged.assets,...duplicate.assets.filter(asset=>!merged.assets.some(current=>current.role===asset.role))];
    for(const item of repository.listReview())if(item.field==='possibleDuplicate'&&[survivorId,duplicateId].includes(item.targetRecordId)&&[survivorId,duplicateId].includes((item.claim.value as {recordId:string}).recordId))tx.decideReview(item.id,'accepted',sessionId);
    tx.moveMergedRecords(duplicateId,survivorId,claimFingerprint);
    // Every incoming canonical reference follows the survivor, including event and membership links. The match is an exact record id, so only real references are rewritten; the survivor payload itself is rewritten before it is saved so a merged record cannot keep pointing at the retired id.
    const rewrite=(value:unknown):unknown=>Array.isArray(value)?value.map(rewrite).filter((item,index,array)=>array.findIndex(other=>stableJson(other)===stableJson(item))===index):value&&typeof value==='object'?Object.fromEntries(Object.entries(value).map(([key,item])=>[key,rewrite(item)])):value===duplicateId?survivorId:value;
    merged=rewrite(merged) as CatalogRecord;
    for(const record of tx.allRecords()) {
      if(record.id===survivorId) continue;
      const rewritten=rewrite(record) as CatalogRecord;
      if(stableJson(record)!==stableJson(rewritten)) tx.saveRecord({...rewritten,revision:record.revision+1,updatedAt:new Date().toISOString()},record.revision);
    }
    if(conflicts.length) merged={...merged,reviewState:'review' as const};
    tx.saveRecord(merged,survivor.revision);
    for(const entry of duplicateClaims) {const item=tx.getReview(entry.id);if(item?.state==='pending')tx.refreshReview(item.id,merged.revision,getFieldValue(merged,item.field));}
    const source=duplicate.kind==='mix'?duplicate.sources[0]:undefined;
    // A stored source carries addedAt/date metadata, but a claim's provider must be a bare ProviderRef.
    const provider=source?{provider:source.provider,resourceType:source.resourceType,externalId:source.externalId,...(source.url?{url:source.url}:{})}:duplicate.kind==='entity'?duplicate.providerRefs[0]:undefined;
    for(const conflict of conflicts) {
      const claim:FieldClaim={targetRecordId:survivorId,field:conflict.field,value:conflict.value,provider:provider??{provider:'archiveorg',resourceType:'curator-merge',externalId:duplicateId},sourceUrl:provider?.url??`https://mixsets.syco23.org/mix/${duplicateId}`,observedAt:new Date().toISOString(),evidence:'curated',matchExplanation:`Curator ${sessionId} merged duplicate ${duplicateId}; conflicting selection retained for review`};
      applyOne(tx,claim);
    }
    const final=tx.getRecord(survivorId)!;
    for(const item of repository.listReview())if(item.targetRecordId===survivorId)tx.refreshReview(item.id,final.revision,getFieldValue(final,item.field));
  });
  return repository.getRecord(survivorId)!;
}
