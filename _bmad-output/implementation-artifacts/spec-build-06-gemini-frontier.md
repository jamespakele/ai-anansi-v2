---
title: 'Anansi v2 — Build 06: Gemini Backend + Frontier Batch Mode'
type: 'feature'
created: '2026-04-26'
status: 'complete'
baseline_commit: 'b229eb6'
completed_audit: '2026-04-27'
context:
  - docs/anansi-v2-spec.md
  - docs/handoff-gemini-frontier.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The pipeline is hard-wired to Ollama. Using a frontier model requires a code change, and there is no way to collapse the N+1 LLM calls into fewer round-trips for models with large context windows.

**Approach:** Add three new backends alongside Ollama: (1) `"gemini"` — shells out to the `gemini` CLI first (Google OAuth, uses existing subscription); if CLI absent and `api_key` is set, falls back to direct Gemini REST. (2) `"openrouter"` — a single REST client that gives access to virtually every frontier model (Gemini, Claude, GPT-4, Mistral, etc.) via one API key; this is the recommended fallback when the gemini CLI is unavailable. (3) Add an optional `mode = "batch"` pipeline optimization that collapses the N Pass-3 calls + 1 Pass-4 call into a single combined call for capable frontier models; standard path is entirely unchanged.

## Boundaries & Constraints

**Always:**
- CLI path is checked first for `"gemini"` backend: try `cli_path` config then `gemini` in PATH; only attempt REST fallback if CLI absent AND `api_key` is set
- `OllamaClient` and all existing standard-mode paths are completely unchanged — no regressions
- `LlmConfig`'s existing Ollama fields stay non-optional — they already have `#[serde(default)]`; only add `gemini` and `openrouter` as optional sub-config structs
- `InferOpts::from_settings` is the constructor name — match what `src/llm.rs` already defines
- Batch prompt must use `{RULES:Downstream-Flow}` (hyphen) to match the `inject_rule` convention in `src/prompt.rs`
- Use `tokio::process::Command` for CLI subprocess (already available via `tokio = { features = ["full"] }`) — do not add Cargo dependencies
- `cargo check` and `cargo test --lib` must pass after all changes

**Ask First:**
- Any change to `Pass3Output` struct fields
- Any change to `MergeStrategy`, `NoteRecord`, or `EdgeRecord` DB-level types
- If `gemini --help` reveals the JSON output flag name differs from `--output-format json` — stop and confirm before committing to that flag

**Never:**
- Modify `OllamaClient` or the `"ollama"` branch of `build_client`
- Make Ollama fields in `LlmConfig` `Option<T>`
- Change `src/merger.rs`, `src/db.rs`, `src/mcp.rs`, or `migrations/`
- Change `prompts/pass-1-toc-extraction.md`, `prompts/pass-3-node-expansion.md`, or `prompts/pass-4-relationship-extraction.md`

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Gemini CLI, standard mode | `backend = "gemini"`, CLI in PATH, `mode` omitted | N+2 LLM calls via subprocess; vault output structurally identical to Ollama run | — |
| Gemini CLI, batch mode | `backend = "gemini"`, CLI found, `mode = "batch"` | Pass-3 loop + Pass-4 replaced by one batch call; same data reaches merger | — |
| Gemini REST fallback | `backend = "gemini"`, no CLI, `api_key` set | Falls back to direct Gemini REST client | — |
| Gemini, neither available | No CLI, no `api_key` | Error: `"gemini CLI not found in PATH and no api_key configured. Consider backend = \"openrouter\" for API-key access to Gemini and other models."` | `build_client` returns `Err` |
| OpenRouter standard | `backend = "openrouter"`, valid API key, any model | Ingest runs via OpenAI-compatible chat completions endpoint; same vault output structure | — |
| OpenRouter batch | `backend = "openrouter"`, `mode = "batch"` | Same batch behavior as Gemini batch — single combined call | — |
| Missing OpenRouter key | `backend = "openrouter"`, no key in config or env | Error: `"OpenRouter API key not found. Set [llm.openrouter] api_key or export ANANSI_OPENROUTER_API_KEY"` | `build_client` returns `Err` |
| Batch malformed JSON | Model returns non-JSON or missing `extractions` | `parse_batch_response` returns descriptive `Err`; not a panic | `anyhow!` with raw snippet |
| Preprocessed TOC + batch | `anansi_toc` frontmatter present, `mode = "batch"` | Pass-1 skipped (`pass1_llm_called: false`), batch call runs Pass-3+4 | — |
| Unknown backend | `backend = "foo"` | Error listing all valid values: `'ollama', 'gemini', 'openrouter'` | `build_client` returns `Err` |

</frozen-after-approval>

## Code Map

- `src/config.rs:6` -- root `Config` struct; add `pipeline: PipelineConfig` field
- `src/config.rs:31` -- `LlmConfig`; add `gemini: Option<GeminiConfig>` and `openrouter: Option<OpenRouterConfig>`
- `src/config.rs:92` -- `Config::load`; add `ANANSI_OPENROUTER_API_KEY` env override (no API key env var needed for Gemini CLI path — CLI handles OAuth)
- `src/config.rs:136` -- `load_example_config` test; update once example gains new sections
- `src/llm.rs:91` -- `build_client`; add `"gemini"` and `"openrouter"` match arms
- `src/llm.rs:29` -- `OllamaClient` — do NOT touch
- `src/prompt.rs:5` -- static template constants; add `PASS3_BATCH_TEMPLATE`
- `src/pipeline.rs:310` -- `parse_pass3_response`; extract inner logic to `parse_pass3_response_from_value`
- `src/pipeline.rs:446` -- `build_pass4_input`; read-only reference
- `src/pipeline.rs:640` -- Pass-3 loop; wrap in `else` branch of batch check
- `src/pipeline.rs:786` -- Pass-4 block; wrap in same `else` branch
- `prompts/pass-3-batch.md` -- NEW: combined Pass-3+4 batch prompt
- `anansi.toml.example` -- add `[llm.gemini]`, `[llm.openrouter]`, and `[pipeline]` sections

## Tasks & Acceptance

**Execution:**

- [x] `src/config.rs` -- add `GeminiConfig { cli_path: Option<String>, model: String, api_key: Option<String>, timeout_s: Option<u64> }` and `OpenRouterConfig { api_key: Option<String>, model: String, base_url: Option<String>, timeout_s: Option<u64> }`, both with `#[derive(Debug, Deserialize, Default, Clone)]`; add `gemini: Option<GeminiConfig>` and `openrouter: Option<OpenRouterConfig>` to `LlmConfig`; add `PipelineConfig { mode: Option<String>, is_batch() -> bool }` with `#[derive(Debug, Deserialize, Clone, Default)]`; add `pipeline: PipelineConfig` with `#[serde(default)]` to root `Config`

- [x] `src/config.rs` -- in `Config::load`, after existing Ollama env var block, add: if `ANANSI_OPENROUTER_API_KEY` is set, write into `config.llm.openrouter.get_or_insert_default().api_key`; add `resolve_openrouter_api_key(cfg: &OpenRouterConfig) -> Result<String>` helper (checks field then env var then returns documented error)

- [x] `src/llm.rs` -- before implementing `GeminiCliClient`, run `gemini --help` to confirm the flags for: non-interactive prompt input, JSON output format, and model selection; document confirmed flag names in a comment block above the impl

- [x] `src/llm.rs` -- add `GeminiCliClient { cli_path: String, model: String, timeout: Duration }`; `::new(cli_path, model, timeout_s)`; `LlmClient::infer`: write prompt to named temp file, spawn `tokio::process::Command` with confirmed flags and JSON output flag, capture stdout, delete temp file, return response text; `LlmClient::ping`: run `gemini --version`, return `Ok(())` on exit code 0

- [x] `src/llm.rs` -- add `GeminiRestClient { api_key, model, base_url, client: reqwest::Client }` (Gemini-direct REST fallback); `LlmClient::infer`: POST to `{base_url}/models/{model}:generateContent?key={api_key}`, set `responseMimeType: "application/json"` when `json_mode`, extract at `candidates[0].content.parts[0].text`; `LlmClient::ping`: GET `{base_url}/models?key={api_key}`

- [x] `src/llm.rs` -- add `OpenRouterClient { api_key, model, base_url, client: reqwest::Client }`; `LlmClient::infer`: POST to `{base_url}/chat/completions` with `Authorization: Bearer {api_key}`, body `{ "model": ..., "messages": [{"role": "user", "content": prompt}], "temperature": ..., "max_tokens": ... }`, add `"response_format": {"type": "json_object"}` when `json_mode`; extract at `choices[0].message.content`; `LlmClient::ping`: GET `{base_url}/models` with auth header

- [x] `src/llm.rs` -- update `build_client`: add `"gemini"` arm (CLI detection → `GeminiCliClient` if found, else `GeminiRestClient` if `api_key` set, else `Err` with message suggesting `openrouter`); add `"openrouter"` arm (`resolve_openrouter_api_key`, construct `OpenRouterClient` with `base_url` defaulting to `"https://openrouter.ai/api/v1"`); update `other =>` error to list all three valid backends

- [x] `prompts/pass-3-batch.md` -- create: combined Pass-3+4 prompt with placeholders `{RULES:Atomicity}`, `{RULES:Downstream-Flow}`, `{TEMPLATE_FIELDS}`, `{SOURCE}`, `{TOC}`, `{IMPLICIT_EDGES}`, `{RELATIONSHIP_TYPES}`; output schema: top-level `extractions` array (fields: `toc_address`, `entity_type`, `entity_name`, `match_key`, `fields`, `roster`, `summary_1`, `summary_5`, `tags`, `entities`) and `relationships` array (`source`, `relationship`, `target`, `why`); field population rules per `docs/handoff-gemini-frontier.md §3.3`

- [x] `src/prompt.rs` -- add `static PASS3_BATCH_TEMPLATE: &str = include_str!("../prompts/pass-3-batch.md")`; add `build_pass3_batch(rules, templates, toc: &str, source_body: &str, implicit_edges: &str) -> Result<String>`: parse unique entity types from TOC lines using existing leaf regex, collect `render_template_fields` output per type, join with `\n\n`, inject all 7 placeholders via existing `inject` / `inject_rule` helpers and `RELATIONSHIP_TYPES` const

- [x] `src/pipeline.rs` -- add `struct RelationshipEdge { source, target, relationship, why: String }`; refactor `parse_pass3_response` to delegate to `parse_pass3_response_from_value(v: &Value) -> Result<Pass3Output>`; add `parse_batch_response(raw: &str) -> Result<(Vec<Pass3Output>, Vec<RelationshipEdge>)>`; add `build_implicit_edges_for_batch(leaves: &[TocLeaf], outline_note_id: &str) -> String` (compute match_keys from leaves without requiring Pass3Output)

- [x] `src/pipeline.rs` -- in `ingest()` after TOC parse/validate: if `ctx.config.pipeline.is_batch()` → `build_implicit_edges_for_batch` → `build_pass3_batch` → `llm.infer` with `from_settings(&synthesis)` + `json_mode = true` → `parse_batch_response` → insert `RelationshipEdge` entries using same `find_note_by_match_key` + `insert_edge_if_not_exists` pattern as line 804; add `// TODO: [llm.batch_settings]` comment; wrap existing Pass-3 loop and Pass-4 block in `else { ... }`

- [x] `anansi.toml.example` -- append three sections: `[llm.gemini]` (cli_path commented out, model, api_key commented with note about CLI-first), `[llm.openrouter]` (api_key, model with example `"google/gemini-2.5-pro"`, base_url commented), `[pipeline]` (both mode lines commented with explanation of when to use batch)

**Acceptance Criteria:**

- Given `backend = "ollama"` with no new sections, when running an ingest, then behavior is identical to pre-build-06 — zero regressions
- Given `backend = "gemini"`, `gemini` CLI found in PATH, standard mode, when ingesting, then vault and DB outputs are structurally identical to an Ollama run
- Given `backend = "openrouter"`, valid API key, any valid model string, when ingesting, then vault and DB outputs are structurally identical to an Ollama run
- Given `backend = "gemini"` or `"openrouter"` with `mode = "batch"`, when ingesting, then note count and edge count match a standard-mode run on the same source
- Given `backend = "gemini"`, no CLI, no `api_key`, when `build_client` is called, then `Err` mentions `openrouter` as an alternative
- Given `backend = "openrouter"`, no key in config or env, when `build_client` is called, then `Err` contains the documented message
- Given `cargo check`, then exits 0
- Given `cargo test --lib`, then all existing tests pass

## Design Notes

**OpenRouter as universal fallback:** OpenRouter's API is OpenAI-compatible — `POST /v1/chat/completions` with `Authorization: Bearer`. Model strings are namespaced: `"google/gemini-2.5-pro"`, `"anthropic/claude-opus-4-7"`, `"openai/gpt-4o"`, etc. One key covers the full model catalogue. This makes it the recommended path for any frontier model access that isn't covered by a local Ollama install.

**Gemini CLI-first rationale:** The `gemini` CLI authenticates via Google OAuth and draws from subscription quota — no per-token billing on top of an existing subscription. `GeminiRestClient` is kept as a Gemini-direct fallback for headless/CI environments where OAuth isn't practical.

**Prompt delivery via temp file (CLI path):** Anansi prompts can be 20–100 KB. OS `ARG_MAX` limits make `-p <prompt>` risky for large sources. Preferred approach: write to `std::env::temp_dir()`, pass path to CLI, delete on completion. Confirm with `gemini --help` whether the flag is `--file`, `@file`, or stdin redirect.

**`RelationshipEdge` vs `EdgeRecord`:** `EdgeRecord` is the DB row type. `RelationshipEdge` is a lightweight parse target for the LLM's batch JSON output before DB insertion.

**`build_implicit_edges_for_batch`:** `build_pass4_input` requires `(NoteRecord, Pass3Output)` pairs that don't exist before the batch call. The batch variant derives match_keys from `leaf.name` + `leaf.entity_type` alone using the existing `match_key()` function. The DB-writing `derive_implicit_edges()` still runs after the batch call to persist structural edges.

**Rule name in batch prompt:** `inject_rule(&prompt, "Downstream-Flow", rules)` replaces `{RULES:Downstream-Flow}`. The template must use that exact spelling.

## Implementation Audit (2026-04-27)

Audit discovered that all build-06 tasks were **already fully implemented** in the codebase.
The spec checkboxes were aspirational at write-time; the code landed without updating this document.

| File | Status | Notes |
|------|--------|-------|
| `src/config.rs` | ✅ Complete | `GeminiConfig`, `OpenRouterConfig`, `PipelineConfig`, `is_batch()`, `resolve_openrouter_api_key`, `ANANSI_OPENROUTER_API_KEY` env override all present |
| `src/llm.rs` | ✅ Complete | `GeminiCliClient` (stdin delivery), `GeminiRestClient` (REST fallback), `OpenRouterClient` (OpenAI-compat), `build_client` with all three arms |
| `src/prompt.rs` | ✅ Complete | `PASS3_BATCH_TEMPLATE`, `build_pass3_batch`, `parse_toc_entity_types` all present |
| `prompts/pass-3-batch.md` | ✅ Complete | All 7 placeholders; `extractions[]` + `relationships[]` schema |
| `src/pipeline.rs` | ✅ Complete | `RelationshipEdge`, `parse_batch_response`, `build_implicit_edges_for_batch`, batch branch in `ingest()` |
| `anansi.toml.example` | ✅ Complete | `[llm.gemini]`, `[llm.openrouter]`, and `[pipeline]` sections present as commented examples (lines 34–63), consistent with pattern used throughout the file |

## Verification

**Automated (run 2026-04-27):**
- `cargo check` — ✅ exits 0, no warnings
- `cargo test --lib` — ✅ 36/36 tests pass

**Manual checks (pending — requires Ollama or API key):**
- Set `backend = "gemini"`, ingest a meeting summary — verify vault output matches Ollama run structure
- Set `backend = "openrouter"`, model `"google/gemini-2.5-pro"`, same source — verify matching structure
- Set `mode = "batch"` with either backend — verify note and edge counts match standard-mode run
- Remove all keys/CLI — verify clear error message with `openrouter` suggestion, no panic
