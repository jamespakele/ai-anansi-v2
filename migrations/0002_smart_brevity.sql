DELETE FROM source_contributions;
DELETE FROM edges;
DROP TABLE IF EXISTS notes;
CREATE TABLE notes (
    id                   TEXT    PRIMARY KEY,
    entity_type          TEXT    NOT NULL,
    name                 TEXT    NOT NULL,
    match_key            TEXT    NOT NULL UNIQUE,
    lede                 TEXT,
    why                  TEXT,
    content              TEXT,
    has_conflicts        INTEGER NOT NULL DEFAULT 0,
    conflicts_updated_at TEXT,
    merge_category       TEXT    NOT NULL DEFAULT '',
    created_from         TEXT    NOT NULL,
    source_count         INTEGER NOT NULL DEFAULT 0,
    created_at           TEXT    NOT NULL,
    updated_at           TEXT    NOT NULL,
    FOREIGN KEY (created_from) REFERENCES sources(id)
);
CREATE INDEX idx_notes_match_key     ON notes (match_key);
CREATE INDEX idx_notes_entity_type   ON notes (entity_type);
CREATE INDEX idx_notes_has_conflicts ON notes (has_conflicts);

DROP TABLE IF EXISTS embeddings;
CREATE TABLE embeddings (
    id          TEXT    PRIMARY KEY,
    note_id     TEXT    NOT NULL,
    model       TEXT    NOT NULL,
    dimensions  INTEGER NOT NULL,
    vector      BLOB    NOT NULL,
    created_at  TEXT    NOT NULL,
    UNIQUE (note_id, model),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
);
