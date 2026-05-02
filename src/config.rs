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
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_web_dir")]
    pub web_dir: PathBuf,
    #[serde(default = "default_rules_dir")]
    pub rules_dir: PathBuf,
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
    #[serde(default = "default_db_file")]
    pub db_file: PathBuf,
}

fn default_web_dir() -> PathBuf { PathBuf::from("anansi/web") }
fn default_rules_dir() -> PathBuf { PathBuf::from("anansi/%Rules") }
fn default_templates_dir() -> PathBuf { PathBuf::from("anansi/templates") }
fn default_db_file() -> PathBuf { PathBuf::from("anansi/web.db") }

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

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: PathsConfig {
                web_dir: default_web_dir(),
                rules_dir: default_rules_dir(),
                templates_dir: default_templates_dir(),
                db_file: default_db_file(),
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
    /// Public base URL used to generate export download links, e.g. "https://vps.pakele.ai"
    pub public_url: Option<String>,
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
