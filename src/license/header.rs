// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
#![allow(unused_variables)] // Legacy tracing field bindings are stringified by telemetry.

use crate::telemetry as tracing;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct HeaderResult {
    pub path: PathBuf,
    pub action: HeaderAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderAction {
    Inserted,
    Updated,
    AlreadyPresent,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct HeaderConfig {
    pub holder: String,
    pub year: i32,
    pub spdx: String,
    pub insert_spdx: bool,
}

fn header_text(cfg: &HeaderConfig, path: &Path) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_hash = matches!(ext, "py" | "sh" | "rb" | "toml" | "yaml" | "yml")
        || filename == "Makefile"
        || filename == "Dockerfile";
    let spdx_line = if cfg.insert_spdx {
        format!("SPDX-License-Identifier: {}", cfg.spdx)
    } else {
        String::new()
    };

    if is_hash {
        let mut s = format!("# Copyright (c) {} {}\n", cfg.year, cfg.holder);
        if !spdx_line.is_empty() {
            s.push_str(&format!("# {}\n", spdx_line));
        }
        s
    } else if ext == "html" || ext == "xml" {
        let mut s = format!("<!-- Copyright (c) {} {} -->\n", cfg.year, cfg.holder);
        if !spdx_line.is_empty() {
            s.push_str(&format!("<!-- {} -->\n", spdx_line));
        }
        s
    } else {
        let mut s = format!("// Copyright (c) {} {}\n", cfg.year, cfg.holder);
        if !spdx_line.is_empty() {
            s.push_str(&format!("// {}\n", spdx_line));
        }
        s
    }
}

fn has_copyright(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("copyright") || lower.contains("spdx-license-identifier")
}

fn is_binary_content(content: &str) -> bool {
    content.contains('\0')
}

/// Pure content transform — no I/O, safe to fuzz directly with arbitrary
/// (including invalid-as-source, non-UTF-8-adjacent) text, since this is
/// exactly what runs against every file `ensure_headers` walks, including
/// ones lwoodz doesn't control the contents of.
pub fn apply_header(content: &str, cfg: &HeaderConfig, path: &Path) -> (String, HeaderAction) {
    if is_binary_content(content) {
        return (content.to_string(), HeaderAction::Skipped);
    }
    if has_copyright(content) {
        let expected_holder = cfg.holder.as_str();
        let expected_year = cfg.year.to_string();
        let expected_spdx = cfg.spdx.as_str();
        let has_holder = content.contains(expected_holder);
        let has_year = content.contains(&expected_year);
        let has_spdx = !cfg.insert_spdx || content.contains(expected_spdx);
        if has_holder && has_year && has_spdx {
            return (content.to_string(), HeaderAction::AlreadyPresent);
        }
        let new_header = header_text(cfg, path);
        let (shebang, rest) = if content.starts_with("#!") {
            if let Some(nl) = content.find('\n') {
                (&content[..nl + 1], &content[nl + 1..])
            } else {
                (content, "")
            }
        } else {
            ("", content)
        };
        // Strip old copyright/spdx lines from the top (first 5 lines)
        let lines: Vec<&str> = rest.lines().collect();
        let mut stripped = 0usize;
        for line in &lines {
            let l = line.to_lowercase();
            if l.contains("copyright") || l.contains("spdx-license-identifier") {
                stripped += 1;
            } else if stripped > 0 && line.trim().is_empty() {
                stripped += 1;
                break;
            } else {
                break;
            }
            if stripped >= 5 {
                break;
            }
        }
        let remaining = if stripped > 0 {
            lines[stripped..].join("\n")
        } else {
            rest.to_string()
        };
        // Ensure newline handling
        let sep = if remaining.is_empty() || remaining.starts_with('\n') {
            ""
        } else {
            "\n"
        };
        let new_content = if remaining.is_empty() {
            format!("{shebang}{new_header}{remaining}")
        } else {
            format!("{shebang}{new_header}{sep}{remaining}")
        };
        return (new_content, HeaderAction::Updated);
    }

    let new_header = header_text(cfg, path);
    let new_content = if content.starts_with("#!") {
        if let Some(nl) = content.find('\n') {
            format!("{}{new_header}{}", &content[..nl + 1], &content[nl + 1..])
        } else {
            format!("{content}{new_header}")
        }
    } else {
        format!("{new_header}{content}")
    };
    (new_content, HeaderAction::Inserted)
}

/// Process a single file: insert or update header.
pub fn process_file(path: &Path, cfg: &HeaderConfig) -> anyhow::Result<HeaderAction> {
    let content = std::fs::read_to_string(path)?;
    let (new_content, action) = apply_header(&content, cfg, path);
    if action == HeaderAction::Inserted || action == HeaderAction::Updated {
        std::fs::write(path, new_content)?;
    }
    Ok(action)
}

pub fn ensure_headers(
    repo_root: &Path,
    cfg: &HeaderConfig,
    exclude: &[String],
    include: &[String],
) -> anyhow::Result<Vec<HeaderResult>> {
    let mut results = Vec::new();

    let allowed_exts = [
        "rs", "go", "py", "js", "ts", "tsx", "jsx", "c", "cc", "cpp", "h", "hpp", "java", "kt",
        "swift", "rb", "sh", "toml", "yaml", "yml", "html", "css", "scss", "php", "pl", "lua",
        "dart", "elm", "ex", "exs", "hs", "ml", "scala",
    ];

    for entry in WalkDir::new(repo_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // Simple exclude matching (glob-like)
        let rel = path.strip_prefix(repo_root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().to_string();
        let rel_unix = rel_str.replace('\\', "/");
        let should_exclude = exclude.iter().any(|pat| {
            let p = pat.replace('\\', "/");
            let needle = p.trim_matches('*').trim_matches('/');
            if p.contains('*') {
                rel_unix.contains(needle)
            } else {
                rel_unix.starts_with(needle) || rel_unix.contains(needle)
            }
        }) || rel_unix.starts_with(".git/")
            || rel_unix.starts_with(".lwoodz/")
            || rel_unix.contains("/target/")
            || rel_unix.contains("target/");
        if should_exclude {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_allowed =
            allowed_exts.contains(&ext) || filename == "Makefile" || filename == "Dockerfile";
        if !is_allowed {
            continue;
        }
        if !include.is_empty() {
            let rel = path.strip_prefix(repo_root).unwrap_or(path);
            let rel_str = rel.to_string_lossy();
            if !include.iter().any(|pat| {
                let needle = pat.trim_matches('*').trim_matches('/');
                rel_str.contains(needle)
            }) {
                continue;
            }
        }
        match process_file(path, cfg) {
            Ok(action) => results.push(HeaderResult {
                path: path.to_path_buf(),
                action,
            }),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "header processing failed");
                results.push(HeaderResult {
                    path: path.to_path_buf(),
                    action: HeaderAction::Skipped,
                });
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_header_py() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hello.py");
        std::fs::write(&p, "x = 1\n").unwrap();
        let cfg = HeaderConfig {
            holder: "Acme".to_string(),
            year: 2024,
            spdx: "MIT".to_string(),
            insert_spdx: true,
        };
        let act = process_file(&p, &cfg).unwrap();
        assert_eq!(act, HeaderAction::Inserted);
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("Copyright"));
        assert!(content.contains("SPDX"));
    }

    #[test]
    fn already_present_noop() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("main.rs");
        let cfg = HeaderConfig {
            holder: "Acme".to_string(),
            year: 2026,
            spdx: "MIT".to_string(),
            insert_spdx: true,
        };
        let header = header_text(&cfg, &p);
        std::fs::write(&p, format!("{}fn main() {{}}", header)).unwrap();
        let act = process_file(&p, &cfg).unwrap();
        assert_eq!(act, HeaderAction::AlreadyPresent);
    }
}
