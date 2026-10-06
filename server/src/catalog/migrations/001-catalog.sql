CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE catalog_records (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('mix', 'entity', 'event')),
  verification TEXT NOT NULL CHECK (verification IN ('proposed', 'source-confirmed', 'curator-confirmed')),
  revision INTEGER NOT NULL CHECK (revision > 0),
  search_name TEXT NOT NULL,
  payload TEXT NOT NULL CHECK (json_valid(payload)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE entity_roles (
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('artist', 'crew', 'label')),
  PRIMARY KEY (record_id, role)
);

CREATE TABLE entity_names (
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('artist', 'crew', 'label')),
  search_key TEXT NOT NULL,
  PRIMARY KEY (record_id, role, search_key)
);

CREATE TABLE provider_sources (
  provider TEXT NOT NULL,
  resource_type TEXT NOT NULL,
  external_id TEXT NOT NULL,
  url TEXT,
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  added_at TEXT NOT NULL,
  UNIQUE (provider, resource_type, external_id),
  UNIQUE (provider, url)
);

CREATE TABLE record_relationships (
  source_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  target_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  relationship TEXT NOT NULL,
  PRIMARY KEY (source_id, target_id, relationship),
  CHECK (source_id <> target_id)
);

CREATE TABLE record_aliases (
  legacy_id TEXT PRIMARY KEY,
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL
);

CREATE INDEX catalog_records_kind_verification ON catalog_records(kind, verification, updated_at DESC);
CREATE INDEX catalog_records_search_name ON catalog_records(search_name);
CREATE INDEX entity_names_lookup ON entity_names(role, search_key);
CREATE INDEX provider_sources_record ON provider_sources(record_id);
CREATE INDEX record_relationships_target ON record_relationships(target_id, relationship);
CREATE INDEX record_aliases_record ON record_aliases(record_id);

INSERT INTO schema_migrations(version, applied_at) VALUES (1, CURRENT_TIMESTAMP);