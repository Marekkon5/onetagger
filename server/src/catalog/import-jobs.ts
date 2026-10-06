import type { DatabaseSync } from 'node:sqlite';
import {
  ImportJobSchema,
  ScoredEvidenceSchema,
  type ImportJobState,
  type ImportJob,
  type RecordId,
  type ScoredEvidence,
} from '@syco23/catalog-domain';

type JobRow = {
  id: string; requested_by: string; provider: string;
  source_url: string; source_external_id: string | null;
  mode: string; state: string; idempotency_key: string;
  mix_id: string | null; attempt: number; error_code: string | null;
  error: string | null; created_at: string; started_at: string | null;
  finished_at: string | null; updated_at: string;
};

function rowToJobV2(row: JobRow): ImportJob {
  return ImportJobSchema.parse({
    id: row.id,
    requestedBy: row.requested_by,
    provider: row.provider,
    sourceUrl: row.source_url,
    ...(row.source_external_id ? { sourceExternalId: row.source_external_id } : {}),
    mode: row.mode,
    state: row.state,
    idempotencyKey: row.idempotency_key,
    ...(row.mix_id ? { mixId: row.mix_id } : {}),
    attempt: row.attempt,
    ...(row.error_code ? { errorCode: row.error_code } : {}),
    ...(row.error ? { error: row.error } : {}),
    createdAt: row.created_at,
    ...(row.started_at ? { startedAt: row.started_at } : {}),
    ...(row.finished_at ? { finishedAt: row.finished_at } : {}),
    updatedAt: row.updated_at,
  });
}

const SELECT_JOB_V2 = `SELECT id, requested_by, provider, source_url, source_external_id, mode, state, idempotency_key, mix_id, attempt, error_code, error, created_at, started_at, finished_at, updated_at FROM import_jobs`;

export function readImportJobV2(database: DatabaseSync, id: RecordId): ImportJob | undefined {
  const row = database.prepare(`${SELECT_JOB_V2} WHERE id = ?`).get(id) as JobRow | undefined;
  return row ? rowToJobV2(row) : undefined;
}

export function readImportJobV2ByKey(database: DatabaseSync, key: string): ImportJob | undefined {
  const row = database.prepare(`${SELECT_JOB_V2} WHERE idempotency_key = ?`).get(key) as JobRow | undefined;
  return row ? rowToJobV2(row) : undefined;
}

export function insertImportJobV2(database: DatabaseSync, job: ImportJob): ImportJob {
  const parsed = ImportJobSchema.parse(job);
  database.prepare(`INSERT INTO import_jobs(id, requested_by, provider, source_url, source_external_id, mode, state, idempotency_key, mix_id, attempt, error_code, error, created_at, started_at, finished_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
    .run(parsed.id, parsed.requestedBy, parsed.provider, parsed.sourceUrl, parsed.sourceExternalId ?? null, parsed.mode, parsed.state, parsed.idempotencyKey, parsed.mixId ?? null, parsed.attempt, parsed.errorCode ?? null, parsed.error ?? null, parsed.createdAt, parsed.startedAt ?? null, parsed.finishedAt ?? null, parsed.updatedAt);
  return parsed;
}

export function updateImportJobV2State(database: DatabaseSync, id: RecordId, state: ImportJobState, extra: { errorCode?: string | null; error?: string | null; mixId?: RecordId | null; attempt?: number } = {}): ImportJob {
  const current = readImportJobV2(database, id);
  if (!current) throw new Error(`Unknown import job ${id}`);
  const now = new Date().toISOString();
  const started = current.startedAt ?? (state !== 'created' && state !== 'policy_check' && state !== 'queued' ? now : undefined);
  const finished = state === 'completed' || state === 'blocked_policy' || state === 'failed' || state === 'cancelled';
  database.prepare(`UPDATE import_jobs SET state = ?, mix_id = ?, attempt = ?, error_code = ?, error = ?, started_at = COALESCE(started_at, ?), finished_at = ?, updated_at = ? WHERE id = ?`)
    .run(state, extra.mixId === undefined ? (current.mixId ?? null) : extra.mixId, extra.attempt ?? current.attempt, extra.errorCode === undefined ? (current.errorCode ?? null) : extra.errorCode, extra.error === undefined ? (current.error ?? null) : extra.error, started ?? null, finished ? now : null, now, id);
  const next = readImportJobV2(database, id);
  if (!next) throw new Error(`Unknown import job ${id}`);
  return next;
}

export function appendImportEventV2(database: DatabaseSync, jobId: RecordId, type: string, payload: Record<string, unknown> = {}): void {
  if (!readImportJobV2(database, jobId)) throw new Error(`Unknown import job ${jobId}`);
  const row = database.prepare('SELECT COALESCE(MAX(sequence), -1) AS max_seq FROM import_events WHERE job_id = ?').get(jobId) as { max_seq: number };
  const sequence = (row?.max_seq ?? -1) + 1;
  const now = new Date().toISOString();
  const id = `evt_${jobId.slice(0, 8)}_${String(sequence).padStart(6, '0')}_${Date.now().toString(36)}`;
  database.prepare('INSERT INTO import_events(id, job_id, sequence, type, payload_json, created_at) VALUES (?, ?, ?, ?, ?, ?)').run(id.slice(0, 128), jobId, sequence, type, JSON.stringify(payload), now);
}

export function enqueueImportJobV2(database: DatabaseSync, jobId: RecordId, delayMs = 0): void {
  const now = new Date();
  const nextRun = new Date(now.getTime() + Math.max(0, delayMs)).toISOString();
  database.prepare(`INSERT INTO import_outbox(job_id, attempt, next_run_at, created_at, updated_at) VALUES (?, 0, ?, ?, ?) ON CONFLICT(job_id) DO UPDATE SET next_run_at = excluded.next_run_at, updated_at = excluded.updated_at`).run(jobId, nextRun, now.toISOString(), now.toISOString());
}

export function claimDueImportJobV2(database: DatabaseSync, owner: string, leaseMs = 30000): ImportJob | undefined {
  const nowIso = new Date().toISOString();
  const row = database.prepare(`SELECT o.job_id AS job_id FROM import_outbox o JOIN import_jobs j ON j.id = o.job_id WHERE o.next_run_at <= ? AND (o.lease_expires_at IS NULL OR o.lease_expires_at <= ?) AND j.state IN ('queued','resolving','acquiring','probing','converting','tagging','enriching','scoring') ORDER BY o.next_run_at LIMIT 1`).get(nowIso, nowIso) as { job_id: string } | undefined;
  if (!row) return undefined;
  const leaseExpires = new Date(Date.now() + leaseMs).toISOString();
  const claimed = database.prepare(`UPDATE import_outbox SET lease_owner = ?, lease_expires_at = ?, updated_at = ? WHERE job_id = ? AND (lease_expires_at IS NULL OR lease_expires_at <= ?)`).run(owner, leaseExpires, nowIso, row.job_id, nowIso);
  if (Number(claimed.changes ?? 0) === 0) return undefined;
  return readImportJobV2(database, row.job_id);
}

export function completeImportOutboxV2(database: DatabaseSync, jobId: RecordId): void {
  database.prepare('DELETE FROM import_outbox WHERE job_id = ?').run(jobId);
}

export function renewImportLeaseV2(database: DatabaseSync, jobId: RecordId, owner: string, leaseMs = 60_000): boolean {
  const nowIso = new Date().toISOString();
  const expires = new Date(Date.now() + leaseMs).toISOString();
  const result = database.prepare(`UPDATE import_outbox SET lease_expires_at = ?, updated_at = ?
    WHERE job_id = ? AND lease_owner = ?`).run(expires, nowIso, jobId, owner);
  return Number(result.changes ?? 0) > 0;
}

export function listImportJobsV2(database: DatabaseSync, options: { state?: ImportJobState; limit?: number } = {}): ImportJob[] {
  const limit = Math.min(Math.max(1, options.limit ?? 50), 200);
  const rows = options.state
    ? database.prepare(`${SELECT_JOB_V2} WHERE state = ? ORDER BY created_at DESC LIMIT ?`).all(options.state, limit) as JobRow[]
    : database.prepare(`${SELECT_JOB_V2} ORDER BY created_at DESC LIMIT ?`).all(limit) as JobRow[];
  return rows.map(rowToJobV2);
}
export function registerImportArtifactV2(database: DatabaseSync, input: {
  id: RecordId; jobId: RecordId; role: string; objectKey: string; sha256: string;
  mimeType?: string | null; sizeBytes?: number | null; codec?: string | null;
  durationMs?: number | null; retentionUntil?: string | null;
}): void {
  database.prepare(`INSERT INTO import_artifacts(id, job_id, role, object_key, sha256,
    mime_type, size_bytes, codec, duration_ms, state, retention_until, created_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'ready', ?, ?)`)
    .run(input.id, input.jobId, input.role, input.objectKey, input.sha256,
      input.mimeType ?? null, input.sizeBytes ?? null, input.codec ?? null,
      input.durationMs ?? null, input.retentionUntil ?? null, new Date().toISOString());
}

export function saveProvenanceV2(database: DatabaseSync, jobId: RecordId, row: {
  provider: string; sourceUrl: string; externalId?: string | null;
  retrievalMethod: string; observedAt: string; termsVersion?: string | null;
  snapshot: Record<string, unknown>;
}): void {
  database.prepare(`INSERT INTO import_provenance(job_id, provider, source_url,
    external_id, retrieval_method, observed_at, terms_version, source_snapshot_json)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(job_id) DO UPDATE SET provider = excluded.provider,
    source_url = excluded.source_url, external_id = excluded.external_id,
    retrieval_method = excluded.retrieval_method, observed_at = excluded.observed_at,
    terms_version = excluded.terms_version,
    source_snapshot_json = excluded.source_snapshot_json`)
    .run(jobId, row.provider, row.sourceUrl, row.externalId ?? null,
      row.retrievalMethod, row.observedAt, row.termsVersion ?? null,
      JSON.stringify(row.snapshot));
}

export function saveRightsConsentV2(database: DatabaseSync, input: {
  id: RecordId; jobId: RecordId; requestedBy: string; basis: string;
  provider: string; sourceUrl?: string | null; attestationVersion: string;
  proofObjectKey?: string | null;
}): void {
  database.prepare(`INSERT INTO rights_consents(id, job_id, requested_by, basis,
    provider, source_url, attestation_version, proof_object_key, granted_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`)
    .run(input.id, input.jobId, input.requestedBy, input.basis, input.provider,
      input.sourceUrl ?? null, input.attestationVersion,
      input.proofObjectKey ?? null, new Date().toISOString());
}

export function saveEvidenceScoreV2(database: DatabaseSync, input: {
  id: RecordId; jobId: RecordId; claimId?: string | null; field: string;
  scored: ScoredEvidence;
}): ScoredEvidence & { id: RecordId; jobId: RecordId } {
  const scored = ScoredEvidenceSchema.parse(input.scored);
  database.prepare(`INSERT INTO evidence_scores(id, job_id, claim_id, field,
    score, algorithm_version, components_json, hard_gates_json, decision,
    evaluated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
    .run(input.id, input.jobId, input.claimId ?? null, input.field,
      scored.score, scored.algorithmVersion, JSON.stringify(scored.components),
      JSON.stringify(scored.hardGates), scored.decision, scored.evaluatedAt);
  return { ...scored, id: input.id, jobId: input.jobId };
}

export function listImportEventsV2(database: DatabaseSync, jobId: RecordId) {
  const rows = database.prepare(`SELECT id, job_id, sequence, type, payload_json,
    created_at FROM import_events WHERE job_id = ? ORDER BY sequence`)
    .all(jobId) as Array<{
      id: string; job_id: string; sequence: number; type: string;
      payload_json: string; created_at: string;
    }>;
  return rows.map((row) => ({
    id: row.id, jobId: row.job_id, sequence: row.sequence, type: row.type,
    payload: JSON.parse(row.payload_json) as Record<string, unknown>,
    createdAt: row.created_at,
  }));
}

export function listImportArtifactsV2(database: DatabaseSync, jobId: RecordId) {
  const rows = database.prepare(`SELECT id, job_id, role, object_key, sha256,
    mime_type, size_bytes, codec, duration_ms, state, created_at
    FROM import_artifacts WHERE job_id = ? ORDER BY created_at, id`)
    .all(jobId) as Array<{
      id: string; job_id: string; role: string; object_key: string;
      sha256: string; mime_type: string | null; size_bytes: number | null;
      codec: string | null; duration_ms: number | null; state: string;
      created_at: string;
    }>;
  return rows.map((row) => ({
    id: row.id, jobId: row.job_id, role: row.role, objectKey: row.object_key,
    sha256: row.sha256, mimeType: row.mime_type, sizeBytes: row.size_bytes,
    codec: row.codec, durationMs: row.duration_ms, state: row.state,
    createdAt: row.created_at,
  }));
}

export function listEvidenceScoresV2(database: DatabaseSync, jobId: RecordId) {
  const rows = database.prepare(`SELECT id, job_id, claim_id, field, score,
    algorithm_version, components_json, hard_gates_json, decision, evaluated_at
    FROM evidence_scores WHERE job_id = ? ORDER BY field, evaluated_at`)
    .all(jobId) as Array<{
      id: string; job_id: string; claim_id: string | null; field: string;
      score: number; algorithm_version: string; components_json: string;
      hard_gates_json: string; decision: string; evaluated_at: string;
    }>;
  return rows.map((row) => ({
    id: row.id, jobId: row.job_id, claimId: row.claim_id, field: row.field,
    score: row.score, algorithmVersion: row.algorithm_version,
    components: JSON.parse(row.components_json) as Record<string, unknown>,
    hardGates: JSON.parse(row.hard_gates_json) as Record<string, unknown>,
    decision: row.decision, evaluatedAt: row.evaluated_at,
  })) as ScoredEvidence[];
}


