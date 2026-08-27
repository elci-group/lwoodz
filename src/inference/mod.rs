// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
#![allow(unused_variables)] // Legacy tracing field bindings are stringified by telemetry.
pub mod groq;

use crate::config::loader::InferenceConfig;
use crate::telemetry as tracing;

#[derive(Debug, Clone)]
pub struct InferenceContext {
    pub project_license: String,
    pub holder: String,
    pub year: i32,
    pub dependencies: Vec<(String, String)>,
    pub compatibility_issues: Vec<String>,
    pub existing_license_text: Option<String>,
}

pub fn resolve_model(config: &InferenceConfig) -> String {
    if config.model != "auto" {
        return config.model.clone();
    }
    config.groq_model.clone()
}

pub fn is_available(config: &InferenceConfig) -> bool {
    if !config.enabled {
        return false;
    }
    let env_key = if config.api_key_env.is_empty() {
        "GROQ_API_KEY"
    } else {
        config.api_key_env.as_str()
    };
    std::env::var(env_key).is_ok()
}

/// High-level dispatch. Only Groq is implemented today; an unrecognized
/// `provider` fails closed (returns `None` with a warning) rather than
/// silently calling Groq's endpoint with whatever model string was meant
/// for a different provider.
pub async fn advise(
    config: &InferenceConfig,
    ctx: &InferenceContext,
    prompt: &str,
) -> Option<String> {
    if !config.enabled {
        return None;
    }
    match config.provider.as_str() {
        "groq" => groq::generate(config, ctx, prompt).await,
        other => {
            tracing::warn!(provider = other, "unsupported inference provider, skipping");
            None
        }
    }
}

/// Suggest a license for the project based on dependencies and intent.
pub async fn suggest_license(config: &InferenceConfig, ctx: &InferenceContext) -> Option<String> {
    let prompt = format!(
        "You are a licensing advisor. Project holder: {} ({}), current license: {}. Dependencies with licenses: {:?}. Issues: {:?}. Recommend the most appropriate SPDX license identifier and give a one-paragraph justification. Reply with SPDX id on first line, then justification.",
        ctx.holder, ctx.year, ctx.project_license, ctx.dependencies, ctx.compatibility_issues
    );
    advise(config, ctx, &prompt).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> InferenceContext {
        InferenceContext {
            project_license: "MIT".to_string(),
            holder: "Acme".to_string(),
            year: 2026,
            dependencies: vec![],
            compatibility_issues: vec![],
            existing_license_text: None,
        }
    }

    #[tokio::test]
    async fn unsupported_provider_fails_closed_without_calling_groq() {
        let cfg = InferenceConfig {
            enabled: true,
            provider: "openai".to_string(),
            model: "auto".to_string(),
            groq_model: "llama-3.3-70b-versatile".to_string(),
            api_key_env: String::new(),
        };
        assert!(advise(&cfg, &ctx(), "hello").await.is_none());
    }

    #[tokio::test]
    async fn disabled_config_short_circuits() {
        let cfg = InferenceConfig {
            enabled: false,
            provider: "groq".to_string(),
            model: "auto".to_string(),
            groq_model: "llama-3.3-70b-versatile".to_string(),
            api_key_env: String::new(),
        };
        assert!(advise(&cfg, &ctx(), "hello").await.is_none());
    }
}
