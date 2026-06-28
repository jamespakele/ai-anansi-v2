# anansi v2

An autonomous knowledge-graph engine. Drop any document into the inbox; Anansi
atomizes it into typed notes (people, projects, concepts, events …), graphs their
relationships, and stores everything in PostgreSQL — queryable over MCP from
Claude Cowork or any MCP client.

---

## How it works

```
/data/inbox/<note>.md
       │
       ▼  (background watcher — no human needed)
  Stage 1 ─────────────────────────────── (parallel)
    ├── para-projects-areas    → projects-areas-toc.md + projects-areas-typed.md
    └── para-resource-entities → resources-toc.md     + resources-typed.md
       │
       ▼
  Stage 2: sb-atomize          → <slug>-atomized.md + <slug>-toc.md
       │
       ▼
  Stage 3: ingest_atomized     → PostgreSQL (notes, edges, sources)
       │
       ▼
  MCP server (port 3738)  ←  Claude Cowork / Claude Desktop / any MCP client
```

Intermediate files are checkpointed in `/data/archive/<slug>-<timestamp>/`.
If a stage fails, drop the source file back into the inbox — completed stages
are skipped automatically.

---

## Local quick start

### Prerequisites

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (Mac/Windows)
  or Docker Engine (Linux)
- A Cloudflare account is **not** required — quick tunnels are anonymous

### 1 — Clone & configure

```bash
git clone https://github.com/jamespakele/ai-anansi-v2
cd ai-anansi-v2
cp .env.example .env
```

Edit `.env` and fill in `POSTGRES_PASSWORD` plus **one** LLM option:

| Option | What to do |
|--------|-----------|
| **Gemini API** (easiest) | Get a free key at [ai.google.dev](https://ai.google.dev) → set `ANANSI_GEMINI_API_KEY` |
| **Ollama** (no API key) | Install [Ollama](https://ollama.com), run `ollama pull qwen2.5:14b`, see config below |
| **OpenRouter** | Set `ANANSI_OPENROUTER_API_KEY` |

### 2 — Configure anansi.toml

Copy and edit the config:

```bash
cp anansi.toml.example data/anansi/anansi.toml
```

**For Gemini API:**
```toml
[llm]
backend = "gemini"
# url and model are ignored for gemini — set in [llm.gemini] if needed
```

**For Ollama (installed on your machine):**
```toml
[llm]
backend = "ollama"
url     = "http://host.docker.internal:11434"   # reaches your local Ollama
model   = "qwen2.5:14b"   # minimum recommended; 7b models may miss delimiters
```

**For OpenRouter:**
```toml
[llm]
backend = "openrouter"

[llm.openrouter]
model = "google/gemini-2.5-pro"
```

### 3 — Start the stack

```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml up -d
```

This starts:
- `db` — PostgreSQL + pgvector
- `anansi` — MCP server + inbox watcher
- `tunnel` — Cloudflare quick tunnel (free, no account needed)

### 4 — Get your HTTPS URL

```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml logs tunnel
```

Look for a line like:
```
INF | Your quick Tunnel has been created! Visit it at: https://abc123.trycloudflare.com
```

### 5 — Connect Cowork

In Claude Cowork → Settings → MCP → Add server → paste your `https://...trycloudflare.com` URL.

### 6 — Try it

Drop a Markdown file into `./data/inbox/` and watch the logs:

```bash
echo "# Meeting Notes\n\nDiscussed roadmap with Alice and Bob from Acme Corp." \
  > data/inbox/test.md

docker compose -f docker-compose.yml -f docker-compose.local.yml logs anansi -f
```

---

## VPS deploy (production)

Uses Traefik for HTTPS — no Cloudflare tunnel needed.

```bash
# On the VPS:
git clone https://github.com/jamespakele/ai-anansi-v2 /docker/ai-anansi-v2
cd /docker/ai-anansi-v2
cp .env.example .env   # fill in POSTGRES_PASSWORD + ANANSI_GEMINI_API_KEY
docker compose up -d
```

Traefik routes `https://your-domain.com` → `127.0.0.1:3738` automatically via
the labels in `docker-compose.yml`. Update the `Host()` rule and `ANANSI_PUBLIC_URL`
to match your domain.

---

## MCP tools

| Tool | Description |
|------|-------------|
| `anansi_search` | Full-text keyword search across all notes |
| `anansi_search_semantic` | Vector similarity search (requires embeddings) |
| `anansi_remember` | Ingest a raw source document via MCP |
| `anansi_ingest_atomized` | Ingest pre-atomized markdown directly |
| `anansi_embed` | Generate/refresh embeddings for notes |
| `anansi_purge` | Remove an ingestion batch by source ID |

---

## Inbox watcher

Drop any `.md` or `.txt` file into `/data/inbox/`. The watcher picks it up
within `poll_interval_secs` (default 30s) and runs the full pipeline.

Enable in `anansi.toml`:

```toml
[inbox]
enabled            = true
watch_dir          = "/data/inbox"
archive_dir        = "/data/archive"
skills_dir         = "/app/skills"     # baked into the image (from llm/plugins); dev override mounts llm/plugins -> /data/skills
poll_interval_secs = 30
llm_backend        = "gemini"          # optional: overrides [llm] backend for inbox only
```

Skills are baked into the image at `/app/skills` (from `llm/plugins` via the Dockerfile); prod reads `/app/skills` (rebuild to update). For local-dev live editing, `docker-compose.local.yml` mounts `llm/plugins/anansi.plugin/skills` -> `/data/skills` (set `skills_dir = "/data/skills"`).

---

## Architecture

| Module | Purpose |
|--------|---------|
| `src/mcp.rs` | Axum HTTP server, JSON-RPC 2.0 dispatcher |
| `src/inbox.rs` | Background watcher, 3-stage pipeline orchestration |
| `src/llm.rs` | Multi-backend LLM client (gemini, openrouter, codex, ollama) |
| `src/atomized_ingest.rs` | Parses atomized markdown → PostgreSQL |
| `src/db.rs` | PostgreSQL connection, migrations, helper queries |
| `src/embed.rs` | Gemini text-embedding-004 vector generation |
| `src/config.rs` | `anansi.toml` + env-var loading |

---

## License

MIT
