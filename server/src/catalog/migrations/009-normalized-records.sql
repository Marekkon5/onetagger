CREATE TABLE mixes(record_id TEXT PRIMARY KEY REFERENCES catalog_records(id) ON DELETE CASCADE,title TEXT NOT NULL,description TEXT,duration_ms INTEGER,recording_date TEXT,recording_precision TEXT);
CREATE TABLE events(record_id TEXT PRIMARY KEY REFERENCES catalog_records(id) ON DELETE CASCADE,name TEXT NOT NULL,start_date TEXT,start_precision TEXT,end_date TEXT,end_precision TEXT,venue TEXT,locality TEXT,country TEXT);
CREATE TABLE entity_details(record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,role TEXT NOT NULL CHECK(role IN ('artist','crew','label')),real_name TEXT,profile TEXT,country TEXT,PRIMARY KEY(record_id,role));
CREATE TABLE entity_aliases(record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,alias TEXT NOT NULL,search_key TEXT NOT NULL,PRIMARY KEY(record_id,search_key));
CREATE INDEX entity_aliases_lookup ON entity_aliases(search_key);
CREATE TABLE record_terms(record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,kind TEXT NOT NULL CHECK(kind IN ('genre','style')),value TEXT NOT NULL,search_key TEXT NOT NULL,PRIMARY KEY(record_id,kind,search_key));
CREATE TABLE record_links(record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,kind TEXT NOT NULL,url TEXT NOT NULL,PRIMARY KEY(record_id,kind,url));
CREATE TABLE record_assets(record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,role TEXT NOT NULL,url TEXT NOT NULL,source TEXT NOT NULL,media_id TEXT REFERENCES media_assets(media_id),PRIMARY KEY(record_id,role,url));
CREATE TABLE selected_evidence(record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,field TEXT NOT NULL,claim_id TEXT NOT NULL REFERENCES field_claims(id),PRIMARY KEY(record_id,field));
INSERT INTO schema_migrations(version,applied_at) VALUES(9,CURRENT_TIMESTAMP);

CREATE TABLE claim_fingerprint_aliases(fingerprint TEXT PRIMARY KEY,claim_id TEXT NOT NULL REFERENCES field_claims(id) ON DELETE CASCADE);
