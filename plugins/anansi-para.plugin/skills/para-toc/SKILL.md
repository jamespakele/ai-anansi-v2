---
name: para-toc
description: >
  Generate a typed PARA Table of Contents from a content file plus
  para-extract and resource-typer outputs. Organizes entities into five
  typed buckets: 1.Projects, 2.Areas, 3.Discussion (content sections from
  the source), 4.Resources (typed as person/organization/note), 5.Concepts
  (tags). Decimal addresses (1.1, 2.3) mark separate notes; alpha
  sub-addresses (3.1.a, 3.1.b) mark canonical sections within a note's
  content — not separate notes. Outputs in anansi frontmatter format
  (anansi_toc: |) ready for daemon ingestion. Use after para-extract and
  resource-typer on the source, or invoke standalone to run the full
  pipeline. Triggers: "generate para toc", "toc from para output",
  "/para-toc", "typed toc", "build the toc".
argument-hint: "[content file path, para-extract output, resource-typer output]"
---

# para-toc

Assembles a typed PARA Table of Contents by combining three inputs: the original content file (for section structure), `para-extract` output (for Projects, Areas, Resources, Concepts), and `resource-typer` output (for typed Resource keys). Produces a TOC in anansi frontmatter format ready for daemon ingestion.

---

## When to invoke

Trigger this skill when the user:

- Says "generate para toc", "/para-toc", "typed toc", "build the toc"
- Has just run `para-extract` + `resource-typer` and wants to turn the output into an anansi TOC
- Drops a content file and asks to prepare it for anansi ingestion using the PARA pipeline
- Is at the third step of the atomization pipeline: para-extract → resource-typer → **para-toc**

Do not invoke for: plain `anansi-toc` generation (which reads templates directly without PARA classification), or for classifying a single item (`para-classify`), or for sub-typing resources (`resource-typer`).

---

## The two address types

Understanding this is essential before generating any TOC line:

**Decimal addresses** (`1.1`, `3.2`, `4.1.1`) → **separate notes**
Each decimal address produces a distinct stored note in anansi. This is every Project, Area, Resource, and Discussion section in the TOC.

**Alpha sub-addresses** (`3.1.a`, `3.1.b`, `3.1.c`) → **sections within a note's content**
Alpha addresses are headings inside the `## Content` block of the parent note at `3.1`. They are NOT separate notes — they are named sections the daemon uses to structure the note's body. The parser identifies them as `(address).(alpha)` where alpha is a lowercase letter.

The rule: if it's an entity that stands alone → decimal. If it's a named section inside another entity's content → alpha.

---

## The five typed buckets

```
1. Projects      — active outcomes with deadlines
2. Areas         — ongoing responsibilities with a standard to uphold
3. Discussion    — content sections from the source document (floor synergies)
4. Resources     — typed entities: person, organization, note
5. Concepts      — tag-shaped references (flat list, no decimal addresses)
```

Top-level bucket headers (`1`, `2`, `3`, `4`, `5`) are comment lines — they are not daemon-parseable leaves. Use `# Section N: Label` format for readability.

---

## Alpha sections — template-driven

**Templates are the single source of truth.** Do not maintain a hardcoded alpha section list in your head. For every Discussion entry, read the matching template from `references/templates/` and use its `toc_structure` field to determine the alpha format. The template's `toc_structure` is always authoritative.

### How to resolve alpha sections for any content_unit

1. Identify the content_unit type for this Discussion entry (infer from source structure and content)
2. Read `references/templates/<entity_type>.md` — use the filename that matches the type (e.g. `book-chapter.md`, `meeting-topic-discussion.md`)
3. Read the `toc_structure` field from frontmatter — it is one of:
   - `"none"` → no alpha sub-addresses for this entry
   - `"a. Label · b. Label · ..."` → label-only alphas; use exactly as written
   - `"content_filled: ..."` → alpha entries must contain actual content (see rule below)
4. If no matching template exists, fall back to `"a. Summary · b. Key Points · c. Action Items"` (label-only)

### Content-filled vs. label-only — critical distinction

When `toc_structure` starts with `content_filled:`, alpha entries must contain **actual content** — a specific idea or claim in Smart Brevity format:

```
Key phrase — dense explanation
```

where the key phrase names the concept (3–6 words) and the explanation is one specific, self-contained clause. **Never write bare structural labels** ("Summary", "Key Points", "Supporting Details", "Thesis", or any template field name) as alpha entries for content-filled types.

When `toc_structure` lists labels (`a. Decisions · b. Action Items`), use those labels verbatim — label-only is correct and expected for meetings, emails, research sections, and most other types.

Only include alpha sections for content types where `toc_structure` is not `"none"`. Identity types (person, organization, project, area, note, book) always get decimal addresses only — no alphas.

---

## Inputs

### Required
- **Content file** — the original source document (markdown). Read via the Read tool.
- **para-extract output** — structured markdown with Projects, Areas, Resources (flat), and Concepts lists. Can be pasted directly or referenced from earlier in the conversation.
- **resource-typer output** — the same extraction with `resource:[slug]` keys replaced by `person:[slug]`, `organization:[slug]`, or `note:[slug]`.

### Optional
- **known_projects / known_areas** — existing vault entries for `vault_match_hint` resolution. If present in the para-extract output, prefer canonical vault names over extracted names.

If any input is missing, ask the user before proceeding. Do not guess at para-extract output from the content file alone — that is `anansi-toc`'s job. `para-toc` assembles; it does not re-classify.

---

## Process

### Step 1: Gather inputs

Read the content file. Confirm you have both the para-extract output and the resource-typer output. If not, ask the user to provide them or offer to run those skills first.

### Step 2: Map Projects → Section 1

For each Project in the para-extract output:
- Assign a decimal address under `1` (e.g., `1.1`, `1.2`)
- Entity type: `[project]`
- If `vault_match_hint` is marked `existing`, add `| hint: existing vault entry`
- If `vault_match_hint` is marked `new`, no hint needed

```
# Section 1: Projects
1.1 Anansi v2 Pipeline [project]
1.2 Cowork Plugin Packaging [project] | hint: existing vault entry
```

### Step 3: Map Areas → Section 2

For each Area in the para-extract output:
- Assign a decimal address under `2` (e.g., `2.1`, `2.2`)
- Entity type: `[area]`

```
# Section 2: Areas
2.1 Anansi v2 Development [area]
2.2 Cowork Skill Development [area]
```

### Step 4: Map Discussion sections → Section 3

Read the content file's heading structure. Each top-level heading (or logical section) becomes a Discussion entry with a decimal address.

For each section:
1. Assign a decimal address under `3` (e.g., `3.1`, `3.2`)
2. Infer the content_unit type from the source structure and content
3. **Read `references/templates/<entity_type>.md`** and pull its `toc_structure` field
4. If `toc_structure` is `"none"` → no alphas; if label-only → use those labels verbatim; if `content_filled:` → write actual Smart Brevity content points
5. Only include alpha addresses if `toc_structure` is not `"none"`

**Book sources require a 3-level hierarchy:** Part/Section groupings get a decimal address with type `[book_section]` and no alphas. Chapters within them get a decimal sub-address with type `[book_chapter]` and content-filled alphas. Example:

```
# Section 3: Discussion
3.1 Introduction [book_section]

3.1.1 How to Read This Book [book_chapter]
  3.1.1.a Five promises — find info fast, gain focus, make things happen, boost creativity, beat FOMO
  3.1.1.b Reading strategy — Part 1 to start; Part 2 after two weeks of practice; Part 3 as needed

3.2 Part 1: The Fundamentals [book_section]

3.2.1 Introducing PARA [book_chapter]
  3.2.1.a Four categories — Projects (goal + deadline), Areas (ongoing standard), Resources (interests), Archives (inactive cold storage)
  3.2.1.b Organize for action not subject — group by current project or goal, not broad academic topic
  3.2.1.c Universal and platform-agnostic — same structure works across every digital tool simultaneously
```

**Meeting/email sources use label-only alphas:**

```
# Section 3: Discussion
3.1 Repo Setup + OneDrive Workarounds [meeting-topic-discussion]
3.1.a Decisions
3.1.b Action Items
3.2 Atomization Scope Reframe [meeting-topic-discussion]
3.2.a Decisions
3.2.b Action Items
3.2.c Open Questions
```

**Note on alpha address daemon compatibility:** The current daemon parser regex (`^\s*\d+(?:\.\d+)*\s+...`) does not yet match alpha addresses — alpha lines will be preserved in the note body but skipped by the daemon until the parser is updated. Include them regardless; they are forward-compatible and self-documenting.

### Step 5: Map typed Resources → Section 4

Use the **resource-typer output** (not the raw para-extract output) for this section. Typed keys determine entity type:

- `person:[slug]` → `[person]`
- `organization:[slug]` → `[organization]`
- `note:[slug]` → `[note]`

Group within Section 4 by type (persons first, then organizations, then notes) for readability, but addresses are continuous:

```
# Section 4: Resources
4.1 Tiago Forte [person]
4.2 Reid Hoffman [person]
4.3 Anthropic [organization]
4.4 Building a Second Brain [note]
```

### Step 6: Map Concepts → Section 5

Concepts are tags — they do not get decimal addresses. Output as a flat `#tag` list on one or more lines:

```
# Section 5: Concepts
#atomization #para-framework #named-entity-recognition #template-as-oracle
#smart-brevity #lean-atomic-identity #alpha-sub-addresses
```

### Step 7: Assemble the full TOC

Combine all five sections into a single TOC block. Apply a quality check:

- Every decimal address line must match: `^\s*\d+(?:\.\d+)*\s+.+?\s+\[\w[\w-]*\]`
- No duplicate decimal addresses
- Alpha lines follow their parent decimal line immediately
- Concept lines start with `#` — do not attempt to parse these as leaves
- No markdown fences, no preamble, no commentary beyond `# Section N:` headers

### Step 8: Assemble the output note

The typed TOC is the **body** of the output note — not frontmatter. Frontmatter is minimal:

```yaml
---
source_type: <inferred or ask user>
title: <title from source>
source_date: <ISO-8601 if known>
toc_author: <model name>
toc_generated_at: <ISO-8601 timestamp>
---
```

The five-bucket TOC follows as the note body. No `anansi_toc:` blob, no version field. The daemon ingests the body directly.

Also update the alpha compatibility note in Step 4 — replace "preserved in frontmatter" with "preserved in the note body."

### Step 9: Write and report

Ask the user for the output path, or default to `<source-slug>-para-toc.md` alongside the source file.

Report:
- Output path
- Counts: Projects / Areas / Discussion sections / Resources (by type) / Concepts
- Whether any vault matches were found (`existing` hints)
- Reminder that alpha addresses require the daemon parser update to be fully processed

---

## Output example

Given a meeting summary with 2 projects, 1 area, 3 discussion topics, and mixed resources:

```
# Section 1: Projects
1.1 Anansi v2 Pipeline [project]
1.2 Skill Builder Plugin [project]

# Section 2: Areas
2.1 Anansi v2 Development [area]

# Section 3: Discussion
3.1 Repo Setup and OneDrive Workarounds [meeting-topic-discussion]
3.1.a Decisions
3.1.b Action Items
3.2 PARA Method Introduction [meeting-topic-discussion]
3.2.a Decisions
3.3 Typed TOC Convention [meeting-topic-discussion]
3.3.a Decisions
3.3.b Action Items
3.3.c Open Questions

# Section 4: Resources
4.1 Tiago Forte [person]
4.2 Anthropic [organization]
4.3 Building a Second Brain [note]

# Section 5: Concepts
#atomization #para-framework #typed-toc #alpha-sub-addresses
#template-as-oracle #lean-atomic-identity
```

---

## Edge cases

### "User only has para-extract output, not resource-typer output"

Offer to run resource-typer inline before generating the TOC. Para-toc requires typed Resource keys — `resource:[slug]` placeholders should not appear in Section 4.

### "A Project also appears as a Discussion topic"

Projects in Section 1 get a decimal address for the project note itself. If the source document also has a section discussing that project's decisions, that section gets a separate Discussion entry in Section 3 with its own decimal address. They are different notes — one is the identity, one is the discussion.

### "No identifiable section structure in the content file"

If the source is flat prose with no headings, create a single Discussion entry for the whole document body:
```
3.1 [Document Title] — Full Content [content_unit_type]
3.1.a Key Points
3.1.b Action Items
```

### "A Resource wasn't sub-typed by resource-typer"

If `resource:[slug]` still appears in the output (resource-typer missed it), use `[note]` as the fallback type and add `| hint: type unconfirmed`.

### "Concepts list is empty"

Omit Section 5 entirely. Do not output an empty bucket.
