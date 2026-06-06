# Spec: Anansi Write Wrapper Skills

**For:** anansi.plugin development agent  
**From:** Konohiki architecture review (2026-06-06)  
**Priority:** High — blocking scheduled skill execution  

---

## Background

Anansi's MCP server blocks write operations (`anansi_capture`, `anansi_update_note`, `anansi_archive_note`, and likely `anansi_relate`) when called directly from a scheduled execution context. The server inspects a `source` field on the request: direct MCP calls carry `source: "mcp"` (blocked); calls routed through the anansi skill layer carry `source: "skill"` (permitted).

The existing skill layer already wraps the two highest-frequency writes:

| Direct MCP tool | Skill wrapper | Status |
|---|---|---|
| `anansi_capture` | `anansi:anansi-atom` | ✅ exists |
| `anansi_update_note` (single field update) | `anansi:anansi-update` | ✅ exists |
| `anansi_update_note` (full document ingest) | `anansi:anansi-remember` | ✅ exists |
| `anansi_archive_note` | **missing** | ❌ needs build |
| `anansi_relate` | **missing** | ❌ needs build |

This spec covers the two missing wrappers.

---

## Skill 1: `anansi:anansi-archive`

### Purpose
Soft-archive a single Anansi note by ID or match_key. Routes through the skill layer so `source: "skill"` is set, permitting execution in scheduled context.

### Trigger phrases
`"anansi-archive"`, `"/anansi-archive"`, `"archive this note"`, `"soft-archive [name]"`, or when invoked programmatically by pm-review Face 2 (merge flow).

### Inputs

| Field | Required | Notes |
|---|---|---|
| `note_id` | One of these two is required | Anansi note UUID |
| `match_key` | One of these two is required | e.g. `project:konohiki-plugin` |
| `reason` | No | Short human-readable reason — appended to the note before archiving |

Resolution order: if `note_id` is provided, use it directly. If only `match_key` is provided, resolve to UUID via `anansi_get(match_key=...)` first.

### Behavior

1. If `reason` is provided: call `anansi_update_note` to append `⚙️ AI [timestamp]: archived — {reason}` to the note's `content` before archiving. This satisfies the Explainability Contract (os.md §9).
2. Call `anansi_archive_note(note_id=resolved_id)`.
3. Confirm: `"Archived: {note name} ({match_key or id})"`

### Error handling
- Note not found: report and stop. Do not error out silently.
- Note already archived: report and stop (idempotent — not a failure).

### SKILL.md location
`anansi.plugin/skills/anansi-archive/SKILL.md`

### Constraints
- Reversible — archive only, never delete. Callers that need hard-delete use `anansi:anansi-delete` (already exists).
- Always append an Explainability Contract line before archiving if a reason is given.
- Resolving match_key → UUID requires one `anansi_get` read call before the archive write — this is acceptable.

---

## Skill 2: `anansi:anansi-relate`

### Purpose
Write one or more edges between Anansi notes. Routes through the skill layer so `source: "skill"` is set, permitting execution in scheduled context.

### Trigger phrases
`"anansi-relate"`, `"/anansi-relate"`, `"wire these edges"`, `"add edge [from] to [to]"`, or when invoked programmatically by any konohiki skill that writes graph edges.

### Inputs

Single edge form:

| Field | Required | Notes |
|---|---|---|
| `source_id` | Yes | UUID of the source note |
| `target_id` | Yes | UUID of the target note |
| `relation` | Yes | Edge type string, e.g. `part_of`, `contains`, `under_area`, `source`, `inferred_deadline_from`, `depends_on` |
| `metadata` | No | Key-value object for edge metadata (e.g. `{"buffer_override": "7d"}`) |

Batch form: accept an array of `{source_id, target_id, relation, metadata?}` objects to write multiple edges in one invocation.

### Behavior

1. For each edge (single or batch):
   - Call `anansi_relate(source_id, target_id, relation, metadata)`.
   - On success: note the edge in the confirmation output.
   - On failure: report the failed edge and continue with remaining edges (do not abort the batch).
2. Confirm: `"Edges written: {N} — {relation} pairs listed"` or a per-edge line if N ≤ 3.

### Error handling
- Duplicate edge (already exists): treat as success (idempotent) — do not error.
- Source or target UUID not found: report and skip that edge. Do not abort.
- Unknown relation type: warn and write anyway — the relation string is free-form in Anansi.

### SKILL.md location
`anansi.plugin/skills/anansi-relate/SKILL.md`

### Constraints
- Does not validate that source/target nodes exist before writing — callers are responsible for resolving UUIDs first (via `anansi_get`).
- Batch mode is the preferred call pattern for callers writing 2+ edges for a single node (e.g. `part_of` + `contains` in pm-new-project).
- Insert-if-not-exists semantics are preferred — the skill should not surface duplicate-edge errors to the user.

---

## Affected callers (konohiki.plugin)

Once both skills exist, the following skills need their SKILL.md updated to reference the new wrappers:

### `anansi:anansi-relate` callers
| Skill | Current text to replace | New reference |
|---|---|---|
| pm-capture-route | `anansi_relate → under_area edge` etc. | `anansi:anansi-relate` |
| pm-intake | `anansi_relate — writes task→source and source→project back-edges` | `anansi:anansi-relate` |
| pm-new-area | `anansi_relate part_of … anansi_relate contains …` | `anansi:anansi-relate` (batch: 2 edges) |
| pm-new-project | `anansi_relate part_of … anansi_relate contains … anansi_relate under_area …` | `anansi:anansi-relate` (batch: 3 edges) |
| pm-review | Face 1 reconciliation — `anansi_relate part_of / contains` | `anansi:anansi-relate` |
| pm-seed-ontology | `anansi_relate → part_of … anansi_relate → contains …` | `anansi:anansi-relate` (batch: 2 edges per node) |
| pm-relate | `anansi_relate — create edges on high-confidence matches` | `anansi:anansi-relate` |

### `anansi:anansi-archive` callers
| Skill | Current text to replace | New reference |
|---|---|---|
| pm-review | Face 2 — `anansi_archive_note` the loser on merge | `anansi:anansi-archive` |

---

## Testing notes

1. **Scheduled context test** — after build, trigger both skills from a scheduled job (not a live session) and confirm Anansi accepts the writes. This is the primary success criterion.
2. **Idempotency test** — call `anansi:anansi-relate` twice with the same edge pair. Confirm no error, confirm no duplicate edge in the graph.
3. **Archive + Explainability test** — call `anansi:anansi-archive` with a reason. Confirm the `⚙️ AI:` line was appended before the note was archived.
4. **Batch edge test** — call `anansi:anansi-relate` with a 3-edge batch (e.g. `part_of` + `contains` + `under_area` for a new project). Confirm all 3 edges written.

---

## Open question: `anansi_relate` block status

It is not yet confirmed whether `anansi_relate` is blocked in scheduled context the same way as `anansi_capture` and `anansi_update_note`. The safest assumption is yes — wrap it preemptively. If a live test shows `anansi_relate` direct calls succeed in scheduled context, the wrapper is still useful for consistency and does no harm.

---

## Delivery

Two new skill directories in `anansi.plugin/skills/`:
```
anansi.plugin/
  skills/
    anansi-archive/
      SKILL.md
    anansi-relate/
      SKILL.md
```

No scripts required — both skills invoke MCP tools directly from SKILL.md instructions. No changes to `anansi.plugin/.claude-plugin/plugin.json` needed (skills are auto-discovered from the `skills/` directory).

After delivery, the konohiki.plugin caller updates listed above can be applied as a separate pass.
