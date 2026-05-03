-- Migration 0005: Fix FTS5 triggers to use COALESCE for nullable fields.
--
-- The original triggers (0003_fts5.sql) passed NULL directly to the FTS5
-- 'delete' operation. FTS5 indexes NULLs as empty strings but the 'delete'
-- command receives a NULL, causing a term-frequency mismatch that corrupts
-- the FTS5 index and manifests as SQLITE_CORRUPT (11) on subsequent writes.
--
-- Fix: wrap all nullable columns in COALESCE(col, '') so the values passed
-- to the 'delete' command match what was originally indexed.
-- Also rebuilds the index to clear any existing corruption.

DROP TRIGGER IF EXISTS notes_ai;
DROP TRIGGER IF EXISTS notes_ad;
DROP TRIGGER IF EXISTS notes_au;

CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, name, lede, why, content)
    VALUES (new.rowid, COALESCE(new.name,''), COALESCE(new.lede,''), COALESCE(new.why,''), COALESCE(new.content,''));
END;

CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, name, lede, why, content)
    VALUES ('delete', old.rowid, COALESCE(old.name,''), COALESCE(old.lede,''), COALESCE(old.why,''), COALESCE(old.content,''));
END;

CREATE TRIGGER notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, name, lede, why, content)
    VALUES ('delete', old.rowid, COALESCE(old.name,''), COALESCE(old.lede,''), COALESCE(old.why,''), COALESCE(old.content,''));
    INSERT INTO notes_fts(rowid, name, lede, why, content)
    VALUES (new.rowid, COALESCE(new.name,''), COALESCE(new.lede,''), COALESCE(new.why,''), COALESCE(new.content,''));
END;

-- Rebuild index to clear any corruption from the previous NULL-passing triggers.
INSERT INTO notes_fts(notes_fts) VALUES('rebuild');
