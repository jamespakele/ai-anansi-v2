# anansi.plugin

A Claude plugin for [anansi v2](https://github.com/pakele-ai/ai-anansi-v2) — a structured
knowledge-pipeline that decomposes source documents into atomic notes and a traversable
knowledge graph.

## Purpose

This plugin offloads **TOC generation (Pass 1)** to a frontier language model — Claude via
[Cowork](https://cowork.anthropic.com) or [Claude Code](https://claude.ai/code) — instead of
running it locally through Ollama. The frontend reads the source document and the vault's
`%Rules` and `templates/` context, then produces a fully-validated `anansi_toc` frontmatter
block. The local anansi daemon receives the augmented file and skips straight to the cheaper
Pass 3 (extraction) and Pass 4 (relationships).

Benefits:
- **Higher quality TOC** — frontier-model reasoning applied to entity identification and typing
- **Faster local pipeline** — Pass 1 (the heaviest LLM call) is eliminated from the daemon
- **Auditability** — the TOC is reviewed by a human-in-the-loop Claude session before ingestion
- **No Ollama required for decomposition** — local hardware only handles extraction/synthesis

> **Codex Desktop compatibility:** The plugin's `commands/` and `skills/` structure is
> compatible with the Claude Code plugin format and should be directly importable into
> Anthropic's Codex Desktop application with no modifications.

## Installation

### Claude Cowork / Claude Code

```sh
# Copy the plugin directory into your Cowork plugins folder
cp -r plugin/anansi.plugin ~/.claude/plugins/anansi.plugin

# Or install via the Cowork CLI (if available)
cowork plugin install anansi.plugin
```

### Codex Desktop

Import the `plugin/anansi.plugin/` directory through the Codex Desktop plugin manager, or
place it in the configured plugins directory. The `.claude-plugin/plugin.json` manifest is
read directly by both runtimes.

## Configuration

Set `ANANSI_ROOT` to the path of your anansi vault root before invoking the plugin:

```sh
export ANANSI_ROOT=/path/to/my-vault
```

You can also provide the root interactively when the plugin prompts for it.

The vault must contain:
- `anansi.toml` — daemon configuration
- `templates/*.md` — entity type schemas
- `%Rules/%Atomicity.md` and `%Rules/%Downstream-Flow.md` — decomposition rules loaded
  into the TOC skill

## Commands

### `/toc <file-path>`

Preprocess a source markdown file for anansi ingestion.

1. Reads the source file from disk.
2. Loads `templates/` and `%Rules` from `<ANANSI_ROOT>`.
3. Produces an enriched Table of Contents covering all significant entities, validated
   against the daemon's leaf-format parser regex.
4. Splices the TOC into the file's frontmatter as `anansi_toc`.
5. Writes the augmented file (defaults to overwriting; optionally saves as
   `<source>.augmented.md`).
6. Prints the `anansi2 ingest` command to trigger daemon ingestion immediately.

**Example:**
```
/toc /path/to/my-vault/q3-planning-meeting.md
```

### `/remember [vault_root]`

Store a source document in the vault in one motion. Claude performs Pass 1 (TOC extraction
with entity typing) in-context; the local daemon handles Pass 3 and Pass 4.

**Example:**
```
/remember /path/to/my-vault
```

## Workflow

```
Source document
     │
     ▼
/toc <file>   ←  Claude (Cowork / Claude Code / Codex Desktop)
     │            reads source + vault rules, produces anansi_toc
     ▼
augmented file with anansi_toc frontmatter
     │              (Pass 1 complete — no LLM call needed by daemon)
     ▼
anansi2 ingest <file>
     │
     ├── Pass 3: extract fields + summaries  (local LLM via Ollama)
     └── Pass 4: infer relationships         (local LLM via Ollama)
          │
          ▼
     atomic notes + knowledge graph in web.db
```

## Skill details

| Skill | File | Purpose |
|-------|------|---------|
| `anansi-toc` | `skills/anansi-toc/SKILL.md` | 9-step TOC generation — reads vault rules, validates leaf format, splices frontmatter |
| `anansi-remember` | `skills/anansi-remember/SKILL.md` | End-to-end store: TOC generation + MCP ingest call in one command |

The `anansi-toc` skill enforces the daemon's parser regex on every output line:

```
^\s*\d+(?:\.\d+)+\s+.+?\s+\[\w+\](?:\s*\|\s*\w+:[^|]*)* \s*$
```

Lines that don't match are silently dropped by the daemon, so the skill performs a
mental quality-check pass before writing.
