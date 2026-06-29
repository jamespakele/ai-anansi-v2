---
name: wiki-recall
description: >
  Read-only retrieval from the local ~/llm-wiki/ folder. Reads the PARA-grouped
  index.md first to locate relevant articles, then reads specific entity files
  and follows [[wikilinks]] — no MCP call, no Docker round-trip. Optionally
  falls back to the Anansi server with --remote for entities not found locally
  or for a broader search.
argument-hint: "[query or match_key] [--local | --remote]"
---

# wiki-recall

Read-only retrieval from the local Karpathy-style LLM wiki.

The wiki is organized by **PARA** (Projects, Areas, Resources, Archives).
The agent reads `~/llm-wiki/index.md` first to locate relevant articles,
then reads the specific entity files and follows `[[wikilinks]]` to
connected notes. No MCP call, no Docker round-trip.

For entities not found locally, pass `--remote` to fall back to
`anansi_get` / `anansi_search` on the server.

---

## When to invoke

- User asks "what do I know about X" — check local wiki first
- User says "recall product:okf" — read the local file
- User says "search for OKF in the wiki" — search via index
- User says "search the server too" — pass `--remote`

---

## Inputs

- **query** — a match_key (e.g. `product:okf`), a name (e.g. `Andrej Karpathy`),
  or a keyword (e.g. `OKF`)
- **`--local`** (default) — search local files only
- **`--remote`** — search local first, then fall back to the server if not found

---

## Schema reference

The wiki uses entity_type suffixes on filenames: `{slug}.{entity_type}.md`.
Templates defining each entity_type's fields live in the Anansi config plugin
at `references/templates/entity-{entity_type}.md` (for identity types) or
`references/templates/{family}-{entity_type}.md` (for source/content_unit types).

PARA mapping:

| PARA section | entity_type(s) |
|---|---|
| **1. Projects** | `project` |
| **2. Areas** | `area` |
| **3. Resources** | Everything else — see below |
| **4. Archives** | `archive-*` (entity_type starts with `archive-`) |

### Core types (always present)

These are the standard second-brain entity types that come with Anansi:

| entity_type | template_class | PARA section |
|---|---|---|
| `person` | identity | 3. Resources |
| `organization` | identity | 3. Resources |
| `project` | identity | 1. Projects |
| `area` | identity | 2. Areas |
| `note` | identity | 3. Resources |
| `book` | identity | 3. Resources |
| `discussion` | content_unit | 3. Resources |
| `email` | content_unit | 3. Resources |
| `event` | utility | 3. Resources |
| `outline` | utility | 3. Resources |
| `product` | identity | 3. Resources |
| `persona` | utility | 3. Resources |
| `mission` | utility | 3. Resources |
| `manifest` | utility | 3. Resources |
| `concept` | identity | 3. Resources |

### Custom types (app-specific)

The templating system is extensible — any application can define new entity
 types for its own data. These are created via the `anansi-new-entity-type`
skill, which:

1. Checks the existing type registry for conflicts
2. Walks through template class, merge strategy, identity fields, and source hints
3. Generates a template file and saves it to the canonical location
4. The new type is immediately available to the MCP server and wiki pipeline

Once created, custom types behave exactly like core types:
- Notes are captured via `anansi_capture` with the new entity_type
- Wiki files are written as `{slug}.{new_type}.md`
- They appear under **3. Resources** in the index, grouped by entity_type
- They are fully recallable via this skill

Examples of custom types that have been added:
- `coruscant_flight` — Coruscant flight tracking (template: `coruscant-flight.md`)
- `coruscant_flight_output` — Coruscant flight output records (template: `coruscant-flight-output.md`)
- `coruscant_operation` — Coruscant operations (template: `coruscant-operation.md`)
- `coruscant_mission` — Coruscant missions (template: `coruscant-mission.md`)
- `coruscant_manifest` — Coruscant flight manifests (template: `coruscant-manifest.md`)
- `anansi_config` — System configuration entries (template: `anansi-config.md`)

To create a new entity type, use the `anansi-new-entity-type` skill.

---

## Step 1 — Classify the query

| Query shape | Implied entity_type | Local action |
|---|---|---|
| `type:slug` (match_key) | The `type` part | Read `{slug}.{type}.md` directly |
| "who is X" / "tell me about [person]" | `person` | Scan index for person entries matching X |
| "what is [org]" / "tell me about [organization]" | `organization` | Scan index for organization entries |
| "what project is X" / "what area is Y" | `project` / `area` | Scan index for project/area entries |
| "what do I know about [topic]" | Any | Scan full index for matching ledes/names |
| Keyword ("OKF", "vector database") | Any | Scan full index for matching ledes/names |

---

## Step 2 — Read the index

Always start by reading `~/llm-wiki/index.md`. The index is the navigation
map — it tells you what articles exist, their entity_type, and their lede.

The index is structured by PARA:

```markdown
## 1. Projects
- [[slug.project|Name]] — Lede

## 2. Areas
- [[slug.area|Name]] — Lede

## 3. Resources
### person
- [[slug.person|Name]] — Lede
### organization
- [[slug.organization|Name]] — Lede
### discussion
...
```

### How to use the index

1. **Scan the index** for entries whose name or lede matches the query.
2. **Filter by entity_type** when the query implies one (see Step 1 table).
3. **Note the PARA section** — this tells you the kind of thing you're looking at
   (a project with a deadline vs. an ongoing area vs. a reference resource).
4. **Collect the filenames** from the `[[slug.ext|Name]]` wikilinks of matching entries.

---

## Step 3 — Read matching files

For each matching entry from the index, read the corresponding file at
`~/llm-wiki/{slug}.{entity_type}.md`.

### What to extract from each file

Each file contains:
- **Frontmatter** (anansi_id, entity_type, name, match_key, updated_at)
- **Body** — the note content (lede, why, content sections)
- **Connections** — `[[wikilinks]]` to related notes

### Follow cross-references

After reading the primary matches, scan the `## Connections` section of
each file for `[[wikilinks]]`. For any linked note that is materially
relevant to the query, read that file too and incorporate its content.

Limit cross-reference depth to one hop (read the linked notes, but do not
recurse into their links) unless the query is very narrow and the
cross-reference is clearly central.

---

## Step 4 — Synthesize the answer

Combine what you found across all read files:

1. **Identify the entity** — name, entity_type, PARA section
2. **Summarize the lede** — what is this thing, in one sentence
3. **Include key details** — from the body (why, content)
4. **Cite sources** — use `[[slug.ext|Name]]` wikilinks for each piece of information
5. **Note connections** — mention related notes when they add context

### Answer format

```
*wiki-recall* — {query}

**{Name}** ({entity_type}) — {PARA section}

{lede}

{key details, with [[wikilink]] citations}

**Connections:**
- {related note} — {how it relates}
```

---

## Step 5 — Remote fallback (only with --remote)

If no local matches were found and `--remote` was passed:

- For match_key queries: call `anansi_get(match_key)`
- For name/keyword queries: call `anansi_search(query)`

Present the results alongside a note that they came from the server.

---

## Step 6 — Report

```
*wiki-recall* — {query}
• Source: {local | remote | local + remote}
• PARA section: {1. Projects | 2. Areas | 3. Resources | 4. Archives}
• Matches: {N}
• Files: {file paths}
```
