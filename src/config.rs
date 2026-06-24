use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};


#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub paths: PathsConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub pipeline: PipelineConfig,
    #[serde(default)]
    pub inbox: InboxConfig,
    #[serde(default)]
    pub wiki: WikiConfig,
    /// PostgreSQL connection URL — overridden by DATABASE_URL env var at load time.
    #[serde(default = "default_database_url")]
    pub database_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_web_dir")]
    pub web_dir: PathBuf,
    #[serde(default = "default_rules_dir")]
    pub rules_dir: PathBuf,
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
}

fn default_web_dir() -> PathBuf { PathBuf::from("anansi/web") }
fn default_rules_dir() -> PathBuf { PathBuf::from("anansi/%Rules") }
fn default_templates_dir() -> PathBuf { PathBuf::from("llm/plugins/anansi.plugin/references/templates") }
fn default_database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://anansi:anansi@localhost:5432/anansi".to_string())
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_ollama_url")]
    pub url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_n_ctx")]
    pub n_ctx: u32,
    #[serde(default = "default_timeout_s")]
    pub timeout_s: u64,
    #[serde(default)]
    pub decomposition: InferSettings,
    #[serde(default)]
    pub extraction: InferSettings,
    #[serde(default)]
    pub synthesis: InferSettings,
    #[serde(default)]
    pub gemini: Option<GeminiConfig>,
    #[serde(default)]
    pub openrouter: Option<OpenRouterConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GeminiConfig {
    /// Path to the gemini CLI binary; if absent, searches PATH
    pub cli_path: Option<String>,
    /// Model name passed to the CLI/REST API (e.g. "gemini-2.5-pro")
    #[serde(default)]
    pub model: String,
    /// REST API key — fallback when CLI is not available
    pub api_key: Option<String>,
    /// Request timeout in seconds (default 120)
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OpenRouterConfig {
    /// OpenRouter API key (also readable from ANANSI_OPENROUTER_API_KEY)
    pub api_key: Option<String>,
    /// Model string e.g. "google/gemini-2.5-pro", "anthropic/claude-opus-4-7"
    #[serde(default)]
    pub model: String,
    /// Base URL — defaults to "https://openrouter.ai/api/v1"
    pub base_url: Option<String>,
    /// Request timeout in seconds (default 120)
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PipelineConfig {
    /// "standard" (default) or "batch"
    /// batch: collapses N Pass-3 + 1 Pass-4 into a single combined call
    pub mode: Option<String>,
}

impl PipelineConfig {
    pub fn is_batch(&self) -> bool {
        self.mode.as_deref() == Some("batch")
    }
}

// ─── Inbox Watcher ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct InboxConfig {
    /// Whether the inbox watcher background task is enabled (default false)
    #[serde(default)]
    pub enabled: bool,
    /// Directory to watch for incoming files (default "/data/inbox")
    #[serde(default = "default_inbox_watch_dir")]
    pub watch_dir: String,
    /// Directory to write archive runs (default "/data/archive")
    #[serde(default = "default_inbox_archive_dir")]
    pub archive_dir: String,
    /// Shared ingestion queue — inbox watcher and anansi_ingest_atomized both drop
    /// atomized .md files here; the queue watcher ingests them into the DB.
    #[serde(default = "default_queue_dir")]
    pub queue_dir: String,
    /// How often the queue watcher polls queue_dir (default 10s)
    #[serde(default = "default_queue_poll_interval_secs")]
    pub queue_poll_interval_secs: u64,
    /// Root directory of the r2-anansi plugin skills tree.
    /// Expects sub-directories: para-projects-areas/, para-resource-entities/, sb-atomize/
    /// each containing a SKILL.md and a references/ sub-directory.
    /// In Docker: mount ./claude-cowork/plugins/r2-anansi.plugin:/data/skills:ro
    #[serde(default = "default_skills_dir")]
    pub skills_dir: String,
    /// Polling interval in seconds (default 30)
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
    /// LLM backend override for inbox pipeline stages.
    /// If None, uses the main llm.backend setting.
    /// Valid: "gemini" | "openrouter" | "codex" | "ollama"
    pub llm_backend: Option<String>,
}

fn default_inbox_watch_dir() -> String { "/data/q-inbox".to_string() }
fn default_inbox_archive_dir() -> String { "/data/archive".to_string() }
fn default_queue_dir() -> String { "/data/q-atomize".to_string() }
fn default_queue_poll_interval_secs() -> u64 { 10 }
fn default_skills_dir() -> String { "/app/skills".to_string() }
fn default_poll_interval_secs() -> u64 { 30 }

impl Default for InboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            watch_dir: default_inbox_watch_dir(),
            archive_dir: default_inbox_archive_dir(),
            queue_dir: default_queue_dir(),
            queue_poll_interval_secs: default_queue_poll_interval_secs(),
            skills_dir: default_skills_dir(),
            poll_interval_secs: default_poll_interval_secs(),
            llm_backend: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: PathsConfig {
                web_dir: default_web_dir(),
                rules_dir: default_rules_dir(),
                templates_dir: default_templates_dir(),
            },
            llm: LlmConfig {
                backend: default_backend(),
                url: default_ollama_url(),
                model: default_model(),
                n_ctx: default_n_ctx(),
                timeout_s: default_timeout_s(),
                decomposition: InferSettings::default(),
                extraction: InferSettings::default(),
                synthesis: InferSettings::default(),
                gemini: None,
                openrouter: None,
            },
            server: ServerConfig::default(),
            pipeline: PipelineConfig::default(),
            inbox: InboxConfig::default(),
            wiki: WikiConfig::default(),
            database_url: default_database_url(),
        }
    }
}

// ─── LLM-Wiki (Build-12) ──────────────────────────────────────────────────────

/// File-native projection of the knowledge graph (Karpathy LLM-wiki pattern).
/// When enabled, successful captures are dual-written from canonical Postgres
/// rows into markdown files under `dir`. Postgres remains the source of truth.
#[derive(Debug, Clone, Deserialize)]
pub struct WikiConfig {
    /// Whether the wiki projection is written on each capture (default false).
    #[serde(default)]
    pub enabled: bool,
    /// Root directory for the wiki files (default "/data/llm-wiki").
    #[serde(default = "default_wiki_dir")]
    pub dir: String,
    /// Whether the background anansi-crawl maintenance task runs (default false).
    /// Requires `enabled = true`.
    #[serde(default)]
    pub crawl_enabled: bool,
    /// Crawl background-task interval in seconds (default 3600).
    #[serde(default = "default_crawl_interval_secs")]
    pub crawl_interval_secs: u64,
}

fn default_wiki_dir() -> String { "/data/llm-wiki".to_string() }
fn default_crawl_interval_secs() -> u64 { 3600 }

impl Default for WikiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dir: default_wiki_dir(),
            crawl_enabled: false,
            crawl_interval_secs: default_crawl_interval_secs(),
        }
    }
}

fn default_backend() -> String { "ollama".to_string() }
fn default_ollama_url() -> String { "http://localhost:11434".to_string() }
fn default_model() -> String { "qwen2.5:14b".to_string() }
fn default_n_ctx() -> u32 { 16384 }
fn default_timeout_s() -> u64 { 600 }

#[derive(Debug, Clone, Deserialize)]
pub struct InferSettings {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub json_mode: bool,
}

fn default_temperature() -> f32 { 0.2 }
fn default_max_tokens() -> u32 { 4096 }

impl Default for InferSettings {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            json_mode: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default)]
    pub read_only: bool,
    /// Public base URL used to generate export download links, e.g. "https://anansi.pakele.ai"
    pub public_url: Option<String>,
    /// Optional API key that must be presented on every MCP request.
    /// Accepted as `Authorization: Bearer <key>` header or `?api_key=<key>` query param.
    /// Override at runtime with the `ANANSI_API_KEY` environment variable.
    pub api_key: Option<String>,
}

fn default_mcp_port() -> u16 { 3738 }
fn default_host() -> String { "0.0.0.0".to_string() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mcp_port: default_mcp_port(),
            host: default_host(),
            read_only: false,
            public_url: None,
            api_key: None,
        }
    }
}

impl Config {
    pub fn load(anansi_root: &Path) -> Result<Self> {
        let path = anansi_root.join("anansi").join("anansi.toml");
        let mut config: Config = match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)
                .with_context(|| format!("parsing config at {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Config::default()
            }
            Err(e) => return Err(e).with_context(|| format!("reading config at {}", path.display())),
        };

        if let Ok(backend) = std::env::var("ANANSI_BACKEND") {
            config.llm.backend = backend;
        }
        if let Ok(url) = std::env::var("ANANSI_OLLAMA_URL") {
            config.llm.url = url;
        }
        if let Ok(model) = std::env::var("ANANSI_OLLAMA_MODEL") {
            config.llm.model = model;
        }
        if let Ok(port) = std::env::var("ANANSI_MCP_PORT") {
            config.server.mcp_port = port
                .parse()
                .with_context(|| format!("parsing ANANSI_MCP_PORT={port:?}"))?;
        }
        if let Ok(key) = std::env::var("ANANSI_GEMINI_API_KEY") {
            config.llm.gemini.get_or_insert_default().api_key = Some(key);
            if config.llm.backend == "ollama" {
                config.llm.backend = "gemini".to_string();
            }
        }
        if let Ok(key) = std::env::var("ANANSI_OPENROUTER_API_KEY") {
            config.llm.openrouter.get_or_insert_default().api_key = Some(key);
        }
        if let Ok(url) = std::env::var("ANANSI_PUBLIC_URL") {
            config.server.public_url = Some(url);
        }
        if let Ok(key) = std::env::var("ANANSI_API_KEY") {
            config.server.api_key = Some(key);
        }
        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database_url = url;
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
}

pub fn resolve_openrouter_api_key(cfg: &OpenRouterConfig) -> anyhow::Result<String> {
    cfg.api_key
        .clone()
        .or_else(|| std::env::var("ANANSI_OPENROUTER_API_KEY").ok())
        .ok_or_else(|| anyhow::anyhow!(
            "OpenRouter API key not found. Set [llm.openrouter] api_key in anansi.toml \
             or export ANANSI_OPENROUTER_API_KEY"
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_example_config() {
        let dir = tempfile::tempdir().unwrap();
        let anansi_dir = dir.path().join("anansi");
        std::fs::create_dir_all(&anansi_dir).unwrap();
        let config_path = anansi_dir.join("anansi.toml");
        let mut f = std::fs::File::create(&config_path).unwrap();
        write!(f, "{}", include_str!("../anansi.toml.example")).unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert_eq!(cfg.llm.backend, "ollama");
        assert_eq!(cfg.server.mcp_port, 3738);
        assert!(!cfg.server.read_only);
    }
}
