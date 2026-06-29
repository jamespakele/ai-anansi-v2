---
name: web-tender-audit
description: >
  Reads the Web Tender queue and produces a human-readable markdown report of
  all items still needing attention. Groups by category, sorts by severity then age.
  Triggers: "web-tender-audit", "/web-tender-audit", "tender audit",
  "show tender queue", "what needs attention", "audit the web".
argument-hint: "[--resolve | --dismiss]"
---

# web-tender-audit

Reads the Web Tender queue and produces a human-readable markdown report of all
items still needing attention. Groups by category, sorts by severity then age
(oldest first).

---

## Report Format

The report has two sections:

### 1. Summary Table

A high-level overview of open items by category:

| Category            | 🔴 High | 🟡 Medium | 🟢 Low | ℹ️ Info | Total |
|---------------------|---------|-----------|--------|---------|-------|
| dedup               | 0       | 3         | 1      | 0       | 4     |
| broken_edge         | 2       | 0         | 0      | 0       | 2     |
| ...                 | ...     | ...       | ...    | ...     | ...   |
| **Total**           | **N**   | **N**     | **N**  | **N**   | **N** |

### 2. Per-Category Detail

For each category with open items, a section with:

```
### 🔴 dedup — 4 items

#### 🟡 Medium — 3 items

1. **ID:** <uuid>
   **Match Key:** `entity-match-key`
   **Related:** `other-key`, `third-key`
   **Confidence:** 0.94
   **Description:** <description text>
   **Created:** 2026-06-27T14:30:00Z
   **Age:** 1 day

2. ...

#### 🟢 Low — 1 item

...
```

**Severity icons:**

| Severity | Icon | Sort Order |
|----------|------|------------|
| high     | 🔴   | 1 (first)  |
| medium   | 🟡   | 2          |
| low      | 🟢   | 3          |
| info     | ℹ️   | 4 (last)   |

Within each severity level, items are sorted by `created_at` ascending (oldest
first) so the most stale items appear at the top.

---

## Usage

### Default (dry-run)

Reads the queue and prints the report. Does not modify any items:

```
/web-tender-audit
```

Equivalent to:

```
/web-tender-audit --dry-run
```

### `--resolve`

After printing the report, marks all currently-open items as `resolved`:

```
/web-tender-audit --resolve
```

Sets `status = 'resolved'`, `resolved_at = NOW()`, and
`resolved_by = 'web-tender-audit'` on every item that was `open` at the time
of the report.

**Confirmation prompt** before applying:

```
About to resolve 12 open items. This cannot be undone.
Proceed? (yes / no)
```

### `--dismiss`

After printing the report, marks all currently-open items as `dismissed`:

```
/web-tender-audit --dismiss
```

Sets `status = 'dismissed'`, `resolved_at = NOW()`, and
`resolved_by = 'web-tender-audit'` on every item that was `open` at the time
of the report.

**Confirmation prompt** before applying:

```
About to dismiss 12 open items. This cannot be undone.
Proceed? (yes / no)
```

---

## SQL Queries

### Summary query

```sql
SELECT
  category,
  severity,
  COUNT(*) AS count
FROM tender_queue
WHERE status = 'open'
GROUP BY category, severity
ORDER BY category, severity;
```

### Detail query (per category)

```sql
SELECT
  id,
  category,
  severity,
  match_key,
  related_keys,
  confidence,
  description,
  created_at
FROM tender_queue
WHERE status = 'open'
ORDER BY category, severity DESC, created_at ASC;
```

### Resolve all open

```sql
UPDATE tender_queue
SET
  status = 'resolved',
  resolved_at = NOW(),
  resolved_by = 'web-tender-audit'
WHERE status = 'open';
```

### Dismiss all open

```sql
UPDATE tender_queue
SET
  status = 'dismissed',
  resolved_at = NOW(),
  resolved_by = 'web-tender-audit'
WHERE status = 'open';
```

---

## Hard Rules

1. **Dry-run by default** — Never modifies the database unless `--resolve` or
   `--dismiss` is explicitly passed.
2. **Confirm before write** — Always prompt for confirmation before applying
   `--resolve` or `--dismiss`. Show the count of items that will be affected.
3. **Read-only queries** — All report queries use `SELECT` only. No
   transactional side effects.
4. **Idempotent report** — Running the report multiple times on the same data
   produces identical output.
5. **Graceful empty state** — If the queue has no open items, print:

   ```
   ✅ Web Tender queue is empty — no items need attention.
   ```
