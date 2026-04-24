# Anansi v2 — Build Document 3: Interfaces & Deployment

**For:** Coding agent (Claude Code) executing against the repo produced by docs 1 and 2.
**Reference:** `anansi-v2-spec.md` §15 (Cowork plugin contract), §16 (module layout), §21 (deployment).
**Prerequisite:** Docs 1 and 2 complete. Pipeline is testable via `cargo test`, integration test passes with mock LLM. Live Ollama smoke test works.
**Output:** A runnable, deployable anansi2 binary. MCP server exposes ingest + search + get + edges tools. CLI entry point supports `ingest`, `serve`, and `init` commands. Dockerfile + compose file build and run. Thin Cowork plugin produces preprocessed-TOC source files.

---

## Goal

Ship the surfaces. External users — CLI invokers, MCP clients (Cowork, Claude Code, Claude Desktop), and HTTP callers — get predictable entry points to the pipeline built in doc 2. Deploy via Docker with a single `docker compose up`. Cowork plugin produces valid preprocessed-TOC files that plug straight into the pipeline.

## Files to create

### 1. `src/mcp.rs`

MCP server over HTTP. Exposes tools via JSON-RPC 2.0 at POST `/` and a health endpoint at GET `/health`.

Tools to implement (v1 minimum):

| Tool | Description |
|---|---|
| `anansi_ingest` | Ingest a source file by path, or raw content + filename. Returns source_id + outline_note_id. Gated behind `read_only: false`. |
| `anansi_search` | SQL LIKE search over notes.name, notes.summary_1, notes.summary_5. Returns list of match with id, name, entity_type, file_path, summary_1. |
| `anansi_get` | Return a note by id or match_key. Includes frontmatter + body + edge count. |
| `anansi_edges` | BFS traversal from a note by id. Optional edge_type filter, depth (1–5, default 1). Returns connected notes + edge metadata. |
| `anansi_relate` | Manually declare an edge between two notes. Writes to `edges` table. Gated behind `read_only: false`. |

Additionally, `GET /health` returns:

```json
{
  "status": "ok",
  "ollama": "reachable",
  "db": "ok",
  "version": "0.1.0"
}
```

The health handler calls `llm.ping()` and `SELECT 1` on the DB; returns 200 if both succeed, 503 if either fails (with details in the body).

Structure:

```rust
use axum::{Router, routing::{post, get}, extract::State, Json};

#[derive(Clone)]
pub struct McpState {
    pub ctx: Arc<IngestContext>,
}

pub fn router(ctx: Arc<IngestContext>) -> Router {
    Router::new()
        .route("/", post(handle_json_rpc))
        .route("/health", get(handle_health))
        .with_state(McpState { ctx })
}

pub async fn serve(ctx: Arc<IngestContext>, host: &str, port: u16) -> Result<()> {
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("MCP server listening on {addr}");
    axum::serve(listener, router(ctx)).await?;
    Ok(())
}

async fn handle_json_rpc(State(state): State<McpState>, Json(req): Json<JsonRpcRequest>) -> Json<Value> {
    match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "tools/list" => handle_tools_list(req.id),
        "tools/call" => handle_tools_call(state, req.id, req.params).await,
        _ => json_rpc_err(req.id, -32601, "Method not found"),
    }
}
```

For the tool schemas, follow the MCP spec's `inputSchema` shape (JSON Schema). Reference the existing `anansi.plugin`'s tool definitions as a template — same names where they overlap.

For `anansi_ingest`:
- Accepts `source_path: string` (absolute or relative to anansi root) OR `content: string` + `filename: string`.
- When `content` is provided, write it to `<anansi-root>/<filename>` first, then invoke the pipeline.
- When `read_only: true` in config, return an error: "this anansi instance is read-only."

For `anansi_search` (v1 simple mode):
```sql
SELECT * FROM notes
WHERE name LIKE ? OR summary_1 LIKE ? OR summary_5 LIKE ?
ORDER BY updated_at DESC
LIMIT ?
```
v1.5 replaces this with FTS5.

For `anansi_edges`: implement BFS with a visited set, capping traversal at `depth`. Return adjacency info.

~280 lines. Keep per-tool handlers small (<40 lines each).

### 2. `src/main.rs`

CLI entry point via `clap`. Three subcommands:

```
anansi2 init [--root PATH]
anansi2 ingest <file> [--root PATH]
anansi2 serve [--root PATH]
```

- `init` — create the anansi root folder structure if missing: `web/`, `%Rules/`, `templates/`, copy `anansi.toml.example` to `anansi.toml`. Seed `%Rules/` and `templates/` with the files from doc 1 if they're not present. Run DB migrations. Idempotent.
- `ingest` — load config, set up `IngestContext`, call `pipeline::ingest`, print the result. Exit 0 on success, non-zero on failure.
- `serve` — load config, set up context, call `mcp::serve` on the configured host/port. Blocks until SIGINT.

```rust
#[derive(Parser)]
#[command(name = "anansi2")]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init,
    Ingest { file: PathBuf },
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => cmd_init(&cli.root).await,
        Command::Ingest { file } => cmd_ingest(&cli.root, &file).await,
        Command::Serve => cmd_serve(&cli.root).await,
    }
}
```

For `init`, use `include_dir!` or equivalent to embed the seed templates and rules into the binary — or keep them as project files and copy from a known relative path. `include_dir` is cleanest; add to `Cargo.toml` if adopted.

~100 lines.

### 3. `Dockerfile`

Per spec §21. Two-stage build, debian-slim runtime. Copy verbatim from the spec; no changes.

### 4. `docker-compose.yml`

Per spec §21. Bind mount `./anansi:/anansi`, expose port 3738, env vars for Ollama URL and model.

### 5. `.dockerignore`

Keep the build context lean:

```
target/
.git/
anansi/
*.md
!README.md
```

### 6. `plugin/anansi.plugin/.claude-plugin/plugin.json`

The Cowork plugin manifest:

```json
{
  "name": "anansi-toc",
  "version": "0.1.0",
  "description": "Produce enriched TOCs for anansi source documents using the Claude model, enabling high-quality preprocessing before daemon ingestion.",
  "author": {
    "name": "Pakele.ai"
  },
  "keywords": ["anansi", "knowledge-vault", "zettelkasten", "toc-preprocessing"]
}
```

### 7. `plugin/anansi.plugin/skills/anansi-toc/SKILL.md`

The main orchestration skill. ~250 lines.

Frontmatter:

```yaml
---
name: anansi-toc
description: >
  Produce an enriched Table of Contents for an anansi source document,
  splice it into frontmatter, and write the augmented file ready for
  daemon ingestion. Use when the user says "preprocess for anansi",
  "generate anansi TOC", "toc this for anansi", or drops a markdown
  file and asks to prepare it for decomposition.
---
```

Body structure:

1. **When to invoke** — triggers list
2. **Prerequisites** — the anansi root folder path must be configured or provided by the user. Templates and %Rules live under `<anansi-root>/templates/` and `<anansi-root>/%Rules/`.
3. **Step 1: Locate the anansi root** — check env var `ANANSI_ROOT`, then ask the user if unset.
4. **Step 2: Read the source file** — use Read tool.
5. **Step 3: Load templates and %Rules** — Read `<root>/templates/*.md` and `<root>/%Rules/%Atomicity.md` + `%Downstream-Flow.md`.
6. **Step 4: Determine source_type** — read from frontmatter if present; ask the user otherwise (meeting_summary, research_paper, email_thread, container).
7. **Step 5: Assemble the Pass 1 prompt mentally** — combine rules + entity types + leaf format + source body.
8. **Step 6: Produce the enriched TOC** — follow the leaf format from spec §9 strictly. Every leaf must have an address, name, entity type in brackets, and a hint annotation. Identity types should add `context_at` when applicable.
9. **Step 7: Splice into frontmatter** — construct the augmented source file with:
   - `anansi_toc_version: 1`
   - `anansi_toc: |` (the TOC as a literal block)
   - `source_type`
   - `title`
   - `source_date` (if known)
   - `toc_author: claude-opus-4-7` (or whatever model is running)
   - `toc_generated_at: <ISO-8601 timestamp>`
10. **Step 8: Write the augmented file** — use Write tool. Default path: `<anansi-root>/<source-slug>.md` (overwriting the original if the user requested it, or writing to a sibling `.augmented.md` file if the user wants to review).
11. **Step 9: Report** — print the output path. Offer a bash command `anansi2 ingest <path>` the user can run if the daemon isn't watching the folder.

Include the full leaf format spec from the spec §9 inline in the skill body so Claude has the grammar at hand.

Include this as a quality rule at the top:

> **IMPORTANT:** The output TOC lines must match the parser regex exactly:
> `^\s*\d+(?:\.\d+)+\s+.+?\s+\[\w+\](?:\s*\|\s*\w+:[^|]*)*\s*$`
> Lines that don't match are silently dropped by the daemon. Do not produce markdown fences, preamble, or commentary — only valid leaf lines plus optional section header comments.

### 8. `plugin/anansi.plugin/commands/toc.md`

A slash command shortcut:

```markdown
---
description: Produce an anansi-compatible enriched TOC for a source document and save the augmented file.
---

Preprocess the following source document for anansi ingestion. Invoke the `anansi-toc` skill with the file path: $ARGUMENTS.

If no path was provided, ask the user for one.
```

### 9. `plugin/anansi.plugin/README.md`

Short README describing the plugin's purpose (producer of preprocessed-TOC files for anansi v2), installation (`cowork plugin install anansi.plugin`), configuration (`ANANSI_ROOT` env var or per-invocation prompt), and the single command it provides.

### 10. Repository `README.md`

Top-level readme for the anansi2 repo. Sections:

- **What it is** — one paragraph summary
- **Quick start** — `anansi2 init ./my-vault && anansi2 ingest ./my-vault/meeting.md`
- **Docker deploy** — `docker compose up -d` after configuring compose
- **Architecture** — point at `anansi-v2-spec.md` for the full design; one-paragraph overview here
- **Configuration** — the `anansi.toml` fields plus env vars
- **Development** — `cargo test`, `cargo check`, pointer at the three build docs
- **License** — whatever you choose

~150 lines.

### 11. `.github/workflows/docker.yml` (optional)

If you want CI-built images:

```yaml
name: docker
on:
  push:
    tags: ['v*']
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.ref_name }}
```

Optional for v1 — run `docker build` locally if you prefer.

---

## End-to-end test

After all files are in place, verify:

1. **Build the binary:** `cargo build --release`. No errors.
2. **Init a vault:** `./target/release/anansi2 init /tmp/test-anansi`. Verify the folder tree matches spec §4. DB file exists with migrations applied.
3. **Drop a source:** write a short meeting note to `/tmp/test-anansi/test-meeting.md` with 5–10 obvious entities.
4. **Ingest:** `./target/release/anansi2 ingest /tmp/test-anansi/test-meeting.md --root /tmp/test-anansi`. Watch stdout; no errors. Check `web/`: outline file present, atomic notes present.
5. **Serve:** `./target/release/anansi2 serve --root /tmp/test-anansi`. Hit `http://localhost:3738/health` with curl — expect 200 with OK body.
6. **MCP search:** send a JSON-RPC `tools/call` with `anansi_search` for an entity name from the source; expect a hit.
7. **Docker:** `docker compose up -d`. Verify the container starts, the health check passes, the MCP port is reachable from the host.
8. **Cowork plugin:** install the plugin locally, run `/toc /path/to/source.md`. Verify an augmented file is written with valid preprocessed-TOC frontmatter. Run `anansi2 ingest` on the augmented file; verify Pass 1 is skipped (only Pass 3 + Pass 4 LLM calls occur).

## Acceptance criteria

1. All commands (`init`, `ingest`, `serve`) work against a fresh anansi root.
2. `docker compose up` produces a healthy container; health endpoint returns 200.
3. MCP server responds to `initialize`, `tools/list`, `tools/call` per MCP spec.
4. Cowork plugin's `/toc` command produces a file whose `anansi_toc` frontmatter passes `validate_preprocessed_toc` in the daemon.
5. Re-ingesting the same source file is a no-op (content_hash dedup).
6. Re-ingesting with an edited `anansi_toc` regenerates the outline and merges atomic notes appropriately.

## Non-goals for this document

- Per-pass backend routing (Codex CLI, Claude CLI shims). v1.5.
- FTS5 search. v1.5.
- Embeddings / vector search. v2.
- Alpine image variant. Ship when VPS migration is imminent.
- Read-only mode on VPS. Add when the VPS is actually deployed.
- GitHub Actions beyond the optional Docker image build.

## Commit message

Suggested: `interfaces: MCP server, CLI, Dockerfile, Cowork plugin scaffold`

---

## What's done after this doc

The v1 is complete and deployable. You have:

- A Rust crate (~2,100 lines) implementing the spec end-to-end.
- A container image deployable via `docker compose up`.
- A Cowork plugin producing compliant preprocessed-TOC files.
- A full, documented MCP interface.
- All 15 templates, all 4 %Rules files, and 3 prompt files.

From here, the natural v1.5 work is:
- FTS5 search in `anansi_search`.
- Per-pass backend routing (enables the VPS scenario).
- A `/search`, `/get`, `/edges` set of Cowork skills for querying the vault from chat.

And the natural v2 work is:
- Vector embeddings over outlines and summaries.
- Classical NER preprocessing before Pass 3.
- Migration to Postgres + pgvector when scale demands.

---

*Doc 3 of 3 · Anansi v2 Build Instructions · 2026-04-23*
