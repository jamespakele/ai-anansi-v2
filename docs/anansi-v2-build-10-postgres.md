# Anansi v2 — Build Document 10: SQLite → PostgreSQL Migration

**For:** Coding agent (Claude Code / Antigravity) executing against the current repo.
**Reference:** `anansi-v2-spec.md` §10 (Database Schema).
**Prerequisite:** Builds 01–07 complete and deployed. Current `cargo build` passes.
**Motivation:** SQLite WAL + FTS5 content-backed tables have caused repeated index corruption in production, requiring manual stop-container-repair-restart cycles. PostgreSQL eliminates this entire class of failure: built-in ACID, native full-text search via `tsvector`, no WAL reader coordination problems, and a clean separation between reader and writer connections.
**Backend choice:** Self-hosted `pgvector/pgvector:pg16` on the VPS via docker-compose — PostgreSQL 16 with the pgvector extension pre-installed. No Supabase dependency, everything stays local. pgvector is enabled from day one so the `embedding vector(768)` column and HNSW index are part of the initial schema, not a future migration.

**Does NOT touch:** Business logic in `src/pipeline.rs`, `src/merger.rs`, `src/atomized_ingest.rs`, `src/atomized_parser.rs`, `src/mcp.rs` (tool handlers), `src/template.rs`, `src/prompt.rs`, prompt files, templates, `%Rules/`.

**Output:** Anansi running entirely on PostgreSQL with:
- Full-text keyword search via `tsvector` GIN index
- Semantic vector search via `pgvector` HNSW index (cosine similarity, 768-dim Gemini embeddings)
- No SQLite files on disk
- Datasette replaced by direct `psql` or pgAdmin for DB inspection

---

## Goal

Replace the SQLite backing store with PostgreSQL. The only files that change are:
- `Cargo.toml` — swap the sqlx sqlite feature for postgres
- `docker-compose.yml` — add postgres service, add `DATABASE_URL` to anansi env
- `src/db.rs` — pool type, connection setup, row type aliases
- `migrations/` — rewrite all migration SQL in PostgreSQL dialect
- `src/config.rs` — replace `db_path` with `database_url`

Every query in the codebase uses `sqlx::query()` with string parameters. PostgreSQL uses `$1`, `$2`, ... positional parameters instead of SQLite's `?`. This is the most mechanical part of the migration — a grep-and-replace across every file that calls `sqlx::query`.

**Implementation order — follow exactly:**

1. Add postgres service to `docker-compose.yml`
2. Update `Cargo.toml` — swap sqlite feature for postgres
3. Update `src/config.rs` — replace `db_path` with `database_url`
4. Rewrite `src/db.rs` — pool type, connection setup, remove SQLite pragmas
5. Rewrite `migrations/` — PostgreSQL dialect + tsvector FTS
6. Update every `sqlx::query` call to use `$N` parameters
7. Remove SQLite-only code paths
8. Update `docker-compose.yml` health check and datasette → pgAdmin or drop datasette
9. Verify: `cargo build`, deploy, smoke test

---

## Step 1 — docker-compose.yml

Replace the entire file:

```yaml
version: "3.8"

services:
  db:
    image: pgvector/pgvector:pg16
    restart: unless-stopped
    environment:
      POSTGRES_DB: anansi
      POSTGRES_USER: anansi
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U anansi"]
      interval: 5s
      timeout: 5s
      retries: 10

  anansi:
    image: ghcr.io/jamespakele/ai-anansi-v2:latest
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://anansi:${POSTGRES_PASSWORD}@db:5432/anansi
      ANANSI_GEMINI_API_KEY: ${ANANSI_GEMINI_API_KEY}
      ANANSI_PUBLIC_URL: https://anansi.pakele.ai
    volumes:
      - ./data/anansi:/data/anansi
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.anansi.rule=Host(`anansi.pakele.ai`)"
      - "traefik.http.routers.anansi.entrypoints=websecure"
      - "traefik.http.routers.anansi.tls.certresolver=letsencrypt"
      - "traefik.http.services.anansi.loadbalancer.server.port=3000"

  traefik:
    image: traefik:v3.0
    restart: unless-stopped
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
      - "--certificatesresolvers.letsencrypt.acme.email=james@pakele.ai"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./data/letsencrypt:/letsencrypt
```

Add `POSTGRES_PASSWORD` to the `.env` file on the VPS (create `/docker/ai-anansi-v2/.env` if it doesn't exist):

```
POSTGRES_PASSWORD=<generate a strong password>
ANANSI_GEMINI_API_KEY=<existing key>
```

**Note:** Datasette is removed — it depended on reading the SQLite file directly and showed stale data due to WAL mode. For DB inspection, use `docker exec -it ai-anansi-v2-db-1 psql -U anansi anansi` or connect pgAdmin to the postgres port (expose `5432:5432` temporarily if needed).

---

## Step 2 — Cargo.toml

Change the `sqlx` dependency features and add the `pgvector` crate:

```toml
[dependencies]
# Before:
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# After:
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "migrate"] }
pgvector = { version = "0.4", features = ["sqlx"] }
```

The `pgvector` crate provides the `Vector` newtype that sqlx uses to bind/retrieve `vector(768)` columns.

Also remove any `rusqlite` dependency if present.

---

## Step 3 — src/config.rs

Replace `db_path` with `database_url` everywhere in the config struct and loader.

Find the `Config` or `AnansiConfig` struct. Replace:
```rust
// Before
pub db_path: PathBuf,   // or String

// After
pub database_url: String,
```

In the loader, read from the `DATABASE_URL` environment variable (which docker-compose injects):
```rust
database_url: std::env::var("DATABASE_URL")
    .unwrap_or_else(|_| "postgres://anansi:anansi@localhost:5432/anansi".to_string()),
```

Remove any SQLite-specific config (WAL mode settings, `db_path` fallback logic).

---

## Step 4 — src/db.rs

This file needs the most change. Replace the pool type and connection setup throughout.

### 4.1 — Change type aliases and imports

```rust
// Before
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow, SqliteConnectOptions};
use sqlx::{Row, ConnectOptions};
pub type DbPool = SqlitePool;

// After
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;
pub type DbPool = PgPool;
```

### 4.2 — Replace `open_and_migrate` with `connect_and_migrate`

```rust
// Before: open_and_migrate took a PathBuf
pub async fn open_and_migrate(db_path: &Path) -> Result<DbPool> {
    // SQLite-specific: SqliteConnectOptions, WAL mode, foreign_keys pragma
}

// After: connect_and_migrate takes a connection URL string
pub async fn connect_and_migrate(database_url: &str) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .with_context(|| format!("connecting to postgres: {database_url}"))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .with_context(|| "running migrations")?;

    Ok(pool)
}
```

### 4.3 — Replace `row_to_source` row type

```rust
// Before
fn row_to_source(row: sqlx::sqlite::SqliteRow) -> SourceRecord {

// After
fn row_to_source(row: PgRow) -> SourceRecord {
```

Same for any other `row_to_*` helpers.

### 4.4 — Update all query parameter placeholders

Every `sqlx::query(...)` call that uses `?` must change to `$1`, `$2`, etc. This is a systematic grep-replace.

**Rule:** The Nth `.bind(...)` call corresponds to `$N` in the query string. Count from 1.

Example:
```rust
// Before (SQLite)
sqlx::query("INSERT INTO sources (id, source_path, title) VALUES (?,?,?)")
    .bind(&rec.id)
    .bind(&rec.source_path)
    .bind(&rec.title)

// After (PostgreSQL)
sqlx::query("INSERT INTO sources (id, source_path, title) VALUES ($1,$2,$3)")
    .bind(&rec.id)
    .bind(&rec.source_path)
    .bind(&rec.title)
```

Do this for ALL queries in `src/db.rs`. Then do the same for all queries in `src/mcp.rs`, `src/atomized_ingest.rs`, `src/merger.rs`, `src/pipeline.rs`.

**Tip:** Run `cargo build` after each file — the compiler won't catch wrong param counts but will catch type errors. Do one file at a time.

### 4.5 — Integer types

PostgreSQL is strict about integer types. `i64` in Rust maps to `BIGINT` in PostgreSQL. The `has_conflicts`, `preprocessed_toc`, `source_count` fields are `i64` in the structs — use `BIGINT` in the PostgreSQL schema (Step 5).

---

## Step 5 — migrations/

Delete all existing migration files. Create new ones in PostgreSQL dialect.

### migrations/0001_schema.sql

```sql
-- Core Anansi schema for PostgreSQL + pgvector

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS sources (
    id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    title TEXT,
    source_type TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    toc_hash TEXT,
    preprocessed_toc BIGINT NOT NULL DEFAULT 0,
    toc_author TEXT,
    toc_generated_at TEXT,
    ingested_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sources_hash ON sources(content_hash);

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    name TEXT NOT NULL,
    match_key TEXT NOT NULL UNIQUE,
    lede TEXT,
    why TEXT,
    content TEXT,
    has_conflicts BIGINT NOT NULL DEFAULT 0,
    conflicts_updated_at TEXT,
    merge_category TEXT NOT NULL,
    created_from TEXT NOT NULL REFERENCES sources(id),
    source_count BIGINT NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Full-text search — auto-maintained computed column, no triggers
    fts_vector tsvector GENERATED ALWAYS AS (
        to_tsvector('english',
            COALESCE(name, '') || ' ' ||
            COALESCE(lede, '') || ' ' ||
            COALESCE(why, '') || ' ' ||
            COALESCE(content, '')
        )
    ) STORED
);

CREATE INDEX IF NOT EXISTS idx_notes_match ON notes(match_key);
CREATE INDEX IF NOT EXISTS idx_notes_type ON notes(entity_type);
CREATE INDEX IF NOT EXISTS idx_notes_created_from ON notes(created_from);
CREATE INDEX IF NOT EXISTS idx_notes_fts ON notes USING gin(fts_vector);

-- Embeddings in a separate table — keeps notes rows thin.
-- ON DELETE CASCADE: purging a note automatically removes its embedding.
-- Composite PK on (note_id, model) allows storing multiple embedding models
-- side-by-side without a schema change.
CREATE TABLE IF NOT EXISTS note_embeddings (
    note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    model   TEXT NOT NULL DEFAULT 'text-embedding-004',
    embedding vector(768) NOT NULL,
    embedded_at TEXT NOT NULL,
    PRIMARY KEY (note_id, model)
);

-- HNSW index for fast approximate cosine-similarity search
CREATE INDEX IF NOT EXISTS idx_note_embeddings_hnsw ON note_embeddings
    USING hnsw(embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE TABLE IF NOT EXISTS source_contributions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(id),
    note_id TEXT NOT NULL REFERENCES notes(id),
    toc_address TEXT,
    hint TEXT,
    contribution_type TEXT NOT NULL,
    payload TEXT,
    contributed_at TEXT NOT NULL,
    UNIQUE (source_id, note_id)
);

CREATE INDEX IF NOT EXISTS idx_contrib_source ON source_contributions(source_id);
CREATE INDEX IF NOT EXISTS idx_contrib_note ON source_contributions(note_id);

CREATE TABLE IF NOT EXISTS edges (
    id TEXT PRIMARY KEY,
    source_note_id TEXT NOT NULL REFERENCES notes(id),
    target_note_id TEXT NOT NULL REFERENCES notes(id),
    edge_type TEXT NOT NULL,
    why TEXT,
    from_source TEXT NOT NULL REFERENCES sources(id),
    weight REAL NOT NULL DEFAULT 1.0,
    metadata TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (source_note_id, target_note_id, edge_type, from_source)
);

CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_note_id);
CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_note_id);
CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(edge_type);

-- Manual source: sentinel for human-authored edges
INSERT INTO sources (id, source_path, title, source_type, content_hash, ingested_at)
VALUES (
    '00000000-0000-0000-0000-000000000000',
    'manual',
    'Manual edges',
    'manual',
    'manual',
    NOW()::TEXT
) ON CONFLICT (id) DO NOTHING;
```

**Notes on the schema:**
- `CREATE EXTENSION IF NOT EXISTS vector` enables pgvector. The `pgvector/pgvector:pg16` Docker image ships the extension pre-installed.
- `fts_vector` is a `GENERATED ALWAYS AS ... STORED` computed column. No triggers — PostgreSQL maintains it automatically on every write.
- `note_embeddings` is a separate table — keeps `notes` rows thin. All regular note reads (search, lookup, ingest) never touch the embedding data.
- `ON DELETE CASCADE` on `note_embeddings.note_id` means purging a note automatically removes its embedding — no manual cleanup in the purge tool.
- `PRIMARY KEY (note_id, model)` allows storing embeddings from multiple models (e.g., swap from `text-embedding-004` to a future model without a migration — just add a new row).
- HNSW params: `m=16, ef_construction=64` are the defaults, suitable for up to ~100k notes. Tune upward if needed.
- `REAL` → PostgreSQL `DOUBLE PRECISION` — sqlx handles this automatically.
- `ON CONFLICT (id) DO NOTHING` replaces SQLite's `INSERT OR IGNORE`.
- The FTS5 virtual table, its shadow tables, and all three trigger functions are gone entirely.

**Delete all other migration files** (0002, 0003, 0004, 0005) — they are SQLite-specific. The new 0001 is the complete schema from scratch.

---

## Step 6 — Update search_notes in src/mcp.rs

The `anansi_search` tool used FTS5 `MATCH`. Replace with PostgreSQL `@@` operator:

```rust
// Before (FTS5)
let rows = sqlx::query(
    "SELECT * FROM notes WHERE notes_fts MATCH ? ORDER BY rank LIMIT ?"
)
.bind(&query)
.bind(limit)

// After (PostgreSQL tsvector)
let rows = sqlx::query(
    "SELECT * FROM notes \
     WHERE fts_vector @@ plainto_tsquery('english', $1) \
     ORDER BY ts_rank(fts_vector, plainto_tsquery('english', $1)) DESC \
     LIMIT $2"
)
.bind(&query)
.bind(limit as i64)
```

Also update the `anansi_search` tool's description in the MCP schema to remove the FTS5-specific language.

---

## Step 7 — Update src/mcp.rs — purge tool

Remove the `INSERT INTO notes_fts(notes_fts) VALUES('rebuild')` call from `tool_purge` — it no longer exists. The purge loop becomes:

```rust
// Delete notes directly. tsvector and embedding columns are managed by PG.
for nid in &note_ids {
    if let Ok(r) = sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(nid)
        .execute(&ctx.db)
        .await
    {
        notes_deleted += r.rows_affected();
    }
}
// No FTS rebuild, no trigger management — PostgreSQL handles everything.
```

---

## Step 7b — Add anansi_embed and anansi_search_semantic to src/mcp.rs

Two new MCP tools for the semantic layer:

### `anansi_embed` — generate and store embeddings

Calls Gemini `text-embedding-004` and writes the resulting vector to `notes.embedding` for one note or a batch. Called by the Researcher cowork skill after ingestion.

**Input schema:**
```json
{
  "note_id": "<uuid>  — embed a single note",
  "batch": "<optional: true>  — embed all notes where embedding IS NULL (max 100 per call)"
}
```

**Implementation sketch:**
```rust
async fn tool_embed(ctx: &AppCtx, id: Value, params: Value) -> Value {
    use pgvector::Vector;

    // Determine which note IDs to embed
    let note_ids: Vec<String> = if params["batch"].as_bool().unwrap_or(false) {
        sqlx::query_scalar(
            "SELECT id FROM notes WHERE embedding IS NULL LIMIT 100"
        )
        .fetch_all(&ctx.db).await.unwrap_or_default()
    } else {
        vec![params["note_id"].as_str().unwrap_or("").to_string()]
    };

    let model = "text-embedding-004";
    let mut embedded = 0usize;
    for nid in &note_ids {
        // Fetch note text
        let text: Option<String> = sqlx::query_scalar(
            "SELECT name || ' ' || COALESCE(lede,'') || ' ' || COALESCE(why,'') \
             || ' ' || COALESCE(content,'') FROM notes WHERE id = $1"
        )
        .bind(&nid)
        .fetch_optional(&ctx.db).await.unwrap_or(None);

        let Some(text) = text else { continue; };

        // Call Gemini embeddings API
        let vec = gemini_embed(&ctx.config, &text).await?;
        let vector = Vector::from(vec);

        // Upsert into note_embeddings (separate table, notes stays thin)
        sqlx::query(
            "INSERT INTO note_embeddings (note_id, model, embedding, embedded_at) \
             VALUES ($1, $2, $3, NOW()::TEXT) \
             ON CONFLICT (note_id, model) DO UPDATE SET embedding = EXCLUDED.embedding, \
             embedded_at = EXCLUDED.embedded_at"
        )
        .bind(&nid)
        .bind(model)
        .bind(&vector)
        .execute(&ctx.db).await.ok();
        embedded += 1;
    }

    json_rpc_ok(id, json!({ "content": [{ "type": "text",
        "text": format!("{embedded} notes embedded") }] }))
}
```

Add a `gemini_embed(config, text) -> Result<Vec<f32>>` helper in `src/llm.rs` (or a new `src/embed.rs`) that calls:
```
POST https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent
{
  "model": "models/text-embedding-004",
  "content": { "parts": [{ "text": "<text>" }] },
  "taskType": "RETRIEVAL_DOCUMENT"
}
```
The response contains `embedding.values` — a `Vec<f32>` of length 768.

### `anansi_search_semantic` — cosine-similarity search

**Input schema:**
```json
{
  "query": "<natural language query>",
  "limit": 10,
  "model": "text-embedding-004  (optional, defaults to text-embedding-004)"
}
```

**Implementation sketch:**
```rust
async fn tool_search_semantic(ctx: &AppCtx, id: Value, params: Value) -> Value {
    use pgvector::Vector;

    let query = params["query"].as_str().unwrap_or("");
    let limit = params["limit"].as_i64().unwrap_or(10).min(50);

    // Embed the query
    let vec = gemini_embed(&ctx.config, query).await?;
    let vector = Vector::from(vec);

    let model = params["model"].as_str().unwrap_or("text-embedding-004");

    // Cosine similarity search — <=> is pgvector's cosine distance operator.
    // JOIN to note_embeddings keeps the notes table scan-free.
    let rows = sqlx::query(
        "SELECT n.id, n.name, n.entity_type, n.lede, n.match_key, \
         1 - (ne.embedding <=> $1) AS similarity \
         FROM note_embeddings ne \
         JOIN notes n ON n.id = ne.note_id \
         WHERE ne.model = $2 \
         ORDER BY ne.embedding <=> $1 \
         LIMIT $3"
    )
    .bind(&vector)
    .bind(model)
    .bind(limit)
    .fetch_all(&ctx.db)
    .await
    .unwrap_or_default();

    // Format results same as anansi_search
    // ...
}
```

Register both tools in the MCP tool dispatch and schema in `src/mcp.rs`.

---

## Step 8 — Update src/main.rs call sites

Find `open_and_migrate` calls and replace with `connect_and_migrate`:

```rust
// Before
let pool = db::open_and_migrate(&config.db_path).await?;

// After
let pool = db::connect_and_migrate(&config.database_url).await?;
```

---

## Step 9 — Data migration (one-time, on first deploy)

The existing SQLite data (notes, edges, sources) needs to be migrated to PostgreSQL on the first deploy. Do this manually after `docker-compose up`:

```bash
# On VPS after new stack is up:
# 1. Export SQLite data
docker exec ai-anansi-v2-anansi-1 sh -c \
  "sqlite3 /data/anansi/web.db .dump" > /tmp/anansi_dump.sql

# 2. The dump will be SQLite SQL — do NOT import directly into PostgreSQL.
# Instead, export as CSV and import table-by-table:
sqlite3 /docker/ai-anansi-v2/data/anansi/web.db \
  ".mode csv" ".headers on" ".output /tmp/sources.csv" "SELECT * FROM sources;"
sqlite3 /docker/ai-anansi-v2/data/anansi/web.db \
  ".mode csv" ".headers on" ".output /tmp/notes.csv" \
  "SELECT id,entity_type,name,match_key,lede,why,content,has_conflicts,conflicts_updated_at,merge_category,created_from,source_count,created_at,updated_at FROM notes;"
sqlite3 /docker/ai-anansi-v2/data/anansi/web.db \
  ".mode csv" ".headers on" ".output /tmp/edges.csv" "SELECT * FROM edges;"
sqlite3 /docker/ai-anansi-v2/data/anansi/web.db \
  ".mode csv" ".headers on" ".output /tmp/contribs.csv" "SELECT * FROM source_contributions;"

# 3. Import into PostgreSQL (run migrations first via anansi startup)
docker exec -i ai-anansi-v2-db-1 psql -U anansi anansi << 'EOF'
\COPY sources FROM '/tmp/sources.csv' CSV HEADER;
\COPY notes(id,entity_type,name,match_key,lede,why,content,has_conflicts,conflicts_updated_at,merge_category,created_from,source_count,created_at,updated_at) FROM '/tmp/notes.csv' CSV HEADER;
\COPY source_contributions FROM '/tmp/contribs.csv' CSV HEADER;
\COPY edges FROM '/tmp/edges.csv' CSV HEADER;
EOF
```

If starting fresh (empty DB is fine), skip the data migration entirely — just run `docker-compose up` and start ingesting.

---

## Step 10 — Remove SQLite artifacts

After the migration is confirmed working:

```bash
# On VPS — backup then remove old SQLite files
mv /docker/ai-anansi-v2/data/anansi/web.db /docker/ai-anansi-v2/data/anansi/web.db.bak
rm -f /docker/ai-anansi-v2/data/anansi/web.db-wal
rm -f /docker/ai-anansi-v2/data/anansi/web.db-shm
```

In the Dockerfile, remove the `libsqlite3-dev` or bundled sqlite dependency if present.

---

## What NOT to change

- `src/pipeline.rs` — pipeline logic, pass 1/3/4 LLM calls
- `src/merger.rs` — merge strategies
- `src/atomized_ingest.rs` — atomized ingest logic
- `src/atomized_parser.rs` — parser
- `src/mcp.rs` tool handlers — only change query parameter syntax (`?` → `$N`) and the search query
- `src/template.rs`, `src/prompt.rs` — unchanged
- `templates/`, `%Rules/`, `prompts/` — unchanged

---

## Key implementation notes

**`?` → `$N` scope:** Every file that calls `sqlx::query(...)` needs this change. Files affected: `src/db.rs`, `src/mcp.rs`, `src/atomized_ingest.rs`, `src/merger.rs`, `src/pipeline.rs`. Use `grep -n "\.bind(" src/**/*.rs` to find every bind call, then count from `$1` per query.

**`rows_affected()` return type:** PostgreSQL returns `u64` from `rows_affected()` — same as SQLite. No change needed.

**`INSERT OR IGNORE` → `INSERT ... ON CONFLICT DO NOTHING`:** SQLite-specific syntax. Replace everywhere. Affected: `insert_note` (uses COALESCE upsert), `insert_source`, `insert_edge_if_not_exists`, manual source seed.

**COALESCE upsert pattern:** The `insert_note` function uses a `COALESCE` trick for SQLite. In PostgreSQL, use `INSERT ... ON CONFLICT (match_key) DO UPDATE SET ...` or `ON CONFLICT DO NOTHING`. Review `insert_note` carefully.

**`INTEGER` vs `BIGINT`:** SQLite uses `INTEGER` (which is i64). PostgreSQL `INTEGER` is i32. The Rust structs use `i64` for `has_conflicts`, `preprocessed_toc`, `source_count`. Use `BIGINT` in the PostgreSQL schema (already done in Step 5).

**`sqlx::migrate!` macro:** Requires the `migrate` feature in sqlx. Add it: `features = ["runtime-tokio", "postgres", "migrate"]` in Cargo.toml.

**Test helpers:** `merger.rs` and other test modules call `open_and_migrate`. Update to `connect_and_migrate` with a test database URL, or use `sqlx::testing` with a PostgreSQL test database. For CI, set `DATABASE_URL=postgres://anansi:anansi@localhost:5432/anansi_test`.

**Generated column support:** PostgreSQL 12+ supports `GENERATED ALWAYS AS ... STORED`. The VPS uses `postgres:16-alpine` so this is available.

---

## Why not Supabase

Supabase is PostgreSQL under the hood, but it adds:
- An external network hop (latency) on every query
- Authentication complexity (service_role keys, JWT)
- Free tier limits (500MB storage, pausing after inactivity)
- Dependency on a third-party service for a local-first tool

The self-hosted postgres container on the VPS gives us everything we need: reliability, full control, pgvector enabled from day one, and zero additional operational cost.
