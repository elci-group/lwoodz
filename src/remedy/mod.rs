// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! `lwoodz remedy` — conducts contextualised fixes to a repository's legal
//! documents (LICENSE, NOTICE, COPYRIGHT, THIRD_PARTY_NOTICES, SPDX manifest,
//! source headers). It replaces the old `--generate` flag: instead of blindly
//! (re)writing templates, it audits first, repairs what's broken, and — where
//! the `ami` market-intelligence CLI is available — uses its project analysis
//! as a context clue to fill in metadata the user never configured.

use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Project context recovered from `ami show-project`, used as a hint when
/// `lwoodz.toml` is missing metadata. Best-effort: `ami` is an optional,
/// separately-installed tool, so absence of a value (or of the whole struct)
/// is expected and never an error.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AmiContext {
    pub name: Option<String>,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub stage: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemedyFix {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemedyGenerateSummary {
    pub license: Option<String>,
    pub notice: Option<String>,
    pub copyright: Option<String>,
    pub attribution: Option<String>,
    pub manifest: Option<String>,
    pub written: Vec<String>,
    pub skipped: Vec<String>,
}

impl RemedyGenerateSummary {
    fn from_result(res: &crate::generate::GenerateResult) -> Self {
        Self {
            license: Some(res.license_path.display().to_string()),
            notice: res.notice_path.as_ref().map(|p| p.display().to_string()),
            copyright: res.copyright_path.as_ref().map(|p| p.display().to_string()),
            attribution: res
                .attribution_path
                .as_ref()
                .map(|p| p.display().to_string()),
            manifest: res.manifest_path.as_ref().map(|p| p.display().to_string()),
            written: res
                .written
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            skipped: res
                .skipped
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemedyReport {
    pub dry_run: bool,
    pub ami_context: Option<AmiContext>,
    pub findings_before: usize,
    pub findings_after: usize,
    pub fixes: Vec<RemedyFix>,
    pub generate: RemedyGenerateSummary,
    pub headers_scanned: usize,
    pub headers_inserted: usize,
    pub headers_updated: usize,
}

/// Run an audit, repair whatever it finds (regenerating legal documents and
/// syncing source headers), and report what changed. In `dry_run` mode
/// nothing is written; the report only describes what would happen.
pub fn run(cfg: &mut Config, dry_run: bool) -> anyhow::Result<RemedyReport> {
    let before = crate::audit::run(cfg)?;
    let ami_context = gather_ami_context(&cfg.repo_path);
    let mut fixes = Vec::new();

    if let Some(ctx) = &ami_context {
        if cfg.project.project_name.is_none() {
            if let Some(name) = ctx.name.clone() {
                if !dry_run {
                    cfg.project.project_name = Some(name.clone());
                }
                fixes.push(RemedyFix {
                    code: "AMI_CONTEXT_PROJECT_NAME".to_string(),
                    message: format!(
                        "{} project_name from ami analysis: '{}'",
                        if dry_run { "Would fill" } else { "Filled" },
                        name
                    ),
                    path: Some("lwoodz.toml".to_string()),
                });
            }
        }
        if cfg.project.project_url.is_none() {
            if let Some(repo) = ctx.repository.clone() {
                if !dry_run {
                    cfg.project.project_url = Some(repo.clone());
                }
                fixes.push(RemedyFix {
                    code: "AMI_CONTEXT_PROJECT_URL".to_string(),
                    message: format!(
                        "{} project_url from ami analysis: '{}'",
                        if dry_run { "Would fill" } else { "Filled" },
                        repo
                    ),
                    path: Some("lwoodz.toml".to_string()),
                });
            }
        }
    }

    let mut generate_summary = RemedyGenerateSummary::default();
    let mut headers_scanned = 0usize;
    let mut headers_inserted = 0usize;
    let mut headers_updated = 0usize;

    if dry_run {
        if !before.findings.is_empty() {
            fixes.push(RemedyFix {
                code: "DRY_RUN_PENDING_FINDINGS".to_string(),
                message: format!(
                    "Would attempt to resolve {} finding(s) via document regeneration and header sync",
                    before.findings.len()
                ),
                path: None,
            });
        }
    } else {
        cfg.generate.overwrite = true;
        let res = crate::generate::generate_license_file(cfg)?;
        generate_summary = RemedyGenerateSummary::from_result(&res);

        if cfg.headers.enabled {
            let hc = crate::license::header::HeaderConfig {
                holder: cfg.project.copyright_holder.clone(),
                year: cfg.project.copyright_year,
                spdx: crate::license::spdx::normalize_spdx(&cfg.project.license),
                insert_spdx: cfg.headers.insert_spdx,
            };
            let results = crate::license::header::ensure_headers(
                &cfg.repo_path,
                &hc,
                &cfg.headers.exclude,
                &cfg.headers.include,
            )?;
            headers_scanned = results.len();
            headers_inserted = results
                .iter()
                .filter(|r| r.action == crate::license::header::HeaderAction::Inserted)
                .count();
            headers_updated = results
                .iter()
                .filter(|r| r.action == crate::license::header::HeaderAction::Updated)
                .count();
        }
    }

    let after = if dry_run {
        before.clone()
    } else {
        crate::audit::run(cfg)?
    };

    let resolved = before.findings.iter().filter(|f| {
        !after
            .findings
            .iter()
            .any(|af| af.code == f.code && af.path == f.path)
    });
    for f in resolved {
        fixes.push(RemedyFix {
            code: format!("RESOLVED_{}", f.code),
            message: format!("Resolved: {}", f.message),
            path: f.path.clone(),
        });
    }

    Ok(RemedyReport {
        dry_run,
        ami_context,
        findings_before: before.findings.len(),
        findings_after: after.findings.len(),
        fixes,
        generate: generate_summary,
        headers_scanned,
        headers_inserted,
        headers_updated,
    })
}

/// Best-effort: shell out to `ami show-project` and scrape its table output.
/// Returns `None` if `ami` isn't installed, fails, or yields nothing useful —
/// this is a context clue, never a hard dependency.
pub fn gather_ami_context(repo_path: &Path) -> Option<AmiContext> {
    let output = std::process::Command::new("ami")
        .arg("show-project")
        .arg("-p")
        .arg(repo_path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_ami_show_project(&String::from_utf8_lossy(&output.stdout))
}

fn parse_ami_show_project(text: &str) -> Option<AmiContext> {
    let mut ctx = AmiContext::default();
    let mut found_any = false;

    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('│') {
            continue;
        }
        let cols: Vec<&str> = line
            .trim_matches('│')
            .split('│')
            .map(|s| s.trim())
            .collect();
        if cols.len() != 2 {
            continue;
        }
        let (field, value) = (cols[0], cols[1]);
        if field.is_empty() || field == "Field" || value.is_empty() {
            continue;
        }
        match field {
            "Name" => {
                ctx.name = Some(value.to_string());
                found_any = true;
            }
            "Description" => {
                ctx.description = Some(value.trim_end_matches('…').trim().to_string());
                found_any = true;
            }
            "Repository" => {
                if let Some(v) = strip_option_wrapper(value) {
                    ctx.repository = Some(v);
                    found_any = true;
                }
            }
            "Development stage" => {
                ctx.stage = Some(value.to_string());
                found_any = true;
            }
            _ => {}
        }
    }

    if let Some(idx) = text.find("Keywords") {
        if let Some(kw_line) = text[idx..].lines().nth(2) {
            let kw_line = kw_line.trim();
            if !kw_line.is_empty() {
                ctx.keywords = kw_line
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                found_any = true;
            }
        }
    }

    found_any.then_some(ctx)
}

/// ami prints `Option<T>` fields in Rust debug form (`Some("...")` / `None`).
fn strip_option_wrapper(value: &str) -> Option<String> {
    if value == "None" {
        return None;
    }
    value
        .strip_prefix("Some(\"")
        .and_then(|s| s.strip_suffix("\")"))
        .map(|s| s.to_string())
        .or_else(|| Some(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_cfg(repo: &Path) -> Config {
        Config {
            repo_path: repo.to_path_buf(),
            ..Config::default()
        }
    }

    #[test]
    fn parses_ami_show_project_table() {
        let sample = "📖 Project information for: .\n\
┌───────────────────┬──────────────────────────────────────────────────────────────────┐\n\
│ Field             │ Value                                                            │\n\
├───────────────────┼──────────────────────────────────────────────────────────────────┤\n\
│ Name              │ lwoodz                                                           │\n\
│ Description       │ Lwoodz is to license document generation and maintenance what K… │\n\
│ Repository        │ Some(\"https://github.com/elci-group/lwoodz\")                     │\n\
│ Website           │ None                                                             │\n\
│ Development stage │ Beta                                                             │\n\
└───────────────────┴──────────────────────────────────────────────────────────────────┘\n\
\n\
🔑 Keywords · 5 total\n\
compliance, daemon, legal, license, spdx\n";
        let ctx = parse_ami_show_project(sample).expect("should parse a context");
        assert_eq!(ctx.name.as_deref(), Some("lwoodz"));
        assert_eq!(
            ctx.repository.as_deref(),
            Some("https://github.com/elci-group/lwoodz")
        );
        assert_eq!(ctx.stage.as_deref(), Some("Beta"));
        assert_eq!(
            ctx.keywords,
            vec!["compliance", "daemon", "legal", "license", "spdx"]
        );
    }

    #[test]
    fn parses_empty_text_as_none() {
        assert!(parse_ami_show_project("no table here").is_none());
    }

    #[test]
    fn remedy_fixes_missing_license_and_reports_resolved_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.project.copyright_holder = "Acme Inc".to_string();
        cfg.project.copyright_year = 2024;

        let report = run(&mut cfg, false).unwrap();
        assert!(dir.path().join("LICENSE").exists());
        assert!(report
            .fixes
            .iter()
            .any(|f| f.code == "RESOLVED_MISSING_LICENSE_FILE"));
        assert_eq!(report.findings_after, 0);
    }

    #[test]
    fn dry_run_does_not_write_license() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());

        let report = run(&mut cfg, true).unwrap();
        assert!(!dir.path().join("LICENSE").exists());
        assert!(report.dry_run);
        assert!(report
            .fixes
            .iter()
            .any(|f| f.code == "DRY_RUN_PENDING_FINDINGS"));
    }
}
