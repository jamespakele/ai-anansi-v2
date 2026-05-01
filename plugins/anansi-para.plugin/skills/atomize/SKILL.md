---
name: atomize
description: >
  Full atomization pipeline for any source document. Single-command
  orchestrator that runs two sequential passes and produces one output
  file ready for Anansi database ingestion.
  Pass 1: para-pipeline → typed PARA TOC (para-extract → resource-typer → para-toc).
  Pass 2: Smart Brevity TOC atomization → content_toc blocks
  (all note types; encyclopedic, graph-aware).
  Output: {source-slug}-atomized.md containing the full TOC block set
  delimited for parser ingestion.
  Triggers: "atomize [file]", "/anansi-atomize [file]",
  "run the atomization pipeline on [file]",
  "process [file] for anansi", "full atomize [file]".
argument-hint: "[file path or pasted source text]"
---

# anansi-atomize

Single-command orchestrator for the full Anansi atomization pipeline.
Two passes, one output file.

This skill is a pure orchestrator. Do **not** inline or re-implement any
sub-skill logic. Read each sub-skill's SKILL.md in full and execute it exactly.
The sub-skill is always the source of truth — this file only wires them together.

**Finding sub-skill paths:** The base directory for this skill is provided in the
invocation context (the line beginning "Base directory for this skill:"). Resolve
sibling paths by replacing `atomize` with the sub-skill name. Example: if the base
directory ends in `.../skills/atomize`, then para-pipeline lives at
`.../skills/para-pipeline/SKILL.md`.

---

## When to invoke

Trigger when the user:

- Says "atomize [file]", "/anansi-atomize", "run the atomization pipeline"
- Says "process [file] for anansi", "full atomize", "prepare [file] for ingestion"
- Wants a single command that produces the complete Anansi-ready output

Do not invoke for individual pipeline steps. Use `para-pipeline` or
`smart-brevity` standalone if the user wants to inspect or modify between steps.

---

## Inputs

- **Source file or text** — required. File path or pasted content.
- **Source slug** — derived from filename or title for output naming.

---

## Execution: Two Sequential Passes

### Pass 1 — PARA pipeline

1. Use the Read tool to load `../para-pipeline/SKILL.md`.
2. Execute para-pipeline **exactly** as its SKILL.md specifies, using the source
   document as input. This runs para-extract → resource-typer → para-toc in full.
3. Hold the complete typed PARA TOC output as **PARA_TOC**.
   Do not show it to the user unless they ask to see intermediate results.

---

### Pass 2 — Smart Brevity TOC atomization

1. Use the Read tool to load `../smart-brevity/SKILL.md`.
2. Execute smart-brevity in **TOC Atomization Mode** exactly as its SKILL.md
   specifies, passing:
   - The original source document
   - PARA_TOC from Pass 1
3. Hold the full block output as **TOC_BLOCKS**.

---

## Output file

Assemble the final output file from TOC_BLOCKS.

### Structure

```
<!-- anansi-atomize: {source title} | {N} toc-blocks | {date} -->

{TOC_BLOCKS — full block set, --- separated}

<!-- concepts: #tag1 #tag2 ... -->
```

### Rules

- Contains all N blocks from TOC_BLOCKS in TOC address order
- Covers all PARA sections: projects (1.x), areas (2.x), discussion (3.x), resources (4.x)
- Closes with the `<!-- concepts: ... -->` tag from the TOC atomization pass
- Parser maps all blocks to `content_toc` field in the DB
- N = total decimal-addressed TOC entries across all sections

---

## File naming and output

Save the file as: `{source-slug}-atomized.md`

Where `{source-slug}` is the kebab-case filename without extension.
Example: source `smart-brevity.txt` → `smart-brevity-atomized.md`

Write to the same directory as the source file, or to the workspace
folder if source was pasted inline.

Show the user:
- The output file path
- Block count: N toc-blocks

---

## Parser contract

The output file is designed for mechanical splitting by a code parser.
The following delimiters are stable and must not be altered:

| Delimiter | Meaning |
|---|---|
| `<!-- anansi-atomize: ... -->` | File header — first line |
| `---` (alone on a line) | Block separator |
| `### Edges` | Edge section header — content goes to edge DB, not `content` field |
| `<!-- concepts: ... -->` | End of block section — last line |

Block header format (stable for parser regex):
```
### {address} {title} [{type-tag}]
```

The first non-empty line after the block header is the `lede`.
The line beginning with `**Why it matters:**` is the `why`.
Lines under `### Edges` are edge records in the format `- [relationship_verb]: [entity_type]:[slug]`.

**Parser field routing:**
- `lede` (first non-empty line after block header) → `lede` field in DB; NOT included in `content`
- `why` (line starting with `**Why it matters:**`) → `why` field in DB; NOT included in `content`
- Lines after lede/why and before `### Edges` → `content` field in DB
- Lines under `### Edges` → edge DB (relationship_verb, source_slug, target_type, target_slug)
- `[person]` blocks never have a `### Edges` section — all relationships surface through convergence blocks

---

## Self-check before saving

- [ ] Pass 1 completed: PARA_TOC generated with all 5 sections
- [ ] Pass 2 completed: TOC_BLOCKS generated — one block per TOC address
- [ ] TOC block count matches number of decimal-addressed entries in PARA_TOC
- [ ] Every TOC block ends with `---`
- [ ] Convergence blocks (organization, note, project, area) have `### Edges` where relationships are inferable
- [ ] `[person]` blocks have NO `### Edges` section
- [ ] No empty `### Edges` sections present
- [ ] Edge slugs match the exact slug in the referenced entity's block header
- [ ] `<!-- concepts: ... -->` present after last TOC block
- [ ] File saved as `{source-slug}-atomized.md`
