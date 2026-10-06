CREATE TABLE media_assets (
  media_id TEXT PRIMARY KEY,
  record_id TEXT NOT NULL REFERENCES catalog_records(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('waveform')),
  relative_path TEXT NOT NULL UNIQUE,
  mime_type TEXT NOT NULL CHECK (mime_type = 'image/png'),
  byte_size INTEGER NOT NULL CHECK (byte_size > 0 AND byte_size <= 8388608),
  source_url TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX media_assets_record ON media_assets(record_id, role);