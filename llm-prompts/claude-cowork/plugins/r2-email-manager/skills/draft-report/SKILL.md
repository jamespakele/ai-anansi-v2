---
name: draft-report
description: >
  Takes any content — status update, meeting recap, data summary, project brief
  — and formats it as a Smart Brevity card-and-bullet email report, then saves
  it as a Gmail draft. Each major topic becomes one card: bold title, lede
  sentence, bullet details. Multiple topics stack as multiple cards. Matches
  voice to Pakele-OS context. Shows for review before saving. Never sends.
  Triggers: "draft a report", "draft-report", "/draft-report", "format this as
  a report email", "card and bullet format", "make this a report draft",
  "format this for email", "draft a status update", "draft an update email",
  "put this in card format", "report email", "email report", "draft this as cards
  and bullets", "format as card and bullet".
argument-hint: "[recipient] [content to format as report]"
---

# draft-report

Format any content as a Smart Brevity card-and-bullet email report and save it
as a Gmail draft. Each topic gets its own card — bold title, one-sentence lede,
bullet details. The whole thing stays scannable at a glance.

---

## Card-and-bullet format

A report email is a stack of cards. Each card covers one topic or section.

```
**{Card Title}**

{Lede — one sentence. The single most important thing about this topic.}

- {Bullet: key detail, decision, or data point}
- {Bullet: key detail, decision, or data point}
- {Bullet: key detail, decision, or data point}
```

Rules:
- Card title: bold, sentence case, ≤6 words
- Lede: ONE sentence. Most important thing first. No throat-clearing.
- Bullets: parallel structure, each ≤15 words, start with a noun or verb
- 3–5 bullets per card is the sweet spot — more than 5 means the card should
  be split or the extras cut
- Cards separated by a blank line — no horizontal rules, no headers, no
  section numbers

**Opening line (before the first card):** one sentence that frames the whole
report — what it covers and why it's landing in the recipient's inbox now.
Format: `{One sentence context setter. No "I hope this finds you well."`

**Closing line (after the last card):** one sentence — the ask, the next step,
or a clear close. If there's nothing to ask or close, omit it entirely.

---

## Step 1 — Understand the input

Extract:
- **Recipient** — who is this report going to
- **Content** — the raw material (status notes, data, meeting recap, etc.)
- **Report scope** — what topics need to be covered
- **Any deadline or ask** — if the report requires a response or action

If the content is sparse or the topics aren't clear, make a single pass to
identify the logical sections before proceeding. Do not ask the user to
re-organize — infer the structure from what's given.

---

## Step 2 — Infer context and voice

Load `../../references/gmail-adapter.md` for the context → voice mapping.
Infer context from the recipient and content signals.

Voice applies to the opening line, card titles, and closing — not to bullet
content, which should be factual and neutral regardless of context.

State the inferred context before showing the draft:
`Context: {context} · Recipient: {recipient}`

---

## Step 3 — Structure the report

Map the input content to cards:

1. Identify the distinct topics or sections in the source material
2. Assign one card per topic
3. For each card:
   - Write a bold title (≤6 words, sentence case)
   - Write the lede (the single most important thing about this topic)
   - Extract 3–5 bullet points (key facts, decisions, data, next steps)
4. Order cards by importance — most critical or time-sensitive first
5. Write the opening line and closing line

**Subject line:** apply the Smart Brevity tease rule — 6 words or fewer,
specific and active. The subject should tell the recipient exactly what kind
of report this is and why it matters now.
Example: "Q2 status — three decisions needed" not "Update on Q2 progress"

---

## Step 4 — Show for review

Display the full formatted draft:

```
---
Context: {context}

To: {recipient}
Subject: {subject line}

{Opening line}

**{Card 1 Title}**

{Lede}

- {Bullet}
- {Bullet}
- {Bullet}

**{Card 2 Title}**

{Lede}

- {Bullet}
- {Bullet}
- {Bullet}

{Closing line or ask — omit if not needed}
---

Ready to save as draft? Say yes to save, or tell me what to adjust.
```

Wait for confirmation. Do not save until approved.

---

## Step 5 — Save as Gmail draft

On confirmation, call `create_draft` with:
- `to`: recipient email
- `subject`: the report subject line
- `body`: the full card-and-bullet report
- `threadId`: if this is a reply or continuation of an existing thread

Report:
```
Draft saved — {subject line}
Open in Gmail to review and send.
```

Never send. Drafts only.
