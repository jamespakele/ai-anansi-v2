# Example 01 — email thread

A short email mentioning one person, one organization, and one referenced spec. The example shows the deterministic mapping the skill produces: three Resource blocks, three `4.x` lines, two concept tags.

## Files

- `input.md` — the source document.
- `resources-typed.md` — the typed Resources output (file 1).
- `resources-toc.md` — the Section 3 / Section 4 / Concepts output (file 2).

## What this example demonstrates

- **Person test fires on Sarah Chen.** First+last name shape, action verb (`Just got off a call`), affiliation (`at Anthropic`). High confidence.
- **Organization test fires on Anthropic.** Capitalized proper noun, institutional framing (`at Anthropic`), possessive (`their MCP rollout`). High confidence.
- **Note fallback fires on the Claude Skills Spec.** Substantial named subject; not a person; not an organization. Disambiguator suffix `(spec)` applied per `references/naming-conventions.md`. Medium confidence — single mention.
- **`MCP rollout` is dropped as a Resource and emitted as a concept tag** (`#mcp`) — it's mentioned in passing as a topic, not framed as a thing the user is tracking as its own page. Same for `Claude Skills` as a concept (`#claude-skills`) since it's invoked alongside the spec.
- **`next week's review` is skipped.** Unnamed and ambiguous between Resource and Area without further signal. The skill stays conservative.
- **No Section 1 or Section 2.** Those belong to `para-projects-areas`. This skill never writes them.

## What `para-projects-areas` would emit on the same input (for reference)

The parallel skill would produce its own pair of files. Sections 1 and 2 in its TOC output would likely be empty here — the email contains no first-person ownership signals, no project outcomes with deadlines, no standards being upheld. Both halves merge cleanly at `para-toc`.
