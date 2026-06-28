---
name: anansi-relate
description: >
  Write one or more edges between Anansi notes. Routes through the skill layer
  so source: "skill" is set, permitting execution in scheduled context.
  Supports single-edge and batch modes. Insert-if-not-exists semantics —
  duplicate edges are treated as success, not errors. Use when the user says
  "anansi-relate", "/anansi-relate", "wire these edges", "add edge [from] to
  [to]", "link [A] to [B]", "connect these notes", or when invoked
  programmatically by any skill that writes graph edges (e.g. pm-capture-route,
  pm-intake, pm-new-area, pm-new-project, pm-review, pm-seed-ontology,
  pm-relate).
argument-hint: "[source_id] [target_id] [relation] or batch array"
---

# anansi-relate

Write one or more edges between Anansi notes. The skill wrapper that routes
`anansi_relate` calls through the skill layer (`source: "skill"`), which is
required for writes in scheduled execution contexts.

> **Insert-if-not-exists.** The underlying `anansi_relate` tool uses
> `insert_edge_if_not_exists` — calling it twice with the same edge pair
> produces no duplicate and no error. This makes the skill safe for
> idempotent callers.

---

## Inputs

### Single edge form

| Field | Required | Notes |
|---|---|---|
| `source_id` | Yes | UUID of the source note |
| `target_id` | Yes | UUID of the target note |
| `relation` | Yes | Edge type string, e.g. `part_of`, `contains`, `under_area`, `source`, `inferred_deadline_from`, `depends_on` |
| `why` | No | Reason for the edge (stored as edge metadata) |

### Batch form

Accept an array of edge objects to write multiple edges in one invocation.
Each object has: `{ source_id, target_id, relation, why? }`.

**Batch mode is the preferred call pattern** for callers writing 2+ edges for
a single node (e.g. `part_of` + `contains` in `pm-new-area`, or `part_of` +
`contains` + `under_area` in `pm-new-project`).

**Example — batch input from a caller:**

```
Edges:
- source_id: {project_uuid}, target_id: {area_uuid}, relation: part_of
- source_id: {area_uuid}, target_id: {project_uuid}, relation: contains
- source_id: {project_uuid}, target_id: {parent_area_uuid}, relation: under_area
```

---

## Step 1 — Wiki check (runs first if wiki is enabled)

If the local wiki exists at `~/llm-wiki/` and the edge references use
**match_keys** (`type:slug`) instead of UUIDs, delegate to `wiki-relate`
first to add the `[[wikilink]]` locally:

1. Read `../wiki-relate/SKILL.md`.
2. Execute it with the source match_key, relation, and target match_key.
3. Then proceed to Step 2 to also write the edge on the server.

If the edge references use UUIDs only, skip the wiki step — match_keys
would need a DB lookup to resolve.

---

## Step 2 — Validate inputs

For each edge (single or batch):

- Verify `source_id`, `target_id`, and `relation` are present and non-empty.
- If any required field is missing, report the error for that edge and skip it.
  Do not abort the entire batch.

The `relation` string is **free-form** in Anansi — there is no fixed enum of
valid values. Accept any non-empty string. If the caller passes an unusual
relation type, proceed without warning.

---

## Step 3 — Write edges

For each edge, call `anansi_relate`:

```json
{
  "source_id": "{source_id}",
  "target_id": "{target_id}",
  "edge_type": "{relation}",
  "why": "{why}",
  "source": "skill"
}
```

> **Field mapping:** The spec uses `relation` for readability; the MCP tool
> parameter is `edge_type`. Map `relation` → `edge_type` in the call.

> **Always include `"source": "skill"`** in every call payload.

**On success:** note the edge in the confirmation output. If the response
shows `"inserted": false`, the edge already existed — treat as success
(idempotent).

**On failure:** report the failed edge and continue with remaining edges.
Do not abort the batch.

---

## Step 4 — Report

**For a single edge:**

```
*Anansi* — edge written ✓
• {source_id} —[{relation}]→ {target_id}
• {inserted: true | already existed}
```

**For a batch (N ≤ 3):** list each edge on its own line:

```
*Anansi* — {N} edges written ✓
• {source_id} —[{relation}]→ {target_id} {✓ | already existed}
• {source_id} —[{relation}]→ {target_id} {✓ | already existed}
• {source_id} —[{relation}]→ {target_id} {✓ | already existed}
```

**For a batch (N > 3):** summarize:

```
*Anansi* — {N} edges written ✓
• Relations: {unique relation types, comma-separated}
• New: {count inserted} | Already existed: {count not inserted}
```

**If any edges failed:** append a failures section:

```
⚠ {F} edge(s) failed:
• {source_id} —[{relation}]→ {target_id}: {error message}
```

---

## Error handling

| Situation | Action |
|---|---|
| Duplicate edge (already exists) | Treat as success — idempotent. Report "already existed" but not as error. |
| Source or target UUID not found | Report and skip that edge. Do not abort batch. |
| Unknown/unusual relation type | Write anyway — relation strings are free-form. |
| `anansi_relate` not available | Tell the caller. Do not attempt workarounds. |
| Missing required field in batch item | Skip that item, report the error, continue with remaining items. |
| All edges in a batch fail | Report all failures. Do not report success. |

---

## Constraints

- **Does not validate that source/target nodes exist** before writing.
  Callers are responsible for resolving UUIDs first (via `anansi_get`).
- **Batch mode is preferred** for callers writing 2+ edges per operation.
- **Insert-if-not-exists semantics** — never surface duplicate-edge errors
  to the user.
- **Always include `"source": "skill"`** in every `anansi_relate` call.
- **No user confirmation required.** Edge creation is non-destructive and
  safe for programmatic callers.
