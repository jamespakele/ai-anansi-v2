-- Migration 0004: drop the toc_text column from sources.
-- The TOC content is already stored in the outline note's content field,
-- making toc_text a large duplicated column with no additional value.
ALTER TABLE sources DROP COLUMN toc_text;
