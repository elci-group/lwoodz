// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Runtime status surface for the daemon, written to `.lwoodz/status.json`
/// on start and after every watch cycle — the same "small JSON file a CLI
/// can read without talking to the running process" pattern kaptaind uses
/// for its own `status.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub pid: u32,
    pub version: String,
    pub license: String,
    pub mode: String,
    pub repo_path: String,
    pub started_at: String,
    pub last_check_at: Option<String>,
    pub last_result: Option<String>,
    pub findings_count: Option<usize>,
}

impl DaemonStatus {
    pub fn starting(cfg: &crate::config::Config) -> Self {
        Self {
            pid: std::process::id(),
            version: crate::VERSION.to_string(),
            license: cfg.project.license.clone(),
            mode: format!("{:?}", cfg.operation.mode),
            repo_path: cfg.repo_path.display().to_string(),
            started_at: now_iso(),
            last_check_at: None,
            last_result: None,
            findings_count: None,
        }
    }

    pub fn record_check(&mut self, passed: bool, findings_count: usize) {
        self.last_check_at = Some(now_iso());
        self.last_result = Some(if passed { "pass" } else { "fail" }.to_string());
        self.findings_count = Some(findings_count);
    }

    pub fn write(&self, repo_root: &Path) -> anyhow::Result<()> {
        let path = default_path(repo_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

pub fn default_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".lwoodz").join("status.json")
}

pub fn read(repo_root: &Path) -> Option<DaemonStatus> {
    let text = std::fs::read_to_string(default_path(repo_root)).ok()?;
    serde_json::from_str(&text).ok()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = crate::config::Config::default();
        let mut status = DaemonStatus::starting(&cfg);
        status.record_check(true, 0);
        status.write(dir.path()).unwrap();

        let read_back = read(dir.path()).unwrap();
        assert_eq!(read_back.pid, std::process::id());
        assert_eq!(read_back.last_result.as_deref(), Some("pass"));
        assert_eq!(read_back.findings_count, Some(0));
    }

    #[test]
    fn read_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).is_none());
    }
}
