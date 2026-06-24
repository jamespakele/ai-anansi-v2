---
title: 'Anansi v2 — Build 18: anansi_export_wiki (full wiki bundle)'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: '3a384f7'
context:
  - _bmad-output/implementation-artifacts/spec-build-12-llm-wiki-notestore.md
  - _bmad-output/implementation-artifacts/spec-build-15-tipping-point-eviction.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Anansi typically runs on a VPS, so the server-side wiki (`[wiki] dir`) and the `rebuild-wiki` CLI live on the remote host — useless for a *local*, Obsidian-readable wiki on the user's own machine. There's no way for a local client (a Claude skill) to pull the whole wiki down. The existing `anansi_export_vault` only does BFS from one note, not a full dump.

**Approach:** Add an `anansi_export_wiki` MCP tool that projects **all** live notes from Postgres into the full Build-12 wiki layout (note files + `index.md`), packages it as a zip, and returns a `download_url` via the existing `/exports/` machinery. Implementation reuses the canonical `WikiStore` renderer: build into a fresh temp dir with a force-enabled, **uncapped** `WikiStore` (the local copy should be the complete graph, not the server's size-bounded resident set), zip it, clean up. A local skill (Build-19) downloads and unpacks this to `~/llm-wiki`.

## Boundaries & Constraints

**Always:**
- Read-only against Postgres — the export never mutates notes/edges and never touches the server's live `[wiki] dir`.
- Build into a throwaway temp dir under the exports folder, using a `WikiStore` with `enabled = true` and `max_bytes = 0, max_notes = 0` (uncapped → every live note projected). The local download is the FULL graph.
- Reuse the existing `WikiStore` projection (note-file + `index.md` rendering) verbatim — no second serializer — so the downloaded wiki is byte-identical to the server's format.
- Return a `download_url` exactly like `anansi_export_vault` (honor `server.public_url`; fall back to a relative `/exports/...`). The zip is served by the existing auth-protected `/exports/{filename}` route.
- Clean up the temp build dir after zipping (best-effort; a leftover temp dir is non-fatal).

**Ask First:**
- Whether to also offer a bounded export matching the server caps (proposed: NO for now — full dump is the point of a local install; a `bounded` flag can come later).

**Never:**
- Do not zip or expose the server's live `[wiki] dir` (could be mid-write by the crawl); always build a fresh temp copy.
- Do not mutate Postgres or the server wiki.
- Do not add the lint phase or require an LLM — this is a pure projection.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Export, notes present | `anansi_export_wiki` | temp wiki built (all live notes + index.md), zipped, `download_url` returned | N/A |
| Empty graph | no live notes | a valid (near-empty: just `index.md`/`log.md`) zip + url | N/A |
| `server.public_url` set | configured | absolute `https://.../exports/<zip>` | falls back to relative if unset |
| Temp build fails mid-projection | per-note error | crawl logs + continues; whatever projected is zipped | propagated only on hard failure |
| Download | GET `/exports/<zip>` with API key | the zip streams (existing route) | 404 if missing (existing) |
| read_only server | `server.read_only = true` | export still works (it's a read) | N/A |

</frozen-after-approval>

## Code Map

- `src/export.rs` -- add `pub async fn export_wiki(pool: &DbPool, exports_dir: &Path) -> Result<String>`: make a unique temp dir under `exports_dir` (e.g. `wiki-build-<uuid>/`); construct `WikiStore { root: temp, enabled: true, max_bytes: 0, max_notes: 0 }`; `wiki.crawl(pool).await` to project all live notes + `index.md` (+ `log.md`) into the temp dir; zip every file in the temp dir into `exports_dir/wiki-<uuid>.zip` (reuse the `zip` crate already used by `export_vault`); remove the temp dir; return the zip filename. Add a small `zip_dir(src_dir, zip_path)` helper (or inline).
- `src/mcp.rs` -- add `anansi_export_wiki` dispatch + `tool_wiki_export` handler modeled on `tool_export_vault` (no args; `exports_dir = anansi_root/exports`; call `export::export_wiki`; return `{status, filename, download_url}` honoring `public_url`). Register in `tools/list`.
- `src/wiki.rs` -- if `WikiStore`'s fields aren't all `pub` already, no change needed (they are: `root`, `enabled`, `max_bytes`, `max_notes`); `crawl` is `pub`. (No code change expected here — verify.)

## Tasks & Acceptance

**Execution:**
- [x] `src/export.rs` -- implement `export_wiki` (temp dir → force-enabled uncapped `WikiStore::crawl` → zip the dir → cleanup → return zip name) + a `zip_dir` helper.
- [x] `src/mcp.rs` -- add `tool_wiki_export` + `anansi_export_wiki` dispatch + tools/list entry, returning a `download_url` like `anansi_export_vault`.

**Acceptance Criteria:**
- Given `cargo check` / `cargo test --no-run`, when run, then both finish with zero errors.
- Given a populated Postgres, when `anansi_export_wiki` is called, then it returns a `download_url`, and the zip contains one `{slug}.{entity_type}.md` per live note plus `index.md`, in the same format the server's `[wiki] dir` holds.
- Given size caps are configured on the server, when `anansi_export_wiki` runs, then the export still contains ALL live notes (uncapped — it does not honor `max_mb`/`max_notes`).
- Given the export completes, when it returns, then the temp build dir is removed and only the `.zip` remains under `exports/`.
- Given `server.public_url` is set, when the tool returns, then `download_url` is absolute.

## Spec Change Log

- **v1.1 (review, 2026-06-24):** One focused adversarial review. Reviewer verified soundness end-to-end: uncapped force-enabled `crawl` yields ALL live notes + `index.md` + `log.md` (the `over` cap-check is inert at 0/0; `write_index` is gated by `!project_failed`, not caps, so the full set is indexed); temp `wiki-build-<uuid>` dir cleaned on both success and error paths; no collision (distinct uuids), no traversal (the `/exports/` route rejects `..`/separators and joins one filename), no interference with the server's real wiki dir (different root, read-only on PG); `download_url`/`public_url` handling mirrors `tool_export_vault`; correctly no `read_only` guard (it's a read). **No patches.** Left as-is (non-blocking nits): `&PathBuf` signature matches the existing `export_vault` for consistency; a panic mid-build would leak one inert temp dir, but there are no panics on the path.

## Suggested Review Order

- Entry — the full-wiki export: temp dir → uncapped crawl → zip → cleanup.
  [`export.rs:260`](../../src/export.rs#L260)
- Flat zip of the built wiki files.
  [`export.rs:293`](../../src/export.rs#L293)
- MCP tool — returns the download_url (mirrors anansi_export_vault).
  [`mcp.rs:1950`](../../src/mcp.rs#L1950)

## Design Notes

**Why build a temp copy instead of zipping `[wiki] dir`.** The live `[wiki] dir` may be (a) absent/disabled, (b) the bounded resident set (size-evicted), or (c) mid-write by the background crawl. Building a fresh, uncapped temp projection guarantees a complete, consistent, self-contained snapshot regardless of server wiki config — and reuses the exact `WikiStore` renderer so the local files match.

**Reuse, not reimplement.** `export_wiki` constructs a `WikiStore` and calls `crawl` — the same code path Builds 12–15 use — so note-file frontmatter, `## Connections` wikilinks, and `index.md` are identical to the server wiki and to what `references/wiki-first.md` documents. No divergent serializer to keep in sync.

## Verification

**Commands:**
- `cargo check` / `cargo test --no-run` -- expected: `Finished`, zero errors.

**Manual checks:**
- Call `anansi_export_wiki` against a populated DB, `curl` the `download_url` (with the API key), unzip, and confirm: `index.md` lists all notes grouped by type, and a sampled `{slug}.{entity_type}.md` has the expected frontmatter + `## Connections`. Confirm `exports/` holds only the `.zip` (no leftover `wiki-build-*` dir).
