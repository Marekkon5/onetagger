-- Deployment-wide rights confirmation. A single operator attests once, so the
-- stored consent for every import uses `global_rights_confirmation` instead of
-- asking for the same per-mixset choice again.
-- SQLite cannot alter a CHECK constraint, so rebuild the child table and keep
-- the existing rows and index.
CREATE TABLE rights_consents_v2 (
  id TEXT PRIMARY KEY,
  job_id TEXT REFERENCES import_jobs(id) ON DELETE CASCADE,
  requested_by TEXT NOT NULL,
  basis TEXT NOT NULL CHECK (basis IN (
    'provider_metadata_only','user_authorized_copy','licensed_archive',
    'rights_holder','separate_agreement','global_rights_confirmation')),
  provider TEXT NOT NULL,
  source_url TEXT,
  attestation_version TEXT NOT NULL,
  proof_object_key TEXT,
  granted_at TEXT NOT NULL,
  expires_at TEXT,
  revoked_at TEXT
);

INSERT INTO rights_consents_v2 (
  id, job_id, requested_by, basis, provider, source_url, attestation_version,
  proof_object_key, granted_at, expires_at, revoked_at
)
SELECT
  id, job_id, requested_by, basis, provider, source_url, attestation_version,
  proof_object_key, granted_at, expires_at, revoked_at
FROM rights_consents;

DROP TABLE rights_consents;
ALTER TABLE rights_consents_v2 RENAME TO rights_consents;
CREATE INDEX IF NOT EXISTS rights_consents_job ON rights_consents(job_id, basis);
