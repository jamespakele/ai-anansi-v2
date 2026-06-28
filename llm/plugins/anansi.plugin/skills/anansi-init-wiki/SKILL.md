---
name: anansi-init-wiki
description: >
  Sets up or refreshes the LOCAL LLM-wiki — a folder of Obsidian-readable
  markdown at ~/llm-wiki that mirrors the Anansi knowledge graph. Use this
  to get the wiki onto THIS machine when Anansi runs remotely (e.g. on a
  VPS): it calls the anansi_export_wiki MCP tool, downloads the full wiki
  zip, and unpacks it into ~/llm-wiki. Two modes: refresh (default — update
  in place) and rebuild (wipe local wiki files first, then unpack a clean
  copy that matches the server exactly). Read-only against Anansi; writes
  only to the local wiki folder. Triggers: "init wiki", "/init-wiki",
  "set up the local wiki", "install the wiki locally", "pull the wiki down",
  "get the wiki on this machine", "refresh my local wiki", "rebuild the
  local wiki", "sync the wiki locally", "download the anansi wiki".
argument-hint: "[refresh | rebuild]"
---

# anansi-init-wiki

Get the Anansi LLM-wiki onto **this** machine as a local, Obsidian-readable folder.

Anansi (server + Postgres) usually runs remotely; its server-side wiki and CLI are on that host. This skill pulls the **full** wiki to your local `~/llm-wiki` so you can browse it in Obsidian and so the read skills (`anansi-recall`, `anansi-recompose`) can read files locally instead of hitting the database. See `references/wiki-first.md`.

> **Read + local-write only.** This skill makes ONE read-side MCP call (`anansi_export_wiki`), downloads a zip, and writes to `~/llm-wiki`. It never modifies the Anansi vault/database and never calls a write/ingest/mutate tool.

---

## Mode

- **refresh** (default, or arg `refresh`) — create `~/llm-wiki` if missing and unzip the current wiki over it. Idempotent; updates every note.
- **rebuild** (arg `rebuild` / "fresh" / "clean") — remove the local wiki files (`*.md` + `.lint-state`) first, then unzip a clean copy. Use this to make the local wiki **exactly** match the server, dropping files for notes that were deleted server-side. Only ever deletes `*.md`/`.lint-state` inside `~/llm-wiki` — never the folder itself or other files.

---

## Steps

### 1 — Export the wiki on the server

Call the MCP tool **`anansi_export_wiki`** (no arguments). It projects the full graph to a zip. The result is a JSON string inside the tool's text content — parse it for `download_url` (and `filename`). If the tool errors, stop and report — do not touch local files.

### 2 — Resolve the local wiki path

The wiki lives at the **parent of your PARA roots**, folder name `llm-wiki` (matches the server's `[wiki] dir` basename):

- **Linux:** `~/llm-wiki` → `/home/<user>/llm-wiki`
- **macOS:** `~/llm-wiki` → `/Users/<user>/llm-wiki`
- **Windows (PowerShell):** `$env:USERPROFILE\llm-wiki`

Let the shell expand `~`/`$HOME` — do not hardcode a username.

### 3 — Download + unpack

Decide the mode from the argument (default refresh). This block is **bash (Linux/macOS)** — on Windows run it under WSL, or translate to PowerShell (`$env:USERPROFILE\llm-wiki`, `Invoke-WebRequest`, `Expand-Archive`).

```bash
set -e
: "${HOME:?HOME not set}"          # refuse to run without a real home dir
WIKI="$HOME/llm-wiki"

URL='<download_url from step 1>'
# If URL starts with "/" it is RELATIVE (server has no public_url set). Prepend
# your anansi server base URL — the host configured for the anansi MCP connection:
#   case "$URL" in /*) URL="https://anansi.pakele.ai$URL" ;; esac
# (Better: set server.public_url in anansi.toml so the tool returns an absolute URL.)

mkdir -p "$WIKI"

# rebuild mode ONLY — wipe the wiki's own files first (never the folder, never other files):
# rm -f "$WIKI"/*.md "$WIKI"/.lint-state

# Download (the /exports/ route is auth-protected — same key as the anansi MCP server):
TMP="$(mktemp -t anansi-wiki-XXXXXX.zip)"
curl -fSL -H "Authorization: Bearer $ANANSI_API_KEY" "$URL" -o "$TMP"
#   ↑ if the host wants the key as a query param instead of a header:
#     curl -fSL "${URL}?api_key=$ANANSI_API_KEY" -o "$TMP"

unzip -o "$TMP" -d "$WIKI"
rm -f "$TMP"
ls "$WIKI" | wc -l
```

- Uncomment the `rm -f` line **only** in rebuild mode.
- If `download_url` is relative (`/exports/...`), prepend your anansi base URL (the host from the anansi MCP config) per the comment — a bare `/exports/...` can't be downloaded.
- `ANANSI_API_KEY` is the same key configured for the anansi MCP connection. If it's unset (and the server requires one), stop and tell the user to `export ANANSI_API_KEY=<key>` (the key lives in their anansi MCP/server config). A 401 means the key is wrong/missing; a 404 means the export expired — re-run step 1.

### 4 — Report

Tell the user:
- the wiki path (`~/llm-wiki`),
- how many files landed (note files + `index.md` [+ `log.md`/`lint.md`]),
- the mode used (refresh/rebuild),
- "Open `~/llm-wiki` in Obsidian — graph view will connect the notes."

Confirm `index.md` exists in the folder.

---

## Hard Rules

- Never call any Anansi **write/ingest/mutate** tool (`anansi_capture`, `anansi_ingest_*`, `anansi_relate`, `anansi_update_note`, `anansi_delete_note`, `anansi_archive_note`, `anansi_purge`). The only MCP call is `anansi_export_wiki`.
- Never delete the `~/llm-wiki` directory itself, or anything outside it. In rebuild, delete only `*.md` and `.lint-state` inside it.
- Never hardcode a home path or username — resolve `~`/`$HOME`/`$env:USERPROFILE`.
- Always remove the temp zip when done.
