-- Build-13: per-note last-access tracking for the anansi-crawl maintenance pass
-- and (later) tipping-point eviction. Nullable TEXT holding an rfc3339 timestamp,
-- bumped only by the user-facing MCP read tools.
ALTER TABLE notes ADD COLUMN last_accessed_at TEXT;

-- Backfill existing rows with the migration-run time: a uniform "everything was
-- present at start" epoch, so pre-existing notes all sort as equally-coldest
-- until real reads differentiate them. Format matches chrono's to_rfc3339()
-- (UTC, microseconds, +00:00 offset) so string ordering stays consistent with
-- values the application writes later.
UPDATE notes
SET last_accessed_at = to_char((now() AT TIME ZONE 'UTC'), 'YYYY-MM-DD"T"HH24:MI:SS.US"+00:00"')
WHERE last_accessed_at IS NULL;
