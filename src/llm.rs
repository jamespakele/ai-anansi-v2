use std::time::Duration;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use crate::config::{LlmConfig, InferSettings, resolve_openrouter_api_key};

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

// ─── Ollama ───────────────────────────────────────────────────────────────────

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

// ─── Gemini CLI ───────────────────────────────────────────────────────────────
//
// Flags (confirmed via `gemini --help`):
//   -p / --prompt     : triggers headless mode; appended to stdin if stdin is provided
//   -m / --model      : model selection (e.g. "gemini-2.5-pro")
//   -o / --output-format : "text" | "json" | "stream-json"
//
// Prompt delivery: write full prompt to stdin, pass -p "" to trigger headless.
// The CLI treats stdin as the primary input and appends the -p value after it,
// so an empty -p leaves the prompt unchanged.

pub struct GeminiCliClient {
    cli_path: String,
    model: String,
    timeout: Duration,
}

impl GeminiCliClient {
    pub fn new(cli_path: String, model: String, timeout_s: u64) -> Self {
        Self {
            cli_path,
            model,
            timeout: Duration::from_secs(timeout_s),
        }
    }
}

#[async_trait]
impl LlmClient for GeminiCliClient {
    async fn infer(&self, prompt: &str, opts: InferOpts) -> Result<String> {
        use tokio::process::Command;
        use std::process::Stdio;

        let mut child = Command::new(&self.cli_path)
            .arg("-p").arg("")
            .arg("-m").arg(&self.model)
            .arg("--output-format").arg("text")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("failed to spawn gemini CLI at '{}': {e}", self.cli_path))?;

        // Write prompt to stdin then close it so the process sees EOF
        if let Some(mut stdin) = child.stdin.take() {
            // json_mode: the prompts already request JSON output; no extra prefix needed
            let _ = opts.json_mode; // acknowledged — handled by prompt instructions
            stdin.write_all(prompt.as_bytes()).await
                .map_err(|e| anyhow!("writing to gemini stdin: {e}"))?;
        }

        let output = tokio::time::timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| anyhow!("gemini CLI timed out after {}s", self.timeout.as_secs()))?
            .map_err(|e| anyhow!("gemini CLI wait failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "gemini CLI exited {:?}: {stderr}",
                output.status.code()
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("gemini CLI returned empty output. stderr: {stderr}"));
        }
        Ok(text)
    }

    async fn ping(&self) -> Result<()> {
        use tokio::process::Command;

        let output = Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| anyhow!("gemini CLI not found at '{}': {e}", self.cli_path))?;

        if !output.status.success() {
            return Err(anyhow!(
                "gemini CLI not reachable at '{}' (exit {:?})",
                self.cli_path,
                output.status.code()
            ));
        }
        Ok(())
    }
}

// ─── Gemini REST (fallback when CLI is unavailable) ───────────────────────────

pub struct GeminiRestClient {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl GeminiRestClient {
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
impl LlmClient for GeminiRestClient {
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
            "contents": [{ "parts": [{ "text": prompt }] }],
            "generationConfig": generation_config,
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini REST request failed ({status}): {text}"));
        }

        let json: serde_json::Value = resp.json().await?;
        let text = json
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("unexpected Gemini REST response shape: {json}"))?;
        Ok(text.to_string())
    }

    async fn ping(&self) -> Result<()> {
        let url = format!("{}/models?key={}", self.base_url, self.api_key);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "Gemini REST API unreachable. Check your API key. Status: {}",
                resp.status()
            ));
        }
        Ok(())
    }
}

// ─── OpenRouter (OpenAI-compatible, access to all frontier models) ────────────

pub struct OpenRouterClient {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model: String, base_url: Option<String>, timeout_s: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_s))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            api_key,
            model,
            base_url: base_url
                .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string()),
            client,
        }
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn infer(&self, prompt: &str, opts: InferOpts) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": opts.temperature,
            "max_tokens": opts.max_tokens,
        });
        if opts.json_mode {
            body["response_format"] = serde_json::json!({ "type": "json_object" });
        }

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenRouter request failed ({status}): {text}"));
        }

        let json: serde_json::Value = resp.json().await?;
        let text = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("unexpected OpenRouter response shape: {json}"))?;
        Ok(text.to_string())
    }

    async fn ping(&self) -> Result<()> {
        let url = format!("{}/models", self.base_url);
        let resp = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "OpenRouter API unreachable. Check your API key. Status: {}",
                resp.status()
            ));
        }
        Ok(())
    }
}

// ─── Codex CLI (OpenAI device-auth) ──────────────────────────────────────────
//
// Uses OpenAI Codex CLI authenticated via device auth (`codex auth login`).
// Token stored in ~/.codex/. No API key needed; uses ChatGPT Plus/Pro subscription.
//
// Prompt delivery: write prompt to stdin, capture stdout.

pub struct CodexCliClient {
    cli_path: String,
    timeout: Duration,
}

impl CodexCliClient {
    pub fn new(cli_path: String, timeout_s: u64) -> Self {
        Self { cli_path, timeout: Duration::from_secs(timeout_s) }
    }
}

#[async_trait]
impl LlmClient for CodexCliClient {
    async fn infer(&self, prompt: &str, _opts: InferOpts) -> Result<String> {
        use tokio::process::Command;
        use std::process::Stdio;

        let mut child = Command::new(&self.cli_path)
            .arg("-q")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("failed to spawn codex CLI at '{}': {e}", self.cli_path))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await
                .map_err(|e| anyhow!("writing to codex stdin: {e}"))?;
        }

        let output = tokio::time::timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| anyhow!("codex CLI timed out after {}s", self.timeout.as_secs()))?
            .map_err(|e| anyhow!("codex CLI wait failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("codex CLI exited {:?}: {stderr}", output.status.code()));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("codex CLI returned empty output. stderr: {stderr}"));
        }
        Ok(text)
    }

    async fn ping(&self) -> Result<()> {
        use tokio::process::Command;
        let output = Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| anyhow!("codex CLI not found at '{}': {e}", self.cli_path))?;
        if !output.status.success() {
            return Err(anyhow!(
                "codex CLI not reachable at '{}' (exit {:?})",
                self.cli_path, output.status.code()
            ));
        }
        Ok(())
    }
}

// ─── build_client ─────────────────────────────────────────────────────────────

pub fn build_client(config: &LlmConfig) -> Result<Box<dyn LlmClient>> {
    build_client_for_backend(&config.backend, config)
}

/// Build a client for a specific backend string, falling back to config defaults.
pub fn build_client_for_backend(backend: &str, config: &LlmConfig) -> Result<Box<dyn LlmClient>> {
    match backend {
        "ollama" => Ok(Box::new(OllamaClient::new(
            config.url.clone(),
            config.model.clone(),
            config.n_ctx,
            config.timeout_s,
        ))),

        "gemini" => {
            let gcfg = config.gemini.as_ref()
                .ok_or_else(|| anyhow!(
                    "backend = 'gemini' requires a [llm.gemini] config section"
                ))?;

            // CLI-first: search cli_path config then fall back to "gemini" in PATH
            let cli_candidate = gcfg.cli_path.clone().unwrap_or_else(|| "gemini".to_string());
            let cli_available = std::process::Command::new(&cli_candidate)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if cli_available {
                let model = if gcfg.model.is_empty() {
                    "gemini-2.5-pro".to_string()
                } else {
                    gcfg.model.clone()
                };
                return Ok(Box::new(GeminiCliClient::new(
                    cli_candidate,
                    model,
                    gcfg.timeout_s.unwrap_or(120),
                )));
            }

            // REST fallback
            if let Some(api_key) = gcfg.api_key.clone()
                .or_else(|| std::env::var("ANANSI_GEMINI_API_KEY").ok())
            {
                let model = if gcfg.model.is_empty() {
                    "gemini-2.5-pro".to_string()
                } else {
                    gcfg.model.clone()
                };
                return Ok(Box::new(GeminiRestClient::new(
                    api_key,
                    model,
                    None,
                    gcfg.timeout_s.unwrap_or(120),
                )));
            }

            Err(anyhow!(
                "gemini CLI not found in PATH and no api_key configured. \
                 Consider backend = \"openrouter\" for API-key access to Gemini and other models."
            ))
        }

        "openrouter" => {
            let ocfg = config.openrouter.as_ref()
                .ok_or_else(|| anyhow!(
                    "backend = 'openrouter' requires a [llm.openrouter] config section"
                ))?;
            let api_key = resolve_openrouter_api_key(ocfg)?;
            let model = if ocfg.model.is_empty() {
                return Err(anyhow!(
                    "backend = 'openrouter' requires [llm.openrouter] model to be set \
                     (e.g. \"google/gemini-2.5-pro\")"
                ));
            } else {
                ocfg.model.clone()
            };
            Ok(Box::new(OpenRouterClient::new(
                api_key,
                model,
                ocfg.base_url.clone(),
                ocfg.timeout_s.unwrap_or(120),
            )))
        }

        "codex" => {
            // OpenAI Codex CLI — device-auth, no API key required.
            // Token stored in ~/.codex/ after running `codex auth login`.
            let cli_candidate = std::env::var("ANANSI_CODEX_CLI_PATH")
                .unwrap_or_else(|_| "codex".to_string());
            Ok(Box::new(CodexCliClient::new(
                cli_candidate,
                config.timeout_s,
            )))
        }

        other => Err(anyhow!(
            "unknown LLM backend: '{other}'. Valid values: 'ollama', 'gemini', 'openrouter', 'codex'"
        )),
    }
}
