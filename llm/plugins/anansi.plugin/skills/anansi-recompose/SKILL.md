---
name: anansi-recompose
description: >
  Recomposes the decomposed file. Pulls the source's outline note from the
  Anansi vault, walks it line by line, strips the address, looks up each
  named entity in the database, and appends the entity's lede / why /
  content underneath in TOC order. The outline becomes the recomposed
  file's TOC at the top; chapters build below it. Pure read-side from
  the database - never modifies the vault, never re-atomizes, never
  reads from disk artifacts. Input is a source identifier: slug,
  source_id, or source title. Output is markdown written to
  output/{slug}/{slug}-recompose.md. Use this when a source has been
  ingested and you want to read what got captured. Triggers:
  "anansi-recompose", "/anansi-recompose", "recompose this source",
  "rebuild this source", "rebuild from the vault", "show me the
  recomposed source", "give me the readable version of", "read what
  got ingested for".
argument-hint: "[source slug, source_id, or source title]"
---

# anansi-recompose

Recompose the decomposed file. Get the outline from the database. Walk it down. Insert lede / why / content from each named entity. The outline becomes the TOC; chapters build under it.

That's the entire job.

> **Database is the source.** This skill reads from the Anansi vault. It does not read from `output/` disk artifacts. Those are intermediate scaffolding from the ingest pipeline; the vault is the durable layer.
>
> **Read-only.** Never call `anansi_capture`, `anansi_relate`, `anansi_ingest_atomized`, or `anansi_purge` from this skill.
>
> **Skill-first compliance.** This skill is the proper anansi wrapper for read-side recomposition. Its body calls `anansi_search` and `anansi_get` as part of executing its purpose, exactly as `r2-remember` calls `anansi_ingest_atomized` for write-side ingest.

---

## When to invoke

When the user says any of:

- `anansi-recompose [source]`, `/anansi-recompose`
- "recompose this source", "rebuild this source", "rebuild from the vault"
- "show me the recomposed source", "give me the readable version of [source]"
- "read what got ingested for [source]"
- After an ingest, "now let me read it" or "show me what's in there"

## Do not invoke for

- Re-atomizing a source (use `sb-atomize`)
- Ingesting to the vault (use `r2-remember`)
- Reading from disk artifacts only with no vault dependency
- Sources that were never ingested into Anansi

---

## Input

One identifier, in this preference order:

1. **source_id** — exact match (e.g. `bh268-2026-04-15`)
2. **source slug** — kebab-case (e.g. `broadband-hui`)
3. **source title** — human-readable string

If multiple sources match, stop and list candidates with their `source_id`s; ask the user to disambiguate.

---

## Output

One file: `output/{slug}/{slug}-recompose.md`.

`{slug}` is derived from the source title (kebab-case) or falls back to `source_id`. Create the `output/{slug}/` directory if it does not exist. If a recompose file already exists at the path, append a timestamp suffix (`-recompose-{ISO-date}.md`) so the prior file is preserved.

---

## Process

### Step 1 — Resolve the source

Call `anansi_search` with the user's identifier. Filter to source records. Confirm a single match. Capture:

- `source_id`
- `source_title`
- `outline_note_id` (the note created by ingest holding the TOC)

If `outline_note_id` is missing (older ingest predating outline-note creation), stop and tell the user. v2 may add a contribution-walk fallback; v1 requires the outline note.

### Step 2 — Get the outline from the database

Call `anansi_get` on `outline_note_id`. The returned note's body is the unified TOC verbatim — same shape as `{slug}-toc.md` from sb-atomize. Sections (any may be absent):

```
## 0. PA Audit            (optional — if PA was empty)
## 1. Projects            (optional)
## 2. Areas               (optional)
## 3. Discussion          (optional)
## 4. Resources           (optional)
## Whispers               (optional)
## Concepts               (final)
```

Hold the body verbatim as `OUTLINE_BODY`.

### Step 3 — Walk the outline line by line

For every line matching the entity-line pattern:

```
- {address} [{type-tag}] {Name}
```

Where:
- `{address}` = `0.0`, `1.N`, `1.N.N`, `2.N`, `3.N`, `4.N`, `w.N`, etc.
- `{type-tag}` = `[type]` (e.g. `[discussion]`, `[note]`, `[whisper]`) or `[type:slug]` (e.g. `[person:nick-winfrey]`, `[organization:dhhl]`)
- `{Name}` = the human-readable name

For each matched line:

1. **Strip the address.** Drop the leading `- {address} `.
2. **Parse the type-tag.**
   - `[type:slug]` form → use `slug` as the canonical lookup key.
   - `[type]` form → use `Name` as the lookup key, scoped to this `source_id`.
3. **Look up the entity in the vault.**
   - Slug form: `anansi_get` by slug (or `anansi_search` filtered to this `source_id` plus the slug).
   - Name form: `anansi_search` filtered to this `source_id`, query the `Name`, type filter for disambiguation.
4. **Pull entity fields:** `lede`, `why` (may be absent), `content`, `edges` (if any).
5. **Append a chapter** to the in-memory recompose body (Step 4 format).

If a line matches the pattern but no entity is found in the vault, append the chapter with body `_(not found in vault)_` and add the line to a `not_found` list for the report. Do not silently drop.

Lines that do not match the entity pattern (section headings like `## 3. Discussion`, the Concepts list, blank lines) are ignored during the entity walk — they live in the rendered TOC at the top.

### Step 4 — Compose the recomposed file

Write with this exact shape:

```
---
source_id: {source_id}
source_title: {source_title}
outline_note_id: {outline_note_id}
generated_at: {now ISO with HST offset}
skill: anansi-recompose
chapters_rendered: {N}
chapters_not_found: {M}
---

# {source_title}

**Source ID:** {source_id}
**Outline note:** {outline_note_id}
**Recomposed:** {now ISO with HST offset}

---

## Table of Contents

{OUTLINE_BODY verbatim}

---

## Chapters

### {address} {Name} [{type}]

**Lede.** {entity.lede}

**Why.** {entity.why}              ← omit this paragraph entirely if entity has no why

**Content.**

{entity.content}

**Edges.**
- {edge_1}
- {edge_2}                          ← omit Edges block entirely if no edges

---

(repeat per TOC entry, in outline order; `---` separator between chapters)
```

The Table of Contents is the outline body verbatim — addresses, type tags, names, sub-project nesting all preserved exactly. Each chapter renders the entity straight from the database record.

### Step 5 — Write and report

Write the file. If the path exists, add a timestamp suffix and note the rename in the report.

Report back:

```
*Anansi-recompose* — {source_title}
• Output: {absolute path}
• Source ID: {source_id}
• Chapters rendered: {N}
• Chapters not found in vault: {M}    (omit line if M = 0)
• Renamed prior file: {old → new}     (omit if no rename happened)
```

If `chapters_not_found > 0`, list each missing entry's address + name beneath the report.

---

## Edge cases

| Situation | Action |
|---|---|
| Source not found by identifier | Stop. List partial matches if any. |
| Multiple sources match | Stop. List candidates with `source_id`. Ask user to disambiguate. |
| Source has no `outline_note_id` | Stop. Tell user; predates outline-note feature. |
| Outline note empty | Stop. Tell user; ingest may have failed mid-stream. |
| Entity in outline but missing from vault | Append `_(not found in vault)_` chapter, list in report. |
| Existing recompose file at output path | Add ISO-date suffix to filename, mention rename in report. |
| Slug ambiguous across sources | Filter by `source_id` first, then by slug. The pair must be unique. |
| Name ambiguous within source | Pick entity matching the type tag if provided. If still ambiguous, render all matches with a `**Note:** multiple vault matches` flag. |

---

## Self-check

- [ ] Source resolved from identifier?
- [ ] Outline note pulled from DB by `outline_note_id`?
- [ ] Outline body walked line by line?
- [ ] Each entity-line address stripped, type/name parsed?
- [ ] Each entity looked up in the vault, scoped to this `source_id`?
- [ ] Each chapter rendered with Lede / (Why if present) / Content / (Edges if present)?
- [ ] Missing entities surfaced as placeholders, not dropped?
- [ ] One file written to `output/{slug}/{slug}-recompose.md`?
- [ ] Frontmatter present with all metadata fields?
- [ ] Table of Contents is the outline body verbatim?
- [ ] No vault writes attempted — read-only run?
- [ ] Existing file preserved (timestamped) if one was already at the output path?

---

## Future work (out of scope for v1)

- **`source_position` on the contribution edge.** Server stores the TOC address (`3.2`, `4.7`, `1.1.1`) on each contribution edge during ingest. Recomposition then collapses to one SQL: `SELECT entity, lede, why, content FROM contributions WHERE source_id = ? ORDER BY source_position`. Single round trip, robust to outline-note deletion, no name ambiguity. Not needed for v1 because the outline-note walk already preserves order — but the right destination if the schema ever grows.
- **Contribution-walk fallback** for sources without an outline note.
- **Cross-ref resolution.** Inline-link discussion-block edges to chapter anchors in the same file.

---

## Why this skill exists

The decomposed file (sb-atomize output) is the form the server ingests. It's not the form a human reads. The vault holds the canonical version of every entity from every source; the outline note holds the TOC structure. This skill joins them back into the readable artifact the source originally was — turning a stored source into a consumable one.

The vault is backup memory. The recomposed file is what gets committed to human memory.
