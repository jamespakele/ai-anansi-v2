---
name: sb-compress
description: |
  Rewrite any input text into Smart Brevity style (Schwartz/Allen/VandeHei,
  2022). Detects the input's type — email, email thread, newsletter,
  workplace memo, meeting agenda or notes, speech, presentation, social
  media post, company-wide update, book chapter, or generic prose — and
  applies matching type-specific rules layered over the universal Core 4
  (tease, lede, axiom, go deeper). Closes every compression with a Varys
  pass: a final, conservative scan for whispers the compression had to
  leave behind — signals, absences, patterns, orphans, nuance, decisions,
  corrections. Use when the user says "smart brevity this", "/brevity",
  "/sb", "/sb-compress", "/sb-distill", "rewrite in smart brevity",
  "compress this", "distill this", "tighten this up", "make this punchier",
  "apply smart brevity rules", "sb compress", "sb distill", or asks to
  compress or distill any pasted text into Smart Brevity form.
---

# sb-compress

A Smart Brevity rewriter. Take any input — email, memo, newsletter, speech, slide deck, post, prose — identify what kind of communication it is, and rewrite it under the rules of *Smart Brevity* by Schwartz, Allen, and VandeHei (Workman Publishing, 2022). Then run a Varys pass — a final read of the source for what the compression had to leave behind.

> "Brevity is confidence. Length is fear."

This skill does **single-document compression only**. It does not atomize content into per-entity vault notes, generate TOCs, or shard output for ingestion. For those, see `plugins/anansi-para.plugin/skills/smart-brevity-atomize/` and the rest of the `anansi-para` pipeline.

The skill's distilled rules live in `references/` and ship inside the `.skill` package. For every rewrite, **always load `general.md` and `inclusive-overlay.md`**, plus the file matching the input type. The references are the operational rule set — every rule traces back to the book, but the skill does not load the book at runtime.

---

## When to invoke

Trigger this skill when the user:

- Pastes text and asks to rewrite it in Smart Brevity style
- Slash commands: `/brevity`, `/sb`, `/sb-compress`, `/sb-distill` (kebab-case — the dot-form `/sb.compress` does not register as a slash command)
- Says "smart brevity this", "sb compress", "sb distill"
- Uses the verbs "compress" or "distill" against a piece of writing — e.g., "compress this", "distill this email", "compress and tighten"
- Says "tighten this up", "make this punchier", "shorten without losing meaning"
- Asks to apply the Smart Brevity rules / framework / format to a piece of writing
- Hands you a draft and asks for the Smart Brevity version

## When NOT to invoke

- They want to *learn about* Smart Brevity → point them at the book itself (Schwartz/Allen/VandeHei, Workman, 2022) or `references/cheat-sheet.md`.
- They want a plain summary without the Smart Brevity skeleton → use a generic summarizer.
- The text is already short and punchy and they want copyediting → do that, don't compress.
- They want per-entity atomization for the Anansi vault → use `smart-brevity-atomize`.

---

## The four guiding principles

Every rewrite has to clear all four. None is optional.

- **Authority.** Write as a trustworthy source. Distill, don't speculate. If you don't know something, don't fake it.
- **Brevity.** Respect the reader's time. Stand out by being short, not shallow. Every word earns its place.
- **Humanity.** Write like you speak. Conversational, with real emotion. Not a Broadway character, not a corporate spokesperson.
- **Clarity.** Frugal with words. Style for impact and scan. Subject. Verb. Object.

---

## The Core 4 skeleton

Every Smart Brevity piece — regardless of type — uses this structure, in this order:

1. **Muscular tease** — headline, subject line, or opening hook. **6 words or fewer.** Strong, specific, active verbs. No cleverness, no irony, no SAT words.
2. **Strong first sentence (lede)** — ONE sentence. The single thing the reader must remember. Distinct from the tease — don't just repeat it.
3. **"Why it matters" axiom** — bolded signpost + 1–2 sentences. Tells the reader why this is significant *for them*. Adds perspective; does NOT repeat the lede.
4. **"Go deeper"** — optional. Links / references / supporting detail for those who want more. Says: *I've done the work so you don't have to.*

Try to fit the whole thing **on one phone screen**, regardless of where it'll be read.

---

## Process

Slow is smooth, smooth is fast. Every cut word is a deliberate decision.

### Step 1 — Identify the input type

Read the input and pick the type from the table below. When ambiguous, prefer the more specific type. Trust the user if they indicate the type in their request.

| Type | Signals |
|---|---|
| **email** | Subject:, salutation, sign-off, one or a few named recipients |
| **email_thread** | Multiple stacked emails, "On [date] X wrote:", forwarding chains |
| **newsletter** | Multiple ranked items, recurring publication name, reader-subscriber framing |
| **workplace_memo** | Internal team comms; "team," "we're rolling out," "all hands"; not recurring |
| **meeting_agenda** | Numbered items, "we'll discuss" — *before* the meeting |
| **meeting_notes** | Decisions, action items, attendees, past-tense recap — *after* the meeting |
| **speech** | Monologue framing, intended for live delivery, a specific audience |
| **presentation** | Slide labels, bulleted-deck shape, "our ask is" framing |
| **social_media** | Very short, hashtag use, platform mention, informal hook |
| **company_update** | Leader to whole org, weekly cadence, "this week at the company" |
| **book** | Published book or long-form nonfiction with named chapters/sections |
| **generic_prose** | Doesn't fit anything above |

### Step 2 — Audience-first analysis

Before writing one word:

- **Picture one specific person.** Name, face, job title. Not "everyone." Not "the team." A specific person.
- **What's the One Big Thing they need to remember?** One sentence. If you can't write it in one sentence, the piece isn't ready.
- **Why is it significant *to them?*** This is the axiom's job — answer it now, not later.
- **Tease?** ≤6 words. Strong verb. Concrete. Picturable.

### Step 3 — Load the rules

From `references/`:

1. `cheat-sheet.md` — the universal one-pager.
2. `general.md` — the foundation. Always load.
3. The matching type-specific file:
   - `email.md` — for `email` and `email_thread`
   - `newsletter.md` — for `newsletter`
   - `workplace-memo.md` — for `workplace_memo`
   - `meeting.md` — for `meeting_agenda` and `meeting_notes`
   - `speech.md` — for `speech`
   - `presentation.md` — for `presentation`
   - `social-media.md` — for `social_media`
   - `company-update.md` — for `company_update` (also load `newsletter.md`)
   - `book.md` — for `book`
4. `inclusive-overlay.md` — always.

For `generic_prose`: load `general.md` + `cheat-sheet.md` + `inclusive-overlay.md` only.

If the references don't cover an edge case, infer from the four guiding principles (Authority, Brevity, Humanity, Clarity) and the Core 4 skeleton. The references are not exhaustive — the principles are.

### Step 4 — Rewrite into the Core 4 skeleton

1. **Drop the input into the type's skeleton.** Don't fight the format — let the structure do the work.
2. **Sharpen the tease.** ≤6 words, active verb, concrete.
3. **Sharpen the lede.** One sentence, the One Big Thing, distinct from the tease.
4. **Bold the axiom.** "Why it matters:" or a brand-fitting alternative. 1–2 sentences. Adds perspective; never repeats the lede.
5. **Bullets wherever 3+ related points exist.** Bold the lead term. 1–2 sentences per bullet, max.
6. **Strip ruthlessly.** Cut adverbs, foggy qualifiers, throat-clearing, jargon, SAT words, repetition.
7. **"Go deeper" line** if the input had links, citations, or natural rabbit-hole material.
8. **Apply the inclusive overlay.** Drop irrelevant identity descriptors. Plain words. Bullets force clarity.

### Step 5 — Add the reading-time marker (if length warrants)

If the rewrite is more than ~100 words, add at the top:

```
*<word count> words, <minutes> minutes*
```

Use **265 words/minute** as the reading-speed assumption. Round to the nearest 0.5 above 1 minute. Under 100 words, you can skip the marker — the cost is self-evident.

### Step 6 — Self-check (universal checklist)

Run the rewrite against this list before moving to the Varys pass:

- [ ] Tease ≤ 6 words, active verb, concrete?
- [ ] First sentence stands alone as the takeaway?
- [ ] **Why it matters** (or named axiom) bolded, 1–2 sentences, adds perspective?
- [ ] One Big Thing identifiable in one sentence?
- [ ] Bullets used wherever 3+ related points exist?
- [ ] Bold marks key names / figures / dates?
- [ ] No adverbs, no foggy qualifiers, no throat-clearing?
- [ ] No SAT words, no journalese, no corporate jargon?
- [ ] Reading-time marker at top (if length warrants)?
- [ ] Reads aloud without gasping?
- [ ] A friend could repeat the One Big Thing back after one read?

If any check fails, fix it before moving on.

### Step 7 — Run the Varys pass (when applicable)

This is the final step on source-material compressions. The pass is **skipped** when the input is a draft you're authoring or a short fragment. See the **When the Varys pass applies** subsection inside the Varys pass section below for the gating rules.

### Step 8 — Compose the output

Assemble in this order (see **Output format** below for full template):

1. The compressed text in its native medium (email-shaped, slide-shaped, etc.)
2. `---`
3. The compression metadata block (Type, Length, optional Notes)
4. **If Varys applied** (per Step 7): `---` and then the Varys block (whispers, or "Pass clean — no whispers.")
5. **If Varys was skipped:** end here. Do not emit a Varys block, not even a "Skipped" line — silence is the signal.

---

## Universal Smart Brevity rules

These apply on every rewrite. Type-specific rules layer on top.

### Tease / headline / subject line

- **6 words maximum.** Hard limit. Memorize it.
- **Active verbs only.** No passive voice.
- **Concrete, not clever.** No irony, no jokes, no riddles.
- **One-syllable words beat two beats three.** Punch over polish.
- **Newsy, accurate, urgent.** Why open this NOW?

Examples (bad → good):
- "The coronavirus variant in California is possibly more infectious" → "California COVID strain is more infectious"
- "Following up on our discussion regarding..." → "Q3 budget: decision needed"
- "Quick question" → "Need your sign-off on contract"
- "Just checking in" → "Status update on Acme deal"

### Lede / first sentence

- **One sentence.** Period.
- **Lead with the news or ask.** No preamble. No throat-clearing.
- **Distinct from the tease.** Don't just expand the headline; say the next thing.
- **The elevator test:** if they're running out the door, what's the one thing you'd shout?

Bad opening: *"Hi James, I hope you're doing well. I wanted to reach out following our conversation last Tuesday..."*
Good opening: *"Need your sign-off on the Acme contract by Friday — terms are below."*

### "Why it matters" — the bolded axiom

Bold a signpost phrase, then write 1–2 sentences of context.

**Common axioms (always bold them):**

- **Why it matters** — most flexible default
- **The big picture** — strategic context
- **By the numbers** — when leading with data
- **What's next** — forward-looking
- **The bottom line** — decisional summary
- **Reality check** — corrects a misconception
- **Between the lines** — hidden meaning
- **Catch up quick** — context for those joining late
- **Zoom in / Zoom out** — focus level shifts
- **The other side** — counterargument

Rules:
- The axiom **must be bold**. Italics are weaker.
- 1 sentence ideally; 2 maximum.
- **Adds perspective.** Never repeats the lede.
- Answers one of: *what changes? what does it signal? what's the larger context?*

Brand voice can invent its own axioms. Keep them sharp and reusable.

### One Big Thing

- **Exactly one** memorable point per piece. No exceptions.
- One sentence.
- The elevator test: if they only see this, is it what you want them to remember?
- After writing, distill: would you yell this at someone running out the door? If not, sharpen.
- Test with a friend: read it aloud, ask them to repeat the One Big Thing back. If they can't, the rewrite isn't done.

### Bullets

Bullets exist to break dense text and let the eye scan. Use them whenever you have:

- **3 or more related data points / supporting ideas**
- A list, sequence, or checklist
- Anything that would otherwise be a "blob of text"

Format rules:
- **1–2 sentences per bullet, max.**
- Verb-first when possible.
- **Bold the lead word/phrase** if there's a key term — eye-trap for skimmers.
- No clumps. Respect rhythm and white space.

Bullets force clarity (Ch. 22): when you separate ideas into bullets, you have to decide what each idea actually IS. The dyslexic reader, the ESL reader, the cognitively overloaded reader — and everyone, sometimes — track better.

### Bold in body

Bold isn't decoration. Use it deliberately to:

- Mark the **axiom** (Why it matters, etc.)
- Highlight **key names, figures, dates, or terms** the skimmer needs to catch
- Signal where attention should land

Italics are weaker than bold. When in doubt, bold.

### Sentence structure

- **Subject. Verb. Object.** Linear, not twisty.
- **Short sentences beat long ones.** If you're gasping for air reading aloud, cut.
- **No nested clauses.** Multiple commas hide meaning.
- **One idea per sentence.** If you say "and" twice, split it.

### Word choice — what to keep

- **Strong, vivid, picturable nouns:** fire, boat, cliff, fish, room, bridge, knife.
- **Active, muscular verbs:** chop, crush, sell, hit, taunt, botch, ship, pull.
- **Plain English** anyone could understand at a bar.
- **One-syllable words** when possible. Then two. Avoid three+.

### Word choice — what to kill

**Adverbs (almost always cut):**
very, really, quite, somewhat, rather, actually, basically, just, literally

**Foggy qualifiers (commit or cut):**
could, may, might, possibly, potentially → say "planned," "considered," "discussed," "feared," "expected," or just *what is*.

**Throat-clearing:**
- "I know you're busy but..."
- "Sorry to bother you..."
- "Following up on..."
- "I just wanted to..."
- "I hope you're doing well..."

**SAT words / journalese (left → right):**
- vociferous → vocal
- prevaricate → lie
- conundrum → jam
- salient → on-point
- ubiquitous → everywhere
- discourse → talk
- challenge → problem
- posit → assume
- dearth → lack
- disseminate → spread
- elucidate → explain
- utilize → use
- commence → start
- terminate → end

**Corporate jargon:**
- price point → price
- core competency → skill
- circle back → talk again
- touch base → check in
- bandwidth → time / capacity
- leverage → use
- synergy → fit / overlap
- moving forward → next
- deep dive → look closer
- low-hanging fruit → easy wins

**The rule:** if you can say it in one fewer syllable, do it. If you wouldn't say it at a bar, don't write it.

### Emojis (Ch. 11)

Used sparingly, an emoji can replace twenty words and signal tone instantly.

- **One per piece is plenty.** Rarely two.
- **Tone signal**, not decoration.
- **Shorthand label** for content type — 🚨 breaking, 📊 data, 🔥 hot, ⏰ deadline, 📈 up, 📉 down, ✅ done, 💡 idea, 🎯 precise, 💣 major news, ⚡ urgent.

Don't pile them up. One signal per post.

### What to cut ruthlessly

Before declaring the rewrite done, strip:

1. Every adverb you can lose without changing meaning.
2. Every foggy qualifier ("could," "may," "might"). Commit or cut.
3. Every throat-clearing phrase.
4. Every apology and over-explanation.
5. Every backstory paragraph the reader doesn't need NOW.
6. Every repetition of a point already made.
7. Every word that adds no information.
8. Every fancy synonym used to sound smart.

> If a sentence can be cut without losing meaning, cut it. If a word can be cut without losing meaning, cut it. If a paragraph can be cut without losing meaning, cut it.

### The two-second test

Readers decide in milliseconds. Plan accordingly.

- **17 milliseconds:** brain decides if it likes what you clicked.
- **26 seconds:** average time spent on a piece of content.
- **80%:** of readers stop after page 1 of long content.

The first 6 words and the first sentence are 80% of the work. Front-load.

---

## Type-specific rules — quick reference

The hard limits and shape of each type. **Always read the matching reference file** for the full rules and worked examples; this table is the cheat sheet, not the rulebook.

| Type | Skeleton | Hard limits |
|---|---|---|
| **email** | Subject (≤6w) → lede → **Why it matters** → bullets → sign-off | <200 words; ONE point per email |
| **email_thread** | Same as email, but lead with the resolution / latest news | <200 words; cut the entire prior chain that doesn't change the takeaway |
| **newsletter** | Name → reading marker → "1 big thing" → numbered items → "1 fun thing" | 5–10 items; ≤200 words/item; <1,000 words total |
| **workplace_memo** | Title (≤6w) → reading marker → lede → axiom-headed sections → bullets | <500 words; weekly cadence works best |
| **meeting_agenda** | Subject → **Objective** → **Why it matters** → ≤3 agenda items → decisions needed → pre-read | ≤3 agenda items; objective in 1 sentence; pre-read = 1 page or 1 link |
| **meeting_notes** | Subject → **Decision** → **Why it matters** → what was decided → action items → open questions | <300 words; bold the owner; date everything; same-day send |
| **speech** | Hook → status quo → contrast → Big Thought (≤15 words) → why it matters → 3–5 numbered points → CTA → reinforce → close | ≤18 minutes (TED standard); Big Thought repeats twice |
| **presentation** | Outcome sentence → 5–12 slides, one message each, ≤20 words/slide → final slide = the ask | ≤12 slides; ≤20 words/slide; "I built this so I can get ___" |
| **social_media** | Image (when possible) → 6-word punch → ≤platform-cap | Twitter ≤200 chars; IG ≤150w; FB ≤100w; LinkedIn ≤200w |
| **company_update** | Newsletter from a leader; mission tie-in at least once/week; "1 fun thing" close | Same as newsletter; same day, same time, every week |
| **book** | Per-chapter blocks: tease → lede → **Why it matters** → 3–5 bullets → optional closing axiom | One block per chapter; never drop a chapter; ≤6-word teases |
| **generic_prose** | Tease → lede → axiom → bullets/sections → optional go-deeper | <500 words target; whatever shape the content needs |

For full per-type rules, examples, and common-mistake tables: open the matching `references/<type>.md`.

---

## Inclusive overlay (always on)

Layered onto every rewrite. The principle: *if you're not communicating inclusively, you're not communicating effectively* (Ch. 22). Smart Brevity is naturally accessible — short sentences, plain words, structured layouts. The overlay catches what the structure alone doesn't.

- **Plain, clear language.** Helps the 65 million Americans with learning disabilities, non-native English readers, and the cognitively overloaded — i.e., everyone, sometimes.
- **Bullets force clarity.** Three points → three bullets. Translation tools handle it. Dyslexic readers track it.
- **Be specific when writing about someone.** Ask how they identify. Name the tribe. Confirm pronouns. Don't generalize across regions or ethnicities when the specific is available.
- **The swap test.** "The Black executive" — would you write "the white executive" in the same place? If not, the descriptor isn't doing work; cut it.
- **Omit irrelevant identity.** Race, ethnicity, religion, national origin, disability — only include if directly relevant. Otherwise, cut.
- **Disability references** only when relevant. Confirm with the person. Avoid medical-model framing unless they use it themselves.

The goal: **the inclusive version is the brief version.** They reinforce each other. If your "more inclusive" rewrite is longer and softer, you've gone wrong.

Full rules: `references/inclusive-overlay.md`.

---

## The Varys pass

Smart Brevity is, by design, reductive. The Core 4 strips away everything that isn't the One Big Thing. That's the feature. But anything reductive leaves things on the cutting-room floor — and sometimes, what's on the floor is exactly the thing that, six weeks later, would have changed the picture.

Varys catches that.

### Who Varys is

In Westeros, **Varys** is the Master of Whisperers — also called *the Spider*. He is not a swordsman, not a king, not a scholar. He is the man who hears what nobody writes down. He commands a network of "little birds" — informants, servants, children who clean tables and listen — across two continents. He is publicly underestimated. People take him for a soft, fawning courtier. In reality he is one of the most consequential operators in the realm, because he is the only one paying attention to the small things while everyone else watches the big ones.

Smart Brevity is the big things. Varys is the small ones. The compression is the official record. The Varys pass is the side-comments and pauses that the official record doesn't carry.

### Why Varys runs last

The Varys pass runs **after** the compression is complete and self-checked. Two reasons:

1. You can only see what's *missing* once you can see what's *present*. The compression has to exist first.
2. The compression is in your working memory. That makes the source's gaps visible — items that were in the source but didn't make it into the rewrite, *and shouldn't have* (the compression was right to cut them), but might still be worth a one-liner before they vanish.

### When the Varys pass applies

The Varys pass exists because every source document leaves things on the cutting-room floor. But not every input has a source. Some inputs *are* the draft. Run Varys against drafts and you're inventing whispers from your own writing — exactly the failure mode the "no invented facts" rule prohibits.

**Run Varys when the input is source material** (something with a separate origin you're distilling from):

- `email_thread`, `meeting_notes`, `meeting_agenda`, `book`, articles, transcripts, interviews, brain-dumps
- A *received* email being compressed (not one you're composing)
- Any `generic_prose` that's a transcript, recording, captured artifact
- Any input >300 words where there's room for buried signal

**Skip Varys when the input is a draft you're authoring or tightening** (the input itself is the artifact, no separate source to mine):

- `social_media` — always; the post is too short and is itself the draft
- Email *drafts* (you composing an outgoing message — no buried signal possible)
- Workplace memo / company update / presentation drafts being sharpened
- Speech drafts being tightened from your own outline
- Any input <150 words that's clearly your own writing
- Short paragraphs being polished

**When Varys is skipped, omit the section entirely.** No "Skipped" line, no empty header, no placeholder. The compression and its metadata are the deliverable. Silence is the signal.

**User overrides:**

- "with varys", "include varys", "run varys" → run the pass even on a draft.
- "no varys", "skip varys", "no whispers" → skip the pass even on source material.

**The single test for ambiguous cases:** *Is there a separate source — transcript, recording, document, conversation — that the compression is distilling from?* If yes, run Varys. If no (the input itself is the artifact being polished), skip.

**Verb hint.** "Compress" leans toward draft-tightening (often Varys-skip). "Distill" leans toward source-extraction (often Varys-on). Same skill, but the user's verb is a soft signal of intent.

### The single test

Re-read the source one more time, holding the compression in working memory, and ask exactly this:

> What in the source, if it turned out to be true and important, would change the whole picture — and didn't make it into the compression?

If a candidate answers yes to both halves, it's a whisper. If it answers no to either, drop it.

### Seven categories of whispers

| # | Category | What it is | Example |
|---|---|---|---|
| 1 | **signal** | Weak indicator — a pause, an offhand mention, a too-fast or too-slow response | *"She paused before answering the budget question."* |
| 2 | **absence** | What was conspicuously NOT said when it should have been | *"The proposal didn't mention pricing once."* |
| 3 | **pattern** | Recurring shape across the source forming an emergent picture | *"Three different participants apologized for not having an update."* |
| 4 | **orphan** | An item mentioned briefly but with potentially massive downstream impact, that didn't fit any thread | *"They casually mentioned the funder is reconsidering the program."* |
| 5 | **nuance** | Context about a person or org that will never appear in any document but changes how everything reads | *"She's technically the PM but real decisions go through her director."* |
| 6 | **decision** | A closed loop reached verbally with no entity to attach it to | *"Going with option B — budget was the deciding factor."* |
| 7 | **correction** | A statement implying something elsewhere is wrong or stale | *"Actually it's October, not November — we keep getting that wrong."* |

### What is NOT a whisper

A whisper is the **gap**. If it's not a gap, it's not a whisper.

- Anything already covered in the compression. Drop.
- Restatements of compression content from a different angle. Drop.
- Pure summary of the source. Not Varys's job. Drop.
- Facts you're inferring rather than observing in the source. If you can't quote it, you can't whisper it.
- Generic context the source is *about*. "This is a meeting about broadband" is the topic, not a whisper.
- Things you *want* to be true. Whisper has to be observable.

Rule of thumb: if it came through cleanly and got picked up in the compression, leave it. Varys captures the gap, not the redundancy.

### Whisper format (hard caps)

```
### Whisper {N} — {≤8-word title} [{category}]
**Lede:** {one sentence, ≤15 words. The whisper itself, exactly as observed.}
**Why it matters:** {one sentence, ≤20 words. What this could change.}
**Source quote:** "{exact words from source, if applicable — else omit this line entirely}"
```

Hard caps:
- Title ≤ 8 words.
- Lede ≤ 15 words.
- Why it matters ≤ 20 words.
- Total whisper body ≤ 80 words.
- One category tag, lowercased, in brackets — pick the dominant one.

If a whisper exceeds these caps, **it is not a whisper.** It is a thing the compression should have covered. Either restore it to the compression, or note it as a follow-up rather than emit it as a whisper. The caps are the signal.

### Stop criteria

Stop when no more candidates pass the "would change the picture" test. Be conservative.

- **Three sharp whispers beat thirty noisy ones.**
- **Zero whispers is a valid outcome.** Most clean compressions land here.
- More than five whispers from a single source is a flag — re-read what you've emitted; most won't survive a second pass.

### Worked Varys examples

**Example A — orphan game-changer**

The source was a 12-minute project status meeting. The compression captured the schedule, the new vendor selection, and the open risks. But buried in a side comment, the engineering lead said: *"And by the way, I'm probably going to want to talk through the team structure once we're past Q3."*

```
### Whisper 1 — Eng lead hinted at team change [orphan]
**Lede:** Eng lead floated a post-Q3 conversation about team structure.
**Why it matters:** Could be a reorg signal — worth a 1:1 before Q3 staffing decisions get baked in.
**Source quote:** "I'm probably going to want to talk through the team structure once we're past Q3."
```

**Example B — absence**

The source was a board update email summarizing the quarter. The compression captured the wins, the misses, and the asks. But the email never mentioned the lawsuit that was the dominant topic at last month's board meeting.

```
### Whisper 2 — Lawsuit not mentioned in board update [absence]
**Lede:** This quarter's board update never references the pending lawsuit.
**Why it matters:** Either the lawsuit resolved quietly, or someone decided to defer it — either way, ask before the next board meeting.
```

(No source quote line — the whisper is about what's not there.)

**Example C — pass clean**

The source was a 200-word internal memo announcing a new vacation policy. Compression was straightforward; the source had no pauses, asides, or buried items.

```
### Varys pass
Pass clean — no whispers.
```

That's a valid result. It tells the reader the pass was run.

---

## Output format

Return the rewrite **as it would appear in its native medium** — email-shaped if it's an email, slide outline if it's a deck, prose if it's prose. Not in a code block. Not annotated. Then the metadata. Then Varys (when it applies).

```
[Optional reading-time marker if length warrants — *N words, M minutes*]

[The compressed text in its native medium]

---

**Type:** <identified type>
**Length:** <before> → <after> words (<reduction>%)
**Notes:** <one-line callouts if useful — otherwise omit this line>

---

### Varys pass — whispers from the source

[One or more whisper blocks, OR the "Pass clean — no whispers." line]
```

The Varys block is present **only when the Varys pass applied** (see "When the Varys pass applies"). When Varys ran but found nothing, the "Pass clean — no whispers." line is the signal. When Varys was skipped (drafts, social posts, short fragments), the block is **omitted entirely** — no header, no skipped-line, just stop after the metadata.

For very short rewrites (e.g., a 4-line social media post), the metadata block can also be skipped if it would be longer than the rewrite itself.

---

## Worked examples

### Example 1 — Received email being compressed

**Input** (168 words — a received email from Sarah, James is compressing it for his records):

> Hi James,
>
> Hope you're doing well! I wanted to follow up on our conversation last Tuesday about the Q3 workstream sequencing. I've been thinking about it a lot since then, and I've also had a chance to talk with the auth team and the dashboard team to get their input on what they think the right path forward should be.
>
> After all of that, I think we should go with Option B — that is, ship the auth refactor in Q3 and push the dashboard work to Q4. The main reason is that the auth refactor is currently blocking three other teams (payments, mobile, and the partner integrations team), so getting it done sooner unblocks a lot of work. The dashboard, on the other hand, doesn't have any downstream dependencies, so it can wait without holding anything up.
>
> I've attached a one-pager with the more detailed analysis. Let me know if you have any questions or want to talk through it!
>
> Sarah

**Output:**

```
Subject: Q3 sequencing — go with Option B

Recommending Option B: ship auth refactor in Q3, dashboard in Q4.

**Why it matters:** Auth is blocking three teams; dashboard has no downstream dependencies. Sequencing this way unblocks the most work.

- **Auth refactor (Q3):** unblocks payments, mobile, and partner teams.
- **Dashboard (Q4):** isolated; can ship later with no cascading delay.
- One-pager attached with full analysis.

Good with this? Happy to take a 15-min call if it's faster.

— Sarah
```

---

**Type:** email (received)
**Length:** 168 → 78 words (54% reduction)

---

### Varys pass — whispers from the source

```
### Whisper 1 — Sarah validated with both teams first [signal]
**Lede:** Sarah validated Option B with auth and dashboard teams before sending.
**Why it matters:** Buy-in is already lined up — approval likely lands cleanly without a separate consultation pass.
**Source quote:** "I've also had a chance to talk with the auth team and the dashboard team."
```

### Example 2 — Email draft you're sending (Varys SKIPPED)

**Input** (95 words — your own draft to a colleague, asking for sign-off):

> Hi Marcus, hope you're well. I wanted to circle back on the vendor evaluation we kicked off last month — I've finished going through all five proposals and I'm prepared to make a recommendation. The short version is that I think Acme is the best fit for what we need, mostly because of their support tier and the integration story. There are a couple of caveats but nothing dealbreaker. Could we find 15 minutes this week to walk through it and lock in the choice? Let me know what works.

**Output:**

```
Subject: Vendor pick: recommending Acme

Recommending Acme on the vendor eval — best support tier and cleanest integration path.

- **Five proposals reviewed**, Acme leads on support and integration.
- **Caveats present** but none are dealbreakers.
- **15-min call this week** to walk through and lock the choice?
```

---

**Type:** email (draft)
**Length:** 95 → 47 words (51% reduction)

(No Varys block — input is a draft you're authoring; nothing to whisper from.)

### Example 3 — Workplace memo

**Input** (one paragraph from a 600-word memo announcing a hybrid work policy change):

> Team — As many of you have heard through various channels and rumors over the past several weeks, we've been working on updates to our hybrid work policy. After a lot of careful discussion with leadership and a survey of the broader org, we've landed on a new approach that we think balances flexibility with the kind of in-person collaboration that drives our best work. Starting June 1, we'll be moving to a "core days" model: Tuesdays and Wednesdays will be in-office for everyone (with limited exceptions for travel, family, or health reasons), while Mondays, Thursdays, and Fridays will be flexible. We believe this gives people the predictability they've been asking for while also creating moments of genuine in-person collaboration. We know this is a change for some of you, and we want to acknowledge that and hear your concerns. There will be an open Q&A on May 20 at 2pm and a follow-up survey on June 30 to evaluate how it's going.

**Output:**

```
# New hybrid policy: Tues–Wed in-office

*108 words, 0.5 minutes*

Starting June 1, Tuesdays and Wednesdays are in-office for everyone; Mon/Thu/Fri stay flexible.

**Why it matters:** Predictable core days give the team room to plan personal life, and create the in-person time the survey said was missing.

## What's changing

- **Core in-office days:** Tuesdays + Wednesdays.
- **Flexible days:** Monday, Thursday, Friday — work from anywhere.
- **Exceptions:** travel, family, health — talk to your manager.

## What's next

- **Q&A:** May 20, 2pm. Bring your concerns.
- **Survey:** June 30 — we'll check whether this is working.
- **Adjust if needed:** this is v1, not a verdict.
```

---

**Type:** workplace_memo (existing — being compressed for records)
**Length:** 240 → 108 words (55% reduction)

---

### Varys pass — whispers from the source

```
### Whisper 1 — Rumors preceded the announcement [signal]
**Lede:** Memo opens by acknowledging the policy was leaking through "channels and rumors" for weeks.
**Why it matters:** Information control on people-policy is loose — worth a process review before the next sensitive change.
**Source quote:** "many of you have heard through various channels and rumors."
```

### Example 4 — Social media post (Varys always SKIPPED)

**Input** (28 words — a draft tweet about a new product launch):

> So excited to announce we are finally launching our new analytics dashboard today after months of work — would love it if you could check it out and let us know what you think!

**Output:**

```
📊 Analytics dashboard ships today.

Cleaner views, faster loads, fewer clicks. Built it with the feedback you sent — thank you.

Try it: [link]
```

---

**Type:** social_media
**Length:** 35 → 18 words (49% reduction)

(No Varys block — social posts are always too short and always drafts.)

---

## Self-check before delivering

Run the universal checklist (Step 6) plus these final questions:

- [ ] Did I rewrite into the matching type's skeleton, not just shorten the prose?
- [ ] Tease ≤ 6 words, active verb, concrete?
- [ ] Lede stands alone as the One Big Thing?
- [ ] Axiom is bold, 1–2 sentences, perspective-adding?
- [ ] Bullets used wherever 3+ related points exist?
- [ ] Inclusive overlay applied — irrelevant identity descriptors cut?
- [ ] Reading-time marker at top if length warrants?
- [ ] Output is in the native medium's shape — not boxed in a code block (except where this skill template explicitly shows code blocks for clarity)?
- [ ] Compression metadata block present (or skipped only because the rewrite is shorter than the metadata would be)?
- [ ] **Varys handled correctly** — block present (with whispers OR "Pass clean — no whispers.") if the input was source material; block **omitted entirely** if the input was a draft / fragment / social post?
- [ ] Whispers respect hard caps (title ≤8w, lede ≤15w, why ≤20w, body ≤80w)?
- [ ] Whisper categories assigned and lowercased?
- [ ] No whispers that are actually entities the compression should have kept?
- [ ] On drafts: confirmed I did not invent whispers from the user's own writing?

If all checks pass, deliver.

---

## What this skill does NOT do

- **No TOC atomization.** Per-entity sharding for the Anansi vault is `smart-brevity-atomize`'s job.
- **No vault writes.** This skill returns text. It does not call `anansi_capture`, `r2_save`, or any vault tool.
- **No summarization without the skeleton.** A plain summary belongs to a generic summarizer.
- **No re-extraction or re-typing of entities.** That's the PARA pipeline's job.
- **No invented facts.** If the source doesn't say it, the compression doesn't say it. The Varys pass especially: if you can't quote it, you can't whisper it.
- **No fluffing.** A rewrite shorter than the original is the point. If the rewrite ends up longer, something has gone wrong.

---

## References

Local to this skill:

- `references/cheat-sheet.md` — universal one-pager (Ch. 23)
- `references/general.md` — universal foundation (Ch. 1–11)
- `references/inclusive-overlay.md` — accessibility lens (Ch. 22), always loaded
- `references/email.md` — email rules (Ch. 15)
- `references/newsletter.md` — newsletter rules (Ch. 13)
- `references/workplace-memo.md` — workplace memo rules (Ch. 14)
- `references/meeting.md` — meeting agenda + notes rules (Ch. 16)
- `references/speech.md` — speech / talk rules (Ch. 17)
- `references/presentation.md` — presentation / slide deck rules (Ch. 18 + Ch. 20)
- `references/social-media.md` — social media rules (Ch. 19)
- `references/company-update.md` — leader-to-org rules (Ch. 21)
- `references/book.md` — long-form / book chapter rules

External attribution:

- *Smart Brevity: The Power of Saying More with Less* — Schwartz, Allen, VandeHei (Workman Publishing, 2022). Every rule in the bundled `references/` traces to a chapter in this book. The book is not packaged with the skill; readers wanting the full source go to the book directly.

Adjacent skills:

- `plugins/anansi-para.plugin/skills/smart-brevity-atomize/` — Stage 3 of the PARA atomization pipeline (per-entity sharding for the Anansi vault). Use that, not this, when 