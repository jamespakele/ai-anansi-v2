---
name: url-ingest
description: >
  Fetches a web URL, strips boilerplate (nav, ads, comments, related-content
  rails), extracts the main article body, and returns the file path. Pure
  extraction — no PARA logic, no atomization, no Anansi writes. The calling
  skill decides what happens next: r2v2:r2-remember hands off to
  anansi:anansi-remember for vault ingest; r2v2:quick-read hands off to
  sb-compress for display. Output is a single markdown file with title, source
  URL, author, publish date, and fetch date in the header — body is the cleaned
  article prose. Prefers WebFetch; falls back to trafilatura or readability-lxml.
  Triggers: "url-ingest", "/url-ingest", "fetch this article", "process this
  link", or when invoked by r2v2:r2-remember or r2v2:quick-read.
argument-hint: "[article URL]"
---

# url-ingest

Fetches a web URL, extracts the article body, and returns the file path to the caller.

This is a **pure extraction skill**. It does one thing: turn a URL into a clean markdown file. It does not classify, atomize, commit to Anansi, or decide what happens next. The calling skill handles routing — `r2v2:r2-remember` sends the file to Anansi, `r2v2:quick-read` sends it to sb-compress.

---

## When to invoke

Invoke when:
- The user drops an article URL with intent to capture, summarize, or remember it
- The user says "url-ingest", "ingest this url", "fetch this article", "remember this article", "process this link"
- An article URL appears in input without a specified action — default to ingest

Do not invoke for:
- YouTube URLs → use `yt-ingest`
- PDF URLs → download via WebFetch and route to a future `pdf-ingest` skill (or hand the file to `r2v2:r2-remember` directly)
- Login-walled URLs (Substack paid posts, paywalled news) — see error handling
- URLs the user is just discussing without intent to capture

---

## Inputs

A single URL. If multiple URLs are passed, ingest each in turn — one file per URL.

If the URL is malformed or missing the scheme (`http://` / `https://`), prepend `https://` and proceed. If still unreachable, stop and ask.

---

## Step 1 — Classify the URL

Quick check before fetching:

| URL pattern | Route |
|---|---|
| `youtube.com`, `youtu.be` | Stop. Use `yt-ingest` instead. |
| Direct PDF (`.pdf` in path) | Download the file, then hand to `r2v2:r2-remember` with the file path. |
| Twitter/X status, Reddit, HN, etc. | Proceed — but expect lower content density. The article body extractor will return only the OP/post body. |
| Everything else | Proceed to Step 2. |

---

## Step 2 — Fetch and extract

**Path A — WebFetch (preferred):**

WebFetch handles JS rendering and returns markdown directly. Pass a prompt that asks for the full article body:

```
WebFetch:
  url: <URL>
  prompt: "Extract the full article body as clean markdown. Preserve headings,
           lists, blockquotes, and inline links. Drop navigation, ads,
           related-content rails, comments, and footers. Include the article
           title as an H1 at the top. Include the author and publish date if
           shown."
```

Take the returned markdown as the article body. WebFetch will not return content for sites it cannot fetch — if the response is empty or clearly truncated, fall back to Path B.

**Path B — curl + trafilatura (fallback):**

```bash
pip install trafilatura --break-system-packages
python3 -c "
import trafilatura, sys, json
url = '<URL>'
downloaded = trafilatura.fetch_url(url)
if downloaded is None:
    sys.exit('fetch_failed')
result = trafilatura.extract(
    downloaded,
    output_format='markdown',
    with_metadata=True,
    include_links=True,
    include_formatting=True,
    favor_precision=True,
)
print(result)
md = trafilatura.bare_extraction(downloaded, with_metadata=True)
print('---META---')
print(json.dumps({k: md.get(k) for k in ['title','author','date','sitename','url']}))
"
```

Trafilatura returns clean markdown plus a metadata block with title, author, date, and canonical URL.

If trafilatura fails or is unavailable, try `readability-lxml` (`pip install readability-lxml`). If both fail, stop and report.

---

## Step 3 — Build the markdown file

Filename: `url-{domain}-{title-slug}.md` in the working outputs directory. Slug: lowercase, alphanumerics + hyphens, max 60 chars. Domain: registered domain only (`example.com`, not `www.example.com`).

Body:

```markdown
---
type: web-article
title: <title>
author: <author or "unknown">
publish_date: <YYYY-MM-DD or "unknown">
url: <canonical URL>
domain: <domain>
fetched_at: <ISO timestamp>
source: url-ingest
---

# <title>

**Source:** <domain> · <author> · <publish_date>
**URL:** <canonical URL>

---

<cleaned article body>
```

The frontmatter is for downstream skills. The body is what anansi-remember sees as content.

---

## Step 4 — Return the file path

Return the file path and a brief summary to the caller. Do not call any downstream skill — routing is the caller's job.

```
*url-ingest* — <title>
• Source: <domain> · <author> · <publish_date>
• URL: <canonical URL>
• Article length: <N> words
• File: <path>
```

---

## Error handling

| Situation | Action |
|---|---|
| URL is unreachable (DNS, 404, 5xx) | Stop. Report the error and the URL. Do not fabricate content. |
| WebFetch refuses the domain (blocklist) | Fall through to Path B. If Path B also fails, stop and tell the user — do NOT use bash/curl/python to bypass WebFetch's domain restrictions. That's a hard rule. |
| Page is paywalled / requires login | Stop. Tell the user. Offer to capture only metadata via anansi-atom (title, URL, source) so the existence of the article is in the vault. Do not attempt to bypass paywalls. |
| Page is mostly empty (article body <100 words) | Note it in the report. Sites with heavy JS or unusual structure sometimes defeat extraction. The user can decide whether to ingest. |
| Extracted body contains obvious nav/footer junk | Re-run trafilatura with `favor_precision=False` and `include_comments=False`. If still polluted, hand it off anyway and note the noise — sb-atomize is robust to some boilerplate. |
| URL contains tracking params (`utm_*`, `fbclid`, `gclid`) | Strip them from the canonical URL stored in frontmatter. Keep the original URL in the report for traceability. |
| URL is a redirect (link shortener) | Follow the redirect. Store the resolved URL in frontmatter. Note the original in the report. |

**Hard rule on web fetching:** if WebFetch and trafilatura both fail, you stop. Do not attempt alternative fetchers, archive sites, or cached versions to retrieve content WebFetch refused. The restrictions on web fetching apply to all retrieval methods.

---

## Design notes

**Extraction-only.** This skill produces one clean file and stops. Routing, classification, and storage belong to the calling skill. Resist any urge to atomize, summarize, or commit to Anansi from within url-ingest.

**Extraction beats fidelity.** A clean article body is more valuable than a faithful HTML reproduction. Drop ads, nav, comments, and rails aggressively — sb-atomize will be confused by them.

**Metadata is load-bearing.** Author, publish date, and canonical URL are entity-relevant — sb-atomize will pull them into the resulting notes. The frontmatter block is the durable record of provenance.

**One URL per call.** Multiple URLs → call this skill multiple times, once per URL. Each gets its own file and its own r2v2:r2-remember run.
