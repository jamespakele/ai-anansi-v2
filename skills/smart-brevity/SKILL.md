---
name: smart-brevity
description: >
  Rewrite any input text into Smart Brevity style (Schwartz/Allen/VandeHei,
  2022). Identifies the input type — email, email thread, newsletter,
  workplace memo, meeting agenda/notes, speech outline, presentation deck,
  social media post, company-wide update, or generic prose — and applies
  matching type-specific rules layered over universal Smart Brevity
  principles. Returns the rewritten text plus a word count and reading-time
  marker. Use when the user pastes text and says "smart brevity this",
  "/brevity", "rewrite in smart brevity", "tighten this up", "make this
  punchier", "apply smart brevity rules", or any request to compress prose
  into Smart Brevity form. Also handles TOC atomization mode: when the user
  provides both a source document and a PARA TOC (with decimal-addressed
  sections and bracket type tags like [architecture], [note], [person]),
  generates one Smart Brevity block per TOC section, labeled by address and
  separated by --- for mechanical parsing into vault notes. Every rule
  traces back to the book.
---

# smart-brevity

A Smart Brevity rewriter. Takes any input text, identifies what kind of communication it is, and rewrites it strictly compliant with the rules from *Smart Brevity* by Schwartz, Allen, and VandeHei (full text at `resources/smart-brevity.txt`).

The plugin's distilled rules live in `references/`. **Always load `general.md` and `inclusive-overlay.md`** plus the file matching the input type. In TOC atomization mode, load `general.md` and `cheat-sheet.md` only.

> "Brevity is confidence. Length is fear."

---

## When to invoke

Trigger this skill when the user:

- Pastes text and asks to rewrite in Smart Brevity style
- Says "smart brevity this", "/brevity", "tighten this", "make this punchier", "compress this"
- Asks to apply the Smart Brevity framework / rules / format to anything
- Says "shorten this without losing meaning"
- Provides a source document **and** a PARA TOC and asks to atomize, extract, or generate vault notes — this triggers **TOC Atomization Mode**

Do not invoke for: requests to *learn about* Smart Brevity (point at the book), summarizing without the Smart Brevity skeleton, or copyediting prose that's already short and punchy.

---

## Process

### Step 0 — Detect mode

**Standard mode** — A single document, email, memo, or prose. No TOC present. Go to Step 1.

**TOC Atomization Mode** — Input contains both a source document AND a PARA TOC (decimal-addressed sections with bracket type tags like `[architecture]`, `[note]`, `[person]`). Skip Steps 1–6 and go to **TOC Atomization Mode** below.

---

### Step 1 — Identify the type (standard mode)

| Type | Signals |
|---|---|
| **email** | Subject:, salutation, sign-off, one or a few recipients |
| **email_thread** | Multiple stacked emails, "On [date] X wrote:", forwarding chains |
| **newsletter** | Multiple ranked items, recurring publication name, reader-subscriber framing |
| **workplace_memo** | Internal team comms; "team," "we're rolling out," "all hands"; not recurring |
| **meeting_agenda** | Agenda, numbered items, "we'll discuss" — *before* the meeting |
| **meeting_notes** | Decisions, action items, attendees, past-tense recap — *after* the meeting |
| **speech** | Monologue framing, intended for live delivery |
| **presentation** | Slide labels, bulleted-deck shape, "our ask is" framing |
| **social_media** | Very short, hashtag use, platform mention, informal hook |
| **company_update** | Leader to whole org, weekly cadence, "this week at the company" |
| **generic_prose** | Doesn't fit anything above |

When ambiguous, prefer the more specific type. Trust the user if they indicate the type.

### Step 2 — Load rules

From `references/`:

1. `cheat-sheet.md` — universal checklist
2. `general.md` — the foundation
3. Type-specific file:
   - `email.md` — email, email_thread
   - `newsletter.md` — newsletter
   - `workplace-memo.md` — workplace_memo
   - `meeting.md` — meeting_agenda, meeting_notes
   - `speech.md` — speech
   - `presentation.md` — presentation
   - `social-media.md` — social_media
   - `company-update.md` — company_update (also load `newsletter.md`)
4. `inclusive-overlay.md` — always

If `generic_prose`: load only `general.md` + `inclusive-overlay.md` + `cheat-sheet.md`.

For edge cases not covered by the reference files, consult `resources/smart-brevity.txt`.

### Step 3 — Audience-first analysis

- **Who is the audience?** Read the input for clues; ask if unclear.
- **One Big Thing?** One sentence the reader must walk away with.
- **Tease?** ≤6 words. Strong verb. Concrete.

### Step 4 — Rewrite

1. Drop into the appropriate Smart Brevity skeleton.
2. Sharpen the tease — ≤6 words, active verb.
3. Sharpen the first sentence — one sentence, the takeaway.
4. Add the bolded axiom — "Why it matters:" 1–2 sentences; adds perspective, doesn't repeat.
5. Bullets for 3+ supporting points — bold the lead term in each.
6. Strip ruthlessly — cut adverbs, qualifiers, throat-clearing, jargon.
7. "Go deeper" if the input had links or citations.
8. Apply inclusive overlay — drop irrelevant identity descriptors.

### Step 5 — Reading-time marker

If the rewrite is more than ~100 words, add at the top:

```
*<word count>, <minutes> minutes*
```

Use **265 words/minute**. Round to nearest 0.5 above 1 minute.

### Step 6 — Self-check

- [ ] Tease ≤ 6 words?
- [ ] First sentence is the takeaway?
- [ ] **Why it matters** bold and adds perspective?
- [ ] Bullets wherever 3+ related points exist?
- [ ] Bold marks key names / figures / dates?
- [ ] No adverbs, qualifiers, throat-clearing, SAT words?
- [ ] Reads aloud without gasping?

---

## TOC Atomization Mode

When a PARA TOC accompanies a source document, the job shifts: generate **one Smart Brevity block per decimal-addressed TOC section**, structured so a parser can split them into individual vault notes on `---` without another LLM call.

Load `references/general.md` and `references/cheat-sheet.md`. Type-specific files aren't needed — every block uses one of the two capsule shapes below.

### Reading the TOC structure

The TOC's formatting signals which block shape to use:

**Lettered sub-entries → Shape A (sectioned block)**

Each lettered entry (`3.2.a`, `3.2.b`) becomes a `##` sub-section. Bullets under a lettered entry are content seeds for that sub-section.

```
3.2 Core Architecture [architecture]
  3.2.a The AIAgent orchestration engine: five phases...
  3.2.b Skill codification: ephemeral trajectories → SKILL.md...
```

**Inline bullet facts → Shape B (capsule block)**

Semicolon-delimited items are facts to compress into the bullet list. No internal sections.

```
4.16 Hermes Agent [note]
  - Open-source autonomous agent runtime; MIT license; Nous Research; Feb 2026...
```

The bracket tag (`[architecture]`, `[note]`, `[person]`, `[organization]`, etc.) is the template selector — carry it into the block header unchanged. The parser uses it to choose the vault template.

---

### Shape A — Sectioned block

Use when lettered sub-entries are present (Section 3 discussion entries).

```
### [address] [title] [type-tag]
One lead sentence — the single most important thing this section establishes.

**Why it matters:** One sentence of context or consequence. Does not repeat the lead.

## [label from lettered entry a]
One tight sentence on this sub-topic.
- **Key term:** supporting fact
- **Key term:** supporting fact

## [label from lettered entry b]
One tight sentence.
- **Key term:** supporting fact

---
```

Rules:
- Lead sentence covers the section's thesis, not any single sub-topic
- Each `##` maps 1:1 to a lettered TOC entry — don't merge or split
- Bullets follow Smart Brevity rules: bold lead term, cut qualifiers
- `---` is the parser's split token — always include it

---

### Shape B — Capsule block

Use for Section 4 resource entries (`[person]`, `[organization]`, `[note]`) and any flat decimal entry without lettered sub-entries.

```
### [address] [title] [type-tag]
One lead sentence — the single most important fact about what this entity is.

**Why it matters:** One sentence on relevance or relationship. Omit if self-evident.

- **Key term:** fact
- **Key term:** fact
- **Key term:** fact

---
```

Rules:
- Lead answers "what is this?" in one punch
- 3–5 bullets from the TOC's semicolon-delimited facts
- Drop `**Why it matters:**` if the entity is self-evident (well-known org, obvious relevance)
- `---` always present

---

### Extraction against the source

The TOC bullets are the already-extracted facts — the primary content source. You don't need to re-read the entire source document for every block. Use the source only to:
- Fill in a specific detail too sparse in the TOC
- Resolve a lettered sub-entry that needs a precise quote, number, or date

The TOC did the extraction pass. Smart-brevity does the compression pass.

---

### Output format

Emit all blocks as one continuous document, `---` separated. No preamble, no wrapper text — the output IS the parseable block set.

Open with a comment header:

```
<!-- smart-brevity atomization: [source title] | [N] blocks | [date] -->
```

Emit blocks in TOC order. **Section 5 concepts** do not become blocks — emit as a closing tag line:

```
<!-- concepts: #tag1 #tag2 #tag3 ... -->
```

---

### Self-check for atomization mode

- [ ] Every decimal TOC entry has exactly one block
- [ ] Shape A for lettered sub-entries; Shape B for flat entries
- [ ] Every block ends with `---`
- [ ] Each block's lead sentence is standalone — not dependent on surrounding blocks
- [ ] No cross-block references ("as noted in 3.2..." — cut it)
- [ ] Section 5 concepts emitted as closing tag line, not as blocks
- [ ] Comment header present

---

## Output format (standard mode)

Return the rewrite **as it would appear in its native medium** — not in a code block, not annotated.

Then optionally, separated by `---`:

```
---

**Type:** <identified type>
**Length:** <before> → <after> words (<reduction>%)
**Notes:** <one-line callouts if useful>
```

Skip this block if the rewrite is short and self-evident.

---

## Worked examples

### Example 1 — Email (standard mode)

**Input:** A 168-word email from Sarah asking James to approve Option B for Q3 workstream sequencing.

**Output:**
```
Subject: Q3 sequencing — go with Option B

Recommending Option B: ship auth refactor in Q3, dashboard in Q4.

**Why it matters:** Auth is blocking three teams; dashboard has no downstream dependencies. Sequencing this way unblocks the most work.

- **Auth refactor (Q3):** unblocks payments, mobile, and partner teams.
- **Dashboard (Q4):** isolated; can ship later with no cascading delay.
- One-pager attached with detailed analysis.

Good with this? Happy to take a 15-minute call if it's faster.

— Sarah

---

**Type:** email
**Length:** 168 → 78 words (54% reduction)
```

---

### Example 2 — TOC Atomization Mode

**Input:** Hermes Agent analysis (source) + PARA TOC excerpt:
```
3.2 Core Architecture [architecture]
  3.2.a The AIAgent orchestration engine: five phases — execute, evaluate, extract, refine, retrieve
  3.2.b Skill codification: ephemeral trajectories → SKILL.md on local disk
  3.2.c TokenMix benchmark: ~40% task-time reduction on recurring workflows

4.16 Hermes Agent [note]
  - Primary subject; open-source autonomous agent runtime; closed learning loop; four-layer memory; MIT license; Nous Research; Feb 2026; 125K GitHub stars; Python
```

**Output:**
```
<!-- smart-brevity atomization: Hermes Agent Analysis | 2 blocks | 2026-04-29 -->

### 3.2 Core Architecture [architecture]
Hermes Agent's closed learning loop converts every completed task into reusable procedural memory.

**Why it matters:** This is what separates Hermes from stateless agents — expertise compounds over time without manual intervention.

## The AIAgent orchestration engine
Five sequential phases run after every complex session: execute → evaluate → extract → refine → retrieve.
- **Trigger:** sessions requiring 5+ discrete tool invocations
- **Output:** reusable operational patterns extracted from the execution trajectory

## Skill codification
Extracted knowledge writes to a physical SKILL.md file on local disk — human-readable and Git-committable.
- **Format:** YAML frontmatter + Markdown body
- **Advantage:** fully transparent; operators can read, edit, or delete any skill

## TokenMix benchmark
Procedural retrieval cuts task time ~40% on recurring workflows.
- **Mechanism:** agent retrieves the skill file instead of reasoning from scratch
- **Compounding:** the loop refines the skill on each encounter

---

### 4.16 Hermes Agent [note]
Hermes Agent is an open-source autonomous agent runtime built for persistent, unattended server deployment.

**Why it matters:** Released Feb 2026 under MIT by Nous Research — the lab behind the Hermes model family — making it production-ready and freely forkable.

- **Architecture:** closed learning loop; four-layer memory (procedural, episodic, user model, archival)
- **Stack:** Python; 125K GitHub stars
- **Positioning:** depth-of-learning over breadth of integrations

---

<!-- concepts: #closed-learning-loop #procedural-memory #cache-aware-execution -->
```

---

## Edge cases

### "TOC section with no source content"

Write from TOC bullets alone. Don't fabricate. If too sparse to write a lead: `<!-- sparse: [address] — minimal source content; expand manually -->`.

### "Section has both lettered sub-entries AND inline bullets"

Use Shape A. Inline bullets feed the lead or `**Why it matters:**` line, not additional `##` sections.

### "Input is already short and tight" (standard)

Don't bloat. Confirm compliance or suggest minor tightening.

### "Input is many pages long" (standard)

You're rewriting, not summarizing. One Big Thing → ~200 words. Cut everything that doesn't serve it.

### "Rewrite loses important nuance" (standard)

Smart Brevity is short, **not shallow**. Restore caveats as a final bullet: "**Caveat:** [tight version]".

---

## What this skill does NOT do

- Does not edit prose stylistically without applying the Smart Brevity skeleton.
- Does not summarize when losing detail is the goal.
- Does not invent facts — if a number or date is missing, ask.
- Does not replace anansi ingestion logic — in atomization mode, the output is a block set for parsing. Vault writes, edge creation, and template instantiation are downstream.

---

## Why this matters

The book's core claim: in a world of overwhelmed readers, the shortest, sharpest version wins.

In atomization mode, the same principle applies at the vault level — every note should earn its place in one punchy lead sentence. If you can't write the lead, the note isn't ready to exist.
