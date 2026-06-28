---
name: anansi-remember
description: >
  Single entry point for committing anything to the Anansi knowledge base.
  Routes automatically across three paths: (A) single named entity or quick
  note → anansi-atom; (B) multi-entity raw text or document → para-process →
  sb-atomize → anansi_ingest_atomized; (C) already-atomized
  content → anansi_ingest_atomized directly. The server parses the
  atomized blocks and maps lede / why / content per entity type — never
  parse blocks in the skill. The user never picks a path. Triggers:
  "remember this", "remember [file]", "remember [name]", "remember that
  [X] is [Y]", "send to anansi", "ingest this", "commit to anansi", "save
  to anansi", "store in anansi", "put this in anansi", "add to the
  knowledge base", "/remember", "/anansi-remember", "quick note", "quick
  note:", "qn:", "jot this down", "just save this", "stash this", "note to
  self".
argument-hint: "[file path, pasted content, atomized .md, or entity facts]"
config:
  # processing: "server" — upload raw content to server inbox, fire and forget.
  #   The server runs the full pipeline (para-process → sb-atomize → ingest → wiki).
  #   Fastest path. No client-side LLM work. Best for large files, batches, URLs.
  #
  # processing: "local" — process everything client-side (current behavior).
  #   The agent extracts, identifies entities, compresses, then calls
  #   anansi_ingest_atomized + wiki-ingest-atomized. Slower but reviewable.
  processing: server
---

# anansi-remember

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

The single entry point for committing anything to the Anansi knowledge base.

The user says "remember this" — this skill figures out what it is and routes
to the right path. Three paths, one skill. The user never picks.

> **Skill-first.** Anansi MCP write calls go through this skill (or
> `anansi-atom` for the single-entity path), never direct.

> **Server-side parse.** For Path B and Path C, ingestion is a single
> `anansi_ingest_atomized` call. The server parses the `---`-delimited
> block set, maps each block's `## Lede` / `## Why` / `## Content` /
> `## Edges` into the right note fields per entity type, creates the
> outline note for document recomposition, and writes hierarchy edges.
> Do not parse blocks in this skill. Do not loop `anansi_capture` over
> blocks. The server handles it.

> **MCP namespace.** The Anansi MCP may surface under more than one
> prefix in the same session — e.g. `mcp__f36a3bbf-...__anansi_*` and
> `mcp__plugin_anansi_anansi__anansi_*`. Both route to the same
> Rust ingest binary and are functionally identical. Pick whichever is
> available; if both are connected, prefer the plugin-scoped one. The
> CLAUDE.md skill-first rule applies regardless of prefix — every
> `anansi_*` call still goes through this skill or `anansi-atom`.

---

## Routing — Step 1 (always run this first)

### Step 0 — Check processing mode

Read the `config.processing` flag from this skill's frontmatter:

| Mode | Behavior |
|---|---|
| `server` (default) | Upload raw content to server inbox. Fire and forget. The server runs the full pipeline. Fastest path. |
| `local` | Process client-side (current Path A/B/C). Reviewable but slower. |

**If `processing: server`:**

1. Read `../anansi-ingest-file/SKILL.md`.
2. Execute it with the same input (file path, URL, or pasted text).
3. Return its confirmation. **Stop here** — the server handles everything.

**If `processing: local`:**

Fall through to the Q0–Q3 classification below (current Path A/B/C behavior).

---

### Q0: Is this a quick note?

If the user's phrasing signals speed over structure — `quick note:`, `qn:`,
`jot this down`, `just save this`, `stash this`, `note to self` — skip
every other check and go straight to **Path A** in quick-note mode
(delegated to `anansi-atom`). Do not count entities. Do not check
atomization state. Just capture it fast.

### Q1: Is the input already atomized?

Look for the header line:

```
<!-- anansi-atomize: ... -->
```

If present → **Path C** (`anansi_ingest_atomized` directly).

If the user also handed over a `{slug}-toc.md` (frontmatter starts with
`source_id:` and `skill: sb-atomize`), keep it and pass it as
the optional `para_toc` argument.

### Q2: Is this a single named entity from conversation?

Apply the **entity-count rule**:

| Input | Entity count | Route |
|---|---|---|
| "remember John Doe, phone (808) 555-1234" | 1 (person) | **Path A** → `anansi-atom` |
| "remember JERA Americas, the LNG client" | 1 (organization) | **Path A** → `anansi-atom` |
| "remember the Waiʻanae Summit, May 15 at the community center" | 1 (event) | **Path A** → `anansi-atom` |
| "John Doe at ABC Corp, phone (808) 555-1234" | 2 (person + org) | **Path B** → pipeline |
| Meeting notes, email thread, transcript, article, file | N (many) | **Path B** → pipeline |
| A file path or document with no atomize header | N (many) | **Path B** → pipeline |

A single entity is one person / organization / event / project / note /
area with no other named entity in the same breath. **When in doubt,
count distinct named things. Two-or-more goes to Path B.**

---

## Path A — Single entity → anansi-atom

Load and execute `../anansi-atom/SKILL.md` exactly as specified, passing
the user's input as the entity description. `anansi-atom` handles type
detection, template lookup, Smart Brevity formatting, the `anansi_capture`
call, and the quick-note override.

Return `anansi-atom`'s confirmation to the user unchanged.

---

## Path B — Multi-entity or raw document → pipeline → ingest

### B-i — Persist source

Pick a **source slug** (kebab-case from filename or first heading; fallback
`pasted-{ISO-date}`).

Always write outputs to `output/{slug}/` (relative to the workspace root,
or the user's explicit folder if they named one). Create the directory if
it doesn't exist.

If the input was pasted text, write it to `output/{slug}/source.txt`
first so the pipeline has a stable on-disk source. Hold the resolved
absolute path as **SOURCE_PATH** for the eventual ingest call.

If the input was a file, leave it where it is. Hold its absolute path as
**SOURCE_PATH**.

### B-ii — Run para-process (Stage 1)

1. Read `../para-process/SKILL.md`.
2. Execute it on the source. It produces (in `output/{slug}/`):
   - `projects-areas-toc.md`
   - `projects-areas-typed.md`
   - `resources-toc.md`
   - `resources-typed.md`

### B-iii — Run sb-atomize (Stage 3)

1. Read `../sb-atomize/SKILL.md`.
2. Execute it with the source + the four pipeline files. It produces:
   - `output/{slug}/{slug}-atomized.md` — the `---`-delimited block set
     (this is the ingest payload)
   - `output/{slug}/{slug}-toc.md` — the unified manifest
     (this is the `para_toc` payload)

### B-iv — Local wiki write (optional, on by default)

If the local LLM wiki is enabled (default: yes), write the atomized content
to `~/llm-wiki/` before sending to the server:

1. Read `../wiki-ingest-atomized/SKILL.md`.
2. Execute it with `ATOMIZED_CONTENT` as input.

This writes each entity as a markdown file in `~/llm-wiki/` and updates
`index.md` / `log.md`. Pure file I/O — no server call.

### B-v — Ingest

Hold:

- **ATOMIZED_CONTENT** ← the in-memory text of `{slug}-atomized.md`
  (preferred; server skips a disk read)
- **PARA_TOC** ← the in-memory text of `{slug}-toc.md`
- **SOURCE_PATH** ← the absolute path from B-i

Call `anansi_ingest_atomized` with:

```
content      = ATOMIZED_CONTENT
para_toc     = PARA_TOC
source_path  = SOURCE_PATH
source       = "skill"
```

Do **not** also pass `path` — content is preferred and the schema treats
`path` as a fallback only.

If the call returns an error, surface it verbatim. The atomized file is
on disk (in `output/{slug}/`), so a manual retry is possible.

Proceed to [Report](#report).

---

## Path C — Already atomized → ingest directly

The user handed you a `{slug}-atomized.md` file (or pasted its contents)
and optionally the matching `{slug}-toc.md`.

1. Hold the atomized content as **ATOMIZED_CONTENT**.
2. If a TOC accompanies it, hold as **PARA_TOC**. Otherwise null.
3. If a source-document path is in context (e.g. from a prior pipeline
   run in the same conversation), hold as **SOURCE_PATH**. Otherwise null.
4. **Local wiki write (optional, on by default):** Read and execute
   `../wiki-ingest-atomized/SKILL.md` with `ATOMIZED_CONTENT` to
   write each entity to `~/llm-wiki/`.
5. Call `anansi_ingest_atomized` with whichever of the four arguments
   are non-null:
   - `content` = ATOMIZED_CONTENT (always)
   - `para_toc` = PARA_TOC (when available)
   - `source_path` = SOURCE_PATH (when available)
   - `source` = `"skill"` (always)
6. Proceed to [Report](#report).

If the user only gives you the TOC, stop and ask for the atomized file —
the TOC alone is a manifest, not ingestible content.

---

## Report

**For Path A (anansi-atom):** Return `anansi-atom`'s confirmation
directly. No additional wrapping.

**For Path B / C (`anansi_ingest_atomized`):**

```
*Anansi* — {source title from atomize header, or filename}
• Path: {B: source → atomized → ingested | C: pre-atomized → ingested}
• Output dir: {output/{slug}/ for Path B; n/a for Path C}
• Local wiki: ~/llm-wiki/ (written before server ingest)
• Notes created: {note_count from response}
• Source ID: {source_id from response}
• Outline note: {outline_note_id from response}
```

If the response includes any per-block warnings or skipped blocks,
list them beneath the report.

If `anansi_ingest_atomized` returns an error:

- Show the error verbatim.
- For Path B, the atomized file is at `output/{slug}/{slug}-atomized.md`
  on disk — offer to retry.
- For Path C, offer to write ATOMIZED_CONTENT to disk so the user can
  retry manually.

---

## Error handling

| Situation | Action |
|---|---|
| File path given but file not found | Stop. Confirm the path with the user. |
| Pre-process or sb-atomize fails | Stop. Report which stage failed and surface its error verbatim. Do not call ingest on partial output. |
| Atomized input has no `<!-- anansi-atomize: ... -->` header | Stop. This is not an atomized file. Re-route through Path B if it looks like a raw document, or ask the user to confirm. |
| `anansi_ingest_atomized` returns an error | Show error. Atomized file is on disk for Path B; offer disk-save for Path C. |
| Entity count is ambiguous (1 vs 2+) | Err toward Path B. The pipeline handles single entities fine and is more thorough. |
| User pasted only a TOC file | Stop. The TOC is a manifest, not content. Ask for the atomized file. |
| Anansi MCP not connected | Tell the user. Save the atomized output to disk so they can ingest later when the connector is back. |

---

## Context handoff

When `remember` is invoked immediately after a `para-process` →
`sb-atomize` run in the same conversation:

- ATOMIZED_CONTENT and PARA_TOC from that run are already in context —
  use them directly as Path C (skip re-running the pipeline).
- The output directory and original source path from the prior run are
  the report's `Output dir` and `source_path` argument.
- This makes "atomize → review → remember" a natural three-step flow
  with no redundant work.

---

## Why this skill exists

Renamed from `r2-remember`, with the same three-path routing and the same
server-side ingest tool. URL extraction (YouTube, articles) now lives in
`r2v2:r2-remember`, which calls this skill after extraction is done. The
server parses the atomized block set, applies the per-type field mapping
internally, creates the outline note, and writes hierarchy edges. The skill
stays out of the parsing business so field-mapping bugs can't happen on the
client side.

## Done

- [ ] Input classified correctly (quick note / atomized / single entity / multi-entity)
- [ ] Processing mode checked (`server` or `local`)
- [ ] If `processing: server`: file uploaded to inbox, confirmation received
- [ ] If `processing: local`: all pipeline stages completed without error
- [ ] If `processing: local`: `wiki-ingest-atomized` called before `anansi_ingest_atomized`
- [ ] Report generated with source title, path, notes created, source ID
