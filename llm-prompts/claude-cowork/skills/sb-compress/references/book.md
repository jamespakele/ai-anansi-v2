# Smart Brevity — Book rules

> Layered on top of `general.md`. Apply both.

---

## When this is the right format

- Input is a published book, long-form nonfiction, or any structured document with named chapters or sections
- You are **processing someone else's content** for retention, reference, or vault ingestion — not drafting original content
- Every chapter carries information the reader needs. Nothing is dropped. Compression happens *within* each block, not by eliminating blocks.

---

## Two output variants

Both are correct. Signal determines which to use.

### Variant A — Direct SB pass (voice-driven)

Use when: capturing the book's editorial voice, hooks, and memorable lines. The output reads like a companion guide you'd want to re-read. Tease lines are punchy and opinionated, sometimes quoting the book directly. The structure template below is the contract — every block is one chapter, headed by a ≤6-word tease and one-sentence lede that double as both.

### Variant B — Chapter reference guide (encyclopedic)

Use when: producing a structured reference optimized for scanning and retrieval. Tease lines are functional labels. Part/section headers appear as non-content dividers. Output reads like a structured index — the bold tease and separate lede sentence sit on different lines.

If the user doesn't specify, default to **Variant A** for books with strong authorial voice; **Variant B** for instructional or reference books.

---

## Structure — Variant A (Direct SB pass)

```
# [Book tease — ≤6 words, adapted from title or One Big Thing]

*~[word count] words, [N] minutes*

[Book lede — one sentence: the single thing the reader should take from the whole book]

**Why it matters:** [1–2 sentences of context — why this book, why now]

---

## [Introduction title or Ch 0: intro]

[Chapter tease — ≤6 words, standalone line, can quote the book]

**Why it matters:** [1–2 sentences]

- **[Key term]:** [supporting fact]
- **[Key term]:** [supporting fact]

---

## Ch [N]: [Chapter Title]

[Chapter tease — ≤6 words]

**Why it matters:** [1–2 sentences]

- **[Key term]:** [supporting fact]
- **[Key term]:** [supporting fact]
- **[Key term]:** [supporting fact]

---
```

---

## Structure — Variant B (Chapter reference guide)

```
# [Book title — full or shortened]
*[Author] · [N] chapters · ~[N] minutes*

---

## [Part N: Part Title]  ← non-content divider; use only when book has named parts

---

### [N] — [Chapter Title]

**[Chapter tease — ≤6 words, bold]**

[Chapter lede — one sentence]

**Why it matters:** [1–2 sentences]

- **[Key term]:** [supporting fact]
- **[Key term]:** [supporting fact]

---

### [N] — [Chapter Title]
...
```

---

## Rules that apply to both variants

### Book-level intro block
- **Variant A:** always present. Tease = ≤6 words as `#` heading. Lede = the book's One Big Thing. Why it matters = why this book matters now. Then `---`.
- **Variant B:** book title as `#` heading + author/chapter-count/reading-time line. No body text before the first chapter block.

### One block per chapter. No exceptions.
Every chapter or named section gets its own block. If a book has 24 chapters, the output has 24 chapter blocks. Dropping a chapter because it felt thin or repetitive is not permitted — compress it, don't delete it.

### Tease (≤6 words)
- **Variant A:** standalone line, active verb, can quote the book directly
- **Variant B:** bold line immediately after the chapter header
- Both: ≤6 words, concrete, no SAT words, no irony

### Lede (one sentence)
- **Variant A:** the tease line doubles as the lede — one punchy sentence that is also the chapter's takeaway
- **Variant B:** a separate sentence after the bold tease — adds the lede as a distinct element
- Both: one sentence, the ONE thing the reader must remember from this chapter

### Why it matters
- Always bolded
- 1–2 sentences
- Adds perspective — does not repeat the lede
- Answers: what changes? what does this signal? why does this chapter exist in the book?

### Bullets
- 3–5 per chapter block
- Bold the lead key term in each
- 1–2 sentences per bullet max
- Ordered by importance — most critical bullet first

### Closing axiom (optional — use sparingly)
Some chapters earn a second named axiom *after* the bullets — a closing punch that crystallizes the lesson. This is distinct from the opening **Why it matters** axiom, which adds context. The closing axiom lands the verdict.

Use it only when the chapter has a genuinely summative statement that hits harder as a standalone line than as a bullet. If every chapter gets one, it loses all force.

```
**The bottom line:** If you see everything, you remember nothing.
```

Other closing axioms from `general.md` that work in this position: **The bottom line**, **Reality check**, **Between the lines**, **What's next**. Do not use **Why it matters** as the closing axiom — it belongs at the opening.

This is not the same as the **Caveat** edge case. A caveat restores nuance that compression risk losing. A closing axiom amplifies the chapter's core lesson. They serve opposite purposes.

### Part/section headers (Variant B only)
- `## Part N: [Part Title]` as a non-content divider when the book has explicit named parts
- Not a block — no lede, no why, no bullets
- Followed immediately by `---` then the first chapter block in that part

### `---` separator
Every block ends with `---`. The separator is the parse token for downstream processing. Never omit it, even on the last block.

### Reading the file in chunks
Books are large. Read in chunks of 200–300 lines using offset/limit. Process each chapter as you read it — produce the block before advancing to the next chunk. Never assume content you haven't read. If a chapter boundary falls mid-chunk, finish reading the chapter before emitting the block.

---

## Length targets

| Element | Target |
|---|---|
| Book tease / title | ≤6 words |
| Book lede | 1 sentence |
| Book why it matters | 1–2 sentences |
| Chapter tease | ≤6 words |
| Chapter lede | 1 sentence |
| Chapter why it matters | 1–2 sentences |
| Bullets per chapter | 3�