---
id: normalization-rules
type: rule
rule: normalization
status: active
version: "1.0"
last_updated: 2026-06-28
---

# Normalization Rules

Rules for normalizing entity names before storage, matching, and deduplication.
Applied at ingest time and during Web Tender scans.

## Name Casing

| Entity Type     | Rule                                                                 |
|-----------------|----------------------------------------------------------------------|
| Person          | **Title Case** — each significant word capitalised (e.g. "John Smith") |
| Organization    | **Title Case** — as the org styles itself when known (e.g. "Google LLC") |
| Project         | **Title Case** — proper project name (e.g. "Project Chimera")         |
| Area            | **Title Case** — e.g. "Personal Finance"                              |
| Concept         | **Lowercase** — generic terms only (e.g. "machine learning")          |
| Topic           | **Lowercase** — generic terms only (e.g. "distributed systems")       |
| Event           | **Title Case** — proper event name (e.g. "AWS re:Invent 2026")        |
| Context         | **Lowercase** — descriptive label (e.g. "weekly sync notes")          |
| Note            | **Title Case** — first word capitalised, rest as written              |
| Task            | **Title Case** — first word capitalised, rest as written             |
| ActionItemList  | **Title Case** — first word capitalised, rest as written             |
| Outline         | **Title Case** — first word capitalised, rest as written             |

**Exception:** Acronyms and initialisms retain their original casing (e.g. "OKF", "API", "NASA").

## Punctuation

- Strip **trailing punctuation** from all names: periods, commas, colons, semicolons, exclamation marks, question marks.
- Preserve internal punctuation that is part of the name (e.g. "O'Brien", "re:Invent", "C++").
- Strip leading and trailing whitespace after punctuation removal.

## Slug Format

Every entity gets a `match_key` slug derived from its `normalized_name`:

1. Convert to **lowercase**.
2. Replace runs of whitespace and non-alphanumeric characters with a single **hyphen** (`-`).
3. Strip leading and trailing hyphens.
4. Truncate to **120 characters** maximum.

**Examples:**

| Raw Name                     | Normalized Name          | Slug (match_key)              |
|-----------------------------|--------------------------|-------------------------------|
| "John Smith!"               | "John Smith"             | `john-smith`                  |
| "Google's OKF"              | "Google's OKF"           | `googles-okf`                 |
| "Machine Learning 101:"     | "machine learning 101"   | `machine-learning-101`        |
| "C++ Best Practices?"       | "C++ Best Practices"     | `c-best-practices`            |
| "  Extra Spaces  "          | "Extra Spaces"           | `extra-spaces`                |

## Edge Cases

- **Empty or blank names** after normalization → reject the entity, do not store.
- **Names that normalise to the same slug** as an existing entity → flag for deduplication review (see `deduplication.md`).
- **Single-character names** after normalization → allowed only for known initials (e.g. "E. Musk" → "E Musk" → `e-musk`).
