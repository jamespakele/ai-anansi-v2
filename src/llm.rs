use std::time::Duration;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::config::LlmConfig;
use crate::config::InferSettings;

pub struct InferOpts {
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

impl InferOpts {
    pub fn from_settings(settings: &InferSettings) -> Self {
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
    async fn ping(&self) -> Result<()>;
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
        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Ollama response missing 'response' field: {json}"))
    }

    async fn ping(&self) -> Result<()> {
        let resp = self.client
            .get(format!("{}/api/tags", self.url))
            .send()
            .await?;
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
