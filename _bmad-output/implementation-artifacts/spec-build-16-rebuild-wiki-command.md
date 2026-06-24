---
title: 'Anansi v2 — Build 16: rebuild-wiki CLI Command'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: '8418267'
context:
  - _bmad-output/implementation-artifacts/spec-build-13-anansi-crawl-core.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The LLM-wiki is a derived file projection of Postgres, but there's no single command to regenerate it from scratch. If the local wiki gets corrupted, the container/host is moved, or the wiki dir is wiped, the only way to repopulate it is to wait for the background crawl (requires the server running with `crawl_enabled`) or to re-capture. Disaster recovery should be one command.

**Approach:** Add a `rebuild-wiki` CLI subcommand that reconstructs the entire wiki from canonical Postgres state in one shot, without the server. It loads config, connects to Postgres, builds a **force-enabled** `WikiStore` (works even if `[wiki] enabled = false`), optionally wipes the existing wiki files (`--clean`), and runs the existing `WikiStore::crawl` — which projects the resident set, rebuilds `index.md`, and GCs anything stale. Respects the configured `[wiki] dir` (e.g. the PARA-parent `~/llm-wiki`) and the eviction caps, so it produces exactly the wiki the running server would maintain.

## Boundaries & Constraints

**Always:**
- Postgres is the source of truth and is never modified — `rebuild-wiki` only reads notes/edges and writes files.
- Force-enabled: the command runs regardless of `[wiki] enabled` (an explicit rebuild is itself the opt-in), but still respects `[wiki] dir`, `max_mb`, and `max_notes`.
- `--clean` only removes the wiki's own files — `*.md` plus `.lint-state` — inside the configured wiki dir; it never deletes the directory itself, non-`.md`/non-state files, or anything outside the wiki dir.
- Reuse `WikiStore::crawl` verbatim — no second projection path. The rebuild IS a crawl from a clean/forced start.
- Non-destructive to Postgres and idempotent: running it repeatedly converges to the same wiki.

**Ask First:**
- Whether `--clean` should also remove non-`.md` files (proposed: no — only `*.md` + `.lint-state`, to avoid nuking user files if `dir` is misconfigured).

**Never:**
- Do not delete the wiki directory itself or recurse outside it.
- Do not mutate Postgres, run the LLM lint, or start the server.
- Do not add a separate projection/serialization path — delegate to the crawl.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Rebuild, wiki dir exists | `anansi2 rebuild-wiki` | crawl regenerates resident files + index.md; stale files GC'd; report printed | per-note errors logged, continue |
| Rebuild, wiki dir missing | dir absent | dir created, full projection written | N/A |
| `--clean` | corrupted/relocated wiki | all `*.md` + `.lint-state` removed first, then fresh projection | missing dir → nothing to clean, proceed |
| `[wiki] enabled = false` | wiki disabled at runtime | rebuild still runs (force-enabled) | N/A |
| Caps configured | `max_notes`/`max_mb` set | rebuild writes the bounded resident set (same as the crawl) | N/A |
| DB unreachable | bad `database_url` | command exits non-zero with the connection error | propagated as `Err` |

</frozen-after-approval>

## Code Map

- `src/main.rs` -- add `Command::RebuildWiki { #[arg(long)] clean: bool }`; dispatch to `cmd_rebuild_wiki(&cli.root, clean)`. Implement `cmd_rebuild_wiki`: `Config::load(root)` → `db::connect_and_migrate` → `let mut wiki = WikiStore::from_config(&config); wiki.enabled = true;` → if `clean`, `clean_wiki_dir(&wiki.root)` → `wiki.crawl(&pool).await` → print the `CrawlReport`. Add `use anansi2::wiki::WikiStore;`. Add a small `clean_wiki_dir(dir)` helper that, if the dir exists, removes only entries whose filename ends in `.md` or equals `.lint-state`.

## Tasks & Acceptance

**Execution:**
- [x] `src/main.rs` -- add the `RebuildWiki { clean }` subcommand + dispatch arm; implement `cmd_rebuild_wiki` (config → pool → force-enabled `WikiStore` → optional `clean_wiki_dir` → `crawl` → print report) and the `clean_wiki_dir` helper (removes only `*.md` + `.lint-state` within the dir); import `WikiStore`.

**Acceptance Criteria:**
- Given `cargo check` / `cargo test --no-run`, when run, then both finish with zero errors.
- Given `anansi2 rebuild-wiki` with a populated Postgres and an empty/missing wiki dir, when run, then the wiki dir is populated with note files + `index.md` and the command prints a resident/evicted/removed report and exits 0.
- Given `[wiki] enabled = false`, when `rebuild-wiki` runs, then it still rebuilds (force-enabled).
- Given `--clean` with pre-existing stray `*.md` files, when run, then those are removed before the rebuild and only resident notes remain.
- Given caps are configured, when `rebuild-wiki` runs, then the wiki holds only the bounded resident set (identical to a normal crawl).
- Given an unreachable `database_url`, when run, then the command exits non-zero with the DB error (no partial wiki writes beyond what the crawl attempted).

## Spec Change Log

- **v1.1 (review patch, 2026-06-24):** One focused adversarial review (proportionate to a ~70-line additive CLI command). Reviewer found the code sound — correct crawl reuse, accurate force-enable, clean error handling, no panics, bounded (non-recursive, top-level `*.md`/`.lint-state` only) deletion. Patched the one MEDIUM hardening: `clean_wiki_dir` used `path.is_file()` (follows symlinks) → switched to `entry.file_type()` (never follows) so symlinks are skipped, only real owned files are unlinked. Kept by design: `--clean` wipes `log.md` (the journal restarts on a fresh-tree rebuild; Postgres is authoritative); no wiki-dir sanity gate (blast radius already bounded and the command is an explicit operator action).

## Suggested Review Order

- Entry — the rebuild command: config → pool → force-enabled WikiStore → crawl.
  [`main.rs:349`](../../src/main.rs#L349)
- The only destructive code — narrow, symlink-safe file removal.
  [`main.rs:378`](../../src/main.rs#L378)
- Subcommand + `--clean` flag.
  [`main.rs:105`](../../src/main.rs#L105)

## Design Notes

**Why force-enable.** `WikiStore::crawl` early-returns when `!enabled`. A rebuild is an explicit operator action, so the command sets `wiki.enabled = true` after `from_config` rather than requiring `[wiki] enabled = true` in config. Caps and `dir` are still honored.

**`--clean` scope.** The crawl already self-heals (overwrites changed files, GCs non-resident), so a plain `rebuild-wiki` fixes most corruption. `--clean` is belt-and-suspenders for a guaranteed-fresh tree; it is deliberately narrow (only `*.md` + `.lint-state`) so a misconfigured `dir` can't cause collateral deletion.

**PARA-parent location.** The wiki is meant to live beside the PARA folders (e.g. `~/llm-wiki`), local and Obsidian-viewable, to absorb DB read traffic. That's the configured `[wiki] dir`; this command writes wherever it points.

## Verification

**Commands:**
- `cargo check` / `cargo test --no-run` -- expected: `Finished`, zero errors.
- `anansi2 rebuild-wiki --help` -- expected: shows the subcommand + `--clean` flag.

**Manual checks:**
- With a populated DB and `[wiki] dir` set, run `anansi2 rebuild-wiki --clean`, then open the dir in Obsidian: note files + `index.md` present, graph view connects them; re-run and confirm it's idempotent (report shows 0 removed on the second pass).
