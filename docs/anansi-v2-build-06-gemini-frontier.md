# Anansi v2 — Build Document 6: Gemini Backend + Frontier Batch Mode

**For:** Coding agent (Claude Code) executing against the repo produced by docs 1–5.
**Reference:** `anansi-v2-spec.md` §14 (LLM backend); `anansi-v2-build-02-pipeline.md` §4 (`src/llm.rs`); `docs/handoff-gemini-frontier.md`.
**Prerequisite:** Docs 1–5 complete. Pipeline, merger, DB, and MCP all working with Ollama backend. `cargo test` passes. `anansi_ingest` MCP tool functional.
**Output:** Gemini REST API backend wired in as a drop-in alternative to Ollama. Optional batch pipeline mode available for frontier models. Standard Ollama path unchanged — no regressions.

---

## Goal

Two additive, independent changes:

**Part A — GeminiClient:** Implements the existing `LlmClient` trait against the Gemini REST API. Set `backend = "gemini"` in `anansi.toml` to activate. Ollama remains default; this is purely additive.

**Part B — Frontier Batch Mode:** An optional pipeline optimization that collapses the N Pass 3 calls + 1 Pass 4 call into a single batch LLM call. Reduces total calls from N+2 to 2 (or 1 if a preprocessed TOC is present). Controlled by `[pipeline] mode = "batch"` in `anansi.toml`. Standard mode is still the default.

The two parts are independent — any combination of backend and mode is valid. The merger, writer, DB, and MCP layers receive identical data regardless of backend or mode; do not touch them.

---

## Part A — GeminiClient

## Files to modify (Part A)

### 1. `src/config.rs`

**Add `GeminiConfig` struct** (new — insert before or after `OllamaConfig` if one exists, otherwise near the top of the config structs):

```rust
#[derive(Debug, Deserialize, Default, Clone)]
pub struct GeminiConfig {
    /// API key — also readable from ANANSI_GEMINI_API_KEY env var
    pub api_key: Option<String>,
    /// Model name — e.g. "gemini-2.5-pro", "gemini-2.0-flash"
    pub model: String,
    /// Base URL — defaults to Google's production endpoint
    pub base_url: Option<String>,
    /// Request timeout in seconds (default 120)
    pub timeout_s: Option<u64>,
}
```

**Update `LlmConfig` struct** — Ollama fields change from required to `Option<T>` so the struct parses correctly when `backend = "gemini"` and those keys are absent. Add `gemini` sub-config field:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub backend: String,          // "ollama" | "gemini"
    // Ollama fields (kept for backward compat — now optional):
    pub url: Option<String>,
    pub model: Option<String>,
    pub n_ctx: Option<u32>,
    pub timeout_s: Option<u64>,
    // Gemini sub-config (present when backend = "gemini"):
    pub gemini: Option<GeminiConfig>,
    // Per-pass inference settings — shared by both backends:
    pub decomposition: InferSettings,
    pub extraction: InferSettings,
    pub synthesis: InferSettings,
}
```

If the existing `LlmConfig` fields `url`, `model`, `n_ctx`, `timeout_s` are currently non-optional, change them to `Option<T>` and update any call sites that `.unwrap()` them to use `.unwrap_or` with the same defaults as before (see `build_client` in `src/llm.rs`). Existing `anansi.toml` files that still provide these keys will parse without change.

**Add `resolve_gemini_api_key` helper** (private function in `src/config.rs` or `src/llm.rs` — whichever owns `build_client`):

```rust
fn resolve_gemini_api_key(cfg: &GeminiConfig) -> Result<String> {
    cfg.api_key
        .clone()
        .or_else(|| std::env::var("ANANSI_GEMINI_API_KEY").ok())
        .ok_or_else(|| anyhow!(
            "Gemini API key not found. Set [llm.gemini] api_key in anansi.toml \
             or export ANANSI_GEMINI_API_KEY"
        ))
}
```

**Add `PipelineConfig` struct** (also in `src/config.rs` — used by Part B, defined here so both parts compile together):

```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PipelineConfig {
    /// "standard" (default) or "batch"
    /// standard: N Pass 3 calls + 1 Pass 4 call
    /// batch: 1 combined call replacing Pass 3 + Pass 4
    pub mode: Option<String>,
}

impl PipelineConfig {
    pub fn is_batch(&self) -> bool {
        self.mode.as_deref() == Some("batch")
    }
}
```

**Update the top-level `Config` struct** — add `pipeline` field:

```rust
pub struct Config {
    // ... existing fields ...
    pub llm: LlmConfig,
    pub pipeline: PipelineConfig,   // new — loads from [pipeline] in anansi.toml
}
```

`PipelineConfig` derives `Default` so omitting `[pipeline]` from `anansi.toml` is safe — it defaults to standard mode.

---

### 2. `src/llm.rs`

**Add `GeminiClient`** after the existing `OllamaClient` implementation. Do NOT modify `OllamaClient` or any existing code.

```rust
pub struct GeminiClient {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String, base_url: Option<String>, timeout_s: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_s))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| {
                "https://generativelanguage.googleapis.com/v1beta".to_string()
            }),
            client,
        }
    }
}
```

**Implement `LlmClient` for `GeminiClient`:**

```rust
#[async_trait]
impl LlmClient for GeminiClient {
    async fn infer(&self, prompt: &str, opts: InferOpts) -> Result<String> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let mut generation_config = serde_json::json!({
            "temperature": opts.temperature,
            "maxOutputTokens": opts.max_tokens,
        });

        if opts.json_mode {
            generation_config["responseMimeType"] =
                serde_json::Value::String("application/json".to_string());
        }

        let body = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": generation_config
        });

        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini request failed ({status}): {text}"));
        }

        let json: serde_json::Value = resp.json().await?;

        // Response path: candidates[0].content.parts[0].text
        let text = json
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Unexpected Gemini response shape: {}", json))?;

        Ok(text.to_string())
    }

    async fn ping(&self) -> Result<()> {
        // List models endpoint — lightweight health check
        let url = format!("{}/models?key={}", self.base_url, self.api_key);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "Gemini API not reachable. Check your API key and network. Status: {}",
                resp.status()
            ));
        }
        Ok(())
    }
}
```

**Update `build_client`** to dispatch on `backend`. The `"ollama"` arm is unchanged in behavior — only update it to use `.unwrap_or` defaults now that the fields are `Option<T>`:

```rust
pub fn build_client(config: &LlmConfig) -> Result<Box<dyn LlmClient>> {
    match config.backend.as_str() {
        "ollama" => {
            let url = config.url.clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = config.model.clone()
                .unwrap_or_else(|| "qwen2.5:14b".to_string());
            let n_ctx = config.n_ctx.unwrap_or(8192);
            let timeout_s = config.timeout_s.unwrap_or(300);
            Ok(Box::new(OllamaClient::new(url, model, n_ctx, timeout_s)))
        }
        "gemini" => {
            let gcfg = config.gemini.as_ref()
                .ok_or_else(|| anyhow!(
                    "backend = 'gemini' requires [llm.gemini] config section"
                ))?;
            let api_key = resolve_gemini_api_key(gcfg)?;
            let model = if gcfg.model.is_empty() {
                "gemini-2.5-pro".to_string()
            } else {
                gcfg.model.clone()
            };
            let timeout_s = gcfg.timeout_s.unwrap_or(120);
            Ok(Box::new(GeminiClient::new(
                api_key,
                model,
                gcfg.base_url.clone(),
                timeout_s,
            )))
        }
        other => Err(anyhow!(
            "unknown LLM backend: '{other}'. Valid values: 'ollama', 'gemini'"
        )),
    }
}
```

If `resolve_gemini_api_key` is defined in `src/config.rs`, import it here. If it fits better in `src/llm.rs` alongside `build_client`, define it there instead — pick one location consistently.

---

### 3. `anansi.toml.example`

Add the Gemini and pipeline sections. Keep the existing Ollama section intact. Insert after the existing `[llm]` block:

```toml
# ──────────────────────────────────────────────────────────────
# LLM Backend — choose "ollama" (default, local) or "gemini"
# ──────────────────────────────────────────────────────────────
[llm]
backend = "ollama"   # change to "gemini" to use Gemini API

# Ollama settings (used when backend = "ollama")
url       = "http://localhost:11434"
model     = "qwen2.5:14b"
n_ctx     = 8192
timeout_s = 300

# Gemini settings (used when backend = "gemini")
# API key can also be set via ANANSI_GEMINI_API_KEY environment variable
[llm.gemini]
api_key   = ""                  # leave blank to use env var
model     = "gemini-2.5-pro"   # or "gemini-2.0-flash" for faster/cheaper
# base_url = "https://generativelanguage.googleapis.com/v1beta"  # default
timeout_s = 120

# ──────────────────────────────────────────────────────────────
# Pipeline mode
# ──────────────────────────────────────────────────────────────
[pipeline]
# mode = "standard"   # default — N Pass 3 calls + 1 Pass 4 call (Ollama-friendly)
# mode = "batch"      # 2 total calls — requires frontier model (Gemini 2.5 Pro, etc.)
```

---

## Part B — Frontier Batch Pipeline

## Files to create (Part B)

### 4. `prompts/pass-3-batch.md`

New prompt file. Used when `pipeline.is_batch()` is true. Replaces both `pass-3-node-expansion.md` and `pass-4-relationship-extraction.md` for that run — the existing prompt files are NOT deleted and continue to be used in standard mode.

Full content of the file:

```
You are extracting structured knowledge from a source document. You have been given:

1. The full source document body
2. A Table of Contents (TOC) listing every entity to extract, with type annotations and hints
3. Template field definitions for each entity type present in the TOC

Your task is to return a single JSON object containing:
- `extractions`: one entry per TOC leaf, with all template fields populated
- `relationships`: edges between entities not already implied by the TOC structure

---

## Rules

{RULES:Atomicity}

{RULES:DownstreamFlow}

---

## Entity Types and Their Fields

{TEMPLATE_FIELDS}

---

## Source Document

{SOURCE}

---

## Table of Contents

{TOC}

---

## Implicit Edges (already derived — do not repeat)

{IMPLICIT_EDGES}

---

## Output Format

Return ONLY a valid JSON object with this exact schema. No preamble, no explanation, no markdown fences.

{
  "extractions": [
    {
      "toc_address": "1.1",
      "entity_type": "person",
      "entity_name": "Ian Kitajima",
      "match_key": "person:ian-kitajima",
      "fields": {
        "field_name": "field_value"
      },
      "roster": {
        "roster_key": [
          {"name": "...", "slug": "...", "role": "..."}
        ]
      },
      "summary_1": "One sentence, source-agnostic description of this entity",
      "summary_5": "Up to five sentences, source-agnostic description",
      "tags": ["tag1", "tag2"],
      "entities": [
        {"name": "...", "entity_type": "...", "slug": "..."}
      ]
    }
  ],
  "relationships": [
    {
      "source": "person:ian-kitajima",
      "relationship": "works_at",
      "target": "organization:pichtr",
      "why": "Introduced as CTO of PICHTR in attendee list"
    }
  ]
}

## Field Population Rules

- Populate every declared field for each entity type. Use "[not mentioned]" for fields the source does not address.
- For pure-atomic types (person, concept, topic, area, note): `summary_1` and `summary_5` must describe the entity INDEPENDENT of this source — no references to "this meeting", "this document", or any specific event.
- For source-bound types (context, event, task, topic_discussion, article_section, etc.): `summary_1` may reference the source context.
- `roster` is present ONLY for container types (organization, project). Omit the key entirely for other types.
- Include ALL named entities referenced by each leaf in its `entities` array — this drives cross-leaf wikilinks and edge derivation.

## Relationship Rules

{RELATIONSHIP_TYPES}

- Do not emit relationships already listed in the Implicit Edges section above.
- `source` and `target` values must be `match_key` format: `entity_type:slug`.
- Only emit relationships you are confident about from the source text. Include `why` to explain the evidence.
```

**Note on `{TEMPLATE_FIELDS}` injection for batch mode:** When assembling this prompt, include field definitions for ALL entity types present in the TOC — not just one type. The `assemble_pass3_batch` function in `src/prompt.rs` handles this aggregation (see §5 below).

---

## Files to modify (Part B)

### 5. `src/prompt.rs`

**Add template constant** alongside the existing `PASS3_TEMPLATE` and `PASS4_TEMPLATE` constants:

```rust
const PASS3_BATCH_TEMPLATE: &str = include_str!("../prompts/pass-3-batch.md");
```

**Add `assemble_pass3_batch` function:**

```rust
pub fn assemble_pass3_batch(
    toc: &str,
    source_body: &str,
    registry: &TemplateRegistry,
    rules: &RulesContext,
    implicit_edges: &str,
) -> String {
    // Collect field blocks for every entity type appearing in the TOC leaves
    let all_entity_types: HashSet<&str> = parse_toc_entity_types(toc);

    let template_fields = all_entity_types
        .iter()
        .filter_map(|et| registry.get(et))
        .map(|t| format!(
            "### {} ({})\n{}",
            t.entity_type, t.description, t.field_blocks_text()
        ))
        .collect::<Vec<_>>()
        .join("\n\n");

    PASS3_BATCH_TEMPLATE
        .replace("{RULES:Atomicity}", &rules.atomicity)
        .replace("{RULES:DownstreamFlow}", &rules.downstream_flow)
        .replace("{TEMPLATE_FIELDS}", &template_fields)
        .replace("{SOURCE}", source_body)
        .replace("{TOC}", toc)
        .replace("{IMPLICIT_EDGES}", implicit_edges)
        .replace("{RELATIONSHIP_TYPES}", &rules.relationship_types)
}
```

**Ensure `parse_toc_entity_types` exists.** If it already exists (used by the TOC parser), import and reuse it. If it does not exist, add it:

```rust
/// Extract the set of unique entity type strings from all TOC leaf annotations.
/// Input is the raw TOC text; output is the set of `[entity_type]` values found.
fn parse_toc_entity_types(toc: &str) -> HashSet<&str> {
    // Match the [entity_type] annotation in each leaf line
    // Reuse the same regex / parse logic used by parse_toc()
    toc.lines()
        .filter_map(|line| {
            // Extract text between first '[' and ']' — matches existing leaf format
            let start = line.find('[')?;
            let end = line.find(']')?;
            if end > start { Some(&line[start + 1..end]) } else { None }
        })
        .collect()
}
```

Align the regex/parsing with how `parse_toc` already identifies entity types — do not introduce a second incompatible parser.

---

### 6. `src/pipeline.rs`

Three changes in this file.

**Change A — Refactor `parse_pass3_response` to expose a `_from_value` variant.**

This allows both the per-leaf standard path and the batch path to share deserialization logic:

```rust
/// Deserialize a Pass3Output from an already-parsed serde_json::Value.
/// Used by both parse_pass3_response (standard) and parse_batch_response (batch).
fn parse_pass3_response_from_value(v: &serde_json::Value) -> Result<Pass3Output> {
    // Move the body of the existing parse_pass3_response here,
    // operating on `v` instead of a raw string.
    // ...
}

/// Parse a raw string response from the per-leaf Pass 3 call.
fn parse_pass3_response(raw: &str) -> Result<Pass3Output> {
    let json_str = strip_markdown_fences(raw);
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Pass 3 response is not valid JSON: {e}\nRaw: {raw}"))?;
    parse_pass3_response_from_value(&v)
}
```

Similarly, ensure `parse_relationship_from_value(v: &serde_json::Value) -> Result<RelationshipEdge>` exists (refactored from the existing `parse_pass4_response` if needed). The batch parser calls this for each element in the `relationships` array.

**Change B — Add `parse_batch_response`:**

```rust
fn parse_batch_response(raw: &str) -> Result<(Vec<Pass3Output>, Vec<RelationshipEdge>)> {
    // Strip markdown fences if model wrapped JSON despite instructions
    let json_str = strip_markdown_fences(raw);

    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Batch response is not valid JSON: {e}\nRaw: {raw}"))?;

    let extractions_raw = value["extractions"]
        .as_array()
        .ok_or_else(|| anyhow!("Batch response missing 'extractions' array"))?;

    let extractions: Vec<Pass3Output> = extractions_raw
        .iter()
        .map(|v| parse_pass3_response_from_value(v))
        .collect::<Result<Vec<_>>>()?;

    let relationships_raw = value["relationships"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let relationships: Vec<RelationshipEdge> = relationships_raw
        .iter()
        .map(|v| parse_relationship_from_value(v))
        .collect::<Result<Vec<_>>>()?;

    Ok((extractions, relationships))
}
```

**Change C — Add batch branch in the main `ingest` function.**

Insert after `validate_toc_leaves(&leaves, &registry)?` and before the merger/writer calls. Replace the current hard-coded Pass 3 + Pass 4 block with a conditional:

```rust
// Pass 3 + 4 — standard mode (N + 1 calls) or batch mode (1 call)
let (extractions, relationships) = if ctx.config.pipeline.is_batch() {
    // BATCH MODE — single LLM call replacing Pass 3 and Pass 4
    // TODO: add [llm.batch_settings] config section for per-batch tuning
    let implicit_edges = derive_implicit_edges(&leaves);
    let batch_prompt = assemble_pass3_batch(
        &toc,
        &source.body,
        &registry,
        &rules,
        &implicit_edges,
    );
    let batch_opts = InferOpts::from(&ctx.config.llm.synthesis); // uses synthesis settings
    let raw = ctx.llm.infer(&batch_prompt, batch_opts).await?;
    parse_batch_response(&raw)?
} else {
    // STANDARD MODE — N Pass 3 calls + 1 Pass 4 call (unchanged behavior)
    let mut extractions: Vec<Pass3Output> = Vec::new();
    for leaf in &leaves {
        let p3_prompt = assemble_pass3(leaf, &source, &registry, &rules);
        let raw = ctx.llm.infer(&p3_prompt, p3_opts).await?;
        extractions.push(parse_pass3_response(&raw)?);
    }
    let p4_prompt = assemble_pass4(&leaves, &extractions, &source, &rules);
    let p4_raw = ctx.llm.infer(&p4_prompt, p4_opts).await?;
    let relationships = parse_pass4_response(&p4_raw)?;
    (extractions, relationships)
};

// Merger + writer — identical in both modes; data contract is unchanged
```

Ensure `derive_implicit_edges` already exists (it is used by `pass-4` assembly to tell the model what edges not to repeat). If it does not yet exist as a standalone function, extract it from the Pass 4 assembly path.

---

## Implementation Order

Execute in this sequence to keep the build green at each checkpoint:

1. **`src/config.rs`** — add `GeminiConfig`, update `LlmConfig` (Ollama fields → `Option<T>`), add `PipelineConfig`, add `pipeline: PipelineConfig` to root `Config`.
2. **`src/llm.rs`** — add `GeminiClient` and `GeminiClient::new`, implement `LlmClient` for `GeminiClient`, update `build_client` with `"gemini"` arm and `.unwrap_or` defaults on Ollama arm.
3. **`anansi.toml.example`** — add `[llm.gemini]` and `[pipeline]` sections.
4. **Verify Part A compiles:** `cargo build` — the `"gemini"` path must be reachable with no warnings on new code. Fix any call-site errors from the `Option<T>` changes before proceeding.
5. **`prompts/pass-3-batch.md`** — create the batch prompt file (verbatim content from §4 above).
6. **`src/prompt.rs`** — add `PASS3_BATCH_TEMPLATE` constant, add `assemble_pass3_batch`, add or verify `parse_toc_entity_types`.
7. **`src/pipeline.rs`** — refactor `parse_pass3_response` to `_from_value` variant; add `parse_relationship_from_value` if not present; add `parse_batch_response`; add batch branch in `ingest`.
8. **`cargo test`** — verify no regressions on existing tests before any live testing.

---

## What NOT to Touch

- **`src/merger.rs`** — receives identical `Pass3Output` + `RelationshipEdge` data regardless of backend or mode. No changes.
- **`src/db.rs`** — no schema or query changes.
- **`src/mcp.rs`** — `anansi_ingest` continues to spawn a background task; the async queue and response contract are unchanged.
- **`migrations/`** — no changes.
- **`prompts/pass-1-toc-extraction.md`** — Pass 1 is not affected by backend or mode.
- **`prompts/pass-3-node-expansion.md`** — still used in standard mode; do not modify.
- **`prompts/pass-4-relationship-extraction.md`** — still used in standard mode; do not modify.

---

## Environment Variables

| Variable | Used by | Description |
|---|---|---|
| `ANANSI_GEMINI_API_KEY` | `build_client` | Gemini API key — alternative to `[llm.gemini] api_key` in `anansi.toml` |
| `ANANSI_OLLAMA_URL` | existing | Ollama URL override — unchanged |
| `ANANSI_OLLAMA_MODEL` | existing | Ollama model override — unchanged |

---

## Batch Mode Inference Settings

The batch call uses `[llm.synthesis]` inference settings (the most complex reasoning pass). Recommended settings for Gemini 2.5 Pro in batch mode — increase `max_tokens` because batch responses are significantly larger than per-leaf responses:

```toml
[llm.synthesis]
temperature = 0.2
max_tokens  = 16384
json_mode   = true
```

A future improvement could add `[llm.batch_settings]` for per-batch tuning independent of `synthesis` settings. Leave a `// TODO: add [llm.batch_settings] for tuning batch opts separately` comment in `pipeline.rs` at the point where `batch_opts` is constructed.

---

## Gemini Model Reference

| Model | Use case | Notes |
|---|---|---|
| `gemini-2.5-pro` | Batch mode, complex documents | Largest context window, best instruction following — recommended for batch mode |
| `gemini-2.0-flash` | Standard mode, high volume | Faster and cheaper — good for per-leaf Pass 3 calls |
| `gemini-2.0-flash-lite` | Development and testing | Cheapest — suitable for verifying JSON schema and prompt structure |

The `model` field in `[llm.gemini]` is passed directly to the API — no mapping needed in code.

---

## Testing Checklist

- [ ] `cargo build` passes with no warnings on new code
- [ ] `cargo test` passes — no regressions on existing tests
- [ ] Ollama standard mode: ingest a meeting summary → outline + atomic notes produced correctly (unchanged behavior)
- [ ] Gemini standard mode: same source → structurally identical output (field values may differ by model, but schema is the same)
- [ ] `anansi_ingest` MCP tool works with Gemini backend — async queue and response contract unchanged
- [ ] Gemini batch mode: same source → structurally identical output to standard mode runs
- [ ] Missing API key: surfaces error `"Gemini API key not found. Set [llm.gemini] api_key in anansi.toml or export ANANSI_GEMINI_API_KEY"` — not a panic
- [ ] Invalid API key: Gemini 400/401 error surfaced as a clear `Err`, not swallowed
- [ ] Batch mode with malformed JSON response: `parse_batch_response` returns a descriptive `Err` with raw content included — not a panic
- [ ] Preprocessed TOC + batch mode: Pass 1 skipped, batch call executes Pass 3+4 — verify `pass1_llm_called: false` in `IngestResult`
- [ ] `[pipeline] mode = "standard"` (explicit) behaves identically to omitting the `[pipeline]` section entirely
