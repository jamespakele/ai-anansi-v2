---
name: anansi-digest
description: >
  DEPRECATED - superseded by anansi-recompose. The earlier disk-driven
  digest skill that read from output/{slug}/ artifacts has been replaced
  by anansi-recompose, which reads from the Anansi vault directly (the
  durable layer) so it works for any ingested source whether or not the
  disk artifacts still exist. If the user says "anansi-digest", route
  to anansi-recompose. Triggers (legacy redirect): "anansi-digest",
  "/anansi-digest", "digest this source", "render the digest", "show me
  the digest".
argument-hint: "[source slug, source_id, or source title]"
---

# anansi-digest (deprecated)

This skill is superseded by **anansi-recompose**. The disk-driven digest path was wrong — it required `output/{slug}/` artifacts that may not exist (cleaned up, archived, or never created for async ingests like the email triage pipeline).

**If you were going to invoke anansi-digest, invoke `anansi-recompose` instead.** It does the right thing: pulls the outline note from the vault, walks it, and recomposes the source from the database.

Load and execute `../anansi-recompose/SKILL.md`, passing the user's input verbatim. Return the recompose skill's output unchanged.

That is the entire job of this skill — pure redirect.
