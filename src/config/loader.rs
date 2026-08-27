// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_license() -> String {
    "MIT".to_string()
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_year() -> i32 {
    crate::util::current_year()
}
fn default_holder() -> String {
    "Your Name".to_string()
}
fn default_license_file() -> String {
    "LICENSE".to_string()
}
fn default_notice_file() -> String {
    "NOTICE".to_string()
}
fn default_copyright_file() -> String {
    "COPYRIGHT".to_string()
}
fn default_attribution_file() -> String {
    "THIRD_PARTY_NOTICES".to_string()
}
fn default_manifest_path() -> String {
    ".lwoodz/manifest.json".to_string()
}

// ---------------------------------------------------------------------------
// Top-level config — mirrors kaptaind.toml philosophy but for legal state.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub operation: OperationConfig,
    #[serde(default)]
    pub watch: WatchConfig,
    #[serde(default)]
    pub generate: GenerateConfig,
    #[serde(default)]
    pub headers: HeadersConfig,
    #[serde(default)]
    pub compatibility: CompatibilityConfig,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub spdx: SpdxConfig,
    #[serde(default)]
    pub audit: AuditConfig,

    /// Resolved repo root (not serialized in file, set at load time).
    #[serde(skip)]
    pub repo_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            operation: OperationConfig::default(),
            watch: WatchConfig::default(),
            generate: GenerateConfig::default(),
            headers: HeadersConfig::default(),
            compatibility: CompatibilityConfig::default(),
            daemon: DaemonConfig::default(),
            inference: InferenceConfig::default(),
            spdx: SpdxConfig::default(),
            audit: AuditConfig::default(),
            repo_path: PathBuf::from("."),
        }
    }
}

// ---------------------------------------------------------------------------
// Project identity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectConfig {
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(default = "default_holder")]
    pub copyright_holder: String,
    #[serde(default = "default_year")]
    pub copyright_year: i32,
    #[serde(default)]
    pub copyright_holder_email: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub project_url: Option<String>,
    #[serde(default)]
    pub dual_license: Option<String>,
    #[serde(default)]
    pub commercial_license: Option<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            license: default_license(),
            copyright_holder: default_holder(),
            copyright_year: default_year(),
            copyright_holder_email: None,
            project_name: None,
            project_url: None,
            dual_license: None,
            commercial_license: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Operation mode (observe vs enforce) — same pattern as kaptaind.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OperationMode {
    #[default]
    Observe,
    Enforce,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OperationConfig {
    #[serde(default)]
    pub mode: OperationMode,
}

// ---------------------------------------------------------------------------
// Watch
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WatchConfig {
    #[serde(default = "default_watch_path")]
    pub path: String,
    #[serde(default = "default_true")]
    pub recursive: bool,
    #[serde(default = "default_ignore_file")]
    pub ignore_file: String,
}

fn default_watch_path() -> String {
    ".".to_string()
}
fn default_ignore_file() -> String {
    ".lwoodignore".to_string()
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            path: default_watch_path(),
            recursive: true,
            ignore_file: default_ignore_file(),
        }
    }
}

// ---------------------------------------------------------------------------
// Generate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateConfig {
    #[serde(default = "default_license_file")]
    pub license_file: String,
    #[serde(default = "default_notice_file")]
    pub notice_file: String,
    #[serde(default = "default_copyright_file")]
    pub copyright_file: String,
    #[serde(default = "default_attribution_file")]
    pub attribution_file: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            license_file: default_license_file(),
            notice_file: default_notice_file(),
            copyright_file: default_copyright_file(),
            attribution_file: default_attribution_file(),
            overwrite: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Headers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeadersConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub insert_spdx: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

impl Default for HeadersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            insert_spdx: true,
            exclude: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                ".git/**".to_string(),
                ".lwoodz/**".to_string(),
                "vendor/**".to_string(),
            ],
            include: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Compatibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompatibilityConfig {
    #[serde(default = "default_true")]
    pub enforce: bool,
    #[serde(default = "default_policy")]
    pub policy: String,
}

fn default_policy() -> String {
    "permissive".to_string()
}

impl Default for CompatibilityConfig {
    fn default() -> Self {
        Self {
            enforce: true,
            policy: default_policy(),
        }
    }
}

// ---------------------------------------------------------------------------
// Daemon
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DaemonConfig {
    #[serde(default)]
    pub startup_guard: bool,
    #[serde(default = "default_shutdown_grace")]
    pub shutdown_grace_secs: u64,
}

fn default_shutdown_grace() -> u64 {
    10
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            startup_guard: false,
            shutdown_grace_secs: default_shutdown_grace(),
        }
    }
}

// ---------------------------------------------------------------------------
// Inference — GROQ is the primary provider.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InferenceConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_groq_model")]
    pub groq_model: String,
    #[serde(default)]
    pub api_key_env: String,
}

fn default_provider() -> String {
    "groq".to_string()
}
fn default_model() -> String {
    "auto".to_string()
}
fn default_groq_model() -> String {
    "llama-3.3-70b-versatile".to_string()
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: default_provider(),
            model: default_model(),
            groq_model: default_groq_model(),
            api_key_env: "GROQ_API_KEY".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// SPDX
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpdxConfig {
    #[serde(default = "default_true")]
    pub produce_manifest: bool,
    #[serde(default = "default_manifest_path")]
    pub manifest_path: String,
    #[serde(default = "default_true")]
    pub include_transitive: bool,
}

impl Default for SpdxConfig {
    fn default() -> Self {
        Self {
            produce_manifest: true,
            manifest_path: default_manifest_path(),
            include_transitive: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_audit_path")]
    pub log_path: String,
    #[serde(default = "default_false")]
    pub fail_on_incompatible: bool,
}

fn default_audit_path() -> String {
    ".lwoodz/audit.jsonl".to_string()
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: default_audit_path(),
            fail_on_incompatible: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

pub fn find_config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("LWOODZ_CONFIG") {
        return Some(PathBuf::from(p));
    }
    let candidates = ["lwoodz.toml", ".lwoodz.toml", ".lwoodz/config.toml"];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    // walk up to repo root
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cur = cwd.as_path();
    loop {
        for c in candidates {
            let p = cur.join(c);
            if p.exists() {
                return Some(p);
            }
        }
        match cur.parent() {
            Some(parent) => cur = parent,
            None => break,
        }
    }
    None
}

pub fn load() -> anyhow::Result<Config> {
    let path = find_config_path();
    load_from(path.as_deref())
}

pub fn load_from(path: Option<&Path>) -> anyhow::Result<Config> {
    let mut cfg = Config::default();

    if let Some(p) = path {
        let raw = std::fs::read_to_string(p)?;
        let parsed: Config = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", p.display(), e))?;
        cfg = parsed;
        // Determine repo_path: parent of config file, handling relative paths like "lwoodz.toml"
        let parent = p
            .parent()
            .filter(|pp| !pp.as_os_str().is_empty())
            .map(|pp| pp.to_path_buf());
        cfg.repo_path = if let Some(pp) = parent {
            pp
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };
        // canonicalise repo_path
        if let Ok(canonical) = cfg.repo_path.canonicalize() {
            cfg.repo_path = canonical;
        } else if cfg.repo_path.as_os_str().is_empty() {
            cfg.repo_path = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .canonicalize()
                .unwrap_or(PathBuf::from("."));
        }
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        cfg.repo_path = cwd.canonicalize().unwrap_or(cfg.repo_path);
        // try to infer project name from directory
        if cfg.project.project_name.is_none() {
            if let Some(name) = cfg.repo_path.file_name().and_then(|n| n.to_str()) {
                cfg.project.project_name = Some(name.to_string());
            }
        }
    }

    // env overrides for inference
    if let Ok(v) = std::env::var("LWOODZ_LICENSE") {
        cfg.project.license = v;
    }
    if let Ok(v) = std::env::var("LWOODZ_HOLDER") {
        cfg.project.copyright_holder = v;
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.project.license, "MIT");
        assert_eq!(c.operation.mode, OperationMode::Observe);
        assert!(c.headers.enabled);
        assert_eq!(c.inference.provider, "groq");
    }

    #[test]
    fn load_from_inline_toml() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
            [project]
            license = "Apache-2.0"
            copyright_holder = "Acme Inc"
            copyright_year = 2024
            [operation]
            mode = "enforce"
        "#
        )
        .unwrap();
        let cfg = load_from(Some(f.path())).unwrap();
        assert_eq!(cfg.project.license, "Apache-2.0");
        assert_eq!(cfg.project.copyright_holder, "Acme Inc");
        assert_eq!(cfg.operation.mode, OperationMode::Enforce);
    }
}
