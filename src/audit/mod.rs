// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::license::compatibility::{build_report, Compatibility};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    pub level: String, // "error" | "warning" | "info"
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub project_license: String,
    pub holder: String,
    pub year: i32,
    pub has_license_file: bool,
    pub detected_license: Option<String>,
    pub spdx_valid: bool,
    pub spdx_manifest_exists: bool,
    pub header_coverage: Option<HeaderCoverage>,
    pub compatibility: CompatibilitySummary,
    pub findings: Vec<AuditFinding>,
    pub passed: bool,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilitySummary {
    pub total_deps: usize,
    pub incompatible: usize,
    pub warnings: usize,
    pub issues: Vec<IssueView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueView {
    pub dependency: String,
    pub dep_license: String,
    pub severity: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderCoverage {
    pub total_files: usize,
    pub with_header: usize,
    pub missing: usize,
}

pub fn run(cfg: &Config) -> anyhow::Result<AuditReport> {
    let mut findings = Vec::new();

    // 1. LICENSE file existence + content sniff
    let license_path = cfg.repo_path.join(&cfg.generate.license_file);
    let has_license_file = license_path.exists();
    if !has_license_file {
        findings.push(AuditFinding {
            level: "error".to_string(),
            code: "MISSING_LICENSE_FILE".to_string(),
            message: format!(
                "{} not found at {}",
                cfg.generate.license_file,
                license_path.display()
            ),
            path: Some(cfg.generate.license_file.clone()),
        });
    }

    let detected_license = if has_license_file {
        let text = std::fs::read_to_string(&license_path).unwrap_or_default();
        crate::license::spdx::detect_spdx_from_text(&text)
    } else {
        None
    };

    if let Some(ref det) = detected_license {
        let norm_project = crate::license::spdx::normalize_spdx(&cfg.project.license);
        if crate::license::spdx::normalize_spdx(det) != norm_project {
            findings.push(AuditFinding {
                level: "warning".to_string(),
                code: "LICENSE_MISMATCH".to_string(),
                message: format!(
                    "LICENSE file appears to be '{}' but lwoodz.toml declares '{}'",
                    det, norm_project
                ),
                path: Some(cfg.generate.license_file.clone()),
            });
        }
    }

    // 2. SPDX validity
    let spdx_valid = crate::license::spdx::is_valid_spdx(&cfg.project.license);
    if !spdx_valid {
        findings.push(AuditFinding {
            level: "error".to_string(),
            code: "INVALID_SPDX".to_string(),
            message: format!(
                "'{}' is not a recognized SPDX identifier",
                cfg.project.license
            ),
            path: Some("lwoodz.toml".to_string()),
        });
    }

    // 3. SPDX manifest
    let manifest_path = cfg.repo_path.join(&cfg.spdx.manifest_path);
    let spdx_manifest_exists = manifest_path.exists();
    if cfg.spdx.produce_manifest && !spdx_manifest_exists {
        findings.push(AuditFinding {
            level: "warning".to_string(),
            code: "MISSING_SPDX_MANIFEST".to_string(),
            message: format!(
                "SPDX manifest not found at {} — run `lwoodz generate`",
                cfg.spdx.manifest_path
            ),
            path: Some(cfg.spdx.manifest_path.clone()),
        });
    }

    // 4. Dependency compatibility
    let manifest = crate::manifest::scan(&cfg.repo_path);
    let license_pairs: Vec<(String, String)> = manifest
        .dependencies
        .iter()
        .filter_map(|d| {
            d.license
                .as_ref()
                .map(|l| (d.name.clone(), crate::license::spdx::normalize_spdx(l)))
        })
        .collect();

    // For deps without license, flag as warning
    let unknown_licenses: Vec<String> = manifest
        .dependencies
        .iter()
        .filter(|d| d.license.is_none())
        .map(|d| d.name.clone())
        .collect();
    if !unknown_licenses.is_empty() {
        findings.push(AuditFinding {
            level: "warning".to_string(),
            code: "UNKNOWN_DEP_LICENSE".to_string(),
            message: format!(
                "{} dependencies have unknown licenses: {}",
                unknown_licenses.len(),
                unknown_licenses.join(", ")
            ),
            path: None,
        });
    }

    let compat_report = build_report(
        &crate::license::spdx::normalize_spdx(&cfg.project.license),
        &license_pairs,
    );
    for issue in &compat_report.issues {
        let level = if issue.compatibility == Compatibility::Incompatible {
            "error"
        } else {
            "warning"
        };
        findings.push(AuditFinding {
            level: level.to_string(),
            code: if issue.compatibility == Compatibility::Incompatible {
                "INCOMPATIBLE_DEP"
            } else {
                "COMPAT_WARNING"
            }
            .to_string(),
            message: issue.reason.clone(),
            path: None,
        });
    }

    // 5. Header coverage (sample scan, capped)
    let header_coverage = if cfg.headers.enabled {
        sample_header_coverage(&cfg.repo_path, &cfg.headers.exclude).map(|(total, with)| {
            HeaderCoverage {
                total_files: total,
                with_header: with,
                missing: total.saturating_sub(with),
            }
        })
    } else {
        None
    };

    if let Some(ref hc) = header_coverage {
        if hc.missing > 0 {
            findings.push(AuditFinding {
                level: "warning".to_string(),
                code: "MISSING_HEADERS".to_string(),
                message: format!(
                    "{} of {} source files missing copyright/SPDX headers",
                    hc.missing, hc.total_files
                ),
                path: None,
            });
        }
    }

    // 6. Dual-license sanity
    if let Some(dual) = &cfg.project.dual_license {
        if !crate::license::spdx::is_valid_spdx(dual) {
            findings.push(AuditFinding {
                level: "error".to_string(),
                code: "INVALID_DUAL_SPDX".to_string(),
                message: format!("dual_license '{}' is not a valid SPDX id", dual),
                path: Some("lwoodz.toml".to_string()),
            });
        }
    }

    let has_error = findings.iter().any(|f| f.level == "error");
    let passed = !has_error
        && (if cfg.audit.fail_on_incompatible {
            compat_report.incompatible_count == 0
        } else {
            true
        });

    // Write audit log if enabled
    if cfg.audit.enabled {
        let log_path = cfg.repo_path.join(&cfg.audit.log_path);
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let line = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "project_license": cfg.project.license,
            "passed": passed,
            "findings": findings.len(),
            "incompatible": compat_report.incompatible_count,
        });
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            use std::io::Write;
            let _ = writeln!(f, "{}", line);
        }
    }

    let issues_view = compat_report
        .issues
        .iter()
        .map(|i| IssueView {
            dependency: i.dependency.clone(),
            dep_license: i.dep_license.clone(),
            severity: if i.compatibility == Compatibility::Incompatible {
                "error".to_string()
            } else {
                "warning".to_string()
            },
            reason: i.reason.clone(),
        })
        .collect();

    Ok(AuditReport {
        project_license: crate::license::spdx::normalize_spdx(&cfg.project.license),
        holder: cfg.project.copyright_holder.clone(),
        year: cfg.project.copyright_year,
        has_license_file,
        detected_license,
        spdx_valid,
        spdx_manifest_exists,
        header_coverage,
        compatibility: CompatibilitySummary {
            total_deps: manifest.dependencies.len(),
            incompatible: compat_report.incompatible_count,
            warnings: compat_report.warning_count,
            issues: issues_view,
        },
        findings,
        passed,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn sample_header_coverage(repo_root: &Path, exclude: &[String]) -> Option<(usize, usize)> {
    use ignore::gitignore::GitignoreBuilder;
    use walkdir::WalkDir;

    let mut builder = GitignoreBuilder::new(repo_root);
    for pat in exclude {
        let _ = builder.add_line(None, pat);
    }
    let _ = builder.add_line(None, ".git/");
    let _ = builder.add_line(None, ".lwoodz/");
    let _ = builder.add_line(None, "target/");
    let ignore = builder.build().ok()?;

    let exts = ["rs", "go", "py", "js", "ts"];
    let mut total = 0usize;
    let mut with = 0usize;
    let mut count = 0usize;
    for entry in WalkDir::new(repo_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if count > 500 {
            break;
        } // cap sample
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if ignore.matched(p, false).is_ignore() {
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !exts.contains(&ext) {
            continue;
        }
        total += 1;
        count += 1;
        if let Ok(content) = std::fs::read_to_string(p) {
            let lower = content.to_lowercase();
            if lower.contains("copyright") || lower.contains("spdx-license-identifier") {
                with += 1;
            }
        }
    }
    if total == 0 {
        None
    } else {
        Some((total, with))
    }
}
