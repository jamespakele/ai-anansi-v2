---
title: 'Anansi v2 — Build 17: Skills Read Wiki-First'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: 'bb46e07'
context:
  - _bmad-output/implementation-artifacts/spec-build-12-llm-wiki-notestore.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The `anansi.plugin` read skills always hit the MCP server (→ Postgres) for retrieval, even though the LLM-wiki is now a local, Obsidian-readable folder at the PARA-parent (`~/llm-wiki`) that mirrors the graph as plain markdown. Reading those files first would absorb most read traffic from the DB and work offline.

**Approach:** Add a **wiki-first read pattern**: a shared reference doc (`references/wiki-first.md`) defining the wiki location, file layout, and a "check the local wiki, fall back to MCP" procedure; then wire the **pure-read** skills (`anansi-recall`, `anansi-recompose`) to consult the wiki before calling the MCP read tools, and document the pattern in `anansi-help`. The wiki is a **cache, not the source of truth** — on a miss (folder absent, note evicted/not-yet-projected, or semantic/deep-graph queries) the skills fall back to the existing MCP path. **Write-path lookups are deliberately excluded** (delete/archive/update/relate/purge resolve a note in order to mutate it and must use canonical Postgres state, never a possibly-stale wiki file).

## Boundaries & Constraints

**Always:**
- The wiki is a cache: a hit short-circuits the DB read; a miss falls through to the existing MCP tools. Never treat a wiki miss as "no result" — always fall back.
- Wiki location is the PARA-parent `llm-wiki/` folder (e.g. `~/llm-wiki`, sibling of Projects/Areas/Resources/Archives), matching the server's `[wiki] dir`. If the folder doesn't exist, skip the wiki silently and use MCP.
- Note-file lookups use the Build-12 convention: `{slug}.{entity_type}.md` where slug = name lowercased, non-alphanumeric → space, collapsed, hyphen-joined; or locate via `index.md` (the catalog of `[[slug.ext|Name]] — lede` lines).
- Skills stay read-only and honor their existing hard rules. Wiki-first changes only WHERE a read is sourced, never what the skill is allowed to do.
- Queries with no wiki equivalent — semantic similarity (`anansi_search_semantic`) and graph traversal deeper than the note's own `## Connections` (`anansi_edges` > 1 hop) — go straight to MCP.

**Ask First:**
- Whether to also wire the write skills' pre-mutation lookups (proposed: NO — mutations require canonical DB state; wiki-first there risks acting on stale/evicted data).
- The exact wiki path convention if not `~/llm-wiki` (proposed: document `~/llm-wiki` as the default and note it tracks `[wiki] dir`).

**Never:**
- Do not make any skill write to the wiki, or treat the wiki as authoritative over Postgres.
- Do not use wiki-first for delete/archive/update/relate/purge target resolution (canonical-state safety).
- Do not change Rust code — this build is skill/reference markdown only.
- Do not remove the existing MCP read paths — wiki-first is layered in front of them as a fast path with fallback.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Named entity, wiki hit | `~/llm-wiki/{slug}.{type}.md` exists | read the file (frontmatter + body + Connections), answer without a DB call | N/A |
| Named entity, wiki miss | file absent / note evicted | fall back to `anansi_get` → `anansi_search` | as today |
| Keyword search | term | grep `~/llm-wiki/*.md` first; complement/fallback with `anansi_search` | fallback to MCP |
| Semantic query | "concepts like X" | go straight to `anansi_search_semantic` (no wiki equivalent) | as today |
| Deep graph (>1 hop) | "everything connected to X" | note's `## Connections` for 1 hop; `anansi_edges` for more | as today |
| Wiki folder absent | no `~/llm-wiki` | skip wiki silently, use MCP exactly as before | N/A |
| Recompose entity lookup | walking an outline | per-entity: try wiki file, fall back to `anansi_get` | fallback to MCP |

</frozen-after-approval>

## Code Map

- `llm/plugins/anansi.plugin/references/wiki-first.md` -- NEW. The shared procedure: wiki location convention (`~/llm-wiki`, tracks `[wiki] dir`), file layout (`index.md`/`log.md`/`lint.md` + `{slug}.{entity_type}.md`), slug rule, note-file anatomy (frontmatter + body + `## Connections`), the decision flow (exact/keyword → wiki-first; semantic/deep-graph → MCP; always fall back on miss), and the cache caveats (may lag the DB by up to a crawl interval; may be size-evicted; read-only; never authoritative).
- `llm/plugins/anansi.plugin/skills/anansi-recall/SKILL.md` -- insert "Step 0 — Try the local wiki first" before the current Step 1 (classification), pointing to `references/wiki-first.md`; clarify that semantic/deep-graph skip the wiki, and that a miss falls through to the existing classification+MCP steps. Keep all existing Hard Rules.
- `llm/plugins/anansi.plugin/skills/anansi-recompose/SKILL.md` -- in the per-entity lookup step, prefer the wiki file (`{slug}.{entity_type}.md`) before `anansi_get`, falling back on miss; add a one-line note that the wiki is the durable DB projection (distinct from the forbidden `output/` scaffolding), so reading it honors "database is the source."
- `llm/plugins/anansi.plugin/skills/anansi-help/SKILL.md` -- add a short "Wiki-first reads" subsection documenting the behavior + location so it's discoverable; point to `references/wiki-first.md`.

## Tasks & Acceptance

**Execution:**
- [x] `references/wiki-first.md` -- author the shared wiki-first procedure (location, layout, slug rule, note anatomy, decision flow, fallback, cache caveats).
- [x] `skills/anansi-recall/SKILL.md` -- add Step 0 wiki-first (with fallback) referencing the doc; preserve read-only Hard Rules.
- [x] `skills/anansi-recompose/SKILL.md` -- prefer the wiki file per entity, fall back to `anansi_get`; clarify wiki-vs-`output/` distinction.
- [x] `skills/anansi-help/SKILL.md` -- document the wiki-first read pattern + location, link the reference.

**Acceptance Criteria:**
- Given the wiki folder exists with a note's file, when `anansi-recall` handles a named lookup, then the skill reads the local file and answers without an MCP read call (per its instructions).
- Given the wiki folder is absent or the note isn't present, when a read skill runs, then it falls back to the existing MCP tools and behaves exactly as before.
- Given a semantic or deep-graph query, when `anansi-recall` runs, then it uses the MCP tools directly (no wiki detour).
- Given a delete/archive/update/relate/purge skill, when it resolves its target, then it still uses MCP/Postgres (wiki-first is NOT applied there).
- Given `anansi-recompose`, when it walks an outline, then per-entity it reads the wiki file first and falls back to `anansi_get` on a miss.

## Design Notes

**Wiki-first decision flow (in `wiki-first.md`):**
```text
1. Locate ~/llm-wiki (or [wiki] dir). Not present → use MCP, done.
2. Exact / named entity → read {slug}.{entity_type}.md (or find it via index.md). Hit → use it.
3. Keyword → grep ~/llm-wiki/*.md. Hits → use them (optionally confirm with anansi_search).
4. Semantic similarity / graph >1 hop → MCP (anansi_search_semantic / anansi_edges).
5. Any miss or need for guaranteed-canonical state → fall back to MCP. The wiki never blocks an answer.
```

**Why write-paths are excluded.** A delete/archive/update/relate resolves a note to obtain its canonical `note_id`/state and then mutates Postgres. A stale or size-evicted wiki file could point at the wrong/old state, so those lookups must stay on the DB. The wiki is a read accelerator only.

**Staleness contract.** The wiki lags Postgres by at most a crawl interval and may omit size-evicted (cold) notes. For pure reads that's an acceptable speed/freshness trade; the reference doc states it plainly so the model knows when to prefer the DB.

## Spec Change Log

- **v1.1 (review patch, 2026-06-24):** One focused review (markdown-only build). Reviewer verified ALL file-format claims match the Rust writer exactly — slug rule (`db::match_key`/`slug_name`), filename `<slug>.<entity_type>.md` (`expected_filename`), frontmatter (`render_note_markdown`), index format (`write_index`), `## Connections` wikilinks (`wikilink`), reserved files — and confirmed the logic/safety is sound (always falls back on miss, semantic/deep-graph → MCP, read-only Hard Rules intact, mutations-via-DB explicit, staleness/eviction caveat stated). Fixed the one real issue (HIGH): the wiki path was written as `~/llm-wiki`, which (a) wouldn't reliably expand in a Read/Grep call and (b) didn't match the shipped `[wiki] dir` default; replaced with the absolute `/home/pakele/llm-wiki` across all four files and added a setup note that the shipped default is `/data/llm-wiki` so `[wiki] dir` must be set for local access. `anansi2 rebuild-wiki` (the reviewer's secondary flag) does exist (Build-16) — no change needed.

## Suggested Review Order

- The shared procedure — location, layout, slug rule, decision flow, caveats.
  [`references/wiki-first.md`](../../llm/plugins/anansi.plugin/references/wiki-first.md)
- Recall — Step 0 wiki-first with explicit MCP fallback.
  [`anansi-recall/SKILL.md`](../../llm/plugins/anansi.plugin/skills/anansi-recall/SKILL.md)
- Recompose — per-entity wiki-first lookup.
  [`anansi-recompose/SKILL.md`](../../llm/plugins/anansi.plugin/skills/anansi-recompose/SKILL.md)
- Help — discoverable documentation of the pattern.
  [`anansi-help/SKILL.md`](../../llm/plugins/anansi.plugin/skills/anansi-help/SKILL.md)

## Verification

**Manual checks:**
- Read `references/wiki-first.md` end-to-end: the location, slug rule, and note anatomy match Build-12's writer (`{slug}.{entity_type}.md`; frontmatter `anansi_id`/`entity_type`/`name`/`match_key`/`updated_at`; `## Connections` with `[[slug.ext|name]]`).
- Confirm `anansi-recall` Step 0 has an explicit fallback to the existing steps, and the Hard Rules (read-only) are intact.
- Confirm no write skill (`anansi-delete`/`-archive`/`-update`/`-relate`/`-purge`) was modified.
- Confirm no `.rs` files changed (`git diff --stat` shows only `llm/plugins/anansi.plugin/**`).
