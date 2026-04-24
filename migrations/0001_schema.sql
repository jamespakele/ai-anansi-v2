CREATE TABLE sources (
    id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    title TEXT,
    source_type TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    toc_hash TEXT,
    preprocessed_toc INTEGER NOT NULL DEFAULT 0,
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
    file_path TEXT NOT NULL,
    summary_1 TEXT,
    summary_5 TEXT,
    merge_category TEXT NOT NULL,
    created_from TEXT NOT NULL,
    source_count INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_from) REFERENCES sources(id)
);
CREATE INDEX idx_notes_match ON notes(match_key);
CREATE INDEX idx_notes_type ON notes(entity_type);

CREATE TABLE source_contributions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    toc_address TEXT,
    hint TEXT,
    contribution_type TEXT NOT NULL,
    payload TEXT,
    contributed_at TEXT NOT NULL,
    UNIQUE (source_id, note_id),
    FOREIGN KEY (source_id) REFERENCES sources(id),
    FOREIGN KEY (note_id) REFERENCES notes(id)
);
CREATE INDEX idx_contrib_source ON source_contributions(source_id);
CREATE INDEX idx_contrib_note ON source_contributions(note_id);

CREATE TABLE edges (
    id TEXT PRIMARY KEY,
    source_note_id TEXT NOT NULL,
    target_note_id TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    why TEXT,
    from_source TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    metadata TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (source_note_id, target_note_id, edge_type, from_source),
    FOREIGN KEY (source_note_id) REFERENCES notes(id),
    FOREIGN KEY (target_note_id) REFERENCES notes(id),
    FOREIGN KEY (from_source) REFERENCES sources(id)
);
CREATE INDEX idx_edges_source ON edges(source_note_id);
CREATE INDEX idx_edges_target ON edges(target_note_id);
CREATE INDEX idx_edges_type ON edges(edge_type);
