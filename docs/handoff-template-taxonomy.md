# Anansi v2 — Template Taxonomy Redesign: Coding Agent Handoff

**Repo:** `jamespakele/ai-anansi-v2`  
**Reference spec:** `docs/anansi-v2-spec.md`  
**Status:** Design complete, not yet implemented  
**Depends on:** Existing codebase (pipeline, merger, db, mcp all working)  
**Does NOT touch:** `src/merger.rs`, `src/db.rs`, `src/mcp.rs`, `migrations/`

---

## 1. What This Change Does

The current template system uses a flat namespace — `person.md`, `meeting_summary.md`, `context.md` — with no structural signal about what tier of the knowledge hierarchy a template belongs to. This makes template loading inefficient (all templates loaded for every ingest), and the TOC decomposition prompt has no way to communicate stopping rules to Pass 1.

This change introduces:

1. **A naming convention** that encodes template tier in the filename prefix
2. **Two new frontmatter fields** — `template_class` and `floor_prompt`
3. **New content-unit floor templates** for meeting topics, article sections, video chapters, and email exchanges
4. **Selective template loading** — only load the template family relevant to the source being ingested
5. **A floor constraint** in the TOC validator — content_unit leaves cannot have children
6. **Floor prompt injection** into Pass 1 — the stopping rule comes from the template, not from %Rules

---

## 2. Naming Convention

Templates are organized into three tiers by filename prefix:

### `identity-` prefix — pure-atomic and container identity types

These represent real-world entities that exist independently of any source. They are reused and merged across sources.

```
identity-person.md
identity-organization.md
identity-concept.md
identity-topic.md
identity-area.md
identity-note.md
identity-project.md
```

### Source-family prefix — source types and their floor types

Each source family shares a prefix. The source template describes the document being ingested; the floor template describes the stopping point for decomposition. Both are loaded together when ingesting that source type.

```
meeting-summary.md              ← source type (atomic: false)
meeting-topic-discussion.md     ← floor type (template_class: content_unit)

research-paper.md               ← source type
research-section.md             ← floor type

youtube-video.md                ← source type
youtube-chapter.md              ← floor type

email-thread.md                 ← source type
email-exchange.md               ← floor type
```

### No-prefix — existing source-bound utility types (unchanged)

These existing types remain as-is. They are always loaded.

```
context.md          ← generic source-bound interaction unit (catch-all)
event.md            ← source-bound event
task.md             ← source-bound task
action_item_list.md ← source-bound action item list
outline.md          ← auto-generated, one per source
container.md        ← generic source wrapper (no floor type needed)
```

---

## 3. New Frontmatter Fields

### `template_class` (required on all templates)

Declares the tier. Valid values:

| Value | Applies to | Meaning |
|---|---|---|
| `identity` | `identity-*` templates | Global entity, pure-atomic or container merge |
| `content_unit` | floor templates | Stopping point — no children allowed in TOC |
| `source` | source templates | Document being ingested, not written as a note |
| `utility` | context, event, task, etc. | Source-bound, always loaded, no floor constraint |

### `floor_prompt` (required on `content_unit` templates only)

A Pass 1 stopping instruction specific to this floor type. Injected verbatim into the Pass 1 prompt after the entity type list. Tells the LLM where decomposition ends for this source family.

```yaml
floor_prompt: |
  Decompose this meeting summary into topic_discussion leaves — one per 
  distinct topic of discussion. The meeting-topic-discussion template 
  defines the anatomy of each leaf: participants, organizations, content, 
  decisions, action items. Do not create sub-leaves within a topic 
  discussion. Individual statements, supporting points, and contributions 
  belong inside the leaf as content, not as separate leaves.
```

---

## 4. Template Migration — Renames Required

Rename these existing template files. The `entity_type` field inside each file stays the same (it matches the logical type name, not the filename).

| Current filename | New filename | entity_type field (unchanged) |
|---|---|---|
| `person.md` | `identity-person.md` | `person` |
| `organization.md` | `identity-organization.md` | `organization` |
| `concept.md` | `identity-concept.md` | `concept` |
| `topic.md` | `identity-topic.md` | `topic` |
| `area.md` | `identity-area.md` | `area` |
| `note.md` | `identity-note.md` | `note` |
| `project.md` | `identity-project.md` | `project` |
| `meeting_summary.md` | `meeting-summary.md` | `meeting_summary` |
| `research_paper.md` | `research-paper.md` | `research_paper` |
| `email_thread.md` | `email-thread.md` | `email_thread` |

Add `template_class` to the frontmatter of each renamed file:
- All `identity-*` files: add `template_class: identity`
- `meeting-summary.md`, `research-paper.md`, `email-thread.md`: add `template_class: source`
- `context.md`, `event.md`, `task.md`, `action_item_list.md`, `outline.md`, `container.md`: add `template_class: utility`

---

## 5. New Templates to Create

### `meeting-topic-discussion.md`

```markdown
---
entity_type: topic_discussion
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A bounded discussion of a single topic within a meeting. Captures who
  participated, what was said, what was decided, and what actions followed.
  This is the atomization floor for meeting-summary sources — do not
  decompose further.
floor_prompt: |
  Decompose this meeting summary into topic_discussion leaves — one per
  distinct topic of discussion. Do not create sub-leaves within a topic
  discussion. Individual statements, speaker turns, and sub-points belong
  inside the leaf as content, not as separate TOC entries. If two agenda
  items blur together in the source, create one leaf. A side comment that
  launches a new direction is a new leaf only if it constitutes a distinct
  topic with its own participants and outcome.
sources:
  meeting_summary:
    hint: >
      Extract what was discussed, not a verbatim transcript. One sentence
      per substantive point. Capture decisions and action items precisely —
      these are the high-value outputs. Participants are people who spoke
      or were directly addressed, not everyone in the room.
---

%%
field: topic_sentence
description: One sentence naming the topic and framing what was being discussed
format: prose
constraints: "Source-specific — name the topic as it was framed in this meeting, not a generic definition"
%%

%%
field: participants
description: People who actively participated in this specific discussion
format: bullets
constraints: "Only people who spoke or were directly addressed in this topic, not all meeting attendees"
%%

%%
field: organizations
description: Organizations mentioned or represented in this discussion
format: bullets
%%

%%
field: content
description: Summary of what was discussed — the substance of the conversation
format: prose
constraints: "3-6 sentences. What was said, not who said it. No verbatim quotes."
%%

%%
field: decisions
description: Decisions reached or agreements made during this discussion
format: bullets
constraints: "Concrete outcomes only. If no decision was reached, write '[no decision reached]'"
%%

%%
field: action_items
description: Specific commitments and next steps from this discussion, with owners where known
format: bullets
constraints: "Format: 'Owner to do X by date' where information is available"
%%

# {{topic_sentence}}

## Participants
{{participants}}

## Organizations
{{organizations}}

## Content
{{content}}

## Decisions
{{decisions}}

## Action Items
{{action_items}}
```

---

### `research-section.md`

```markdown
---
entity_type: article_section
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A subsection or paragraph of a research paper or article that makes one
  complete argument, claim, or point. This is the atomization floor for
  research-paper sources — do not decompose into individual sentences or
  claims.
floor_prompt: |
  Decompose this research paper into article_section leaves — one per
  distinct subsection or coherent argumentative unit. A leaf must be able
  to stand alone: a complete claim with its supporting evidence or
  reasoning. Do not create separate leaves for individual sentences,
  data points, or sub-claims within an argument. An introduction gets one
  leaf. A conclusion gets one leaf. Each numbered or headed subsection
  typically gets one leaf unless it contains two clearly independent
  arguments, in which case it may get two.
sources:
  research_paper:
    hint: >
      Capture the argument, not just the topic. The topic sentence is the
      claim being made. Supporting details are the evidence. Do not
      summarize so aggressively that the distinction between claim and
      evidence is lost.
---

%%
field: topic_sentence
description: The central claim or point this section makes
format: prose
constraints: "One sentence. This is the argument, not just the subject."
%%

%%
field: supporting_details
description: Evidence, examples, data, or reasoning that supports the topic sentence
format: bullets
constraints: "3-7 bullets. Concrete and specific. No repetition of the topic sentence."
%%

%%
field: content
description: Full prose summary of this section preserving the author's argument
format: prose
constraints: "2-4 sentences. Maintain the argumentative structure — do not flatten into a topic list."
%%

%%
field: entities
description: Concepts, people, organizations, and references mentioned in this section
format: bullets
%%

# {{topic_sentence}}

## Supporting Details
{{supporting_details}}

## Content
{{content}}

## Entities
{{entities}}
```

---

### `youtube-video.md`

```markdown
---
entity_type: youtube_video
template_class: source
atomic: false
merge_strategy: source_bound
template_version: "2.0"
description: >
  A YouTube video or recorded presentation. Non-atomic source type —
  decomposes into youtube_chapter floor nodes.
---

%%
field: overview
description: >
  Convergence narrative — who made this video, why, what it is trying to
  convey, and what context it sits in. Who is the intended audience.
format: prose
%%

%%
field: topics
description: >
  Per-chapter breakdown using ### subsections, one per youtube_chapter
  leaf. Each subsection contains wikilinks to entities mentioned in
  that chapter.
format: prose
%%

# {{title}}

## Overview
{{overview}}

## Topics
{{topics}}
```

---

### `youtube-chapter.md`

```markdown
---
entity_type: youtube_chapter
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A chapter or inferred thematic segment of a YouTube video or recorded
  presentation. This is the atomization floor for youtube-video sources —
  do not decompose into individual statements or timestamps.
floor_prompt: |
  Decompose this video transcript into youtube_chapter leaves — one per
  explicit chapter marker or inferred thematic segment. A segment ends
  when the speaker clearly transitions to a new topic or the content
  shifts focus. Do not create leaves for individual statements, examples,
  or asides within a segment. A brief tangent that returns to the main
  thread is content inside the current leaf, not a new leaf. Aim for
  5-15 leaves for a standard one-hour video — fewer for tightly focused
  content, more for wide-ranging discussions.
sources:
  youtube_video:
    hint: >
      Capture the substance of what was presented in this segment, not a
      transcript summary. What was the key point? What examples or
      evidence were given? What entities were mentioned?
---

%%
field: topic_sentence
description: One sentence describing what this chapter or segment is about
format: prose
%%

%%
field: key_points
description: The main points, arguments, or demonstrations in this segment
format: bullets
constraints: "3-7 bullets. Concrete and specific."
%%

%%
field: content
description: Prose summary of what was covered in this segment
format: prose
constraints: "2-4 sentences."
%%

%%
field: entities
description: People, organizations, concepts, and tools mentioned in this segment
format: bullets
%%

# {{topic_sentence}}

## Key Points
{{key_points}}

## Content
{{content}}

## Entities
{{entities}}
```

---

### `email-exchange.md`

```markdown
---
entity_type: email_exchange
template_class: content_unit
atomic: true
merge_strategy: source_bound
template_version: "2.0"
description: >
  A cohesive exchange within an email thread organized around one question,
  decision, or topic. This is the atomization floor for email-thread
  sources.
floor_prompt: |
  Decompose this email thread into email_exchange leaves — one per
  coherent exchange around a distinct question, decision, or topic. An
  exchange ends when the thread moves to a new subject or a decision
  is reached. Do not create separate leaves for individual emails within
  an exchange. A back-and-forth negotiation about one decision is one
  leaf. A thread that pivots to a new topic partway through gets a new
  leaf at the pivot point.
sources:
  email_thread:
    hint: >
      Capture what was being resolved, not who sent what when. The
      participants are who was in the exchange. The outcome is what
      was decided or agreed.
---

%%
field: subject
description: What question or topic this exchange is about
format: prose
%%

%%
field: participants
description: People involved in this specific exchange
format: bullets
%%

%%
field: content
description: Summary of the exchange — what was being discussed and how it unfolded
format: prose
%%

%%
field: outcome
description: The decision, agreement, or resolution reached in this exchange
format: prose
constraints: "If unresolved, write '[unresolved — thread continues]'"
%%

# {{subject}}

## Participants
{{participants}}

## Content
{{content}}

## Outcome
{{outcome}}
```

---

## 6. Daemon Changes Required

### 6.1 Template Loader

**File:** `src/prompt.rs` (or wherever templates are currently loaded — search for `templates/` path construction)

**Current behavior:** Loads all `*.md` files from the `templates/` directory at startup.

**New behavior:** Selective loading based on source type being ingested.

Change the template loading function to accept a `source_family: Option<&str>` parameter and load:

1. Always: all `identity-*.md` templates
2. Always: all utility types — `context.md`, `event.md`, `task.md`, `action_item_list.md`, `outline.md`, `container.md`
3. Conditionally: all `{source_family}-*.md` templates where `source_family` is derived from the source type

```rust
fn source_family(source_type: &str) -> &str {
    match source_type {
        "meeting_summary" => "meeting",
        "research_paper"  => "research",
        "email_thread"    => "email",
        "youtube_video"   => "youtube",
        _                 => "container",   // fallback — loads no floor types
    }
}
```

Template loading glob sequence:
```rust
let globs = vec![
    "identity-*.md",
    &format!("{}-*.md", source_family),
    // utility types by explicit name:
    "context.md",
    "event.md",
    "task.md",
    "action_item_list.md",
    "outline.md",
    "container.md",
];
```

Parse `template_class` from each template's frontmatter and store it in the `TemplateRegistry` entry alongside `entity_type`, `merge_strategy`, etc.

### 6.2 Pass 1 Prompt Assembly

**File:** `src/prompt.rs`, function that assembles the Pass 1 prompt

After assembling the standard entity type list, collect `floor_prompt` fields from all loaded templates where `template_class == "content_unit"` and append them to the prompt:

```rust
let floor_prompts: Vec<String> = registry
    .templates()
    .filter(|t| t.template_class == TemplateClass::ContentUnit)
    .filter_map(|t| t.floor_prompt.as_ref())
    .cloned()
    .collect();

if !floor_prompts.is_empty() {
    prompt.push_str("\n\n## Decomposition Floor Rules\n\n");
    for fp in &floor_prompts {
        prompt.push_str(fp);
        prompt.push('\n');
    }
}
```

The floor prompt text should appear in the Pass 1 prompt AFTER the entity type descriptions and BEFORE the source body.

### 6.3 TOC Leaf Validator

**File:** `src/toc.rs` or wherever TOC lines are parsed into `TocLeaf` structs (search for the leaf parsing regex)

After parsing each leaf, check whether its entity_type has `template_class == ContentUnit`. If it does, validate that no subsequent leaf has a dotted address that is a child of this leaf's address.

```rust
fn validate_no_floor_children(leaves: &[TocLeaf], registry: &TemplateRegistry) -> Result<()> {
    for leaf in leaves {
        if registry.get(&leaf.entity_type)
            .map(|t| t.template_class == TemplateClass::ContentUnit)
            .unwrap_or(false)
        {
            // Check no other leaf starts with leaf.address + "."
            for other in leaves {
                if other.address.starts_with(&format!("{}.", leaf.address)) {
                    return Err(anyhow!(
                        "TOC validation error: leaf {} ({}) is a content_unit floor type \
                         and cannot have children. Found child at address {}.",
                        leaf.address, leaf.entity_type, other.address
                    ));
                }
            }
        }
    }
    Ok(())
}
```

Call this validation after parsing the TOC, before spawning Pass 3 calls.

### 6.4 `%Atomicity.md` Update

Add Rule 3 to `anansi/%Rules/%Atomicity.md`:

```markdown
## Rule 3 — Minimum Viable Granularity

Do not decompose below the natural content unit for the source type. A
topic discussion, article section, video chapter, or email exchange is
the floor. A sentence is not a note. A sub-point within an argument is
not a note.

The test: can this unit be read without the surrounding source and still
convey complete, standalone meaning? If yes, it may be a content unit.
If it requires surrounding context to make sense, it belongs inside the
content unit above it.

Floor types are declared in the template file for each source family.
The `floor_prompt` field in each content_unit template encodes the
specific stopping rule for that source-floor pair.
```

---

## 7. `anansi.toml` — No Changes Required

Template loading is path-based. The only config change needed is if the vault operator adds a new source family — they just add template files; no config entry needed.

---

## 8. Implementation Order

Execute in this sequence to keep the codebase compiling at each step:

1. **Rename existing template files** (no code changes — just `git mv`)
2. **Add `template_class` to renamed files' frontmatter** (text edits only)
3. **Create new floor templates** (new files — paste from §5 above)
4. **Update `TemplateRegistry` struct** to hold `template_class: TemplateClass` and `floor_prompt: Option<String>` fields
5. **Update template loader** to parse the new frontmatter fields
6. **Update template loading** to use selective glob loading by source family
7. **Update Pass 1 prompt assembly** to inject `floor_prompt` fields
8. **Add TOC validator** for content_unit floor constraints
9. **Update `%Atomicity.md`** with Rule 3
10. **Test** with a meeting summary ingest — verify that TOC leaves are all `topic_discussion` typed and that a sub-leaf of a `topic_discussion` is rejected

---

## 9. What NOT to Change

- `src/merger.rs` — merge strategies are unchanged; `topic_discussion`, `article_section` etc. use `source_bound` which is already implemented
- `src/db.rs` — schema unchanged; new types produce rows in the same `notes` table
- `src/mcp.rs` — no interface changes
- `migrations/` — no schema changes
- `src/pipeline.rs` Pass 3 loop — each floor-type leaf goes through the same Pass 3 call as any other source-bound leaf
- Existing `context`, `event`, `task`, `action_item_list` types — these remain valid TOC leaf types; `topic_discussion` etc. are additions, not replacements

---

## 10. Testing Checklist

- [ ] `cargo build` passes after template renames
- [ ] Template registry loads without errors; `identity-person.md` registers as entity_type `person`
- [ ] Ingesting a `meeting_summary` source loads `meeting-*` + `identity-*` templates; does NOT load `youtube-*` or `research-*`
- [ ] Pass 1 prompt for a meeting summary includes the `floor_prompt` from `meeting-topic-discussion.md`
- [ ] A TOC with a child leaf under a `topic_discussion` leaf fails validation with a clear error message
- [ ] A valid TOC with `topic_discussion` leaves passes validation and produces expanded notes via Pass 3
- [ ] Existing entity types (`person`, `organization`, `context`, etc.) continue to work — no regressions
