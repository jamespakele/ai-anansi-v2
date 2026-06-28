---
name: wiki-recall
description: >
  Read-only retrieval from the local ~/llm-wiki/ folder. Reads entity
  files directly — no MCP call, no Docker round-trip. Optionally falls
  back to the Anansi server with --remote for entities not found locally
  or for a broader search.
argument-hint: "[query or match_key] [--local | --remote]"
---

# wiki-recall

Read-only retrieval from the local Karpathy-style LLM wiki.

The agent reads markdown files from `~/llm-wiki/` directly — no MCP call,
no Docker round-trip. For entities not found locally, pass `--remote` to
fall back to `anansi_get` / `anansi_search` on the server.

---

## When to invoke

- User asks "what do I know about X" — check local wiki first
- User says "recall product:okf" — read the local file
- User says "search for OKF in the wiki" — grep local files
- User says "search the server too" — pass `--remote`

---

## Inputs

- **query** — a match_key (e.g. `product:okf`), a name (e.g. `Andrej Karpathy`),
  or a keyword (e.g. `OKF`)
- **`--local`** (default) — search local files only
- **`--remote`** — search local first, then fall back to the server if not found

---

## Step 1 — Classify the query

| Query shape | Local action | Remote fallback |
|---|---|---|
| `type:slug` (match_key) | Read `{slug}.{type}.md` directly | `anansi_get(match_key)` |
| Proper name ("Andrej Karpathy") | `grep -ri "name" ~/llm-wiki/*.md` | `anansi_search(query)` |
| Keyword ("OKF", "vector database") | `grep -ri "keyword" ~/llm-wiki/*.md` | `anansi_search(query)` |

---

## Step 2 — Local search

### Exact match_key lookup

Read the file at `{wiki_dir}/{slug}.{type}.md`. If it exists, return the
full content (frontmatter + body + connections). If not, and `--remote`
was passed, proceed to Step 3.

### Name or keyword search

Use `grep` to find matching files:

```bash
grep -ril "{query}" ~/llm-wiki/*.md
```

This returns filenames that contain the query string. Read the matching
files and present a summary (name, entity_type, lede) for each.

If no local matches and `--remote` was passed, proceed to Step 3.

---

## Step 3 — Remote fallback (only with --remote)

If the entity wasn't found locally and `--remote` was passed:

- For match_key queries: call `anansi_get(match_key)`
- For name/keyword queries: call `anansi_search(query)`

Present the results alongside a note that they came from the server.

---

## Step 4 — Report

```
*wiki-recall* — {query}
• Source: {local | remote | local + remote}
• Matches: {N}
• Files: {file paths, if local}
```
