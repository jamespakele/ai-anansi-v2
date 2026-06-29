-- Build-10: Web Tender queue table
-- Stores issues detected by the background maintenance crawler for review or
-- auto-resolution. Each row represents one finding — a dedup candidate, a
-- broken edge, an orphan node, etc.

CREATE TABLE tender_queue (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  category      TEXT NOT NULL,
  severity      TEXT NOT NULL DEFAULT 'info',
  match_key     TEXT,
  related_keys  TEXT[],
  description   TEXT,
  confidence    REAL,
  status        TEXT NOT NULL DEFAULT 'open',
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_at   TIMESTAMPTZ,
  resolved_by   TEXT
);

CREATE INDEX idx_tender_queue_status ON tender_queue(status);
CREATE INDEX idx_tender_queue_category ON tender_queue(category);
CREATE INDEX idx_tender_queue_severity ON tender_queue(severity);
CREATE INDEX idx_tender_queue_status_created ON tender_queue(status, created_at);

COMMENT ON TABLE tender_queue IS 'Web Tender queue: background maintenance crawler findings';
COMMENT ON COLUMN tender_queue.category IS 'One of: dedup, broken_edge, orphan, stale, circular, template_drift, edge_inference, type_consistency, conflicting_edges';
COMMENT ON COLUMN tender_queue.severity IS 'One of: low, medium, high, info';
COMMENT ON COLUMN tender_queue.status IS 'One of: open, resolved, dismissed';
COMMENT ON COLUMN tender_queue.match_key IS 'Primary entity match_key this finding relates to';
COMMENT ON COLUMN tender_queue.related_keys IS 'Additional entity match_keys involved (e.g. dedup pair, edge endpoints)';
COMMENT ON COLUMN tender_queue.confidence IS 'Confidence score 0.0–1.0 for the finding';
COMMENT ON COLUMN tender_queue.resolved_by IS 'Who or what resolved this item (skill name, user, or auto)';
