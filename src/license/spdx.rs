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
];

/// Normalize an SPDX expression: trim, canonicalize common aliases.
pub fn normalize_spdx(input: &str) -> String {
    let s = input.trim();
    match s {
        "Apache 2.0" | "Apache2" | "Apache-2" => "Apache-2.0".to_string(),
        "GPL-2.0+" | "GPL2+" => "GPL-2.0-or-later".to_string(),
        "GPL-3.0+" | "GPL3+" => "GPL-3.0-or-later".to_string(),
        "BSD2" | "BSD-2" => "BSD-2-Clause".to_string(),
        "BSD3" | "BSD-3" => "BSD-3-Clause".to_string(),
        "MPL2" | "MPL 2.0" => "MPL-2.0".to_string(),
        other => other.to_string(),
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
    fn detect_mit() {
        let txt = "MIT License\nPermission is hereby granted, free of charge";
        assert_eq!(detect_spdx_from_text(txt), Some("MIT".to_string()));
    }
}
