---
name: inbox-pull
description: >
  Fetches Gmail threads via the Gmail MCP and produces a single atomization-ready
  markdown file for r2-remember. Always runs two passes in parallel: (1) first pass
  — everything except Social, Promotions, Newsletters, and Updates — fetches full
  thread content formatted as email_thread entities for para-resource-entities; (2)
  updates pass — Updates category, is:important by default — fetches sender and
  subject only, written as a stub bullet list that the atomization pipeline will NOT
  decompose. Both passes combine into one output file. Use --updates-all to drop the
  importance filter on the updates pass. Triggers: "pull my inbox", "fetch email",
  "get my email", "pull email", "inbox-pull", "/inbox-pull", "check my inbox",
  "what's in my inbox", "fetch unread email", "pull email for triage",
  "pull email for atomization", "get email for atomization".
argument-hint: "[--updates-all] [--since YYYY-MM-DD] [--unread-only]"
---

# inbox-pull

Pulls Gmail threads and writes a single structured markdown file ready to pass
directly to `r2-remember`. Always runs both passes simultaneously. First-pass
threads are fully atomizable email_thread entities. The Updates section is an
intentional stub — sender + subject only — so the pipeline never atomizes noisy
Updates threads into the vault unless you explicitly pull them in full later.

---

## Flags

| Flag | Meaning |
|------|---------|
| *(default)* | Updates pass uses `is:important` to filter high-signal threads only |
| `--updates-all` | Drop the importance filter on the updates pass (expect more noise) |
| `--since YYYY-MM-DD` | Limit both passes to threads with activity on or after this date. Default: 48 hours. |
| `--unread-only` | Only include threads with at least one unread message. Default: true. |

---

## Step 1 — Build queries and fetch in parallel

Run both queries simultaneously.

**First-pass query:**
```
is:inbox -category:social -category:promotions -category:updates is:unread
```
Append `-label:newsletters` if that label exists (check via `list_labels`).
Append `-label:email-manager/processed` if that label exists.

**Updates query (default):**
```
is:inbox category:updates is:unread is:important -label:email-manager/processed
```
With `--updates-all`: drop `is:important`.

For both: append `after:YYYY/MM/DD` if `--since` is set. Default lookback: 48 hours.
Remove `is:unread` if `--unread-only false`.

Thread limit: 30 per pass. Warn if either hits the limit.

---

## Step 2 — Fetch content

**First-pass threads:** call `get_thread` for each result to retrieve full message
bodies. Strip quoted reply chains to reduce noise (remove `>` prefixed lines and
`On [date], [name] wrote:` separators) — but preserve the first occurrence of each
unique sender's message.

**Updates threads:** the `search_threads` snippet response is sufficient. Do NOT
call `get_thread` for Updates — stub entries need only sender name and subject.

---

## Step 3 — Write the output file

Output path:
```
output/inbox-{YYYY-MM-DD}/inbox-{YYYY-MM-DD}.md
```

Use today's date in Hawaii time (`America/Honolulu`).

### File structure

```markdown
---
source_type: email_pull
pull_date: {YYYY-MM-DD}
pulled_at: {ISO 8601 timestamp HST}
first_pass_threads: {N}
updates_threads: {N}
---

# Email pull — {YYYY-MM-DD}

Pulled at: {timestamp HST}
First-pass threads: {N} · Updates threads: {N}

---

{## Thread blocks — one per first-pass thread}

---

## Updates

{stub list}
```

---

### Full thread block format

One block per first-pass thread, ordered by most recent activity descending.
Separated from the next block by `---`.

```markdown
## {Subject line}

- **Thread ID:** {gmail thread id}
- **Subject:** {full subject}
- **From:** {name <email> of most recent sender}
- **To:** {recipient name(s) <email(s)>}
- **CC:** {cc names <emails> — omit line if empty}
- **Participants:** {all unique names and addresses across all messages, comma-separated}
- **Date range:** {first message date} → {most recent message date}
- **Messages:** {count}
- **Labels:** {current gmail labels on thread}
- **Has attachment:** {yes / no}

### Thread content

**{sender name} — {date}:**
{message body}

**{sender name} — {date}:**
{message body}
```

`para-resource-entities` identifies sections with `From:`, `To:`, `Participants:`,
and `Date range:` fields as `email_thread` entities. The full bodies become source
content for `sb-atomize` to decompose into `email_exchange` leaves.

---

### Updates stub section

A single `## Updates` section at the end of the file, after all full thread blocks.

```markdown
## Updates

- **{Sender Name}:** {Subject line}
- **{Sender Name}:** {Subject line}
- **{Sender Name}:** {Subject line}
```

**Why stubs only:** Updates threads are intentionally not fetched in full. The
stub list keeps them visible for human review without putting noisy, low-signal
content into the atomization pipeline. If a thread in the Updates list warrants
full capture, pull it individually with `get_thread` and pass it to `r2-remember`
on its own.

`para-resource-entities` treats a flat bullet list without thread header fields
as a Discussion block — it will not attempt to type these as `email_thread` entities.

If the updates pass returns zero threads, omit the `## Updates` section.

---

## Step 4 — Report

```
Inbox pull complete — {YYYY-MM-DD}

First-pass threads:  {N}  (full content — ready for atomization)
Updates threads:     {N}  (stubs — review list only, not atomized)
Output: output/inbox-{YYYY-MM-DD}/inbox-{YYYY-MM-DD}.md

To atomize: r2-remember output/inbox-{YYYY-MM-DD}/inbox-{YYYY-MM-DD}.md
To label after triage: email-label
```
