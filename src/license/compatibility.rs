// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
/// Simplified compatibility matrix.
/// Rows = project license, Cols = dependency license.
/// Values: Compatible, Incompatible, Warning (check).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Compatibility {
    Compatible,
    Incompatible,
    Warning,
}

impl Compatibility {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Compatible)
    }
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Incompatible)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompatibilityIssue {
    pub dependency: String,
    pub dep_license: String,
    pub project_license: String,
    pub compatibility: Compatibility,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompatibilityReport {
    pub project_license: String,
    pub issues: Vec<CompatibilityIssue>,
    pub total_deps: usize,
    pub incompatible_count: usize,
    pub warning_count: usize,
}

impl CompatibilityReport {
    pub fn is_clean(&self) -> bool {
        self.incompatible_count == 0 && self.warning_count == 0
    }
    pub fn has_blockers(&self) -> bool {
        self.incompatible_count > 0
    }
}

/// Core compatibility rules — conservative, favors flagging over silence.
/// Based on FSF/OSI guidance + SPDX compatibility data.
pub fn check_compatibility(project_license: &str, dep_license: &str) -> Compatibility {
    let proj = crate::license::spdx::normalize_spdx(project_license);
    let dep = crate::license::spdx::normalize_spdx(dep_license);

    if proj == dep {
        return Compatibility::Compatible;
    }

    // Handle compound expressions (MIT OR Apache-2.0, etc.)
    if dep.contains(" OR ") {
        let parts: Vec<&str> = dep.split(" OR ").collect();
        // If any alternative is compatible, the whole expression is compatible (choose best license)
        let mut has_compatible = false;
        let mut has_incompatible = false;
        for part in &parts {
            let c = check_compatibility(
                project_license,
                part.trim().trim_matches(|c| c == '(' || c == ')'),
            );
            if c == Compatibility::Compatible {
                has_compatible = true;
            }
            if c == Compatibility::Incompatible {
                has_incompatible = true;
            }
        }
        if has_compatible {
            return Compatibility::Compatible;
        }
        if has_incompatible
            && parts.len()
                == parts
                    .iter()
                    .filter(|p| {
                        check_compatibility(project_license, p.trim())
                            == Compatibility::Incompatible
                    })
                    .count()
        {
            return Compatibility::Incompatible;
        }
        return Compatibility::Warning;
    }
    if dep.contains(" AND ") {
        let parts: Vec<&str> = dep.split(" AND ").collect();
        let mut worst = Compatibility::Compatible;
        for part in &parts {
            let c = check_compatibility(
                project_license,
                part.trim().trim_matches(|c| c == '(' || c == ')'),
            );
            if c == Compatibility::Incompatible {
                return Compatibility::Incompatible;
            }
            if c == Compatibility::Warning {
                worst = Compatibility::Warning;
            }
        }
        return worst;
    }

    // Permissive deps are compatible with almost everything.
    let permissive = [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Zlib",
        "BSL-1.0",
        "CC0-1.0",
        "Unlicense",
        "0BSD",
    ];
    if permissive.contains(&dep.as_str()) {
        return Compatibility::Compatible;
    }

    // Weak copyleft
    let weak = [
        "MPL-2.0",
        "LGPL-2.1-only",
        "LGPL-2.1",
        "LGPL-3.0-only",
        "LGPL-3.0",
        "LGPL-2.0-only",
    ];
    // Strong copyleft
    let strong = [
        "GPL-2.0-only",
        "GPL-2.0",
        "GPL-3.0-only",
        "GPL-3.0",
        "AGPL-3.0-only",
        "AGPL-3.0",
    ];

    match proj.as_str() {
        "MIT" | "BSD-2-Clause" | "BSD-3-Clause" | "ISC" | "Zlib" | "BSL-1.0" | "Unlicense"
        | "CC0-1.0" => {
            if weak.contains(&dep.as_str()) {
                return Compatibility::Warning;
            }
            if strong.contains(&dep.as_str()) {
                return Compatibility::Incompatible;
            }
            Compatibility::Warning
        }
        "Apache-2.0" => {
            if dep == "GPL-2.0-only" || dep == "GPL-2.0" {
                return Compatibility::Incompatible;
            } // Apache-2.0 incompatible with GPL-2.0
            if weak.contains(&dep.as_str()) {
                return Compatibility::Compatible;
            }
            if strong.contains(&dep.as_str()) {
                if dep.contains("3.0") {
                    return Compatibility::Compatible;
                } // Apache-2.0 compatible with GPL-3.0
                return Compatibility::Incompatible;
            }
            Compatibility::Compatible
        }
        "MPL-2.0" => {
            if permissive.contains(&dep.as_str()) {
                return Compatibility::Compatible;
            }
            if dep == "Apache-2.0" {
                return Compatibility::Compatible;
            }
            if strong.contains(&dep.as_str()) {
                return Compatibility::Warning;
            }
            Compatibility::Warning
        }
        "LGPL-2.1-only" | "LGPL-2.1" | "LGPL-3.0-only" | "LGPL-3.0" => {
            if strong.contains(&dep.as_str()) {
                return Compatibility::Warning;
            }
            Compatibility::Compatible
        }
        "GPL-2.0-only" | "GPL-2.0" => {
            if dep == "Apache-2.0" {
                return Compatibility::Incompatible;
            }
            if dep == "GPL-3.0-only" || dep == "GPL-3.0" {
                return Compatibility::Incompatible;
            } // v2-only vs v3
            Compatibility::Compatible // GPL-2.0 can absorb permissive + LGPL etc.
        }
        "GPL-3.0-only" | "GPL-3.0" | "AGPL-3.0-only" | "AGPL-3.0" => {
            // GPL-3.0 is broadly compatible as the strongest copyleft (can absorb anything GPL-compatible)
            if dep == "GPL-2.0-only" {
                return Compatibility::Incompatible;
            }
            Compatibility::Compatible
        }
        _ => Compatibility::Warning,
    }
}

pub fn build_report(project_license: &str, deps: &[(String, String)]) -> CompatibilityReport {
    let mut issues = Vec::new();
    let mut incompatible_count = 0;
    let mut warning_count = 0;

    for (dep_name, dep_license) in deps {
        let c = check_compatibility(project_license, dep_license);
        if c != Compatibility::Compatible {
            let reason = match c {
                Compatibility::Incompatible => format!(
                    "Dependency '{}' is licensed under '{}' which is incompatible with project license '{}'. Distribution may violate license terms.",
                    dep_name, dep_license, project_license
                ),
                Compatibility::Warning => format!(
                    "Dependency '{}' ({}) may impose additional obligations when combined with '{}'. Review copyleft / attribution requirements.",
                    dep_name, dep_license, project_license
                ),
                Compatibility::Compatible => unreachable!(),
            };
            if c == Compatibility::Incompatible {
                incompatible_count += 1;
            }
            if c == Compatibility::Warning {
                warning_count += 1;
            }
            issues.push(CompatibilityIssue {
                dependency: dep_name.clone(),
                dep_license: dep_license.clone(),
                project_license: project_license.to_string(),
                compatibility: c,
                reason,
            });
        }
    }

    CompatibilityReport {
        project_license: project_license.to_string(),
        issues,
        total_deps: deps.len(),
        incompatible_count,
        warning_count,
    }
}

/// Explain policy levels.
pub fn policy_allows(policy: &str, dep_license: &str) -> bool {
    match policy {
        "strict" => {
            // Only permissive
            [
                "MIT",
                "Apache-2.0",
                "BSD-2-Clause",
                "BSD-3-Clause",
                "ISC",
                "Zlib",
                "BSL-1.0",
                "CC0-1.0",
                "Unlicense",
                "0BSD",
            ]
            .contains(&crate::license::spdx::normalize_spdx(dep_license).as_str())
        }
        "permissive" => {
            // Permissive + weak copyleft
            ![
                "GPL-2.0-only",
                "GPL-2.0",
                "GPL-3.0-only",
                "GPL-3.0",
                "AGPL-3.0-only",
                "AGPL-3.0",
            ]
            .contains(&crate::license::spdx::normalize_spdx(dep_license).as_str())
        }
        _ => true, // copyleft-allowed allows everything
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpl_into_mit_is_incompatible() {
        assert_eq!(
            check_compatibility("MIT", "GPL-3.0-only"),
            Compatibility::Incompatible
        );
        assert_eq!(
            check_compatibility("MIT", "GPL-2.0-only"),
            Compatibility::Incompatible
        );
    }

    #[test]
    fn mit_into_gpl_is_compatible() {
        assert_eq!(
            check_compatibility("GPL-3.0-only", "MIT"),
            Compatibility::Compatible
        );
    }

    #[test]
    fn apache_gpl2_incompatible() {
        assert_eq!(
            check_compatibility("Apache-2.0", "GPL-2.0-only"),
            Compatibility::Incompatible
        );
    }

    #[test]
    fn apache_gpl3_compatible() {
        assert_eq!(
            check_compatibility("Apache-2.0", "GPL-3.0-only"),
            Compatibility::Compatible
        );
    }

    #[test]
    fn build_report_counts() {
        let deps = vec![
            ("dep-a".to_string(), "MIT".to_string()),
            ("dep-b".to_string(), "GPL-3.0-only".to_string()),
        ];
        let r = build_report("MIT", &deps);
        assert_eq!(r.incompatible_count, 1);
    }
}
