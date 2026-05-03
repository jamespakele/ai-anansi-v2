//! Gemini embedding API client.
//!
//! Calls `text-embedding-004` via Google's REST API to produce 768-dimensional
//! float vectors. Uses the existing `reqwest` client (0.12, json + rustls-tls).

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;

/// Maximum bytes of text to send in a single embedding request.
/// Keeps us well within the 2048-token API limit.
const MAX_TEXT_BYTES: usize = 8_000;

/// Call `text-embedding-004` and return a 768-dimensional embedding vector.
///
/// # Arguments
/// * `api_key` - Gemini REST API key (from `ANANSI_GEMINI_API_KEY`)
/// * `text`    - Text to embed; truncated to `MAX_TEXT_BYTES` bytes if longer
///
/// # Errors
/// Returns `Err` on HTTP failure, non-200 response, or unexpected vector dimension.
pub async fn gemini_embed(api_key: &str, text: &str) -> Result<Vec<f32>> {
    // Truncate at a UTF-8 boundary to stay within the API token limit
    let text = if text.len() > MAX_TEXT_BYTES {
        let mut end = MAX_TEXT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    } else {
        text
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={api_key}"
    );

    let body = json!({
        "model": "models/text-embedding-004",
        "content": {
            "parts": [{ "text": text }]
        },
        "taskType": "RETRIEVAL_DOCUMENT"
    });

    let client = Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini embed API error {status}: {body_text}"));
    }

    let json: serde_json::Value = response.json().await?;
    let values = json["embedding"]["values"]
        .as_array()
        .ok_or_else(|| anyhow!("Gemini embed response missing embedding.values field"))?;

    let vec: Vec<f32> = values
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    if vec.len() != 768 {
        return Err(anyhow!(
            "Unexpected embedding dimension: expected 768, got {}",
            vec.len()
        ));
    }

    Ok(vec)
}
