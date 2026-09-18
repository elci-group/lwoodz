// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! Open Source vs Open Standard differentiation.
//!
//! These are two independent axes, not synonyms:
//!
//! - **Open source** describes the *copyright license* a dependency's code
//!   is distributed under — does it grant the freedoms the OSI's Open
//!   Source Definition requires ([`OpenSourceStatus`]).
//! - **Open standard** describes whether a dependency *implements a
//!   specification governed by a recognized standards body* (IETF, W3C,
//!   WHATWG, ISO/IEC, the Unicode Consortium, an open industry
//!   consortium, ...), independent of that code's own license
//!   ([`StandardReference`]).
//!
//! A dependency can be either, both, or neither: an OSI-approved MIT crate
//! implementing no standard at all; a proprietary, closed-source library
//! implementing an open standard; or an open-source implementation of a
//! standard that is itself *not* royalty-free — governance by a standards
//! body does not by itself mean unencumbered. AV1 (an open, royalty-free
//! codec by its consortium's charter) and H.264 (an ISO/IEC and ITU-T
//! standard with a RAND patent-pool licensing regime) are both "open
//! standards" under a materially different royalty commitment; conflating
//! the two would hide that difference from a licensing decision.
//!
//! Like [`crate::assessment`], this module produces evidence, not a
//! recommendation about which dependency to keep, replace or license
//! under what terms — that judgment belongs to the consumer of the
//! evidence (a release reviewer, or a downstream tool such as Amber).
use crate::license::spdx::SpdxExpression;
use crate::license::templates::LicenseId;
use crate::manifest::ManifestReport;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

pub const SCHEMA: &str = "lwoodz.openness-analysis/v1";
pub const REVIEWED: &str = "2026-09-18";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenSourceStatus {
    /// The declared license resolves to an identifier on the OSI's
    /// approved list.
    OsiApproved,
    /// A public-domain dedication (CC0-1.0): free to use, but not itself
    /// an OSI-approved *license* for software.
    PublicDomainEquivalent,
    /// No license resolved, or none declared — absence of evidence, not
    /// evidence of a proprietary/closed license.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardsBody {
    Ietf,
    W3c,
    Whatwg,
    IsoIec,
    Unicode,
    /// A multi-vendor open consortium that publishes a specification
    /// outside the classic SDO process (e.g. the Alliance for Open
    /// Media), still openly governed but not a formal standards body.
    IndustryConsortium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoyaltyStatus {
    RoyaltyFree,
    ReasonableAndNonDiscriminatory,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardReference {
    pub body: StandardsBody,
    pub name: String,
    pub citation: String,
    pub royalty_status: RoyaltyStatus,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Openness {
    pub open_source: OpenSourceStatus,
    pub open_standard: Option<StandardReference>,
}

/// Classifies a (possibly compound, `OR`-joined) SPDX expression. Unresolved
/// identifiers are ignored rather than treated as disqualifying — an `OR`
/// expression only needs one resolvable branch to offer an OSI-approved
/// choice. An expression that resolves nothing yields [`OpenSourceStatus::Unknown`].
pub fn classify_license(expression: &str) -> OpenSourceStatus {
    let ids = SpdxExpression::parse(expression.to_string()).identifiers();
    let mut best = OpenSourceStatus::Unknown;
    for id in &ids {
        let status = match LicenseId::from_spdx(id) {
            Some(LicenseId::Cc0_10) => OpenSourceStatus::PublicDomainEquivalent,
            Some(_) => OpenSourceStatus::OsiApproved,
            None => OpenSourceStatus::Unknown,
        };
        if rank(status) > rank(best) {
            best = status;
        }
    }
    best
}

fn rank(status: OpenSourceStatus) -> u8 {
    match status {
        OpenSourceStatus::OsiApproved => 2,
        OpenSourceStatus::PublicDomainEquivalent => 1,
        OpenSourceStatus::Unknown => 0,
    }
}

fn reference(
    body: StandardsBody,
    name: &str,
    citation: &str,
    royalty_status: RoyaltyStatus,
    url: &str,
) -> StandardReference {
    StandardReference {
        body,
        name: name.into(),
        citation: citation.into(),
        royalty_status,
        url: url.into(),
    }
}

/// The maintained set of known standard-implementing dependencies. Match
/// tokens are compared against a dependency's `-`/`_`/`.`/`/`-separated
/// name segments (never a raw substring — "curl" must never match a "url"
/// pattern), so extend this by adding a distinctive token rather than
/// widening one to a fragment that could appear inside an unrelated name.
fn standards_catalog() -> Vec<(Vec<&'static str>, StandardReference)> {
    use RoyaltyStatus::{ReasonableAndNonDiscriminatory, RoyaltyFree};
    use StandardsBody::{Ietf, IndustryConsortium, IsoIec, Unicode, Whatwg};
    vec![
        (
            vec!["rustls", "openssl", "boringssl", "mbedtls", "tls"],
            reference(
                Ietf,
                "Transport Layer Security (TLS) 1.3",
                "RFC 8446",
                RoyaltyFree,
                "https://www.rfc-editor.org/rfc/rfc8446",
            ),
        ),
        (
            vec!["webpki", "x509"],
            reference(
                Ietf,
                "X.509 Public Key Infrastructure Certificate",
                "RFC 5280",
                RoyaltyFree,
                "https://www.rfc-editor.org/rfc/rfc5280",
            ),
        ),
        (
            vec!["hyper", "h2", "httparse", "http"],
            reference(
                Ietf,
                "Hypertext Transfer Protocol (HTTP)",
                "RFC 9110 / RFC 9113",
                RoyaltyFree,
                "https://www.rfc-editor.org/rfc/rfc9110",
            ),
        ),
        (
            vec!["tungstenite", "websocket", "ws"],
            reference(
                Ietf,
                "The WebSocket Protocol",
                "RFC 6455",
                RoyaltyFree,
                "https://www.rfc-editor.org/rfc/rfc6455",
            ),
        ),
        (
            vec!["url", "idna"],
            reference(
                Whatwg,
                "URL Standard",
                "WHATWG URL Living Standard",
                RoyaltyFree,
                "https://url.spec.whatwg.org/",
            ),
        ),
        (
            vec!["json"],
            reference(
                Ietf,
                "JavaScript Object Notation (JSON)",
                "RFC 8259 (also ECMA-404)",
                RoyaltyFree,
                "https://www.rfc-editor.org/rfc/rfc8259",
            ),
        ),
        (
            vec!["unicode", "icu", "icu4x"],
            reference(
                Unicode,
                "The Unicode Standard",
                "Unicode 15.x",
                RoyaltyFree,
                "https://www.unicode.org/standard/standard.html",
            ),
        ),
        (
            vec!["openh264", "x264", "avc"],
            reference(
                IsoIec,
                "Advanced Video Coding (H.264/AVC)",
                "ITU-T H.264 / ISO/IEC 14496-10",
                ReasonableAndNonDiscriminatory,
                "https://www.itu.int/rec/T-REC-H.264",
            ),
        ),
        (
            vec!["rav1e", "dav1d", "aom", "av1"],
            reference(
                IndustryConsortium,
                "AV1 Video Codec",
                "Alliance for Open Media",
                RoyaltyFree,
                "https://aomedia.org/av1/",
            ),
        ),
    ]
}

fn tokens(name: &str) -> Vec<String> {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[must_use]
pub fn standard_for(dependency_name: &str) -> Option<StandardReference> {
    static CATALOG: OnceLock<Vec<(Vec<&'static str>, StandardReference)>> = OnceLock::new();
    let catalog = CATALOG.get_or_init(standards_catalog);
    let toks = tokens(dependency_name);
    catalog
        .iter()
        .find(|(patterns, _)| patterns.iter().any(|p| toks.iter().any(|t| t == p)))
        .map(|(_, reference)| reference.clone())
}

#[must_use]
pub fn classify(dependency_name: &str, license_expression: Option<&str>) -> Openness {
    Openness {
        open_source: license_expression.map_or(OpenSourceStatus::Unknown, classify_license),
        open_standard: standard_for(dependency_name),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyOpenness {
    pub dependency: String,
    pub ecosystem: String,
    pub license: Option<String>,
    #[serde(flatten)]
    pub openness: Openness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: String,
    pub rules_reviewed_on: String,
    pub limitations: Vec<String>,
    pub dependencies: Vec<DependencyOpenness>,
}

impl Report {
    pub fn standard_encumbered(&self) -> impl Iterator<Item = &DependencyOpenness> {
        self.dependencies.iter().filter(|d| {
            matches!(
                d.openness.open_standard.as_ref().map(|s| s.royalty_status),
                Some(RoyaltyStatus::ReasonableAndNonDiscriminatory)
            )
        })
    }
}

pub fn analyze(manifest: &ManifestReport) -> Report {
    let dependencies = manifest
        .dependencies
        .iter()
        .map(|d| DependencyOpenness {
            dependency: d.name.clone(),
            ecosystem: d.source.clone(),
            license: d.license.clone(),
            openness: classify(&d.name, d.license.as_deref()),
        })
        .collect();
    Report {
        schema_version: SCHEMA.into(),
        rules_reviewed_on: REVIEWED.into(),
        limitations: vec![
            "Declared-context evidence, not legal advice. Open-source status is derived from the SPDX expression a scanner already resolved; open-standard status is matched by dependency name against a maintained, necessarily incomplete catalog.".into(),
            "A standards body governing a specification is independent of that specification being royalty-free: some standards carry a RAND/FRAND patent-licensing commitment instead. Recheck the cited body's current policy and any applicable patent pool before relying on a royalty-free assumption.".into(),
            "'Public domain equivalent' (e.g. CC0-1.0) is free to use but is not on the OSI's approved-license list; recheck https://opensource.org/licenses for the current list before treating any result here as OSI approval.".into(),
        ],
        dependencies,
    }
}

pub fn render(report: &Report) -> String {
    let mut out = format!(
        "Open-source / open-standard differentiation ({}) — catalog reviewed {}\n",
        report.schema_version, report.rules_reviewed_on
    );
    for note in &report.limitations {
        out.push_str(&format!("Note: {note}\n"));
    }
    let osi = report
        .dependencies
        .iter()
        .filter(|d| d.openness.open_source == OpenSourceStatus::OsiApproved)
        .count();
    let standard = report
        .dependencies
        .iter()
        .filter(|d| d.openness.open_standard.is_some())
        .count();
    out.push_str(&format!(
        "\n{} dependencies: {} OSI-approved license, {} implement a catalogued open standard.\n",
        report.dependencies.len(),
        osi,
        standard
    ));
    for d in &report.dependencies {
        if d.openness.open_standard.is_none()
            && d.openness.open_source == OpenSourceStatus::OsiApproved
        {
            continue; // the common, unremarkable case — keep the listing focused
        }
        out.push_str(&format!(
            "\n{} ({}) — license {}: {:?}\n",
            d.dependency,
            d.ecosystem,
            d.license.as_deref().unwrap_or("unknown"),
            d.openness.open_source
        ));
        if let Some(s) = &d.openness.open_standard {
            out.push_str(&format!(
                "  Open standard: {} [{}] royalty: {:?}\n  Reference: {}\n",
                s.name, s.citation, s.royalty_status, s.url
            ));
        }
    }
    let encumbered: Vec<&str> = report
        .standard_encumbered()
        .map(|d| d.dependency.as_str())
        .collect();
    if !encumbered.is_empty() {
        out.push_str(&format!(
            "\nRAND/FRAND-encumbered standard implementations requiring patent review: {}\n",
            encumbered.join(", ")
        ));
    }
    out
}

pub fn run(repo_root: &Path, json: bool) -> anyhow::Result<()> {
    let manifest = crate::manifest::scan(repo_root);
    let report = analyze(&manifest);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render(&report));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Dependency;

    fn dep(name: &str, license: Option<&str>, source: &str) -> Dependency {
        Dependency {
            name: name.into(),
            version: Some("1.0.0".into()),
            license: license.map(Into::into),
            source: source.into(),
        }
    }

    #[test]
    fn osi_approved_license_implementing_a_royalty_free_standard() {
        let o = classify("rustls", Some("Apache-2.0 OR MIT"));
        assert_eq!(o.open_source, OpenSourceStatus::OsiApproved);
        let std = o.open_standard.expect("rustls implements TLS");
        assert_eq!(std.royalty_status, RoyaltyStatus::RoyaltyFree);
    }

    #[test]
    fn cc0_is_public_domain_equivalent_not_osi_approved() {
        assert_eq!(
            classify_license("CC0-1.0"),
            OpenSourceStatus::PublicDomainEquivalent
        );
    }

    #[test]
    fn missing_license_is_unknown_not_proprietary() {
        let o = classify("some-crate", None);
        assert_eq!(o.open_source, OpenSourceStatus::Unknown);
    }

    #[test]
    fn short_token_patterns_do_not_false_positive_on_substrings() {
        // "curl" must never match the "url" standard pattern via naive substring matching.
        assert!(standard_for("curl-sys").is_none());
        assert!(standard_for("url").is_some());
    }

    #[test]
    fn same_domain_standards_can_carry_different_royalty_regimes() {
        let h264 = standard_for("openh264-sys").unwrap();
        let av1 = standard_for("dav1d").unwrap();
        assert_eq!(
            h264.royalty_status,
            RoyaltyStatus::ReasonableAndNonDiscriminatory
        );
        assert_eq!(av1.royalty_status, RoyaltyStatus::RoyaltyFree);
    }

    #[test]
    fn report_flags_rand_encumbered_dependencies() {
        let manifest = ManifestReport {
            dependencies: vec![
                dep("openh264", Some("BSD-2-Clause"), "cargo"),
                dep("dav1d", Some("BSD-2-Clause"), "cargo"),
                dep("serde", Some("MIT OR Apache-2.0"), "cargo"),
            ],
            sources: vec!["cargo".into()],
        };
        let report = analyze(&manifest);
        let flagged: Vec<&str> = report
            .standard_encumbered()
            .map(|d| d.dependency.as_str())
            .collect();
        assert_eq!(flagged, vec!["openh264"]);
    }

    #[test]
    fn every_catalog_entry_has_distinct_nonempty_patterns() {
        for (patterns, reference) in standards_catalog() {
            assert!(
                !patterns.is_empty(),
                "{} has no match patterns",
                reference.name
            );
            assert!(!reference.citation.is_empty());
            assert!(!reference.url.is_empty());
        }
    }
}
