---
name: resource-typer
description: >
  Sub-type each Resource from a para-extract output by matching it against
  anansi entity templates loaded from `references/templates/`. Returns the
  same extraction with `resource:[slug]` placeholder match keys replaced
  by real anansi-form keys: `person:[slug]`, `organization:[slug]`, or
  `note:[slug]`. Template-driven, not hardcoded — adding a new entity_type
  is dropping a new `entity-[type].md` file into `references/templates/`.
  Use when user says "type these resources", "sub-type the para-extract",
  "/resource-typer", "what kind of resource is X?", or as the second step
  after `/para-extract` in an atomization pipeline. Templates are anansi's
  authoritative entity definitions; the skill's job is to read each
  template's description + identity_fields and assign each Resource to the
  best-fitting type.
argument-hint: "[para-extract output, or a list of Resources to sub-type]"
---

# resource-typer

A template-driven entity classifier. Take a `para-extract` output (or a list of un-typed Resources) and assign each Resource an anansi `entity_type` by matching against templates loaded from `references/templates/`.

This is **named-entity sub-typing**, anansi-flavored. The templates ARE the taxonomy — adding a new type is dropping a new `entity-<type>.md` file into `references/templates/` with the right frontmatter shape. No skill code changes required.

---

## When to invoke

Trigger this skill when the user:

- Says "type these resources", "sub-type the para-extract", "what kind of resource is X?"
- Says "/resource-typer" or any sub-typing phrasing
- Pastes a `para-extract` output and wants the `resource:` placeholders resolved to real entity_types
- Is running an atomization pipeline and needs the second step (after `/para-extract`)

Do not invoke for: the initial PARA classification (use `para-extract`), single-item judgment (use `para-classify`), or rewriting the input text (use `smart-brevity`).

---

## What it does

For every Resource in the input:

1. Read the templates in `references/templates/` (each is an `entity-*.md` file with anansi frontmatter).
2. For each template, compare the Resource's name + evidence quote against the template's `description`, `identity_fields`, and `sources` hints.
3. Pick the best-fitting template.
4. Rewrite the Resource's `vault_match_hint` from `resource:<slug>` to `<entity_type>:<slug>`.
5. Add a `Type:` line and a `Reasoning:` line explaining the pick.

Projects, Areas, and Concept tags from the input pass through unchanged — only Resources get sub-typed.

---

## Templates as the type system

Every file in `references/templates/` matching `entity-*.md` is a candidate type. The skill reads each template's frontmatter to learn:

- **`entity_type`** — the canonical type label (`person`, `organization`, `note`)
- **`description`** — what kind of thing this represents, in plain language
- **`identity_fields`** — the data shape (e.g., person has `contact_email`, `contact_phone`; organization has `domain`, `type`)
- **`sources`** — per-source extraction hints (cues for what to look for in different document types)

The match is reasoning-based: given the Resource's name and evidence, which template's combined signals fit best?

### Templates in scope

Read every `entity-*.md` file in `references/templates/` at invocation time — do not use a hardcoded list. The full set as of the current plugin build:

| File | entity_type | template_class | When to pick |
|---|---|---|---|
| `entity-person.md` | `person` | identity | Named individual human being — name, contact, role. |
| `entity-organization.md` | `organization` | identity | Named group, company, NGO, government body, team, agency. |
| `entity-area.md` | `area` | identity | An ongoing domain of responsibility with no end date (PARA Area). Only if user-owned — otherwise `note`. |
| `entity-project.md` | `project` | identity | A bounded effort with a clear outcome and deadline (PARA Project). Only if user-owned — otherwise `note`. |
| `entity-note.md` | `note` | identity | Catchall fallback — substantial named subject that isn't any more specific type. Topics, referenced documents, concepts worth their own page. |

**Note:** `entity-area.md` and `entity-project.md` are included as identity templates but should only be assigned when the Resource is user-owned (first-person signals). Third-party projects and areas that the user is merely referencing belong in `note`, not `project` or `area` — that distinction is `para-extract`'s job.

New templates are picked up automatically. To add or remove templates, use `para-pipeline add-template` / `para-pipeline remove-template`.

---

## Decision rules

For each Resource:

### Step 1 — Load templates

Read all `entity-*.md` files from `references/templates/`. Parse each template's `entity_type`, `description`, `identity_fields`, and `sources` hints. This is your candidate type set.

### Step 2 — Walk templates in specificity order

Test each template against the Resource's name and evidence quote. Work from most-specific to least-specific — `note` is always the last candidate since it is the declared catchall (`entity-note.md`).

General specificity ordering (most → least specific):
1. `person` — named individual human being
2. `organization` — named group, company, agency, institution
3. `area` / `project` — only when user-owned (first-person signals); otherwise skip and continue
4. Any other identity templates present in `references/templates/`
5. `note` — catchall fallback

For each template, ask: do the Resource's name shape and evidence quote match this template's `description` and `identity_fields`?

**Person signals:** first-name + last-name shape, titles (Dr., Prof., CEO), evidence verbs like "said", "wrote", "presented", "argued", personal pronouns.

**Organization signals:** named group with employees or members, corporate-shaped capitalized name, evidence frames it as an institutional affiliation ("from X", "at X", "X announced").

**Area/Project signals:** first-person possessives ("our", "my", "we"), active commitment verbs, user-owned framing. If third-party framing, skip — belongs in `note`.

**Note signals:** substantial named subject that doesn't match any more specific template — topics, referenced documents, frameworks, named initiatives the user doesn't own.

### Step 3 — Pick and assign

Assign the first template that clearly fits. If multiple templates fit equally, **prefer the more specific**. `note` is only assigned when no other template matches.

### Step 4 — Tie-breaking

If two non-note templates seem equally valid, pick the one whose `identity_fields` fit the evidence best (e.g., does the document mention an email → `person` has `contact_email`). Note the ambiguity; emit medium confidence.

If no template fits cleanly, default to `note:<slug>` and flag low confidence with a Notes entry.

---

## Output format

The output is the **same para-extract structure** with Resource section updated. Projects, Areas, Concepts, Notes pass through unchanged.

```
# Resource sub-typing for: <source title>

## Projects

(unchanged from input)

## Areas

(unchanged from input)

## Resources (typed)

### person:<slug>          ← was resource:<slug>
- **Name:** <canonical name>
- **Type:** person
- **Email:** <extracted from From/To/CC headers or signature block; omit if not found>
- **Phone:** <extracted from signature block; omit if not found>
- **Met via:** <one sentence — project, email thread, introducing person; omit if not inferable>
- **Reasoning:** <one sentence — which template's signals fit, and why>
- **Confidence:** <high | medium | low>
- **Evidence:** "<quote from input>"
- **Vault match hint:** person:<slug>

### organization:<slug>
- **Name:** <canonical name>
- **Type:** organization
- **Reasoning:** <one sentence>
- **Confidence:** <high | medium | low>
- **Evidence:** "<quote>"
- **Vault match hint:** organization:<slug>

### note:<slug>
- **Name:** <canonical name>
- **Type:** note
- **Reasoning:** <one sentence>
- **Confidence:** <high | medium | low>
- **Evidence:** "<quote>"
- **Vault match hint:** note:<slug>

## Concepts

(unchanged from input)

## Notes

- (carry over input's Notes)
- (add any sub-typing observations: ambiguous picks, low-confidence flags, suggestions for new templates)
```

If the input wasn't a para-extract output (e.g., user pasted a raw list of Resources), skip the Projects/Areas/Concepts sections and emit only the typed Resource list.

---

## Process

When invoked:

1. **Parse the input.** Identify the Resources section. If the input is a full para-extract output, retain the other sections to pass through unchanged.
2. **Load templates.** Read every `entity-*.md` file in `references/templates/`. For each, parse the frontmatter to extract `entity_type`, `description`, `identity_fields`, `sources` hints. Build a candidate type table ordered by specificity (person/organization first, note last).
3. **For each Resource, walk the decision rules.** Person test → Organization test → Note fallback. For each pick, draft a one-sentence reasoning citing the template signals that fit.
4. **Compute the new vault_match_hint.** Replace `resource:<slug>` with `<entity_type>:<slug>`.
5. **Calibrate confidence:**
   - **high** — clear match on multiple signals (proper-noun shape + corroborating evidence + template's identity_fields fit naturally).
   - **medium** — the type is right but signals are sparse, or one signal contradicts.
   - **low** — fell to `note` as fallback without strong note-shaped signal, OR the input is too short to support strong reasoning.
6. **Emit the structured output.** Pass Projects/Areas/Concepts/Notes from input; rewrite Resources with type fields.
7. **Add Notes entries** for: low-confidence sub-typings, Resources where multiple templates fit (ambiguity), and suggestions for new templates if the user's content has a recurring shape that doesn't fit well into any current type.

---

## Worked example

**Input** (a para-extract output):

```
# PARA extraction: Q3 Board Deck — please review (email)

## Projects

### project:q3-board-deck
- **Name:** Q3 Board Deck
- **Evidence:** "I've drafted the Q3 board deck and would love your eyes on it before May 15"
- **Outcome:** finalize Q3 board deck
- **Deadline:** May 15
- **Confidence:** high
- **Vault match hint:** project:q3-board-deck

## Areas

### area:dcs-board-operations
- **Name:** DCS Board Operations
- **Evidence:** "next Tuesday's DCS board meeting"
- **Confidence:** high
- **Vault match hint:** area:dcs-board-operations

## Resources

### resource:ron-nishihara
- **Name:** Ron Nishihara
- **Evidence:** "Ron Nishihara from Continest will be presenting"
- **Confidence:** high
- **Vault match hint:** resource:ron-nishihara

### resource:continest
- **Name:** Continest
- **Evidence:** "Ron Nishihara from Continest will be presenting the modular housing pilot"
- **Confidence:** high
- **Vault match hint:** resource:continest

### resource:modular-housing-pilot
- **Name:** Modular Housing Pilot
- **Evidence:** "the modular housing pilot — I think it's worth a section on how it ties into our sovereignty work"
- **Confidence:** medium
- **Vault match hint:** resource:modular-housing-pilot

## Concepts

- `#sovereignty` — "how it ties into our sovereignty work"
```

**Output:**

```
# Resource sub-typing for: Q3 Board Deck — please review

## Projects

### project:q3-board-deck
(unchanged)

## Areas

### area:dcs-board-operations
(unchanged)

## Resources (typed)

### person:ron-nishihara
- **Name:** Ron Nishihara
- **Type:** person
- **Reasoning:** First-name + last-name shape; evidence verb "will be presenting" (a person-action). Matches `entity-person.md` description ("a unique individual").
- **Confidence:** high
- **Evidence:** "Ron Nishihara from Continest will be presenting"
- **Vault match hint:** person:ron-nishihara

### organization:continest
- **Name:** Continest
- **Type:** organization
- **Reasoning:** Capitalized name with no person-shape; "from Continest" frames it as an institutional affiliation. Matches `entity-organization.md` description ("named group — company, ngo, government body, team").
- **Confidence:** high
- **Evidence:** "Ron Nishihara from Continest will be presenting the modular housing pilot"
- **Vault match hint:** organization:continest

### note:modular-housing-pilot
- **Name:** Modular Housing Pilot
- **Type:** note
- **Reasoning:** Substantial named subject; not a person or organization. Falls to `entity-note.md` (catchall for substantial named subjects worth their own vault page). The "pilot" framing suggests it could be a project for some other party (Continest), but for the user it's a referenced topic.
- **Confidence:** medium
- **Evidence:** "the modular housing pilot — I think it's worth a section on how it ties into our sovereignty work"
- **Vault match hint:** note:modular-housing-pilot

## Concepts

- `#sovereignty` — "how it ties into our sovereignty work"

## Notes

- "Modular Housing Pilot" sub-typed `note` at medium confidence. If it's actually Continest's named project (which para-extract didn't confirm), and you want to track it as such, consider adding an `entity-pilot.md` or `entity-initiative.md` template — or just keep as `note` and use a tag.
- All other sub-types confident.
```

---

## Edge cases

### "Resource is ambiguous between person and organization"

Some names are ambiguous — "Pakele" could be a person's last name OR a company name. Use the evidence verb to disambiguate:
- "Pakele said…" → person
- "Pakele's marketing department…" → organization
- Truly unclear → flag in Notes; pick the more probable based on context; emit medium confidence.

### "Resource is a project or area belonging to a third party"

"Apple's iPhone 18 launch" — Apple's project, but to the user it's a Resource (per para-extract). Sub-type: this is a `note` (substantial named subject). Don't try to sub-type it as a "third-party project" — anansi doesn't have that as a template, and the Resource is reference material, not actionable.

### "Resource is a referenced document"

"the Q3 OKR doc," "yesterday's all-hands transcript" — sub-type as `note`. Anansi has separate source-type templates for documents (`meeting-summary.md`, `email-thread.md`, etc.) but those are for documents being **ingested**, not documents being **referenced**. Referenced docs sit as notes.

### "Multiple templates seem equally valid"

Walk the tie-breaking rule: prefer the more specific (person/organization > note). If still tied, pick based on which template's `identity_fields` fit best (e.g., person has `contact_email` — does the document mention an email address?). Note the ambiguity; emit medium confidence.

### "No template fits"

Fall to `note` as the catchall. Note in the output that the type is the catchall and that adding a new template might be warranted if this shape recurs.

### "Resource appears multiple times in the input"

Already deduplicated by para-extract. If somehow the typer sees duplicates, type once and merge.

---

## What this skill does NOT do (out of scope)

- **Does not re-classify Projects or Areas.** Those came from para-extract with the right tier; the typer only handles Resources.
- **Does not look up entities in the vault.** Output is hint-only; vault lookup is the next downstream skill (`para-route`).
- **Does not create or edit anansi templates.** Use `para-pipeline add-template` / `para-pipeline remove-template` for that.
- **Does not handle Concepts.** Concept tags pass through unchanged.
- **Does not run para-extract first** — it expects a para-extract output (or compatible Resource list) as input. If user pastes raw text, ask them to run `/para-extract` first.

---

## Why this matters

Resource sub-typing turns para-extract's flat `resource:` placeholders into real anansi match keys, which is what the downstream lookup step (`para-route`) needs to query the vault correctly.

The template-driven design means the type system grows with the user's needs. New templates dropped in → new types reachable. The skill is a thin layer over the templates — the templates are the knowledge.

The companion `para-extract` skill identifies *that* something is a Resource. This skill identifies *what kind*. They compose: para-extract → resource-typer → para-route.
