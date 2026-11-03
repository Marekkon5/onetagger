-- Evidence rows may carry the sourced value that produced the score. Finalize
-- reads this value instead of inventing one from a URL or artifact presence.
ALTER TABLE evidence_scores ADD COLUMN value_json TEXT;
