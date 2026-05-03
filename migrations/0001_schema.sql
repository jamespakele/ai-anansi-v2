-- Build-10: PostgreSQL schema (replaces all SQLite migrations 0001-0005)
-- Uses pgvector for semantic search and native tsvector for full-text search.

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE sources (
    id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    title TEXT,
    source_type TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    toc_hash TEXT,
    preprocessed_toc BIGINT NOT NULL DEFAULT 0,
    toc_author TEXT,
    toc_generated_at TEXT,
    ingested_at TEXT NOT NULL,
    toc_text TEXT
);
CREATE INDEX idx_sources_hash ON sources(content_hash);

CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    name TEXT NOT NULL,
    match_key TEXT NOT NULL UNIQUE,
    lede TEXT,
    why TEXT,
    content TEXT,
    has_conflicts BIGINT NOT NULL DEFAULT 0,
    conflicts_updated_at TEXT,
    merge_category TEXT NOT NULL,
    created_from TEXT NOT NULL REFERENCES sources(id),
    source_count BIGINT NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Full-text search vector: updated automatically on INSERT/UPDATE
    fts_vector tsvector GENERATED ALWAYS AS (
        to_tsvector('english',
            COALESCE(name, '') || ' ' ||
            COALESCE(lede, '') || ' ' ||
            COALESCE(why, '') || ' ' ||
            COALESCE(content, '')
        )
    ) STORED
);
CREATE INDEX idx_notes_match ON notes(match_key);
CREATE INDEX idx_notes_type ON notes(entity_type);
CREATE INDEX idx_notes_fts ON notes USING GIN(fts_vector);

CREATE TABLE note_embeddings (
    note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    embedding vector(768) NOT NULL,
    embedded_at TEXT NOT NULL,
    PRIMARY KEY (note_id, model)
);
-- HNSW index for fast approximate nearest-neighbour search (cosine distance)
CREATE INDEX idx_note_embeddings_hnsw ON note_embeddings
    USING hnsw (embedding vector_cosine_ops);

CREATE TABLE source_contributions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id),
    note_id TEXT NOT NULL REFERENCES notes(id),
    toc_address TEXT,
    hint TEXT,
    contribution_type TEXT NOT NULL,
    payload TEXT,
    contributed_at TEXT NOT NULL,
    UNIQUE (source_id, note_id)
);
CREATE INDEX idx_contrib_source ON source_contributions(source_id);
CREATE INDEX idx_contrib_note ON source_contributions(note_id);

CREATE TABLE edges (
    id TEXT PRIMARY KEY,
    source_note_id TEXT NOT NULL REFERENCES notes(id),
    target_note_id TEXT NOT NULL REFERENCES notes(id),
    edge_type TEXT NOT NULL,
    why TEXT,
    from_source TEXT NOT NULL REFERENCES sources(id),
    weight DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    metadata TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (source_note_id, target_note_id, edge_type, from_source)
);
CREATE INDEX idx_edges_source ON edges(source_note_id);
CREATE INDEX idx_edges_target ON edges(target_note_id);
CREATE INDEX idx_edges_type ON edges(edge_type);

-- Seed the manual-edges sentinel source (idempotent)
INSERT INTO sources (id, source_path, title, source_type, content_hash, ingested_at)
VALUES ('00000000-0000-0000-0000-000000000000', 'manual', 'Manual edges', 'manual', 'manual', NOW()::TEXT)
ON CONFLICT (id) DO NOTHING;
