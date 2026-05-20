# email-manager

Gmail management layer for Pakele-OS. Three skills that close the loop on
email: pull from Gmail directly, draft context-matched replies, and apply
labels after triage.

## Skills

### inbox-pull

Fetches Gmail threads and formats them into a triage-ready markdown bundle.

**Two pass modes:**
- **First pass (default):** everything except Social, Promotions, Newsletters,
  and Updates — the signal-dense threads you need to act on
- **Updates pass (`--updates`):** Updates category only — higher noise, reviewed
  separately

**Usage:**
```
pull my inbox
fetch email for triage
inbox-pull --since 2026-05-01
pull updates email
```

**Output:** `output/inbox-{date}/inbox-{date}-{pass}.md`

---

### email-reply

Drafts a Gmail reply to a specific thread. Infers Pakele-OS context (me/dcs/iq/ai/anykine)
and applies the matching voice. Saves as a Gmail draft — never sends automatically.

**Voice by context:**
- `personal` — warm, informal, first-name
- `dcs` — mission-driven, respectful, we/our framing
- `iq` — professional, crisp, client-facing
- `ai` — direct, collaborative, technically specific
- `anykine` — civic, accessible, community-minded

**Usage:**
```
draft a reply to the thread from Lori about the LNG report
reply to this email — just say I'll have it Friday
email-reply --tone brief
```

---

### email-label

Applies Gmail labels to threads after triage. Creates the label hierarchy if
not present. Marks processed threads so `inbox-pull` skips them on future pulls.

**Labels used:**
- Content: `personal`, `dcs`, `iq`, `ai`, `anykine`, `calendar`
- Operational: `email-manager/processed`, `email-manager/needs-reply`

**Usage:**
```
apply labels after triage
label and archive processed email
email-label --archive
```

---

## Workflow

The intended flow:

```
inbox-pull → email-triage (anthropic-skills) → email-label
                    ↓
              inbox-triage (r2v2) → TickTick tasks
                    ↓
              r2-remember → Anansi vault
```

1. `inbox-pull` — fetch threads from Gmail
2. `email-triage` — classify by GTD/PARA, produce structured markdown
3. `email-label` — apply Gmail labels, mark processed, optionally archive
4. `inbox-triage` (r2v2) — extract action items → TickTick
5. `r2-remember` — commit thread content → Anansi

`email-reply` can be invoked at any point for threads that need a drafted response.

## Required connectors

- **Gmail MCP** (`mcp__297b8e49-d6fc-4f9c-9e47-e6d94d68aaa3`) — all three
  skills require this connector to be active.
