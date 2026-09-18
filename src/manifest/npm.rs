// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use super::Dependency;
use std::path::Path;

pub fn scan(repo_root: &Path) -> Option<Vec<Dependency>> {
    let pkg = repo_root.join("package.json");
    if !pkg.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&pkg).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    let mut deps = Vec::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(obj) = val.get(section).and_then(|v| v.as_object()) {
            for (name, ver) in obj {
                deps.push(Dependency {
                    name: name.clone(),
                    version: ver.as_str().map(|s| s.to_string()),
                    license: None,
                    source: "npm".to_string(),
                });
            }
        }
    }

    // npm (v2/v3 lockfiles) records each resolved package's own declared
    // `license` under `packages["node_modules/<name>"]` — read straight from
    // the lockfile npm already generated, rather than guessing. This also
    // picks up transitive dependencies that never appear in package.json.
    let lock = repo_root.join("package-lock.json");
    if lock.exists() {
        if let Ok(lc) = std::fs::read_to_string(&lock) {
            if let Ok(lv) = serde_json::from_str::<serde_json::Value>(&lc) {
                if let Some(pkgs) = lv.get("packages").and_then(|v| v.as_object()) {
                    for (p, info) in pkgs {
                        if p.is_empty() || p == "node_modules" {
                            continue; // root package entry
                        }
                        let Some(name) = package_name_from_lock_key(p) else {
                            continue;
                        };
                        let license = info
                            .get("license")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let version = info
                            .get("version")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        match deps.iter_mut().find(|d| d.name == name) {
                            Some(d) => {
                                if d.license.is_none() {
                                    d.license = license;
                                }
                                if d.version.is_none() {
                                    d.version = version;
                                }
                            }
                            None => deps.push(Dependency {
                                name,
                                version,
                                license,
                                source: "npm".to_string(),
                            }),
                        }
                    }
                }
            }
        }
    }

    // For anything still unresolved, fall back to the installed package's
    // own package.json (or a LICENSE file next to it) under node_modules —
    // still offline, still the package's own declaration.
    let node_modules = repo_root.join("node_modules");
    if node_modules.is_dir() {
        for d in &mut deps {
            if d.license.is_some() {
                continue;
            }
            d.license = resolve_from_node_modules(&node_modules, &d.name);
        }
    }

    if deps.is_empty() {
        None
    } else {
        Some(deps)
    }
}

/// `package-lock.json` keys packages by path, e.g. `node_modules/lodash` or,
/// for scoped/nested packages, `node_modules/foo/node_modules/@scope/bar`.
/// The package name is always the last one or two segments after the final
/// `node_modules/`.
fn package_name_from_lock_key(key: &str) -> Option<String> {
    let last = key.rsplit("node_modules/").next()?;
    if last.is_empty() {
        return None;
    }
    Some(last.to_string())
}

fn resolve_from_node_modules(node_modules: &Path, name: &str) -> Option<String> {
    let dir = node_modules.join(name);
    let pkg_json = dir.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_json) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(lic) = val.get("license").and_then(|v| v.as_str()) {
                return Some(lic.to_string());
            }
            // Legacy `{ "license": { "type": "MIT" } }` form.
            if let Some(lic) = val
                .get("license")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
            {
                return Some(lic.to_string());
            }
        }
    }
    for fname in ["LICENSE", "LICENSE.md", "LICENSE.txt", "license"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(fname)) {
            if let Some(spdx) = crate::license::spdx::detect_spdx_from_text(&text) {
                return Some(spdx);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_missing_returns_none() {
        assert!(scan(Path::new("/nonexistent_xyz_123")).is_none());
    }

    #[test]
    fn resolves_license_from_lockfile_including_transitive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"left-pad": "^1.0.0"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package-lock.json"),
            r#"{
                "packages": {
                    "": {},
                    "node_modules/left-pad": {"version": "1.3.0", "license": "WTFPL"},
                    "node_modules/left-pad/node_modules/inner": {"version": "2.0.0", "license": "MIT"}
                }
            }"#,
        )
        .unwrap();
        let deps = scan(dir.path()).unwrap();
        let left_pad = deps.iter().find(|d| d.name == "left-pad").unwrap();
        assert_eq!(left_pad.license.as_deref(), Some("WTFPL"));
        // transitive dep, absent from package.json, still surfaced
        let inner = deps.iter().find(|d| d.name == "inner").unwrap();
        assert_eq!(inner.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn falls_back_to_node_modules_package_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"foo": "^1.0.0"}}"#,
        )
        .unwrap();
        let pkg_dir = dir.path().join("node_modules/foo");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("package.json"), r#"{"license": "ISC"}"#).unwrap();
        let deps = scan(dir.path()).unwrap();
        assert_eq!(deps[0].license.as_deref(), Some("ISC"));
    }
}
