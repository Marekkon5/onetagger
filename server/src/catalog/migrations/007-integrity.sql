CREATE INDEX field_claims_disposition ON field_claims(record_id, disposition);
CREATE INDEX media_assets_created ON media_assets(created_at);
INSERT INTO schema_migrations(version, applied_at) VALUES (7, CURRENT_TIMESTAMP);
