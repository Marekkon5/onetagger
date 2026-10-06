-- Durable per-run enrichment log. A run is written before provider work starts so an
-- interrupted run (container restart, crash) is visible and retryable instead of the
-- record being left implicitly "enriching" with no trace.
CREATE TABLE enrichment_runs (
  id TEXT PRIMARY KEY,
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  state TEXT NOT NULL CHECK (state IN ('running', 'completed', 'interrupted', 'failed')),
  attempted_providers TEXT NOT NULL CHECK (json_valid(attempted_providers)),
  applied INTEGER NOT NULL DEFAULT 0 CHECK (applied >= 0),
  corroborated INTEGER NOT NULL DEFAULT 0 CHECK (corroborated >= 0),
  reviewed INTEGER NOT NULL DEFAULT 0 CHECK (reviewed >= 0),
  errors_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(errors_json)),
  missing_fields TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(missing_fields)),
  actor TEXT,
  started_at TEXT NOT NULL,
  finished_at TEXT
);

CREATE INDEX enrichment_runs_record_started ON enrichment_runs(record_id, started_at DESC);
CREATE INDEX enrichment_runs_state ON enrichment_runs(state, started_at DESC);

-- At most one unfinished run per record. A new run cannot start while an earlier one is
-- still 'running', which is what keeps a record from being marked as enriching forever.
CREATE UNIQUE INDEX enrichment_runs_single_running ON enrichment_runs(record_id) WHERE state = 'running';