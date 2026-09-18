// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
#![allow(unused_variables)] // Legacy tracing field bindings are stringified by telemetry.

use crate::config::Config;
use crate::telemetry as tracing;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAction {
    Written,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct GenerateResult {
    pub license_path: PathBuf,
    pub notice_path: Option<PathBuf>,
    pub copyright_path: Option<PathBuf>,
    pub attribution_path: Option<PathBuf>,
    pub manifest_path: Option<PathBuf>,
    pub spdx: String,
    pub written: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

impl GenerateResult {
    fn empty(repo: &Path, spdx: String) -> Self {
        Self {
            license_path: repo.join("LICENSE"),
            notice_path: None,
            copyright_path: None,
            attribution_path: None,
            manifest_path: None,
            spdx,
            written: Vec::new(),
            skipped: Vec::new(),
        }
    }

    fn push(&mut self, path: PathBuf, action: FileAction) {
        match action {
            FileAction::Written => self.written.push(path),
            FileAction::Skipped => self.skipped.push(path),
        }
    }
}

pub fn write_if_allowed(
    path: &Path,
    content: impl AsRef<[u8]>,
    overwrite: bool,
) -> anyhow::Result<FileAction> {
    if path.exists() && !overwrite {
        return Ok(FileAction::Skipped);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content.as_ref())?;
    Ok(FileAction::Written)
}

pub fn preview_license_body(cfg: &Config) -> anyhow::Result<String> {
    let spdx = crate::license::spdx::normalize_spdx(&cfg.project.license);
    crate::license::templates::render_license(
        &spdx,
        &cfg.project.copyright_holder,
        cfg.project.copyright_year,
    )
}

pub fn generate_license_file(cfg: &Config) -> anyhow::Result<GenerateResult> {
    let holder = &cfg.project.copyright_holder;
    let year = cfg.project.copyright_year;
    let spdx = crate::license::spdx::normalize_spdx(&cfg.project.license);

    if crate::license::templates::find_template(&spdx).is_none() {
        anyhow::bail!(
            "unknown license '{}'. Known: {}",
            spdx,
            crate::license::templates::all_licenses()
                .iter()
                .map(|t| t.spdx.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let body = crate::license::templates::render_license(&spdx, holder, year)?;
    let repo = &cfg.repo_path;
    let license_path = repo.join(&cfg.generate.license_file);

    let mut res = GenerateResult::empty(repo, spdx.clone());
    res.license_path = license_path.clone();

    let action = write_if_allowed(&license_path, &body, cfg.generate.overwrite)?;
    if action == FileAction::Skipped {
        tracing::info!(path = %license_path.display(), "LICENSE exists and overwrite=false; skipping");
    } else {
        tracing::info!(path = %license_path.display(), spdx = %spdx, "wrote LICENSE");
    }
    res.push(license_path, action);

    // NOTICE
    if !cfg.generate.notice_file.is_empty() {
        let p = repo.join(&cfg.generate.notice_file);
        let content = generate_notice(cfg);
        let action = if content.trim().is_empty() {
            FileAction::Skipped
        } else {
            write_if_allowed(&p, content, cfg.generate.overwrite)?
        };
        res.notice_path = Some(p.clone());
        res.push(p, action);
    }

    // COPYRIGHT
    if !cfg.generate.copyright_file.is_empty() {
        let p = repo.join(&cfg.generate.copyright_file);
        let content = generate_copyright(cfg);
        let action = write_if_allowed(&p, content, cfg.generate.overwrite)?;
        res.copyright_path = Some(p.clone());
        res.push(p, action);
    }

    // Attribution / THIRD_PARTY_NOTICES
    if !cfg.generate.attribution_file.is_empty() {
        let p = repo.join(&cfg.generate.attribution_file);
        let report = crate::manifest::scan(repo);
        let content = generate_attribution(cfg, &report);
        let action = write_if_allowed(&p, content, cfg.generate.overwrite)?;
        res.attribution_path = Some(p.clone());
        res.push(p, action);
    }

    // SPDX manifest
    if cfg.spdx.produce_manifest {
        let p = repo.join(&cfg.spdx.manifest_path);
        let manifest = generate_spdx_manifest(cfg);
        let action = write_if_allowed(
            &p,
            serde_json::to_string_pretty(&manifest)?,
            cfg.generate.overwrite,
        )?;
        res.manifest_path = Some(p.clone());
        res.push(p, action);
    }

    // DCO / contributor assignments
    ensure_contributor_file(cfg)?;

    Ok(res)
}

fn generate_notice(cfg: &Config) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "NOTICE for {}\n",
        cfg.project
            .project_name
            .as_deref()
            .unwrap_or("this project")
    ));
    s.push_str(&format!(
        "Copyright (c) {} {}\n",
        cfg.project.copyright_year, cfg.project.copyright_holder
    ));
    s.push_str(&format!(
        "SPDX-License-Identifier: {}\n\n",
        crate::license::spdx::normalize_spdx(&cfg.project.license)
    ));
    s.push_str("This product includes software developed by the project contributors.\n");
    if let Some(dual) = &cfg.project.dual_license {
        s.push_str(&format!(
            "\nDual-licensed under {} and {}.\n",
            cfg.project.license, dual
        ));
    }
    if let Some(commercial) = &cfg.project.commercial_license {
        s.push_str(&format!(
            "\nCommercial licensing available: {}.\n",
            commercial
        ));
    }
    s
}

fn generate_copyright(cfg: &Config) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "Copyright (c) {} {}\n",
        cfg.project.copyright_year, cfg.project.copyright_holder
    ));
    if let Some(email) = &cfg.project.copyright_holder_email {
        s.push_str(&format!("Contact: {}\n", email));
    }
    s.push_str(&format!(
        "License: {}\n",
        crate::license::spdx::normalize_spdx(&cfg.project.license)
    ));
    // Try to read git contributors
    if let Ok(output) = std::process::Command::new("git")
        .arg("log")
        .arg("--format=%an <%ae>")
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut contributors: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for line in text.lines() {
                let l = line.trim();
                if !l.is_empty() {
                    contributors.insert(l.to_string());
                }
            }
            if !contributors.is_empty() {
                s.push_str("\nContributors (from git history):\n");
                for c in contributors {
                    s.push_str(&format!("  - {}\n", c));
                }
            }
        }
    }
    s
}

fn generate_attribution(cfg: &Config, report: &crate::manifest::ManifestReport) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# Third-Party Notices for {}\n\n",
        cfg.project
            .project_name
            .as_deref()
            .unwrap_or("this project")
    ));
    s.push_str(&format!(
        "Project license: {}\n\n",
        crate::license::spdx::normalize_spdx(&cfg.project.license)
    ));
    if report.dependencies.is_empty() {
        s.push_str("No dependencies detected.\n");
        return s;
    }
    s.push_str("| Dependency | Version | License | Source |\n");
    s.push_str("|------------|---------|---------|--------|\n");
    for d in &report.dependencies {
        s.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            d.name,
            d.version.as_deref().unwrap_or("-"),
            d.license.as_deref().unwrap_or("NOASSERTION"),
            d.source
        ));
    }
    s.push_str("\n---\nGenerated by lwoodz. Review licenses for compliance before distribution.\n");
    s
}

fn spdx_ref_id(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '-' || character == '.' {
                character
            } else {
                '-'
            }
        })
        .collect();
    format!("SPDXRef-Package-{sanitized}")
}

fn document_namespace(cfg: &Config) -> String {
    if let Some(url) = &cfg.project.project_url {
        let base = url.trim_end_matches('/');
        let name = cfg.project.project_name.as_deref().unwrap_or("project");
        return format!("{base}/spdx/{name}-{}", chrono::Utc::now().timestamp());
    }
    format!(
        "https://lwoodz.dev/spdx/{}-{}",
        cfg.project.project_name.as_deref().unwrap_or("project"),
        chrono::Utc::now().timestamp()
    )
}

fn generate_spdx_manifest(cfg: &Config) -> serde_json::Value {
    let report = crate::manifest::scan(&cfg.repo_path);
    let project_name = cfg
        .project
        .project_name
        .clone()
        .unwrap_or_else(|| "project".to_string());
    let project_spdx = crate::license::spdx::normalize_spdx(&cfg.project.license);
    let project_copyright = format!(
        "Copyright (c) {} {}",
        cfg.project.copyright_year, cfg.project.copyright_holder
    );
    let mut packages = vec![serde_json::json!({
        "name": project_name,
        "SPDXID": "SPDXRef-Package-Project",
        "downloadLocation": cfg.project.project_url.as_deref().unwrap_or("NOASSERTION"),
        "filesAnalyzed": false,
        "licenseConcluded": project_spdx,
        "licenseDeclared": project_spdx,
        "copyrightText": project_copyright,
    })];
    let mut relationships = vec![serde_json::json!({
        "spdxElementId": "SPDXRef-DOCUMENT",
        "relationshipType": "DESCRIBES",
        "relatedSpdxElement": "SPDXRef-Package-Project"
    })];
    for dependency in &report.dependencies {
        let dependency_spdx = dependency
            .license
            .clone()
            .unwrap_or_else(|| "NOASSERTION".to_string());
        let spdx_id = spdx_ref_id(&dependency.name);
        packages.push(serde_json::json!({
            "name": dependency.name,
            "SPDXID": spdx_id,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": false,
            "licenseConcluded": dependency_spdx,
            "licenseDeclared": dependency_spdx,
            "copyrightText": "NOASSERTION",
        }));
        relationships.push(serde_json::json!({
            "spdxElementId": "SPDXRef-Package-Project",
            "relationshipType": "DEPENDS_ON",
            "relatedSpdxElement": spdx_id,
        }));
    }

    serde_json::json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": project_name,
        "documentNamespace": document_namespace(cfg),
        "creationInfo": {
            "created": chrono::Utc::now().to_rfc3339(),
            "creators": [
                format!("Tool: lwoodz-{}", env!("CARGO_PKG_VERSION")),
                format!("Person: {}", cfg.project.copyright_holder)
            ]
        },
        "packages": packages,
        "relationships": relationships,
        "generatedBy": format!("lwoodz {}", env!("CARGO_PKG_VERSION")),
    })
}

fn ensure_contributor_file(cfg: &Config) -> anyhow::Result<()> {
    // Maintain .lwoodz/contributors.json and DCO sign-off hint
    let dir = cfg.repo_path.join(".lwoodz");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("contributors.json");
    if !path.exists() {
        let initial = serde_json::json!({
            "holders": [cfg.project.copyright_holder],
            "dco": true,
            "signOff": format!("Signed-off-by: {} <{}>", cfg.project.copyright_holder, cfg.project.copyright_holder_email.as_deref().unwrap_or("noreply@example.com")),
            "updated": chrono::Utc::now().to_rfc3339(),
        });
        std::fs::write(&path, serde_json::to_string_pretty(&initial)?)?;
    }
    Ok(())
}

pub fn generate_dual_variant(
    cfg: &Config,
    second_license: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let repo = &cfg.repo_path;
    let holder = &cfg.project.copyright_holder;
    let year = cfg.project.copyright_year;
    let primary = crate::license::templates::render_license(
        &crate::license::spdx::normalize_spdx(&cfg.project.license),
        holder,
        year,
    )?;
    let secondary = crate::license::templates::render_license(
        &crate::license::spdx::normalize_spdx(second_license),
        holder,
        year,
    )?;
    let dest = repo.join(format!(
        "LICENSE.{}",
        crate::license::spdx::normalize_spdx(second_license)
    ));
    let combined = format!("DUAL LICENSE\n===========\nThis project is dual-licensed under {} and {}.\nYou may choose either license.\n\n--- {} ---\n\n{}\n\n--- {} ---\n\n{}\n",
        cfg.project.license, second_license, cfg.project.license, primary, second_license, secondary);
    std::fs::write(&dest, combined)?;
    Ok(dest)
}
