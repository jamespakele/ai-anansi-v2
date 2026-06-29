---
id: template-mapping-rules
type: rule
rule: template-mapping
status: active
version: "1.0"
last_updated: 2026-06-28
---

# Template-to-DB Mapping Rules

Maps each template field to its corresponding database column. Used by the
Web Tender for `template_drift` detection and by ingest pipelines for field
routing.

## Global Field Mapping

| Template Field     | DB Column       | Notes                                    |
|--------------------|-----------------|------------------------------------------|
| `name`             | `name`          | Normalized per `normalization.md`        |
| `match_key`        | `match_key`     | Slug derived from normalized name        |
| `entity_type`      | `entity_type`   | Must match a known entity type           |
| `lede`             | `lede`          | One-sentence summary                     |
| `why`              | `why`           | Why this entity matters                   |
| `content`          | `content`       | Free-text body                           |
| `source_id`        | (contribution)  | Stored in `contributions` table          |
| `aliases`          | (not stored)    | Used for matching only, not persisted    |
| `tags`             | (not stored)    | Used for matching only, not persisted    |

## Entity Type Field Requirements

### Person

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case                     |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `person`                |
| `lede`      | yes      | TEXT     | Who they are, one sentence     |
| `why`       | no       | TEXT     | Relevance to the vault         |
| `content`   | no       | TEXT     | Biography, role, context       |

### Organization

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case                     |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `organization`          |
| `lede`      | yes      | TEXT     | What the org does              |
| `why`       | no       | TEXT     | Relevance to the vault         |
| `content`   | no       | TEXT     | Description, size, location    |

### Concept

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Lowercase                      |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `concept`               |
| `lede`      | yes      | TEXT     | Definition, one sentence       |
| `why`       | no       | TEXT     | Why it matters                 |
| `content`   | no       | TEXT     | Deep dive, examples            |

### Topic

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Lowercase                      |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `topic`                 |
| `lede`      | yes      | TEXT     | Scope of the topic             |
| `why`       | no       | TEXT     | Why it's being tracked         |
| `content`   | no       | TEXT     | Notes, references              |

### Project

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case                     |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `project`               |
| `lede`      | yes      | TEXT     | Outcome + deadline             |
| `why`       | no       | TEXT     | Why this project exists        |
| `content`   | no       | TEXT     | Status, milestones, notes      |

### Area

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case                     |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `area`                  |
| `lede`      | yes      | TEXT     | Standard of responsibility     |
| `why`       | no       | TEXT     | Why this area matters          |
| `content`   | no       | TEXT     | Ongoing notes, references      |

### Event

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case                     |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `event`                 |
| `lede`      | yes      | TEXT     | What + when + where            |
| `why`       | no       | TEXT     | Significance                   |
| `content`   | no       | TEXT     | Agenda, notes, outcomes        |

### Context

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Lowercase descriptive label    |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `context`               |
| `lede`      | yes      | TEXT     | What this context covers       |
| `why`       | no       | TEXT     | Why it was captured            |
| `content`   | no       | TEXT     | Full context body              |

### Note

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case, first word capped  |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `note`                  |
| `lede`      | yes      | TEXT     | One-sentence summary           |
| `why`       | no       | TEXT     | Why this note exists           |
| `content`   | no       | TEXT     | Full note body                 |

### Task

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case, first word capped  |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `task`                  |
| `lede`      | yes      | TEXT     | Actionable one-liner           |
| `why`       | no       | TEXT     | Why this task matters          |
| `content`   | no       | TEXT     | Details, checklist, notes      |

### ActionItemList

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case, first word capped  |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `action_item_list`      |
| `lede`      | yes      | TEXT     | Context for the action items   |
| `why`       | no       | TEXT     | Why this list exists           |
| `content`   | no       | TEXT     | Structured action items        |

### Outline

| Field       | Required | Type     | Notes                          |
|-------------|----------|----------|--------------------------------|
| `name`      | yes      | TEXT     | Title Case, first word capped  |
| `match_key` | yes      | TEXT     | Slug from name                 |
| `entity_type` | yes    | TEXT     | Always `outline`               |
| `lede`      | yes      | TEXT     | What this outline covers       |
| `why`       | no       | TEXT     | Why it was created             |
| `content`   | no       | TEXT     | Numbered outline body          |

## Template Drift Detection

The Web Tender checks for `template_drift` by comparing each entity's stored
fields against the template requirements above:

1. **Missing required field** → `severity: 'high'`
2. **Missing optional field** → `severity: 'low'`
3. **Extra field not in template** → `severity: 'info'` (may indicate a
   custom extension or data corruption)
4. **Field type mismatch** (e.g. TEXT stored where JSON expected) →
   `severity: 'high'`
