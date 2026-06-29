-- Build-10 fix: persistent tender run log
CREATE TABLE tender_log (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  notes_processed BIGINT NOT NULL DEFAULT 0,
  edges_removed   BIGINT NOT NULL DEFAULT 0,
  duplicates_merged BIGINT NOT NULL DEFAULT 0,
  wikilinks_fixed BIGINT NOT NULL DEFAULT 0,
  flags_inserted  BIGINT NOT NULL DEFAULT 0,
  errors          BIGINT NOT NULL DEFAULT 0,
  dry_run         BOOLEAN NOT NULL DEFAULT true,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_tender_log_created ON tender_log(created_at);

COMMENT ON TABLE tender_log IS 'Web Tender run log — one row per completed pass';
