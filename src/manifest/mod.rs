// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
pub mod cargo;
pub mod go;
pub mod npm;
pub mod python;

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub license: Option<String>,
    pub source: String, // "cargo" | "npm" | "python" | "go"
}

#[derive(Debug, Clone)]
pub struct ManifestReport {
    pub dependencies: Vec<Dependency>,
    pub sources: Vec<String>,
}

impl ManifestReport {
    pub fn len(&self) -> usize {
        self.dependencies.len()
    }
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }
    pub fn license_pairs(&self) -> Vec<(String, String)> {
        self.dependencies
            .iter()
            .filter_map(|d| d.license.as_ref().map(|l| (d.name.clone(), l.clone())))
            .collect()
    }
}

/// One ecosystem's dependency scanner. Adding support for a new ecosystem
/// (Maven, Gradle, NuGet, RubyGems, ...) means implementing this trait for a
/// new type and adding one line to [`scanners`] — [`scan`] itself never
/// needs to change.
pub trait Scanner {
    /// Short, lowercase ecosystem name — matches `Dependency::source` and is
    /// what shows up in `ManifestReport::sources`.
    fn name(&self) -> &'static str;

    /// Returns `None` when this ecosystem's manifest isn't present in
    /// `repo_root` at all; `Some(deps)` (possibly empty) once it is,
    /// regardless of whether every license resolved.
    fn scan(&self, repo_root: &Path) -> Option<Vec<Dependency>>;
}

struct CargoScanner;
impl Scanner for CargoScanner {
    fn name(&self) -> &'static str {
        "cargo"
    }
    fn scan(&self, repo_root: &Path) -> Option<Vec<Dependency>> {
        cargo::scan(repo_root)
    }
}

struct NpmScanner;
impl Scanner for NpmScanner {
    fn name(&self) -> &'static str {
        "npm"
    }
    fn scan(&self, repo_root: &Path) -> Option<Vec<Dependency>> {
        npm::scan(repo_root)
    }
}

struct PythonScanner;
impl Scanner for PythonScanner {
    fn name(&self) -> &'static str {
        "python"
    }
    fn scan(&self, repo_root: &Path) -> Option<Vec<Dependency>> {
        python::scan(repo_root)
    }
}

struct GoScanner;
impl Scanner for GoScanner {
    fn name(&self) -> &'static str {
        "go"
    }
    fn scan(&self, repo_root: &Path) -> Option<Vec<Dependency>> {
        go::scan(repo_root)
    }
}

/// The registry: every ecosystem lwoodz knows how to scan. This is the only
/// place a new ecosystem needs to be wired in.
fn scanners() -> Vec<Box<dyn Scanner>> {
    vec![
        Box::new(CargoScanner),
        Box::new(NpmScanner),
        Box::new(PythonScanner),
        Box::new(GoScanner),
    ]
}

pub fn scan(repo_root: &Path) -> ManifestReport {
    let mut deps = Vec::new();
    let mut sources = Vec::new();

    for scanner in scanners() {
        if let Some(mut v) = scanner.scan(repo_root) {
            if !v.is_empty() {
                sources.push(scanner.name().to_string());
                deps.append(&mut v);
            }
        }
    }

    // Deduplicate by name (keep first)
    let mut seen = std::collections::HashSet::new();
    let mut uniq = Vec::new();
    for d in deps {
        if seen.insert(d.name.clone()) {
            uniq.push(d);
        }
    }

    ManifestReport {
        dependencies: uniq,
        sources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_match_dependency_source_convention() {
        let names: Vec<&str> = scanners().iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["cargo", "npm", "python", "go"]);
    }

    #[test]
    fn scan_empty_dir_returns_empty_report() {
        let dir = tempfile::tempdir().unwrap();
        let report = scan(dir.path());
        assert!(report.is_empty());
        assert!(report.sources.is_empty());
    }
}
