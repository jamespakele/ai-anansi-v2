-- Migration 0003: FTS5 full-text search index over notes
--
-- Creates a content-backed FTS5 virtual table so that anansi_search
-- can do fast full-text matching across name, lede, why, AND content
-- instead of a slow LIKE table scan.
--
-- "content=notes" means FTS5 stores only a shadow index; the actual
-- text lives in the notes table and is fetched via content_rowid.
-- Triggers below keep the index in sync on every write.

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    name,
    lede,
    why,
    content,
    content=notes,
    content_rowid=rowid
);

-- Populate index from existing rows
INSERT INTO notes_fts(rowid, name, lede, why, content)
    SELECT rowid, name, lede, why, content FROM notes;

-- Keep in sync: INSERT
CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, name, lede, why, content)
    VALUES (new.rowid, new.name, new.lede, new.why, new.content);
END;

-- Keep in sync: DELETE
CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, name, lede, why, content)
    VALUES ('delete', old.rowid, old.name, old.lede, old.why, old.content);
END;

-- Keep in sync: UPDATE
CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, name, lede, why, content)
    VALUES ('delete', old.rowid, old.name, old.lede, old.why, old.content);
    INSERT INTO notes_fts(rowid, name, lede, why, content)
    VALUES (new.rowid, new.name, new.lede, new.why, new.content);
END;
