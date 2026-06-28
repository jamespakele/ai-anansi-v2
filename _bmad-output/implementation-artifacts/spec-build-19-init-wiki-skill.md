---
title: 'Anansi v2 — Build 19: anansi-init-wiki local-install skill'
type: 'feature'
created: '2026-06-24'
status: 'done'
baseline_commit: '8a576e0'
context:
  - _bmad-output/implementation-artifacts/spec-build-18-export-wiki-tool.md
  - _bmad-output/implementation-artifacts/spec-build-17-skills-wiki-first.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** With anansi on a VPS, the only way to get a local, Obsidian-readable `~/llm-wiki` is for a client-side skill to pull it down. Build-18 added the server tool (`anansi_export_wiki` → a full-wiki zip + download URL); nothing yet drives it from the local machine.

**Approach:** Add a local Claude skill `anansi-init-wiki` that: calls `anansi_export_wiki` to get a `download_url`, downloads the zip (authenticated), and unpacks it into the local `~/llm-wiki` (the shell resolves `~` per-OS). Two modes: **refresh** (default — create the folder, unzip over it) and **rebuild** (wipe local wiki files first, then unzip — drops files for notes deleted server-side). This is the user-facing "set up / refresh my local wiki" action. Then point `references/wiki-first.md` and `anansi-help` at it.

## Boundaries & Constraints

**Always:**
- Write ONLY to the local wiki folder (`~/llm-wiki`, resolved by the shell); never to the Anansi database or the server. Read-only against Anansi (one MCP tool call + an authenticated HTTP download).
- Resolve the home dir via the local shell (`~`/`$HOME`), so it works on Linux (`/home/...`), macOS (`/Users/...`); document the Windows path (`%USERPROFILE%`).
- Authenticate the download with the same key as the anansi MCP server (`ANANSI_API_KEY` env, or `?api_key=`/`Authorization: Bearer`), since `/exports/` is auth-protected.
- **rebuild** mode removes only the wiki's own files first (`*.md` + `.lint-state`) inside `~/llm-wiki`, never the folder itself or unrelated files — then unzips. **refresh** mode just unzips over the existing folder (idempotent).
- Verify and report: number of files written, the wiki path, and that it's ready to open in Obsidian.
- Clean up the temp zip after unpacking.

**Ask First:**
- If `~/llm-wiki` already contains non-wiki files in rebuild mode (proposed: only ever delete `*.md`/`.lint-state`, so this can't clobber user files — no prompt needed).

**Never:**
- Do not modify the Anansi vault/DB, call any write/ingest/mutate MCP tool, or run the pipeline.
- Do not delete the `~/llm-wiki` directory itself or anything outside it; in rebuild, only remove `*.md`/`.lint-state`.
- Do not hardcode a single OS home path — resolve `~` at runtime.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| First install | no `~/llm-wiki` | folder created, full wiki unzipped, report written | N/A |
| Refresh (default) | `~/llm-wiki` exists | re-download, unzip over it (files updated) | N/A |
| Rebuild | `anansi-init-wiki rebuild` | wipe `*.md`+`.lint-state`, then unzip fresh (drops server-deleted notes) | N/A |
| Export tool fails | server error | report the failure, do not touch local files | surfaced, abort |
| Download 401/404 | bad key / missing zip | report auth/URL problem with the fix (set `ANANSI_API_KEY`) | surfaced, abort |
| Windows host | `%USERPROFILE%` | document the equivalent path/command | noted in skill |

</frozen-after-approval>

## Code Map

- `llm/plugins/anansi.plugin/skills/anansi-init-wiki/SKILL.md` -- NEW skill. Frontmatter: name, trigger description ("set up / install / refresh / rebuild my local wiki", "/init-wiki", "pull the wiki down", "get the wiki locally"), `argument-hint: "[refresh | rebuild]"`. Body steps: (1) call `anansi_export_wiki` → capture `download_url`; (2) determine the wiki path (`~/llm-wiki`, matching the server's `[wiki] dir` basename) and mode (refresh default, or rebuild if the arg says so); (3) `mkdir -p ~/llm-wiki`; in rebuild, `rm -f ~/llm-wiki/*.md ~/llm-wiki/.lint-state`; (4) download the zip to a temp file with auth (`curl -fSL -H "Authorization: Bearer $ANANSI_API_KEY"` or `?api_key=`); (5) `unzip -o` into `~/llm-wiki`; (6) remove the temp zip; (7) report files-written + path + "open in Obsidian". Include the Windows/macOS path note and a read-only/safety section.
- `llm/plugins/anansi.plugin/references/wiki-first.md` -- add a short "Setting up the local wiki" note pointing to `anansi-init-wiki` (so a "folder doesn't exist" reader knows how to create it).
- `llm/plugins/anansi.plugin/skills/anansi-help/SKILL.md` -- mention `anansi-init-wiki` in the wiki-first section + the skills table.

## Tasks & Acceptance

**Execution:**
- [x] `skills/anansi-init-wiki/SKILL.md` -- author the skill (export → authenticated download → unzip to `~/llm-wiki`; refresh vs rebuild; cross-OS home; safety/read-only section).
- [x] `references/wiki-first.md` -- add the "set up the local wiki via anansi-init-wiki" pointer.
- [x] `skills/anansi-help/SKILL.md` -- document `anansi-init-wiki` (wiki-first section + skills table).

**Acceptance Criteria:**
- Given anansi is reachable and the API key is set, when `anansi-init-wiki` runs, then it calls `anansi_export_wiki`, downloads the zip, and `~/llm-wiki` contains `index.md` + the note files; the skill reports the count and path.
- Given `anansi-init-wiki rebuild`, when run, then local `*.md`/`.lint-state` are removed before unzip (so notes deleted server-side disappear locally), and nothing outside `~/llm-wiki` is touched.
- Given the API key is missing/invalid, when the download is attempted, then the skill reports the auth problem and the fix, and makes no partial local changes beyond the temp file (which it cleans up).
- Given the skill completes, when inspected, then it made no write/ingest/mutate MCP calls (it is read + local-write only) and the temp zip is removed.

## Spec Change Log

- **v1.1 (review patches, 2026-06-24):** One focused review. Verified against source: tool name/JSON keys (`download_url`/`filename` inside the tool's text content), auth (Bearer + `?api_key=` both accepted by `/exports/`), zip contents (flat `*.md`+`index.md`+`log.md`), and the rebuild delete-scope (`*.md`+`.lint-state`) all match. `curl -f`+`set -e` correctly abort before unzip on 401/404; the catastrophic `rm` path is unreachable (the `/llm-wiki` suffix + `set -e`). Patched: (F4, MED) the relative-`download_url` case (server without `public_url`) — the old "fallback" still used the relative URL; now the skill detects a leading `/` and prepends the anansi base URL; (F1, MED) added a `: "${HOME:?}"` guard before the rebuild `rm` for parity with the server's defensive `clean_wiki_dir`; (F5) noted the tool result is JSON-inside-text; (F7) flagged the block as bash (Linux/macOS) with a Windows/WSL/PowerShell note. Doc edits (wiki-first.md, anansi-help) confirmed consistent.

## Suggested Review Order

- The skill — export → authed download → unzip to ~/llm-wiki; refresh vs rebuild; safety rules.
  [`anansi-init-wiki/SKILL.md`](../../llm/plugins/anansi.plugin/skills/anansi-init-wiki/SKILL.md)
- Reference pointer to the setup skill.
  [`wiki-first.md`](../../llm/plugins/anansi.plugin/references/wiki-first.md)

## Design Notes

**Auth for the download.** `/exports/{file}` is behind the MCP auth middleware (Bearer or `?api_key=`). The skill uses `ANANSI_API_KEY` (the same key configured for the anansi MCP connection). If unset, it tells the user to export the key (and where the anansi MCP config holds it).

**Refresh vs rebuild.** The export zip is the full current graph, so unzipping over the folder (refresh) updates every note. Only **rebuild** removes local files first — needed to drop notes that were deleted on the server (a plain overwrite leaves their stale local files). Rebuild is the safe "make my local copy exactly match the server" action.

**Cross-OS home.** The skill uses the shell's `~`/`$HOME`, so the path resolves to `/home/<user>` (Linux), `/Users/<user>` (macOS). For Windows/PowerShell it documents `$env:USERPROFILE\llm-wiki`. The folder basename (`llm-wiki`) matches the server's `[wiki] dir`.

## Verification

**Manual checks:**
- Run `anansi-init-wiki` against a reachable anansi; confirm `~/llm-wiki/index.md` + note files appear and open in Obsidian (graph view connects them).
- Run `anansi-init-wiki rebuild` after deleting a note server-side; confirm the note's local file is gone afterward.
- Confirm the skill body calls only `anansi_export_wiki` (no write/ingest tools) and cleans up the temp zip.
- Confirm `references/wiki-first.md` and `anansi-help` point to the skill.
