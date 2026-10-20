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
///
/// Both sides are parsed as SPDX expressions (see
/// [`crate::license::spdx::parse_expression`]), so legacy `MIT/Apache-2.0`,
/// parenthesised and mixed `AND`/`OR` expressions are evaluated with real
/// precedence: a dependency `OR` is compatible when *any* branch is (the
/// licensee picks), a dependency `AND` only when *every* part is.
pub fn check_compatibility(project_license: &str, dep_license: &str) -> Compatibility {
    use crate::license::spdx::{parse_expression, LicenseExpr};

    let proj = crate::license::spdx::normalize_spdx(project_license);
    let dep = crate::license::spdx::normalize_spdx(dep_license);
    if proj == dep {
        return Compatibility::Compatible;
    }

    // Project expression: the dependency must be compatible with every
    // license the project is offered under.
    if let Some(expr @ (LicenseExpr::Or(_) | LicenseExpr::And(_) | LicenseExpr::With(..))) =
        parse_expression(&proj)
    {
        let ids = leaf_ids(&expr);
        return ids
            .iter()
            .map(|p| check_compatibility(p, &dep))
            .fold(Compatibility::Compatible, worst);
    }

    match parse_expression(&dep) {
        Some(expr) => eval_dep(&proj, &expr),
        None => check_single(&proj, &dep),
    }
}

fn rank(c: Compatibility) -> u8 {
    match c {
        Compatibility::Compatible => 2,
        Compatibility::Warning => 1,
        Compatibility::Incompatible => 0,
    }
}

fn worst(a: Compatibility, b: Compatibility) -> Compatibility {
    if rank(a) <= rank(b) {
        a
    } else {
        b
    }
}

fn best(a: Compatibility, b: Compatibility) -> Compatibility {
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

fn leaf_ids(expr: &crate::license::spdx::LicenseExpr) -> Vec<String> {
    use crate::license::spdx::LicenseExpr;
    match expr {
        LicenseExpr::Id(id) => vec![id.clone()],
        LicenseExpr::With(base, _) => leaf_ids(base),
        LicenseExpr::And(parts) | LicenseExpr::Or(parts) => {
            parts.iter().flat_map(leaf_ids).collect()
        }
    }
}

fn eval_dep(proj: &str, expr: &crate::license::spdx::LicenseExpr) -> Compatibility {
    use crate::license::spdx::LicenseExpr;
    match expr {
        LicenseExpr::Id(id) => check_single(proj, id),
        LicenseExpr::With(base, _) => eval_dep(proj, base),
        LicenseExpr::Or(parts) => parts
            .iter()
            .map(|p| eval_dep(proj, p))
            .fold(Compatibility::Incompatible, best),
        LicenseExpr::And(parts) => parts
            .iter()
            .map(|p| eval_dep(proj, p))
            .fold(Compatibility::Compatible, worst),
    }
}

/// Single identifier vs single identifier.
fn check_single(proj: &str, dep: &str) -> Compatibility {
    let proj = proj.to_string();
    let dep = dep.to_string();
    if proj == dep {
        return Compatibility::Compatible;
    }

    // Apache-2.0's patent-termination and indemnity terms are "further
    // restrictions" under GPLv2 — incompatible even though Apache-2.0 is
    // otherwise permissive, so this must be decided before the permissive
    // shortcut below.
    if matches!(proj.as_str(), "GPL-2.0-only" | "GPL-2.0") && dep == "Apache-2.0" {
        return Compatibility::Incompatible;
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
        "MIT-0",
        "Unicode-3.0",
        "Unicode-DFS-2016",
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

    /// Regression: deckhand's `lwoodz --check` flagged
    /// `android_system_properties (MIT/Apache-2.0)` as COMPAT_WARNING against
    /// an MIT project. The legacy slash form means OR; the licensee picks MIT.
    #[test]
    fn legacy_slash_dual_license_is_compatible_with_mit() {
        for dep in [
            "MIT/Apache-2.0",
            "Apache-2.0/MIT",
            "MIT / Apache-2.0",
            "Apache-2.0 / MIT",
            "MIT OR Apache-2.0",
            "(MIT OR Apache-2.0)",
            "mit or apache-2.0",
            "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
            "(MIT OR Apache-2.0) AND Unicode-3.0",
            "Unlicense/MIT",
        ] {
            assert_eq!(
                check_compatibility("MIT", dep),
                Compatibility::Compatible,
                "{dep} should be compatible with MIT"
            );
        }
        let r = build_report(
            "MIT",
            &[(
                "android_system_properties".to_string(),
                "MIT/Apache-2.0".to_string(),
            )],
        );
        assert!(r.is_clean(), "{:?}", r.issues);
    }

    #[test]
    fn or_is_compatible_if_any_branch_is_and_requires_all() {
        assert_eq!(
            check_compatibility("MIT", "GPL-3.0-only OR MIT"),
            Compatibility::Compatible
        );
        assert_eq!(
            check_compatibility("MIT", "GPL-3.0-only/GPL-2.0-only"),
            Compatibility::Incompatible
        );
        // AND binds tighter than OR: this is MIT OR (Apache AND GPL) — MIT wins.
        assert_eq!(
            check_compatibility("MIT", "MIT OR Apache-2.0 AND GPL-3.0-only"),
            Compatibility::Compatible
        );
        // ...but (MIT OR Apache) AND GPL requires the GPL part.
        assert_eq!(
            check_compatibility("MIT", "(MIT OR Apache-2.0) AND GPL-3.0-only"),
            Compatibility::Incompatible
        );
        assert_eq!(
            check_compatibility("MIT", "MPL-2.0 OR GPL-3.0-only"),
            Compatibility::Warning
        );
    }

    #[test]
    fn gpl2_only_project_rejects_apache_dep() {
        // Previously short-circuited to Compatible by the permissive shortcut.
        assert_eq!(
            check_compatibility("GPL-2.0-only", "Apache-2.0"),
            Compatibility::Incompatible
        );
        assert_eq!(
            check_compatibility("GPL-2.0-only", "MIT OR Apache-2.0"),
            Compatibility::Compatible
        );
    }

    #[test]
    fn dual_licensed_project_requires_compat_with_each_branch() {
        assert_eq!(
            check_compatibility("MIT OR Apache-2.0", "BSD-3-Clause"),
            Compatibility::Compatible
        );
        assert_eq!(
            check_compatibility("MIT OR Apache-2.0", "GPL-3.0-only"),
            Compatibility::Incompatible
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
