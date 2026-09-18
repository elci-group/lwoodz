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
    pub service_terms: ServiceTermsSummary,
    pub openness: OpennessSummary,
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

/// Structured summary of [`crate::service_terms::analyze`], embedded so a
/// consumer of `lwoodz --json audit` (e.g. Uni's lwoodz adapter) doesn't have
/// to parse the free-text `SERVICE_TERMS_REVIEW` finding message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTermsSummary {
    pub matched: usize,
    pub providers: Vec<String>,
}

/// Structured summary of [`crate::openness::analyze`], embedded for the same
/// reason as [`ServiceTermsSummary`] — a consumer needs the counts without
/// re-deriving them from the `STANDARD_PATENT_RISK` finding message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpennessSummary {
    pub total_deps: usize,
    pub osi_approved: usize,
    pub open_standard: usize,
    pub rand_encumbered: usize,
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
    /// Set when the sample was capped: the full eligible file count found
    /// before the cap was applied, so a capped sample is distinguishable
    /// from a complete scan.
    pub sampled_of_total: Option<usize>,
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

    // 4b. Online-service / inference terms-of-use
    let service_terms_report = crate::service_terms::analyze(&manifest);
    if !service_terms_report.findings.is_empty() {
        let providers: Vec<&str> = service_terms_report
            .findings
            .iter()
            .map(|f| f.provider.as_str())
            .collect();
        findings.push(AuditFinding {
            level: "info".to_string(),
            code: "SERVICE_TERMS_REVIEW".to_string(),
            message: format!(
                "{} dependencies talk to a catalogued online service or inference provider ({}); their terms of use are separate from the SPDX license of the client library — run `lwoodz-cli service-terms` for details",
                service_terms_report.findings.len(),
                providers.join(", ")
            ),
            path: None,
        });
    }

    // 4c. Open-source vs open-standard differentiation
    let openness_report = crate::openness::analyze(&manifest);
    let rand_encumbered: Vec<&str> = openness_report
        .standard_encumbered()
        .map(|d| d.dependency.as_str())
        .collect();
    if !rand_encumbered.is_empty() {
        findings.push(AuditFinding {
            level: "warning".to_string(),
            code: "STANDARD_PATENT_RISK".to_string(),
            message: format!(
                "{} dependencies implement a RAND/FRAND-encumbered standard, not a royalty-free one ({}); a governing standards body does not by itself mean unencumbered — review the applicable patent pool before relying on royalty-free use, and run `lwoodz-cli openness` for citations",
                rand_encumbered.len(),
                rand_encumbered.join(", ")
            ),
            path: None,
        });
    }

    // 5. Header coverage (sample scan, capped)
    let header_coverage = if cfg.headers.enabled {
        sample_header_coverage(&cfg.repo_path, &cfg.headers.exclude).map(
            |(total, with, eligible_total)| HeaderCoverage {
                total_files: total,
                with_header: with,
                missing: total.saturating_sub(with),
                sampled_of_total: if eligible_total > total {
                    Some(eligible_total)
                } else {
                    None
                },
            },
        )
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
        service_terms: ServiceTermsSummary {
            matched: service_terms_report.findings.len(),
            providers: service_terms_report
                .findings
                .iter()
                .map(|f| f.provider.clone())
                .collect(),
        },
        openness: OpennessSummary {
            total_deps: openness_report.dependencies.len(),
            osi_approved: openness_report
                .dependencies
                .iter()
                .filter(|d| {
                    d.openness.open_source == crate::openness::OpenSourceStatus::OsiApproved
                })
                .count(),
            open_standard: openness_report
                .dependencies
                .iter()
                .filter(|d| d.openness.open_standard.is_some())
                .count(),
            rand_encumbered: rand_encumbered.len(),
        },
        findings,
        passed,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Directories excluded from the header-coverage sample independent of
/// whatever `headers.exclude` a project's own `.lwoodz` config sets —
/// mirrors elci-group/chakra's `walker::DEFAULT_EXCLUDES`. Applied
/// unconditionally so a project's own config can only add exclusions on
/// top of this baseline, never fall below it.
const BASELINE_EXCLUDES: [&str; 12] = [
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".cache",
    "vendor",
    ".mypy_cache",
];

/// Returns `(sampled, with_header, eligible_total)`: `sampled` is the
/// number of files actually read (capped), `eligible_total` is the full
/// count of matching files found by the walk before the cap was applied.
fn sample_header_coverage(repo_root: &Path, exclude: &[String]) -> Option<(usize, usize, usize)> {
    use ignore::overrides::OverrideBuilder;
    use ignore::WalkBuilder;

    const SAMPLE_CAP: usize = 500;

    let mut walker = WalkBuilder::new(repo_root);
    walker.hidden(false).git_ignore(true).git_exclude(true);

    let mut overrides = OverrideBuilder::new(repo_root);
    for dir in BASELINE_EXCLUDES {
        let _ = overrides.add(&format!("!**/{dir}/"));
    }
    for pat in exclude {
        let _ = overrides.add(&format!("!{pat}"));
    }
    if let Ok(built) = overrides.build() {
        walker.overrides(built);
    }

    let exts = ["rs", "go", "py", "js", "ts"];
    let mut candidates: Vec<std::path::PathBuf> = walker
        .build()
        .flatten()
        .map(|entry| entry.into_path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            exts.contains(&ext)
        })
        .collect();
    // Deterministic ordering: the cap below must apply after a stable
    // sort, not raw filesystem walk order, so repeated runs against an
    // unchanged repo sample the same files.
    candidates.sort();

    let eligible_total = candidates.len();
    if eligible_total == 0 {
        return None;
    }

    let mut with = 0usize;
    let sampled: Vec<_> = candidates.into_iter().take(SAMPLE_CAP).collect();
    let total = sampled.len();
    for p in &sampled {
        if let Ok(content) = std::fs::read_to_string(p) {
            let lower = content.to_lowercase();
            if lower.contains("copyright") || lower.contains("spdx-license-identifier") {
                with += 1;
            }
        }
    }
    Some((total, with, eligible_total))
}

#[cfg(test)]
mod header_coverage_tests {
    use super::sample_header_coverage;
    use std::fs;
    use std::process::Command;

    /// A custom-named vendor directory, outside the hardcoded
    /// `BASELINE_EXCLUDES` list, should still be excluded via the
    /// repository's own `.gitignore` now that the sampler is
    /// gitignore-aware.
    #[test]
    fn excludes_gitignored_custom_vendor_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .expect("git init");

        fs::write(root.join(".gitignore"), "legacy_thirdparty/\n").unwrap();

        fs::write(
            root.join("main.rs"),
            "// Copyright 2026 sal\nfn main() {}\n",
        )
        .unwrap();

        let vendor_dir = root.join("legacy_thirdparty");
        fs::create_dir(&vendor_dir).unwrap();
        fs::write(vendor_dir.join("bundled.rs"), "fn bundled() {}\n").unwrap();

        let (total, with, eligible_total) = sample_header_coverage(root, &[]).unwrap();

        assert_eq!(total, 1, "gitignored vendor file must not enter the sample");
        assert_eq!(with, 1);
        assert_eq!(eligible_total, 1);
    }

    /// Two runs against the same unchanged repository must sample the same
    /// files in the same order — the cap applies after a deterministic
    /// sort, not raw filesystem walk order.
    #[test]
    fn sampling_is_deterministic_across_runs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["z.rs", "a.rs", "m.rs"] {
            fs::write(root.join(name), "fn f() {}\n").unwrap();
        }

        let first = sample_header_coverage(root, &[]).unwrap();
        let second = sample_header_coverage(root, &[]).unwrap();
        assert_eq!(first, second);
    }
}
