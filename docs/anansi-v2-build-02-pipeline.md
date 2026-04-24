# Anansi v2 — Build Document 2: Pipeline & Engine

**For:** Coding agent (Claude Code) executing against the repo produced by doc 1.
**Reference:** `anansi-v2-spec.md` §3 (architecture), §9 (TOC schema), §11 (pipeline), §12 (merge semantics), §14 (LLM backend).
**Prerequisite:** Doc 1 complete. Rust crate compiles; templates, %Rules, config, DB migrations, vault helpers are in place. Ollama running locally with `qwen2.5:14b` pulled.
**Output:** A working ingestion pipeline. Given a source file at the anansi root, the CLI (not yet built — stubbed for this doc) can run the three passes end-to-end, producing an outline file and atomic notes in `anansi/web/` with edges in the DB.

---

## Goal

Build the engine. Three prompt files (for Pass 1, Pass 3, Pass 4), the Ollama client, the deterministic prompt assembler, the writer (file output + wikilinks), the merger (three merge categories), and the pipeline orchestrator that stitches it all together. No external interfaces yet — doc 3 adds the MCP server and CLI entry.

## Files to create

### 1. `prompts/pass-1-toc-extraction.md`

The Pass 1 prompt. Produces the enriched TOC per spec §9 — leaves with `[entity_type]` annotations plus pipe-delimited `hint` and `context_at` metadata.

The prompt must:
- Reference `{RULES:Atomicity}` and `{RULES:Downstream-Flow}` (prompt.rs will expand these)
- Inject `{ENTITY_TYPES}` — the list of atomic types from the registry
- Explain the leaf format with the documented pipe-annotation syntax
- Require output to be ONLY the TOC lines (no preamble, no markdown fences)
- Enforce the governing rules inline

Key requirements for each leaf:
- Every leaf must have a hint (1–2 sentences about what the source specifically says about this entity).
- Identity-type leaves (person, org, concept, topic) SHOULD include `context_at` when source-specific content about them exists — the addresses point to context nodes in section 3.
- Task leaves with an assignee use the format `[Verb + object] — [Assignee]` so `toc.rs` can pick up the assignment.
- Ambiguous leaves use `[?]` and are skipped downstream.

Output format: plain text leaf lines, no preamble, no trailing text, no markdown fences. The daemon parses lines matching the regex in spec §9.

Starter content should mirror spec §9 leaf format examples. ~120 lines.

### 2. `prompts/pass-3-node-expansion.md`

The Pass 3 prompt. Produces one atomic note per leaf as structured JSON.

The prompt must:
- Reference `{RULES:Atomicity}`
- Inject `{TEMPLATE_FIELDS}` — the `%% field %%` blocks from the leaf's entity type template
- Inject `{SOURCE_HINT}` — the source-type-specific hint from the template (e.g., `template.sources["meeting_summary"].hint`)
- Inject `{LEAF_HINT}` — the hint from the TOC leaf
- Inject `{CONTEXT_AT}` — the downstream routing addresses
- Inject `{ENTITY_TYPE}`, `{ENTITY_NAME}`, `{TOC_ADDRESS}`
- Inject `{SOURCE}` — the source body
- Require JSON output with the exact schema below

Output schema (strict):

```json
{
  "fields": {
    "<field_name>": "<value>"
  },
  "roster": {
    "<roster_key>": [
      {"name": "...", "slug": "...", "role": "..."}
    ]
  },
  "summary_1": "...",
  "summary_5": "...",
  "tags": ["...", "..."],
  "entities": [
    {"name": "...", "entity_type": "...", "slug": "..."}
  ]
}
```

- `fields` populates the template's declared fields. Use `"[not mentioned]"` for absent fields.
- `roster` is present ONLY for container types (org, project); keys match the template's `roster_sections`.
- `entities` lists every named entity referenced — used by downstream notes' `## Entities` sections and by Pass 4 edge derivation.

The prompt must enforce anti-contamination rules for pure-atomic types: `summary_1` and `summary_5` describe the entity independent of any source; do NOT reference this document, this meeting, or any specific event.

### 3. `prompts/pass-4-relationship-extraction.md`

The Pass 4 prompt. Produces a JSON array of edges not already derived from TOC structure.

Inputs injected:
- `{IMPLICIT_EDGES}` — already-derived structural edges (do not repeat)
- `{NODES}` — all Pass 3-produced notes as `<match_key> | <entity_type> | <name> | <summary_1>` lines
- `{TOC}` — the full enriched TOC
- `{RELATIONSHIP_TYPES}` — the canonical list from spec §11

Output: JSON array of `{source, relationship, target, why}` objects. Source and target are `match_key` values.

### 4. `src/llm.rs`

Ollama client with JSON mode, `InferOpts` passthrough, and per-pass settings plumbed from config.

```rust
pub struct InferOpts {
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

impl InferOpts {
    pub fn from(settings: &InferSettings) -> Self {
        Self {
            temperature: settings.temperature,
            max_tokens: settings.max_tokens,
            json_mode: settings.json_mode,
        }
    }
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn infer(&self, prompt: &str, opts: InferOpts) -> Result<String>;
    async fn ping(&self) -> Result<()>;  // health check
}

pub struct OllamaClient {
    url: String,
    model: String,
    n_ctx: u32,
    client: reqwest::Client,
}

impl OllamaClient {
    pub fn new(url: String, model: String, n_ctx: u32, timeout_s: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_s))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { url, model, n_ctx, client }
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn infer(&self, prompt: &str, opts: InferOpts) -> Result<String> {
        let mut body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": opts.temperature,
                "num_predict": opts.max_tokens,
                "num_ctx": self.n_ctx,
            }
        });
        if opts.json_mode {
            body["format"] = serde_json::Value::String("json".to_string());
        }

        let resp = self.client
            .post(format!("{}/api/generate", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama request failed ({status}): {text}"));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json["response"].as_str().unwrap_or("").to_string())
    }

    async fn ping(&self) -> Result<()> {
        let resp = self.client.get(format!("{}/api/tags", self.url)).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Ollama not reachable at {}", self.url));
        }
        Ok(())
    }
}

pub fn build_client(config: &LlmConfig) -> Result<Box<dyn LlmClient>> {
    match config.backend.as_str() {
        "ollama" => Ok(Box::new(OllamaClient::new(
            config.url.clone(),
            config.model.clone(),
            config.n_ctx,
            config.timeout_s,
        ))),
        other => Err(anyhow!("unknown LLM backend: {other} (only 'ollama' supported in v1)")),
    }
}
```

~120 lines. Keep stub branches for `codex-cli` and `claude-cli` commented out; they arrive in a later doc.

### 5. `src/prompt.rs`

Deterministic prompt assembly — no LLM calls. Loads the three prompt template files at startup (compile them in via `include_str!`), then for each pass builds the full prompt by substituting placeholders.

```rust
const PASS1_TEMPLATE: &str = include_str!("../prompts/pass-1-toc-extraction.md");
const PASS3_TEMPLATE: &str = include_str!("../prompts/pass-3-node-expansion.md");
const PASS4_TEMPLATE: &str = include_str!("../prompts/pass-4-relationship-extraction.md");

pub fn build_pass1(
    rules: &RuleRegistry,
    registry: &TemplateRegistry,
    source_type: &str,
    source_body: &str,
) -> String { ... }

pub fn build_pass3(
    rules: &RuleRegistry,
    template: &Template,
    source_type: &str,
    entity_name: &str,
    toc_address: &str,
    leaf_hint: &str,
    context_at: &[String],
    source_body: &str,
) -> String { ... }

pub fn build_pass4(
    rules: &RuleRegistry,
    implicit_edges: &str,
    nodes_block: &str,
    toc_text: &str,
) -> String { ... }

// Substitution helper:
fn inject(template: &str, vars: &HashMap<&str, &str>) -> String {
    let mut out = template.to_string();
    for (key, val) in vars {
        // {RULES:Name} syntax
        let rules_marker = format!("{{RULES:{key}}}");
        out = out.replace(&rules_marker, val);
        // {KEY} syntax
        let plain_marker = format!("{{{}}}", key.to_uppercase());
        out = out.replace(&plain_marker, val);
    }
    out
}
```

Resolve `{RULES:Atomicity}` by looking up `rules.get("Atomicity")` and injecting the rule body (sans frontmatter). Resolve `{ENTITY_TYPES}` by iterating the registry's atomic types and rendering each as `[type] — description`.

For Pass 3's `{TEMPLATE_FIELDS}`: render the template's `field_blocks` as labeled sections. For `{CONTEXT_AT}`: if the list is non-empty, render as "Content about this entity specific to the source belongs in context nodes at: 3.2, 3.5. Do not duplicate that content in this note." If empty, omit the line.

~180 lines. Unit tests for each `build_*` with small fixtures.

### 6. `src/writer.rs`

Markdown file writing. Owns YAML frontmatter emission, `{{placeholder}}` substitution (with `\{` unescape), and atomic writes.

Key functions:

```rust
/// Write an atomic note (pure or container). File path from vault helpers.
pub fn write_atomic_note(
    vault: &Vault,
    note: &NoteRecord,
    body: &str,
) -> Result<()> { ... }

/// Render a roster section for a container note.
pub fn render_roster_section(
    section: &RosterSection,
    rows: &[HashMap<String, String>],
) -> String {
    let mut out = format!("{}\n\n", section.render_as);
    for row in rows {
        let rendered = format_row(&section.row_format, row);
        out.push_str(&format!("- {rendered}\n"));
    }
    out
}

/// Render the `## Entities` section for a source-bound note.
pub fn render_entities_section(entities: &[EntityRef], vault: &Vault) -> String {
    let mut out = String::from("## Entities\n\n");
    for e in entities {
        let link = vault.wikilink(&e.entity_type, &e.name);
        out.push_str(&format!("- {link} ({})\n", e.entity_type));
    }
    out
}

/// Write an outline file. Body is the rendered TOC with wikilinks to each leaf.
pub fn write_outline(
    vault: &Vault,
    source: &SourceRecord,
    outline_note_id: &str,
    leaves_by_section: &[OutlineSection],
) -> Result<PathBuf> { ... }

/// Atomic write: write to a tmp file next to the target, then rename.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Substitute {{field}} placeholders; unescape \{ to {.
pub fn render_body(template_body: &str, fields: &HashMap<String, String>) -> String { ... }
```

Frontmatter emission uses `serde_yaml` with a small wrapper struct per note type to ensure keys come out in a stable order (name, entity_type, anansi_id first). ~200 lines.

### 7. `src/merger.rs`

The three merge categories. One entrypoint; dispatches on the template's `merge_strategy`.

```rust
pub async fn merge_note(
    pool: &DbPool,
    vault: &Vault,
    template: &Template,
    new: &Pass3Output,
    source: &SourceRecord,
    toc_address: &str,
    leaf_hint: &str,
) -> Result<MergeOutcome> {
    match template.merge_strategy {
        MergeStrategy::PureAtomic => merge_pure_atomic(pool, vault, template, new, source).await,
        MergeStrategy::Container => merge_container(pool, vault, template, new, source).await,
        MergeStrategy::SourceBound => merge_source_bound(pool, vault, template, new, source, toc_address).await,
    }
}

pub enum MergeOutcome {
    Created { note_id: String, path: PathBuf },
    FilledFields { note_id: String, filled: Vec<String> },
    AddedRoster { note_id: String, section: String, rows_added: usize },
    Regenerated { note_id: String, path: PathBuf },
    Conflict { note_id: String, field: String, existing: String, new: String },
    Noop { note_id: String },
}
```

Implement each of the three strategies per spec §12:

**`merge_pure_atomic`:**
1. Compute `match_key`. Look up existing by match_key.
2. If none: create note row, write file via writer, insert contribution (`created`).
3. If exists: for each identity field, if existing is blank/`[not mentioned]` and new has content, apply. If existing is filled and new differs, insert contribution (`conflict`) with both values in payload JSON — do NOT overwrite. Rewrite file with merged identity. Increment source_count. Insert contribution (`filled_fields` or `conflict` per outcome).

**`merge_container`:**
1. Identity merge same as pure_atomic.
2. For each `roster_section` declared in the template, parse existing rows from the file's section (regex on the `render_as` heading; split rows at `^- `). Apply set union keyed by `dedupe_by`. Write merged file. Insert contribution (`added_roster` with payload listing added tuples).
3. For each new roster row, write an `edge` row: container-note_id → has_member → member-note_id, with `metadata: {"role": "..."}` JSON.

**`merge_source_bound`:**
1. Key is `(source.id, toc_address)`. Look up by that pair.
2. If not found: create note row, write file, insert contribution (`created`).
3. If found and source's `content_hash` differs from the prior ingest's content_hash: overwrite file, update note row, insert contribution (`regenerated`).
4. If found and content_hash matches: noop.

~220 lines with tests for each strategy using in-memory SQLite.

### 8. `src/pipeline.rs`

The orchestrator. One public function: `ingest(root, source_path)`. Runs Pass 1 (unless the source has `anansi_toc` in frontmatter), TOC spawn, Pass 3 per leaf, Pass 4.

Input types:

```rust
pub struct IngestContext {
    pub anansi_root: PathBuf,
    pub config: Config,
    pub vault: Vault,
    pub db: DbPool,
    pub templates: TemplateRegistry,
    pub rules: RuleRegistry,
    pub llm: Box<dyn LlmClient>,
}

pub struct IngestResult {
    pub source_id: String,
    pub outline_note_id: String,
    pub atomic_notes_created: usize,
    pub atomic_notes_merged: usize,
    pub edges_created: usize,
    pub duration_ms: u64,
}
```

Flow:

```rust
pub async fn ingest(ctx: &IngestContext, source_path: &Path) -> Result<IngestResult> {
    // 1. Read source file, split frontmatter from body
    let (frontmatter, body) = parse_source_file(source_path)?;
    let content_hash = sha256(&body);

    // 2. Dedup check
    if let Some(existing) = find_source_by_content_hash(&ctx.db, &content_hash).await? {
        // Re-ingest path: check if TOC changed; if not, noop.
        // ... (handle the edited-TOC re-ingest case; details below)
    }

    // 3. Determine source_type from frontmatter or infer from filename
    let source_type = frontmatter.get("source_type")
        .unwrap_or("container")  // fallback

    // 4. Pass 1 — get the TOC
    let toc_text = if let Some(preprocessed) = frontmatter.get("anansi_toc") {
        validate_preprocessed_toc(preprocessed, &ctx.templates)?
    } else {
        let prompt = prompt::build_pass1(&ctx.rules, &ctx.templates, source_type, &body);
        let opts = InferOpts::from(&ctx.config.llm.decomposition);
        ctx.llm.infer(&prompt, opts).await?
    };

    // 5. Insert source record
    let source_id = Uuid::new_v4().to_string();
    insert_source(&ctx.db, &SourceRecord { ... }).await?;

    // 6. Parse TOC, create outline note row, write outline file
    let leaves = parse_toc(&toc_text);
    let outline_id = create_outline_note(&ctx.db, &ctx.vault, &source_id, &leaves).await?;

    // 7. Pass 3 — per leaf
    let mut notes_created = 0;
    let mut notes_merged = 0;
    let mut all_pass3_outputs = Vec::new();

    for leaf in &leaves {
        let template = ctx.templates.get(&leaf.entity_type)
            .ok_or_else(|| anyhow!("unknown entity type: {}", leaf.entity_type))?;

        let prompt = prompt::build_pass3(
            &ctx.rules, template, source_type,
            &leaf.name, &leaf.address, &leaf.hint.as_deref().unwrap_or(""),
            &leaf.context_at, &body,
        );
        let opts = InferOpts::from(&ctx.config.llm.extraction);
        let raw = ctx.llm.infer(&prompt, opts).await?;
        let response = parse_pass3_response(&raw)?;

        let outcome = merger::merge_note(
            &ctx.db, &ctx.vault, template, &response, &source,
            &leaf.address, leaf.hint.as_deref().unwrap_or(""),
        ).await?;

        match &outcome {
            MergeOutcome::Created { .. } => notes_created += 1,
            _ => notes_merged += 1,
        }

        all_pass3_outputs.push((leaf.clone(), response, outcome));
    }

    // 8. Emit structural edges (outline contains → each leaf, TOC-derived edges)
    let structural_edges = derive_implicit_edges(&leaves, &all_pass3_outputs, &outline_id);
    for edge in &structural_edges {
        insert_edge_if_not_exists(&ctx.db, edge).await?;
    }

    // 9. Pass 4 — semantic edges
    let nodes_block = build_nodes_block(&all_pass3_outputs);
    let implicit_text = format_edges_for_prompt(&structural_edges);
    let prompt = prompt::build_pass4(&ctx.rules, &implicit_text, &nodes_block, &toc_text);
    let opts = InferOpts::from(&ctx.config.llm.extraction);
    let raw = ctx.llm.infer(&prompt, opts).await?;
    let edges: Vec<EdgeJson> = serde_json::from_str(&strip_markdown_fences(&raw))?;

    let mut edges_created = structural_edges.len();
    for e in &edges {
        // resolve source/target by match_key; insert if not duplicate
    }

    Ok(IngestResult { ... })
}
```

Also implement:
- `parse_source_file` — split frontmatter from body using the same rules as the DB's frontmatter reader
- `validate_preprocessed_toc` — apply spec §9 validation; return fallback-to-Pass1 signal if invalid
- `parse_toc` — the leaf regex from spec §9; return a `Vec<TocLeaf>` including hint and context_at annotations
- `derive_implicit_edges` — per spec §11, the structural edges: outline-contains-leaf, person-under-org, context-belongs-to-event, task-assigned-to-name

~260 lines. Keep it orchestration-only — push logic into helper functions in other modules when it grows past a screen.

### 9. Update `Cargo.toml`

Add to dependencies if not already there:
- `sha2 = "0.10"` — for content_hash
- Enable `chrono` features already in doc 1.

---

## Testing strategy

1. **Unit tests per module** — each `build_pass*` function with small fixtures; merger each strategy with an in-memory SQLite.
2. **Integration test** — drop a small hand-authored source file (5–10 leaves) into a tempdir, run `ingest`, assert the expected files appear in `web/`, assert DB rows exist, assert edges exist. Use a mock `LlmClient` that returns canned responses for each prompt.
3. **Live smoke test** (manual) — point at a running Ollama, ingest a real short document (e.g., a small meeting note). Verify the output visually in Obsidian.

Tests live in `tests/` (integration) and inline `#[cfg(test)]` modules (unit).

## Acceptance criteria

1. `cargo check` and `cargo test --lib` pass.
2. The integration test with a mock LLM produces: 1 source row, 1 outline row, N atomic note rows, correct number of edges.
3. With a live Ollama and the test document, `anansi2::pipeline::ingest` (called from a test harness) runs end-to-end without panics and produces plausible output in `web/`.
4. Pass 1 skip path works: a source with `anansi_toc` in frontmatter is processed without hitting the LLM for Pass 1 (verify via LLM-call-count assertion).
5. Preprocessed TOC validation falls back to Pass 1 on invalid input (test with a deliberately malformed `anansi_toc`).

## Non-goals for this document

- Do not implement the MCP server or CLI (`mcp.rs`, `main.rs`). Doc 3.
- Do not build the Dockerfile or Cowork plugin. Doc 3.
- Do not implement per-pass backend routing or Codex/Claude CLI shims. v1.5 / per-pass routing instruction doc.
- Do not add embeddings. v2.
- Do not add FTS5 search. v1.5.

## Commit message

Suggested: `pipeline: three passes, Ollama client, prompt assembly, writer, merger`

---

*Doc 2 of 3 · Anansi v2 Build Instructions · 2026-04-23*
