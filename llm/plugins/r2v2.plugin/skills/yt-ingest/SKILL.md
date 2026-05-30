---
name: yt-ingest
description: >
  Pulls a YouTube video transcript and cleans it into a prose markdown file.
  Pure extraction — no PARA logic, no atomization, no Anansi writes. The
  caller decides what happens next: r2v2:r2-remember routes to Anansi,
  r2v2:quick-read routes to sb-compress. Output is a single markdown file
  with title, channel, URL, video ID, and fetch date in the header — the
  rest is the cleaned transcript. Prefers yt-dlp; falls back to
  youtube-transcript-api. Triggers: "yt-ingest", "/yt-ingest", "pull this
  youtube transcript", "transcribe this youtube", or any YouTube URL when
  called by an orchestrating skill.
argument-hint: "[YouTube URL or video ID]"
---

# yt-ingest

Pulls a YouTube transcript and cleans it into a prose markdown file. That's it.

This is a **pure extraction skill**. It does one thing: turn a YouTube URL into clean text. It does not classify, atomize, commit to Anansi, or decide what happens next. The calling skill handles routing — `r2v2:r2-remember` sends it to Anansi, `r2v2:quick-read` sends it to sb-compress.

---

## When to invoke

Invoke when:
- The user drops a YouTube URL with intent to capture, summarize, or remember it
- The user says "yt-ingest", "ingest this youtube video", "pull this transcript", "remember this youtube video"
- A YouTube URL appears in input without a specified action — default to ingest

Do not invoke:
- For YouTube URLs the user is just discussing without intent to capture
- When the user explicitly wants only a summary or chat-only Q&A — those don't need vault commitment

---

## Inputs

One of:
- A full YouTube URL (`https://www.youtube.com/watch?v=...`, `https://youtu.be/...`, `https://www.youtube.com/shorts/...`)
- A bare 11-character video ID

If neither is parseable, stop and ask the user for the URL.

---

## Step 1 — Resolve the video ID

Extract the 11-character video ID from the URL. Standard patterns:

| URL form | ID location |
|---|---|
| `youtube.com/watch?v=ID` | `v` query param |
| `youtu.be/ID` | path segment |
| `youtube.com/shorts/ID` | path segment after `/shorts/` |
| `youtube.com/embed/ID` | path segment after `/embed/` |

Strip any trailing query params (`&t=...`, `&list=...`).

---

## Step 2 — Pull transcript

**Path A — yt-dlp (preferred):**

```bash
yt-dlp \
  --skip-download \
  --write-auto-subs \
  --write-subs \
  --sub-langs "en.*,en" \
  --sub-format "vtt" \
  --convert-subs srt \
  --print "%(title)s\t%(channel)s\t%(upload_date)s" \
  -o "/tmp/yt-ingest-%(id)s.%(ext)s" \
  "https://www.youtube.com/watch?v=<VIDEO_ID>"
```

This writes a `.en.srt` (or similar) alongside printing title/channel/upload_date. Read both.

If yt-dlp is missing, install it:

```bash
pip install yt-dlp --break-system-packages
```

**Path B — youtube-transcript-api (fallback):**

```bash
pip install youtube-transcript-api --break-system-packages
python3 -c "
from youtube_transcript_api import YouTubeTranscriptApi
import sys, json
t = YouTubeTranscriptApi.get_transcript('<VIDEO_ID>', languages=['en','en-US','en-GB'])
print(json.dumps(t))
"
```

Returns a list of `{text, start, duration}` segments. No title/channel — fetch those separately via the oEmbed endpoint:

```bash
curl -s "https://www.youtube.com/oembed?url=https://www.youtube.com/watch?v=<VIDEO_ID>&format=json"
```

---

## Step 3 — Clean the transcript

Reduce to readable prose. Apply in order:

1. **Strip SRT/VTT artifacts** — remove sequence numbers, `-->` timestamp lines, and the `WEBVTT` header.
2. **Collapse duplicate caption lines** — auto-generated subtitles often repeat the tail of the previous line at the head of the next.
3. **Join word-wrapped lines** — captions break mid-sentence on display width, not grammar. Join lines that don't end with sentence-final punctuation.
4. **Normalize whitespace** — single space between words, single blank line between paragraphs.
5. **Insert paragraph breaks** every ~4–6 sentences, or at clear topic shifts. Don't over-segment.
6. **Strip stage directions** — `[Music]`, `[Applause]`, `[Inaudible]`, etc. — unless they're load-bearing for context (rare).

Do **not** summarize, paraphrase, or remove content. Cleaning ≠ rewriting.

---

## Step 4 — Build the markdown file

Filename: `yt-{video_id}-{title-slug}.md` in the working outputs directory. Slug: lowercase, alphanumerics + hyphens, max 60 chars.

Body:

```markdown
---
type: youtube-transcript
title: <title>
channel: <channel>
url: https://www.youtube.com/watch?v=<VIDEO_ID>
video_id: <VIDEO_ID>
upload_date: <YYYY-MM-DD>
fetched_at: <ISO timestamp>
source: yt-ingest
---

# <title>

**Channel:** <channel>
**Uploaded:** <YYYY-MM-DD>
**URL:** https://www.youtube.com/watch?v=<VIDEO_ID>

---

<cleaned transcript prose>
```

The frontmatter is for downstream skills (sb-atomize will read it). The body is what anansi-remember sees as content.

---

## Step 5 — Return to caller

Return the file path and a brief summary. The calling skill decides what comes next.

```
*yt-ingest* — <title>
• Channel: <channel>
• URL: https://www.youtube.com/watch?v=<VIDEO_ID>
• Transcript length: <N> words / <N> paragraphs
• File: <path>
```

---

## Error handling

| Situation | Action |
|---|---|
| URL not parseable | Stop. Ask the user for a valid YouTube URL or video ID. |
| Video has no transcript (neither manual nor auto) | Stop. Tell the user — note that very short Shorts and music-only videos often lack transcripts. Offer to capture metadata only via anansi-atom. |
| yt-dlp install fails AND youtube-transcript-api install fails | Stop. Report both failures verbatim. Do not fall through to scraping. |
| Transcript is in a non-English language | Pull as-is. Add `language: <code>` to the frontmatter. Do not translate — that's lossy and outside this skill's scope. |
| Video is age-restricted or private | yt-dlp will report it. Tell the user — this skill cannot bypass auth. |
| Transcript is unusually short (<200 words) for a long video | Note it in the report. Auto-captions may have failed silently. The user can decide whether to ingest. |

---

## Design notes

**Extraction-only.** This skill produces one clean file and stops. Routing, classification, and storage belong to the calling skill — never to this one. Resist any urge to atomize, summarize, or commit to Anansi from within yt-ingest.

**Cleaning ≠ rewriting.** The atomization pipeline downstream will produce Smart Brevity notes. This skill's job is to give it clean prose to work with — not to pre-distill it.

**Metadata is load-bearing.** Title, channel, URL, and upload date are all entity-relevant — sb-atomize will pull them into the resulting notes. The frontmatter block is the durable record of provenance.
