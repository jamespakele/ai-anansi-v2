# Anansi v2 — Build Document 1: Foundation & Data Model

**For:** Coding agent (Claude Code) executing against a fresh repository
**Reference:** `anansi-v2-spec.md` is the authoritative design spec. This doc translates spec §2, §4–§7, §10, §11 (schema parts) into concrete files to create.
**Prerequisite:** Empty repository. Rust toolchain installed (`rustc >= 1.80`). SQLite CLI available for migration testing.
**Output:** A `cargo check`-clean Rust crate with data model, config, templates, and %Rules in place. No pipeline, no LLM, no MCP yet — those come in doc 2 and doc 3.

---

## Goal

Stand up the structural skeleton of anansi v2: the crate, dependencies, database migrations, configuration loader, template parser, rules loader, vault path helpers, and the seed content (15 templates + 4 %Rules files) that the pipeline will consume. When this doc is complete, `cargo check` passes, migrations apply cleanly to a fresh SQLite file, and every template + rule file parses without error against its loader.

## Files to create

Create in this order. Each depends only on what came before.

### 1. `Cargo.toml`

Minimum dependencies per spec §16. Use these exact versions:

```toml
[package]
name = "anansi2"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
axum = "0.7"
clap = { version = "4", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
regex = "1"
once_cell = "1"
slug = "0.1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3"
```

### 2. `migrations/0001_schema.sql`

Per spec §10, verbatim. Four tables: `sources`, `notes`, `source_contributions`, `edges`. All foreign keys must be declared (set `PRAGMA foreign_keys = ON` in the connection options, not in the migration). Keep the indices exactly as specified — they're sized for the query patterns in later docs.

### 3. `anansi.toml.example`

Ships as the template config. The runtime reads from `<anansi-root>/anansi.toml`; env vars override. Contents:

```toml
[paths]
# Paths are relative to the anansi root unless absolute.
web_dir = "web"
rules_dir = "%Rules"
templates_dir = "templates"
db_file = "web.db"

[llm]
backend = "ollama"
url = "http://localhost:11434"
model = "qwen2.5:14b"
n_ctx = 16384
timeout_s = 600

[llm.decomposition]
temperature = 0.2
max_tokens = 4096
json_mode = false

[llm.extraction]
temperature = 0.2
max_tokens = 4096
json_mode = true

[llm.synthesis]
temperature = 0.5
max_tokens = 8192
json_mode = false

[server]
mcp_port = 3738
host = "0.0.0.0"
read_only = false
```

### 4. `src/config.rs`

Load `anansi.toml` from the anansi root. Structure:

```rust
pub struct Config {
    pub paths: PathsConfig,
    pub llm: LlmConfig,
    pub server: ServerConfig,
}

pub struct PathsConfig {
    pub web_dir: PathBuf,
    pub rules_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub db_file: PathBuf,
}

pub struct LlmConfig {
    pub backend: String,
    pub url: String,
    pub model: String,
    pub n_ctx: u32,
    pub timeout_s: u64,
    pub decomposition: InferSettings,
    pub extraction: InferSettings,
    pub synthesis: InferSettings,
}

pub struct InferSettings {
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

pub struct ServerConfig {
    pub mcp_port: u16,
    pub host: String,
    pub read_only: bool,
}

impl Config {
    pub fn load(anansi_root: &Path) -> Result<Self> {
        let path = anansi_root.join("anansi.toml");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let mut config: Config = toml::from_str(&text)?;

        // env-var overrides (12-factor pattern)
        if let Ok(url) = std::env::var("ANANSI_OLLAMA_URL") {
            config.llm.url = url;
        }
        if let Ok(model) = std::env::var("ANANSI_OLLAMA_MODEL") {
            config.llm.model = model;
        }
        if let Ok(port) = std::env::var("ANANSI_MCP_PORT") {
            config.server.mcp_port = port.parse()?;
        }

        Ok(config)
    }

    pub fn web_path(&self, root: &Path) -> PathBuf {
        root.join(&self.paths.web_dir)
    }

    pub fn rules_path(&self, root: &Path) -> PathBuf {
        root.join(&self.paths.rules_dir)
    }

    pub fn templates_path(&self, root: &Path) -> PathBuf {
        root.join(&self.paths.templates_dir)
    }

    pub fn db_path(&self, root: &Path) -> PathBuf {
        root.join(&self.paths.db_file)
    }
}
```

All struct fields use `#[derive(Debug, Clone, Deserialize)]`. Use `#[serde(default)]` on `ServerConfig.read_only` and on any field that may reasonably be omitted. ~120 lines.

### 5. `src/db.rs`

Three responsibilities: (a) open/migrate the SQLite pool, (b) basic typed CRUD for each of the four tables, (c) the normalization function used for `match_key`.

Key functions:

```rust
pub type DbPool = sqlx::SqlitePool;

pub async fn open_and_migrate(db_path: &Path) -> Result<DbPool> {
    // connection options: enable foreign keys, WAL mode, create-if-missing
    // run sqlx::migrate!("./migrations") on the pool
}

pub fn match_key(name: &str, entity_type: &str) -> String {
    // lowercase, non-alphanumeric → space, collapse whitespace → hyphen
    // return format!("{entity_type}:{slug}")
}

// Records — one per table. Derive Serialize/Deserialize/FromRow/Clone/Debug.
pub struct SourceRecord { /* per spec §10 */ }
pub struct NoteRecord { /* per spec §10 */ }
pub struct SourceContributionRecord { /* per spec §10 */ }
pub struct EdgeRecord { /* per spec §10 */ }

// CRUD per record type — create, get_by_id, get_by_match_key (notes only), 
//   update_*, list_*. Keep functions small — ≤15 lines each.

pub async fn insert_source(pool: &DbPool, rec: &SourceRecord) -> Result<()> { ... }
pub async fn get_source(pool: &DbPool, id: &str) -> Result<Option<SourceRecord>> { ... }
pub async fn find_source_by_content_hash(pool: &DbPool, hash: &str) -> Result<Option<SourceRecord>> { ... }

pub async fn insert_note(pool: &DbPool, rec: &NoteRecord) -> Result<()> { ... }
pub async fn get_note(pool: &DbPool, id: &str) -> Result<Option<NoteRecord>> { ... }
pub async fn find_note_by_match_key(pool: &DbPool, key: &str) -> Result<Option<NoteRecord>> { ... }
pub async fn update_note_file_path(pool: &DbPool, id: &str, path: &str) -> Result<()> { ... }
pub async fn increment_source_count(pool: &DbPool, note_id: &str) -> Result<()> { ... }

pub async fn insert_contribution(pool: &DbPool, rec: &SourceContributionRecord) -> Result<()> { ... }

pub async fn insert_edge_if_not_exists(pool: &DbPool, rec: &EdgeRecord) -> Result<bool> { ... }
pub async fn edges_for_note(pool: &DbPool, note_id: &str) -> Result<Vec<EdgeRecord>> { ... }
```

Use `sqlx::query_as!` where possible for compile-time SQL checking. Fall back to `sqlx::query` when needed. All timestamps are ISO-8601 UTC strings (`chrono::Utc::now().to_rfc3339()`).

The `match_key` function must match the spec §12 normalization exactly:

```rust
pub fn match_key(name: &str, entity_type: &str) -> String {
    let normalized: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let slug = normalized.split_whitespace().collect::<Vec<_>>().join("-");
    format!("{entity_type}:{slug}")
}
```

Unit tests for `match_key` covering the three examples from the spec. ~350 lines total.

### 6. `templates/*.md` (15 files)

Per spec §5 type inventory + §6 template format. Each file is YAML frontmatter + `%% field %%` blocks + body template with `{{placeholder}}` substitutions.

Create all fifteen. Use `person.md` and `organization.md` from spec §6 as reference for `pure_atomic` and `container` respectively. The rest follow the same shape — copy the structure, adapt the fields per the purity table in spec §1 and the v1 Anansi handoff.

Files to create in `templates/`:
- `concept.md` (pure_atomic)
- `topic.md` (pure_atomic)
- `person.md` (pure_atomic) — spec §6 example
- `area.md` (pure_atomic)
- `note.md` (pure_atomic, fallback)
- `organization.md` (container) — spec §6 example
- `project.md` (container) — has `roster_sections.contributors` similar to org's `people`
- `context.md` (source_bound) — has `## Entities`, `## Topics`, `## Decisions` sections
- `event.md` (source_bound) — has `## Participants`, `## Outcomes`
- `task.md` (source_bound) — single owner, completion criterion
- `action_item_list.md` (source_bound) — list of tasks
- `outline.md` (source_bound) — see spec §8, body is the rendered TOC
- `container.md` (non-atomic source type)
- `email_thread.md` (non-atomic source type)
- `meeting_summary.md` (non-atomic source type)
- `research_paper.md` (non-atomic source type)

For non-atomic source types, set `atomic: false` in frontmatter; they're not rendered as atomic notes but provide source_hints for Pass 3 when processing documents of that type.

Each template must declare:
- `entity_type`
- `atomic`
- `merge_strategy` (`pure_atomic`, `container`, or `source_bound`)
- `template_version: "2.0"`
- `description`
- `identity_fields` (map of field → type/description)
- `sources` (map of source_type → hint)

Container templates add `roster_sections`. See spec §6 for the full shape.

### 7. `src/template.rs`

Template loader and parser. Reads `templates/*.md`, parses YAML frontmatter + `%% field %%` blocks + body. Exposes a `TemplateRegistry`:

```rust
pub struct TemplateRegistry {
    templates: HashMap<String, Template>,
}

pub struct Template {
    pub entity_type: String,
    pub atomic: bool,
    pub merge_strategy: MergeStrategy,
    pub template_version: String,
    pub description: String,
    pub identity_fields: HashMap<String, FieldDef>,
    pub sources: HashMap<String, SourceHint>,
    pub roster_sections: HashMap<String, RosterSection>,
    pub field_blocks: Vec<FieldBlock>,  // parsed %% blocks
    pub body: String,                   // template after last %% block
}

pub enum MergeStrategy {
    PureAtomic,
    Container,
    SourceBound,
}

pub struct FieldDef { /* type, required, format, description, target */ }
pub struct SourceHint { pub hint: String }
pub struct RosterSection {
    pub source_field: String,
    pub render_as: String,
    pub row_format: String,
    pub dedupe_by: Vec<String>,
}
pub struct FieldBlock {
    pub field: String,
    pub description: String,
    pub format: Option<String>,
    pub constraints: Option<String>,
}

impl TemplateRegistry {
    pub fn load(templates_dir: &Path) -> Result<Self> { ... }
    pub fn get(&self, entity_type: &str) -> Option<&Template> { ... }
    pub fn atomic_types(&self) -> Vec<&str> { ... }
    pub fn source_types(&self) -> Vec<&str> { ... }
    pub fn all_entity_types(&self) -> Vec<&str> { ... }
}

impl Template {
    pub fn render_body(&self, fields: &HashMap<String, String>) -> String {
        // substitute {{field_name}} with values; unescape \{ to {
    }
}
```

Parsing logic:
1. Split file at first `---` ... `---` pair (frontmatter).
2. After frontmatter, scan for `%% ... %%` blocks and parse each.
3. Everything after the last `%%` is `body`.
4. Deserialize frontmatter via `serde_yaml`.

~230 lines. Unit test: load all 15 templates, assert every one parses, assert `atomic_types()` returns the expected 12 atomic types (11 from the table in spec §5 + `outline`), assert `source_types()` returns 4.

### 8. `%Rules/%Atomicity.md`

The two governing rules from the original handoff, with preamble adjustments to match the v2 data model:

```markdown
---
id: atomicity-rules
type: rule
status: active
version: "2.0"
last_updated: 2026-04-23
---

# Atomicity Rules

## Rule 1 — Maximum Reusability

Every atomic note must make full sense WITHOUT the source document in hand.

**The merge test:** If a second document mentions the same entity, can you
enrich the existing node rather than creating a new one? If yes, the note
passes. If it's so source-specific it would need rewriting, it failed.

## Rule 2 — Entity Purity and Downstream Flow

Each atomic note contains ONLY information intrinsic to its entity type.
Source-specific content flows downstream to context, event, project, or
task nodes.

In anansi v2, this rule is enforced structurally by the three merge
categories:

- **Pure atomic** (person, concept, topic, area, note) — identity fields
  only. The note has no section that can accumulate source-specific prose.
- **Container** (organization, project) — identity fields plus declared
  roster sections that accept only structured, deduplicated additive merges.
  No free-text accumulation.
- **Source-bound** (context, event, task, action_item_list, outline) —
  source-specific by design. Carry `## Entities`, `## Topics`, `## Decisions`
  sections that wikilink upstream to identity/definition notes.

Because downstream content has nowhere to live in an upstream note, purity
is maintained by the data model, not by prompt discipline.
```

### 9. `%Rules/%Downstream-Flow.md`

Explains where source-specific content routes. Reference the context_at annotation from the preprocessed-TOC schema. ~80 lines.

### 10. `%Rules/%Merge-Strategy.md`

Spec §12 merge semantics, written as guidance the LLM and the author can both read. Explains the three categories, how container rosters accumulate, how source-bound nodes regenerate, and when conflicts are logged.

### 11. `%Rules/%Template-Schema.md`

Reference doc for template authors. Describes the frontmatter fields, `%% field %%` block format, `roster_sections` shape, and the source_hints map. Use spec §6 content as the basis.

### 12. `src/rules.rs`

Loader for the `%Rules/` folder. Reads every `%*.md` file, keyed by filename stem (minus `%`). Provides lookup by name:

```rust
pub struct RuleRegistry {
    rules: HashMap<String, String>,  // name → body (sans frontmatter)
}

impl RuleRegistry {
    pub fn load(rules_dir: &Path) -> Result<Self> { ... }
    pub fn get(&self, name: &str) -> Option<&str> { ... }
}
```

~70 lines.

### 13. `src/vault.rs`

Path helpers and slug generation. Owns the rules for where every file goes.

```rust
pub struct Vault {
    pub root: PathBuf,
    pub web: PathBuf,
}

impl Vault {
    pub fn new(root: PathBuf, web_dir: &Path) -> Self { ... }

    pub fn source_path(&self, source_slug: &str) -> PathBuf {
        self.root.join(format!("{source_slug}.md"))
    }

    pub fn outline_path(&self, source_slug: &str) -> PathBuf {
        self.web.join(format!("{source_slug}.outline.md"))
    }

    pub fn atomic_note_path(&self, entity_type: &str, name: &str) -> PathBuf {
        let s = slug::slugify(name);
        let filename = match entity_type {
            "note" => format!("{s}.md"),
            _ => format!("-{s}.md"),
        };
        self.web.join(filename)
    }

    pub fn source_bound_path(&self, toc_address: &str, name: &str, source_slug: &str) -> PathBuf {
        let addr = toc_address.replace('.', "-");
        let s = slug::slugify(name);
        self.web.join(format!("{addr}-{s}-{source_slug}.md"))
    }

    /// Render a wikilink: `[[-ian-kitajima|Ian Kitajima]]`
    pub fn wikilink(&self, entity_type: &str, name: &str) -> String {
        let s = slug::slugify(name);
        let prefix = match entity_type {
            "note" => "",
            _ => "-",
        };
        format!("[[{prefix}{s}|{name}]]")
    }
}
```

Unit tests: each path helper and wikilink for representative inputs. ~140 lines.

---

## Acceptance criteria

1. `cargo check` passes with no warnings (beyond unused-import or dead-code warnings, which are expected at this stage).
2. `sqlite3 /tmp/test-anansi.db < migrations/0001_schema.sql` creates all four tables with the declared indices and foreign keys. `PRAGMA foreign_key_check;` returns empty.
3. `cargo test --lib` passes. Tests must include:
   - `template::tests::loads_all_fifteen_templates`
   - `template::tests::atomic_types_returns_expected_set`
   - `db::tests::match_key_canonical_forms`
   - `vault::tests::path_conventions`
4. All 15 templates parse without error.
5. All 4 %Rules files load and return non-empty bodies via `RuleRegistry::get`.

## Non-goals for this document

- Do not implement `llm.rs`, `prompt.rs`, `pipeline.rs`, `writer.rs`, `merger.rs`, `mcp.rs`, or `main.rs`. Those are doc 2 and doc 3.
- Do not write prompt template files (they belong with the pipeline in doc 2).
- Do not add the Dockerfile or Cowork plugin (doc 3).
- Do not add optional v1.5 tables (e.g., `field_conflicts`). v1 stores conflicts in `source_contributions.payload` JSON.
- Do not add FTS5 tables.

## Commit message

Suggested: `foundation: crate, schema, config, templates, rules, vault helpers`

Do not commit unless the user explicitly asks.

---

*Doc 1 of 3 · Anansi v2 Build Instructions · 2026-04-23*
