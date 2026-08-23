// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use crate::config::Config;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAction {
    Written,
    Skipped,
}

/// Result of generating legal artifacts.
///
/// The `*_path` fields are preserved for backward compatibility. New callers
/// should inspect `written` and `skipped` to know which files were actually
/// touched.
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

/// Write `content` to `path` only when allowed by `overwrite`.
///
/// Returns `FileAction::Written` when a write happened, `FileAction::Skipped`
/// when the file already exists and `overwrite` is `false`.
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

/// Render the project license body for preview/dry-run purposes.
pub fn preview_license_body(cfg: &Config) -> anyhow::Result<String> {
    let spdx = crate::license::spdx::normalize_spdx(&cfg.project.license);
    crate::license::templates::render_license(
        &spdx,
        &cfg.project.copyright_holder,
        cfg.project.copyright_year,
    )
}

/// Generate LICENSE, NOTICE, COPYRIGHT, THIRD_PARTY_NOTICES and an SPDX
/// manifest according to `cfg`.
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
    if action == FileAction::Written {
        tracing::info!(path = %license_path.display(), spdx = %spdx, "wrote LICENSE");
    } else {
        tracing::info!(path = %license_path.display(), "LICENSE exists and overwrite=false; skipping");
    }
    res.push(license_path, action);

    // NOTICE
    if !cfg.generate.notice_file.is_empty() {
        let p = repo.join(&cfg.generate.notice_file);
        let content = generate_notice(cfg);
        if content.trim().is_empty() {
            res.notice_path = Some(p.clone());
            res.push(p, FileAction::Skipped);
        } else {
            let action = write_if_allowed(&p, content, cfg.generate.overwrite)?;
            res.notice_path = Some(p.clone());
            res.push(p, action);
        }
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
    // Try to read git contributors from the target repo.
    if let Ok(output) = std::process::Command::new("git")
        .arg("log")
        .arg("--format=%an <%ae>")
        .current_dir(&cfg.repo_path)
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
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("SPDXRef-Package-{}", sanitized)
}

fn document_namespace(cfg: &Config) -> String {
    if let Some(url) = &cfg.project.project_url {
        let base = url.trim_end_matches('/');
        let name = cfg.project.project_name.as_deref().unwrap_or("project");
        return format!("{}/spdx/{}-{}", base, name, chrono::Utc::now().timestamp());
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

    for d in &report.dependencies {
        let dep_spdx = d
            .license
            .clone()
            .unwrap_or_else(|| "NOASSERTION".to_string());
        let spdx_id = spdx_ref_id(&d.name);
        packages.push(serde_json::json!({
            "name": d.name,
            "SPDXID": spdx_id,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": false,
            "licenseConcluded": dep_spdx,
            "licenseDeclared": dep_spdx,
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
    let normalized = crate::license::spdx::normalize_spdx(second_license);
    if crate::license::templates::find_template(&normalized).is_none() {
        anyhow::bail!(
            "unknown dual license '{}'. Known: {}",
            second_license,
            crate::license::templates::all_licenses()
                .iter()
                .map(|t| t.spdx.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let repo = &cfg.repo_path;
    let holder = &cfg.project.copyright_holder;
    let year = cfg.project.copyright_year;
    let primary = crate::license::templates::render_license(
        &crate::license::spdx::normalize_spdx(&cfg.project.license),
        holder,
        year,
    )?;
    let secondary = crate::license::templates::render_license(&normalized, holder, year)?;
    let dest = repo.join(format!("LICENSE.{}", normalized));

    if dest.exists() && !cfg.generate.overwrite {
        anyhow::bail!(
            "dual-license file {} already exists and overwrite=false",
            dest.display()
        );
    }

    let combined = format!("DUAL LICENSE\n===========\nThis project is dual-licensed under {} and {}.\nYou may choose either license.\n\n--- {} ---\n\n{}\n\n--- {} ---\n\n{}\n",
        cfg.project.license, second_license, cfg.project.license, primary, second_license, secondary);
    write_if_allowed(&dest, combined, true)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::io::Write;

    fn test_cfg(repo: &Path) -> Config {
        Config {
            repo_path: repo.to_path_buf(),
            ..Config::default()
        }
    }

    #[test]
    fn mit_generation_substitutes_holder_and_year() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.project.copyright_holder = "Acme Inc".to_string();
        cfg.project.copyright_year = 2024;

        let res = generate_license_file(&cfg).unwrap();
        assert_eq!(res.spdx, "MIT");
        let body = std::fs::read_to_string(&res.license_path).unwrap();
        assert!(body.contains("Acme Inc"));
        assert!(body.contains("2024"));
        assert!(res.written.contains(&res.license_path));
        assert!(res.notice_path.is_some());
        assert!(res.copyright_path.is_some());
        assert!(res.attribution_path.is_some());
        assert!(res.manifest_path.is_some());
    }

    #[test]
    fn unknown_license_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.project.license = "WTFPL".to_string();

        let err = generate_license_file(&cfg).unwrap_err().to_string();
        assert!(err.contains("unknown license"));
        assert!(err.contains("WTFPL"));
    }

    #[test]
    fn overwrite_false_skips_existing_license() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.generate.overwrite = false;
        let license_path = dir.path().join("LICENSE");
        std::fs::write(&license_path, "existing").unwrap();

        let res = generate_license_file(&cfg).unwrap();
        assert!(res.skipped.contains(&license_path));
        assert_eq!(std::fs::read_to_string(&license_path).unwrap(), "existing");
    }

    #[test]
    fn overwrite_true_replaces_existing_license() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.generate.overwrite = true;
        let license_path = dir.path().join("LICENSE");
        std::fs::write(&license_path, "existing").unwrap();

        let res = generate_license_file(&cfg).unwrap();
        assert!(res.written.contains(&license_path));
        let body = std::fs::read_to_string(&license_path).unwrap();
        assert!(body.contains("MIT License"));
    }

    #[test]
    fn empty_output_paths_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.generate.notice_file = String::new();
        cfg.generate.copyright_file = String::new();
        cfg.generate.attribution_file = String::new();
        cfg.spdx.produce_manifest = false;

        let res = generate_license_file(&cfg).unwrap();
        assert!(res.notice_path.is_none());
        assert!(res.copyright_path.is_none());
        assert!(res.attribution_path.is_none());
        assert!(res.manifest_path.is_none());
        assert_eq!(res.written.len(), 1);
    }

    #[test]
    fn dual_variant_requires_known_license() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        let err = generate_dual_variant(&cfg, "WTFPL")
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown dual license"));
    }

    #[test]
    fn dual_variant_creates_combined_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.project.license = "MIT".to_string();
        cfg.project.copyright_holder = "Acme".to_string();
        cfg.project.copyright_year = 2024;

        let path = generate_dual_variant(&cfg, "Apache-2.0").unwrap();
        assert!(path.exists());
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("DUAL LICENSE"));
        assert!(body.contains("MIT"));
        assert!(body.contains("Apache-2.0"));
    }

    #[test]
    fn spdx_manifest_has_required_fields() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        let manifest = generate_spdx_manifest(&cfg);
        assert_eq!(manifest["spdxVersion"], "SPDX-2.3");
        assert_eq!(manifest["dataLicense"], "CC0-1.0");
        assert_eq!(manifest["SPDXID"], "SPDXRef-DOCUMENT");
        assert!(manifest["documentNamespace"]
            .as_str()
            .unwrap()
            .starts_with("https://"));
        let packages = manifest["packages"].as_array().unwrap();
        assert!(!packages.is_empty());
        assert_eq!(packages[0]["SPDXID"], "SPDXRef-Package-Project");
        let relationships = manifest["relationships"].as_array().unwrap();
        assert!(!relationships.is_empty());
        assert_eq!(relationships[0]["relationshipType"], "DESCRIBES");
    }

    #[test]
    fn write_if_allowed_respects_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("file.txt");
        assert_eq!(
            write_if_allowed(&p, "a", false).unwrap(),
            FileAction::Written
        );
        assert_eq!(
            write_if_allowed(&p, "b", false).unwrap(),
            FileAction::Skipped
        );
        assert_eq!(
            write_if_allowed(&p, "c", true).unwrap(),
            FileAction::Written
        );
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "c");
    }

    #[test]
    fn spdx_ref_id_sanitizes_names() {
        assert_eq!(spdx_ref_id("serde_json"), "SPDXRef-Package-serde-json");
        assert_eq!(spdx_ref_id("tokio-macros"), "SPDXRef-Package-tokio-macros");
        assert_eq!(spdx_ref_id("my.lib"), "SPDXRef-Package-my.lib");
    }

    #[test]
    fn document_namespace_uses_project_url() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_cfg(dir.path());
        cfg.project.project_url = Some("https://example.org/repo".to_string());
        let ns = document_namespace(&cfg);
        assert!(ns.starts_with("https://example.org/repo/spdx/project-"));
    }

    #[test]
    fn load_from_inline_toml_with_project_url() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
            [project]
            license = "Apache-2.0"
            copyright_holder = "Acme Inc"
            copyright_year = 2024
            project_url = "https://example.org/repo"
        "#
        )
        .unwrap();
        let cfg = crate::config::load_from(Some(f.path())).unwrap();
        assert_eq!(
            cfg.project.project_url.as_deref(),
            Some("https://example.org/repo")
        );
    }
}
