# Output format

The atomized output is **one markdown file**, named `{source-slug}-atomized.md`, with `---`-delimited blocks.

This file is the contract. Every block in the output conforms to one of the templates below.

---

## Document scaffold

```
<!-- anansi-atomize: {source title} | {N} blocks | {YYYY-MM-DD} | source_id: {source_id} -->

[optional ## 0 PA Audit block — only when PA is empty]

---

[Section 1 blocks: Projects, addresses 1.N, in TOC order]

---

[Section 2 blocks: Areas, addresses 2.N, in TOC order]

---

[Section 3 blocks: Discussion shards, addresses 3.N, in TOC order]

---

[Section 4 blocks: Resources (person/organization/note), addresses 4.N, in TOC order]

---

[Whisper blocks: addresses w.1, w.2, ..., emitted last and only after every block above]

<!-- concepts: #tag1 #tag2 #tag3 ... -->
```

Empty sections are entirely omitted (no heading, no placeholder). Block heading + body + trailing `---`. Empty sections do not produce empty `---`-delimited slots in the output.

---

## Block-level metadata

Every block has an HTML comment line directly under the heading:

```
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: ... -->
```

Whisper blocks add a `whisper_category:` field before `confidence:`.

Low-confidence + reviewer-note blocks also emit, *before* the metadata comment:

```
<!-- review-needed: low confidence -->
```

---

## Block templates by type

### `### 0.0` — PA audit (only when PA is empty)

```
### 0.0 PA Audit [note]
<!-- source_id: {source_id} | generated_at: {generated_at} | confidence: high | vault_match_hint: note:{source-slug}-pa-audit -->
{Lede: one sentence summary, ≤25 words, drawn from the rationale.}

**Why it matters:** {one sentence on what this signals about the source}

## Considered and dropped
- {Compressed bullet for each candidate the rationale named, with reason}

## Borderline candidates
- {Compressed bullet for each "would extract if X" candidate}

---
```

The `## Borderline candidates` section is omitted if the rationale named none.

---

### `### 1.N` — Project

Template: `entity-project.md`.

```
### 1.{N} {Name} [project]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: project:{slug} -->

## Lede
{One sentence stating the project's outcome and scope, ≤25 words.}

## Why
{1–2 sentences on stakes or deadline. Does not repeat the lede.}

## Content

### Identity
- Goal: {goal field, verb-noun phrase}
- Status: {Active | Completed | On-hold | Cancelled}
- End Date: {hard date, target quarter, or named milestone}

### Summary
{One paragraph, source-agnostic.}

### Narrative
{2–4 paragraphs. What this project is, why it matters, who is driving it, what makes it significant. Smart Brevity tone — bold key terms, no adverbs, no qualifiers.}

## Edges
- contributed_by: person:{slug} — {role}
- under_area: area:{slug}
- sub_project_of: project:{slug}           (when this is a sub-project, e.g., 1.2.1 nested under 1.2)
- stakeholder: person:{slug} — {role}
- references: note:{slug}

## Review
**Reviewer note:** {if present in upstream typed file, else omit entire ## Review section}

---
```

---

### `### 2.N` — Area

Template: `entity-area.md`.

```
### 2.{N} {Name} [area]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: area:{slug} -->

## Lede
{One sentence naming the territory and the standard, ≤25 words.}

## Why
{1–2 sentences on why this responsibility exists or what it serves.}

## Content

### Identity
- Standard: {the quality bar, in the user's own words from the source when possible}
- Owner: {person or role, distinct from the user when named}
- Description: {scope and subject — what territory the standard applies to}

### Summary
{One paragraph.}

### Narrative
{2–4 paragraphs. What this area covers, who owns it, what standard is being upheld, why it matters, scope and boundaries.}

## Edges
- stakeholder: person:{slug} — {role}
- supported_by: organization:{slug}

## Review
**Reviewer note:** {if present}

---
```

---

### `### 3.N` — Discussion (topic-discussion / email-exchange / chapter)

Template: depends on source type. Default `meeting-topic-discussion.md`. For email threads use `email-exchange.md`. For books use `book-chapter.md`. See SKILL.md Step 3.

Default (meeting-topic-discussion shape):

```
### 3.{N} {short title from TOC} [discussion]
<!-- source_id: ... | generated_at: ... | confidence: high | vault_match_hint: discussion:{short-slug} -->

## Lede
{One sentence naming the topic and what was discussed, ≤25 words.}

## Why
{1–2 sentences. Adds perspective or downstream consequence. Does not repeat the lede.}

## Content

### Narrative
{3–6 sentences. What was said, not who said it. No verbatim quotes. Smart Brevity tone.}

### Decisions
- {concrete outcome 1}
- {concrete outcome 2}
[OR if none: - [no decision reached]]

### Action Items
- {Owner} to {do X} by {date}
- {Owner} to {do X}

## Edges
- participated_by: person:{slug}            (people who spoke — only active participants, not all attendees)
- mentioned_org: organization:{slug}        (orgs mentioned or represented)
- presented_by: person:{slug}              (who drove the topic)
- references: note:{slug}                  (named documents/programs/events)
- under_project: project:{slug}            (when discussion advances a Section 1 project)
- under_area: area:{slug}                  (same for areas)
- related_to: person:{slug}                (persons relevant but not present)

## Review
**Reviewer note:** {if applicable}

---
```

**Do NOT use `## Participants` or `## Organizations` as separate sections.** All participants, mentioned orgs, and cross-entity links go in `## Edges` as typed edge entries.

---

### `### 4.N` — Person (identity-only)

Template: `entity-person.md`.

```
### 4.{N} {Name} [person]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: person:{slug} -->

## Lede
{Full name} is {single identifying noun phrase}.

## Content

### Identity
- Met via: {One short phrase, ≤15 words. How the user came to know this person.}

## Review
**Reviewer note:** {if present, else omit entire ## Review section}

---
```

**Strict prohibitions:**

- No `## Edges` section. Persons are not convergence points. Relationships surface through the blocks of the things they're connected to.
- No `## Contact` section unless contact info (email/phone) is explicitly present in the source.
- No narrative content about involvement, role in the meeting, or what they said. That belongs in discussion blocks.
- No `## Why` block.
- `## Lede` is identity-only — one noun phrase. "Jaren is the Broadband Program Administrator at DHHL." Not "Jaren is the Broadband Program Administrator at DHHL who presented..."

---

### `### 4.N` — Organization

Template: `entity-organization.md`.

```
### 4.{N} {Name} [organization]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: organization:{slug} -->

## Lede
{Org name} is a {type} {brief descriptor}.

## Content

### Identity
- Full name: {full_name — only if different from name}
- Type: {type}
- Domain: {domain}

### Context
- {Source-bound canonicalization fact or stable identity fact 1 — name variants, founding, key leadership}
- {Fact 2}

## Edges
- affiliated_with: person:{slug}
- affiliated_with: person:{slug}

## Review
**Reviewer note:** {if present}

---
```

`### Context` (inside `## Content`) holds source-bound canonicalization facts and stable identity facts only — name variants resolved, founding history, key leadership. It does NOT contain project-specific activity or meeting-specific behavior. Those belong in discussion blocks (Section 3) or project blocks (Section 1).

`## Edges` only carries `affiliated_with: person:slug` for people who work at, lead, or are named affiliates of this org. Cross-entity relationships travel on the other entity's block.

---

### `### 4.N` — Note

Template: `entity-note.md`.

```
### 4.{N} {Name} [note]
<!-- source_id: ... | generated_at: ... | confidence: ... | vault_match_hint: note:{slug} -->

## Lede
{One sentence naming the most important fact about what this is, ≤25 words.}

## Why
{One sentence on relevance — omit entire ## Why block if self-evident}

## Content

### Summary
{One sentence.}

### Narrative
{2–4 sentences of source-agnostic description. Smart Brevity tone.}

## Edges
- presented_by: person:{slug}
- produced_by: organization:{slug}
- under_project: project:{slug}
- references: note:{slug}
- referenced_by: person:{slug}
- managed_by: organization:{slug}
- concerns: organization:{slug}

## Review
**Reviewer note:** {if present}

---
```

Pick edge verbs by what the source actually says. The list above is illustrative, not exhaustive. Match the most precise verb the source supports.

---

### `### w.N` — Whisper

See `varys-detection-heuristics.md` for the full taxonomy and stop criteria. Block template:

```
### w.{N} {≤8-word title} [whisper]
<!-- source_id: ... | generated_at: ... | whisper_category: signal|absence|pattern|orphan|nuance|decision|correction | confidence: low|medium|high | vault_match_hint: whisper:{slug} -->

## Lede
{One sentence, ≤15 words. The whisper itself, exactly as observed.}

## Why
{One sentence, ≤20 words. What this could change.}

**Source quote:** "{exact words from source — omit line entirely if not quoting}"

## Edges
- {related_to | concerns | corrects | points_at}: {entity_type}:{slug} — {one-phrase relationship}

---
```

**No `## Content` block on whispers.** Only `## Lede`, `## Why`, optional source quote, `## Edges`.

Hard caps:

- Title ≤8 words.
- `## Lede` ≤15 words.
- `## Why` ≤20 words.
- Body ≤80 words excluding `## Edges`.

Whisper exceeds caps → it is not a whisper. Flag as a Reviewer note on the closest related entity instead.

---

## Closing line

Last line of the file:

```
<!-- concepts: #tag1 #tag2 #tag3 ... -->
```

Concatenate every hashtag from `resources-toc.md` `## Concepts`, in source order, separated by single spaces. The closing line carries no trailing period or other punctuation.

---

## File naming and location

Output file path: `{source-slug}-atomized.md` written to the same directory as the four input pipeline files (or wherever the calling skill expects it; confirm with the caller).

Source slug is derived from the source file's basename minus extension, kebab-cased and lowercased. e.g., `transcript.txt` from `output/broadband-hui/` → `broadband-hui-atomized.md` (the parent directory's name is the source slug, since `transcript.txt` is generic).
