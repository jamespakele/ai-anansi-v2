# Naming conventions

Canonical naming forms per Resource type. Same input → same canonical name → same slug → same vault upsert key. This is what keeps the knowledge graph from fragmenting.

The templates in `references/templates/` are authoritative for type semantics. This file is authoritative for the **shape of the name** that goes into the canonical Name field and the slug.

---

## The slug algorithm (anansi `match_key`)

Identical to `para-extract`:

1. Lowercase the canonical name.
2. Replace every non-alphanumeric character with a space.
3. Collapse whitespace runs to a single space.
4. Trim. Replace remaining spaces with hyphens.
5. Drop disambiguator parentheticals before slugging — `(talk)`, `(podcast)`, `(article)`, `(spec)`, `(paper)`, etc. do not appear in the slug. They live only in the canonical Name. (Note: `(book)` is **not** used as a disambiguator — books take the dedicated `book:` type, which carries the disambiguation in the type label itself.)

Examples:

| Canonical Name | Slug |
|---|---|
| `James K. Pakele` | `james-k-pakele` |
| `Anthropic` | `anthropic` |
| `O'Brien & Co.` | `o-brien-co` |
| `Why Sovereign AI Matters (talk)` | `why-sovereign-ai-matters` |
| `Building a Second Brain` (a `book:` Resource) | `building-a-second-brain` |
| `PICHTR` | `pichtr` |
| `Q3 Board Deck` | `q3-board-deck` |

The disambiguator parenthetical's omission from the slug is deliberate — two books with the same title (rare but real) would collide; in that case append a year or author to the canonical name itself, which then propagates into the slug: `The Pragmatic Programmer (Hunt 1999)` → `the-pragmatic-programmer-hunt-1999`.

---

## Persons

**Form:** full canonical name as given in the source. First + Last at minimum; include middle name or initial if the source provides it; include suffixes (`Jr.`, `III`) only when the source does.

| Source mention | Canonical Name | Slug |
|---|---|---|
| "Sarah Chen at Anthropic" | `Sarah Chen` | `sarah-chen` |
| "Dr. Tiago Forte" | `Tiago Forte` | `tiago-forte` |
| "James K. Pakele" | `James K. Pakele` | `james-k-pakele` |
| "Reid Hoffman" | `Reid Hoffman` | `reid-hoffman` |
| "Marcus Aurelius" | `Marcus Aurelius` | `marcus-aurelius` |

### Honorifics and titles

Strip honorifics (`Mr.`, `Mrs.`, `Ms.`, `Dr.`, `Prof.`) from the canonical name. Job titles (`CEO`, `VP of Engineering`, `Senator`) are not part of the name — they belong in `met_via` or in adjacent context, not in the Name field.

### Single-name mentions

Only one name is given (`Sarah said…`, `met with Yuto`)? Use it as-is at lower confidence. Slug = the single name. Type confidence drops to medium because re-identification on a future ingest is fragile. Add an evidence quote that anchors the mention.

### Two people, same first name

Disambiguate in the canonical name and slug. Keep both as `person:` entries — never collapse.

| Source mentions | Canonical Names |
|---|---|
| "Sarah Chen and Sarah Park" | `Sarah Chen` (`sarah-chen`), `Sarah Park` (`sarah-park`) |
| "James" (multiple Jameses) | append surname or context: `James Pakele`, `James Chen` |

### Identity fields (per `entity-person.md`)

Populate when the source provides them:

- `name` — required, the canonical Name above.
- `contact_email` — extract verbatim from headers, signatures, or explicit mention. Don't construct.
- `contact_phone` — extract verbatim from signature or explicit mention.
- `met_via` — one short sentence on how the user met or came to know this person. Smart-brevity style. Examples: `via Q3 board deck email thread`, `introduced by Ron Nishihara`, `met at the 2025 Hawaii AI Summit`.

If a field is not in the source, omit it from the `Identity fields` line. Don't write `<unknown>` or guess.

---

## Organizations

**Form:** the official name as commonly used. Strip legal suffixes (`, Inc.`, `, LLC`, `, PBC`) **unless** they are needed to disambiguate from another entity sharing the same short name.

| Source mention | Canonical Name | Slug |
|---|---|---|
| "Anthropic, PBC" | `Anthropic` | `anthropic` |
| "Continest" | `Continest` | `continest` |
| "PICHTR" (acronym only) | `PICHTR` | `pichtr` |
| "the Department of Hawaiian Home Lands" | `Department of Hawaiian Home Lands` | `department-of-hawaiian-home-lands` |
| "Apple Inc." (no ambiguity) | `Apple` | `apple` |

### Acronyms

If the source uses only the acronym, keep the acronym as the canonical Name. If the source provides both forms (`PICHTR (Pacific International Center for High Technology Research)`), put the long form in `full_name` and keep the acronym as `name`.

### Identity fields (per `entity-organization.md`)

Populate when the source provides them:

- `name` — required.
- `full_name` — expanded or legal name if different.
- `type` — company, NGO, government agency, research institution, etc. Source from explicit mention; if not stated, omit (don't infer from name shape).
- `domain` — primary field of operation if stated.

---

## Books

**Form:** the full title as printed, including subtitle if the source provides it. Subtitle stays attached to the title (em-dash or colon as the source has it). **No `(book)` disambiguator** — the type itself (`book:`) carries the disambiguation.

| Source mention | Canonical Name | Slug |
|---|---|---|
| "Building a Second Brain by Tiago Forte" | `Building a Second Brain` | `building-a-second-brain` |
| "The Pragmatic Programmer (Hunt 1999)" | `The Pragmatic Programmer` (or `The Pragmatic Programmer (Hunt 1999)` to disambiguate from later editions) | `the-pragmatic-programmer` |
| "Atomic Habits" | `Atomic Habits` | `atomic-habits` |
| "Tao Te Ching" | `Tao Te Ching` | `tao-te-ching` |

### Subtitle handling

If the source provides the subtitle, include it in the canonical Name with the punctuation the source uses:

- "Building a Second Brain — A Proven Method to Organize Your Digital Life" → `Building a Second Brain — A Proven Method to Organize Your Digital Life`
- "Smart Brevity: The Power of Saying More with Less" → `Smart Brevity: The Power of Saying More with Less`

Slugs use the full title-and-subtitle when subtitles are present; the slug algorithm collapses punctuation cleanly.

### Two books with the same title

Append a year or author parenthetical to the canonical Name to disambiguate:

- `The Pragmatic Programmer (Hunt 1999)` and `The Pragmatic Programmer (20th Anniversary Edition)` if both are in scope.
- `Sovereignty (Mason 2018)` and `Sovereignty (Krasner 1999)`.

The disambiguator becomes part of the slug (`the-pragmatic-programmer-hunt-1999`) — this is intentional, since the two are genuinely different vault entries.

### Identity fields (per `entity-book.md`)

Required:

- `title` — the canonical Name above. Verbatim title (with subtitle when given).

Optional, populate only if source provides:

- `author` — `Last, First` format. One per line if multiple authors.
- `year` — four-digit publication year.
- `publisher` — publisher name only; omit imprint parent unless relevant.
- `isbn` — 13-digit ISBN with hyphens.
- `description` — one-sentence description of the book's core claim, only if the source provides one verbatim.

If a field is not in the source, omit it from the `Identity fields` line. Don't construct ISBNs or guess publishers.

---

## Notes (the catchall)

The `note` type covers every Resource that is neither a person, an organization, nor a book. Talks, podcasts, articles, specifications, frameworks, products, places, events, named documents, third-party projects/areas, named initiatives the user does not own.

**Form:** descriptive title-case noun phrase. The Name should read naturally and be unambiguous on its own.

### Disambiguator suffixes

When the bare name could plausibly be confused for a different entity type or for a different work, append a parenthetical disambiguator:

| Kind | Suffix | Example |
|---|---|---|
| Talk / lecture | `(talk)` | `Why Sovereign AI Matters (talk)` |
| Podcast / episode | `(podcast)` or `(episode)` | `Lex Fridman Podcast (podcast)`, `Acquired — TSMC (episode)` |
| Article / paper | `(article)` or `(paper)` | `The Bitter Lesson (article)`, `Attention Is All You Need (paper)` |
| Specification / standard | `(spec)` or `(standard)` | `Claude Skills Spec (spec)`, `OAuth 2.1 (spec)` |
| Framework / methodology | bare; only disambiguate if confusable with a product | `PARA Method`, `Smart Brevity` |
| Product | `(product)` only when name is generic | `Claude (product)` (only if there's also `Claude` the person in scope) |
| Place | bare unless ambiguous | `Kahoolawe` |
| Event | bare unless ambiguous | `2025 Hawaii AI Summit` |

`(book)` is **not** in this table — books take the dedicated `book:` type, not `note:`. See the Books section above.

### When to use a disambiguator

- The bare title is a generic-sounding noun phrase that could also plausibly name a person, organization, or product.
- The same title exists in multiple media (a book and a podcast both called "Sovereignty").
- Internal nicknames the source uses (`the Q3 OKR doc`) — type-named so reviewers later understand the shape: `Q3 OKR Doc (document)`.

### When NOT to use a disambiguator

- The name is already clearly a thing of its kind: `Tao Te Ching`, `Bushido`, `Honolulu Star-Advertiser`, `Sequoia Capital`.
- The entity type prefix already carries the disambiguation — `book:` never gets `(book)`, `person:` never gets `(person)`.
- The name is a unique proper noun with no plausible confusion — `Kahoolawe`, `Anthropic`.

When in doubt, add the disambiguator. The cost of a redundant parenthetical is low; the cost of a misidentified entity is a broken edge in the vault.
