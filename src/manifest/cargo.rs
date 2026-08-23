// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use super::Dependency;
use std::path::Path;
use std::process::Command;

/// Resolves Cargo dependencies and their SPDX licenses via `cargo metadata`.
///
/// `cargo metadata` is the authoritative source: it returns each resolved
/// package's own declared `license` (straight from that crate's Cargo.toml),
/// not a guess. We shell out to it rather than re-implementing a registry
/// client, and prefer `--offline` first since the crates are already present
/// in the local registry cache for any project that has been built.
pub fn scan(repo_root: &Path) -> Option<Vec<Dependency>> {
    let cargo_toml = repo_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }

    match cargo_metadata_deps(repo_root, true).or_else(|| cargo_metadata_deps(repo_root, false)) {
        Some(deps) => Some(deps),
        // cargo itself is unavailable, or metadata resolution failed (e.g. a
        // Cargo.lock referencing crates never fetched, offline, no cargo on
        // PATH). Fall back to a manifest-only scan so callers still see the
        // dependency graph — honestly reporting unresolved licenses rather
        // than silently returning nothing.
        None => manifest_only_deps(repo_root),
    }
}

fn cargo_metadata_deps(repo_root: &Path, offline: bool) -> Option<Vec<Dependency>> {
    let mut cmd = Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(repo_root.join("Cargo.toml"));
    if offline {
        cmd.arg("--offline");
    }

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let meta: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let packages = meta.get("packages")?.as_array()?;

    let workspace_members: std::collections::HashSet<&str> = meta
        .get("workspace_members")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut deps = Vec::new();
    for pkg in packages {
        let id = pkg.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if workspace_members.contains(id) {
            continue; // skip the scanned project's own package(s)
        }
        let name = pkg.get("name")?.as_str()?.to_string();
        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let license = resolve_package_license(pkg);
        // A path dependency outside the workspace is first-party code from
        // the audit's point of view, even though Cargo includes it in the
        // resolved package graph. Keep that distinction in the existing
        // source field so callers do not classify it as third-party.
        let source = if pkg.get("source").and_then(|v| v.as_str()).is_none() {
            "cargo-local"
        } else {
            "cargo"
        };
        deps.push(Dependency {
            name,
            version,
            license,
            source: source.to_string(),
        });
    }
    Some(deps)
}

/// Resolve a package's declared license without guessing from its repository.
/// Cargo normally supplies `license`; `license_file` is the package metadata
/// fallback for crates that publish their SPDX declaration in a file.
fn resolve_package_license(pkg: &serde_json::Value) -> Option<String> {
    pkg.get("license")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            let license_file = pkg.get("license_file").and_then(|v| v.as_str())?;
            let manifest_path = pkg.get("manifest_path").and_then(|v| v.as_str())?;
            let path = Path::new(manifest_path).parent()?.join(license_file);
            let text = std::fs::read_to_string(path).ok()?;
            crate::license::spdx::detect_spdx_from_text(&text)
        })
}

/// Last-resort fallback when `cargo metadata` can't run at all: parse
/// Cargo.toml/Cargo.lock directly for names and versions, with licenses left
/// unresolved rather than guessed.
fn manifest_only_deps(repo_root: &Path) -> Option<Vec<Dependency>> {
    let cargo_toml = repo_root.join("Cargo.toml");
    let lock = repo_root.join("Cargo.lock");
    let mut deps = Vec::new();

    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        if let Ok(val) = content.parse::<toml::Value>() {
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(tbl) = val.get(section).and_then(|v| v.as_table()) {
                    for (name, v) in tbl {
                        deps.push(Dependency {
                            name: name.clone(),
                            version: extract_version(v),
                            license: None,
                            source: "cargo".to_string(),
                        });
                    }
                }
                if let Some(ws) = val.get("workspace").and_then(|v| v.as_table()) {
                    if let Some(tbl) = ws.get(section).and_then(|v| v.as_table()) {
                        for (name, v) in tbl {
                            if deps.iter().any(|d: &Dependency| &d.name == name) {
                                continue;
                            }
                            deps.push(Dependency {
                                name: name.clone(),
                                version: extract_version(v),
                                license: None,
                                source: "cargo".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    if lock.exists() {
        if let Ok(content) = std::fs::read_to_string(&lock) {
            if let Ok(val) = content.parse::<toml::Value>() {
                if let Some(pkgs) = val.get("package").and_then(|v| v.as_array()) {
                    for pkg in pkgs {
                        if let Some(name) = pkg.get("name").and_then(|v| v.as_str()) {
                            if !deps.iter().any(|d| d.name == name) {
                                deps.push(Dependency {
                                    name: name.to_string(),
                                    version: pkg
                                        .get("version")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    license: None,
                                    source: "cargo".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if deps.is_empty() {
        None
    } else {
        Some(deps)
    }
}

fn extract_version(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_missing_returns_none() {
        assert!(scan(Path::new("/nonexistent_xyz_123")).is_none());
    }

    #[test]
    fn scan_self_resolves_real_licenses() {
        // lwoodz's own repo root — exercises the real `cargo metadata` path
        // and checks that well-known deps resolve to their actual license,
        // not a hardcoded guess.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let deps = scan(root).expect("cargo metadata should resolve this workspace");
        let serde = deps.iter().find(|d| d.name == "serde");
        assert!(
            serde.is_some(),
            "serde should appear in the dependency graph"
        );
        assert_eq!(
            serde.unwrap().license.as_deref(),
            Some("MIT OR Apache-2.0"),
            "license should come from serde's own Cargo.toml via `cargo metadata`, not a guess"
        );
    }

    #[test]
    fn manifest_only_fallback_leaves_license_unresolved() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n[dependencies]\nfoo = \"1\"\n",
        )
        .unwrap();
        let deps = manifest_only_deps(dir.path()).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "foo");
        assert!(deps[0].license.is_none());
    }

    #[test]
    fn package_license_file_is_used_when_license_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nlicense-file = \"LICENSE\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("LICENSE"),
            "MIT License\nPermission is hereby granted, free of charge, to any person obtaining a copy.\n",
        )
        .unwrap();

        let package = serde_json::json!({
            "license": null,
            "license_file": "LICENSE",
            "manifest_path": manifest,
        });
        assert_eq!(resolve_package_license(&package).as_deref(), Some("MIT"));
    }
}
