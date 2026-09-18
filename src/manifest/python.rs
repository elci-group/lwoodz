// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use super::Dependency;
use std::path::{Path, PathBuf};

pub fn scan(repo_root: &Path) -> Option<Vec<Dependency>> {
    let mut deps = Vec::new();
    let mut found = false;

    // pyproject.toml (PEP 621)
    let pyproject = repo_root.join("pyproject.toml");
    if pyproject.exists() {
        found = true;
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if let Ok(val) = content.parse::<toml::Value>() {
                if let Some(project) = val.get("project").and_then(|v| v.as_table()) {
                    if let Some(arr) = project.get("dependencies").and_then(|v| v.as_array()) {
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                let name = parse_requirement_name(s);
                                if !name.is_empty() {
                                    deps.push(Dependency {
                                        name,
                                        version: Some(s.to_string()),
                                        license: None,
                                        source: "python".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
                // poetry dependencies
                if let Some(tool) = val.get("tool").and_then(|v| v.as_table()) {
                    if let Some(poetry) = tool.get("poetry").and_then(|v| v.as_table()) {
                        if let Some(tbl) = poetry.get("dependencies").and_then(|v| v.as_table()) {
                            for (k, v) in tbl {
                                if k == "python" {
                                    continue;
                                }
                                deps.push(Dependency {
                                    name: k.clone(),
                                    version: v.as_str().map(|s| s.to_string()),
                                    license: None,
                                    source: "python".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // requirements.txt
    for fname in ["requirements.txt", "requirements-dev.txt"] {
        let p = repo_root.join(fname);
        if p.exists() {
            found = true;
            if let Ok(content) = std::fs::read_to_string(&p) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                        continue;
                    }
                    let name = parse_requirement_name(line);
                    if !name.is_empty() {
                        deps.push(Dependency {
                            name,
                            version: Some(line.to_string()),
                            license: None,
                            source: "python".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Enrich from an installed environment, if one is discoverable: read the
    // real `License:` / `Classifier:` fields from each package's own
    // *.dist-info/METADATA — the same "ask the artifact, don't guess"
    // approach used for Cargo (`cargo metadata`) and npm (lockfile/
    // package.json).
    if let Some(site_packages) = find_site_packages(repo_root) {
        for d in &mut deps {
            d.license = resolve_from_dist_info(&site_packages, &d.name);
        }
    }

    if found {
        Some(deps)
    } else {
        None
    }
}

fn parse_requirement_name(spec: &str) -> String {
    spec.split([' ', '>', '<', '=', '~', '!', '['])
        .next()
        .unwrap_or(spec)
        .trim()
        .to_string()
}

fn find_site_packages(repo_root: &Path) -> Option<PathBuf> {
    for venv_name in [".venv", "venv", "env", ".env"] {
        let venv = repo_root.join(venv_name);
        if !venv.is_dir() {
            continue;
        }
        // Unix: <venv>/lib/python3.X/site-packages
        if let Ok(entries) = std::fs::read_dir(venv.join("lib")) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("site-packages");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
        // Windows: <venv>/Lib/site-packages
        let win = venv.join("Lib").join("site-packages");
        if win.is_dir() {
            return Some(win);
        }
    }
    None
}

/// dist-info directories normalize the project name (PEP 503 / PEP 427):
/// runs of `-_.` collapse to a single `_`. Try the couple of variants that
/// cover the vast majority of real packages without a full PEP 503 pass.
fn resolve_from_dist_info(site_packages: &Path, name: &str) -> Option<String> {
    let normalized = name.replace(['-', '.'], "_");
    let entries = std::fs::read_dir(site_packages).ok()?;
    let prefix_lower = format!("{}-", normalized.to_lowercase());

    for entry in entries.flatten() {
        let fname = entry.file_name();
        let fname = fname.to_string_lossy();
        if !fname.ends_with(".dist-info") {
            continue;
        }
        if !fname.to_lowercase().starts_with(&prefix_lower) {
            continue;
        }
        let metadata_path = entry.path().join("METADATA");
        if let Ok(text) = std::fs::read_to_string(&metadata_path) {
            return parse_metadata_license(&text);
        }
    }
    None
}

fn parse_metadata_license(text: &str) -> Option<String> {
    let mut explicit_license: Option<String> = None;
    let mut classifier_spdx: Option<String> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("License:") {
            let v = rest.trim();
            if !v.is_empty() && v != "UNKNOWN" {
                explicit_license = Some(v.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("Classifier: License :: OSI Approved") {
            classifier_spdx = classifier_to_spdx(rest.trim().trim_start_matches("::").trim());
        }
    }

    // A short, unambiguous `License:` field (e.g. "MIT") is more precise
    // than the classifier bucket; a long free-text blob is not, so prefer
    // the classifier mapping in that case.
    match explicit_license {
        Some(l) if l.len() <= 40 && !l.contains('\n') => Some(l),
        _ => classifier_spdx.or(explicit_license),
    }
}

fn classifier_to_spdx(license_name: &str) -> Option<String> {
    // These are PyPI's own fixed classifier strings (Trove classifiers),
    // matched verbatim — not derived by stripping "License" off the input.
    let spdx = match license_name {
        "MIT License" => "MIT",
        "Apache Software License" | "Apache License 2.0" => "Apache-2.0",
        "BSD License" => "BSD-3-Clause",
        "GNU General Public License v2 (GPLv2)" => "GPL-2.0-only",
        "GNU General Public License v3 (GPLv3)" => "GPL-3.0-only",
        "GNU Lesser General Public License v3 (LGPLv3)" => "LGPL-3.0-only",
        "GNU Lesser General Public License v2 (LGPLv2)" => "LGPL-2.0-only",
        "Mozilla Public License 2.0 (MPL 2.0)" => "MPL-2.0",
        "ISC License (ISCL)" => "ISC",
        "The Unlicense (Unlicense)" => "Unlicense",
        _ => return None,
    };
    Some(spdx.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_missing_returns_none() {
        assert!(scan(Path::new("/nonexistent_xyz_123")).is_none());
    }

    #[test]
    fn resolves_license_from_dist_info_metadata() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "requests>=2.0\n").unwrap();

        let dist_info = dir
            .path()
            .join(".venv/lib/python3.12/site-packages/requests-2.31.0.dist-info");
        std::fs::create_dir_all(&dist_info).unwrap();
        std::fs::write(
            dist_info.join("METADATA"),
            "Metadata-Version: 2.1\nName: requests\nVersion: 2.31.0\nLicense: Apache-2.0\n",
        )
        .unwrap();

        let deps = scan(dir.path()).unwrap();
        assert_eq!(deps[0].license.as_deref(), Some("Apache-2.0"));
    }

    #[test]
    fn falls_back_to_classifier_when_license_field_is_freeform() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "django>=4.0\n").unwrap();

        let dist_info = dir
            .path()
            .join(".venv/lib/python3.12/site-packages/django-4.2.dist-info");
        std::fs::create_dir_all(&dist_info).unwrap();
        std::fs::write(
            dist_info.join("METADATA"),
            "Metadata-Version: 2.1\nName: django\nClassifier: License :: OSI Approved :: BSD License\n",
        )
        .unwrap();

        let deps = scan(dir.path()).unwrap();
        assert_eq!(deps[0].license.as_deref(), Some("BSD-3-Clause"));
    }
}
