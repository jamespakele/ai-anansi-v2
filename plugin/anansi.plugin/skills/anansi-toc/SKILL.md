---
name: anansi-toc
description: >
  Produce an enriched Table of Contents for an anansi source document,
  splice it into frontmatter, and write the augmented file ready for
  daemon ingestion. Use when the user says "preprocess for anansi",
  "generate anansi TOC", "toc this for anansi", or drops a markdown
  file and asks to prepare it for decomposition.
---

## When to invoke

Trigger this skill when the user:
- Says "preprocess for anansi" or "generate anansi TOC"
- Says "toc this for anansi" or "prepare this for anansi ingestion"
- Drops a markdown file and asks to prepare it for decomposition
- Asks to run `/toc` on a file

## Prerequisites

The anansi root folder path must be configured or provided by the user. Templates
and `%Rules` live under `<anansi-root>/templates/` and `<anansi-root>/%Rules/`.

If `ANANSI_ROOT` is set in the environment, use that. Otherwise ask the user.

---

> **IMPORTANT:** The output TOC lines must match the parser regex exactly:
> `^\s*\d+(?:\.\d+)*\s+.+?\s+\[\w+\](?:\s*\|\s*\w+:[^|]*)*\s*$`
> Note: `(?:\.\d+)*` allows top-level addresses (`1`, `2`) as well as nested
> ones (`1.1`, `2.3.1`). Lines that don't match are silently dropped by the daemon.
> Do not produce markdown fences, preamble, or commentary — only valid leaf lines
> plus optional `# Section comment` lines for readability.

---

## Leaf format grammar

Each leaf line follows this exact format:

```
<address> <name> [<entity_type>] | <annotation>: <value> | <annotation>: <value>
```

Where:
- **address** — dotted decimal (e.g., `1.1`, `2.3.1`). Max 6 levels. No duplicates.
- **name** — the canonical name of the entity. No brackets, no pipes.
  - For tasks: append ` — <assignee>` (em-dash) if assignee is known.
- **[entity_type]** — one of the valid entity types listed below. Use `[?]` to mark an
  entity whose type is unclear — these are skipped by the daemon.
- **Annotations** (optional, `|`-separated after the entity type):
  - `hint: <text>` — a short extraction hint for the daemon.
  - `context_at: <address>,<address>` — comma-separated addresses where extra
    context for this entity appears in the source.

Valid entity types are determined by reading `<anansi-root>/templates/` in Step 3 —
each `.md` filename stem (e.g. `person.md` → `person`) is a valid type. The list
grows with each build; never use a hardcoded set here.

Examples of valid leaf lines:
```
1 Digital Futures Workshop [event] | hint: Strategic planning session at PICHTR
1.1 Ian Kitajima [person] | hint: see attendee list
1.2 PICHTR [organization]
1.3 Sovereign AI [concept] | context_at: 2.1,2.2
2.1 Deploy inference cluster [task] | hint: Q3 milestone — Ian Kitajima
2.2 Hawaii AI Policy Summit [event] | context_at: 1.1,1.3
```

---

## Step 1: Locate the anansi root

Check the environment variable `ANANSI_ROOT`. If set, use it as the root path.

If not set, ask the user: "What is your anansi root directory?" before proceeding.

Store the root as `<anansi-root>`.

## Step 2: Read the source file

Use the Read tool to read the full content of the source file (frontmatter + body).

Note any existing frontmatter fields, especially `source_type`, `title`,
`source_date`, and whether `anansi_toc` already exists (if so, confirm with the
user whether to regenerate).

## Step 3: Load templates and %Rules

Read the following files:
- `<anansi-root>/anansi/templates/*.md` — **list these files first** to get the current
  set of valid entity type names (filename stem = type name). Read their content
  for extraction field schemas and source-type hints.
- `<anansi-root>/anansi/%Rules/%Atomicity.md` — atomicity rules for entity decomposition.
- `<anansi-root>/anansi/%Rules/%Downstream-Flow.md` — flow rules for source-to-note
  contribution.

Do not use a hardcoded type list — the template set evolves with each build.
You do not need to read `%Merge-Strategy.md` or `%Template-Schema.md` for TOC
generation. Load those only if the user asks for deeper context.

## Step 4: Determine source_type

If `source_type` is already set in the source file frontmatter, use it.

Otherwise ask the user to choose one of:
`meeting_summary`, `research_paper`, `email_thread`, `container`

The source_type determines which template hints apply during Pass 3 extraction
inside the daemon. Choosing correctly improves extraction quality.

## Step 5: Assemble the Pass 1 prompt mentally

Combine mentally (do not output):
- The atomicity rules from `%Atomicity.md`
- The entity types and their schemas from the templates directory
- The leaf format grammar above
- The source body

This is the same logic the daemon runs in Pass 1 when no preprocessed TOC is
present. Your goal is to produce a TOC that is at least as good, which will let
the daemon skip Pass 1 entirely.

## Step 6: Produce the enriched TOC

Write a TOC covering all significant entities in the source. Follow these rules:

1. **Every leaf must be on its own line** in the exact format shown above.
2. **Addresses must be unique** and use dotted decimal notation.
3. **Entity types must be from the valid list.** Use `[?]` only as a last resort.
4. **Provide hints** whenever the entity's canonical name differs from how it
   appears in the text, or when there is relevant context the daemon should use
   during extraction.
5. **Use `context_at`** for entities whose significant context appears in
   sections other than where they are first mentioned.
6. **Tasks** should include assignees in the name when known: `Task name — Assignee`.
7. **Do not produce markdown fences, commentary, or preamble** — only the leaf
   lines (plus optional `# Section comment` lines for readability).
8. **Quality check:** Run every line mentally against the regex
   `^\s*\d+(?:\.\d+)*\s+.+?\s+\[\w+\](?:\s*\|\s*\w+:[^|]*)*\s*$` before
   including it. Drop any line that would not match.

Aim for completeness: it is better to have more leaves (and let the daemon's
merge logic handle duplicates) than to miss entities.

## Step 7: Splice into frontmatter

Construct the augmented source file. The frontmatter must include:

```yaml
---
anansi_toc_version: 1
anansi_toc: |
  <the TOC you produced, indented 2 spaces>
source_type: <source_type>
title: <title from source or inferred>
source_date: <ISO-8601 date if known, else omit>
toc_author: <model name, e.g. claude-opus-4-7>
toc_generated_at: <ISO-8601 timestamp>
---
```

Preserve any other frontmatter fields that were already present in the source.
Place the anansi-specific fields at the top of the frontmatter block.

The body of the file (everything after the closing `---`) must remain unchanged.

## Step 8: Write the augmented file

Use the Write tool to save the augmented file.

Default output path: `<anansi-root>/<source-slug>.md` (overwriting the original).

If the user wants to review before overwriting, offer an alternative path:
`<anansi-root>/<source-slug>.augmented.md`.

Ask the user which they prefer if they haven't specified.

## Step 9: Report

Print a short summary:
- The output path
- The number of TOC leaves produced
- The entity types found

Then offer the bash command the user can run to ingest immediately:

```bash
anansi2 ingest <output-path> --root <anansi-root>
```

If the anansi daemon is watching the folder, they can skip the manual command.
Otherwise, running `anansi2 ingest` will trigger Pass 3 and Pass 4 only (Pass 1
will be skipped because `anansi_toc` is present in the frontmatter).
