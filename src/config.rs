use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub paths: PathsConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub server: ServerConfig,
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

fn default_web_dir() -> PathBuf { PathBuf::from("web") }
fn default_rules_dir() -> PathBuf { PathBuf::from("%Rules") }
fn default_templates_dir() -> PathBuf { PathBuf::from("templates") }
fn default_db_file() -> PathBuf { PathBuf::from("web.db") }

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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerConfig {
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default)]
    pub read_only: bool,
}

fn default_mcp_port() -> u16 { 3738 }
fn default_host() -> String { "0.0.0.0".to_string() }

impl Config {
    pub fn load(anansi_root: &Path) -> Result<Self> {
        let path = anansi_root.join("anansi.toml");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let mut config: Config = toml::from_str(&text)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_example_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("anansi.toml");
        let mut f = std::fs::File::create(&config_path).unwrap();
        write!(f, "{}", include_str!("../anansi.toml.example")).unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert_eq!(cfg.llm.backend, "ollama");
        assert_eq!(cfg.server.mcp_port, 3738);
        assert!(!cfg.server.read_only);
    }
}
