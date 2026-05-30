---
name: anansi-update
description: >
  Updates an existing Anansi note in place. Retrieves the record by name or
  match_key, shows the current state, then writes updated field values back via
  anansi_capture (COALESCE upsert — only supplied fields change; unmentioned
  fields are preserved). Supports partial updates: the user only needs to
  provide what changed. Use when James says "update [entity]", "change [field]
  on [name]", "edit this note", "fix the lede for [name]", "add [field] to
  [entity]", "update the why for [name]", "correct [name]", "/anansi-update",
  or wants to enrich or correct an existing vault entry without creating a new
  one.
argument-hint: "[entity name or match_key] [what to change]"
---

# anansi-update

Find an existing Anansi record, show the user what's there, apply their
changes, and write it back.

This is a targeted edit skill, not a pipeline. The COALESCE upsert behavior of
`anansi_capture` does the heavy lifting — supplying a field overwrites it;
omitting a field leaves it untouched.

---

## Step 1 — Find the record

From the user's input, extract the entity **name or match_key** they want to
update.

Match_key format: `{entity_type}:{slug}` — e.g. `person:james-pakele`,
`org:iq360`. Slug is lowercase, hyphens, no special chars.

**Try `anansi_get` first** (fast, exact):

```json
{ "match_key": "person:john-doe" }
```

If `anansi_get` returns nothing, fall through to `anansi_search`:

```json
{ "q": "John Doe" }
```

Pick the best match from results. If multiple candidates surface, list them
and ask the user to confirm which one before continuing.

**If no match found:** Tell the user clearly.

> "No record found for 'John Doe' in the vault. Did you mean someone else, or
> would you like to create this entry via anansi-atom instead?"

Do not proceed without a confirmed match.

---

## Step 2 — Show the current record

Before making any changes, display what's currently stored. This lets the user
catch stale data and confirm they have the right entity.

Format:

```
*Anansi* — current record
• *match_key:* person:john-doe
• *lede:* Key contact at JERA Americas on the iQ360 contract.
• *why:* Introduced at the Q1 kickoff; primary point of contact for LNG negotiations.
• *content:*
  - **Phone:** (808) 555-1234
  - **Email:** john@example.com
```

Only show fields that have content. Omit blank/null fields.

Ask the user to confirm the fields and values they want to update if they
haven't already stated them clearly.

---

## Step 3 — Build the update payload

Construct the `anansi_capture` payload with **only the fields the user wants
to change**, plus the required identity fields:

- `entity_type` — must match the existing record
- `name` — canonical name (must match to trigger COALESCE)

**Do not include fields the user hasn't mentioned.** Omitted fields stay as-is
in the vault. This is what makes it a safe partial update.

**Example — updating a phone number:**

```json
{
  "entity_type": "person",
  "name": "John Doe",
  "content": "- **Phone:** (808) 555-9999\n- **Email:** john@example.com",
  "source": "skill"
}
```

**Example — updating just the why:**

```json
{
  "entity_type": "person",
  "name": "John Doe",
  "why": "Now the primary decision-maker at JERA Americas following Q2 leadership change.",
  "source": "skill"
}
```

**Example — updating the lede:**

```json
{
  "entity_type": "person",
  "name": "John Doe",
  "lede": "Senior VP at JERA Americas; primary contact for LNG contract.",
  "source": "skill"
}
```

Always include `"source": "skill"` in every write call.

---

## Step 4 — Call `anansi_capture`

Fire the payload. The COALESCE upsert will merge the supplied fields into the
existing record without touching anything else.

---

## Step 5 — Report

```
*Anansi* — updated: {name} [{entity_type}]
• match_key: {match_key}
• fields changed: {list of updated fields}
```

If the response indicates a new record was created rather than merged
(shouldn't happen if name matches), flag it:

> "⚠ New record created instead of updating — name may not have matched.
> Check: {match_key}"

---

## Content field updates

When the user wants to update a specific field **within** the `content` block
(e.g. change a phone number), you must reconstruct the full content string
with the updated value. Pull the existing content from Step 2's snapshot,
apply the edit, and submit the complete updated block.

Never submit a partial content string that only contains the changed line —
that would overwrite all existing content fields with just the one line.

---

## Edge cases

| Situation | Action |
|---|---|
| Multiple search matches | List all candidates, ask user to confirm before proceeding |
| User wants to add a NEW content field | Add the new field to the existing content block, keep everything else |
| User wants to clear a field | Set it explicitly to empty string (`""`) or a null-equivalent replacement |
| Entity type unknown from recall | Ask: "What type is this — person, org, project, area, event, or note?" |
| `anansi_capture` not available | Tell the user. Show the payload they'd need. |
| `anansi_capture` returns error | Show the error, do not retry silently. |
