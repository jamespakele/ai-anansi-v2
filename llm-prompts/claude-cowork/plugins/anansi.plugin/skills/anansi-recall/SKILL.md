---
name: anansi-recall
description: >
  Read-only retrieval from the Anansi knowledge vault. Routes queries to the
  right tool: named entity or match_key → anansi_get; concept or keyword
  search → anansi_search + anansi_search_semantic; graph traversal → anansi_edges
  or anansi_export_context; type/date listing → anansi_filter. Never writes.
  Use when James says "what do I know about X", "pull context on Y", "who is Z",
  "find notes about", "search anansi for", "what's in the vault about",
  "show me everything on", "recall X", "look up X in anansi", "/recall",
  "anansi search", "pull up", or asks about a person, org, event, or topic
  by name and wants vault context rather than a fresh capture.
---

# anansi-recall

Pure read path into the Anansi vault. No writes, no captures, no pipeline.

---

## Query Classification

Classify the input before calling any tool. One pass, one decision.

| Query type | Signal | Tool(s) |
|---|---|---|
| **Exact entity lookup** | Known match_key (e.g. `person:tiago-forte`) or UUID | `anansi_get` |
| **Named entity, no key** | "Who is [name]", "tell me about [org]" | `anansi_get` with inferred match_key, fall back to `anansi_search` |
| **Concept / keyword search** | "What do I know about LNG", "notes on ConCon" | `anansi_search` + `anansi_search_semantic` in parallel |
| **Graph traversal** | "Show me everything connected to X", "what's related to Y" | `anansi_edges` (hops: 2) |
| **LLM context pull** | "Give me context on X for a draft / meeting" | `anansi_export_context` |
| **Type listing** | "List all people", "show orgs", "events this month" | `anansi_filter` |

When ambiguous between keyword and semantic, run both and merge — deduplicate by note UUID.

---

## Step 1 — Classify

Read the query. Pick the tool(s) from the table above. Announce the choice in one line before calling:

> "Searching vault for 'LNG consulting' — running keyword + semantic search."

> "Looking up person:tiago-forte — calling anansi_get."

---

## Step 2 — Call the tool(s)

**`anansi_get`** — use when you have or can infer a match_key.

Match_key format: `{entity_type}:{slug}` — e.g. `person:james-pakele`, `org:iq360`, `project:concon`. Slug is lowercase, hyphens, no special chars.

If `anansi_get` returns nothing, fall through to `anansi_search`.

**`anansi_search`** — full-text across name, lede, why, content. Good for keywords, names, acronyms.

**`anansi_search_semantic`** — vector similarity. Good for concepts, themes, questions. Run alongside `anansi_search` for concept queries — they complement each other.

**`anansi_edges`** — BFS from a known note UUID. Use when the user wants the neighborhood around an entity, not just the entity itself. Default hops: 2.

**`anansi_export_context`** — BFS traversal flattened to markdown optimized for LLM injection. Use when the user needs context for a draft, meeting prep, or wants to paste into a prompt. Returns a download URL or inline markdown.

**`anansi_filter`** — list notes by `entity_type` (e.g. `person`, `org`, `project`, `event`) and optional date range. Use for "show me all X" queries.

---

## Step 3 — Present results

Format depends on result count and type.

**Single note** — render inline: name, lede, why, key content fields. Include match_key and source for traceability.

**Multiple notes (≤10)** — render as a numbered list: name · lede · match_key. Offer to expand any entry.

**Many results (>10)** — surface the top 5 by relevance score, note total count, offer to filter.

**Graph / edges result** — render as a two-level outline: the anchor note, then its connected notes grouped by edge type.

**Export context** — hand the markdown directly to the user (or to whatever called this skill).

**No results** — say so clearly. Suggest: did you mean [closest match]? Or offer to capture the entity via `anansi:anansi-atom` if it should exist but doesn't.

---

## Hard Rules

- Never call `anansi_capture`, `anansi_ingest_*`, `anansi_relate`, or `anansi_purge`. This skill is read-only.
- Never fabricate vault content. If a note doesn't exist, say so.
- Never call `anansi_embed` — that's a maintenance operation, not retrieval.
