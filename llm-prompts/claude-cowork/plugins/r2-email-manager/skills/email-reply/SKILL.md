---
name: email-reply
description: >
  Drafts a Gmail reply to a specific thread using the correct Pakele-OS context
  voice. Infers context (me/dcs/iq/ai/anykine) from the thread content and
  applies the matching tone — warm/personal for me, mission-driven for dcs,
  professional for iq, collaborative-technical for ai, civic-accessible for
  anykine. Saves the draft via Gmail MCP. Does not send — user reviews and
  sends manually. Triggers: "draft a reply", "reply to this email", "write a
  reply to", "draft reply to [subject/sender]", "email-reply", "/email-reply",
  "respond to this thread", "help me reply to", "compose a reply",
  "draft an email back to".
argument-hint: "[thread ID or subject] [--tone formal|casual|brief] [intent: what to say]"
---

# email-reply

Drafts a Gmail reply for a thread. Context-matched voice, saved as a Gmail
draft. User reviews and sends — this skill never sends on its own.

---

## Step 1 — Identify the thread

Determine which thread to reply to from the user's input:

- If a thread ID is provided: call `get_thread` directly.
- If a subject or sender name is provided: call `search_threads` with
  `subject:"{subject}"` or `from:{sender}` to locate it. If multiple matches,
  surface a short list and ask the user to confirm.
- If the user pastes thread content directly into the conversation: use that.
  No API call needed.

---

## Step 2 — Read `../../references/gmail-adapter.md`

Load the context inference signals and voice/tone table before drafting.

---

## Step 3 — Infer context and tone

From the thread subject, sender, body, and any existing labels, determine:

**Context** (one of: `me` / `dcs` / `iq` / `ai` / `anykine`) — use the
inference signals in `gmail-adapter.md`.

**Tone override** (from user flag or request):
- `--tone formal` → elevate register regardless of context
- `--tone casual` → loosen register regardless of context
- `--tone brief` → cap reply at 3–5 sentences, no pleasantries
- No flag → use the default voice for the inferred context

If context is ambiguous, state the inferred context before drafting and let
the user correct it.

---

## Step 4 — Understand the intent

Determine what the reply needs to accomplish:

1. If the user stated intent ("confirm the meeting", "decline", "ask for the
   doc", "follow up on status") — use that.
2. If no intent was stated — read the thread and infer the obvious next reply
   (acknowledge, answer a question, confirm, follow up). State the inferred
   intent in one line before drafting: `Drafting: [inferred intent]`.
3. If intent is genuinely unclear — ask one focused question before proceeding.

---

## Step 5 — Draft the reply

Write the reply body using the voice for the inferred context:

| Context | Voice rules |
|---------|-------------|
| `me` | Warm, first-name basis, contractions fine, conversational |
| `dcs` | Respectful, mission-forward, we/our framing, avoid jargon |
| `iq` | Professional, crisp, client-facing, no fluff |
| `ai` | Direct, collaborative, technically specific, peer tone |
| `anykine` | Civic, accessible, community-minded, inclusive language |

**Universal rules for all contexts:**
- No filler openers ("Hope this finds you well", "Thanks for reaching out")
- Lead with the point
- Match the length of the incoming email — don't write a paragraph to a one-liner
- If action is needed from the recipient, state it clearly at the end
- If `--tone brief`: hard cap 3–5 sentences, strip all pleasantries

---

## Step 6 — Show draft for review

Display the draft in full before saving:

```
---
To: {recipient(s)}
Subject: Re: {original subject}
Context: {inferred context} · Tone: {voice used}

{draft body}
---

Ready to save as draft? Say yes to save, or tell me what to change.
```

Wait for the user to confirm or request changes. Do not save until confirmed.

---

## Step 7 — Save as Gmail draft

On confirmation, call `create_draft` with:
- `to`: original sender (and any others if the user says reply-all)
- `subject`: `Re: {original subject}` (or as-is if already prefixed)
- `body`: confirmed draft text
- `threadId`: the thread ID from Step 1

Report:
```
Draft saved — Re: {subject}
Open in Gmail to review and send.
```

Never call `send` — drafts only. The user sends from Gmail.
