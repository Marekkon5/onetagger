CREATE TABLE field_claims (
  id TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL UNIQUE,
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  field TEXT NOT NULL,
  claim_json TEXT NOT NULL CHECK (json_valid(claim_json)),
  disposition TEXT NOT NULL CHECK (disposition IN ('selected', 'corroborated', 'pending', 'rejected')),
  created_at TEXT NOT NULL
);

CREATE TABLE review_items (
  id TEXT PRIMARY KEY REFERENCES field_claims(id) ON DELETE CASCADE,
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  field TEXT NOT NULL,
  current_value_json TEXT CHECK (current_value_json IS NULL OR json_valid(current_value_json)),
  record_revision INTEGER NOT NULL CHECK (record_revision > 0),
  state TEXT NOT NULL CHECK (state IN ('pending', 'accepted', 'rejected')),
  created_at TEXT NOT NULL,
  decided_at TEXT,
  actor TEXT
);

CREATE TABLE review_decisions (
  id INTEGER PRIMARY KEY,
  review_id TEXT NOT NULL REFERENCES review_items(id) ON DELETE CASCADE,
  decision TEXT NOT NULL CHECK (decision IN ('accepted', 'rejected')),
  actor TEXT NOT NULL,
  decided_at TEXT NOT NULL
);

CREATE INDEX field_claims_record ON field_claims(record_id, field);
CREATE INDEX review_items_state_created ON review_items(state, created_at);

INSERT INTO schema_migrations(version, applied_at) VALUES (2, CURRENT_TIMESTAMP);