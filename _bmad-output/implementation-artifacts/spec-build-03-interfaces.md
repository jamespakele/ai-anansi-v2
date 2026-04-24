---
title: 'Anansi v2 — Build 03: Interfaces & Deployment'
type: 'feature'
created: '2026-04-24'
status: 'done'
baseline_commit: '8c4d4e5'
context:
  - docs/anansi-v2-build-03-interfaces.md
  - docs/anansi-v2-spec.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The anansi v2 pipeline has no external surface — no CLI, no MCP server, no deployment packaging. External users (CLI invokers, MCP clients, Claude Desktop, Cowork) have no entry points.

**Approach:** Add `src/mcp.rs` (JSON-RPC 2.0 MCP server with 5 tools and `/health`), `src/main.rs` (CLI: init/ingest/serve), Dockerfile + docker-compose per spec §21, and the `plugin/anansi.plugin` Cowork plugin (SKILL.md + `/toc` slash command).

## Boundaries & Constraints

**Always:**
- `axum`, `clap`, and `tokio` are already in `Cargo.toml` — do not re-add them.
- `include_str!` embeds the 15 templates + 4 `%Rules` files in `main.rs`; paths are `"../templates/<file>.md"` and `"../%Rules/<file>.md"` (relative to `src/main.rs`). No `include_dir` dep needed.
- Dockerfile from spec §21 verbatim, except add `COPY templates ./templates` and `COPY %Rules ./%Rules` in the build stage so `include_str!` paths resolve at compile time inside Docker.
- MCP server implements JSON-RPC 2.0 at `POST /`; dispatches `initialize`, `tools/list`, `tools/call` only; unknown method → error -32601.
- `anansi_ingest` and `anansi_relate` must check `config.server.read_only`; return JSON-RPC error "this anansi instance is read-only" if true.
- Config must apply env-var overrides: `ANANSI_OLLAMA_URL` → `config.llm.url`, `ANANSI_OLLAMA_MODEL` → `config.llm.model`, checked after TOML is loaded.
- `ServerConfig` field is `mcp_port: u16` (not `port`).
- `src/lib.rs` must declare `pub mod mcp;`.
- `cmd_ingest` builds `IngestContext` from `<root>/anansi.toml` — same setup as the integration test's `make_context` (open DB, load templates, load rules, build Vault and LlmClient).

**Ask First:**
- Adding any Cargo dependency not already in `Cargo.toml`.
- Any new DB columns or schema migrations.

**Never:**
- FTS5 search, embeddings, or vector search (v1.5/v2).
- Alpine Dockerfile variant.
- Per-pass backend routing or Codex CLI shims (v1.5).
- Read-only VPS mode beyond the existing `read_only` flag check in tool handlers.

## I/O & Edge-Case Matrix

| Scenario | Input | Expected Output | Error Handling |
|----------|-------|-----------------|----------------|
| anansi_ingest via path | `{"source_path":"/path/note.md"}` | `{"source_id":"…","outline_note_id":"…"}` | JSON-RPC error on pipeline failure |
| anansi_ingest via content | `{"content":"…","filename":"note.md"}` | Write to `<root>/note.md`, ingest, return IDs | Error if file write fails |
| anansi_ingest read-only | Any ingest call, `read_only:true` | JSON-RPC error: "this anansi instance is read-only" | — |
| anansi_search hit | `{"query":"Ian","limit":10}` | Array of `{id,name,entity_type,file_path,summary_1}` | Empty array if no hits |
| anansi_edges depth clamp | `{"id":"…","depth":99}` | BFS clamped to depth 5 | — |
| /health all OK | GET /health, Ollama up, DB up | 200 `{"status":"ok","ollama":"reachable","db":"ok","version":"0.1.0"}` | — |
| /health Ollama down | GET /health, Ollama unreachable | 503 with `{"status":"degraded","ollama":"unreachable",…}` | — |
| init idempotent | `anansi2 init /existing/root` | No-op for existing files; migrations run on existing DB | — |

</frozen-after-approval>

## Code Map

- `src/config.rs` -- add env-var override logic in `Config::load()`
- `src/db.rs` -- add `search_notes(pool, query, limit)` for anansi_search LIKE query
- `src/mcp.rs` -- new: McpState, router(), serve(), JSON-RPC dispatcher, 5 tool handlers, health handler (~280 lines)
- `src/lib.rs` -- add `pub mod mcp;`
- `src/main.rs` -- new: Cli/Command via clap, cmd_init/cmd_ingest/cmd_serve (~120 lines)
- `Dockerfile` -- spec §21 verbatim + COPY for templates/%Rules
- `docker-compose.yml` -- spec §21 verbatim
- `.dockerignore` -- lean build context
- `plugin/anansi.plugin/.claude-plugin/plugin.json` -- plugin manifest
- `plugin/anansi.plugin/skills/anansi-toc/SKILL.md` -- 9-step TOC preprocessing skill (~250 lines)
- `plugin/anansi.plugin/commands/toc.md` -- /toc slash command
- `plugin/anansi.plugin/README.md` -- plugin readme
- `README.md` -- 6-section repo readme (~150 lines)
- `.github/workflows/docker.yml` -- optional CI for ghcr.io image on version tags

## Tasks & Acceptance

**Execution:**
- [x] `src/config.rs` -- after loading TOML in `Config::load()`, check `ANANSI_OLLAMA_URL` and `ANANSI_OLLAMA_MODEL` env vars and override `config.llm.url`/`config.llm.model` if set
- [x] `src/db.rs` -- add `pub async fn search_notes(pool: &DbPool, query: &str, limit: i64) -> Result<Vec<NoteRecord>>` using `%{query}%` LIKE against `name`, `summary_1`, `summary_5`, ordered by `updated_at DESC`
- [x] `src/mcp.rs` -- implement per build-03 §1: `McpState { ctx: Arc<IngestContext> }`, `router(ctx) -> Router`, `serve(ctx, host, port)`, `handle_json_rpc` dispatching initialize/tools_list/tools_call, `handle_health` (llm.ping() + `SELECT 1`), and handlers for `anansi_ingest`, `anansi_search`, `anansi_get`, `anansi_edges` (BFS via `edges_for_note` + visited HashSet, depth clamped to 1–5), `anansi_relate`
- [x] `src/lib.rs` -- add `pub mod mcp;`
- [x] `src/main.rs` -- Cli + Command (Init, Ingest { file }, Serve); `cmd_init`: create dirs, write `include_str!`-embedded templates/rules only if absent, copy `anansi.toml.example` → `anansi.toml` if absent, run `open_and_migrate`; `cmd_ingest`: load config, build IngestContext, call `pipeline::ingest`, print result; `cmd_serve`: load config, build IngestContext, call `mcp::serve`
- [x] `Dockerfile` -- spec §21 verbatim with two additions in the build stage: `COPY templates ./templates` and `COPY %Rules ./%Rules`
- [x] `docker-compose.yml` -- spec §21 verbatim
- [x] `.dockerignore` -- `target/`, `.git/`, `anansi/`, `*.md` (except `!README.md`)
- [x] `plugin/anansi.plugin/.claude-plugin/plugin.json` -- manifest per build-03 §6
- [x] `plugin/anansi.plugin/skills/anansi-toc/SKILL.md` -- full 9-step skill per build-03 §7; inline the leaf format grammar and the parser regex quality warning
- [x] `plugin/anansi.plugin/commands/toc.md` -- slash command per build-03 §8
- [x] `plugin/anansi.plugin/README.md` -- plugin readme per build-03 §9
- [x] `README.md` -- 6-section readme: What it is, Quick start, Docker deploy, Architecture, Configuration, Development; per build-03 §10
- [x] `.github/workflows/docker.yml` -- CI workflow per build-03 §11

**Acceptance Criteria:**
- Given the full codebase, when `cargo check` is run, then it exits 0 with no errors
- Given a fresh empty directory, when `anansi2 init <path>` is run, then `web/`, `%Rules/`, `templates/`, `anansi.toml`, and a migrated `web.db` exist at `<path>`
- Given a running MCP server, when `POST /` with `{"jsonrpc":"2.0","method":"initialize","id":1}` is sent, then a response with `protocolVersion` and `serverInfo` fields is returned
- Given a running MCP server, when `POST /` with `{"jsonrpc":"2.0","method":"tools/list","id":1}` is sent, then all 5 tool names appear in the response
- Given a healthy instance, when `GET /health` is called, then 200 with `{"status":"ok"}` is returned
- Given `read_only: true` in config, when `anansi_ingest` is called via MCP, then the response contains a JSON-RPC error mentioning "read-only"
- Given a Cowork-produced source file with valid `anansi_toc` frontmatter, when `anansi2 ingest` runs on it, then `pass1_llm_called: false` appears in stdout

## Design Notes

**`anansi_get` body read:** Read `note.file_path` from disk via `std::fs::read_to_string`. Return full file content (frontmatter + body). If file is missing, return the DB record fields only — don't fail the call.

**MCP `initialize` response shape:**
```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"anansi2","version":"0.1.0"}}}
```

**`cmd_ingest` context construction:** `Config::load(&root.join("anansi.toml"))` → `open_and_migrate(&root.join(&config.paths.db_file))` → `TemplateRegistry::load(&root.join(&config.paths.templates_dir))` → `RuleRegistry::load(&root.join(&config.paths.rules_dir))` → `Vault::new(root, &config.paths.web_dir)` → `llm::build_client(&config.llm)`. Wrap llm in `Box<dyn LlmClient>`.

**`include_str!` path note:** From `src/main.rs`, templates are at `"../templates/<name>.md"` and rules at `"../%Rules/<name>.md"`. The Dockerfile `COPY` additions place these at the correct paths during the Docker build stage.

## Spec Change Log

## Verification

**Commands:**
- `cargo check` -- expected: exits 0, no errors
- `cargo test --lib` -- expected: all unit tests pass (no new unit tests required for mcp.rs — covered by acceptance checks)

## Suggested Review Order

**MCP server — entry point**

- Router wiring + `serve()` — how the Axum server is assembled and started
  [`mcp.rs:73`](../../src/mcp.rs#L73)

- JSON-RPC dispatcher — method routing: initialize / tools/list / tools/call
  [`mcp.rs:199`](../../src/mcp.rs#L199)

**MCP tools — security boundaries**

- `tool_ingest` — read-only gate, filename path-traversal guard, content write, pipeline call
  [`mcp.rs:229`](../../src/mcp.rs#L229)

- `tool_relate` — read-only gate; uses `MANUAL_SOURCE_ID` to satisfy FK constraint
  [`mcp.rs:498`](../../src/mcp.rs#L498)

**MCP tools — read paths**

- `tool_search` — LIKE with wildcard-escaped pattern, limit clamped 1–1000
  [`mcp.rs:286`](../../src/mcp.rs#L286)

- `tool_get` — reads note from DB + file content from disk; graceful file-missing fallback
  [`mcp.rs:329`](../../src/mcp.rs#L329)

- `tool_edges` — BFS with visited-node + visited-edge deduplication, depth clamped 1–5
  [`mcp.rs:387`](../../src/mcp.rs#L387)

- `handle_health` — LLM ping + SELECT 1; returns 200/503
  [`mcp.rs:552`](../../src/mcp.rs#L552)

**DB additions**

- `MANUAL_SOURCE_ID` sentinel + `seed_manual_source` — ensures manual edges have a valid FK target
  [`db.rs:10`](../../src/db.rs#L10)

- `search_notes` — LIKE with ESCAPE clause; wildcard-safe pattern construction
  [`db.rs:312`](../../src/db.rs#L312)

**CLI entry point**

- `cmd_init` — dir creation, `include_str!` seeding (skip-if-present), migration
  [`main.rs:93`](../../src/main.rs#L93)

- `cmd_ingest` / `cmd_serve` — IngestContext construction from config; same pattern both use
  [`main.rs:165`](../../src/main.rs#L165)

- Embedded seed data — 16 templates + 4 rules via `include_str!`; no runtime file dependency
  [`main.rs:21`](../../src/main.rs#L21)

**Deployment**

- Dockerfile — two-stage build; COPY for templates/%Rules; `curl` in runtime for healthcheck
  [`Dockerfile:1`](../../Dockerfile#L1)

- docker-compose.yml — bind mount, env-var defaults, healthcheck
  [`docker-compose.yml:1`](../../docker-compose.yml#L1)

**Cowork plugin**

- SKILL.md — 9-step TOC preprocessing skill; parser regex quality rule; leaf format grammar
  [`SKILL.md:1`](../../plugin/anansi.plugin/skills/anansi-toc/SKILL.md#L1)
