# Gmail Adapter Reference

## MCP Tool Names

All Gmail tools are on server `mcp__297b8e49-d6fc-4f9c-9e47-e6d94d68aaa3`.

| Tool | Purpose |
|------|---------|
| `search_threads` | Search Gmail threads by query string (Gmail search syntax) |
| `get_thread` | Fetch a full thread by thread ID |
| `create_draft` | Create a draft reply or new email |
| `list_drafts` | List existing drafts |
| `list_labels` | List all Gmail labels |
| `create_label` | Create a new Gmail label |

## Gmail Search Syntax Quick Reference

| Query | Meaning |
|-------|---------|
| `is:unread` | Unread messages |
| `is:inbox` | In inbox |
| `label:LABEL` | Has this label |
| `after:YYYY/MM/DD` | After this date |
| `from:email@domain.com` | From sender |
| `subject:"text"` | Subject contains |
| `-label:email-manager/processed` | Exclude processed |

## Context Labels

After triage, apply these Gmail labels to route email by Pakele-OS context.
These are top-level labels — no `email-manager/` prefix. Use `list_labels` to
check existence; use `create_label` if missing.

| Context | Gmail Label | Notes |
|---------|-------------|-------|
| me | `personal` | Personal, family, recreation |
| dcs | `dcs` | DCS/POW nonprofit work |
| iq | `iq` | iQ360/JERA consulting |
| ai | `ai` | Pakele.ai / AI product work |
| anykine | `anykine` | Civic, community, ConCon, Kahoolawe |
| calendar | `calendar` | Calendar invites, scheduling, event confirmations — filter before triage |
| processed | `email-manager/processed` | Applied to every triaged thread; used by inbox-pull to exclude re-pulls |
| needs-reply | `email-manager/needs-reply` | Thread needs a drafted reply |
| archived | Archive (remove `\Inbox` label) | |

The `email-manager/processed` and `email-manager/needs-reply` labels use the
`email-manager/` prefix to keep operational metadata separate from content labels.
The five context labels (`personal`, `dcs`, `iq`, `ai`, `anykine`) and `calendar`
are flat top-level labels.

## Pakele-OS Context Inference for Email

| Signals | Context |
|---------|---------|
| DCS, POW, veteran, nonprofit, 501c3 | `dcs` |
| iQ360, JERA, client work, consulting, invoice | `iq` |
| Pakele.ai, AI product, Claude, plugin, Cowork | `ai` |
| ConCon, Kahoolawe, civic, community, Hawaii policy | `anykine` |
| Personal, family, health, recreation, finance | `me` |

When a thread spans multiple contexts, pick the dominant one. If unclear, surface it to the user.

## Voice / Tone by Context

| Context | Voice |
|---------|-------|
| `me` | Warm, informal, personal |
| `dcs` | Respectful, mission-driven, clear |
| `iq` | Professional, concise, client-facing |
| `ai` | Collaborative, technical, direct |
| `anykine` | Civic, community-minded, accessible |
