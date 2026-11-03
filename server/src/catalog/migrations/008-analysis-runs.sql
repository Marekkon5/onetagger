CREATE TABLE IF NOT EXISTS analysis_runs (
  id TEXT PRIMARY KEY,
  record_id TEXT NOT NULL REFERENCES catalog_records(id),
  state TEXT NOT NULL CHECK(state IN ('queued','running','done','error','interrupted')),
  payload TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS analysis_record_state ON analysis_runs(record_id,state);
