---
name: anansi-ingest-file
description: >
  Upload a local file directly to the Anansi server queue using a curl command.
  Calls anansi_get_upload_url to discover the correct server URL dynamically,
  then executes a curl upload — faster than passing file content through MCP.
  Use for raw source documents (inbox pipeline) or already-atomized files
  (atomize queue). Requires bash/terminal access (Claude Code context).
  Triggers: "upload this file to anansi", "ingest this file", "send to anansi",
  "queue this file", "/anansi-ingest-file", "anansi-ingest-file".
argument-hint: "[local file path] [inbox|atomize (default: inbox)]"
---

# anansi-ingest-file

Upload a local file to the Anansi server without reading its content into the
conversation. The server receives the file and queues it for processing.

---

## When to use which queue

- **inbox** (default): Raw source document — meeting notes, article, book chapter,
  earnings release, etc. The server runs the full pipeline:
  `para-projects-areas` + `para-resource-entities` → `sb-atomize` → DB.

- **atomize**: Already-atomized content (output of `sb-atomize` skill). Skips
  the LLM pipeline and ingests directly into the database.

---

## Steps

### Step 1 — Discover the upload URL

Call `anansi_get_upload_url` with the local file path:

```
anansi_get_upload_url { "local_path": "/path/to/file.md" }
```

The tool returns:
- `curl_inbox` — ready-to-run curl command for the inbox queue
- `curl_atomize` — ready-to-run curl command for the atomize queue
- `inbox_url` / `atomize_url` — raw URLs if you need to construct your own command

### Step 2 — Execute the curl command

Run the appropriate curl command from the tool response using bash:

**For a raw source document (full pipeline):**
```bash
curl -F 'file=@/path/to/file.md' https://vps.pakele.ai/upload/inbox
```

**For already-atomized content:**
```bash
curl -F 'file=@/path/to/atomized.md' https://vps.pakele.ai/upload/atomize
```

The server responds with:
```json
{
  "status": "queued",
  "filename": "file.md",
  "queue_dir": "/data/q-inbox",
  "bytes": 12345
}
```

### Step 3 — Confirm and report

Tell the user the file has been queued and what to expect:
- **inbox queue**: Processing takes 1–3 minutes (LLM pipeline). Use
  `anansi_filter` or `anansi_search` after a couple of minutes to verify.
- **atomize queue**: Processing is fast (seconds). The queue watcher polls
  every 10 seconds.

---

## Notes

- Never read file content into the conversation — that's what this skill avoids.
- If `anansi_get_upload_url` returns a `localhost` URL but you're on a remote
  server, the admin needs to set `public_url` in `anansi.toml` under `[server]`.
- Path traversal is blocked — filename must not contain `/` or `..`.
