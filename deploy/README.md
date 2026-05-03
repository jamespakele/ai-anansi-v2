# Anansi

Autonomous knowledge-graph engine. Drop documents into an inbox folder; Anansi
atomizes them into typed notes and stores them in PostgreSQL — queryable from
Claude via MCP.

---

## Quick start (5 minutes)

### 1 — Prerequisites

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) installed
- A Gemini API key **or** [Ollama](https://ollama.com) installed locally

### 2 — Clone this folder and configure

```bash
git clone https://github.com/jamespakele/anansi-deploy
cd anansi-deploy
cp .env.example .env
```

Edit `.env` — fill in `POSTGRES_PASSWORD` and your LLM choice:

| I have | What to set |
|--------|------------|
| **Gemini API key** ([free](https://ai.google.dev)) | `ANANSI_GEMINI_API_KEY=AIza...` |
| **Ollama** + `qwen2.5:14b` pulled | See `anansi.toml` below |
| **OpenRouter** account | `ANANSI_OPENROUTER_API_KEY=sk-or-...` |

### 3 — Set up anansi.toml

```bash
mkdir -p data/anansi
cp anansi.toml.example data/anansi/anansi.toml
```

Edit `data/anansi/anansi.toml` for your LLM:

**Gemini API:**
```toml
[llm]
backend = "gemini"
```

**Ollama (no API key — runs on your machine):**
```toml
[llm]
backend = "ollama"
url     = "http://host.docker.internal:11434"
model   = "qwen2.5:14b"
```

**OpenRouter:**
```toml
[llm]
backend = "openrouter"

[llm.openrouter]
model = "google/gemini-2.5-pro"
```

### 4 — Start the stack

```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml up -d
```

### 6 — Get your HTTPS URL

Claude Cowork requires HTTPS. The local overlay starts a free Cloudflare tunnel:

```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml logs tunnel
# Look for: https://abc123.trycloudflare.com
```

### 7 — Connect Claude Cowork

Claude Cowork → Settings → MCP → Add server → paste the `https://...trycloudflare.com` URL.

---

## Using the inbox

Drop any `.md` or `.txt` file into `./data/inbox/`. Anansi picks it up within
30 seconds, runs the full PARA → Smart Brevity pipeline, and ingests the results
into PostgreSQL. Check progress:

```bash
docker compose logs anansi -f
```

---

## MCP tools available in Claude

| Tool | What it does |
|------|-------------|
| `anansi_search` | Keyword search across all notes |
| `anansi_search_semantic` | Vector similarity search |
| `anansi_remember` | Ingest a document via MCP |
| `anansi_embed` | Generate embeddings for notes |
| `anansi_purge` | Remove an ingestion batch |
