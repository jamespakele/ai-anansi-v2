---
id: template-schema
type: rule
status: active
version: "2.0"
last_updated: 2026-04-23
---

# Template Schema Reference

This document defines the format for template files in `anansi/templates/`.

## File Format

Each template is `templates/{entity_type}.md` with three sections:

1. **YAML frontmatter** (between `---` delimiters)
2. **`%% field %%` blocks** (zero or more, each defining one extractable field)
3. **Body template** (everything after the last `%%` block)

## Frontmatter Fields

### Required

| Field | Type | Description |
|---|---|---|
| `entity_type` | string | Matches filename stem. Used as the registry lookup key. |
| `atomic` | bool | `true` if this type produces atomic notes. `false` for non-atomic source types. |
| `merge_strategy` | enum | `pure_atomic` \| `container` \| `source_bound` |
| `template_version` | string | Schema version. Currently `"2.0"`. |
| `description` | string | Short human description of what this entity type represents. |

### Recommended

| Field | Type | Description |
|---|---|---|
| `atomic_criteria` | string | What makes this type atomic; when to split into a new note vs enrich existing. |
| `identity_fields` | map | Field definitions for the identity zone. Each key is a field name. |
| `sources` | map | Per-source-type extraction hints. Keys: `meeting_summary`, `email_thread`, `research_paper`, `container`. |

### Container-Only

| Field | Type | Description |
|---|---|---|
| `roster_sections` | map | Declared mergeable list sections. See below. |

## identity_fields Schema

```yaml
identity_fields:
  field_name:
    type: string        # string | bool | integer
    required: true      # optional, defaults false
    format: email       # optional: email | phone | prose | bullets | numbered | table
    description: "..."  # what to extract
```

## sources Schema

```yaml
sources:
  meeting_summary:
    hint: "What to look for and what NOT to include when extracting from meeting notes"
  email_thread:
    hint: "..."
  research_paper:
    hint: "..."
  container:
    hint: "..."
```

## roster_sections Schema (container types only)

```yaml
roster_sections:
  section_key:
    source_field: people        # key in Pass 3 output JSON roster object
    render_as: "## People"      # markdown heading used in the note file
    row_format: "- [[-{slug}|{name}]] — {role}"  # row template with {placeholders}
    dedupe_by: [slug, role]     # fields that together identify a unique row
```

## %% Field Blocks

After the closing `---` of the frontmatter, add one block per extractable field:

```
%%
field: field_name
description: What to extract for this field
format: prose          # optional: prose | bullets | numbered | table
constraints: "..."     # optional: rules the LLM must follow for this field
%%
```

These blocks are parsed into the `{TEMPLATE_FIELDS}` placeholder in the
Pass 3 prompt. They define what the LLM should extract for each field.

## Body Template

Everything after the last `%%` block is the body template. Use
`{{field_name}}` placeholders — they are substituted at write time with
the values extracted by Pass 3.

To include a literal `{` or `}` in the body, escape with `\{` or `\}`.
The renderer unescapes these after substitution.

## Adding a New Entity Type

1. Create `templates/<entity_type>.md` with the correct `merge_strategy`.
2. If `merge_strategy: container`, declare `roster_sections`.
3. Add `sources` hints for each source type the daemon will process.
4. Restart the daemon — templates are loaded at startup.

No Rust changes required for new types.
