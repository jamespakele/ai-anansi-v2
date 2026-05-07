---
name: quick-read
description: >
  Extracts and Smart Brevity-compresses any content for immediate reading —
  YouTube videos, article URLs, local files, emails, or pasted text. Produces
  a tight, scannable digest you can read right now, with the clean source file
  saved and available for Anansi ingestion afterward via r2v2:r2-remember.
  Does NOT commit to Anansi — this is read-first, ingest-later. Use
  quick-read whenever the user wants to quickly consume a piece of content:
  "quick read this", "/qr", "summarize this video", "sb this article",
  "compress and show me this", "read this for me", "give me the gist of",
  "digest this", "tl;dr this url", "what does this say", or drops a YouTube
  URL, article URL, or file and wants to understand it fast. Also fires when
  the user wants Smart Brevity applied to external content rather than their
  own writing.
---

# quick-read

Turn any input into a Smart Brevity digest you can read in under two minutes.

Two legs, one skill:
1. **Extract** — get clean text from whatever arrived (URL, file, or
   pasted content).
2. **Compress** — hand it to `sb-compress` and display the result.

No Anansi pipeline, no vault writes. The extracted source file is saved to
the outputs directory and is ready for `r2v2:r2-remember` anytime, but
that's optional and separate.

---

## When to invoke

- User drops a YouTube URL and wants to understand it quickly
- User drops an article URL and wants the gist
- User pastes a document, email, transcript, or block of text and says
  "compress this", "quick read", "digest this", "tl;dr"
- User says "/qr", "quick-read", "give me the gist", "what does this say"
- User wants Smart Brevity applied to content they didn't write

Do NOT invoke when:
- The user wants to commit to Anansi → use `r2v2:r2-remember`
- The user wants to compress their own outgoing draft → use `sb-compress`
  directly (no extraction needed here)
- The user wants task extraction and TickTick routing → use
  `r2v2:inbox-triage`

---

## Step 1 — Classify the input

| Input | Type | Extraction |
|---|---|---|
| `youtube.com/watch`, `youtu.be`, `youtube.com/shorts` | **youtube** | `r2v2:yt-ingest` |
| Any other `http://` or `https://` URL | **article** | `r2v2:url-ingest` |
| File path (`.pdf`, `.docx`, `.txt`, `.md`, etc.) | **file** | Read directly |
| Email-shaped text or any pasted content | **text** | Use as-is |

If multiple URLs or files arrive, process each in turn.

---

## Step 2 — Extract

**YouTube URL:** Invoke `r2v2:yt-ingest`. It returns a clean markdown file
path. Hold the file contents for Step 3.

**Article URL:** Invoke `r2v2:url-ingest`. It returns a clean markdown file
path. Hold the file contents for Step 3.

**File:** Read the file with the Read tool (`.txt`, `.md`) or via bash for
PDFs and DOCX:

```bash
# PDF
pip install pdfminer.six --break-system-packages
python3 -c "
from pdfminer.high_level import extract_text
print(extract_text('<path>'))
"

# DOCX
pip install python-docx --break-system-packages
python3 -c "
import docx
doc = docx.Document('<path>')
print('\n'.join(p.text for p in doc.paragraphs if p.text.strip()))
"
```

**Pasted text / email:** Use the content as-is. No extraction step needed.

---

## Step 3 — Invoke sb-compress

Pass the extracted content to `sb-compress`. Invoke the skill directly.
Hand it the full text — do not pre-summarize or pre-chunk. Let sb-compress
identify the content type, apply the matching Smart Brevity skeleton, and
run the Varys pass. It will return the compressed text, compression
metadata, and any whispers.

---

## Step 4 — Display

Present the sb-compress output inline. Add a compact two-line footer:

```
---
**quick-read** · <type> · <title or URL>
Source file: <path> — run r2v2:r2-remember on it to commit to Anansi
```

The footer is a pointer, not a summary. Keep it two lines.

---

## Error handling

| Situation | Action |
|---|---|
| yt-ingest or url-ingest fails | Surface the error from the ingest skill. Stop. |
| File not found or unreadable | Stop. Ask the user to confirm the path. |
| Extracted content is very short (<200 words on a long source) | Note it in the footer and proceed — let the user decide. |
| Input type is ambiguous | Ask before proceeding. One question beats a wrong extraction path. |

---

## Design notes

**Extraction is delegated, not inlined.** yt-ingest and url-ingest own
their extraction logic. quick-read calls them and receives a file path
back. This keeps the skill thin and ensures extraction improvements
propagate automatically.

**Don't pre-summarize before sb-compress.** Hand the full extracted text
to sb-compress. Pre-summarizing loses the signal Varys is designed to
catch.

**One input, one compress.** Multiple URLs → process in sequence, one
digest per source. Smart Brevity's One Big Thing structure requires a
single subject.

**The source file is the artifact.** The compressed output is for reading
now. The source markdown file — produced by yt-ingest or url-ingest —
carries the provenance metadata the PARA pipeline needs if the user later
decides to commit it to Anansi via r2v2:r2-remember.
