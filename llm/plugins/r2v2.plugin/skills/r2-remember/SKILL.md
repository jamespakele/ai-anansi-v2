---
name: r2-remember
description: >
  Ingests any content into the Anansi knowledge base — YouTube videos,
  article URLs, local files, pasted text, or named entities. Handles source
  extraction for URLs (yt-ingest for YouTube, url-ingest for articles), then
  hands the resulting file to anansi:anansi-remember for vault commit.
  Non-URL inputs (files, pasted text, entities) go directly to
  anansi-remember. This is the primary "remember this" entry point in the
  r2v2 plugin. Triggers: "remember this", "remember this article",
  "remember this video", "save this to anansi", "ingest this url",
  "remember this youtube", "send to anansi", "ingest this", "commit to
  anansi", "/remember", or any YouTube URL or article URL dropped with
  intent to capture and store.
argument-hint: "[YouTube URL | article URL | file path | pasted content | entity facts]"
---

# r2-remember

The r2v2 entry point for committing anything to Anansi. Handles the full
path from raw URL to vault — extraction first, then commit.

Two legs, one skill:
1. **Extract** — if the input is a URL, call the right ingest skill to
   produce a clean markdown file.
2. **Commit** — hand the file (or the original input, if no extraction was
   needed) to `anansi:anansi-remember`.

---

## Step 1 — Classify the input

| Input | Action |
|---|---|
| YouTube URL (`youtube.com`, `youtu.be`, `youtube.com/shorts`) | Extract via `r2v2:yt-ingest` → commit |
| Any other `http://` or `https://` URL | Extract via `r2v2:url-ingest` → commit |
| Local file path | Skip extraction → commit directly |
| Pasted text, entity facts, atomized content | Skip extraction → commit directly |

---

## Step 2 — Extract (URL inputs only)

**YouTube:** Invoke `r2v2:yt-ingest` with the URL. It returns a clean
markdown file path. Hold that path for Step 3.

**Article URL:** Invoke `r2v2:url-ingest` with the URL. It returns a clean
markdown file path. Hold that path for Step 3.

Do not inline the extraction logic — the ingest skills own that. Pass the
URL and receive the file path back.

---

## Step 3 — Commit to Anansi

Invoke `anansi:anansi-remember` with:
- The file path from Step 2 (for URL inputs), or
- The original input as-is (for files, pasted text, entities).

`anansi-remember` handles all routing from there — single entity →
anansi-atom, raw document → para-process → sb-atomize → ingest,
pre-atomized → direct ingest. Do not duplicate that logic here.

---

## Report

Surface whatever `anansi-remember` returns. For URL inputs, prepend a
single line noting the extraction source:

```
Extracted via yt-ingest → committed to Anansi
[anansi-remember report follows]
```

---

## Error handling

| Situation | Action |
|---|---|
| yt-ingest or url-ingest fails | Surface the error from the ingest skill. Do not proceed to commit. |
| anansi-remember fails | Surface the error. The extracted file is on disk — offer to retry the commit. |
| Input type is ambiguous (URL vs file path) | Prefer URL classification if the string starts with `http`. Ask if still unclear. |
