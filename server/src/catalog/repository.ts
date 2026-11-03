import { backup, type DatabaseSync } from 'node:sqlite';
import {
  CatalogPageSchema,
  CatalogRecordSchema,
  EntityRecordSchema,
  PageQuerySchema,
  ProviderRefSchema,
  type CatalogDetail,
  type CatalogPage,
  type CatalogRecord,
  type EnrichmentReport,
  type EntityRecord,
  type EntityRole,
  type FieldClaim,
  type ImportJob,
  type ImportJobState,
  type ScoredEvidence,
  type IndexKind,
  type MigrationResult,
  type PageQuery,
  type ProviderRef,
  type RecordId,
  type ReviewItem
} from '@syco23/catalog-domain';
import { normalizeName, normalizeProviderRef } from '@syco23/catalog-domain';
import { openDatabase } from './database.js';
import {
  appendImportEventV2,
  claimDueImportJobV2,
  completeImportOutboxV2,
  enqueueImportJobV2,
  insertImportJobV2,
  listEvidenceScoresV2,
  listImportArtifactsV2,
  listImportEventsV2,
  listImportJobsV2,
  readImportJobV2,
  readImportJobV2ByKey,
  registerImportArtifactV2,
  renewImportLeaseV2,
  saveEvidenceScoreV2,
  saveProvenanceV2,
  saveRightsConsentV2,
  updateImportJobV2State,
} from './import-jobs.js';

export interface CatalogTransaction {
  allRecords(): CatalogRecord[];
  moveMergedRecords(duplicate: RecordId, survivor: RecordId, fingerprint: (claim: FieldClaim)=>string): void;
  getRecord(id: RecordId): CatalogRecord | undefined;
  getClaim(fingerprint: string): StoredClaim | undefined;
  getReview(id: RecordId): ReviewItem | undefined;
  listClaims(recordId: RecordId): StoredClaim[];
  findBySource(ref: ProviderRef): RecordId | undefined;
  findByProvider(ref: ProviderRef): RecordId | undefined;
  saveRecord(record: CatalogRecord, expectedRevision?: number): CatalogRecord;
  addProviderSource(recordId: RecordId, ref: ProviderRef): void;
  addClaim(id: RecordId, fingerprint: string, claim: FieldClaim, disposition: ClaimDisposition, recordRevision: number, currentValue?: unknown): void;
  refreshReview(id: RecordId, recordRevision: number, currentValue?: unknown): void;
  decideReview(id: RecordId, state: 'accepted' | 'rejected', actor: string): void;
  addRelationship(sourceId: RecordId, targetId: RecordId, relationship: EntityRole | 'event' | 'member' | 'parent-label' | 'sub-label'): void;
  addLegacyAlias(legacyId: string, recordId: RecordId): void;
  repointAlias(legacyId: string, recordId: RecordId): void;
  retireRecord(duplicateId: RecordId, survivorId: RecordId): void;
  addMediaAsset(asset: StoredMediaAsset): void;
  getImportJob(id: RecordId): ImportJob | undefined;
  getImportJobByIdempotency(key: string): ImportJob | undefined;
  createImportJob(job: ImportJob): ImportJob;
  updateImportJobState(id: RecordId, state: ImportJobState, extra?: { errorCode?: string | null; error?: string | null; mixId?: RecordId | null; attempt?: number }): ImportJob;
  appendImportEvent(jobId: RecordId, type: string, payload?: Record<string, unknown>): void;
  enqueueImportJob(jobId: RecordId, delayMs?: number): void;
  claimDueImportJob(owner: string, leaseMs?: number): ImportJob | undefined;
  completeImportOutbox(jobId: RecordId): void;
  renewImportLease(jobId: RecordId, owner: string, leaseMs?: number): boolean;
  listImportEvents(jobId: RecordId): Array<{ id: string; jobId: string; sequence: number; type: string; payload: Record<string, unknown>; createdAt: string }>;
  registerImportArtifact(input: { id: RecordId; jobId: RecordId; role: string; objectKey: string; sha256: string; mimeType?: string | null; sizeBytes?: number | null; codec?: string | null; durationMs?: number | null; retentionUntil?: string | null }): void;
  listImportArtifacts(jobId: RecordId): Array<{ id: string; jobId: string; role: string; objectKey: string; sha256: string; mimeType: string | null; sizeBytes: number | null; codec: string | null; durationMs: number | null; state: string; createdAt: string }>;
  saveImportProvenance(jobId: RecordId, row: { provider: string; sourceUrl: string; externalId?: string | null; retrievalMethod: string; observedAt: string; termsVersion?: string | null; snapshot: Record<string, unknown> }): void;
  saveImportRightsConsent(input: { id: RecordId; jobId: RecordId; requestedBy: string; basis: string; provider: string; sourceUrl?: string | null; attestationVersion: string; proofObjectKey?: string | null }): void;
  saveImportEvidenceScore(input: { id: RecordId; jobId: RecordId; claimId?: string | null; field: string; scored: ScoredEvidence }): ScoredEvidence & { id: RecordId; jobId: RecordId };
  listImportEvidenceScores(jobId: RecordId): Array<{ id: string; jobId: string; claimId: string | null; field: string; score: number; algorithmVersion: string; components: Record<string, unknown>; hardGates: Record<string, unknown>; decision: string; evaluatedAt: string }>;
}

export type ClaimDisposition = 'selected' | 'corroborated' | 'pending' | 'rejected';
export interface StoredClaim {
  id: RecordId;
  fingerprint: string;
  claim: FieldClaim;
  disposition: ClaimDisposition;
}

export interface CuratorSession {
  id: RecordId;
  tokenHash: string;
  expiresAt: string;
  createdAt: string;
}

export interface StoredAnalysisJob {
  id: string;
  recordId: RecordId;
  sourceUrl: string;
  state: 'queued' | 'running' | 'done' | 'error' | 'interrupted';
  progress: number;
  error?: string;
  analyzedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface StoredMediaAsset {
  mediaId: RecordId;
  recordId: RecordId;
  role: 'waveform';
  relativePath: string;
  mimeType: 'image/png';
  byteSize: number;
  sourceUrl: string;
}

export interface EnrichmentRunInput {
  id: string;
  recordId: RecordId;
  attemptedProviders: string[];
  actor: string;
  startedAt: string;
}

export interface EnrichmentRun {
  id: string;
  recordId: RecordId;
  state: 'running' | 'completed' | 'interrupted' | 'failed';
  attemptedProviders: string[];
  applied: number;
  corroborated: number;
  reviewed: number;
  errors: Array<{ provider: string; message: string }>;
  missingFields: string[];
  startedAt: string;
  finishedAt?: string;
}

export interface CatalogRepository {
  saveAnalysisJob(job: StoredAnalysisJob): void;
  getAnalysisJob(id: string): StoredAnalysisJob | undefined;
  listAnalysisJobs(recordId?: RecordId): StoredAnalysisJob[];
  interruptStaleAnalysisJobs(): number;
  readonly path: string;
  backupTo(destination: string): Promise<void>;
  listMediaAssets(): StoredMediaAsset[];
  listClaims(recordId: RecordId): StoredClaim[];
  getRecord(id: RecordId): CatalogDetail | undefined;
  listIndex(kind: IndexKind, query: PageQuery): CatalogPage;
  /**
   * Mixes that reference an entity.
   *
   * Derived by scanning each mix payload rather than from a stored back-reference:
   * `EventRecord.mixIds` is only populated by legacy migration, so trusting it would
   * leave an artist page empty for every mix that arrived through import or
   * enrichment. The scan answers "which sets did this artist play on" from the same
   * source of truth the mix detail page renders.
   */
  listRelatedMixes(entityId: RecordId): CatalogRecord[];
  findByProvider(ref: ProviderRef): RecordId | undefined;
  findBySource(ref: ProviderRef): RecordId | undefined;
  findNameCandidates(role: EntityRole, name: string): EntityRecord[];
  listReview(): ReviewItem[];
  getReview(id: RecordId): ReviewItem | undefined;
  getClaim(fingerprint: string): StoredClaim | undefined;
  createSession(session: CuratorSession): void;
  getSession(tokenHash: string): CuratorSession | undefined;
  deleteSession(tokenHash: string): void;
  getMediaAsset(mediaId: RecordId): StoredMediaAsset | undefined;
  listClaims(recordId: RecordId): StoredClaim[];
  getMigrationBatch(batchId: string): MigrationResult | undefined;
  saveMigrationBatch(result: MigrationResult): MigrationResult;
  startEnrichmentRun(run: EnrichmentRunInput): void;
  finishEnrichmentRun(runId: string, state: 'completed' | 'interrupted' | 'failed', report?: EnrichmentReport): void;
  interruptStaleEnrichmentRuns(): number;
  listEnrichmentRuns(recordId: RecordId): EnrichmentRun[];
  getImportJob(id: RecordId): ImportJob | undefined;
  getImportJobByIdempotency(key: string): ImportJob | undefined;
  createImportJob(job: ImportJob): ImportJob;
  updateImportJobState(id: RecordId, state: ImportJobState, extra?: { errorCode?: string | null; error?: string | null; mixId?: RecordId | null; attempt?: number }): ImportJob;
  appendImportEvent(jobId: RecordId, type: string, payload?: Record<string, unknown>): void;
  enqueueImportJob(jobId: RecordId, delayMs?: number): void;
  claimDueImportJob(owner: string, leaseMs?: number): ImportJob | undefined;
  completeImportOutbox(jobId: RecordId): void;
  renewImportLease(jobId: RecordId, owner: string, leaseMs?: number): boolean;
  listImportEvents(jobId: RecordId): Array<{ id: string; jobId: string; sequence: number; type: string; payload: Record<string, unknown>; createdAt: string }>;
  registerImportArtifact(input: { id: RecordId; jobId: RecordId; role: string; objectKey: string; sha256: string; mimeType?: string | null; sizeBytes?: number | null; codec?: string | null; durationMs?: number | null; retentionUntil?: string | null }): void;
  listImportArtifacts(jobId: RecordId): Array<{ id: string; jobId: string; role: string; objectKey: string; sha256: string; mimeType: string | null; sizeBytes: number | null; codec: string | null; durationMs: number | null; state: string; createdAt: string }>;
  saveImportProvenance(jobId: RecordId, row: { provider: string; sourceUrl: string; externalId?: string | null; retrievalMethod: string; observedAt: string; termsVersion?: string | null; snapshot: Record<string, unknown> }): void;
  saveImportRightsConsent(input: { id: RecordId; jobId: RecordId; requestedBy: string; basis: string; provider: string; sourceUrl?: string | null; attestationVersion: string; proofObjectKey?: string | null }): void;
  saveImportEvidenceScore(input: { id: RecordId; jobId: RecordId; claimId?: string | null; field: string; scored: ScoredEvidence }): ScoredEvidence & { id: RecordId; jobId: RecordId };
  listImportEvidenceScores(jobId: RecordId): Array<{ id: string; jobId: string; claimId: string | null; field: string; score: number; algorithmVersion: string; components: Record<string, unknown>; hardGates: Record<string, unknown>; decision: string; evaluatedAt: string }>;
  listImportJobs(options?: { state?: ImportJobState; limit?: number }): ImportJob[];
  transaction<T>(operation: (tx: CatalogTransaction) => T): T;
  close(): void;
}

type StoredRecord = {
  id: string;
  kind: string;
  verification: string;
  revision: number;
  search_name: string;
  payload: string;
};

function parseRecord(row: StoredRecord | undefined): CatalogRecord | undefined {
  if (!row) return undefined;
  return CatalogRecordSchema.parse(JSON.parse(row.payload));
}

function recordName(record: CatalogRecord): string {
  if (record.kind === 'mix') return record.title;
  if (record.kind === 'entity') return record.displayName;
  return record.name;
}

function recordById(database: DatabaseSync, id: RecordId): CatalogRecord | undefined {
  const row = database.prepare('SELECT id, kind, verification, revision, search_name, payload FROM catalog_records WHERE id = ?').get(id) as StoredRecord | undefined;
  if (row) return parseRecord(row);
  const alias = database.prepare('SELECT record_id FROM record_aliases WHERE legacy_id = ?').get(id) as { record_id: string } | undefined;
  return alias ? recordById(database, alias.record_id) : undefined;
}

function putNameKeys(database: DatabaseSync, record: CatalogRecord) {
  database.prepare('DELETE FROM entity_names WHERE record_id = ?').run(record.id);
  database.prepare('DELETE FROM entity_roles WHERE record_id = ?').run(record.id);
  if (record.kind !== 'entity') return;

  const roleInsert = database.prepare('INSERT INTO entity_roles(record_id, role) VALUES (?, ?)');
  const nameInsert = database.prepare('INSERT OR IGNORE INTO entity_names(record_id, role, search_key) VALUES (?, ?, ?)');
  for (const role of record.roles) {
    roleInsert.run(record.id, role);
    for (const name of [record.displayName, ...record.aliases]) {
      nameInsert.run(record.id, role, normalizeName(name));
    }
  }
}

function createTransaction(database: DatabaseSync): CatalogTransaction {
  return {
    allRecords() { return (database.prepare('SELECT * FROM catalog_records').all() as StoredRecord[]).map((row)=>parseRecord(row)!); },
    moveMergedRecords(duplicate, survivor, fingerprint) {
      for (const table of ['provider_sources','field_claims','review_items','media_assets','record_aliases','enrichment_runs','analysis_runs']) database.prepare(`UPDATE ${table} SET record_id = ? WHERE record_id = ?`).run(survivor,duplicate);
      const relations=database.prepare('SELECT source_id,target_id,relationship FROM record_relationships WHERE source_id=? OR target_id=?').all(duplicate,duplicate) as Array<{source_id:string;target_id:string;relationship:string}>;
      database.prepare('DELETE FROM record_relationships WHERE source_id=? OR target_id=?').run(duplicate,duplicate);
      for(const relation of relations) { const source=relation.source_id===duplicate?survivor:relation.source_id; const target=relation.target_id===duplicate?survivor:relation.target_id; if(source!==target) database.prepare('INSERT OR IGNORE INTO record_relationships(source_id,target_id,relationship) VALUES (?,?,?)').run(source,target,relation.relationship); }
      const analysis=database.prepare('SELECT id,payload FROM analysis_runs WHERE record_id=?').all(survivor) as Array<{id:string;payload:string}>;
      for(const row of analysis){const job=JSON.parse(row.payload) as StoredAnalysisJob;if(job.recordId===duplicate)database.prepare('UPDATE analysis_runs SET payload=? WHERE id=?').run(JSON.stringify({...job,recordId:survivor}),row.id);}
      const claims=database.prepare('SELECT id,claim_json FROM field_claims WHERE record_id=?').all(survivor) as Array<{id:string;claim_json:string}>;
      for(const row of claims) { const claim=JSON.parse(row.claim_json) as FieldClaim; if(claim.targetRecordId===duplicate){const moved={...claim,targetRecordId:survivor};database.prepare('UPDATE field_claims SET claim_json=? WHERE id=?').run(JSON.stringify(moved),row.id);database.prepare('INSERT OR IGNORE INTO claim_fingerprint_aliases(fingerprint,claim_id) VALUES(?,?)').run(fingerprint(moved),row.id);} }
      database.prepare('DELETE FROM catalog_records WHERE id=?').run(duplicate);
      database.prepare('INSERT INTO record_aliases(legacy_id,record_id,created_at) VALUES (?,?,?)').run(duplicate,survivor,new Date().toISOString());
    },
    getRecord(id) {
      return recordById(database, id);
    },

    getClaim(fingerprint) {
      const row = database.prepare('SELECT id, fingerprint, claim_json, disposition FROM field_claims WHERE fingerprint = ? OR id IN (SELECT claim_id FROM claim_fingerprint_aliases WHERE fingerprint = ?)').get(fingerprint,fingerprint) as {
        id: string; fingerprint: string; claim_json: string; disposition: ClaimDisposition;
      } | undefined;
      return row ? { id: row.id, fingerprint: row.fingerprint, claim: JSON.parse(row.claim_json) as FieldClaim, disposition: row.disposition } : undefined;
    },

    getReview(id) {
      return readReview(database, id);
    },

    findBySource(input) {const ref=normalizeProviderRef(input);const row=ref.url?database.prepare('SELECT record_id FROM provider_sources WHERE provider=? AND url=?').get(ref.provider,ref.url) as {record_id:string}|undefined:undefined;return row?.record_id??this.findByProvider(ref);},
    findByProvider(input) {
      const ref = normalizeProviderRef(ProviderRefSchema.parse(input));
      const row = database.prepare(`SELECT record_id FROM provider_sources
        WHERE provider = ? AND resource_type = ? AND external_id = ?`).get(ref.provider, ref.resourceType, ref.externalId) as { record_id: string } | undefined;
      return row?.record_id;
    },

    listClaims(recordId) {
      const rows = database.prepare('SELECT fingerprint FROM field_claims WHERE record_id=? ORDER BY created_at,id').all(recordId) as {fingerprint:string}[];
      return rows.map((row)=>this.getClaim(row.fingerprint)!);
    },

    saveRecord(input, expectedRevision) {
      const record = CatalogRecordSchema.parse(input);
      const current = database.prepare('SELECT revision FROM catalog_records WHERE id = ?').get(record.id) as { revision: number } | undefined;
      if (current) {
        if (expectedRevision !== current.revision) throw new Error(`Revision conflict for ${record.id}`);
        if (record.revision !== current.revision + 1) throw new Error(`Record revision must advance to ${current.revision + 1}`);
        database.prepare(`UPDATE catalog_records
          SET kind = ?, verification = ?, revision = ?, search_name = ?, payload = ?, updated_at = ?
          WHERE id = ?`).run(record.kind, record.verification, record.revision, normalizeName(recordName(record)), JSON.stringify(record), record.updatedAt, record.id);
      } else {
        if (expectedRevision !== undefined) throw new Error(`Record ${record.id} does not exist`);
        if (record.revision !== 1) throw new Error('New records must start at revision 1');
        database.prepare(`INSERT INTO catalog_records(id, kind, verification, revision, search_name, payload, created_at, updated_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?)`).run(record.id, record.kind, record.verification, record.revision, normalizeName(recordName(record)), JSON.stringify(record), record.createdAt, record.updatedAt);
      }
      putNameKeys(database, record);
      return record;
    },

    addClaim(id, fingerprint, claim, disposition, recordRevision, currentValue) {
      database.prepare(`INSERT INTO field_claims(id, fingerprint, record_id, field, claim_json, disposition, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)`).run(id, fingerprint, claim.targetRecordId, claim.field, JSON.stringify(claim), disposition, claim.observedAt);
      if (disposition === 'pending') {
        database.prepare(`INSERT INTO review_items(id, record_id, field, current_value_json, record_revision, state, created_at)
          VALUES (?, ?, ?, ?, ?, 'pending', ?)`).run(id, claim.targetRecordId, claim.field, currentValue === undefined ? null : JSON.stringify(currentValue), recordRevision, claim.observedAt);
      }
    },

    refreshReview(id, recordRevision, currentValue) {
      const review = readReview(database, id);
      if (!review || review.state !== 'pending') throw new Error(`Pending review item ${id} does not exist`);
      database.prepare('UPDATE review_items SET record_revision = ?, current_value_json = ? WHERE id = ?')
        .run(recordRevision, currentValue === undefined ? null : JSON.stringify(currentValue), id);
    },

    decideReview(id, state, actor) {
      const review = readReview(database, id);
      if (!review) throw new Error(`Review item ${id} does not exist`);
      if (review.state !== 'pending') throw new Error(`Review item ${id} is already decided`);
      const decidedAt = new Date().toISOString();
      database.prepare('UPDATE review_items SET state = ?, decided_at = ?, actor = ? WHERE id = ?').run(state, decidedAt, actor, id);
      database.prepare('UPDATE field_claims SET disposition = ? WHERE id = ?').run(state === 'accepted' ? 'selected' : 'rejected', id);
      database.prepare('INSERT INTO review_decisions(review_id, decision, actor, decided_at) VALUES (?, ?, ?, ?)').run(id, state, actor, decidedAt);
    },

    addProviderSource(recordId, input) {
      const ref = normalizeProviderRef(ProviderRefSchema.parse(input));
      const owner=this.findByProvider(ref);if(owner){if(owner!==recordId)throw new Error('Provider identity belongs to another record');return;}
      if (!recordById(database, recordId)) throw new Error(`Unknown record ${recordId}`);
      database.prepare(`INSERT INTO provider_sources(provider, resource_type, external_id, url, record_id, added_at)
        VALUES (?, ?, ?, ?, ?, ?)`).run(ref.provider, ref.resourceType, ref.externalId, ref.url ?? null, recordId, new Date().toISOString());
    },

    addRelationship(sourceId, targetId, relationship) {
      const source = recordById(database, sourceId);
      const target = recordById(database, targetId);
      if (!source || !target) throw new Error('Relationship references an unknown record');
      validateRelationship(source,target,relationship);
      if (['artist', 'crew', 'label'].includes(relationship)) {
        if (target.kind !== 'entity' || !target.roles.includes(relationship as EntityRole)) {
          throw new Error(`Relationship target does not have the ${relationship} role`);
        }
      }
      if (relationship === 'event' && target.kind !== 'event') throw new Error('Event relationship target must be an event');
      database.prepare('INSERT INTO record_relationships(source_id, target_id, relationship) VALUES (?, ?, ?)').run(sourceId, targetId, relationship);
    },

    addLegacyAlias(legacyId, recordId) {
      if (!legacyId || legacyId.length > 200) throw new Error('Legacy ID is invalid');
      if (!recordById(database, recordId)) throw new Error(`Unknown record ${recordId}`);
      const existing=recordById(database,legacyId);
      if(existing) { if(existing.id!==recordId) throw new Error('Legacy ID collision requires review'); return; }
      database.prepare('INSERT INTO record_aliases(legacy_id, record_id, created_at) VALUES (?, ?, ?)').run(legacyId, recordId, new Date().toISOString());
    },

    repointAlias(legacyId, recordId) {
      // Existing links that pointed at a merged duplicate must now resolve to the
      // survivor, otherwise a previously shared detail link would 404 after a merge.
      if (!legacyId || legacyId.length > 200) throw new Error('Legacy ID is invalid');
      if (!recordById(database, recordId)) throw new Error(`Unknown record ${recordId}`);
      database.prepare(`UPDATE record_aliases SET record_id = ?, created_at = ?
        WHERE legacy_id = ? AND record_id <> ?`).run(recordId, new Date().toISOString(), legacyId, recordId);
    },

    retireRecord(duplicateId, survivorId) {
      if (duplicateId === survivorId) throw new Error('Cannot merge a record with itself');
      if (!recordById(database, duplicateId)) throw new Error(`Unknown record ${duplicateId}`);
      if (!recordById(database, survivorId)) throw new Error(`Unknown record ${survivorId}`);

      // The old id must resolve to the survivor before the row disappears, because
      // recordById only consults the alias table when no live record carries the id.
      database.prepare(`INSERT INTO record_aliases(legacy_id, record_id, created_at) VALUES (?, ?, ?)
        ON CONFLICT(legacy_id) DO UPDATE SET record_id = excluded.record_id`)
        .run(duplicateId, survivorId, new Date().toISOString());

      // Nothing selected is discarded: claims, their review decisions and media all
      // follow the survivor so the provenance of the duplicate stays auditable.
      // The stored claim still points at the retired id, so its embedded target must be
      // rewritten too or a later review decision would resolve to a record that is gone.
      database.prepare(`UPDATE field_claims SET record_id = ?, claim_json = json_set(claim_json, '$.targetRecordId', ?) WHERE record_id = ?`)
        .run(survivorId, survivorId, duplicateId);
      database.prepare('UPDATE review_items SET record_id = ? WHERE record_id = ?').run(survivorId, duplicateId);
      database.prepare('UPDATE media_assets SET record_id = ? WHERE record_id = ?').run(survivorId, duplicateId);
      database.prepare('UPDATE record_relationships SET target_id = ? WHERE target_id = ? AND source_id <> ?')
        .run(survivorId, duplicateId, survivorId);
      database.prepare('UPDATE record_relationships SET source_id = ? WHERE source_id = ? AND target_id <> ?')
        .run(survivorId, duplicateId, survivorId);

      // Provider identities move to the survivor, keeping the uniqueness constraint on
      // (provider, resource_type, external_id) intact. A clash means the two records
      // already claimed the same identity, which is exactly the merge the curator did.
      database.prepare(`UPDATE OR IGNORE provider_sources SET record_id = ?, added_at = ?
        WHERE record_id = ?`).run(survivorId, new Date().toISOString(), duplicateId);
      database.prepare(`DELETE FROM provider_sources WHERE record_id = ?`).run(duplicateId);

      database.prepare('DELETE FROM entity_roles WHERE record_id = ?').run(duplicateId);
      database.prepare('DELETE FROM entity_names WHERE record_id = ?').run(duplicateId);
      database.prepare('DELETE FROM catalog_records WHERE id = ?').run(duplicateId);
    },

    addMediaAsset(asset) {
      if (asset.relativePath !== `${asset.mediaId}.png`) throw new Error('Media path is invalid');
      database.prepare(`INSERT INTO media_assets(media_id, record_id, role, relative_path, mime_type, byte_size, source_url, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)`).run(asset.mediaId, asset.recordId, asset.role, asset.relativePath, asset.mimeType, asset.byteSize, asset.sourceUrl, new Date().toISOString());
    },

    getImportJob(id) {
      return readImportJobV2(database, id);
    },

    getImportJobByIdempotency(key) {
      return readImportJobV2ByKey(database, key);
    },

    createImportJob(job) {
      const created = insertImportJobV2(database, job);
      appendImportEventV2(database, created.id, 'created', { state: created.state });
      return created;
    },

    updateImportJobState(id, state, extra) {
      const next = updateImportJobV2State(database, id, state, extra);
      appendImportEventV2(database, id, 'state', { state });
      if (state === 'completed' || state === 'blocked_policy' || state === 'failed' || state === 'cancelled') {
        completeImportOutboxV2(database, id);
      }
      return next;
    },

    appendImportEvent(jobId, type, payload) {
      appendImportEventV2(database, jobId, type, payload);
    },

    enqueueImportJob(jobId, delayMs) {
      enqueueImportJobV2(database, jobId, delayMs);
    },

    claimDueImportJob(owner, leaseMs) {
      return claimDueImportJobV2(database, owner, leaseMs);
    },

    completeImportOutbox(jobId) {
      completeImportOutboxV2(database, jobId);
    },

    renewImportLease(jobId, owner, leaseMs) {
      return renewImportLeaseV2(database, jobId, owner, leaseMs);
    },

    listImportEvents(jobId) {
      return listImportEventsV2(database, jobId);
    },

    registerImportArtifact(input) {
      registerImportArtifactV2(database, input);
    },

    listImportArtifacts(jobId) {
      return listImportArtifactsV2(database, jobId);
    },

    saveImportProvenance(jobId, row) {
      saveProvenanceV2(database, jobId, row);
    },

    saveImportRightsConsent(input) {
      saveRightsConsentV2(database, input);
    },

    saveImportEvidenceScore(input) {
      return saveEvidenceScoreV2(database, input);
    },

    listImportEvidenceScores(jobId: RecordId): Array<{
      id: string; jobId: string; claimId: string | null; field: string;
      score: number; algorithmVersion: string; components: Record<string, unknown>;
      hardGates: Record<string, unknown>; decision: string; evaluatedAt: string;
    }> {
      return listEvidenceScoresV2(database, jobId) as unknown as Array<{
        id: string; jobId: string; claimId: string | null; field: string;
        score: number; algorithmVersion: string; components: Record<string, unknown>;
        hardGates: Record<string, unknown>; decision: string; evaluatedAt: string;
      }>;
    }
  };
}

export function openCatalog(path: string): CatalogRepository {
  const database = openDatabase(path);
  let inTransaction = false;
  database.exec('BEGIN IMMEDIATE');
  try {validateReferences(database);syncNormalizedTables(database);database.exec('COMMIT');}catch(error){database.exec('ROLLBACK');database.close();throw error;}

  return {
    path,
    saveAnalysisJob(job) {
      const target=recordById(database,job.recordId);if(!target)throw new Error('Analysis target record not found');job={...job,recordId:target.id};
      database.prepare(`INSERT INTO analysis_runs(id,record_id,state,payload,updated_at) VALUES(?,?,?,?,?)
        ON CONFLICT(id) DO UPDATE SET state=excluded.state,payload=excluded.payload,updated_at=excluded.updated_at`)
        .run(job.id,job.recordId,job.state,JSON.stringify(job),job.updatedAt);
    },
    getAnalysisJob(id) {
      const row=database.prepare('SELECT payload FROM analysis_runs WHERE id=?').get(id) as {payload:string}|undefined;
      return row ? JSON.parse(row.payload) as StoredAnalysisJob : undefined;
    },
    listAnalysisJobs(recordId) {
      const rows=(recordId ? database.prepare('SELECT payload FROM analysis_runs WHERE record_id=? ORDER BY updated_at DESC').all(recordId)
        : database.prepare('SELECT payload FROM analysis_runs ORDER BY updated_at DESC LIMIT 100').all()) as Array<{payload:string}>;
      return rows.map(row=>JSON.parse(row.payload) as StoredAnalysisJob);
    },
    interruptStaleAnalysisJobs() {
      const rows=database.prepare("SELECT payload FROM analysis_runs WHERE state IN ('queued','running')").all() as Array<{payload:string}>;
      for(const row of rows){ const job=JSON.parse(row.payload) as StoredAnalysisJob;
        this.saveAnalysisJob({...job,state:'interrupted',error:'Audio analysis was interrupted; retry the run',updatedAt:new Date().toISOString()}); }
      return rows.length;
    },
    async backupTo(destination) { await backup(database,destination); },
    listMediaAssets() { const rows=database.prepare('SELECT media_id FROM media_assets').all() as {media_id:string}[]; return rows.map((row)=>this.getMediaAsset(row.media_id)!); },
    listClaims(recordId) { const rows=database.prepare('SELECT fingerprint FROM field_claims WHERE record_id=? ORDER BY created_at,id').all(recordId) as {fingerprint:string}[]; return rows.map((row)=>this.getClaim(row.fingerprint)!); },
    getRecord(id) {
      const record = recordById(database, id);
      return record ? CatalogRecordSchema.parse(record) : undefined;
    },

    listReview() {
      const rows = database.prepare(`SELECT id FROM review_items WHERE state = 'pending' ORDER BY created_at, id`).all() as { id: string }[];
      return rows.map((row) => readReview(database, row.id)!).filter(Boolean);
    },

    listRelatedMixes(entityId) {
      const rows = database.prepare(`SELECT r.id, r.kind, r.verification, r.revision, r.search_name, r.payload
        FROM catalog_records r
        WHERE r.kind = 'mix' AND r.verification <> 'proposed'
        ORDER BY r.updated_at DESC, r.id`).all() as StoredRecord[];
      const mixes: CatalogRecord[] = [];
      for (const row of rows) {
        const record = parseRecord(row);
        if (record?.kind === 'mix' && record.people.some((person) => person.entityId === entityId)) mixes.push(record);
      }
      return mixes;
    },

    getReview(id) {
      return readReview(database, id);
    },

    getClaim(fingerprint) {
      return createTransaction(database).getClaim(fingerprint);
    },

    createSession(session) {
      database.prepare('INSERT INTO curator_sessions(id, token_hash, expires_at, created_at) VALUES (?, ?, ?, ?)')
        .run(session.id, session.tokenHash, session.expiresAt, session.createdAt);
    },

    getSession(tokenHash) {
      const row = database.prepare('SELECT id, token_hash, expires_at, created_at FROM curator_sessions WHERE token_hash = ?')
        .get(tokenHash) as { id: string; token_hash: string; expires_at: string; created_at: string } | undefined;
      return row ? { id: row.id, tokenHash: row.token_hash, expiresAt: row.expires_at, createdAt: row.created_at } : undefined;
    },

    deleteSession(tokenHash) {
      database.prepare('DELETE FROM curator_sessions WHERE token_hash = ?').run(tokenHash);
    },

    getMediaAsset(mediaId) {
      const row = database.prepare(`SELECT media_id, record_id, role, relative_path, mime_type, byte_size, source_url
        FROM media_assets WHERE media_id = ?`).get(mediaId) as {
          media_id: string; record_id: string; role: 'waveform'; relative_path: string;
          mime_type: 'image/png'; byte_size: number; source_url: string;
        } | undefined;
      return row ? {
        mediaId: row.media_id,
        recordId: row.record_id,
        role: row.role,
        relativePath: row.relative_path,
        mimeType: row.mime_type,
        byteSize: row.byte_size,
        sourceUrl: row.source_url
      } : undefined;
    },

    getMigrationBatch(batchId) {
      const row = database.prepare('SELECT result_json FROM migration_batches WHERE batch_id = ?').get(batchId) as { result_json: string } | undefined;
      return row ? JSON.parse(row.result_json) as MigrationResult : undefined;
    },

    saveMigrationBatch(result) {
      database.prepare('INSERT INTO migration_batches(batch_id, result_json, created_at) VALUES (?, ?, ?) ON CONFLICT(batch_id) DO UPDATE SET result_json=excluded.result_json')
        .run(result.batchId, JSON.stringify(result), new Date().toISOString());
      const row = database.prepare('SELECT result_json FROM migration_batches WHERE batch_id = ?').get(result.batchId) as { result_json: string } | undefined;
      if (!row) throw new Error(`Migration batch ${result.batchId} could not be stored`);
      return JSON.parse(row.result_json) as MigrationResult;
    },

    startEnrichmentRun(run) {
      // The partial unique index on state='running' rejects overlapping runs for the
      // same record, so a crashed run blocks a retry until it is marked interrupted.
      database.prepare(`INSERT INTO enrichment_runs(id, record_id, state, attempted_providers, actor, started_at)
        VALUES (?, ?, 'running', ?, ?, ?)`).run(run.id, run.recordId, JSON.stringify(run.attemptedProviders), run.actor, run.startedAt);
    },

    finishEnrichmentRun(runId, state, report) {
      database.prepare(`UPDATE enrichment_runs
        SET state = ?, applied = ?, corroborated = ?, reviewed = ?, errors_json = ?, missing_fields = ?, finished_at = ?
        WHERE id = ? AND state = 'running'`)
        .run(state, report?.applied ?? 0, report?.corroborated ?? 0, report?.reviewed ?? 0,
          JSON.stringify(report?.errors ?? []), JSON.stringify(report?.missingFields ?? []), new Date().toISOString(), runId);
    },

    interruptStaleEnrichmentRuns() {
      // Called at worker start: runs left 'running' by a previous process can never
      // finish, so they are closed out and become visible as retryable.
      const result = database.prepare(`UPDATE enrichment_runs SET state = 'interrupted', finished_at = ?
        WHERE state = 'running'`).run(new Date().toISOString());
      return Number(result.changes ?? 0);
    },

    listEnrichmentRuns(recordId) {
      const rows = database.prepare(`SELECT id, record_id, state, attempted_providers, applied, corroborated,
        reviewed, errors_json, missing_fields, started_at, finished_at
        FROM enrichment_runs WHERE record_id = ? ORDER BY started_at DESC LIMIT 20`).all(recordId) as Array<{
          id: string; record_id: string; state: EnrichmentRun['state']; attempted_providers: string;
          applied: number; corroborated: number; reviewed: number; errors_json: string;
          missing_fields: string; started_at: string; finished_at: string | null;
        }>;
      return rows.map((row) => ({
        id: row.id,
        recordId: row.record_id,
        state: row.state,
        attemptedProviders: JSON.parse(row.attempted_providers) as string[],
        applied: row.applied,
        corroborated: row.corroborated,
        reviewed: row.reviewed,
        errors: JSON.parse(row.errors_json) as Array<{ provider: string; message: string }>,
        missingFields: JSON.parse(row.missing_fields) as string[],
        startedAt: row.started_at,
        ...(row.finished_at ? { finishedAt: row.finished_at } : {})
      }));
    },

    getImportJob(id) {
      return createTransaction(database).getImportJob(id);
    },

    getImportJobByIdempotency(key) {
      return createTransaction(database).getImportJobByIdempotency(key);
    },

    createImportJob(job) {
      return createTransaction(database).createImportJob(job);
    },

    updateImportJobState(id, state, extra) {
      return createTransaction(database).updateImportJobState(id, state, extra);
    },

    appendImportEvent(jobId, type, payload) {
      return createTransaction(database).appendImportEvent(jobId, type, payload);
    },

    enqueueImportJob(jobId, delayMs) {
      return createTransaction(database).enqueueImportJob(jobId, delayMs);
    },

    claimDueImportJob(owner, leaseMs) {
      return createTransaction(database).claimDueImportJob(owner, leaseMs);
    },

    completeImportOutbox(jobId) {
      return createTransaction(database).completeImportOutbox(jobId);
    },

    renewImportLease(jobId, owner, leaseMs) {
      return createTransaction(database).renewImportLease(jobId, owner, leaseMs);
    },

    listImportEvents(jobId) {
      return createTransaction(database).listImportEvents(jobId);
    },

    registerImportArtifact(input) {
      return createTransaction(database).registerImportArtifact(input);
    },

    listImportArtifacts(jobId) {
      return createTransaction(database).listImportArtifacts(jobId);
    },

    saveImportProvenance(jobId, row) {
      return createTransaction(database).saveImportProvenance(jobId, row);
    },

    saveImportRightsConsent(input) {
      return createTransaction(database).saveImportRightsConsent(input);
    },

    saveImportEvidenceScore(input) {
      return createTransaction(database).saveImportEvidenceScore(input);
    },

    listImportEvidenceScores(jobId) {
      return createTransaction(database).listImportEvidenceScores(jobId);
    },

    listImportJobs(options) {
      return listImportJobsV2(database, options ?? {});
    },

    listIndex(kind, input) {
      const query = PageQuerySchema.parse(input);
      const offset = (query.page - 1) * query.pageSize;
      const index = kind === 'mix' || kind === 'event' ? kind : 'entity';
      const baseParams = kind === 'artist' || kind === 'crew' || kind === 'label'
        ? [kind]
        : [index];
      const from = kind === 'artist' || kind === 'crew' || kind === 'label'
        ? 'catalog_records r JOIN entity_roles er ON er.record_id = r.id'
        : 'catalog_records r';
      const filters = [
        kind === 'artist' || kind === 'crew' || kind === 'label' ? 'er.role = ?' : 'r.kind = ?',
        "r.verification <> 'proposed'"
      ];
      const params: (string | number)[] = [...baseParams];
      if (query.query) {
        filters.push('(r.search_name LIKE ? ESCAPE \'\\\' OR EXISTS (SELECT 1 FROM entity_names n WHERE n.record_id=r.id AND n.search_key LIKE ? ESCAPE \'\\\'))');
        const search=`%${normalizeName(query.query).replace(/[\\%_]/g, '\\$&')}%`;
        params.push(search,search);
      }
      const where = filters.join(' AND ');
      const rows = database.prepare(`SELECT r.id, r.kind, r.verification, r.revision, r.search_name, r.payload
        FROM ${from} WHERE ${where} ORDER BY r.updated_at DESC, r.id LIMIT ? OFFSET ?`)
        .all(...params, query.pageSize, offset) as StoredRecord[];
      const totalRow = database.prepare(`SELECT COUNT(DISTINCT r.id) AS total FROM ${from} WHERE ${where}`).get(...params) as { total: number };
      const page: CatalogPage = {
        items: rows.map((row) => parseRecord(row)!).filter((record) => record.kind === 'mix' || record.kind === 'event' || record.roles.includes(kind as EntityRole)),
        page: query.page,
        pageSize: query.pageSize,
        total: totalRow.total
      };
      return CatalogPageSchema.parse(page);
    },

    findByProvider(input) {
      const ref = normalizeProviderRef(ProviderRefSchema.parse(input));
      const row = database.prepare(`SELECT record_id FROM provider_sources
        WHERE provider = ? AND resource_type = ? AND external_id = ?`).get(ref.provider, ref.resourceType, ref.externalId) as { record_id: string } | undefined;
      return row?.record_id;
    },

    findBySource(input) {
      const ref = normalizeProviderRef(ProviderRefSchema.parse(input));
      if (ref.url) {
        const row = database.prepare('SELECT record_id FROM provider_sources WHERE provider = ? AND url = ?').get(ref.provider, ref.url) as { record_id: string } | undefined;
        if (row) return row.record_id;
      }
      return this.findByProvider(ref);
    },

    findNameCandidates(role, name) {
      const key = normalizeName(name);
      const rows = database.prepare(`SELECT r.id, r.kind, r.verification, r.revision, r.search_name, r.payload
        FROM entity_names n JOIN catalog_records r ON r.id = n.record_id
        WHERE n.role = ? AND n.search_key = ? ORDER BY r.updated_at DESC`).all(role, key) as StoredRecord[];
      return rows.map((row) => EntityRecordSchema.parse(parseRecord(row)));
    },

    transaction(operation) {
      if (inTransaction) throw new Error('Nested catalog transactions are not supported');
      inTransaction = true;
      database.exec('BEGIN IMMEDIATE');
      try {
        const result = operation(createTransaction(database));
        if (result && typeof result === 'object' && 'then' in result) throw new Error('Catalog transactions must be synchronous');
        validateReferences(database);
        syncNormalizedTables(database);
        database.exec('COMMIT');
        return result;
      } catch (error) {
        database.exec('ROLLBACK');
        throw error;
      } finally {
        inTransaction = false;
      }
    },

    close() {
      database.close();
    }
  };
}

function readReview(database: DatabaseSync, id: RecordId): ReviewItem | undefined {
  const row = database.prepare(`SELECT r.id, r.record_id, r.field, r.current_value_json, r.record_revision, r.state, r.created_at, c.claim_json
    FROM review_items r JOIN field_claims c ON c.id = r.id WHERE r.id = ?`).get(id) as {
      id: string; record_id: string; field: string; current_value_json: string | null; record_revision: number;
      state: 'pending' | 'accepted' | 'rejected'; created_at: string; claim_json: string;
    } | undefined;
  return row ? {
    id: row.id,
    targetRecordId: row.record_id,
    field: row.field,
    claim: JSON.parse(row.claim_json) as FieldClaim,
    state: row.state,
    createdAt: row.created_at,
    recordRevision: row.record_revision,
    ...(row.current_value_json === null ? {} : { currentValue: JSON.parse(row.current_value_json) as unknown })
  } : undefined;
}
function validateRelationship(source: CatalogRecord,target: CatalogRecord,relationship: string) {
  if(['artist','crew','label','event'].includes(relationship) && source.kind!=='mix') throw new Error('Relationship source must be a mix');
  if(relationship==='member' && (source.kind!=='entity'||target.kind!=='entity'||!target.roles.includes('artist')||!source.roles.some((role)=>role==='artist'||role==='crew'))) throw new Error('Membership relationship requires artist/crew and artist records');
  if(['parent-label','sub-label'].includes(relationship) && (source.kind!=='entity'||target.kind!=='entity'||!source.roles.includes('label')||!target.roles.includes('label'))) throw new Error('Label relationship requires label records');
  if(!['artist','crew','label','event','member','parent-label','sub-label'].includes(relationship)) throw new Error('Relationship type is unsupported');
}
function validateReferences(database: DatabaseSync) {
  const rows=database.prepare('SELECT * FROM catalog_records').all() as StoredRecord[];
  const records=new Map(rows.map((row)=>{const record=parseRecord(row)!;return [record.id,record];}));
  const check=(source:CatalogRecord,id:string,kind:string,role?:EntityRole)=>{const target=records.get(id);if(!target||target.kind!==kind||(role&&(target.kind!=='entity'||!target.roles.includes(role)))) throw new Error(`Relationship reference ${id} has an unknown record or wrong role`);if(source.id===id) throw new Error('Relationship cannot reference itself');if(source.kind==='mix'&&source.verification!=='proposed'&&target.verification==='proposed')throw new Error('Relationship target identity must be confirmed before selecting its link');};
  for(const source of records.values()) {
    if(source.kind==='mix') {for(const person of source.people) check(source,person.entityId,'entity',person.role);for(const id of source.eventIds) check(source,id,'event');}
    if(source.kind==='event') for(const id of source.mixIds) check(source,id,'mix');
    if(source.kind==='entity') {for(const id of source.artist?.groupIds??[]) check(source,id,'entity','artist');for(const id of [...source.artist?.memberIds??[],...source.crew?.memberIds??[]]) check(source,id,'entity','artist');if(source.label?.parentId) check(source,source.label.parentId,'entity','label');for(const id of source.label?.subLabelIds??[]) check(source,id,'entity','label');}
  }
  const relationships=database.prepare('SELECT source_id,target_id,relationship FROM record_relationships').all() as Array<{source_id:string;target_id:string;relationship:string}>;
  for(const link of relationships) {const source=records.get(link.source_id),target=records.get(link.target_id);if(!source||!target) throw new Error('Relationship references an unknown record');validateRelationship(source,target,link.relationship);if(['artist','crew','label'].includes(link.relationship)) check(source,target.id,'entity',link.relationship as EntityRole);if(link.relationship==='event')check(source,target.id,'event');}
}

function syncNormalizedTables(database:DatabaseSync) {
  for(const table of ['mixes','events','entity_details','entity_aliases','record_terms','record_links','record_assets','selected_evidence'])database.exec(`DELETE FROM ${table}`);
  const records=(database.prepare('SELECT * FROM catalog_records').all() as StoredRecord[]).map(row=>parseRecord(row)!);
  const relation=(source:string,target:string,type:string)=>database.prepare('INSERT OR IGNORE INTO record_relationships(source_id,target_id,relationship) VALUES(?,?,?)').run(source,target,type);
  const link=(id:string,type:string,url:string)=>database.prepare('INSERT OR IGNORE INTO record_links(record_id,kind,url) VALUES(?,?,?)').run(id,type,url);
  for(const record of records) {
    for(const asset of record.assets)database.prepare('INSERT INTO record_assets(record_id,role,url,source,media_id) VALUES(?,?,?,?,?)').run(record.id,asset.role,asset.url,asset.source,asset.mediaId??null);
    for(const [field,id] of Object.entries(record.selectedEvidence??{}))database.prepare('INSERT INTO selected_evidence(record_id,field,claim_id) VALUES(?,?,?)').run(record.id,field,id);
    if(record.kind==='mix'){
      database.prepare('INSERT INTO mixes VALUES(?,?,?,?,?,?)').run(record.id,record.title,record.description??null,record.durationMs??null,record.recordingDate?.value??null,record.recordingDate?.precision??null);
      for(const [kind,values] of [['genre',record.genres],['style',record.styles]] as const)for(const value of values)database.prepare('INSERT OR IGNORE INTO record_terms VALUES(?,?,?,?)').run(record.id,kind,value,normalizeName(value));
      for(const person of record.people)relation(record.id,person.entityId,person.role);for(const id of record.eventIds)relation(record.id,id,'event');for(const url of record.playbackUrls??[])link(record.id,'playback',url);
    }else if(record.kind==='event'){
      database.prepare('INSERT INTO events VALUES(?,?,?,?,?,?,?,?,?)').run(record.id,record.name,record.startDate?.value??null,record.startDate?.precision??null,record.endDate?.value??null,record.endDate?.precision??null,record.venue??null,record.locality??null,record.country??null);
      for(const url of record.sourceUrls)link(record.id,'source',url);for(const id of record.mixIds)relation(id,record.id,'event');
    }else{
      for(const alias of record.aliases)database.prepare('INSERT OR IGNORE INTO entity_aliases VALUES(?,?,?)').run(record.id,alias,normalizeName(alias));
      for(const role of record.roles){const details=record[role];database.prepare('INSERT INTO entity_details VALUES(?,?,?,?,?)').run(record.id,role,role==='artist'?record.artist?.realName??null:null,details?.profile??record.profile??null,details?.country??record.country??null);if(role!=='artist')for(const url of record[role]?.websiteUrls??[])link(record.id,`${role}-website`,url);}
      for(const id of record.artist?.groupIds??[])relation(id,record.id,'member');for(const id of [...record.artist?.memberIds??[],...record.crew?.memberIds??[]])relation(record.id,id,'member');if(record.label?.parentId)relation(record.id,record.label.parentId,'parent-label');for(const id of record.label?.subLabelIds??[])relation(record.id,id,'sub-label');
    }
  }
}
