---
name: anansi-ingest-file
description: >
  Sends a raw document to the Anansi server-side inbox pipeline. Prefers
  direct file upload via curl (calls anansi_get_upload_url + bash) when a
  local file path is given - faster and avoids passing large content through
  MCP. Falls back to anansi_ingest_file with content string for pasted text
  or when bash is unavailable. The server runs the full pipeline:
  para-projects-areas + para-resource-entities -> sb-atomize -> DB.
  Triggers: "ingest this file", "send to inbox", "drop this in the inbox",
  "server-side ingest", "anansi-ingest-file", "/anansi-ingest-file",
  "queue this for anansi", "ingest this document", "upload to anansi inbox",
  "drop this in anansi", "process this server-side", "upload this file to anansi",
  "send to anansi", "fire and forget to anansi", "let the server handle this".
argument-hint: "[file path or pasted document content]"
---

# anansi-ingest-file

Drop a raw document into the Anansi server-side inbox pipeline.

The server handles everything: PARA extraction, Smart Brevity atomization,
and ingest into the knowledge base. You hand it content; the server does
the work.

This is the **server-side path**. Use it when:
- You want fire-and-forget — no need to watch the pipeline steps
- The document is large and client-side processing would be slow
- You're batching several documents quickly

Use **r2-remember** instead when you want to review the pipeline output
(atomized notes, TOC) before it enters the vault.

---

## Step 1 — Resolve input and choose path

**File path given AND bash is available (Claude Code context):**
→ Use the **curl upload path** (Step 2A). Faster, no content in MCP.

**Pasted content, or bash is not available:**
→ Use the **content path** (Step 2B).

---

## Step 2A — Curl upload path (preferred for file paths)

1. Call `anansi_get_upload_url` with the local file path:
   ```
   anansi_get_upload_url { "local_path": "/path/to/file.md" }
   ```
2. The tool returns `curl_inbox` — a ready-to-run curl command. Execute it via bash:
   ```bash
   curl -F 'file=@/path/to/file.md' https://anansi.pakele.ai/upload/inbox
   ```
3. Expected response: `{ "status": "queued", "filename": "...", "bytes": N }`

Do **not** read the file content into the conversation.

---

## Step 2B — Content path (pasted content or no bash)

1. Use the pasted text as-is. Do not rewrite or summarize.
2. Derive a filename from the first heading or first line, kebab-cased with
   a `.md` extension (e.g. `broadband-hui-notes.md`). If no heading,
   use `pasted-{ISO-date}.md`.
3. Call `anansi_ingest_file`:
   ```json
   {
     "content": "<CONTENT>",
     "filename": "<derived filename>",
     "source": "skill"
   }
   ```
   Always include `"source": "skill"`.

---

## Step 3 — Report

```
*Anansi inbox* — {filename}
• Queued for server-side pipeline
• {any job_id or queue confirmation from the response}
• The server will run: PARA extraction → sb-atomize → ingest
```

If the response includes a queue ID, job reference, or status field, include
it in the report so the user has a handle to track the run.

---

## Error handling

| Situation | Action |
|---|---|
| File path given but file not found | Stop. Confirm path with the user. |
| `anansi_ingest_file` not available | Tell the user. Offer to save content to disk for manual upload when the connector is back. |
| `anansi_ingest_file` returns an error | Show the error verbatim. Offer to save CONTENT to disk so the user can retry. |
| Content is already atomized (has `<!-- anansi-atomize: ... -->` header) | Note this to the user. The server will re-atomize it. If the user wants direct ingest instead, suggest using r2-remember Path C (`anansi_ingest_atomized`). |

---

## vs. r2-remember

| | anansi-ingest-file | r2-remember (Path B) |
|---|---|---|
| Pipeline location | Server-side | Client-side (in-conversation) |
| Intermediate output | None (opaque) | Atomized .md + TOC files on disk |
| Speed | Fast drop | Slower; full pipeline visible |
| Reviewability | None before ingest | Review atomized not