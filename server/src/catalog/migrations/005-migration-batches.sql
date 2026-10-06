CREATE TABLE migration_batches (
  batch_id TEXT PRIMARY KEY,
  result_json TEXT NOT NULL CHECK (json_valid(result_json)),
  created_at TEXT NOT NULL
);