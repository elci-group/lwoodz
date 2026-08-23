// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use super::Dependency;
use std::path::{Path, PathBuf};

pub fn scan(repo_root: &Path) -> Option<Vec<Dependency>> {
    let gomod = repo_root.join("go.mod");
    if !gomod.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&gomod).ok()?;
    let mut deps = parse_go_mod(&content);

    // Enrich from the local module cache, if discoverable: a downloaded
    // module's own LICENSE file is right there on disk. Same "ask the
    // artifact, don't guess" approach used for Cargo/npm/Python — just
    // reading a file instead of a manifest field, since Go modules don't
    // carry a machine-readable license field anywhere.
    if let Some(mod_cache) = module_cache_dir() {
        for d in &mut deps {
            let Some(version) = d.version.as_deref() else {
                continue;
            };
            d.license = resolve_from_module_cache(&mod_cache, &d.name, version);
        }
    }

    Some(deps)
}

pub fn parse_go_mod(content: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();
    let mut in_require = false;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t.starts_with("require (") {
            in_require = true;
            continue;
        }
        if t == "require" {
            continue;
        }
        if in_require && t == ")" {
            in_require = false;
            continue;
        }
        if let Some(rest) = t.strip_prefix("require ") {
            // single line require
            push_dep(&mut deps, rest.trim());
            continue;
        }
        if in_require && !t.starts_with("//") {
            push_dep(&mut deps, t);
        }
    }
    deps
}

fn push_dep(deps: &mut Vec<Dependency>, spec: &str) {
    // Strip a trailing "// indirect" (or any trailing comment) before
    // splitting into name/version.
    let spec = spec.split("//").next().unwrap_or(spec).trim();
    let mut parts = spec.split_whitespace();
    let (Some(name), Some(ver)) = (parts.next(), parts.next()) else {
        return;
    };
    deps.push(Dependency {
        name: name.to_string(),
        version: Some(ver.to_string()),
        license: None,
        source: "go".to_string(),
    });
}

/// `go env GOMODCACHE` is authoritative and respects any user override; fall
/// back to the conventional `$GOPATH/pkg/mod` (default `~/go/pkg/mod`) when
/// the `go` toolchain isn't on PATH.
fn module_cache_dir() -> Option<PathBuf> {
    if let Ok(out) = std::process::Command::new("go")
        .args(["env", "GOMODCACHE"])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    let gopath = std::env::var("GOPATH")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join("go")))?;
    Some(gopath.join("pkg").join("mod"))
}

/// Go module cache directories escape uppercase letters in the module path
/// as `!` + the lowercase letter (so a proxy running on a case-insensitive
/// filesystem can't collide `Foo` and `foo`). See `golang.org/x/mod/module`
/// `EscapePath`.
fn escape_module_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c.is_ascii_uppercase() {
            out.push('!');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn resolve_from_module_cache(mod_cache: &Path, module: &str, version: &str) -> Option<String> {
    let dir_name = format!("{}@{}", escape_module_path(module), version);
    // Module paths can contain '/', which is a real directory separator here.
    let dir = mod_cache.join(dir_name);
    for fname in [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "COPYING",
        "COPYING.md",
    ] {
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
    fn parses_single_line_and_block_requires_ignoring_indirect_comment() {
        let gomod = r#"
module example.com/foo

go 1.21

require github.com/single/dep v1.2.3

require (
    github.com/block/one v0.1.0
    github.com/block/two v2.0.0 // indirect
)
"#;
        let deps = parse_go_mod(gomod);
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "github.com/single/dep",
                "github.com/block/one",
                "github.com/block/two",
            ]
        );
        let two = deps
            .iter()
            .find(|d| d.name == "github.com/block/two")
            .unwrap();
        assert_eq!(two.version.as_deref(), Some("v2.0.0"));
    }

    #[test]
    fn escapes_uppercase_module_path_segments() {
        assert_eq!(
            escape_module_path("github.com/BurntSushi/toml"),
            "github.com/!burnt!sushi/toml"
        );
        assert_eq!(escape_module_path("golang.org/x/mod"), "golang.org/x/mod");
    }

    #[test]
    fn resolves_license_from_module_cache_license_file() {
        let dir = tempfile::tempdir().unwrap();
        let module_dir = dir.path().join("github.com/foo/bar@v1.0.0");
        std::fs::create_dir_all(&module_dir).unwrap();
        std::fs::write(
            module_dir.join("LICENSE"),
            "MIT License\nPermission is hereby granted, free of charge",
        )
        .unwrap();

        let license = resolve_from_module_cache(dir.path(), "github.com/foo/bar", "v1.0.0");
        assert_eq!(license.as_deref(), Some("MIT"));
    }

    #[test]
    fn missing_module_cache_entry_leaves_license_unresolved() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            resolve_from_module_cache(dir.path(), "github.com/never/downloaded", "v9.9.9")
                .is_none()
        );
    }
}
