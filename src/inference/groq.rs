// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use crate::config::loader::InferenceConfig;
use crate::inference::InferenceContext;
use serde::{Deserialize, Serialize};

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

pub fn is_available() -> bool {
    std::env::var("GROQ_API_KEY").is_ok()
}

fn api_key(config: &InferenceConfig) -> Option<String> {
    let env_name = if config.api_key_env.is_empty() {
        "GROQ_API_KEY"
    } else {
        config.api_key_env.as_str()
    };
    std::env::var(env_name).ok()
}

pub async fn generate(
    config: &InferenceConfig,
    _ctx: &InferenceContext,
    user_prompt: &str,
) -> Option<String> {
    let key = api_key(config)?;
    if key.trim().is_empty() {
        return None;
    }

    let model = crate::inference::resolve_model(config);

    let req = ChatRequest {
        model,
        messages: vec![
            Message { role: "system".to_string(), content: "You are Lwoodz, a precise software licensing advisor. You answer concisely, cite SPDX identifiers, and never hallucinate license text. When asked to generate a file, output only the file content.".to_string() },
            Message { role: "user".to_string(), content: user_prompt.to_string() },
        ],
        temperature: 0.2,
        max_tokens: 2048,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let body = send_with_retry(&client, &key, &req).await?;
    let text = body.choices.first()?.message.content.clone();
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// A network hiccup or a transient 5xx from Groq shouldn't sink an
/// otherwise-good request. Retries only what's actually transient — never a
/// 4xx (bad key, bad request), which a retry can't fix — with a short
/// exponential backoff so a real outage doesn't turn one call into a stall.
async fn send_with_retry(
    client: &reqwest::Client,
    key: &str,
    req: &ChatRequest,
) -> Option<ChatResponse> {
    const MAX_ATTEMPTS: u32 = 3;
    const BASE_DELAY: std::time::Duration = std::time::Duration::from_millis(250);

    let mut last_err: Option<String> = None;
    for attempt in 0..MAX_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(BASE_DELAY * 2u32.pow(attempt - 1)).await;
        }

        let sent = client
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {key}"))
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await;

        let resp = match sent {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(e.to_string());
                tracing::warn!(attempt, error = %e, "Groq request failed, will retry if attempts remain");
                continue;
            }
        };

        let status = resp.status();
        if status.is_success() {
            return resp.json::<ChatResponse>().await.ok();
        }

        let retryable = status.is_server_error() || status.as_u16() == 429;
        let body_text = resp.text().await.unwrap_or_default();
        if retryable && attempt + 1 < MAX_ATTEMPTS {
            tracing::warn!(attempt, %status, body = %body_text, "Groq API error, retrying");
            last_err = Some(format!("{status}: {body_text}"));
            continue;
        }
        tracing::warn!(%status, body = %body_text, "Groq API error, giving up");
        return None;
    }

    tracing::warn!(error = ?last_err, "Groq request exhausted retries");
    None
}

/// Lightweight helper used by CLI `lwoodz explain` — streams a short answer.
pub async fn explain_license(config: &InferenceConfig, spdx: &str) -> Option<String> {
    let ctx = InferenceContext {
        project_license: spdx.to_string(),
        holder: String::new(),
        year: 0,
        dependencies: vec![],
        compatibility_issues: vec![],
        existing_license_text: None,
    };
    generate(config, &ctx, &format!("Explain the SPDX license '{}' in 3 bullet points: permissions, conditions, limitations. Keep it concise.", spdx)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn api_key_missing_returns_none() {
        // ensure no key leaks in test — just check the helper handles missing gracefully
        let cfg = InferenceConfig {
            enabled: true,
            provider: "groq".to_string(),
            model: "auto".to_string(),
            groq_model: "llama-3.3-70b-versatile".to_string(),
            api_key_env: "LWOODZ_TEST_MISSING_KEY_12345".to_string(),
        };
        // no env set => generate will return None without network
        assert!(api_key(&cfg).is_none());
    }
}
