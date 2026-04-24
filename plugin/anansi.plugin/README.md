# anansi.plugin

A Cowork plugin for [anansi v2](https://github.com/pakele-ai/ai-anansi-v2) — a structured
knowledge-pipeline that decomposes source documents into atomic notes and a traversable
knowledge graph.

## Purpose

This plugin produces **preprocessed-TOC** source files ready for direct ingestion into the
anansi daemon. When a source file has a valid `anansi_toc` frontmatter block, the daemon
skips its LLM Pass 1 (decomposition) and runs only the cheaper extraction and synthesis
passes. This means:

- Faster ingestion
- Higher quality entity recognition (you — or Claude — review the TOC before it goes in)
- Auditability: the TOC is committed alongside the source

## Installation

```sh
cowork plugin install anansi.plugin
```

Or copy the `anansi.plugin/` directory into your Cowork plugins folder manually.

## Configuration

Set `ANANSI_ROOT` to the path of your anansi vault root before invoking the plugin:

```sh
export ANANSI_ROOT=/path/to/my-vault
```

You can also specify the root interactively when the plugin prompts for it.

## Commands

### `/toc <file-path>`

Preprocess a source markdown file for anansi ingestion.

1. Reads the source file.
2. Loads templates and `%Rules` from `<ANANSI_ROOT>`.
3. Produces an enriched Table of Contents covering all significant entities.
4. Splices the TOC into the file's frontmatter as `anansi_toc`.
5. Writes the augmented file (defaults to overwriting the original; you can opt for
   a `.augmented.md` sibling).
6. Prints an `anansi2 ingest` command you can run to trigger ingestion immediately.

**Example:**

```
/toc /path/to/my-vault/q3-planning-meeting.md
```

## Workflow

```
Source document
     │
     ▼
/toc <file>          ← this plugin
     │
     ▼
augmented file with anansi_toc frontmatter
     │
     ▼
anansi2 ingest <file>
     │
     ▼
atomic notes + knowledge graph in web.db
```

## Skill details

The underlying `anansi-toc` skill is documented in
`skills/anansi-toc/SKILL.md`. It follows a 9-step process and enforces the
anansi leaf format parser regex to guarantee the daemon will accept every TOC line.
