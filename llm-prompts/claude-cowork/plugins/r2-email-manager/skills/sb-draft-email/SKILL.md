---
name: sb-draft-email
description: >
  Takes rough intent or notes — bullet points, rambling thoughts, a brain dump
  — polishes them into a coherent email, passes the result to sb-compress for
  full Smart Brevity compression (Core 4 + type-specific rules + Varys pass),
  matches voice to Pakele-OS context, shows the draft for review, then saves it
  to Gmail drafts via Gmail MCP. Never sends. Triggers: "sb draft", "smart
  brevity draft", "draft this email", "sb this into an email", "write an email
  and sb it", "/sb-draft-email", "draft an sb email", "polish and draft",
  "turn this into an email", "smart brevity this and draft it", "write a tight
  email about", "draft email smart brevity".
argument-hint: "[recipient] [rough content / intent / bullet points]"
---

# sb-draft-email

Two passes, then save. Pass 1: polish rough input into a coherent, purposeful
email. Pass 2: hand that draft to `sb-compress` for full Smart Brevity
compression — Core 4, email-specific rules, Varys pass, all of it. Then
review and save to Gmail. Never send.

---

## Step 1 — Understand the input

Extract from the user's message:

- **Recipient** — name, email, or description ("Lori", "the client", "the board")
- **Rough content** — whatever they provided: bullets, rambling notes, a single
  sentence of intent, or a longer brain dump
- **Tone hint** — any explicit instruction ("keep it brief", "be warm", "formal")
- **Any deadline or call to action** — if stated

If recipient is unclear, infer from context or ask one focused question before
proceeding. If content is extremely sparse ("just say I'm running late"), proceed
directly — no clarification needed.

---

## Step 2 — Infer context and voice

Use the recipient, subject matter, and content signals to infer Pakele-OS context.
Load `../../references/gmail-adapter.md` for the context → voice mapping.

| Context | Voice |
|---------|-------|
| `me` | Warm, informal, contractions fine |
| `dcs` | Mission-forward, respectful, we/our framing |
| `iq` | Professional, crisp, no fluff |
| `ai` | Direct, collaborative, technically specific |
| `anykine` | Civic, accessible, community-minded |

State the inferred context in one line before proceeding:
`Context: {context} · Voice: {voice description}`

---

## Step 3 — Polish pass

Turn the rough input into a coherent, purposeful email draft. This pass
focuses on structure and completeness — not brevity yet. `sb-compress` handles
that in the next step.

Rules for the polish pass:
- Identify the one thing this email needs to accomplish
- Ensure every necessary piece of information is present
- Sequence logically: context → point → ask or close
- Write in the voice for the inferred context
- Write complete, clean prose — `sb-compress` will tighten it

---

## Step 4 — Smart Brevity compression via sb-compress

Pass the polished draft to `sb-compress` with input type `email`.

`sb-compress` applies the full email rule set: Core 4 (muscular subject line,
strong lede, why it matters, go deeper), email-specific formatting, and a
Varys pass to catch anything the compression leaves behind.

Do not apply Smart Brevity rules manually in this skill. `sb-compress` owns
that logic — defer to it entirely.

The output from `sb-compress` is the draft body. Take the subject line it
produces as the email subject.

---

## Step 5 — Show for review

Display the compressed draft in full before saving:

```
---
Context: {context} · Voice: {voice}

To: {recipient}
Subject: {subject line from sb-compress}

{compressed body from sb-compress}
---

Ready to save as draft? Say yes to save, or tell me what to change.
```

If `sb-compress` produced one or more Varys whisper notes, surface them below
the draft in the chat:
```
📡 Varys: {whisper — signal the compression left behind}
```

These will be appended to the Gmail draft in a styled red callout box (see
Step 6) so they're visible in Gmail and easy to delete before sending.

Wait for confirmation. Do not save until the user approves.

---

## Step 6 — Save as Gmail draft

On confirmation, build the draft body as HTML so Varys whispers can be styled.

### HTML body structure

```html
<div style="font-family: sans-serif; font-size: 14px; line-height: 1.6; color: #1a1a1a;">

  {email body — convert line breaks to <br>, bold markdown to <strong>}

  {if one or more Varys whispers exist, append the following block:}

  <div style="margin-top: 24px; padding: 10px 14px; background-color: rgba(220, 38, 38, 0.07); border-left: 3px solid rgba(220, 38, 38, 0.35); border-radius: 4px;">
    <div style="font-size: 11px; font-weight: 600; color: rgba(180, 20, 20, 0.65); margin-bottom: 6px; text-transform: uppercase; letter-spacing: 0.05em;">⚠ Varys signals — delete before sending</div>
    {for each whisper:}
    <div style="font-size: 13px; color: #4a1a1a; margin-bottom: 4px;">📡 {whisper lede}</div>
    <div style="font-size: 12px; color: #6b2a2a; margin-left: 16px; margin-bottom: 8px;">↳ {whisper why}</div>
  </div>

</div>
```

If there are no Varys whispers, omit the callout block entirely and use plain
text body instead of HTML (no need to add HTML wrapper overhead for a clean draft).

Call `create_draft` with:
- `to`: recipient email (ask if only a name was provided and not resolvable)
- `subject`: subject line from `sb-compress`
- `body`: HTML body as constructed above
- `mimeType`: `text/html` when Varys whispers are present
- `threadId`: if this is a reply to an existing thread, include it

Report:
```
Draft saved — {subject line}
Open in Gmail to review and send.
{if whispers: "Varys signals are highlighted in the draft — delete the red box before sending."}
```

Never send. Drafts only.
