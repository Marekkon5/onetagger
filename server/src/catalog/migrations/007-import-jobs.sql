-- Import control plane v2 (isolated): full audit job lifecycle, private artifacts,
-- provenance snapshots, rights consents, versioned evidence scores,
-- append-only events, durable outbox queue.
-- Additive only: no existing table is altered.
CREATE TABLE IF NOT EXISTS import_jobs (
  id TEXT PRIMARY KEY,
  requested_by TEXT NOT NULL,
  provider TEXT NOT NULL,
  source_url TEXT NOT NULL,
  source_external_id TEXT,
  mode TEXT NOT NULL CHECK (mode IN ('metadata', 'audio')),
  state TEXT NOT NULL CHECK (state IN (
    'created','policy_check','queued','resolving','acquiring','probing',
    'converting','tagging','enriching','scoring','review','completed',
    'blocked_policy','failed','cancelled')),
  idempotency_key TEXT NOT NULL UNIQUE,
  mix_id TEXT REFERENCES catalog_records(id) ON DELETE SET NULL,
  attempt INTEGER NOT NULL DEFAULT 0 CHECK (attempt >= 0 AND attempt <= 1000),
  error_code TEXT,
  error TEXT,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_artifacts (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN (
    'original','staging','normalized','artwork','waveform','raw','metadata')),
  object_key TEXT NOT NULL UNIQUE,
  sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
  mime_type TEXT,
  size_bytes INTEGER CHECK (size_bytes IS NULL OR (size_bytes > 0 AND size_bytes <= 4294967296)),
  codec TEXT,
  duration_ms INTEGER CHECK (duration_ms IS NULL OR (duration_ms > 0 AND duration_ms <= 86400000)),
  state TEXT NOT NULL DEFAULT 'ready' CHECK (state IN ('uploading','ready','verified','quarantined','expired')),
  retention_until TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_provenance (
  job_id TEXT PRIMARY KEY REFERENCES import_jobs(id) ON DELETE CASCADE,
  provider TEXT NOT NULL,
  source_url TEXT NOT NULL,
  external_id TEXT,
  retrieval_method TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  terms_version TEXT,
  source_snapshot_json TEXT NOT NULL CHECK (json_valid(source_snapshot_json))
);

CREATE TABLE IF NOT EXISTS rights_consents (
  id TEXT PRIMARY KEY,
  job_id TEXT REFERENCES import_jobs(id) ON DELETE CASCADE,
  requested_by TEXT NOT NULL,
  basis TEXT NOT NULL CHECK (basis IN (
    'provider_metadata_only','user_authorized_copy','licensed_archive',
    'rights_holder','separate_agreement')),
  provider TEXT NOT NULL,
  source_url TEXT,
  attestation_version TEXT NOT NULL,
  proof_object_key TEXT,
  granted_at TEXT NOT NULL,
  expires_at TEXT,
  revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS evidence_scores (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
  claim_id TEXT,
  field TEXT NOT NULL,
  score INTEGER NOT NULL CHECK (score >= 0 AND score <= 100),
  algorithm_version TEXT NOT NULL DEFAULT 'evidence-v1',
  components_json TEXT NOT NULL CHECK (json_valid(components_json)),
  hard_gates_json TEXT NOT NULL CHECK (json_valid(hard_gates_json)),
  decision TEXT NOT NULL CHECK (decision IN ('auto_apply','review','reject')),
  evaluated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_events (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL,
  type TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload_json)),
  created_at TEXT NOT NULL,
  UNIQUE (job_id, sequence)
);

CREATE TABLE IF NOT EXISTS import_outbox (
  job_id TEXT PRIMARY KEY REFERENCES import_jobs(id) ON DELETE CASCADE,
  attempt INTEGER NOT NULL DEFAULT 0,
  next_run_at TEXT NOT NULL,
  lease_owner TEXT,
  lease_expires_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS import_jobs_state ON import_jobs(state, created_at DESC);
CREATE INDEX IF NOT EXISTS import_jobs_provider ON import_jobs(provider, state);
CREATE INDEX IF NOT EXISTS import_jobs_idem ON import_jobs(idempotency_key);
CREATE INDEX IF NOT EXISTS import_artifacts_job ON import_artifacts(job_id, role);
CREATE INDEX IF NOT EXISTS import_artifacts_sha ON import_artifacts(sha256);
CREATE INDEX IF NOT EXISTS rights_consents_job ON rights_consents(job_id, basis);
CREATE INDEX IF NOT EXISTS evidence_scores_job ON evidence_scores(job_id, field);
CREATE INDEX IF NOT EXISTS import_events_job ON import_events(job_id, sequence);
CREATE INDEX IF NOT EXISTS import_outbox_due ON import_outbox(next_run_at);
