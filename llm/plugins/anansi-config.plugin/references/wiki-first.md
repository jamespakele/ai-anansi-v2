# Wiki-First Reads

The Anansi LLM-wiki is a **local, Obsidian-readable mirror of the knowledge graph** — plain markdown files that the server projects from Postgres. For retrieval, **read the local wiki first** and fall back to the MCP tools only on a miss. This absorbs read traffic from the database and works offline.

> The wiki is a **cache, never the source of truth.** Postgres is authoritative. A wiki hit is a fast path; anything the wiki can't answer falls through to the MCP tools.

---

## Where the wiki lives

A single folder at the **parent of your PARA roots** — a sibling of `Projects/`, `Areas/`, `Resources/`, `Archives/`:

```
/home/pakele/llm-wiki/
```

This is the server's `[wiki] dir` setting. **Setup note:** the shipped default for `[wiki] dir` is `/data/llm-wiki` (the container path); for local Obsidian access set `[wiki] dir = "/home/pakele/llm-wiki"` in `anansi.toml` so the server writes here. **If the folder doesn't exist, skip the wiki silently and use the MCP tools** exactly as before — do not announce a "miss," just use the database.

> **Getting the wiki onto this machine.** When Anansi runs remotely (VPS), the wiki files aren't local until you pull them. Run the **`anansi-init-wiki`** skill — it calls `anansi_export_wiki`, downloads the full wiki, and unpacks it to `~/llm-wiki` (use `anansi-init-wiki rebuild` to make the local copy exactly match the server). After that, wiki-first reads work locally.

## Layout

```
/home/pakele/llm-wiki/
  index.md                    catalog of every resident note, grouped by entity_type
  log.md                      append-only ingest/crawl journal
  lint.md                     semantic-lint findings (contradictions, stale, gaps)
  <slug>.<entity_type>.md     one file per note
```

**Slug rule** (same as the note's `match_key`): take the note **name**, lowercase it, turn every non-alphanumeric character into a space, collapse runs of whitespace, and join with hyphens. The file is then `<slug>.<entity_type>.md`.
- `Ian Kitajima` + `person` → `ian-kitajima.person.md`
- `Sovereign AI` + `concept` → `sovereign-ai.concept.md`

**`index.md`** lists each note as `- [[slug.ext|Name]] — lede`, grouped under `## <entity_type>` headings. Grep it by name or skim by type to locate a file.

## A note file's anatomy

```markdown
---
anansi_id: <uuid>
entity_type: person
name: Ian Kitajima
match_key: person:ian-kitajima
updated_at: 2026-06-24T...
---

# Ian Kitajima

<lede>

*<why>*

<content>

## Connections

- [[pichtr.organization|PICHTR]] `works_at` — <why>
```

Reading one file gives you everything `anansi_get` returns **plus** the note's one-hop connections.

---

## Decision flow

1. **Locate the wiki.** No `/home/pakele/llm-wiki` folder → use the MCP tools, done.
2. **Exact / named entity** ("who is X", known `match_key`) → read `<slug>.<entity_type>.md` directly (compute the slug), or find it via `index.md`. **Hit → answer from the file, no DB call.**
3. **Keyword search** ("notes about LNG") → `grep` across `/home/pakele/llm-wiki/*.md`. Use the matches; optionally confirm/expand with `anansi_search`.
4. **Semantic similarity** ("concepts like X", thematic questions) → no wiki equivalent → `anansi_search_semantic`.
5. **Graph traversal** → the note's own `## Connections` covers one hop; for deeper neighborhoods use `anansi_edges`.
6. **Any miss, or you need guaranteed-current state** → fall back to the MCP read tools. **The wiki never blocks an answer.**

---

## Caveats (when to prefer the database)

- **Staleness:** the wiki lags Postgres by at most one crawl interval, so a very recently captured/updated note may not be reflected yet. For "what's the latest on X," prefer `anansi_get`.
- **Eviction:** the wiki is size-bounded — cold (rarely-read) notes may have no file even though they exist in the DB. A missing file ≠ a missing note. Always fall back.
- **Read-only:** never write to or modify wiki files from a skill. The server owns them; edits are lost on the next crawl.
- **Mutations need canonical state:** any operation that will *change* a note (delete, archive, update, relate, purge) must resolve its target via the MCP tools / Postgres — **never** wiki-first — so it acts on the authoritative `note_id` and current state.
