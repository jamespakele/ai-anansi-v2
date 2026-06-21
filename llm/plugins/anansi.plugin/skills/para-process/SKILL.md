---
name: para-process
description: >
  Stage 1 orchestrator for the PARA atomization pipeline. Accepts a file
  path or pasted text and runs the same source through both
  para-projects-areas and para-resource-entities in parallel,
  producing the four paired files that downstream stages consume:
  projects-areas-toc.md, projects-areas-typed.md, resources-toc.md, and
  resources-typed.md. All four files share a single source_id and
  generated_at so the merge step can rejoin them. When the input is
  pasted text rather than a file, the raw text is first persisted to
  source.txt in the output directory so every later stage has a stable
  on-disk source to point at. Pure orchestrator — never inlines or
  re-implements the sub-skills. Triggers: "para-process [file]",
  "/para-process", "run para-process on this", "stage 1 atomize this",
  "PA + Resources extraction on this", "run para-projects-areas and
  para-resource-entities on this", "produce the four pipeline files
  for this source".
argument-hint: "[file path or pasted source text]"
---

# para-process

Stage 1 of the PARA atomization pipeline. One command in, four files out.

This skill is a **pure orchestrator**. Do not inline or re-implement any
sub-skill logic. Read each sub-skill's `SKILL.md` in full and execute it
exactly. The sub-skills are always the source of truth — this file only
wires them together and handles the file/text input split.

**Finding sub-skill paths:** the base directory for this skill is provided
in the invocation context. Resolve sibling paths by replacing
`para-process` with the sub-skill name. Example: if the base directory
ends in `.../skills/para-process`, then para-projects-areas lives at
`.../skills/para-projects-areas/SKILL.md` and para-resource-entities
lives at `.../skills/para-resource-entities/SKILL.md`.

---

## When to invoke

Trigger when the user:

- Says "para-process [file]", "/para-process", "run para-process on this"
- Says "stage 1 atomize this", "PA + Resources extraction on this"
- Says "run para-projects-areas and para-resource-entities on this"
- Wants the four paired stage-1 files (`projects-areas-toc.md`,
  `projects-areas-typed.md`, `resources-toc.md`, `resources-typed.md`)
  produced for a given source so a downstream stage (e.g.
  sb-atomize) can consume them
- Pastes raw text and wants it persisted as `source.txt` plus the four
  pipeline files

Do **not** invoke for:

- A single-skill run — call `para-projects-areas` or
  `para-resource-entities` directly.
- The full atomization pipeline — use `atomize` (which can call this
  skill as its first stage) or the legacy `para-pipeline`.
- Anansi writes — those happen in later stages.

---

## Inputs

- **Source** — required. One of:
  - **File path** to a markdown / text / transcript / email source.
  - **Pasted text** dropped directly into the conversation.
- **Output directory** — optional. Defaults to a sibling of the source
  file named after the source slug (see *File naming* below). If no file
  was supplied, defaults to a new directory under the workspace
  folder.

---

## Source slug

The source slug drives the output directory name and is the user-facing
identifier for the run.

- **File input** — slug = the source filename without extension,
  kebab-cased. Example: `Broadband Hui Notes.md` → `broadband-hui-notes`.
- **Pasted input** — slug = derived from the first non-empty heading or
  the first sentence (max 60 chars), kebab-cased. If nothing usable is
  available, fall back to `pasted-{ISO-8601-date}` — e.g.
  `pasted-2026-05-02`.

Kebab-case rules: lowercase, non-alphanumerics → spaces, collapse
whitespace, join with hyphens.

---

## Output directory layout

All four files plus (when applicable) `source.txt` land in a single
directory named after the slug:

```
<output-dir>/
  source.txt                     (only when input was pasted text)
  projects-areas-toc.md
  projects-areas-typed.md
  resources-toc.md
  resources-typed.md
```

If the source was a file, do **not** copy it — the four pipeline files
sit alongside the original (or in the explicit output directory the user
named). If the source was pasted text, persist it verbatim to
`source.txt` first so every downstream stage has a stable on-disk source
to read.

---

## Shared run identifiers

The four output files must share a single `source_id` and a single
`generated_at`. Both sub-skills compute these from their input — running
them on the same source produces the same `source_id` deterministically
because both define it as a stable hash of the trimmed input text. To
guarantee the values match:

1. Compute `source_id` once here (stable hash of trimmed input text)
   and `generated_at` once (current ISO-8601 timestamp).
2. Pass both values to each sub-skill invocation as a header / preamble
   so they emit them in their frontmatter rather than recomputing.
3. After both sub-skills return, verify the four files carry identical
   `source_id` and `generated_at` values. If they diverge, fail loudly
   — downstream merges depend on the join.

---

## Execution: two parallel passes

Pass A and Pass B are independent and **must run in parallel**. They
read the same source, share the same `source_id` / `generated_at`, and
write to disjoint output files — there is no data dependency between
them.

### Pass A — para-projects-areas

1. Read `../para-projects-areas/SKILL.md` in full.
2. Execute it exactly as specified, with the source content as input
   and the precomputed `source_id` / `generated_at` carried through.
3. Capture its two output files: `projects-areas-toc.md` and
   `projects-areas-typed.md`.

### Pass B — para-resource-entities

1. Read `../para-resource-entities/SKILL.md` in full.
2. Execute it exactly as specified, with the same source content and the
   same precomputed `source_id` / `generated_at`.
3. Capture its two output files: `resources-toc.md` and
   `resources-typed.md`.

Run Pass A and Pass B concurrently. Do not serialize. If the runtime
forces serialization, run them in either order — output is identical
because the passes do not coordinate.

---

## Resolving the input

1. If the user supplied a path, resolve it. If the path does not exist,
   stop and report.
2. If the user pasted text, capture it verbatim (preserve whitespace and
   line endings as given).
3. Trim trailing whitespace once for the purpose of computing
   `source_id`. Do not modify the source content otherwise — the
   sub-skills must see exactly what the user provided.
4. If the input is pasted text, write it to `<output-dir>/source.txt`
   **before** invoking either sub-skill, so both passes can read from
   that on-disk file rather than holding the text in memory across
   sub-skill boundaries.

---

## Self-check before reporting completion

- [ ] `source.txt` exists in the output directory iff the input was
  pasted text
- [ ] All four files exist:
  `projects-areas-toc.md`, `projects-areas-typed.md`,
  `resources-toc.md`, `resources-typed.md`
- [ ] All four files carry identical `source_id` in their frontmatter
- [ ] All four files carry identical `generated_at` in their frontmatter
- [ ] `projects-areas-toc.md` opens with `## 1. Projects` (or
  `_(none found)_` under it) and `## 2. Areas`
- [ ] `resources-toc.md` carries `## 3. Discussion`, `## 4. Resources`,
  and a `## Concepts` block (any may be empty per their own contracts)
- [ ] Discussion entry count matches the structural unit count of the
  source's content_unit floor template — one entry per floor-level unit
  (chapter, agenda topic, newsletter item, etc.), not per higher-level
  container. If the source has visible structural units and Discussion
  count is lower than the unit count, flag and re-run Pass B
- [ ] No file is empty; every file ends with a single trailing newline

If any check fails, surface the failure rather than silently producing a
partial run. Stage 2 (sb-atomize) consumes all four files and
will misbehave on a partial set.

---

## What to show the user

After both passes complete, report:

- The output directory path
- The four file paths (and `source.txt` when applicable)
- The shared `source_id` and `generated_at`
- A one-line count: *N projects, M areas, K resources, J concepts*
  (read from the typed and toc files)

Do not dump the file contents into chat unless the user asks. The files
are the deliverable.

---

## Failure modes

- **File path not found** — stop. Ask the user to confirm the path.
- **Pasted input is empty** — stop. Nothing to process.
- **Sub-skill failure** — surface the sub-skill's error verbatim. Do not
  fabricate a partial set of outputs.
- **`source_id` mismatch between the two passes** — re-run with the
  precomputed values explicitly passed through. If still mismatched,
  fail and report.
