---
title: 'Anansi v2 — Build 11: Autonomous Inbox Watcher Pipeline'
type: 'feature'
created: '2026-05-03'
status: 'complete'
baseline_commit: 'c5d5421'
context:
  - _bmad-output/implementation-artifacts/spec-build-10-postgres.md
  - claude-cowork/plugins/r2-anansi.plugin/skills/r2-remember/SKILL.md
  - claude-cowork/plugins/r2-anansi.plugin/skills/para-process/SKILL.md
  - claude-cowork/plugins/r2-anansi.plugin/skills/para-projects-areas/SKILL.md
  - claude-cowork/plugins/r2-anansi.plugin/skills/para-resource-entities/SKILL.md
  - claude-cowork/plugins/r2-anansi.plugin/skills/sb-atomize/SKILL.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Knowledge ingestion currently requires manual Claude plugin invocation — a human must copy content into Claude, trigger the `r2-remember` skill chain, collect the atomized output, and then call the MCP `anansi_ingest_atomized` tool. This creates a workflow bottleneck that prevents the knowledge base from growing autonomously.

**Approach:** Implement a server-side inbox watcher as a background Tokio task inside the existing `anansi2` binary. When `inbox.enabled = true` in `anansi.toml`, the watcher polls a `/data/inbox/` directory for `.md` and `.txt` files and runs the full three-stage PARA-atomization pipeline against them — Stage 1 (para-projects-areas + para-resource-entities in parallel) → Stage 2 (sb-atomize) → Stage 3 (ingest_atomized into PostgreSQL). All intermediate files are archived to `/data/archive/<slug>-<YYYYMMDD-HHmmss>/`. LLM calls use the existing multi-backend `LlmClient` trait, with support for four backends: `gemini` (CLI device-auth + REST fallback), `openrouter` (API key), `codex` (OpenAI Codex CLI device-auth — no API key), and `ollama`.

## Boundaries & Constraints

**Always:**
- Stage 1 output files (`projects-areas-toc.md`, `projects-areas-typed.md`, `resources-toc.md`, `resources-typed.md`) must be written to the archive directory *before* Stage 2 begins. These are the checkpoint files.
- If Stage 2 (`sb-atomize`) fails and Stage 1 checkpoints already exist, re-dropping the source file into `/data/inbox/` must skip Stage 1 and retry Stage 2 directly.
- Every pipeline run writes a `pipeline.log` file of newline-delimited JSON event objects to the archive directory.
- The Codex backend means **OpenAI Codex CLI** (`codex` binary, device-auth, `~/.codex/` token store) — NOT Claude/Anthropic.
- All three LLM prompt templates are embedded at compile time via `include_str!` from `prompts/`.
- `anansi.toml` must keep backward compatibility — `[inbox]` section is optional and defaults to `enabled = false`.

**Ask First:**
- Whether to run Stage 1 passes sequentially or in parallel (parallel was chosen — `tokio::join!`).
- Whether the archive folder should be Docker-volume-managed or host-mounted separately.

**Never:**
- Block the MCP server event loop — the watcher runs in a detached `tokio::spawn` task.
- Delete Stage 1 checkpoint files on Stage 2 failure.
- Re-process a file that already has all checkpoint files AND a completed atomized file (full skip guard).
- Expose inbox/archive paths on the network.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| File dropped into inbox | `<slug>.md` file present | Archive dir created, source moved, Stage 1 runs | Error logged to pipeline.log; source stays in archive |
| Stage 1a LLM output malformed | `<<<FILE:` markers missing | Stage 1 fails; checkpoint files NOT written | Log `stage_end ok=false`; pipeline_end failed_stage=para-projects-areas |
| Stage 1 checkpoints exist; Stage 2 previously failed | Source re-dropped into inbox | Stage 1 skipped; Stage 2 retried | Stage 2 retry proceeds from checkpoint files |
| Stage 2 fails | sb-atomize LLM error | Checkpoint files preserved; atomized file NOT written | Error logged; user instructed to retry via MCP |
| Stage 3 ingest fails | DB error | atomized.md preserved in archive | Error logged; user instructed to use `anansi_ingest_atomized` via MCP |
| File already fully processed | Source re-dropped, all checkpoints + atomized file present | Full skip | Log "[inbox] already fully processed" |
| `inbox.enabled = false` | Default config | Watcher not spawned; MCP server runs normally | Log "inbox watcher disabled" |
| `codex` backend selected | `codex` binary not in PATH | `build_client_for_backend` returns Err | Pipeline logs llm_init failure |
| Two files dropped simultaneously | Two `.md` files in inbox | Both processed sequentially in scan order | Each gets its own archive dir and log |

</frozen-after-approval>

## Code Map

- `src/inbox.rs` — NEW: background watcher task (`run_inbox_watcher`), file scanner, per-file pipeline orchestrator, prompt builders, two-file LLM output parser, slug/title utilities, JSON event logger
- `src/config.rs` — add `InboxConfig` struct (enabled, watch_dir, archive_dir, poll_interval_secs, llm_backend override); add to `Config` as `#[serde(default)]`; wire into `Config::default()`
- `src/llm.rs` — add `CodexCliClient` (shells out to `codex -q`, stdin/stdout, device-auth); refactor `build_client` into `build_client` + `build_client_for_backend(backend: &str, config: &LlmConfig)` for per-stage routing
- `src/lib.rs` — add `pub mod inbox;`
- `src/main.rs` — in `cmd_serve`: after building `ctx`, check `ctx.config.inbox.enabled`; if true, `tokio::spawn(inbox::run_inbox_watcher(Arc::new(ctx.config.clone()), ctx.db.clone()))`
- `prompts/stage1a-projects-areas.txt` — para-projects-areas LLM prompt with `{{SOURCE}}`, `{{SOURCE_ID}}`, `{{GENERATED_AT}}` placeholders; outputs two `<<<FILE:>>>` delimited blocks
- `prompts/stage1b-resource-entities.txt` — para-resource-entities LLM prompt; same placeholder/delimiter convention
- `prompts/stage2-sb-atomize.txt` — sb-atomize LLM prompt with additional `{{SOURCE_TITLE}}`, `{{SOURCE_SLUG}}`, `{{PROJECTS_AREAS_TOC}}`, `{{PROJECTS_AREAS_TYPED}}`, `{{RESOURCES_TOC}}`, `{{RESOURCES_TYPED}}` placeholders
- `anansi.toml.example` — add commented `[inbox]` section and Codex CLI backend docs

## Tasks & Acceptance

**Execution:**

- [x] `src/config.rs` — add `InboxConfig` struct with `enabled: bool`, `watch_dir: String`, `archive_dir: String`, `poll_interval_secs: u64`, `llm_backend: Option<String>`; default `watch_dir="/data/inbox"`, `archive_dir="/data/archive"`, `poll_interval_secs=30`, `enabled=false`; add `#[serde(default)] pub inbox: InboxConfig` to `Config`; add `inbox: InboxConfig::default()` to `Config::default()`
- [x] `src/llm.rs` — add `CodexCliClient { cli_path: String, timeout: Duration }` implementing `LlmClient`; `infer`: spawn `codex -q`, pipe prompt to stdin, capture stdout; `ping`: run `codex --version`; extract `build_client_for_backend(backend: &str, config: &LlmConfig) -> Result<Box<dyn LlmClient>>`; `build_client` delegates to it; add `"codex"` arm that reads `ANANSI_CODEX_CLI_PATH` env or defaults to `"codex"`
- [x] `prompts/stage1a-projects-areas.txt` — translate para-projects-areas SKILL.md decision rules into a self-contained LLM prompt; include Forte definitions, Project/Area markers, precision rule, slug convention, `<<<FILE:projects-areas-toc.md>>>` + `<<<FILE:projects-areas-typed.md>>>` output format; use `{{SOURCE}}`, `{{SOURCE_ID}}`, `{{GENERATED_AT}}` placeholders
- [x] `prompts/stage1b-resource-entities.txt` — translate para-resource-entities SKILL.md; include entity type hierarchy (person→org→book→note), discussion sections, concepts, `<<<FILE:resources-typed.md>>>` + `<<<FILE:resources-toc.md>>>` output format
- [x] `prompts/stage2-sb-atomize.txt` — translate sb-atomize SKILL.md; include all block formats (project, area, discussion, person, org, note, whisper), Smart Brevity rules, Varys pass, output as `<<<FILE:<slug>-atomized.md>>>` + `<<<FILE:<slug>-toc.md>>>`
- [x] `src/inbox.rs` — implement `run_inbox_watcher(config: Arc<Config>, pool: DbPool)`: loop `scan_and_process` every `poll_interval_secs`; `scan_and_process`: `read_dir` watch_dir, filter `.md`/`.txt`, call `process_file` per entry; `process_file`: compute slug from filename stem using anansi `match_key` algorithm (lowercase, non-alphanum→space, collapse, join hyphens, truncate 60); compute `source_id` = first 16 chars of SHA-256 of trimmed content; create archive dir `<archive_dir>/<slug>-<YYYYMMDD-HHmmss>`; rename source into `archive_dir/source.md`; check checkpoint files; if no checkpoints: run Stage 1 via `tokio::join!(llm.infer(prompt_1a, ...), llm.infer(prompt_1b, ...))`, parse two-file output per pass, write checkpoint files; run Stage 2: `llm.infer(prompt_2, ...)`, parse two-file output, write `<slug>-atomized.md` + `<slug>-toc.md`; run Stage 3: `ingest_atomized(pool, &atomized, Some(&toc), Some(&source_path))`; log all events as NDJSON to `archive_dir/pipeline.log`
- [x] `src/lib.rs` — add `pub mod inbox;`
- [x] `src/main.rs` — in `cmd_serve` after building `ctx`: `if ctx.config.inbox.enabled { tokio::spawn(inbox::run_inbox_watcher(Arc::new(ctx.config.clone()), ctx.db.clone())); }`; eprintln log either spawned or disabled
- [x] `anansi.toml.example` — append commented `[inbox]` section with all fields; append Codex CLI backend docs block

**Acceptance Criteria:**
- Given `cargo check`, when run after all changes, then output is `Finished` with zero errors
- Given `inbox.enabled = false` (default), when `cmd_serve` starts, then logs show "inbox watcher disabled" and MCP server starts normally
- Given `inbox.enabled = true` and a `.md` file dropped into `/data/inbox/`, when the poll interval fires, then the file is moved to `/data/archive/<slug>-<ts>/source.md`, four Stage 1 checkpoint files appear, `<slug>-atomized.md` appears, and `pipeline.log` ends with `pipeline_end status=ok`
- Given Stage 2 fails and source is re-dropped, when poll fires again, then `pipeline.log` shows "Stage 1 checkpoint files found — skipping to Stage 2" and Stage 2 is retried
- Given `llm_backend = "codex"`, when the `codex` binary is in PATH and authenticated, then `CodexCliClient` is selected and the pipeline proceeds without an API key

## Design Notes

**Pipeline architecture (sequential within a file, parallel within Stage 1):**
```
inbox/<slug>.md
  └─ Stage 0: compute slug, source_id; create archive dir; move source
  └─ Stage 1 (parallel): tokio::join!
       ├─ 1a: para-projects-areas → projects-areas-toc.md + projects-areas-typed.md
       └─ 1b: para-resource-entities → resources-toc.md + resources-typed.md
  └─ Stage 2: sb-atomize → <slug>-atomized.md + <slug>-toc.md
  └─ Stage 3: ingest_atomized(pool, &atomized, Some(&toc), Some(&source_path))
```

**LLM output delimiter convention:**
The prompts instruct the LLM to separate its two file outputs with `<<<FILE:filename>>>` markers. The parser `parse_two_file_output` scans for all occurrences of `<<<FILE:` and slices content between marker header lines.

**Checkpoint resume logic:**
```rust
let checkpoints_exist = pa_toc.exists() && pa_typed.exists()
    && res_toc.exists() && res_typed.exists();

if checkpoints_exist && atomized_path.exists() {
    // fully done — skip
    return Ok(());
}
if !checkpoints_exist {
    // run Stage 1
}
// always run Stage 2+ if atomized doesn't exist
```

**Slug algorithm (anansi match_key):**
```rust
fn to_slug(name: &str) -> String {
    name.to_lowercase()
        .chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect::<String>()
        .split_whitespace().collect::<Vec<_>>().join("-")
        .chars().take(60).collect()
}
```

**LLM backend routing for inbox:**
`inbox.llm_backend` overrides the global `llm.backend` for pipeline stages only. Uses `llm::build_client_for_backend(backend, &config.llm)` — same config section, different backend string.

**Codex CLI vs Gemini CLI:**
Both use the same pattern: shell out to the CLI binary, pipe prompt to stdin, capture stdout. OAuth device-flow auth stores tokens in `~/.codex/` (Codex) or `~/.config/gemini/` (Gemini). No API key needed. `ANANSI_CODEX_CLI_PATH` env var overrides the binary path if not in PATH.

**pipeline.log format (NDJSON, one event per line):**
```json
{"event":"pipeline_start","slug":"my-note","archive":"/data/archive/my-note-20260503-152345","ts":"..."}
{"event":"stage_start","stage":"para-projects-areas","ts":"..."}
{"event":"stage_end","stage":"para-projects-areas","ok":true,"ts":"..."}
{"event":"stage_start","stage":"sb-atomize","ts":"..."}
{"event":"stage_end","stage":"sb-atomize","ok":true,"ts":"..."}
{"event":"stage_start","stage":"ingest","ts":"..."}
{"event":"stage_end","stage":"ingest","ok":true,"source_id":"abc123","notes_created":7,"ts":"..."}
{"event":"pipeline_end","status":"ok","source_id":"abc123","notes_created":7,"ts":"..."}
```

**ingest_atomized signature (confirmed from src/atomized_ingest.rs):**
```rust
pub async fn ingest_atomized(
    pool: &DbPool,
    content: &str,
    para_toc: Option<&str>,
    source_path: Option<&str>,
) -> Result<IngestAtomizedReport>
```
Returns `IngestAtomizedReport { source_id, notes_created, ... }`.

## Spec Change Log

- **v1 (initial):** Defined three-stage pipeline, parallel Stage 1, checkpoint resume, four LLM backends (gemini/openrouter/codex/ollama). Corrected "codex" to mean OpenAI Codex CLI (device-auth), NOT Claude — `codex auth login` stores token in `~/.codex/`. OpenRouter model default set to `openai/gpt-4.1` (not Anthropic). LLM routing refactored to `build_client_for_backend` for per-stage backend selection.

## Verification

**Commands:**
- `cargo check` — expected: `Finished` with zero errors (5 pre-existing dead_code warnings in merger.rs/mcp.rs are acceptable)
- `cargo test` — expected: all tests pass (merger tests may skip if no local postgres)
- `docker compose up --build -d && docker compose logs anansi-1 -f` — expected: "MCP server listening on 0.0.0.0:3738"; if inbox.enabled=true: "inbox watcher spawned — watching '/data/inbox'"

**Manual smoke test:**
```bash
# SSH to VPS, drop a test file
ssh root@100.118.188.14 "echo '# Test Note\n\nThis is a test of the inbox watcher pipeline.' > /docker/ai-anansi-v2/data/inbox/test-note.md"

# Watch logs
ssh root@100.118.188.14 "docker logs ai-anansi-v2-anansi-1 -f"

# Expected:
# [inbox] [test-note] Stage 1 — running para-process
# [inbox] [test-note] Stage 2 — sb-atomize
# [inbox] [test-note] Stage 3 — ingest
# [inbox] [test-note] ✓ done — N notes created (source_id=XXXXXXXXXXXXXXXX)

# Verify archive
ssh root@100.118.188.14 "ls /docker/ai-anansi-v2/data/archive/"
# Expected: test-note-YYYYMMDD-HHmmss/

# Verify DB
ssh root@100.118.188.14 "docker exec ai-anansi-v2-db-1 psql -U anansi -d anansi -c 'SELECT COUNT(*) FROM notes;'"
```

## Dev Agent Record

### Agent Model Used

Antigravity (Google Deepmind Advanced Agentic Coding) — gemini-2.5-pro

### Completion Notes List

- `cargo check` clean as of commit `510b363`
- All prompt templates embedded via `include_str!` — no runtime file reads
- `build_client_for_backend` refactored from `build_client` — backward compatible
- `IngestAtomizedReport` return type confirmed from `atomized_ingest.rs` (not a custom wrapper)
- Checkpoint resume: `atomized_path.exists() && checkpoints_exist` triggers full skip

### File List

- `src/inbox.rs` (NEW)
- `src/config.rs` (modified — InboxConfig added)
- `src/llm.rs` (modified — CodexCliClient added, build_client_for_backend extracted)
- `src/lib.rs` (modified — `pub mod inbox` added)
- `src/main.rs` (modified — watcher spawn in cmd_serve)
- `prompts/stage1a-projects-areas.txt` (NEW)
- `prompts/stage1b-resource-entities.txt` (NEW)
- `prompts/stage2-sb-atomize.txt` (NEW)
- `anansi.toml.example` (modified — inbox + codex sections added)
