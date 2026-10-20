// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
/// Known SPDX identifiers (curated subset — covers >95% of ecosystem usage).
/// Full list at https://spdx.org/licenses/
pub const KNOWN_SPDX: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "GPL-2.0-only",
    "GPL-2.0-or-later",
    "GPL-2.0",
    "GPL-3.0-only",
    "GPL-3.0-or-later",
    "GPL-3.0",
    "LGPL-2.0-only",
    "LGPL-2.0-or-later",
    "LGPL-2.0",
    "LGPL-2.1-only",
    "LGPL-2.1-or-later",
    "LGPL-2.1",
    "LGPL-3.0-only",
    "LGPL-3.0-or-later",
    "LGPL-3.0",
    "AGPL-3.0-only",
    "AGPL-3.0-or-later",
    "AGPL-3.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
    "CDDL-1.0",
    "EPL-1.0",
    "EPL-2.0",
    "Unlicense",
    "CC0-1.0",
    "CC-BY-4.0",
    "CC-BY-SA-4.0",
    "BSL-1.0",
    "Zlib",
    "0BSD",
    "Python-2.0",
    "Artistic-2.0",
    "OFL-1.1",
    "MIT-0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
];

/// Normalize an SPDX expression: trim, canonicalize common aliases, and
/// rewrite the legacy slash form (`MIT/Apache-2.0`, `Apache-2.0 / MIT`) that
/// older crates still declare into the SPDX `OR` form it means.
pub fn normalize_spdx(input: &str) -> String {
    let s = input.trim();
    if let Some(or_form) = legacy_slash_to_or(s) {
        return or_form;
    }
    normalize_single(s)
}

/// `MIT/Apache-2.0` → `MIT OR Apache-2.0`. Only applies when every
/// slash-separated part looks like a bare license identifier, so a URL or a
/// `SEE LICENSE IN docs/LICENSE` string is never mangled.
fn legacy_slash_to_or(s: &str) -> Option<String> {
    if !s.contains('/') {
        return None;
    }
    let parts: Vec<&str> = s.split('/').map(str::trim).collect();
    let looks_like_id = |p: &&str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
    };
    if parts.len() < 2 || !parts.iter().all(looks_like_id) {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|p| normalize_single(p))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

fn normalize_single(s: &str) -> String {
    match s {
        "Apache 2.0" | "Apache2" | "Apache-2" => "Apache-2.0".to_string(),
        "GPL-2.0+" | "GPL2+" => "GPL-2.0-or-later".to_string(),
        "GPL-3.0+" | "GPL3+" => "GPL-3.0-or-later".to_string(),
        "BSD2" | "BSD-2" => "BSD-2-Clause".to_string(),
        "BSD3" | "BSD-3" => "BSD-3-Clause".to_string(),
        "MPL2" | "MPL 2.0" => "MPL-2.0".to_string(),
        other => KNOWN_SPDX
            .iter()
            .find(|known| known.eq_ignore_ascii_case(other))
            .map(|known| known.to_string())
            .unwrap_or_else(|| other.to_string()),
    }
}

/// A parsed SPDX license expression. `WITH` binds tightest, then `AND`,
/// then `OR` (SPDX spec, Annex D). The legacy `/` separator is read as `OR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseExpr {
    Id(String),
    /// `<license> WITH <exception>` — an exception only ever grants extra
    /// permissions, so for compatibility purposes it is its base license.
    With(Box<LicenseExpr>, String),
    And(Vec<LicenseExpr>),
    Or(Vec<LicenseExpr>),
}

/// Parse an SPDX expression (case-insensitive operators, parentheses,
/// legacy `/`). Returns `None` for input that isn't a well-formed
/// expression (unbalanced parens, dangling operator, empty).
pub fn parse_expression(input: &str) -> Option<LicenseExpr> {
    let mut tokens = Vec::new();
    let spaced = input
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace('/', " / ");
    for raw in spaced.split_whitespace() {
        let tok = match raw.to_ascii_uppercase().as_str() {
            "AND" => Tok::And,
            "OR" | "/" => Tok::Or,
            "WITH" => Tok::With,
            "(" => Tok::Open,
            ")" => Tok::Close,
            _ => Tok::Id(normalize_single(raw)),
        };
        tokens.push(tok);
    }
    if tokens.is_empty() {
        return None;
    }
    let mut pos = 0;
    let expr = parse_or(&tokens, &mut pos)?;
    (pos == tokens.len()).then_some(expr)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Id(String),
    And,
    Or,
    With,
    Open,
    Close,
}

fn parse_or(t: &[Tok], pos: &mut usize) -> Option<LicenseExpr> {
    let mut parts = vec![parse_and(t, pos)?];
    while t.get(*pos) == Some(&Tok::Or) {
        *pos += 1;
        parts.push(parse_and(t, pos)?);
    }
    Some(if parts.len() == 1 {
        parts.pop().unwrap()
    } else {
        LicenseExpr::Or(parts)
    })
}

fn parse_and(t: &[Tok], pos: &mut usize) -> Option<LicenseExpr> {
    let mut parts = vec![parse_with(t, pos)?];
    while t.get(*pos) == Some(&Tok::And) {
        *pos += 1;
        parts.push(parse_with(t, pos)?);
    }
    Some(if parts.len() == 1 {
        parts.pop().unwrap()
    } else {
        LicenseExpr::And(parts)
    })
}

fn parse_with(t: &[Tok], pos: &mut usize) -> Option<LicenseExpr> {
    let base = parse_atom(t, pos)?;
    if t.get(*pos) == Some(&Tok::With) {
        *pos += 1;
        if let Some(Tok::Id(exc)) = t.get(*pos) {
            *pos += 1;
            return Some(LicenseExpr::With(Box::new(base), exc.clone()));
        }
        return None;
    }
    Some(base)
}

fn parse_atom(t: &[Tok], pos: &mut usize) -> Option<LicenseExpr> {
    match t.get(*pos)? {
        Tok::Id(id) => {
            *pos += 1;
            Some(LicenseExpr::Id(id.clone()))
        }
        Tok::Open => {
            *pos += 1;
            let inner = parse_or(t, pos)?;
            if t.get(*pos) != Some(&Tok::Close) {
                return None;
            }
            *pos += 1;
            Some(inner)
        }
        _ => None,
    }
}

pub fn is_valid_spdx(id: &str) -> bool {
    let norm = normalize_spdx(id);
    KNOWN_SPDX.contains(&norm.as_str())
        || norm.contains(" AND ")
        || norm.contains(" OR ")
        || norm.contains(" WITH ")
}

#[derive(Debug, Clone)]
pub struct SpdxExpression {
    pub raw: String,
    pub normalized: String,
    pub is_compound: bool,
}

impl SpdxExpression {
    pub fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let normalized = normalize_spdx(&raw);
        let is_compound = normalized.contains(" AND ")
            || normalized.contains(" OR ")
            || normalized.contains(" WITH ");
        Self {
            raw,
            normalized,
            is_compound,
        }
    }

    pub fn identifiers(&self) -> Vec<String> {
        // Naive split on AND/OR/WITH and parens for compound expressions.
        let cleaned = self.normalized.replace(['(', ')'], " ");
        let parts: Vec<String> = cleaned
            .split(' ')
            .filter(|s| !s.is_empty() && *s != "AND" && *s != "OR" && *s != "WITH")
            .map(|s| s.to_string())
            .collect();
        if parts.is_empty() {
            vec![self.normalized.clone()]
        } else {
            parts
        }
    }

    pub fn is_known(&self) -> bool {
        self.identifiers()
            .iter()
            .all(|id| KNOWN_SPDX.contains(&id.as_str()) || id == "NOASSERTION" || id == "NONE")
    }
}

/// Generate an SPDX LicenseRef mapping for unknown licenses.
pub fn license_ref(name: &str) -> String {
    let slug = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase();
    format!("LicenseRef-{}", slug)
}

pub fn all_spdx_ids() -> Vec<&'static str> {
    KNOWN_SPDX.to_vec()
}

pub fn detect_spdx_from_text(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    // Heuristic sniffing for common license texts.
    if lower.contains("mit license")
        || lower.contains("permission is hereby granted, free of charge")
    {
        return Some("MIT".to_string());
    }
    if lower.contains("apache license") && lower.contains("version 2.0") {
        return Some("Apache-2.0".to_string());
    }
    if lower.contains("gnu general public license") {
        if lower.contains("version 3") {
            return Some("GPL-3.0-only".to_string());
        }
        if lower.contains("version 2") {
            return Some("GPL-2.0-only".to_string());
        }
        return Some("GPL-3.0-only".to_string());
    }
    if lower.contains("bsd 2-clause") || lower.contains("bsd-2-clause") {
        return Some("BSD-2-Clause".to_string());
    }
    if lower.contains("bsd 3-clause") || lower.contains("bsd-3-clause") {
        return Some("BSD-3-Clause".to_string());
    }
    if lower.contains("mozilla public license") && lower.contains("2.0") {
        return Some("MPL-2.0".to_string());
    }
    if lower.contains("the unlicense") || lower.contains("unlicense") {
        return Some("Unlicense".to_string());
    }
    if lower.contains("isc license") {
        return Some("ISC".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_aliases() {
        assert_eq!(normalize_spdx("Apache 2.0"), "Apache-2.0");
        assert_eq!(normalize_spdx("MIT"), "MIT");
    }

    #[test]
    fn compound_detection() {
        let e = SpdxExpression::parse("MIT AND Apache-2.0");
        assert!(e.is_compound);
        assert_eq!(e.identifiers(), vec!["MIT", "Apache-2.0"]);
    }

    #[test]
    fn legacy_slash_forms_normalize_to_or() {
        assert_eq!(normalize_spdx("MIT/Apache-2.0"), "MIT OR Apache-2.0");
        assert_eq!(normalize_spdx("Apache-2.0/MIT"), "Apache-2.0 OR MIT");
        assert_eq!(normalize_spdx("MIT / Apache-2.0"), "MIT OR Apache-2.0");
        assert!(is_valid_spdx("MIT/Apache-2.0"));
        // Not an identifier list — must be left alone.
        assert_eq!(
            normalize_spdx("SEE LICENSE IN docs/LICENSE"),
            "SEE LICENSE IN docs/LICENSE"
        );
    }

    #[test]
    fn expression_precedence_with_and_or() {
        use LicenseExpr::*;
        let id = |s: &str| Id(s.to_string());
        assert_eq!(
            parse_expression("(MIT OR Apache-2.0) AND Unicode-3.0"),
            Some(And(vec![
                Or(vec![id("MIT"), id("Apache-2.0")]),
                id("Unicode-3.0")
            ]))
        );
        assert_eq!(
            parse_expression("MIT OR Apache-2.0 AND BSD-3-Clause"),
            Some(Or(vec![
                id("MIT"),
                And(vec![id("Apache-2.0"), id("BSD-3-Clause")])
            ]))
        );
        assert_eq!(
            parse_expression("Apache-2.0 WITH LLVM-exception OR MIT"),
            Some(Or(vec![
                With(Box::new(id("Apache-2.0")), "LLVM-exception".to_string()),
                id("MIT")
            ]))
        );
        assert_eq!(
            parse_expression("MIT/Apache-2.0"),
            Some(Or(vec![id("MIT"), id("Apache-2.0")]))
        );
        assert_eq!(
            parse_expression("mit or apache-2.0"),
            Some(Or(vec![id("MIT"), id("Apache-2.0")]))
        );
        assert_eq!(parse_expression("(MIT OR"), None);
        assert_eq!(parse_expression(""), None);
    }

    #[test]
    fn detect_mit() {
        let txt = "MIT License\nPermission is hereby granted, free of charge";
        assert_eq!(detect_spdx_from_text(txt), Some("MIT".to_string()));
    }
}
