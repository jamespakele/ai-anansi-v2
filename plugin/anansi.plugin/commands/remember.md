# /remember

Store a source document in the anansi knowledge vault. Claude performs Pass 1 (TOC extraction with entity typing) in-context; the local anansi daemon handles Pass 3 (entity expansion) and Pass 4 (relationship extraction).

**Usage:**
```
/remember [vault_root]
```

- `vault_root` (optional): path to the vault root directory containing `anansi/anansi.toml`. If omitted, Claude will ask.

**What it does:**
1. Reads the source document (from conversation or a provided path)
2. Identifies all entities and their types (person, organization, topic, concept, event, project, note)
3. Builds an `anansi_toc` block and splices it into the source frontmatter
4. Calls `anansi_ingest` with `content + filename` in a single MCP call
5. Falls back to writing the augmented file to disk + printing the CLI command if the server is unreachable

**Requires:** `anansi2 --root <vault_root> serve` running and registered as an MCP server.

Follow the `anansi-remember` skill.
