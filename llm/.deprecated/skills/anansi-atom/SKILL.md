---
name: anansi-atom
description: >
  Commits a single entity to the Anansi knowledge base via anansi_capture.
  Two modes: (1) typed capture -- Claude identifies the entity type, reads the
  matching template from references/templates/, applies Smart Brevity formatting,
  then calls anansi_capture; (2) quick-note mode -- skip all typing and
  formatting, stash the raw content immediately as a note with a timestamp in
  the why field, for when the user just wants to record something fast. Always
  use this skill when the user wants to remember a single named thing or jot
  something down quickly without running a full document through the pipeline.
  Triggers: "remember [name]", "save this to anansi", "capture this",
  "add [person/org/event] to the vault", "anansi-atom", "/anansi-atom",
  "commit this to anansi", "store this person/contact/event/note",
  "remember that [X] is [Y]", "quick note", "quick note:", "jot this down",
  "just save this", "stash this", "note to self", "qn:".
argument-hint: "[entity description, facts to capture, or quick note content]"
---

# anansi-atom

Commit a single named entity to the Anansi knowledge base.

Two modes. The user's phrasing tells you which one to use:

- **Typed capture** -- "remember John Doe, phone (808) 555-1234" -- identify the entity type, find the template, format properly, call `anansi_capture`
- **Quick note** -- "quick note: the iQ360 kickoff is rescheduled to June 3" -- skip all that, stash it raw with a timestamp, done

The user should never have to explain which mode they want. Read the intent.

---

## Mode detection

**Quick-note mode** when the user says any of:
- "quick note", "quick note:", "qn:", "note to self"
- "jot this down", "just save this", "stash this"
- Any phrasing that signals speed over structure -- "just remember that...", "don't let me forget..."

**Typed capture mode** for everything else -- named entities, contacts, organizations, events, facts about a specific named thing.

When in doubt and no name is clearly identifiable, default to quick-note mode. Speed and zero friction are more valuable than a perfectly typed note that never gets captured.

---

## Quick-note mode

No typing. No template. No formatting overhead. Just capture.

Quick notes are **intentionally ephemeral** -- they're a fast landing zone, not
a permanent vault entry. The expected lifecycle is:

1. Captured during the week as raw content with a timestamp
2. Surfaced during weekly review (queryable by `why` field starting with `quick-note`)
3. Either **promoted** (user says "make this a real note/person/org" -- re-run as typed capture) or **discarded** (deleted from the vault)

This keeps the database clean. Quick notes that never get reviewed and promoted
are noise -- the timestamp makes it easy to find and purge the whole cohort.

### How to capture

1. Use the raw content from the user's message as-is -- do not rewrite, summarize, or restructure it.
2. Generate a name: use the first few words of the content as a short title, e.g. "iQ360 kickoff rescheduled" or "Quick note 2026-05-01 10:32".
3. Call `anansi_capture` with:

```json
{
  "entity_type": "note",
  "name": "[short title from content]",
  "lede": "[first sentence or the full content if short]",
  "why": "quick-note [ISO timestamp]",
  "content": "[full raw content if longer than the lede]",
  "source": "skill"
}
```

The `why: "quick-note [timestamp]"` field is the lifecycle marker -- it's what
makes the whole cohort queryable for weekly review and bulk discard.

**Example:**

User says: "quick note: the iQ360 kickoff is rescheduled to June 3, Lynn confirmed"

Call:
```json
{
  "entity_type": "note",
  "name": "iQ360 kickoff rescheduled to June 3",
  "lede": "iQ360 kickoff rescheduled to June 3, Lynn confirmed.",
  "why": "quick-note 2026-05-01T10:32:00",
  "source": "skill"
}
```

Report:
```
*Anansi* -- quick note saved
* note:iq360-kickoff-rescheduled-to-june-3
* [quick-note -- surfaces in weekly review]
```

That's it. Fast.

### Promotion (when user wants to upgrade a quick note)

If the user later says "promote that quick note about iQ360" or "make that a
real entity":
- Re-run as typed capture mode with the same content
- The COALESCE upsert will merge into the existing note and update its fields
- The `why` field will be overwritten, removing the `quick-note` marker --
  effectively graduating it out of the ephemeral cohort

---

## Typed capture mode

### Step 1 -- Identify entity type and name

From the user's input, determine:

1. **Name** -- the canonical name of the entity (required for all types)
2. **Entity type** -- which anansi template fits best

### Quick type mapping

| What the user mentions | Entity type |
|---|---|
| A person, contact, individual human | `person` |
| A company, nonprofit, agency, team, institution | `organization` |
| A meeting, summit, workshop, conference, event | `event` |
| A project, initiative (user-owned, has end date) | `project` |
| An area of responsibility (user-owned, ongoing) | `area` |
| Anything else -- topic, concept, decision, fact | `note` |

When in doubt: read the entity templates in `../references/templates/` (files
named `entity-*.md`). Each template's frontmatter `description` field tells
you what kind of thing it represents. Use `entity-note.md` as the fallback.

---

### Step 2 -- Read the matching template

Read the template file that matches the entity type:

- `../references/templates/entity-person.md` for `person`
- `../references/templates/entity-organization.md` for `organization`
- `../references/templates/entity-project.md` for `project`
- `../references/templates/entity-area.md` for `area`
- `../references/templates/entity-note.md` for `note`
- `../references/templates/event.md` for `event`

Read the template's `identity_fields` section. Populate what you can from the
user's input. Omit fields you have no basis for. **Do not fabricate.**

---

### Step 3 -- Format the entity (the prework)

Build the `anansi_capture` payload using Smart Brevity conventions.

| `anansi_capture` param | What goes here |
|---|---|
| `entity_type` | The type string (`person`, `organization`, `event`, etc.) |
| `name` | Canonical entity name |
| `lede` | One-sentence lede -- who/what this is, crisp and active |
| `why` | Why it matters to the user -- context, relationship, relevance (optional) |
| `content` | Bullet-point identity fields -- one fact per bullet, bold labels |

**Lede:** One sentence. Active voice. Specific. No filler.
- Good: `"Key contact at JERA Americas, the LNG client on the iQ360 contract."`
- Bad: `"This is a person who works in the energy sector."`

**Content examples:**

For `person`:
```
- **Phone:** (808) 555-1234
- **Email:** john@example.com
- **Met via:** Introduced at the iQ360 kickoff
```

For `organization`:
```
- **Type:** Company
- **Domain:** LNG, energy
- **Context:** Client on active iQ360 contract as of 2026
```

For `event`:
```
- **Date:** 2026-05-15
- **Location:** Waianu Community Center
- **Organizer:** POW
- **Summary:** Annual community summit on food sovereignty and housing
```

If the user only said "remember John Doe, phone (808) 555-1234" -- you have name and phone. That's all you put in. Lede can be minimal: `"Contact in the Anansi network."` COALESCE will fill in the rest as more facts arrive.

---

### Step 4 -- Call `anansi_capture`

Always include `"source": "skill"` in every write call payload.

**Minimal:**
```json
{
  "entity_type": "person",
  "name": "John Doe",
  "lede": "Contact captured via conversation.",
  "content": "- **Phone:** (808) 555-1234",
  "source": "skill"
}
```

**Rich:**
```json
{
  "entity_type": "organization",
  "name": "JERA Americas",
  "lede": "US subsidiary of JERA Co.; LNG client on the iQ360 contract.",
  "why": "Primary client on the active iQ360 engagement.",
  "content": "- **Type:** Company\n- **Domain:** LNG, energy\n- **Context:** Client on active iQ360 contract as of 2026",
  "source": "skill"
}
```

---

### Step 5 -- Report

```
*Anansi* -- {name} [{entity_type}]
* {created | updated} -> {match_key from response}
```

If merged with an existing note: `* updated -> person:john-doe (merged with existing note)`

---

## COALESCE behavior

`anansi_capture` upserts by match_key -- calling again with the same name
merges incoming fields into the existing note rather than overwriting. No need
to check whether the entity exists first. Safe to call repeatedly.

---

## Multiple entities in one message

If the user gives you two things to capture, handle them as separate calls.
"Remember John and his company Continest" -> one call for `person:john-doe`,
one for `organization:continest`. Report both.

If the user provides a clear relationship ("John works at Continest") and
`anansi_relate` is available, optionally call it after both captures:

```json
{
  "from_key": "person:john-doe",
  "to_key": "organization:continest",
  "relationship": "employed_by",
  "source": "skill"
}
```

Only when the relationship is explicit. Don't infer edges.

---

## Error handling

| Situation | Action |
|---|---|
| Entity type ambiguous | Default to `note`. Note the choice. |
| Template file not found | Use `entity-note.md` fallback. |
| `anansi_capture` not available | Tell the user. Offer to show the formatted payload. |
| `anansi_capture` returns error | Show the error. Offer to retry or show payload. |
| No name identifiable (typed mode) | Switch to quick-note mode -- use content as-is. |
