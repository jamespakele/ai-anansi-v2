# Anansi v2 — Gemini Backend + Frontier Batch Mode: Coding Agent Handoff

**Repo:** `jamespakele/ai-anansi-v2`  
**Reference spec:** `docs/anansi-v2-spec.md` §14 (LLM backend), `docs/anansi-v2-build-02-pipeline.md` §4 (`src/llm.rs`)  
**Status:** Design complete, not yet implemented  
**Depends on:** Existing codebase (pipeline, merger, db, mcp all working with Ollama)  
**Does NOT touch:** `src/merger.rs`, `src/db.rs`, `src/mcp.rs`, `migrations/`, template files

---

## 1. Overview

This change adds Google Gemini as an alternative LLM backend alongside Ollama. Ollama remains the default and continues to work exactly as before — this is additive only.

Two related but independent additions:

**Part A — GeminiClient:** Implements the existing `LlmClient` trait for the Gemini REST API. Drop-in replacement for `OllamaClient`. Set `backend = "gemini"` in `anansi.toml` to activate.

**Part B — Batch pipeline mode:** An optional pipeline optimization for frontier models (Gemini or any capable backend) that collapses the N Pass 3 calls + 1 Pass 4 call into a single batch call. Reduces total LLM calls from N+2 to 2 (or 1 if a preprocessed TOC is present). Standard mode remains the default.

These two parts are independent — you can use Gemini in standard mode, or use Ollama in batch mode, or mix. They share the same config section.

---

## 2. Part A — GeminiClient

### 2.1 `src/config.rs` Changes

**Current `LlmConfig` struct** (approximate — match what's in the file):
```rust
pub struct LlmConfig {
    pub backend: String,      // "ollama"
    pub url: String,
    pub model: String,
    pub n_ctx: u32,
    pub timeout_s: u64,
    pub decomposition: InferSettings,
    pub extraction: InferSettings,
    pub synthesis: InferSettings,
}
```

**Add a `gemini` sub-config** (new struct, nested under `LlmConfig`):

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

**Updated `LlmConfig`:**

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    pub backend: String,          // "ollama" | "gemini"
    // Ollama fields (kept for backward compat):
    pub url: Option<String>,
    pub model: Option<String>,
    pub n_ctx: Option<u32>,
    pub timeout_s: Option<u64>,
    // Gemini sub-config:
    pub gemini: Option<GeminiConfig>,
    // Per-pass inference settings (shared by both backends):
    pub decomposition: InferSettings,
    pub extraction: InferSettings,
    pub synthesis: InferSettings,
}
```

**API key resolution** (in `build_client` or a helper):

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

### 2.2 `src/llm.rs` Changes

Add `GeminiClient` after the existing `OllamaClient` implementation. Do not modify `OllamaClient`.

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
            generation_config["responseMimeType"] = serde_json::Value::String(
                "application/json".to_string()
            );
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

        // Gemini response path: candidates[0].content.parts[0].text
        let text = json
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Unexpected Gemini response shape: {}", json))?;

        Ok(text.to_string())
    }

    async fn ping(&self) -> Result<()> {
        // List models endpoint — lightweight health check
        let url = format!(
            "{}/models?key={}",
            self.base_url, self.api_key
        );
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

**Update `build_client`:**

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
                .ok_or_else(|| anyhow!("backend = 'gemini' requires [llm.gemini] config section"))?;
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

### 2.3 `anansi.toml.example` — Add Gemini section

Add this section to the example config file (keep existing Ollama section):

```toml
# ──────────────────────────────────────────────────────────────
# LLM Backend — choose "ollama" (default, local) or "gemini"
# ──────────────────────────────────────────────────────────────
[llm]
backend = "ollama"   # change to "gemini" to use Gemini API

# Ollama settings (used when backend = "ollama")
url     = "http://localhost:11434"
model   = "qwen2.5:14b"
n_ctx   = 8192
timeout_s = 300

# Gemini settings (used when backend = "gemini")
# API key can also be set via ANANSI_GEMINI_API_KEY environment variable
[llm.gemini]
api_key  = ""                    # leave blank to use env var
model    = "gemini-2.5-pro"     # or "gemini-2.0-flash" for faster/cheaper
# base_url = "https://generativelanguage.googleapis.com/v1beta"  # default
timeout_s = 120
```

---

## 3. Part B — Frontier Batch Pipeline Mode

### 3.1 What Batch Mode Does

In standard mode the pipeline makes:
- 0 or 1 Pass 1 calls (0 if preprocessed TOC present)
- N Pass 3 calls (one per TOC leaf)
- 1 Pass 4 call

Total: N+1 or N+2 calls.

In batch mode, Pass 3 and Pass 4 are collapsed into a single call that receives the full TOC, all leaf contexts, and the complete source body, and returns a structured JSON response containing all entity extractions plus all relationships. Total: 1 or 2 calls.

Batch mode is designed for frontier models (Gemini 2.5 Pro, Claude Opus, etc.) that have large context windows and strong instruction following. It is NOT recommended for smaller local models via Ollama — use standard mode for those.

**Batch mode does not change the output.** The merger, writer, and DB receive identical data regardless of mode. Only the number of LLM calls and the prompt structure differ.

### 3.2 `src/config.rs` — Add Pipeline Config

Add a new config section:

```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PipelineConfig {
    /// "standard" (default) or "batch"
    /// standard: N Pass3 calls + 1 Pass4 call
    /// batch: 1 combined call replacing Pass3 + Pass4
    pub mode: Option<String>,
}

impl PipelineConfig {
    pub fn is_batch(&self) -> bool {
        self.mode.as_deref() == Some("batch")
    }
}
```

Add `pipeline: PipelineConfig` to the top-level config struct and load it from `[pipeline]` in `anansi.toml`.

**Add to `anansi.toml.example`:**

```toml
# ──────────────────────────────────────────────────────────────
# Pipeline mode
# ──────────────────────────────────────────────────────────────
[pipeline]
# mode = "standard"    # default — N Pass3 calls + 1 Pass4 call (good for Ollama)
# mode = "batch"       # 2 total calls — requires frontier model (Gemini 2.5 Pro, etc.)
```

### 3.3 New Prompt File: `prompts/pass-3-batch.md`

Create this file. It replaces the separate Pass 3 and Pass 4 prompts when batch mode is active.

```markdown
You are extracting structured knowledge from a source document. You have been given:

1. The full source document body
2. A Table of Contents (TOC) listing every entity to extract, with type annotations and hints
3. Template field definitions for each entity type

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

```json
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
```

## Field Population Rules

- Populate every declared field for each entity type. Use `"[not mentioned]"` for fields the source does not address.
- For pure-atomic types (person, concept, topic, area, note): `summary_1` and `summary_5` must describe the entity INDEPENDENT of this source — no references to "this meeting", "this document", or any specific event.
- For source-bound types (context, event, task, topic_discussion, article_section, etc.): `summary_1` may reference the source context.
- `roster` is present ONLY for container types (organization, project). Omit the key entirely for other types.
- Include ALL named entities referenced by each leaf in its `entities` array — this is how cross-leaf wikilinks and edges are derived.

## Relationship Rules

{RELATIONSHIP_TYPES}

- Do not emit relationships already in `{IMPLICIT_EDGES}`.
- `source` and `target` must be `match_key` values — `entity_type:slug` format.
- Only emit relationships you are confident about from the source text. Include `why` to explain the evidence.
```

---

**Note on the prompt template:** The `{TEMPLATE_FIELDS}` injection when batch mode is active should include ALL field definitions for ALL entity types present in the TOC (not just one type, as in the per-leaf Pass 3 prompt). The prompt assembler needs to aggregate field blocks from every entity type appearing in the TOC leaves.

### 3.4 `src/prompt.rs` — Batch Prompt Assembly

Add a constant alongside the existing prompt constants:

```rust
const PASS3_BATCH_TEMPLATE: &str = include_str!("../prompts/pass-3-batch.md");
```

Add a new assembly function:

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
        .map(|t| format!("### {} ({})\n{}", t.entity_type, t.description, t.field_blocks_text()))
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

### 3.5 `src/pipeline.rs` — Batch Branch

In the main `ingest` function, after Pass 1 (TOC extraction) and before Pass 3, add the batch mode branch.

**Current flow (standard mode, simplified):**
```rust
// Pass 1 (or skip if preprocessed)
let toc = if source.has_preprocessed_toc() {
    source.anansi_toc.clone()
} else {
    let prompt = assemble_pass1(&source, &registry, &rules);
    ctx.llm.infer(&prompt, pass1_opts).await?
};

let leaves = parse_toc(&toc)?;
validate_toc_leaves(&leaves, &registry)?;  // includes floor constraint check

// Pass 3 — N calls
let mut extractions: Vec<Pass3Output> = Vec::new();
for leaf in &leaves {
    let p3_prompt = assemble_pass3(leaf, &source, &registry, &rules);
    let raw = ctx.llm.infer(&p3_prompt, p3_opts).await?;
    extractions.push(parse_pass3_response(&raw)?);
}

// Pass 4 — 1 call
let p4_prompt = assemble_pass4(&leaves, &extractions, &source, &rules);
let p4_raw = ctx.llm.infer(&p4_prompt, p4_opts).await?;
let relationships = parse_pass4_response(&p4_raw)?;
```

**New flow with batch branch:**
```rust
// Pass 1 (or skip if preprocessed) — unchanged
let toc = if source.has_preprocessed_toc() {
    source.anansi_toc.clone()
} else {
    let prompt = assemble_pass1(&source, &registry, &rules);
    ctx.llm.infer(&prompt, pass1_opts).await?
};

let leaves = parse_toc(&toc)?;
validate_toc_leaves(&leaves, &registry)?;

// Pass 3 + 4 — standard or batch
let (extractions, relationships) = if ctx.config.pipeline.is_batch() {
    // BATCH MODE — single call
    let implicit_edges = derive_implicit_edges(&leaves);
    let batch_prompt = assemble_pass3_batch(
        &toc,
        &source.body,
        &registry,
        &rules,
        &implicit_edges,
    );
    let batch_opts = InferOpts::from(&ctx.config.llm.synthesis);  // use synthesis settings
    let raw = ctx.llm.infer(&batch_prompt, batch_opts).await?;
    parse_batch_response(&raw)?
} else {
    // STANDARD MODE — N + 1 calls (unchanged from current implementation)
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

// Merger + writer — identical in both modes
```

### 3.6 `parse_batch_response` function

Add to `src/pipeline.rs`:

```rust
fn parse_batch_response(raw: &str) -> Result<(Vec<Pass3Output>, Vec<RelationshipEdge>)> {
    // Strip markdown fences if the model wrapped the JSON despite instructions
    let json_str = strip_markdown_fences(raw);
    
    let value: serde_json::Value = serde_json::from_str(&json_str)
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

**Note:** You will need to refactor `parse_pass3_response` to have a `parse_pass3_response_from_value(v: &serde_json::Value)` variant that both the per-leaf standard path and the batch path can call. The existing string-parsing version can call this after deserializing.

---

## 4. Environment Variables

| Variable | Used by | Description |
|---|---|---|
| `ANANSI_GEMINI_API_KEY` | `build_client` | Gemini API key — alternative to `anansi.toml` `api_key` field |
| `ANANSI_OLLAMA_URL` | existing | Ollama URL override — unchanged |
| `ANANSI_OLLAMA_MODEL` | existing | Ollama model override — unchanged |

---

## 5. InferSettings for Batch Mode

In `anansi.toml`, the batch call uses the `synthesis` inference settings (since it's doing the most complex reasoning). However, operators may want to tune this separately. A future improvement could add `[llm.batch_settings]` — leave a `// TODO` comment in the code where batch opts are selected.

Recommended settings for Gemini 2.5 Pro in batch mode:
```toml
[llm.synthesis]
temperature = 0.2
max_tokens  = 16384   # batch responses are large — increase from default
json_mode   = true
```

---

## 6. Implementation Order

1. **`src/config.rs`** — add `GeminiConfig`, update `LlmConfig`, add `PipelineConfig`
2. **`src/llm.rs`** — add `GeminiClient`, update `build_client`
3. **`anansi.toml.example`** — add Gemini and pipeline sections
4. **Verify Part A compiles** — `cargo build` with `backend = "gemini"` path reachable
5. **`src/config.rs`** — ensure `pipeline.mode` loads correctly from toml
6. **`prompts/pass-3-batch.md`** — create the batch prompt file
7. **`src/prompt.rs`** — add `PASS3_BATCH_TEMPLATE` constant, add `assemble_pass3_batch`
8. **`src/pipeline.rs`** — refactor `parse_pass3_response` to have a `_from_value` variant, add `parse_batch_response`, add batch branch
9. **Test standard mode** — existing Ollama ingest still works, `cargo test`
10. **Test Gemini standard mode** — set `backend = "gemini"`, ingest a small source, verify output identical to Ollama run
11. **Test batch mode** — set `mode = "batch"` with Gemini, verify same output as standard mode on same source

---

## 7. What NOT to Change

- `src/merger.rs` — receives identical `Pass3Output` + `RelationshipEdge` data regardless of mode
- `src/db.rs` — no schema or query changes
- `src/mcp.rs` — no interface changes; `anansi_ingest` continues to spawn background task
- `migrations/` — no changes
- `prompts/pass-1-toc-extraction.md` — unchanged; Pass 1 is not affected by backend or mode
- `prompts/pass-3-node-expansion.md` — unchanged; still used in standard mode
- `prompts/pass-4-relationship-extraction.md` — unchanged; still used in standard mode

---

## 8. Testing Checklist

- [ ] `cargo build` passes with no warnings related to new code
- [ ] `cargo test` passes — no regressions on existing tests
- [ ] Ollama standard mode: ingest a meeting summary, verify outline + atomic notes produced correctly
- [ ] Gemini standard mode: same source, same vault — output should be structurally identical (field values may differ due to model differences, but schema is the same)
- [ ] `anansi_ingest` MCP tool works with Gemini backend — async queue and response unchanged
- [ ] Gemini batch mode: same source — output structurally identical to standard mode runs
- [ ] Missing API key: clear error message `"Gemini API key not found. Set [llm.gemini] api_key..."` — not a panic
- [ ] Invalid API key: Gemini 400/401 error is surfaced clearly, not swallowed
- [ ] Batch mode with malformed JSON response: `parse_batch_response` returns descriptive `Err`, not a panic
- [ ] Preprocessed TOC + batch mode: Pass 1 skipped, batch call runs Pass 3+4 — verify `pass1_llm_called: false` in `IngestResult`
- [ ] `[pipeline] mode = "standard"` (explicit) behaves identically to omitting the field

---

## 9. Notes on Gemini Model Selection

| Model | Use case | Notes |
|---|---|---|
| `gemini-2.5-pro` | Batch mode, complex documents | Largest context, best instruction following |
| `gemini-2.0-flash` | Standard mode, high volume | Faster and cheaper, good for Pass 3 per-leaf calls |
| `gemini-2.0-flash-lite` | Development and testing | Cheapest, suitable for verifying JSON schema |

The `model` field in `[llm.gemini]` is passed directly to the API — no mapping needed in code.
