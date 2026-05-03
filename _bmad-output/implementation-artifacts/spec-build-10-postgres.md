---
title: 'Anansi v2 — Build 10: SQLite → PostgreSQL + pgvector'
type: 'feature'
created: '2026-05-02'
status: 'ready-for-dev'
baseline_commit: '37b8ae0'
context:
  - docs/anansi-v2-build-10-postgres.md
  - docs/anansi-v2-spec.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** SQLite WAL + FTS5 content-backed tables have caused repeated index corruption in production (twice in a single day), requiring manual stop-container-repair-restart cycles. The architecture is fundamentally fragile for a write-heavy knowledge pipeline.

**Approach:** Replace the SQLite backing store with a self-hosted `pgvector/pgvector:pg16` Docker container. Swap the `sqlx` sqlite driver for postgres, rewrite all migrations in PostgreSQL dialect (replacing FTS5 with a `GENERATED ALWAYS AS` tsvector column and a separate `note_embeddings` table for vector search), and update every SQL query from `?` placeholders to `$N`. Add two new MCP tools — `anansi_embed` and `anansi_search_semantic` — for Gemini-powered vector search.

## Boundaries & Constraints

**Always:**
- Keep the `notes` table thin — the `embedding vector(768)` column lives in `note_embeddings` with `ON DELETE CASCADE`, not on `notes` directly
- Use `pgvector/pgvector:pg16` (not `postgres:16-alpine`) — the pgvector extension must be present from first boot
- All existing MCP tools (`anansi_search`, `anansi_purge`, `anansi_ingest`, `anansi_remember`) must continue to function after the migration
- Migration files 0002–0005 are SQLite-specific and must be deleted; `0001_schema.sql` is the single authoritative schema
- The `MANUAL_SOURCE_ID` sentinel row seed moves from Rust `seed_manual_source()` into `0001_schema.sql` using `ON CONFLICT (id) DO NOTHING`

**Ask First:**
- Whether to attempt a live data migration from the existing SQLite DB (CSV export → `\COPY` into PG) or start with a fresh empty database
- If the Gemini `text-embedding-004` model is the correct embedding choice or if a different model should be used

**Never:**
- Use Supabase — this is self-hosted only
- Expose port 5432 publicly — postgres is internal to the docker network only
- Keep Datasette — it cannot read PostgreSQL and showed stale WAL data anyway; remove it
- Use `sqlx::query!` compile-time macros — the project uses runtime `sqlx::query()` throughout; maintain that pattern

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Cold start (empty DB) | `DATABASE_URL` set, postgres healthy | Migrations run, sentinel source seeded, MCP server ready | Startup fails with clear error if DB unreachable |
| `anansi_search` keyword query | Search term string | Results ranked by `ts_rank(fts_vector, ...)` | Returns empty array, never panics |
| `anansi_purge` source deletion | Valid `source_id` | Notes/edges/contribs deleted; `note_embeddings` rows auto-deleted via CASCADE | Reports rows deleted; no FTS rebuild step |
| `anansi_embed` single note | `{ "note_id": "<uuid>" }` | Embedding upserted in `note_embeddings` | Returns error text if Gemini call fails; note is unchanged |
| `anansi_embed` batch | `{ "batch": true }` | Up to 100 notes without an entry in `note_embeddings` (for the default model) embedded in sequence | Partial success reported; does not abort on single failure |
| `anansi_search_semantic` | `{ "query": "...", "limit": 10 }` | Notes ranked by cosine similarity descending | Returns empty array if no embeddings exist yet |
| DB connection lost mid-ingest | Network drop to postgres | Ingest task errors out cleanly | Error surfaced via MCP response; no corruption |

</frozen-after-approval>

## Code Map

- `Cargo.toml` — swap `sqlx` features from `sqlite` to `postgres,migrate`; add `pgvector = { version = "0.4", features = ["sqlx"] }` (`reqwest` already present with `json,rustls-tls` features — no change needed)
- `docker-compose.yml` — add `pgvector/pgvector:pg16` service; remove `datasette` service; add `DATABASE_URL` env to anansi service; **keep port `3738`** (current production port)
- `src/config.rs` — replace `db_path()` method with `database_url: String` field; read from `DATABASE_URL` env var
- `src/db.rs` — primary migration target: `SqlitePool→PgPool`, `open_and_migrate→connect_and_migrate`, all `?` → `$N`, `INSERT OR IGNORE` → `ON CONFLICT DO NOTHING`, `SqliteRow` → `PgRow`, remove WAL/foreign_keys pragmas; 60 `.bind()` calls to update
- `src/main.rs` — 3 call sites for `db_path`/`open_and_migrate` → `database_url`/`connect_and_migrate`; add `mod embed;` declaration
- `src/mcp.rs` — 18 `.bind()` calls to update; remove `INSERT INTO notes_fts(notes_fts) VALUES('rebuild')` from purge; update `anansi_search` to use `fts_vector @@ plainto_tsquery`; add `anansi_embed` and `anansi_search_semantic` tool handlers and MCP schema entries
- `src/merger.rs` — 10 `.bind()` calls; `INSERT OR IGNORE INTO source_contributions` → `ON CONFLICT DO NOTHING`; test helper uses env-guard pattern
- `src/atomized_ingest.rs` — 3 direct `.bind()` calls to update; calls to `db.rs` helpers (`insert_note`, `insert_edge_if_not_exists`) already covered by `db.rs` task
- `src/embed.rs` — NEW: `pub async fn gemini_embed(api_key: &str, text: &str) -> Result<Vec<f32>>` using existing `reqwest` client, calling `text-embedding-004` REST API
- `migrations/0001_schema.sql` — REWRITE: PostgreSQL dialect, `CREATE EXTENSION vector`, tsvector generated column, `note_embeddings` table, HNSW index; remove FTS5 virtual table
- `migrations/0002_*.sql` → `migrations/0005_*.sql` — DELETE: all SQLite-specific, replaced by 0001

## Tasks & Acceptance

**Execution:**

- [ ] `Cargo.toml` — replace `sqlx` features `["runtime-tokio", "sqlite"]` with `["runtime-tokio", "postgres", "migrate"]`; add `pgvector = { version = "0.4", features = ["sqlx"] }`; remove any `rusqlite` dependency
- [ ] `docker-compose.yml` — add `db` service using `pgvector/pgvector:pg16` with healthcheck (`pg_isready -U anansi`); add `DATABASE_URL: postgres://anansi:${POSTGRES_PASSWORD}@db:5432/anansi` env to `anansi` service; add `depends_on: db: condition: service_healthy`; remove `datasette` service entirely; keep Traefik config intact including **port `3738`** and existing `ANANSI_GEMINI_API_KEY` and `ANANSI_SKILL_TOKEN` env vars; volume mount stays `./data:/data` (unified, matches current VPS layout — do NOT split into `./data/postgres` and `./data/anansi`); postgres data volume: `./data/postgres:/var/lib/postgresql/data`; add `.env` comment for `POSTGRES_PASSWORD`
- [ ] `migrations/` — delete `0002_*.sql` through `0005_*.sql`; rewrite `0001_schema.sql` in PostgreSQL dialect per the schema in `docs/anansi-v2-build-10-postgres.md` Step 5 (includes `CREATE EXTENSION IF NOT EXISTS vector`, tsvector GENERATED column, `note_embeddings` table with HNSW index, `ON CONFLICT DO NOTHING` seed for manual source)
- [ ] `src/config.rs` — remove `db_path()` helper; add `database_url: String` field to config struct; populate from `std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://anansi:anansi@localhost:5432/anansi".to_string())`
- [ ] `src/db.rs` — (1) replace imports: `sqlx::sqlite::{...}` → `sqlx::postgres::{PgPool, PgPoolOptions, PgRow}`; (2) `pub type DbPool = PgPool`; (3) rename `open_and_migrate(db_path: &Path)` → `connect_and_migrate(database_url: &str)` using `PgPoolOptions::new().max_connections(10).connect(database_url)`; (4) remove `seed_manual_source()` function (seeded in migration now); (5) replace all `?` params with `$1`, `$2`, … counting from 1 per query; (6) replace `INSERT OR IGNORE` with `INSERT ... ON CONFLICT DO NOTHING`; (7) fix `notes_fts MATCH ?` search query to `fts_vector @@ plainto_tsquery('english', $1) ORDER BY ts_rank(fts_vector, plainto_tsquery('english', $1)) DESC`; (8) update `row_to_source` and `row_to_note` to accept `PgRow`
- [ ] `src/main.rs` — replace all `config.db_path(root)` → `config.database_url`; replace all `db::open_and_migrate(...)` → `db::connect_and_migrate(...)`; remove any `std::fs::create_dir_all` calls for the SQLite data directory; add `mod embed;` at the top of the file with other module declarations
- [ ] `src/merger.rs` — update 10 `.bind()` calls from `?` to `$N`; update `INSERT OR IGNORE INTO source_contributions` → `INSERT INTO source_contributions ... ON CONFLICT (source_id, note_id) DO NOTHING`; update `open_and_migrate` test helper: add env-guard at start of each test function `if std::env::var("DATABASE_URL").is_err() { return; }` then call `connect_and_migrate(&std::env::var("DATABASE_URL").unwrap())`
- [ ] `src/atomized_ingest.rs` — update 3 `.bind()` calls from `?` to `$N`
- [ ] `src/embed.rs` — NEW file: implement `pub async fn gemini_embed(api_key: &str, text: &str) -> Result<Vec<f32>>` using the existing `reqwest` client (version 0.12, `json+rustls-tls` features); call `POST https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={api_key}` with body `{"model":"models/text-embedding-004","content":{"parts":[{"text":"..."}]},"taskType":"RETRIEVAL_DOCUMENT"}`; on non-200 response return `Err(anyhow!("embed API error: {status}"))`; parse `response["embedding"]["values"]` as `Vec<f32>`; if `values.len() != 768` return `Err(anyhow!("unexpected embedding dimension: {}", len))`; truncate input text at 8000 bytes before sending to stay within the 2048-token API limit
- [ ] `src/mcp.rs` — (1) update 18 `.bind()` calls from `?` to `$N`; (2) in `tool_purge`: keep existing deletion order (edges → contributions → notes) — `note_embeddings` rows are auto-deleted via `ON DELETE CASCADE` when notes are deleted; remove the `INSERT INTO notes_fts(notes_fts) VALUES('rebuild')` line; (3) update `anansi_search` SQL to `SELECT * FROM notes WHERE fts_vector @@ plainto_tsquery('english', $1) ORDER BY ts_rank(fts_vector, plainto_tsquery('english', $1)) DESC LIMIT $2` — the `ts_rank` score is used only for ordering, not returned in the result struct; (4) remove Datasette reference from `anansi_purge` tool description; (5) add `anansi_embed` and `anansi_search_semantic` to MCP schema JSON (`tools/list` handler) and to the `tools/call` dispatch match arm; (6) implement `tool_embed`: accepts `{note_id}` or `{batch:true}`, gets API key via `ctx.config.llm.gemini.as_ref().and_then(|g| g.api_key.as_deref()).ok_or(anyhow!("no Gemini API key"))`, batch query uses `SELECT id FROM notes WHERE id NOT IN (SELECT note_id FROM note_embeddings WHERE model = 'text-embedding-004') LIMIT 100`, upserts result into `note_embeddings`; (7) implement `tool_search_semantic`: accepts `{query, limit?, model?}`, embeds query, runs `SELECT n.id, n.name, n.entity_type, n.lede, n.match_key, 1-(ne.embedding<=>$1) AS similarity FROM note_embeddings ne JOIN notes n ON n.id=ne.note_id WHERE ne.model=$2 ORDER BY ne.embedding<=>$1 LIMIT $3`
- [ ] `cargo build` — must compile clean with zero errors after all changes
- [ ] `cargo test` — all existing tests must pass; update `merger.rs` test helper to skip gracefully if `DATABASE_URL` not set in test environment

**Acceptance Criteria:**
- Given a fresh VPS deploy with `DATABASE_URL` set, when the anansi container starts, then `docker compose logs anansi` shows "Applied N migration(s)" and the MCP server starts on port 3738
- Given a keyword search via `anansi_search "james"`, when the query runs, then results are returned; `grep -r 'notes_fts\|MATCH'` across `src/` returns no matches
- Given a source purge via `anansi_purge`, when the source and its notes are deleted, then `SELECT COUNT(*) FROM note_embeddings WHERE note_id IN (deleted_ids)` returns 0, and no "FTS rebuild" step is executed
- Given `anansi_embed { "batch": true }`, when called and notes exist without embeddings, then the tool response contains "N notes embedded" with N > 0, and `SELECT COUNT(*) FROM note_embeddings` increases by N
- Given `anansi_search_semantic { "query": "distributed systems" }`, when at least 2 notes are embedded, then the first result has a higher similarity score than the last result
- Given `cargo build`, when run after all changes, then output is `Finished` with zero errors and zero warnings about unused imports from sqlite

## Design Notes

**`?` → `$N` counting rule:** The Nth `.bind(value)` call in a query chain corresponds to `$N`. Count starts at 1 and resets per new `sqlx::query(...)` call. This is purely mechanical — do one file at a time and run `cargo build` after each file to catch mistakes early.

**`INSERT OR IGNORE` → PostgreSQL pattern:**
```sql
-- SQLite
INSERT OR IGNORE INTO edges (...) VALUES (...)

-- PostgreSQL
INSERT INTO edges (...) VALUES (...) ON CONFLICT (source_note_id, target_note_id, edge_type, from_source) DO NOTHING
```
Check the actual UNIQUE constraint name in the schema for each table.

**tsvector search replacement:**
```rust
// Before (FTS5)
"SELECT n.* FROM notes n JOIN notes_fts f ON n.rowid = f.rowid WHERE notes_fts MATCH ? ORDER BY rank LIMIT ?"

// After (PostgreSQL)
"SELECT * FROM notes WHERE fts_vector @@ plainto_tsquery('english', $1) ORDER BY ts_rank(fts_vector, plainto_tsquery('english', $1)) DESC LIMIT $2"
```

**pgvector bind type:** Use `pgvector::Vector` from the `pgvector` crate to bind `vector(768)` columns. Example:
```rust
use pgvector::Vector;
let api_key = ctx.config.llm.gemini.as_ref().and_then(|g| g.api_key.as_deref()).ok_or(anyhow!("no Gemini API key configured"))?;
let vec: Vec<f32> = gemini_embed(api_key, &text).await?;
let vector = Vector::from(vec);
sqlx::query("INSERT INTO note_embeddings (note_id, model, embedding, embedded_at) VALUES ($1,$2,$3,$4) ON CONFLICT (note_id, model) DO UPDATE SET embedding=EXCLUDED.embedding, embedded_at=EXCLUDED.embedded_at")
    .bind(&note_id)
    .bind("text-embedding-004")
    .bind(vector)
    .bind(now_rfc3339())
    .execute(pool).await?;
```

**Merger test helper:** `merger.rs` has an `open_and_migrate` call inside a `#[tokio::test]`. Add an env-guard at the top of each DB-dependent test: `if std::env::var("DATABASE_URL").is_err() { eprintln!("[skip] DATABASE_URL not set"); return; }`. Then call `connect_and_migrate(&std::env::var("DATABASE_URL").unwrap()).await.unwrap()`. This makes `cargo test` pass in environments without postgres rather than panicking or hanging.

**`embed.rs` API shape:**
```
POST https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={api_key}
Content-Type: application/json

{
  "model": "models/text-embedding-004",
  "content": { "parts": [{ "text": "<text>" }] },
  "taskType": "RETRIEVAL_DOCUMENT"
}

Response: { "embedding": { "values": [0.123, ...] } }   // 768 floats
```

## Spec Change Log

- **Review loopback 1 (three-layer review):** Fixed port `3000`→`3738`; added `mod embed` declaration to `src/main.rs`; corrected config API key path to `ctx.config.llm.gemini.as_ref().and_then(|g| g.api_key.as_deref())`; specified `reqwest` as HTTP client for `embed.rs` with truncation at 8000 bytes; fixed batch embed query to use `NOT IN (SELECT note_id FROM note_embeddings)` instead of `embedding IS NULL`; made purge deletion order explicit (edges→contributions→notes, CASCADE handles embeddings); clarified volume mount stays unified `./data:/data`; strengthened ACs with verifiable assertions; consolidated three `mcp.rs` tasks into one; added merger test env-guard pattern.

## Verification

**Commands:**
- `cargo build` — expected: `Finished \`dev\` profile`
- `cargo test` — expected: all tests pass (merger tests may skip if no local postgres)
- `docker compose up --build -d && docker compose logs anansi` — expected: migrations run, MCP server listening on port 3738
- `curl -X POST http://localhost:3738 -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"anansi_search","arguments":{"query":"test"}}}' -H 'Content-Type: application/json'` — expected: valid JSON-RPC response with `content` array

**Manual checks:**
- `docker exec -it <db-container> psql -U anansi anansi -c '\dt'` — should list: sources, notes, note_embeddings, source_contributions, edges
- `docker exec -it <db-container> psql -U anansi anansi -c "SELECT COUNT(*) FROM pg_extension WHERE extname='vector'"` — should return 1
- No `web.db`, `web.db-wal`, or `web.db-shm` files present in container after migration
