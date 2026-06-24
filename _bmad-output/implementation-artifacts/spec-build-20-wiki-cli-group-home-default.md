---
title: 'Anansi v2 — Build 20: wiki CLI group + ~ resolution + ~/llm-wiki default'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: 'a8609e7'
context:
  - _bmad-output/implementation-artifacts/spec-build-16-rebuild-wiki-command.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Two loose ends from the wiki work: (1) `[wiki] dir` must be an absolute path — `~/llm-wiki` doesn't expand, so a cross-platform "home" convention can't be written in config; and (2) the Build-16 `rebuild-wiki` command only models recovery, while the agreed shape is a `wiki` group with `init` (first-time/refresh) and `rebuild` (wipe+regenerate).

**Approach:** (a) Expand a leading `~`/`~/` in `[wiki] dir` to the OS home at config load (`$HOME` on Linux/macOS, `%USERPROFILE%` on Windows), and change the default from `/data/llm-wiki` to `~/llm-wiki` so a local-first install "just works" cross-OS (containers override to `/data/llm-wiki`). (b) Replace the `rebuild-wiki` subcommand with a `wiki` group: `anansi2 wiki init` (create + populate from Postgres, idempotent) and `anansi2 wiki rebuild` (wipe + regenerate) — both reusing the existing `cmd_rebuild_wiki(root, clean)` (init = `clean:false`, rebuild = `clean:true`). This is for the case where anansi runs locally; the skill path (Builds 18–19) covers remote.

## Boundaries & Constraints

**Always:**
- `~` expansion happens once, at `Config::load`, so `config.wiki.dir` is absolute everywhere downstream (`WikiStore::from_config`, the crawl, the CLI). Only a leading `~` or `~/` is expanded; `~user` is NOT supported (left as-is).
- Home resolution is dependency-free: `$HOME`, falling back to `%USERPROFILE%` (Windows). If neither is set, leave the path unchanged (don't fabricate one) so the misconfig is visible rather than silently writing to a `~`-named dir.
- The default `[wiki] dir` becomes `~/llm-wiki` (then expanded). Backward note: the wiki is still default-DISABLED (`enabled=false`), so this changes nothing until a user turns the wiki on; no running deployment depends on the old `/data/llm-wiki` default (the epic is unreleased on `feat/llm-wiki`).
- `wiki init` and `wiki rebuild` reuse `cmd_rebuild_wiki` verbatim — same force-enable, same caps/dir, same safe narrow `clean_wiki_dir`. Only the CLI surface changes.

**Ask First:**
- Whether to keep a `rebuild-wiki` alias for one release (proposed: NO — the epic is unreleased; clean rename to `wiki init`/`wiki rebuild`).

**Never:**
- Do not expand `~` inside arbitrary config strings — ONLY `[wiki] dir` (scope the change).
- Do not change the wiki write/projection/eviction logic — this is config + CLI surface only.
- Do not break the containerized path: `/data/llm-wiki` must still be settable and documented for Docker.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| `dir = "~/llm-wiki"` (Linux) | `$HOME=/home/pakele` | resolves to `/home/pakele/llm-wiki` | N/A |
| `dir = "~/llm-wiki"` (macOS) | `$HOME=/Users/james` | `/Users/james/llm-wiki` | N/A |
| `dir = "~/llm-wiki"` (Windows) | `%USERPROFILE%=C:\Users\James` | `C:\Users\James\llm-wiki` | N/A |
| absolute dir | `dir = "/data/llm-wiki"` | unchanged (no `~`) | N/A |
| no HOME/USERPROFILE | env unset | path left literal (`~/llm-wiki`); visible in logs | not fabricated |
| `anansi2 wiki init` | local anansi | create + populate (idempotent) | as Build-16 |
| `anansi2 wiki rebuild` | corruption | wipe `*.md`/`.lint-state` + regenerate | as Build-16 |
| `anansi2 wiki --help` | — | shows `init` and `rebuild` subcommands | N/A |

</frozen-after-approval>

## Code Map

- `src/config.rs` -- add `fn expand_home(path: &str) -> String` (expand a leading `~`/`~/` via `env::var_os("HOME")` then `"USERPROFILE"`; else return unchanged); change `default_wiki_dir()` to return `"~/llm-wiki"`; in `Config::load`, after the env overrides, set `config.wiki.dir = expand_home(&config.wiki.dir);`.
- `src/main.rs` -- replace `Command::RebuildWiki { clean }` with `Command::Wiki { #[command(subcommand)] action: WikiAction }`; add `enum WikiAction { Init, Rebuild }`; dispatch `Init → cmd_rebuild_wiki(root, false)`, `Rebuild → cmd_rebuild_wiki(root, true)`. Keep `cmd_rebuild_wiki`/`clean_wiki_dir` as-is (optionally rename `cmd_rebuild_wiki` → `cmd_wiki` for clarity).
- `anansi.toml.example` -- change the `[wiki] dir` example to `~/llm-wiki`, document cross-OS resolution and the container override (`/data/llm-wiki`).

## Tasks & Acceptance

**Execution:**
- [x] `src/config.rs` -- add `expand_home`; default `~/llm-wiki`; apply expansion in `Config::load`.
- [x] `src/main.rs` -- `wiki` subcommand group (`init`/`rebuild`) replacing `rebuild-wiki`, reusing `cmd_rebuild_wiki(root, clean)`.
- [x] `anansi.toml.example` -- update `dir` example + cross-OS / container docs.

**Acceptance Criteria:**
- Given `cargo check` / `cargo test --no-run`, when run, then both finish with zero errors.
- Given `dir = "~/llm-wiki"` and `$HOME=/home/pakele`, when config loads, then `config.wiki.dir == "/home/pakele/llm-wiki"`.
- Given an absolute `dir`, when config loads, then it is unchanged.
- Given no `[wiki]` config and `$HOME` set, when config loads, then `wiki.dir` defaults to `<home>/llm-wiki`.
- Given `anansi2 wiki --help`, when run, then it lists `init` and `rebuild`; `wiki init` populates idempotently and `wiki rebuild` wipes+regenerates (same behavior as the Build-16 command with/without `--clean`).

## Spec Change Log

- **v1.1 (build verification, 2026-06-24):** Implemented as specced. `expand_home_with` unit-tested (tilde→home, `~` alone, absolute untouched, non-tilde relative untouched, `~user` left literal, no-home→literal). `cargo check` clean; `cargo test config` 2/2; `anansi2 wiki --help` lists `init`/`rebuild`; full test crate compiles. Also updated the `wiki.rs` module-doc example from `/data/llm-wiki` to `~/llm-wiki` for accuracy. Given the change is small (pure home-expansion + a CLI-surface rename reusing `cmd_rebuild_wiki`) and unit-tested, a single self-review was used in place of the multi-reviewer panel.

## Suggested Review Order

- Home expansion (pure core + env wrapper) + default `~/llm-wiki`.
  [`config.rs`](../../src/config.rs)
- Applied once at config load.
  [`config.rs`](../../src/config.rs)
- The `wiki` subcommand group (init/rebuild) reusing cmd_rebuild_wiki.
  [`main.rs`](../../src/main.rs)

## Design Notes

**Why expand at load, not in `from_config`.** Expanding once in `Config::load` means every consumer (`WikiStore::from_config`, the crawl, `cmd_wiki`, the export tool) sees an absolute path — no scattered `~`-handling. The skill path (Build-19) resolves `~` in the local shell instead, because it writes on a *different* machine than the server.

**`~user` intentionally unsupported.** Only a bare `~` / `~/` prefix is expanded (the common case). `~otheruser` is rare and POSIX-shell-specific; left literal.

**Unit test.** `expand_home` is pure given an injected home — test: `~/llm-wiki` + home → joined; absolute → unchanged; no-`~` relative → unchanged. (Use a small inner fn taking the home explicitly so the test doesn't depend on the process env.)

## Verification

**Commands:**
- `cargo check` / `cargo test --no-run` -- expected: `Finished`, zero errors.
- `cargo test config` -- expected: `expand_home` unit test passes.
- `anansi2 wiki --help` / `anansi2 wiki init --help` -- expected: subcommands shown.
