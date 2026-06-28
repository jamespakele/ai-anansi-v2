# Anansi Entity Templates — Authoritative Reference

This directory holds all 30 entity templates used by the anansi plugin to atomize source documents into typed knowledge graph nodes. **This is the single source of truth.** All other copies (vendored under individual skills) are mirrors that need to be re-synced when something here changes.

---

## Design principles

**Fields and structures are kept in alignment with the documented frameworks the templates serve.** Every identity field, discovery hint, and naming choice traces back to a citation in one of three external references rather than being invented in-house:

1. **Tiago Forte's PARA framework** — *Building a Second Brain* (2022) and *The PARA Method* (2023). The conceptual definitions of Project, Area, Resource, and Archive. When `entity-area.md` calls a field `standard`, that's not a label we picked — it's the word Forte uses on the page (PARA Method Ch. 3: "A standard to be maintained"). When `entity-project.md` requires a `goal` and an `end_date`, those are the two markers Forte names as defining traits of a Project (BASB Ch. 5; PARA Method Ch. 3).
2. **Smart Brevity input-type vocabulary** — Schwartz, Allen & VandeHei (2022) and the smart-brevity skill's `references/` folder. The eleven document types the compression pass recognizes (`email`, `email_thread`, `newsletter`, `meeting`, `presentation`, `speech`, `social_media`, `workplace_memo`, `company_update`, `book`, `generic_prose`). Templates' `sources:` keys use this vocabulary verbatim so the same type tags flow through the whole pipeline.
3. **The anansi knowledge graph schema** — `merge_strategy`, `roster_sections`, `toc_structure`, instance key conventions. These names come from the daemon's data model, not from arbitrary template-author preference.

This alignment is deliberate and is the reason the templates compose cleanly. When a field name in a template matches the word a framework uses on the page, the LLM has a clear concept to populate; when downstream code reads that same field, it knows what semantic to expect; when a human looks at the rendered note, it reads like the framework it came from. **When you see drift between a template and one of these references — fix the template, not the reference.** That's the maintenance contract.

When a fourth framework gets introduced (a new compression style, a new productivity vocabulary, a new graph schema), the same principle applies: align field names to its documented terminology rather than translating into custom labels.

---

## What a template is

A template is a YAML-frontmatter markdown file that defines:

1. **What kind of thing this is** — `entity_type`, `template_class`, whether it's `atomic`.
2. **What identifies it** — `identity_fields` and a `merge_strategy` for upserts.
3. **How to find it in different sources** — `sources` block with per-source-type discovery hints (aligned with the Smart Brevity input vocabulary).
4. **How it decomposes (if applicable)** — `floor_prompt` for content_units, source→content_unit mapping for source templates.
5. **How it renders in a TOC** — `toc_structure` field, which para-toc consults.
6. **What body shape the rendered note has** — body section after the YAML, with field comment blocks (`%% field: ... %%`) followed by the markdown skeleton.

Templates are **prompts and contracts at the same time**: they tell the extracting LLM what fields to populate, and they give downstream code a stable shape to parse. Edit them with both audiences in mind.

---

## The four template classes

Each template carries a `template_class` field with one of four values. Together they describe the atomization architecture: sources decompose into content_units, content_units reference identities, and utilities are flexible glue.

### `identity` — atomic types (the nouns)

Six pure named-thing templates. Filename prefix `entity-`. All `atomic: true`.

| Template | merge_strategy | What it represents |
|---|---|---|
| `entity-person.md` | `pure_atomic` | A real human |
| `entity-organization.md` | `container` | A company, nonprofit, agency, group |
| `entity-book.md` | `title_author` | A book (deduplicated by title + author) |
| `entity-note.md` | `pure_atomic` | A standalone subject-matter note |
| `entity-area.md` | `container` | A PARA Area — ongoing responsibility |
| `entity-project.md` | `container` | A PARA Project — named initiative with end date |

These are the only templates the para-resource-entities skill walks against when typing Resources, and the only templates the para-projects-areas skill consults for project/area field-set alignment.

### `source` — the trees that get atomized

Seven whole-document container types. All `atomic: false`. Each gets decomposed into its corresponding `content_unit` floor, never stored as a single node.

| `source` template | atomization floor (content_unit) |
|---|---|
| `meeting-summary.md` | `meeting-topic-discussion.md` |
| `email-thread.md` | `email-exchange.md` |
| `research-paper.md` | `research-section.md` |
| `youtube-video.md` | `youtube-chapter.md` |
| `presentation.md` | `presentation-slide.md` |
| `newsletter.md` | `newsletter-item.md` |
| `company-update.md` | `company-update-item.md` |

### `content_unit` — convergence templates with floors

Eight templates representing bounded units of meaning where many people and organizations converge (a topic discussion, a chapter, an exchange). Most carry a `floor_prompt` field that drives the decomposition.

From `meeting-topic-discussion.md`:

> "A bounded discussion of a single topic within a meeting. Captures who participated, what was said, what was decided, and what actions followed. This is the atomization floor for meeting-summary sources — do not decompose further."

| Template | atomic | Notes |
|---|---|---|
| `book-section.md` | false | Multi-chapter section (e.g., a Part) — non-atomic, gets decomposed |
| `book-chapter.md` | true | Atomization floor for books |
| `email-exchange.md` | true | Atomization floor for email threads |
| `meeting-topic-discussion.md` | true | Atomization floor for meeting summaries |
| `newsletter-item.md` | true | Atomization floor for newsletters |
| `presentation-slide.md` | true | Atomization floor for presentations |
| `research-section.md` | true | Atomization floor for research papers |
| `company-update-item.md` | true | Atomization floor for company updates |
| `youtube-chapter.md` | true | Atomization floor for YouTube videos |

### `utility` — flexible glue

Nine templates that don't fit cleanly into the source/content_unit hierarchy but show up across many flows. Mix of atomic and non-atomic.

| Template | atomic | Purpose |
|---|---|---|
| `container.md` | false | Generic wrapper — holds multiple sub-documents (a collection, report, archive) |
| `context.md` | true | Source-specific contextual note attached to an entity |
| `event.md` | true | A bounded happening with a date — distinct from a project (no completable outcome) |
| `task.md` | true | A single actionable item with one owner |
| `action_item_list.md` | true | Aggregate list attached to a meeting or thread |
| `memo.md` | true | A workplace memo |
| `outline.md` | true | A structured outline |
| `social-post.md` | true | A single post on a social network |
| `speech.md` | true | A delivered speech or talk |

---

## Frontmatter field semantics

Every template's frontmatter defines the same core schema (with class-specific extensions):

| Field | Required | Meaning |
|---|---|---|
| `entity_type` | yes | The slug used in TOC tags (e.g., `[project]`, `[meeting_summary]`) and as the prefix for instance keys (e.g., `project:q3-board-deck`). |
| `template_class` | yes | One of `identity`, `source`, `content_unit`, `utility`. |
| `atomic` | yes | `true` if instances of this template are stored as single nodes; `false` if they're always decomposed into a content_unit floor. |
| `merge_strategy` | yes | How the daemon resolves duplicates: `pure_atomic`, `container`, `title_author`, `source_bound`. |
| `template_version` | yes | Bump when fields are added, removed, renamed, or semantically changed. Use semver-ish strings ("2.0", "3.1"). |
| `description` | yes | One-line description of what the template represents. |
| `atomic_criteria` | identity & content_unit | Multi-line description of what counts as one instance vs. two. |
| `identity_fields` | identity, source, utility | Map of field name → `{type, required?, description}`. The fields the LLM populates. |
| `roster_sections` | identity (sometimes) | Render rules for embedded relationship lists (Contributors, Stakeholders, etc.). |
| `sources` | all | Per-source-type discovery hints — keyed by Smart Brevity's input-type vocabulary. |
| `toc_structure` | all | How this entity renders in a para-toc TOC. `"none"` = no alpha sub-addresses; otherwise a label string ("a. Decisions · b. Action Items"). |
| `floor_prompt` | content_unit (most) | The decomposition prompt. Tells the LLM how to break a parent source into instances of this content_unit. |

---

## The `sources:` vocabulary (Smart Brevity alignment)

The `sources:` block in every template uses keys drawn from the Smart Brevity input-type list, so the same vocabulary flows from extraction → entity typing → compression. Eleven canonical keys:

| Key | Smart Brevity ref file | Description |
|---|---|---|
| `email` | `email.md` | A single message |
| `email_thread` | `email.md` | A multi-message exchange |
| `newsletter` | `newsletter.md` | A periodic publication |
| `meeting` | `meeting.md` | Meeting agenda, summary, or transcript |
| `presentation` | `presentation.md` | A slide deck |
| `speech` | `speech.md` | A delivered speech or talk |
| `social_media` | `social-media.md` | A post on a social network |
| `workplace_memo` | `workplace-memo.md` | An internal memo |
| `company_update` | `company-update.md` | A company-wide update or announcement |
| `book` | `book.md` | A book (chapter or whole) |
| `generic_prose` | `general.md` | Any prose that doesn't match a more specific type |

Older templates may still carry legacy keys like `research_paper` (folds into `book` or `generic_prose`) or `container` (folds into `generic_prose`). Re-sync to align.

---

## entity-project.md — Forte-grounded

**Current version: 2.1**

The Project template encodes Forte's three-marker definition (Building a Second Brain, Ch. 5; The PARA Method, Ch. 2):

> "Projects have a couple of features that make them an ideal way to organize modern work. First, they have a beginning and an end; they take place during a specific period of time and then they finish. Second, they have a specific, clear outcome that needs to happen in order for them to be checked off as complete, such as 'finalize,' 'green-light,' 'launch,' or 'publish.'"

Plus the third marker from Ch. 3:

> "A deadline adds a time limit to achieving your goal."

Identity fields:

| Field | Maps to Forte trait | Notes |
|---|---|---|
| `name` | (label) | Required. |
| `goal` | Specific completable outcome | Verb-noun phrase that can be marked done. The first defining trait. |
| `status` | (state) | Active, completed, on-hold, cancelled. Categorical only — the active-commitment evidence trace lives in skill-side `Evidence`, not in the template. |
| `end_date` | Finite endpoint | Hard date, target quarter, or milestone. The second defining trait. Indefinite framing ("someday") disqualifies. |
| `summary` | (description) | One-paragraph source-agnostic description. |
| `content` | (description) | 2–4 paragraph profile: what, why, who, what makes it significant. |

### Changes in v2.1

- Field descriptions now cite Forte directly (BASB Ch. 5, PARA Method Ch. 2/3) so the LLM has explicit grounding.
- `sources:` aligned to Smart Brevity vocabulary (email, email_thread, newsletter, meeting, presentation, speech, social_media, workplace_memo, company_update, book, generic_prose). Dropped legacy `research_paper` (folds into `book` or `generic_prose`) and renamed legacy `container` → `generic_prose`.
- `status: Active` no longer doubles as the active-commitment marker — that responsibility moved to skill-side `Evidence`. The third Forte marker (active commitment) is therefore captured at extraction time, not at template-fill time.

---

## entity-area.md — Forte-grounded

**Current version: 3.1**

The Area template encodes Forte's two-marker definition (The PARA Method, Ch. 3):

> "There are facets of your work and life that don't have a clear end goal or deadline. We call them 'areas of responsibility.' An area of responsibility has: a standard to be maintained, [and] an indefinite end date... Instead of a goal, an area of responsibility has a standard you're trying to maintain."

Plus the responsibility test from Ch. 4:

> "The line between areas and resources is an opportunity to be completely honest with yourself: What is inside the circle of your responsibilities, which no one else is going to take care of for you, and what is outside?"

And the sharpest practical gate, Ch. 7:

> "If no one else would notice you dropped it, it's a resource, not an area."

Identity fields:

| Field | Maps to Forte trait | Notes |
|---|---|---|
| `name` | (label) | Required. |
| `standard` | The standard to be maintained | **Required.** Forte's first defining trait. The quality bar the user upholds indefinitely. Examples: "pay all bills on time and provide for family's needs" (Finances); "spend quality time with kids every evening" (Parenting); "upgrade speed/performance, fix bugs quickly, approve new releases" (Product Development). |
| `owner` | Direct responsibility | Person or role primarily responsible. Distinct from the user when named. |
| `description` | (scope) | What this area covers — the territory the standard applies to. Distinct from `standard`, which is the quality bar. |
| `summary` | (description) | One-paragraph source-agnostic description. |
| `content` | (description) | 2–4 paragraph profile: what, who, what standard, why it matters, scope and boundaries. |

### Changes in v3.1

- **Added `standard` as a required identity_field**, positioned right after `name`. This was missing in v3.0 — Forte explicitly defines an Area as "a standard to be maintained + an indefinite end date" in PARA Method Ch. 3, making `standard` non-negotiable. The template now matches the book.
- `description` clarified as scope/subject, distinct from the quality-bar `standard`. Both are emitted; they answer different questions.
- `sources:` aligned to Smart Brevity vocabulary (same eleven keys as project). Legacy keys removed.
- Body section updated to render `Standard:` first, then `Owner:`, then `Description:` — matching the new identity_fields ordering.

---

## Where templates are vendored

The canonical source for all entity templates is `anansi-config.plugin/references/templates/`.
This is the single authoritative location. There are no mirrors.

The skill-based mirrors that previously existed at
`plugins/anansi.plugin/skills/para-resource-entities/references/templates/` and
`plugins/anansi.plugin/skills/sb-atomize/references/templates/` have been **removed**.
Templates are now loaded from the database at runtime — the server reads template definitions
from `anansi_config` notes and caches them in memory. The disk copy in `anansi-config.plugin`
is compiled into the binary via `include_str!` and serves as the compile-time fallback; the
DB copy (if present) overrides it at runtime, refreshed via `anansi_reload_templates`.

`para-projects-areas` references `anansi-config.plugin/references/templates/` directly by
design — Projects and Areas are stable PARA primitives and the field set is bounded.

`para-process` is a pure orchestrator and reads no templates directly.

---

## How to add or update a template

1. **Edit the single canonical copy** at `anansi-config.plugin/references/templates/`.
2. **Bump `template_version`** in the frontmatter. Use the convention: minor bump for additive field changes (new optional field, new `sources:` key); major bump for breaking changes (renamed required field, changed merge_strategy).
3. **Update this README's per-template section** if the change is structurally significant (new required field, semantic shift, citation update).
4. **Recompile and restart** the Anansi server so the new templates are picked up. If you also maintain a DB override, run `anansi_reload_templates` to refresh the runtime cache without a restart.

---

## Why this matters

Templates are the load-bearing connection between three loose layers:

1. **Forte's PARA framework** — the conceptual definitions of Project, Area, Resource, Archive. Every identity field and discovery hint should be traceable back to a citation in *Building a Second Brain* (2022) or *The PARA Method* (2023).
2. **The Smart Brevity input vocabulary** — the eleven document types the compression pass recognizes. Templates' `sources:` keys use this vocabulary so the same type tags flow through discovery → entity typing → compression.
3. **The anansi knowledge graph** — the concrete schema of stored notes. Templates' identity_fields define what gets stored, and merge_strategy defines how duplicates resolve.

When the templates are right, the layers compose cleanly: a meeting summary comes in, gets decomposed into topic_discussion content_units, those reference person/organization/area identities, and the smart-brevity pass compresses each one using the matching input-type rules. When the templates drift from any of those three layers, the whole pipeline produces noise.

The recent v2.1 / v3.1 updates close gaps that had been silently accumulating: the `standard` field that Forte explicitly defines as load-bearing for Areas was missing from v3.0; the `sources:` keys had drifted from Smart Brevity's vocabulary; and several field descriptions lacked the Forte citations that would let the LLM disambiguate edge cases. The result was a pipeline that mostly worked but lost specificity on hard cases — exactly the situation focused per-pass skills like `para-projects-areas` and `para-resource-entities` are designed to address.
